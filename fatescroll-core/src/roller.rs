// ABOUTME: Roll execution engine for simple and compound tables.
// ABOUTME: Handles dice evaluation, chain resolution, and result text interpolation.

use regex::Regex;
use std::sync::LazyLock;

use crate::error::RollError;
use crate::models::{RollResult, Table};
use crate::registry::Registry;

const MAX_CHAIN_DEPTH: usize = 10;
const MAX_REROLL_ATTEMPTS: usize = 100;

static DICE_INTERPOLATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{([^}]+)\}").unwrap());

/// How the top-level lookup value for a table is determined.
///
/// `Dice` rolls the table's expression (optionally shifting by a modifier);
/// `Direct` skips rolling and looks up a caller-supplied value. Only the
/// top-level call carries the caller's choice — chained children are always
/// rolled (`Dice { modifier: None }`).
#[derive(Clone, Copy)]
enum Lookup {
    Dice { modifier: Option<i32> },
    Direct(i32),
}

pub fn roll(registry: &Registry, table_id: &str) -> Result<RollResult, RollError> {
    roll_with_rng(registry, table_id, &mut diceman::FastRng::new())
}

pub fn roll_with_rng(
    registry: &Registry,
    table_id: &str,
    rng: &mut impl diceman::Rng,
) -> Result<RollResult, RollError> {
    roll_recursive(
        registry,
        table_id,
        "",
        rng,
        0,
        &[],
        Lookup::Dice { modifier: None },
    )
}

pub fn roll_with_modifier(
    registry: &Registry,
    table_id: &str,
    modifier: Option<i32>,
) -> Result<RollResult, RollError> {
    roll_with_rng_modifier(registry, table_id, modifier, &mut diceman::FastRng::new())
}

pub fn roll_with_rng_modifier(
    registry: &Registry,
    table_id: &str,
    modifier: Option<i32>,
    rng: &mut impl diceman::Rng,
) -> Result<RollResult, RollError> {
    roll_recursive(
        registry,
        table_id,
        "",
        rng,
        0,
        &[],
        Lookup::Dice { modifier },
    )
}

pub fn roll_with_value(
    registry: &Registry,
    table_id: &str,
    value: i32,
) -> Result<RollResult, RollError> {
    roll_with_rng_value(registry, table_id, value, &mut diceman::FastRng::new())
}

pub fn roll_with_rng_value(
    registry: &Registry,
    table_id: &str,
    value: i32,
    rng: &mut impl diceman::Rng,
) -> Result<RollResult, RollError> {
    roll_recursive(registry, table_id, "", rng, 0, &[], Lookup::Direct(value))
}

fn roll_recursive(
    registry: &Registry,
    table_id: &str,
    current_namespace: &str,
    rng: &mut impl diceman::Rng,
    depth: usize,
    reroll_values: &[u32],
    lookup: Lookup,
) -> Result<RollResult, RollError> {
    if depth > MAX_CHAIN_DEPTH {
        return Err(RollError::ChainDepthExceeded {
            table: table_id.to_string(),
            limit: MAX_CHAIN_DEPTH,
        });
    }

    // Resolve the table: try as FQID first, then resolve with namespace
    let (resolved_fqid, table) = if let Some(t) = registry.get(table_id) {
        (table_id, t)
    } else if !current_namespace.is_empty() {
        registry
            .resolve(table_id, current_namespace)
            .ok_or_else(|| RollError::TableNotFound {
                id: table_id.to_string(),
            })?
    } else {
        return Err(RollError::TableNotFound {
            id: table_id.to_string(),
        });
    };

    // Extract namespace from resolved FQID for child resolution
    let namespace = resolved_fqid
        .rsplit_once('.')
        .map(|(ns, _)| ns)
        .unwrap_or("");

    match table {
        Table::Simple {
            name,
            roll: roll_expr,
            results,
            modifier_range,
            ..
        } => {
            let (lookup_value, entry) = match lookup {
                // Direct lookup: skip the dice roll entirely and find the entry
                // covering the caller-supplied value. No clamping — a value outside
                // the entries is an out-of-range error, not silently pinned to the
                // nearest entry (that distinguishes --value from --modifier).
                Lookup::Direct(value) => {
                    let entry = results
                        .iter()
                        .find(|e| value >= e.min && value <= e.max)
                        .ok_or_else(|| RollError::RollOutOfRange {
                            table: name.clone(),
                            value: value as i64,
                        })?;
                    (value, entry.clone())
                }
                Lookup::Dice { modifier } => {
                    if modifier.is_some() && modifier_range.is_none() {
                        return Err(RollError::ModifierNotSupported {
                            table: name.clone(),
                        });
                    }

                    let mut attempts = 0;
                    loop {
                        let dice_result = diceman::roll_with_rng(roll_expr, rng).map_err(|e| {
                            RollError::DiceEvaluation {
                                table: name.clone(),
                                expr: roll_expr.clone(),
                                reason: e.to_string(),
                            }
                        })?;

                        let roll_value = dice_result.outcome.as_numeric().ok_or_else(|| {
                            RollError::DiceEvaluation {
                                table: name.clone(),
                                expr: roll_expr.clone(),
                                reason: "dice notation produced a non-numeric outcome".to_string(),
                            }
                        })?;
                        if roll_value < 0 {
                            return Err(RollError::NegativeRoll { value: roll_value });
                        }
                        let roll_i32 = match checked_total_to_i32(roll_value) {
                            Some(v) => v,
                            // A total outside i32 range matches no entry; reuse
                            // RollOutOfRange (its value field is i64) rather than a
                            // dedicated variant.
                            None => {
                                return Err(RollError::RollOutOfRange {
                                    table: name.clone(),
                                    value: roll_value,
                                });
                            }
                        };

                        let lookup = match modifier_range {
                            Some(_) => match (
                                results.iter().map(|e| e.min).min(),
                                results.iter().map(|e| e.max).max(),
                            ) {
                                (Some(entry_min), Some(entry_max)) => ((roll_i32 as i64)
                                    + (modifier.unwrap_or(0) as i64))
                                    .clamp(entry_min as i64, entry_max as i64)
                                    as i32,
                                // Empty results: unreachable for validated tables; falls through to RollOutOfRange below (roller is public; don't .unwrap() the min/max).
                                _ => roll_i32,
                            },
                            None => roll_i32,
                        };

                        let entry = results
                            .iter()
                            .find(|e| lookup >= e.min && lookup <= e.max)
                            .ok_or_else(|| RollError::RollOutOfRange {
                                table: name.clone(),
                                value: lookup as i64,
                            })?;

                        // Reroll matches on the raw die, not the modified `lookup`:
                        // reroll_values come from a parent chain ref and target a child
                        // table's own rolls, while a modifier applies only at the top
                        // level (children are rolled with modifier None). The two never
                        // coexist, so `roll_i32 == lookup` whenever reroll_values is
                        // non-empty. The `as u32` cast is safe — the NegativeRoll guard
                        // above guarantees `roll_i32 >= 0`.
                        if reroll_values.contains(&(roll_i32 as u32)) {
                            attempts += 1;
                            if attempts >= MAX_REROLL_ATTEMPTS {
                                return Err(RollError::RerollExhausted {
                                    table: name.clone(),
                                    attempts,
                                    reroll_values: reroll_values.to_vec(),
                                });
                            }
                            continue;
                        }

                        break (lookup, entry.clone());
                    }
                }
            };

            let text = entry.text.as_ref().map(|t| interpolate_dice(t, rng));

            let mut children = Vec::new();
            if let Some(chains) = &entry.chain {
                for chain_ref in chains {
                    let child = roll_recursive(
                        registry,
                        chain_ref.table_id(),
                        namespace,
                        rng,
                        depth + 1,
                        chain_ref.reroll_values(),
                        Lookup::Dice { modifier: None },
                    )?;
                    children.push(child);
                }
            }

            Ok(RollResult {
                table_name: name.clone(),
                roll: Some(lookup_value),
                text,
                children,
            })
        }
        Table::Compound {
            name,
            tables: sub_tables,
            ..
        } => {
            match lookup {
                Lookup::Direct(_) => {
                    return Err(RollError::DirectValueNotSupported {
                        table: name.clone(),
                    });
                }
                Lookup::Dice { modifier: Some(_) } => {
                    return Err(RollError::ModifierNotSupported {
                        table: name.clone(),
                    });
                }
                Lookup::Dice { modifier: None } => {}
            }

            let mut children = Vec::new();
            for table_ref in sub_tables {
                let child = roll_recursive(
                    registry,
                    table_ref,
                    namespace,
                    rng,
                    depth + 1,
                    &[],
                    Lookup::Dice { modifier: None },
                )?;
                children.push(child);
            }

            Ok(RollResult {
                table_name: name.clone(),
                roll: None,
                text: None,
                children,
            })
        }
    }
}

/// Narrow an i64 dice total to i32, returning `None` when it falls outside the
/// i32 range. Entries and lookups are i32, so an out-of-range total matches no
/// entry and is reported as `RollOutOfRange` by the caller.
fn checked_total_to_i32(total: i64) -> Option<i32> {
    i32::try_from(total).ok()
}

fn interpolate_dice(text: &str, rng: &mut impl diceman::Rng) -> String {
    DICE_INTERPOLATION
        .replace_all(text, |caps: &regex::Captures| {
            let expr = &caps[1];
            match diceman::roll_with_rng(expr, rng) {
                Ok(result) => match result.outcome.as_numeric() {
                    Some(v) => v.to_string(),
                    None => caps[0].to_string(),
                },
                Err(_) => caps[0].to_string(),
            }
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChainRef, ResultEntry, Table};
    use crate::registry::Registry;

    fn build_test_registry() -> Registry {
        let mut reg = Registry::new();

        reg.register(
            "test.simple".into(),
            Table::Simple {
                id: "simple".into(),
                name: "Simple Test".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d6".into(),
                modifier_range: None,
                results: vec![
                    ResultEntry {
                        min: 1,
                        max: 3,
                        text: Some("Low".into()),
                        chain: None,
                    },
                    ResultEntry {
                        min: 4,
                        max: 6,
                        text: Some("High".into()),
                        chain: None,
                    },
                ],
            },
        )
        .unwrap();

        reg.register(
            "test.chained".into(),
            Table::Simple {
                id: "chained".into(),
                name: "Chained".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![
                    ResultEntry {
                        min: 1,
                        max: 2,
                        text: Some("Follow up".into()),
                        chain: Some(vec![ChainRef::Simple("simple".into())]),
                    },
                    ResultEntry {
                        min: 3,
                        max: 4,
                        text: Some("No chain".into()),
                        chain: None,
                    },
                ],
            },
        )
        .unwrap();

        reg.register(
            "test.compound".into(),
            Table::Compound {
                id: "compound".into(),
                name: "Compound Test".into(),
                tags: vec![],
                notes: vec![],
                tables: vec!["simple".into()],
            },
        )
        .unwrap();

        reg.register(
            "test.interpolated".into(),
            Table::Simple {
                id: "interpolated".into(),
                name: "Interpolated".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Found {2d6} gold coins".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();

        reg
    }

    #[test]
    fn roll_simple_table() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "test.simple", &mut rng).unwrap();
        assert_eq!(result.table_name, "Simple Test");
        assert!(result.roll.is_some());
        assert!(result.text.is_some());
        assert!(result.children.is_empty());
    }

    #[test]
    fn roll_not_found() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(42);
        let err = roll_with_rng(&reg, "nonexistent", &mut rng).unwrap_err();
        assert!(matches!(err, RollError::TableNotFound { .. }));
    }

    #[test]
    fn roll_compound_table() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "test.compound", &mut rng).unwrap();
        assert_eq!(result.table_name, "Compound Test");
        assert!(result.roll.is_none());
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].table_name, "Simple Test");
    }

    #[test]
    fn roll_with_chain_triggered() {
        // Use a table where every result chains, so we always exercise chain resolution
        let mut reg = Registry::new();
        reg.register(
            "ns.parent".into(),
            Table::Simple {
                id: "parent".into(),
                name: "Parent".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Always chains".into()),
                    chain: Some(vec![ChainRef::Simple("child".into())]),
                }],
            },
        )
        .unwrap();
        reg.register(
            "ns.child".into(),
            Table::Simple {
                id: "child".into(),
                name: "Child".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d6".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 6,
                    text: Some("Child result".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "ns.parent", &mut rng).unwrap();
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].table_name, "Child");
        assert!(result.children[0].text.is_some());
    }

    #[test]
    fn roll_interpolates_dice_in_text() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "test.interpolated", &mut rng).unwrap();
        let text = result.text.as_ref().unwrap();
        assert!(!text.contains('{'));
        assert!(text.starts_with("Found "));
        assert!(text.ends_with(" gold coins"));
    }

    #[test]
    fn chain_depth_limit() {
        let mut reg = Registry::new();
        reg.register(
            "loop.a".into(),
            Table::Simple {
                id: "a".into(),
                name: "A".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Loop".into()),
                    chain: Some(vec![ChainRef::Simple("b".into())]),
                }],
            },
        )
        .unwrap();
        reg.register(
            "loop.b".into(),
            Table::Simple {
                id: "b".into(),
                name: "B".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Loop".into()),
                    chain: Some(vec![ChainRef::Simple("a".into())]),
                }],
            },
        )
        .unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let err = roll_with_rng(&reg, "loop.a", &mut rng).unwrap_err();
        assert!(matches!(err, RollError::ChainDepthExceeded { .. }));
    }

    #[test]
    fn roll_negative_result_errors() {
        let mut reg = Registry::new();
        reg.register(
            "test.negative".into(),
            Table::Simple {
                id: "negative".into(),
                name: "Negative".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4-10".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Unreachable".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let err = roll_with_rng(&reg, "test.negative", &mut rng).unwrap_err();
        assert!(matches!(err, RollError::NegativeRoll { value } if value < 0));
    }

    #[test]
    fn roll_out_of_range_errors() {
        let mut reg = Registry::new();
        reg.register(
            "test.gap".into(),
            Table::Simple {
                id: "gap".into(),
                name: "Gap".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d6".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 100,
                    max: 100,
                    text: Some("Unreachable".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let err = roll_with_rng(&reg, "test.gap", &mut rng).unwrap_err();
        assert!(matches!(err, RollError::RollOutOfRange { .. }));
    }

    #[test]
    fn roll_invalid_dice_expression_errors() {
        let mut reg = Registry::new();
        reg.register(
            "test.baddice".into(),
            Table::Simple {
                id: "baddice".into(),
                name: "BadDice".into(),
                tags: vec![],
                notes: vec![],
                roll: "not_a_dice_expr".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 6,
                    text: Some("Unreachable".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let err = roll_with_rng(&reg, "test.baddice", &mut rng).unwrap_err();
        assert!(matches!(err, RollError::DiceEvaluation { .. }));
    }

    #[test]
    fn interpolate_dice_preserves_non_dice_braces() {
        let mut reg = Registry::new();
        reg.register(
            "test.fallback".into(),
            Table::Simple {
                id: "fallback".into(),
                name: "Fallback".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Hello {world} and {2d6} gold".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "test.fallback", &mut rng).unwrap();
        let text = result.text.unwrap();
        // Non-dice expression preserved as-is
        assert!(text.contains("{world}"));
        // Dice expression replaced with a number
        assert!(!text.contains("{2d6}"));
        assert!(text.starts_with("Hello {world} and "));
        assert!(text.ends_with(" gold"));
    }

    #[test]
    fn roll_entry_with_no_text() {
        let mut reg = Registry::new();
        reg.register(
            "test.notext".into(),
            Table::Simple {
                id: "notext".into(),
                name: "NoText".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: None,
                    chain: None,
                }],
            },
        )
        .unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "test.notext", &mut rng).unwrap();
        assert!(result.text.is_none());
    }

    #[test]
    fn roll_with_reroll_avoids_excluded_values() {
        let mut reg = Registry::new();
        reg.register(
            "ns.reroll-parent".into(),
            Table::Simple {
                id: "reroll-parent".into(),
                name: "Reroll Parent".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![
                    ResultEntry {
                        min: 1,
                        max: 1,
                        text: Some("Chains with reroll".into()),
                        chain: Some(vec![ChainRef::Modified {
                            table: "reroll-target".into(),
                            reroll: vec![1],
                        }]),
                    },
                    ResultEntry {
                        min: 2,
                        max: 4,
                        text: Some("Normal".into()),
                        chain: None,
                    },
                ],
            },
        )
        .unwrap();
        reg.register(
            "ns.reroll-target".into(),
            Table::Simple {
                id: "reroll-target".into(),
                name: "Reroll Target".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![
                    ResultEntry {
                        min: 1,
                        max: 1,
                        text: Some("Should be skipped".into()),
                        chain: None,
                    },
                    ResultEntry {
                        min: 2,
                        max: 4,
                        text: Some("Valid result".into()),
                        chain: None,
                    },
                ],
            },
        )
        .unwrap();

        for seed in 0..100 {
            let mut rng = diceman::FastRng::with_seed(seed);
            let result = roll_with_rng(&reg, "ns.reroll-parent", &mut rng).unwrap();
            if !result.children.is_empty() {
                assert_ne!(
                    result.children[0].roll.unwrap(),
                    1,
                    "seed {seed}: reroll should have prevented value 1"
                );
            }
        }
    }

    #[test]
    fn roll_entry_with_multiple_chains() {
        let mut reg = Registry::new();
        reg.register(
            "ns.multi".into(),
            Table::Simple {
                id: "multi".into(),
                name: "MultiChain".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Branches".into()),
                    chain: Some(vec![
                        ChainRef::Simple("child_a".into()),
                        ChainRef::Simple("child_b".into()),
                    ]),
                }],
            },
        )
        .unwrap();
        reg.register(
            "ns.child_a".into(),
            Table::Simple {
                id: "child_a".into(),
                name: "Child A".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d6".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 6,
                    text: Some("Result A".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();
        reg.register(
            "ns.child_b".into(),
            Table::Simple {
                id: "child_b".into(),
                name: "Child B".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d6".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 6,
                    text: Some("Result B".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "ns.multi", &mut rng).unwrap();
        assert_eq!(result.children.len(), 2);
        assert_eq!(result.children[0].table_name, "Child A");
        assert_eq!(result.children[1].table_name, "Child B");
    }

    #[test]
    fn reroll_exhaustion_returns_error() {
        let mut reg = Registry::new();
        reg.register(
            "ns.exhaust-parent".into(),
            Table::Simple {
                id: "exhaust-parent".into(),
                name: "Exhaust Parent".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Always chains".into()),
                    chain: Some(vec![ChainRef::Modified {
                        table: "exhaust-target".into(),
                        reroll: vec![1, 2, 3, 4],
                    }]),
                }],
            },
        )
        .unwrap();
        reg.register(
            "ns.exhaust-target".into(),
            Table::Simple {
                id: "exhaust-target".into(),
                name: "Exhaust Target".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d4".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Unreachable".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let err = roll_with_rng(&reg, "ns.exhaust-parent", &mut rng).unwrap_err();
        assert!(matches!(err, RollError::RerollExhausted { .. }));
    }

    fn carousing_registry() -> Registry {
        let mut reg = Registry::new();
        let results = (1..=14)
            .map(|v| ResultEntry {
                min: v,
                max: v,
                text: Some(format!("E{v}")),
                chain: None,
            })
            .collect();
        reg.register(
            "ns.carousing".into(),
            Table::Simple {
                id: "carousing".into(),
                name: "Carousing".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d8".into(),
                modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
                results,
            },
        )
        .unwrap();
        reg
    }

    #[test]
    fn modifier_shifts_lookup_value() {
        let reg = carousing_registry();
        let mut rng = diceman::FastRng::with_seed(7);
        let raw = roll_with_rng_modifier(&reg, "ns.carousing", None, &mut rng)
            .unwrap()
            .roll
            .unwrap();
        let mut rng2 = diceman::FastRng::with_seed(7);
        let modded = roll_with_rng_modifier(&reg, "ns.carousing", Some(3), &mut rng2)
            .unwrap()
            .roll
            .unwrap();
        assert_eq!(modded, raw + 3);
    }

    #[test]
    fn modifier_clamps_overflow_high() {
        let reg = carousing_registry();
        for seed in 0..50 {
            let mut rng = diceman::FastRng::with_seed(seed);
            let r = roll_with_rng_modifier(&reg, "ns.carousing", Some(100), &mut rng).unwrap();
            assert_eq!(r.roll.unwrap(), 14);
        }
    }

    #[test]
    fn modifier_clamps_overflow_low() {
        let reg = carousing_registry();
        for seed in 0..50 {
            let mut rng = diceman::FastRng::with_seed(seed);
            let r = roll_with_rng_modifier(&reg, "ns.carousing", Some(-100), &mut rng).unwrap();
            assert_eq!(r.roll.unwrap(), 1);
        }
    }

    #[test]
    fn modifier_on_table_without_modifier_range_errors() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(1);
        let err = roll_with_rng_modifier(&reg, "test.simple", Some(2), &mut rng).unwrap_err();
        assert!(matches!(err, RollError::ModifierNotSupported { .. }));
    }

    #[test]
    fn modifier_on_compound_errors() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(1);
        let err = roll_with_rng_modifier(&reg, "test.compound", Some(1), &mut rng).unwrap_err();
        assert!(matches!(err, RollError::ModifierNotSupported { .. }));
    }

    #[test]
    fn modifier_does_not_leak_to_children() {
        // Every parent entry chains, so the child is always rolled. If the parent's
        // modifier leaked into the child (which declares no modifier_range), the
        // child roll would error ModifierNotSupported and unwrap() would panic.
        let mut reg = Registry::new();
        let parent_results = (1..=14)
            .map(|v| ResultEntry {
                min: v,
                max: v,
                text: Some("P".into()),
                chain: Some(vec![ChainRef::Simple("child".into())]),
            })
            .collect();
        reg.register(
            "ns.parent".into(),
            Table::Simple {
                id: "parent".into(),
                name: "Parent".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d8".into(),
                modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
                results: parent_results,
            },
        )
        .unwrap();
        reg.register(
            "ns.child".into(),
            Table::Simple {
                id: "child".into(),
                name: "Child".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d6".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 6,
                    text: Some("C".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();
        for seed in 0..100 {
            let mut rng = diceman::FastRng::with_seed(seed);
            let r = roll_with_rng_modifier(&reg, "ns.parent", Some(6), &mut rng).unwrap();
            assert_eq!(
                r.children.len(),
                1,
                "seed {seed}: child should always be rolled"
            );
            let child = &r.children[0];
            assert!(child.roll.unwrap() >= 1 && child.roll.unwrap() <= 6);
        }
    }

    #[test]
    fn validated_modifier_table_never_out_of_range() {
        let reg = carousing_registry();
        for seed in 0..500 {
            for m in [-100, -6, 0, 6, 100] {
                let mut rng = diceman::FastRng::with_seed(seed);
                assert!(roll_with_rng_modifier(&reg, "ns.carousing", Some(m), &mut rng).is_ok());
            }
        }
    }

    #[test]
    fn extreme_modifier_clamps_without_overflow() {
        let reg = carousing_registry();
        let mut rng = diceman::FastRng::with_seed(3);
        assert_eq!(
            roll_with_rng_modifier(&reg, "ns.carousing", Some(i32::MAX), &mut rng)
                .unwrap()
                .roll
                .unwrap(),
            14
        );
        let mut rng = diceman::FastRng::with_seed(3);
        assert_eq!(
            roll_with_rng_modifier(&reg, "ns.carousing", Some(i32::MIN), &mut rng)
                .unwrap()
                .roll
                .unwrap(),
            1
        );
    }

    #[test]
    fn checked_total_to_i32_in_range() {
        assert_eq!(checked_total_to_i32(0), Some(0));
        assert_eq!(checked_total_to_i32(i32::MAX as i64), Some(i32::MAX));
        assert_eq!(checked_total_to_i32(i32::MIN as i64), Some(i32::MIN));
    }

    #[test]
    fn checked_total_to_i32_out_of_range() {
        assert_eq!(checked_total_to_i32(i32::MAX as i64 + 1), None);
        assert_eq!(checked_total_to_i32(i64::MAX), None);
        assert_eq!(checked_total_to_i32(i32::MIN as i64 - 1), None);
        assert_eq!(checked_total_to_i32(i64::MIN), None);
    }

    #[test]
    fn value_direct_lookup_skips_dice() {
        // A direct value selects the matching entry regardless of the RNG, so the
        // same value yields the same entry across seeds (dice never consulted).
        let reg = build_test_registry();
        for seed in 0..50 {
            let mut rng = diceman::FastRng::with_seed(seed);
            let result = roll_with_rng_value(&reg, "test.simple", 5, &mut rng).unwrap();
            assert_eq!(result.roll, Some(5));
            assert_eq!(result.text.as_deref(), Some("High"));
        }
    }

    #[test]
    fn value_out_of_range_errors() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(1);
        // test.simple covers 1..=6; 7 is outside every entry.
        let err = roll_with_rng_value(&reg, "test.simple", 7, &mut rng).unwrap_err();
        assert!(matches!(err, RollError::RollOutOfRange { value, .. } if value == 7));
    }

    #[test]
    fn value_does_not_clamp() {
        // The defining difference from --modifier: a value past the entry envelope
        // errors out, it is NOT pinned to the nearest entry. carousing covers 1..=14.
        let reg = carousing_registry();
        let mut rng = diceman::FastRng::with_seed(1);
        let err = roll_with_rng_value(&reg, "ns.carousing", 100, &mut rng).unwrap_err();
        assert!(matches!(err, RollError::RollOutOfRange { value, .. } if value == 100));
    }

    #[test]
    fn value_negative_lookup() {
        // Direct lookup ignores how a table validated, so an in-memory table with
        // negative entries (the Traveller boarding-action shape) resolves directly.
        let mut reg = Registry::new();
        let results = (-7..=7)
            .map(|v| ResultEntry {
                min: v,
                max: v,
                text: Some(format!("Outcome {v}")),
                chain: None,
            })
            .collect();
        reg.register(
            "ns.boarding".into(),
            Table::Simple {
                id: "boarding".into(),
                name: "Boarding Action".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d8".into(),
                modifier_range: None,
                results,
            },
        )
        .unwrap();
        let mut rng = diceman::FastRng::with_seed(1);
        let result = roll_with_rng_value(&reg, "ns.boarding", -7, &mut rng).unwrap();
        assert_eq!(result.roll, Some(-7));
        assert_eq!(result.text.as_deref(), Some("Outcome -7"));
    }

    #[test]
    fn value_resolves_chains() {
        // test.chained entry 1 chains to test.simple; --value must follow chains.
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(1);
        let result = roll_with_rng_value(&reg, "test.chained", 1, &mut rng).unwrap();
        assert_eq!(result.roll, Some(1));
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].table_name, "Simple Test");
    }

    #[test]
    fn value_interpolates_text() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(1);
        let result = roll_with_rng_value(&reg, "test.interpolated", 2, &mut rng).unwrap();
        let text = result.text.as_ref().unwrap();
        assert!(!text.contains('{'));
        assert!(text.starts_with("Found "));
        assert!(text.ends_with(" gold coins"));
    }

    #[test]
    fn value_on_compound_errors() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(1);
        let err = roll_with_rng_value(&reg, "test.compound", 1, &mut rng).unwrap_err();
        assert!(matches!(err, RollError::DirectValueNotSupported { .. }));
    }

    #[test]
    fn value_does_not_leak_to_children() {
        // Every parent entry chains to a child whose entries (1..=6) do not cover
        // the parent value (10). If Direct(10) leaked into the child it would roll
        // out of range; instead the child is rolled normally and stays in 1..=6.
        let mut reg = Registry::new();
        let parent_results = (1..=14)
            .map(|v| ResultEntry {
                min: v,
                max: v,
                text: Some("P".into()),
                chain: Some(vec![ChainRef::Simple("child".into())]),
            })
            .collect();
        reg.register(
            "ns.parent".into(),
            Table::Simple {
                id: "parent".into(),
                name: "Parent".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d14".into(),
                modifier_range: None,
                results: parent_results,
            },
        )
        .unwrap();
        reg.register(
            "ns.child".into(),
            Table::Simple {
                id: "child".into(),
                name: "Child".into(),
                tags: vec![],
                notes: vec![],
                roll: "1d6".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 6,
                    text: Some("C".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();
        for seed in 0..100 {
            let mut rng = diceman::FastRng::with_seed(seed);
            let r = roll_with_rng_value(&reg, "ns.parent", 10, &mut rng).unwrap();
            assert_eq!(r.roll, Some(10));
            assert_eq!(r.children.len(), 1);
            let child_roll = r.children[0].roll.unwrap();
            assert!(
                (1..=6).contains(&child_roll),
                "seed {seed}: child rolled {child_roll}, value must not leak"
            );
        }
    }

    #[test]
    fn roller_ignores_notes() {
        let mut reg = Registry::new();
        reg.register(
            "ns.noted".into(),
            Table::Simple {
                id: "noted".into(),
                name: "Noted".into(),
                tags: vec![],
                notes: vec!["This note must not affect rolling".into()],
                roll: "1d6".into(),
                modifier_range: None,
                results: vec![ResultEntry {
                    min: 1,
                    max: 6,
                    text: Some("Only outcome".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "ns.noted", &mut rng).unwrap();
        assert_eq!(result.table_name, "Noted");
        assert_eq!(result.text.as_deref(), Some("Only outcome"));
    }

    #[test]
    fn self_referential_chain_with_reroll() {
        let mut reg = Registry::new();
        reg.register(
            "ns.mishap".into(),
            Table::Simple {
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
                        max: 2,
                        text: Some("Hands glow blue".into()),
                        chain: None,
                    },
                    ResultEntry {
                        min: 3,
                        max: 3,
                        text: Some("Lose sense of smell".into()),
                        chain: None,
                    },
                    ResultEntry {
                        min: 4,
                        max: 4,
                        text: Some("Hair turns white".into()),
                        chain: None,
                    },
                ],
            },
        )
        .unwrap();

        for seed in 0..200 {
            let mut rng = diceman::FastRng::with_seed(seed);
            let result = roll_with_rng(&reg, "ns.mishap", &mut rng).unwrap();
            if result.roll == Some(1) {
                assert_eq!(result.children.len(), 2);
                for child in &result.children {
                    assert_ne!(
                        child.roll.unwrap(),
                        1,
                        "seed {seed}: self-referential reroll should prevent value 1"
                    );
                }
            }
        }
    }
}
