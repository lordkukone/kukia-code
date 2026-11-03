# LA1 + LA2 — LIVE ACTION STUDIO conectado a ComfyUI real y dos clips medidos

Fecha: 2026-11-01 · Máquina: i9-10900K · 64 GB · RTX 5070 12 GB (GPU0) · RTX 5060 Ti 16 GB (GPU1)
ComfyUI: 0.43.0 en `127.0.0.1:8188` · 2745 nodos

## 1. Resumen de una línea

Los dos clips salieron de verdad por ComfyUI: **Anime2Real LTX en 905,6 s** y **Wan Animate 2 en 503,6 s**, con picos medidos de **15.995 MiB** y **13.924 MiB** de VRAM y **57,7 GB** y **44,3 GB** de RAM usada.

## 2. LA1 — el estudio habla con el ComfyUI real

Lo que quedó conectado y comprobado contra el servidor, no contra suposiciones:

| Pieza | Estado |
|---|---|
| `system_stats`, `queue`, `history`, `prompt` | responden |
| Modelos visibles | se preguntan **nodo a nodo** (`/object_info/<clase>`), no el volcado de 22 MB que tardaba 43 s |
| Preflight por plantilla | compara `requiere` del manifest contra los modelos que ComfyUI ve de verdad |
| Conversión | `POST /prompt` + sondeo de `/history` con tope de 1800 s |
| Historial | `output/anime_real/historial.jsonl` |

Plantillas en el manifest (6): tres de imagen ya existentes, `anime2real_ditto_video` sigue `pendiente_conversion` (es WanGP, no ComfyUI), y **dos nuevas de vídeo, medidas y listas**:

| id | motor | estado | segundos medidos |
|---|---|---|---|
| `anime2real_ltx_video` | LTX 2.3 22B fp8 + LoRA IC anime2real | listo | 906 |
| `wan_animate_video_api` | Wan 2.2 Animate 2 distilled int8 + lightx2v | listo | 504 |

Ficheros nuevos:
- `workflows/anime_real/anime2real_ltx_api.json` (25 nodos)
- `workflows/anime_real/wan_animate_api.json` (15 nodos)
- `tools/ui_a_api.py` (conversor UI→API, para los workflows que se descargan con subgrafos)
- `tools/vigilar_cola.py` y `tools/medir_clip.py` (medición: tiempo + pico VRAM por GPU + pico RAM)

## 3. LA2 — los dos clips, con números

### Clip 1 — Anime2Real con LTX 2.3 22B

| Dato | Valor |
|---|---|
| Inicio → fin (historial de ComfyUI) | 21:23:19 → 21:38:25 = **905,6 s** |
| Resolución / fotogramas | 448×768 · **49 frames** @ 25 fps |
| Modelo | `ltx-2.3-22b-dev-fp8` + `ltx-2.3-22b-distilled-lora-384` (0,5) + `ltx23_anime2real_rank64_v1_4500` (1,0) |
| Text encoder | `comfy_gemma_3_12B_it.safetensors` |
| Muestreo | 9 sigmas (`euler_ancestral`), cfg 1 |
| Entrada | `a2r_guia.mp4` (1080×1920, 150 frames, 5 s) |
| **Pico VRAM GPU1** | **15.995 MiB** de 16.311 (98 %) |
| **Pico VRAM GPU0** | 6.436 MiB de 12.227 |
| **Pico RAM usada** | **57,73 GB** de 63,88 (mínimo 6,15 GB libres) |
| Salida | `output/anime_real/kukia_a2r_00001_.webp` (1,28 MB) |

### Clip 2 — Wan Animate 2 (transferencia de movimiento a acción real)

| Dato | Valor |
|---|---|
| Inicio → fin | 21:50:17 → 21:58:41 = **503,6 s** |
| Resolución / fotogramas | 480×832 · **37 frames** @ 16 fps |
| Modelo | `wan_animate_2_distill_int8_convrot` + `lightx2v_I2V_14B_480p_cfg_step_distill_rank64_bf16` |
| Text encoder / VAE / CLIP vision | `umt5_xxl_fp8_e4m3fn_scaled` · `Wan2_1_VAE_bf16` · `clip_vision_h` |
| Muestreo | 6 pasos, cfg 1, `euler`/`simple`, shift 5 |
| Entrada | `animate2_guide.mp4` (pose) + `animate2_ref.jpeg` (personaje) |
| **Pico VRAM GPU1** | **13.924 MiB** de 16.311 (85 %) |
| **Pico VRAM GPU0** | 2.462 MiB |
| **Pico RAM usada** | **44,34 GB** de 63,88 |
| Salida | `output/anime_real/kukia_wan_animate_00001_.webp` (1,15 MB) |

Muestras de fotogramas en `output/anime_real/muestras/`:
- `a2r_ltx_f16.png`: sigue siendo anime.
- `wan_animate_f16.png`: cara fotorrealista, piel con poros.

## 4. Calidad: lo que dicen las imágenes, no lo que me gustaría

- **Wan Animate 2 convierte a acción real de verdad** con estos ajustes (6 pasos, cfg 1). Contra: toma la identidad de la imagen de referencia, no la del vídeo de guía. Es el comportamiento esperado de la transferencia de movimiento, pero hay que decirlo.
- **Anime2Real LTX, con 9 sigmas y cfg 1, NO rompe el estilo**: el fotograma 16 sigue en anime. El pipeline funciona y genera 49 frames, pero el ajuste es de coste, no de calidad. Para que convierta hay que subir pasos (el workflow comunitario usa 121 frames y más sigmas) y probablemente subir `strength` del IC-LoRA.

## 5. Tropezones técnicos y cómo se resolvieron

1. **El workflow descargado venía en formato UI con subgrafo de 43 nodos** → ComfyUI no lo acepta en `/prompt`. Escribí `tools/ui_a_api.py`; resuelve subgrafos y promociones, pero los combos dinámicos (`ResizeImageMaskNode`) desalinean los widgets, así que para medir monté grafos API limpios.
2. **`UNETLoader` nativo con el checkpoint KJ** → `KeyError: 'blocks.0.ffn.0.weight'`. Ese fichero no está en layout nativo.
3. **`WanVideoModelLoader` con el mismo KJ** → `KeyError: 'blocks.0.self_attn.k.weight'`.
4. **`LoadWanVideoT5TextEncoder`** → rechaza fp8 escalado; con quanto int8 → `KeyError 'blocks.0.attn.q.weight'`.
5. **Solución real**: el checkpoint que sí carga es `WanVideo\2_2\wan_animate_2_distill_int8_convrot.safetensors` (ya instalado, layout `int8_convrot`, el mismo que pide el blueprint oficial `Motion Transfer (Wan Animate 2).json`), con CLIP/VAE/CLIP-vision nativos.
6. **RAM**: al empezar solo había **6,6 GB libres** porque LM Studio tenía DeepSeek-R1-32B cargado con `mmap+mlock` (35,7 GB). Se expulsó con el CLI oficial `lms unload --all` (no maté procesos). Ahora hay 41 GB libres; el modelo se puede volver a cargar en LM Studio cuando quieras.

## 6. Qué NO se ha tocado

- Ni un archivo de `I:\CEREBRO`.
- Ni modelos movidos ni borrados: los checkpoints siguen donde estaban (solo se leyó el header de algunos para ver el layout).
- Ni configuración de ComfyUI, drivers, CUDA ni Python.
- LM Studio: única acción, expulsar el modelo con su propio CLI (reversible).

## 7. Siguiente paso natural

1. Subir calidad del clip LTX: más sigmas y `strength` del IC-LoRA, y comparar el frame 16 contra el anime original.
2. Comprobar si el limite real es la 5060 Ti al 98 % de VRAM: mismo clip con `--lowvram` o repartiendo el VAE en la 5070.
3. Añadir el clip al panel con barra de progreso real (porcentaje de nodos ejecutados) en vez de solo "en cola".
