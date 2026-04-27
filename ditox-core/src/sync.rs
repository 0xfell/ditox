use crate::error::{DitoxError, Result};
use base64::Engine;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const PROTOCOL_VERSION: u32 = 1;
pub const SERVICE_TYPE: &str = "_ditox._tcp.local.";
pub const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_SHA256";
pub const DEFAULT_PORT: u16 = 9001;
pub const MAX_PORT: u16 = 9100;
pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntryDigest {
    pub id: String,
    pub entry_hash: String,
    pub updated_at: String,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntryPayload {
    pub id: String,
    pub entry_hash: String,
    pub entry_type: String,
    pub byte_size: u64,
    pub created_at: String,
    pub last_used: String,
    pub pinned: bool,
    pub formats: Vec<FormatPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FormatPayload {
    pub format_name: String,
    pub format_hash: String,
    pub body: FormatBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatBody {
    Inline(Vec<u8>),
    BlobChunk(BlobChunk),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlobChunk {
    pub blob_hash: String,
    pub total_bytes: u64,
    pub offset: u64,
    pub data: Vec<u8>,
    pub last: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct SyncRoundSummary {
    pub remote_digests: usize,
    pub requested_entries: usize,
    pub imported_entries: usize,
    pub skipped_entries: usize,
}

pub fn encode_frame(payload: &[u8]) -> Result<Vec<u8>> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(DitoxError::Other(format!(
            "sync frame too large: {} bytes",
            payload.len()
        )));
    }
    let len = u32::try_from(payload.len())
        .map_err(|_| DitoxError::Other("sync frame length exceeds u32".into()))?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

pub fn decode_frame(frame: &[u8]) -> Result<&[u8]> {
    if frame.len() < 4 {
        return Err(DitoxError::Other(
            "sync frame is missing length prefix".into(),
        ));
    }
    let len = u32::from_be_bytes(
        frame[..4]
            .try_into()
            .expect("slice with exactly four bytes"),
    ) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(DitoxError::Other(format!(
            "sync frame too large: {len} bytes"
        )));
    }
    if frame.len() - 4 != len {
        return Err(DitoxError::Other(format!(
            "sync frame length mismatch: prefix={len}, actual={}",
            frame.len() - 4
        )));
    }
    Ok(&frame[4..])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdvertisedPeer {
    pub name: String,
    pub public_key: [u8; 32],
    pub fingerprint: String,
    pub address: String,
    pub protocol_version: u32,
}

impl AdvertisedPeer {
    pub fn new(name: impl Into<String>, public_key: [u8; 32], address: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fingerprint: public_key_fingerprint(&public_key),
            public_key,
            address: address.into(),
            protocol_version: PROTOCOL_VERSION,
        }
    }

    pub fn txt_records(&self) -> Vec<String> {
        vec![
            format!("version={}", self.protocol_version),
            format!(
                "key={}",
                base64::engine::general_purpose::STANDARD
                    .encode(public_key_fingerprint_bytes(&self.public_key))
            ),
            format!(
                "pub={}",
                base64::engine::general_purpose::STANDARD.encode(self.public_key)
            ),
            format!("name={}", self.name),
        ]
    }

    fn txt_property_values(&self) -> (String, String, String, String) {
        (
            self.protocol_version.to_string(),
            base64::engine::general_purpose::STANDARD
                .encode(public_key_fingerprint_bytes(&self.public_key)),
            base64::engine::general_purpose::STANDARD.encode(self.public_key),
            self.name.clone(),
        )
    }
}

pub trait DiscoveryBackend: Send + Sync {
    fn advertise(&self, peer: AdvertisedPeer) -> Result<()>;
    fn discover(&self) -> Result<Vec<AdvertisedPeer>>;
    fn remove(&self, fingerprint: &str) -> Result<()>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryDiscovery {
    peers: Arc<Mutex<HashMap<String, AdvertisedPeer>>>,
}

impl InMemoryDiscovery {
    pub fn shared() -> Self {
        Self::default()
    }
}

impl DiscoveryBackend for InMemoryDiscovery {
    fn advertise(&self, peer: AdvertisedPeer) -> Result<()> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| DitoxError::Other("sync discovery lock poisoned".into()))?;
        peers.insert(peer.fingerprint.clone(), peer);
        Ok(())
    }

    fn discover(&self) -> Result<Vec<AdvertisedPeer>> {
        let peers = self
            .peers
            .lock()
            .map_err(|_| DitoxError::Other("sync discovery lock poisoned".into()))?;
        let mut found: Vec<_> = peers.values().cloned().collect();
        found.sort_by(|a, b| a.fingerprint.cmp(&b.fingerprint));
        Ok(found)
    }

    fn remove(&self, fingerprint: &str) -> Result<()> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| DitoxError::Other("sync discovery lock poisoned".into()))?;
        peers.remove(fingerprint);
        Ok(())
    }
}

pub struct MdnsDiscovery {
    daemon: mdns_sd::ServiceDaemon,
    receiver: Mutex<mdns_sd::Receiver<mdns_sd::ServiceEvent>>,
    registered_fullnames: Mutex<HashMap<String, String>>,
}

impl MdnsDiscovery {
    pub fn new() -> Result<Self> {
        let daemon = mdns_sd::ServiceDaemon::new()
            .map_err(|e| DitoxError::Other(format!("failed to start mDNS daemon: {e}")))?;
        let receiver = daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| DitoxError::Other(format!("failed to browse ditox mDNS: {e}")))?;
        Ok(Self {
            daemon,
            receiver: Mutex::new(receiver),
            registered_fullnames: Mutex::new(HashMap::new()),
        })
    }

    fn peer_to_service_info(peer: &AdvertisedPeer) -> Result<mdns_sd::ServiceInfo> {
        let socket_addr =
            peer.address.to_socket_addrs()?.next().ok_or_else(|| {
                DitoxError::Other(format!("invalid sync address: {}", peer.address))
            })?;
        let instance = format!("ditox-{}", &peer.fingerprint[..8]);
        let host = format!("{instance}.local.");
        let ip = socket_addr.ip().to_string();
        let port = socket_addr.port();
        let (version, key, public_key, name) = peer.txt_property_values();
        let properties = [
            ("version", version.as_str()),
            ("key", key.as_str()),
            ("pub", public_key.as_str()),
            ("name", name.as_str()),
        ];
        mdns_sd::ServiceInfo::new(SERVICE_TYPE, &instance, &host, ip, port, &properties[..])
            .map_err(|e| DitoxError::Other(format!("failed to create ditox mDNS service: {e}")))
    }

    fn resolved_to_peer(resolved: &mdns_sd::ResolvedService) -> Option<AdvertisedPeer> {
        let version = resolved
            .get_property_val_str("version")?
            .parse::<u32>()
            .ok()?;
        if version != PROTOCOL_VERSION {
            return None;
        }
        let public_key = base64::engine::general_purpose::STANDARD
            .decode(resolved.get_property_val_str("pub")?)
            .ok()?;
        let public_key = validate_public_key(&public_key).ok()?;
        let fingerprint = public_key_fingerprint(&public_key);
        let advertised_fingerprint = resolved.get_property_val_str("key")?;
        let expected_key = base64::engine::general_purpose::STANDARD
            .encode(public_key_fingerprint_bytes(&public_key));
        if advertised_fingerprint != expected_key {
            return None;
        }
        let name = resolved
            .get_property_val_str("name")
            .unwrap_or_else(|| resolved.get_fullname())
            .to_string();
        let address = resolved
            .get_addresses_v4()
            .into_iter()
            .next()
            .map(|ip| format!("{}:{}", ip, resolved.get_port()))
            .unwrap_or_else(|| format!("{}:{}", resolved.get_hostname(), resolved.get_port()));
        Some(AdvertisedPeer {
            name,
            public_key,
            fingerprint,
            address,
            protocol_version: version,
        })
    }

    pub fn shutdown(&self) -> Result<()> {
        self.daemon
            .shutdown()
            .map(|_| ())
            .map_err(|e| DitoxError::Other(format!("failed to stop mDNS daemon: {e}")))
    }
}

impl DiscoveryBackend for MdnsDiscovery {
    fn advertise(&self, peer: AdvertisedPeer) -> Result<()> {
        let service = Self::peer_to_service_info(&peer)?;
        let fullname = service.get_fullname().to_string();
        self.daemon
            .register(service)
            .map_err(|e| DitoxError::Other(format!("failed to register ditox mDNS: {e}")))?;
        let mut registered = self
            .registered_fullnames
            .lock()
            .map_err(|_| DitoxError::Other("mDNS registry lock poisoned".into()))?;
        registered.insert(peer.fingerprint, fullname);
        Ok(())
    }

    fn discover(&self) -> Result<Vec<AdvertisedPeer>> {
        let receiver = self
            .receiver
            .lock()
            .map_err(|_| DitoxError::Other("mDNS receiver lock poisoned".into()))?;
        let mut peers = HashMap::new();
        while let Ok(event) = receiver.try_recv() {
            if let mdns_sd::ServiceEvent::ServiceResolved(resolved) = event {
                if let Some(peer) = Self::resolved_to_peer(&resolved) {
                    peers.insert(peer.fingerprint.clone(), peer);
                }
            }
        }
        let mut peers: Vec<_> = peers.into_values().collect();
        peers.sort_by(|a, b| a.fingerprint.cmp(&b.fingerprint));
        Ok(peers)
    }

    fn remove(&self, fingerprint: &str) -> Result<()> {
        let fullname = {
            let mut registered = self
                .registered_fullnames
                .lock()
                .map_err(|_| DitoxError::Other("mDNS registry lock poisoned".into()))?;
            registered.remove(fingerprint)
        };
        if let Some(fullname) = fullname {
            self.daemon
                .unregister(&fullname)
                .map_err(|e| DitoxError::Other(format!("failed to unregister ditox mDNS: {e}")))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct NoiseConfig {
    params: snow::params::NoiseParams,
}

impl NoiseConfig {
    pub fn new() -> Result<Self> {
        let params = NOISE_PATTERN
            .parse::<snow::params::NoiseParams>()
            .map_err(|e| DitoxError::Other(format!("invalid Noise pattern: {e}")))?;
        Ok(Self { params })
    }

    pub fn params(&self) -> &snow::params::NoiseParams {
        &self.params
    }

    pub fn builder(&self) -> snow::Builder<'_> {
        snow::Builder::new(self.params.clone())
    }
}

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
    hex::encode(public_key_fingerprint_bytes(public_key))
}

pub fn public_key_fingerprint_bytes(public_key: &[u8; 32]) -> [u8; 12] {
    let digest = Sha256::digest(public_key);
    digest[..12]
        .try_into()
        .expect("SHA-256 digest prefix length is fixed")
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
    fn advertised_peer_txt_records_match_mdns_contract() {
        let key = [3u8; 32];
        let peer = AdvertisedPeer::new("desk", key, "127.0.0.1:9001");
        assert_eq!(peer.fingerprint, public_key_fingerprint(&key));
        assert_eq!(
            peer.txt_records(),
            vec![
                "version=1".to_string(),
                format!(
                    "key={}",
                    base64::engine::general_purpose::STANDARD
                        .encode(public_key_fingerprint_bytes(&key))
                ),
                format!(
                    "pub={}",
                    base64::engine::general_purpose::STANDARD.encode(key)
                ),
                "name=desk".to_string(),
            ]
        );
    }

    #[test]
    fn noise_config_parses_expected_pattern() {
        let config = NoiseConfig::new().unwrap();
        let _builder = config.builder();
    }

    #[test]
    fn in_memory_discovery_advertises_updates_and_removes() {
        let discovery = InMemoryDiscovery::shared();
        let key = [1u8; 32];
        let first = AdvertisedPeer::new("one", key, "127.0.0.1:9001");
        let second = AdvertisedPeer::new("two", key, "127.0.0.1:9002");

        discovery.advertise(first.clone()).unwrap();
        assert_eq!(discovery.discover().unwrap(), vec![first]);

        discovery.advertise(second.clone()).unwrap();
        assert_eq!(discovery.discover().unwrap(), vec![second.clone()]);

        discovery.remove(&second.fingerprint).unwrap();
        assert!(discovery.discover().unwrap().is_empty());
    }

    #[test]
    fn frame_encoding_round_trips_payload() {
        let payload = b"protobuf bytes";
        let frame = encode_frame(payload).unwrap();
        assert_eq!(&frame[..4], &(payload.len() as u32).to_be_bytes());
        assert_eq!(decode_frame(&frame).unwrap(), payload);
    }

    #[test]
    fn frame_decoder_rejects_length_mismatch() {
        let mut frame = encode_frame(b"abc").unwrap();
        frame.push(b'd');
        assert!(decode_frame(&frame).is_err());
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
