"""
agent.py — Punto de entrada del Warden Agent.

NOTA ARQUITECTÓNICA:
  El Warden Agent usa un flujo DETERMINISTA, no un LlmAgent autónomo.
  Cerebro ya sabe qué acción quiere ejecutar → el sidecar la ejecuta
  y pasa el resultado crudo al LLM solo para análisis/síntesis.

  Esto es intencionalmente más simple y predecible que un agente
  con LlmAgent.run() porque:
    1. La selección de acción ya la hizo Cerebro.
    2. El Core Rust ya tiene toda la lógica de escaneo.
    3. El LLM solo agrega comprensión del resultado, no decisiones.

  Si en el futuro se requiere razonamiento multi-paso (e.g., "escanea Y
  si encuentras secretos también corre audit-deps"), ese orquestamiento
  debe vivir en Cerebro, no en Warden.

  Ver: cerebro_bridge.py (flujo completo)
       llm_client.py (análisis multi-proveedor)
       tools.py (wrappers del Core Rust)
"""

# Este módulo se mantiene como punto de importación limpio del agente.
# La lógica real está distribuida en los módulos especializados.
from .cerebro_bridge import handle_command, report_to_cerebro
from .llm_client import analyze_result
from .tools import ACTION_MAP

__all__ = ["handle_command", "report_to_cerebro", "analyze_result", "ACTION_MAP"]
