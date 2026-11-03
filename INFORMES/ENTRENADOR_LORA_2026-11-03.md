# Entrenador de LoRA (LA8) — 03/11/2026

## De dónde viene

`INFORMES/AUDITORIA_LIVE_ACTION_STUDIO.md` marcaba el área 11 (*Character LoRA
Studio*) como **FALTANTE**: «sin dataset builder, sin captions, sin
entrenamiento». La fase **LA8** del plan era justo eso, estimada en 2–3 sesiones y
marcada como riesgo alto por «descargas y horas de GPU».

Esto es la primera entrega de esa fase: funciona de punta a punta y se ha probado
ejecutándola, no compilándola.

## Qué hay ahora

```
KUKIA CODE
├─ trainer/entrenar_lora.py     entrenamiento real (diffusers + peft)
├─ core/src/entrenador.rs       proyectos, dataset, captions, runs, proceso
├─ app/src/views/Entrenador.tsx vista con dataset, captions y progreso
└─ %APPDATA%\KukiaCode\entrenador\<proyecto>\{dataset,lora}
```

**Un proyecto** guarda nombre, clase (personaje/estilo/objeto/escenario), modelo
base, imágenes y sus captions. Tres tablas nuevas en la base que ya existía:
`lora_proyectos`, `lora_imagenes`, `lora_runs`.

**Dataset.** Se importa por carpeta: las imágenes **se copian** al proyecto (el
original no se toca) y se leen ancho y alto sin dependencias, mirando la cabecera
IHDR del PNG y el segmento SOF del JPEG.

**Captions.** Se escriben a mano o se piden a un modelo de visión local (Ollama,
`/api/generate` con la imagen en base64). El caption propuesto **no se guarda
solo**: aparece en el campo y el usuario decide. Al exportar se escribe un `.txt`
por imagen; las que no tienen caption no se exportan, y el script las omite
avisando cuántas son.

**Preflight antes de gastar GPU.** Dice cuántas imágenes hay, cuántas tienen
caption, qué VRAM libre queda (de la capa universal de GPU) y no autoriza si falta
modelo base o no hay ni un caption.

**Entrenamiento.** KUKIA lanza `python trainer/entrenar_lora.py` y sigue su salida:
una línea JSON por evento (`inicio`, `paso`, `guardado`, `fin`, `error`). El
progreso de la interfaz es el `paso` que declara el proceso, no una estimación por
tiempo. Se puede cancelar, y se mata **ese** pid, nunca por nombre.

**Formato de salida.** El LoRA se guarda con nombres al estilo sd-scripts/kohya
(`lora_unet_down_blocks_0_attentions_0_...to_q.lora_down.weight` + `.alpha`), que es
lo que cargan ComfyUI y las interfaces habituales, en vez de los nombres internos de
peft. Se traduce en `clave_kohya()`.

## Pruebas ejecutadas

| Prueba | Resultado |
|---|---|
| `cargo test -p kukia-core --lib` | **189 pasan, 0 fallan, 2 ignoradas** |
| `entrenador::tests` (8) | proyectos, importación con tamaño real, exportación de captions, preflight honesto, eventos, base64 |
| **De punta a punta** (ignorada, lanzada a mano) | Rust → Python → **estado `terminado`, 2/2 pasos, `.safetensors` de 75.160 bytes** |
| Script suelto `--probar` | 3 pasos, 32 claves LoRA, 96 tensores, todas `lora_unet_*`, 32 `alpha` |
| `cargo check -p kukia-core --all-targets` | 0 errores, 0 avisos |
| `cargo check -p kukia-code` | 0 errores |
| `npx tsc --noEmit` | 0 errores |

## Lo que se ha instalado en la máquina

`pip install peft` (0.21.2). Ya estaban `torch 2.12.1+cu130` con CUDA disponible,
`diffusers`, `transformers`, `accelerate`, `safetensors`, `PIL` y `numpy`.
crates.io responde 403 en esta máquina, así que el lado Rust **no** añade ninguna
dependencia: DXGI/PDH por FFI y el base64 escrito a mano.

## Lo que no está probado

- **Un entrenamiento real sobre un modelo de difusión.** La prueba de humo usa una
  UNet diminuta: valida adaptador, optimizador, guardado y formato de claves, pero
  **no** dice nada de la calidad del resultado ni de si SDXL cabe en 12 GB con
  resolución 1024. Para eso hace falta un `.safetensors` base y horas de GPU, y el
  usuario tiene que elegir cuál.
- La ruta SD 1.5 (`--modelo` con un repo o un `single_file` que no sea SDXL) está
  escrita pero no ejecutada: el camino probado es el de la UNet de prueba.
- Los captions automáticos necesitan un modelo de visión descargado en Ollama; no se
  descarga nada por su cuenta.

## Siguiente paso natural

Probar con un modelo real pequeño (SDXL a 512, 100 pasos, rango 4) midiendo pico de
VRAM y tiempo por paso, y ajustar el preflight con esos números en vez de con una
estimación.
