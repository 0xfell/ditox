use crate::config::Config;
use crate::db::Database;
use crate::error::{DitoxError, Result};
use crate::sync::{
    bind_sync_tcp_listener, AdvertisedPeer, DiscoveryBackend, LocalIdentity, MdnsDiscovery,
    PeerTrustState, SyncDirection, SyncStatus,
};
use std::net::{TcpListener, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

const DISCOVERY_INTERVAL: Duration = Duration::from_secs(5);
const PULL_INTERVAL: Duration = Duration::from_secs(5);
const IDLE_SLEEP: Duration = Duration::from_millis(100);

pub struct SyncRuntimeHandle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl SyncRuntimeHandle {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for SyncRuntimeHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub fn start_if_enabled(config: &Config) -> Result<Option<SyncRuntimeHandle>> {
    if !config.sync.enabled {
        return Ok(None);
    }
    let config = config.clone();
    let identity = load_runtime_identity()?;
    let listener = bind_sync_tcp_listener(config.sync.port_range())?;
    listener.set_nonblocking(true)?;
    let local_addr = listener.local_addr()?;
    let advertise_port = local_addr.port();
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let join = thread::Builder::new()
        .name("ditox-sync-runtime".into())
        .spawn(move || run_runtime(config, identity, listener, advertise_port, thread_stop))
        .map_err(|e| DitoxError::Other(format!("failed to spawn sync runtime: {e}")))?;
    Ok(Some(SyncRuntimeHandle {
        stop,
        join: Some(join),
    }))
}

fn run_runtime(
    config: Config,
    identity: LocalIdentity,
    listener: TcpListener,
    port: u16,
    stop: Arc<AtomicBool>,
) {
    let discovery = match MdnsDiscovery::new() {
        Ok(discovery) => Some(discovery),
        Err(error) => {
            warn!(%error, "sync runtime continuing without mDNS discovery");
            None
        }
    };
    if let Some(discovery) = discovery.as_ref() {
        let peer = local_advertised_peer(&config, &identity, port);
        if let Err(error) = discovery.advertise(peer) {
            warn!(%error, "failed to advertise sync peer via mDNS");
        }
    }

    info!(port, "sync runtime started");
    let mut last_discovery = Instant::now() - DISCOVERY_INTERVAL;
    let mut last_pull = Instant::now() - PULL_INTERVAL;

    while !stop.load(Ordering::Relaxed) {
        accept_available(&listener, &identity, config.sync.digest_limit as usize);

        if let Some(discovery) = discovery.as_ref() {
            if last_discovery.elapsed() >= DISCOVERY_INTERVAL {
                if let Err(error) = ingest_discovery(discovery) {
                    warn!(%error, "sync discovery ingestion failed");
                }
                last_discovery = Instant::now();
            }
        }

        if last_pull.elapsed() >= PULL_INTERVAL {
            if let Err(error) = pull_from_auto_peers(&identity, config.sync.digest_limit as usize) {
                warn!(%error, "sync auto-pull failed");
            }
            last_pull = Instant::now();
        }

        thread::sleep(IDLE_SLEEP);
    }

    if let Some(discovery) = discovery.as_ref() {
        let _ = discovery.remove(&identity.fingerprint());
        let _ = discovery.shutdown();
    }
    info!("sync runtime stopped");
}

fn accept_available(listener: &TcpListener, identity: &LocalIdentity, digest_limit: usize) {
    loop {
        match listener.accept() {
            Ok((stream, addr)) => {
                let identity = identity.clone();
                let max_messages = digest_limit.saturating_add(1).max(1);
                if let Err(error) = thread::Builder::new()
                    .name("ditox-sync-inbound".into())
                    .spawn(move || {
                        if let Err(error) = serve_inbound(stream, identity, max_messages) {
                            warn!(%error, %addr, "inbound sync session failed");
                        }
                    })
                {
                    warn!(%error, %addr, "failed to spawn inbound sync session");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(error) => {
                warn!(%error, "sync listener accept failed");
                break;
            }
        }
    }
}

fn serve_inbound(
    mut stream: std::net::TcpStream,
    identity: LocalIdentity,
    max_messages: usize,
) -> Result<()> {
    let db = open_runtime_db()?;
    let peer = db.serve_trusted_sync_connection(&mut stream, &identity, max_messages)?;
    db.append_sync_log(
        &peer.id,
        SyncDirection::Receive,
        None,
        None,
        SyncStatus::Ok,
        Some("trusted inbound sync session"),
    )?;
    Ok(())
}

fn ingest_discovery(discovery: &MdnsDiscovery) -> Result<()> {
    let db = open_runtime_db()?;
    let peers = db.ingest_discovered_peers(discovery)?;
    if !peers.is_empty() {
        debug!(count = peers.len(), "ingested discovered sync peers");
    }
    Ok(())
}

fn pull_from_auto_peers(identity: &LocalIdentity, limit: usize) -> Result<()> {
    let db = open_runtime_db()?;
    let peers = db.list_peers()?;
    for peer in peers {
        if peer.trust_state != PeerTrustState::Pinned || !peer.auto_send {
            continue;
        }
        let Some(address) = peer.addresses.first() else {
            continue;
        };
        match db.pull_from_trusted_address(address.as_str(), identity, limit) {
            Ok(summary) => {
                db.append_sync_log(
                    &peer.id,
                    SyncDirection::Receive,
                    None,
                    None,
                    SyncStatus::Ok,
                    Some(&format!(
                        "auto-pull imported={}, skipped={}, requested={}, digests={}",
                        summary.imported_entries,
                        summary.skipped_entries,
                        summary.requested_entries,
                        summary.remote_digests
                    )),
                )?;
            }
            Err(error) => {
                let _ = db.append_sync_log(
                    &peer.id,
                    SyncDirection::Receive,
                    None,
                    None,
                    SyncStatus::Error,
                    Some(&error.to_string()),
                );
            }
        }
    }
    Ok(())
}

fn open_runtime_db() -> Result<Database> {
    let db = Database::open()?;
    db.init_schema()?;
    Ok(db)
}

fn load_runtime_identity() -> Result<LocalIdentity> {
    let config_path = Config::get_config_path()?;
    let config_dir = config_path
        .parent()
        .ok_or_else(|| DitoxError::Config("Could not determine config directory".into()))?;
    LocalIdentity::load_or_generate(config_dir)
}

fn local_advertised_peer(config: &Config, identity: &LocalIdentity, port: u16) -> AdvertisedPeer {
    let name = config
        .sync
        .name
        .clone()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_else(|| "ditox".to_string());
    AdvertisedPeer::new(
        name,
        identity.public_key(),
        format!("{}:{port}", local_lan_ip()),
    )
}

fn local_lan_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("8.8.8.8:80")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_advertisement_uses_configured_name_and_port() {
        let mut config = Config::default();
        config.sync.name = Some("desk".to_string());
        let identity = LocalIdentity::generate();

        let peer = local_advertised_peer(&config, &identity, 9009);

        assert_eq!(peer.name, "desk");
        assert!(peer.address.ends_with(":9009"));
        assert_eq!(peer.public_key, identity.public_key());
    }
}
