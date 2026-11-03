# KUKIA LIVE ACTION STUDIO — AUDITORÍA FASE 1 (SOLO LECTURA)

**Fecha:** 1 de noviembre de 2026
**Alcance:** inventario real del entorno (ComfyUI, Wan2GP, Maestro, modelos, nodos, GPUs, discos) y del código existente de KUKIA CODE, para decidir qué se puede construir de verdad.
**Modo:** lectura. No se ha instalado, descargado, actualizado, renombrado ni borrado nada. No se ha tocado la configuración de ComfyUI, Wan2GP, Pinokio, CUDA ni drivers. `git status` en KUKIA_CODE solo muestra el volcado temporal `.tmp_audit/` generado por esta auditoría.

---

## 1. RESUMEN EJECUTIVO

1. **El pipeline Anime2Real existe, completo, en tu disco.** El workflow `LTX 2.3 – Anime2Real` (43 nodos) y sus cuatro modelos están presentes y verificados uno a uno. No hay que descargarlo: hay que **probarlo**, porque nunca se ha ejecutado.
2. **KUKIA nunca ha generado un solo vídeo.** En el historial de ComfyUI hay 16 trabajos y **todos son imágenes SDXL** (Realistic Vision, animagineXL, IP-Adapter). Cero vídeo, cero LTX, cero Wan. El módulo `anime_real` de KUKIA es un conversor de **stills** con SDXL.
3. **ComfyUI solo ve una GPU.** Reporta un único dispositivo, la RTX 5060 Ti (16 GB). La 5070 no está expuesta al proceso. Y en este momento ambas tarjetas están casi llenas: **173 MiB libres** en la 5070 y **1,9 GB** en la 5060 Ti.
4. **La VRAM es el techo real, no la tecnología.** El caso medido con más autoridad (RTX 4090, 24 GB) para este mismo workflow consumió **23,4 GB de VRAM + 15,2 GB de memoria compartida + 77 GB de RAM de sistema**, a 1280×720. Tú tienes 16 GB de VRAM y 63,9 GB de RAM. A 960×544 (lo que ya trae el workflow) es viable, pero **va a ir por offload** y hay que medirlo, no prometerlo.
5. **El caballo de batalla realistamente rápido es Wan 2.2 Animate en GGUF Q8.** Hay un caso documentado con **tu misma tarjeta (5060 Ti 16 GB) y 64 GB de RAM**: 5 s a 1024×574 en **~7 minutos**. Ese es el motor que puede sostener una producción por planos.
6. **`I:` está al 100 % (27 GB libres) con 5,6 TB de modelos.** No hay sitio para una producción de vídeo sin liberar o ampliar. Los duplicados reales son solo **0,18 TB**, así que no se arregla borrando duplicados.
7. **Falta un módulo, no un motor.** Todo el músculo (LTX 2.3, Wan 2.2 Animate, VACE, AnimateDiff, RIFE, upscalers, SAM3, FFmpeg 8.1.2, TTS, Whisper) ya está instalado. Lo que no existe es el **orquestador**: proyectos, escenas, planos, fichas de personaje, cola con control de VRAM, control de calidad y montaje. Eso es exactamente lo que hay que escribir dentro de KUKIA.

---

## 2. HARDWARE Y ESTADO EN VIVO

| Elemento | Valor medido |
|---|---|
| CPU | Intel Core i9-10900K @ 3,70 GHz |
| RAM | **63,9 GB** |
| GPU 0 (nvidia-smi) | RTX 5070 — 12.227 MiB totales, **11.771 usados**, **173 libres** |
| GPU 1 (nvidia-smi) | RTX 5060 Ti — 16.311 MiB totales, **14.136 usados**, **1.915 libres** |
| Driver | 621.01 en ambas |
| Qué ocupa la 5070 | `llama-server.exe` (LM Studio, PID 24124, 18,6 GB de RAM de trabajo) con un modelo cargado |
| Qué ocupa la 5060 Ti | ComfyUI (PID 17752), que retiene modelos en VRAM aunque esté en reposo |

### Discos

| Unidad | Tamaño | Libre | Uso | Relevante para |
|---|---:|---:|---:|---|
| H: | 2,8 T | **98 G** | 97 % | KUKIA_CODE, salidas del estudio |
| I: | 3,7 T | **27 G** | **100 %** | ComfyUI y sus 5,6 TB de modelos |
| J: | 932 G | 30 G | 97 % | `models\video`, `maestro_ckpts` |
| K: | — | — | — | un modelo LTX 2.5 enlazado por junction desde `models/diffusion_models` |
| D:/E:/F:/G:/L: | 2,8–7,3 T | 128–524 G | 91–94 % | descargas y respaldos |

### Consecuencia operativa

- **No se puede generar vídeo ahora mismo sin liberar VRAM.** Queda 1,9 GB.
- ComfyUI arranca con `--disable-smart-memory`, lo que significa que **no devuelve los modelos a RAM**: hay que llamar a `/free` o reiniciarlo para recuperar la tarjeta. Esto es la mitad de un «GPU Scheduler».
- **I: al 100 %** es un bloqueo duro para una producción de episodios. Un plano de 5 s a 960×544 ocupa poco, pero un capítulo completo con intermedios, máscaras, pose y reescalados no cabe.

---

## 3. COMFYUI — LO QUE HAY INSTALADO

| Dato | Valor |
|---|---|
| Versión | **0.43.0** (comfyui_version) |
| Frontend | 1.57.6 |
| Python | 3.10.20 |
| PyTorch | **2.11.0+cu128** |
| GPU expuesta al proceso | **solo** `cuda:0 = NVIDIA GeForce RTX 5060 Ti` |
| Flags de arranque | `--port 8188 --listen 127.0.0.1 --disable-auto-launch --use-pytorch-cross-attention --disable-smart-memory` |
| Packs de nodos | **53** |
| Nodos registrados | **2745** |
| Respuesta HTTP | sí, 8188 activo |

Dos detalles técnicos con consecuencias:

- `--use-pytorch-cross-attention`: **no** está usando SageAttention. Hay margen de rendimiento sin cambiar de modelo.
- `--disable-smart-memory`: modelos clavados en VRAM. Explica los 14,1 GB ocupados en reposo.

### Packs instalados relevantes (53 en total)

**LTX / vídeo:** `ComfyUI-LTXVideo`, `ComfyUI-LTX2.3-CLSS`, `ComfyUI_LTX2_SM`, `ComfyUI-H3-to-LTX-Adapter`, `ComfyUI-H3-LongTake`, `ComfyUI-H3VAE_TRT`
**Wan:** `ComfyUI-WanVideoWrapper` (con `WanVideoAnimateEmbeds`, `WanVideoAddStandInLatent`, `WanVideoVACEEncode`, `WanVideoBlockSwap`, `unianimate`), `ComfyUI-Wan-VACE-Video-Joiner`
**Identidad / cara:** `ComfyUI-IPAdapter-plus` (IPAdapterFaceID, IPAdapterInsightFaceLoader), `facefusion-comfyui`, `ComfyUI-IDV2V` (SAM3 por vídeo)
**Control:** `ComfyUI-ControlNet-Aux`, `ComfyUI-Advanced-ControlNet`, `ComfyUI-AnimateDiff-Evolved`
**Post / calidad:** `ComfyUI-Frame-Interpolation` (RIFE VFI, FILM VFI), `ComfyUI-SUPIR`, `ComfyUI-LayerDivine`, `ComfyUI-efficiency-nodes`, `ComfyUI-KJNodes`, `ComfyUI-Impact-Pack`, `ComfyUI-Inspire-Pack`, `WAS-Node-Suite`, `rgthree-comfy`
**Análisis:** `ComfyUI-Florence2` (captioning), `ComfyUI-Zero123`
**Multimotor:** `ComfyUI-MultiGPU` — aporta la familia **DisTorch2** (`CheckpointLoaderSimpleDisTorch2MultiGPU`, `CLIPLoaderDisTorch2MultiGPU`…), que reparte bloques del modelo entre varias GPU.

### Modelos en `I:\pinokio\api\comfy.git\app\models`

| Carpeta | Archivos | Tamaño |
|---|---:|---:|
| diffusion_models | 955 | **2.714,5 GB** |
| loras | 944 | **1.460,7 GB** |
| checkpoints | 52 | 541,5 GB |
| unet | 31 | 373,9 GB |
| text_encoders | 41 | 190,7 GB |
| diffusers | 107 | 95,0 GB |
| flux3 | 69 | 85,4 GB |
| clip | 15 | 52,9 GB |
| vae | 40 | 33,5 GB |
| controlnet | 12 | 28,1 GB |
| gguf | 2 | 21,8 GB |
| LLM | 25 | 16,1 GB |
| **TOTAL** | **2.339** | **5.627,5 GB** |

Comprobado: `ditto/`, `wan_animate_14B/`, `wan2.1_text2video_14B/` y `ltx25_comfy/` **no son copias**, son hardlinks (mismo inodo). El desperdicio real por duplicados es de **0,18 TB**, no de terabytes. Borrar duplicados no resuelve el problema de espacio.

---

## 4. ACTIVOS REALES POR FUNCIÓN

### 4.1 Vídeo — LTX 2.3 (el pipeline Anime2Real)

| Archivo | Ruta | Tamaño |
|---|---|---:|
| `ltx-2.3-22b-dev-fp8.safetensors` | checkpoints/ | **29,15 GB** |
| `ltx-2.3-22b-distilled-lora-384.safetensors` | loras/ | 7,61 GB |
| `ltx23_anime2real_rank64_v1_4500.safetensors` | loras/ | **0,65 GB** |
| `gemma_3_12B_it.safetensors` | text_encoders/ | 9,45 GB |

Alternativas ya presentes para bajar VRAM: GGUF `ltx-2.3-22b-dev-Q4_K_M.gguf` (14,33 GB), `ltx-2.3-22b-distilled-1.1-Q3_K_M.gguf` (14,70 GB) + `ComfyUI-GGUF` instalado. LTX **2.5** también está: Q4_K_M (15,77 GB), `nvfp4`, LoRA destilado 450, encoder Gemma 4 12B int8, upscalers espacial (1,0 GB) y temporal (0,3 GB), audio VAE y vocoder.

### 4.2 Identidad de personaje

| Activo | Estado |
|---|---|
| `ltx2\Best_FaceID_v1.0_LoRA.safetensors` (2,47 GB) | presente |
| `ltx2\Best_FaceID_CharacterSheet_v1.0_LoRA.safetensors` (1,31 GB) | presente |
| `ltx2\Best_FaceID_v1.0_ArcFace_Projector.safetensors` (0,07 GB) | presente |
| `ltx2\ltx-face-prior-f1-profile-correction-step11019.safetensors` | presente |
| `head_swap_ltx25_r128_v1.1.safetensors` (2,47 GB) | presente |
| IP-Adapter FaceID + InsightFace | nodos presentes; `insightface 0.7.3` y `onnxruntime_gpu` instalados; `buffalo_l` (0,2 GB) en `J:\models\maestro_ckpts` |
| FaceFusion | pack instalado (`facefusion-comfyui`) |

### 4.3 Control de movimiento y cámara

`ltx-2.3-22b-ic-lora-motion-track-control-ref0.5` (0,33 GB), `LTX23_IC_MotionTrack`, `union-control-ref0.5` (0,65 GB), `LTX2.3-22B_IC-LoRA-Cameraman_v2_14000`, `CrossView-Warp`, `Dual-Character`, `refocus`, `relight`, `clean-plate`, `outpaint`, `uncompress`, `lipdub` (2,47 GB). En Wan: `Wan22FunControlToVideo`, `WanCameraEmbedding`, `WanAnimate2ToVideo`, `WanAnimateToVideo`, DWPose/pose/depth/mask en `J:\models\maestro_ckpts\pose|depth|mask`.

### 4.4 Audio y voz

`LTXVAudioVAELoader` + audio VAE/vocoder de LTX 2.3, `ltx-2.3-22b-ic-lora-lipdub-0.9`. Fuera de ComfyUI, en `maestro_ckpts`: `qwen3_tts` (base/customvoice/voicedesign, bf16 e int8), `chatterbox`, `kokoro`, `index_tts2`, `ace_step_v1_5` (música), `whisper`/`whisper_medium` (ASR), `pyannote` (diarización), `roformer` (separación de stems), `bigvgan`, `mmaudio`, `rife4.26.pkl`.

### 4.5 Post y montaje

RIFE VFI y FILM VFI (interpolación), `RTX_VSR_Long_Video`, FlashVSR (restauración de vídeo largo), `ClarityCrystalVideoUpscaleNode`, SUPIR, `Image Rembg`, SAM3 vídeo (`IDV2V_SAM3VideoSegment`), Florence2. **FFmpeg 8.1.2 y ffprobe** instalados en el sistema (Gyan build vía WinGet).

---

## 5. EL PIPELINE ANIME2REAL QUE YA TIENES

`I:\...\app\user\default\workflows\LTX23_Anime2Real.json`

Estructura: 5 nodos externos + **un subgrafo de 43 nodos** llamado `LTX 2.3 - Anime2Real`. Es el workflow del Comfy Hub (autor *Purz*, id `6fec31e40f4a`, que también está guardado como `6fec31e40f4a.json`).

Configuración real que trae:

| Parámetro | Valor |
|---|---|
| Resolución de generación | **960 × 544** |
| Frames | **121** (5,04 s a 24 fps) |
| CFG | 1 |
| Sigma manual | **8 pasos** (`1.0 → 0.421`) |
| Sampler | `euler_ancestral` |
| Fuerza img2video | 0.7 |
| Preprocesado | 1280, `LTXVPreprocess` 18 |
| Audio | latente de audio 97×25 + `LTXVAudioVAEDecode` (vídeo **con** audio) |
| Decode | `VAEDecodeTiled` 512/64/512/4 |
| LoRA Anime2Real | peso **1.0** |
| LoRA destilado | peso 0.5 |

**Lo que esto significa:** no es un «filtro de estilo». Es un *video-to-video* con IC-LoRA que reescribe fotogramas manteniendo la estructura y el movimiento del original, y añade audio. Es la base correcta para «anime → acción real».

**Lo que NO está verificado:** que termine en tu hardware. Nunca se ha lanzado. La referencia externa más cercana (RTX 4090, 24 GB) consumió 23,4 GB de VRAM **más** 15,2 GB de memoria compartida y 77 GB de RAM a 1280×720. Con 16 GB y 63,9 GB de RAM el riesgo de thrashing es alto y **esto hay que medirlo antes de prometer nada**.

### Evidencia de uso real de ComfyUI

16 trabajos en historial. Todos `success` salvo uno, todos con **SDXL**: `0x7RealisticFreedom_omegaSDXL`, `Realistic_Vision_V6.0_B1`, `animagineXL40_v4Opt`, e IP-Adapter SDXL en 4 de ellos. Salidas: `kukia_anime2real_0000N_.png`, `kukia_real2anime_0000N_.png`, `kukia_ipa_a/b_00001_.png`. **0 vídeos** en `output/`.

---

## 6. WAN 2.2 / ANIMATE / VACE / STAND-IN

| Elemento | Estado verificado |
|---|---|
| ComfyUI nativo | `WanAnimateToVideo`, **`WanAnimate2ToVideo`**, `WanAnimate2Cache`, `Wan22FunControlToVideo`, `Wan22ImageToVideoLatent`, `WanCameraEmbedding` |
| Kijai wrapper | `WanVideoAnimateEmbeds`, `WanVideoAddStandInLatent`, `WanVideoVACEEncode`, `WanVideoBlockSwap`, carpeta `unianimate` |
| Pesos Animate | `wan_animate_2_bf16.safetensors` (32,79 GB), `Wan2_2-Animate-14B_fp8_e4m3fn_scaled_KJ` (17,93 GB), `wan2.2_animate_2_14B_distilled_int8_convrot` (16,65 GB), `Wan2.2-Animate-14B-Q4_K_M.gguf` (11,50 GB) |
| Pesos base Wan 2.2 | `wan2.2_text2video_14B_high_*` (9,5–14,9 GB), `wan2.2_image2video_14B_high_quanto_mbf16_int8` (14,9 GB), VAE fp32/bf16 |
| VACE | `Wan2_1-VACE_module_14B_fp8_e4m3fn` (3,05 GB), `wan2.1_Vace_1_3B_module` (1,47 GB) |
| Distiladores | `wan2.1_StepDistill-CfgDistill_14B_quanto_bf16_int8` (11,61 GB), `lightx2v 4step` (10,1 GB) |
| Otros vídeo | Hunyuan Video 1.5 (i2v 480/720, 8,5 GB), LongCat Avatar, Viggle-Animate, SCAIL 2, MiniMax H3 (+ 333 nodos H3 y LoRAs `ref2v_anime_2_realism`, `SS_FaceSwap_MiniMax_H3_REF2VA`) |

**Lectura honesta:** Wan no tiene un LoRA anime2real. Wan hace **sustitución de personaje con transferencia de movimiento**, que para «convertir un personaje dibujado en un actor» es incluso más útil que un filtro de estilo, porque conserva la actuación. Y es el único con licencia Apache 2.0.

**Caso documentado con tu misma tarjeta:** RTX 5060 Ti 16 GB + 64 GB RAM, Wan Animate GGUF **Q8**, 5 s a 1024×574 con SageAttention ≈ **7 minutos**.

---

## 7. WAN2GP Y MAESTRO

| Elemento | Estado |
|---|---|
| Wan2GP | `I:\pinokio\api\wan.git\app`, commit **3a8ced9c, 30/10/2026** («fixed low RAM crash») |
| Modelos Wan2GP | `ckpts` → **`J:\models\maestro_ckpts`** (junction), con `K_models` 1.105 GB y `llm` 152 GB |
| Soporte Animate 2 | **sí**: `models/wan/animate2.py` con KV cache del vídeo conductor |
| Entorno | env `wan2gp` con **torch 2.10.0+cu130**, `insightface 0.7.3`, `onnxruntime_gpu` |
| Maestro (7860) | **caído**. `HTTP 000`. No responde |

---

## 8. KUKIA CODE — LO QUE YA EXISTE

| Elemento | Estado |
|---|---|
| Repo | `H:\KUKIA_CODE`, último commit `35d69e6` |
| Módulos del core | agents, **anime_real**, config, gaming, gpu, logs, memory, permissions, providers, radar, research, security, vram |
| Vistas de la app (18) | agents, **anime**, benchmarks, chat, files, gaming, gpu, history, memory, models, radar, research, security, settings, tasks, terminal, tools, vram |
| Módulo Anime2Real | `core/src/anime_real.rs` (**729 líneas**) + `app/src/views/AnimeReal.tsx` (358 líneas) |
| Qué hace hoy | SDXL img2img: `anime2real` y `real2anime`, plantillas JSON, detección de capacidades, listado de modelos, imágenes de entrada, historial JSONL |
| Historial real | **1 trabajo**, `anime2real_id_sdxl`, 16,65 s, una PNG |
| Vídeo | **ninguno**. No hay cliente de vídeo, ni cola, ni control de VRAM por trabajo |

Lo reutilizable de inmediato: el **cliente HTTP de ComfyUI** (subida de imagen, encolado, sondeo de historial), el **historial JSONL**, la detección de capacidades y el patrón de vistas. Todo eso se hereda tal cual.

---

## 9. QUÉ DICE LA EVIDENCIA EXTERNA (y qué no)

Datos con fuente, no estimaciones propias:

| Dato | Fuente |
|---|---|
| Anime2Real = IC-LoRA **comunitario** de Alissonerdx sobre LTX 2.3, workflow de *Purz* en Comfy Hub, declarado **experimental** | huggingface.co/Alissonerdx/LTX-LoRAs · comfy.org/hub/workflows/ltx-2-3-anime2real-6fec31e40f4a |
| 4090 24 GB: 1280×720 ≈ 6 min, **23,4 GB VRAM + 15,2 GB compartida + 77 GB RAM** | note.com/kongo_jun |
| **No existe** LTX 2.5 anime2real confirmado | búsqueda sin resultado; los LoRAs 2.5 son Ingredients/Restore/Upscaler/Alpha-Gen |
| Wan 2.2 Animate-14B y **Animate-2** (07/08/2026): **Apache 2.0**; Wan 2.5/2.6/3.0 **NO** son open weights | github.com/Wan-Video/Wan2.2 · atlascloud.ai |
| 5060 Ti 16 GB: Wan Animate Q8 GGUF, 5 s 1024×574 ≈ **7 min** | huggingface.co/Wan-AI/Wan2.2-Animate-14B/discussions/4 |
| Block swap de Kijai: 40 bloques en el 14B; **64 GB RAM es el número de comodidad** | comfy.icu/node/WanVideoBlockSwap |
| LTX 2.3 FP8 cabe en 16 GB **solo** con el transformer aislado y el Gemma cuantizado; el pico real está en el VAE decode | ltxworkflow.com/blog/fix-cuda-out-of-memory-ltx-2-3-comfyui |
| Regresión conocida: ComfyUI reciente empeoró el offload en 12–16 GB → **fijar versión** | github.com/Comfy-Org/ComfyUI/issues/14345 |
| **Stand-In**: integración «WIP» solo en el wrapper de Kijai; la comunidad prefiere LoRA de personaje sobre I2V | github.com/kijai/ComfyUI-WanVideoWrapper/issues/1815 |

**No verificado y por tanto no afirmado:** VRAM oficial de Animate-2, licencia del LoRA anime2real, rendimiento exacto del workflow en *tu* máquina, y si DisTorch2 ayuda realmente con LTX (los loaders están instalados; su efecto no está medido).

---

## 10. LAS DOS GPUs: QUÉ ES POSIBLE Y QUÉ NO

**Lo que NO es posible:** sumar 12 + 16 = 28 GB como una sola VRAM. No hay NVLink, y ambas van por **PCIe 3.0 x8**. Ningún pipeline de estas familias reparte tensores entre dos tarjetas a velocidad útil.

**Lo que SÍ es posible, y ya está instalado:**

1. **Reparto por componentes**, que es lo que hacen de verdad los `DisTorch2MultiGPU`: el text encoder (Gemma 12B, 9,45 GB) en una tarjeta y el transformer en otra. Esto tiene impacto inmediato: hoy el Gemma compite por los mismos 16 GB que el LTX.
2. **Reparto por trabajos**: ComfyUI en la 5060 Ti, Wan2GP/LLM en la 5070. Dos colas, dos presupuestos de VRAM.
3. **Wan2GP con `--gpu 1`** ya soporta elegir tarjeta por proceso.

**Requisito previo:** ComfyUI **solo ve la 5060 Ti**. Para aprovechar la 5070 hay que reiniciarlo permitiendo ver las dos (o instanciar un segundo ComfyUI en otro puerto sobre la 5070). Eso implica **parar el servidor vivo**, y eso necesita tu autorización.

---

## 11. CLASIFICACIÓN DE LAS 15 ÁREAS DEL ENCARGO

| # | Área | Estado | Comentario |
|---|---|---|---|
| — | **Motores de conversión** | | |
| 1 | LTX 2.3 / Anime2Real / IC-LoRA | **DISPONIBLE (sin probar)** | Workflow + 4 modelos verificados. Cero ejecuciones |
| 2 | LTX I2V / V2V | **DISPONIBLE** | Nodos `LTXVImgToVideoConditionOnly`, `LTXAddVideoICLoRAGuide` |
| 3 | LTX audio + lip-sync | **DISPONIBLE** | Audio VAE, vocoder, IC-LoRA lipdub |
| 4 | Character consistency | **PARCIAL** | FaceID LoRAs y ArcFace projector presentes; sin sistema de verificación ni locking |
| 5 | Wan 2.2 Animate / Animate 2 | **DISPONIBLE** | Nodos nativos + pesos int8/GGUF |
| 6 | Wan Stand-In | **EXPERIMENTAL** | Nodo presente; integración reconocida como inmadura |
| 7 | Wan VACE | **DISPONIBLE** | Módulos 14B fp8 y 1.3B |
| 8 | Motion / pose control | **DISPONIBLE** | DWPose, pose, depth, mask, motion-track IC-LoRA |
| — | **Estudio** | | |
| 9 | Anime Character Realizer | **PARCIAL** | Existe en KUKIA como img2img SDXL. No hay ficha de personaje persistente |
| 10 | Human Identity Lock | **FALTANTE** | No hay almacén de referencias por personaje ni comparador entre planos |
| 11 | Character LoRA Studio | **FALTANTE** | Sin dataset builder, sin captions, sin entrenamiento. Florence2 disponible para captioning |
| 12 | Video → Video anime a película | **FALTANTE** | El workflow existe; no hay sistema de escenas/planos ni lotes |
| 13 | Cinematic Studio | **FALTANTE** | Cameraman IC-LoRA disponible, sin controles |
| 14 | Voice & Lip Sync Studio | **PARCIAL** | Motores listos (Qwen3-TTS, Chatterbox, Kokoro, Whisper, pyannote, roformer); sin conectores |
| 15 | Film Production / montaje | **FALTANTE** | FFmpeg presente; sin editor, sin timeline, sin export |
| 16 | Quality Control | **FALTANTE** | InsightFace + FFmpeg permiten hacerlo; no existe |
| 17 | Integración en KUKIA | **FALTANTE** | Vistas y comandos del estudio no existen |
| 18 | Investigación continua | **PARCIAL** | Hay módulo `research`; sin vigilancia de estos modelos |
| — | **Plataforma** | | |
| 19 | GPU Scheduler real | **PARCIAL** | KUKIA vigila VRAM; no decide ni libera (`/free`) |
| 20 | Almacenamiento | **BLOQUEO** | I: 27 GB libres |
| 21 | Licencias | **REQUIERE DECISIÓN** | Wan Apache 2.0; MiniMax H3 excluye UE/UK/EEUU; LTX 2.x con umbral de ingresos |

---

## 12. ARQUITECTURA PROPUESTA — DENTRO DE KUKIA, SIN OTRA APP

```
KUKIA CODE (Tauri + React + core Rust)
│
├─ core/src/studio/            ← módulo nuevo
│   ├─ mod.rs                  modelos de datos y estado
│   ├─ comfy.rs                cliente ComfyUI (reutiliza el de anime_real)
│   ├─ graph.rs                construcción de grafos: LTX A2R, Wan Animate, VACE
│   ├─ characters.rs           fichas de personaje + referencias + hashes
│   ├─ scenes.rs               proyecto → escena → plano
│   ├─ queue.rs                cola con prioridad, reintentos y cancelación
│   ├─ gpu_budget.rs           presupuesto de VRAM, /free, espera, reserva por tarjeta
│   ├─ qc.rs                   puntuación por plano (cara, continuidad, artefactos)
│   └─ render.rs               montaje y export con FFmpeg
│
├─ app/src/views/Studio.tsx    ← 1 vista nueva con pestañas
│      Proyecto · Personajes · Motores · Cola · Calidad · GPU · Investigación
│
└─ %APPDATA%\KukiaCode\studio.db   SQLite (ya hay rusqlite en el proyecto)
       proyectos, personajes, referencias, planos, jobs, qc, versiones
```

**Decisiones de diseño, y por qué:**

- **ComfyUI es el motor, no el producto.** KUKIA no reimplementa difusión: construye grafos, encola, vigila y mide. Así cada mejora de ComfyUI se hereda gratis.
- **Todo por planos, nunca un episodio de golpe.** Un plano = un job = un registro con hash, parámetros y puntuación. Es la única forma de repetir solo lo defectuoso.
- **GPU budget antes de encolar.** Preflight: consultar VRAM libre, `/free` en ComfyUI, descargar modelos de LM Studio si hace falta, y solo entonces enviar. Nada de OOM a mitad de un plano de 20 minutos.
- **Identidad como dato, no como prompt.** Cada personaje guarda referencia, embeddings y versiones; el QC compara contra la referencia y puntúa desviación. Sin eso, el mismo personaje cambia de cara en cada plano, que es el fallo clásico de estos pipelines.
- **Nada de descargas automáticas.** Los modelos se detectan; si falta uno, se avisa y se espera autorización.

---

## 13. PLAN POR FASES

| Fase | Qué | Riesgo | Toca algo externo | Sesiones |
|---|---|---|---|---|
| **LA1** | Conector ComfyUI completo + preflight de VRAM + `/free` + catálogo real de modelos del estudio | Bajo | No | 1 |
| **LA2** | Prueba de fuego medida: 1 clip Anime2Real a 960×544×121 y 1 Wan Animate Q8, con tiempos, VRAM y RAM reales | **Medio** (usa GPU) | Solo ejecuta | 1 |
| **LA3** | Proyecto → escena → plano, cola persistente, reanudar/cancelar, historial con hash | Bajo | No | 1–2 |
| **LA4** | Personajes: fichas, referencias, FaceID, comparador de identidad y QC por plano | Medio | No | 2 |
| **LA5** | Video-to-video por lotes: detectar planos del anime (FFmpeg), convertir, mantener audio original | Medio | No | 2 |
| **LA6** | Montaje y export: timeline, transiciones, mezcla de audio, 720p/1080p, FFmpeg | Bajo | No | 1–2 |
| **LA7** | Voz y lip-sync: TTS local + lipdub IC-LoRA | Medio | No | 1–2 |
| **LA8** | LoRA Studio (dataset, captions, entrenamiento) | **Alto** | Requiere descargas y horas de GPU | 2–3 |

**Total: 11–15 sesiones.** LA1–LA3 dan un estudio usable; LA4 y LA5 dan el salto de calidad; LA7–LA8 son opcionales.

**Criterio de aceptación de LA2 (el que de verdad importa):** un clip de 5 s convertido, con su tiempo medido, su pico de VRAM y su pico de RAM. Si no cabe, se baja resolución o se pasa a GGUF Q4 **con el número delante**, no con una promesa.

---

## 14. RIESGOS Y BLOQUEOS

| Riesgo | Nivel | Mitigación |
|---|---|---|
| **I: al 100 % (27 GB)** | **Bloqueante** | Liberar espacio o mover salidas a H:/D: antes de producción real |
| VRAM 16 GB para LTX 22B fp8 | **Alto** | GGUF Q4 + Gemma cuantizado + resolución 960×544; medir en LA2 |
| RAM 63,9 GB vs 77 GB del caso 4090 | **Alto** | Block swap conservador, clips cortos, cerrar LM Studio durante el render |
| Ambas GPU ocupadas en reposo | Alto | `/free` + política de reserva por tarjeta en LA1 |
| ComfyUI solo ve una GPU | Medio | Reinicio con las dos visibles (necesita autorización) |
| Regresión de offload en versiones nuevas de ComfyUI | Medio | **Fijar 0.43.0**; no actualizar sin medir antes/después |
| Stand-In inmaduro | Medio | No usarlo como base; LoRA de personaje sobre I2V |
| Licencia MiniMax H3 (excluye UE) | Medio | No usar para distribución comercial |
| Borrar modelos sin querer | **Alto** | El estudio **nunca** borra modelos; solo lee |
| Prometer calidad de película | Alto | QC por plano y tandas cortas; nada de «episodio completo» |

---

## 15. LO QUE NECESITO DE TI PARA ARRANCAR

1. **¿Autorizas LA1?** Es aditivo: módulo nuevo, vista nueva, cero cambios en ComfyUI.
2. **¿Autorizas LA2 con GPU?** Implica ejecutar 2 clips de prueba (~10–30 min de tarjeta) para tener números reales.
3. **¿Reinicio ComfyUI** para que vea las dos GPU, o prefieres un segundo ComfyUI en otro puerto solo para la 5070?
4. **¿Desbloqueamos espacio en I:** o muevo las salidas del estudio a H:/D: desde el principio?
5. **¿Qué manda: máxima calidad (LTX 22B, lento) o ritmo de producción (Wan Animate Q8, ~7 min por plano)?** Mi recomendación: **Wan Animate para el cuerpo y la actuación, LTX para el plano de lucimiento**, y decidirlo por plano.

---

## 16. ESTADO DE ESTA AUDITORÍA

- Inventario de motores: **hecho**.
- Versiones: **hecho**.
- Modelos y LoRAs del estudio: **hecho** (con rutas y tamaños).
- Nodos de ComfyUI: **hecho** (53 packs, 2745 nodos, búsqueda por función).
- Estado de Wan2GP y Maestro: **hecho** (Wan2GP vivo, Maestro caído).
- Compatibilidad LTX 2.3 Anime2Real en tu Windows: **archivos verificados; ejecución NO probada**.
- Compatibilidad Wan 2.2: **nodos y pesos verificados; ejecución NO probada**.
- Posibilidades reales de las dos GPU: **hecho** (una visible a ComfyUI; DisTorch2 disponible; sin suma de VRAM).
- Requisitos de VRAM/RAM: **documentados con fuente externa; no medidos en esta máquina**.
- Dependencias faltantes: Gemma 3 12B cuantizado (fp8/fp4), GGUF Q4 de LTX, `buffalo_l` accesible desde ComfyUI para FaceID, DWPose/SAM2 para Animate.
- Arquitectura y plan: **este documento**.

**No se ha modificado nada del entorno. No se declara probado nada que no se haya ejecutado.**
