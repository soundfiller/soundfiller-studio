use rustfft::FftPlanner;
use std::collections::HashMap;
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
    pub confidence: f64,
}

#[derive(Clone)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration_seconds: f64,
    pub num_channels: u16,
}

// ---------------------------------------------------------------------------
// Feature extraction types
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct FeatureFrame {
    rms: f64,
    spectral_centroid: f64,
    spectral_flatness: f64,
    sub_kick: f64,   // 60-100 Hz energy
    sub_bass: f64,   // 30-150 Hz energy
    sub_mid: f64,    // 150 Hz - 2 kHz energy
    sub_high: f64,   // 4-12 kHz energy
    kick_present: f64, // 0.0 or 1.0
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
// Feature extraction (~1 Hz resolution)
// ---------------------------------------------------------------------------

fn extract_features(audio: &AudioData, _bpm: f64) -> Vec<FeatureFrame> {
    let sr = audio.sample_rate as usize;
    let fft_size = 4096usize;
    let window_samples = sr; // 1-second window
    let total_windows = (audio.samples.len() + window_samples - 1) / window_samples;
    let sr_f64 = audio.sample_rate as f64;
    let freq_res = sr_f64 / fft_size as f64;

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut features = Vec::with_capacity(total_windows);

    // For kick presence: sliding window of median kick-band energy
    let mut kick_energy_history: Vec<f64> = Vec::with_capacity(8);

    for win_idx in 0..total_windows {
        let start = win_idx * window_samples;
        let end = (start + window_samples).min(audio.samples.len());
        let win = &audio.samples[start..end];

        // ---- RMS ----
        let rms = (win.iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>() / (end - start) as f64).sqrt();

        // ---- FFT ----
        // Center a 4096-pt window within the 1-second window
        let fft_start = (win.len() / 2).saturating_sub(fft_size / 2);
        let fft_end = (fft_start + fft_size).min(win.len());
        let actual_fft_size = fft_end - fft_start;

        let mut w: Vec<f64> = Vec::with_capacity(fft_size);
        for i in 0..fft_size {
            if i < actual_fft_size {
                let h = 0.5 * (1.0 - (2.0 * PI * i as f64 / (actual_fft_size - 1).max(1) as f64).cos());
                w.push(win[fft_start + i] as f64 * h);
            } else {
                w.push(0.0); // zero-pad
            }
        }

        // Zero-pad if the FFT window is smaller than 4096
        while w.len() < fft_size {
            w.push(0.0);
        }
        w.truncate(fft_size);

        let mut spec: Vec<rustfft::num_complex::Complex<f64>> =
            w.drain(..).map(|s| rustfft::num_complex::Complex::new(s, 0.0)).collect();

        fft.process(&mut spec);

        let half = fft_size / 2;
        let mut magnitudes: Vec<f64> = Vec::with_capacity(half);
        for bin in 0..half {
            magnitudes.push(spec[bin].norm());
        }

        // ---- Spectral centroid ----
        let total_mag: f64 = magnitudes.iter().sum();
        let centroid = if total_mag > 0.0 {
            let weighted: f64 = magnitudes.iter().enumerate().map(|(bin, &m)| bin as f64 * freq_res * m).sum();
            weighted / total_mag
        } else {
            0.0
        };

        // ---- Spectral flatness ----
        // Per bin: geometric / arithmetic mean of magnitudes
        let flatness = if total_mag > 0.0 && magnitudes.len() > 0 {
            let n = magnitudes.len() as f64;
            let log_sum: f64 = magnitudes.iter().map(|&m| (m + 1e-10).ln()).sum();
            let geom = (log_sum / n).exp();
            let arith = total_mag / n;
            (geom / arith).min(1.0)
        } else {
            1.0
        };

        // ---- Sub-band energy ----
        let mut sum_60_100 = 0.0f64;
        let mut sum_30_150 = 0.0f64;
        let mut sum_150_2k = 0.0f64;
        let mut sum_4k_12k = 0.0f64;
        let mut total_band_energy = 0.0f64;

        for bin in 0..half {
            let freq = bin as f64 * freq_res;
            let mag = magnitudes[bin];
            if freq >= 60.0 && freq <= 100.0 {
                sum_60_100 += mag;
                sum_30_150 += mag;
            }
            if freq >= 30.0 && freq <= 150.0 {
                sum_30_150 += mag;
            }
            if freq >= 150.0 && freq <= 2000.0 {
                sum_150_2k += mag;
            }
            if freq >= 4000.0 && freq <= 12000.0 {
                sum_4k_12k += mag;
            }
            // 30-60 Hz is added to sum_30_150 already
            total_band_energy += mag;
        }

        let sub_bass = if total_band_energy > 0.0 { sum_30_150 / total_band_energy } else { 0.0 };
        let sub_kick = if total_band_energy > 0.0 { sum_60_100 / total_band_energy } else { 0.0 };
        let sub_mid = if total_band_energy > 0.0 { sum_150_2k / total_band_energy } else { 0.0 };
        let sub_high = if total_band_energy > 0.0 { sum_4k_12k / total_band_energy } else { 0.0 };

        // ---- Kick presence (onset on 60-100 Hz band) ----
        // Trailing median of last 4 kick-band energies
        kick_energy_history.push(sub_kick);
        if kick_energy_history.len() > 4 {
            kick_energy_history.remove(0);
        }

        let kick_present = if kick_energy_history.len() >= 4 {
            let mut sorted = kick_energy_history.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = sorted[1]; // median of 4 values: average of 2nd and 3rd
            if sub_kick > median * 2.0 { 1.0 } else { 0.0 }
        } else {
            0.0
        };

        features.push(FeatureFrame {
            rms,
            spectral_centroid: centroid,
            spectral_flatness: flatness,
            sub_kick,
            sub_bass,
            sub_mid,
            sub_high,
            kick_present,
        });
    }

    features
}

// ---------------------------------------------------------------------------
// Change point detection via sliding-window cost function
// ---------------------------------------------------------------------------

fn detect_change_points(features: &[FeatureFrame], bpm: f64) -> Vec<usize> {
    let n = features.len();
    if n < 2 {
        return vec![];
    }

    // Minimum segment length: 16 bars → seconds
    let min_seg_sec = 16.0 * 4.0 * 60.0 / bpm;

    // Window in feature frames (1 Hz = 1 frame per second)
    let window_frames = min_seg_sec as usize;

    if n < window_frames * 2 + 1 {
        return vec![];
    }

    // Flatten feature vectors for each frame
    let flat: Vec<Vec<f64>> = features
        .iter()
        .map(|f| {
            vec![
                f.rms,
                f.spectral_centroid / 12000.0, // normalize centroid by 12 kHz (nyquist-ish)
                f.spectral_flatness,
                f.sub_kick,
                f.sub_bass,
                f.sub_mid,
                f.sub_high,
                f.kick_present,
            ]
        })
        .collect();

    let n_feat = flat[0].len();

    // Compute cost function for each candidate frame
    let mut costs = vec![0.0f64; n];

    for t in window_frames..(n - window_frames) {
        // Left window: [t - window_frames, t)
        // Right window: [t, t + window_frames)
        let left: Vec<f64> = (0..n_feat)
            .map(|f| {
                let sum: f64 = flat[t - window_frames..t].iter().map(|v| v[f]).sum();
                sum / window_frames as f64
            })
            .collect();

        let right: Vec<f64> = (0..n_feat)
            .map(|f| {
                let sum: f64 = flat[t..t + window_frames].iter().map(|v| v[f]).sum();
                sum / window_frames as f64
            })
            .collect();

        let mut cost = 0.0;
        for f in 0..n_feat {
            let diff = left[f] - right[f];
            cost += diff * diff;
        }

        costs[t] = cost;
    }

    // Normalize costs to 0-1
    let min_cost = costs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_cost = costs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_cost - min_cost;
    if range > 0.0 {
        for c in costs.iter_mut() {
            *c = (*c - min_cost) / range;
        }
    }

    // Find peaks above mean + 0.5 * std
    let mean_cost = costs.iter().sum::<f64>() / n as f64;
    let variance = costs.iter().map(|c| (c - mean_cost).powi(2)).sum::<f64>() / n as f64;
    let std_cost = variance.sqrt();
    let threshold = mean_cost + 0.5 * std_cost;

    let mut boundaries = Vec::new();
    let mut i = window_frames;
    while i < n - window_frames {
        if costs[i] > threshold {
            // Local peak detection: check if this is a peak within a window around it
            let peak_radius = (window_frames / 2).max(1);
            let start_peak = i.saturating_sub(peak_radius);
            let end_peak = (i + peak_radius).min(n - window_frames);

            let is_peak = (start_peak..=end_peak)
                .filter(|&j| j != i)
                .all(|j| costs[i] >= costs[j]);

            if is_peak {
                boundaries.push(i);
                i += window_frames; // skip ahead to enforce minimum segment size
                continue;
            }
        }
        i += 1;
    }

    boundaries
}

// ---------------------------------------------------------------------------
// Section classification
// ---------------------------------------------------------------------------

fn classify_sections(
    features: &[FeatureFrame],
    boundaries: &[usize],
    bpm: f64,
    duration_seconds: f64,
) -> Vec<SectionData> {
    if features.is_empty() || duration_seconds <= 0.0 {
        return vec![];
    }

    // Build segment boundaries: start frames in seconds (feature frame indices)
    let mut seg_starts: Vec<usize> = Vec::new();
    seg_starts.push(0);
    seg_starts.extend_from_slice(boundaries);
    seg_starts.push(features.len()); // end of last segment

    // Convert feature frame indices to bar positions
    let seconds_per_bar = 4.0 * 60.0 / bpm;

    // Helper: aggregate features for a segment [start_frame, end_frame)
    let segment_features = |start: usize, end: usize| -> (f64, f64, f64, f64, f64) {
        let len = (end - start).max(1);
        let mut sum_kick = 0.0;
        let mut sum_sub_bass = 0.0;
        let mut sum_sub_mid = 0.0;
        let mut sum_sub_high = 0.0;
        let mut _kick_samples = 0;
        let mut rms_values = Vec::with_capacity(len);
        for i in start..end.min(features.len()) {
            let f = &features[i];
            sum_kick += f.kick_present;
            sum_sub_bass += f.sub_bass;
            sum_sub_mid += f.sub_mid;
            sum_sub_high += f.sub_high;
            if f.kick_present > 0.0 {
                _kick_samples += 1;
            }
            rms_values.push(f.rms);
        }
        let count = (end - start).min(features.len() - start) as f64;
        let mean_kick = if count > 0.0 { sum_kick / count } else { 0.0 };
        let mean_rms = rms_values.iter().sum::<f64>() / count.max(1.0);
        let mean_sub_bass = if count > 0.0 { sum_sub_bass / count } else { 0.0 };
        let mean_sub_mid = if count > 0.0 { sum_sub_mid / count } else { 0.0 };
        let _mean_sub_high = if count > 0.0 { sum_sub_high / count } else { 0.0 };

        // Energy trend (slope of RMS vs time index via linear regression)
        let energy_trend = if len >= 2 {
            let n = len.min(rms_values.len()) as f64;
            let t_vals: Vec<f64> = (0..rms_values.len()).map(|i| i as f64).collect();
            let sum_t: f64 = t_vals.iter().sum();
            let sum_e: f64 = rms_values.iter().sum();
            let sum_te: f64 = t_vals.iter().zip(rms_values.iter()).map(|(&t, &e)| t * e).sum();
            let sum_tt: f64 = t_vals.iter().map(|&t| t * t).sum();
            let denom = n * sum_tt - sum_t * sum_t;
            if denom.abs() > 1e-10 {
                (n * sum_te - sum_t * sum_e) / denom
            } else {
                0.0
            }
        } else {
            0.0
        };

        (mean_kick, mean_rms, energy_trend, mean_sub_bass, mean_sub_mid)
    };

    // Compute global max RMS for normalization
    let global_max_rms = features.iter().map(|f| f.rms).fold(f64::NEG_INFINITY, f64::max);
    let global_max_rms = if global_max_rms > 0.0 { global_max_rms } else { 1.0 };

    let mut sections = Vec::new();

    for seg_idx in 0..seg_starts.len().saturating_sub(1) {
        let start_frame = seg_starts[seg_idx];
        let end_frame = seg_starts[seg_idx + 1];
        let start_sec = start_frame as f64;
        let end_sec = end_frame as f64;
        let start_bar = (start_sec / seconds_per_bar).round() as u32 + 1;
        let end_bar = (end_sec / seconds_per_bar).round() as u32 + 1;
        let bar_count = (end_bar - start_bar).max(1);

        let position = (start_sec + (end_sec - start_sec) / 2.0) / duration_seconds;

        let (mean_kick, mean_rms_raw, energy_trend, _mean_sub_bass, _mean_sub_mid) =
            segment_features(start_frame, end_frame);
        let mean_rms = (mean_rms_raw / global_max_rms).min(1.0);

        let segment_bars = (bar_count as f64).round() as u32;

        // Classification rules (priority order)
        let (name, color, confidence) = classify_segment(
            mean_kick,
            mean_rms,
            energy_trend,
            position,
            segment_bars,
        );

        sections.push(SectionData {
            name,
            start_bar,
            bars: segment_bars,
            color,
            confidence,
        });
    }

    sections
}

fn classify_segment(
    mean_kick: f64,
    mean_rms: f64,
    energy_trend: f64,
    position: f64,
    bars: u32,
) -> (String, String, f64) {
    // Rule priority order from spec
    if mean_kick < 0.2 && mean_rms < 0.3 && position >= 0.25 && position <= 0.85 {
        // Breakdown
        let conf = confidence_from_margins_breakdown(mean_kick, mean_rms);
        ("Breakdown".into(), "#3366CC".into(), conf)
    } else if (mean_kick < 0.3 || mean_kick.is_nan() || false) && mean_rms < 0.3 && position < 0.25 {
        // Intro (corrected: kick_present < 0.3 AND mean_rms < 0.3 AND position < 0.25)
        let conf = confidence_from_margins_intro(mean_kick, mean_rms, position);
        ("Intro".into(), "#666666".into(), conf)
    } else if mean_kick > 0.5 && energy_trend > 0.05 && bars <= 32 {
        // Build
        let conf = confidence_from_margins_build(mean_kick, energy_trend);
        ("Build".into(), "#B8860B".into(), conf)
    } else if mean_kick > 0.5 && mean_rms > 0.7 {
        // Drop
        let conf = confidence_from_margins_drop(mean_kick, mean_rms);
        ("Drop".into(), "#CC3333".into(), conf)
    } else if mean_kick > 0.5 && mean_rms < 0.3 && position > 0.75 {
        // Outro
        let conf = 0.65 + 0.30 * (mean_kick.min(1.0) * (1.0 - mean_rms.min(1.0))).min(1.0);
        ("Outro".into(), "#666666".into(), conf)
    } else if mean_kick > 0.5 && mean_rms < 0.3 {
        // Early quiet with kick = Intro
        let conf = 0.55 + 0.40 * (mean_kick.min(1.0) * (1.0 - mean_rms.min(1.0))).min(1.0);
        ("Intro".into(), "#666666".into(), conf)
    } else if mean_kick > 0.5 {
        // Default for high-energy = Drop
        let conf = 0.55 + 0.40 * (mean_kick.min(1.0) * mean_rms.min(1.0)).min(1.0);
        ("Drop".into(), "#CC3333".into(), conf)
    } else {
        // Default: Breakdown
        let conf = 0.50 + 0.45 * (1.0 - mean_kick.min(1.0)) * (1.0 - mean_rms.min(1.0));
        ("Breakdown".into(), "#3366CC".into(), conf.min(0.95))
    }
}

/// Confidence: how far the segment's features are from the decision boundaries.
/// 0.5 = marginal, 0.95 = very clear.
fn confidence_from_margins_breakdown(mean_kick: f64, mean_rms: f64) -> f64 {
    let kick_margin = (0.2 - mean_kick).max(0.0);
    let rms_margin = (0.3 - mean_rms).max(0.0);
    let margin = (kick_margin + rms_margin) / 2.0;
    0.50 + 0.45 * margin.min(1.0)
}

fn confidence_from_margins_intro(_mean_kick: f64, mean_rms: f64, _position: f64) -> f64 {
    let rms_margin = (0.3 - mean_rms).max(0.0);
    0.55 + 0.40 * rms_margin.min(1.0)
}

fn confidence_from_margins_build(mean_kick: f64, energy_trend: f64) -> f64 {
    let kick_margin = (mean_kick - 0.5).max(0.0);
    let trend_margin = (energy_trend - 0.05).max(0.0) / 0.5;
    0.50 + 0.45 * ((kick_margin + trend_margin) / 2.0).min(1.0)
}

fn confidence_from_margins_drop(mean_kick: f64, mean_rms: f64) -> f64 {
    let kick_margin = (mean_kick - 0.5).max(0.0);
    let rms_margin = (mean_rms - 0.7).max(0.0);
    0.50 + 0.45 * ((kick_margin + rms_margin) / 2.0).min(1.0)
}

// ---------------------------------------------------------------------------
// Section segmentation (M3 pipeline)
// ---------------------------------------------------------------------------

pub fn segment_track(
    audio: &AudioData,
    _beat_positions: &[f64],
    bpm: f64,
) -> Vec<SectionData> {
    let features = extract_features(audio, bpm);
    if features.is_empty() {
        return vec![SectionData {
            name: "Full Track".into(),
            start_bar: 1,
            bars: ((audio.duration_seconds / (4.0 * 60.0 / bpm)).ceil() as u32).max(8),
            color: "#555".into(),
            confidence: 0.0,
        }];
    }

    let boundaries = detect_change_points(&features, bpm);
    let sections = classify_sections(&features, &boundaries, bpm, audio.duration_seconds);

    if sections.is_empty() {
        return vec![SectionData {
            name: "Full Track".into(),
            start_bar: 1,
            bars: ((audio.duration_seconds / (4.0 * 60.0 / bpm)).ceil() as u32).max(8),
            color: "#555".into(),
            confidence: 0.0,
        }];
    }

    sections
}

// ---------------------------------------------------------------------------
// Element activity inference (M3 §6.11)
// ---------------------------------------------------------------------------

pub fn infer_element_activity(
    audio: &AudioData,
    sections: &[SectionData],
    bpm: f64,
) -> HashMap<String, Vec<Vec<f64>>> {
    let seconds_per_bar = 4.0 * 60.0 / bpm;
    let sr = audio.sample_rate as usize;
    let fft_size = 4096usize;
    let sr_f64 = audio.sample_rate as f64;
    let freq_res = sr_f64 / fft_size as f64;

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Pre-compute features per-second for the whole track (same as extract_features but
    // we also need spectral_flatness per frame for some rows)
    let window_samples = sr;
    let total_windows = (audio.samples.len() + window_samples - 1) / window_samples;

    // Store per-second per-frame data needed for activity inference:
    // kick_present, sub_bass, sub_mid, sub_high, spectral_flatness
    let mut per_sec: Vec<[f64; 5]> = Vec::with_capacity(total_windows);

    for win_idx in 0..total_windows {
        let start = win_idx * window_samples;
        let end = (start + window_samples).min(audio.samples.len());
        let win = &audio.samples[start..end];

        // FFT
        let fft_start = (win.len() / 2).saturating_sub(fft_size / 2);
        let fft_end = (fft_start + fft_size).min(win.len());
        let actual_fft_size = fft_end - fft_start;

        let mut w: Vec<f64> = Vec::with_capacity(fft_size);
        for i in 0..fft_size {
            if i < actual_fft_size {
                let h = 0.5 * (1.0 - (2.0 * PI * i as f64 / (actual_fft_size - 1).max(1) as f64).cos());
                w.push(win[fft_start + i] as f64 * h);
            } else {
                w.push(0.0);
            }
        }
        while w.len() < fft_size { w.push(0.0); }
        w.truncate(fft_size);

        let mut spec: Vec<rustfft::num_complex::Complex<f64>> =
            w.drain(..).map(|s| rustfft::num_complex::Complex::new(s, 0.0)).collect();
        fft.process(&mut spec);

        let half = fft_size / 2;
        let mut magnitudes: Vec<f64> = Vec::with_capacity(half);
        for bin in 0..half {
            magnitudes.push(spec[bin].norm());
        }

        let total_mag: f64 = magnitudes.iter().sum();

        // Spectral flatness
        let flatness = if total_mag > 0.0 && !magnitudes.is_empty() {
            let n = magnitudes.len() as f64;
            let log_sum: f64 = magnitudes.iter().map(|&m| (m + 1e-10).ln()).sum();
            let geom = (log_sum / n).exp();
            let arith = total_mag / n;
            (geom / arith).min(1.0)
        } else {
            1.0
        };

        // Sub-band energy
        let mut sum_60_100 = 0.0;
        let mut sum_30_150 = 0.0;
        let mut sum_150_2k = 0.0;
        let mut sum_4k_12k = 0.0;
        let mut total_band = 0.0;

        for bin in 0..half {
            let freq = bin as f64 * freq_res;
            let mag = magnitudes[bin];
            if freq >= 60.0 && freq <= 100.0 {
                sum_60_100 += mag;
                sum_30_150 += mag;
            }
            if freq >= 30.0 && freq <= 150.0 {
                sum_30_150 += mag;
            }
            if freq >= 150.0 && freq <= 2000.0 {
                sum_150_2k += mag;
            }
            if freq >= 4000.0 && freq <= 12000.0 {
                sum_4k_12k += mag;
            }
            total_band += mag;
        }

        let sub_bass = if total_band > 0.0 { sum_30_150 / total_band } else { 0.0 };
        let sub_mid = if total_band > 0.0 { sum_150_2k / total_band } else { 0.0 };
        let sub_high = if total_band > 0.0 { sum_4k_12k / total_band } else { 0.0 };

        // Kick presence (simplified: energy-based)
        let kick_present = if total_band > 0.0 { sum_60_100 / total_band } else { 0.0 };

        per_sec.push([kick_present, sub_bass, sub_mid, sub_high, flatness]);
    }

    // For each section, split into 8-bar blocks and compute activity per row
    let rows = ["Kick", "Sub/Bass", "Pads/Strings", "Perc/Hats", "Mid/Lead", "FX/Risers"];
    let mut activity: HashMap<String, Vec<Vec<f64>>> = HashMap::new();
    for row in &rows {
        activity.insert(row.to_string(), Vec::new());
    }

    let mut all_block_values: HashMap<String, Vec<f64>> = HashMap::new();
    for row in &rows {
        all_block_values.insert(row.to_string(), Vec::new());
    }

    // First pass: collect raw block values for normalization
    // Second pass: normalize and bin

    // We need to map per-second features to 8-bar blocks within each section
    let mut section_block_values: Vec<HashMap<String, Vec<f64>>> = Vec::new();

    for section in sections {
        let _start_sec = (section.start_bar - 1) as f64 * seconds_per_bar;
        let _end_sec = (section.start_bar - 1 + section.bars) as f64 * seconds_per_bar;
        let num_block_8bar = (section.bars as f64 / 8.0).ceil() as usize;

        let mut block_values: HashMap<String, Vec<f64>> = HashMap::new();
        for row in &rows {
            block_values.insert(row.to_string(), vec![0.0; num_block_8bar]);
        }

        // Compute spectral_flatness trend per 8-bar block
        for block_idx in 0..num_block_8bar {
            let block_start_bar = section.start_bar as f64 + block_idx as f64 * 8.0;
            let block_end_bar = (block_start_bar + 8.0).min(section.start_bar as f64 + section.bars as f64);
            let block_start_sec = (block_start_bar - 1.0) * seconds_per_bar;
            let block_end_sec = (block_end_bar - 1.0) * seconds_per_bar;

            let start_frame = (block_start_sec as usize).min(per_sec.len().saturating_sub(1));
            let end_frame = (block_end_sec as usize).min(per_sec.len());

            if start_frame >= end_frame {
                continue;
            }

            let block_len = (end_frame - start_frame).max(1);

            // Mean values per row type
            let mut sum_kick = 0.0;
            let mut sum_sub_bass = 0.0;
            let mut sum_sub_mid = 0.0;
            let mut sum_sub_high = 0.0;
            let mut sum_flatness = 0.0;

            for fi in start_frame..end_frame {
                sum_kick += per_sec[fi][0];
                sum_sub_bass += per_sec[fi][1];
                sum_sub_mid += per_sec[fi][2];
                sum_sub_high += per_sec[fi][3];
                sum_flatness += per_sec[fi][4];
            }

            let mean_kick = sum_kick / block_len as f64;
            let mean_sub_bass = sum_sub_bass / block_len as f64;
            let mean_sub_mid = sum_sub_mid / block_len as f64;
            let mean_sub_high = sum_sub_high / block_len as f64;
            let mean_flatness = sum_flatness / block_len as f64;

            // Compute per-row raw values using PRD formula
            // Kick row: kick_presence binary
            let kick_val = mean_kick.min(1.0);

            // Sub/Bass row: sub_bass energy
            let sub_bass_val = mean_sub_bass.min(1.0);

            // Pads/Strings row: sub_mid × (1 - spectral_flatness)
            let pads_val = mean_sub_mid * (1.0 - mean_flatness);

            // Perc/Hats row: sub_high energy
            let perc_val = mean_sub_high.min(1.0);

            // Mid/Lead row: sub_mid × spectral_flatness
            let lead_val = mean_sub_mid * mean_flatness;

            // FX/Risers row: spectral_flatness trend (simplified: mean flatness per block)
            let fx_val = mean_flatness;

            let vals = vec![("Kick", kick_val), ("Sub/Bass", sub_bass_val), ("Pads/Strings", pads_val),
                           ("Perc/Hats", perc_val), ("Mid/Lead", lead_val), ("FX/Risers", fx_val)];

            for (row, val) in vals {
                if let Some(bv) = block_values.get_mut(row) {
                    bv[block_idx] = val;
                }
                if let Some(abv) = all_block_values.get_mut(row) {
                    abv.push(val);
                }
            }
        }

        section_block_values.push(block_values);
    }

    // Second pass: normalize all_block_values to 0-1 across whole track per row
    let mut row_min_max: HashMap<String, (f64, f64)> = HashMap::new();
    for (row, vals) in &all_block_values {
        let min_val = vals.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = if (max_val - min_val).abs() > 1e-10 {
            max_val - min_val
        } else {
            1.0
        };
        row_min_max.insert(row.clone(), (min_val, range));
    }

    // Third pass: bin to 0-3 using normalized values
    for (sec_idx, bv) in section_block_values.iter().enumerate() {
        for row in &rows {
            if let Some(block_vals) = bv.get(*row) {
                let (min_val, range) = row_min_max.get(*row).copied().unwrap_or((0.0, 1.0));
                let binned: Vec<f64> = block_vals
                    .iter()
                    .map(|&v| {
                        let normalized = if range > 0.0 {
                            ((v - min_val) / range).clamp(0.0, 1.0)
                        } else {
                            0.5
                        };
                        if normalized < 0.25 { 0.0 }
                        else if normalized < 0.5 { 1.0 }
                        else if normalized < 0.75 { 2.0 }
                        else { 3.0 }
                    })
                    .collect();

                if let Some(act) = activity.get_mut(*row) {
                    // Ensure we have room for this section
                    while act.len() <= sec_idx {
                        act.push(Vec::new());
                    }
                    act[sec_idx] = binned;
                }
            }
        }
    }

    activity
}

// ---------------------------------------------------------------------------
// ArrangementDoc conversion
// ---------------------------------------------------------------------------

pub fn analysis_to_arrangement_doc(result: &AnalysisResult, filename: &str) -> serde_json::Value {
    let total_bars = result.sections.last().map(|s| s.start_bar + s.bars - 1).unwrap_or(16).max(16);

    // Determine style based on BPM (>=128 = techno, <128 = prog house)
    let bpm = result.bpm;
    let is_techno = bpm >= 128.0;

    let row_names: Vec<&str> = if is_techno {
        vec!["Kick", "Sub/Bass", "Perc/Hats", "Mid/Lead", "Pads/Strings", "Vox", "FX/Risers"]
    } else {
        vec!["Kick", "Sub/Bass", "Pluck/Arp", "Mid/Lead", "Pads/Strings", "Vox", "Perc/Hats", "FX/Risers"]
    };

    // Use placeholders for now — analyzed_arrangement provides activity via MakeDoc helper
    let sections: Vec<serde_json::Value> = result.sections.iter().map(|s| {
        let bc = (s.bars as f64 / 8.0).ceil() as usize;
        let mut activity = serde_json::Map::new();
        for row in &row_names {
            activity.insert(
                row.to_string(),
                serde_json::json!(vec![2; bc]),
            );
        }
        serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "name": s.name,
            "color": s.color,
            "start_bar": s.start_bar,
            "bars": s.bars,
            "confidence": s.confidence,
            "activity": serde_json::Value::Object(activity),
            "notes": [],
            "references": []
        })
    }).collect();

    serde_json::json!({
        "schema_version": "0.1.0",
        "id": uuid::Uuid::new_v4().to_string(),
        "type": "analysis",
        "title": format!("Analysis: {}", filename),
        "style": if is_techno { "techno" } else { "prog_house" },
        "reference_artists": [],
        "bpm": bpm.round() as i64,
        "bpm_range": [(bpm - 2.0).round() as i64, (bpm + 2.0).round() as i64],
        "swing_percent": 0,
        "ghost_note_density": 0,
        "rows": row_names,
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
