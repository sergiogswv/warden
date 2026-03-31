"""
llm_client.py — Cliente LLM multi-proveedor para Warden.

Soporta: Gemini (Google), Claude (Anthropic), OpenAI, Ollama (local).
Selección por variable de entorno: LLM_PROVIDER=gemini|claude|openai|ollama

El rol de este módulo es recibir un contexto + resultado crudo de Warden Core
y retornar un análisis/síntesis en texto que va de vuelta a Cerebro.
"""

import os
from typing import Optional
from .settings import settings


# ──────────────────────────────────────────────
# Prompt base para análisis de seguridad
# ──────────────────────────────────────────────

SYSTEM_PROMPT = """Eres el Agente Warden de Skrymir Suite — un experto en seguridad de software.
Tu trabajo es analizar los resultados de herramientas de análisis estático y producir
un reporte conciso, claro y accionable en español.

Reglas:
- Si hay hallazgos críticos, ponlos primero y en negrita.
- Para cada hallazgo importante, da una recomendación concreta.
- Si el contexto histórico indica que un archivo ya tuvo problemas antes, menciónalo.
- Si no hay nada relevante, dilo claramente en una sola línea.
- Sé directo y útil. No repitas datos que ya están en el JSON, interprétalos.
- Máximo 400 palabras en la respuesta."""


def _build_analysis_prompt(action: str, raw_result: dict, memory_context: Optional[dict]) -> str:
    """
    Construye el prompt que recibe el LLM.
    Combina el resultado crudo del Core Rust + contexto de memoria histórica.
    """
    import json

    lines = [
        f"## Acción ejecutada: `{action}`",
        "",
        "### Resultado del Warden Core:",
        "```json",
        json.dumps(raw_result, indent=2, ensure_ascii=False)[:3000],  # Truncar si es muy largo
        "```",
    ]

    if memory_context:
        hot_files = memory_context.get("hot_files", [])
        recent_critical = memory_context.get("recent_critical_findings", [])

        if hot_files:
            lines += [
                "",
                "### Archivos con mayor historial de riesgo (memoria):",
                "```json",
                json.dumps(hot_files[:5], indent=2, ensure_ascii=False),
                "```",
            ]
        if recent_critical:
            lines += [
                "",
                "### Hallazgos críticos recientes (memoria):",
                "```json",
                json.dumps(recent_critical[:3], indent=2, ensure_ascii=False),
                "```",
            ]

    lines += [
        "",
        "Analiza los resultados anteriores y produce un reporte accionable.",
    ]

    return "\n".join(lines)


# ──────────────────────────────────────────────
# Implementaciones por proveedor
# ──────────────────────────────────────────────

async def _analyze_with_gemini(prompt: str) -> str:
    try:
        import google.generativeai as genai
        genai.configure(api_key=settings.google_api_key)
        model = genai.GenerativeModel(
            model_name=settings.gemini_model,
            system_instruction=SYSTEM_PROMPT,
        )
        response = await model.generate_content_async(prompt)
        return response.text
    except ImportError:
        return "[Error] google-generativeai no instalado. Ejecuta: pip install google-generativeai"
    except Exception as exc:
        return f"[Error Gemini] {exc}"


async def _analyze_with_claude(prompt: str) -> str:
    try:
        import anthropic
        client = anthropic.AsyncAnthropic(api_key=settings.anthropic_api_key)
        message = await client.messages.create(
            model=settings.claude_model,
            max_tokens=1024,
            system=SYSTEM_PROMPT,
            messages=[{"role": "user", "content": prompt}],
        )
        return message.content[0].text
    except ImportError:
        return "[Error] anthropic no instalado. Ejecuta: pip install anthropic"
    except Exception as exc:
        return f"[Error Claude] {exc}"


async def _analyze_with_openai(prompt: str) -> str:
    try:
        from openai import AsyncOpenAI
        client = AsyncOpenAI(api_key=settings.openai_api_key)
        response = await client.chat.completions.create(
            model=settings.openai_model,
            messages=[
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": prompt},
            ],
            max_tokens=1024,
        )
        return response.choices[0].message.content or ""
    except ImportError:
        return "[Error] openai no instalado. Ejecuta: pip install openai"
    except Exception as exc:
        return f"[Error OpenAI] {exc}"


async def _analyze_with_ollama(prompt: str) -> str:
    """
    Llama a Ollama usando su endpoint OpenAI-compatible /v1/chat/completions.
    No requiere librerías adicionales: usa httpx (ya es dependencia).
    """
    import httpx

    url = f"{settings.ollama_base_url.rstrip('/')}/v1/chat/completions"
    payload = {
        "model": settings.ollama_model,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user",   "content": prompt},
        ],
        "stream": False,
    }
    try:
        async with httpx.AsyncClient(timeout=300.0) as client:
            resp = await client.post(url, json=payload)
            resp.raise_for_status()
            data = resp.json()
            return data["choices"][0]["message"]["content"]
    except httpx.ConnectError:
        return (
            f"[Error Ollama] No hay conexión en {settings.ollama_base_url}. "
            "Verifica que Ollama esté corriendo con: ollama serve"
        )
    except httpx.HTTPStatusError as exc:
        return f"[Error Ollama] HTTP {exc.response.status_code}: {exc.response.text[:200]}"
    except Exception as exc:
        return f"[Error Ollama] {exc}"


# ──────────────────────────────────────────────
# Interfaz pública
# ──────────────────────────────────────────────

async def analyze_result(
    action: str,
    raw_result: dict,
    memory_context: Optional[dict] = None,
) -> str:
    """
    Punto de entrada principal.
    Toma el resultado crudo del Warden Core y lo analiza con el LLM configurado.

    Args:
        action:          Acción que generó el resultado (scan, risk-assess, etc.)
        raw_result:      JSON retornado por el Warden Core (Rust)
        memory_context:  Contexto histórico de la memoria SQLite (opcional)

    Returns:
        Análisis textual conciso y accionable en español.
    """
    prompt = _build_analysis_prompt(action, raw_result, memory_context)
    provider = settings.llm_provider

    print(f"🤖 [LLM:{provider}] Analizando resultado de '{action}'...")

    if provider == "gemini":
        return await _analyze_with_gemini(prompt)
    elif provider == "claude":
        return await _analyze_with_claude(prompt)
    elif provider == "openai":
        return await _analyze_with_openai(prompt)
    elif provider == "ollama":
        return await _analyze_with_ollama(prompt)
    else:
        return (
            f"[Error] Proveedor LLM desconocido: '{provider}'. "
            "Usa: gemini | claude | openai | ollama"
        )
