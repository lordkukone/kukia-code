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
