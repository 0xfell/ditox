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
    /// Start clipboard watcher daemon
    Watch,

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
