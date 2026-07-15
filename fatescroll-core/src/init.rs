// ABOUTME: Generates table YAML skeletons from dice expressions or entry counts.
// ABOUTME: Supports explicit, flat, and bell curve distribution modes.

use crate::dice::{dice_range, digit_dice_params, digit_dice_values, validate_dice_counts};
use crate::error::Error;

/// Generate a table YAML skeleton from a dice expression.
/// Each possible value gets its own result entry with empty text.
/// For digit-dice expressions (D66, D666, etc.), only valid digit-dice
/// values are emitted — not the full contiguous range.
pub fn generate_template(roll_expr: &str, name: &str) -> Result<String, Error> {
    let parsed = diceman::parse(roll_expr)?;
    validate_dice_counts(&parsed)?;
    let values: Vec<u32> = match digit_dice_params(&parsed) {
        Some((sides, count)) => digit_dice_values(sides, count),
        None => {
            let (min, max) = dice_range(roll_expr)?;
            (min..=max).collect()
        }
    };
    let mut output = String::new();
    output.push_str(&format!("name: {name}\n"));
    output.push_str("type: simple\n");
    output.push_str("tags: []\n");
    output.push_str(&format!("roll: {roll_expr}\n"));
    output.push_str("results:\n");
    for value in values {
        output.push_str(&format!("  - min: {value}\n"));
        output.push_str(&format!("    max: {value}\n"));
        output.push_str("    text: \"\"\n");
    }
    Ok(output)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Distribution {
    Flat,
    Bell,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub expression: String,
    pub num_dice: u32,
    pub entries: u32,
    pub range_min: u32,
    pub range_max: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DistributionResult {
    Exact(String),
    Suggestions(Vec<Suggestion>),
}

/// Calculate a dice expression for a given entry count and distribution.
///
/// For flat: returns 1dN.
/// For bell: tries XdY for X=2,3. Distinct values = X*(Y-1)+1.
/// Returns exact match or nearest suggestions.
pub fn calculate_distribution(entries: u32, dist: Distribution) -> DistributionResult {
    match dist {
        Distribution::Flat => DistributionResult::Exact(format!("1d{entries}")),
        Distribution::Bell => {
            let mut suggestions = Vec::new();

            for num_dice in 2..=3u32 {
                let numerator = entries - 1;
                if numerator.is_multiple_of(num_dice) {
                    let sides = numerator / num_dice + 1;
                    if sides >= 2 {
                        return DistributionResult::Exact(format!("{num_dice}d{sides}"));
                    }
                }

                let y_floor = numerator / num_dice + 1;
                let y_ceil = y_floor + 1;

                if y_floor >= 2 {
                    let floor_entries = num_dice * (y_floor - 1) + 1;
                    suggestions.push(Suggestion {
                        expression: format!("{num_dice}d{y_floor}"),
                        num_dice,
                        entries: floor_entries,
                        range_min: num_dice,
                        range_max: num_dice * y_floor,
                    });
                }

                let ceil_entries = num_dice * (y_ceil - 1) + 1;
                suggestions.push(Suggestion {
                    expression: format!("{num_dice}d{y_ceil}"),
                    num_dice,
                    entries: ceil_entries,
                    range_min: num_dice,
                    range_max: num_dice * y_ceil,
                });
            }

            DistributionResult::Suggestions(suggestions)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_template_1d6() {
        let output = generate_template("1d6", "Test Table").unwrap();
        assert!(output.contains("name: Test Table"));
        assert!(output.contains("type: simple"));
        assert!(output.contains("roll: 1d6"));
        assert!(output.contains("min: 1"));
        assert!(output.contains("max: 6"));
        let min_count = output.matches("  - min:").count();
        assert_eq!(min_count, 6);
    }

    #[test]
    fn generate_template_2d6() {
        let output = generate_template("2d6", "Bell Table").unwrap();
        assert!(output.contains("roll: 2d6"));
        assert!(output.contains("min: 2"));
        assert!(output.contains("max: 12"));
        let min_count = output.matches("  - min:").count();
        assert_eq!(min_count, 11);
    }

    #[test]
    fn flat_distribution_8_entries() {
        let result = calculate_distribution(8, Distribution::Flat);
        assert_eq!(result, DistributionResult::Exact("1d8".to_string()));
    }

    #[test]
    fn bell_exact_match_2d6() {
        let result = calculate_distribution(11, Distribution::Bell);
        assert_eq!(result, DistributionResult::Exact("2d6".to_string()));
    }

    #[test]
    fn bell_exact_match_3d6() {
        let result = calculate_distribution(16, Distribution::Bell);
        assert_eq!(result, DistributionResult::Exact("3d6".to_string()));
    }

    #[test]
    fn bell_no_exact_match_shows_suggestions() {
        let result = calculate_distribution(12, Distribution::Bell);
        match result {
            DistributionResult::Suggestions(suggestions) => {
                assert!(!suggestions.is_empty());
                assert!(suggestions.iter().any(|s| s.expression == "2d6"));
                assert!(suggestions.iter().any(|s| s.expression == "2d7"));
            }
            other => panic!("Expected Suggestions, got {:?}", other),
        }
    }

    #[test]
    fn bell_minimum_entries() {
        let result = calculate_distribution(3, Distribution::Bell);
        assert_eq!(result, DistributionResult::Exact("2d2".to_string()));
    }

    #[test]
    fn generate_template_d66() {
        let output = generate_template("D66", "D66 Test").unwrap();
        assert!(output.contains("name: D66 Test"));
        assert!(output.contains("roll: D66"));
        let entry_count = output.matches("  - min:").count();
        assert_eq!(entry_count, 36);
        assert!(output.contains("min: 11"));
        assert!(output.contains("max: 66"));
        // Must not contain impossible values like 17
        assert!(!output.contains("min: 17"));
    }
}
