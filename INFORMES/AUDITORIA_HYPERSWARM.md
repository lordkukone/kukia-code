# KUKIA CODE — HYPERSWARM: AUDITORÍA Y PLAN (solo lectura)

**Fecha:** 01/11/2026 · **Autor:** Xoxete
**Modo:** solo lectura. **Nada modificado, nada instalado, nada ejecutado.**

---

## 0. VEREDICTO EN UNA LÍNEA

Un **registro** de millones de agentes lógicos cabe y es barato. Un **millón de agentes
ejecutando** en este hardware es aritmética imposible: el techo no es el software, es la
inferencia. Hoy, además, no hay ni cola, ni herramientas, ni cancelación de chat: el
Hyperswarm no está al 10 %, está en el papel.

---

## 1. QUÉ EXISTE HOY (con evidencia)

| Pieza | Estado real |
|---|---|
| `core/src/agents.rs` (181 líneas) | **8 agentes estáticos**: `id`, nombre, descripción, `system_prompt`, `tools: Vec<String>`, color. `create` escribe en `%APPDATA%\KukiaCode\config\agents.json` |
| Herramientas de los agentes | **Nombres en una lista** (`"files.read"`, `"terminal.run"`…). Ningún código las ejecuta |
| `agent_policies` | 9 filas reales: `allow_read/write/shell/network/download/tools`, `read_paths`, `write_paths`, `read_only_paths` |
| `tasks` | Tabla creada con `id, title, agent, kind, status, progress, checkpoint, error, created_at, updated_at`… **0 filas y 0 comandos Tauri**. Nadie la usa |
| `audit` | Tabla con `agent, tool, action, targets, approved, result, error`… conectada (Fase 3.0), **0 filas** porque nadie ejecuta |
| `approvals` | Tabla creada, 0 filas, sin comandos |
| Cola / scheduler | **No existe** |
| Bus de comunicación | **No existe** |
| Cancelación de una generación | **No existe**: `command_cancel` solo cancela `run_command`, no `chat_stream` |
| Verificación independiente | No existe |
| Git orchestrator | No existe (0 llamadas a git en el código) |
| Multi-PC | Solo diseño en `INFORMES/ARQUITECTURA_MULTI_PC.md` |
| Panel | Vistas estáticas; `Tasks.tsx` tiene **22 líneas** y no habla con nada |

En resumen: hay **identidades** y hay **permisos**, pero no hay **ejecución**.

---

## 2. CAPACIDAD REAL DE ESTA MÁQUINA (lo que decide todo)

Datos medidos hoy, no supuestos:

| Dato | Valor | Fuente |
|---|---|---|
| CPU | Intel i9-10900K · **10 núcleos / 20 hilos** | WMI |
| RAM visible | **63,9 GB** | WMI |
| GPU | RTX 5070 12.227 MiB + RTX 5060 Ti 16.311 MiB | `nvidia-smi` |
| Ollama `NUM_PARALLEL` | **sin definir → 1 petición a la vez** | variables de usuario |
| tok/s medidos | **61,73** (9B en Ollama) · **6,84** (30B en LM Studio) | tabla `benchmarks` |
| VRAM libre ahora | 0,1 GB (5070) · 1,9 GB (5060 Ti) | VRAM Guard |

### La cuenta que hay que hacer antes de prometer nada

Con **2 GPUs** y `OLLAMA_NUM_PARALLEL=1`, el techo optimista es **2 generaciones simultáneas**
(una por tarjeta), sin contar la VRAM que ya ocupa ComfyUI o LM Studio.

| Escenario | Tokens/s agregados | 1 tarea = 400 tokens | 1.000.000 de tareas |
|---|---:|---|---|
| 2 × 9B a 60 tok/s (mejor caso medido) | ~120 | ~3,3 s/tarea en paralelo | **~926 horas** (38,6 días sin parar) |
| 2 × 30B a 6,8 tok/s (realista en 12 GB) | ~13,7 | ~29 s/tarea | **~8.100 horas** (338 días) |
| Un solo modelo cargado (hoy, con ComfyUI residente) | ~60 | ~6,7 s/tarea | ~1.852 horas |

**Conclusión ineludible:** «un millón de agentes» solo tiene sentido como **registro de
identidades y tareas**, no como ejecución simultánea. Un millón de tareas con este hardware es
**de un mes a un año de cálculo continuo** según el modelo.

### Lo que sí escala de verdad (y es barato)

| Recurso | Capacidad razonable | Comentario |
|---|---|---|
| Identidades en registro | **millones** | 1 M de filas × ~500 B ≈ **500 MB** en SQLite; H: tiene 98 GB libres |
| Tareas encoladas | **millones** | Cola en SQLite con WAL; escritura por lotes |
| Tareas **no-LLM** (ficheros, git, parseo, HTTP) | **miles por minuto** | Limitadas por CPU/I/O, no por GPU |
| Generaciones LLM concurrentes | **2–4** como máximo realista | Cada una necesita su modelo en VRAM |
| Profundidad de jerarquía útil | **3–4 niveles** | Cada nivel extra es un salto de LLM: multiplica latencia y tokens |

---

## 3. CUELLOS DE BOTELLA (por orden de dureza)

1. **Inferencia (el muro real).** 1 petición en paralelo por motor. Subir `NUM_PARALLEL` no
   regala velocidad: reparte la misma VRAM y multiplica el KV cache.
2. **Tool calling inexistente.** Sin él, los agentes no pueden *hacer* nada: solo conversar.
3. **Sin cola ni scheduler.** El `tasks` vacío es la prueba.
4. **Sin cancelación de chat.** Un trabajo colgado no se puede parar salvo cerrando la app.
5. **VRAM ya ocupada.** Con ComfyUI residente y LM Studio cargado, no queda sitio para un tercer
   modelo: el propio VRAM Guard lo detectó (competencia a las 15:11).
6. **Jerarquía cara.** Con 8 niveles, una orden simple pasa por 8 llamadas: 8× latencia, 8× tokens.
7. **Sin bus ni deduplicación.** Miles de agentes pidiendo lo mismo = tokens tirados.

---

## 4. ARQUITECTURA QUE PROPONGO (adaptada a lo que hay)

Nada de procesos por agente. **Registro + cola + pool acotado.**

```
Usuario
  └─ Supreme Commander (1 agente, planifica y reparte)
       ├─ Branch Commanders (los que hagan falta, no 27 fijos)
       │    └─ Specialists (definiciones reutilizables, sin estado propio)
       │         └─ Workers (pool acotado: ejecuta llamadas LLM y herramientas)
       ├─ Verification Swarm (revisa con evidencia; no puede ser el ejecutor)
       └─ Bus de eventos (SQLite WAL + notificaciones)
```

| Componente | Implementación propuesta | Reutiliza |
|---|---|---|
| Registro de agentes | Tabla `swarm_agents` (id, rol, rama, modelo, política, estado, métricas) | `agent_policies` (ya existe) |
| Cola de tareas | Tabla `swarm_tasks` con `status`, `depends_on`, `attempts`, `deadline`, `dedupe_key` | `tasks` (ya tiene el esquema base) |
| Scheduler | Rust: bucle que toma tareas listas, respeta `max_parallel` y VRAM | `vram::fits` (ya existe) |
| Worker pool | N workers acotados, cada uno una llamada a `providers::chat_stream` | `providers` (ya existe) |
| Cancelación | `oneshot` por tarea en el chat, no solo en comandos | patrón ya usado en `run_command` |
| Bus | Notificaciones Tauri + tabla `swarm_events` | `Emitter` (ya existe) |
| Verificación | Agente distinto del ejecutor, con evidencia obligatoria (salida de test, hash, log) | `audit` (ya existe) |
| Git orchestrator | Ramas por trabajo, `git diff` antes de integrar, sin tocar `main` | `run_command` |
| Panel | Números **reales**: agentes registrados vs activos vs ejecutando, cola, tok/s, VRAM, parada global | vistas actuales |
| Multi-PC | HTTP+TLS con token por nodo; trabajo remoto solo con allowlist | `permissions` |

**Regla de honestidad del panel:** un agente «registrado» no es un agente «activo», y uno
«activo» no es uno «ejecutando». Tres contadores distintos, siempre visibles.

---

## 5. ESCALADO POR FASES (con criterio de aceptación medible)

| Fase | Alcance | Criterio para darla por hecha | Sesiones |
|---|---|---|---:|
| **A** | Registro + cola + scheduler + pool acotado + panel con números reales + parada global | 100 tareas encoladas, ejecutadas de una en una, sin fugas; `tasks/min` medido a `max_parallel` 1 y 2 | 3–4 |
| **B** | Miles de agentes lógicos + deduplicación + verificación independiente + presupuesto de tokens por tarea | 1.000 tareas con 5 % de duplicados detectados y 0 bucles; coste en tokens registrado y acotado | 4–6 |
| **C** | Segundo PC: auth por token, cola compartida, recuperación al desconectar | Un trabajo enviado al nodo remoto termina; al matar el nodo, la tarea vuelve a la cola | 4–6 |
| **D** | Varios nodos, reparto por capacidad, métricas por nodo | 3 nodos simulados en local; reparto verificable | 8+ |
| **E** | Millones de identidades **registradas** | 1 M de filas insertadas y consultables en < 1 s; ejecución sigue limitada por hardware | 2–3 (solo registro) |

**Lo que no planifico con honestidad:** «millones de agentes ejecutando a la vez». No es una
cuestión de programar más: es física de memoria y ancho de banda.

---

## 6. RIESGOS

| Riesgo | Gravedad | Mitigación |
|---|---|---|
| Gasto de tokens sin techo (agentes hablando entre sí) | **Crítica** | Presupuesto por tarea y por rama; corte duro; contador visible |
| RCE por nodo remoto mal autenticado | **Crítica** | Token por nodo + allowlist + no exponer `run_command` remoto en Fase C |
| Bucles de delegación (A pide a B que pide a A) | Alta | Límite de profundidad, `dedupe_key`, detección de ciclos |
| VRAM saturada por N modelos «paralelos» | Alta | `max_parallel` derivado de VRAM real, no de lo que quepa en la cabeza |
| Agentes «activos» que en realidad no hacen nada (fachada) | Alta (credibilidad) | Distinguir registrado / activo / ejecutando en el panel |
| Escrituras concurrentes en SQLite | Media | WAL + una sola conexión escritora |
| Jerarquía de 8 niveles para todo | Media | 3–4 niveles por defecto; los demás, solo cuando la tarea lo pida |
| Tocar KUKIA Network por un agente curioso | Media | Rutas protegidas en política + confirmación humana |

---

## 7. BENCHMARKS PROPUESTOS

| Qué | Cómo se mide |
|---|---|
| Rendimiento de la cola | tareas/min con `max_parallel` = 1, 2 y 4, mismo prompt |
| Coste por tarea | tokens in/out por agente y por rama (tabla `usage` que aún no existe) |
| Ahorro del bus | nº de mensajes vs nº de tareas (objetivo: 1 mensaje por transición, no más) |
| Calidad de verificación | % de tareas donde el verificador encontró un fallo real |
| Latencia de una orden | tiempo desde la orden hasta el informe final, con jerarquía de 2 y de 4 niveles |
| Escala del registro | inserción de 1 M de agentes, tiempo de consulta por rama |

---

## 8. LO QUE NO HARÍA AHORA (y por qué)

1. **Abrir el panel Hyperswarm con 27 ramas y 8 niveles**: sería una maqueta. Tu propio encargo
   prohíbe presentar perfiles como agentes funcionales.
2. **Añadir `OLLAMA_NUM_PARALLEL=4` para «ir más rápido»**: reparte la misma VRAM y multiplica KV
   cache; sin medir, es empeorar.
3. **Fase C (segundo PC) antes de la Fase A**: sin cola estable, distribuir no arregla nada.
4. **Verificación por el mismo agente que ejecuta**: es teatro, no control de calidad.
5. **Prometer millones de ejecuciones**: ya está la cuenta arriba.

---

## 9. NECESITO DE TI PARA EMPEZAR

1. ¿Autorizas la **Fase A** (registro + cola + scheduler + pool acotado + panel real + parada
   global)? Es la única que se puede hacer sin tool calling.
2. ¿Cuántas **ramas** quieres de verdad en la primera versión? Propongo empezar con 5
   (código, pruebas, investigación, hardware/GPU, documentación) en vez de 27 vacías.
3. ¿Presupuesto máximo de tokens por orden? Sin eso, un swarm es una cuenta bancaria abierta.
4. ¿Confirmas que **no** habrá ejecución remota de comandos en Fase C hasta tener auth y
   allowlist probadas?

**Nada modificado. Nada instalado. Nada ejecutado. Nada declarado implementado que no lo esté.**
