# OMEGA — SQ1: COLA PERSISTENTE Y WORKERS

**Fecha:** 02/11/2026
**Estado:** implementado, compilado y probado.

---

## 1. Qué se ha construido

Una cola de trabajos **persistente en SQLite** (`%APPDATA%\KukiaCode\kukia.db`), con:

- **Scheduler acotado**: máximo `MAX_WORKERS = 4`. Los workers reales son hilos de Tokio lanzados
  al arrancar; el número activo se puede bajar en caliente (1–4) desde la vista Tareas.
- **Reclamo atómico**: `claim_next()` usa `UPDATE ... WHERE state='queued' ... RETURNING`, así que
  dos workers nunca cogen el mismo trabajo (hay un test que lo comprueba).
- **Reintentos**: cada trabajo guarda `attempts` / `max_attempts`. Si falla y quedan intentos, vuelve
  a `queued`; si no, queda `failed` con el error real del motor. Nada de errores inventados.
- **Recuperación al arrancar**: `requeue_stale()` devuelve a la cola lo que quedó en `running` de una
  sesión anterior (app cerrada a lo bruto). Un trabajo no se pierde al cerrar la app.
- **Parada global**: `AtomicBool` compartido. Al activarla, los workers dejan de reclamar y los
  trabajos largos (medición) abortan antes de empezar. Reanudable sin reiniciar.
- **Cancelación**: solo lo `queued`. Un trabajo en ejecución **no** se corta, para no matar una
  generación a medias.

## 2. Archivos

**Creados**

| Archivo | Líneas | Contenido |
|---|---|---|
| `core/src/jobs.rs` | 464 | Motor de la cola: enqueue, claim, finish, fail, cancel, requeue, stats, list, clear, progress + 9 tests |
| `app/src/views/Tasks.tsx` | 460 | Vista Tareas completa |
| `INFORMES/OMEGA_SQ1_COLA.md` | — | Este informe |

**Modificados**

| Archivo | Cambio |
|---|---|
| `core/src/lib.rs` | `pub mod jobs;` |
| `core/src/memory.rs` | Tabla `jobs` + índice `idx_jobs_state` en el esquema |
| `src-tauri/src/lib.rs` | 10 comandos `jobs_*`, pool de workers, `ejecutar_job()` con 4 tipos reales |
| `app/src/lib/api.ts` | Interfaces `Job`, `JobStats`, `JobsEstado` + 9 métodos |
| `app/src/App.tsx` | Vista `tasks` registrada en la navegación y el router |

## 3. Tipos de trabajo que se ejecutan de verdad

| Tipo | Qué hace | Backend real |
|---|---|---|
| `bench` | Mide el modelo **ya cargado**: tok/s, prefill tok/s, TTFT, pico de VRAM y RAM | `lab::benchmark` + guarda en el histórico de benchmarks |
| `vram_release` | Libera un motor, una GPU o ambas, midiendo VRAM antes/después | `vram::release_engine` / `release_gpu` / `release_everything` |
| `research_scan` | Escanea arXiv, Hugging Face o GitHub y registra novedades y alertas | `research::scan_one` + upsert y alertas en BD |
| `noop` | Trabajo de prueba con espera | `tokio::time::sleep` |

Los workers **no cargan modelos**: miden lo que tú hayas cargado, liberan lo que pidas o escanean
fuentes públicas. Cargar es una decisión que sigue siendo tuya (y del preflight de CC1–CC4).

## 4. Comandos nuevos

`jobs_enqueue`, `jobs_list`, `jobs_stats`, `jobs_cancel`, `jobs_cancel_all`, `jobs_clear`,
`jobs_stop`, `jobs_resume`, `jobs_estado`, `jobs_workers`.

## 5. Pruebas ejecutadas (reales)

```
cd core && cargo test --lib jobs
  9 passed; 0 failed
  - un_tipo_inventado_se_rechaza
  - dos_workers_no_cogen_el_mismo_trabajo
  - la_cola_respeta_prioridad_y_orden
  - un_fallo_reintenta_y_luego_queda_failed
  - al_arrancar_se_recupera_lo_que_quedo_a_medias
  - no_se_cancela_lo_que_esta_corriendo
  - terminar_guarda_resultado_y_limpia_error
  - las_estadisticas_cuentan_cada_estado
  - limpiar_solo_borra_lo_terminado

cd core && cargo test --lib
  114 passed; 0 failed   (nada roto en el resto del core)

cd . && npx tsc --noEmit
  exit 0

cd . && cargo check -p kukia-code
  Finished dev profile in 12.55s, exit 0
```

## 6. Límites honestos

- **Progreso**: la columna existe y la UI la muestra, pero solo se rellena si el motor lo publica.
  Cuando no, pone «no disponible» en vez de inventar un porcentaje.
- **Un trabajo en ejecución no se cancela** ni con parada global: se deja terminar.
- **Sin límite de tiempo por trabajo** todavía: un motor colgado bloquearía su worker (los otros
  tres siguen libres). Pendiente de timeout por tipo.
- **Los workers viven en el proceso Tauri**: si la app se cierra, mueren, y lo que estaba corriendo
  vuelve a la cola al siguiente arranque (por diseño, no se pierde).

## 7. Lo que falta de la fase A completa

SQ1 era la base. Quedan, en este orden: timeout por trabajo, panel con histórico de duración media
por tipo, y enganchar `bench` al preflight de VRAM antes de lanzar (para no medir sobre un modelo
que no existe).
