use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Evento que Warden envía al Cerebro
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: String,
    pub source: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub severity: String,
    pub timestamp: String,
    pub payload: HashMap<String, serde_json::Value>,
}

/// Comando que el Cerebro envía a Warden
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorCommand {
    pub action: String,
    pub target: Option<String>,
    pub options: Option<HashMap<String, serde_json::Value>>,
    pub request_id: Option<String>,
}

/// Respuesta de Warden al Cerebro tras recibir un comando
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAck {
    pub request_id: Option<String>,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}
