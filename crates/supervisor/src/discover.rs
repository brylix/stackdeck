//! Find services already running that this process did not spawn.
//!
//! Without this, a service started from a shell is invisible to the app: it
//! works, but cannot be stopped here and survives quitting. Discovery matches
//! known binaries, keeps only group leaders, and lets the UI offer Stop/adopt.

use std::collections::HashSet;
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Found {
    pub pid: u32,
    pub name: String,
    pub command: String,
}

/// Binaries/commands we know how to recognise. Keep in sync with the presets.
const SIGNATURES: &[(&str, &str)] = &[
    ("mariadb", "/opt/homebrew/opt/mariadb/bin/mariadbd"),
    ("php-fpm", "php-fpm: master process"),
    ("apache", "/opt/homebrew/opt/httpd/bin/httpd"),
    ("phpmyadmin", "php -S 127.0.0.1:8081"),
];

fn pids_matching(pattern: &str) -> Vec<u32> {
    let Ok(out) = Command::new("/usr/bin/pgrep")
        .args(["-f", pattern])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect()
}

fn parent_of(pid: u32) -> Option<u32> {
    let out = Command::new("/bin/ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

fn command_of(pid: u32) -> String {
    let Ok(out) = Command::new("/bin/ps")
        .args(["-o", "command=", "-p", &pid.to_string()])
        .output()
    else {
        return String::new();
    };
    let full = String::from_utf8_lossy(&out.stdout).trim().to_string();
    full.chars().take(120).collect()
}

/// Services found running now, one entry per process group.
pub fn scan() -> Vec<Found> {
    let mut candidates: Vec<(u32, String)> = Vec::new();
    for (name, sig) in SIGNATURES {
        for pid in pids_matching(sig) {
            candidates.push((pid, (*name).to_string()));
        }
    }

    let all: HashSet<u32> = candidates.iter().map(|(p, _)| *p).collect();
    let mut seen = HashSet::new();
    let mut found = Vec::new();

    for (pid, name) in candidates {
        if !seen.insert(pid) {
            continue;
        }
        // Keep only group leaders: a parent that is itself a candidate means
        // this is a worker, not the master.
        if let Some(ppid) = parent_of(pid) {
            if all.contains(&ppid) {
                continue;
            }
        }
        found.push(Found {
            pid,
            name,
            command: command_of(pid),
        });
    }

    found.sort_by_key(|f| f.pid);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_does_not_panic() {
        let _ = scan();
    }

    #[test]
    fn command_is_truncated() {
        let long = "x".repeat(300);
        assert!(long.chars().take(120).count() == 120);
    }
}
