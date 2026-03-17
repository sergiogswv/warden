/// Configuración del agente Warden para conectarse al Cerebro
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// URL del Cerebro (Orquestador)
    pub cerebro_url: String,
    /// Puerto en el que Warden expone su servidor HTTP
    pub port: u16,
    /// Si debe reportar al Cerebro al terminar un análisis
    pub report_enabled: bool,
}

impl AgentConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        Self {
            cerebro_url: std::env::var("CEREBRO_URL")
                .unwrap_or_else(|_| "http://localhost:4000".to_string()),
            port: std::env::var("WARDEN_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(4003),
            report_enabled: std::env::var("CEREBRO_REPORT_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
        }
    }

    pub fn events_endpoint(&self) -> String {
        format!("{}/api/events", self.cerebro_url)
    }
}
