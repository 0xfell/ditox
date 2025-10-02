use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use ditox_core::clipboard::NoopClipboard as SystemClipboard;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use ditox_core::clipboard::{ArboardClipboard as SystemClipboard, Clipboard as _};
use ditox_core::{ClipKind, Query, Store, StoreImpl};
use image::ImageEncoder;
use std::path::PathBuf;
// config module is declared at the top; avoid duplicate re-declaration here
mod copy_helpers;
mod doctor;
mod lazy_store;
mod picker;
mod theme;
mod managed_daemon;
mod xfer;

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
    /// Prefer wl-copy for copy operations (Linux), even if Wayland not detected
    #[arg(long, default_value_t = false)]
    force_wl_copy: bool,
    #[command(subcommand)]
    command: Option<Commands>,
    /// Timestamp precision for printed times (sec/ms/us/ns)
    #[arg(long, value_enum, default_value_t = TsPrec::Ns)]
    ts_precision: TsPrec,
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
    /// Interactive picker (built-in TUI)
    #[command(alias = "tui")]
    Pick {
        #[arg(long)]
        favorites: bool,
        /// Pick images instead of text entries
        #[arg(long)]
        images: bool,
        /// Force using the remote (Turso/libsql) backend directly
        #[arg(long, default_value_t = false)]
        remote: bool,
        /// Optional tag filter
        #[arg(long)]
        tag: Option<String>,
        /// Bypass daemon IPC and read DB directly
        #[arg(long)]
        no_daemon: bool,
        /// Managed daemon mode: managed|external|off (default managed)
        #[arg(long, value_enum)]
        daemon: Option<DaemonMode>,
        /// Managed daemon sampling interval, e.g. 200ms, 1s
        #[arg(long)]
        daemon_sample: Option<String>,
        /// Enable/disable managed daemon image capture
        #[arg(long)]
        daemon_images: Option<bool>,
        /// Theme name (built-in) or file path
        #[arg(long)]
        theme: Option<String>,
        /// Force ASCII mode (no Unicode borders/icons)
        #[arg(long, default_value_t = false)]
        ascii: bool,
        /// Color output: auto|always|never
        #[arg(long, value_enum, default_value_t = ColorWhen::Auto)]
        color: ColorWhen,
        /// Draw in main screen buffer (no alt-screen)
        #[arg(long, default_value_t = false)]
        no_alt_screen: bool,
        /// List available themes and exit
        #[arg(long, default_value_t = false)]
        themes: bool,
        /// Print an ASCII preview of a theme (no alt screen) and exit
        #[arg(long)]
        preview: Option<String>,
        /// Dump detected terminal capabilities and exit
        #[arg(long, default_value_t = false)]
        dump_caps: bool,
        /// Glyph pack name (built-in) or file path
        #[arg(long)]
        glyphs: Option<String>,
        /// Layout pack name (built-in) or file path
        #[arg(long)]
        layout: Option<String>,
        /// List available glyph packs and exit
        #[arg(long, default_value_t = false)]
        glyphsets: bool,
        /// List available layouts and exit
        #[arg(long, default_value_t = false)]
        layouts: bool,
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
        /// Show created/last_used timestamps (uses --ts-precision)
        #[arg(long, default_value_t = false)]
        show_times: bool,
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
    /// Export clips to a directory (JSONL + images)
    Export {
        dir: PathBuf,
        #[arg(long)]
        favorites: bool,
        #[arg(long)]
        images: bool,
        #[arg(long)]
        tag: Option<String>,
    },
    /// Import clips from a directory or file
    Import {
        path: PathBuf,
        /// Keep original IDs when present in input
        #[arg(long)]
        keep_ids: bool,
    },
    /// Manage tags for a clip
    Tag {
        #[command(subcommand)]
        cmd: TagCmd,
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
    /// Generate thumbnails for images (PNG 256px long side)
    Thumbs,
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

#[derive(Subcommand)]
enum TagCmd {
    /// List tags for a clip
    Ls { id: String },
    /// Add one or more tags to a clip
    Add { id: String, tags: Vec<String> },
    /// Remove one or more tags from a clip
    Rm { id: String, tags: Vec<String> },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
enum DaemonMode {
    Managed,
    External,
    Off,
}

// Fuzzy is the only matching mode (clipse-like)

fn main() -> Result<()> {
    let cli = Cli::parse();
    let settings = load_settings();
    let store: Box<dyn Store> = match &cli.command {
        // For `pick`, avoid opening DBs up front; we pass a lazy store below.
        Some(Commands::Pick { .. }) => Box::new(ditox_core::MemStore::new()),
        // Migrations are local-only by design; keep read-only local store here.
        Some(Commands::Migrate { .. }) => build_store_readonly(&cli, &settings)?,
        // Others follow configured backend.
        _ => build_store(&cli, &settings)?,
    };

    match cli.command.unwrap_or(Commands::Pick {
        favorites: false,
        images: false,
        remote: false,
        tag: None,
        no_daemon: false,
        daemon: None,
        daemon_sample: None,
        daemon_images: None,
        theme: None,
        ascii: false,
        color: ColorWhen::Auto,
        no_alt_screen: false,
        themes: false,
        preview: None,
        dump_caps: false,
        glyphs: None,
        layout: None,
        glyphsets: false,
        layouts: false,
    }) {
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
            show_times,
        } => {
            if images {
                let items = store.list_images(Query {
                    contains: None,
                    favorites_only: favorites,
                    limit,
                    tag: None,
                    rank: false,
                })?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&items.iter().map(|(c,m)| serde_json::json!({
                        "id": c.id,
                        "favorite": c.is_favorite,
                        "created_at": fmt_ts_prec(&c.created_at, cli.ts_precision),
                        "path": c.image_path,
                        "meta": {"format": m.format, "width": m.width, "height": m.height, "size_bytes": m.size_bytes}
                    })).collect::<Vec<_>>())?);
                } else {
                    for (c, m) in items {
                        if show_times {
                            let last = c
                                .last_used_at
                                .map(|t| fmt_ts_prec(&t, cli.ts_precision))
                                .unwrap_or_else(|| "never".into());
                            println!(
                                "{}\t{}\t{}\t{}\t{}x{} {} {}",
                                c.id,
                                if c.is_favorite { "*" } else { " " },
                                fmt_ts_prec(&c.created_at, cli.ts_precision),
                                last,
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
                        } else {
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
                }
            } else {
                let items = store.list(Query {
                    contains: None,
                    favorites_only: favorites,
                    limit,
                    tag: None,
                    rank: false,
                })?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&items.iter().map(|c| serde_json::json!({
                        "id": c.id,
                        "favorite": c.is_favorite,
                        "created_at": fmt_ts_prec(&c.created_at, cli.ts_precision),
                        "last_used_at": c.last_used_at.map(|t| fmt_ts_prec(&t, cli.ts_precision)),
                        "text": c.text,
                    })).collect::<Vec<_>>())?);
                } else if show_times {
                    for c in items {
                        let last = c
                            .last_used_at
                            .map(|t| fmt_ts_prec(&t, cli.ts_precision))
                            .unwrap_or_else(|| "never".into());
                        println!(
                            "{}\t{}\t{}\t{}\t{}",
                            c.id,
                            if c.is_favorite { "*" } else { " " },
                            fmt_ts_prec(&c.created_at, cli.ts_precision),
                            last,
                            preview(&c.text)
                        );
                    }
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
        Commands::Pick {
            favorites,
            images,
            remote,
            tag,
            no_daemon,
            daemon,
            daemon_sample,
            daemon_images,
            theme,
            ascii,
            color,
            no_alt_screen,
            themes,
            preview,
            dump_caps,
            glyphs,
            layout,
            glyphsets,
            layouts } => {
            if no_alt_screen { std::env::set_var("DITOX_TUI_ALT_SCREEN", "0"); }
            // Utility modes that don't start the TUI
            if themes {
                for name in theme::available_themes() {
                    println!("{}", name);
                }
                return Ok(());
            }
            if glyphsets {
                for name in theme::available_glyph_packs() {
                    println!("{}", name);
                }
                return Ok(());
            }
            if layouts {
                for name in theme::available_layouts() {
                    println!("{}", name);
                }
                return Ok(());
            }
            if dump_caps {
                let caps = theme::detect_caps();
                println!("color_depth: {}", caps.color_depth);
                println!("unicode: {}", if caps.unicode { "yes" } else { "no" });
                println!("no_color: {}", if caps.no_color { "yes" } else { "no" });
                return Ok(());
            }
            if let Some(name) = &preview {
                if let Some(t) = &theme {
                    std::env::set_var("DITOX_TUI_THEME", t);
                } else {
                    std::env::set_var("DITOX_TUI_THEME", name);
                }
                match color {
                    ColorWhen::Auto => std::env::set_var("DITOX_TUI_COLOR", "auto"),
                    ColorWhen::Always => std::env::set_var("DITOX_TUI_COLOR", "always"),
                    ColorWhen::Never => std::env::set_var("DITOX_TUI_COLOR", "never"),
                }
                if ascii {
                    std::env::set_var("DITOX_TUI_ASCII", "1");
                }
                if let Some(g) = &glyphs {
                    std::env::set_var("DITOX_TUI_GLYPHS", g);
                }
                if let Some(l) = &layout {
                    std::env::set_var("DITOX_TUI_LAYOUT", l);
                }
                theme::print_ascii_preview(name);
                return Ok(());
            }
            // Set env that the theme loader will read
            if let Some(t) = &theme {
                std::env::set_var("DITOX_TUI_THEME", t);
            }
            match color {
                ColorWhen::Auto => std::env::set_var("DITOX_TUI_COLOR", "auto"),
                ColorWhen::Always => std::env::set_var("DITOX_TUI_COLOR", "always"),
                ColorWhen::Never => std::env::set_var("DITOX_TUI_COLOR", "never"),
            }
            if ascii {
                std::env::set_var("DITOX_TUI_ASCII", "1");
            }
            if let Some(g) = &glyphs {
                std::env::set_var("DITOX_TUI_GLYPHS", g);
            }
            if let Some(l) = &layout {
                std::env::set_var("DITOX_TUI_LAYOUT", l);
            }
            // Fuzzy is the only mode
            std::env::set_var("DITOX_MATCH_MODE", "fuzzy");
            // Apply settings defaults if CLI did not override
            if std::env::var("DITOX_TUI_THEME").is_err() {
                if let Some(t) = settings.tui.as_ref().and_then(|t| t.theme.clone()) {
                    std::env::set_var("DITOX_TUI_THEME", t);
                }
            }
            if std::env::var("DITOX_TUI_COLOR").is_err() {
                if let Some(c) = settings.tui.as_ref().and_then(|t| t.color.clone()) {
                    std::env::set_var("DITOX_TUI_COLOR", c);
                }
            }
            if std::env::var("DITOX_TUI_ASCII").is_err()
                && settings
                    .tui
                    .as_ref()
                    .and_then(|t| t.box_chars.clone())
                    .map(|v| v.eq_ignore_ascii_case("ascii"))
                    .unwrap_or(false)
            {
                std::env::set_var("DITOX_TUI_ASCII", "1");
            }
            if std::env::var("DITOX_TUI_ALT_SCREEN").is_err() {
                if let Some(on) = settings.tui.as_ref().and_then(|t| t.alt_screen) {
                    std::env::set_var("DITOX_TUI_ALT_SCREEN", if on { "1" } else { "0" });
                }
            }
            if std::env::var("DITOX_TUI_DATE_FMT").is_err() {
                if let Some(fmt) = settings.tui.as_ref().and_then(|t| t.date_format.clone()) {
                    std::env::set_var("DITOX_TUI_DATE_FMT", fmt);
                }
            }
            if std::env::var("DITOX_TUI_AUTO_DAYS").is_err() {
                if let Some(days) = settings.tui.as_ref().and_then(|t| t.auto_recent_days) {
                    std::env::set_var("DITOX_TUI_AUTO_DAYS", days.to_string());
                }
            }
            if std::env::var("DITOX_TUI_GLYPHS").is_err() {
                if let Some(g) = settings.tui.as_ref().and_then(|t| t.glyphs.clone()) {
                    std::env::set_var("DITOX_TUI_GLYPHS", g);
                }
            }
            if std::env::var("DITOX_TUI_LAYOUT").is_err() {
                if let Some(l) = settings.tui.as_ref().and_then(|t| t.layout.clone()) {
                    std::env::set_var("DITOX_TUI_LAYOUT", l);
                }
            }
            // Build a lazy store so the first TUI frame appears instantly.
            // Policy:
            // - --remote forces Turso/libsql and disables daemon.
            // - Otherwise, use local SQLite for picker operations so daemon path and direct DB writes stay consistent.
            use std::sync::Arc;
            let lazy = if remote {
                #[cfg(feature = "libsql")]
                {
                    match &settings.storage {
                        config::Storage::Turso { url, auth_token } => {
                            Arc::new(lazy_store::LazyStore::remote_libsql(url.clone(), auth_token.clone()))
                        }
                        _ => {
                            eprintln!(
                                "remote backend not configured; falling back to local SQLite"
                            );
                            Arc::new(lazy_store::LazyStore::local_sqlite(default_db_path(), false))
                        }
                    }
                }
                #[cfg(not(feature = "libsql"))]
                {
                    eprintln!(
                        "built without 'libsql' feature; --remote unavailable — using local SQLite"
                    );
                    Arc::new(lazy_store::LazyStore::local_sqlite(default_db_path(), false))
                }
            } else {
                // Local store (matches clipd’s DB) — use configured path when present
                let path = match &settings.storage {
                    config::Storage::LocalSqlite { db_path } => {
                        db_path.clone().unwrap_or_else(default_db_path)
                    }
                    config::Storage::Turso { .. } => default_db_path(),
                };
                Arc::new(lazy_store::LazyStore::local_sqlite(path, false))
            };
            // If --remote, bypass daemon even if running
            let bypass_daemon = no_daemon || remote;
            // Managed daemon determination
            let env_mode = std::env::var("DITOX_DAEMON").ok();
            let cfg_mode = settings
                .daemon
                .as_ref()
                .and_then(|d| d.mode.clone())
                .and_then(|m| match m.to_ascii_lowercase().as_str() {
                    "managed" => Some(DaemonMode::Managed),
                    "external" => Some(DaemonMode::External),
                    "off" => Some(DaemonMode::Off),
                    _ => None,
                });
            let mut mode = daemon
                .or_else(|| env_mode.as_deref().and_then(|m| match m {
                    "managed" => Some(DaemonMode::Managed),
                    "external" => Some(DaemonMode::External),
                    "off" => Some(DaemonMode::Off),
                    _ => None,
                }))
                .or(cfg_mode)
                .unwrap_or(DaemonMode::Managed);
            if bypass_daemon { mode = DaemonMode::Off; }
            let sample_str = daemon_sample
                .or_else(|| std::env::var("DITOX_DAEMON_SAMPLE").ok())
                .or_else(|| settings.daemon.as_ref().and_then(|d| d.sample.clone()))
                .unwrap_or_else(|| "200ms".into());
            let sample_ms = parse_sample_ms(&sample_str).unwrap_or(200);
            let images_on = daemon_images
                .or_else(|| {
                    std::env::var("DITOX_DAEMON_IMAGES").ok().and_then(|v| match v.as_str() {
                        "1" | "true" | "on" => Some(true),
                        "0" | "false" | "off" => Some(false),
                        _ => None,
                    })
                })
                .or_else(|| settings.daemon.as_ref().and_then(|d| d.images))
                .unwrap_or(true);

            let mut managed_handle: Option<crate::managed_daemon::ManagedHandle> = None;
            if !remote {
                match mode {
                    DaemonMode::Managed => {
                        if crate::managed_daemon::detect_external_clipd() {
                            std::env::set_var("DITOX_CAPTURE_STATUS", "external");
                        } else {
                            match crate::managed_daemon::start_managed(lazy.clone(), crate::managed_daemon::DaemonConfig { sample_ms, images: images_on }) {
                                Ok(h) => {
                                    let ctrl = h.control();
                                    crate::managed_daemon::set_global_control(ctrl);
                                    managed_handle = Some(h);
                                    std::env::set_var("DITOX_CAPTURE_STATUS", format!("managed(active,{}ms,images:{})", sample_ms, images_on));
                                }
                                Err(_) => {
                                    std::env::set_var("DITOX_CAPTURE_STATUS", "off");
                                }
                            }
                        }
                    }
                    DaemonMode::External => { std::env::set_var("DITOX_CAPTURE_STATUS", "external"); }
                    DaemonMode::Off => { std::env::set_var("DITOX_CAPTURE_STATUS", "off"); }
                }
            }

            picker::run_picker_default(
                &*lazy,
                favorites,
                images,
                tag,
                bypass_daemon,
                cli.force_wl_copy,
                remote,
            )?;
            // Drop handle after TUI exits to stop managed daemon and clean lock
            drop(managed_handle);
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
                tag: None,
                rank: false,
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
                match c.kind {
                    ClipKind::Text => {
                        copy_helpers::copy_text(&c.text, cli.force_wl_copy)?;
                        println!("copied {}", id);
                    }
                    ClipKind::Image => {
                        if let Some(img) = store.get_image_rgba(&id)? {
                            copy_helpers::copy_image(&img, cli.force_wl_copy)?;
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
                            c.id, fmt_ts_prec(&c.created_at, cli.ts_precision), c.is_favorite, c.text.len(), preview(&c.text));
                    }
                    ClipKind::Image => {
                        if let Some(m) = store.get_image_meta(&id)? {
                            println!("id:\t{}\nkind:\timage\ncreated:\t{}\nfavorite:\t{}\nformat:\t{}\nsize:\t{} bytes\ndims:\t{}x{}\nsha256:\t{}\npath:\t{}",
                                c.id, fmt_ts_prec(&c.created_at, cli.ts_precision), c.is_favorite, m.format, m.size_bytes, m.width, m.height, m.sha256, c.image_path.as_deref().unwrap_or("<in-entry>"));
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
        Commands::Export {
            dir,
            favorites,
            images,
            tag,
        } => {
            xfer::export_all(&*store, &dir, favorites, images, tag.as_deref())?;
            println!("exported to {}", dir.display());
        }
        Commands::Import { path, keep_ids } => {
            let n = xfer::import_all(&*store, &path, keep_ids)?;
            println!("imported {} items", n);
        }
        Commands::Tag { cmd } => match cmd {
            TagCmd::Ls { id } => {
                let tags = store.list_tags(&id)?;
                if tags.is_empty() {
                    println!("<none>");
                } else {
                    println!("{}", tags.join(" "));
                }
            }
            TagCmd::Add { id, tags } => {
                store.add_tags(&id, &tags)?;
                println!("tags added to {}", id);
            }
            TagCmd::Rm { id, tags } => {
                store.remove_tags(&id, &tags)?;
                println!("tags removed from {}", id);
            }
        },
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
            let cb_res = cb.get_text();
            let cb_ok = cb_res.is_ok();
            println!("clipboard: {}", if cb_ok { "ok" } else { "unavailable" });
            if let Err(e) = cb_res {
                println!("clipboard_detail: {}", e);
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                println!("clipboard_hint: other apps may lock the clipboard; try retrying or closing clipboard managers.");
            }
            // Tool round-trip checks (OS-specific)
            doctor::clipboard_tools_roundtrip();
            // Store check: run a quick FTS probe via list(search)
            let _ = store.add("_doctor_probe_");
            let has_fts = store
                .list(Query {
                    contains: Some("_doctor_probe_".into()),
                    favorites_only: false,
                    limit: Some(1),
                    tag: None,
                    rank: false,
                })
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            println!(
                "search (fts or like): {}",
                if has_fts { "ok" } else { "failed" }
            );
            // Daemon check
            let clipd_info = config::config_dir().join("clipd.json");
            if let Ok(s) = std::fs::read_to_string(&clipd_info) {
                let v: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
                println!(
                    "clipd: present (port={})",
                    v.get("port").and_then(|p| p.as_u64()).unwrap_or(0)
                );
            } else {
                println!("clipd: not running");
            }
            // Managed capture lock presence
            let lp = crate::config::state_dir().join("managed-daemon.lock");
            if lp.exists() {
                println!("managed: lock present ({})", lp.display());
            } else {
                println!("managed: off");
            }
        }
        Commands::Thumbs => {
            // best-effort: iterate images and create thumbs under config dir
            let imgs = store.list_images(Query {
                contains: None,
                favorites_only: false,
                limit: None,
                tag: None,
                rank: false,
            })?;
            let root = config::config_dir();
            let thumbs = root.join("thumbs");
            std::fs::create_dir_all(&thumbs)?;
            let mut made = 0usize;
            for (c, _m) in imgs {
                if let Some(img) = store.get_image_rgba(&c.id)? {
                    let mut buf = Vec::new();
                    image::codecs::png::PngEncoder::new(&mut buf).write_image(
                        &img.bytes,
                        img.width,
                        img.height,
                        image::ExtendedColorType::Rgba8,
                    )?;
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(&buf);
                    let sha = hex::encode(hasher.finalize());
                    let (a, b) = (&sha[0..2], &sha[2..4]);
                    let dir = thumbs.join(a).join(b);
                    std::fs::create_dir_all(&dir)?;
                    let path = dir.join(format!("{}_256.png", sha));
                    if !path.exists() {
                        std::fs::write(&path, &buf)?;
                        made += 1;
                    }
                }
            }
            println!("thumbnails generated: {}", made);
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

pub fn preview(s: &str) -> String {
    let s = s.replace('\n', " ");
    const MAX: usize = 60;
    if s.chars().count() > MAX {
        let cut = s
            .char_indices()
            .nth(MAX)
            .map(|(idx, _)| idx)
            .unwrap_or_else(|| s.len());
        format!("{}…", &s[..cut])
    } else {
        s
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum StoreKind {
    Sqlite,
    Mem,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum TsPrec {
    Sec,
    Ms,
    Us,
    Ns,
}

fn fmt_ts_prec(ts: &time::OffsetDateTime, p: TsPrec) -> String {
    match p {
        TsPrec::Sec => ts
            .format(
                &time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                    .unwrap(),
            )
            .unwrap_or_else(|_| ts.to_string()),
        TsPrec::Ms => ts
            .format(
                &time::format_description::parse(
                    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]",
                )
                .unwrap(),
            )
            .unwrap_or_else(|_| ts.to_string()),
        TsPrec::Us => ts
            .format(
                &time::format_description::parse(
                    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]",
                )
                .unwrap(),
            )
            .unwrap_or_else(|_| ts.to_string()),
        TsPrec::Ns => ts
            .format(
                &time::format_description::parse(
                    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:9]",
                )
                .unwrap(),
            )
            .unwrap_or_else(|_| ts.to_string()),
    }
}

fn build_store(cli: &Cli, settings: &config::Settings) -> Result<Box<dyn Store>> {
    // CLI flag precedence: mem/sqlite flags override settings.backend
    match cli.store {
        StoreKind::Mem => return Ok(Box::new(ditox_core::MemStore::new())),
        StoreKind::Sqlite => {
            // fall through to local sqlite path resolution below
        }
    }

    // No explicit sqlite flag: respect settings. If configured for Turso and built with libsql,
    // use the remote store. Images remain unsupported in the remote backend.
    #[cfg(feature = "libsql")]
    if let config::Storage::Turso { url, auth_token } = &settings.storage {
        // Only use remote for commands that are explicitly remote-centric (e.g., sync)
        // All other commands default to local SQLite unless the command itself opts into remote
        match &cli.command {
            Some(Commands::Sync { .. }) => {
                let s = ditox_core::libsql_backend::LibsqlStore::new(url, auth_token.as_deref())?;
                return Ok(Box::new(s));
            }
            _ => { /* fall through to local sqlite */ }
        }
    }

    // Fallback to local SQLite store.
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
    Ok(Box::new(s))
}

fn build_store_readonly(cli: &Cli, settings: &config::Settings) -> Result<Box<dyn Store>> {
    // CLI flag precedence: honor mem/sqlite explicitly; otherwise, fallback to settings.
    match cli.store {
        StoreKind::Mem => Ok(Box::new(ditox_core::MemStore::new())),
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
            Ok(Box::new(s))
        }
    }
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

#[allow(dead_code)]
fn fmt_ts(ts: &time::OffsetDateTime) -> String {
    // Example: 2025-09-30 07:07:00.490854340
    static FMT: once_cell::sync::Lazy<Vec<time::format_description::FormatItem>> =
        once_cell::sync::Lazy::new(|| {
            time::format_description::parse(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:9]",
            )
            .unwrap()
        });
    ts.format(&FMT).unwrap_or_else(|_| ts.to_string())
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

fn parse_sample_ms(s: &str) -> Result<u64> {
    // Accept numbers with optional unit: ms|s. Default to ms if no unit.
    let s = s.trim();
    if s.is_empty() { return Ok(200); }
    let (num, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()));
    let n: u64 = num.parse().unwrap_or(200);
    let ms = match unit.trim() {
        "s" => n.saturating_mul(1000),
        "ms" | "" => n,
        _ => n,
    };
    Ok(ms)
}
