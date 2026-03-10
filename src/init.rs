// ABOUTME: Generates table YAML skeletons from dice expressions or entry counts.
// ABOUTME: Supports explicit, flat, and bell curve distribution modes.

use crate::error::Error;

/// Determine the min and max values of a dice expression via simulation.
/// Uses diceman::simulate_seeded which returns SimResult with min and max fields.
/// Same approach used by the validator (validator.rs:71).
pub fn dice_range(expr: &str) -> Result<(u32, u32), Error> {
    diceman::parse(expr)?;
    let sim = diceman::simulate_seeded(expr, 100_000, 42)?;
    if sim.min < 0 || sim.max < 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("dice expression '{expr}' produces negative values"),
        )
        .into());
    }
    Ok((sim.min as u32, sim.max as u32))
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
}
