# CC1–CC4 — CENTRO DE CONTROL DE MOTORES

Fecha: 01/11/2026 · Alcance: lo autorizado en `AUDITORIA_CONTROL_CENTER.md` §6 (CC1–CC4).

---

## Qué se ha implementado

Módulo nuevo `core/src/engines.rs` (937 líneas) con las cuatro fases, más comandos
Tauri, tipos en `api.ts` y la vista `Models.tsx`. `providers.rs` no se ha tocado:
el catálogo de chat sigue igual y esto es una capa aparte que no duplica nada.

### CC1 — Estado real de los servidores

`engines::status(engine)` y `status_all()` devuelven, por motor:

| Dato | Ollama | LM Studio |
|---|---|---|
| versión | `/api/version` → `0.40.13` | no la expone por API/CLI |
| puerto | fijo 11434 (API nativa) | **descubierto**: `lms server status` → 1235, si falla sonda TCP a 1234/1235 |
| instalados | `/api/tags` → 82 | `/api/v0/models` → 32 |
| cargados | `/api/ps` | campo `state == "loaded"` |
| latencia | medida con `Instant` alrededor de la petición | ídem |

El puerto **no se cambia nunca**: solo se lee. `port_source` dice de dónde salió
(`lms server status`, `sonda TCP` o `fijo`), para que no haya magia.

### CC2 — Ficha completa de modelo

`engines::cards(engine)` lista rápido (una llamada por motor) y
`engines::card(engine, model)` trae la ficha fina.

- **Ollama**: `/api/tags` para la lista y `/api/show` para la ficha (familia,
  `parameter_size`, cuantización, formato, `capabilities`, y el contexto real
  que vive en `model_info.<familia>.context_length`).
- **LM Studio**: `/api/v1/models` (arquitectura, `params_string`, cuantización,
  formato, `max_context_length`, capacidades, `loaded_instances`).

Ejemplo medido: `aratan/Qwythos-9B-...` → `qwen35`, 9.2B, Q4_K_M,
contexto **1.048.576**, capacidades `completion, thinking, tools, vision`.

La lista de Ollama deja `context_length` vacío a propósito: `/api/tags` no lo
publica y rellenarlo con una suposición sería mentir.

### CC3 — Parámetros de inferencia por modelo

`InferenceParams` (temperatura, top_p, top_k, num_ctx, num_gpu, repeat_penalty,
seed, max_tokens, keep_alive), guardado en `%APPDATA%\KukiaCode\inferencia.json`
con clave `motor::modelo`. **Vacío = no se envía nada** y decide el motor.

Validación real por motor (`no_soportados`): en LM Studio se avisa de que
`top_k`, `num_gpu` (es un porcentaje, no capas) y `keep_alive` no se aplican; en
Ollama se avisa de `max_tokens` (allí es `options.num_predict`). Los avisos se
devuelven a la interfaz, no se tiran.

### CC4 — Carga controlada con preflight

`engines::preflight(engine, model, params)` **no toca la VRAM**:

- LM Studio: `lms load --estimate-only` → «Estimated GPU Memory: 21.29 GiB».
- Ollama: tamaño del GGUF de `/api/tags` × 1.12 + KV estimada por `num_ctx`.

Se compara con la **mejor GPU libre** de `vram::gpus_vram()` y se responde
`fits`. `engines::load()` **exige `fits == true`**: si no cabe, no se carga y se
dice qué falta. La descarga reutiliza `vram::unload` (Ollama `keep_alive: 0`,
LM Studio `/api/v1/models/unload`, ComfyUI `/free`).

---

## Verificación ejecutada

`cargo test --lib engines::tests -- --nocapture` → **6/6 en verde**, con salida real:

```
ollama    online=true  puerto=Some(11434) (fijo (API nativa)) version=Some("0.40.13")
          modelos=82 cargados=1 428ms
lmstudio  online=true  puerto=Some(1235) (lms server status) modelos=32 cargados=1 2ms
ficha completa: aratan/Qwythos-9B-... · family=Some("qwen35") params=Some("9.2B")
                quant=Some("Q4_K_M") ctx=Some(1048576) caps=["completion","thinking","tools","vision"]
preflight aratan/Qwythos-9B-... → necesita≈7.37 GB fits=false
          · archivo 6.36 GB + 12 % + KV(4096) ≈ 7.37 GB · GPU0 (RTX 5070): 0.33 GB libres → NO cabe
```

- `cargo check` núcleo y Tauri: **OK**.
- `npx tsc --noEmit`: **OK**.

---

## Límites, dichos claros

1. **La carga real no se ha ejecutado**: solo el preflight. Cargar algo en tu
   VRAM sin pedírtelo sería meterme donde no me llaman. El preflight ya
   demuestra que el freno funciona (rechazó un 9B por falta de VRAM).
2. **La versión de LM Studio se queda en `null`**: ni su REST ni `lms` la
   publican en lo que se ha probado. No se inventa.
3. **El preflight de Ollama es una estimación propia**, no del motor: el texto
   de `detail` explica la fórmula para que se pueda discutir.
4. **No hay capturas de la ventana** de esta vista: los números salen de los
   tests, no de un recorrido visual.
5. Falta recompilar el EXE para que estas pantallas estén en la app instalada.
