"""
memory.py — Base de datos SQLite para la memoria persistente de Warden.

Persiste:
  - Historial de eventos de seguridad (findings, CVEs, secretos)
  - Decisiones del usuario (accepted / false_positive / risk_accepted)
  - Perfil de riesgo por archivo (risk_profile)
  - Historial de análisis ejecutados
"""

import aiosqlite
import json
from datetime import datetime, timezone
from typing import Optional
from .settings import settings


DB_PATH = settings.warden_db_path

SCHEMA = """
CREATE TABLE IF NOT EXISTS findings (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at  TEXT    NOT NULL,
    event_type  TEXT    NOT NULL,          -- 'scan_completed', 'secret_found', 'cve_alert', etc.
    severity    TEXT    NOT NULL,          -- 'info' | 'warning' | 'error' | 'critical'
    target      TEXT,                      -- archivo o directorio analizado
    payload     TEXT    NOT NULL,          -- JSON del resultado completo
    decision    TEXT    DEFAULT NULL,      -- 'accepted' | 'false_positive' | 'risk_accepted'
    decision_at TEXT    DEFAULT NULL
);

CREATE TABLE IF NOT EXISTS risk_profiles (
    file_path   TEXT    PRIMARY KEY,
    last_seen   TEXT    NOT NULL,
    total_scans INTEGER NOT NULL DEFAULT 0,
    critical_count INTEGER NOT NULL DEFAULT 0,
    alert_count    INTEGER NOT NULL DEFAULT 0,
    last_risk_value REAL NOT NULL DEFAULT 0.0,
    notes       TEXT   DEFAULT NULL
);

CREATE TABLE IF NOT EXISTS analysis_runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    run_at      TEXT    NOT NULL,
    action      TEXT    NOT NULL,          -- 'scan', 'risk-assess', 'predict-critical', 'churn-report'
    target      TEXT,
    summary     TEXT    NOT NULL,          -- JSON resumido del resultado
    duration_ms INTEGER DEFAULT NULL
);
"""


async def init_db():
    """Inicializa el schema de la base de datos si no existe."""
    async with aiosqlite.connect(DB_PATH) as db:
        await db.executescript(SCHEMA)
        await db.commit()


async def save_finding(
    event_type: str,
    severity: str,
    payload: dict,
    target: Optional[str] = None,
) -> int:
    """Guarda un hallazgo de seguridad y retorna su ID."""
    now = datetime.now(timezone.utc).isoformat()
    async with aiosqlite.connect(DB_PATH) as db:
        cursor = await db.execute(
            """
            INSERT INTO findings (created_at, event_type, severity, target, payload)
            VALUES (?, ?, ?, ?, ?)
            """,
            (now, event_type, severity, target, json.dumps(payload)),
        )
        await db.commit()
        return cursor.lastrowid


async def set_finding_decision(finding_id: int, decision: str):
    """Actualiza la decisión del usuario sobre un hallazgo."""
    now = datetime.now(timezone.utc).isoformat()
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute(
            "UPDATE findings SET decision = ?, decision_at = ? WHERE id = ?",
            (decision, now, finding_id),
        )
        await db.commit()


async def update_risk_profile(file_path: str, risk_value: float, severity: str):
    """
    Actualiza el perfil de riesgo acumulado de un archivo.
    Si no existe, lo crea. Si ya existe, acumula los contadores.
    """
    now = datetime.now(timezone.utc).isoformat()
    is_critical = 1 if severity == "critical" else 0
    is_alert    = 1 if severity in ("error", "warning") else 0

    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute(
            """
            INSERT INTO risk_profiles (file_path, last_seen, total_scans, critical_count, alert_count, last_risk_value)
            VALUES (?, ?, 1, ?, ?, ?)
            ON CONFLICT(file_path) DO UPDATE SET
                last_seen       = excluded.last_seen,
                total_scans     = total_scans + 1,
                critical_count  = critical_count + excluded.critical_count,
                alert_count     = alert_count + excluded.alert_count,
                last_risk_value = excluded.last_risk_value
            """,
            (file_path, now, is_critical, is_alert, risk_value),
        )
        await db.commit()


async def get_risk_profile(file_path: str) -> Optional[dict]:
    """Devuelve el perfil de riesgo histórico de un archivo, o None si no existe."""
    async with aiosqlite.connect(DB_PATH) as db:
        db.row_factory = aiosqlite.Row
        async with db.execute(
            "SELECT * FROM risk_profiles WHERE file_path = ?", (file_path,)
        ) as cursor:
            row = await cursor.fetchone()
            return dict(row) if row else None


async def get_hot_files(limit: int = 10) -> list[dict]:
    """
    Devuelve los archivos con más historial de riesgo acumulado.
    Útil para contextualizar al agente ADK sobre el estado del proyecto.
    """
    async with aiosqlite.connect(DB_PATH) as db:
        db.row_factory = aiosqlite.Row
        async with db.execute(
            """
            SELECT file_path, total_scans, critical_count, alert_count, last_risk_value, last_seen
            FROM risk_profiles
            ORDER BY critical_count DESC, last_risk_value DESC
            LIMIT ?
            """,
            (limit,),
        ) as cursor:
            rows = await cursor.fetchall()
            return [dict(r) for r in rows]


async def save_analysis_run(action: str, target: Optional[str], summary: dict, duration_ms: Optional[int] = None):
    """Registra cada ejecución de análisis para tracking histórico."""
    now = datetime.now(timezone.utc).isoformat()
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute(
            """
            INSERT INTO analysis_runs (run_at, action, target, summary, duration_ms)
            VALUES (?, ?, ?, ?, ?)
            """,
            (now, action, target, json.dumps(summary), duration_ms),
        )
        await db.commit()


async def get_recent_findings(limit: int = 20, severity_filter: Optional[str] = None) -> list[dict]:
    """
    Retorna los hallazgos más recientes, opcionalmente filtrados por severity.
    Este método lo usará el agente ADK para recordar el contexto de seguridad.
    """
    async with aiosqlite.connect(DB_PATH) as db:
        db.row_factory = aiosqlite.Row
        query = "SELECT * FROM findings"
        params: list = []
        if severity_filter:
            query += " WHERE severity = ?"
            params.append(severity_filter)
        query += " ORDER BY created_at DESC LIMIT ?"
        params.append(limit)
        async with db.execute(query, params) as cursor:
            rows = await cursor.fetchall()
            result = []
            for row in rows:
                d = dict(row)
                d["payload"] = json.loads(d["payload"] or "{}")
                result.append(d)
            return result
