// ABOUTME: Per-type validation for tables, result entries, and namespaces.
// ABOUTME: Cross-reference validation (chain/compound refs) is separate; see loader.

use crate::error::ValidationError;
use crate::models::{ResultEntry, Table};
use regex::Regex;
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

pub fn validate_result_entry(
    entry: &ResultEntry,
    table_name: &str,
) -> Result<(), ValidationError> {
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
        Table::Simple { name, roll, results, .. } => {
            // Validate dice expression is parseable
            diceman::parse(roll).map_err(|e| ValidationError::InvalidDiceExpression {
                table: name.clone(),
                expr: roll.clone(),
                reason: e.to_string(),
            })?;

            // Validate each result entry
            for entry in results {
                validate_result_entry(entry, name)?;
            }

            // Get dice expression range via simulation
            let sim = diceman::simulate_seeded(roll, 100_000, 42)
                .map_err(|e| ValidationError::InvalidDiceExpression {
                    table: name.clone(),
                    expr: roll.clone(),
                    reason: e.to_string(),
                })?;
            let dice_min = sim.min as u32;
            let dice_max = sim.max as u32;

            // Check range coverage: every value in [dice_min, dice_max]
            // must be covered exactly once
            let mut coverage = vec![0u32; (dice_max - dice_min + 1) as usize];
            for entry in results {
                let start = entry.min.saturating_sub(dice_min) as usize;
                let end = entry.max.saturating_sub(dice_min) as usize;
                for i in start..=end.min(coverage.len() - 1) {
                    coverage[i] += 1;
                }
            }

            let missing: Vec<u32> = coverage.iter().enumerate()
                .filter(|(_, count)| **count == 0)
                .map(|(i, _)| i as u32 + dice_min)
                .collect();
            if !missing.is_empty() {
                return Err(ValidationError::RangeGap {
                    table: name.clone(),
                    missing,
                });
            }

            let overlapping: Vec<u32> = coverage.iter().enumerate()
                .filter(|(_, count)| **count > 1)
                .map(|(i, _)| i as u32 + dice_min)
                .collect();
            if !overlapping.is_empty() {
                return Err(ValidationError::RangeOverlap {
                    table: name.clone(),
                    overlapping,
                });
            }

            Ok(())
        }
        Table::Compound { .. } => {
            // Per-type validation for compound tables is minimal.
            // Reference resolution checked in cross-ref validation.
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ResultEntry, Table};

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
        let entry = ResultEntry { min: 1, max: 3, text: Some("test".into()), chain: None };
        assert!(validate_result_entry(&entry, "test-table").is_ok());
    }

    #[test]
    fn reversed_range_entry() {
        let entry = ResultEntry { min: 5, max: 2, text: Some("test".into()), chain: None };
        let err = validate_result_entry(&entry, "test-table").unwrap_err();
        assert!(matches!(err, crate::error::ValidationError::RangeReversed { .. }));
    }

    #[test]
    fn valid_simple_table_full_coverage() {
        let table = Table::Simple {
            name: "Test".into(),
            tags: vec![],
            roll: "1d6".into(),
            results: vec![
                ResultEntry { min: 1, max: 3, text: Some("Low".into()), chain: None },
                ResultEntry { min: 4, max: 6, text: Some("High".into()), chain: None },
            ],
        };
        assert!(validate_table(&table).is_ok());
    }

    #[test]
    fn simple_table_with_gap() {
        let table = Table::Simple {
            name: "Gappy".into(),
            tags: vec![],
            roll: "1d6".into(),
            results: vec![
                ResultEntry { min: 1, max: 2, text: Some("Low".into()), chain: None },
                ResultEntry { min: 5, max: 6, text: Some("High".into()), chain: None },
            ],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(err, crate::error::ValidationError::RangeGap { .. }));
    }

    #[test]
    fn simple_table_with_overlap() {
        let table = Table::Simple {
            name: "Overlapping".into(),
            tags: vec![],
            roll: "1d6".into(),
            results: vec![
                ResultEntry { min: 1, max: 4, text: Some("Low".into()), chain: None },
                ResultEntry { min: 3, max: 6, text: Some("High".into()), chain: None },
            ],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(err, crate::error::ValidationError::RangeOverlap { .. }));
    }

    #[test]
    fn simple_table_bad_dice_expression() {
        let table = Table::Simple {
            name: "BadDice".into(),
            tags: vec![],
            roll: "1z6".into(),
            results: vec![
                ResultEntry { min: 1, max: 6, text: Some("X".into()), chain: None },
            ],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(err, crate::error::ValidationError::InvalidDiceExpression { .. }));
    }

    #[test]
    fn compound_table_validates_ok() {
        let table = Table::Compound {
            name: "Compound".into(),
            tags: vec![],
            tables: vec!["a".into(), "b".into()],
        };
        // Per-type validation for compound tables always passes
        // (reference resolution is cross-ref validation in Task 7)
        assert!(validate_table(&table).is_ok());
    }
}
