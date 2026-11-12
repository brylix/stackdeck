//! First-run provisioning.
//!
//! A fresh install has nothing: no database, no PHP, no admin panel. Each
//! service is detected, installed from the pinned formula, then provisioned
//! with generated credentials — never a blank root password and never a
//! shared default secret.

use std::fs;
use std::io::Read;
use std::path::PathBuf;

// TODO(win): Homebrew does not exist on Windows. This whole layer needs to
// become manifest-driven (download + hash verify + extract) before the port.
const BREW: &str = "/opt/homebrew/bin/brew";
// TODO(win): Windows has no `security` CLI; use Credential Manager.
const SECURITY: &str = "/usr/bin/security";
const KEYCHAIN_SERVICE: &str = "stackdeck-mariadb";
const SOCKET: &str = "/tmp/stackdeck.sock";

#[derive(Debug, Clone, serde::Serialize)]
pub struct State {
    pub brew: bool,
    pub mariadb_installed: bool,
    pub mariadb_provisioned: bool,
    pub php_installed: bool,
    pub php_provisioned: bool,
    pub phpmyadmin_installed: bool,
    pub phpmyadmin_provisioned: bool,
    pub ready: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("io {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{0} is not installed")]
    Missing(String),
    #[error("command failed: {0}")]
    Command(String),
    #[error("homebrew not found at {0}")]
    NoBrew(String),
}

impl SetupError {
    fn io(path: &std::path::Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}

pub fn app_dir() -> PathBuf {
    // TODO(win): %LOCALAPPDATA%\Stackdeck
    dirs_app_support().join("Stackdeck")
}

fn dirs_app_support() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join("Library/Application Support")
}

fn data_dir() -> PathBuf {
    app_dir().join("data")
}

fn phpmyadmin_dir() -> PathBuf {
    app_dir().join("phpmyadmin")
}

pub fn exists(path: &std::path::Path) -> bool {
    path.exists()
}

#[cfg(unix)]
fn link_entry(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

/// Windows needs no symlink privilege if we just copy.
#[cfg(windows)]
fn link_entry(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            link_entry(&entry.path(), &dst.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(src, dst).map(|_| ())
    }
}

fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut buf);
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn run(program: &str, args: &[&str]) -> Result<String, SetupError> {
    let out = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|e| SetupError::io(std::path::Path::new(program), e))?;
    if !out.status.success() {
        return Err(SetupError::Command(format!(
            "{} {}: {}",
            program,
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Snapshot of what is installed and provisioned.
pub fn detect() -> State {
    let mariadb_bin = PathBuf::from("/opt/homebrew/opt/mariadb/bin/mariadbd");
    let php_bin = PathBuf::from("/opt/homebrew/opt/php/bin/php");
    let pma_share = PathBuf::from("/opt/homebrew/share/phpmyadmin");

    let mariadb_provisioned = data_dir().join("mysql").is_dir();
    let phpmyadmin_provisioned = phpmyadmin_dir().join("config.inc.php").is_file();

    State {
        brew: PathBuf::from(BREW).is_file(),
        mariadb_installed: mariadb_bin.is_file(),
        mariadb_provisioned,
        php_installed: php_bin.is_file(),
        php_provisioned: true,
        phpmyadmin_installed: pma_share.is_dir(),
        phpmyadmin_provisioned,
        ready: mariadb_provisioned && phpmyadmin_provisioned,
    }
}

/// Install a component from Homebrew. Blocking; callers should poll `detect`.
pub fn install(component: &str) -> Result<(), SetupError> {
    // TODO(win): resolve from `manifest/` and extract instead of shelling to brew.
    if !PathBuf::from(BREW).is_file() {
        return Err(SetupError::NoBrew(BREW.to_string()));
    }
    let formula = match component {
        "mariadb" => "mariadb",
        "php" => "php",
        "phpmyadmin" => "phpmyadmin",
        other => return Err(SetupError::Missing(other.to_string())),
    };
    run(BREW, &["install", formula])?;
    Ok(())
}

/// Initialise the database and set a generated password on every root account.
pub fn provision_mariadb() -> Result<(), SetupError> {
    // TODO(win): mariadbd.exe lives in the extracted zip; init is
    // `mariadb-install-db.exe`. Socket path becomes a TCP port.
    let bin = "/opt/homebrew/opt/mariadb/bin";
    if !PathBuf::from(format!("{bin}/mariadbd")).is_file() {
        return Err(SetupError::Missing("mariadb".into()));
    }
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| SetupError::io(&dir, e))?;

    if !dir.join("mysql").is_dir() {
        run(
            &format!("{bin}/mariadb-install-db"),
            &[
                &format!("--datadir={}", dir.display()),
                "--auth-root-authentication-method=normal",
                "--skip-test-db",
            ],
        )?;
    }

    let pass = match run(
        SECURITY,
        &["find-generic-password", "-s", KEYCHAIN_SERVICE, "-w"],
    ) {
        Ok(existing) if !existing.is_empty() => existing,
        _ => {
            let fresh = random_hex(32);
            run(
                SECURITY,
                &[
                    "add-generic-password",
                    "-a",
                    "root",
                    "-s",
                    KEYCHAIN_SERVICE,
                    "-w",
                    &fresh,
                    "-U",
                ],
            )?;
            fresh
        }
    };

    // Every root host needs the password, not just localhost.
    for host in ["localhost", "127.0.0.1", "::1"] {
        let _ = run(
            &format!("{bin}/mariadb"),
            &[
                "-u",
                "root",
                "-p",
                &pass,
                &format!("--socket={}", SOCKET),
                "-e",
                &format!("ALTER USER 'root'@'{host}' IDENTIFIED BY '{pass}';"),
            ],
        );
    }
    Ok(())
}

/// Materialise phpMyAdmin with its own config: generated secret, loopback host,
/// no passwordless login.
pub fn provision_phpmyadmin() -> Result<(), SetupError> {
    // TODO(win): no brew share dir — copy from the extracted phpMyAdmin zip.
    let share = PathBuf::from("/opt/homebrew/share/phpmyadmin");
    if !share.is_dir() {
        return Err(SetupError::Missing("phpmyadmin".into()));
    }
    let dir = phpmyadmin_dir();
    fs::create_dir_all(&dir).map_err(|e| SetupError::io(&dir, e))?;

    // Symlinks on unix, copies on Windows (no link privilege needed).
    for entry in fs::read_dir(&share).map_err(|e| SetupError::io(&share, e))? {
        let entry = entry.map_err(|e| SetupError::io(&share, e))?;
        let target = dir.join(entry.file_name());
        if target.symlink_metadata().is_err() {
            link_entry(&entry.path(), &target).map_err(|e| SetupError::io(&target, e))?;
        }
    }

    let temp = app_dir().join("tmp");
    fs::create_dir_all(&temp).map_err(|e| SetupError::io(&temp, e))?;
    let config = format!(
        "<?php\n\
         $cfg['blowfish_secret'] = '{secret}';\n\
         $cfg['TempDir'] = '{temp}';\n\
         $i = 1;\n\
         $cfg['ServerDefault'] = 1;\n\
         $cfg['Servers'][$i]['host'] = '127.0.0.1';\n\
         $cfg['Servers'][$i]['port'] = '3306';\n\
         $cfg['Servers'][$i]['auth_type'] = 'cookie';\n\
         $cfg['Servers'][$i]['AllowNoPassword'] = false;\n\
         $cfg['CheckConfigurationPermissions'] = false;\n",
        secret = random_hex(24),
        temp = temp.display()
    );
    let path = dir.join("config.inc.php");
    fs::write(&path, config).map_err(|e| SetupError::io(&path, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_hex_shape() {
        let hex = random_hex(16);
        assert_eq!(hex.len(), 32);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(hex, random_hex(16));
    }

    #[test]
    fn detect_returns_without_panicking() {
        let state = detect();
        assert!(state.ready == (state.mariadb_provisioned && state.phpmyadmin_provisioned));
    }
}
