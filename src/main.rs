// ABOUTME: CLI binary for fatescroll random table tool.
// ABOUTME: Thin wrapper over the fatescroll library using clap.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
#[command(
    name = "fatescroll",
    version,
    about = "RPG random table manager and roller"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a table collection
    Validate {
        /// Path to collection directory (containing manifest.yaml)
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Automatically fix id field issues
        #[arg(long)]
        fix: bool,
    },
    /// Roll on a table
    Roll {
        /// Path to collection directory
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Fully qualified table ID (e.g., "dmg.treasure.gems")
        table_id: String,
    },
    /// Search for tables
    Search {
        /// Path to collection directory
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Search by table name
        #[arg(long)]
        name: Option<String>,
        /// Search by tag
        #[arg(long)]
        tag: Option<String>,
        /// Search by namespace
        #[arg(long)]
        namespace: Option<String>,
        /// List all unique tags in the collection
        #[arg(long, conflicts_with_all = ["name", "tag", "namespace"])]
        tags: bool,
    },
    /// Import table files into a collection
    Import {
        /// Path to collection directory
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Directory within the collection to import into
        #[arg(long)]
        target_dir: String,
        /// Files to import
        files: Vec<PathBuf>,
    },
}

/// Resolve the collection path from explicit flag or CWD detection.
fn resolve_collection(explicit: Option<PathBuf>) -> Result<PathBuf, fatescroll::Error> {
    if let Some(path) = explicit {
        return Ok(path);
    }

    let cwd = std::env::current_dir()?;
    if cwd.join("manifest.yaml").exists() {
        return Ok(cwd);
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "No collection found. Provide --collection or run from a collection directory.",
    )
    .into())
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Validate { collection, fix } => {
            resolve_collection(collection).and_then(|collection| {
                if fix {
                    cmd_fix(&collection)
                } else {
                    cmd_validate(&collection)
                }
            })
        }
        Commands::Roll {
            collection,
            table_id,
        } => resolve_collection(collection).and_then(|collection| cmd_roll(&collection, &table_id)),
        Commands::Search {
            collection,
            name,
            tag,
            namespace,
            tags,
        } => resolve_collection(collection).and_then(|collection| {
            cmd_search(
                &collection,
                name.as_deref(),
                tag.as_deref(),
                namespace.as_deref(),
                tags,
            )
        }),
        Commands::Import {
            collection,
            target_dir,
            files,
        } => resolve_collection(collection)
            .and_then(|collection| cmd_import(&collection, &target_dir, &files)),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn cmd_fix(collection: &Path) -> Result<(), fatescroll::Error> {
    let manifest_path = collection.join("manifest.yaml");
    let result = fatescroll::fixer::fix_collection(&manifest_path)?;

    for action in &result.actions {
        match action {
            fatescroll::fixer::FixAction::Added { path, id } => {
                println!("Added id '{id}' to {}", path.display());
            }
            fatescroll::fixer::FixAction::Corrected {
                path,
                old_id,
                id,
            } => {
                println!(
                    "Corrected id '{old_id}' -> '{id}' in {}",
                    path.display()
                );
            }
            fatescroll::fixer::FixAction::Ok { path } => {
                println!("OK: {}", path.display());
            }
        }
    }

    if !result.errors.is_empty() {
        eprintln!("\nErrors encountered:");
        for err in &result.errors {
            eprintln!("  - {err}");
        }
        return Err(std::io::Error::other(
            format!("{} file(s) could not be processed", result.errors.len()),
        )
        .into());
    }

    println!("\nFix complete.");
    Ok(())
}

fn cmd_validate(collection: &Path) -> Result<(), fatescroll::Error> {
    let _registry = fatescroll::load_collection(collection)?;
    println!("Collection is valid.");
    Ok(())
}

fn cmd_roll(collection: &Path, table_id: &str) -> Result<(), fatescroll::Error> {
    let registry = fatescroll::load_collection(collection)?;
    let result = fatescroll::roller::roll(&registry, table_id)?;
    print_roll_result(&result, 0);
    Ok(())
}

fn print_roll_result(result: &fatescroll::RollResult, indent: usize) {
    let pad = "  ".repeat(indent);
    match (result.roll, &result.text) {
        (Some(roll), Some(text)) => {
            println!("{pad}{} (rolled {}): {}", result.table_name, roll, text);
        }
        (Some(roll), None) => {
            println!("{pad}{} (rolled {})", result.table_name, roll);
        }
        (None, Some(text)) => {
            println!("{pad}{}: {}", result.table_name, text);
        }
        (None, None) => {
            println!("{pad}{}", result.table_name);
        }
    }
    for child in &result.children {
        print_roll_result(child, indent + 1);
    }
}

fn cmd_search(
    collection: &Path,
    name: Option<&str>,
    tag: Option<&str>,
    namespace: Option<&str>,
    tags: bool,
) -> Result<(), fatescroll::Error> {
    let registry = fatescroll::load_collection(collection)?;

    if tags {
        let all_tags = fatescroll::search::collect_tags(&registry);
        if all_tags.is_empty() {
            println!("No tags found.");
        } else {
            for tag in &all_tags {
                println!("{tag}");
            }
        }
        return Ok(());
    }

    let results: Vec<(&str, &fatescroll::Table)> = if let Some(name) = name {
        fatescroll::search::search_by_name(&registry, name)
    } else if let Some(tag) = tag {
        fatescroll::search::search_by_tag(&registry, tag)
    } else if let Some(ns) = namespace {
        fatescroll::search::search_by_namespace(&registry, ns)
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "specify --name, --tag, --namespace, or --tags",
        )
        .into());
    };

    if results.is_empty() {
        println!("No tables found.");
    } else {
        for (fqid, table) in &results {
            let tags = table.tags();
            if tags.is_empty() {
                println!("  {fqid} — {}", table.name());
            } else {
                println!("  {fqid} — {} [{}]", table.name(), tags.join(", "));
            }
        }
    }
    Ok(())
}

fn cmd_import(
    collection: &Path,
    target_dir: &str,
    files: &[PathBuf],
) -> Result<(), fatescroll::Error> {
    let dest = collection.join(target_dir);
    if !dest.is_dir() {
        std::fs::create_dir_all(&dest)?;
    }

    for file in files {
        let filename = file.file_name().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("no filename in path: {}", file.display()),
            )
        })?;
        std::fs::copy(file, dest.join(filename))?;
        println!("Imported: {}", filename.to_string_lossy());
    }

    println!("Validating collection...");
    let _registry = fatescroll::load_collection(collection)?;
    println!("Collection is valid after import.");
    Ok(())
}
