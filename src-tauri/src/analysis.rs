use rustfft::FftPlanner;
use std::f64::consts::PI;
use symphonia::core::audio::{AudioBuffer, Signal, SignalSpec};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AnalysisResult {
    pub bpm: f64,
    pub bpm_confidence: f64,
    pub key_camelot: String,
    pub key_standard: String,
    pub key_confidence: f64,
    pub downbeat_offset_seconds: f64,
    pub beat_positions_seconds: Vec<f64>,
    pub duration_seconds: f64,
    pub sections: Vec<SectionData>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SectionData {
    pub name: String,
    pub start_bar: u32,
    pub bars: u32,
    pub color: String,
}

#[derive(Clone)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration_seconds: f64,
    pub num_channels: u16,
}

// ---------------------------------------------------------------------------
// Audio loading with symphonia
// ---------------------------------------------------------------------------

pub fn load_audio(path: &std::path::Path) -> Result<AudioData, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open file: {}", e))?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());

    let probed = symphonia::default::get_probe()
        .format(
            &symphonia::core::probe::Hint::new(),
            mss,
            &symphonia::core::formats::FormatOptions::default(),
            &symphonia::core::meta::MetadataOptions::default(),
        )
        .map_err(|e| format!("Cannot probe format: {}", e))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CodecParameters::default().codec)
        .or_else(|| format.tracks().first())
        .ok_or_else(|| "No audio tracks found".to_string())?;

    let codec_params = track.codec_params.clone();
    let track_id = track.id;

    let mut codec = symphonia::default::get_codecs()
        .make(&codec_params, &symphonia::core::codecs::DecoderOptions::default())
        .map_err(|e| format!("Cannot create decoder: {}", e))?;

    let sample_rate = codec_params.sample_rate.unwrap_or(44100);
    let channels_spec = codec_params.channels.unwrap_or_default();
    let num_channels = channels_spec.count() as u16;

    // Accumulate decoded frames — interleaved f32
    let mut accum: Vec<f32> = Vec::new();

    loop {
        match format.next_packet() {
            Ok(packet) => {
                if packet.track_id() != track_id {
                    continue;
                }
                let decoded = match codec.decode(&packet) {
                    Ok(d) => d,
                    Err(_) => continue,
                };

                let frames = decoded.frames();
                if frames == 0 || num_channels == 0 {
                    continue;
                }

                // Convert decoded buffer to f32 AudioBuffer
                let spec = SignalSpec::new(sample_rate, channels_spec);
                let mut dest = AudioBuffer::new(frames as u64, spec);
                decoded.convert(&mut dest);

                // Extract interleaved samples
                for frame in 0..frames {
                    for ch in 0..num_channels as usize {
                        accum.push(dest.chan(ch)[frame]);
                    }
                }
            }
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(_) => break,
        }
    }

    if accum.is_empty() {
        return Err("No audio data decoded".to_string());
    }

    let total_frames = accum.len() / num_channels as usize;
    let duration_seconds = total_frames as f64 / sample_rate as f64;

    // De-interleave to mono (average channels)
    let mut mono = vec![0.0f32; total_frames];
    for frame in 0..total_frames {
        let mut sum = 0.0f32;
        for ch in 0..num_channels as usize {
            sum += accum[frame * num_channels as usize + ch];
        }
        mono[frame] = sum / num_channels as f32;
    }

    Ok(AudioData {
        samples: mono,
        sample_rate,
        duration_seconds,
        num_channels: 1,
    })
}

// ---------------------------------------------------------------------------
// BPM Detection via autocorrelation on onset envelope
// ---------------------------------------------------------------------------

pub fn detect_bpm(audio: &AudioData) -> (f64, f64) {
    let hop_size = 256usize;
    let env_rate = audio.sample_rate as f32 / hop_size as f32;

    // RMS energy envelope
    let mut envelope: Vec<f32> = Vec::new();
    let mut j = 0;
    while j + hop_size <= audio.samples.len() {
        let frame = &audio.samples[j..j + hop_size];
        let rms = (frame.iter().map(|&s| s * s).sum::<f32>() / hop_size as f32).sqrt();
        envelope.push(rms);
        j += hop_size;
    }

    if envelope.len() < 4 {
        return (128.0, 0.0);
    }

    // Rectified first difference
    let mut onset: Vec<f32> = Vec::with_capacity(envelope.len());
    onset.push(0.0);
    for w in envelope.windows(2) {
        let diff = w[1] - w[0];
        onset.push(if diff > 0.0 { diff } else { 0.0 });
    }

    // Remove DC
    let mean = onset.iter().sum::<f32>() / onset.len() as f32;
    let onset_ac: Vec<f32> = onset.iter().map(|&s| s - mean).collect();

    // Autocorrelation for lag range 60–200 BPM
    let min_lag = (env_rate / 200.0) as usize;
    let max_lag = (env_rate / 60.0) as usize;

    if min_lag >= onset_ac.len() || max_lag >= onset_ac.len() {
        return (128.0, 0.0);
    }

    let n = onset_ac.len();
    let mut correlations: Vec<(usize, f32)> = Vec::new();

    for lag in min_lag..=max_lag.min(n - 1) {
        let mut corr = 0.0f32;
        for k in 0..(n - lag) {
            corr += onset_ac[k] * onset_ac[k + lag];
        }
        corr /= (n - lag) as f32;
        correlations.push((lag, corr));
    }

    if correlations.is_empty() {
        return (128.0, 0.0);
    }

    let max_corr = correlations.iter().map(|(_, c)| *c).fold(f32::NEG_INFINITY, f32::max);
    let mean_corr = correlations.iter().map(|(_, c)| *c).sum::<f32>() / correlations.len() as f32;

    let best = correlations.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
    let bpm = 60.0 * env_rate as f64 / best.0 as f64;

    let confidence = if mean_corr > 0.0 {
        ((max_corr / mean_corr) as f64).min(1.0)
    } else {
        0.0
    };
    let confidence = if confidence.is_nan() { 0.0 } else { confidence };

    ((bpm * 10.0).round() / 10.0, confidence.min(1.0))
}

// ---------------------------------------------------------------------------
// Beat grid generation
// ---------------------------------------------------------------------------

pub fn detect_beats(audio: &AudioData, bpm: f64) -> (f64, Vec<f64>) {
    let beat_interval_samples = audio.sample_rate as f64 * 60.0 / bpm;
    let hop_size = 256usize;

    // Search first few beats for downbeat
    let search_end = ((beat_interval_samples * 4.0) as usize).min(audio.samples.len());

    let mut onset_strength: Vec<f32> = Vec::new();
    let mut i = 0;
    while i + hop_size <= search_end {
        let frame = &audio.samples[i..i + hop_size];
        let rms = (frame.iter().map(|&s| s * s).sum::<f32>() / hop_size as f32).sqrt();
        onset_strength.push(rms);
        i += hop_size;
    }

    if onset_strength.is_empty() {
        return (0.0, vec![]);
    }

    let mut sorted = onset_strength.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[sorted.len() / 2];
    let threshold = median * 1.5;

    let first = onset_strength.iter().position(|&s| s > threshold).unwrap_or(0);
    let offset_sec = (first as f64 * hop_size as f64) / audio.sample_rate as f64;

    // Generate beat positions from downbeat
    let total_samples = audio.samples.len();
    let mut beats: Vec<f64> = Vec::new();
    let mut counter = 0;

    loop {
        let pos = first as f64 * hop_size as f64 + counter as f64 * beat_interval_samples;
        if pos >= total_samples as f64 {
            break;
        }
        beats.push(pos / audio.sample_rate as f64);
        counter += 1;
    }

    (offset_sec, beats)
}

// ---------------------------------------------------------------------------
// Key detection (Camelot notation) using Krumhansl-Kessler profiles
// ---------------------------------------------------------------------------

fn kk_major() -> [f64; 12] {
    [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88]
}

fn kk_minor() -> [f64; 12] {
    [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17]
}

fn camelot(pc: usize, minor: bool) -> &'static str {
    const CAMELOT_MAJ: [&str; 12] = [
        "8B", "3B", "10B", "5B", "12B", "7B", "2B", "9B", "4B", "11B", "6B", "1B",
    ];
    const CAMELOT_MIN: [&str; 12] = [
        "5A", "12A", "7A", "2A", "9A", "4A", "11A", "6A", "1A", "8A", "3A", "10A",
    ];
    if minor { CAMELOT_MIN[pc] } else { CAMELOT_MAJ[pc] }
}

fn note(pc: usize) -> &'static str {
    ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"][pc]
}

pub fn detect_key(audio: &AudioData) -> (String, String, f64) {
    let fft_size = 4096usize;
    let hop_size = 2048usize;
    let sr = audio.sample_rate as f64;

    let mut chroma = [0.0f64; 12];
    let mut frames = 0usize;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut pos = 0;
    while pos + fft_size <= audio.samples.len() {
        // Hann windowed frame
        let mut w: Vec<f64> = (0..fft_size)
            .map(|i| {
                let h = 0.5 * (1.0 - (2.0 * PI * i as f64 / (fft_size - 1) as f64).cos());
                audio.samples[pos + i] as f64 * h
            })
            .collect();

        let mut spec: Vec<rustfft::num_complex::Complex<f64>> =
            w.drain(..).map(|s| rustfft::num_complex::Complex::new(s, 0.0)).collect();

        fft.process(&mut spec);

        let half = fft_size / 2;
        let freq_res = sr / fft_size as f64;

        for bin in 0..half {
            let mag = spec[bin].norm_sqr();
            let freq = bin as f64 * freq_res;
            if freq < 30.0 || freq > 4000.0 {
                continue;
            }
            let midi = 12.0 * (freq / 440.0).log2() + 69.0;
            let pc = ((midi + 0.5).round() as i32).rem_euclid(12) as usize;
            chroma[pc] += mag;
        }
        frames += 1;
        pos += hop_size;
    }

    if frames == 0 {
        return ("8A".to_string(), "A minor".to_string(), 0.0);
    }

    // Normalize
    let mx = chroma.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if mx > 0.0 {
        for c in chroma.iter_mut() { *c /= mx; }
    }

    let major = kk_major();
    let minor = kk_minor();

    let mut best_corr = f64::NEG_INFINITY;
    let mut best_pc = 0usize;
    let mut best_minor = false;

    for shift in 0..12 {
        let cm: f64 = chroma.iter().zip(major.iter().cycle().skip(shift)).map(|(&c, &p)| c * p).sum();
        let cn: f64 = chroma.iter().zip(minor.iter().cycle().skip(shift)).map(|(&c, &p)| c * p).sum();
        if cm > best_corr { best_corr = cm; best_pc = shift; best_minor = false; }
        if cn > best_corr { best_corr = cn; best_pc = shift; best_minor = true; }
    }

    // Confidence = 1 - (2nd best / best)
    let mut all: Vec<f64> = (0..12)
        .flat_map(|s| {
            let cm: f64 = chroma.iter().zip(major.iter().cycle().skip(s)).map(|(&c, &p)| c * p).sum();
            let cn: f64 = chroma.iter().zip(minor.iter().cycle().skip(s)).map(|(&c, &p)| c * p).sum();
            [cm, cn]
        })
        .collect();
    all.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let conf = if all.len() > 1 && all[0] > 0.0 {
        (1.0 - all[1] / all[0]).max(0.0).min(1.0)
    } else {
        0.5
    };

    (camelot(best_pc, best_minor).to_string(), format!("{} {}", note(best_pc), if best_minor { "minor" } else { "major" }), conf)
}

// ---------------------------------------------------------------------------
// Section segmentation
// ---------------------------------------------------------------------------

pub fn segment_track(
    audio: &AudioData,
    beat_positions: &[f64],
    bpm: f64,
) -> Vec<SectionData> {
    if beat_positions.len() < 8 {
        return vec![SectionData { name: "Full Track".into(), start_bar: 1, bars: 16, color: "#555".into() }];
    }

    let bars = ((beat_positions.len() as f64 / 4.0).ceil() as u32).max(8);
    let win = 8u32;
    let nw = (bars + win - 1) / win;
    let duration = audio.duration_seconds;

    let mut energies = Vec::new();
    for w in 0..nw {
        let s_bar = w as f64 * win as f64;
        let e_bar = ((w + 1) * win).min(bars) as f64;
        let start_s = s_bar * 4.0 * 60.0 / bpm / duration * audio.samples.len() as f64;
        let end_s = (e_bar * 4.0 * 60.0 / bpm / duration * audio.samples.len() as f64).min(audio.samples.len() as f64);

        let si = start_s as usize;
        let ei = end_s as usize;
        if ei <= si || si >= audio.samples.len() {
            energies.push(0.0);
            continue;
        }
        let rms = (audio.samples[si..ei].iter().map(|&s| s * s).sum::<f32>() / (ei - si) as f32).sqrt();
        energies.push(rms);
    }

    if energies.is_empty() {
        return vec![SectionData { name: "Full Track".into(), start_bar: 1, bars, color: "#555".into() }];
    }

    let mx = energies.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    if mx > 0.0 { for e in energies.iter_mut() { *e /= mx; } }

    let tw = energies.len();
    let mut classes: Vec<&str> = Vec::new();

    for (i, &e) in energies.iter().enumerate() {
        let f = i as f64 / tw as f64;
        let c = if f < 0.15 && e < 0.3 { "Intro" }
            else if e > 0.7 { "Drop" }
            else if e > 0.5 {
                if i >= 2 && energies[i - 1] > energies[i - 2] && energies[i] >= energies[i - 1] { "Build" } else { "Drop" }
            } else if e < 0.3 && f > 0.25 && f < 0.85 { "Breakdown" }
            else if f > 0.85 && e < 0.3 { "Outro" }
            else if e < 0.3 { "Breakdown" }
            else { "Drop" };
        classes.push(c);
    }

    let mut merged: Vec<(String, u32, u32)> = Vec::new(); // name, start_win, num_win
    for &c in &classes {
        if let Some(last) = merged.last_mut() {
            if last.0 == c { last.2 += 1; continue; }
        }
        let start = merged.last().map(|m| m.1 + m.2).unwrap_or(0);
        merged.push((c.to_string(), start, 1));
    }

    let colors = ["#4A90D9", "#7B68EE", "#E06C75", "#98C379", "#D19A66", "#61AFEF", "#C678DD", "#56B6C2"];

    merged.iter().enumerate().map(|(idx, (name, start_win, num_win))| {
        SectionData {
            name: name.clone(),
            start_bar: start_win * win + 1,
            bars: num_win * win,
            color: colors[idx % colors.len()].to_string(),
        }
    }).collect()
}

// ---------------------------------------------------------------------------
// ArrangementDoc conversion
// ---------------------------------------------------------------------------

pub fn analysis_to_arrangement_doc(result: &AnalysisResult, filename: &str) -> serde_json::Value {
    let total_bars = result.sections.last().map(|s| s.start_bar + s.bars - 1).unwrap_or(16).max(16);

    let sections: Vec<serde_json::Value> = result.sections.iter().map(|s| {
        let bc = (s.bars as f64 / 8.0).ceil() as usize;
        serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "name": s.name,
            "color": s.color,
            "start_bar": s.start_bar,
            "bars": s.bars,
            "activity": {
                "Kick": vec![2; bc],
                "Clap": vec![0; bc],
                "Hat": vec![1; bc],
                "Snare": vec![0; bc],
                "Bass": vec![2; bc],
                "Synth": vec![1; bc],
                "Pad": vec![1; bc],
                "FX": vec![0; bc]
            },
            "notes": [],
            "references": []
        })
    }).collect();

    let bpm = result.bpm;
    serde_json::json!({
        "schema_version": "0.1.0",
        "id": uuid::Uuid::new_v4().to_string(),
        "type": "analysis",
        "title": format!("Analysis: {}", filename),
        "style": "techno",
        "reference_artists": [],
        "bpm": bpm.round() as i64,
        "bpm_range": [(bpm - 2.0).round() as i64, (bpm + 2.0).round() as i64],
        "swing_percent": 0,
        "ghost_note_density": 0,
        "rows": ["Kick", "Clap", "Hat", "Snare", "Bass", "Synth", "Pad", "FX"],
        "total_bars": total_bars,
        "sections": sections,
        "analysis_metadata": {
            "source_file": filename,
            "source_kind": "local",
            "duration_seconds": result.duration_seconds,
            "key_camelot": result.key_camelot,
            "key_standard": result.key_standard,
            "bpm_confidence": result.bpm_confidence,
            "downbeat_offset_seconds": result.downbeat_offset_seconds,
            "analysed_at": chrono::Utc::now().format("%+").to_string(),
            "analyser_version": "0.1.0"
        }
    })
}

// ---------------------------------------------------------------------------
// Main analysis function
// ---------------------------------------------------------------------------

pub fn analyze_file(path: &std::path::Path) -> Result<AnalysisResult, String> {
    let audio = load_audio(path)?;
    let (bpm, bpm_confidence) = detect_bpm(&audio);
    let (downbeat_offset_seconds, beat_positions) = detect_beats(&audio, bpm);
    let (key_camelot, key_standard, key_confidence) = detect_key(&audio);
    let sections = segment_track(&audio, &beat_positions, bpm);

    Ok(AnalysisResult {
        bpm,
        bpm_confidence,
        key_camelot,
        key_standard,
        key_confidence,
        downbeat_offset_seconds,
        beat_positions_seconds: beat_positions,
        duration_seconds: audio.duration_seconds,
        sections,
    })
}
