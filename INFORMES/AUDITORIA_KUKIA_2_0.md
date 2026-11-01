# AUDITORÍA KUKIA CODE 2.0 — SOLO LECTURA

**Fecha:** 01/11/2026 · **Autor:** Xoxete (para Kuki / LordKukone)
**Raíz:** `H:\KUKIA_CODE` · **Laboratorio:** `H:\KUKIA_RESEARCH` · **Datos:** `%APPDATA%\KukiaCode`
**Modo:** auditoría de solo lectura. **No se ha modificado una sola línea de código ni configuración.**
Lo único escrito es este informe.

---

## VEREDICTO

El proyecto **no está a cero ni hay que reconstruirlo**: hay 10.410 líneas propias, 55 comandos,
18 vistas, 56 tests en verde y tres motores integrados de verdad. Pero el superprompt 2.0 pide un
salto de categoría (cerebro, router, verdad, autonomía) y para eso **faltan cimientos que aún no
existen**: no hay parámetros de inferencia, no hay adaptadores de motor, no hay router, no hay
verificación independiente y no hay memoria semántica.

Y hay dos cosas que pesan más que cualquier módulo nuevo: **la app instalada es de Fase 2.1** y
**todo el trabajo está sin commitear**.

---

## A. ESTADO DE ARQUITECTURA

```
app/         React 19 + TypeScript 5.8 + Vite 7        (3.538 líneas · 18 vistas)
src-tauri/   Tauri 2, comandos y estado IPC             (1.288 líneas · 55 comandos)
core/        kukia-core, lógica pura Rust sin Tauri     (5.584 líneas · 14 módulos)
core/tests/  4 suites de integración                    (738 líneas)
```

| Dato | Valor medido |
|---|---|
| Módulos del núcleo | 13 + `lib` (`memory` 1.298 · `anime_real` 729 · `providers` 642 · `gaming` 548 · `permissions` 509 · `research` 486 · `vram` 439 · `security` 318 · `agents` 181 · `gpu` 146 · `config` 123 · `radar` 101 · `logs` 41) |
| Comandos Tauri | **55** |
| Vistas | **18** (Chat, Modelos, Agentes, Archivos, Terminal, Tareas, Seguridad, Memoria, Herramientas, GPU, Benchmarks, Radar, Research, Anime↔Real, VRAM Guard, Gaming Radar, Historial, Configuración) |
| Tests | **56 unitarios** + 22 de integración, **todos en verde hoy** |
| Dependencias núcleo | 10 directas (serde, serde_json, reqwest, tokio, futures-util, rusqlite, chrono, sysinfo, thiserror, uuid) |
| Dependencias Tauri | 6 (tauri, serde, serde_json, tokio, chrono, kukia-core) |
| Dependencias UI | 3 de ejecución (`@tauri-apps/api`, `react`, `react-dom`) + 6 de desarrollo |
| Separación de capas | ✅ correcta: `core` no conoce Tauri; se prueba en solitario |
| Dependencias circulares | ninguna detectada |

**Lo que la arquitectura NO tiene todavía:** capa de adaptadores por motor (§10 del superprompt),
sistema de eventos tipados, colas asíncronas de tareas, migraciones versionadas de SQLite,
y abstracción de herramientas para agentes.

---

## B. FUNCIONALIDADES EXISTENTES Y PROBADAS (15)

| # | Funcionalidad | Prueba ejecutada |
|---|---|---|
| B1 | Proyecto único sin duplicar | `H:\KUKIA_CODE`, 2 commits en `master` |
| B2 | Chat Ollama con streaming y tokens/s | conector nativo NDJSON, 81 modelos |
| B3 | Chat LM Studio / OpenAI-compatible | puerto **1235** responde 200 ahora |
| B4 | Memoria SQLite: sesiones, mensajes, memorias | 16 tablas, `integrity_check ok` |
| B5 | Monitor GPU (VRAM, uso, temperatura, relojes) | `nvidia-smi` detecta 5070 + 5060 Ti |
| B6 | Permisos por herramienta aplicados | 7 puntos de control + 9 tests |
| B7 | Allowlist de rutas y protección contra `..` | `escritura_fuera_de_las_rutas_se_deniega` |
| B8 | Timeout y cancelación de comandos | `timeout_secs` ×4, cancelación ×16 |
| B9 | Auditoría real y consultable | helper `auditar()` con 21 llamadas + vista |
| B10 | Radar de releases (Ollama, llama.cpp) | `radar_check` con caché |
| B11 | Research Radar arXiv/HF/GitHub | 35 fichas, informe 17.824 B |
| B12 | VRAM Guard con 3 motores | `competencia: true` con Ollama + ComfyUI |
| B13 | Gaming Radar | 19 fichas, 3 alertas, informe 20.725 B |
| B14 | Anime ↔ Real (imagen) | 3 plantillas, 16,7 s por la ruta Rust |
| B15 | Logs de aplicación | `logs_tail` + vista |

---

## C. FUNCIONALIDADES INCOMPLETAS (16)

| # | Qué | Existe | Falta |
|---|---|---|---|
| C1 | **KV Cache** | nada | tipo, estimación, medición real, comparación, recomendación |
| C2 | **Parámetros de inferencia** | `temperature`, `max_tokens` | `num_ctx`, `num_gpu`, `num_batch`, `num_parallel`, `keep_alive`, `top_k/p`, `min_p`, `repeat_penalty`, `seed`, `num_predict`, JSON, tools |
| C3 | AI Engine Center | chat y listado | adaptadores, cargar/descargar, VRAM y latencia por motor, logs |
| C4 | Panel Ollama | `list_models`, `ps` | descargar, borrar, GGUF, Modelfile, blobs, huérfanos, integridad |
| C5 | Panel LM Studio | detección 1234/1235 | offload, capas, flash attn, KV K/V, batch, threads, mmap/mlock |
| C6 | GPU Router | perfil 35/65 guardado | reparto real de capas, fijar GPU por motor, PCIe, comparación |
| C7 | Auto Optimizer | nada | 10 perfiles, simulación, comparación con lo actual |
| C8 | PolyStrata | instalado en `D:\PolyStrata`, **8090 cerrado** | arranque, API, modelo, comparación |
| C9 | ComfyUI multimodal | solo img2img SDXL | vídeo, audio, FLUX, ControlNet, PuLID/InstantID |
| C10 | Anime ↔ Real vídeo | 2 grafos localizados | formato API y prueba |
| C11 | Agentes | 8 perfiles con `tools` como texto | **ejecución real de herramientas** |
| C12 | Memoria / RAG | `LIKE` sobre SQLite | embeddings, FTS5, búsqueda semántica |
| C13 | Benchmarks | `benchmark_run` tok/s | comparador entre motores con condiciones |
| C14 | Gaming de juegos | radar de tecnologías | detección de juegos, perfiles DLSS, conflictos |
| C15 | Dev Lab | — | Nuitka, NASM, Ghidra, Volatility, Sysinternals |
| C16 | Interfaz | 18 vistas | faltan ~13 paneles del listado §28 |

---

## D. MÓDULOS HUÉRFANOS

**Módulos: cero.** Verificado módulo a módulo — todos los del núcleo tienen referencias desde
`src-tauri`: `logs` 21, `providers` 14, `anime_real` 13, `research` 12, `config` 10, `memory` 10,
`gaming` 8, `permissions` 7, `security` 7, `agents` 4, `gpu` 4, `vram` 3, `radar` 2.

**Función huérfana: un subsistema entero.** La tabla `tasks` tiene tres funciones escritas y
**cero llamadas**: `task_upsert` 0 · `tasks_interrupted` 0 · `task_set_status` 0. La vista
`Tareas` sigue siendo estática y lo dice en pantalla. Es el único resto del patrón antiguo
(escribir el motor y no conectarlo) que ya se corrigió en VRAM y Gaming.

---

## E. PROBLEMAS DE SEGURIDAD

**Lo que ya está bien (con prueba):**

- Canonicalización de rutas resolviendo junctions y quitando `\\?\` (`permissions.rs:64-74`).
- Test específico `junction_no_permite_escapar` (E-04).
- 14 patrones destructivos bloqueados (`rd /s /q`, `truncate table`, …).
- Salida de comandos recortada a **1 MB** (`MAX_OUTPUT_BYTES`) con marca `truncated`.
- `policy_save` **exige** `approved: true`; un agente no puede cambiarse los permisos solo.
- `files_write` copia y verifica tamaño antes de sobrescribir.
- Auditoría de 21 puntos, incluida la exportación.

**Lo que falta:**

| # | Problema | Impacto |
|---|---|---|
| E-1 | La base `kukia.db` está **sin cifrar** | Quien lea el disco lee tu historial y la auditoría |
| E-2 | Sin **WAL** (journal por defecto) | Escrituras más lentas y bloqueos con lectura simultánea |
| E-3 | Sin **gestión de secretos** (la clave cloud va por variable de entorno) | Correcto hoy, insuficiente para varias claves |
| E-4 | Sin **rollback** más allá del backup de un archivo | Un cambio en varios archivos no se deshace solo |
| E-5 | Protección de `I:\CEREBRO` es **política de la app**, no del sistema | Si otra herramienta escribe, la app no lo impide |
| E-6 | Sin límite de salida acumulada en streaming | Un comando muy hablador puede llenar memoria |

---

## F. PROBLEMAS DE RENDIMIENTO

| # | Medido | Efecto |
|---|---|---|
| F1 | `journal_mode` = **0 apariciones** (sin WAL) | Escrituras y lecturas compiten |
| F2 | Solo **2 índices** (`messages.session_id`, `audit.created_at`) | Búsquedas por sesión/fecha sin índice en tablas grandes |
| F3 | `memory_search` usa **LIKE** (2 apariciones) | Sin relevancia ni escalado |
| F4 | `/object_info` de ComfyUI son **~22 MB** y se parsea entero por consulta | 1-2 s por comprobación de capacidades |
| F5 | Sondeo de ComfyUI cada **1,5 s** | Aceptable en local, mejorable con WebSocket |
| F6 | **81 modelos** en Ollama sin caché de metadatos | `/api/tags` completo en cada listado |
| F7 | `target/` ocupa **8,1 GB** | Con 102 GB libres, un release completo aprieta |
| F8 | PowerShell bloqueante por comando | Sin reutilización de proceso |

---

## G. DEPENDENCIAS

**Núcleo (10):** serde, serde_json, reqwest (json+stream), tokio (full), futures-util, rusqlite
(bundled), chrono, sysinfo, thiserror, uuid.
**Tauri (6):** tauri, serde, serde_json, tokio, chrono, kukia-core (path).
**UI (3 + 6 dev):** `@tauri-apps/api`, react, react-dom · vite, typescript, plugin-react, cli, tipos.

**Ausencias que el superprompt 2.0 necesitará:** embeddings (`fastembed`/`candle`), FTS5
(ya disponible en rusqlite), MCP (`rmcp`), pty (`portable-pty`), Monaco, cliente HTTP con
WebSocket, vector store.

---

## H. ESPACIO DISPONIBLE

| Ruta | Estado |
|---|---|
| `H:` | **102 GB libres** de 2,8 TB → **97 % ocupado** |
| `H:\KUKIA_CODE` | 8,2 GB, de los cuales **8,1 GB son `target/`** |
| `node_modules` | 86 MB · `output` 16 MB · `dist` 305 KB · `backups` 294 KB |
| `H:\KUKIA_RESEARCH` | `AI_EXPERIMENTAL` + `GAMING_GRAPHICS` |

**Consecuencia:** limpiar `target/` liberaría 8,1 GB sin tocar código (requiere tu permiso).
Descargar Flint 40B (≥20 GB en cualquier cuantización usable) **no cabe con holgura**.

---

## I. INTEGRACIONES EXISTENTES (comprobadas ahora)

| Integración | Estado |
|---|---|
| Ollama `127.0.0.1:11434` | ✅ **200** · 81 modelos |
| LM Studio `127.0.0.1:1235` | ✅ **200** (1234 cerrado) |
| ComfyUI `127.0.0.1:8188` | ✅ **200** · 2.745 nodos · 61 checkpoints · 660 LoRAs |
| GitHub API | ✅ **200** (sin token: 10 búsquedas/min) |
| arXiv API | ✅ **301** → funciona con redirección |
| Hugging Face API | usada por `research.rs` |
| PolyStrata `:8090` | ❌ **cerrado** (instalado en disco) |
| `nvidia-smi` / sysinfo | ✅ 2 GPUs |

---

## J. QUÉ SE PUEDE REUTILIZAR (no rehacer)

| Pieza | Sirve para |
|---|---|
| `permissions.rs` (509 líneas) | base del Security Center |
| `memory.rs` (1.298 líneas, 16 tablas) | Eternal Memory + auditoría + políticas |
| `providers.rs` | base del Engine Center (Ollama y OpenAI-compatibles ya escritos) |
| `vram.rs` | Digital Twin y GPU Router (ya mide 3 motores y competencia) |
| `research.rs` + `gaming.rs` | Knowledge/Research Engine y Gaming (scanners ya probados) |
| `anime_real.rs` | Multimodal Studio (cliente ComfyUI genérico) |
| `benchmark_run` + tabla | Model Lab y Benchmark comparador |
| `auditar()` + `audit_log` | Truth Engine (evidencia de cada acción) |
| Vista `Seguridad` | centro de permisos y auditoría |
| `radar.rs` | aviso de actualizaciones |

---

## K. QUÉ REQUIERE DESARROLLO NUEVO

Cerebro coordinador · Model Router con puntuación · adaptadores de motor independientes ·
KV Cache Lab · GPU Router con asignación · Auto Optimizer con simulación · Digital Twin ·
Truth Engine · memoria semántica con embeddings y FTS5 · Knowledge Graph · Experience Engine ·
Task Market con dependencias · Autopilot con límites · Self-Improve con rama aislada ·
Desktop Intelligence (UIA/OCR) · Multi-PC con workers · Dev Lab · agentes con herramientas reales
y MCP · actualizador firmado.

---

## L. RIESGOS POR FASE

| Fase | Riesgo principal | Mitigación |
|---|---|---|
| Rebuild + commit | Instalador cambia el programa del sistema | Backup del exe actual + hash antes/después |
| KV Cache | Variables **globales**: afectan a todo | Backup, cambio temporal, medición, rollback |
| Engine Center | Borrar modelos por error | Confirmación por nombre + papelera, no `rm` |
| Router | Decisiones opacas | Registrar cada elección con motivo |
| Truth Engine | Falsa autoridad | Nunca validar un modelo con sigo mismo |
| Agentes con tools | Comandos sin control | Permisos + timeout + presupuesto ya probados |
| Multi-PC | Prometer VRAM sumada | Medir `llama.cpp RPC` antes de afirmar nada |
| Vídeo anime↔real | VRAM y tiempo | Una plantilla, medida, antes de UI |

---

## M. ESTIMACIÓN DE SESIONES (honesta)

| Bloque | Sesiones |
|---|---|
| Cerrar deuda (commit, build, instalar, docs §32) | 1-1,5 |
| KV Cache + parámetros + panel Ollama/LM Studio | 4-6 |
| Engine Center + adaptadores | 3-4 |
| Model Router + Benchmark comparador | 3-4 |
| GPU Router + Digital Twin + Auto Optimizer | 4-6 |
| Truth Engine + Eternal Memory + Knowledge Graph | 7-10 |
| Agentes reales + MCP + Task Market | 8-11 |
| Autopilot + Self-Improve | 5-7 |
| Multimodal + Anime↔Real vídeo | 5-7 |
| Desktop Intelligence | 4-6 |
| Multi-PC | 5-7 |
| Dev Lab + Gaming de juegos | 3-4 |
| Release firmado + documentación | 3-4 |
| **Total** | **55-80 sesiones** |

Cualquier plan que diga menos, te está mintiendo.

---

## N. ORDEN ÓPTIMO (por dependencia real, no por el PDF)

1. **Deuda cero:** commit + build + instalar + los 7 documentos del §32.
2. **KV Cache + parámetros de inferencia** (C1, C2) — es el suelo de todo lo demás.
3. **Benchmark comparador** (C13) — sin medir, el router y el optimizer son adivinación.
4. **Engine Center con adaptadores** (C3, C4, C5) — fijar GPU por motor incluido.
5. **Model Router** (C6 parcial + nuevo) — ya con datos del benchmark.
6. **GPU Router + Digital Twin + Auto Optimizer** (C6, C7).
7. **Truth Engine** — antes de dar autonomía a nada.
8. **Eternal Memory + Knowledge Graph** (C12).
9. **Agentes con herramientas + MCP + Task Market.**
10. **Autopilot + Self-Improve** (solo con 7 y 9 hechos).
11. **Multimodal + vídeo anime↔real** (independiente; se puede adelantar si lo prefieres).
12. **Desktop Intelligence**, **Multi-PC**, **Dev Lab**, **Release**.

**Regla de oro:** nada de autonomía (Autopilot, Self-Improve, Swarm) antes de tener Truth Engine
y Benchmark. Si no, el sistema se cree sus propias mentiras.

---

## LAS CINCO MEJORAS CON MÁS BENEFICIO Y MENOS COSTE

| # | Mejora | Coste | Beneficio real |
|---|---|---|---|
| **1** | **Commit + recompilar + instalar** (y los 7 documentos del §32) | **0,5-1 sesión** | Pasa todo lo hecho de invisible a usable; asegura el trabajo ante un fallo de disco |
| **2** | **KV Cache Lab + parámetros de inferencia** | **2-3 sesiones** | Con 12/16 GB, el tipo de KV y el contexto son la diferencia entre 8K y 32K reales. Es la palanca de rendimiento más grande que tienes |
| **3** | **Benchmark comparador entre motores** | **1 sesión** | Convierte "creo que va mejor" en números. Reutiliza `benchmark_run` y su tabla ya existentes |
| **4** | **Fijar GPU por motor + Engine Center mínimo** | **2-3 sesiones** | Hoy Ollama y ComfyUI se metieron los dos en la 5060 Ti (medido). Fijar dispositivo lo evita |
| **5** | **Truth Engine mínimo: verificación por ejecución** | **2 sesiones** | Tests, hashes y salidas reales como evidencia. Es lo que impide que la app te venda humo, incluido el mío |

Suma: **7,5-10 sesiones** para tener una plataforma que se ve, se mide y no se cree a sí misma.

---

## LO QUE NECESITO DE TI

1. ¿Apruebas la **mejora 1** (commit + recompilar + instalar)?
2. ¿Apruebas el **orden** de la sección N, o adelantamos el vídeo anime↔real?
3. ¿Me dejas **tocar variables globales de Ollama** en la mejora 2 (con backup y rollback)?
4. ¿Puedo **limpiar `target/`** (8,1 GB) antes del release?

**No he modificado nada. Espero tu palabra antes de escribir la primera línea.**
