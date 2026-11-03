# KUKIA CODE OMEGA — FASE 0 (AUDITORÍA)

**Fecha:** 01/11/2026 · **Estado:** solo lectura. **No se ha modificado ni una línea de código en esta fase.**

Backup de referencia de la fase anterior: `BACKUPS/MODEL_CONTROL_20261101_222304` (SHA-256 verificado).

---

## 1. Qué hay hoy, medido

| Dato | Valor |
|---|---|
| Núcleo Rust | **11.426 líneas** en 20 ficheros (`core/src`) |
| Interfaz | **6.076 líneas** en 22 ficheros (`app/src`) |
| Comandos Tauri | **94** |
| Tablas SQLite | **20** (`sessions`, `messages`, `memories`, `approvals`, `benchmarks`, `compat`, `research_*`, `radar_cache`, `audit`, `agent_policies`, `tasks`, `gaming_*`, `projects`) |
| Vistas de la app | **20** |
| Tests del núcleo | **105** unitarios + 4 ficheros de integración real |

### Módulos del núcleo, por peso

```
memory.rs 1430 · lab.rs 1183 · vram.rs 1125 · control.rs 1069 · engines.rs 1039
anime_real.rs 755 · providers.rs 711 · studio/detect.rs 668 · permissions.rs 608
gaming.rs 548 · research.rs 486 · config.rs 338 · security.rs 318
studio/engines.rs 291 · studio/styles.rs 275 · agents.rs 181 · gpu.rs 146
logs.rs 113 · radar.rs 101 · studio/mod.rs 13
```

---

## 2. Mapa de arquitectura (lo que ya existe)

```
┌──────────────────────────────────────────────────────────────────────┐
│ INTERFAZ (React 19 + TS + Vite 7)                                    │
│  Chat · Modelos(+Control avanzado) · Laboratorio · VRAM Guard        │
│  GPU · Benchmarks · Convertidor · Anime↔Real · Agentes · Memoria     │
│  Seguridad · Tareas · Herramientas · Research · Radar · Gaming       │
│  Historial · Configuración · Archivos · Terminal                     │
└───────────────┬──────────────────────────────────────────────────────┘
                │ invoke() — 94 comandos
┌───────────────▼──────────────────────────────────────────────────────┐
│ src-tauri/lib.rs  (estado, watchers, logs, política)                 │
└───────────────┬──────────────────────────────────────────────────────┘
                │
┌───────────────▼──────────────────────────────────────────────────────┐
│ core/                                                                │
│  ADAPTADORES: providers.rs · engines.rs (Ollama/LM Studio)           │
│               anime_real.rs · studio/* (ComfyUI/Wan2GP/Maestro)      │
│  MEMORIA/GPU: vram.rs · gpu.rs · control.rs · lab.rs                 │
│  SEGURIDAD:   permissions.rs · security.rs · logs.rs                 │
│  CONOCIMIENTO: research.rs · gaming.rs · radar.rs                    │
│  DATOS:       memory.rs (SQLite) · config.rs                         │
│  AGENTES:     agents.rs (registro y políticas, sin ejecución)        │
└───────────────┬──────────────────────────────────────────────────────┘
                │
        SQLite (%APPDATA%\KukiaCode) · nvidia-smi · HTTP a motores
```

---

## 3. Duplicidades reales (antes de añadir nada)

1. **Tres escáneres de descubrimiento** con el mismo trabajo: `research.rs` (arXiv,
   HuggingFace, GitHub), `gaming.rs` (GitHub releases, GitHub search, arXiv) y
   `radar.rs` (actualizaciones de repos). piden el mismo tipo de datos con tres
   implementaciones distintas. Deben converger en un solo motor de descubrimiento.
2. **Dos sistemas de medición**: `benchmark_run` (SQLite `benchmarks`) y
   `lab::benchmark` (`laboratorio.json`). Miden lo mismo con esquemas distintos.
3. **Tres formas de «liberar memoria»**: `vram_unload`, `vram_unload_all` y
   `vram_release_*`. Las tres funcionan, pero la interfaz ya solo usa las medidas;
   las dos antiguas deberían quedar como envoltorios o retirarse con tu permiso.
4. **Tres ejecutores de comandos**: `run_command`, `run_command_stream`,
   `run_powershell` y `security::run_controlled`. Hay política aplicada de forma
   desigual: el camino directo no pasa por `permissions::check`.
5. **Dos almacenes de perfiles**: `inferencia.json` (por modelo) y
   `perfiles_modelo.json` (perfiles). Conviven sin problema, pero conviene unificar
   la lectura en un solo punto.

---

## 4. Matriz de capacidades reales (medida esta semana, no supuesta)

| Capacidad | Ollama 0.40.13 | LM Studio (1235) | ComfyUI 8188 | Wan2GP/Maestro |
|---|---|---|---|---|
| Ver modelos cargados | sí (`/api/ps`) | sí (`state: loaded`) | **no** (solo VRAM) | **no** |
| Descargar modelo | sí (`keep_alive:0`) | sí (`/api/v1/models/unload`) | sí, global (`/free`) | **no existe** |
| Contexto por petición | sí (`num_ctx`, recarga) | sí al cargar (`-c`) | n/a | n/a |
| Capas en GPU | sí (`num_gpu`) | ratio (`--gpu off/max/0..1`) | n/a | n/a |
| Flash attention | sí por petición (recarga) | **no** | n/a | n/a |
| KV cache dtype | solo por entorno (**aquí q4_0**) | **no** | n/a | n/a |
| Tensor/pipeline parallel | **no** | **no** | n/a | n/a |
| Speculative decoding | **no** | sí (`--speculative-draft-mtp`) | n/a | n/a |
| Paralelismo | entorno (`OLLAMA_NUM_PARALLEL`) | `--parallel` | cola propia | cola propia |
| RoPE/YaRN | **no** | **no** | n/a | n/a |
| Progreso de carga | solo al terminar (`load_duration`) | `--estimate-only` previo | websocket de progreso | logs |

Entorno real de Ollama en esta máquina: `OLLAMA_KV_CACHE_TYPE=q4_0`,
`OLLAMA_CONTEXT_LENGTH=32768`, modelos en `D:\ollama\models`.

Medidas de hoy: `qwen3-vl:4b` a 8K → 3,45 GB; descarga medida → **18,50 GB**
liberados (8,85 + 9,22 por GPU); `flint-local` (39,8B) → **64,5 tok/s**, prefill
118,9 tok/s, TTFT 94 ms, pico 11,7 + 14,3 GB de VRAM, 29,8 GB de RAM.

---

## 5. Auditoría módulo por módulo (1–20)

| # | Módulo | Estado | Qué hay exactamente | Qué falta |
|---|---|---|---|---|
| 1 | Advanced Model Control | **80 %** | `Models.tsx`, `ModelAdvanced.tsx`, `engines.rs`, `control.rs`: contexto con presets y máximo real, KV, GPU, motor, generación, perfiles por modelo | preset 256K (hoy tope 128K), micro-batch, afinidad de CPU (no expuesta por ningún motor) |
| 2 | KV Cache Manager | **45 %** | tabla de 6 formatos con coste y ahorro, estado real del servidor (q4_0), aviso del nativo | K/V separadas, FP8/INT8, ON/OFF, offload de KV: **no existen en las APIs**; medir “pérdida de rendimiento” es imposible porque no se puede cambiar |
| 3 | Multi-GPU Center | **40 %** | VRAM/RAM/temperatura/vatios por GPU, `num_gpu`, aviso de que Ollama reparte solo | selección de GPU (exige `CUDA_VISIBLE_DEVICES` del servidor), tensor split, TP/PP (no soportados), PCIe, reparto por modelo |
| 4 | VRAM Manager | **85 %** | liberar por GPU/motor/modelo, medida antes/después, aviso de competencia | detección de fragmentación (sin API), modelos duplicados e inactivos (factible) |
| 5 | Model Loading Monitor | **0 %** | nada | todo: fases disco→RAM→VRAM muestreadas, tipo de disco real, «% no disponible» cuando no lo haya |
| 6 | Intelligent Memory Engine | **50 %** | preflight, `fits`, matemática de KV, guardia | planificador de residencia múltiple con números medidos |
| 7 | Auto Pilot | **0 %** | nada | barrido controlado de configuraciones sobre `lab::benchmark`, con aprobación |
| 8 | GPU Diagnostics | **35 %** | stats en vivo, métricas de bench, historial JSON | PCIe, fragmentación, gráficas históricas, CPU por núcleo |
| 9 | Experimental AI Lab | **90 %** | 44 experimentos clasificados, banco real, A/B, historial | repeticiones con mediana (hoy una sola medida) |
| 10 | Model Intelligence | **50 %** | ficha con arquitectura, cuantización, contexto, capacidades | MoE y parámetros activos, predicción de memoria, comparador y recomendación |
| 11 | Intelligent Model Router | **0 %** | solo lista de proveedores/modelos | puntuación por tarea con datos medidos y cambio con consentimiento |
| 12 | Agent Swarm | **25 %** | `agents.rs` (registro + políticas), `tasks` (tabla vacía), vista **stub** | cola persistente, scheduler acotado, pool de workers, parada global, vista real |
| 13 | Memory Brain | **30 %** | sesiones, mensajes, `memories`, vista Memoria | embeddings locales (Ollama puede), búsqueda semántica, RAG de repos, dedupe, exportar/borrar |
| 14 | Multimodal Center | **55 %** | ComfyUI imagen, estudio de vídeo, convertidor de estilos | cola unificada con supervisión; audio (Fish Speech) y 3D no integrados |
| 15 | Automation & Tools | **35 %** | terminal, política de permisos, aprobaciones, auditoría | MCP, git, backups, runner de pruebas, permisos por agente en todo camino |
| 16 | Live Action / Reality Converter | **70 %** | registro de motores, catálogo de 13 estilos, detección con VLM, preflight | ejecución de difusiones (RC3/RC4) y montaje por planos |
| 17 | Security Center | **65 %** | política por agente, modo lectura, auditoría, aprobaciones | reglas explícitas para KUKIA Network (ledger, wallets), verificación de backups |
| 18 | Profile Manager | **60 %** | 6 perfiles de fábrica, guardar/duplicar/borrar/aplicar/restaurar | PROGRAMACIÓN, RAZONAMIENTO, MULTIMODAL, EXPERIMENTAL, importar/exportar |
| 19 | Benchmark Arena | **55 %** | `benchmark_run`, A/B del laboratorio, historial | comparación entre modelos, calidad, exportación |
| 20 | Discovery Engine | **40 %** | tres escáneres (arXiv/HF/GitHub) con alertas | unificación, clasificación por utilidad/madurez/compatibilidad |

**Resumen:** 4 módulos hechos o casi (1, 4, 9, 16), 9 parciales con base sólida,
7 sin empezar (5, 7, 11, y las partes nuevas de 3, 8, 12, 19).

---

## 6. Plan por fases (con dependencias reales)

### Fase 1 — Cimientos que faltan (1–2 semanas de trabajo)
- **SQ1. Cola y workers (módulo 12).** Tabla `jobs` + scheduler acotado + pool con
  límite de concurrencia + parada global + vista Tareas real. Es la base de
  Auto Pilot, Arena y descubrimiento: sin esto, todo lo demás son botones que bloquean.
- **SQ2. Perfiles completos (módulo 18).** Los 5 que faltan + importar/exportar JSON.
- **SQ3. Un solo motor de medición (módulos 8 y 19).** Unificar `benchmark_run` y
  `lab::benchmark`; añadir repeticiones con mediana y dispersión.

### Fase 2 — Memoria y carga (módulos 5, 6)
- **SQ4. Model Loading Monitor.** Muestreo de RSS del proceso del motor, VRAM por GPU
  y `load_duration` real; tipo de disco con `Get-PhysicalDisk`; «% no disponible»
  donde el motor no publique progreso (que es el caso de Ollama).
- **SQ5. Planificador de residencia.** Dado un conjunto de modelos, decir qué cabe,
  qué se queda en RAM y qué se rechaza, con los números medidos y margen.

### Fase 3 — Inteligencia (módulos 10, 11, 7)
- **SQ6. Model Intelligence:** MoE y parámetros activos desde `model_info`, predicción
  de memoria (pesos + KV con el formato activo) y comparador de modelos.
- **SQ7. Router por tarea:** puntuación con datos reales (bench + capacidades + VRAM),
  nunca cambio automático sin confirmación.
- **SQ8. Auto Pilot:** barrido sobre la cola con `bench` como juez; guarda el mejor
  perfil, no lo aplica solo.

### Fase 4 — Memoria del conocimiento (módulo 13)
- **SQ9. Embeddings locales** con Ollama (`nomic-embed-text` ya disponible o el que
  elijas), índice SQLite con similitud coseno, RAG de repos y documentos, dedupe.

### Fase 5 — Agentes y multimodal (módulos 14, 15)
- **SQ10. Swarm sobre la cola:** equipos, delegación, supervisor, reintentos.
- **SQ11. MCP + git + backups** con permisos por agente en todos los caminos.

### Fase 6 — Descubrimiento y validación (módulos 20, 3, 8, 17)
- **SQ12. Unificar los tres escáneres** y clasificar novedades.
- **SQ13. PCIe y gráficas históricas; reglas de KUKIA Network en el centro de seguridad.**
- **SQ14. Batería de integración** de todo lo anterior.

---

## 7. Riesgos

1. **Competencia de VRAM real**: hoy había dos modelos cargados a la vez (4B + 39,8B)
   usando 27,6 GB de 28,5. Cualquier carga nueva puede expulsar al que estés usando.
   Ya pasó en esta sesión al probar `flash_attn`.
2. **La cola de trabajos toca modelos**: si un worker carga modelos, choca con la
   Guardia. Regla propuesta: los workers **no cargan** modelos; usan el que ya esté y
   la Guardia decide.
3. **Tres escáneres hacen peticiones a la red**: unificarlos reduce tráfico y fallos.
4. **`run_command` sin política**: hoy un comando puede saltarse `permissions::check`.
   Es el agujero más serio del centro de seguridad.
5. **Auto Pilot mal entendido**: puede parecer que “optimiza solo”. Propuesta: mide,
   propone y guarda perfil; aplicar es siempre un clic tuyo.

---

## 8. Lo que necesito de ti para empezar la Fase 1

1. **Autorización para SQ1** (cola `jobs`, scheduler, pool, parada global, vista
   Tareas): modifica `memory.rs` (tabla nueva), añade `core/src/jobs.rs`, comandos y
   una vista. Backup verificado antes.
2. **Autorización para SQ2 y SQ3** (perfiles completos y motor de medición único):
   pequeños, sin riesgo, dependen de lo de arriba.
3. Confirmar si quieres que **unifique los escáneres** (SQ12) antes de tiempo: elimina
   duplicidad pero toca código que funciona.
4. Confirmar si puedo **cerrar el agujero de `run_command`** haciendo que todo comando
   pase por la política (cambio de seguridad, puede requerir aprobaciones nuevas).

Nada de esto se instala, descarga ni reinicia: todo es código del proyecto.
