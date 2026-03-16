// ABOUTME: Generates table YAML skeletons from dice expressions or entry counts.
// ABOUTME: Supports explicit, flat, and bell curve distribution modes.

use crate::error::{Error, ValidationError};
use diceman::{Expr, Op};

/// Returns all valid digit-dice outcomes as a sorted Vec.
/// For D66 (sides=6, count=2): generates all combinations where each digit
/// is 1..=sides, concatenated as decimal digits: [11,12,...,16,21,...,66].
pub fn digit_dice_values(sides: u32, count: u32) -> Vec<u32> {
    fn recurse(sides: u32, count: u32, current: u32, out: &mut Vec<u32>) {
        if count == 0 {
            out.push(current);
            return;
        }
        for digit in 1..=sides {
            let place = 10u32.pow(count - 1);
            recurse(sides, count - 1, current + digit * place, out);
        }
    }
    let mut out = Vec::new();
    recurse(sides, count, 0, &mut out);
    out.sort_unstable();
    out
}

/// Compute the min and max values of a simple dice expression analytically.
/// Supports XdY, XdY±Z, and DNN (digit-dice) forms only. Rejects expressions
/// with modifiers (keep/drop, exploding, reroll) or complex arithmetic
/// (dice+dice, mul/div).
pub fn dice_range(expr: &str) -> Result<(u32, u32), Error> {
    let parsed = diceman::parse(expr)?;
    match parsed {
        Expr::DigitRoll { sides, count } => {
            let values = digit_dice_values(sides, count);
            Ok((*values.first().unwrap(), *values.last().unwrap()))
        }
        Expr::Roll(roll) => {
            if !roll.modifiers.is_empty() {
                return Err(unsupported(expr, "dice modifiers"));
            }
            let sides = roll.sides.count();
            Ok((roll.count, roll.count * sides))
        }
        Expr::BinOp { op, left, right } => {
            let roll = match *left {
                Expr::Roll(r) => r,
                _ => return Err(unsupported(expr, "complex left-hand expression")),
            };
            if !roll.modifiers.is_empty() {
                return Err(unsupported(expr, "dice modifiers"));
            }
            let z = match *right {
                Expr::Number(n) => n,
                _ => return Err(unsupported(expr, "non-literal right-hand expression")),
            };
            let sides = roll.sides.count() as i64;
            let count = roll.count as i64;
            let (min, max) = match op {
                Op::Add => (count + z, count * sides + z),
                Op::Sub => (count - z, count * sides - z),
                _ => return Err(unsupported(expr, "operator (only +/- supported)")),
            };
            if min < 0 || max < 0 {
                return Err(Error::Validation(
                    ValidationError::UnsupportedDiceExpression {
                        expr: expr.to_string(),
                        reason: "produces negative values".to_string(),
                    },
                ));
            }
            Ok((min as u32, max as u32))
        }
        _ => Err(unsupported(expr, "expression type")),
    }
}

fn unsupported(expr: &str, what: &str) -> Error {
    Error::Validation(ValidationError::UnsupportedDiceExpression {
        expr: expr.to_string(),
        reason: what.to_string(),
    })
}

/// Generate a table YAML skeleton from a dice expression.
/// Each possible value gets its own result entry with empty text.
/// For digit-dice expressions (D66, D666, etc.), only valid digit-dice
/// values are emitted — not the full contiguous range.
pub fn generate_template(roll_expr: &str, name: &str) -> Result<String, Error> {
    let parsed = diceman::parse(roll_expr)?;
    let values: Vec<u32> = match parsed {
        Expr::DigitRoll { sides, count } => digit_dice_values(sides, count),
        _ => {
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
    fn dice_range_1d6() {
        let (min, max) = dice_range("1d6").unwrap();
        assert_eq!(min, 1);
        assert_eq!(max, 6);
    }

    #[test]
    fn dice_range_2d6() {
        let (min, max) = dice_range("2d6").unwrap();
        assert_eq!(min, 2);
        assert_eq!(max, 12);
    }

    #[test]
    fn dice_range_1d8_plus_1() {
        let (min, max) = dice_range("1d8+1").unwrap();
        assert_eq!(min, 2);
        assert_eq!(max, 9);
    }

    #[test]
    fn dice_range_invalid_expression() {
        assert!(dice_range("1z6").is_err());
    }

    #[test]
    fn dice_range_rejects_keep_modifier() {
        let err = dice_range("4d6kh3").unwrap_err();
        assert!(
            err.to_string().contains("unsupported"),
            "Expected unsupported error, got: {err}"
        );
    }

    #[test]
    fn dice_range_rejects_exploding() {
        let err = dice_range("1d6!").unwrap_err();
        assert!(
            err.to_string().contains("unsupported"),
            "Expected unsupported error, got: {err}"
        );
    }

    #[test]
    fn dice_range_rejects_dice_plus_dice() {
        let err = dice_range("1d6+1d4").unwrap_err();
        assert!(
            err.to_string().contains("unsupported"),
            "Expected unsupported error, got: {err}"
        );
    }

    #[test]
    fn dice_range_rejects_multiplication() {
        let err = dice_range("1d6*2").unwrap_err();
        assert!(
            err.to_string().contains("unsupported"),
            "Expected unsupported error, got: {err}"
        );
    }

    #[test]
    fn dice_range_1d8_plus_6() {
        let (min, max) = dice_range("1d8+6").unwrap();
        assert_eq!(min, 7);
        assert_eq!(max, 14);
    }

    #[test]
    fn dice_range_1d6_minus_1() {
        let (min, max) = dice_range("1d6-1").unwrap();
        assert_eq!(min, 0);
        assert_eq!(max, 5);
    }

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
    fn digit_dice_values_d66() {
        let values = digit_dice_values(6, 2);
        assert_eq!(values.len(), 36);
        assert_eq!(values[0], 11);
        assert_eq!(values[values.len() - 1], 66);
        assert!(values.contains(&35));
        assert!(!values.contains(&17));
    }

    #[test]
    fn digit_dice_values_d44() {
        let values = digit_dice_values(4, 2);
        assert_eq!(values.len(), 16);
        assert_eq!(values[0], 11);
        assert_eq!(values[values.len() - 1], 44);
    }

    #[test]
    fn digit_dice_values_d666() {
        let values = digit_dice_values(6, 3);
        assert_eq!(values.len(), 216);
        assert_eq!(values[0], 111);
        assert_eq!(values[values.len() - 1], 666);
    }

    #[test]
    fn dice_range_d66() {
        let (min, max) = dice_range("D66").unwrap();
        assert_eq!(min, 11);
        assert_eq!(max, 66);
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
