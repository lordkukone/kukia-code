# KUKIA CODE — Arquitectura

> Versión 0.1.0 (Fase 2) · 01/11/2026

## Decisión tecnológica

**Tauri 2 + React + TypeScript + núcleo Rust + SQLite.**

Motivos (verificados en la auditoría, no supuestos):

1. **WebView2 156 ya está instalado** en el equipo → Tauri no añade runtime propio.
2. **Rust 1.98.1 con toolchain MSVC + MSVC 14.44 + Windows SDK** ya funcionan → compilación nativa sin instalar nada global.
3. **Instalador NSIS/MSI** generado por el bundler oficial de Tauri, sin depender de Inno Setup/WiX preinstalados.
4. Binario final ligero (~10-20 MB) frente a ~150-200 MB de Electron.
5. El núcleo en Rust permite terminal (pty), procesos, GPU y red sin puentes frágiles.
6. **Node 24.18 + npm 11.16 + Vite 7** ya disponibles para la UI.

Descartado: Electron (más peso y RAM para el mismo resultado), PySide6 (peor empaquetado y menos control del núcleo), .NET/WinUI (no hay SDK .NET instalado y no aporta ventaja aquí).

## Capas

```
app/            React + TypeScript + Vite (UI)
core/           crate kukia-core (lógica pura, testeable, sin Tauri)
src-tauri/      crate kukia-code (comandos Tauri, estado, ventana)
```

`core` no conoce Tauri: se puede probar con `cargo test -p kukia-core` y reutilizar en CLI o servicio.

## Módulos del núcleo

| Módulo | Responsabilidad |
|---|---|
| `config` | Configuración por usuario en `%APPDATA%\KukiaCode`, perfil GPU 35/65 |
| `providers` | Ollama (API nativa), LM Studio y cloud (OpenAI-compatible), chat y streaming |
| `memory` | SQLite: sesiones, mensajes, memorias, aprobaciones, benchmarks, compatibilidad, caché del radar |
| `gpu` | `nvidia-smi` (utilización, VRAM, temperatura, consumo, relojes) + `sysinfo` (CPU/RAM) |
| `agents` | 8 agentes iniciales + agentes personalizados (`config/agents.json`) |
| `security` | Ejecución PowerShell con aprobación obligatoria y límite de lectura de archivos |
| `logs` | Log de aplicación en `%APPDATA%\KukiaCode\logs\kukia.log` |
| `radar` | Consulta releases oficiales (GitHub API) y fuentes manuales |
| `research` | KUKIA EXPERIMENTAL AI RADAR: arXiv, Hugging Face, GitHub, alertas, informes y descarga aislada |
| `permissions` | Política por agente **y por proyecto**: allowlist, solo-lectura, 17 verbos destructivos, junctions resueltos |
| `logs` | Log de aplicación, con carpeta aislable (`KUKIA_LOG_DIR`) para que los tests no ensucien el log real |
| `vram` | Guardia de VRAM: GPUs reales, motores con modelo cargado (Ollama, LM Studio, ComfyUI), aviso de competencia y cálculo de "¿cabe?" |
| `gaming` | KUKIA Future Gaming Radar: releases, búsqueda GitHub y arXiv, fichas de 23 campos, alertas e informes |
| `anime_real` | KUKIA ANIME ↔ REAL STUDIO: cliente de ComfyUI, plantillas API, comprobación de modelos y conversión verificada |

## Flujo de chat

1. La UI llama `chat_stream` con `Channel<ChatEvent>`.
2. El comando clona la configuración y lanza el streaming del proveedor.
3. `core::providers` emite `token`, `done` o `error` por un canal tokio.
4. Un forwarder traduce cada evento al `Channel` de Tauri.
5. La UI acumula tokens y guarda el mensaje final en SQLite.

## Seguridad

- `files.write` y `terminal.run` **exigen** `approved: true`; sin eso devuelven error.
- Cada aprobación se registra en la tabla `approvals` (auditoría inmutable desde la UI).
- Antes de sobrescribir un archivo existente se copia a `%APPDATA%\KukiaCode\backups\`.
- La clave cloud **no se guarda**: se lee de `KUKIA_CLOUD_KEY`.
- Nada se instala ni actualiza automáticamente.

## Datos de usuario

```
%APPDATA%\KukiaCode\
  config\config.json
  config\agents.json
  logs\kukia.log
  backups\
  benchmarks\
  kukia.db
```

## Multi-GPU

- Perfil inicial: GPU 0 (RTX 5070) 35 %, GPU 1 (RTX 5060 Ti) 65 %.
- El reparto se guarda en configuración.
- **No se confunde VRAM con cálculo**: Ollama/llama.cpp aplican offload por capas; LM Studio gestiona su propia VRAM. KUKIA CODE no impone métodos que el motor no soporte.

## KUKIA Experimental AI Radar

- Fuentes verificadas con API pública: **arXiv** (export API), **Hugging Face** (`/api/models`), **GitHub** (search API).
- Fuentes manuales documentadas: Papers with Code, NVIDIA Research, PyTorch, llama.cpp, LM Studio, Ollama (se consultan vía release/changelog, sin inventar datos).
- Cada descubrimiento guarda la ficha de 12 campos (nombre, fuente, fecha, requisitos, licencia, Windows, GPUs, VRAM, mejora esperada, medido, riesgos, recomendación).
- Puntuación heurística transparente; **nada se marca compatible sin prueba**.
- Alertas automáticas por palabras de alto valor (NVFP4, FP4, expert parallel, speculative decoding, MoE, KV cache, etc.).
- Informe diario en `H:\KUKIA_RESEARCH\AI_EXPERIMENTAL\reports\YYYY-MM-DD.md`.
- Descarga aislada solo https, máximo 25 MB, a `downloads\`; nunca se ejecuta nada.
- Vigilancia automática cada 6 h si `research_auto` está activo (solo metadatos).

## Permisos por proyecto (Fase 3A)

- Tabla `projects`: id, nombre, raíz, rutas de lectura/escritura, rutas de solo lectura y los
  interruptores `allow_write` / `allow_shell` / `allow_network`.
- El proyecto activo se guarda en `config.json` (`active_project`).
- Resolución en `permissions::check_scoped` y `check_cwd_in_scope_scoped`:
  **política del agente Y ámbito del proyecto**. El proyecto recorta, nunca amplía.
- Se aplica en los mismos cinco puntos que la política del agente: `files_read`, `files_write`,
  `run_command`, `run_command_stream` y `audit_export`.
- Comandos `projects_list`, `project_create`, `project_update` (exige aprobación),
  `project_activate`, `project_active`. UI dentro de la vista **Seguridad**.
- Detalle en `docs/SEGURIDAD.md`.

## Guardia de VRAM

- `vram::report()` mide GPUs con `nvidia-smi` y consulta **tres** motores: Ollama (`/api/ps`),
  LM Studio (`/api/v0/models` en 1234 y 1235) y ComfyUI (`/system_stats` en 8188).
- Avisa de **competencia** cuando hay dos o más motores con modelo cargado a la vez.
- ComfyUI no publica qué checkpoint tiene: se deduce por VRAM ocupada con umbral de 800 MB,
  y se dice tal cual en la interfaz.
- En esta máquina Windows/WDDM `nvidia-smi` devuelve `N/A` en memoria por proceso, así que la
  vista filtra esas filas y explica por qué se mide por motor.
- Comandos: `vram_report`, `vram_fits`. Vista: `app/src/views/Vram.tsx`.

## KUKIA Future Gaming Radar

- `gaming::scan_all()` consulta 9 repos (releases), 6 búsquedas de GitHub y 5 términos de arXiv,
  y **deduplica por id**: el mismo proyecto puede salir en releases y en búsqueda (medido: 3
  duplicados en un escaneo de 39).
- Cada descubrimiento es una ficha de 23 campos con estado, licencia, requisitos, riesgos y
  veredicto por GPU (RTX 5070 / RTX 5060 Ti).
- Solo lectura: no descarga ejecutables ni toca juegos, drivers o configuraciones.
- Laboratorio en `H:\KUKIA_RESEARCH\GAMING_GRAPHICS` (8 subcarpetas + informe diario).
- Comandos: `gaming_scan`, `gaming_items`, `gaming_alerts`, `gaming_report`, `gaming_lab_path`.
  Vista: `app/src/views/Gaming.tsx`.

## KUKIA ANIME ↔ REAL STUDIO

- Vive en `core/src/anime_real.rs` + `workflows/anime_real/` (workflow API + `manifest.json`).
- Flujo: `/object_info` para saber qué modelos ve ComfyUI → comprobar los `requiere` de cada
  plantilla → `POST /prompt` → sondear `/history/{id}` → descargar por `/view` → guardar en
  `output/anime_real/` y anotar una línea en `historial.jsonl`.
- El resultado vuelve a la UI como *data URL* (entrada y salida), así el antes/después funciona
  sin protocolo de assets ni permisos extra de Tauri.
- Cada conversión queda en `audit_log` (agente `studio`, herramienta `anime_real.convert`).
- No instala, no descarga modelos y no modifica ComfyUI: solo encola y espera.
- Detalle completo en `docs/ANIME_REAL.md`.

## Radar de actualizaciones

- `radar_check` consulta GitHub Releases de Ollama y llama.cpp (API pública).
- LM Studio, PolyStrata, CUDA y driver NVIDIA se listan como consulta manual con enlace oficial: no se inventan versiones.
- Los resultados se cachean en `radar_cache`.
- **Nunca descarga ni instala**: solo informa.
