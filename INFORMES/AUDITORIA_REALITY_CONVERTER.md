# KUKIA UNIVERSAL REALITY CONVERTER — AUDITORÍA Y DISEÑO (FASE 1, SOLO LECTURA)

**Fecha:** 1 de noviembre de 2026
**Base:** amplía `KUKIA LIVE ACTION STUDIO` (ver `AUDITORIA_LIVE_ACTION_STUDIO.md`). No es un proyecto nuevo: es el mismo hub con un registro de motores y estilos en vez de un pipeline fijo de anime.
**Modo:** lectura. Nada instalado, descargado ni modificado. Ni ComfyUI, ni Wan2GP, ni Pinokio, ni drivers han sido tocados.

---

## 1. RESUMEN EJECUTIVO

1. **La conversión bidireccional ya es posible en imágenes**, con lo instalado: Flux Kontext fp8 (edición por instrucción), Krea2 lineart edit, Qwen-Image-Edit, Flux Redux (estilo por referencia), Flux 2 Klein, SDXL + 190 LoRAs de estilo y 12 ControlNets.
2. **En vídeo, las dos direcciones están cubiertas de forma desigual.** *Cualquier cosa → real* tiene tres vías reales (LTX anime2real, Wan Animate, `ditto sim2real`). *Real → cualquier estilo* solo tiene una vía seria: **Wan VACE v2v + LoRA de estilo**, y ya tienes un workflow montado (`wan_vace_v2v.json`, 19 nodos, Wan 2.1 VACE 14B Q4 GGUF + Canny).
3. **Hallazgo que cambia el encargo: ya has hecho pruebas de sim2real.** `ditto_sim2real_comfy.safetensors` (6,10 GB), `Wan21_14B_VACE_lora_ditto_sim2real_bf16` y el workflow `ditto_dbz.json` existen. Eso es exactamente «videojuego → acción real», el punto 4 de tu encargo.
4. **Falta un solo LoRA para cerrar la dirección inversa**: `ltx23_real2anime_rank64_v1_5000.safetensors` (654 MB, Alissonerdx), el espejo del anime2real. **No está en disco.** Es la descarga más rentable de todo el proyecto.
5. **La detección automática de estilo es viable offline hoy**: `qwen3-vl:4b`, `qwen3-vl-ablit`, `llava:34b` y `moondream` en Ollama, más Florence2 en ComfyUI.
6. **Identidad universal tiene un agujero honesto**: FaceID/ArcFace funciona en caras humanas. Para criaturas, animales, personajes pixel art o avatares 3D **no hay equivalente**: hay que resolverlo con referencia visual (IP-Adapter/Redux/FaceID LoRA) y aceptar que la verificación automática no existe.
7. **El cuello de botella sigue siendo disco y VRAM**, no algoritmos: `I:` con 27 GB libres y 16 GB de tarjeta. Un episodio de 24 min en planos de 5 s son ~288 planos; solo los fotogramas intermedios en PNG a 960×544 son **~17 GB por episodio** (hasta 4× si subes a 1080p).

---

## 2. TAXONOMÍA: QUÉ ESTILOS TIENEN MOTOR REAL

Clasificación por evidencia en disco. **No se marca nada como disponible sin un archivo o nodo que lo respalde.**

### 2.1 Orígenes reconocibles automáticamente

| Origen | Detección | Evidencia |
|---|---|---|
| Anime / manga / 2D cel | sí | `qwen3-vl:4b`, `llava:34b`, Florence2 |
| Cartoon occidental | sí | mismos VLM |
| 3D / CGI / Pixar-like | sí | mismos VLM |
| Pixel art / retro | sí | mismos VLM (clase visual clara) |
| Cómic / novela gráfica | sí | mismos VLM |
| Fotografía / vídeo real | sí | mismos VLM |
| Gameplay / cinemática | sí | mismos VLM + heurística de HUD |

La detección es **clasificación por texto**, no un modelo entrenado para ello. Es suficiente para enrutar, no para certificar. Se debe mostrar el estilo detectado **con su confianza y permitir corregirlo a mano**.

### 2.2 Destinos con motor presente

| Destino | Motor real | Estado |
|---|---|---|
| **Acción real / fotorrealismo (vídeo)** | LTX 2.3 + `ltx23_anime2real_rank64` · Wan Animate · `ditto sim2real` | DISPONIBLE, **sin probar** |
| **Acción real (imagen)** | SDXL Realistic Vision / 0x7RealisticFreedom / ultrarealFineTune · Flux + skin LoRAs | **FUNCIONA** (probado en KUKIA) |
| **Anime (imagen)** | animagineXL v4Opt · animePastelDream · noobaiXL | **FUNCIONA** (probado en KUKIA) |
| **Anime (vídeo)** | `LTXANIME_Text-Video`, `H3_Anime_2D_Style`, `anime_anime2d_h3_v1` (t2v, no v2v) | PARCIAL |
| **Cartoon / cómic** | `comic book page style v2.1`, lineart Krea2/SDXL | PARCIAL (imagen) |
| **3D / CGI** | `3DREAL-strong-v2`, Krea2, Claymation H3 | PARCIAL / EXPERIMENTAL |
| **Pixel art** | `pixeltune_h3_full_canvas4_v3`, `Golden_Rich_Retro_Bodies` | EXPERIMENTAL |
| **Estilo libre por referencia** | Flux Redux (`flux1-redux-dev`) + IP-Adapter | DISPONIBLE (imagen) |
| **Estilo por instrucción de texto** | Flux Kontext fp8 (11,90 GB) · Qwen-Image-Edit | DISPONIBLE (imagen) |
| **Vídeo → vídeo con estructura preservada** | Wan 2.1 VACE 14B (GGUF Q4) + Canny/Depth/Pose | **Workflow ya montado** |
| **Vídeo → vídeo manteniendo composición** | LTX `union-control-ref0.5`, `motion-track-control` | DISPONIBLE |
| **Estilo por referencia en vídeo** | MiniMax H3 ref2v (+ `H3_Ref2V_Anime_Slider`) | EXPERIMENTAL (licencia excluye UE) |

### 2.3 Lo que NO hay

| Falta | Impacto | Coste de resolverlo |
|---|---|---|
| `ltx23_real2anime_rank64_v1_5000` (654 MB) | Sin él, «real → anime en vídeo» depende de VACE + LoRA de imagen, con peor estabilidad temporal | **1 descarga pequeña** |
| LoRA específico cartoon→real | Cartoon→real se hace con el anime2real, **sin garantía** | prueba primero, luego decidir |
| LoRA específico CGI/3D→real | ídem | prueba primero |
| `gemma_3_12B_it` cuantizado (fp8/fp4) | Es la clave para que LTX 22B quepa en 16 GB | ~5–9 GB |
| GGUF Q4 de LTX 2.3 (ya existe: 14,33 GB) | — | ya está |
| DWPose y SAM2 accesibles desde ComfyUI | Requisito de Wan Animate (pose + máscara) | ~1–2 GB |
| `buffalo_l` en la ruta de ComfyUI | FaceID/verificación de identidad | ya está en J:, hay que enlazarlo (sin mover) |
| Modelo de embeddings de estilo | Comparar «¿se parece al estilo X?» de forma medible | requiere decisión (CLIP ya presente) |

---

## 3. MATRIZ DE LAS 10 CONVERSIONES QUE PIDES

Estado según lo que hay instalado, no según lo que sería deseable.

| # | Conversión | Motor propuesto | Estado | Riesgo principal |
|---|---|---|---|---|
| 1 | **Anime → acción real** | LTX 2.3 + anime2real IC-LoRA (960×544×121) | **DISPONIBLE sin probar** | 16 GB VRAM + 64 GB RAM; el caso 4090 pidió 77 GB de RAM |
| 2 | **Cartoon → acción real** | mismo motor, prompt de estilo cartoon | **REQUIERE PRUEBA** | el LoRA se entrenó con anime; cartoon puede derivar a «plástico» |
| 3 | **CGI/Pixar → acción real** | LTX + anime2real, o Wan Animate sustituyendo personaje | **REQUIERE PRUEBA** | identidad de personaje 3D → humano es el caso más difícil |
| 4 | **Videojuego → acción real** | `ditto_sim2real` + Wan VACE (workflow `ditto_dbz.json` ya existe) | **PARCIAL** (piezas montadas, sin ejecución registrada) | HUD, UI y texto en pantalla se convierten en basura; hay que enmascararlos |
| 5 | **Dibujo/boceto → acción real** | LTX anime2real, o imagen primero (SDXL img2img) y luego I2V | **REQUIERE PRUEBA** | el boceto no tiene color ni luz: hay que inventar demasiado |
| 6 | **Real → anime (vídeo)** | Wan VACE v2v + `LTXANIME`/H3 anime LoRA; idealmente `ltx23_real2anime` | **PARCIAL / FALTA LoRA** | sin el real2anime de LTX, la coherencia temporal baja mucho |
| 7 | **Real → cartoon (vídeo)** | Wan VACE v2v + comic/lineart LoRA | **EXPERIMENTAL** | LoRAs de imagen aplicadas a vídeo suelen parpadear |
| 8 | **Real → videojuego (vídeo)** | VACE v2v + estética retro/pixel; o inverso de sim2real | **EXPERIMENTAL** | no existe modelo entrenado para esta dirección |
| 9 | **Anime → cartoon** | LTX union-control IC-LoRA + LoRA cartoon (imagen) | **EXPERIMENTAL** | cambio de estilo sin cambio de estructura: poco margen de control |
| 10 | **Cartoon → anime** | mismo que 9 | **EXPERIMENTAL** | ídem |

**Traducción honesta:** de las 10, **1 está lista**, **5 requieren una prueba** para saber si están listas, y **4 son experimentales de verdad**. Cualquier producto que prometa las 10 con calidad cinematográfica está mintiendo.

---

## 4. IDENTITY LOCK UNIVERSAL — DÓNDE ESTÁ EL LÍMITE

| Tipo de sujeto | Método real | Verificación automática |
|---|---|---|
| Persona real | FaceID LoRA / IP-Adapter FaceID / ArcFace | **sí** (embeddings + distancia) |
| Personaje de anime | referencia + `Best_FaceID` + IP-Adapter | parcial (cara dibujada → embedding ruidoso) |
| Cartoon | referencia visual (IP-Adapter/Redux) | **no** fiable |
| Personaje de videojuego | referencia + máscara (SAM3/rembg) | **no** |
| Criatura / animal | referencia visual | **no** |
| Avatar 3D / Metahuman | referencia + ControlNet | **no** |

**Diseño correcto:** tres niveles de bloqueo, y decirlo en la interfaz.
- **Nivel 1 (fuerte):** persona humana → FaceID + métrica de distancia facial por plano.
- **Nivel 2 (medio):** personaje 2D con cara consistente → FaceID LoRA + verificación por similitud CLIP.
- **Nivel 3 (débil):** criatura/objeto → solo referencia visual y revisión humana. El sistema **no** puntúa identidad que no puede medir.

Ese tercer nivel es la diferencia entre un estudio y un juguete: no se inventa una nota de identidad que no existe.

---

## 5. MOTION LOCK — QUÉ SE PUEDE PROMETER

| Técnica | Preserva | Evidencia instalada |
|---|---|---|
| Wan VACE v2v con Canny/Depth | estructura y silueta | workflow `wan_vace_v2v.json` (19 nodos) |
| Wan Animate (pose + face) | actuación, gestos, expresión | `WanAnimateToVideo`, `WanAnimate2ToVideo`, DWPose |
| LTX IC-LoRA motion-track | trayectoria de movimiento | `ltx-2.3-22b-ic-lora-motion-track-control-ref0.5` |
| LTX union-control | composición y profundidad | `union-control-ref0.5` |
| Cameraman IC-LoRA | movimiento de cámara | `LTX2.3-22B_IC-LoRA-Cameraman_v2_14000` |
| RIFE / FILM | fluidez (no movimiento nuevo) | `RIFE VFI`, `FILM VFI` |

**Límite honesto:** ningún motor conserva el movimiento *exacto* fotograma a fotograma cuando cambia el estilo 2D→real; conserva **estructura y trayectoria**, y la cara se regenera. Cuando el original tiene animación limitada (anime a 8 fps con repetición de celdas), el resultado realista necesita **interpolar y reinterpretar**, y ahí es donde el sistema debe avisar en vez de fingir fidelidad.

---

## 6. ARQUITECTURA — REGISTRO DE MOTORES Y PAQUETES DE ESTILO

```
core/src/studio/
├─ hub.rs            registro de motores con diagnóstico de compatibilidad
├─ registry/
│   ├─ comfy_ltx.rs        LTX 2.3/2.5: I2V, V2V, IC-LoRA, audio
│   ├─ comfy_wan.rs        Wan 2.2: VACE, Animate, sim2real, block swap
│   ├─ comfy_image.rs      SDXL/Flux/Kontext/Redux/Qwen-Edit (imagen)
│   ├─ wan2gp.rs           motor secundario (J:, --gpu 1)
│   └─ maestro.rs          motor opcional (hoy caído, 7860)
├─ styles.rs         catálogo de estilos con: nombre, motor, plantilla, LoRAs, fuerza, límites
├─ detect.rs         detección de estilo de entrada (Ollama VLM + Florence2) con confianza
├─ identity.rs       perfiles universales + nivel de bloqueo 1/2/3
├─ motion.rs         selección de técnica de movimiento por material
├─ graph.rs          construcción de grafos por estilo (plantillas versionadas)
├─ queue.rs · gpu_budget.rs · qc.rs · render.rs   (heredados del Studio)
```

**Un «paquete de estilo» es un dato, no código:**

```json
{
  "id": "anime_to_photoreal",
  "origen": ["anime", "manga", "2d_cel"],
  "destino": "photoreal",
  "motor": "comfy_ltx",
  "plantilla": "ltx23_anime2real_v1",
  "modelos": {
    "checkpoint": "ltx-2.3-22b-dev-fp8.safetensors",
    "lora_estilo": "ltx23_anime2real_rank64_v1_4500.safetensors",
    "lora_destilado": "ltx-2.3-22b-distilled-lora-384.safetensors",
    "text_encoder": "gemma_3_12B_it.safetensors"
  },
  "resolucion": [960, 544], "frames": 121, "pasos": 8, "fuerza_estructura": 0.7,
  "identidad": "faceid_opcional", "movimiento": "ic_lora_track",
  "limites": { "vram_min_gb": 16, "ram_min_gb": 64, "verificado": false },
  "licencia": "comunidad — revisar antes de uso comercial"
}
```

Añadir un estilo nuevo = añadir un JSON y, si hace falta, una plantilla de grafo. **No se toca el núcleo.**

### Flujo universal (una sola ruta para todo)

1. **Ingesta** → identificar tipo (imagen, clip, episodio, captura de juego).
2. **Sondeo** (ffprobe) → resolución, fps, duración, audio.
3. **Detección de estilo** → VLM, con confianza, editable.
4. **Segmentación** → FFmpeg scene detect + detección de cortes; un plano = un job.
5. **Enrutado** → el registro elige motor y paquete según origen, destino y presupuesto de VRAM.
6. **Identidad** → extraer o cargar perfil de personaje; decidir nivel 1/2/3.
7. **Movimiento** → elegir técnica según material (pose disponible, cámara, texto en pantalla).
8. **Preflight de GPU** → `/free`, VRAM libre, cerrar modelos de LM Studio si hace falta.
9. **Ejecución** → grafo enviado a ComfyUI, con hash de parámetros.
10. **QC** → identidad, continuidad, artefactos, desincronía; nota por plano.
11. **Repetición selectiva** → solo los planos por debajo del umbral.
12. **Montaje** → FFmpeg: concatenar, audio original o nuevo, transiciones, export.

### Base de datos (SQLite, `%APPDATA%\KukiaCode\studio.db`)

`projects` · `sources` · `scenes` · `shots` · `characters` · `character_refs` · `entities` · `style_packs` · `engines` · `jobs` · `job_params` (hash) · `qc_scores` · `renders` · `versions`

Cada job guarda **hash de parámetros + versión de plantilla + versión de ComfyUI**: sin eso, comparar dos resultados es imposible.

---

## 7. PANEL — KUKIA UNIVERSAL REALITY CONVERTER

Una vista nueva, dos modos, los campos que pediste y ninguno decorativo:

**Modo automático:** arrastras material, el sistema detecta estilo, propone destino y motor, muestra el coste estimado (VRAM, tiempo, disco) y arranca.

**Modo avanzado:**

| Bloque | Campos |
|---|---|
| Origen | archivo/carpeta, tipo, estilo detectado + confianza (editable), fps, duración |
| Destino | estilo, paquete, fuerza de estilo, fuerza de estructura, prompt negativo |
| Identidad | personaje, referencias (1–9), nivel de bloqueo, verificación on/off |
| Movimiento | técnica (VACE/Animate/IC-LoRA/ninguna), pose, profundidad, cámara |
| Salida | resolución, fps, formato, codec, audio (original / TTS / silencio) |
| Motor | ComfyUI / Wan2GP / Maestro, GPU asignada, presupuesto de VRAM, offload |
| Cola | trabajos, progreso, tiempos, cancelar, pausar, reintentar |
| Calidad | nota por plano, motivo, antes/después, regenerar solo el fallo |

Extra obligatorio: **«qué no puedo hacer con este material»**. Si el origen es pixel art de 320×240, el sistema lo dice antes de gastar 20 minutos: no hay información para un primer plano realista. Eso es lo que separa una herramienta de una promesa.

---

## 8. CÓMO SE PRUEBA (matriz de aceptación)

Mismo protocolo para cada par: clip de 5 s, 960×544 (o 480p en los baratos), semilla fija, sin edición manual.

| Prueba | Material | Métricas |
|---|---|---|
| T1 anime→real | clip 2D limpio | tiempo, pico VRAM, pico RAM, nota identidad, artefactos |
| T2 cartoon→real | clip cartoon | ídem + deriva a plástico (sí/no) |
| T3 CGI→real | cinemática 3D | ídem + conservación de vestuario |
| T4 juego→real | gameplay con HUD | ídem + HUD eliminado (máscara) |
| T5 dibujo→real | boceto | ídem + coherencia de color |
| T6 real→anime | clip real | ídem + parpadeo temporal |
| T7 real→cartoon | clip real | ídem + estabilidad de línea |
| T8 real→juego | clip real | ídem |
| T9 anime→cartoon | clip 2D | ídem |
| T10 cartoon→anime | clip cartoon | ídem |

**Umbral de aceptación propuesto:** un par se declara DISPONIBLE solo si (a) termina sin OOM en ≤ 30 min por 5 s, (b) no cambia la identidad entre planos del mismo personaje, (c) un humano lo aprueba a simple vista. Si no, se queda en EXPERIMENTAL con la nota puesta.

**Coste de ejecutar la matriz completa:** 10 clips × 7–30 min ≈ **2–5 horas de GPU**, más ~5 GB de disco. Requiere autorización.

---

## 9. HUECOS, DESCARGAS Y BLOQUEOS

| Necesidad | Tamaño | Estado |
|---|---|---|
| `ltx23_real2anime_rank64_v1_5000` | 654 MB | **no está** — cierra real→anime en vídeo |
| Gemma 3 12B fp8 / fp4 | 5–9 GB | no está (solo bf16 9,45 GB) |
| DWPose + SAM2 para ComfyUI | 1–2 GB | no verificado en la ruta de ComfyUI |
| `buffalo_l` enlazado a ComfyUI | 0 (ya está en J:) | requiere enlace, no copia |
| Espacio: `I:` | **27 GB libres** | **bloqueo** para producción por episodios |
| Espacio: salidas | 17–70 GB/episodio | mover a H:/D: |

Sin descargas nuevas, el sistema puede demostrar **7 de las 10** conversiones. Con dos descargas pequeñas (≈1,5 GB) sube a **9**. La décima (real→videojuego) no tiene modelo y probablemente no lo tendrá.

---

## 10. PLAN

Solapa con LA1–LA8 para no duplicar trabajo.

| Fase | Qué | Depende de | Sesiones |
|---|---|---|---|
| **RC1** | Registro de motores + catálogo de estilos + diagnóstico de compatibilidad (sin ejecutar nada) | LA1 | 1 |
| **RC2** | Detección de estilo con VLM + segmentación de planos con FFmpeg + preflight | RC1 | 1 |
| **RC3** | Paquetes de estilo de imagen (real→anime y vuelta, cartoon, 3D, pixel) — la vía barata y rápida | RC1 | 1–2 |
| **RC4** | Vídeo: VACE v2v real→X y LTX X→real, con la matriz T1–T5 | LA2, RC2 | 2 |
| **RC5** | Identidad universal (3 niveles) + QC por plano + repetición selectiva | LA4 | 2 |
| **RC6** | Matriz completa T1–T10, informe de resultados y umbrales | RC4, RC5 | 1–2 |
| **RC7** | Estudio de personaje 3D/juego (máscaras de HUD, escenarios) | RC5 | 1–2 |

**Total: 9–12 sesiones añadidas al plan anterior.** RC3 da resultados visibles el primer día porque la vía imagen ya funciona; RC4 es donde se decide si el vídeo es una promesa o una realidad.

---

## 11. RIESGOS

| Riesgo | Nivel | Mitigación |
|---|---|---|
| Prometer las 10 conversiones y entregar 3 | **Alto** | Matriz pública con estado real por par; nada se marca disponible sin pasar T-n |
| `I:` al 100 % | **Alto** | Salidas fuera de I:, limpieza de intermedios automática |
| 16 GB VRAM vs 22B | Alto | GGUF Q4 + Gemma cuantizado + 960×544; medir antes de escalar |
| 64 GB RAM vs 77 GB del caso 4090 | Alto | Block swap, clips cortos, cerrar LM Studio |
| Parpadeo temporal en estilos con LoRA de imagen | Medio | QC temporal (diferencia entre fotogramas) y regeneración por plano |
| HUD/texto de videojuegos convertido en basura | Medio | Máscara SAM3 + inpainting antes de convertir |
| Licencias (MiniMax excluye UE; LTX con umbral de ingresos) | Medio | Marcar licencia por paquete de estilo |
| Identidad no verificable en criaturas | Medio | Nivel 3 explícito, sin nota ficticia |
| Regresión de ComfyUI | Medio | Fijar 0.43.0 y registrar versión por job |

---

## 12. LO QUE NECESITO DE TI

1. **¿Autorizo RC1 + RC2?** Son aditivos: registro de motores, catálogo de estilos, detección con VLM y segmentación. No ejecutan difusión, no tocan ComfyUI.
2. **¿Autorizo RC3** (imagen bidireccional, resultados visibles rápido)?
3. **¿Descargo el `ltx23_real2anime` (654 MB)** y el Gemma cuantizado para cerrar real→anime en vídeo?
4. **¿Ejecuto la matriz T1–T10** cuando esté montado? Son 2–5 h de GPU y ~5 GB.
5. **¿Muevo las salidas del estudio fuera de `I:`** antes de cualquier producción?

Mi recomendación: **RC1 → RC2 → RC3** (rápido y visible), y en paralelo autorizar los 654 MB del real2anime, que es la mejor relación resultado/riesgo de todo el proyecto.

---

## 13. ESTADO DE ESTA FASE

- Inventario de estilos con motor real: **hecho** (evidencia por archivo).
- Matriz de las 10 conversiones: **hecha**, con estado y riesgo por par.
- Detección automática de estilo: **verificada como posible** (VLM locales presentes), no implementada.
- Identity lock universal: **analizado**, con límites explícitos.
- Motion lock: **analizado** con técnicas presentes.
- Arquitectura, BD y panel: **diseñados**.
- Ejecución: **cero**. Ningún par probado todavía.
- Cambios en el entorno: **ninguno**.
