# CAMBIOS — registro de modificaciones

Regla de la casa: **antes** de modificar cualquier archivo existente se guarda un `.zip` y se anota
aquí su SHA-256. Nunca se borra nada.

| Fecha | Archivo | Acción | Backup | SHA-256 |
|---|---|---|---|---|
| 04/11/2026 | `PLAN_3GPU.md` | creado (Fase 0, plan 3 GPUs) | — (nuevo) | ver abajo |
| 04/11/2026 | `CAMBIOS.md` | creado (este registro) | — (nuevo) | ver abajo |

**Archivos existentes modificados: ninguno.** La Fase 0 no toca código.

SHA-256 de los archivos creados:

```
dc20e86202aba8e155e76afc72dd217197c598a2beecea07767d564836b71a55 *PLAN_3GPU.md
9d5705ec4a76dd5cfa044239dc6cebfd89948c01cdd7f541e877733f04074cb3 *CAMBIOS.md
```

## Fase 1 — modulo localgpu (04/11/2026)

Backup previo: `backups/3gpu_2026-11-04/antes_fase1.zip` (sha256 `adfb465a042bac951efdc51c51d4a73243a0ef7f460d38c3ce77c81f07d1079e`)

| Archivo | Accion | SHA-256 despues |
|---|---|---|
| `core/src/lib.rs` | modificado (anadir modulo / campo / comandos) | `c4da958a6086d237d299bd66900bd87d884ae7d497fd07a0fec7ada93abbd128` |
| `core/src/config.rs` | modificado (anadir modulo / campo / comandos) | `c7ce7bbe0dc1718ac4e8a033d9b457cc5f776199c552180fd1462ff6c8bad7b9` |
| `src-tauri/src/lib.rs` | modificado (anadir modulo / campo / comandos) | `bef3e011ad9960b36ac065a6a1edb0ccac38d21a2b11336a1bb1df72aec18775` |
| `core/src/localgpu/mod.rs` | creado | `49f586049dc3038f8e13615090399c46126137838384ad350fcba237609368f8` |
| `core/src/localgpu/flags.rs` | creado | `79ef60092e9fddec13c1cd6c4ee00fea925bfaf3c6c8014ceca1b59733324cfc` |
| `core/src/localgpu/proceso.rs` | creado | `655af823c51e23e9feee9f448c57a68053079c88dc04560728a1d5bb1cabfc8b` |
| `core/src/localgpu/rpc.rs` | creado | `b4b18ffa6a4a62542b7cfccd3853f6b6a82747bced36cbef615e0a158d7dc93e` |

Nada borrado. Los tres archivos modificados se pueden recuperar del zip.

## Fase 2 — proveedor, comandos e interfaz (04/11/2026)

| Archivo | Accion | SHA-256 despues |
|---|---|---|
| `core/src/providers.rs` | motor `llamacpp` (base, chat y stream) | `7fc8c21fd2a2a075b9b1aa45d5aca82624cd5459310a3758e6fbd42645546879` |
| `src-tauri/src/lib.rs` | comando `localgpu_set` + registro | `e8df850d82e5b04e916c4b2cd3f74e8e3252cac3b9af9518662eeb5faf1a0dd8` |
| `app/src/lib/api.ts` | tipos y 7 invocaciones del motor local | `4192e6d5cbf9aa7f2fd0925eaf7bc874b009272655f12fae4dce82425fd7bd87` |
| `app/src/views/Gpu.tsx` | tarjeta "Motor local" (perfil, modelo, arrancar/parar, sonda, log) | `2123e4815d76c670cdca5eed9cc5fee81d4ebdc10bbeb1b90e361fe4c62c934e` |
| `core/examples/localgpu_smoke.rs` | creado: prueba real de arranque y parada | `157c122eece45857e50ad7d63b980271a84e1983d63fcc5792ee45f75462d736` |

Config del usuario: copia en `backups/3gpu_2026-11-04/config_antes_fase2.json`.
Se anadieron solo las claves de `local_gpu`; el resto del JSON queda igual.

Prueba real (04/11 16:07): llama.cpp b12110 CUDA 13.4 en `H:\llama`; modelo Qwen3.5-9B Q6_K.
Modelo cargado en 9,5 s, escuchando en 127.0.0.1:8099, parada limpia (sin hijos vivos).
Dato: en esta version el servidor RPC se llama `ggml-rpc-server.exe`, no `rpc-server.exe`.

Correccion (04/11): la primera escritura fue a una copia de la carpeta de backups.
La config viva (`%APPDATA%\KukiaCode\config\config.json`) se copio antes a
`backups/3gpu_2026-11-04/config_viva_antes_fase2.json` y se parcheo despues.

## Fase 3 — herramientas en el motor local y medida del Flint (04/11/2026)

| Archivo | Cambio | SHA-256 |
|---|---|---|
| `core/src/providers.rs` | `stream_openai` ofrece herramientas, recoge `tool_calls` del stream, las ejecuta y sigue la conversacion (hasta 6 pasos); resultado recortado a 3000 caracteres | `f4981c33be1179d3624782ff5cd6f8cbd60e929f10919e3e1eb71dee0197beab` |
| `core/examples/llamacpp_tools.rs` | creado: chat real con herramientas contra llama-server | `228c49a015cb9cb6b182f91248a4b999cee35c316e7c9616e82cc61db3aea9db` |

Medidas reales (Flint-40B-A2.5B-NVFP4, 22,9 GB, 2 GPUs del i9, ctx 4096):
generacion **83,7 tok/s** (160 tokens), **100,4 tok/s** en una llamada de herramienta,
prompt corto 32,1 tok/s. El modelo emite `tool_calls` correctamente con `--jinja`.
Herramientas usadas de verdad en la prueba: `files_list`, `files_find`, `dir_tree` (33 disponibles).

Hallazgos:
- El puerto 8080 esta ocupado por `nvpair-proxy.exe` (PAIR de NVIDIA): la config pasa a **8090**.
- El ajuste automatico de memoria de b12110 aborta si se fuerza `-ngl 99` sin sitio: no pasar `-ngl`.
- Un `dir_tree` de raiz supero el contexto (7364 > 4096 tokens): por eso el recorte a 3000 caracteres.

## Verificacion de 3 GPUs (04/11/2026)

| Archivo | Cambio | SHA-256 |
|---|---|---|
| `core/examples/localgpu_rpc_check.rs` | creado: sonda real + decision de perfil | `fa03739712d42066267b51061c9d65240d806c6d2ad15a656836580f5d2cab56` |

Resultados medidos:

- i7 `192.168.50.2`: responde a ping, WinRM 5985 abierto, **RPC 50052 cerrado** (expira a los 10 s).
- Sonda de KUKIA: `ok=false`, 10.002 ms, decision `SoloI9` con motivo
  «el rpc-server del i7 no responde: se cae a las 2 GPUs del i9». Criterio de <=10 s: **CUMPLE**.
- Arranque forzado de 3 GPUs (sin pasar por la sonda): llama-server recibe
  `--tensor-split 35,40,25 --rpc 192.168.50.2:50052`, falla con
  `Failed to connect to 192.168.50.2:50052` y muere (codigo -1073740791) a los 24,6 s.
  Confirma que el guardia de la app (sondear antes) es imprescindible: sin el, no hay fallback.

**Conclusion: las 3 GPUs no estan verificadas.** Falta arrancar `ggml-rpc-server.exe` en el i7
(`H:\llama\ggml-rpc-server.exe -H 0.0.0.0 -p 50052`) con el firewall abierto en 50052.

## Tokens/s y contexto maximo con Flint 40B (04/11/2026, 2 GPUs del i9)

Medido con un solo llama-server, sin `-ngl` (ajuste automatico) y sin LM Studio compitiendo:

| Contexto | Gen (prompt corto) | Gen (prompt 4,7k) | Prefill (4,7k) |
|---|---|---|---|
| 32.768 | 99,3 tok/s | 98,3 tok/s | 2.230 tok/s |
| **65.536** | **100,7 tok/s** | **96,5 tok/s** | **2.377 tok/s** |
| 131.072 | 69,8 tok/s | 72,4 tok/s | 899 tok/s |

- 65.536 es el punto dulce: todo el modelo y el KV caben en las dos GPUs y no baja la velocidad.
- 131.072 tambien arranca y funciona, pero pierde ~30% porque parte de las capas caen a CPU
  (GPU1 a 14.363 de 16.311 MB, GPU0 a 11.506 de 12.227 MB).
- Config de la app: `local_gpu.ctx = 65536`.
- Con la 2070 del i7 por RPC habria ~8 GB mas y el 131k deberia caber entero, pero el RPC sigue apagado.

## 3 GPUs verificadas: RPC del i7 en marcha (04/11/2026)

Montaje hecho en el i7 (`DESKTOP-6CK4U6H`, admin por WinRM):

- `H:\llama` copiado a `C:\llama` (736 MB, con ggml-cuda.dll y ggml-rpc.dll).
- Firewall: reglas «llama.cpp RPC 50052» y «llama.cpp RPC 50052 (programa)», TCP entrante, perfil Any.
- `C:\llamarrancar_rpc.bat` + tarea programada **`llama-rpc`** al iniciar sesion (RunLevel Highest).
  Importante: si el proceso se lanza dentro de una sesion de WinRM, muere al cerrarla; la tarea no.
- Verificado desde el i9: `192.168.50.2:50052` abre en **11 ms**.

Medidas con `--rpc 192.168.50.2:50052` (Flint 40B NVFP4):

| Montaje | ctx | Gen (corto, caliente) | Gen (prompt 4,7k) | Prefill 4,7k |
|---|---|---|---|---|
| 2 GPUs | 65.536 | 100,7 tok/s | 96,5 tok/s | 2.377 tok/s |
| 3 GPUs | 65.536 | ~77 tok/s | 77,5 tok/s | 122 tok/s (frio) |
| 2 GPUs | 131.072 | 69,8 tok/s | 72,4 tok/s | 899 tok/s |
| **3 GPUs** | **131.072** | **80,5 tok/s** | **71,0 tok/s** | **1.196 tok/s** |

- Con 3 GPUs el 131.072 cabe **entero en VRAM** (i9: 11.528 / 12.103 MB; i7 aporta ~7 GB),
  por eso a contexto grande es mas rapido que con 2 GPUs (sin capas en CPU).
- A 65.536 las 2 GPUs siguen siendo mas rapidas: el RPC por LAN cuesta mas de lo que aporta.
- Config de la app: `perfil = tres_gpus`, `ctx = 131072`, `puerto = 8090`.
