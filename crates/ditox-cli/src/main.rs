use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use directories::ProjectDirs;
use std::path::PathBuf;
use ditox_core::{Store, Query, StoreImpl, ClipKind};
#[cfg(all(target_os = "linux"))]
use ditox_core::clipboard::{ArboardClipboard as SystemClipboard, Clipboard as _};
#[cfg(not(target_os = "linux"))]
use ditox_core::clipboard::NoopClipboard as SystemClipboard;

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
        #[arg(long, conflicts_with="text")]
        image_path: Option<PathBuf>,
        /// Read image from system clipboard
        #[arg(long, conflicts_with="text")]
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
    Search { query: String, #[arg(long)] favorites: bool, #[arg(long)] json: bool },
    /// Mark/unmark an entry as favorite
    Favorite { id: String },
    Unfavorite { id: String },
    /// Copy entry back to clipboard (placeholder)
    Copy { id: String },
    /// Remove an entry or clear all
    Delete { id: Option<String> },
    /// Show details about an entry
    Info { id: String },
    /// Self-check for environment capabilities (placeholder)
    Doctor,
    /// Database migrations
    Migrate { #[arg(long)] status: bool, #[arg(long)] backup: bool },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = match cli.command { Commands::Migrate { .. } => build_store_readonly(&cli)?, _ => build_store(&cli)? };

    match cli.command {
        Commands::InitDb => {
            store.init()?;
            println!("database initialized (placeholder)");
        }
        Commands::Add { text, image_path, image_from_clipboard } => {
            if let Some(path) = image_path {
                let bytes = std::fs::read(&path)?;
                let img = image::load_from_memory(&bytes)?;
                let rgba = img.to_rgba8();
                let (w,h) = rgba.dimensions();
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
        Commands::List { favorites, images, limit, json } => {
            if images {
                let items = store.list_images(Query { contains: None, favorites_only: favorites, limit })?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&items.iter().map(|(c,m)| serde_json::json!({
                        "id": c.id,
                        "favorite": c.is_favorite,
                        "created_at": c.created_at,
                        "meta": {"format": m.format, "width": m.width, "height": m.height, "size_bytes": m.size_bytes}
                    })).collect::<Vec<_>>())?);
                } else {
                    for (c,m) in items { println!("{}\t{}\t{}x{} {}", c.id, if c.is_favorite {"*"} else {" "}, m.width, m.height, m.format); }
                }
            } else {
                let items = store.list(Query { contains: None, favorites_only: favorites, limit })?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&items)?);
                } else {
                    for c in items { println!("{}\t{}\t{}", c.id, if c.is_favorite {"*"} else {" "}, preview(&c.text)); }
                }
            }
        }
        Commands::Search { query, favorites, json } => {
            let items = store.list(Query { contains: Some(query), favorites_only: favorites, limit: None })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                for c in items { println!("{}\t{}\t{}", c.id, if c.is_favorite {"*"} else {" "}, preview(&c.text)); }
            }
        }
        Commands::Favorite { id } => { store.favorite(&id, true)?; println!("favorited {}", id); }
        Commands::Unfavorite { id } => { store.favorite(&id, false)?; println!("unfavorited {}", id); }
        Commands::Copy { id } => {
            if let Some(c) = store.get(&id)? {
                let cb = SystemClipboard::new();
                match c.kind {
                    ClipKind::Text => { cb.set_text(&c.text)?; println!("copied {}", id); }
                    ClipKind::Image => {
                        if let Some(img) = store.get_image_rgba(&id)? { cb.set_image(&img)?; println!("copied image {} ({}x{})", id, img.width, img.height); }
                        else { eprintln!("image data not found: {}", id); }
                    }
                }
            } else { eprintln!("not found: {}", id); }
        }
        Commands::Delete { id } => {
            if let Some(id) = id { store.delete(&id)?; println!("deleted {}", id); }
            else { store.clear()?; println!("cleared"); }
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
            } else { eprintln!("not found: {}", id); }
        }
        Commands::Doctor => {
            // Clipboard check
            let cb_ok = {
                let cb = SystemClipboard::new();
                match cb.get_text() { Ok(_) => true, Err(_) => false }
            };
            println!("clipboard: {}", if cb_ok {"ok"} else {"unavailable"});
            // Store check: run a quick FTS probe via list(search)
            let _ = store.add("_doctor_probe_");
            let has_fts = store.list(Query { contains: Some("_doctor_probe_".into()), favorites_only: false, limit: Some(1) }).map(|v| !v.is_empty()).unwrap_or(false);
            println!("search (fts or like): {}", if has_fts {"ok"} else {"failed"});
        }
        Commands::Migrate { status, backup } => {
            // Only meaningful for SQLite
            let path = match &cli.db { Some(p) => p.clone(), None => default_db_path() };
            let store_impl = StoreImpl::new_with(&path, false)?;
            if status {
                let s = store_impl.migration_status()?;
                println!("current: {}\nlatest: {}\npending: {}", s.current, s.latest, s.pending.join(", "));
            } else {
                if backup { backup_db(&path)?; }
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
    if s.len() > MAX { format!("{}…", &s[..MAX]) } else { s }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum StoreKind { Sqlite, Mem }

fn build_store(cli: &Cli) -> Result<Box<dyn Store>> {
    Ok(match cli.store {
        StoreKind::Mem => Box::new(ditox_core::MemStore::new()),
        StoreKind::Sqlite => {
            let path = match &cli.db {
                Some(p) => p.clone(),
                None => default_db_path(),
            };
            std::fs::create_dir_all(path.parent().unwrap())?;
            let s = ditox_core::StoreImpl::new_with(path, cli.auto_migrate)?;
            Box::new(s)
        }
    })
}

fn build_store_readonly(cli: &Cli) -> Result<Box<dyn Store>> {
    Ok(match cli.store {
        StoreKind::Mem => Box::new(ditox_core::MemStore::new()),
        StoreKind::Sqlite => {
            let path = match &cli.db { Some(p) => p.clone(), None => default_db_path() };
            std::fs::create_dir_all(path.parent().unwrap())?;
            let s = ditox_core::StoreImpl::new_with(path, false)?;
            Box::new(s)
        }
    })
}

fn default_db_path() -> PathBuf {
    if let Some(pd) = ProjectDirs::from("tech", "Ditox", "ditox") {
        pd.data_dir().join("ditox.db")
    } else {
        PathBuf::from("./ditox.db")
    }
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
    dt.format(&time::format_description::parse("yyyyMMddHHmmss").unwrap()).unwrap()
}
