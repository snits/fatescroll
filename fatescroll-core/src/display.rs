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
            modifier_range,
            results,
            ..
        } => {
            writeln!(out, "{name} ({fqid})").unwrap();
            if !tags.is_empty() {
                writeln!(out, "Tags: {}", tags.join(", ")).unwrap();
            }
            writeln!(out, "Roll: {roll}").unwrap();
            if let Some(mr) = modifier_range {
                writeln!(out, "Modifier: {} to {}", mr.min, mr.max).unwrap();
            }
            writeln!(out).unwrap();

            // Calculate range column width for alignment
            let range_width = results
                .iter()
                .map(|r| {
                    if r.min == r.max {
                        format!("{}", r.min).len()
                    } else {
                        format!("{}-{}", r.min, r.max).len()
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
                        let refs: Vec<String> = chains.iter().map(|c| c.to_string()).collect();
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
            modifier_range: None,
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
    fn format_table_with_reroll_chain() {
        let table = Table::Simple {
            id: "mishap".into(),
            name: "Wizard Mishap".into(),
            tags: vec![],
            roll: "1d4".into(),
            modifier_range: None,
            results: vec![
                ResultEntry {
                    min: 1,
                    max: 1,
                    text: Some("Roll twice and combine".into()),
                    chain: Some(vec![
                        ChainRef::Modified {
                            table: "mishap".into(),
                            reroll: vec![1],
                        },
                        ChainRef::Modified {
                            table: "mishap".into(),
                            reroll: vec![1],
                        },
                    ]),
                },
                ResultEntry {
                    min: 2,
                    max: 4,
                    text: Some("Normal".into()),
                    chain: None,
                },
            ],
        };
        let output = format_table("ns.mishap", &table);
        assert!(output.contains("mishap"));
        assert!(output.contains("reroll"));
    }

    #[test]
    fn format_table_no_tags() {
        let table = Table::Simple {
            id: "minimal".into(),
            name: "Minimal".into(),
            tags: vec![],
            roll: "1d4".into(),
            modifier_range: None,
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

    #[test]
    fn format_table_shows_modifier_range() {
        let table = Table::Simple {
            id: "carousing".into(),
            name: "Carousing".into(),
            tags: vec![],
            roll: "1d8".into(),
            modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
            results: (1..=14)
                .map(|v| ResultEntry {
                    min: v,
                    max: v,
                    text: Some("E".into()),
                    chain: None,
                })
                .collect(),
        };
        let output = format_table("ns.carousing", &table);
        assert!(output.contains("Modifier: 0 to 6"));
    }

    #[test]
    fn format_table_without_modifier_range_omits_line() {
        let table = Table::Simple {
            id: "plain".into(),
            name: "Plain".into(),
            tags: vec![],
            roll: "1d6".into(),
            modifier_range: None,
            results: vec![ResultEntry {
                min: 1,
                max: 6,
                text: Some("X".into()),
                chain: None,
            }],
        };
        let output = format_table("ns.plain", &table);
        assert!(!output.contains("Modifier:"));
    }

    #[test]
    fn format_table_with_negative_entries_aligns() {
        let table = Table::Simple {
            id: "aging".into(),
            name: "Aging".into(),
            tags: vec![],
            roll: "1d6".into(),
            modifier_range: None,
            results: vec![
                ResultEntry {
                    min: -2,
                    max: -1,
                    text: Some("Decline".into()),
                    chain: None,
                },
                ResultEntry {
                    min: 0,
                    max: 6,
                    text: Some("Stable".into()),
                    chain: None,
                },
            ],
        };
        let output = format_table("ns.aging", &table);
        assert!(output.contains("-2--1"));
        assert!(output.contains("Decline"));
        assert!(output.contains("Stable"));
    }
}
