# KUKIA CODE — INFORME COMPLETO DE TRASPASO (para otra IA)

Fecha: 01/11/2026 · Autor original: Xoxete (para Kuki / LordKukone)
Estado: Fase 2 completada y compilada; reconstrucción final en curso con el módulo Research incluido.

## 1. QUÉ ES KUKIA CODE

Aplicación de escritorio Windows 11, independiente de OpenCode y Freebuff, con:

- Interfaz gráfica propia.
- Núcleo de IA con selector universal de modelos.
- Conectores locales (Ollama, LM Studio) y cloud OpenAI-compatible.
- Memoria persistente SQLite.
- Monitor GPU y benchmarks reales.
- Radar de actualizaciones y radar experimental de IA (arXiv/HF/GitHub).
- Sistema de aprobaciones y auditoría.
- Instalador EXE (NSIS).

## 2. UBICACIONES

- Proyecto: `H:\KUKIA_CODE`
- Laboratorio de investigación: `H:\KUKIA_RESEARCH\AI_EXPERIMENTAL`
- Datos de usuario en ejecución: `%APPDATA%\KukiaCode` (config, logs, backups, kukia.db)
- Auditoría inicial: `H:\KUKIA_CODE\INFORMES\AUDITORIA_INICIAL.md`
- Informe de fase: `H:\KUKIA_CODE\INFORMES\FASE_2.md`
- Documentación: `docs\ARQUITECTURA.md`, `docs\MANUAL.md`, `docs\ROADMAP.md`, `docs\SEGURIDAD.md`
- Repo Git local inicializado (sin remoto).

## 3. ENTORNO VERIFICADO (NO SUPUESTO)

- Windows 11 Pro build 26300 x64.
- CPU Intel i9-10900K (10C/20T), RAM 63,9 GB.
- GPU0 RTX 5070 12 GB (pantalla/juegos), GPU1 RTX 5060 Ti 16 GB. Driver 621.01.
- CUDA Toolkit 13.4 (nvcc 13.4.92). PyTorch 2.12.1+cu130 con CUDA disponible y 2 GPUs.
- Node 24.18.0, npm 11.16.0, Bun 1.4.4.
- Python 3.12.9, pip 26.1.2. PySide6 6.11.1, FastAPI, cryptography 49, torch.
- Rust 1.98.1 toolchain MSVC (stable-x86_64-pc-windows-msvc), cargo 1.98.1.
- MSVC 14.44.35207 en `D:\VisualStudio2022` + Windows SDK 10.0.19041/26100.
- GCC 16.2.0 (MSYS2 `C:\msys64\mingw64`), Go 1.24.5, Git 2.55.0.
- VS Code 1.143.0. WebView2 Runtime 156.0.4314.50 (clave para Tauri).
- .NET SDK NO instalado. Clang NO disponible. Inno Setup/NSIS/WiX NO preinstalados (Tauri descarga NSIS solo).
- Ollama 0.40.13 activo en `127.0.0.1:11434`, ~80 modelos, 518,7 GB en `D:\ollama\models`.
- LM Studio 1.19.3, 14 GGUF (196,2 GB) en `C:\Users\kuki\.lmstudio\models`; API `:1234` apagada por defecto.
- OpenCode 1.19.3 y Freebuff Desktop (Electron) instalados y en uso: NO tocar.
- Discos: H: 111,6 GB libres; C: 111,1; D: 511,9; I: 26,2; J: 29,4; K: 18,8; O: 709,9.

## 4. DECISIÓN TECNOLÓGICA

Elegido: **Tauri 2 + React + TypeScript + Rust + SQLite**.

Motivos verificados:

- WebView2 ya instalado → sin runtime extra.
- Rust MSVC + MSVC + Windows SDK ya operativos.
- Bundler NSIS oficial → instalador EXE sin dependencias previas.
- Binario ligero (~2,6 MB instalador) frente a 150-200 MB de Electron.
- Núcleo Rust testeable sin la UI.

Descartado: Electron (peso/RAM), PySide6 (empaquetado y control), .NET/WinUI (sin SDK .NET).

## 5. ESTRUCTURA

```
H:\KUKIA_CODE
  app\            UI React + TS (14 vistas)
  core\           crate kukia-core (lógica pura)
  src-tauri\      crate kukia-code (comandos Tauri, icono, capabilities)
  agents\ providers\ models\ memory\ tools\ terminal\ mcp\ research\
  security\ gpu\ benchmarks\ plugins\ config\ logs\ backups\ docs\ tests\ installer\
  INFORMES\
  package.json, tsconfig.json, vite.config.ts, Cargo.toml (workspace)
```

## 6. MÓDULOS DEL NÚCLEO (`core/src`)

- **config.rs**: `AppConfig` (default_provider, default_model, temperature, gpu_split [35,65], theme,
  cloud_base_url, cloud_model, has_cloud_key, research_auto). Datos en `%APPDATA%\KukiaCode`.
- **providers.rs**: `ProviderInfo`, `ModelInfo`, `ChatRequest/Response`, `ChatEvent` (token/done/error).
  Ollama: `GET /api/tags`, `POST /api/chat` (stream NDJSON). LM Studio/cloud: `GET /v1/models`,
  `POST /v1/chat/completions` (stream SSE). Tokens/s desde `prompt_eval_count`/`eval_count`/`total_duration`.
- **memory.rs**: SQLite con tablas `sessions`, `messages`, `memories`, `approvals`, `benchmarks`, `compat`,
  `radar_cache`, `research_items`, `research_alerts`, `research_runs`. Búsqueda LIKE, auditoría, backups.
- **gpu.rs**: `nvidia-smi` (uso, VRAM, temperatura, consumo, relojes) + `sysinfo` (CPU/RAM).
- **agents.rs**: 8 agentes iniciales (MASTER, CODER, AI RESEARCH DIRECTOR, EXPERIMENTAL RADAR,
  CYBERSECURITY, SYSTEM ADMIN, WEALTH RADAR, GPU OPTIMIZER) + agentes custom en `config/agents.json`.
- **security.rs**: PowerShell solo con `approved=true`; lectura de archivos con límite 2 MB.
- **logs.rs**: `%APPDATA%\KukiaCode\logs\kukia.log`.
- **radar.rs**: releases oficiales (GitHub API: `ollama/ollama`, `ggml-org/llama.cpp`) + fuentes manuales
  (LM Studio changelog, PolyStrata sin API pública verificada, CUDA, driver NVIDIA).
- **research.rs**: KUKIA EXPERIMENTAL AI RADAR. Escaneo arXiv (export API), Hugging Face (`/api/models`),
  GitHub (search). Ficha de 12 campos, puntuación heurística, alertas por NVFP4/FP4/expert parallel/
  MoE/KV cache/speculative decoding, informe diario, descarga aislada https máx 25 MB.

## 7. COMANDOS TAURI EXPUESTOS (camelCase en JS)

```
get_system_info, config_get, config_set,
list_providers, list_models, chat, chat_stream (Channel<ChatEvent>),
gpu_stats,
sessions_list, session_create, session_rename, messages_list, message_add,
memory_add, memory_search,
agents_list, agent_create,
files_list, files_read, files_write (approved),
run_command (approved),
benchmark_run, benchmarks_list, approvals_list,
radar_check, compat_add, compat_list,
research_scan, research_items, research_alerts, research_report, research_fetch, research_lab_path,
logs_tail
```

## 8. UI (`app/src/views`)

Chat (streaming + sesiones + agente), Modelos (proveedores + catálogo), Agentes (8 + crear),
Archivos (explorador + editor + aprobación), Terminal (PowerShell con aprobación), Tareas (estado real),
Memoria (búsqueda/alta), Herramientas (estado real), GPU (métricas 2 s + histórico SVG),
Benchmarks (tokens/s + tabla), Radar (releases + matriz compatibilidad), Research (escaneo + alertas +
fichas + informes + descarga aislada), Historial, Configuración (modelo, temperatura, split 35/65,
cloud, research_auto).

## 9. SEGURIDAD

- `files.write` y `terminal.run` exigen `approved=true`; sin ello devuelven error.
- Cada aprobación se registra en tabla `approvals`.
- Backup automático en `%APPDATA%\KukiaCode\backups` antes de sobrescribir.
- `KUKIA_CLOUD_KEY` solo por variable de entorno; nunca en disco.
- No auto-actualización, no instalación de nada, no modificación de Ollama/LM Studio/Pinokio/CUDA/drivers/modelos.
- No se toca KUKIA Network, ledger ni wallets.

## 10. PRUEBAS Y EVIDENCIAS REALES

- `npm run build`: `tsc --noEmit` OK; vite 7.3.7; 45+ módulos; `dist` generado.
- `npx tsc --noEmit` con vista Research: OK.
- `cargo test -p kukia-core`: 18 tests, **18 passed, 0 failed** (config, agents, memory, providers,
  gpu, security, logs, radar).
- Ollama real: `llama3-ablit` → prompt_eval_count 124, eval_count 21, total 366 ms;
  `xoxete-mistral` → eval_count 32, total 33.731 ms. Contrato API confirmado.
- Radar real: GitHub API → Ollama **v0.40.13** (coincide con instalada), llama.cpp **v0.8.0**.
- `npm run tauri:build`: release OK en 5m24s.
  - Binario: `H:\KUKIA_CODE\target\release\kukia-code.exe`
  - Instalador: `H:\KUKIA_CODE\target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe` (2,58 MiB)
- Reconstrucción final en curso para incluir el módulo Research en el binario.

## 11. CÓMO COMPILAR Y EJECUTAR

```powershell
cd H:\KUKIA_CODE
npm install
npm run tauri:dev      # desarrollo
npm run tauri:build    # instalador NSIS
```

Salida: `src-tauri\target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe`

## 12. LIMITACIONES REALES (no maquillar)

- Cliente MCP: **no implementado** (F6).
- Investigación autónoma con ejecución: **no implementada** (F9); el radar solo descubre e informa.
- Terminal pty (xterm.js): **no**; hay ejecución PowerShell con aprobación.
- Editor con resaltado (Monaco): **no**; editor de texto plano.
- Actualizador firmado: **no**; el radar solo informa.
- Reparto multi-GPU activo: **solo perfil guardado 35/65**; cada motor aplica lo que soporta.
- Descarga/gestión de modelos desde la app: **no** (F4).
- Licencias/DRM/pagos: **no** (F10).
- Firma de código: **no** (SmartScreen avisará).

## 13. REGLAS INNEGOCIABLES DEL PROYECTO

- Trabajar solo en `H:\KUKIA_CODE` (y laboratorio `H:\KUKIA_RESEARCH`).
- No modificar OpenCode, Freebuff, Pinokio, CUDA, drivers ni modelos.
- No borrar nada sin confirmación. Copiar → verificar → preguntar.
- Backup antes de modificar archivos.
- No cambios globales sin autorización.
- No servicios de pago sin permiso.
- No declarar terminado nada sin prueba real.

## 14. GOTCHAS TÉCNICOS ÚTILES

- Tauri 2: args de comandos en **camelCase** desde JS (`session_id` → `sessionId`).
- Streaming: `tauri::ipc::Channel<ChatEvent>` + tokio mpsc; eventos `{kind:"token"|"done"|"error"}`.
- `generate_context!` exige que exista `dist/` antes de compilar el crate Tauri.
- npm 11 `allow-scripts`: esbuild necesita `node node_modules/esbuild/install.js` si no se aprobó el postinstall.
- Icono: ICO con PNG embebido (cabecera 22 bytes) válido para el bundler.
- `rusqlite` bundled compila `sqlite3.c` con MSVC (primera build lenta).
- No usar ambas GPUs a la vez para modelos grandes.

## 15. SIGUIENTE FASE PROPUESTA

F3: permisos por herramienta y por proyecto, auditoría avanzada, recuperación de tareas interrumpidas.
Después: F4 conectores (PolyStrata/llama.cpp/gestión de modelos), F5 UI avanzada, F6 MCP,
F7 memoria semántica, F8 pty, F9 research autónomo, F10 updater firmado y release 1.0.
