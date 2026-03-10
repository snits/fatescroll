// ABOUTME: Formats table data for human-readable display output.
// ABOUTME: Renders simple tables as range/text grids and compound tables as sub-table lists.

use std::fmt::Write;

use crate::models::Table;

/// Format a table for display. Returns the formatted string.
pub fn format_table(fqid: &str, table: &Table) -> String {
    let mut out = String::new();

    match table {
        Table::Simple {
            name,
            tags,
            roll,
            results,
            ..
        } => {
            writeln!(out, "{name} ({fqid})").unwrap();
            if !tags.is_empty() {
                writeln!(out, "Tags: {}", tags.join(", ")).unwrap();
            }
            writeln!(out, "Roll: {roll}").unwrap();
            writeln!(out).unwrap();

            // Calculate range column width for alignment
            let range_width = results
                .iter()
                .map(|r| {
                    if r.min == r.max {
                        digit_count(r.min)
                    } else {
                        digit_count(r.min) + 1 + digit_count(r.max)
                    }
                })
                .max()
                .unwrap_or(1);

            for entry in results {
                let range_str = if entry.min == entry.max {
                    format!("{}", entry.min)
                } else {
                    format!("{}-{}", entry.min, entry.max)
                };

                let text = entry.text.as_deref().unwrap_or("");
                let chain_str = match &entry.chain {
                    Some(chains) if !chains.is_empty() => {
                        let refs: Vec<&str> = chains.iter().map(|c| c.table_id()).collect();
                        format!(" → {}", refs.join(", "))
                    }
                    _ => String::new(),
                };

                writeln!(
                    out,
                    "  {:>width$}  {text}{chain_str}",
                    range_str,
                    width = range_width
                )
                .unwrap();
            }
        }
        Table::Compound {
            name, tags, tables, ..
        } => {
            writeln!(out, "{name} ({fqid})").unwrap();
            if !tags.is_empty() {
                writeln!(out, "Tags: {}", tags.join(", ")).unwrap();
            }
            writeln!(out, "Tables:").unwrap();
            for t in tables {
                writeln!(out, "  - {t}").unwrap();
            }
        }
    }

    out
}

fn digit_count(n: u32) -> usize {
    if n == 0 {
        return 1;
    }
    (n as f64).log10().floor() as usize + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChainRef, ResultEntry};

    fn simple_table() -> Table {
        Table::Simple {
            id: "wilderness-encounter".into(),
            name: "Wilderness Encounter".into(),
            tags: vec!["encounter".into(), "wilderness".into()],
            roll: "1d8".into(),
            results: vec![
                ResultEntry {
                    min: 1,
                    max: 3,
                    text: Some("Animal encounter".into()),
                    chain: Some(vec![ChainRef::Simple("animal-type".into())]),
                },
                ResultEntry {
                    min: 4,
                    max: 5,
                    text: Some("Bandit camp".into()),
                    chain: Some(vec![
                        ChainRef::Simple("bandit-strength".into()),
                        ChainRef::Simple("bandit-motivation".into()),
                    ]),
                },
                ResultEntry {
                    min: 6,
                    max: 7,
                    text: Some("Abandoned campsite".into()),
                    chain: None,
                },
                ResultEntry {
                    min: 8,
                    max: 8,
                    text: Some("Merchant".into()),
                    chain: Some(vec![ChainRef::Simple("merchant-goods".into())]),
                },
            ],
        }
    }

    fn compound_table() -> Table {
        Table::Compound {
            id: "quick-npc".into(),
            name: "Quick NPC Generator".into(),
            tags: vec!["npc".into(), "generator".into()],
            tables: vec![
                "npc-occupation".into(),
                "npc-disposition".into(),
                "npc-quirk".into(),
            ],
        }
    }

    #[test]
    fn format_simple_table_output() {
        let table = simple_table();
        let output = format_table("test.encounters.wilderness-encounter", &table);

        assert!(output.contains("Wilderness Encounter (test.encounters.wilderness-encounter)"));
        assert!(output.contains("Tags: encounter, wilderness"));
        assert!(output.contains("Roll: 1d8"));
        // Range collapse: 8-8 should display as just 8
        assert!(!output.contains("8-8"));
        // Chain references with arrow
        assert!(output.contains("→ animal-type"));
        assert!(output.contains("→ bandit-strength, bandit-motivation"));
        // Regular range
        assert!(output.contains("1-3"));
    }

    #[test]
    fn format_compound_table_output() {
        let table = compound_table();
        let output = format_table("test.npc.quick-npc", &table);

        assert!(output.contains("Quick NPC Generator (test.npc.quick-npc)"));
        assert!(output.contains("Tags: npc, generator"));
        assert!(output.contains("Tables:"));
        assert!(output.contains("  - npc-occupation"));
        assert!(output.contains("  - npc-disposition"));
        assert!(output.contains("  - npc-quirk"));
    }

    #[test]
    fn format_table_no_tags() {
        let table = Table::Simple {
            id: "minimal".into(),
            name: "Minimal".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![ResultEntry {
                min: 1,
                max: 4,
                text: Some("Something".into()),
                chain: None,
            }],
        };
        let output = format_table("test.minimal", &table);

        assert!(output.contains("Minimal (test.minimal)"));
        assert!(!output.contains("Tags:"));
    }
}
