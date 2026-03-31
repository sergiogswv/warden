# 🔱 WARDEN — Plan de Upgrade a Agente de IA

> **Objetivo:** Evolucionar a Warden de un escáner estático (Rust) a un **Agente de Seguridad Inteligente** con análisis LLM y memoria persistente.

> **Estrategia:** Patrón **Sidecar Determinista** — El Warden Core (Rust, `:4003`) sigue siendo el motor de ejecución.
> Se agrega un proceso Python (`warden_agent/`, `:4013`) con el siguiente flujo:
>
> ```
> Cerebro ordena action  →  Core Rust ejecuta (ya sabe hacerlo)  →  resultado crudo
>   →  SQLite persiste  →  LLM analiza resultado + historial  →  síntesis a Cerebro
> ```
>
> **¿Por qué no un LlmAgent autónomo?**
> Cerebro ya decide *qué* acción ejecutar. El LLM de Warden solo debe *interpretar resultados*,
> no seleccionar herramientas. Esto es más predecible, barato y alineado con el contrato Skrymir.
>
> **LLM soportados:** Gemini · Claude · OpenAI (selección vía `LLM_PROVIDER` en `.env`)

---

## 📊 Estado de Tareas

### ✅ Fase 1 — Infraestructura del Sidecar

| ID    | Tarea                                            | Archivo                             | Estado |
|-------|--------------------------------------------------|-------------------------------------|--------|
| F1-1  | Módulo de configuración multi-LLM (`pydantic-settings`) | `warden_agent/settings.py`  | ✅ Implementado |
| F1-2  | Requirements (Gemini/Claude/OpenAI opcionales)   | `warden_agent/requirements.txt`     | ✅ Implementado |
| F1-3  | Template `.env` con los 3 proveedores            | `warden_agent/.env.example`         | ✅ Implementado |
| F1-4  | Package init                                     | `warden_agent/__init__.py`          | ✅ Implementado |

### ✅ Fase 2 — Memoria Persistente (SQLite)

| ID    | Tarea                                            | Archivo                             | Estado |
|-------|--------------------------------------------------|-------------------------------------|--------|
| F2-1  | Schema SQL (`findings`, `risk_profiles`, `analysis_runs`) | `warden_agent/memory.py` | ✅ Implementado |
| F2-2  | `save_finding()` — guarda cada hallazgo          | `warden_agent/memory.py`            | ✅ Implementado |
| F2-3  | `update_risk_profile()` — perfil acumulado por archivo | `warden_agent/memory.py`      | ✅ Implementado |
| F2-4  | `get_hot_files()` — top archivos por historial de riesgo | `warden_agent/memory.py`   | ✅ Implementado |
| F2-5  | `get_recent_findings()` — hallazgos recientes para contexto | `warden_agent/memory.py` | ✅ Implementado |
| F2-6  | `save_analysis_run()` — registro de cada análisis ejecutado | `warden_agent/memory.py` | ✅ Implementado |
| F2-7  | `set_finding_decision()` — marca false_positive o risk_accepted | `warden_agent/memory.py` | ✅ Implementado |

### ✅ Fase 3 — Wrappers del Core Rust (Tools)

> Las tools son wrappers HTTP puros. No deciden nada. Solo ejecutan y persisten.

| ID    | Tarea                                            | Archivo                             | Estado |
|-------|--------------------------------------------------|-------------------------------------|--------|
| F3-1  | `execute_scan(target)` → Core + SQLite           | `warden_agent/tools.py`             | ✅ Implementado |
| F3-2  | `execute_risk_assess(target)` → persiste perfiles por archivo | `warden_agent/tools.py` | ✅ Implementado |
| F3-3  | `execute_predict_critical(target)`               | `warden_agent/tools.py`             | ✅ Implementado |
| F3-4  | `execute_churn_report(target)`                   | `warden_agent/tools.py`             | ✅ Implementado |
| F3-5  | `execute_audit_deps()`                           | `warden_agent/tools.py`             | ✅ Implementado |
| F3-6  | `execute_check_secrets(target)`                  | `warden_agent/tools.py`             | ✅ Implementado |
| F3-7  | `execute_report(target)` — reporte completo      | `warden_agent/tools.py`             | ✅ Implementado |
| F3-8  | `ACTION_MAP` dict para dispatch dinámico         | `warden_agent/tools.py`             | ✅ Implementado |

### ✅ Fase 4 — Cliente LLM Multi-Proveedor

| ID    | Tarea                                            | Archivo                             | Estado |
|-------|--------------------------------------------------|-------------------------------------|--------|
| F4-1  | `analyze_result()` — interfaz única multi-LLM    | `warden_agent/llm_client.py`        | ✅ Implementado |
| F4-2  | Soporte Gemini (`google-generativeai`)           | `warden_agent/llm_client.py`        | ✅ Implementado |
| F4-3  | Soporte Claude (`anthropic`)                     | `warden_agent/llm_client.py`        | ✅ Implementado |
| F4-4  | Soporte OpenAI (`openai`)                        | `warden_agent/llm_client.py`        | ✅ Implementado |
| F4-5  | `SYSTEM_PROMPT` del experto en seguridad         | `warden_agent/llm_client.py`        | ✅ Implementado |
| F4-6  | `_build_analysis_prompt()` = raw + memoria       | `warden_agent/llm_client.py`        | ✅ Implementado |
| F4-7  | `agent.py` simplificado — facade + doc. de la decisión | `warden_agent/agent.py`      | ✅ Implementado |

### ✅ Fase 5 — Bridge con Cerebro (Flujo Determinista)

| ID    | Tarea                                            | Archivo                             | Estado |
|-------|--------------------------------------------------|-------------------------------------|--------|
| F5-1  | `handle_command()` — flujo: ejecutar→persistir→analizar→reportar | `warden_agent/cerebro_bridge.py` | ✅ Implementado |
| F5-2  | `report_to_cerebro()` → `POST /api/events`       | `warden_agent/cerebro_bridge.py`    | ✅ Implementado |
| F5-3  | Dispatch por `ACTION_MAP` con fallback al Core   | `warden_agent/cerebro_bridge.py`    | ✅ Implementado |
| F5-4  | `_infer_severity()` — severity basada en resultado + análisis LLM | `warden_agent/cerebro_bridge.py` | ✅ Implementado |
| F5-5  | `status` sin Core/LLM — retorna estado del sidecar | `warden_agent/cerebro_bridge.py` | ✅ Implementado |

### ✅ Fase 6 — Servidor FastAPI (Mismo Contrato)

| ID    | Tarea                                            | Archivo                             | Estado |
|-------|--------------------------------------------------|-------------------------------------|--------|
| F6-1  | `POST /command` con modelo OrchestratorCommand   | `warden_agent/main.py`              | ✅ Implementado |
| F6-2  | `GET /health`                                    | `warden_agent/main.py`              | ✅ Implementado |
| F6-3  | `GET /memory/context` (debug/dashboard)          | `warden_agent/main.py`              | ✅ Implementado |
| F6-4  | Evento `warden_adk_ready` al arrancar            | `warden_agent/main.py`              | ✅ Implementado |
| F6-5  | Init de BD SQLite en startup                     | `warden_agent/main.py`              | ✅ Implementado |

### ✅ Fase 6.5 — Integración Frontend (Skrymir Dashboard)

| ID    | Tarea                                            | Archivo                             | Estado |
|-------|--------------------------------------------------|-------------------------------------|--------|
| F6.5-1| Renderizado completo de reportes LLM (sin truncar 500 chars) | `cerebro_bridge.py / EventCard` | ✅ Implementado |
| F6.5-2| Inspector JSON crudo en Timeline de Cerebro      | `EventFeed / CommsTimeline.jsx`     | ✅ Implementado |
| F6.5-3| Soporte nativo para Ollama en el panel de UI     | `WardenConfigModal / llm_client.py` | ✅ Implementado |
| F6.5-4| Custom Scroll estético para lecturas largas de IA| `ScrollableContainer / index.jsx`   | ✅ Implementado |

---

## 🧪 Listo para Probar

### ⏳ Setup — Instalar dependencias

```bash
cd warden/warden_agent
cp .env.example .env
# Edita .env y añade GOOGLE_API_KEY=tu_clave
pip install -r requirements.txt
```

### ⏳ Levantar el sidecar

```bash
# Desde warden/
python -m warden_agent.main
# Escucha en http://localhost:4013
```

### ⏳ Prueba manual (curl)

```bash
# Health check
curl http://localhost:4013/health

# Comando de seguridad (mismo contrato que el Core Rust)
curl -X POST http://localhost:4013/command \
  -H "Content-Type: application/json" \
  -d '{"action": "scan", "target": "."}'

# Ver memoria histórica
curl http://localhost:4013/memory/context

# Evaluación de riesgos
curl -X POST http://localhost:4013/command \
  -H "Content-Type: application/json" \
  -d '{"action": "risk-assess", "target": "."}'
```

### ⏳ Configurar Cerebro para usar el sidecar ADK (opcional)

Para que Cerebro use el agente ADK en vez del Core Rust, actualizar su configuración:

```bash
# En cerebro/.env
WARDEN_URL=http://localhost:4013   # Apunta al sidecar ADK
# En vez de:
# WARDEN_URL=http://localhost:4003  # Core Rust (sin IA)
```

---

## 🔄 Fases Pendientes

### ⬜ Fase 7 — Pruebas de Integración

| ID    | Tarea                                                     | Estado |
|-------|-----------------------------------------------------------|--------|
| F7-1  | Test: Cerebro envía `scan` → sidecar responde            | ✅ Completado |
| F7-2  | Test: Memoria persiste entre reinicios del sidecar       | ✅ Completado |
| F7-3  | Test: `get_memory_context` retorna datos históricos      | ✅ Completado |
| F7-4  | Test: Factor de riesgo se incrementa en archivos "calientes" | ✅ Completado |
| F7-5  | Test: El agente menciona historial en su respuesta       | ✅ Completado |

### ⬜ Fase 8 — Vector DB (Amenazas Semánticas)

| ID    | Tarea                                                     | Estado |
|-------|-----------------------------------------------------------|--------|
| F8-1  | Instalar y configurar ChromaDB                           | ⬜ Pendiente |
| F8-2  | Embeber cada hallazgo en el vector store                 | ⬜ Pendiente |
| F8-3  | `warden_search_similar_threats(query)` como nueva tool   | ⬜ Pendiente |
| F8-4  | El agente usa la búsqueda semántica antes de actuar      | ⬜ Pendiente |

### ⬜ Fase 9 — Feedback Loop

| ID    | Tarea                                                     | Estado |
|-------|-----------------------------------------------------------|--------|
| F9-1  | Endpoint `POST /feedback` para marcar false_positive     | ⬜ Pendiente |
| F9-2  | El agente consulta decisiones pasadas antes de escalar   | ⬜ Pendiente |
| F9-3  | Integración con Cerebro: el usuario aprueba/rechaza vía Telegram | ⬜ Pendiente |

### ⬜ Fase 10 — Auditoría Real de Dependencias

| ID    | Tarea                                                     | Estado |
|-------|-----------------------------------------------------------|--------|
| F10-1 | Completar `audit-deps` en el Core Rust (actualmente stub)| ⬜ Pendiente |
| F10-2 | Integrar `cargo audit` para proyectos Rust               | ⬜ Pendiente |
| F10-3 | Integrar `npm audit` para proyectos Node                 | ⬜ Pendiente |

---

## 🏗️ Arquitectura del Sidecar

```
Cerebro
  │ POST /command {action, target}
  ▼
warden_agent/main.py  (FastAPI :4013)
  │ llamada a handle_command()
  ▼
warden_agent/cerebro_bridge.py
  │ convierte command → prompt
  ▼
warden_agent/agent.py  (LlmAgent ADK)
  │ Gemini 2.0 Flash decide qué tools invocar
  ├─► warden_get_memory_context()  ← SQLite
  ├─► warden_scan()                → POST :4003/command (Rust)
  ├─► warden_risk_assess()         → POST :4003/command (Rust) + persiste perfiles
  ├─► warden_check_secrets()       → POST :4003/command (Rust)
  └─► warden_predict_critical()    → POST :4003/command (Rust)
  │
  ▼
warden_agent/memory.py  (SQLite)
  │ Guarda: findings / risk_profiles / analysis_runs
  ▼
cerebro_bridge.report_to_cerebro()
  │ POST Cerebro/api/events
  ▼
Cerebro recibe el evento y lo propaga al Dashboard/Telegram
```

---

## 📁 Archivos del Sidecar

```
warden/
└── warden_agent/
    ├── __init__.py          ✅ Package init
    ├── settings.py          ✅ Configuración (pydantic-settings)
    ├── memory.py            ✅ Memoria persistente (SQLite)
    ├── tools.py             ✅ Tools ADK (wrappers del Core Rust)
    ├── agent.py             ✅ LlmAgent (Gemini 2.0 Flash)
    ├── cerebro_bridge.py    ✅ Bridge Cerebro ↔ ADK Agent
    ├── main.py              ✅ FastAPI server (:4013)
    ├── requirements.txt     ✅ Dependencias Python
    └── .env.example         ✅ Plantilla de configuración
```

---

## 💬 Comunicación con Cerebro (Sin Cambios en el Contrato)

El Core Rust (`:4003`) y el Sidecar ADK (`:4013`) son **intercambiables**.
Cerebro solo necesita cambiar el `WARDEN_URL` en su `.env`.

```
Cerebro
  ├── WARDEN_URL=:4003  → Core Rust (rápido, determinista, sin IA)
  └── WARDEN_URL=:4013  → Sidecar ADK (con Gemini, memoria y razonamiento)
```

*Documento vivo — actualizar con cada fase completada.*
