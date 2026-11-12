//! Manifest of pinned, hash-verified component downloads.
//!
//! Nothing is installed from a mutable "latest" URL: every entry names an
//! exact version and digest, so a compromised mirror cannot swap a binary.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub component: String,
    pub os: String,
    pub arch: String,
    pub version: String,
    pub url: String,
    /// Lowercase hex SHA-256 of the artifact.
    pub sha256: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("read manifest: {0}")]
    Read(#[from] std::io::Error),
    #[error("parse manifest: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unsupported platform: {0}/{1}")]
    Platform(String, String),
    #[error("digest mismatch for {file}: expected {expected}, got {actual}")]
    Digest {
        file: String,
        expected: String,
        actual: String,
    },
}

pub fn load(path: &Path) -> Result<Vec<Entry>, ManifestError> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

/// Resolve the single entry matching this build target.
pub fn resolve<'a>(entries: &'a [Entry], component: &str) -> Result<&'a Entry, ManifestError> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    entries
        .iter()
        .find(|e| e.component == component && e.os == os && e.arch == arch)
        .ok_or_else(|| ManifestError::Platform(os.into(), arch.into()))
}

pub fn sha256_file(path: &Path) -> Result<String, ManifestError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Verify a downloaded artifact before it is ever executed.
pub fn verify_file(path: &Path, expected_hex: &str) -> Result<(), ManifestError> {
    let actual = sha256_file(path)?;
    if actual.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(ManifestError::Digest {
            file: path.display().to_string(),
            expected: expected_hex.to_string(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_roundtrip() {
        let dir = std::env::temp_dir().join("stackdeck-manifest-test");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("blob");
        fs::write(&file, b"stackdeck").unwrap();
        // sha256("stackdeck")
        let hex = sha256_file(&file).unwrap();
        assert!(verify_file(&file, &hex).is_ok());
        assert!(verify_file(&file, "0000").is_err());
        fs::remove_file(&file).ok();
    }

    #[test]
    fn resolves_current_platform() {
        let entries = vec![Entry {
            component: "php".into(),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            version: "0".into(),
            url: "https://example.invalid/php".into(),
            sha256: "00".into(),
        }];
        assert!(resolve(&entries, "php").is_ok());
        assert!(resolve(&entries, "caddy").is_err());
    }
}
