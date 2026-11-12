//! Stackdeck supervisor: the privileged half of the app.
//!
//! Kept free of Tauri so it can be unit-tested and, if ever needed, driven
//! headlessly.

pub mod health;
pub mod manifest;
pub mod process;

pub use manifest::{Entry, ManifestError};
pub use process::{ProcError, ProcSpec, Spawned};

use std::path::Path;
use std::thread;
use std::time::Duration;

/// Restart a process with exponential backoff.
///
/// A component that crashes on boot should not spin the CPU, and one that
/// keeps dying should give up so the caller can surface it.
pub fn restart_with_backoff(
    spec: &ProcSpec,
    log_dir: &Path,
    attempts: u32,
) -> Result<Spawned, ProcError> {
    let mut last: Option<ProcError> = None;
    for attempt in 0..attempts.max(1) {
        match process::spawn(spec, log_dir) {
            Ok(spawned) => return Ok(spawned),
            Err(err) => {
                last = Some(err);
                thread::sleep(Duration::from_millis(250 * 2u64.pow(attempt.min(5))));
            }
        }
    }
    Err(last.expect("at least one attempt"))
}
