use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
#[cfg(not(target_os = "linux"))]
use ditox_core::clipboard::NoopClipboard as SystemClipboard;
#[cfg(target_os = "linux")]
use ditox_core::clipboard::{ArboardClipboard as SystemClipboard, Clipboard as _};
use ditox_core::{ClipKind, Query, Store, StoreImpl};
use image::ImageEncoder;
use std::path::PathBuf;
// config module is declared at the top; avoid duplicate re-declaration here

#[derive(Parser)]
#[command(name = "ditox", version, about = "Ditox clipboard CLI (scaffold)")]
struct Cli {
    /// Store backend
    #[arg(long, value_enum, default_value_t = StoreKind::Sqlite)]
    store: StoreKind,
    /// Path to SQLite database file (when --store sqlite)
    #[arg(long)]
    db: Option<PathBuf>,
    /// Automatically apply pending migrations on startup
    #[arg(long, default_value_t = true)]
    auto_migrate: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize local database (placeholder)
    InitDb,
    /// Add a new entry. If --image-* is used, stores an image.
    Add {
        /// Plain text; if omitted, read from STDIN (conflicts with image flags)
        #[arg(conflicts_with_all=["image_path","image_from_clipboard"])]
        text: Option<String>,
        /// Read image from file path
        #[arg(long, conflicts_with = "text")]
        image_path: Option<PathBuf>,
        /// Read image from system clipboard
        #[arg(long, conflicts_with = "text")]
        image_from_clipboard: bool,
    },
    /// Sync commands
    Sync {
        #[command(subcommand)]
        cmd: SyncCmd,
    },
    /// List recent entries
    List {
        #[arg(long)]
        favorites: bool,
        /// List images instead of text entries
        #[arg(long)]
        images: bool,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        json: bool,
    },
    /// Search entries by substring
    Search {
        query: String,
        #[arg(long)]
        favorites: bool,
        #[arg(long)]
        json: bool,
    },
    /// Mark/unmark an entry as favorite
    Favorite {
        id: String,
    },
    Unfavorite {
        id: String,
    },
    /// Copy entry back to clipboard (placeholder)
    Copy {
        id: String,
    },
    /// Remove an entry or clear all
    Delete {
        id: Option<String>,
    },
    /// Show details about an entry
    Info {
        id: String,
    },
    /// Prune history by max items and/or age in days
    Prune {
        #[arg(long)]
        max_items: Option<usize>,
        #[arg(long)]
        max_age: Option<String>,
        #[arg(long, default_value_t = true)]
        keep_favorites: bool,
    },
    /// Self-check for environment capabilities (placeholder)
    Doctor,
    /// Database migrations
    Migrate {
        #[arg(long)]
        status: bool,
        #[arg(long)]
        backup: bool,
    },
    /// Print effective configuration and paths
    Config {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum SyncCmd {
    /// Run one sync iteration (push+pull by default)
    Run {
        #[arg(long)]
        push_only: bool,
        #[arg(long)]
        pull_only: bool,
    },
    /// Show sync status
    Status,
    /// Inspect remote: prints PRAGMA user_version and required tables/columns
    Doctor,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let settings = load_settings();
    let store = match cli.command {
        Commands::Migrate { .. } => build_store_readonly(&cli, &settings)?,
        _ => build_store(&cli, &settings)?,
    };

    match cli.command {
        Commands::InitDb => {
            store.init()?;
            println!("database initialized (placeholder)");
        }
        Commands::Add {
            text,
            image_path,
            image_from_clipboard,
        } => {
            let path_mode = settings
                .images
                .as_ref()
                .and_then(|i| i.local_file_path_mode)
                .unwrap_or(false);
            if let Some(path) = image_path {
                if path_mode {
                    let clip = store.add_image_from_path(&path)?;
                    println!("added image {} (file) {}", clip.id, path.display());
                } else {
                    let bytes = std::fs::read(&path)?;
                    let img = image::load_from_memory(&bytes)?;
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let clip = store.add_image_rgba(w, h, &rgba.into_raw())?;
                    println!("added image {} ({}x{})", clip.id, w, h);
                }
            } else if image_from_clipboard {
                let cb = SystemClipboard::new();
                if let Some(img) = cb.get_image()? {
                    if path_mode {
                        let dir = crate::config::images_dir(&settings);
                        std::fs::create_dir_all(&dir)?;
                        let dest = dir.join(format!("{}.png", chrono_like_timestamp()));
                        image::codecs::png::PngEncoder::new(std::fs::File::create(&dest)?)
                            .write_image(
                                &img.bytes,
                                img.width,
                                img.height,
                                image::ExtendedColorType::Rgba8,
                            )?;
                        let clip = store.add_image_from_path(&dest)?;
                        println!("added image {} (file) {}", clip.id, dest.display());
                    } else {
                        let clip = store.add_image_rgba(img.width, img.height, &img.bytes)?;
                        println!("added image {} ({}x{})", clip.id, img.width, img.height);
                    }
                } else {
                    eprintln!("no image in clipboard");
                }
            } else {
                let text = match text {
                    Some(t) => t,
                    None => {
                        use std::io::{self, Read};
                        let mut buf = String::new();
                        io::stdin().read_to_string(&mut buf)?;
                        buf
                    }
                };
                let clip = store.add(&text)?;
                println!("added {}", clip.id);
            }
        }
        Commands::List {
            favorites,
            images,
            limit,
            json,
        } => {
            if images {
                let items = store.list_images(Query {
                    contains: None,
                    favorites_only: favorites,
                    limit,
                })?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&items.iter().map(|(c,m)| serde_json::json!({
                        "id": c.id,
                        "favorite": c.is_favorite,
                        "created_at": c.created_at,
                        "path": c.image_path,
                        "meta": {"format": m.format, "width": m.width, "height": m.height, "size_bytes": m.size_bytes}
                    })).collect::<Vec<_>>())?);
                } else {
                    for (c, m) in items {
                        println!(
                            "{}\t{}\t{}x{} {} {}",
                            c.id,
                            if c.is_favorite { "*" } else { " " },
                            m.width,
                            m.height,
                            m.format,
                            c.image_path
                                .as_deref()
                                .map(|p| std::path::Path::new(p)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(p))
                                .unwrap_or("")
                        );
                    }
                }
            } else {
                let items = store.list(Query {
                    contains: None,
                    favorites_only: favorites,
                    limit,
                })?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&items)?);
                } else {
                    for c in items {
                        println!(
                            "{}\t{}\t{}",
                            c.id,
                            if c.is_favorite { "*" } else { " " },
                            preview(&c.text)
                        );
                    }
                }
            }
        }
        Commands::Search {
            query,
            favorites,
            json,
        } => {
            let items = store.list(Query {
                contains: Some(query),
                favorites_only: favorites,
                limit: None,
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                for c in items {
                    println!(
                        "{}\t{}\t{}",
                        c.id,
                        if c.is_favorite { "*" } else { " " },
                        preview(&c.text)
                    );
                }
            }
        }
        Commands::Favorite { id } => {
            store.favorite(&id, true)?;
            println!("favorited {}", id);
        }
        Commands::Unfavorite { id } => {
            store.favorite(&id, false)?;
            println!("unfavorited {}", id);
        }
        Commands::Copy { id } => {
            if let Some(c) = store.get(&id)? {
                let cb = SystemClipboard::new();
                match c.kind {
                    ClipKind::Text => {
                        cb.set_text(&c.text)?;
                        println!("copied {}", id);
                    }
                    ClipKind::Image => {
                        if let Some(img) = store.get_image_rgba(&id)? {
                            cb.set_image(&img)?;
                            println!("copied image {} ({}x{})", id, img.width, img.height);
                        } else {
                            eprintln!("image data not found: {}", id);
                        }
                    }
                }
            } else {
                eprintln!("not found: {}", id);
            }
        }
        Commands::Delete { id } => {
            if let Some(id) = id {
                store.delete(&id)?;
                println!("deleted {}", id);
            } else {
                store.clear()?;
                println!("cleared");
            }
        }
        Commands::Info { id } => {
            if let Some(c) = store.get(&id)? {
                match c.kind {
                    ClipKind::Text => {
                        println!("id:\t{}\nkind:\ttext\ncreated:\t{}\nfavorite:\t{}\nlen:\t{}\npreview:\t{}",
                            c.id, c.created_at, c.is_favorite, c.text.len(), preview(&c.text));
                    }
                    ClipKind::Image => {
                        if let Some(m) = store.get_image_meta(&id)? {
                            println!("id:\t{}\nkind:\timage\ncreated:\t{}\nfavorite:\t{}\nformat:\t{}\nsize:\t{} bytes\ndims:\t{}x{}\nsha256:\t{}\npath:\t{}",
                                c.id, c.created_at, c.is_favorite, m.format, m.size_bytes, m.width, m.height, m.sha256, c.image_path.as_deref().unwrap_or("<in-entry>"));
                        } else {
                            println!(
                                "id:\t{}\nkind:\timage (metadata missing)\npath:\t{}",
                                id,
                                c.image_path.as_deref().unwrap_or("<unknown>")
                            );
                        }
                    }
                }
            } else {
                eprintln!("not found: {}", id);
            }
        }
        Commands::Prune {
            max_items,
            max_age,
            keep_favorites,
        } => {
            let age =
                match max_age.or_else(|| settings.prune.as_ref().and_then(|p| p.max_age.clone())) {
                    Some(s) => Some(parse_human_duration(&s)?),
                    None => None,
                };
            let n = store.prune(
                max_items.or_else(|| settings.prune.as_ref().and_then(|p| p.max_items)),
                age,
                keep_favorites
                    || settings
                        .prune
                        .as_ref()
                        .and_then(|p| p.keep_favorites)
                        .unwrap_or(true),
            )?;
            println!("pruned {} entries", n);
        }
        Commands::Doctor => {
            // Clipboard check
            let cb = SystemClipboard::new();
            let cb_ok = cb.get_text().is_ok();
            println!("clipboard: {}", if cb_ok { "ok" } else { "unavailable" });
            // Store check: run a quick FTS probe via list(search)
            let _ = store.add("_doctor_probe_");
            let has_fts = store
                .list(Query {
                    contains: Some("_doctor_probe_".into()),
                    favorites_only: false,
                    limit: Some(1),
                })
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            println!(
                "search (fts or like): {}",
                if has_fts { "ok" } else { "failed" }
            );
        }
        Commands::Migrate { status, backup } => {
            // Only meaningful for SQLite
            let path = match &cli.db {
                Some(p) => p.clone(),
                None => default_db_path(),
            };
            let store_impl = StoreImpl::new_with(&path, false)?;
            if status {
                let s = store_impl.migration_status()?;
                println!(
                    "current: {}\nlatest: {}\npending: {}",
                    s.current,
                    s.latest,
                    s.pending.join(", ")
                );
            } else {
                if backup {
                    backup_db(&path)?;
                }
                store_impl.migrate_all()?;
                let s = store_impl.migration_status()?;
                println!("migrated to version {}", s.current);
            }
        }
        Commands::Sync { cmd } => {
            let cfg = &settings;
            let local_db = match &cli.db {
                Some(p) => p.clone(),
                None => default_db_path(),
            };
            let (url, token) = match &cfg.storage {
                config::Storage::Turso { url, auth_token } => {
                    (Some(url.as_str()), auth_token.as_deref())
                }
                _ => (None, None),
            };
            let device_id = cfg
                .sync
                .as_ref()
                .and_then(|s| s.device_id.clone())
                .or_else(|| std::env::var("DITOX_DEVICE_ID").ok())
                .or_else(|| whoami::fallible::hostname().ok())
                .unwrap_or_else(|| "local".into());
            let batch = cfg.sync.as_ref().and_then(|s| s.batch_size).unwrap_or(500);
            let engine =
                ditox_core::sync::SyncEngine::new(&local_db, url, token, Some(&device_id), batch)?;
            match cmd {
                SyncCmd::Run {
                    push_only,
                    pull_only,
                } => {
                    let rep = engine.run(push_only, pull_only)?;
                    println!("sync: pushed={} pulled={}", rep.pushed, rep.pulled);
                }
                SyncCmd::Status => {
                    let st = engine.status()?;
                    println!(
                        "last_push_updated_at={:?}\nlast_pull_updated_at={:?}\npending_local={}\nlocal_text={}\nlocal_images={}\nremote_ok={:?}\nlast_error={:?}",
                        st.last_push, st.last_pull, st.pending_local, st.local_text, st.local_images, st.remote_ok, st.last_error
                    );
                }
                SyncCmd::Doctor => {
                    #[cfg(feature = "libsql")]
                    {
                        if let (Some(url), Some(token)) = (url, token) {
                            use libsql::Builder;
                            let rt = tokio::runtime::Runtime::new()?;
                            let db = rt.block_on(async {
                                Builder::new_remote(url.to_string(), token.to_string())
                                    .build()
                                    .await
                            })?;
                            let conn = db.connect()?;
                            // PRAGMA user_version
                            let user_version: i64 = rt.block_on(async {
                                let mut rows = conn.query("PRAGMA user_version", ()).await?;
                                let r = rows.next().await?;
                                Ok::<i64, libsql::Error>(match r {
                                    Some(row) => row.get::<i64>(0)?,
                                    None => 0,
                                })
                            })?;
                            // Has clips table?
                            let has_clips: bool = rt.block_on(async {
                                let mut rows = conn
                                    .query(
                                        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='clips'",
                                        (),
                                    )
                                    .await?;
                                Ok::<bool, libsql::Error>(rows.next().await?.is_some())
                            })?;
                            // Columns present?
                            let mut cols: Vec<String> = Vec::new();
                            if has_clips {
                                let mut rows = rt.block_on(async {
                                    conn.query("PRAGMA table_info('clips')", ()).await
                                })?;
                                while let Some(r) = rt.block_on(async { rows.next().await })? {
                                    let name: String = r.get::<String>(1)?; // cid, name, type, notnull, dflt_value, pk
                                    cols.push(name);
                                }
                            }
                            // Row count
                            let count: i64 = if has_clips {
                                rt.block_on(async {
                                    let mut rows =
                                        conn.query("SELECT COUNT(1) FROM clips", ()).await?;
                                    let r = rows.next().await?;
                                    Ok::<i64, libsql::Error>(match r {
                                        Some(row) => row.get::<i64>(0)?,
                                        None => 0,
                                    })
                                })?
                            } else {
                                0
                            };
                            println!(
                                "remote_ok=true\nremote_user_version={}\nhas_clips={}\nclips_columns={}\nclips_count={}",
                                user_version,
                                has_clips,
                                if cols.is_empty() { "<none>".into() } else { cols.join(", ") },
                                count
                            );
                        } else {
                            println!("remote_ok=false (backend not configured as turso)");
                        }
                    }
                    #[cfg(not(feature = "libsql"))]
                    {
                        println!("built without 'libsql' feature; remote doctor unavailable");
                    }
                }
            }
        }
        Commands::Config { json } => {
            let cfg_dir = config::config_dir();
            let settings_path = cfg_dir.join("settings.toml");
            let db_path = match &settings.storage {
                config::Storage::LocalSqlite { db_path } => {
                    db_path.clone().unwrap_or_else(default_db_path)
                }
                _ => default_db_path(),
            };
            if json {
                let v = serde_json::json!({
                    "config_dir": cfg_dir,
                    "settings_path": settings_path,
                    "db_path": db_path,
                    "storage": match &settings.storage {
                        config::Storage::LocalSqlite { db_path } => serde_json::json!({"backend":"localsqlite","db_path":db_path}),
                        config::Storage::Turso { url, .. } => serde_json::json!({"backend":"turso","url":url}),
                    },
                    "prune": settings.prune,
                    "max_storage_mb": settings.max_storage_mb,
                });
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                println!("config_dir: {}", cfg_dir.display());
                println!("settings:  {}", settings_path.display());
                println!("db_path:   {}", db_path.display());
                match &settings.storage {
                    config::Storage::LocalSqlite { db_path } => println!(
                        "storage:  localsqlite (db_path={})",
                        db_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or("default".into())
                    ),
                    config::Storage::Turso { url, .. } => println!("storage:  turso (url={})", url),
                }
                if let Some(p) = &settings.prune {
                    println!(
                        "prune:     every={:?} keep_favorites={:?} max_items={:?} max_age={:?}",
                        p.every, p.keep_favorites, p.max_items, p.max_age
                    );
                }
                if let Some(m) = settings.max_storage_mb {
                    println!("max_storage_mb: {}", m);
                }
            }
        }
    }

    Ok(())
}

fn preview(s: &str) -> String {
    let s = s.replace('\n', " ");
    const MAX: usize = 60;
    if s.len() > MAX {
        format!("{}…", &s[..MAX])
    } else {
        s
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum StoreKind {
    Sqlite,
    Mem,
}

fn build_store(cli: &Cli, settings: &config::Settings) -> Result<Box<dyn Store>> {
    // Always operate on local SQLite (or mem when explicitly requested).
    Ok(match cli.store {
        StoreKind::Mem => Box::new(ditox_core::MemStore::new()),
        StoreKind::Sqlite => {
            let path = cli
                .db
                .clone()
                .or_else(|| match &settings.storage {
                    config::Storage::LocalSqlite { db_path } => db_path.clone(),
                    _ => None, // when backend=turso, still use default local DB path
                })
                .unwrap_or_else(default_db_path);
            std::fs::create_dir_all(path.parent().unwrap())?;
            let s = ditox_core::StoreImpl::new_with(path, cli.auto_migrate)?;
            Box::new(s)
        }
    })
}

fn build_store_readonly(cli: &Cli, settings: &config::Settings) -> Result<Box<dyn Store>> {
    // Always return local stores (or mem) for read-only operations as well.
    Ok(match cli.store {
        StoreKind::Mem => Box::new(ditox_core::MemStore::new()),
        StoreKind::Sqlite => {
            let path = cli
                .db
                .clone()
                .or_else(|| match &settings.storage {
                    config::Storage::LocalSqlite { db_path } => db_path.clone(),
                    _ => None,
                })
                .unwrap_or_else(default_db_path);
            std::fs::create_dir_all(path.parent().unwrap())?;
            let s = ditox_core::StoreImpl::new_with(path, false)?;
            Box::new(s)
        }
    })
}

fn default_db_path() -> PathBuf {
    let cfg = config::config_dir();
    let p = cfg.join("db").join("ditox.db");
    std::fs::create_dir_all(p.parent().unwrap()).ok();
    p
}

fn backup_db(path: &PathBuf) -> Result<PathBuf> {
    use std::fs;
    let ts = chrono_like_timestamp();
    let backup = path.with_extension(format!("bak.{}", ts));
    fs::copy(path, &backup)?;
    println!("backup: {}", backup.display());
    Ok(backup)
}

fn chrono_like_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let dt: time::OffsetDateTime = now.into();
    dt.format(&time::format_description::parse("yyyyMMddHHmmss").unwrap())
        .unwrap()
}

fn parse_human_duration(s: &str) -> Result<time::Duration> {
    use anyhow::bail;
    let s = s.trim();
    if s.is_empty() {
        bail!("empty duration")
    }
    let (num, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()));
    let n: i64 = num
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid number in duration: {}", s))?;
    let dur = match unit.trim().to_ascii_lowercase().as_str() {
        "s" => time::Duration::seconds(n),
        "m" => time::Duration::minutes(n),
        "h" => time::Duration::hours(n),
        "d" | "" => time::Duration::days(n),
        "w" => time::Duration::weeks(n),
        other => bail!("invalid unit '{}', use s/m/h/d/w", other),
    };
    Ok(dur)
}
mod config;
use config::load_settings;
