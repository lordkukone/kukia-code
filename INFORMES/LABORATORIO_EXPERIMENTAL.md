# KUKIA CODE — LABORATORIO EXPERIMENTAL

**Fecha:** 01/11/2026 · **Fase:** auditoría de solo lectura → implementación → prueba real
**Backup verificado:** `BACKUPS/MODEL_CONTROL_20261101_222304`

---

## 1. Lo que la máquina dice de sí misma

Antes de escribir una línea del catálogo se leyó el entorno real de Ollama. Aquí está
la clave de todo:

| Variable | Valor real | Qué implica |
|---|---|---|
| `OLLAMA_KV_CACHE_TYPE` | **q4_0** | la caché KV **ya** está cuantizada a 4 bits: por eso el modelo de 39,8B cabe a 262K de contexto |
| `OLLAMA_CONTEXT_LENGTH` | **32768** | contexto por defecto del servidor, mayor que el 4096 de fábrica |
| `OLLAMA_FLASH_ATTENTION` | sin definir | flash attention no está activo a nivel de servidor |
| `OLLAMA_NUM_PARALLEL` | sin definir | sin paralelismo configurado |

Se leen del proceso **y del registro de Windows**, porque la variable de usuario no
siempre está en la sesión del shell: la primera versión del laboratorio decía «sin
definir» para `OLLAMA_CONTEXT_LENGTH` y eso era falso.

Otras medidas de esta sesión:

- `flash_attn` + `num_batch` por petición → HTTP 200 y **`load_duration` de 30,6 s**:
  son opciones de carga, recargan el modelo.
- `kv_cache_type: q8_0` por petición → **misma VRAM (3,45 GB)**: se ignora.

---

## 2. El catálogo: 44 técnicas, 23 marcadas como incompatibles

| Nivel | Cuántas |
|---|---|
| estable | 9 |
| experimental | 7 |
| muy experimental | 5 |
| **incompatible** | **23** |

Que 23 sean incompatibles no es pesimismo, es lo medido: ni Ollama 0.40.13 ni LM Studio
exponen tensor parallel, FP8/NVFP4/EXL2/AWQ/GPTQ, Medusa, EAGLE, paged attention
configurable, chunked prefill, K y V con precisiones distintas, NUMA ni memory pool.
Cada una dice **por qué** y qué haría falta (vLLM o TensorRT-LLM, que no están instalados
y no se instalan sin autorización).

Las **7 activables desde KUKIA** son las que tienen mecanismo real:

`flash_attn` · `layer_split` (num_gpu) · `mmap` · `mlock` · `speculative` (LM Studio) ·
`parallel_batching` (LM Studio) · `reparto_uniforme_vs_custom` (medible con A/B).

Las de `variable_servidor` (KV Q4/Q8, contexto por defecto, flash a nivel de servidor)
**no** se ofrecen como botón: muestran el valor actual y la orden exacta para que la
cambies tú y reinicies Ollama. KUKIA no reinicia procesos.

---

## 3. Banco de pruebas: medidas reales

Contra `flint-local:latest` (39,8B Q4_K_M), 32 tokens:

```
ok=true · 64,5 tok/s · 32 tokens
prefill 118,9 tok/s · TTFT 94,0 ms · ITL 15,5 ms · carga 9,8 ms · total 599 ms
pico GPU0 11.726 MB · pico GPU1 14.314 MB
pico RAM 29,8 GB · 36 °C · 49 W
```

Los tiempos salen de `eval_count`, `eval_duration`, `prompt_eval_duration` y
`load_duration` de Ollama; el pico de VRAM/RAM/temperatura/vatios se muestrea cada 250 ms
mientras la generación corre. En LM Studio se mide el tiempo real de la petición y los
campos que su API no publica (`prefill`, `TTFT`, `ITL`) van a `—`, no a un número inventado.

A/B de control con A = B (misma configuración) para validar la resta:

```
A 64,3 tok/s · B 64,1 tok/s · delta −0,3 % → «sin diferencia clara: vuelve a medir»
delta VRAM: [(0, −6), (1, 0)]
```

Ese −0,3 % es el ruido de la máquina, y el laboratorio se niega a llamarlo mejora:
el umbral de veredicto es el 3 %.

---

## 4. Cómo queda en la app

Pestaña nueva **«Laboratorio»** (`app/src/views/Lab.tsx`) con:

- selector de motor y modelo (toma los modelos reales de cada motor, marca el cargado);
- catálogo filtrable por nivel y texto, con estado real por técnica;
- detalle con descripción, requisitos, riesgos, variable y valor actual, y botones
  **Activar experimento** / **Restaurar configuración** (la anterior se guarda antes de
  tocar nada, en `laboratorio_previo.json`);
- banco de pruebas con prompt editable y tokens;
- comparación A/B con cuatro presets que cambian **una sola cosa**;
- historial de las últimas 100 medidas en `laboratorio.json`.

Comandos: `lab_catalogo`, `lab_bench`, `lab_ab`, `lab_historial`, `lab_activar`,
`lab_restaurar`.

---

## 5. Verificación

- `cargo test --lib` → **105 tests OK** (10 nuevos del laboratorio).
- `cargo test --test validacion_lab` → 3 pruebas reales contra Ollama, con las cifras
  de arriba.
- `cargo check` en `src-tauri` limpio · `npm run build` limpio (54 módulos).

---

## 6. Límites, dichos

1. **Nada se instala ni se descarga**: si una técnica necesita vLLM, TensorRT-LLM o un
   driver distinto, se queda en «incompatible» con el motivo.
2. **No se reinicia Ollama**: los experimentos de variable de entorno son informativos,
   con la orden exacta para hacerlo a mano.
3. **Una sola medida no es una prueba**: el A/B avisa de que hay que repetir y no decide
   por ti.
4. **La calidad no se mide automáticamente**: el banco da velocidad, memoria, temperatura
   y potencia; la calidad de la respuesta hay que mirarla con tus ojos.
5. **Speculative decoding** depende de que el modelo traiga cabeza MTP o de tener un draft
   compatible; si no, el CLI falla y se informa del fallo tal cual.
6. Durante la auditoría se cambió sin querer el contexto del modelo que otra aplicación
   tenía cargado (de 262.144 a 4.096) al probar `flash_attn` y `num_batch`. Se avisa aquí
   porque es exactamente el tipo de efecto que este laboratorio debe evitar: por eso todas
   las pruebas nuevas usan el modelo que ya está cargado **sin cambiarle nada**.
