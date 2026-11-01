# KUKIA CODE OMEGA — AUDITORÍA DE RENDIMIENTO E INTELIGENCIA

**Fecha:** 01/11/2026 · **Autor:** Xoxete
**Modo:** **solo lectura**. No se ha modificado código, ni instalado, ni descargado nada.

---

## 0. RESUMEN EN UNA TARJETA

KUKIA CODE hoy es un buen **panel de observación** y un mal **panel de control**: mide GPUs, motores y
VRAM de verdad, pero de todos los parámetros de inferencia **solo envía `temperature`**. Todo lo que
pediría «Advanced Memory Engine» e «Inference Acceleration» exige, primero, **plomería de parámetros**.

Tres hallazgos que resumen el estado:

1. **`gpu_split = [35, 65]` es decorativo.** Se guarda y se dibuja en Configuración; no se aplica a
   ningún motor (`grep` en `core/`, `src-tauri/` y `app/`: solo aparece en config, tipos y la vista).
2. **Las capacidades de los modelos se adivinan por el nombre.** `providers.rs` marca «visión» si el
   nombre contiene `vl` o `llava`, y «código» si contiene `coder`, y deja `context: None`.
   Ollama publica las capacidades reales: **81 modelos → 58 con `tools`, 13 con `vision`,
   44 con `thinking`**, además de `context_length` y `parameter_count` vía `/api/show`. No se usan.
3. **El benchmark no es reproducible**: guarda proveedor, modelo, tokens y `total_ms`, pero **no
   guarda el prompt, ni TTFT, ni VRAM, ni temperatura**. Sin eso no se puede comparar nada.

---

## 1. ESTADO POR MÓDULO (lo que existe, con evidencia)

| Módulo pedido | Qué hay | Veredicto |
|---|---|---|
| **1. GPU Intelligence** | `gpu.rs`: 2 GPU con nombre, VRAM, uso, temperatura, relojes. `vram.rs`: VRAM Guard con competencia y `fits()` con 12 % de margen. `gpu_split` guardado | **Parcial**: mide bien, no enruta nada. El reparto de capas no se aplica |
| **2. Advanced Memory Engine** | Lectura de VRAM/RAM por GPU. Nada de KV cache, contexto por modelo, offload ni MoE | **Casi todo por hacer** |
| **3. Inference Acceleration** | Nada: ni `num_gpu`, ni `num_batch`, ni Flash Attention, ni draft model, ni paralelismo | **No implementado** |
| **4. Intelligent Agents** | 8 agentes con prompt y **lista de nombres de herramientas que no se ejecutan**. 9 políticas sembradas, permisos reales y auditoría | **Parcial**: política sí, herramientas no |
| **5. Self-Optimization Lab** | `benchmarks` con `tokens_per_s`; **9** mediciones guardadas, 8 columnas | **Parcial**: sin TTFT, sin prompt, sin condiciones |
| **6. Security & Recovery** | Permisos por agente y proyecto, timeout con cancelación, `audit_log`, backups verificados, allowlist | **Bien**: es lo más sólido del proyecto |
| **7. Provider Engine** | Ollama (11434), LM Studio (1234/1235), ComfyUI (8188), cloud OpenAI-compatible. GreenBoost: auditado, no integrado | **Parcial**: tres motores reales, uno solo documentado |

---

## 2. LO QUE YA ESTÁ MEDIDO EN TU MÁQUINA (base para comparar)

`benchmarks` en la base real, **9 filas** y 8 columnas (`id, provider, model, prompt_tokens,
completion_tokens, total_ms, tokens_per_s, created_at`) — ni prompt, ni TTFT, ni VRAM:

| Proveedor | Tokens in/out | ms | tok/s | Cuándo |
|---|---|---:|---:|---|
| ollama `Qwythos-9B` | 61 / 652 | 10.562 | **61,73** | 11:39 |
| ollama `Qwythos-9B` | 61 / 247 | 4.083 | **60,49** | 11:39 |
| lmstudio `qwen3-coder-heretic-30b` | 25 / 140 | 20.454 | 6,84 | 13:55 |
| lmstudio `qwen3-coder-heretic-30b` | 25 / 134 | 52.300 | **2,56** | 13:56 |
| lmstudio `qwen3-coder-heretic-30b` | 25 / 148 | 103.036 | **1,44** | 12:50 |

La dispersión 61,73 → 1,44 tok/s **no es del modelo**: es la competencia de VRAM que el propio
VRAM Guard detectó (ComfyUI residente + LM Studio). Es exactamente el fenómeno que el aviso nuevo
de Fase 3.4 vigila. Sin guardar la VRAM del momento, estas filas no se pueden interpretar.

**Entorno actual de Ollama** (leído del usuario de Windows):

```
OLLAMA_MODELS=D:\ollama\models
OLLAMA_CONTEXT_LENGTH=32768
OLLAMA_KV_CACHE_TYPE=q4_0
```

---

## 3. TECNOLOGÍAS: QUÉ ES COMPATIBLE DE VERDAD

Separado por **nivel de certeza**, porque el encargo prohíbe inventar compatibilidades.

### 3.1 Verificado en tu máquina

| Tecnología | Estado | Dato |
|---|---|---|
| Capacidades reales por modelo | ✅ disponible y sin usar | `/api/tags` y `/api/show`: 58 `tools`, 13 `vision`, 44 `thinking` |
| Contexto y tamaño de parámetros | ✅ disponible y sin usar | `llama.context_length = 8192`, `parameter_count = 8.030.261.248` |
| KV cache cuantizado | ✅ **ya activo** | `OLLAMA_KV_CACHE_TYPE=q4_0` global, contexto 32.768 |
| VRAM por GPU y por proceso | ✅ funcionando | `nvidia-smi` + VRAM Guard |
| Competencia entre motores | ✅ funcionando | log 15:11:41 del 01/11 |

### 3.2 Documentado por los motores (aplicable, hay que implementarlo)

| Tecnología | Motor | Cómo |
|---|---|---|
| Contexto, capas en GPU, batch, keep_alive | Ollama | `options`: `num_ctx`, `num_gpu`, `num_batch`, `num_predict`, `top_k`, `top_p`, `min_p`, `repeat_penalty`, `seed` + `keep_alive` fuera de `options` |
| Flash Attention | Ollama | `OLLAMA_FLASH_ATTENTION=1` (variable **global**, afecta a todo) |
| Offload de expertos MoE a CPU | llama.cpp | `--override-tensor exps=CPU` (`-ot`) |
| KV cache por modelo | llama.cpp | `-ctk` / `-ctv` (`f16`, `q8_0`, `q4_0`) |
| Speculative decoding | llama.cpp | `--model-draft` |
| Split por capas entre GPU | llama.cpp | `--split-mode layer --tensor-split` |
| Parámetros de carga | LM Studio | Su API v0 permite fijar contexto y offload **al cargar**; **por verificar** en tu versión (el servidor no estaba levantado durante la auditoría) |

### 3.3 No prometible hoy (y por qué)

| Tecnología | Por qué no |
|---|---|
| **NVFP4 / FP4** | No existe en GGUF. Requiere TensorRT-LLM, que en Windows nativo no es una vía práctica. **No confundir con Q4** |
| **FP8** | Solo en pesos que lo traigan (no GGUF estándar) y con runtimes que lo soporten; Ollama/LM Studio no lo exponen por API |
| **Tensor Parallelism real** | llama.cpp no hace TP; `--split-mode row` no es TP. vLLM en Windows nativo es un calvario. **Bloqueado como promesa** |
| **MTP** | Depende de que el modelo traiga cabezas MTP (GLM/DeepSeek) y de que el motor las use. Hoy ni Ollama ni LM Studio lo exponen por API. **No se puede prometer** |
| **VRAM sumada entre las dos GPU** | Ya medido antes: no es un bloque único. Solo split por capas |

---

## 4. MEJORAS PROPUESTAS, POR ORDEN DE RETORNO

| # | Mejora | Archivos | Riesgo | Retorno |
|---|---|---|---|---|
| **P0** | **Plomería de parámetros**: `InferenceParams` en config (contexto, capas GPU, batch, top_k/p, min_p, repeat, seed, keep_alive) enviados a Ollama y a LM Studio; por modelo | `core/src/config.rs`, `providers.rs`, vista Modelos | Bajo (opcional, con valores por defecto actuales) | **Alto**: desbloquea todo lo demás |
| **P1** | **Registro real de modelos**: leer `capabilities`, `context_length` y `parameter_count` de `/api/tags` + `/api/show` y dejar de adivinar por nombre | `providers.rs` | Bajo | Alto: el router y el catálogo se vuelven fiables |
| **P2** | **Benchmark reproducible**: guardar prompt, TTFT, VRAM antes/después, temperatura y nº de motores activos | `memory.rs`, `Benchmarks.tsx` | Bajo (tabla con columnas nuevas, migración aditiva) | Alto: comparaciones con sentido |
| **P3** | **Estimador de memoria**: por modelo y contexto, calcular VRAM = pesos + KV (según tipo) + overhead, y avisar antes de cargar. Con `q4_0` ya activo se puede comparar f16/q8_0/q4_0 | `core/src/vram.rs` (nuevo `estimate`) | Bajo (solo calcula) | Medio-alto: evita el OOM |
| **P4** | **Proveedor llama.cpp** con flags explícitos (MoE a CPU, KV por modelo, draft model), aislado y desactivable | nuevo `core/src/llamacpp.rs` | Medio | Medio |
| **P5** | **Router con puntuación** (VRAM disponible, contexto pedido, capacidades necesarias, tok/s medidos) | `core/src/providers.rs` | Medio | Medio |
| **P6** | **Agentes con herramientas reales**: hoy son nombres. Depende de tool calling, que no existe | `agents.rs`, `providers.rs` | **Alto** | Alto, pero después de P0/P1 |
| **P7** | **GreenBoost** como motor experimental | ver `GREENBOOST_AUDITORIA.md` | Alto (driver 621.01 sin probar + inyección) | Desconocido hasta medir |

**Regla que propongo para todo esto:** ningún ajuste se aplica solo. Se muestra la simulación
(memoria estimada, riesgo, comparación con lo actual) y se pide permiso.

---

## 5. RIESGOS Y DEPENDENCIAS

| Riesgo | Gravedad | Nota |
|---|---|---|
| Tocar variables **globales** de Ollama (`OLLAMA_FLASH_ATTENTION`, `OLLAMA_KV_CACHE_TYPE`) | **Alta** | Afecta a todo lo que uses, no solo a KUKIA. Cualquier cambio exige backup y aviso |
| Cambiar contexto fuerza recarga del modelo | Media | Se nota como «se ha quedado tonto» durante segundos |
| `num_gpu` no se puede cambiar con el modelo cargado | Media | Hay que descargar y recargar |
| Migración de la tabla `benchmarks` | Baja | Aditiva con `ALTER TABLE ... ADD COLUMN`; backup previo obligatorio |
| Prometer aceleración sin medir | **Alta (reputación)** | Cada mejora debe traer antes/después con tok/s y VRAM |
| MoE a CPU puede ser **más lento** que dejar el modelo entero en GPU si cabe | Media | Medir, no asumir |

Dependencias entre fases: **P1 depende de P0**, **P2 es independiente** (se puede hacer ya),
**P5 y P6 dependen de P1**, **P7 depende de tu autorización** y de la prueba de laboratorio.

---

## 6. PROTOCOLO DE BENCHMARK PROPUESTO (antes de tocar nada)

Para que las cifras signifiquen algo, todas las mediciones deben fijar:

1. **Mismo prompt** (guardado en la fila) y mismos tokens de salida.
2. **Un solo motor cargado** (VRAM Guard sin competencia, comprobado antes).
3. **Misma GPU** (5060 Ti para pruebas; la 5070 es de pantalla).
4. Registro de **TTFT**, tok/s, **VRAM antes y después**, RAM, temperatura y **contexto usado**.
5. Tres repeticiones y se guarda la mediana.
6. Línea base **antes** de cada cambio, en el mismo minuto y con el mismo modelo.

Sin ese protocolo, subir o bajar `num_batch` «porque suena mejor» es ruido.

---

## 7. PLAN POR FASES

| Fase | Contenido | Depende de | Sesiones | Riesgo |
|---|---|---|---:|---|
| **F-P0** | Parámetros de inferencia por modelo + UI en Modelos/Configuración | — | 1–2 | Bajo |
| **F-P1** | Registro real de modelos (capacidades, contexto, parámetros) | — | 1 | Bajo |
| **F-P2** | Benchmark reproducible (prompt, TTFT, VRAM, temperatura) | — | 1 | Bajo |
| **F-P3** | Estimador de memoria y comparador de KV cache | F-P0 | 1 | Bajo |
| **F-P4** | Proveedor llama.cpp con flags (MoE, KV, draft) | F-P0 | 2 | Medio |
| **F-P5** | Router con puntuación y explicación | F-P1, F-P2 | 1–2 | Medio |
| **F-P6** | Agentes con herramientas reales | F-P1 + tool calling | 3–4 | Alto |
| **F-P7** | GreenBoost experimental | tu autorización | 1–2 | Alto |

**Orden recomendado: F-P2 → F-P1 → F-P0 → F-P3 → F-P5 → F-P4 → F-P6 → F-P7.**
Empiezo por el benchmark porque sin medición todo lo demás es fe.

---

## 8. LO QUE NO HARÍA AHORA

- **P6 (agentes con herramientas)** antes de P0/P1: ejecutarían cosas con datos de modelo inventados.
- **P7 (GreenBoost)** sin la prueba de laboratorio: el driver 621.01 no está probado por el proyecto.
- **Cualquier cosa que cambie variables globales de Ollama** sin backup y sin avisarte: se lleva por
  delante tus otros usos del PC.
- **Anunciar FP4/FP8/MTP/tensor parallel** como si estuvieran al caer: en Windows nativo con Ollama
  y LM Studio, hoy no hay vía por API.

---

## 9. NECESITO DE TI

1. ¿Autorizas **F-P2 + F-P1** (benchmark reproducible y registro real de modelos)? Son las dos de
   riesgo bajo y las que hacen medible todo lo demás.
2. ¿Quieres que la plomería de parámetros (F-P0) sea **por modelo** o solo un perfil global?
3. ¿Autorizas tocar `OLLAMA_FLASH_ATTENTION` en algún momento, con backup y aviso? Hoy el KV cache
   ya está en `q4_0`, que **normalmente exige flash attention**; conviene verificar que está activo
   antes de sacar conclusiones de rendimiento.

**Nada modificado. Nada instalado. Nada descargado.**
