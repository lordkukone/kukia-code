# KUKIA CODE — FUNCIONALIDADES PENDIENTES

**Fecha:** 01/11/2026 · **Autor:** Xoxete
**Base:** auditoría de solo lectura del 01/11/2026 (`INFORME_AUDITORIA_FREEBUFF.md`)
**Clasificación:** las cinco categorías que pediste, sin excepciones ni matices de marketing.

---

## 0. LEYENDA Y RECUENTO

| Clase | Significado |
|---|---|
| **A** | IMPLEMENTADA Y VERIFICADA — hay código y una prueba ejecutada hoy |
| **B** | IMPLEMENTADA SIN VERIFICAR — hay código, no hay prueba de hoy |
| **C** | PARCIAL — hay una pieza, falta otra (código sin conectar, o UI sin motor) |
| **D** | NO IMPLEMENTADA — no existe el código |
| **E** | BLOQUEADA — no se puede hacer con lo que hay ahora mismo |

| Clase | Cuántas |
|---|---:|
| A | 9 |
| B | 2 |
| C | 11 |
| D | 18 |
| E | 2 |

---

## 1. CLASE A — IMPLEMENTADAS Y VERIFICADAS HOY

| # | Funcionalidad | Prueba que lo respalda |
|---|---|---|
| A1 | Chat con Ollama (modelos, streaming, tokens/s) | `cargo test` + conector documentado en F2.1 con 81 modelos |
| A2 | Chat con LM Studio / OpenAI-compatible (puertos 1234 y 1235) | sondeo en `providers.rs`; F2.1: 32 modelos, `"OK"` en 531 ms |
| A3 | Memoria SQLite (sesiones, mensajes, memorias) | 39 tests; base real `integrity_check = ok`, 1 sesión / 2 mensajes |
| A4 | Monitor GPU (nvidia-smi: VRAM, uso, temperatura, relojes) | `vram::tests::detecta_gpus_reales` detecta RTX 5070 y RTX 5060 Ti |
| A5 | Radar de releases (Ollama, llama.cpp) | `radar::tests`; consulta real en F2.1 |
| A6 | KUKIA Experimental AI Radar (arXiv/HF/GitHub, alertas, informe, descarga aislada) | 35 items / 10 alertas / informe 17.824 bytes; descarga https ≤25 MB registrada |
| A7 | Logs de aplicación | `logs::tests::append_and_tail` |
| A8 | Backups verificados antes de sobrescribir | `files_write` cancela si el tamaño no coincide (corregido en F2.1) |
| A9 | Instalador NSIS + arranque instalado | hashes `a333bbb7…` y `859a8078…` idénticos a lo documentado |

---

## 2. CLASE B — IMPLEMENTADAS SIN VERIFICAR

| # | Funcionalidad | Qué falta exactamente |
|---|---|---|
| B1 | Persistencia de configuración en `%APPDATA%\KukiaCode\config\config.json` | **El archivo no existe**. `save_config` está escrito y escribe ahí, pero en la app instalada nadie ha guardado ajustes. No hay prueba de que el JSON sobreviva a un reinicio con datos reales |
| B2 | Proveedor cloud (`KUKIA_CLOUD_KEY`) | Código presente; **nunca ejecutado** contra un endpoint real. Sin clave configurada |

---

## 3. CLASE C — PARCIALES (lo importante: qué lado falta)

Estas once son el **trabajo real pendiente de Fase 3**. En todas, el lado que falta es el mismo:
**la conexión con la aplicación**.

| # | Funcionalidad | Existe | Falta |
|---|---|---|---|
| C1 | Permisos por herramienta | `permissions::check` + 6 herramientas + 10 tests | Comando `policy_get`/`policy_set` y vista; **y que `files_write`/`run_command` lo consulten** |
| C2 | Allowlist de directorios | `path_in_list`, `normalize` (bloquea `..`), `read_only_paths` | Aplicación en la ruta de escritura real |
| C3 | Permisos por proyecto | `AgentPolicy` con `read_paths`/`write_paths` | Concepto de «proyecto» (no existe en el modelo de datos) |
| C4 | Auditoría consultable con filtros | Tabla `audit`, `audit_search`, `audit_export_csv` | **Nadie llama a `audit_log`** → 0 filas. No hay vista ni exportación en UI |
| C5 | Recuperación de tareas interrumpidas | Tabla `tasks`, `task_upsert`, `tasks_interrupted`, `task_set_status` | Ningún comando; la vista `Tareas` es estática |
| C6 | Estado persistente de agentes | `agents.json` + `agent_create` | El estado de ejecución de un agente no se guarda |
| C7 | VRAM Guard | `vram.rs` completo, competencia Ollama/LM Studio, `fits()` con 12 % de margen | Comando `vram_report` y vista (la actual `GPU` no lo usa) |
| C8 | Gaming Technologies Radar | `gaming.rs`: 9 repos, 6 búsquedas, arXiv, clasificación por madurez y `RIESGO_ALTO` para unlock | Comando, vista e informe semanal conectados |
| C9 | Benchmark Center | `benchmark_run` mide tok/s y persiste | Comparación entre modelos/condiciones; guardar VRAM y temperatura junto al resultado |
| C10 | Model Router | `list_models`, selector manual, perfil GPU 35/65 guardado | Puntuación automática por VRAM/contexto/velocidad/calidad; **el perfil 35/65 no reparte nada** |
| C11 | Laboratorio experimental | `H:\KUKIA_RESEARCH\AI_EXPERIMENTAL` con 8 subcarpetas, ficha de 12 campos | Nada multimodal (imagen/vídeo/audio); sin ComfyUI, sin LoRAs, sin TTS |

---

## 4. CLASE D — NO IMPLEMENTADAS

Ordenadas por dependencia real (no por el orden del superprompt).

| # | Funcionalidad | Qué implicaría |
|---|---|---|
| D1 | **Timeout y cancelación de comandos** | `run_command` lanza PowerShell y espera sin límite. Un comando colgado bloquea la app |
| D2 | **Protección anti-bucle / anti-duplicados** | Sin supervisor de ejecución no hay nada que proteger |
| D3 | **Terminal pty real (xterm.js)** | `portable-pty` no está en dependencias |
| D4 | **Cliente MCP** | `mcp/` vacía; sin `rmcp` en `Cargo.toml` |
| D5 | **Editor con resaltado (Monaco)** | Texto plano |
| D6 | **Memoria semántica / embeddings / RAG** | `memory_search` es `LIKE`; sin modelo de embeddings |
| D7 | **Agentes con tools reales** | Los 8 agentes son prompts con una lista de nombres de herramientas que **no se ejecutan** |
| D8 | **Agent Swarm (paralelo, revisión cruzada)** | Requiere D7 y D1 primero; y bloqueo por archivo para no pisarse |
| D9 | **Universal Desktop Control (UIA/FlaUI/pywinauto)** | Sin crate `windows`/`uiautomation`, sin OCR, sin visión |
| D10 | **Distributed AI (segundo PC)** | Sin red, sin cola, sin descubrimiento, sin autenticación |
| D11 | **MULTI-PC Center** | Depende de D10 |
| D12 | **Descubrimiento de red local** | — |
| D13 | **Scheduler remoto de trabajos** | — |
| D14 | **Model Registry con capacidades medidas** | Hay nombres de modelos, no capacidades verificadas |
| D15 | **Investigación autónoma con ejecución por pasos** | El radar descubre e informa; no investiga solo |
| D16 | **Actualizador firmado** | Sin `tauri-plugin-updater`, sin firma |
| D17 | **Firma de código / licencias** | — |
| D18 | **`PROGRESO_GENERAL.md` / `CONTINUAR_TRABAJO.md` / `FASES_COMPLETADAS.md`** | No existen; la continuidad se lleva en `INFORMES\` |

---

## 5. CLASE E — BLOQUEADAS (y por qué)

| # | Funcionalidad | Bloqueo real |
|---|---|---|
| E1 | **Sumar VRAM de dos equipos (tensor parallel entre PCs)** | **No es un problema de programación.** Medido en tu propia máquina: repartir un modelo entre GPU0 y GPU1 ya penaliza; entre equipos, con enlace gigabit, el cuello es la red. La única vía realista es `llama.cpp RPC`, y hay que **medirla antes de prometerla**. Con 106 GB libres en H: además no hay sitio para pesos duplicados |
| E2 | **«Control universal» de cualquier aplicación Windows** | Bloqueada **como afirmación**: con UI Automation se cubren apps Win32/WPF/UIA; juegos con anti-cheat, apps Electron mal etiquetadas y canvas propietarios **no** se controlan por árbol de accesibilidad. Se puede hacer «amplio», no «universal». Declararlo universal sería mentir |

---

## 6. DEFINICIÓN DE «HECHO» PARA FASE 3

Para no volver a tener módulos escritos y desconectados, cada funcionalidad de Fase 3 se dará por
terminada solo si cumple **las cuatro**:

1. **Comando Tauri** registrado en `generate_handler!`.
2. **Vista** que lo use (o botón dentro de una vista existente).
3. **Test** que lo ejecute sin red cuando sea posible.
4. **Entrada en el informe** con la salida real pegada.

Si falta cualquiera de las cuatro, la casilla se queda en **C (parcial)**. Es la regla que impide
repetir el patrón actual: `permissions.rs`, `vram.rs` y `gaming.rs` cumplen 3 de 4 (les falta el 1 y
el 2).

---

## 7. LO QUE YO **NO** HARÍA TODAVÍA

- **Agent Swarm** antes de D1 (timeout) y C1 (permisos aplicados): varios agentes ejecutando
  comandos sin límite y sin allowlist es la forma más rápida de romper el proyecto.
- **Distributed AI** antes de tener un solo equipo estable con Fase 3 conectada.
- **Universal Desktop Control** antes de decidir si de verdad lo necesitas: es la funcionalidad más
  cara de mantener de toda la lista y la que más se degrada con cada actualización de Windows.
- **Cualquier cosa que toque drivers, anti-cheat o inyectores de juegos**: está fuera de las reglas
  del proyecto y de lo que la app debe hacer.
