# KUKIA CODE OMEGA — AUDITORÍA DE LOS 15 MÓDULOS

**Fecha:** 01/11/2026 · **Autor:** Xoxete
**Modo:** **solo lectura**. Ni una línea de código modificada, nada instalado, nada descargado.

Regla aplicada a rajatabla: **un prompt no es un módulo y un botón no es una función.** Si solo hay
interfaz o texto, va como «no implementado», aunque exista el archivo.

---

## 0. LO QUE HAY HOY, EN NÚMEROS

| Dato | Valor |
|---|---|
| Núcleo Rust | **6.271 líneas** en 14 módulos |
| Vistas React | **2.928 líneas** en 18 vistas |
| Comandos Tauri | 62 |
| Tests | 98 pasan · 12 ignorados |
| Motores reales conectados | Ollama (81 modelos), LM Studio, ComfyUI, cloud OpenAI-compatible |
| Almacén | SQLite (`%APPDATA%\KukiaCode\kukia.db`) |

**Módulos del núcleo reutilizables:** `providers` (711), `memory` (1.430), `permissions` (608),
`vram` (539), `gaming` (548), `anime_real` (729), `research` (486), `config` (338),
`security` (318), `agents` (181), `gpu` (146), `logs` (113), `radar` (101).

---

## 1. ESTADO DE LOS 15 MÓDULOS

| # | Módulo | Estado | Qué hay exactamente |
|---|---|---|---|
| 1 | Universal RAG Engine | **D — no existe** | `memory_search` es `LIKE '%texto%'` sobre una tabla. Cero embeddings, cero vectores, cero índice |
| 2 | Long Context Manager | **C — parcial y tosco** | `Chat.tsx` manda `messages.slice(-20)`: corta por **número de mensajes**, no por tokens. Sin resumen, sin compresión |
| 3 | Model Council | **D — no existe** | No hay `council` ni `consejo` en el código. Solo selector manual de un modelo |
| 4 | Adaptive Model Router | **D — no existe** | Cero apariciones de `router`. Hay selector manual y perfil GPU decorativo (35/65) |
| 5 | Knowledge Graph | **D — no existe** | Sin tablas de nodos/aristas. El «conocimiento» son memorias sueltas en SQLite |
| 6 | Autonomous Coding Engine | **C — piezas sueltas** | `files_write` con permisos y backup verificado, `run_command` con timeout y cancelación, `audit_log`. **Falta el motor**: analizar, planificar, parchear, probar, revertir |
| 7 | Multi-Agent Debugger | **C — fachada** | 8 agentes con prompt y una **lista de nombres de herramientas que no se ejecutan**. Sin tool calling, sin colaboración |
| 8 | Code Intelligence Engine | **D — no existe** | Sin AST, sin tree-sitter, sin análisis de complejidad. Ni siquiera hay detección de duplicados |
| 9 | Automatic Testing Engine | **C — ejecuta, no genera** | Los tests existen y se ejecutan a mano (`cargo test`, `npm run build`). No hay generación, ni cobertura, ni informes antes/después |
| 10 | Git Intelligence | **D — no existe** | **Ninguna llamada a git** en el código (la única aparición es un test de comandos destructivos). Los commits los hago yo a mano |
| 11 | Smart Model Cache | **D — no existe** | No hay `keep_alive`, ni descarga de modelos, ni política de carga. Ollama decide solo |
| 12 | Prompt Cache Engine | **D — no existe** | Ni prefijos, ni reutilización, ni medición de ahorro |
| 13 | Multimodal Studio | **C — una de seis** | ComfyUI sí (Anime ↔ Real, con imagen real). Wan, LTX, RVC, TTS, STT: **nada** |
| 14 | Workflow Engine | **D — no existe** | `workflows/` son JSON de ComfyUI, no un motor de flujos. Sin DAG, sin pasos, sin reintentos |
| 15 | Distributed AI Engine | **D — solo diseño** | `INFORMES/ARQUITECTURA_MULTI_PC.md`. Sin red, sin auth, sin cola |

**Recuento: 0 módulos implementados · 5 parciales (2, 6, 7, 9, 13) · 10 inexistentes.**

---

## 2. LO REUTILIZABLE (no hay que escribirlo de cero)

| Pieza existente | Sirve para |
|---|---|
| `permissions` + `policy_*` + `audit_log` | Base de seguridad de **cualquier** módulo que ejecute algo (6, 7, 9, 14) |
| `files_write` con backup verificado | Escritura segura del motor de código (6) |
| `run_command` con timeout y cancelación | Ejecutar tests, git, compilaciones (9, 10) |
| `memory` (SQLite, sesiones, mensajes, memorias) | Almacén de RAG, grafo, workflows e historial (1, 5, 12, 14) |
| `providers` (chat + streaming + catálogo) | Base del consejo, del router y del motor de agentes (3, 4, 7) |
| `vram` (VRAM real, competencia, `fits`) | Smart Model Cache y decisiones del router (11, 4) |
| `anime_real` (conector ComfyUI, plantillas, historial) | Base del Multimodal Studio (13) |
| `Benchmark` + datos reales | Router que aprende de mediciones (4) |

---

## 3. TECNOLOGÍA RECOMENDADA (Windows 11 nativo, local, sin nube)

| Módulo | Tecnología | Por qué esta |
|---|---|---|
| RAG (1) | **Embeddings con Ollama** (`nomic-embed-text` o `bge-m3`) + **sqlite-vec** sobre la base existente | Todo nativo, sin servidores, sin Docker. Alternativa: coseno en Rust sobre BLOBs `f32` (más simple, suficiente <100k fragmentos) |
| Long Context (2) | Resumen jerárquico con el **modelo local** + conteo de tokens por `tiktoken`-equivalente | Sin dependencias raras; el corte por tokens sustituye al `slice(-20)` |
| Council (3) | Consultas **secuenciales** (no paralelas) con registro de evidencia | Con 12+16 GB no caben tres modelos a la vez: paralelo = thrashing de VRAM |
| Router (4) | Reglas + tabla de benchmarks reales | Empezar con reglas explicables; aprender solo cuando haya mediciones |
| Knowledge Graph (5) | **Tablas SQLite** `nodes` / `edges` + vista de navegación | Un grafo de 10⁴ nodos no necesita Neo4j |
| Autocoding (6) | Diff/patch + `cargo test`/`npm run build` + parada antes de aplicar | Aprovecha permisos y backups ya hechos |
| Debugger (7) | Requiere **tool calling** (no existe hoy) | Sin herramientas reales, 8 agentes son 8 charlas |
| Code Intelligence (8) | **tree-sitter** (crates Rust) o heurísticas + `clippy`/`tsc` | AST real solo donde importa; `clippy` ya da vulnerabilidades y código muerto gratis |
| Testing (9) | Orquestar `cargo test`, `npm`, y **cargo-llvm-cov** (soportado en Windows MSVC) | `tarpaulin` no funciona bien en Windows |
| Git (10) | CLI `git` desde `run_command` (ya existe) o `git2` | Empezar **solo lectura** (`status`, `log`, `diff`); push y ramas, con aprobación |
| Model Cache (11) | `keep_alive` de Ollama + carga/descarga de LM Studio | Control real de VRAM, medido |
| Prompt Cache (12) | Aprovechar caché interna de cada backend y **medir con TTFT** | No hay API para forzarla; se mide, no se inventa |
| Multimodal (13) | Conectores HTTP a ComfyUI (hecho), Wan2GP, TTS/STT locales | Cada uno aislado, como `anime_real` |
| Workflow (14) | DAG propio en SQLite + ejecutor con timeout y auditoría | Reutiliza `run_command` y permisos |
| Distributed (15) | HTTP + TLS con token por nodo y cola persistente | Sin sumar VRAM; trabajos independientes |

---

## 4. COSTE DE RECURSOS (estimado, no medido)

| Módulo | VRAM | RAM | Disco |
|---|---|---|---|
| RAG | 0–0,6 GB (embedding pequeño) o CPU | 0,5–2 GB con índice de 100k fragmentos | 1–2 GB por 100k fragmentos |
| Long Context | 0 (usa el modelo ya cargado) | Bajo | Bajo |
| Council | **Sin VRAM extra** si es secuencial | Bajo | Bajo |
| Router | 0 | 0 | 0 |
| Knowledge Graph | 0 | <200 MB | <500 MB |
| Autocoding / Tests | Usa el modelo del chat | Picos de compilación (Rust ~2 GB) | `target/` ya ocupa lo suyo |
| Model Cache | **Libera** VRAM | — | — |
| Multimodal | Wan/LTX necesitan 8–16 GB **exclusivos** | Alto | Decenas de GB |
| Workflow | 0 | Bajo | Bajo |
| Distributed | 0 local | Bajo | Cola pequeña |

---

## 5. RIESGOS

| Riesgo | Gravedad | A quién afecta |
|---|---|---|
| **Tool calling inexistente** deja sin sentido a 7 (debugger) y 6 (autocoding) | **Alta** | Núcleo: `providers.rs` |
| **VRAM compartida**: council paralelo o multimodal + chat = colapso (ya medido 83 → 0,13 tok/s) | **Alta** | Todos |
| Generar tests o parches con un modelo de 30B en 12 GB produce basura plausible | Alta | 6, 9 |
| Grafo y RAG mal indexados dan respuestas seguras y falsas | Media | 1, 5 |
| Git automático puede tocar ramas y commits | Media | 10 |
| Workflows con reintentos pueden crear bucles de ejecución | Media | 14 |
| Red entre PCs sin auth = ejecución remota de comandos | **Crítica** | 15 |
| 15 módulos a la vez = 15 frentes a medias | Alta (proyecto) | Todos |

---

## 6. DEPENDENCIAS ENTRE MÓDULOS

```
Tool calling ──► 7 (debugger) ──► 6 (autocoding con validación)
       │
       └────────► 14 (workflow con herramientas)

Mediciones (F-P2) ──► 4 (router) ──► 11 (model cache)

Embeddings (1) ──► 12 (prompt cache mide ahorro)
       └────────► 5 (grafo enlaza fragmentos)

Permisos + audit (HECHO) ──► 6, 7, 9, 10, 14
ComfyUI (HECHO) ──► 13
Auth de red (0) ──► 15
```

---

## 7. ORDEN DE IMPLEMENTACIÓN PROPUESTO

### Ola A — fundamentos medibles (bajo riesgo, alto retorno)

| Orden | Módulo | Sesiones | Por qué primero |
|---|---|---:|---|
| A1 | **12 Prompt Cache** (medir TTFT y ahorro) | 1 | No añade nada, mide lo que ya pasa |
| A2 | **11 Smart Model Cache** (`keep_alive`, liberar VRAM) | 1 | Ataque directo al problema real: la VRAM |
| A3 | **2 Long Context** (corte por tokens + resumen) | 1–2 | Sustituye un `slice(-20)` ciego |
| A4 | **4 Router v1** (reglas explicables) | 1 | Necesita A1/A2 para tener datos |

### Ola B — conocimiento (medio riesgo)

| Orden | Módulo | Sesiones |
|---|---|---:|
| B1 | **1 Universal RAG** (embeddings + sqlite-vec + citas) | 2–3 |
| B2 | **8 Code Intelligence** (`clippy`/`tsc` + tree-sitter mínimo) | 2–3 |
| B3 | **5 Knowledge Graph** (nodos/aristas en SQLite) | 2 |

### Ola C — autonomía (riesgo alto, requiere tool calling y aprobación)

| Orden | Módulo | Sesiones |
|---|---|---:|
| C0 | **Tool calling** en `providers.rs` (prerrequisito, no está en la lista pero sin él nada) | 2 |
| C1 | **10 Git Intelligence** (solo lectura primero) | 1–2 |
| C2 | **9 Automatic Testing** (orquestar + cobertura) | 1–2 |
| C3 | **6 Autocoding** con parada obligatoria antes de aplicar | 3–4 |
| C4 | **7 Multi-Agent Debugger** | 3–4 |
| C5 | **14 Workflow Engine** | 3–4 |
| C6 | **3 Model Council** (secuencial) | 1–2 |
| C7 | **13 Multimodal** (Wan, LTX, TTS, STT por conectores) | 4–6 |
| C8 | **15 Distributed** (auth primero, luego tareas) | 5+ |

**Total estimado: 33–45 sesiones.** Con lo que hay hoy, hacerlo en una sola tanda sería mentira.

---

## 8. BENCHMARKS PROPUESTOS

| Módulo | Qué medir | Con qué |
|---|---|---|
| 12 Prompt cache | TTFT con y sin prefijo repetido, tokens ahorrados | Protocolo de F-P2 (aún no implementado) |
| 11 Model cache | tok/s en 1ª vs 2ª generación, VRAM liberada al descargar | `nvidia-smi` + `ollama ps` |
| 2 Long context | Calidad de respuesta con 8k/16k/32k y coste en tokens | Prompt fijo + 3 repeticiones |
| 4 Router | Aciertos de elección frente a elección humana | Tabla de decisiones |
| 1 RAG | Precisión@5 en 50 preguntas con respuesta conocida | Corpus propio (`I:\CEREBRO` en solo lectura) |
| 8 Code intel | Verdaderos positivos de código muerto/duplicado | Comparación con `clippy` |
| 9 Tests | Cobertura antes/después | `cargo-llvm-cov` |

---

## 9. LO QUE NO VOY A HACER AUNQUE LO PIDAS EN CALIENTE

- **«Sumar VRAM de dos equipos»**: ya está bloqueado y medido; el cuello es la red.
- **Council en paralelo** con modelos grandes: con 28 GB totales y dos tarjetas de distinto tamaño,
  es thrashing garantizado.
- **Autocoding sin tool calling**: escribiría parches sin poder compilarlos ni probarlos.
- **Distributed sin autenticación**: sería un RCE con interfaz bonita.
- **Declarar 15 módulos «hechos»** porque exista un botón: el encargo lo prohíbe y mi trabajo es
  que la app sirva, no que la lista esté llena.

---

## 10. NECESITO DE TI PARA EMPEZAR

1. ¿Autorizas la **Ola A** (12 → 11 → 2 → 4)? Es la de menor riesgo y la que ataca tus problemas
   reales: VRAM y contexto.
2. ¿Aceptas que **tool calling (C0)** sea el siguiente paso obligado antes de 6, 7 y 14?
3. ¿El RAG indexa también `I:\CEREBRO` (en **solo lectura**, sin copiar) o solo `H:\KUKIA_CODE`?
4. ¿Confirmas que no quieres todavía nada de **Distributed** hasta tener un equipo estable?

**Nada modificado. Nada instalado. Nada declarado implementado que no lo esté.**
