//! Tauri shell: commands only. All real work lives in `stackdeck_supervisor`.

use serde::Serialize;
use stackdeck_supervisor::{process, ProcSpec};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Clone)]
struct Running {
    pid: u32,
    name: String,
}

#[derive(Default)]
struct State(Mutex<Vec<Running>>);

fn env_dir() -> PathBuf {
    // Caller-side app data dir placeholder until per-OS resolution lands.
    std::env::temp_dir().join("stackdeck-logs")
}

#[tauri::command]
fn start_service(
    state: tauri::State<State>,
    name: String,
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
) -> Result<u32, String> {
    let mut spec = ProcSpec::new(&name, program);
    spec.args = args;
    spec.cwd = cwd.map(PathBuf::from);

    let spawned = process::spawn(&spec, &env_dir()).map_err(|e| e.to_string())?;
    state
        .0
        .lock()
        .map_err(|_| "state poisoned".to_string())?
        .push(Running {
            pid: spawned.pid,
            name: name.clone(),
        });
    Ok(spawned.pid)
}

#[tauri::command]
fn stop_service(state: tauri::State<State>, pid: u32) -> Result<(), String> {
    process::stop(pid).map_err(|e| e.to_string())?;
    if let Ok(mut running) = state.0.lock() {
        running.retain(|r| r.pid != pid);
    }
    Ok(())
}

#[tauri::command]
fn running_services(state: tauri::State<State>) -> Vec<Running> {
    let mut running = state.0.lock().map(|r| r.clone()).unwrap_or_default();
    running.retain(|r| process::alive(r.pid));
    running
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(State::default())
        .invoke_handler(tauri::generate_handler![
            start_service,
            stop_service,
            running_services
        ])
        .run(tauri::generate_context!())
        .expect("error while running Stackdeck");
}
