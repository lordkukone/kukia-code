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
