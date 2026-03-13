// ABOUTME: CLI binary for fatescroll random table tool.
// ABOUTME: Thin wrapper over the fatescroll-core library using clap.

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
        /// Path to collection directory or manifest file or manifest file
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Automatically fix id field issues
        #[arg(long)]
        fix: bool,
        /// Update stale references after id corrections (requires --fix)
        #[arg(long, requires = "fix")]
        update_refs: bool,
    },
    /// Roll on a table
    Roll {
        /// Path to collection directory or manifest file
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Fully qualified table ID (e.g., "dmg.treasure.gems")
        table_id: String,
    },
    /// Search for tables
    Search {
        /// Path to collection directory or manifest file
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
    /// Display a table's contents
    Show {
        /// Path to collection directory or manifest file
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Fully qualified table ID (e.g., "dmg.treasure.gems")
        table_id: String,
    },
    /// Generate a table YAML template
    Init {
        /// Dice expression (e.g., "1d6", "2d8+1")
        #[arg(long, conflicts_with = "entries")]
        roll: Option<String>,
        /// Number of result entries desired
        #[arg(long, conflicts_with = "roll", requires = "distribution")]
        entries: Option<u32>,
        /// Distribution type: flat or bell
        #[arg(long, requires = "entries")]
        distribution: Option<String>,
        /// Table display name
        #[arg(long, default_value = "Untitled Table")]
        name: String,
        /// Write output to file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Import table files into a collection
    Import {
        /// Path to collection directory or manifest file
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Directory within the collection to import into
        #[arg(long)]
        target_dir: String,
        /// Files to import
        files: Vec<PathBuf>,
    },
}

/// Find all manifest files in a directory.
fn find_manifests(dir: &Path) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    let default = dir.join("manifest.yaml");
    if default.is_file() {
        manifests.push(default);
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if !entry.path().is_file() {
                continue;
            }
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".manifest.yaml") {
                manifests.push(entry.path());
            }
        }
    }
    manifests.sort();
    manifests
}

fn resolve_manifest_in_dir(dir: &Path) -> Result<PathBuf, fatescroll_core::Error> {
    let manifests = find_manifests(dir);
    match manifests.len() {
        0 => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No collection found. Provide --collection or run from a collection directory.",
        )
        .into()),
        1 => Ok(manifests.into_iter().next().unwrap()),
        _ => {
            let names: Vec<String> = manifests
                .iter()
                .filter_map(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .collect();
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Multiple manifests found: {}. Specify one with --collection <path>.",
                    names.join(", ")
                ),
            )
            .into())
        }
    }
}

/// Resolve the manifest path from explicit flag or CWD detection.
///
/// Accepts either a manifest file path or a directory containing manifests.
fn resolve_collection(explicit: Option<PathBuf>) -> Result<PathBuf, fatescroll_core::Error> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path);
        }
        if path.is_dir() {
            return resolve_manifest_in_dir(&path);
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path not found: {}", path.display()),
        )
        .into());
    }

    let cwd = std::env::current_dir()?;
    resolve_manifest_in_dir(&cwd)
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Validate {
            collection,
            fix,
            update_refs,
        } => resolve_collection(collection).and_then(|collection| {
            if fix {
                cmd_fix(&collection, update_refs)
            } else {
                cmd_validate(&collection)
            }
        }),
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
        Commands::Show {
            collection,
            table_id,
        } => resolve_collection(collection).and_then(|collection| cmd_show(&collection, &table_id)),
        Commands::Init {
            roll,
            entries,
            distribution,
            name,
            output,
        } => cmd_init(roll, entries, distribution, &name, output),
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

fn cmd_fix(manifest_path: &Path, update_refs: bool) -> Result<(), fatescroll_core::Error> {
    let ref_handling = if update_refs {
        fatescroll_core::fixer::RefHandling::Update
    } else {
        fatescroll_core::fixer::RefHandling::WarnOnly
    };
    let result = fatescroll_core::fixer::fix_collection(manifest_path, ref_handling)?;

    for action in &result.actions {
        match action {
            fatescroll_core::fixer::FixAction::Added { path, id } => {
                println!("Added id '{id}' to {}", path.display());
            }
            fatescroll_core::fixer::FixAction::Corrected { path, old_id, id } => {
                println!("Corrected id '{old_id}' -> '{id}' in {}", path.display());
            }
            fatescroll_core::fixer::FixAction::Ok { path } => {
                println!("OK: {}", path.display());
            }
            fatescroll_core::fixer::FixAction::UpdatedReference {
                path,
                old_ref,
                new_ref,
            } => {
                println!(
                    "Updated reference '{old_ref}' -> '{new_ref}' in {}",
                    path.display()
                );
            }
        }
    }

    if !result.warnings.is_empty() {
        eprintln!("\nWarnings:");
        for warning in &result.warnings {
            match warning {
                fatescroll_core::fixer::FixWarning::StaleReference {
                    path,
                    reference,
                    suggested,
                } => {
                    eprintln!(
                        "  Warning: stale reference '{}' in {} (should be '{}')",
                        reference,
                        path.display(),
                        suggested,
                    );
                }
            }
        }
        eprintln!("\nUse --update-refs to automatically fix stale references.");
    }

    if !result.errors.is_empty() {
        eprintln!("\nErrors encountered:");
        for err in &result.errors {
            eprintln!("  - {err}");
        }
        return Err(std::io::Error::other(format!(
            "{} file(s) could not be processed",
            result.errors.len()
        ))
        .into());
    }

    println!("\nFix complete.");
    Ok(())
}

fn cmd_validate(collection: &Path) -> Result<(), fatescroll_core::Error> {
    let _registry = fatescroll_core::load_collection(collection)?;
    println!("Collection is valid.");
    Ok(())
}

fn cmd_roll(collection: &Path, table_id: &str) -> Result<(), fatescroll_core::Error> {
    let registry = fatescroll_core::load_collection(collection)?;
    let result = fatescroll_core::roller::roll(&registry, table_id)?;
    print_roll_result(&result, 0);
    Ok(())
}

fn cmd_show(collection: &Path, table_id: &str) -> Result<(), fatescroll_core::Error> {
    let registry = fatescroll_core::load_collection(collection)?;
    let table =
        registry
            .get(table_id)
            .ok_or_else(|| fatescroll_core::error::RollError::TableNotFound {
                id: table_id.to_string(),
            })?;
    print!(
        "{}",
        fatescroll_core::display::format_table(table_id, table)
    );
    Ok(())
}

fn print_roll_result(result: &fatescroll_core::RollResult, indent: usize) {
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
) -> Result<(), fatescroll_core::Error> {
    let registry = fatescroll_core::load_collection(collection)?;

    if tags {
        let all_tags = fatescroll_core::search::collect_tags(&registry);
        if all_tags.is_empty() {
            println!("No tags found.");
        } else {
            for tag in &all_tags {
                println!("{tag}");
            }
        }
        return Ok(());
    }

    let results: Vec<(&str, &fatescroll_core::Table)> = if let Some(name) = name {
        fatescroll_core::search::search_by_name(&registry, name)
    } else if let Some(tag) = tag {
        fatescroll_core::search::search_by_tag(&registry, tag)
    } else if let Some(ns) = namespace {
        fatescroll_core::search::search_by_namespace(&registry, ns)
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

fn cmd_init(
    roll: Option<String>,
    entries: Option<u32>,
    distribution: Option<String>,
    name: &str,
    output: Option<PathBuf>,
) -> Result<(), fatescroll_core::Error> {
    let roll_expr = if let Some(expr) = roll {
        expr
    } else if let Some(count) = entries {
        let dist = match distribution.as_deref() {
            Some("flat") => fatescroll_core::init::Distribution::Flat,
            Some("bell") => fatescroll_core::init::Distribution::Bell,
            Some(other) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown distribution type: '{other}' (expected 'flat' or 'bell')"),
                )
                .into());
            }
            None => unreachable!("clap requires --distribution with --entries"),
        };

        if count == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "entries must be at least 1",
            )
            .into());
        }

        if matches!(dist, fatescroll_core::init::Distribution::Bell) && count < 3 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "bell curves require at least 3 entries (minimum is 2d2)",
            )
            .into());
        }

        match fatescroll_core::init::calculate_distribution(count, dist) {
            fatescroll_core::init::DistributionResult::Exact(expr) => expr,
            fatescroll_core::init::DistributionResult::Suggestions(suggestions) => {
                eprintln!("No exact match for {count} entries with bell curve.");
                let mut current_dice = 0;
                for s in &suggestions {
                    if s.num_dice != current_dice {
                        current_dice = s.num_dice;
                        eprintln!("  With {} dice:", s.num_dice);
                    }
                    eprintln!(
                        "    {} → {} entries (range {}-{})",
                        s.expression, s.entries, s.range_min, s.range_max
                    );
                }
                eprintln!("Use --roll <expression> to generate.");
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "no exact bell curve match",
                )
                .into());
            }
        }
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "specify --roll <expression> or --entries <count> --distribution <type>",
        )
        .into());
    };

    let template = fatescroll_core::init::generate_template(&roll_expr, name)?;

    if let Some(path) = output {
        if path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("output file already exists: {}", path.display()),
            )
            .into());
        }
        std::fs::write(&path, &template)?;
        eprintln!("Wrote template to {}", path.display());
    } else {
        print!("{template}");
    }

    Ok(())
}

fn cmd_import(
    manifest_path: &Path,
    target_dir: &str,
    files: &[PathBuf],
) -> Result<(), fatescroll_core::Error> {
    let collection_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let dest = collection_dir.join(target_dir);
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
    let _registry = fatescroll_core::load_collection(manifest_path)?;
    println!("Collection is valid after import.");
    Ok(())
}
