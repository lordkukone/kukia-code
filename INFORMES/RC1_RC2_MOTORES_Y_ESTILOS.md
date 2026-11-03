# RC1 + RC2 — Registro de motores, catálogo de estilos, detección y segmentación

Fecha: 2026-11-01 · Base: `AUDITORIA_REALITY_CONVERTER.md`
Alcance autorizado: **RC1** (registro + catálogo + diagnóstico) y **RC2** (detección VLM + planos + preflight).
**No se ha ejecutado difusión.** No se ha tocado ComfyUI, ni drivers, ni configuraciones.

---

## 1. Qué hay ahora

| Pieza | Dónde | Qué hace |
|---|---|---|
| Registro de motores | `core/src/studio/engines.rs` | Sondea ComfyUI, Ollama, Wan2GP y Maestro; **capacidades comprobadas nodo a nodo**, no declaradas |
| Presupuesto | `core/src/studio/engines.rs` | VRAM libre por tarjeta y RAM disponible, en el momento de preguntar |
| Catálogo de estilos | `workflows/studio/styles.json` | 13 paquetes como **dato**: origen, destino, modelos, resolución, frames, pasos, VRAM/RAM mínimas, licencia y límites |
| Diagnóstico | `core/src/studio/styles.rs` | Cruza el catálogo con los modelos que el motor ve y con la memoria libre → estado real y veredicto |
| Sondeo | `core/src/studio/detect.rs` | ffprobe: tipo, resolución, fps, duración, audio, códec, bytes |
| Detección de estilo | `core/src/studio/detect.rs` | VLM local por Ollama, con **confianza, motivo y texto crudo**; editable |
| Segmentación | `core/src/studio/detect.rs` | FFmpeg scene-detect → un plano = un job; `partir_plano` recorta sin recomprimir |
| Preflight | `core/src/studio/detect.rs` | Junta todo y decide: `listo` / `con_avisos` / `bloqueado`, con avisos concretos y disco estimado |
| Vista | `app/src/views/Reality.tsx` | «Convertidor»: motores, presupuesto, catálogo diagnosticado y analizador de material |
| Comandos | `src-tauri/src/lib.rs` | `studio_motores`, `studio_presupuesto`, `studio_estilos`, `studio_diagnostico`, `studio_preflight`, `studio_planos`, `studio_detectar`, `studio_cortar_plano` |
| Prueba real | `core/examples/studio_check.rs` | `cargo run --example studio_check -- <archivo>` |

## 2. Verificación con datos reales (hoy, en esta máquina)

### Motores

```
ComfyUI  ONLINE  v0.43.0   cuda:0 RTX 5060 Ti · 15094 MiB libres de 16283
  capacidades: ltx_i2v, ltx_ic_lora, wan_vace, wan_animate2, wan_animate_wrapper,
               wan_wrapper, flux_kontext, ipadapter, style_model, video_in, video_out, video_out_webp
Ollama   ONLINE  v0.40.13  82 modelos · 13 con visión
  capacidades: vision, deteccion_estilo, bakllava:latest, llava:34b, llava:latest, qwen3-vl:4b
Wan2GP   offline  http://127.0.0.1:7861  (puerto por defecto; configurable con KUKIA_WAN2GP)
Maestro  offline  http://127.0.0.1:7860
```

### Catálogo diagnosticado

```
listo              4   anime_to_photoreal, anime_to_photoreal_image, real_to_anime_image, wan_animate_transfer
listo_sin_probar   4   cartoon/cgi/sketch → photoreal, game_to_photoreal
faltan_modelos     1   real_to_anime  (falta ltx23_real2anime_rank64_v1_5000, los 654 MB del informe)
experimental       4   real→cartoon, real→game, anime↔cartoon
```

**Ninguno se marca «listo» por estar instalado**: los cuatro «listo» incluyen dos medidos hoy con su tiempo y su pico de VRAM, y los otros dos son los paquetes de imagen ya probados en KUKIA.

### Preflight real sobre `a2r_guia.mp4`

```
sondeo: video · 1080×1920 · 30 fps · 5.00 s · sin audio · h264
estilo: anime (95 %) con qwen3-vl:4b
        "imagen de animación japonesa 2D, estilo clásico de anime con trazos y sombreado característicos"
planos: 1  (#00 0.00 → 5.00 s)
paquete: Anime → acción real (vídeo) · anime_to_photoreal · plantilla anime2real_ltx_video
VEREDICTO: con_avisos
  · el material no tiene audio: el montaje saldrá mudo salvo que se añada
  · el paquete pide 16 GB de VRAM y hay 6.5 GB libres
  · pide 48 GB de RAM y hay 36.8 GB libres
```

## 3. Dos fallos que destapó la prueba (y se corrigieron)

1. **El inventario de modelos no miraba los text encoders.** `modelos_visibles` consultaba checkpoints, LoRAs, unets, VAEs, clip vision e IP-Adapter, pero no `CLIPLoader`. Resultado: `umt5_xxl` salía como ausente y tres paquetes que sí estaban completos se marcaban «faltan modelos». Añadido `CLIPLoader/clip_name`.
2. **El VLM elegido era flojo y respondía texto en vez de JSON.** Con `bakllava` la detección daba 0 %. Ahora se prefiere `qwen3-vl` y, si la respuesta no es JSON, se busca la palabra del catálogo en el texto y se marca con confianza 0,4 y el crudo guardado, en vez de inventar un porcentaje.

## 4. Límites que hay que decir en voz alta

- **Las capacidades del motor se demuestran, la calidad no.** Que ComfyUI tenga `WanAnimate2ToVideo` no significa que el estilo 3D→real vaya a quedar bien: eso es RC4/RC6.
- **La detección de estilo es clasificación por texto**, no un modelo entrenado para enrutar. Acierta a 95 % en el caso probado, pero se permite corregirla a mano y avisa cuando baja del 60 %.
- **La detección consume VRAM**: el VLM cargado ocupó ~5 GB en la 5060 Ti y el preflight leyó 6,5 GB libres en vez de los 15,7 de antes. El aviso de VRAM del preflight es honesto pero incluye al propio detector. Queda pendiente liberar el VLM al terminar (una línea, cuando se autorice).
- **Wan2GP y Maestro están offline** y el registro lo dice sin adornos. El puerto de Wan2GP es un valor por defecto configurable (`KUKIA_WAN2GP`), no un dato verificado de su instalación.
- **No se ejecuta ningún trabajo**: RC1/RC2 preparan y avisan. Lanzar es RC3/RC4.

## 5. Comprobaciones pasadas

- `cargo test --lib` en el core: **84 tests, 0 fallos** (9 de ellos nuevos de `studio`).
- `cargo check` en `src-tauri`: limpio, con los 8 comandos registrados.
- `npm run build` (tsc + vite): limpio, 52 módulos.
- Prueba funcional real con `cargo run --example studio_check -- <archivo>`: motores, catálogo y preflight con datos de verdad.

## 6. Siguiente paso natural

1. **RC3**: paquetes de estilo de imagen (real→anime y vuelta, cartoon, 3D, pixel) — resultados visibles el primer día porque la vía imagen ya funciona.
2. Descargar el `ltx23_real2anime_rank64_v1_5000` (654 MB) y cerrar `real_to_anime` en vídeo.
3. Liberar el VLM de VRAM tras detectar, y medir el preflight con la tarjeta limpia.
