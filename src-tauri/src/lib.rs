//! Tauri shell: commands only. All real work lives in `stackdeck_supervisor`.

use serde::Serialize;
use stackdeck_supervisor::{discover, process, provision, ProcSpec};
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

#[tauri::command]
fn setup_state() -> provision::State {
    provision::detect()
}

/// Homebrew installs take minutes, so this returns immediately and the UI
/// polls `setup_state` until the component appears.
#[tauri::command]
fn install_component(component: String) -> Result<(), String> {
    std::thread::spawn(move || {
        let _ = provision::install(&component);
    });
    Ok(())
}

#[tauri::command]
fn provision_component(component: String) -> Result<(), String> {
    match component.as_str() {
        "mariadb" => provision::provision_mariadb().map_err(|e| e.to_string()),
        "phpmyadmin" => provision::provision_phpmyadmin().map_err(|e| e.to_string()),
        other => Err(format!("unknown component: {other}")),
    }
}

#[tauri::command]
fn discover_services(state: tauri::State<State>) -> Vec<discover::Found> {
    let tracked: Vec<u32> = state
        .0
        .lock()
        .map(|r| r.iter().map(|x| x.pid).collect())
        .unwrap_or_default();
    discover::scan()
        .into_iter()
        .filter(|f| !tracked.contains(&f.pid))
        .collect()
}

/// Take ownership of a process started outside the app, so Stop and quit work.
#[tauri::command]
fn adopt_service(state: tauri::State<State>, pid: u32, name: String) -> Result<(), String> {
    if !process::alive(pid) {
        return Err(format!("pid {pid} is gone"));
    }
    state
        .0
        .lock()
        .map_err(|_| "state poisoned".to_string())?
        .push(Running { pid, name });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(State::default())
        .invoke_handler(tauri::generate_handler![
            start_service,
            stop_service,
            running_services,
            discover_services,
            adopt_service,
            setup_state,
            install_component,
            provision_component
        ])
        .run(tauri::generate_context!())
        .expect("error while running Stackdeck");
}
