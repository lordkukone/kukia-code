# KUKIA CODE — ROADMAP RECOMENDADO

**Fecha:** 01/11/2026 · **Autor:** Xoxete
**Base:** auditoría de solo lectura (`INFORME_AUDITORIA_FREEBUFF.md`) y hallazgos
(`ERRORES_Y_RIESGOS.md`).

---

## 0. PRINCIPIO

El superprompt propone ocho fases (3A–3H). El orden que propongo **cambia uno solo**, pero es
importante: **cerrar y conectar** lo que ya está escrito va antes que cualquier funcionalidad nueva.
Tenemos 1.188 líneas de Fase 3 que el usuario no puede tocar y dos agujeros de seguridad abiertos; eso
primero.

**Regla de oro de aquí en adelante:** una funcionalidad está terminada cuando tiene **comando Tauri +
vista + test + evidencia pegada** (`FUNCIONALIDADES_PENDIENTES.md` §6). Mientras falte una, se queda
en PARCIAL.

---

## FASE 3.0 — CIERRE Y CONEXIÓN (prioridad máxima, 1 sesión)

**Objetivo:** que lo ya escrito funcione y que la app no se pueda colgar ni escribir donde no debe.

| # | Trabajo | Criterio de aceptación medible |
|---|---|---|
| 1 | **E-01** aplicar `permissions::check` en `files_read`, `files_write`, `run_command` | Test de integración: escritura en `C:\Windows\x` → error; escritura en `H:\proyecto\x` con política → OK |
| 2 | **E-03** timeout + tope de salida en `run_powershell` | Test con comando de 60 s y timeout 2 s → devuelve error «cancelado por tiempo» en ≤3 s |
| 3 | **E-02** `audit_log` en cada comando sensible | Una escritura deja **1 fila** en `audit`; `audit_search` la encuentra |
| 4 | **E-07** helper de mutex sin pánico | Un test provoca pánico en un hilo y la siguiente consulta responde error, no pánico |
| 5 | Comandos `policy_get/policy_set/policies_list` + **vista Permisos** | Se ve la política de los 8 agentes y se guarda un cambio que persiste tras reiniciar |
| 6 | Comando `vram_report` + pestaña **VRAM Guard** | Con dos motores cargados, la vista muestra el aviso de competencia (reproducible) |
| 7 | Quitar warning `gaming.rs:153` y `E-06` tests en temporal | `cargo check` y `cargo test` **sin warnings**, log real sin líneas `[test]` |

**Por qué primero:** sin 1–4, cualquier fase posterior construye encima de una base que escribe sin
permiso y sin registro.

---

## FASE 3A — PERMISOS COMPLETOS (1–2 sesiones)

Depende de 3.0.

- **Permisos por proyecto:** hoy no existe el concepto. Añadir tabla `projects(id, name, root)` y que
  `write_paths` pueda referirse a un proyecto en vez de a una ruta literal.
- **Permisos que el agente no puede cambiar:** `policy_set` debe exigir aprobación del usuario y
  rechazar que un agente modifique su propia política (campo `origin` en la auditoría).
- **E-04/E-05:** `canonicalize` contra junction + lista de destructivos por verbo.
- **Vista Auditoría:** filtros por fecha/acción/resultado + exportación CSV (`audit_export_csv` ya
  está escrito).

**Criterio:** intentar saltarse la allowlist con `..`, junction y `rd /s /q` → los tres bloqueados y
registrados.

---

## FASE 3B — VRAM GUARD + MODEL ROUTER (2 sesiones)

- **VRAM Guard ya está escrito**: conectar y añadir histórico (el `Gpu.tsx` actual solo pinta
  instantáneas). Guardar en `benchmarks` la VRAM libre junto a cada tok/s.
- **Model Router** (nuevo, 1 sesión): puntuar cada modelo con datos **medidos**, no supuestos:
  `vram_mb` del GGUF, contexto nativo, tok/s del último benchmark, si cabe en GPU1. Perfiles:
  `máxima inteligencia`, `programación`, `velocidad`, `bajo consumo`, `razonamiento`.
- **Regla que ya está demostrada:** un modelo por GPU, y nunca dos motores con modelos cargados.
  El router debe **avisar antes de cargar** si no cabe (`vram::fits` ya lo calcula con 12 % de margen).

**Criterio:** con GPU1 ocupada por LM Studio, pedir un modelo de 20 GB en Ollama → aviso previo con
los MB que faltan, no un fallo a los 3 minutos.

---

## FASE 3C — RADARES (1–2 sesiones)

- Conectar `gaming.rs` (ya escrito): comando `gaming_scan`, vista **Gaming Radar**, informe semanal en
  `H:\KUKIA_RESEARCH\GAMING_GRAPHICS\reports\`.
- **E-14:** cachear por `ETag`, respetar cuota y avisar cuando GitHub diga `403`.
- Ampliar el World AI Radar con **Hugging Face `/api/models?sort=trending`** ya usado, más `papers
  with code` vía arXiv (misma API). Nada de scraping.

**Criterio:** un barrido completo consume **< 20 peticiones** y la segunda ejecución del día no hace
ninguna (caché).

---

## FASE 3D — TERMINAL REAL Y CANCELACIÓN (1–2 sesiones)

Antes que Desktop Control, porque lo necesita.

- `portable-pty` + `xterm.js` en la vista Terminal (hoy es PowerShell bloqueante).
- **Cola de trabajos** con estado en la tabla `tasks` (ya existe), progreso, **cancelar** y
  recuperación al arrancar (`tasks_interrupted` ya existe).
- Anti-duplicados: clave `(kind, target)` activa → rechazar el segundo lanzamiento.

**Criterio:** lanzar `npm run build`, ver progreso en vivo, cancelar, cerrar la app, reabrir →
la tarea aparece como «interrumpida» con su checkpoint.

---

## FASE 3E — AGENTES CON HERRAMIENTAS (2–3 sesiones)

- Que las `tools` de cada agente **se ejecuten de verdad**, pasando por `permissions::check`.
- `Agent Swarm` solo después: requiere FASE 3D (cola) y 3A (permisos) para que dos agentes no se
  pisen. Bloqueo por ruta antes de escritura; un solo escritor por archivo.
- Comparación con OpenCode **con la misma tarea y el mismo modelo**, y publicar los números. Sin
  evidencia, no se declara superioridad.

**Criterio:** dos agentes editan ficheros distintos en paralelo sin corromper nada; el intento sobre
el mismo fichero se serializa y se registra.

---

## FASE 3F — MULTI-PC (3–4 sesiones, cuando llegue el i7-12700)

Ver `ARQUITECTURA_MULTI_PC.md`. Resumen:

1. Inventario **de solo lectura** del worker.
2. Nodo worker con agente HTTP + autenticación por token, solo LAN, sin puertos públicos.
3. **Cola de trabajos** ya existente de 3D, con `worker_id`.
4. **Medir** con una tarea real (TTS o transcripción) antes de prometer nada.

**No prometer:** suma de VRAM, tensor parallel entre equipos, o que repartir acelera. Medido aquí: ni
siquiera entre las dos GPUs del mismo equipo el reparto ayuda.

---

## FASE 3G — MULTIMODAL (3+ sesiones)

ComfyUI (8188/8189) y audio ya funcionan en el equipo; lo que falta es el puente desde KUKIA CODE:

- Cola de trabajos → plantilla de workflow → `POST /prompt` → recoger imagen/vídeo.
- TTS con Fish Speech (puerto 8080, ojo: ya ocupado).
- **Nunca** cargar un modelo de ComfyUI y uno de inferencia de texto a la vez en la misma GPU (regla
  medida: 0,13 tok/s).

---

## FASE 3H — BENCHMARKS Y OPTIMIZACIÓN (1–2 sesiones)

- Benchmark comparativo: mismo prompt, misma temperatura, misma VRAM de partida, N repeticiones,
  mediana y desviación. Guardar también VRAM/RAM/temperatura.
- **Speculative decoding**: única vía pendiente con base técnica para subir tok/s sin perder calidad.
  Medir antes/después o no publicarlo.

---

## FASE 3I — DESKTOP CONTROL (evaluar antes de construir)

Es la funcionalidad más cara de mantener de toda la lista. Antes de escribir una línea:

1. Elegir **3 aplicaciones concretas** que de verdad necesites automatizar (p. ej. Ollama, LM Studio,
   ComfyUI).
2. Comprobar que exponen **API** — las tres lo hacen, y por API es más estable que por UI Automation.
3. Solo si alguna no tiene API, añadir `uiautomation`/`windows` en Rust para esa.

**No se declarará «control universal»**: se declarará «estas N aplicaciones, probadas en esta
versión de Windows».

---

## FASE 3J — RELEASE

- `PROGRESO_GENERAL.md`, `CONTINUAR_TRABAJO.md`, `FASES_COMPLETADAS.md` (no existen).
- Decidir `perMachine` vs `currentUser` (E-15) y firma de código (E-16).
- Versionar: la próxima build **no** es 0.1.0.
- `CARGO_TARGET_DIR` fuera de H: o `cargo clean` antes de compilar (E-09).

---

## ORDEN RECOMENDADO, EN UNA TABLA

| Orden | Fase | Sesiones | Depende de |
|---|---|---:|---|
| 1 | **3.0 Cierre y conexión** | 1 | — |
| 2 | 3A Permisos completos | 1–2 | 3.0 |
| 3 | 3B VRAM Guard + Router | 2 | 3.0 |
| 4 | 3D Terminal real y cola | 1–2 | 3.0 |
| 5 | 3C Radares | 1–2 | 3.0 |
| 6 | 3E Agentes con tools | 2–3 | 3A, 3D |
| 7 | 3H Benchmarks | 1–2 | 3B |
| 8 | 3F Multi-PC | 3–4 | 3D + el i7-12700 |
| 9 | 3G Multimodal | 3+ | 3D |
| 10 | 3I Desktop Control | a decidir | 3D |
| 11 | 3J Release | 1 | todas |

**Total realista hasta release funcional (sin 3I): 15–22 sesiones de trabajo.** Cualquier promesa
menor es humo.

---

## LO QUE NO HARÍA, Y POR QUÉ

1. **Agent Swarm primero.** Varios procesos ejecutando comandos sin timeout ni allowlist = la forma
   más rápida de romper el equipo.
2. **Distributed AI antes de Fase 3.** Repartir entre dos PCs algo que aún no está terminado en uno
   multiplica los fallos.
3. **Reescribir el núcleo.** Está bien: sin `panic!`, sin `todo!`, con denegación por defecto y 47
   tests. Lo que le falta es conexión, no reescritura.
4. **Añadir dependencias grandes** (`rmcp`, `portable-pty`, `uiautomation`) todas en la misma sesión:
   cada una trae su propio peso de compilación y sus propios fallos en Windows.
5. **Declarar nada PASS sin la salida del comando pegada.** Es la norma que ha salvado este proyecto
   cuatro veces.
