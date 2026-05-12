use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::analysis::{AnalysisResult, analyze_file};

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

pub struct AppState {
    pub analysis_state: AnalysisState,
}

pub struct AnalysisState {
    pub queue: Mutex<Vec<(String, PathBuf)>>,                      // (id, file_path)
    pub results: Mutex<HashMap<String, AnalysisResult>>,           // id -> result
    pub failures: Mutex<HashMap<String, String>>,                  // id -> error message
    pub current: Mutex<Option<String>>,                            // currently analysing id
    pub statuses: Mutex<HashMap<String, String>>,                  // id -> "queued" | "analysing" | "done" | "error: ..."
}

impl AnalysisState {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            results: Mutex::new(HashMap::new()),
            failures: Mutex::new(HashMap::new()),
            current: Mutex::new(None),
            statuses: Mutex::new(HashMap::new()),
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

    // We don't store filenames separately, so derive from stored analysis path
    // For now, we only return results, not filenames.
    let mut tracks: Vec<(String, String, AnalysisResult)> = Vec::new();
    for (id, result) in results.iter() {
        let status = statuses.get(id);
        let done = status.map(|s| s == "done").unwrap_or(false);
        if done {
            let filename = format!("track_{}", &id[..8]);
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
    Ok(())
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
