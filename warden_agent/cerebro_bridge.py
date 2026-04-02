"""
cerebro_bridge.py — Puente entre Cerebro ↔ Warden Core ↔ LLM.

FLUJO por comando:
  1. Cerebro envía OrchestratorCommand { action, target }
  2. El action ya está definido — no necesitamos un LLM para decidir qué ejecutar.
  3. Warden Core (Rust) ejecuta la acción y retorna raw JSON.
  4. La memoria SQLite persiste el resultado.
  5. El LLM (Gemini/Claude/OpenAI) recibe:
       - El resultado crudo
       - El contexto histórico (hot_files, hallazgos críticos recientes)
     Y produce una síntesis accionable en texto.
  6. El bridge reporta a Cerebro:
       - POST /api/events con el evento estructurado
       - CommandAck con { status, result: { raw, analysis, memory_id } }

CONTRATO MANTENIDO:
  Input:  OrchestratorCommand { action, target?, options?, request_id? }
  Output: CommandAck { request_id?, status, result?, error? }
"""

import uuid
import httpx
from datetime import datetime, timezone
from typing import Optional

from .settings import settings
from . import memory
from .tools import ACTION_MAP, call_core
from .llm_client import analyze_result


# ──────────────────────────────────────────────
# Reporte al Cerebro
# ──────────────────────────────────────────────

async def report_to_cerebro(event_type: str, severity: str, payload: dict):
    """Envía un AgentEvent al endpoint POST /api/events del Cerebro."""
    event = {
        "id": str(uuid.uuid4()),
        "source": "warden",
        "type": event_type,
        "severity": severity,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "payload": payload,
    }
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            resp = await client.post(f"{settings.cerebro_url}/api/events", json=event)
            if resp.status_code >= 400:
                print(f"⚠️  [Cerebro] Respuesta inesperada: {resp.status_code}")
            else:
                print(f"✅ [Cerebro] Evento reportado: {event_type} ({severity})")
    except Exception as exc:
        print(f"⚠️  [Cerebro] No disponible: {exc}")


# ──────────────────────────────────────────────
# Handler principal de comandos
# ──────────────────────────────────────────────

async def handle_command(
    action: str,
    target: Optional[str],
    request_id: Optional[str] = None,
) -> dict:
    """
    Procesa un OrchestratorCommand completo.

    Retorna un CommandAck con:
      - raw:      resultado crudo del Core Rust
      - analysis: síntesis del LLM
      - memory_id: ID del hallazgo guardado en SQLite
    """
    print(f"🔱 [Warden] Procesando: action='{action}' target='{target}'")

    # ── 1. Status — no requiere Core ni LLM ──────────────────────────
    if action == "status":
        ctx = await memory.get_hot_files(5)
        recent = await memory.get_recent_findings(5)
        result_payload = {
            "agent": "warden-adk",
            "version": "1.0.0",
            "llm_provider": settings.llm_provider,
            "core_url": settings.warden_core_url,
            "hot_files_tracked": len(ctx),
            "recent_findings": len(recent),
        }
        await report_to_cerebro("warden_status", "info", result_payload)
        return {
            "request_id": request_id,
            "status": "completed",
            "result": result_payload,
            "error": None,
        }

    # ── 2. Acciones desconocidas ──────────────────────────────────────
    executor = ACTION_MAP.get(action)
    if not executor:
        # Intentar pasar directo al Core por si es una acción nueva
        print(f"⚠️  Acción '{action}' no en ACTION_MAP — enviando directo al Core")
        raw = await call_core(action, target=target)
        return {
            "request_id": request_id,
            "status": raw.get("status", "error"),
            "result": raw.get("result"),
            "error": raw.get("error"),
        }

    # ── 3. Ejecutar en Core Rust ──────────────────────────────────────
    try:
        # Las funciones de ACTION_MAP aceptan target como arg posicional si corresponde
        if action in ("audit-deps",):
            raw_result, memory_id = await executor()
        else:
            raw_result, memory_id = await executor(target or ".")
    except Exception as exc:
        error_msg = f"Error ejecutando '{action}': {exc}"
        print(f"❌ {error_msg}")
        await report_to_cerebro(f"warden_{action}_error", "error", {"error": error_msg, "action": action})
        return {"request_id": request_id, "status": "error", "result": None, "error": error_msg}

    # ── 4. Recuperar contexto histórico de memoria ────────────────────
    mem_context = None
    try:
        hot_files = await memory.get_hot_files(limit=5)
        recent_critical = await memory.get_recent_findings(limit=3, severity_filter="critical")
        mem_context = {
            "hot_files": hot_files,
            "recent_critical_findings": recent_critical,
        }
    except Exception:
        pass  # La memoria falla silenciosamente, no bloquea el análisis

    # ── 5. LLM analiza el resultado crudo + contexto ──────────────────
    analysis = ""
    try:
        analysis = await analyze_result(
            action=action,
            raw_result=raw_result,
            memory_context=mem_context,
        )
    except Exception as exc:
        analysis = f"[Análisis LLM no disponible: {exc}]"
        print(f"⚠️  LLM falló, se retorna raw result: {exc}")

    # ── 6. Determinar severidad final para el evento ──────────────────
    severity = _infer_severity(action, raw_result, analysis)

    # ── 7. Extraer info para Auto-Fix ──────────────────────────────────
    # Extraer finding/recomendación del LLM para el Auto-Fix
    res_data = raw_result.get("result", {})
    top_risks = res_data.get("top_risks", [])
    secrets_found = res_data.get("secrets_found", [])

    # Construir descripción del problema para Cerebro
    finding_desc = f"Análisis {action}: {len(top_risks)} riesgos detectados"
    if raw_result.get("status") == "error":
        finding_desc = f"Error en análisis {action}: {raw_result.get('error', 'Unknown error')}"
    elif secrets_found:
        finding_desc = f"¡ALERTA! {len(secrets_found)} secretos expuestos detectados"

    # Extraer primer archivo afectado si existe
    affected_file = None
    if top_risks and len(top_risks) > 0:
        first_risk = top_risks[0]
        if isinstance(first_risk, dict):
            affected_file = first_risk.get("file") or first_risk.get("path")
    elif secrets_found and len(secrets_found) > 0:
        first_secret = secrets_found[0]
        if isinstance(first_secret, dict):
            affected_file = first_secret.get("file")

    # ── 8. Reportar a Cerebro ─────────────────────────────────────────
    await report_to_cerebro(
        event_type=f"warden_{action.replace('-', '_')}_completed",
        severity=severity,
        payload={
            "action": action,
            "target": target,
            "summary": analysis,
            "memory_id": memory_id,
            "raw_status": raw_result.get("status"),
            # Campos para Auto-Fix (compatibles con Architect/Cerebro)
            "finding": finding_desc,
            "recommendation": analysis[:2000] if analysis else "Revisar hallazgos de seguridad detectados",
            "file": affected_file or target,
            "risks_count": len(top_risks),
            "secrets_count": len(secrets_found),
        },
    )

    # ── 8. Retornar CommandAck completo ───────────────────────────────
    return {
        "request_id": request_id,
        "status": "completed",
        "result": {
            "action": action,
            "target": target,
            "raw": raw_result,          # Resultado crudo del Core Rust (para Dashboard)
            "analysis": analysis,       # Síntesis del LLM (para Telegram/Dashboard)
            "memory_id": memory_id,
            "severity": severity,
        },
        "error": None,
    }


def _infer_severity(action: str, raw_result: dict, analysis: str) -> str:
    """
    Determina la severidad del evento reportado a Cerebro.
    Prioridad: error del Core → palabras clave en análisis → action por defecto.
    """
    if raw_result.get("status") == "error":
        return "error"

    analysis_lower = analysis.lower()
    if any(w in analysis_lower for w in ("crítico", "crítica", "critical", "secreto expuesto", "credencial")):
        return "critical"
    if any(w in analysis_lower for w in ("alto riesgo", "vulnerabilidad", "cve", "advertencia", "warning")):
        return "error"
    if action in ("check-secrets", "audit-deps"):
        return "warning"
    return "info"
