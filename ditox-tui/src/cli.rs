use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ditox")]
#[command(author, version, about = "Terminal clipboard manager for Wayland")]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start clipboard watcher daemon, or control the running one.
    Watch {
        /// Stop the running watcher daemon (sends SIGTERM on Unix,
        /// TerminateProcess on Windows).
        #[arg(long, conflicts_with_all = ["status", "journal"])]
        stop: bool,

        /// Print the watcher daemon's status and exit. JSON via
        /// `--status --json`.
        #[arg(long, conflicts_with = "stop")]
        status: bool,

        /// JSON output (only with `--status`).
        #[arg(long, requires = "status")]
        json: bool,

        /// Log to systemd journal instead of stderr (Linux only).
        #[arg(long, conflicts_with_all = ["stop", "status"])]
        journal: bool,
    },

    /// List recent clipboard entries
    List {
        /// Number of entries to show
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Show only favorite entries
        #[arg(long)]
        favorites: bool,
    },

    /// Get full content of entry by index (1-based) or ID
    Get {
        /// Entry index (1-based) or UUID
        target: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Fuzzy search clipboard entries
    Search {
        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Copy entry to clipboard by index (1-based) or ID
    Copy {
        /// Entry index (1-based) or UUID
        target: String,
    },

    /// Delete entry by index (1-based) or ID
    Delete {
        /// Entry index (1-based) or UUID
        target: String,
    },

    /// Toggle favorite status of entry by index (1-based) or ID
    Favorite {
        /// Entry index (1-based) or UUID
        target: String,
    },

    /// Clear clipboard history
    Clear {
        /// Skip confirmation prompt
        #[arg(long)]
        confirm: bool,
    },

    /// Print entry count
    Count,

    /// Show watcher status and statistics
    Status,

    /// Show usage statistics
    Stats {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Reconcile the image store with the database.
    ///
    /// Removes orphan files (on disk but not in DB) and dangling rows
    /// (in DB but blob missing). With `--fix-hashes` also verifies that
    /// each referenced file's SHA-256 matches the DB hash and quarantines
    /// mismatches under `images/.quarantine/` for manual review.
    Repair {
        /// Report what would be done without touching anything.
        #[arg(long)]
        dry_run: bool,

        /// Additionally verify and quarantine hash-mismatched files.
        #[arg(long)]
        fix_hashes: bool,
    },

    /// Open the entry's text content in the default browser via a
    /// configured URL template (Phase 3 sub-task 3.8). Templates live
    /// under `[actions]` in `config.toml`; the placeholder `{q}` is
    /// substituted with the URL-encoded clip text.
    ///
    /// Examples:
    ///
    /// ```sh
    /// ditox open 1 translate     # Opens translate.google.com with entry #1
    /// ditox open <uuid> search   # Opens DuckDuckGo with that entry's text
    /// ditox open 2 -p            # Print the resolved URL instead of opening
    /// ```
    Open {
        /// Entry index (1-based) or UUID.
        target: String,

        /// Action name. Canonical: `translate`, `search`. Synonyms:
        /// `tr`/`trans` for translate; `web`/`websearch`/`web-search`
        /// for search.
        #[arg(default_value = "translate")]
        action: String,

        /// Print the resolved URL to stdout instead of launching the
        /// browser. Useful for piping into a custom browser script
        /// or for sanity-checking templates.
        #[arg(short = 'p', long)]
        print_only: bool,
    },

    /// Apply a transform to an entry's text and copy the result to
    /// the clipboard (Phase 3 sub-task 3.1). Original entry is never
    /// mutated.
    ///
    /// Examples:
    ///
    /// ```sh
    /// ditox transform --list               # List all built-in transforms
    /// ditox transform 1 upper-case         # UPPERCASE entry #1, copy
    /// ditox transform <uuid> slugify       # URL-safe slug, copy
    /// ditox transform 1 kebab-case -p      # Print result instead of copying
    /// ```
    Transform {
        /// List all available transforms with their descriptions and
        /// exit. Mutually exclusive with `target` / `transform`.
        #[arg(long, conflicts_with_all = ["target", "transform"])]
        list: bool,

        /// Output `--list` as JSON.
        #[arg(long, requires = "list")]
        json: bool,

        /// Entry index (1-based) or UUID.
        #[arg(required_unless_present = "list")]
        target: Option<String>,

        /// Transform id (kebab-case, e.g. `upper-case`, `slugify`).
        /// Run `ditox transform --list` for the full set.
        #[arg(required_unless_present = "list")]
        transform: Option<String>,

        /// Print the transformed text to stdout instead of writing
        /// it to the clipboard. Useful for previewing in a pipeline.
        #[arg(short = 'p', long, conflicts_with = "list")]
        print_only: bool,
    },

    /// Manage collections
    #[command(subcommand)]
    Collection(CollectionCommands),
}

#[derive(Subcommand)]
pub enum CollectionCommands {
    /// List all collections
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Create a new collection
    Create {
        /// Collection name
        name: String,

        /// Color (hex code, e.g., "#ff5500")
        #[arg(short, long)]
        color: Option<String>,

        /// Quick access key (1-9)
        #[arg(short, long)]
        keybind: Option<char>,
    },

    /// Delete a collection
    Delete {
        /// Collection name or ID
        target: String,
    },

    /// Rename a collection
    Rename {
        /// Current collection name or ID
        target: String,

        /// New name
        new_name: String,
    },

    /// Add entry to a collection
    Add {
        /// Entry index (1-based) or ID
        entry: String,

        /// Collection name or ID
        collection: String,
    },

    /// Remove entry from its collection
    Remove {
        /// Entry index (1-based) or ID
        entry: String,
    },

    /// Show entries in a collection
    Show {
        /// Collection name or ID
        target: String,

        /// Number of entries to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
