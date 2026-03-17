//! Predictive analytics engine
//!
//! Forecasts churn degradation using linear regression and calculates
//! days until files become unmaintainable. Generates prediction warnings
//! and confidence scores to help identify files that will likely need
//! refactoring in the future.

use crate::models::{AnalysisResult, FileMetrics, ChurnPrediction, PredictionWarning, Prediction, AlertSeverity};
use std::collections::HashMap;

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

/// Predictor struct for high-level prediction operations
pub struct Predictor;

impl Predictor {
    /// Predict which files will become critical within N days
    pub fn predict_critical(analysis: &AnalysisResult, days: usize, threshold: f64) -> Vec<Prediction> {
        let mut predictions = Vec::new();

        for (file, metrics) in &analysis.file_metrics {
            if let Some(prediction) = Self::analyze_file(file, metrics, days, threshold) {
                predictions.push(prediction);
            }
        }

        // Sort by severity (Critical first) then by confidence
        predictions.sort_by(|a, b| {
            b.severity.cmp(&a.severity)
                .then_with(|| b.confidence.partial_cmp(&a.confidence).unwrap())
        });

        predictions
    }

    fn analyze_file(file: &str, metrics: &FileMetrics, days: usize, threshold: f64) -> Option<Prediction> {
        let churns: Vec<f64> = metrics.churn_history.iter()
            .map(|c| c.churn_percentage)
            .collect();

        if churns.len() < 2 {
            return None;
        }

        // Calculate churn velocity (rate of change)
        let velocity = Self::calculate_velocity(&churns);

        // Calculate average recent churn (last 3 entries or all)
        let recent_count = churns.len().min(3);
        let avg_churn: f64 = churns.iter()
            .rev()
            .take(recent_count)
            .sum::<f64>() / recent_count as f64;

        // Predict days to critical (churn > 80%)
        let days_to_critical = Self::predict_days_to_threshold(avg_churn, velocity, 80.0);

        // Determine severity based on days and threshold
        let (severity, confidence) = if days_to_critical <= 7.0 {
            (AlertSeverity::Critical, 0.9)
        } else if days_to_critical <= (days / 2) as f64 {
            (AlertSeverity::Warning, 0.75)
        } else if avg_churn >= threshold * 100.0 {
            (AlertSeverity::Warning, 0.6)
        } else {
            return None; // Not at risk within timeframe
        };

        Some(Prediction {
            file: file.to_string(),
            severity,
            message: format!(
                "Alto riesgo: churn promedio {:.1}% con velocidad {:.2}",
                avg_churn, velocity
            ),
            days_to_unmaintainable: Some(days_to_critical as i32),
            confidence,
        })
    }

    fn calculate_velocity(churns: &[f64]) -> f64 {
        if churns.len() < 2 {
            return 0.0;
        }

        let mut changes = Vec::new();
        for i in 1..churns.len() {
            changes.push(churns[i] - churns[i - 1]);
        }

        changes.iter().sum::<f64>() / changes.len() as f64
    }

    fn predict_days_to_threshold(current: f64, velocity: f64, threshold: f64) -> f64 {
        if velocity <= 0.0 {
            return 999.0; // Stable or improving, not heading to threshold
        }

        let remaining = threshold - current;
        if remaining <= 0.0 {
            return 0.0; // Already at or above threshold
        }

        // Assuming velocity is per-commit, estimate ~2 commits/day average
        let commits_needed = remaining / velocity;
        commits_needed / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChurnMetric, LOCMetric};
    use chrono::{Duration, Utc};

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

    // Tests for Predictor::predict_critical
    #[test]
    fn test_predict_critical_high_churn() {
        use crate::models::{FileMetrics, ChurnMetric};
        let metrics = FileMetrics {
            file: "src/main.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![
                ChurnMetric { file: "src/main.rs".to_string(), timestamp: Utc::now() - Duration::days(14), churn_percentage: 45.0 },
                ChurnMetric { file: "src/main.rs".to_string(), timestamp: Utc::now() - Duration::days(7), churn_percentage: 65.0 },
                ChurnMetric { file: "src/main.rs".to_string(), timestamp: Utc::now(), churn_percentage: 75.0 },
            ],
            authors: vec![],
            complexity_history: vec![],
        };

        let mut file_metrics = HashMap::new();
        file_metrics.insert("src/main.rs".to_string(), metrics);

        let analysis = AnalysisResult {
            repository_path: ".".to_string(),
            analysis_period: "6m".to_string(),
            files_analyzed: 1,
            total_commits: 10,
            authors_count: 1,
            file_metrics,
            predictions: vec![],
            overall_trend: crate::models::Trend::Stable,
            timestamp: Utc::now(),
        };

        let predictions = Predictor::predict_critical(&analysis, 30, 0.5);
        assert!(!predictions.is_empty());
        assert_eq!(predictions[0].severity, AlertSeverity::Critical);
        assert!(predictions[0].confidence >= 0.8);
    }

    #[test]
    fn test_calculate_velocity_increasing() {
        let churns = vec![20.0, 40.0, 60.0];
        let velocity = Predictor::calculate_velocity(&churns);
        assert!(velocity > 0.0);
    }

    #[test]
    fn test_calculate_velocity_stable() {
        let churns = vec![50.0, 50.0, 50.0];
        let velocity = Predictor::calculate_velocity(&churns);
        assert!((velocity).abs() < 0.01);
    }
}
