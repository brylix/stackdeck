//! Child-process supervision with correct tree teardown on both platforms.
//!
//! Servers are started with their own process group (Unix) or inside a job
//! object (Windows). That means one `stop` takes down the whole tree — the
//! actual failure mode of every hand-rolled launcher is an orphaned `mysqld`
//! still holding the data directory and its port.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ProcSpec {
    pub name: String,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

impl ProcSpec {
    pub fn new(name: &str, program: impl Into<PathBuf>) -> Self {
        Self {
            name: name.to_string(),
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }
}

#[derive(Debug)]
pub struct Spawned {
    pub pid: u32,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ProcError {
    #[error("start {name}: {source}")]
    Spawn {
        name: String,
        #[source]
        source: std::io::Error,
    },
    #[error("not running: pid {0}")]
    NotRunning(u32),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

fn exited_map() -> &'static Mutex<HashMap<u32, bool>> {
    static M: OnceLock<Mutex<HashMap<u32, bool>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

fn mark(pid: u32, exited: bool) {
    if let Ok(mut m) = exited_map().lock() {
        m.insert(pid, exited);
    }
}

/// Start a supervised process, streaming its output to `log_dir`.
pub fn spawn(spec: &ProcSpec, log_dir: &Path) -> Result<Spawned, ProcError> {
    fs::create_dir_all(log_dir)?;
    let stdout = log_dir.join(format!("{}.out.log", spec.name));
    let stderr = log_dir.join(format!("{}.err.log", spec.name));

    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .stdin(Stdio::null())
        .stdout(OpenOptions::new().create(true).append(true).open(&stdout)?)
        .stderr(OpenOptions::new().create(true).append(true).open(&stderr)?);
    if let Some(dir) = &spec.cwd {
        cmd.current_dir(dir);
    }
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }

    platform::prepare(&mut cmd);
    let mut child = cmd.spawn().map_err(|source| ProcError::Spawn {
        name: spec.name.clone(),
        source,
    })?;
    let pid = child.id();
    platform::register(&child, pid);
    mark(pid, false);

    // Reaper: avoids zombies and makes `alive` authoritative.
    thread::spawn(move || {
        let _ = child.wait();
        mark(pid, true);
    });

    Ok(Spawned {
        pid,
        stdout,
        stderr,
    })
}

pub fn alive(pid: u32) -> bool {
    let exited = exited_map()
        .lock()
        .ok()
        .and_then(|m| m.get(&pid).copied())
        .unwrap_or(false);
    if exited {
        return false;
    }
    platform::probe(pid)
}

/// Terminate the whole process tree, escalating after a grace period.
pub fn stop(pid: u32) -> Result<(), ProcError> {
    if !alive(pid) {
        return Err(ProcError::NotRunning(pid));
    }
    platform::signal_tree(pid);

    for _ in 0..40 {
        if !alive(pid) {
            mark(pid, true);
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    platform::kill_tree(pid);
    mark(pid, true);
    Ok(())
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::os::unix::process::CommandExt;

    pub fn prepare(cmd: &mut Command) {
        // New process group: pgid == pid, so we can signal the tree.
        cmd.process_group(0);
    }

    pub fn register(_child: &std::process::Child, _pid: u32) {}

    pub fn probe(pid: u32) -> bool {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    pub fn signal_tree(pid: u32) {
        unsafe {
            libc::kill(-(pid as i32), libc::SIGTERM);
        }
    }

    pub fn kill_tree(pid: u32) {
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };

    fn jobs() -> &'static Mutex<HashMap<u32, usize>> {
        static M: OnceLock<Mutex<HashMap<u32, usize>>> = OnceLock::new();
        M.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn prepare(_cmd: &mut Command) {}

    /// Put the child in a job object so it dies with us, even on a crash.
    pub fn register(child: &std::process::Child, pid: u32) {
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            AssignProcessToJobObject(job, child.as_raw_handle() as _);
            if let Ok(mut m) = jobs().lock() {
                m.insert(pid, job as usize);
            }
        }
    }

    pub fn probe(pid: u32) -> bool {
        unsafe {
            let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if h.is_null() {
                return false;
            }
            let state = WaitForSingleObject(h, 0);
            CloseHandle(h);
            state == WAIT_TIMEOUT
        }
    }

    pub fn signal_tree(pid: u32) {
        kill_tree(pid);
    }

    pub fn kill_tree(pid: u32) {
        if let Ok(mut m) = jobs().lock() {
            if let Some(job) = m.remove(&pid) {
                unsafe {
                    TerminateJobObject(job as _, 1);
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    #[test]
    fn starts_and_stops_a_tree() {
        let dir = std::env::temp_dir().join("stackdeck-proc-test");
        let spec = ProcSpec::new("sleeper", "/bin/sh")
            .arg("-c")
            .arg("sleep 30");
        let s = spawn(&spec, &dir).unwrap();
        assert!(alive(s.pid));
        stop(s.pid).unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(!alive(s.pid));
    }
}
