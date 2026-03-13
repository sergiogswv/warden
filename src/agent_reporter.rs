use std::collections::HashMap;
use uuid::Uuid;
use crate::agent_config::AgentConfig;
use crate::agent_models::AgentEvent;

/// Envía un evento al Cerebro (Orquestador)
pub async fn report_event(
    config: &AgentConfig,
    event_type: &str,
    severity: &str,
    payload: HashMap<String, serde_json::Value>,
) -> anyhow::Result<()> {
    if !config.report_enabled {
        return Ok(());
    }

    let event = AgentEvent {
        id: Uuid::new_v4().to_string(),
        source: "warden".to_string(),
        event_type: event_type.to_string(),
        severity: severity.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        payload,
    };

    let client = reqwest::Client::new();
    match client
        .post(&config.events_endpoint())
        .json(&event)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            println!("✅ [Cerebro] Evento reportado: {} ({})", event_type, severity);
        }
        Ok(resp) => {
            eprintln!("⚠️  [Cerebro] Respuesta inesperada: {}", resp.status());
        }
        Err(e) => {
            // No abortar el análisis si el Cerebro no está disponible
            eprintln!("⚠️  [Cerebro] No disponible: {}", e);
        }
    }

    Ok(())
}

/// Construye un payload estándar de análisis para enviar al Cerebro
pub fn build_analysis_payload(
    finding: &str,
    file: Option<&str>,
    recommendation: Option<&str>,
) -> HashMap<String, serde_json::Value> {
    let mut payload = HashMap::new();
    payload.insert("finding".to_string(), serde_json::json!(finding));
    if let Some(f) = file {
        payload.insert("file".to_string(), serde_json::json!(f));
    }
    if let Some(r) = recommendation {
        payload.insert("recommendation".to_string(), serde_json::json!(r));
    }
    payload
}
