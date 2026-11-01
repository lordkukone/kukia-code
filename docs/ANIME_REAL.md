# KUKIA ANIME ↔ REAL STUDIO

> Fase 3.2 · 01/11/2026 · verificado contra ComfyUI real, no simulado.

## Qué es

Módulo de conversión bidireccional entre estética anime y realismo fotográfico, dentro de
KUKIA CODE. Cada botón encola un workflow real en ComfyUI, espera el resultado y lo descarga.

No hay ninguna interfaz ficticia: si un motor no está probado, aparece como
`sin convertir` o `faltan modelos` con la lista exacta de lo que falta.

## Piezas

| Capa | Archivo | Responsabilidad |
|---|---|---|
| Núcleo | [`core/src/anime_real.rs`](../core/src/anime_real.rs) | Cliente ComfyUI: estado, capacidades, conversión, historial |
| Plantillas | [`workflows/anime_real/`](../workflows/anime_real) | Workflows en formato API + `manifest.json` |
| Comandos | [`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) | `anime_status`, `anime_templates`, `anime_capabilities`, `anime_inputs`, `anime_output_path`, `anime_history`, `anime_convert` |
| UI | [`app/src/views/AnimeReal.tsx`](../app/src/views/AnimeReal.tsx) | Vista "Anime ↔ Real": origen/destino, parámetros, antes/después, capacidades, historial |
| Pruebas | [`core/tests/validacion_anime_real.rs`](../core/tests/validacion_anime_real.rs) | Conversión real de punta a punta (ignoradas por defecto) |
| Salida | `H:\KUKIA_CODE\output\anime_real\` | PNG del resultado + `historial.jsonl` |

## Plantillas verificadas

| id | Dirección | Motor | Medido | Nota |
|---|---|---|---|---|
| `anime2real_id_sdxl` | Anime → Real | SDXL img2img + IPAdapter ViT-H (peso 0,40) | **16,5 s** | La que mejor conserva el personaje |
| `anime2real_sdxl` | Anime → Real | SDXL img2img (denoise 0,58) | **14,0 s** | Más libre: cambia más la cara |
| `real2anime_sdxl` | Real → Anime | Animagine XL 4.0 img2img (denoise 0,55) | **20,1 s** | Conserva identidad y traje muy bien |

Medidas en RTX 5060 Ti 16 GB, ComfyUI 0.43.0, torch 2.11.0+cu128, 1024×1024, 28 pasos.

## Cómo decide qué se puede hacer

1. Pide `/object_info` a ComfyUI y extrae la lista real de checkpoints, LoRAs, unets, VAEs,
   clip vision e IPAdapters.
2. Para cada plantilla del manifest comprueba que **todos** sus `requiere` están presentes.
3. Si falta alguno, la plantilla sale `disponible: false` con la lista de lo que falta y el
   selector de la UI la deshabilita.

## Ajustes que importan

- **Fuerza (denoise)**: baja = conserva estructura y parecido; alta = más estilo y más cambio.
  La diferencia medida en el mismo personaje: 0,58 reescribe la cara; 0,42 mantiene el pelo y
  el traje casi exactos.
- **IPAdapter**: copia identidad y composición. A 0,85 se pelea con el denoise y rompe la
  figura; a **0,40** es donde se midió el mejor equilibrio.
- **Semilla**: fija la misma para comparar variantes sin que cambie el resto.

## Límites conocidos (medidos, no supuestos)

- El img2img por sí solo **no garantiza** conservar la identidad facial: con la plantilla libre
  un Goku masculino puede salir como mujer. Con IPAdapter se conserva silueta, ropa y encuadre,
  pero el rostro se reinterpreta.
- Para bloquear rostro de forma estricta faltaría PuLID o InstantID, que **no están instalados**
  (solo hay `insightface` a través de IPAdapter FaceID, sin comprobar).
- Las conversiones de **vídeo** (Ditto/VACE y LTX 2.3 Anime2Real) existen como grafo en ComfyUI
  y WanGP, pero no están convertidas a formato API: se listan como `sin convertir` y no se ofrecen.
- Todo corre en un solo worker de ComfyUI (GPU1). No hay distribución entre las dos GPUs.

## Riesgo de modelos rotos

Comprobado en esta máquina: `0x7RealisticFreedom` y `animagineXL40` (sin `_v4Opt`) producen
ruido o fallan al detectar el tipo de modelo. Las plantillas usan los que sí funcionan
(`Realistic_Vision_V6.0_B1`, `animagineXL40_v4Opt`). Si una plantilla empieza a devolver basura,
lo primero es revisar el tamaño del checkpoint en `models/checkpoints`.

## Uso

```bash
# ComfyUI arriba en 8188 (GPU1)
cd H:\KUKIA_CODE\app && npm run dev        # UI → vista "Anime ↔ Real"

# Prueba real del núcleo, sin UI
cd H:\KUKIA_CODE
cargo test -p kukia-core --test validacion_anime_real -- --ignored --nocapture

# Encolar un workflow a mano
python tests/comfy_verificar_workflow.py workflows/anime_real/real2anime_sdxl.json refer.jpeg
```

## Añadir una plantilla

1. Exporta el grafo desde ComfyUI con **Save (API Format)**.
2. Guárdalo en `workflows/anime_real/`.
3. Añade su entrada en `manifest.json` con los ids de nodo que la app debe poder tocar
   (`imagen`, `positivo`, `negativo`, `muestreo`, `escalado`) y todo lo que `requiere`.
4. `cargo test -p kukia-core anime_real` avisa si un id de nodo no existe en el archivo.
5. Prueba con `comfy_verificar_workflow.py` y apunta los segundos reales en `verificado`.
