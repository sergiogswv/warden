use axum::{extract::State, routing::post, Json, Router};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::agent_config::AgentConfig;
use crate::agent_models::{CommandAck, OrchestratorCommand};
use crate::agent_reporter::{report_event, build_analysis_payload};
use crate::{git_parser, metrics, predictor, churn_reporter, risk_scorer};
use std::path::PathBuf;

/// Estado compartido del servidor
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AgentConfig>,
    /// Guarda la última acción recibida (útil para debug/status)
    pub last_command: Arc<Mutex<Option<String>>>,
}

/// Realiza el análisis base del repositorio (parsing + metrics)
fn perform_base_analysis(repo_path: &PathBuf) -> anyhow::Result<(usize, std::collections::HashMap<String, crate::models::FileMetrics>)> {
    println!("🔍 Parsing Git history...");
    let commits = git_parser::parse_git_history(repo_path, "6m").unwrap_or_else(|_| vec![]);
    println!("   ✓ {} commits analyzed", commits.len());

    println!("📈 Calculating file metrics...");
    let file_metrics = metrics::process_commits(&commits)?;
    println!("   ✓ {} files analyzed", file_metrics.len());

    Ok((commits.len(), file_metrics))
}

/// Handler del endpoint POST /command
/// Recibe instrucciones del Cerebro
async fn handle_command(
    State(state): State<AppState>,
    Json(cmd): Json<OrchestratorCommand>,
) -> Json<CommandAck> {
    println!(
        "📨 [Cerebro→Warden] action={} target={:?} request_id={:?}",
        cmd.action,
        cmd.target,
        cmd.request_id
    );

    *state.last_command.lock().await = Some(cmd.action.clone());

    let ack = match cmd.action.as_str() {
        "scan" => {
            let target = cmd.target.as_deref().unwrap_or(".");
            println!("🔍 Iniciando escaneo sobre: {}", target);

            // Reporte al cerebro de que comenzamos el escaneo
            let payload = build_analysis_payload(
                "Escaneo iniciado por Cerebro",
                Some(target),
                None,
            );
            let _ = report_event(&state.config, "scan_started", "info", payload).await;

            CommandAck {
                request_id: cmd.request_id,
                status: "accepted".to_string(),
                result: Some(serde_json::json!({ "target": target, "action": "scan" })),
                error: None,
            }
        }

        "audit-deps" => {
            println!("📦 Auditando dependencias...");
            // TODO: integrar con cargo audit o similar
            let payload = build_analysis_payload(
                "Auditoría de dependencias iniciada",
                None,
                Some("Ejecutar 'cargo audit' manualmente para ver vulnerabilidades"),
            );
            let _ = report_event(&state.config, "audit_deps_started", "info", payload).await;

            CommandAck {
                request_id: cmd.request_id,
                status: "accepted".to_string(),
                result: Some(serde_json::json!({ "action": "audit-deps" })),
                error: None,
            }
        }

        "check-secrets" => {
            let target = cmd.target.as_deref().unwrap_or(".");
            println!("🔑 Verificando secretos en: {}", target);
            // TODO: integrar con detección de secretos
            let payload = build_analysis_payload(
                "Verificación de secretos iniciada",
                Some(target),
                Some("Revisar archivos .env y configuraciones"),
            );
            let _ = report_event(&state.config, "check_secrets_started", "info", payload).await;

            CommandAck {
                request_id: cmd.request_id,
                status: "accepted".to_string(),
                result: Some(serde_json::json!({ "target": target, "action": "check-secrets" })),
                error: None,
            }
        }

        "report" => {
            println!("📊 Generando reporte de seguridad...");
            let payload = build_analysis_payload(
                "Reporte de seguridad generado",
                None,
                Some("Ver análisis histórico con 'warden .'"),
            );
            let _ = report_event(&state.config, "report_generated", "info", payload).await;

            CommandAck {
                request_id: cmd.request_id,
                status: "completed".to_string(),
                result: Some(serde_json::json!({ "action": "report", "status": "ok" })),
                error: None,
            }
        }

        "status" => {
            let last = state.last_command.lock().await.clone();
            CommandAck {
                request_id: cmd.request_id,
                status: "completed".to_string(),
                result: Some(serde_json::json!({
                    "agent": "warden",
                    "version": "0.1.0",
                    "last_command": last,
                    "ready": true
                })),
                error: None,
            }
        }

        "predict-critical" => {
            let target = cmd.target.as_deref().unwrap_or(".");
            let days = 30;
            let threshold = 0.5;

            println!("🔮 Analizando archivos críticos para: {}", target);

            // Reporte de inicio
            let payload = build_analysis_payload(
                &format!("Predicción de críticos iniciada para {}", target),
                Some(target),
                Some(&format!("Analizando {} días adelante con threshold {}", days, threshold)),
            );
            let _ = report_event(&state.config, "predict_critical_started", "info", payload).await;

            // Ejecutar análisis real
            let repo_path = if std::path::Path::new(target).is_absolute() {
                PathBuf::from(target)
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                    .join(target)
            };

            match perform_base_analysis(&repo_path) {
                Ok((total_commits, file_metrics)) => {
                    use chrono::Utc;
                    use crate::models::{AnalysisResult, Trend};

                    let analysis = AnalysisResult {
                        repository_path: repo_path.to_string_lossy().to_string(),
                        analysis_period: "6m".to_string(),
                        files_analyzed: file_metrics.len(),
                        total_commits,
                        authors_count: 0,
                        file_metrics,
                        predictions: vec![],
                        overall_trend: Trend::Stable,
                        timestamp: Utc::now(),
                    };

                    // Ejecutar predictor
                    let predictions = predictor::Predictor::predict_critical(&analysis, days, threshold);

                    println!("📊 Se encontraron {} archivos en riesgo", predictions.len());

                    // Reportar resultados
                    let results_payload = serde_json::json!({
                        "target": target,
                        "action": "predict-critical",
                        "files_at_risk": predictions.len(),
                        "predictions": predictions.iter().take(10).map(|p| {
                            serde_json::json!({
                                "file": p.file,
                                "severity": format!("{:?}", p.severity),
                                "confidence": p.confidence,
                                "days_to_unmaintainable": p.days_to_unmaintainable
                            })
                        }).collect::<Vec<_>>()
                    });

                    // Convertir serde_json::Map a HashMap para report_event
                    let mut event_payload = std::collections::HashMap::new();
                    event_payload.insert("result".to_string(), results_payload.clone());
                    let _ = report_event(&state.config, "predict_critical_completed", "info", event_payload).await;

                    CommandAck {
                        request_id: cmd.request_id,
                        status: "completed".to_string(),
                        result: Some(results_payload),
                        error: None,
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error en predict-critical: {}", e);
                    let error_payload = build_analysis_payload(
                        &format!("Error en predicción: {}", e),
                        Some(target),
                        None,
                    );
                    let _ = report_event(&state.config, "predict_critical_error", "error", error_payload).await;

                    CommandAck {
                        request_id: cmd.request_id,
                        status: "error".to_string(),
                        result: None,
                        error: Some(e.to_string()),
                    }
                }
            }
        }

        "risk-assess" => {
            let target = cmd.target.as_deref().unwrap_or(".");

            println!("📊 Evaluando riesgos para: {}", target);

            let payload = build_analysis_payload(
                &format!("Evaluación de riesgos iniciada para {}", target),
                Some(target),
                Some("Calculando scores compuestos: churn × loc × authors × complexity"),
            );
            let _ = report_event(&state.config, "risk_assess_started", "info", payload).await;

            // Ejecutar análisis real
            let repo_path = if std::path::Path::new(target).is_absolute() {
                PathBuf::from(target)
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                    .join(target)
            };

            match perform_base_analysis(&repo_path) {
                Ok((total_commits, file_metrics)) => {
                    // Calcular risk scores
                    match risk_scorer::calculate_risk_scores(&file_metrics, total_commits) {
                        Ok(risk_scores) => {
                            println!("📊 Se evaluaron {} archivos", risk_scores.len());

                            // Agrupar por nivel de riesgo
                            let critical_count = risk_scores.iter().filter(|r| r.risk_level == crate::models::RiskLevel::Critical).count();
                            let alert_count = risk_scores.iter().filter(|r| r.risk_level == crate::models::RiskLevel::Alert).count();

                            // Reportar resultados
                            let results_payload = serde_json::json!({
                                "target": target,
                                "action": "risk-assess",
                                "total_files": risk_scores.len(),
                                "critical_files": critical_count,
                                "alert_files": alert_count,
                                "top_risks": risk_scores.iter().take(10).map(|r| {
                                    serde_json::json!({
                                        "file": r.file,
                                        "risk_value": r.risk_value,
                                        "risk_level": format!("{}", r.risk_level),
                                        "churn_percentage": r.churn_percentage,
                                        "loc": r.loc,
                                        "trend": format!("{}", r.trend),
                                        "recommendation": r.recommendation
                                    })
                                }).collect::<Vec<_>>()
                            });

                            // Convertir serde_json::Map a HashMap para report_event
                            let mut event_payload = std::collections::HashMap::new();
                            event_payload.insert("result".to_string(), results_payload.clone());
                            let _ = report_event(&state.config, "risk_assess_completed", "info", event_payload).await;

                            CommandAck {
                                request_id: cmd.request_id,
                                status: "completed".to_string(),
                                result: Some(results_payload),
                                error: None,
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Error calculando risk scores: {}", e);
                            let error_payload = build_analysis_payload(
                                &format!("Error en risk assessment: {}", e),
                                Some(target),
                                None,
                            );
                            let _ = report_event(&state.config, "risk_assess_error", "error", error_payload).await;

                            CommandAck {
                                request_id: cmd.request_id,
                                status: "error".to_string(),
                                result: None,
                                error: Some(e.to_string()),
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error en risk-assess: {}", e);
                    let error_payload = build_analysis_payload(
                        &format!("Error en evaluación de riesgos: {}", e),
                        Some(target),
                        None,
                    );
                    let _ = report_event(&state.config, "risk_assess_error", "error", error_payload).await;

                    CommandAck {
                        request_id: cmd.request_id,
                        status: "error".to_string(),
                        result: None,
                        error: Some(e.to_string()),
                    }
                }
            }
        }

        "churn-report" => {
            let target = cmd.target.as_deref().unwrap_or(".");
            let weeks = 12;

            println!("📈 Generando reporte de churn para: {}", target);

            let payload = build_analysis_payload(
                &format!("Reporte de churn generado para {} semanas", weeks),
                Some(target),
                Some("Analizando tendencias semanales y patrones"),
            );
            let _ = report_event(&state.config, "churn_report_started", "info", payload).await;

            // Ejecutar análisis real
            let repo_path = if std::path::Path::new(target).is_absolute() {
                PathBuf::from(target)
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                    .join(target)
            };

            match perform_base_analysis(&repo_path) {
                Ok((total_commits, file_metrics)) => {
                    use chrono::Utc;
                    use crate::models::{AnalysisResult, Trend};

                    let analysis = AnalysisResult {
                        repository_path: repo_path.to_string_lossy().to_string(),
                        analysis_period: "6m".to_string(),
                        files_analyzed: file_metrics.len(),
                        total_commits,
                        authors_count: 0,
                        file_metrics,
                        predictions: vec![],
                        overall_trend: Trend::Stable,
                        timestamp: Utc::now(),
                    };

                    // Generar reporte de churn
                    let report = churn_reporter::ChurnReporter::generate_report(&analysis, weeks);

                    println!("📊 Reporte de churn generado: {} semanas analizadas", report.weekly_trends.len());

                    // Reportar resultados
                    let results_payload = serde_json::json!({
                        "target": target,
                        "action": "churn-report",
                        "weeks": weeks,
                        "summary": {
                            "total_commits": report.summary.total_commits,
                            "avg_churn": report.summary.avg_churn,
                            "max_churn": report.summary.max_churn,
                            "trend_direction": report.summary.trend_direction
                        },
                        "weekly_trends": report.weekly_trends.iter().map(|w| {
                            serde_json::json!({
                                "week_start": w.week_start,
                                "avg_churn": w.avg_churn,
                                "commit_count": w.commit_count,
                                "most_changed_file": w.most_changed_file
                            })
                        }).collect::<Vec<_>>(),
                        "top_churned_files": report.top_churned_files.iter().take(10).map(|f| {
                            serde_json::json!({
                                "file": f.file,
                                "total_churn": f.total_churn,
                                "change_count": f.change_count
                            })
                        }).collect::<Vec<_>>(),
                        "patterns": report.patterns.iter().map(|p| {
                            serde_json::json!({
                                "description": p.description,
                                "severity": p.severity
                            })
                        }).collect::<Vec<_>>()
                    });

                    // Convertir serde_json::Map a HashMap para report_event
                    let mut event_payload = std::collections::HashMap::new();
                    event_payload.insert("result".to_string(), results_payload.clone());
                    let _ = report_event(&state.config, "churn_report_completed", "info", event_payload).await;

                    CommandAck {
                        request_id: cmd.request_id,
                        status: "completed".to_string(),
                        result: Some(results_payload),
                        error: None,
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error en churn-report: {}", e);
                    let error_payload = build_analysis_payload(
                        &format!("Error en reporte de churn: {}", e),
                        Some(target),
                        None,
                    );
                    let _ = report_event(&state.config, "churn_report_error", "error", error_payload).await;

                    CommandAck {
                        request_id: cmd.request_id,
                        status: "error".to_string(),
                        result: None,
                        error: Some(e.to_string()),
                    }
                }
            }
        }

        unknown => {
            eprintln!("⚠️  Acción desconocida: {}", unknown);
            CommandAck {
                request_id: cmd.request_id,
                status: "rejected".to_string(),
                result: None,
                error: Some(format!("Acción '{}' no reconocida", unknown)),
            }
        }
    };

    Json(ack)
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "agent": "warden-core",
        "version": "0.1.0"
    }))
}

/// Levanta el servidor HTTP para recibir comandos del Cerebro
pub async fn start_server(config: AgentConfig) -> anyhow::Result<()> {
    let port = config.port;
    let report_enabled = config.report_enabled;
    let config_arc = Arc::new(config);

    let state = AppState {
        config: Arc::clone(&config_arc),
        last_command: Arc::new(Mutex::new(None)),
    };

    let app = Router::new()
        .route("/health", axum::routing::get(health_check))
        .route("/command", post(handle_command))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("🛡️  Warden Agent escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Enviar evento ready cuando el servidor está levantado
    if report_enabled {
        let _ = report_ready_event(&config_arc).await;
    }

    axum::serve(listener, app).await?;
    Ok(())
}

async fn report_ready_event(config: &Arc<AgentConfig>) -> anyhow::Result<()> {
    use crate::agent_reporter::report_event;
    let mut payload = std::collections::HashMap::new();
    payload.insert("message".to_string(), serde_json::Value::String("Warden listo para escaneo de seguridad".to_string()));
    payload.insert("version".to_string(), serde_json::Value::String("0.1.0".to_string()));

    let _ = report_event(
        config,
        "warden_ready",
        "info",
        payload
    ).await;
    Ok(())
}
