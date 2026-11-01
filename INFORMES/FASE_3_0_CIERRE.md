# KUKIA CODE — FASE 3.0: CIERRE Y CONEXIÓN

**Fecha:** 01/11/2026 · **Autor:** Xoxete (para Kuki / LordKukone)
**Alcance pedido:** aplicar la política de permisos en escritura y comandos, timeout con cancelación,
conectar `audit_log`, con backup verificado y tests.
**Estado:** **completado.** Todo lo que se afirma aquí tiene una salida de comando detrás.

---

## 0. RESUMEN

| Objetivo | Estado |
|---|---|
| Política de permisos aplicada a escritura | ✅ `files_write` consulta `permissions::check` |
| Política aplicada a comandos | ✅ `run_command` + alcance de directorio |
| Timeout en la ejecución | ✅ 60 s por defecto, ajustable; mata el árbol |
| Cancelación | ✅ botón en la terminal + `command_cancel` |
| `audit_log` conectado | ✅ lectura, escritura, comando, export y cambio de permisos |
| Backup verificado | ✅ 10 archivos, SHA-256 comprobado, manifiesto |
| Tests | ✅ **68 pasan** (51 unitarios + 8 validación + 9 Fase 3.0) |
| Compilación | ✅ núcleo + Tauri + frontend, **sin warnings de código** |
| App arranca | ✅ ventana «KUKIA CODE», 34,8 MB, siembra 9 políticas |

**Dos bugs reales encontrados y corregidos durante la fase**, ambos preexistentes:

1. **`audit_search` ignoraba el filtro de texto** si no filtraba además por herramienta: el
   parámetro `?1` estaba enlazado a la herramienta en vez de al término de búsqueda.
2. **`is_destructive` no detectaba `rd /s /q`** (faltaba el espacio inicial en el patrón), justo el
   comando que borra un árbol entero en Windows.

---

## 1. BACKUP VERIFICADO (hecho antes de tocar nada)

```
backups/fase3_0_20261101_113849/   + MANIFIESTO.md
10 archivos copiados y verificados por SHA-256

core/src/permissions.rs  9698 B  da668bb52b9e29b2
core/src/security.rs     2247 B  2f60fa2c021f64c3
core/src/memory.rs      46597 B  91a6b0c8b25e31b8
core/src/lib.rs           494 B  976c8a6470e2514c
src-tauri/src/lib.rs    16452 B  e47bcda2d2b1ab41
app/src/lib/api.ts       7341 B  3828d4aeb77c4854
app/src/App.tsx          7012 B  683c59023fc4c39f
app/src/views/Terminal.tsx 2542 B cb00dd5c929ed3b1
app/src/styles.css      13372 B  9cd3d1cee5ae0fb1
docs/SEGURIDAD.md        1926 B  565b82d4651faaba
```

El script aborta si un solo hash no cuadra. Ningún archivo se modificó antes del backup.

---

## 2. PERMISOS APLICADOS (lo que faltaba)

`core/src/permissions.rs` ya existía; ahora **se consulta de verdad** en cada operación sensible
(`src-tauri/src/lib.rs`):

| Comando | Comprobaciones, en orden |
|---|---|
| `files_read` | `allow_read` → `read_paths` |
| `files_write` | `allow_write` → aprobación → **solo lectura** → allowlist → backup → escritura |
| `run_command` | `allow_shell` → aprobación → destructivos → `cwd` dentro de las rutas |
| `run_command_stream` | igual que `run_command` |
| `audit_export` | `allow_write` + aprobación + allowlist sobre la ruta del CSV |
| `policy_save` | aprobación explícita del usuario |

### 2.1 Rutas reales, no de texto

`real_path()` canonicaliza la ruta (y el padre si el archivo aún no existe) y quita el prefijo
`\\?\`. Consecuencia medida: **un junction de Windows ya no permite escapar** de la allowlist —
test `junction_no_permite_escapar` crea `mklink /J` y exige denegación.

### 2.2 Destructivos por verbo

`is_destructive` ahora trocea el comando respetando comillas y compara **tokens** contra 17 verbos,
más secuencias (`git reset --hard`, `git clean -f`, `robocopy /MIR`…).

| Comando | Antes | Ahora |
|---|---|---|
| `rd /s /q C:\datos` | **no detectado** | detectado |
| `Get-ChildItem C:\tmp \| Remove-Item -Force` | detectado | detectado |
| `diskpart`, `takeown`, `robocopy /MIR` | no | detectado |
| `Get-Content "C:\model del dia.txt"` | falso positivo | **no** se marca |
| `npm run build`, `git status` | correcto | correcto |

### 2.3 Terminal acotado

`check_cwd_in_scope` exige que el directorio de trabajo esté en las rutas autorizadas y no sea de
solo lectura. Sin esto, la allowlist sería decorativa: bastaría `Set-Content` desde el terminal.

### 2.4 Políticas sembradas

Al arrancar, si no hay ninguna, se crean para los **9 agentes**:

```
kukia-coder     write=1 shell=1 rutas=C:\Users\kuki;H:\KUKIA_CODE;H:\KUKIA_RESEARCH
kukia-cyber     write=1 shell=1  (idem)
kukia-gaming    write=1 shell=1  (idem)
kukia-gpu       write=1 shell=1  (idem)
kukia-master    write=1 shell=1  (idem)
kukia-radar     write=1 shell=1  (idem)
kukia-research  write=1 shell=1  (idem)
kukia-sysadmin  write=1 shell=1  (idem)
kukia-wealth    write=1 shell=1  (idem)
```

- `I:\CEREBRO` queda en `read_only_paths` de todas: escribir ahí está prohibido y el mensaje lo dice
  («es de solo lectura»), no un genérico «fuera de las rutas».
- Es una decisión explícita, no el `default_for` permisivo del código: se permite escribir en el
  perfil del usuario y en los dos directorios del proyecto, y nada más.

---

## 3. TIMEOUT, TOPE Y CANCELACIÓN

`core/src/security.rs` reescrito:

- `tokio::process::Command` con `kill_on_drop(true)`.
- **Timeout** configurable (60 s por defecto) → al agotarse: `start_kill()` **+ `taskkill /PID /T /F`**
  para el árbol entero.
- **Tope de salida de 1 MB** por flujo, siguiendo a drenar la tubería para que el hijo nunca se
  quede bloqueado escribiendo.
- Devuelve `duration_ms`, `pid`, `truncated`, `timed_out`, `cancelled`.
- `run_command_stream` devuelve un **id inmediato** (no bloquea) y emite `started` / `done` /
  `failed` por un `Channel`; `command_cancel(id)` envía la señal; `commands_running` lista lo vivo.

Tests que lo demuestran (ejecutados, no supuestos):

| Test | Comprueba |
|---|---|
| `el_timeout_mata_el_proceso` | `Start-Sleep 30` con timeout 1 s → `timed_out=true` en < 15 s |
| `la_cancelacion_corta_la_ejecucion` | cancela a los 0,9 s → `cancelled=true` |
| `recorta_la_salida_enorme` | 2 MB de salida → `truncated=true`, exactamente 1 MB guardado |
| `ejecuta_y_devuelve_codigo` | código 0, salida correcta y PID informado |

---

## 4. AUDITORÍA CONECTADA

Cada operación sensible escribe en la tabla `audit` (agente, herramienta, acción, objetivo,
aprobación, resultado, error). Antes **nadie llamaba a `audit_log`**: la tabla existía vacía.

Ahora: `files.read`, `files.write` (incluidas las denegaciones del backup), `terminal.run` (con
estado `ok` / `error` / `tiempo agotado` / `cancelado` / `denegado`), `audit.export` y
`policy.save`.

Vista nueva **Seguridad** (🛡️) con dos bloques:

- **Permisos**: los 9 agentes, toggles por herramienta, rutas de escritura / solo lectura / lectura,
  y guardado que exige confirmación (y queda auditado como `usuario / policy.save`).
- **Auditoría**: últimas 60 acciones con fecha, agente, herramienta, objetivo y resultado, botón de
  recarga y exportación a CSV.

Y las tres trampas del informe anterior, cerradas:

| Hallazgo | Estado |
|---|---|
| E-01 allowlist decorativa | **cerrado** — se consulta en lectura, escritura, comando y export |
| E-02 `audit_log` nunca llamado | **cerrado** — 6 puntos de escritura + vista + CSV |
| E-03 sin timeout ni tope | **cerrado** — timeout, `taskkill /T`, 1 MB, cancelación |
| E-04 junction | **cerrado** — canonicalización + test con `mklink /J` |
| E-05 `rd /s /q` | **cerrado** — detección por verbo + 3 tests |
| E-07 mutex envenenado | **cerrado** — `guard()` recupera el contenido en vez de paniquear |
| E-12 warning `mut` en gaming.rs | **cerrado** |
| E-13 `set_var` en tests | pendiente (Fase 3A) |
| E-06 tests escriben en el log real | **pendiente** — siguen apareciendo líneas `[test]` en `kukia.log` |
| E-09 H: al 97 % | pendiente (operativo) |
| E-14 cuota GitHub | pendiente (Fase 3C) |

---

## 5. PRUEBAS EJECUTADAS (salidas reales)

### 5.1 Núcleo

```
cargo test -p kukia-core --offline

  51 passed; 0 failed              (unitarios, eran 39)
   8 passed; 5 ignored             (validacion_fase21, red)
   9 passed; 0 failed              (validacion_fase30, nuevo)
```

Tests nuevos de aceptación (`core/tests/validacion_fase30.rs`, 9):

`las_politicas_se_siembran_y_son_idempotentes` · `el_cerebro_queda_como_solo_lectura` ·
`escritura_fuera_de_las_rutas_se_deniega` · `escritura_dentro_de_las_rutas_pasa` ·
`politica_sin_rutas_deniega_todo` · `comando_destructivo_exige_permiso_de_escritura` ·
`el_terminal_solo_corre_en_rutas_autorizadas` · `la_auditoria_se_filtra_y_se_exporta` ·
`la_politica_manda_sobre_la_aprobacion`

Los tres primeros fallaron en la primera pasada y **destaparon los dos bugs de §0**. Se corrigió el
código, no el test.

### 5.2 Compilación

```
cargo check --workspace --offline
  Checking kukia-core v0.1.0
  Checking kukia-code v0.1.0
  Finished `dev` profile in 3.51s                       ✅ sin warnings de código

cargo build --workspace --offline
  Finished `dev` profile in 1m 32s                      ✅ enlazado
  target/debug/kukia-code.exe   21.294.592 bytes

npm run build
  tsc --noEmit sin errores
  ✓ 47 modules transformed
  index-DfKkv9v4.js  276,17 kB │ gzip 83,36 kB          ✅
```

Único aviso restante: `linker_messages` al crear el import-lib de la DLL del crate `lib` — es del
enlazador de Windows, no del código, y no afecta al binario.

### 5.3 Arranque real y siembra

```
ANTES : 11 tablas
Arranque: PID 12404 · proceso vivo True · ventana «KUKIA CODE» · 34,8 MB
DESPUÉS: 17 tablas  (+ agent_policies, audit, tasks, gaming_items, game alerts, gaming_runs)
         agent_policies: 9 filas
         integridad sqlite: ok
Cierre : limpio
```

---

## 6. LO QUE **NO** SE HA PROBADO (dicho claro)

1. **Los clics en la interfaz.** Compilé el frontend y verifiqué que el binario arranca, pero no he
   pulsado «Guardar permisos» ni «Cancelar» dentro de la ventana. La cadena de permisos está probada
   por los 9 tests de aceptación; la UI, por compilación de tipos estricta.
2. **El instalador NSIS nuevo.** No se ha recompilado: la app instalada sigue siendo la de Fase 2.1
   (`859a8078…`) y **no contiene** esto. Hay que ejecutar `npm run tauri:build` para publicarla.
3. **E-06**: los tests siguen escribiendo líneas `[test]` en el log real de producción.

---

## 7. ARCHIVOS

Modificados (11): `core/src/security.rs`, `core/src/permissions.rs`, `core/src/memory.rs`,
`src-tauri/src/lib.rs`, `app/src/lib/api.ts`, `app/src/App.tsx`, `app/src/views/Terminal.tsx`,
`app/src/styles.css`, `core/tests/validacion_fase21.rs`, `docs/SEGURIDAD.md`, `core/src/gaming.rs`
(un `mut`).

Nuevos (2): `core/tests/validacion_fase30.rs`, `app/src/views/Security.tsx`.

`git diff --stat` contra `HEAD`: **1.821 inserciones, 122 borrados**. Ojo: ese total incluye
también lo que ya estaba sin commitear de la sesión anterior (`gaming.rs`, `vram.rs`,
`permissions.rs` y sus tablas), no solo Fase 3.0. Sin commitear: no se ha pedido commit.

---

## 8. SIGUIENTE PASO

1. `npm run tauri:build` y reinstalar (con tu autorización) para que la app instalada lleve Fase 3.0.
2. **Fase 3A**: permisos por proyecto, `set_var` en tests (E-13), y sacar los tests del log real (E-06).
3. **Fase 3B**: conectar `vram.rs` con su vista (el motor ya está escrito y probado).

> Nada se declara terminado sin la salida pegada arriba. Los dos bugs que encontré los encontraron
> los tests, no mi vista.
