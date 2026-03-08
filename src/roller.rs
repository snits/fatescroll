// ABOUTME: Roll execution engine for simple and compound tables.
// ABOUTME: Handles dice evaluation, chain resolution, and result text interpolation.

use regex::Regex;
use std::sync::LazyLock;

use crate::error::RollError;
use crate::models::{RollResult, Table};
use crate::registry::Registry;

const MAX_CHAIN_DEPTH: usize = 10;

static DICE_INTERPOLATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{([^}]+)\}").unwrap());

pub fn roll(registry: &Registry, table_id: &str) -> Result<RollResult, RollError> {
    roll_with_rng(registry, table_id, &mut diceman::FastRng::new())
}

pub fn roll_with_rng(
    registry: &Registry,
    table_id: &str,
    rng: &mut impl diceman::Rng,
) -> Result<RollResult, RollError> {
    roll_recursive(registry, table_id, "", rng, 0)
}

fn roll_recursive(
    registry: &Registry,
    table_id: &str,
    current_namespace: &str,
    rng: &mut impl diceman::Rng,
    depth: usize,
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

    match table.clone() {
        Table::Simple {
            name,
            roll: roll_expr,
            results,
            ..
        } => {
            let dice_result =
                diceman::roll_with_rng(&roll_expr, rng).map_err(|e| RollError::DiceEvaluation {
                    table: name.clone(),
                    expr: roll_expr.clone(),
                    reason: e.to_string(),
                })?;

            let roll_value = dice_result.total;
            if roll_value < 0 {
                return Err(RollError::NegativeRoll { value: roll_value });
            }
            let roll_u32 = roll_value as u32;

            let entry = results
                .iter()
                .find(|e| roll_u32 >= e.min && roll_u32 <= e.max)
                .ok_or_else(|| RollError::RollOutOfRange {
                    table: name.clone(),
                    value: roll_value,
                })?;

            let text = entry.text.as_ref().map(|t| interpolate_dice(t, rng));

            let mut children = Vec::new();
            if let Some(chains) = &entry.chain {
                for chain_ref in chains {
                    let child = roll_recursive(registry, chain_ref, namespace, rng, depth + 1)?;
                    children.push(child);
                }
            }

            Ok(RollResult {
                table_name: name,
                roll: Some(roll_u32),
                text,
                children,
            })
        }
        Table::Compound {
            name,
            tables: sub_tables,
            ..
        } => {
            let mut children = Vec::new();
            for table_ref in &sub_tables {
                let child = roll_recursive(registry, table_ref, namespace, rng, depth + 1)?;
                children.push(child);
            }

            Ok(RollResult {
                table_name: name,
                roll: None,
                text: None,
                children,
            })
        }
    }
}

fn interpolate_dice(text: &str, rng: &mut impl diceman::Rng) -> String {
    DICE_INTERPOLATION
        .replace_all(text, |caps: &regex::Captures| {
            let expr = &caps[1];
            match diceman::roll_with_rng(expr, rng) {
                Ok(result) => result.total.to_string(),
                Err(_) => caps[0].to_string(),
            }
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ResultEntry, Table};
    use crate::registry::Registry;

    fn build_test_registry() -> Registry {
        let mut reg = Registry::new();

        reg.register(
            "test.simple".into(),
            Table::Simple {
                id: "simple".into(),
                name: "Simple Test".into(),
                tags: vec![],
                roll: "1d6".into(),
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
                roll: "1d4".into(),
                results: vec![
                    ResultEntry {
                        min: 1,
                        max: 2,
                        text: Some("Follow up".into()),
                        chain: Some(vec!["simple".into()]),
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
                roll: "1d4".into(),
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
                roll: "1d4".into(),
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Always chains".into()),
                    chain: Some(vec!["child".into()]),
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
                roll: "1d6".into(),
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
                roll: "1d4".into(),
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Loop".into()),
                    chain: Some(vec!["b".into()]),
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
                roll: "1d4".into(),
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Loop".into()),
                    chain: Some(vec!["a".into()]),
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
                roll: "1d4-10".into(),
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
                roll: "1d6".into(),
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
                roll: "not_a_dice_expr".into(),
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
                roll: "1d4".into(),
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
                roll: "1d4".into(),
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
    fn roll_entry_with_multiple_chains() {
        let mut reg = Registry::new();
        reg.register(
            "ns.multi".into(),
            Table::Simple {
                id: "multi".into(),
                name: "MultiChain".into(),
                tags: vec![],
                roll: "1d4".into(),
                results: vec![ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Branches".into()),
                    chain: Some(vec!["child_a".into(), "child_b".into()]),
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
                roll: "1d6".into(),
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
                roll: "1d6".into(),
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
}
