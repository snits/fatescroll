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

/// Generate a table YAML skeleton from a dice expression.
/// Each possible value gets its own result entry with empty text.
pub fn generate_template(roll_expr: &str, name: &str) -> Result<String, Error> {
    let (min, max) = dice_range(roll_expr)?;
    let mut output = String::new();
    output.push_str(&format!("name: {name}\n"));
    output.push_str("type: simple\n");
    output.push_str("tags: []\n");
    output.push_str(&format!("roll: {roll_expr}\n"));
    output.push_str("results:\n");
    for value in min..=max {
        output.push_str(&format!("  - min: {value}\n"));
        output.push_str(&format!("    max: {value}\n"));
        output.push_str("    text: \"\"\n");
    }
    Ok(output)
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
}
