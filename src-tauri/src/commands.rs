use std::collections::HashMap;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::analysis::{AnalysisResult, analyze_file};

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

pub struct AppState {
    pub analysis_state: AnalysisState,
}

#[derive(Clone, serde::Serialize)]
pub struct YoutubeMeta {
    pub title: String,
    pub uploader: String,
    pub source_url: String,
}

pub struct AnalysisState {
    pub queue: Mutex<Vec<(String, PathBuf)>>,                      // (id, file_path)
    pub results: Mutex<HashMap<String, AnalysisResult>>,           // id -> result
    pub failures: Mutex<HashMap<String, String>>,                  // id -> error message
    pub current: Mutex<Option<String>>,                            // currently analysing id
    pub statuses: Mutex<HashMap<String, String>>,                  // id -> "queued" | "analysing" | "downloading" | "error: ..."
    pub youtube_meta: Mutex<HashMap<String, YoutubeMeta>>,         // id -> youtube metadata
}

impl AnalysisState {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            results: Mutex::new(HashMap::new()),
            failures: Mutex::new(HashMap::new()),
            current: Mutex::new(None),
            statuses: Mutex::new(HashMap::new()),
            youtube_meta: Mutex::new(HashMap::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// Background worker
// ---------------------------------------------------------------------------

pub fn start_background_worker(app: AppHandle) {
    std::thread::spawn(move || loop {
        // Small sleep to avoid busy-looping
        std::thread::sleep(std::time::Duration::from_millis(500));

        let state: State<AppState> = app.state::<AppState>();
        let analysis = &state.analysis_state;

        // Check if we're already processing something
        {
            let current = analysis.current.lock().unwrap();
            if current.is_some() {
                continue;
            }
        }

        // Dequeue next item
        let next: Option<(String, PathBuf)> = {
            let mut queue = analysis.queue.lock().unwrap();
            if queue.is_empty() {
                None
            } else {
                Some(queue.remove(0))
            }
        };

        if let Some((id, path)) = next {
            // Mark as analysing
            {
                let mut current = analysis.current.lock().unwrap();
                *current = Some(id.clone());
                let mut statuses = analysis.statuses.lock().unwrap();
                statuses.insert(id.clone(), "analysing".to_string());
            }
            let _ = app.emit("analysis-status-update", serde_json::json!({"id": &id, "status": "analysing"}));

            // Run analysis
            match analyze_file(&path) {
                Ok(result) => {
                    let mut results = analysis.results.lock().unwrap();
                    results.insert(id.clone(), result.clone());
                    let mut statuses = analysis.statuses.lock().unwrap();
                    statuses.insert(id.clone(), "done".to_string());
                    let _ = app.emit("analysis-complete", serde_json::json!({"id": &id, "result": &result}));
                }
                Err(err) => {
                    let mut failures = analysis.failures.lock().unwrap();
                    failures.insert(id.clone(), err.clone());
                    let mut statuses = analysis.statuses.lock().unwrap();
                    statuses.insert(id.clone(), format!("error: {}", err));
                    let _ = app.emit("analysis-error", serde_json::json!({"id": &id, "error": &err}));
                }
            }

            // Clear current
            {
                let mut current = analysis.current.lock().unwrap();
                *current = None;
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn ingest_audio_file(
    path: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<String, String> {
    let src = std::path::Path::new(&path);
    if !src.exists() {
        return Err(format!("File not found: {}", path));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let extension = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3");

    // Get app data directory
    let app_dir = get_audio_dir(&app);
    std::fs::create_dir_all(&app_dir).map_err(|e| format!("Cannot create data dir: {}", e))?;

    let dest = app_dir.join(format!("{}.{}", id, extension));
    std::fs::copy(src, &dest).map_err(|e| format!("Cannot copy file: {}", e))?;

    // Add to queue
    {
        let mut queue = state.analysis_state.queue.lock().unwrap();
        queue.push((id.clone(), dest));
        let mut statuses = state.analysis_state.statuses.lock().unwrap();
        statuses.insert(id.clone(), "queued".to_string());
    }

    let _ = app.emit("analysis-status-update", serde_json::json!({"id": &id, "status": "queued"}));

    Ok(id)
}

#[tauri::command]
pub fn get_analysis_status(
    id: String,
    state: State<AppState>,
) -> Result<String, String> {
    let statuses = state.analysis_state.statuses.lock().unwrap();
    Ok(statuses.get(&id).cloned().unwrap_or_else(|| "unknown".to_string()))
}

#[tauri::command]
pub fn get_analysis_result(
    id: String,
    state: State<AppState>,
) -> Result<AnalysisResult, String> {
    // Check results
    {
        let results = state.analysis_state.results.lock().unwrap();
        if let Some(result) = results.get(&id) {
            return Ok(result.clone());
        }
    }
    // Check failures
    {
        let failures = state.analysis_state.failures.lock().unwrap();
        if let Some(err) = failures.get(&id) {
            return Err(format!("Analysis failed: {}", err));
        }
    }
    // Check status
    let status = {
        let statuses = state.analysis_state.statuses.lock().unwrap();
        statuses.get(&id).cloned()
    };
    match status {
        Some(s) if s == "queued" => Err("Analysis is queued".to_string()),
        Some(s) if s == "analysing" => Err("Analysis in progress".to_string()),
        Some(s) => Err(format!("Status: {}", s)),
        None => Err("Unknown ID".to_string()),
    }
}

#[tauri::command]
pub fn list_analysed_tracks(
    state: State<AppState>,
) -> Result<Vec<(String, String, AnalysisResult)>, String> {
    // Returns (id, filename (from path), result)
    let results = state.analysis_state.results.lock().unwrap();
    let statuses = state.analysis_state.statuses.lock().unwrap();

    let yt_meta = state.analysis_state.youtube_meta.lock().unwrap();
    let mut tracks: Vec<(String, String, AnalysisResult)> = Vec::new();
    for (id, result) in results.iter() {
        let status = statuses.get(id);
        let done = status.map(|s| s == "done").unwrap_or(false);
        if done {
            let filename = if let Some(meta) = yt_meta.get(id) {
                format!("{} — {}", meta.uploader, meta.title)
            } else {
                format!("track_{}", &id[..8])
            };
            tracks.push((id.clone(), filename, result.clone()));
        }
    }
    Ok(tracks)
}

#[tauri::command]
pub fn delete_analysis(
    id: String,
    state: State<AppState>,
) -> Result<(), String> {
    state.analysis_state.results.lock().unwrap().remove(&id);
    state.analysis_state.failures.lock().unwrap().remove(&id);
    state.analysis_state.statuses.lock().unwrap().remove(&id);
    state.analysis_state.youtube_meta.lock().unwrap().remove(&id);
    Ok(())
}

// ---------------------------------------------------------------------------
// YouTube ingestion
// ---------------------------------------------------------------------------

fn is_valid_youtube_url(url: &str) -> bool {
    url.contains("youtube.com/watch")
        || url.contains("youtu.be/")
        || url.contains("youtube.com/shorts/")
        || url.contains("m.youtube.com/")
}

fn parse_download_progress(line: &str) -> Option<f64> {
    // Parse yt-dlp output: "[download]  12.3% of 7.42MiB at ..."
    if !line.contains("[download]") {
        return None;
    }
    if let Some(pct_start) = line.find('%') {
        let before_pct = &line[..pct_start];
        if let Some(num_start) = before_pct.rfind(|c: char| !c.is_ascii_digit() && c != '.') {
            let num_str = &before_pct[num_start + 1..];
            if let Ok(pct) = num_str.trim().parse::<f64>() {
                return Some(pct);
            }
        }
    }
    None
}

#[tauri::command]
pub fn ingest_youtube(
    url: String,
    ack_token: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<String, String> {
    // Validate acknowledgement
    if ack_token != "accepted_v1" {
        return Err("Legal acknowledgement required. Please accept the terms first.".to_string());
    }

    // Validate URL
    if !is_valid_youtube_url(&url) {
        return Err("Invalid YouTube URL. Supported: youtube.com/watch, youtu.be, youtube.com/shorts".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let app_dir = get_audio_dir(&app);
    std::fs::create_dir_all(&app_dir).map_err(|e| format!("Cannot create data dir: {}", e))?;

    // Set initial status
    {
        let mut statuses = state.analysis_state.statuses.lock().unwrap();
        statuses.insert(id.clone(), "downloading".to_string());
    }
    let _ = app.emit(
        "analysis-status-update",
        serde_json::json!({"id": &id, "status": "downloading"}),
    );

    // Spawn download thread
    let id_clone = id.clone();
    let url_clone = url.clone();
    let app_clone = app.clone();
    std::thread::spawn(move || {
        fn set_error(app: &AppHandle, id: &str, err_msg: &str) {
            let state = app.state::<AppState>();
            let mut statuses = state.analysis_state.statuses.lock().unwrap();
            statuses.insert(id.to_string(), format!("error: {}", err_msg));
            let _ = app.emit(
                "analysis-error",
                serde_json::json!({"id": id, "error": err_msg}),
            );
        }

        let output_path = app_dir.join(format!("{}.%(ext)s", id_clone));
        let output_template = output_path.to_string_lossy().to_string();

        // Step 1: Fetch metadata (title + uploader)
        let meta = match Command::new("yt-dlp")
            .args([
                "--print", "%(title)s",
                "--print", "%(uploader)s",
                "--skip-download",
                &url_clone,
            ])
            .output()
        {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = stdout.lines().collect();
                let title = lines.first().map(|s| s.to_string()).unwrap_or_else(|| "Unknown Track".to_string());
                let uploader = lines.get(1).map(|s| s.to_string()).unwrap_or_else(|| "Unknown Artist".to_string());
                YoutubeMeta {
                    title,
                    uploader,
                    source_url: url_clone.clone(),
                }
            }
            Err(e) => {
                set_error(&app_clone, &id_clone, &format!("yt-dlp not found: {}. Is yt-dlp installed?", e));
                return;
            }
        };

        // Store metadata
        {
            let state = app_clone.state::<AppState>();
            let mut yt_meta = state.analysis_state.youtube_meta.lock().unwrap();
            yt_meta.insert(id_clone.clone(), meta.clone());
        }

        // Step 2: Download + extract audio
        let mut child = match Command::new("yt-dlp")
            .args([
                "-x",
                "--audio-format", "wav",
                "--audio-quality", "0",
                "-o", &output_template,
                "--newline",
                &url_clone,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                set_error(&app_clone, &id_clone, &format!("Failed to start yt-dlp: {}", e));
                return;
            }
        };

        // Read stderr line by line for progress
        let stderr = child.stderr.take();
        if let Some(stderr) = stderr {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(l) = line {
                    if let Some(pct) = parse_download_progress(&l) {
                        let _ = app_clone.emit(
                            "youtube-download-progress",
                            serde_json::json!({"id": &id_clone, "progress": pct}),
                        );
                    }
                }
            }
        }

        // Wait for child to finish
        match child.wait() {
            Ok(status) if status.success() => {},
            Ok(status) => {
                set_error(&app_clone, &id_clone, &format!("yt-dlp exited with code {:?}", status.code()));
                return;
            }
            Err(e) => {
                set_error(&app_clone, &id_clone, &format!("yt-dlp process error: {}", e));
                return;
            }
        }

        // Find the output WAV file
        let wav_path = app_dir.join(format!("{}.wav", id_clone));
        if !wav_path.exists() {
            set_error(&app_clone, &id_clone, "Download completed but WAV file not found");
            return;
        }

        // Queue for analysis
        {
            let state = app_clone.state::<AppState>();
            let mut queue = state.analysis_state.queue.lock().unwrap();
            queue.push((id_clone.clone(), wav_path));
            let mut statuses = state.analysis_state.statuses.lock().unwrap();
            statuses.insert(id_clone.clone(), "queued".to_string());
        }
        let _ = app_clone.emit(
            "analysis-status-update",
            serde_json::json!({"id": &id_clone, "status": "queued"}),
        );
    });

    Ok(id)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_audio_dir(app: &AppHandle) -> PathBuf {
    let base = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join("audio")
}
