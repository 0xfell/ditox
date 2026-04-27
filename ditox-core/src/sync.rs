use crate::error::{DitoxError, Result};
use base64::Engine;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const PROTOCOL_VERSION: u32 = 1;
pub const SERVICE_TYPE: &str = "_ditox._tcp.local.";
pub const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_SHA256";
pub const DEFAULT_PORT: u16 = 9001;
pub const MAX_PORT: u16 = 9100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalIdentity {
    signing_key: SigningKey,
}

impl LocalIdentity {
    pub fn generate() -> Self {
        Self {
            signing_key: SigningKey::generate(&mut OsRng),
        }
    }

    pub fn public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn private_key(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    pub fn fingerprint(&self) -> String {
        public_key_fingerprint(&self.public_key())
    }

    pub fn public_key_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(self.public_key())
    }

    pub fn load_or_generate(config_dir: impl AsRef<Path>) -> Result<Self> {
        let config_dir = config_dir.as_ref();
        let key_path = config_dir.join("identity.key");
        let pub_path = config_dir.join("identity.pub");
        if key_path.exists() {
            return Self::load_from_file(&key_path);
        }

        std::fs::create_dir_all(config_dir)?;
        let identity = Self::generate();
        identity.save_to_files(&key_path, &pub_path)?;
        Ok(identity)
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw.trim())
            .map_err(|e| DitoxError::Config(format!("invalid sync identity key: {e}")))?;
        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| DitoxError::Config("sync identity key must be 32 bytes".into()))?;
        Ok(Self {
            signing_key: SigningKey::from_bytes(&key),
        })
    }

    pub fn save_to_files(
        &self,
        key_path: impl AsRef<Path>,
        pub_path: impl AsRef<Path>,
    ) -> Result<()> {
        let key_path = key_path.as_ref();
        let pub_path = pub_path.as_ref();
        if let Some(parent) = key_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        write_private_key(
            key_path,
            &base64::engine::general_purpose::STANDARD.encode(self.private_key()),
        )?;
        std::fs::write(pub_path, format!("{}\n", self.public_key_base64()))?;
        Ok(())
    }
}

pub fn public_key_fingerprint(public_key: &[u8; 32]) -> String {
    let digest = Sha256::digest(public_key);
    hex::encode(&digest[..12])
}

pub fn validate_public_key(public_key: &[u8]) -> Result<[u8; 32]> {
    let key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| DitoxError::Other("sync peer public key must be 32 bytes".into()))?;
    VerifyingKey::from_bytes(&key)
        .map_err(|e| DitoxError::Other(format!("invalid sync peer public key: {e}")))?;
    Ok(key)
}

pub fn identity_paths(config_dir: impl AsRef<Path>) -> (PathBuf, PathBuf) {
    let config_dir = config_dir.as_ref();
    (
        config_dir.join("identity.key"),
        config_dir.join("identity.pub"),
    )
}

#[cfg(unix)]
fn write_private_key(path: &Path, body: &str) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    writeln!(file, "{body}")?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private_key(path: &Path, body: &str) -> Result<()> {
    std::fs::write(path, format!("{body}\n"))?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerTrustState {
    Untrusted,
    Pinned,
    Rejected,
}

impl PeerTrustState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Untrusted => "untrusted",
            Self::Pinned => "pinned",
            Self::Rejected => "rejected",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "untrusted" => Ok(Self::Untrusted),
            "pinned" => Ok(Self::Pinned),
            "rejected" => Ok(Self::Rejected),
            other => Err(DitoxError::Other(format!(
                "unknown peer trust state: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub public_key: Vec<u8>,
    pub fingerprint: String,
    pub trust_state: PeerTrustState,
    pub auto_send: bool,
    pub last_seen: Option<String>,
    pub last_sync: Option<String>,
    pub addresses: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncDirection {
    Send,
    Receive,
}

impl SyncDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Send => "send",
            Self::Receive => "receive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Ok,
    Error,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncLogEntry {
    pub id: i64,
    pub peer_id: String,
    pub direction: String,
    pub entry_id: Option<String>,
    pub bytes: Option<i64>,
    pub status: String,
    pub message: Option<String>,
    pub occurred_at: String,
}

impl SyncStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Rejected => "rejected",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_first_twelve_sha256_bytes_hex() {
        let key = [7u8; 32];
        let digest = Sha256::digest(key);
        assert_eq!(public_key_fingerprint(&key), hex::encode(&digest[..12]));
    }

    #[test]
    fn identity_round_trips_through_disk() {
        let temp = tempfile::tempdir().unwrap();
        let original = LocalIdentity::load_or_generate(temp.path()).unwrap();
        let loaded = LocalIdentity::load_or_generate(temp.path()).unwrap();
        assert_eq!(original.private_key(), loaded.private_key());
        assert!(temp.path().join("identity.key").exists());
        assert!(temp.path().join("identity.pub").exists());
    }
}
