// ABOUTME: Dice utility functions for computing valid outcomes of dice expressions.
// ABOUTME: Used by both the template generator and table validator.

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
