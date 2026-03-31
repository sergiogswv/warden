"""
tools.py — Wrappers HTTP hacia el Warden Core (Rust, :4003).

FILOSOFÍA:
  Estas funciones NO toman decisiones. Solo ejecutan la acción que
  Cerebro ya decidió y persisten el resultado crudo en memoria.
  El análisis/razonamiento lo hace el LLM en cerebro_bridge.py.

  Flujo:
    Cerebro solicita action  →  tool llama al Core Rust
    Core retorna raw JSON    →  tool persiste en SQLite
    raw JSON sube al bridge  →  LLM analiza y produce síntesis
    síntesis va a Cerebro    →  Cerebro decide qué hacer

Contratos (mismo que el Core Rust):
  - Input:  { action, target? }
  - Output: CommandAck { status, result?, error? }
"""

import time
import httpx
from typing import Optional

from .settings import settings
from . import memory

CORE = settings.warden_core_url


# ──────────────────────────────────────────────
# Helper interno de llamada al Core Rust
# ──────────────────────────────────────────────

async def call_core(action: str, target: Optional[str] = None, options: Optional[dict] = None) -> dict:
    """
    Envía un OrchestratorCommand al Warden Core (Rust).
    Es el único punto de contacto con el proceso Rust.
    Maneja errores de red sin crashear el sidecar.
    """
    payload: dict = {"action": action}
    if target:
        payload["target"] = target
    if options:
        payload["options"] = options

    try:
        async with httpx.AsyncClient(timeout=120.0) as client:
            resp = await client.post(f"{CORE}/command", json=payload)
            resp.raise_for_status()
            return resp.json()
    except httpx.ConnectError:
        return {"status": "error", "error": f"Warden Core no disponible en {CORE}"}
    except httpx.TimeoutException:
        return {"status": "error", "error": "Timeout al llamar a Warden Core (>120s)"}
    except Exception as exc:
        return {"status": "error", "error": str(exc)}


# ──────────────────────────────────────────────
# Acciones disponibles con persistencia
# Cada función = 1 acción del Core + persistencia en SQLite
# ──────────────────────────────────────────────

async def execute_scan(target: str = ".") -> tuple[dict, int]:
    """Escaneo de seguridad. Retorna (raw_result, memory_id)."""
    start = time.monotonic()
    result = await call_core("scan", target=target)
    duration_ms = int((time.monotonic() - start) * 1000)

    severity = "error" if result.get("status") == "error" else "info"
    mid = await memory.save_finding("scan_completed", severity, result, target)
    await memory.save_analysis_run("scan", target, {"status": result.get("status")}, duration_ms)
    return result, mid


async def execute_risk_assess(target: str = ".") -> tuple[dict, int]:
    """Evaluación de riesgo compuesto. Persiste perfiles por archivo. Retorna (raw_result, memory_id)."""
    start = time.monotonic()
    result = await call_core("risk-assess", target=target)
    duration_ms = int((time.monotonic() - start) * 1000)

    # Actualizar perfiles de riesgo individuales por archivo
    if result.get("status") == "completed":
        top_risks = (result.get("result") or {}).get("top_risks", [])
        for risk in top_risks:
            await memory.update_risk_profile(
                file_path=risk.get("file", "unknown"),
                risk_value=float(risk.get("risk_value", 0.0)),
                severity="critical" if risk.get("risk_level") in ("Critical", "Alert") else "info",
            )

    severity = "error" if result.get("status") == "error" else "warning"
    mid = await memory.save_finding("risk_assess_completed", severity, result, target)
    await memory.save_analysis_run("risk-assess", target, {"status": result.get("status")}, duration_ms)
    return result, mid


async def execute_predict_critical(target: str = ".") -> tuple[dict, int]:
    """Predicción de archivos en riesgo. Retorna (raw_result, memory_id)."""
    start = time.monotonic()
    result = await call_core("predict-critical", target=target)
    duration_ms = int((time.monotonic() - start) * 1000)

    mid = await memory.save_finding("predict_critical_completed", "warning", result, target)
    await memory.save_analysis_run("predict-critical", target, {"status": result.get("status")}, duration_ms)
    return result, mid


async def execute_churn_report(target: str = ".") -> tuple[dict, int]:
    """Reporte de churn (12 semanas). Retorna (raw_result, memory_id)."""
    start = time.monotonic()
    result = await call_core("churn-report", target=target)
    duration_ms = int((time.monotonic() - start) * 1000)

    mid = await memory.save_finding("churn_report_completed", "info", result, target)
    await memory.save_analysis_run("churn-report", target, {"status": result.get("status")}, duration_ms)
    return result, mid


async def execute_audit_deps() -> tuple[dict, int]:
    """Auditoría de dependencias (CVEs). Retorna (raw_result, memory_id)."""
    result = await call_core("audit-deps")
    mid = await memory.save_finding("audit_deps_completed", "info", result, None)
    return result, mid


async def execute_check_secrets(target: str = ".") -> tuple[dict, int]:
    """Búsqueda de secretos hardcoded. Retorna (raw_result, memory_id)."""
    result = await call_core("check-secrets", target=target)
    # Si hay secretos, la severidad es crítica aunque el status sea ok
    has_secrets = bool((result.get("result") or {}).get("secrets_found"))
    severity = "critical" if has_secrets else "info"
    mid = await memory.save_finding("check_secrets_completed", severity, result, target)
    return result, mid


async def execute_report(target: str = ".") -> tuple[dict, int]:
    """Reporte completo de seguridad. Retorna (raw_result, memory_id)."""
    result = await call_core("report", target=target)
    mid = await memory.save_finding("report_generated", "info", result, target)
    return result, mid


# ──────────────────────────────────────────────
# Mapa de acciones (usado en cerebro_bridge)
# ──────────────────────────────────────────────

ACTION_MAP = {
    "scan":             execute_scan,
    "risk-assess":      execute_risk_assess,
    "predict-critical": execute_predict_critical,
    "churn-report":     execute_churn_report,
    "audit-deps":       execute_audit_deps,
    "check-secrets":    execute_check_secrets,
    "report":           execute_report,
}
