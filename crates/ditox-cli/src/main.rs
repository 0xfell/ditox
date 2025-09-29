use anyhow::Result;
use clap::{Parser, Subcommand};
use ditox_core::{MemStore, Store, Query};

#[derive(Parser)]
#[command(name = "ditox", version, about = "Ditox clipboard CLI (scaffold)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize local database (placeholder)
    InitDb,
    /// Add a new text entry (or read from STDIN if omitted)
    Add { text: Option<String> },
    /// List recent entries
    List {
        #[arg(long)]
        favorites: bool,
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
    /// Self-check for environment capabilities (placeholder)
    Doctor,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    // For scaffolding, use in-memory store. Will be swapped for SQLite.
    let store = MemStore::new();

    match cli.command {
        Commands::InitDb => {
            store.init()?;
            println!("database initialized (placeholder)");
        }
        Commands::Add { text } => {
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
        Commands::List { favorites, limit, json } => {
            let items = store.list(Query { contains: None, favorites_only: favorites, limit })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                for c in items { println!("{}\t{}\t{}", c.id, if c.is_favorite {"*"} else {" "}, preview(&c.text)); }
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
        Commands::Copy { id } => { println!("copy {} (clipboard integration pending)", id); }
        Commands::Delete { id } => {
            if let Some(id) = id { store.delete(&id)?; println!("deleted {}", id); }
            else { store.clear()?; println!("cleared"); }
        }
        Commands::Doctor => { println!("doctor: running checks (not implemented)"); }
    }

    Ok(())
}

fn preview(s: &str) -> String {
    let s = s.replace('\n', " ");
    const MAX: usize = 60;
    if s.len() > MAX { format!("{}…", &s[..MAX]) } else { s }
}

