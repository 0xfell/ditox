use anyhow::Result;
use clap::Parser;
use directories::BaseDirs;
use ditox_core::clipboard::Clipboard; // bring clipboard trait into scope
use ditox_core::Store; // bring trait into scope
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use time::OffsetDateTime;

#[derive(Parser, Debug)]
#[command(name = "clipd", version, about = "Ditox clipboard daemon")]
struct Cli {
    /// Optional database path (overrides settings)
    #[arg(long)]
    db: Option<PathBuf>,
    /// Listening port on 127.0.0.1 (0 = auto)
    #[arg(long, default_value_t = 0)]
    port: u16,
    /// Disable clipboard watcher (polling)
    #[arg(long, default_value_t = false)]
    no_watch: bool,
    /// Clipboard poll interval in milliseconds
    #[arg(long, default_value_t = 200)]
    poll_ms: u64,
    /// Exit automatically after N milliseconds (for CI/testing)
    #[arg(long)]
    exit_after_ms: Option<u64>,
    /// Serve a one-shot Health response, then exit (binds to --port)
    #[arg(long, default_value_t = false)]
    health_once: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DaemonInfo {
    port: u16,
    started_at: i64,
    pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
enum Request {
    Health,
    List {
        images: bool,
        favorites: bool,
        limit: Option<usize>,
        offset: Option<usize>,
        query: Option<String>,
        tag: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Item {
    Text {
        id: String,
        favorite: bool,
        created_at: i64,
        last_used_at: Option<i64>,
        text: String,
    },
    Image {
        id: String,
        favorite: bool,
        created_at: i64,
        last_used_at: Option<i64>,
        width: u32,
        height: u32,
        format: String,
        path: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Response<T> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Page<T> {
    items: Vec<T>,
    more: bool,
    total: Option<usize>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let settings = ditox_cli_compat::load_settings();
    let db_path = cli
        .db
        .clone()
        .or_else(|| match &settings.storage {
            ditox_cli_compat::Storage::LocalSqlite { db_path } => db_path.clone(),
            _ => None,
        })
        .unwrap_or_else(default_db_path);

    std::fs::create_dir_all(db_path.parent().unwrap())?;
    let store = ditox_core::StoreImpl::new_with(&db_path, true)?;
    store.init()?;

    let listener = TcpListener::bind(("127.0.0.1", cli.port))?;
    let port = listener.local_addr()?.port();
    write_daemon_info(port)?;

    let store = Arc::new(store);
    if cli.health_once {
        if let Ok((mut stream, _addr)) = listener.accept() {
            let resp: Response<serde_json::Value> = Response {
                ok: true,
                data: Some(serde_json::json!({
                    "version": env!("CARGO_PKG_VERSION"),
                    "now": OffsetDateTime::now_utc().unix_timestamp(),
                })),
                error: None,
            };
            let s = serde_json::to_string(&resp)?;
            writeln!(stream, "{}", s)?;
        }
        return Ok(());
    }
    if let Some(ms) = cli.exit_after_ms {
        let _guard = thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            std::process::exit(0);
        });
    }
    let watcher_store = store.clone();
    let no_watch = cli.no_watch;
    if !no_watch {
        thread::spawn(move || clipboard_watch_loop(watcher_store, cli.poll_ms));
    }

    eprintln!("clipd listening on 127.0.0.1:{}", port);
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let st = store.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_client(st, s) {
                        eprintln!("client error: {e}");
                    }
                });
            }
            Err(e) => {
                eprintln!("accept error: {e}");
            }
        }
    }
    Ok(())
}

fn handle_client(store: Arc<ditox_core::StoreImpl>, stream: TcpStream) -> Result<()> {
    let peer = stream.peer_addr()?;
    let mut writer = stream.try_clone()?;
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let resp = match serde_json::from_str::<Request>(&line) {
            Ok(Request::Health) => Response {
                ok: true,
                data: Some(serde_json::json!({
                    "version": env!("CARGO_PKG_VERSION"),
                    "now": OffsetDateTime::now_utc().unix_timestamp(),
                })),
                error: None,
            },
            Ok(Request::List {
                images,
                favorites,
                limit,
                offset,
                query,
                tag,
            }) => {
                let off = offset.unwrap_or(0);
                if images {
                    match list_images(&store, favorites, limit, offset, tag.as_deref()) {
                        Ok(items) => {
                            let total = store
                                .count_images(ditox_core::Query {
                                    contains: None,
                                    favorites_only: favorites,
                                    limit: None,
                                    tag: tag.as_deref().map(|s| s.to_string()),
                                    rank: false,
                                })
                                .ok();
                            let more = limit.map(|l| items.len() > off + l).unwrap_or(false);
                            let slice = if let Some(l) = limit {
                                &items[off..items.len().min(off + l)]
                            } else {
                                &items[off..]
                            };
                            let page = Page {
                                items: slice.to_vec(),
                                more,
                                total,
                            };
                            Response {
                                ok: true,
                                data: Some(serde_json::to_value(page).unwrap()),
                                error: None,
                            }
                        }
                        Err(e) => Response::<serde_json::Value> {
                            ok: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    }
                } else {
                    match list_text(
                        &store,
                        favorites,
                        limit,
                        offset,
                        query.as_deref(),
                        tag.as_deref(),
                    ) {
                        Ok(items) => {
                            let (count_q, _) =
                                build_text_query(favorites, query.as_deref(), tag.as_deref());
                            let total = store.count(count_q).ok();
                            let more = limit.map(|l| items.len() > off + l).unwrap_or(false);
                            let slice = if let Some(l) = limit {
                                &items[off..items.len().min(off + l)]
                            } else {
                                &items[off..]
                            };
                            let page = Page {
                                items: slice.to_vec(),
                                more,
                                total,
                            };
                            Response {
                                ok: true,
                                data: Some(serde_json::to_value(page).unwrap()),
                                error: None,
                            }
                        }
                        Err(e) => Response::<serde_json::Value> {
                            ok: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    }
                }
            }
            Err(e) => Response::<serde_json::Value> {
                ok: false,
                data: None,
                error: Some(format!("bad request: {e}")),
            },
        };
        let s = serde_json::to_string(&resp)?;
        writeln!(writer, "{}", s)?;
        writer.flush()?;
    }
    eprintln!("client {} disconnected", peer);
    Ok(())
}

fn list_text(
    store: &ditox_core::StoreImpl,
    favorites: bool,
    limit: Option<usize>,
    offset: Option<usize>,
    query: Option<&str>,
    tag: Option<&str>,
) -> Result<Vec<Item>> {
    let (mut base_q, _fav_resolved) = build_text_query(favorites, query, tag);
    let off = offset.unwrap_or(0);
    base_q.limit = limit.map(|l| off + l + 1);
    let q = base_q;
    let items = store.list(q)?;
    let mut out = Vec::with_capacity(items.len());
    for c in items {
        out.push(Item::Text {
            id: c.id,
            favorite: c.is_favorite,
            created_at: c.created_at.unix_timestamp_nanos() as i64,
            last_used_at: c.last_used_at.map(|t| t.unix_timestamp_nanos() as i64),
            text: c.text,
        });
    }
    Ok(out)
}

#[allow(dead_code)]
fn contains_from(q: Option<&str>) -> Option<String> {
    q.and_then(|qs| {
        let mut rest: Vec<&str> = Vec::new();
        for tok in qs.split_whitespace() {
            if tok.starts_with("tag:")
                || tok.eq_ignore_ascii_case("is:fav")
                || tok.eq_ignore_ascii_case("is:favorite")
            {
                continue;
            }
            rest.push(tok);
        }
        if rest.is_empty() {
            None
        } else {
            Some(rest.join(" "))
        }
    })
}

fn build_text_query(
    favorites: bool,
    query: Option<&str>,
    tag: Option<&str>,
) -> (ditox_core::Query, bool) {
    // Parse simple operators in query: tag:foo, is:fav
    let mut fav = favorites;
    let mut tag_opt = tag.map(|s| s.to_string());
    let mut contains: Option<String> = None;
    if let Some(qs) = query {
        let mut rest: Vec<&str> = Vec::new();
        for tok in qs.split_whitespace() {
            if let Some(val) = tok.strip_prefix("tag:") {
                if !val.is_empty() {
                    tag_opt = Some(val.to_string());
                }
            } else if tok.eq_ignore_ascii_case("is:fav") || tok.eq_ignore_ascii_case("is:favorite")
            {
                fav = true;
            } else {
                rest.push(tok);
            }
        }
        if !rest.is_empty() {
            contains = Some(rest.join(" "));
        }
    }
    (
        ditox_core::Query {
            contains,
            favorites_only: fav,
            limit: None,
            tag: tag_opt,
            rank: false,
        },
        fav,
    )
}

fn list_images(
    store: &ditox_core::StoreImpl,
    favorites: bool,
    limit: Option<usize>,
    offset: Option<usize>,
    tag: Option<&str>,
) -> Result<Vec<Item>> {
    let off = offset.unwrap_or(0);
    let fetch_limit = limit.map(|l| off + l + 1);
    let q = ditox_core::Query {
        contains: None,
        favorites_only: favorites,
        limit: fetch_limit,
        tag: tag.map(|s| s.to_string()),
        rank: false,
    };
    let items = store.list_images(q)?;
    let mut out = Vec::with_capacity(items.len());
    for (c, m) in items {
        out.push(Item::Image {
            id: c.id,
            favorite: c.is_favorite,
            created_at: c.created_at.unix_timestamp_nanos() as i64,
            last_used_at: c.last_used_at.map(|t| t.unix_timestamp_nanos() as i64),
            width: m.width,
            height: m.height,
            format: m.format,
            path: c.image_path,
        });
    }
    Ok(out)
}

fn write_daemon_info(port: u16) -> Result<()> {
    let info = DaemonInfo {
        port,
        started_at: OffsetDateTime::now_utc().unix_timestamp(),
        pid: std::process::id(),
    };
    let path = clipd_info_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, serde_json::to_vec_pretty(&info)?)?;
    Ok(())
}

fn clipd_info_path() -> PathBuf {
    config_dir().join("clipd.json")
}

fn config_dir() -> PathBuf {
    if let Some(bd) = BaseDirs::new() {
        bd.config_dir().join("ditox")
    } else {
        PathBuf::from("./.config/ditox")
    }
}

fn default_db_path() -> PathBuf {
    let cfg = config_dir();
    let p = cfg.join("db").join("ditox.db");
    let _ = std::fs::create_dir_all(p.parent().unwrap());
    p
}

fn clipboard_watch_loop(store: Arc<ditox_core::StoreImpl>, poll_ms: u64) {
    #[cfg(target_os = "linux")]
    let cb = ditox_core::clipboard::ArboardClipboard::new();
    #[cfg(not(target_os = "linux"))]
    let cb = ditox_core::clipboard::NoopClipboard;
    let last = Arc::new(Mutex::new((None::<String>, 0usize)));
    loop {
        if let Ok(Some(mut text)) = cb.get_text() {
            // Normalize simple trailing newlines from some apps
            if text.ends_with('\n') {
                text.pop();
            }
            let mut guard = last.lock().unwrap();
            let changed =
                guard.0.as_ref().map(|s| s != &text).unwrap_or(true) || guard.1 != text.len();
            if changed {
                // Deduplicate by exact text; update last_used when found, else insert
                match store.find_id_by_exact_text(&text) {
                    Ok(Some(id)) => {
                        let _ = store.touch_last_used(&id);
                    }
                    Ok(None) => {
                        let _ = store.add(&text);
                    }
                    Err(_) => {
                        let _ = store.add(&text);
                    }
                }
                *guard = (Some(text), guard.1);
                guard.1 = guard.0.as_ref().map(|s| s.len()).unwrap_or(0);

                // Burst drain: quickly sample a few times to catch rapid changes
                drop(guard);
                for _ in 0..8 {
                    std::thread::sleep(std::time::Duration::from_millis(30));
                    if let Ok(Some(mut txt2)) = cb.get_text() {
                        if txt2.ends_with('\n') {
                            txt2.pop();
                        }
                        let mut g = last.lock().unwrap();
                        let diff =
                            g.0.as_ref().map(|s| s != &txt2).unwrap_or(true) || g.1 != txt2.len();
                        if diff {
                            match store.find_id_by_exact_text(&txt2) {
                                Ok(Some(id)) => {
                                    let _ = store.touch_last_used(&id);
                                }
                                Ok(None) => {
                                    let _ = store.add(&txt2);
                                }
                                Err(_) => {
                                    let _ = store.add(&txt2);
                                }
                            }
                            *g = (Some(txt2), g.1);
                            g.1 = g.0.as_ref().map(|s| s.len()).unwrap_or(0);
                        }
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(poll_ms));
    }
}

// Minimal shim to reuse cli config loader without creating a hard dependency cycle
mod ditox_cli_compat {
    use serde::Deserialize;
    use std::path::PathBuf;
    #[derive(Debug, Clone, Deserialize)]
    #[allow(dead_code)]
    pub struct Settings {
        pub storage: Storage,
    }
    #[derive(Debug, Clone, Deserialize)]
    #[allow(dead_code)]
    #[serde(tag = "backend", rename_all = "lowercase")]
    pub enum Storage {
        LocalSqlite {
            db_path: Option<PathBuf>,
        },
        Turso {
            url: String,
            auth_token: Option<String>,
        },
    }
    pub fn config_dir() -> std::path::PathBuf {
        if let Some(bd) = directories::BaseDirs::new() {
            bd.config_dir().join("ditox")
        } else {
            std::path::PathBuf::from("./.config/ditox")
        }
    }
    pub fn load_settings() -> Settings {
        let path = config_dir().join("settings.toml");
        if let Ok(s) = std::fs::read_to_string(&path) {
            toml::from_str(&s).unwrap_or(Settings {
                storage: Storage::LocalSqlite { db_path: None },
            })
        } else {
            Settings {
                storage: Storage::LocalSqlite { db_path: None },
            }
        }
    }
}
