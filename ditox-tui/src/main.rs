mod cli;
mod keybindings;
mod ui;

use clap::Parser;
use cli::{Cli, CollectionCommands, Commands};
use ditox_core::{
    Clipboard, Collection, Config, Database, DitoxError, Entry, EntryType, Result, Watcher,
};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as MatcherConfig, Matcher};
use tracing_subscriber::EnvFilter;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("ditox=info")),
        )
        .init();

    let cli = Cli::parse();
    let config = Config::load()?;
    let mut db = Database::open()?;
    db.init_schema()?;

    match cli.command {
        None => run_tui(db, config),
        Some(Commands::Watch) => run_watcher(db, config),
        Some(Commands::List {
            limit,
            json,
            favorites,
        }) => cmd_list(&db, limit, json, favorites),
        Some(Commands::Get { target, json }) => cmd_get(&db, &target, json),
        Some(Commands::Search { query, limit, json }) => cmd_search(&db, &query, limit, json),
        Some(Commands::Copy { target }) => cmd_copy(&db, &target),
        Some(Commands::Delete { target }) => cmd_delete(&mut db, &target),
        Some(Commands::Favorite { target }) => cmd_favorite(&db, &target),
        Some(Commands::Clear { confirm }) => cmd_clear(&mut db, confirm),
        Some(Commands::Count) => cmd_count(&db),
        Some(Commands::Status) => cmd_status(&db),
        Some(Commands::Stats { json }) => cmd_stats(&db, json),
        Some(Commands::Repair {
            dry_run,
            fix_hashes,
        }) => cmd_repair(&mut db, dry_run, fix_hashes),
        Some(Commands::Collection(subcmd)) => cmd_collection(&db, subcmd),
    }
}

fn run_tui(db: Database, config: Config) -> Result<()> {
    ui::run(db, config)
}

fn run_watcher(db: Database, config: Config) -> Result<()> {
    let mut watcher = Watcher::new(db, config);
    watcher.run()
}

fn cmd_list(db: &Database, limit: usize, json: bool, favorites_only: bool) -> Result<()> {
    let mut entries = db.get_all(limit)?;

    if favorites_only {
        entries.retain(|e| e.favorite);
    }

    if json {
        let json_output = serde_json::to_string_pretty(&entries)
            .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);
    } else {
        if entries.is_empty() {
            println!("No clipboard entries found.");
            return Ok(());
        }

        println!(
            "{:>3} │ {:^4} │ {:^3} │ {:<40} │ {:>6}",
            "#", "Type", "Fav", "Content", "Age"
        );
        println!("────┼──────┼─────┼──────────────────────────────────────────┼────────");

        for (i, entry) in entries.iter().enumerate() {
            println!(
                "{:>3} │ {:^4} │ {:^3} │ {:<40} │ {:>6}",
                i + 1,
                entry.entry_type.short(),
                if entry.favorite { "⭐" } else { "" },
                entry.preview(40),
                entry.relative_time()
            );
        }
    }

    Ok(())
}

fn cmd_copy(db: &Database, target: &str) -> Result<()> {
    let entry = resolve_target(db, target)?;

    match entry {
        Some(entry) => {
            match entry.entry_type {
                EntryType::Text => {
                    Clipboard::set_text(&entry.content)?;
                    println!("Copied: {}", entry.preview(50));
                }
                EntryType::Image => {
                    let path = entry
                        .image_path()
                        .ok_or_else(|| DitoxError::Other("image entry missing extension".into()))?;
                    Clipboard::set_image(&path.to_string_lossy())?;
                    println!("Copied image: {}", entry.preview(50));
                }
            }
            // Update last_used timestamp
            db.touch(&entry.id)?;
            Ok(())
        }
        None => Err(DitoxError::NotFound(format!("Entry not found: {}", target))),
    }
}

fn cmd_clear(db: &mut Database, confirm: bool) -> Result<()> {
    if !confirm {
        print!("Clear all clipboard history? [y/N] ");
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // `clear_all` queues every image blob for pruning inside the same SQL
    // transaction and then drains the queue, so we don't need (and must not
    // do) a separate `remove_dir_all` — that would clobber pinned images or
    // the quarantine directory managed by `ditox repair`.
    let count = db.clear_all()?;
    println!("Cleared {} entries.", count);

    Ok(())
}

fn cmd_status(db: &Database) -> Result<()> {
    let count = db.count()?;
    let data_dir = Database::get_data_dir()?;
    let images_dir = Database::get_images_dir()?;

    println!("Ditox Status");
    println!("────────────");
    println!("Entries:     {}", count);
    println!("Data dir:    {}", data_dir.display());
    println!("Images dir:  {}", images_dir.display());

    // Check if images directory exists and count files
    if images_dir.exists() {
        let image_count = std::fs::read_dir(&images_dir)
            .map(|entries| entries.count())
            .unwrap_or(0);
        println!("Image files: {}", image_count);
    }

    Ok(())
}

fn cmd_stats(db: &Database, json: bool) -> Result<()> {
    let stats = db.get_stats()?;

    if json {
        let json_output = serde_json::to_string_pretty(&stats)
            .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);
    } else {
        print!("{}", stats.display());
    }

    Ok(())
}

fn cmd_get(db: &Database, target: &str, json: bool) -> Result<()> {
    let entry = resolve_target(db, target)?;

    match entry {
        Some(entry) => {
            if json {
                let json_output = serde_json::to_string_pretty(&entry)
                    .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
                println!("{}", json_output);
            } else {
                // Print raw content for piping
                print!("{}", entry.content);
            }
            Ok(())
        }
        None => Err(DitoxError::NotFound(format!("Entry not found: {}", target))),
    }
}

fn cmd_search(db: &Database, query: &str, limit: usize, json: bool) -> Result<()> {
    // Load all entries and perform fuzzy search (same as TUI)
    let entries = db.get_all(1000)?; // Load enough entries for searching

    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(MatcherConfig::DEFAULT);

    let mut matches: Vec<(&Entry, u32)> = entries
        .iter()
        .filter_map(|e| {
            let haystack = &e.content;
            let mut buf = Vec::new();
            let score = pattern.score(
                nucleo_matcher::Utf32Str::new(haystack, &mut buf),
                &mut matcher,
            )?;
            Some((e, score))
        })
        .collect();

    // Sort by score descending.
    matches.sort_by_key(|m| std::cmp::Reverse(m.1));

    // Take only up to limit
    let results: Vec<&Entry> = matches.iter().take(limit).map(|(e, _)| *e).collect();

    if json {
        let json_output = serde_json::to_string_pretty(&results)
            .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);
    } else {
        if results.is_empty() {
            println!("No matches found for: {}", query);
            return Ok(());
        }

        println!(
            "{:>3} │ {:^4} │ {:^3} │ {:<40} │ {:>6}",
            "#", "Type", "Pin", "Content", "Age"
        );
        println!("────┼──────┼─────┼──────────────────────────────────────────┼────────");

        for (i, entry) in results.iter().enumerate() {
            println!(
                "{:>3} │ {:^4} │ {:^3} │ {:<40} │ {:>6}",
                i + 1,
                entry.entry_type.short(),
                if entry.favorite { "⭐" } else { "" },
                entry.preview(40),
                entry.relative_time()
            );
        }
    }

    Ok(())
}

fn cmd_delete(db: &mut Database, target: &str) -> Result<()> {
    let entry = resolve_target(db, target)?;

    match entry {
        Some(entry) => {
            let preview = entry.preview(30);
            let id = entry.id.clone();

            // `Database::delete` handles the blob cleanup via the pending
            // prune queue; don't unlink by hand here (doing so would race
            // with the queue drain and could delete an unrelated blob if
            // hashes ever collided).
            db.delete(&id)?;
            println!("Deleted: {}", preview);
            Ok(())
        }
        None => Err(DitoxError::NotFound(format!("Entry not found: {}", target))),
    }
}

fn cmd_favorite(db: &Database, target: &str) -> Result<()> {
    let entry = resolve_target(db, target)?;

    match entry {
        Some(entry) => {
            let preview = entry.preview(30);
            let was_favorite = entry.favorite;
            db.toggle_favorite(&entry.id)?;

            if was_favorite {
                println!("Removed from favorites: {}", preview);
            } else {
                println!("Added to favorites: {}", preview);
            }
            Ok(())
        }
        None => Err(DitoxError::NotFound(format!("Entry not found: {}", target))),
    }
}

fn cmd_count(db: &Database) -> Result<()> {
    let count = db.count()?;
    println!("{}", count);
    Ok(())
}

/// Reconcile the image store with the database. See the `Repair` variant in
/// cli.rs for user-facing docs. Exit code is 0 on success (even if fixes
/// were applied); callers distinguish dry-run vs fix via flags, not exit.
fn cmd_repair(db: &mut Database, dry_run: bool, fix_hashes: bool) -> Result<()> {
    use std::collections::HashSet;

    let mode = if dry_run { "[dry-run] " } else { "" };

    // 1. Dangling rows: DB says "image" but the blob is gone.
    let rows = db.image_rows_with_paths()?;
    let mut dangling: Vec<(String, String)> = Vec::new(); // (id, preview)
    for (id, hash, ext, path) in &rows {
        if !path.exists() {
            dangling.push((
                id.clone(),
                format!("{}.{}", &hash[..8.min(hash.len())], ext),
            ));
        }
    }

    // 2. Orphan files: on disk but no live row points at them.
    let referenced: HashSet<(String, String)> = db.referenced_image_blobs()?.into_iter().collect();
    let files = db.scan_image_files()?;
    let mut orphans: Vec<std::path::PathBuf> = Vec::new();
    for f in &files {
        let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let ext = f.extension().and_then(|s| s.to_str()).unwrap_or("");
        let key = (stem.to_string(), ext.to_string());
        if !referenced.contains(&key) {
            orphans.push(f.clone());
        }
    }

    // 3. (Optional) Hash verification for referenced files.
    let mut mismatched: Vec<(String, String, String, std::path::PathBuf, String)> = Vec::new();
    // Each entry: (id, db_hash, ext, path, actual_hash)
    if fix_hashes {
        for (id, hash, ext, path) in &rows {
            if !path.exists() {
                continue; // dangling, handled above
            }
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("warn: could not read {}: {}", path.display(), e);
                    continue;
                }
            };
            let actual = Clipboard::hash(&bytes);
            if &actual != hash {
                mismatched.push((id.clone(), hash.clone(), ext.clone(), path.clone(), actual));
            }
        }
    }

    println!("{mode}Repair report:");
    println!("  dangling rows:  {}", dangling.len());
    println!("  orphan files:   {}", orphans.len());
    if fix_hashes {
        println!("  hash mismatches:{}", mismatched.len());
    }

    if dry_run {
        for (id, preview) in &dangling {
            println!("  would delete dangling row {} ({})", id, preview);
        }
        for p in &orphans {
            println!("  would remove orphan file {}", p.display());
        }
        for (id, db_hash, _, path, actual) in &mismatched {
            println!(
                "  would quarantine {} (db={}, actual={})",
                path.display(),
                &db_hash[..8.min(db_hash.len())],
                &actual[..8.min(actual.len())],
            );
            let _ = id;
        }
        return Ok(());
    }

    // Apply.
    for (id, _) in &dangling {
        let _ = db.delete_dangling_row(id);
    }
    for p in &orphans {
        if let Err(e) = std::fs::remove_file(p) {
            eprintln!("warn: could not remove {}: {}", p.display(), e);
        }
    }
    if fix_hashes {
        for (_id, db_hash, ext, path, actual) in &mismatched {
            match Database::quarantine_file(path, db_hash, actual, ext) {
                Ok(dest) => println!("  quarantined {} -> {}", path.display(), dest.display()),
                Err(e) => eprintln!("warn: could not quarantine {}: {}", path.display(), e),
            }
        }
    }

    println!(
        "{mode}applied: {} dangling rows deleted, {} orphan files removed{}",
        dangling.len(),
        orphans.len(),
        if fix_hashes {
            format!(", {} files quarantined", mismatched.len())
        } else {
            String::new()
        }
    );

    Ok(())
}

/// Helper to resolve a target (index or ID) to an entry
fn resolve_target(db: &Database, target: &str) -> Result<Option<Entry>> {
    if let Ok(index) = target.parse::<usize>() {
        if index == 0 {
            return Err(DitoxError::NotFound("Index must be 1 or greater".into()));
        }
        db.get_by_index(index - 1)
    } else {
        db.get_by_id(target)
    }
}

/// Helper to resolve a collection target (name or ID)
fn resolve_collection(db: &Database, target: &str) -> Result<Option<Collection>> {
    // First try by ID, then by name
    if let Some(col) = db.get_collection_by_id(target)? {
        return Ok(Some(col));
    }
    db.get_collection_by_name(target)
}

fn cmd_collection(db: &Database, subcmd: CollectionCommands) -> Result<()> {
    match subcmd {
        CollectionCommands::List { json } => cmd_collection_list(db, json),
        CollectionCommands::Create {
            name,
            color,
            keybind,
        } => cmd_collection_create(db, name, color, keybind),
        CollectionCommands::Delete { target } => cmd_collection_delete(db, &target),
        CollectionCommands::Rename { target, new_name } => {
            cmd_collection_rename(db, &target, new_name)
        }
        CollectionCommands::Add { entry, collection } => {
            cmd_collection_add(db, &entry, &collection)
        }
        CollectionCommands::Remove { entry } => cmd_collection_remove(db, &entry),
        CollectionCommands::Show {
            target,
            limit,
            json,
        } => cmd_collection_show(db, &target, limit, json),
    }
}

fn cmd_collection_list(db: &Database, json: bool) -> Result<()> {
    let collections = db.get_all_collections()?;

    if json {
        let json_output = serde_json::to_string_pretty(&collections)
            .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);
    } else {
        if collections.is_empty() {
            println!("No collections found. Create one with: ditox collection create <name>");
            return Ok(());
        }

        println!(
            "{:>3} │ {:<20} │ {:^7} │ {:^3} │ {:>6}",
            "#", "Name", "Color", "Key", "Entries"
        );
        println!("────┼──────────────────────┼─────────┼─────┼────────");

        for (i, col) in collections.iter().enumerate() {
            let entry_count = db.count_entries_in_collection(&col.id)?;
            let color_display = col.color.as_deref().unwrap_or("-");
            let keybind_display = col
                .keybind
                .map(|k| k.to_string())
                .unwrap_or_else(|| "-".to_string());

            println!(
                "{:>3} │ {:<20} │ {:^7} │ {:^3} │ {:>6}",
                i + 1,
                col.name,
                color_display,
                keybind_display,
                entry_count
            );
        }
    }

    Ok(())
}

fn cmd_collection_create(
    db: &Database,
    name: String,
    color: Option<String>,
    keybind: Option<char>,
) -> Result<()> {
    // Check if collection with this name already exists
    if db.get_collection_by_name(&name)?.is_some() {
        return Err(DitoxError::Other(format!(
            "Collection '{}' already exists",
            name
        )));
    }

    // Get position (after last collection)
    let collections = db.get_all_collections()?;
    let position = collections.len() as i32;

    let collection = Collection::with_options(name.clone(), color, keybind, position);
    db.create_collection(&collection)?;

    println!("Created collection: {}", name);
    Ok(())
}

fn cmd_collection_delete(db: &Database, target: &str) -> Result<()> {
    let collection = resolve_collection(db, target)?;

    match collection {
        Some(col) => {
            let name = col.name.clone();
            db.delete_collection(&col.id)?;
            println!("Deleted collection: {}", name);
            Ok(())
        }
        None => Err(DitoxError::NotFound(format!(
            "Collection not found: {}",
            target
        ))),
    }
}

fn cmd_collection_rename(db: &Database, target: &str, new_name: String) -> Result<()> {
    let collection = resolve_collection(db, target)?;

    match collection {
        Some(mut col) => {
            // Check if new name already exists
            if db.get_collection_by_name(&new_name)?.is_some() {
                return Err(DitoxError::Other(format!(
                    "Collection '{}' already exists",
                    new_name
                )));
            }

            let old_name = col.name.clone();
            col.name = new_name.clone();
            db.update_collection(&col)?;
            println!("Renamed collection '{}' to '{}'", old_name, new_name);
            Ok(())
        }
        None => Err(DitoxError::NotFound(format!(
            "Collection not found: {}",
            target
        ))),
    }
}

fn cmd_collection_add(db: &Database, entry_target: &str, collection_target: &str) -> Result<()> {
    let entry = resolve_target(db, entry_target)?;
    let collection = resolve_collection(db, collection_target)?;

    match (entry, collection) {
        (Some(entry), Some(col)) => {
            db.set_entry_collection(&entry.id, Some(&col.id))?;
            println!("Added '{}' to collection '{}'", entry.preview(30), col.name);
            Ok(())
        }
        (None, _) => Err(DitoxError::NotFound(format!(
            "Entry not found: {}",
            entry_target
        ))),
        (_, None) => Err(DitoxError::NotFound(format!(
            "Collection not found: {}",
            collection_target
        ))),
    }
}

fn cmd_collection_remove(db: &Database, entry_target: &str) -> Result<()> {
    let entry = resolve_target(db, entry_target)?;

    match entry {
        Some(entry) => {
            db.set_entry_collection(&entry.id, None)?;
            println!("Removed '{}' from its collection", entry.preview(30));
            Ok(())
        }
        None => Err(DitoxError::NotFound(format!(
            "Entry not found: {}",
            entry_target
        ))),
    }
}

fn cmd_collection_show(db: &Database, target: &str, limit: usize, json: bool) -> Result<()> {
    let collection = resolve_collection(db, target)?;

    match collection {
        Some(col) => {
            let entries = db.get_entries_in_collection(&col.id, limit)?;

            if json {
                let json_output = serde_json::to_string_pretty(&entries)
                    .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
                println!("{}", json_output);
            } else {
                if entries.is_empty() {
                    println!("No entries in collection '{}'", col.name);
                    return Ok(());
                }

                println!("Collection: {}", col.name);
                println!(
                    "{:>3} │ {:^4} │ {:^3} │ {:<40} │ {:>6}",
                    "#", "Type", "Pin", "Content", "Age"
                );
                println!("────┼──────┼─────┼──────────────────────────────────────────┼────────");

                for (i, entry) in entries.iter().enumerate() {
                    println!(
                        "{:>3} │ {:^4} │ {:^3} │ {:<40} │ {:>6}",
                        i + 1,
                        entry.entry_type.short(),
                        if entry.favorite { "⭐" } else { "" },
                        entry.preview(40),
                        entry.relative_time()
                    );
                }
            }
            Ok(())
        }
        None => Err(DitoxError::NotFound(format!(
            "Collection not found: {}",
            target
        ))),
    }
}
