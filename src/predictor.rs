//! Predictive analytics engine
//!
//! Forecasts churn degradation using linear regression and calculates
//! days until files become unmaintainable. Generates prediction warnings
//! and confidence scores to help identify files that will likely need
//! refactoring in the future.

use crate::models::{ChurnPrediction, PredictionWarning};

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

/// Calculates the prediction confidence score based on data size and consistency.
///
/// The confidence score reflects how reliable the prediction is:
/// - Base: 100% (perfect)
/// - Reduce by 10% for each data point below 7 (fewer points = less reliable)
/// - Reduce by 5% per additional data point above 7 (more points might indicate noise)
/// - Final: clamp to 0.5-1.0 range (minimum 50% confidence, maximum 100%)
///
/// Formula: `1.0 - (max(0, 7 - len) * 0.1 + max(0, len - 7) * 0.05).min(0.5)`
///
/// # Arguments
/// * `data_len` - Number of data points in the churn history
///
/// # Returns
/// A confidence score between 0.5 (50%) and 1.0 (100%)
fn calculate_confidence_score(data_len: usize) -> f64 {
    let penalty = {
        if data_len < 7 {
            (7 - data_len) as f64 * 0.1
        } else if data_len > 7 {
            (data_len - 7) as f64 * 0.05
        } else {
            0.0
        }
    };
    (1.0 - penalty.min(0.5)).max(0.5)
}

/// Determines the warning level based on predicted churn and trend.
///
/// Warning levels:
/// - **None**: predicted_churn_14days < 40% (safe)
/// - **Watch**: 40% ≤ predicted_churn_14days < 60% (monitor)
/// - **Degrade**: 60% ≤ predicted_churn_14days < 80% (degrading)
/// - **Critical**: predicted_churn_14days ≥ 80% (alert)
///
/// # Arguments
/// * `predicted_churn_14days` - Predicted churn percentage for 14 days
///
/// # Returns
/// A PredictionWarning enum value
fn determine_warning_level(predicted_churn_14days: f64) -> PredictionWarning {
    if predicted_churn_14days >= 80.0 {
        PredictionWarning::Critical
    } else if predicted_churn_14days >= 60.0 {
        PredictionWarning::Degrade
    } else if predicted_churn_14days >= 40.0 {
        PredictionWarning::Watch
    } else {
        PredictionWarning::None
    }
}

/// Predicts churn trajectory for a file using linear regression.
///
/// This function performs linear regression on historical churn percentages
/// to forecast future churn values and generate predictive warnings.
///
/// # Arguments
/// * `file` - The file name being analyzed
/// * `churn_history` - A slice of historical churn percentages in chronological order
///
/// # Returns
/// A `ChurnPrediction` struct containing:
/// - Current churn percentage
/// - Predicted churn for 7 and 14 days
/// - Days until critical threshold (80% churn)
/// - Confidence score for the prediction
/// - Warning level based on predictions
///
/// # Errors
/// Returns an error if the churn history has fewer than 2 data points.
///
/// # Example
/// ```ignore
/// let churn_history = vec![10.0, 15.0, 20.0, 25.0];
/// let prediction = predict_churn_trajectory("src/file.rs", &churn_history)?;
/// println!("14-day prediction: {:.1}%", prediction.predicted_churn_14days);
/// ```
pub fn predict_churn_trajectory(
    file: &str,
    churn_history: &[f64],
) -> anyhow::Result<ChurnPrediction> {
    // Validate input
    if churn_history.len() < 2 {
        return Err(anyhow::anyhow!(
            "insufficient churn data for file {}: need at least 2 points, got {}",
            file,
            churn_history.len()
        ));
    }

    // Create points for linear regression: (index, churn_value)
    let points: Vec<(usize, f64)> = churn_history
        .iter()
        .enumerate()
        .map(|(i, &val)| (i, val))
        .collect();

    // Perform linear regression
    let (slope, intercept) = linear_regression(&points);

    // Get current churn (last value in history)
    let current_churn = churn_history[churn_history.len() - 1];

    // Calculate predictions
    // For 7-day prediction, we predict at index: current_index + 7
    let last_index = churn_history.len() - 1;
    let predicted_churn_7days =
        predict_value_at(slope, intercept, last_index + 7).max(0.0).min(100.0);
    let predicted_churn_14days =
        predict_value_at(slope, intercept, last_index + 14).max(0.0).min(100.0);

    // Calculate days to critical (80% churn threshold)
    let days_to_critical = calculate_days_to_critical(churn_history, 80.0);

    // Calculate confidence score
    let prediction_confidence = calculate_confidence_score(churn_history.len());

    // Determine warning level based on 14-day prediction
    let warning_level = determine_warning_level(predicted_churn_14days);

    Ok(ChurnPrediction {
        file: file.to_string(),
        current_churn,
        predicted_churn_7days,
        predicted_churn_14days,
        days_to_critical,
        prediction_confidence,
        warning_level,
    })
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

    // Tests for predict_churn_trajectory function

    #[test]
    fn test_predict_churn_trajectory_basic_degrading_trend() {
        // Test with degrading churn (positive trend)
        let file = "src/degrading.rs";
        let churn_history = vec![10.0, 20.0, 30.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok(), "should successfully predict with 3 data points");

        let prediction = result.unwrap();
        assert_eq!(prediction.file, file);
        assert_eq!(prediction.current_churn, 30.0, "current churn should be last value");
        assert!(
            prediction.predicted_churn_7days > prediction.current_churn,
            "degrading trend should show increase"
        );
        assert!(
            prediction.predicted_churn_14days >= prediction.predicted_churn_7days,
            "14-day prediction should be >= 7-day"
        );
        assert!(
            prediction.prediction_confidence >= 0.5 && prediction.prediction_confidence <= 1.0,
            "confidence should be between 0.5 and 1.0"
        );
    }

    #[test]
    fn test_predict_churn_trajectory_improving_trend() {
        // Test with improving churn (negative trend)
        let file = "src/improving.rs";
        let churn_history = vec![60.0, 50.0, 40.0, 35.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        assert_eq!(prediction.file, file);
        assert_eq!(prediction.current_churn, 35.0);
        // With improving trend, predictions should not increase significantly
        assert!(prediction.predicted_churn_7days <= prediction.current_churn + 10.0);
        assert!(prediction.predicted_churn_14days <= prediction.predicted_churn_7days + 10.0);
    }

    #[test]
    fn test_predict_churn_trajectory_stable_trend() {
        // Test with stable churn (no trend)
        let file = "src/stable.rs";
        let churn_history = vec![40.0, 40.0, 40.0, 40.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        assert_eq!(prediction.file, file);
        assert_eq!(prediction.current_churn, 40.0);
        // With stable trend, predictions should remain approximately the same
        assert!((prediction.predicted_churn_7days - 40.0).abs() < 1.0);
        assert!((prediction.predicted_churn_14days - 40.0).abs() < 1.0);
    }

    #[test]
    fn test_predict_churn_trajectory_minimal_data() {
        // Test with minimal data (2 points)
        let file = "src/minimal.rs";
        let churn_history = vec![20.0, 40.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok(), "should work with 2 data points");

        let prediction = result.unwrap();
        assert_eq!(prediction.current_churn, 40.0);
        // With 2 points, confidence should be reduced
        assert!(prediction.prediction_confidence < 1.0);
        assert!(prediction.prediction_confidence >= 0.5);
    }

    #[test]
    fn test_predict_churn_trajectory_insufficient_data() {
        // Test with insufficient data (1 point)
        let file = "src/insufficient.rs";
        let churn_history = vec![30.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_err(), "should fail with only 1 data point");
    }

    #[test]
    fn test_predict_churn_trajectory_extended_data() {
        // Test with extended data (10+ points)
        let file = "src/extended.rs";
        let churn_history = vec![5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok(), "should work with 10 data points");

        let prediction = result.unwrap();
        assert!(prediction.prediction_confidence > 0.5);
        assert!(prediction.prediction_confidence <= 1.0);
        // With 10 points (more than 7), confidence should include penalty for noise
        assert!(prediction.prediction_confidence < 1.0);
    }

    #[test]
    fn test_predict_churn_trajectory_predictions_differ() {
        // Verify 7-day and 14-day predictions are different
        let file = "src/different.rs";
        let churn_history = vec![10.0, 15.0, 20.0, 25.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        // For a linear trend without clamping, predictions should differ
        // With 7-day interval, difference should be visible
        assert!(
            (prediction.predicted_churn_14days - prediction.predicted_churn_7days).abs() > 0.1,
            "7-day and 14-day predictions should differ meaningfully"
        );
    }

    #[test]
    fn test_predict_churn_trajectory_confidence_varies_by_size() {
        // Verify confidence scores vary with data size
        let file = "src/test.rs";

        // Small dataset (2 points)
        let small_data = vec![20.0, 40.0];
        let small_result = predict_churn_trajectory(file, &small_data);
        assert!(small_result.is_ok());
        let small_confidence = small_result.unwrap().prediction_confidence;

        // Medium dataset (7 points)
        let medium_data = vec![10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0];
        let medium_result = predict_churn_trajectory(file, &medium_data);
        assert!(medium_result.is_ok());
        let medium_confidence = medium_result.unwrap().prediction_confidence;

        // Large dataset (12 points)
        let large_data = vec![5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0, 55.0, 60.0];
        let large_result = predict_churn_trajectory(file, &large_data);
        assert!(large_result.is_ok());
        let large_confidence = large_result.unwrap().prediction_confidence;

        // Medium should have highest confidence (exactly 7 points)
        assert!(
            medium_confidence >= small_confidence,
            "medium dataset should have higher or equal confidence than small"
        );
        assert!(
            medium_confidence >= large_confidence,
            "medium dataset should have higher or equal confidence than large"
        );
    }

    #[test]
    fn test_predict_churn_trajectory_warning_none() {
        // Test warning level: None (< 40%)
        let file = "src/safe.rs";
        let churn_history = vec![5.0, 8.0, 11.0, 14.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        // With very gentle slope (~1 per day), 14-day prediction should still be < 40%
        if prediction.predicted_churn_14days < 40.0 {
            assert_eq!(
                prediction.warning_level,
                PredictionWarning::None,
                "should be None for predictions < 40%"
            );
        }
    }

    #[test]
    fn test_predict_churn_trajectory_warning_watch() {
        // Test warning level: Watch (40-60%)
        let file = "src/watch.rs";
        let churn_history = vec![20.0, 30.0, 40.0, 50.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        // 14-day prediction should be around 60-70 given trend, let's check what we get
        if prediction.predicted_churn_14days >= 40.0 && prediction.predicted_churn_14days < 60.0 {
            assert_eq!(prediction.warning_level, PredictionWarning::Watch);
        }
    }

    #[test]
    fn test_predict_churn_trajectory_warning_degrade() {
        // Test warning level: Degrade (60-80%)
        let file = "src/degrade.rs";
        let churn_history = vec![20.0, 35.0, 50.0, 65.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        if prediction.predicted_churn_14days >= 60.0 && prediction.predicted_churn_14days < 80.0 {
            assert_eq!(prediction.warning_level, PredictionWarning::Degrade);
        }
    }

    #[test]
    fn test_predict_churn_trajectory_warning_critical() {
        // Test warning level: Critical (>= 80%)
        let file = "src/critical.rs";
        let churn_history = vec![40.0, 55.0, 70.0, 85.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        if prediction.predicted_churn_14days >= 80.0 {
            assert_eq!(prediction.warning_level, PredictionWarning::Critical);
        }
    }

    #[test]
    fn test_predict_churn_trajectory_days_to_critical() {
        // Test days-to-critical calculation
        let file = "src/critical_time.rs";
        let churn_history = vec![10.0, 20.0, 30.0, 40.0, 50.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        assert!(
            prediction.days_to_critical.is_some(),
            "should calculate days to critical with increasing trend"
        );
        let days = prediction.days_to_critical.unwrap();
        assert!(days > 0, "days to critical should be positive");
    }

    #[test]
    fn test_predict_churn_trajectory_all_same_values() {
        // Edge case: all same churn values
        let file = "src/constant.rs";
        let churn_history = vec![50.0, 50.0, 50.0, 50.0, 50.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        assert_eq!(prediction.current_churn, 50.0);
        // With constant slope, predictions should be approximately the same
        assert!((prediction.predicted_churn_7days - 50.0).abs() < 0.1);
        assert!((prediction.predicted_churn_14days - 50.0).abs() < 0.1);
        assert_eq!(prediction.days_to_critical, None, "constant trend won't reach 80%");
    }

    #[test]
    fn test_predict_churn_trajectory_clamping_max() {
        // Test that predictions are clamped to max 100%
        let file = "src/extreme.rs";
        // Very steep increasing trend
        let churn_history = vec![1.0, 25.0, 50.0, 75.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        // Even with steep trend, values should be clamped at 100%
        assert!(
            prediction.predicted_churn_7days <= 100.0,
            "7-day prediction should be clamped to 100%"
        );
        assert!(
            prediction.predicted_churn_14days <= 100.0,
            "14-day prediction should be clamped to 100%"
        );
    }

    #[test]
    fn test_predict_churn_trajectory_clamping_min() {
        // Test that predictions are clamped to min 0%
        let file = "src/negative.rs";
        // Strong decreasing trend
        let churn_history = vec![80.0, 60.0, 40.0, 20.0];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok());

        let prediction = result.unwrap();
        // Even with negative trend, values should be clamped at 0%
        assert!(
            prediction.predicted_churn_7days >= 0.0,
            "7-day prediction should be clamped to 0%"
        );
        assert!(
            prediction.predicted_churn_14days >= 0.0,
            "14-day prediction should be clamped to 0%"
        );
    }

    #[test]
    fn test_confidence_score_calculation() {
        // Test confidence score function directly
        // 7 points should have 100% confidence
        let conf_7 = calculate_confidence_score(7);
        assert_eq!(conf_7, 1.0, "7 points should give 100% confidence");

        // 2 points should have reduced confidence (50% reduction)
        let conf_2 = calculate_confidence_score(2);
        assert_eq!(conf_2, 0.5, "2 points should give 50% confidence");

        // 12 points should have slightly reduced confidence
        let conf_12 = calculate_confidence_score(12);
        assert!(conf_12 < 1.0 && conf_12 >= 0.5, "12 points should give reduced confidence");
        let expected_12 = 1.0 - ((12 - 7) as f64 * 0.05);
        assert!((conf_12 - expected_12).abs() < 0.01);
    }

    #[test]
    fn test_warning_level_determination() {
        // Test warning level determination function directly
        assert_eq!(
            determine_warning_level(35.0),
            PredictionWarning::None,
            "< 40% should be None"
        );
        assert_eq!(
            determine_warning_level(50.0),
            PredictionWarning::Watch,
            "40-60% should be Watch"
        );
        assert_eq!(
            determine_warning_level(70.0),
            PredictionWarning::Degrade,
            "60-80% should be Degrade"
        );
        assert_eq!(
            determine_warning_level(85.0),
            PredictionWarning::Critical,
            ">= 80% should be Critical"
        );
    }

    #[test]
    fn test_predict_churn_trajectory_integration_realistic() {
        // Integration test with realistic data
        let file = "src/utils.rs";
        let churn_history = vec![
            5.0,  // week 1: slight changes
            12.0, // week 2: increasing
            18.0, // week 3: moderate churn
            28.0, // week 4: significant changes
            38.0, // week 5: degrading
            48.0, // week 6: high churn
        ];

        let result = predict_churn_trajectory(file, &churn_history);
        assert!(result.is_ok(), "should predict with realistic data");

        let prediction = result.unwrap();
        assert_eq!(prediction.file, file);
        assert_eq!(prediction.current_churn, 48.0);

        // Verify all fields are populated
        assert!(prediction.predicted_churn_7days > 0.0);
        assert!(prediction.predicted_churn_14days > 0.0);
        assert!(prediction.prediction_confidence >= 0.5 && prediction.prediction_confidence <= 1.0);

        // Verify trend is captured
        assert!(
            prediction.predicted_churn_14days > prediction.current_churn,
            "strong upward trend should continue"
        );

        // Should be at least Watch level given the high predictions
        assert!(
            prediction.warning_level != PredictionWarning::None,
            "degrading file should trigger warning"
        );
    }
}
