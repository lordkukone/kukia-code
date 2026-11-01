# KUKIA AI CONTROL CENTER — AUDITORÍA (Ollama + LM Studio)

**Fecha:** 01/11/2026 · **Autor:** Xoxete
**Modo:** solo lectura. Se han llamado únicamente endpoints que **no modifican nada**.

---

## 0. VERSIONES REALES MEDIDAS HOY

| Dato | Valor |
|---|---|
| Ollama | **0.40.13** en `127.0.0.1:11434` (`/api/version` responde) |
| Ollama modelos | **81** (`/api/tags`) |
| LM Studio cliente | `C:\Users\kuki\.lmstudio\bin\lms.exe` (120 MB, CLI commit `dcefd5a`) |
| LM Studio servidor | **puerto 1235** (¡el 1234 NO responde!) |
| LM Studio modelos | **32**, de ellos **27 con `tool_use`**; **1 cargado** a 65.536 de contexto |
| Campos que da LM Studio | `arch, capabilities, compatibility_type, id, loaded_context_length, max_context_length, publisher, state, type` |

**Primer hallazgo:** la documentación y el código asumen 1234; en tu máquina el servidor está en
**1235**. KUKIA ya prueba 1234 y cae a 1235, pero no lo descubre: lo tiene cableado.

---

## 1. QUÉ USA KUKIA HOY (literal, verificado en el código)

| Motor | Endpoint | Para qué |
|---|---|---|
| Ollama | `/api/tags` | listar modelos |
| Ollama | `/api/chat` | chat y streaming |
| Ollama | `/api/ps` | VRAM Guard |
| LM Studio | `/v1/models` | listar modelos |
| LM Studio | `/v1/chat/completions` | chat y streaming |
| LM Studio | `/api/v0/models` | estado de carga (solo VRAM Guard) |

**Y de todos los parámetros de inferencia, solo se envía `temperature`.** Ni contexto, ni capas,
ni batch, ni KV cache, ni `keep_alive`, ni semilla, ni top-k/p.

---

## 2. MATRIZ: FUNCIÓN POR FUNCIÓN

Clases: **IMPLEMENTADA** · **PARCIAL** · **FALTANTE** · **NO SOPORTADA** · **REQUIERE INVESTIGACIÓN**

### 2.1 Servidor y conexión

| Función | Ollama | LM Studio | Vía real |
|---|---|---|---|
| Estado del servicio | **FALTANTE** | **FALTANTE** | `GET /api/version` · `lms server status` |
| Iniciar / detener | **FALTANTE** | **FALTANTE** | `ollama serve` / `ollama stop` · `lms server start|stop` |
| Host, puerto, endpoint | **PARCIAL** (base en config) | **PARCIAL** | config + `lms server status` |
| Latencia / tiempo de respuesta | **FALTANTE** | **FALTANTE** | medible con petición `GET` |
| Logs | **FALTANTE** | **FALTANTE** | `lms log`; Ollama escribe a su propio log |
| Diagnóstico | **FALTANTE** | **FALTANTE** | versión + endpoints + VRAM |

### 2.2 Modelos

| Función | Ollama | LM Studio | Vía real |
|---|---|---|---|
| Listar | **IMPLEMENTADA** | **IMPLEMENTADA** (`/v1/models`) | — |
| Ficha rica (arquitectura, cuantización, contexto, capacidades) | **FALTANTE** (se adivina por nombre) | **FALTANTE** (los datos existen en `/api/v0/models`, solo se usa el estado) | `/api/show` · `/api/v0/models` |
| Descargar con progreso | **FALTANTE** | **FALTANTE** | `/api/pull` (stream NDJSON) · `lms get` |
| Cancelar descarga | **FALTANTE** | **FALTANTE** | cerrar el stream |
| Eliminar | **FALTANTE** | **NO SOPORTADA** | `ollama rm` / `DELETE /api/delete` · **`lms` no tiene borrado** (solo UI o disco) |
| Copiar / duplicar | **FALTANTE** | **NO SOPORTADA** | `ollama cp` |
| Importar GGUF | **FALTANTE** | **PARCIAL** | `ollama create` con Modelfile · `lms import` |
| Crear con Modelfile | **FALTANTE** | — | `ollama create -f` |
| Tamaño real y cuantización | **PARCIAL** (en `/api/tags`) | **FALTANTE** | `/api/tags` · `/api/v0/models` |
| Cargar / descargar modelo | **PARCIAL** (sin control) | **FALTANTE** | `keep_alive` · `lms load` / `lms unload` |

### 2.3 Parámetros de inferencia

| Función | Ollama | LM Studio | Nota |
|---|---|---|---|
| temperature | **IMPLEMENTADA** | **IMPLEMENTADA** | único parámetro real hoy |
| top_k, top_p, min_p, repeat_penalty, seed, num_predict, stop | **FALTANTE** | **FALTANTE** | API de ambos lo admite |
| typical_p, tfs_z, mirostat (tau/eta), repeat_last_n | **FALTANTE** | **NO SOPORTADA** (específico de llama.cpp vía Ollama) | solo Ollama |
| frequency/presence penalty | **FALTANTE** | **FALTANTE** | solo LM Studio/OpenAI |
| JSON / structured output | **FALTANTE** | **FALTANTE** | `format: json` · `response_format` |
| Tool calling | **NO SOPORTADA en KUKIA** (no se envían `tools`) | **NO SOPORTADA en KUKIA** | 58 modelos Ollama y 27 de LM Studio lo soportan |

### 2.4 Memoria, contexto y GPU

| Función | Ollama | LM Studio | Vía real |
|---|---|---|---|
| `num_ctx` (contexto) | **FALTANTE** | **FALTANTE** | `/api/chat options` · `lms load -c` |
| `num_batch` | **FALTANTE** | **PARCIAL** | opción de Ollama · eval batch en LM Studio (preset) |
| `num_gpu` / capas | **FALTANTE** | **FALTANTE** | opción de Ollama · `lms load --gpu off|max|0..1` |
| Main GPU | **FALTANTE** | **FALTANTE** | `OLLAMA_*` · selección de dispositivo (preset) |
| Parallel requests | **FALTANTE** | **FALTANTE** | `OLLAMA_NUM_PARALLEL` (global) · `lms load --parallel` |
| Max loaded models | **FALTANTE** | — | `OLLAMA_MAX_LOADED_MODELS` (global) |
| Keep alive / TTL | **FALTANTE** | **FALTANTE** | `keep_alive` por petición · `lms load --ttl` |
| Descarga automática | — | **FALTANTE** | `lms load --ttl` |
| Estimación de memoria **antes** de cargar | **FALTANTE** | **FALTANTE (pero existe)** | **`lms load --estimate-only`** calcula sin cargar |
| Flash Attention | **FALTANTE** | **REQUIERE INVESTIGACIÓN** | `OLLAMA_FLASH_ATTENTION` · LM Studio no lo expone en CLI |
| KV Cache: tipo/cuantización | **REQUIERE INVESTIGACIÓN** | **REQUIERE INVESTIGACIÓN** | `OLLAMA_KV_CACHE_TYPE` **es global** (no por modelo) · LM Studio vía preset/engine config |
| Speculative decoding (draft) | **NO SOPORTADA** | **FALTANTE (y existe)** | `lms load --speculative-draft-simple --speculative-draft-model` |
| MTP | **NO SOPORTADA** | **FALTANTE (y existe)** | `lms load --speculative-draft-mtp` (si el modelo lo trae) |
| Tensor parallelism | **NO SOPORTADA** | **NO SOPORTADA** | ningún motor lo expone por API aquí |

### 2.5 Monitorización, benchmarks y API

| Función | Estado | Vía |
|---|---|---|
| Modelos cargados y VRAM | **IMPLEMENTADA** (VRAM Guard) | `/api/ps` · `/api/v0/models` |
| Aviso de competencia | **IMPLEMENTADA** (Fase 3.4) | vigilante de fondo |
| tok/s por generación | **PARCIAL** (se mide, no se expone por modelo) | respuesta de chat |
| TTFT | **FALTANTE** | medir primer chunk del stream |
| Benchmark comparable | **PARCIAL** (sin prompt ni VRAM) | tabla `benchmarks` |
| Comparar Ollama vs LM Studio | **FALTANTE** | — |
| Administrador de API (endpoints, errores, peticiones) | **FALTANTE** | — |

---

## 3. RESUMEN NUMÉRICO

| Clase | Cuántas | Ejemplos |
|---|---:|---|
| IMPLEMENTADA | **7** | listar modelos, chat, streaming, VRAM, competencia, logs propios, benchmark básico |
| PARCIAL | **6** | ficha de modelo, cargar modelo, tok/s, benchmark, base URL, importar |
| FALTANTE | **23** | arrancar/detener, descargar, eliminar, parámetros, contexto, capas, keep_alive, TTFT… |
| NO SOPORTADA | **7** | borrar en LM Studio, tensor parallel, MTP en Ollama, mirostat en LM Studio… |
| REQUIERE INVESTIGACIÓN | **3** | flash attention en LM Studio, KV cache por modelo, engine-config-file |

---

## 4. LOS TRES HALLAZGOS QUE CAMBIAN EL PLAN

1. **`lms load --estimate-only`**: calcula el consumo de un modelo **sin cargarlo**. Es el
   preflight perfecto: KUKIA puede decir «esto necesita X GB y no cabe» antes de tocar la VRAM.
2. **LM Studio soporta MTP y draft model por CLI** (`--speculative-draft-mtp`,
   `--speculative-draft-simple --speculative-draft-model`). En la auditoría anterior lo di por no
   disponible «por API»; **rectifico**: no está en la API REST, pero sí en el CLI. Depende del modelo.
3. **El KV Cache de Ollama es global** (`OLLAMA_KV_CACHE_TYPE`), no por modelo. Un «KV Cache Lab»
   honesto debe decir eso: en Ollama se cambia **para todo** y exige reiniciar; en LM Studio va por
   preset de carga.

---

## 5. ARQUITECTURA PROPUESTA

```
core/src/engine.rs          (trait común: status, models, load, unload, params)
   ├── core/src/ollama.rs   (HTTP /api/* + CLI para lo que la API no cubre)
   └── core/src/lmstudio.rs (HTTP /api/v0/* + /v1/* + CLI `lms`)
core/src/params.rs          (InferenceParams por modelo, validado según motor y versión)
```

- **Detección de capacidades por versión**, no por suposición: si `0.40.13` no admite una opción, no
  se muestra.
- **Todas las acciones destructivas** (borrar, descargar, expulsar modelo, reiniciar servicio):
  confirmación explícita y registro en `audit`.
- **Preflight** obligatorio antes de cargar: `lms load --estimate-only` y estimación propia para Ollama.
- **Perfiles** en `%APPDATA%\KukiaCode\perfiles.json`, sin tocar configuraciones originales.
- **Descubrimiento de puerto**: 1234, 1235 y el que diga `lms server status`.

---

## 6. PLAN POR FASES

| Fase | Contenido | Riesgo | Sesiones |
|---|---|---|---:|
| **CC1** | Estado real de ambos servidores (versión, puerto, modelos, cargado, latencia) + descubrimiento 1234/1235 | Bajo | 1 |
| **CC2** | Ficha completa de modelo (Ollama `/api/show`, LM Studio `/api/v0/models`) y Unified Model Manager | Bajo | 1–2 |
| **CC3** | Parámetros de inferencia por modelo (la plomería que falta) + validación por motor | Bajo | 1–2 |
| **CC4** | Carga/descarga controlada: `keep_alive`, `lms load`/`unload`, `--ttl`, preflight `--estimate-only` | Medio | 1–2 |
| **CC5** | Descargas con progreso y cancelación (`/api/pull`, `lms get`) + borrado solo Ollama | Medio | 1 |
| **CC6** | Benchmark comparable (TTFT, tok/s, VRAM, prompt) entre motores y GPUs | Bajo | 1 |
| **CC7** | KV Cache Lab honesto: global en Ollama, por preset en LM Studio, con reinicio explícito | **Alto** | 1–2 |
| **CC8** | Speculative/MTP donde el modelo lo permita, medido | Medio | 1–2 |
| **CC9** | Perfiles y router (depende de CC2/CC3) | Medio | 2 |

**Total: 10–14 sesiones.** CC1–CC4 es lo que convierte el panel en un centro de control de verdad.

---

## 7. RIESGOS

| Riesgo | Gravedad | Mitigación |
|---|---|---|
| Parar `ollama serve` mientras otro proceso lo usa | **Alta** | Aviso previo + confirmación + comprobar `/api/ps` |
| Cambiar `OLLAMA_KV_CACHE_TYPE` afecta a **todo** | **Alta** | Cambio explícito, backup del valor, reinicio avisado |
| Borrar un modelo es irreversible | **Alta** | Confirmación con nombre escrito + `audit` |
| Asumir puerto 1234 | Media | Descubrimiento real (hoy está en 1235) |
| `--gpu max` puede dejar sin VRAM a la GPU de pantalla | Media | Recomendar 5060 Ti y avisar por GPU0 |
| `lms` CLI puede cambiar entre versiones | Media | Detección de capacidades + fallo explícito, no silencioso |
| Presentar el KV Cache por modelo en Ollama | Alta (credibilidad) | No existe: es global, y se dirá |

---

## 8. LO QUE NO HARÍA

- **Cambiar puertos automáticamente**: rompe LM Studio y a ti.
- **Borrar modelos desde la app en LM Studio**: su CLI no lo permite; meterse en el disco es peor.
- **Enseñar un control de KV Cache «por modelo» en Ollama**: es mentira, es global.
- **Añadir `--parallel 4` por defecto**: reparte VRAM y KV cache; se mide antes.
- **Ejecutar MTP/draft sin comprobar que el modelo lo trae**: daría un error críptico o basura.

---

## 9. NECESITO DE TI

1. ¿Autorizas **CC1–CC4**? (estado real, fichas completas, parámetros y carga controlada con preflight)
2. ¿Aceptas que en Ollama el KV Cache se trate como **global** con reinicio avisado, en vez de
   fingir que es por modelo?
3. ¿Quieres que KUKIA pueda **iniciar/parar** `ollama serve` y `lms server`, o solo informar?
4. Para borrar modelos: ¿solo Ollama (que sí tiene `rm`), dejando LM Studio a su interfaz?

**Nada modificado. Nada instalado. Nada descargado. Nada borrado.**
