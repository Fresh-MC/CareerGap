// NoGap Dashboard - Helpers Module
// Operator-based comparison utilities for expected_state validation

use serde_yaml::Value as YamlValue;

/// Compare two values using the specified operator
///
/// # Arguments
/// * `actual` - The actual value retrieved from the system
/// * `expected` - The expected value from the policy
/// * `operator` - The comparison operator (eq, ne, gte, lte, gt, lt)
///
/// # Returns
/// * `Ok(true)` if the comparison passes
/// * `Ok(false)` if the comparison fails
/// * `Err(String)` if the comparison cannot be performed
#[allow(dead_code)]
pub fn compare_with_operator(
    actual: &YamlValue,
    expected: &YamlValue,
    operator: &str,
) -> Result<bool, String> {
    match operator {
        "eq" => compare_values(actual, expected, ComparisonOp::Equal),
        "ne" => compare_values(actual, expected, ComparisonOp::NotEqual),
        "gte" => compare_values(actual, expected, ComparisonOp::GreaterThanOrEqual),
        "lte" => compare_values(actual, expected, ComparisonOp::LessThanOrEqual),
        "gt" => compare_values(actual, expected, ComparisonOp::GreaterThan),
        "lt" => compare_values(actual, expected, ComparisonOp::LessThan),
        _ => Err(format!("Unsupported operator: {}", operator)),
    }
}

/// Comparison operators enum
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum ComparisonOp {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Compare two YAML values based on the comparison operator
///
/// # Arguments
/// * `actual` - The actual value retrieved from the system
/// * `expected` - The expected value from the policy
/// * `op` - The comparison operator to apply
///
/// # Returns
/// * `Ok(true)` if the comparison passes
/// * `Ok(false)` if the comparison fails
/// * `Err(String)` if the values cannot be compared
pub(crate) fn compare_values(
    actual: &YamlValue,
#[allow(dead_code)]
    expected: &YamlValue,
    op: ComparisonOp,
) -> Result<bool, String> {
    // Try numeric comparison first
    if let (Some(actual_num), Some(expected_num)) = (as_number(actual), as_number(expected)) {
        return Ok(match op {
            ComparisonOp::Equal => (actual_num - expected_num).abs() < f64::EPSILON,
            ComparisonOp::NotEqual => (actual_num - expected_num).abs() >= f64::EPSILON,
            ComparisonOp::GreaterThan => actual_num > expected_num,
            ComparisonOp::LessThan => actual_num < expected_num,
            ComparisonOp::GreaterThanOrEqual => actual_num >= expected_num,
            ComparisonOp::LessThanOrEqual => actual_num <= expected_num,
        });
    }

    // Fall back to string comparison
    let actual_str = value_to_string(actual);
    let expected_str = value_to_string(expected);

    Ok(match op {
        ComparisonOp::Equal => actual_str == expected_str,
        ComparisonOp::NotEqual => actual_str != expected_str,
        ComparisonOp::GreaterThan => actual_str > expected_str,
        ComparisonOp::LessThan => actual_str < expected_str,
        ComparisonOp::GreaterThanOrEqual => actual_str >= expected_str,
        ComparisonOp::LessThanOrEqual => actual_str <= expected_str,
    })
}

/// Convert a YAML value to a number if possible
fn as_number(value: &YamlValue) -> Option<f64> {
    match value {
#[allow(dead_code)]
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(i as f64)
            } else if let Some(u) = n.as_u64() {
                Some(u as f64)
            } else {
                n.as_f64()
            }
        }
        YamlValue::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Convert a YAML value to a string representation
fn value_to_string(value: &YamlValue) -> String {
    match value {
#[allow(dead_code)]
        YamlValue::String(s) => s.clone(),
        YamlValue::Number(n) => n.to_string(),
        YamlValue::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        YamlValue::Null => String::new(),
        _ => format!("{:?}", value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Number;

    #[test]
    fn test_compare_with_operator_eq() {
        let actual = YamlValue::Number(Number::from(10));
        let expected = YamlValue::Number(Number::from(10));
        assert_eq!(
            compare_with_operator(&actual, &expected, "eq").unwrap(),
            true
        );

        let actual = YamlValue::String("test".to_string());
        let expected = YamlValue::String("test".to_string());
        assert_eq!(
            compare_with_operator(&actual, &expected, "eq").unwrap(),
            true
        );
    }

    #[test]
    fn test_compare_with_operator_ne() {
        let actual = YamlValue::Number(Number::from(10));
        let expected = YamlValue::Number(Number::from(20));
        assert_eq!(
            compare_with_operator(&actual, &expected, "ne").unwrap(),
            true
        );
    }

    #[test]
    fn test_compare_with_operator_gte() {
        let actual = YamlValue::Number(Number::from(15));
        let expected = YamlValue::Number(Number::from(14));
        assert_eq!(
            compare_with_operator(&actual, &expected, "gte").unwrap(),
            true
        );

        let actual = YamlValue::Number(Number::from(14));
        let expected = YamlValue::Number(Number::from(14));
        assert_eq!(
            compare_with_operator(&actual, &expected, "gte").unwrap(),
            true
        );
    }

    #[test]
    fn test_compare_with_operator_lte() {
        let actual = YamlValue::Number(Number::from(10));
        let expected = YamlValue::Number(Number::from(14));
        assert_eq!(
            compare_with_operator(&actual, &expected, "lte").unwrap(),
            true
        );
    }

    #[test]
    fn test_compare_with_operator_gt() {
        let actual = YamlValue::Number(Number::from(15));
        let expected = YamlValue::Number(Number::from(14));
        assert_eq!(
            compare_with_operator(&actual, &expected, "gt").unwrap(),
            true
        );
    }

    #[test]
    fn test_compare_with_operator_lt() {
        let actual = YamlValue::Number(Number::from(10));
        let expected = YamlValue::Number(Number::from(14));
        assert_eq!(
            compare_with_operator(&actual, &expected, "lt").unwrap(),
            true
        );
    }

    #[test]
    fn test_compare_strings() {
        let actual = YamlValue::String("Administrator".to_string());
        let expected = YamlValue::String("Administrator".to_string());
        assert_eq!(
            compare_with_operator(&actual, &expected, "eq").unwrap(),
            true
        );

        let actual = YamlValue::String("WinAdmin".to_string());
        let expected = YamlValue::String("Administrator".to_string());
        assert_eq!(
            compare_with_operator(&actual, &expected, "ne").unwrap(),
            true
        );
    }

    #[test]
    fn test_compare_mixed_types() {
        let actual = YamlValue::String("10".to_string());
        let expected = YamlValue::Number(Number::from(10));
        assert_eq!(
            compare_with_operator(&actual, &expected, "eq").unwrap(),
            true
        );
    }

    #[test]
    fn test_unsupported_operator() {
        let actual = YamlValue::Number(Number::from(10));
        let expected = YamlValue::Number(Number::from(10));
        assert!(compare_with_operator(&actual, &expected, "invalid").is_err());
    }
}
