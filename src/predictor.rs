//! Predictive analytics engine
//!
//! Forecasts churn degradation using linear regression and calculates
//! days until files become unmaintainable. Generates prediction warnings
//! and confidence scores to help identify files that will likely need
//! refactoring in the future.

/// Performs least-squares linear regression on a set of points.
///
/// Takes a slice of (x, y) tuples where x is typically a time index (days)
/// and y is the metric value (churn percentage, LOC, etc).
///
/// Returns a tuple of (slope, intercept) that defines the line y = slope*x + intercept.
///
/// # Arguments
/// * `points` - A slice of (usize, f64) tuples representing (time_index, value)
///
/// # Returns
/// A tuple containing (slope, intercept) for the regression line.
/// Both components are f64 to handle decimal values accurately.
///
/// # Panics
/// Panics if the input slice contains fewer than 2 points.
///
/// # Example
/// ```ignore
/// let points = vec![(0, 10.0), (1, 20.0), (2, 30.0), (3, 40.0)];
/// let (slope, intercept) = linear_regression(&points);
/// assert!((slope - 10.0).abs() < 0.01);
/// assert!((intercept - 10.0).abs() < 0.01);
/// ```
pub fn linear_regression(points: &[(usize, f64)]) -> (f64, f64) {
    assert!(
        points.len() >= 2,
        "linear_regression requires at least 2 points"
    );

    let n = points.len() as f64;

    // Calculate means
    let sum_x: f64 = points.iter().map(|(x, _)| *x as f64).sum();
    let sum_y: f64 = points.iter().map(|(_, y)| y).sum();

    let mean_x = sum_x / n;
    let mean_y = sum_y / n;

    // Calculate slope using least-squares formula
    // slope = Σ((x - mean_x)(y - mean_y)) / Σ((x - mean_x)²)
    let numerator: f64 = points
        .iter()
        .map(|(x, y)| {
            let x_f64 = *x as f64;
            (x_f64 - mean_x) * (y - mean_y)
        })
        .sum();

    let denominator: f64 = points
        .iter()
        .map(|(x, _)| {
            let x_f64 = *x as f64;
            (x_f64 - mean_x).powi(2)
        })
        .sum();

    let slope = if denominator != 0.0 {
        numerator / denominator
    } else {
        // All x values are the same, no valid slope
        0.0
    };

    // Calculate intercept: intercept = mean_y - slope * mean_x
    let intercept = mean_y - slope * mean_x;

    (slope, intercept)
}

/// Predicts the value at a given x-axis point using linear regression parameters.
///
/// Uses the equation: y = slope * x + intercept
///
/// # Arguments
/// * `slope` - The slope from linear regression
/// * `intercept` - The intercept from linear regression
/// * `x` - The x-axis point at which to predict
///
/// # Returns
/// The predicted y value at the given x point
///
/// # Example
/// ```ignore
/// let predicted = predict_value_at(2.5, 10.0, 5);
/// // predicted = 2.5 * 5 + 10.0 = 22.5
/// ```
pub fn predict_value_at(slope: f64, intercept: f64, x: usize) -> f64 {
    slope * (x as f64) + intercept
}

/// Calculates the number of days until churn exceeds a critical threshold.
///
/// Performs linear regression on the churn values to forecast when the
/// churn percentage will reach or exceed the specified critical threshold.
/// Days are counted from the end of the provided data (most recent point = day 0).
///
/// # Arguments
/// * `churn_values` - A slice of churn percentages in chronological order
/// * `threshold` - The critical churn threshold (e.g., 80.0 for 80%)
///
/// # Returns
/// - `Some(days)` - Number of days until threshold is reached
/// - `None` - If threshold is already exceeded or cannot be reached (negative slope)
///
/// # Example
/// ```ignore
/// let churn_values = vec![10.0, 15.0, 20.0, 25.0];
/// let days = calculate_days_to_critical(&churn_values, 50.0);
/// // Returns Some(10) if prediction shows threshold will be reached in 10 days
/// ```
pub fn calculate_days_to_critical(churn_values: &[f64], threshold: f64) -> Option<usize> {
    // Need at least 2 points to do regression
    if churn_values.len() < 2 {
        return None;
    }

    // Create points for regression: (index, churn_value)
    let points: Vec<(usize, f64)> = churn_values
        .iter()
        .enumerate()
        .map(|(i, &val)| (i, val))
        .collect();

    let (slope, intercept) = linear_regression(&points);

    // Check if we can reach the threshold with positive slope
    if slope <= 0.0 {
        // If slope is 0 or negative, we won't reach the threshold (or already past it)
        return None;
    }

    let current_churn = churn_values[churn_values.len() - 1];

    // If already at or above threshold, return None
    if current_churn >= threshold {
        return None;
    }

    // Solve for x: threshold = slope * x + intercept
    // x = (threshold - intercept) / slope
    let x = (threshold - intercept) / slope;

    // Days to critical is the difference from the last point
    let last_point_index = (churn_values.len() - 1) as f64;
    let days_to_critical = x - last_point_index;

    if days_to_critical > 0.0 {
        Some(days_to_critical.ceil() as usize)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for linear_regression function
    #[test]
    fn test_linear_regression_simple_line() {
        // Test with perfectly linear data: y = 2x + 1
        let points = vec![(0, 1.0), (1, 3.0), (2, 5.0), (3, 7.0)];
        let (slope, intercept) = linear_regression(&points);
        assert!((slope - 2.0).abs() < 0.01, "slope should be 2.0");
        assert!((intercept - 1.0).abs() < 0.01, "intercept should be 1.0");
    }

    #[test]
    fn test_linear_regression_positive_slope() {
        // Test with increasing churn
        let points = vec![(0, 10.0), (1, 20.0), (2, 30.0)];
        let (slope, _) = linear_regression(&points);
        assert!(slope > 0.0, "slope should be positive");
        assert!((slope - 10.0).abs() < 0.01, "slope should be approximately 10.0");
    }

    #[test]
    fn test_linear_regression_negative_slope() {
        // Test with decreasing churn (improving)
        let points = vec![(0, 50.0), (1, 40.0), (2, 30.0), (3, 20.0)];
        let (slope, _) = linear_regression(&points);
        assert!(slope < 0.0, "slope should be negative");
        assert!((slope - (-10.0)).abs() < 0.01, "slope should be approximately -10.0");
    }

    #[test]
    fn test_linear_regression_constant_value() {
        // Test with constant values
        let points = vec![(0, 30.0), (1, 30.0), (2, 30.0), (3, 30.0)];
        let (slope, intercept) = linear_regression(&points);
        assert!((slope - 0.0).abs() < 0.01, "slope should be 0.0");
        assert!((intercept - 30.0).abs() < 0.01, "intercept should be 30.0");
    }

    #[test]
    fn test_linear_regression_two_points() {
        // Test with minimum viable data (2 points)
        let points = vec![(0, 5.0), (10, 25.0)];
        let (slope, intercept) = linear_regression(&points);
        // Line should be y = 2x + 5
        assert!((slope - 2.0).abs() < 0.01);
        assert!((intercept - 5.0).abs() < 0.01);
    }

    #[test]
    #[should_panic]
    fn test_linear_regression_single_point() {
        let points = vec![(0, 10.0)];
        let _ = linear_regression(&points);
    }

    #[test]
    #[should_panic]
    fn test_linear_regression_empty_data() {
        let points: Vec<(usize, f64)> = vec![];
        let _ = linear_regression(&points);
    }

    // Tests for predict_value_at function
    #[test]
    fn test_predict_value_at_basic() {
        let slope = 2.0;
        let intercept = 1.0;
        let predicted = predict_value_at(slope, intercept, 5);
        assert!((predicted - 11.0).abs() < 0.01, "2*5 + 1 = 11");
    }

    #[test]
    fn test_predict_value_at_zero() {
        let slope = 2.0;
        let intercept = 1.0;
        let predicted = predict_value_at(slope, intercept, 0);
        assert!((predicted - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_predict_value_at_large_x() {
        let slope = 0.5;
        let intercept = 10.0;
        let predicted = predict_value_at(slope, intercept, 100);
        assert!((predicted - 60.0).abs() < 0.01, "0.5*100 + 10 = 60");
    }

    #[test]
    fn test_predict_value_at_negative_slope() {
        let slope = -2.0;
        let intercept = 50.0;
        let predicted = predict_value_at(slope, intercept, 10);
        assert!((predicted - 30.0).abs() < 0.01, "-2*10 + 50 = 30");
    }

    // Tests for calculate_days_to_critical function
    #[test]
    fn test_calculate_days_to_critical_basic() {
        // Churn is steadily increasing: 10, 20, 30, 40
        // Should reach 80 eventually
        let churn_values = vec![10.0, 20.0, 30.0, 40.0];
        let days = calculate_days_to_critical(&churn_values, 80.0);
        assert!(days.is_some(), "should reach critical threshold");
        let days_val = days.unwrap();
        assert!(days_val > 0, "should take days to reach threshold");
    }

    #[test]
    fn test_calculate_days_to_critical_already_exceeded() {
        let churn_values = vec![50.0, 60.0, 70.0, 85.0];
        let days = calculate_days_to_critical(&churn_values, 80.0);
        assert_eq!(days, None, "should return None if already above threshold");
    }

    #[test]
    fn test_calculate_days_to_critical_stable_churn() {
        // Churn is stable at 30%, won't reach 80%
        let churn_values = vec![30.0, 30.0, 30.0, 30.0];
        let days = calculate_days_to_critical(&churn_values, 80.0);
        assert_eq!(days, None, "stable churn won't reach threshold");
    }

    #[test]
    fn test_calculate_days_to_critical_improving() {
        // Churn is improving (decreasing), won't reach 80%
        let churn_values = vec![50.0, 40.0, 30.0, 20.0];
        let days = calculate_days_to_critical(&churn_values, 80.0);
        assert_eq!(days, None, "improving churn won't reach threshold");
    }

    #[test]
    fn test_calculate_days_to_critical_insufficient_data() {
        let churn_values = vec![30.0];
        let days = calculate_days_to_critical(&churn_values, 80.0);
        assert_eq!(days, None, "need at least 2 points");
    }

    #[test]
    fn test_calculate_days_to_critical_close_to_threshold() {
        // Churn is 70% and increasing by 3% per day, reaches 80% in ~3-4 days
        let churn_values = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0];
        let days = calculate_days_to_critical(&churn_values, 80.0);
        assert!(days.is_some(), "should reach threshold");
        // With slope of 10.0 per step and starting at 70.0 at index 6,
        // we need 10.0 more to reach 80.0, which is 1 more step
        let days_val = days.unwrap();
        assert!(days_val <= 2, "should reach 80% in about 1 step");
    }

    // Integration tests
    #[test]
    fn test_regression_and_prediction_integration() {
        // Create a dataset and use both functions together
        let churn_values = vec![10.0, 15.0, 20.0, 25.0, 30.0];
        let points: Vec<(usize, f64)> = churn_values
            .iter()
            .enumerate()
            .map(|(i, &val)| (i, val))
            .collect();

        let (slope, intercept) = linear_regression(&points);

        // Predict at day 7 (index 7)
        let predicted_7 = predict_value_at(slope, intercept, 7);
        assert!(predicted_7 > 30.0, "should predict higher churn in future");

        // Predict at day 14
        let predicted_14 = predict_value_at(slope, intercept, 14);
        assert!(predicted_14 > predicted_7, "churn should increase further");
    }

    #[test]
    fn test_days_to_critical_with_real_churn_data() {
        // Simulate realistic churn progression
        let churn_values = vec![
            5.0,  // day 0
            12.0, // day 1
            18.0, // day 2
            25.0, // day 3
            32.0, // day 4
            40.0, // day 5
            48.0, // day 6
            56.0, // day 7
        ];

        let days = calculate_days_to_critical(&churn_values, 80.0);
        assert!(days.is_some(), "should predict reaching 80% churn");
        let days_val = days.unwrap();
        assert!(days_val > 0 && days_val <= 10, "should be within reasonable range");
    }

    #[test]
    fn test_churn_prediction_confidence_scenario() {
        // Test a scenario where we have consistent degradation
        let churn_values = vec![20.0, 30.0, 40.0, 50.0, 60.0, 70.0];

        // Calculate regression
        let points: Vec<(usize, f64)> = churn_values
            .iter()
            .enumerate()
            .map(|(i, &val)| (i, val))
            .collect();
        let (slope, intercept) = linear_regression(&points);

        // Predict 7 days from now (index 6 + 7 = 13)
        let predicted_7days = predict_value_at(slope, intercept, 13);

        // Should predict approximately 130% or more (can go above 100% in predictions)
        assert!(predicted_7days > 100.0, "should predict high churn after 7 days");

        // Days to critical (80%)
        let days_to_80 = calculate_days_to_critical(&churn_values, 80.0);
        assert!(days_to_80.is_some());
        let days_val = days_to_80.unwrap();
        assert!(days_val > 0, "days_to_critical should be positive");
    }
}
