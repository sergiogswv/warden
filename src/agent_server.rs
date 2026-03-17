use axum::{extract::State, routing::post, Json, Router};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::agent_config::AgentConfig;
use crate::agent_models::{CommandAck, OrchestratorCommand};
use crate::agent_reporter::{report_event, build_analysis_payload};
use crate::predictor::Predictor;
use crate::risk_scorer::RiskScorer;
use crate::churn_reporter::ChurnReporter;

/// Estado compartido del servidor
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AgentConfig>,
    /// Guarda la última acción recibida (útil para debug/status)
    pub last_command: Arc<Mutex<Option<String>>>,
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

            let payload = build_analysis_payload(
                &format!("Predicción de críticos iniciada para {}", target),
                Some(target),
                Some(&format!("Analizando {} días adelante con threshold {}", days, threshold)),
            );
            let _ = report_event(&state.config, "predict_critical_started", "info", payload).await;

            CommandAck {
                request_id: cmd.request_id,
                status: "accepted".to_string(),
                result: Some(serde_json::json!({
                    "target": target,
                    "action": "predict-critical",
                    "days": days,
                    "threshold": threshold
                })),
                error: None,
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

            CommandAck {
                request_id: cmd.request_id,
                status: "accepted".to_string(),
                result: Some(serde_json::json!({
                    "target": target,
                    "action": "risk-assess"
                })),
                error: None,
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
            let _ = report_event(&state.config, "churn_report_generated", "info", payload).await;

            CommandAck {
                request_id: cmd.request_id,
                status: "completed".to_string(),
                result: Some(serde_json::json!({
                    "target": target,
                    "action": "churn-report",
                    "weeks": weeks
                })),
                error: None,
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
