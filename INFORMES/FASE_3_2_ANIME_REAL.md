# FASE 3.2 — KUKIA ANIME ↔ REAL STUDIO

**Fecha:** 01/11/2026
**Estado:** entregado y verificado contra ComfyUI real.
**Proyecto:** `H:\KUKIA_CODE` (el existente, sin crear nada nuevo).

---

## 1. Qué se ha construido

Un módulo de conversión bidireccional anime ↔ real conectado a un motor **real**:

- **Núcleo Rust** (`core/src/anime_real.rs`): cliente de ComfyUI (estado, capacidades,
  conversión, descarga de resultados, historial).
- **Plantillas** (`workflows/anime_real/`): tres workflows en formato API probados + dos
  declarados como no convertidos.
- **Comandos Tauri**: `anime_status`, `anime_templates`, `anime_capabilities`, `anime_inputs`,
  `anime_output_path`, `anime_history`, `anime_convert`.
- **Vista** "Anime ↔ Real" en la UI: origen→destino, motor, imagen, estilo, negativo, fuerza,
  semilla, tamaño, comparación antes/después, tabla de capacidades reales e historial.

Cada conversión queda registrada en `audit_log` (agente `studio`) y en
`output/anime_real/historial.jsonl`.

## 2. Números medidos (no estimados)

Entorno: ComfyUI **0.43.0**, RTX **5060 Ti 16 GB** (cuda:0), torch **2.11.0+cu128**,
16.283 MB de VRAM (14.440–15.108 MB libres), 1024×1024, 28 pasos.

| Conversión | Motor | Tiempo | Resultado |
|---|---|---|---|
| Anime → Real (identidad) | SDXL + IPAdapter ViT-H 0,40 | **16,5 s** | PNG 1.452–1.487 KB |
| Anime → Real (libre) | SDXL img2img denoise 0,58 | **14,0 s** | PNG 1.024 KB |
| Anime → Real (estructura) | SDXL img2img denoise 0,42 | **13,5 s** | PNG 1.244 KB |
| Real → Anime | Animagine XL 4.0 denoise 0,55 | **20,1 s** | PNG 1.128 KB |
| Conversión por la ruta Rust | idéntico | **16,7 s** | 1 archivo, historial escrito |

Capacidades detectadas en vivo: **2.745** clases de nodo, **61** checkpoints, **660** LoRAs,
**96** GGUF, **20** imágenes de entrada visibles.

## 3. Qué funciona de verdad (comprobado a ojo, no solo "hay archivo")

**Real → Anime:** excelente. Una foto fotorrealista de Goku se convierte en un Goku anime con
el mismo personaje, mismo traje, misma pose y misma expresión. La identidad se conserva.

**Anime → Real con IPAdapter 0,40:** bueno. El Goku anime sale como persona fotográfica
conservando la **silueta puntiaguda del pelo**, el traje naranja con camiseta azul y el encuadre;
la piel tiene poros y pecas reales.

## 4. Qué falló y se corrigió (medido)

| Intento | Resultado |
|---|---|
| IPAdapter peso **0,85** + denoise 0,62 | **Roto**: figura deformada, contornos tipo cel partido (40,0 s tirados) |
| IPAdapter peso **0,40** + denoise 0,55 | Correcto — es el que quedó por defecto |
| IPAdapter peso 0,25 | Correcto pero conserva menos el personaje |
| Checkpoint `0x7RealisticFreedom` | **Roto**: solo ruido |
| Checkpoint `animagineXL40` | **Roto**: 0,21 GB, descarga a medias → se usa `animagineXL40_v4Opt` |

## 5. Límites reales (sin adornos)

1. **La identidad facial no está bloqueada.** Con la plantilla libre, un personaje masculino
   puede salir como mujer: el checkpoint realista empuja a retrato femenino. Con IPAdapter se
   conserva silueta, ropa y encuadre, pero la cara se reinterpreta.
2. **No hay PuLID ni InstantID instalados**, que son los que fijan rostro de verdad. Existe
   IPAdapter FaceID con insightface, **sin probar**.
3. **Vídeo todavía no.** Ditto/VACE y LTX 2.3 Anime2Real existen como grafo en ComfyUI/WanGP,
   pero no están convertidos a formato API. Se listan como `sin convertir` y la app **no** las
   ofrece: no se ha inventado un botón que no funciona.
4. **Un solo worker** (GPU1). No hay reparto entre las dos GPUs ni cola con prioridades.
5. La UI no permite aún arrastrar un archivo propio: elige entre las imágenes que ComfyUI ya
   tiene en su carpeta `input`. Añadir subida es el siguiente paso natural.

## 6. Verificación ejecutada

```
cargo test -p kukia-core                      → 4 unit nuevos + resto del núcleo: OK
cargo test -p kukia-core --test validacion_anime_real -- --ignored
                                              → 3/3 OK (conversión real, capacidades, rechazo)
cd app && npm run build                       → tsc --noEmit + vite: OK (48 módulos, 284 kB)
```

El test de integración **encontró un fallo real**: la comprobación de modelos no miraba la lista
de IPAdapter, así que la plantilla con identidad salía como no disponible aunque el modelo
estaba instalado. Corregido y vuelto a pasar en verde.

## 7. Pendiente, por orden de valor

1. **Subida de imagen propia** desde la UI (copiar a `input` de ComfyUI con el nombre elegido).
2. **PuLID / InstantID** para bloquear rostro; medir contra IPAdapter 0,40 con el mismo par.
3. **Convertir Ditto/VACE a formato API** y probar vídeo anime→real (primer vídeo real).
4. **Cola con progreso** por WebSocket de ComfyUI en vez de sondeo cada 1,5 s.
5. Reparto entre las dos GPUs para tandas largas.

## 8. Propuesta de apunte para el CEREBRO

> **01/11/2026 — KUKIA CODE F3.2 · ANIME ↔ REAL STUDIO.** Módulo en `H:\KUKIA_CODE`
> (`core/src/anime_real.rs` + `workflows/anime_real/` + vista `AnimeReal.tsx`). Cliente real de
> ComfyUI con comprobación de modelos antes de ofrecer cada conversión. Medido en RTX 5060 Ti:
> anime→real con IPAdapter 0,40 = 16,5 s; sin IPAdapter = 14,0 s; real→anime = 20,1 s (1024²,
> 28 pasos, ComfyUI 0.43.0). Cierto: real→anime conserva identidad; anime→real conserva silueta,
> ropa y encuadre, pero la cara se reinterpreta (no hay PuLID/InstantID). IPAdapter a 0,85 rompe
> la figura. Vídeo (Ditto, LTX 2.3) aún sin convertir a API. Modelos rotos detectados:
> `0x7RealisticFreedom` y `animagineXL40` incompleto; los buenos son `Realistic_Vision_V6.0_B1`
> y `animagineXL40_v4Opt`.
