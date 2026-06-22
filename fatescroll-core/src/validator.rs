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
            ..
        } => {
            // Validate dice expression is parseable
            let parsed =
                diceman::parse(roll).map_err(|e| ValidationError::InvalidDiceExpression {
                    table: name.clone(),
                    expr: roll.clone(),
                    reason: e.to_string(),
                })?;

            // Validate each result entry
            for entry in results {
                validate_result_entry(entry, name)?;
            }

            // Dispatch to digit-dice or regular coverage checking
            if let Expr::DigitRoll { sides, count } = parsed {
                validate_digit_dice_coverage(name, roll, results, sides, count)
            } else {
                validate_contiguous_coverage(name, roll, results)
            }
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

/// Check coverage for a contiguous dice expression (1d6, 2d6, 1d8+1, etc.).
/// Every integer value in [dice_min, dice_max] must be covered exactly once.
fn validate_contiguous_coverage(
    name: &str,
    roll: &str,
    results: &[ResultEntry],
) -> Result<(), ValidationError> {
    let sim = diceman::simulate_seeded(roll, 100_000, 42).map_err(|e| {
        ValidationError::InvalidDiceExpression {
            table: name.to_string(),
            expr: roll.to_string(),
            reason: e.to_string(),
        }
    })?;
    if sim.min < 0 || sim.max < 0 {
        return Err(ValidationError::InvalidDiceExpression {
            table: name.to_string(),
            expr: roll.to_string(),
            reason: format!(
                "dice range [{}, {}] includes negative values",
                sim.min, sim.max
            ),
        });
    }
    let dice_min = sim.min as i32;
    let dice_max = sim.max as i32;

    // Pre-check: every entry must fall within dice range
    for entry in results {
        if entry.min < dice_min || entry.max > dice_max {
            return Err(ValidationError::EntryOutOfRange {
                table: name.to_string(),
                entry_min: entry.min,
                entry_max: entry.max,
                dice_min,
                dice_max,
            });
        }
    }

    // Check range coverage: every value in [dice_min, dice_max]
    // must be covered exactly once
    let mut coverage = vec![0u32; (dice_max - dice_min + 1) as usize];
    for entry in results {
        let start = (entry.min - dice_min) as usize;
        let end = (entry.max - dice_min) as usize;
        for slot in coverage.iter_mut().take(end + 1).skip(start) {
            *slot += 1;
        }
    }

    let missing: Vec<i32> = coverage
        .iter()
        .enumerate()
        .filter(|(_, count)| **count == 0)
        .map(|(i, _)| i as i32 + dice_min)
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
        .filter(|(_, count)| **count > 1)
        .map(|(i, _)| i as i32 + dice_min)
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
            results,
        };
        let err = validate_table(&table).unwrap_err();
        assert!(
            matches!(err, ValidationError::EntryOutOfRange { .. }),
            "Expected EntryOutOfRange, got: {err:?}"
        );
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
}
