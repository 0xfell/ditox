mod cli;
mod keybindings;
mod ui;

use clap::Parser;
use cli::{Cli, CollectionCommands, Commands, RulesCommands};
use ditox_core::filter::{FilterAction, FilterRule, PatternKind};
use ditox_core::logging;
use ditox_core::{
    Clipboard, Collection, Config, Database, DitoxError, Entry, EntryType, Result, Watcher,
};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as MatcherConfig, Matcher};

fn main() {
    if let Err(e) = run() {
        // Last-resort: tracing may not be initialised on early errors.
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Initialise logging via the shared helper. RUST_LOG honoured.
    logging::init(logging::Mode::Stderr);

    let cli = Cli::parse();
    let config = Config::load()?;
    // Apply [storage].data_dir override (if any) before opening the DB
    // so all path resolution (db path, images dir, watcher PID) lands
    // in the user-chosen location.
    if let Some(override_dir) = config.apply_storage_override()? {
        tracing::info!("data_dir override active: {}", override_dir.display());
        // Soft-warn if the user pointed somewhere new while a legacy
        // default DB still exists — they're effectively starting fresh.
        if Config::legacy_db_exists_outside(&override_dir) {
            tracing::warn!(
                "data_dir override points at {} (no ditox.db there yet) but \
                 a legacy default ditox.db exists. The override starts a \
                 new history; copy or move the legacy DB if you want it.",
                override_dir.display()
            );
        }
    }
    let mut db = Database::open()?;
    db.init_schema()?;

    match cli.command {
        None => run_tui(db, config),
        Some(Commands::Watch {
            stop,
            status,
            json,
            journal: _journal,
        }) => {
            if stop {
                cmd_watch_stop()
            } else if status {
                cmd_watch_status(json)
            } else {
                run_watcher(db, config)
            }
        }
        Some(Commands::List {
            limit,
            json,
            favorites,
        }) => cmd_list(&db, limit, json, favorites),
        Some(Commands::Get { target, json }) => cmd_get(&db, &target, json),
        Some(Commands::Search { query, limit, json }) => cmd_search(&db, &query, limit, json),
        Some(Commands::Copy { target }) => cmd_copy(&db, &target),
        Some(Commands::Save) => cmd_save(&db),
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
        Some(Commands::Open {
            target,
            action,
            print_only,
        }) => cmd_open(&db, &config, &target, &action, print_only),
        Some(Commands::Rules(subcmd)) => cmd_rules(&db, subcmd),
        Some(Commands::Transform {
            list,
            json,
            target,
            transform,
            print_only,
        }) => {
            if list {
                cmd_transform_list(json)
            } else {
                cmd_transform_apply(
                    &db,
                    target
                        .as_deref()
                        .expect("clap requires target unless --list"),
                    transform
                        .as_deref()
                        .expect("clap requires transform unless --list"),
                    print_only,
                )
            }
        }
        Some(Commands::Collection(subcmd)) => cmd_collection(&db, subcmd),
        Some(Commands::Tag { entry, name, color }) => cmd_tag_add(&db, &entry, &name, color),
        Some(Commands::Untag { entry, tag }) => cmd_tag_remove(&db, &entry, &tag),
        Some(Commands::TagList { entry, json }) => cmd_tag_list(&db, entry.as_deref(), json),
    }
}

fn cmd_rules(db: &Database, sub: RulesCommands) -> Result<()> {
    match sub {
        RulesCommands::List { json } => cmd_rules_list(db, json),
        RulesCommands::Add {
            name,
            pattern,
            kind,
            process,
            action,
        } => cmd_rules_add(db, &name, &pattern, &kind, process.as_deref(), &action),
        RulesCommands::Show { target, json } => cmd_rules_show(db, &target, json),
        RulesCommands::Delete { target } => cmd_rules_delete(db, &target),
        RulesCommands::Enable { target } => cmd_rules_set_enabled(db, &target, true),
        RulesCommands::Disable { target } => cmd_rules_set_enabled(db, &target, false),
        RulesCommands::Reorder { target, position } => cmd_rules_reorder(db, &target, position),
    }
}

fn cmd_rules_list(db: &Database, json: bool) -> Result<()> {
    let rules = db.list_filter_rules()?;
    if json {
        let out = serde_json::to_string_pretty(&rules)
            .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", out);
        return Ok(());
    }

    if rules.is_empty() {
        println!("No filter rules configured.");
        return Ok(());
    }

    println!(
        "{:<5} {:<10} {:<6} {:<22} {:<10} {:<14} NAME / PATTERN",
        "POS", "ENABLED", "KIND", "ID", "PROCESS", "ACTION"
    );
    let bar = "─".repeat(100);
    println!("{}", bar);
    for r in &rules {
        let id_short = if r.id.len() > 8 { &r.id[..8] } else { &r.id };
        let process = r.process_glob.as_deref().unwrap_or("-");
        println!(
            "{:<5} {:<10} {:<6} {:<22} {:<10} {:<14} {}",
            r.position,
            if r.enabled { "yes" } else { "no" },
            r.pattern_kind.as_str(),
            id_short,
            process,
            r.action.to_storage(),
            r.name
        );
        println!(
            "{:<5} {:<10} {:<6} {:<22} {:<10} {:<14}   {}",
            "", "", "", "", "", "", r.pattern
        );
    }
    Ok(())
}

fn cmd_rules_add(
    db: &Database,
    name: &str,
    pattern: &str,
    kind_str: &str,
    process: Option<&str>,
    action_str: &str,
) -> Result<()> {
    let kind = match PatternKind::from_str_lossy(kind_str) {
        Some(k) => k,
        None => {
            eprintln!(
                "ditox rules add: unknown --kind '{}'. Valid: regex, glob, contains.",
                kind_str
            );
            std::process::exit(2);
        }
    };
    let action = match FilterAction::from_storage(action_str) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ditox rules add: {}", e);
            std::process::exit(2);
        }
    };

    let position = db.max_filter_rule_position()? + 1;
    let rule = FilterRule::new_now(
        name,
        pattern,
        kind,
        process.map(String::from),
        action,
        position,
    );

    db.add_filter_rule(&rule)?;
    println!(
        "Added rule {} \"{}\" ({}) at position {}",
        &rule.id[..8],
        rule.name,
        rule.action.to_storage(),
        rule.position
    );
    Ok(())
}

fn cmd_rules_show(db: &Database, target: &str, json: bool) -> Result<()> {
    let rule = match db.get_filter_rule(target)? {
        Some(r) => r,
        None => {
            eprintln!("Rule not found: {}", target);
            std::process::exit(1);
        }
    };
    if json {
        let out = serde_json::to_string_pretty(&rule)
            .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", out);
        return Ok(());
    }
    println!("ID:           {}", rule.id);
    println!("Name:         {}", rule.name);
    println!("Pattern:      {}", rule.pattern);
    println!("Pattern kind: {}", rule.pattern_kind.as_str());
    println!(
        "Process glob: {}",
        rule.process_glob.as_deref().unwrap_or("-")
    );
    println!("Action:       {}", rule.action.to_storage());
    println!("Enabled:      {}", rule.enabled);
    println!("Position:     {}", rule.position);
    println!("Created:      {}", rule.created_at);
    Ok(())
}

fn cmd_rules_delete(db: &Database, target: &str) -> Result<()> {
    if db.delete_filter_rule(target)? {
        println!("Deleted rule {}", target);
        Ok(())
    } else {
        eprintln!("Rule not found: {}", target);
        std::process::exit(1);
    }
}

fn cmd_rules_set_enabled(db: &Database, target: &str, enabled: bool) -> Result<()> {
    if db.set_filter_rule_enabled(target, enabled)? {
        println!(
            "Rule {} {}",
            target,
            if enabled { "enabled" } else { "disabled" }
        );
        Ok(())
    } else {
        eprintln!("Rule not found: {}", target);
        std::process::exit(1);
    }
}

fn cmd_rules_reorder(db: &Database, target: &str, position: i64) -> Result<()> {
    if db.set_filter_rule_position(target, position)? {
        println!("Rule {} moved to position {}", target, position);
        Ok(())
    } else {
        eprintln!("Rule not found: {}", target);
        std::process::exit(1);
    }
}

fn cmd_transform_list(json: bool) -> Result<()> {
    use ditox_core::transforms::registry;

    if json {
        let arr: Vec<serde_json::Value> = registry()
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id(),
                    "name": t.name(),
                    "description": t.description(),
                })
            })
            .collect();
        let out = serde_json::to_string_pretty(&arr)
            .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", out);
        return Ok(());
    }

    println!("{:<22} NAME / DESCRIPTION", "ID");
    let bar = "─".repeat(80);
    println!("{}", bar);
    for t in registry() {
        println!("{:<22} {}", t.id(), t.name());
        println!("{:<22}   {}", "", t.description());
    }
    Ok(())
}

fn cmd_transform_apply(
    db: &Database,
    target: &str,
    transform_id: &str,
    print_only: bool,
) -> Result<()> {
    use ditox_core::transforms;

    let entry = match resolve_target(db, target)? {
        Some(e) => e,
        None => {
            eprintln!("Entry not found: {}", target);
            std::process::exit(1);
        }
    };

    if entry.entry_type != EntryType::Text {
        eprintln!(
            "ditox transform: entry {} is not a text entry; transforms operate on text",
            target
        );
        std::process::exit(1);
    }

    let transform = match transforms::get(transform_id) {
        Some(t) => t,
        None => {
            eprintln!(
                "ditox transform: unknown transform '{}'. Run 'ditox transform --list' for available IDs.",
                transform_id
            );
            std::process::exit(2);
        }
    };

    let transformed = transform.apply_text(&entry.content)?;

    if print_only {
        // Print without trailing newline so pipelines see exactly
        // the transformed bytes. The user's terminal will add its
        // own newline at exit.
        print!("{}", transformed);
        return Ok(());
    }

    Clipboard::set_text(&transformed)?;
    println!(
        "Applied {} to entry {} → copied {} bytes",
        transform.id(),
        &entry.id[..8.min(entry.id.len())],
        transformed.len()
    );
    Ok(())
}

fn cmd_open(
    db: &Database,
    config: &Config,
    target: &str,
    action_name: &str,
    print_only: bool,
) -> Result<()> {
    use ditox_core::url_template::{open_in_browser, UrlAction};

    let entry = match resolve_target(db, target)? {
        Some(e) => e,
        None => {
            eprintln!("Entry not found: {}", target);
            std::process::exit(1);
        }
    };

    // Only text entries make sense to open in a URL template;
    // images would emit `<sha256>` as the URL query, which is
    // useless. Be explicit rather than silently doing the wrong
    // thing.
    if entry.entry_type != EntryType::Text {
        eprintln!(
            "ditox open: entry {} is not a text entry; URL templates need a text query",
            target
        );
        std::process::exit(1);
    }

    let action = match UrlAction::from_name(action_name) {
        Some(a) => a,
        None => {
            eprintln!(
                "ditox open: unknown action '{}'. Valid: translate, search (synonyms: tr, trans, web, websearch, web-search).",
                action_name
            );
            std::process::exit(2);
        }
    };

    let url = action.url_for(&config.actions, &entry.content);

    if print_only {
        println!("{}", url);
        return Ok(());
    }

    open_in_browser(&url)?;
    Ok(())
}

fn run_tui(db: Database, config: Config) -> Result<()> {
    ui::run(db, config)
}

fn run_watcher(db: Database, config: Config) -> Result<()> {
    let mut watcher = Watcher::new(db, config);
    watcher.run()
}

fn cmd_watch_stop() -> Result<()> {
    use ditox_core::watcher;
    match watcher::stop_watcher()? {
        true => {
            println!("watcher stop signal sent");
            // Poll for up to 3 seconds for confirmation.
            let start = std::time::Instant::now();
            while start.elapsed() < std::time::Duration::from_secs(3) {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let st = watcher::watcher_status();
                if !st.pid_alive {
                    println!("watcher stopped");
                    return Ok(());
                }
            }
            eprintln!("watcher PID still alive after 3s; you may need SIGKILL manually");
            std::process::exit(2);
        }
        false => {
            println!("no watcher running");
            std::process::exit(1);
        }
    }
}

fn cmd_watch_status(json: bool) -> Result<()> {
    use ditox_core::watcher;
    let st = watcher::watcher_status();
    if json {
        let out = serde_json::to_string_pretty(&st)
            .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", out);
        return Ok(());
    }
    println!("Watcher Status");
    println!("──────────────");
    match st.pid {
        Some(p) => println!("PID:           {}", p),
        None => println!("PID:           (no PID file)"),
    }
    println!("PID alive:     {}", if st.pid_alive { "yes" } else { "no" });
    println!("Lock held:     {}", if st.locked { "yes" } else { "no" });
    match st.last_heartbeat {
        Some(ts) => println!(
            "Heartbeat:     {} ({}s ago)",
            ts,
            st.heartbeat_age_secs.unwrap_or(0)
        ),
        None => println!("Heartbeat:     (none)"),
    }
    println!("Healthy:       {}", if st.healthy { "yes" } else { "no" });
    if !st.healthy && st.pid.is_some() {
        std::process::exit(1);
    }
    Ok(())
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

fn cmd_save(db: &Database) -> Result<()> {
    let text =
        Clipboard::get_text()?.ok_or_else(|| DitoxError::Other("clipboard has no text".into()))?;
    let hash = Clipboard::hash(text.as_bytes());
    if let Some(existing) = db.get_by_hash(&hash)? {
        db.touch(&existing.id)?;
        println!("Saved duplicate: usage bumped for {}", existing.id);
    } else {
        let entry = Entry::new_text(text);
        db.insert(&entry)?;
        println!("Saved: {}", entry.preview(50));
    }
    Ok(())
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
    let platform = ditox_core::platform::detect();

    println!("Ditox Status");
    println!("────────────");
    println!("Entries:     {}", count);
    println!("Data dir:    {}", data_dir.display());
    println!("Images dir:  {}", images_dir.display());
    println!("Platform:    {}", platform.slug());
    println!(
        "  layer-shell:    {}",
        if platform.supports_layer_shell() {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "  wlr-toplevel:   {}",
        if platform.supports_wlr_foreign_toplevel() {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "  global hotkey:  {}",
        if platform.supports_global_hotkey_in_app() {
            "in-app"
        } else {
            "compositor-managed"
        }
    );
    let chain = platform.paste_synthesizer_chain();
    if chain.is_empty() {
        println!("  paste chain:    (none)");
    } else {
        println!("  paste chain:    {}", chain.join(" → "));
    }

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
                    tracing::warn!("could not read {}: {}", path.display(), e);
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
            tracing::warn!("could not remove {}: {}", p.display(), e);
        }
    }
    if fix_hashes {
        for (_id, db_hash, ext, path, actual) in &mismatched {
            match Database::quarantine_file(path, db_hash, actual, ext) {
                Ok(dest) => println!("  quarantined {} -> {}", path.display(), dest.display()),
                Err(e) => tracing::warn!("could not quarantine {}: {}", path.display(), e),
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

/// Helper to resolve a tag target (name or ID).
fn resolve_tag(db: &Database, target: &str) -> Result<Option<ditox_core::Tag>> {
    if let Some(tag) = db.get_tag_by_id(target)? {
        return Ok(Some(tag));
    }
    db.get_tag_by_name(target)
}

fn cmd_tag_add(
    db: &Database,
    entry_target: &str,
    tag_name: &str,
    color: Option<String>,
) -> Result<()> {
    let entry = resolve_target(db, entry_target)?;
    match entry {
        Some(entry) => {
            let tag = db.add_tag_to_entry_by_name(&entry.id, tag_name, color.as_deref())?;
            println!("Tagged '{}' with '{}'", entry.preview(30), tag.name);
            Ok(())
        }
        None => Err(DitoxError::NotFound(format!(
            "Entry not found: {}",
            entry_target
        ))),
    }
}

fn cmd_tag_remove(db: &Database, entry_target: &str, tag_target: &str) -> Result<()> {
    let entry = resolve_target(db, entry_target)?;
    let tag = resolve_tag(db, tag_target)?;

    match (entry, tag) {
        (Some(entry), Some(tag)) => {
            if db.remove_tag_from_entry(&entry.id, &tag.id)? {
                println!("Removed tag '{}' from '{}'", tag.name, entry.preview(30));
                Ok(())
            } else {
                Err(DitoxError::NotFound(format!(
                    "Entry '{}' does not have tag '{}'",
                    entry_target, tag_target
                )))
            }
        }
        (None, _) => Err(DitoxError::NotFound(format!(
            "Entry not found: {}",
            entry_target
        ))),
        (_, None) => Err(DitoxError::NotFound(format!(
            "Tag not found: {}",
            tag_target
        ))),
    }
}

fn cmd_tag_list(db: &Database, entry_target: Option<&str>, json: bool) -> Result<()> {
    let tags = if let Some(target) = entry_target {
        let entry = resolve_target(db, target)?;
        match entry {
            Some(entry) => db.get_tags_for_entry(&entry.id)?,
            None => return Err(DitoxError::NotFound(format!("Entry not found: {}", target))),
        }
    } else {
        db.get_all_tags()?
    };

    if json {
        let out = serde_json::to_string_pretty(&tags)
            .map_err(|e| DitoxError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", out);
        return Ok(());
    }

    if tags.is_empty() {
        println!("No tags found.");
        return Ok(());
    }

    println!("{:<22} {:<20} {:>7}", "ID", "NAME", "ENTRIES");
    let bar = "─".repeat(54);
    println!("{}", bar);
    for tag in tags {
        let entry_count = db.count_entries_with_tag(&tag.id)?;
        let id_short = if tag.id.len() > 8 {
            &tag.id[..8]
        } else {
            &tag.id
        };
        let color = tag.color.as_deref().unwrap_or("-");
        println!(
            "{:<22} {:<20} {:>7}  {}",
            id_short, tag.name, entry_count, color
        );
    }
    Ok(())
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
