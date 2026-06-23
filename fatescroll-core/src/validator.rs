// ABOUTME: Validation for tables, result entries, namespaces, and cross-references.
// ABOUTME: Per-type checks run during load; cross-ref checks run after registry is populated.

use crate::error::ValidationError;
use crate::models::{ResultEntry, Table};
use crate::registry::Registry;
use diceman::Expr;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

static NAMESPACE_SEGMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_-]*$").unwrap());

/// Maximum span of a modifier table's entry envelope. Bounds the coverage
/// allocation and guards against overflow from absurd modifier_range bounds.
const MAX_ENVELOPE_WIDTH: i64 = 100_000;

/// Narrow an i64 outcome envelope `[min, max]` to i32, rejecting envelopes too
/// wide to allocate a coverage vec for, or whose endpoints fall outside i32.
/// On rejection returns the offending `width` (`Err`) for the caller's error.
/// Callers pass non-overflowing i64 endpoints (i32-derived or non-negative),
/// so `max - min` cannot overflow.
fn bounded_envelope(min: i64, max: i64) -> Result<(i32, i32), i64> {
    let width = max - min;
    if width > MAX_ENVELOPE_WIDTH || min < i32::MIN as i64 || max > i32::MAX as i64 {
        return Err(width);
    }
    Ok((min as i32, max as i32))
}

pub fn validate_namespace(namespace: &str) -> Result<(), ValidationError> {
    if namespace.is_empty() {
        return Err(ValidationError::InvalidNamespace {
            namespace: namespace.to_string(),
            reason: "namespace cannot be empty".into(),
        });
    }
    for segment in namespace.split('.') {
        if segment.is_empty() {
            return Err(ValidationError::InvalidNamespace {
                namespace: namespace.to_string(),
                reason: "empty segment (double dot)".into(),
            });
        }
        if !NAMESPACE_SEGMENT.is_match(segment) {
            return Err(ValidationError::InvalidNamespace {
                namespace: namespace.to_string(),
                reason: format!("segment '{segment}' must match [a-z][a-z0-9_-]*"),
            });
        }
    }
    Ok(())
}

pub fn validate_result_entry(entry: &ResultEntry, table_name: &str) -> Result<(), ValidationError> {
    if entry.max < entry.min {
        return Err(ValidationError::RangeReversed {
            table: table_name.to_string(),
            min: entry.min,
            max: entry.max,
        });
    }
    Ok(())
}

/// Validates a table's internal consistency (ranges, dice expression).
/// Does NOT check cross-references (chains, compound refs).
pub fn validate_table(table: &Table) -> Result<(), ValidationError> {
    match table {
        Table::Simple {
            name,
            roll,
            results,
            modifier_range,
            ..
        } => {
            let parsed =
                diceman::parse(roll).map_err(|e| ValidationError::InvalidDiceExpression {
                    table: name.clone(),
                    expr: roll.clone(),
                    reason: e.to_string(),
                })?;
            for entry in results {
                validate_result_entry(entry, name)?;
            }

            if let Expr::DigitRoll { sides, count } = parsed {
                if modifier_range.is_some() {
                    return Err(ValidationError::ModifierUnsupportedForDigitDice {
                        table: name.clone(),
                        expr: roll.clone(),
                    });
                }
                return validate_digit_dice_coverage(name, roll, results, sides, count);
            }

            let (envelope_min, envelope_max) = match modifier_range {
                Some(mr) => {
                    if mr.min > mr.max {
                        return Err(ValidationError::ModifierRangeReversed {
                            table: name.clone(),
                            min: mr.min,
                            max: mr.max,
                        });
                    }
                    let (d_min, d_max) = crate::dice::dice_range(roll).map_err(|e| match e {
                        crate::error::Error::Validation(v) => v,
                        other => ValidationError::InvalidDiceExpression {
                            table: name.clone(),
                            expr: roll.clone(),
                            reason: other.to_string(),
                        },
                    })?;
                    let env_min = d_min as i64 + mr.min as i64;
                    let env_max = d_max as i64 + mr.max as i64;
                    match bounded_envelope(env_min, env_max) {
                        Ok(pair) => pair,
                        Err(width) => {
                            return Err(ValidationError::ModifierRangeTooWide {
                                table: name.clone(),
                                width,
                                max: MAX_ENVELOPE_WIDTH,
                            });
                        }
                    }
                }
                None => {
                    let sim = diceman::simulate_seeded(roll, 100_000, 42).map_err(|e| {
                        ValidationError::InvalidDiceExpression {
                            table: name.clone(),
                            expr: roll.clone(),
                            reason: e.to_string(),
                        }
                    })?;
                    if sim.min < 0 || sim.max < 0 {
                        return Err(ValidationError::InvalidDiceExpression {
                            table: name.clone(),
                            expr: roll.clone(),
                            reason: format!(
                                "dice range [{}, {}] includes negative values",
                                sim.min, sim.max
                            ),
                        });
                    }
                    match bounded_envelope(sim.min, sim.max) {
                        Ok(pair) => pair,
                        Err(width) => {
                            return Err(ValidationError::DiceRangeTooWide {
                                table: name.clone(),
                                width,
                                max: MAX_ENVELOPE_WIDTH,
                            });
                        }
                    }
                }
            };
            validate_envelope_coverage(name, envelope_min, envelope_max, results)
        }
        Table::Compound { .. } => {
            // Per-type validation for compound tables is minimal.
            // Reference resolution checked in cross-ref validation.
            Ok(())
        }
    }
}

/// Check coverage for a digit-dice expression (D66, D666, etc.).
/// Valid values are non-contiguous (e.g., 11-16, 21-26, ..., 61-66 for D66).
/// Every valid value must be covered exactly once; entries must not fall outside the valid set.
fn validate_digit_dice_coverage(
    name: &str,
    _roll: &str,
    results: &[ResultEntry],
    sides: u32,
    count: u32,
) -> Result<(), ValidationError> {
    let valid_values: HashSet<u32> = crate::dice::digit_dice_values(sides, count)
        .into_iter()
        .collect();
    let dice_min = *valid_values.iter().min().unwrap() as i32;
    let dice_max = *valid_values.iter().max().unwrap() as i32;

    // Pre-check: every entry must only reference valid digit-dice values
    for entry in results {
        for v in entry.min..=entry.max {
            if v < 0 || !valid_values.contains(&(v as u32)) {
                return Err(ValidationError::EntryOutOfRange {
                    table: name.to_string(),
                    entry_min: entry.min,
                    entry_max: entry.max,
                    dice_min,
                    dice_max,
                });
            }
        }
    }

    // Build a coverage count per valid value
    let mut coverage: std::collections::HashMap<u32, u32> =
        valid_values.iter().map(|&v| (v, 0)).collect();
    for entry in results {
        for v in entry.min..=entry.max {
            if v >= 0
                && let Some(count) = coverage.get_mut(&(v as u32))
            {
                *count += 1;
            }
        }
    }

    let mut missing: Vec<i32> = coverage
        .iter()
        .filter(|(_, c)| **c == 0)
        .map(|(&v, _)| v as i32)
        .collect();
    missing.sort_unstable();
    if !missing.is_empty() {
        return Err(ValidationError::RangeGap {
            table: name.to_string(),
            missing,
        });
    }

    let mut overlapping: Vec<i32> = coverage
        .iter()
        .filter(|(_, c)| **c > 1)
        .map(|(&v, _)| v as i32)
        .collect();
    overlapping.sort_unstable();
    if !overlapping.is_empty() {
        return Err(ValidationError::RangeOverlap {
            table: name.to_string(),
            overlapping,
        });
    }

    Ok(())
}

/// Check that `results` form a contiguous, non-overlapping cover of
/// [envelope_min, envelope_max] exactly. Any value outside the envelope,
/// any gap, or any overlap is an error.
fn validate_envelope_coverage(
    name: &str,
    envelope_min: i32,
    envelope_max: i32,
    results: &[ResultEntry],
) -> Result<(), ValidationError> {
    for entry in results {
        if entry.min < envelope_min || entry.max > envelope_max {
            return Err(ValidationError::EntryOutOfRange {
                table: name.to_string(),
                entry_min: entry.min,
                entry_max: entry.max,
                dice_min: envelope_min,
                dice_max: envelope_max,
            });
        }
    }
    let len = (envelope_max - envelope_min + 1) as usize;
    let mut coverage = vec![0u32; len];
    for entry in results {
        let start = (entry.min - envelope_min) as usize;
        let end = (entry.max - envelope_min) as usize;
        for slot in coverage.iter_mut().take(end + 1).skip(start) {
            *slot += 1;
        }
    }
    let missing: Vec<i32> = coverage
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == 0)
        .map(|(i, _)| i as i32 + envelope_min)
        .collect();
    if !missing.is_empty() {
        return Err(ValidationError::RangeGap {
            table: name.to_string(),
            missing,
        });
    }
    let overlapping: Vec<i32> = coverage
        .iter()
        .enumerate()
        .filter(|(_, c)| **c > 1)
        .map(|(i, _)| i as i32 + envelope_min)
        .collect();
    if !overlapping.is_empty() {
        return Err(ValidationError::RangeOverlap {
            table: name.to_string(),
            overlapping,
        });
    }
    Ok(())
}

/// Validate that all chain and compound table references resolve in the registry.
/// Returns collected errors (not just the first one).
pub fn validate_references(registry: &Registry) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    for (fqid, table) in registry.all_tables() {
        // Extract the namespace from the FQID (everything up to the last dot)
        let current_namespace = fqid.rsplit_once('.').map(|(ns, _)| ns).unwrap_or("");

        match table {
            Table::Simple { name, results, .. } => {
                for entry in results {
                    if let Some(chains) = &entry.chain {
                        for chain_ref in chains {
                            if registry
                                .resolve(chain_ref.table_id(), current_namespace)
                                .is_none()
                            {
                                errors.push(ValidationError::UnresolvedChain {
                                    table: name.clone(),
                                    reference: chain_ref.table_id().to_string(),
                                });
                            }
                        }
                    }
                }
            }
            Table::Compound { name, tables, .. } => {
                for table_ref in tables {
                    if registry.resolve(table_ref, current_namespace).is_none() {
                        errors.push(ValidationError::UnresolvedCompoundRef {
                            table: name.clone(),
                            reference: table_ref.clone(),
                        });
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChainRef, ResultEntry, Table};

    #[test]
    fn valid_namespace_single_segment() {
        assert!(validate_namespace("test").is_ok());
    }

    #[test]
    fn valid_namespace_multi_segment() {
        assert!(validate_namespace("dmg.treasure.gems").is_ok());
    }

    #[test]
    fn invalid_namespace_starts_with_digit() {
        assert!(validate_namespace("2e-dmg").is_err());
    }

    #[test]
    fn invalid_namespace_uppercase() {
        assert!(validate_namespace("DMG").is_err());
    }

    #[test]
    fn invalid_namespace_empty_segment() {
        assert!(validate_namespace("dmg..treasure").is_err());
    }

    #[test]
    fn valid_result_entry() {
        let entry = ResultEntry {
            min: 1,
            max: 3,
            text: Some("test".into()),
            chain: None,
        };
        assert!(validate_result_entry(&entry, "test-table").is_ok());
    }

    #[test]
    fn reversed_range_entry() {
        let entry = ResultEntry {
            min: 5,
            max: 2,
            text: Some("test".into()),
            chain: None,
        };
        let err = validate_result_entry(&entry, "test-table").unwrap_err();
        assert!(matches!(
            err,
            crate::error::ValidationError::RangeReversed { .. }
        ));
    }

    #[test]
    fn valid_simple_table_full_coverage() {
        let table = Table::Simple {
            id: "test".into(),
            name: "Test".into(),
            tags: vec![],
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
        };
        assert!(validate_table(&table).is_ok());
    }

    #[test]
    fn simple_table_with_gap() {
        let table = Table::Simple {
            id: "gappy".into(),
            name: "Gappy".into(),
            tags: vec![],
            roll: "1d6".into(),
            modifier_range: None,
            results: vec![
                ResultEntry {
                    min: 1,
                    max: 2,
                    text: Some("Low".into()),
                    chain: None,
                },
                ResultEntry {
                    min: 5,
                    max: 6,
                    text: Some("High".into()),
                    chain: None,
                },
            ],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(
            err,
            crate::error::ValidationError::RangeGap { .. }
        ));
    }

    #[test]
    fn simple_table_with_overlap() {
        let table = Table::Simple {
            id: "overlapping".into(),
            name: "Overlapping".into(),
            tags: vec![],
            roll: "1d6".into(),
            modifier_range: None,
            results: vec![
                ResultEntry {
                    min: 1,
                    max: 4,
                    text: Some("Low".into()),
                    chain: None,
                },
                ResultEntry {
                    min: 3,
                    max: 6,
                    text: Some("High".into()),
                    chain: None,
                },
            ],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(
            err,
            crate::error::ValidationError::RangeOverlap { .. }
        ));
    }

    #[test]
    fn simple_table_bad_dice_expression() {
        let table = Table::Simple {
            id: "bad-dice".into(),
            name: "BadDice".into(),
            tags: vec![],
            roll: "1z6".into(),
            modifier_range: None,
            results: vec![ResultEntry {
                min: 1,
                max: 6,
                text: Some("X".into()),
                chain: None,
            }],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(
            err,
            crate::error::ValidationError::InvalidDiceExpression { .. }
        ));
    }

    #[test]
    fn entry_below_dice_min() {
        let table = Table::Simple {
            id: "below".into(),
            name: "Below".into(),
            tags: vec![],
            roll: "1d6".into(), // range 1-6
            modifier_range: None,
            results: vec![
                ResultEntry {
                    min: 0, // below dice_min of 1
                    max: 3,
                    text: Some("Bad".into()),
                    chain: None,
                },
                ResultEntry {
                    min: 4,
                    max: 6,
                    text: Some("OK".into()),
                    chain: None,
                },
            ],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(err, ValidationError::EntryOutOfRange { .. }));
    }

    #[test]
    fn entry_above_dice_max() {
        let table = Table::Simple {
            id: "above".into(),
            name: "Above".into(),
            tags: vec![],
            roll: "1d6".into(), // range 1-6
            modifier_range: None,
            results: vec![
                ResultEntry {
                    min: 1,
                    max: 3,
                    text: Some("OK".into()),
                    chain: None,
                },
                ResultEntry {
                    min: 4,
                    max: 8, // above dice_max of 6
                    text: Some("Bad".into()),
                    chain: None,
                },
            ],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(err, ValidationError::EntryOutOfRange { .. }));
    }

    use crate::registry::Registry;
    use crate::test_utils::fixtures_path;

    #[test]
    fn validate_refs_valid_collection() {
        let manifest_path = fixtures_path("valid-collection/manifest.yaml");
        let registry = crate::loader::load_collection(&manifest_path).unwrap();
        assert!(validate_references(&registry).is_ok());
    }

    #[test]
    fn validate_refs_catches_broken_chain() {
        let mut registry = Registry::new();
        registry
            .register(
                "test.broken".into(),
                Table::Simple {
                    id: "broken".into(),
                    name: "Broken".into(),
                    tags: vec![],
                    roll: "1d4".into(),
                    modifier_range: None,
                    results: vec![
                        ResultEntry {
                            min: 1,
                            max: 2,
                            text: Some("X".into()),
                            chain: Some(vec![ChainRef::Simple("nonexistent".into())]),
                        },
                        ResultEntry {
                            min: 3,
                            max: 4,
                            text: Some("Y".into()),
                            chain: None,
                        },
                    ],
                },
            )
            .unwrap();

        let errors = validate_references(&registry).unwrap_err();
        assert!(!errors.is_empty());
        assert!(matches!(
            &errors[0],
            ValidationError::UnresolvedChain { .. }
        ));
    }

    #[test]
    fn validate_refs_catches_broken_compound() {
        let mut registry = Registry::new();
        registry
            .register(
                "test.comp".into(),
                Table::Compound {
                    id: "bad-compound".into(),
                    name: "Bad Compound".into(),
                    tags: vec![],
                    tables: vec!["nonexistent-a".into(), "nonexistent-b".into()],
                },
            )
            .unwrap();

        let errors = validate_references(&registry).unwrap_err();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn negative_dice_range_returns_error() {
        let table = Table::Simple {
            id: "negative".into(),
            name: "Negative".into(),
            tags: vec![],
            roll: "1d4-3".into(), // range -2 to 1
            modifier_range: None,
            results: vec![ResultEntry {
                min: 1,
                max: 4,
                text: Some("X".into()),
                chain: None,
            }],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidDiceExpression { .. }));
    }

    #[test]
    fn valid_d66_table_full_coverage() {
        // Build all 36 valid D66 values as individual entries (min==max each)
        let values = crate::dice::digit_dice_values(6, 2);
        let results: Vec<ResultEntry> = values
            .iter()
            .map(|&v| ResultEntry {
                min: v as i32,
                max: v as i32,
                text: Some(format!("Result {v}")),
                chain: None,
            })
            .collect();
        let table = Table::Simple {
            id: "d66-full".into(),
            name: "D66 Full".into(),
            tags: vec![],
            roll: "D66".into(),
            modifier_range: None,
            results,
        };
        assert!(validate_table(&table).is_ok());
    }

    #[test]
    fn d66_table_with_gap() {
        // Build all 36 valid D66 values except 35
        let values = crate::dice::digit_dice_values(6, 2);
        let results: Vec<ResultEntry> = values
            .iter()
            .filter(|&&v| v != 35)
            .map(|&v| ResultEntry {
                min: v as i32,
                max: v as i32,
                text: Some(format!("Result {v}")),
                chain: None,
            })
            .collect();
        let table = Table::Simple {
            id: "d66-gap".into(),
            name: "D66 Gap".into(),
            tags: vec![],
            roll: "D66".into(),
            modifier_range: None,
            results,
        };
        let err = validate_table(&table).unwrap_err();
        assert!(
            matches!(err, ValidationError::RangeGap { .. }),
            "Expected RangeGap, got: {err:?}"
        );
    }

    #[test]
    fn d66_table_entry_outside_valid_range() {
        // Include an entry for value 17 which is not a valid D66 outcome
        let values = crate::dice::digit_dice_values(6, 2);
        let mut results: Vec<ResultEntry> = values
            .iter()
            .map(|&v| ResultEntry {
                min: v as i32,
                max: v as i32,
                text: Some(format!("Result {v}")),
                chain: None,
            })
            .collect();
        // Add an invalid entry for 17
        results.push(ResultEntry {
            min: 17,
            max: 17,
            text: Some("Invalid".into()),
            chain: None,
        });
        let table = Table::Simple {
            id: "d66-invalid".into(),
            name: "D66 Invalid".into(),
            tags: vec![],
            roll: "D66".into(),
            modifier_range: None,
            results,
        };
        let err = validate_table(&table).unwrap_err();
        assert!(
            matches!(err, ValidationError::EntryOutOfRange { .. }),
            "Expected EntryOutOfRange, got: {err:?}"
        );
    }

    #[test]
    fn modifier_table_strict_expand_valid_shadowdark() {
        let results = (1..=14)
            .map(|v| ResultEntry {
                min: v,
                max: v,
                text: Some(format!("E{v}")),
                chain: None,
            })
            .collect();
        let table = Table::Simple {
            id: "carousing".into(),
            name: "Carousing".into(),
            tags: vec![],
            roll: "1d8".into(),
            modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
            results,
        };
        assert!(validate_table(&table).is_ok());
    }

    #[test]
    fn modifier_table_strict_expand_valid_traveller_negative() {
        let results = (-5..=6)
            .map(|v| ResultEntry {
                min: v,
                max: v,
                text: Some(format!("E{v}")),
                chain: None,
            })
            .collect();
        let table = Table::Simple {
            id: "aging".into(),
            name: "Aging".into(),
            tags: vec![],
            roll: "1d6".into(),
            modifier_range: Some(crate::models::ModifierRange { min: -6, max: 0 }),
            results,
        };
        assert!(validate_table(&table).is_ok());
    }
    #[test]
    fn modifier_table_gap_errors() {
        let results: Vec<ResultEntry> = (1..=14)
            .filter(|&v| v != 7)
            .map(|v| ResultEntry {
                min: v,
                max: v,
                text: Some("E".into()),
                chain: None,
            })
            .collect();
        let table = Table::Simple {
            id: "carousing".into(),
            name: "Carousing".into(),
            tags: vec![],
            roll: "1d8".into(),
            modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
            results,
        };
        assert!(matches!(
            validate_table(&table).unwrap_err(),
            ValidationError::RangeGap { .. }
        ));
    }
    #[test]
    fn modifier_table_entry_beyond_envelope_errors() {
        let mut results: Vec<ResultEntry> = (1..=14)
            .map(|v| ResultEntry {
                min: v,
                max: v,
                text: Some("E".into()),
                chain: None,
            })
            .collect();
        results.push(ResultEntry {
            min: 15,
            max: 15,
            text: Some("Over".into()),
            chain: None,
        });
        let table = Table::Simple {
            id: "carousing".into(),
            name: "Carousing".into(),
            tags: vec![],
            roll: "1d8".into(),
            modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
            results,
        };
        assert!(matches!(
            validate_table(&table).unwrap_err(),
            ValidationError::EntryOutOfRange { .. }
        ));
    }
    #[test]
    fn modifier_range_reversed_errors() {
        let table = Table::Simple {
            id: "bad".into(),
            name: "Bad".into(),
            tags: vec![],
            roll: "1d8".into(),
            modifier_range: Some(crate::models::ModifierRange { min: 6, max: 0 }),
            results: vec![ResultEntry {
                min: 1,
                max: 8,
                text: Some("X".into()),
                chain: None,
            }],
        };
        assert!(matches!(
            validate_table(&table).unwrap_err(),
            ValidationError::ModifierRangeReversed { .. }
        ));
    }
    #[test]
    fn modifier_range_on_digit_dice_errors() {
        let table = Table::Simple {
            id: "d66".into(),
            name: "D66".into(),
            tags: vec![],
            roll: "D66".into(),
            modifier_range: Some(crate::models::ModifierRange { min: 0, max: 1 }),
            results: vec![ResultEntry {
                min: 11,
                max: 11,
                text: Some("X".into()),
                chain: None,
            }],
        };
        assert!(matches!(
            validate_table(&table).unwrap_err(),
            ValidationError::ModifierUnsupportedForDigitDice { .. }
        ));
    }
    #[test]
    fn modifier_range_on_complex_expr_errors() {
        let table = Table::Simple {
            id: "kh".into(),
            name: "KH".into(),
            tags: vec![],
            roll: "4d6kh3".into(),
            modifier_range: Some(crate::models::ModifierRange { min: 0, max: 1 }),
            results: vec![ResultEntry {
                min: 3,
                max: 19,
                text: Some("X".into()),
                chain: None,
            }],
        };
        assert!(validate_table(&table).is_err());
    }
    #[test]
    fn absurd_modifier_range_errors_not_panics() {
        let table = Table::Simple {
            id: "huge".into(),
            name: "Huge".into(),
            tags: vec![],
            roll: "1d8".into(),
            modifier_range: Some(crate::models::ModifierRange {
                min: 0,
                max: i32::MAX,
            }),
            results: vec![ResultEntry {
                min: 1,
                max: 8,
                text: Some("X".into()),
                chain: None,
            }],
        };
        assert!(matches!(
            validate_table(&table).unwrap_err(),
            ValidationError::ModifierRangeTooWide { .. }
        ));
    }
    #[test]
    fn modifier_range_endpoint_overflow_errors_not_panics() {
        // Small width but an endpoint past i32::MAX must error cleanly, not wrap/panic.
        let table = Table::Simple {
            id: "edge".into(),
            name: "Edge".into(),
            tags: vec![],
            roll: "1d8".into(),
            modifier_range: Some(crate::models::ModifierRange {
                min: i32::MAX - 10,
                max: i32::MAX,
            }),
            results: vec![ResultEntry {
                min: 1,
                max: 8,
                text: Some("X".into()),
                chain: None,
            }],
        };
        assert!(matches!(
            validate_table(&table).unwrap_err(),
            ValidationError::ModifierRangeTooWide { .. }
        ));
    }

    #[test]
    fn compound_table_validates_ok() {
        let table = Table::Compound {
            id: "compound".into(),
            name: "Compound".into(),
            tags: vec![],
            tables: vec!["a".into(), "b".into()],
        };
        // Per-type validation for compound tables always passes
        // (reference resolution is cross-ref validation in Task 7)
        assert!(validate_table(&table).is_ok());
    }

    #[test]
    fn non_modifier_dice_range_too_wide_is_rejected() {
        // 1dN with N well past MAX_ENVELOPE_WIDTH: a single entry covers the whole
        // span. Over 100_000 seeded samples of 1d400000 the observed span is ~400k,
        // comfortably above the 100k cap, so the width guard must reject it.
        let sides = MAX_ENVELOPE_WIDTH * 4; // 400_000
        let table = Table::Simple {
            id: "wide".into(),
            name: "Wide".into(),
            tags: vec![],
            roll: format!("1d{sides}"),
            modifier_range: None,
            results: vec![ResultEntry {
                min: 1,
                max: sides as i32,
                text: Some("x".into()),
                chain: None,
            }],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(
            matches!(err, ValidationError::DiceRangeTooWide { .. }),
            "expected DiceRangeTooWide, got: {err:?}"
        );
    }

    #[test]
    fn bounded_envelope_accepts_normal_width() {
        assert_eq!(bounded_envelope(1, 6), Ok((1, 6)));
        // exact cap boundary: width == MAX_ENVELOPE_WIDTH is accepted (guard is `>`)
        assert_eq!(bounded_envelope(0, MAX_ENVELOPE_WIDTH), Ok((0, 100_000)));
    }

    #[test]
    fn bounded_envelope_rejects_too_wide() {
        // width = MAX_ENVELOPE_WIDTH + 1
        let err = bounded_envelope(0, MAX_ENVELOPE_WIDTH + 1).unwrap_err();
        assert_eq!(err, MAX_ENVELOPE_WIDTH + 1);
    }

    #[test]
    fn bounded_envelope_rejects_endpoint_beyond_i32() {
        // width is small, but max exceeds i32::MAX
        let big = i32::MAX as i64 + 1;
        assert!(bounded_envelope(big - 5, big).is_err());
    }
}
