# KUKIA CODE — PROGRESO GENERAL

**Última actualización:** 01/11/2026 · **Autor:** Xoxete

## Identidad del proyecto

- Código: `H:\KUKIA_CODE` (repo git `master`, sin remoto)
- Laboratorio: `H:\KUKIA_RESEARCH`
- Datos de usuario: `%APPDATA%\KukiaCode` (`kukia.db`, `config\config.json`, `logs\`, `secrets.json`)
- Instalada: `C:\Program Files\KUKIA CODE\kukia-code.exe`
- Stack: Tauri 2 · React 19 · TypeScript · Vite 7 · Rust · SQLite

## Estado actual, en números

| Dato | Valor (01/11/2026) |
|---|---|
| Tests Rust | **98 pasan · 12 ignorados · 0 fallos** |
| Comandos Tauri | 62 |
| Vistas de interfaz | 18 en la barra + estados internos |
| Instalador | NSIS 0.1.0 · SHA-256 `509646014c93dd858c6f6fededccdb883c813423aaa2e6461bdf910faadf088f` |
| Rebuild completo | 2 min 23 s – 3 min 13 s (`npx tauri build`) |
| Instalación | `perMachine`, requiere administrador |

## Fases cerradas

| Fase | Contenido | Prueba |
|---|---|---|
| 1 | Núcleo: proveedores, memoria, GPU, radar, logs | 39 tests |
| 2 | MVP instalable, chat local, SQLite, benchmarks | 94→ tests, instalador NSIS |
| 2.1 | Validación de release, 4 fallos corregidos | informe + benchmark tok/s |
| 3.0 | Permisos aplicados en escritura y comando, timeout con cancelación, `audit_log` conectado | `validacion_fase30.rs` |
| 3A | Permisos por proyecto; tests fuera del log y de la base reales | `validacion_fase3a.rs` |
| 3.1 | VRAM Guard y Gaming Radar conectados a comandos y vistas | `validacion_gaming.rs`, VRAM real |
| 3.2 | Anime ↔ Real con ComfyUI (SDXL, IP-Adapter, img2img) | `validacion_anime_real.rs` |
| 3.3 | Commit, release e instalación verificada | `INFORMES/FASE_3_3_RELEASE.md` |
| 3.4 | Aviso automático de competencia de VRAM + CSP real + etiqueta Fase 3 | `INFORMES/FASE_3_4_AVISO_VRAM.md` |

## Lo que funciona hoy, comprobado en la app instalada

- Chat con Ollama, LM Studio y proveedor cloud (OpenAI-compatible), con streaming.
- Selector de proveedor: `opencode`, `freebuff`, `openai`, personalizado; clave en cascada
  (entorno → `secrets.json` → `auth.json` de OpenCode en solo lectura) y nunca mostrada.
- VRAM Guard midiendo GPUs y motores reales, **con aviso automático** cuando dos motores
  tienen modelos cargados a la vez.
- Gaming Radar con fichas y alertas por nivel.
- Anime ↔ Real contra ComfyUI real, con historial y previsualizaciones.
- Seguridad: política de permisos por agente y por proyecto, timeout y cancelación de comandos,
  auditoría consultable y exportable.
- 9 políticas de agente sembradas en la base al arrancar.

## Lo que NO está

- Tool calling en ningún proveedor (bloquea agentes cloud).
- Almacenamiento seguro de claves (hoy `secrets.json` en claro, sin DPAPI).
- Coste/presupuesto de nube, router automático local+cloud, kill switch de nube.
- Multi-PC real (solo diseño en `INFORMES/ARQUITECTURA_MULTI_PC.md`).
- Updater firmado, firma de código, notificaciones del sistema.
- Memoria semántica con embeddings (la búsqueda es `LIKE`).
- Terminal pty real, MCP, editor con resaltado.

## Mapa de documentos

| Para qué | Dónde |
|---|---|
| Qué está hecho y qué no, por clase | `INFORMES/FUNCIONALIDADES_PENDIENTES.md`, `MATRIZ_FUNCIONALIDADES.md` |
| Auditoría de la ampliación cloud | `INFORMES/AUDITORIA_CLOUD_OMEGA.md` |
| Cómo seguir mañana | `CONTINUAR_TRABAJO.md` |
| Historial de cambios | `CHANGELOG.md` |
| Plan de release | `PLAN_RELEASE.md` |
| Arquitectura y seguridad | `docs/ARQUITECTURA.md`, `docs/SEGURIDAD.md` |
