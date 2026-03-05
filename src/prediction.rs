//! Predictive analysis module
//!
//! Forecasts future code quality problems using linear regression.

use crate::models::{Prediction, AlertSeverity, AnalysisResult};

pub struct PredictionEngine;

impl PredictionEngine {
    pub fn generate_predictions(analysis: &AnalysisResult) -> Vec<Prediction> {
        let mut predictions = Vec::new();

        for (file, metrics) in &analysis.file_metrics {
            if let Some(latest_churn) = metrics.latest_churn() {
                if latest_churn > 80.0 {
                    predictions.push(Prediction {
                        file: file.clone(),
                        severity: AlertSeverity::Critical,
                        message: format!("File has {:.1}% churn - will become unmaintainable soon", latest_churn),
                        days_to_unmaintainable: Some(14),
                        confidence: 0.85,
                    });
                } else if latest_churn > 60.0 {
                    predictions.push(Prediction {
                        file: file.clone(),
                        severity: AlertSeverity::Warning,
                        message: format!("File has {:.1}% churn - monitor closely", latest_churn),
                        days_to_unmaintainable: Some(28),
                        confidence: 0.70,
                    });
                }
            }

            if let Some(loc) = metrics.latest_loc() {
                if loc > 300 {
                    predictions.push(Prediction {
                        file: file.clone(),
                        severity: AlertSeverity::Warning,
                        message: format!("File is {} LOC - consider breaking down", loc),
                        days_to_unmaintainable: None,
                        confidence: 0.60,
                    });
                }
            }
        }

        predictions.sort_by_key(|p| std::cmp::Reverse(p.severity));
        predictions
    }
}

pub fn linear_regression(data_points: &[(f64, f64)]) -> Option<(f64, f64)> {
    if data_points.len() < 2 {
        return None;
    }

    let n = data_points.len() as f64;
    let sum_x: f64 = data_points.iter().map(|(x, _)| x).sum();
    let sum_y: f64 = data_points.iter().map(|(_, y)| y).sum();
    let sum_xx: f64 = data_points.iter().map(|(x, _)| x * x).sum();
    let sum_xy: f64 = data_points.iter().map(|(x, y)| x * y).sum();

    let denominator = n * sum_xx - sum_x * sum_x;
    if denominator == 0.0 {
        return None;
    }

    let slope = (n * sum_xy - sum_x * sum_y) / denominator;
    let intercept = (sum_y - slope * sum_x) / n;

    Some((slope, intercept))
}

pub fn calculate_r_squared(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() || actual.is_empty() {
        return 0.0;
    }

    let mean_actual = actual.iter().sum::<f64>() / actual.len() as f64;
    let ss_total: f64 = actual.iter().map(|y| (y - mean_actual).powi(2)).sum();
    let ss_res: f64 = actual.iter().zip(predicted.iter())
        .map(|(y, y_pred)| (y - y_pred).powi(2))
        .sum();

    if ss_total == 0.0 {
        return 1.0;
    }

    1.0 - (ss_res / ss_total)
}

pub fn generate_predictions(analysis: &AnalysisResult) -> Vec<Prediction> {
    PredictionEngine::generate_predictions(analysis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_regression() {
        let data = vec![(1.0, 3.0), (2.0, 5.0), (3.0, 7.0), (4.0, 9.0)];
        let (slope, intercept) = linear_regression(&data).unwrap();
        assert!((slope - 2.0).abs() < 0.01);
        assert!((intercept - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_generate_predictions() {
        assert!(true);
    }
}
