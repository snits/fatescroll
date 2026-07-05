// ABOUTME: Dice utility functions for computing valid outcomes of dice expressions.
// ABOUTME: Used by both the template generator and table validator.

use crate::error::{Error, ValidationError};
use diceman::{Expr, Op, ScoringMode};

/// If `expr` is a digit-dice roll (D66, D666, ...), return its `(sides, count)`.
///
/// Digit dice lower to a `Roll` whose scoring concatenates the faces as
/// decimal digits; the side count comes from the pool's die kind.
pub fn digit_dice_params(expr: &Expr) -> Option<(u32, u32)> {
    match expr {
        Expr::Roll(roll) if matches!(roll.scoring, ScoringMode::DigitConcatenate) => {
            Some((roll.pool.kind.count(), roll.pool.count))
        }
        _ => None,
    }
}

/// Compute the min and max values of a simple dice expression analytically.
/// Supports XdY, XdY±Z, and DNN (digit-dice) forms only. Rejects expressions
/// with modifiers (keep/drop, exploding, reroll) or complex arithmetic
/// (dice+dice, mul/div).
pub fn dice_range(expr: &str) -> Result<(u32, u32), Error> {
    let parsed = diceman::parse(expr)?;
    match parsed {
        Expr::Roll(roll) => {
            if !roll.modifiers.is_empty() {
                return Err(unsupported(expr, "dice modifiers"));
            }
            match roll.scoring {
                ScoringMode::DigitConcatenate => {
                    let values = digit_dice_values(roll.pool.kind.count(), roll.pool.count);
                    Ok((*values.first().unwrap(), *values.last().unwrap()))
                }
                ScoringMode::Sum => {
                    let sides = roll.pool.kind.count();
                    Ok((roll.pool.count, roll.pool.count * sides))
                }
                _ => Err(unsupported(expr, "scoring mode")),
            }
        }
        Expr::BinOp { op, left, right } => {
            let roll = match *left {
                Expr::Roll(r) => r,
                _ => return Err(unsupported(expr, "complex left-hand expression")),
            };
            if !roll.modifiers.is_empty() {
                return Err(unsupported(expr, "dice modifiers"));
            }
            if !matches!(roll.scoring, ScoringMode::Sum) {
                return Err(unsupported(expr, "scoring mode"));
            }
            let z = match *right {
                Expr::Number(n) => n,
                _ => return Err(unsupported(expr, "non-literal right-hand expression")),
            };
            let sides = roll.pool.kind.count() as i64;
            let count = roll.pool.count as i64;
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
    fn dice_range_d66() {
        let (min, max) = dice_range("D66").unwrap();
        assert_eq!(min, 11);
        assert_eq!(max, 66);
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
}
