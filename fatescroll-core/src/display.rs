// ABOUTME: Formats table data for human-readable display output.
// ABOUTME: Renders simple tables as range/text grids and compound tables as sub-table lists.

use std::fmt::Write;

use crate::models::Table;

/// Write a `Notes:` block when notes should be shown and exist.
fn render_notes(out: &mut String, show_notes: bool, notes: &[String]) {
    if show_notes && !notes.is_empty() {
        writeln!(out, "Notes:").unwrap();
        for note in notes {
            writeln!(out, "  - {note}").unwrap();
        }
    }
}

/// Format a table for display. Returns the formatted string.
pub fn format_table(fqid: &str, table: &Table, show_notes: bool) -> String {
    let mut out = String::new();

    match table {
        Table::Simple {
            name,
            tags,
            roll,
            modifier_range,
            notes,
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
            render_notes(&mut out, show_notes, notes);
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
            name,
            tags,
            notes,
            tables,
            ..
        } => {
            writeln!(out, "{name} ({fqid})").unwrap();
            if !tags.is_empty() {
                writeln!(out, "Tags: {}", tags.join(", ")).unwrap();
            }
            render_notes(&mut out, show_notes, notes);
            if show_notes && !notes.is_empty() {
                writeln!(out).unwrap();
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
            notes: vec![],
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
            notes: vec![],
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
        let output = format_table("test.encounters.wilderness-encounter", &table, false);

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
        let output = format_table("test.npc.quick-npc", &table, false);

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
            notes: vec![],
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
        let output = format_table("ns.mishap", &table, false);
        assert!(output.contains("mishap"));
        assert!(output.contains("reroll"));
    }

    #[test]
    fn format_table_no_tags() {
        let table = Table::Simple {
            id: "minimal".into(),
            name: "Minimal".into(),
            tags: vec![],
            notes: vec![],
            roll: "1d4".into(),
            modifier_range: None,
            results: vec![ResultEntry {
                min: 1,
                max: 4,
                text: Some("Something".into()),
                chain: None,
            }],
        };
        let output = format_table("test.minimal", &table, false);

        assert!(output.contains("Minimal (test.minimal)"));
        assert!(!output.contains("Tags:"));
    }

    #[test]
    fn format_table_shows_modifier_range() {
        let table = Table::Simple {
            id: "carousing".into(),
            name: "Carousing".into(),
            tags: vec![],
            notes: vec![],
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
        let output = format_table("ns.carousing", &table, false);
        assert!(output.contains("Modifier: 0 to 6"));
    }

    #[test]
    fn format_table_without_modifier_range_omits_line() {
        let table = Table::Simple {
            id: "plain".into(),
            name: "Plain".into(),
            tags: vec![],
            notes: vec![],
            roll: "1d6".into(),
            modifier_range: None,
            results: vec![ResultEntry {
                min: 1,
                max: 6,
                text: Some("X".into()),
                chain: None,
            }],
        };
        let output = format_table("ns.plain", &table, false);
        assert!(!output.contains("Modifier:"));
    }

    #[test]
    fn format_table_with_negative_entries_aligns() {
        let table = Table::Simple {
            id: "aging".into(),
            name: "Aging".into(),
            tags: vec![],
            notes: vec![],
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
        let output = format_table("ns.aging", &table, false);
        assert!(output.contains("-2--1"));
        assert!(output.contains("Decline"));
        assert!(output.contains("Stable"));
    }

    #[test]
    fn format_table_shows_notes_when_requested() {
        let table = Table::Simple {
            id: "boarding".into(),
            name: "Boarding".into(),
            tags: vec![],
            roll: "2d6".into(),
            modifier_range: None,
            notes: vec![
                "Attacker rolls 2d6 minus defender 2d6".into(),
                "DMs: +2 boarding equipment".into(),
            ],
            results: vec![ResultEntry {
                min: 1,
                max: 12,
                text: Some("Outcome".into()),
                chain: None,
            }],
        };
        let output = format_table("ns.boarding", &table, true);
        assert!(output.contains("Notes:"));
        assert!(output.contains("- Attacker rolls 2d6 minus defender 2d6"));
        assert!(output.contains("- DMs: +2 boarding equipment"));
        // Notes block precedes the results grid.
        assert!(output.find("Notes:").unwrap() < output.find("Outcome").unwrap());
    }

    #[test]
    fn format_table_hides_notes_by_default() {
        let table = Table::Simple {
            id: "boarding".into(),
            name: "Boarding".into(),
            tags: vec![],
            roll: "2d6".into(),
            modifier_range: None,
            notes: vec!["Attacker rolls 2d6 minus defender 2d6".into()],
            results: vec![ResultEntry {
                min: 1,
                max: 12,
                text: Some("Outcome".into()),
                chain: None,
            }],
        };
        let output = format_table("ns.boarding", &table, false);
        assert!(!output.contains("Notes:"));
        assert!(!output.contains("Attacker rolls"));
    }

    #[test]
    fn format_table_without_notes_omits_block_even_when_requested() {
        let table = Table::Simple {
            id: "plain".into(),
            name: "Plain".into(),
            tags: vec![],
            roll: "1d6".into(),
            modifier_range: None,
            notes: vec![],
            results: vec![ResultEntry {
                min: 1,
                max: 6,
                text: Some("X".into()),
                chain: None,
            }],
        };
        let output = format_table("ns.plain", &table, true);
        assert!(!output.contains("Notes:"));
    }

    #[test]
    fn format_compound_table_shows_notes_when_requested() {
        let table = Table::Compound {
            id: "quick-npc".into(),
            name: "Quick NPC".into(),
            tags: vec![],
            notes: vec!["Combine occupation and disposition into one line".into()],
            tables: vec!["npc-occupation".into(), "npc-disposition".into()],
        };
        let output = format_table("ns.quick-npc", &table, true);
        assert!(output.contains("Notes:"));
        assert!(output.contains("- Combine occupation and disposition into one line"));
        // Notes block precedes the Tables: list.
        assert!(output.find("Notes:").unwrap() < output.find("Tables:").unwrap());
    }
}
