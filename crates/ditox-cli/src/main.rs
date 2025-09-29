use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use directories::ProjectDirs;
#[cfg(not(target_os = "linux"))]
use ditox_core::clipboard::NoopClipboard as SystemClipboard;
#[cfg(all(target_os = "linux"))]
use ditox_core::clipboard::{ArboardClipboard as SystemClipboard, Clipboard as _};
use ditox_core::{ClipKind, Query, Store, StoreImpl};
use std::path::PathBuf;

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
            if let Some(path) = image_path {
                let bytes = std::fs::read(&path)?;
                let img = image::load_from_memory(&bytes)?;
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let clip = store.add_image_rgba(w, h, &rgba.into_raw())?;
                println!("added image {} ({}x{})", clip.id, w, h);
            } else if image_from_clipboard {
                let cb = SystemClipboard::new();
                if let Some(img) = cb.get_image()? {
                    let clip = store.add_image_rgba(img.width, img.height, &img.bytes)?;
                    println!("added image {} ({}x{})", clip.id, img.width, img.height);
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
                        "meta": {"format": m.format, "width": m.width, "height": m.height, "size_bytes": m.size_bytes}
                    })).collect::<Vec<_>>())?);
                } else {
                    for (c, m) in items {
                        println!(
                            "{}\t{}\t{}x{} {}",
                            c.id,
                            if c.is_favorite { "*" } else { " " },
                            m.width,
                            m.height,
                            m.format
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
                            println!("id:\t{}\nkind:\timage\ncreated:\t{}\nfavorite:\t{}\nformat:\t{}\nsize:\t{} bytes\ndims:\t{}x{}\nsha256:\t{}",
                                c.id, c.created_at, c.is_favorite, m.format, m.size_bytes, m.width, m.height, m.sha256);
                        } else {
                            println!("id:\t{}\nkind:\timage (metadata missing)", id);
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
            let cb_ok = {
                let cb = SystemClipboard::new();
                match cb.get_text() {
                    Ok(_) => true,
                    Err(_) => false,
                }
            };
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
    // Prefer Turso/libSQL if requested in settings
    if let config::Storage::Turso { url, auth_token } = &settings.storage {
        #[cfg(feature = "libsql")]
        {
            let s = ditox_core::libsql_backend::LibsqlStore::new(url, auth_token.as_deref())?;
            return Ok(Box::new(s));
        }
        #[cfg(not(feature = "libsql"))]
        {
            eprintln!("warning: settings.backend=turso but this binary is built without 'libsql' feature; using local sqlite instead");
        }
    }
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
            let s = ditox_core::StoreImpl::new_with(path, cli.auto_migrate)?;
            Box::new(s)
        }
    })
}

fn build_store_readonly(cli: &Cli, settings: &config::Settings) -> Result<Box<dyn Store>> {
    if let config::Storage::Turso { url, auth_token } = &settings.storage {
        #[cfg(feature = "libsql")]
        {
            let s = ditox_core::libsql_backend::LibsqlStore::new(url, auth_token.as_deref())?;
            return Ok(Box::new(s));
        }
        #[cfg(not(feature = "libsql"))]
        {
            eprintln!("warning: settings.backend=turso but this binary is built without 'libsql' feature; using local sqlite instead");
        }
    }
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
use config::{load_settings, settings_path};
