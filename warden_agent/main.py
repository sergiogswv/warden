"""
main.py — Entrypoint del servidor HTTP del Warden ADK Agent.

Expone exactamente el mismo contrato que el servidor Rust (Warden Core):
  POST /command  → recibe OrchestratorCommand, retorna CommandAck

Esto permite que Cerebro lo invoque sin cambiar nada en su dispatcher.
El agente puede correr en paralelo al Core Rust (puerto diferente: 4013).
"""

import asyncio
import sys
if sys.platform == "win32":
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")

import uvicorn
from fastapi import FastAPI
from pydantic import BaseModel
from typing import Optional

from .memory import init_db
from .cerebro_bridge import handle_command, report_to_cerebro
from .settings import settings


app = FastAPI(
    title="Warden ADK Agent",
    description="Agente de seguridad inteligente con memoria persistente (Google ADK)",
    version="1.0.0",
)


# ──────────────────────────────────────────────
# Modelos de entrada/salida (mismo contrato Skrymir)
# ──────────────────────────────────────────────

class OrchestratorCommand(BaseModel):
    action: str
    target: Optional[str] = None
    options: Optional[dict] = None
    request_id: Optional[str] = None


class CommandAck(BaseModel):
    request_id: Optional[str] = None
    status: str
    result: Optional[dict] = None
    error: Optional[str] = None


# ──────────────────────────────────────────────
# Endpoints
# ──────────────────────────────────────────────

@app.on_event("startup")
async def on_startup():
    """Inicializa la base de datos y notifica a Cerebro que el agente está listo."""
    await init_db()
    print(f"🔱 Warden ADK Agent iniciado en puerto {settings.warden_adk_port}")
    await report_to_cerebro(
        event_type="warden_adk_ready",
        severity="info",
        payload={
            "message": "Warden ADK Agent listo — modo IA activo",
            "version": "1.0.0",
            "port": settings.warden_adk_port,
        },
    )


@app.post("/command", response_model=CommandAck)
async def command_endpoint(cmd: OrchestratorCommand) -> CommandAck:
    """
    Recibe un OrchestratorCommand del Cerebro y lo procesa con el LlmAgent.
    Mismo contrato que el Warden Core Rust para intercambiabilidad.
    """
    ack = await handle_command(
        action=cmd.action,
        target=cmd.target,
        request_id=cmd.request_id,
    )
    return CommandAck(**ack)


@app.get("/health")
async def health():
    """Health check básico."""
    return {"status": "ok", "agent": "warden-adk", "version": "1.0.0"}


@app.get("/memory/context")
async def get_memory():
    """Endpoint de debug: retorna el contexto de memoria histórica del agente."""
    from .memory import get_hot_files, get_recent_findings
    hot = await get_hot_files(10)
    recent = await get_recent_findings(10)
    return {"hot_files": hot, "recent_findings": recent}


# ──────────────────────────────────────────────
# Runner
# ──────────────────────────────────────────────

def start():
    uvicorn.run(
        "warden_agent.main:app",
        host="0.0.0.0",
        port=settings.warden_adk_port,
        reload=False,
    )


if __name__ == "__main__":
    start()
