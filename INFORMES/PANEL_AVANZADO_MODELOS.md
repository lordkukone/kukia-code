# KUKIA CODE — ADVANCED MODEL CONTROL CENTER

**Fecha:** 01/11/2026 · **Fase:** auditoría de solo lectura → implementación → prueba real
**Backup verificado:** `BACKUPS/MODEL_CONTROL_20261101_222304` (SHA-256 comprobado en origen y copia)

---

## 1. Auditoría: qué se puede cambiar de verdad

### Ollama 0.40.13 — medido, no leído de un manual

| Prueba | Resultado literal | Conclusión |
|---|---|---|
| `num_ctx` 8192 | VRAM declarada **3,45 GB**, `context_length` 8192 en `/api/ps` | se aplica |
| `num_ctx` 2048 (después) | el modelo **se recarga** con la nueva config | el cambio exige recarga |
| `num_gpu: 0` | VRAM del modelo **0,0 GB**; la 5060 Ti bajó de 4.443 a 212 MB | el reparto de capas existe y funciona |
| `kv_cache_type: q8_0` | VRAM idéntica: **3,45 GB** otra vez | **ignorado por petición** |

La matemática de la caché KV cuadra con lo medido: 36 capas × 8 cabezas KV × dim 80 ×
8.192 tokens a fp16 = **0,70 GB**, que sumados a los ~2,75 GB de pesos explican los
3,45 GB que declaró Ollama. Esa misma fórmula, aplicada al modelo que había cargado
después (`flint-local`, 39,8B), predice **12,25 GB** de caché a 262.144 tokens.

### LM Studio — lo que acepta su CLI (`lms load --help`)

`--gpu off|max|0..1` · `-c/--context-length` · `--parallel` · `--ttl` ·
`--speculative-draft-mtp|simple` · `--estimate-only`.
**No** expone: formato de caché KV, flash attention, RoPE/YaRN, batch size, tensor split.

### ComfyUI, Wan2GP y Maestro

No son motores de texto: no tienen parámetros de inferencia de modelo que ajustar. El
panel solo los menciona para no ofrecer controles que no existen.

---

## 2. Matriz de estados por parámetro

Cada control lleva su estado **real**, con el motivo:

| Estado | Qué significa | Ejemplos |
|---|---|---|
| `caliente` | se aplica por petición | temperature, top_p, top_k, min_p, repeat_penalty, presence/frequency penalty, seed, stop, num_predict |
| `recarga` | el motor recarga el modelo | num_ctx, num_gpu, flash_attn, num_batch, num_thread, mmap, mlock |
| `no_compatible` | el motor no lo admite | kv_cache_type, RoPE/YaRN, tensor split, selección de GPU, gpu_offload en Ollama |
| `sin_verificar` | no probado en esta instalación | top_k / min_p / repeat_penalty en LM Studio |

Los `sin_verificar` **no se envían**: se muestran desactivados y con el motivo. No se
finge que un parámetro se ha aplicado.

---

## 3. Lo implementado

**`core/src/control.rs`** (nuevo)
- Lee la arquitectura real del modelo de `/api/show` (capas, cabezas, cabezas KV,
  embedding, contexto nativo) sin asumir la familia.
- Calcula la caché KV para 6 formatos (fp16, bf16, q8_0, q6_k, q5_k, q4_0) con su
  ahorro en GB y en %.
- Declara el contexto actual (de `/api/ps`), el nativo del modelo y lo que añade cada
  8K extra.
- Matriz de capacidades y de estados por motor, con motivo en cada fila.
- Perfiles: 6 de fábrica (`EQUILIBRADO`, `MÁXIMA VELOCIDAD`, `MÁXIMA CALIDAD`,
  `AHORRO DE VRAM`, `CONTEXTO EXTENDIDO`, `GPU + RAM OFFLOAD`), guardar/duplicar/borrar,
  **aplicar guardando antes lo que había** y `Restaurar anterior`.
- Métricas en vivo: VRAM por GPU, temperatura, vatios, RAM, CPU y modelos cargados.

**`core/src/engines.rs`**
- `InferenceParams` crece con los campos reales: min_p, repeat_last_n, presence/frequency
  penalty, stop, num_batch, num_thread, flash_attn, mmap, mlock, gpu_offload, parallel y
  speculative. Todos se traducen a `options` de Ollama o a argumentos de `lms load`.
- `no_soportados` explica campo a campo lo que cada motor no admite.

**Comandos Tauri:** `control_capacidades`, `control_metricas`, `perfiles_listar`,
`perfil_guardar`, `perfil_borrar`, `perfil_aplicar`, `perfil_restaurar_anterior`.

**UI** (`app/src/views/ModelAdvanced.tsx`, dentro de la ficha de modelo que ya existía):
selector de contexto con presets 4K–128K, deslizador y campo exacto; tabla de caché KV;
panel de las dos GPU con uso, libres, temperatura y vatios; capas en GPU; parámetros del
motor; generación; y perfiles. Cada control muestra su estado a color y su motivo al
pasar el ratón.

---

## 4. Pruebas

`cargo test --lib` → **98 tests OK** (9 nuevos de control, 5 de liberación de VRAM en la
fase anterior).

`cargo test --test validacion_control` (contra la máquina):

```
arch: flint · 49 capas · 32 cabezas · 4 KV · ctx nativo 262144
contexto: actual 262144 · fuente Ollama /api/ps (context_length del modelo cargado)
GB de KV por cada 8K: 0.375
  KV fp16    12.25 GB · por_defecto
  KV q8_0     6.51 GB · no_aplicable
  KV q4_0     3.45 GB · no_aplicable
aviso: con el contexto nativo (262144) la caché KV sola pediría 12.2 GB
```

Y con el 4B a 8.192 tokens: arquitectura 36 capas / 8 cabezas KV / 262.144 nativo,
contexto aplicado 8.192 en `/api/ps`, KV fp16 0,70 GB, 27.603 MB de VRAM en uso entre
las dos GPU.

Perfiles: aplicar `MÁXIMA CALIDAD` deja `num_ctx = 32768`, y `Restaurar anterior`
devuelve el `2048` con `temperature 1.5` que había antes. Borrar y sobrescribir por
nombre tampoco duplica.

`cargo check` en `src-tauri` limpio · `npm run build` limpio (53 módulos).

---

## 5. Límites, escritos

1. **KV cache cuantizada no se puede aplicar** en Ollama 0.40.13 por petición (medido) y
   LM Studio no la expone. Se muestra el ahorro que habría, marcado `no_aplicable`.
2. **No se elige la GPU**: Ollama reparte solo (hoy: 11,5 + 9,6 GB sin preguntar). Fijar
   tarjeta exige `CUDA_VISIBLE_DEVICES`, que es del proceso del servidor, y KUKIA no
   reinicia procesos.
3. **RoPE/YaRN no existe en ninguna de las dos APIs**: en vez de un control falso se
   avisa del contexto nativo y de lo que cuesta alcanzarlo.
4. **Tokens/s no se muestra en vivo**: exige generar, y no se genera nada a espaldas del
   usuario.
5. **LM Studio**: su arquitectura no se publica, así que allí no hay tabla KV ni máximo de
   contexto calculado; solo lo que su CLI acepta.
6. Aplicar un perfil que pida más contexto del que cabe **puede provocar la recarga** del
   modelo por parte del motor; el preflight de carga sigue siendo el que decide si se
   carga o no.
