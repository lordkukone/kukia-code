# KUKIA CODE — ERRORES Y RIESGOS

**Fecha:** 01/11/2026 · **Autor:** Xoxete
**Base:** auditoría de solo lectura del 01/11/2026
**Regla:** cada hallazgo lleva la evidencia que lo demuestra y el arreglo concreto. Nada aquí es
suposición sin marcar.

Gravedad: **CRÍTICA** (puede destruir datos o dejar la app inservible) · **ALTA** (funcionalidad
prometida que no protege) · **MEDIA** (degradación o deuda que costará cara) · **BAJA** (limpieza).

---

## E-01 · La allowlist de rutas existe pero NO se aplica — **ALTA**

**Evidencia.** `core/src/permissions.rs:196` implementa `check()` con allowlist de escritura, rutas de
solo lectura y bloqueo de `..`. `src-tauri/src/lib.rs` **no referencia `permissions::` en ninguna
línea** (`grep` → 0 resultados). `files_write` solo comprueba `if !approved { return Err(...) }` y
después escribe en la ruta que le llegue.

**Impacto.** Con una casilla marcada, la app puede escribir en `C:\Windows`, en el CEREBRO
(`I:\CEREBRO`, que es sagrado) o en las wallets de KUKIA. La política que se escribió en Fase 3 no
protege de nada hoy. Es el hallazgo más importante del informe.

**Arreglo.** En `files_write` y `run_command`, antes de tocar disco:
`permissions::check(&db.policy_get(agent)?, Tool::Write, &path, approved)` y devolver el `reason` si
`!allowed`. Igual en `files_read`. Añadir test de integración que intente escribir en
`C:\Windows\x` y exija error.

---

## E-02 · La tabla `audit` nunca se escribe — **ALTA**

**Evidencia.** `memory.rs` define `audit`, `audit_log` (línea 788), `audit_search` (817) y
`audit_export_csv` (864). `grep -rn "audit_log" --include=*.rs . | grep -v memory.rs` → **vacío**.
En la base real: 11 tablas, `audit` **no existe** (se creará al arrancar la build nueva), y
`approvals` tiene **0 filas**.

**Impacto.** El superprompt pide «historial auditable» y «registro de comandos». Hoy lo único que se
registra es `approval_log`, y ni siquiera se ha ejecutado nunca en la app instalada. No hay forma de
responder «¿qué hizo KUKIA CODE ayer?».

**Arreglo.** Llamar `audit_log` en cada comando sensible (escritura, comando, descarga, política
cambiada) con `actor`, `action`, `target`, `result`, `detail`. Vista con filtros + botón de exportar
CSV. Test que compruebe que una escritura deja exactamente una fila.

---

## E-03 · `run_command` no tiene timeout ni tope de salida — **ALTA**

**Evidencia.** `security::run_powershell` usa `cmd.output()` sin `timeout` ni límite de bytes
(`grep timeout security.rs lib.rs` → sin resultados). `Command::output()` acumula **todo** el stdout
y stderr en memoria.

**Impacto.** Dos escenarios reales: (1) un comando que no termina (`ping -t`, un `npm install`
colgado) deja el comando Tauri sin responder **para siempre** y el proceso hijo vivo, sin botón de
cancelar en ninguna parte; (2) un comando charlatán (`dir /s C:\`) puede llenar la RAM con la salida
capturada. En una app cuyo objetivo es ejecutar cosas, esto es un fallo de diseño, no un detalle.

**Arreglo.** `tokio::process::Command` + `tokio::time::timeout` (30 s por defecto, configurable),
`kill_on_drop(true)`, captura con tope de 1 MB truncando y avisando, y devolver el PID para poder
cancelar. Test con un comando que duerme 60 s y timeout de 2 s.

---

## E-04 · La allowlist se puede saltar con un *junction* de Windows — **MEDIA**

**Evidencia.** `permissions::normalize` resuelve `.` y `..` de forma **lógica** (no toca el disco) y
compara cadenas en minúsculas. No hay `canonicalize()` ni comprobación de enlaces.

**Impacto.** Un junction `H:\proyecto\link → D:\` (se crea sin privilegios con `mklink /J`) hace que
`H:\proyecto\link\wallet.dat` pase la comprobación de prefijo y escriba en `D:\wallet.dat`. Cuando
E-01 se arregle, este será el siguiente agujero.

**Arreglo.** Antes de comparar, `std::fs::canonicalize` de la ruta **y** de cada base permitida, y
rechazar si el resultado real no cuelga de la base real. Añadir test con junction real en `%TEMP%`.

---

## E-05 · La lista de comandos destructivos tiene un falso negativo — **MEDIA**

**Evidencia.** `permissions::is_destructive` busca 14 patrones entre ellos `" rd "` y `"rmdir"`.
La cadena `rd /s /q C:\datos` en minúsculas es `rd /s /q c:\datos`: **no contiene** `" rd "`
(falta el espacio inicial), ni `"rmdir"`, ni `"del "`. Test actual sólo cubre `Remove-Item`, `del `
en medio y `rm -rf`.

**Impacto.** El comando que borra un árbol entero en Windows (`rd /s /q`) **no se detecta como
destructivo**, así que no exige el permiso de escritura adicional. Igual pasa con `robocopy /MIR`,
`Format-Volume` con otro nombre, `diskpart` y `git clean -fdx`.

**Arreglo.** Normalizar a límites de palabra (tokenizar por `^`/`;`/`|`/`&&` y comparar el primer
verbo), no por subcadena; añadir `rd`, `rmdir`, `del`, `erase`, `robocopy`+`/mir`, `diskpart`,
`git clean`, `git checkout --`, `takeown`, `icacls`. Tests con cada uno.

---

## E-06 · Los tests escriben en los datos de producción — **BAJA**

**Evidencia.** `logs::tests::append_and_tail` escribe en el log real: en
`%APPDATA%\KukiaCode\logs\kukia.log` hay **8 líneas** `[test] linea de prueba kukia` mezcladas con
los arranques reales de la app.

**Impacto.** Contamina la auditoría: no se puede distinguir una acción tuya de una prueba. Si algún
día un test escribe en la base de verdad, el daño es peor.

**Arreglo.** `config::data_dir()` ya soporta `KUKIA_DATA_DIR`. Que *todos* los tests lo fijen a un
temporal (`gaming.rs` ya lo hace con `KUKIA_GAMING_LAB`: buen ejemplo a copiar).

---

## E-07 · Mutex envenenado: un solo `panic` deja la app inservible — **MEDIA**

**Evidencia.** `state.db.lock().unwrap()` aparece en las 34 rutas de `src-tauri/src/lib.rs` y en 10
puntos de `memory.rs`.

**Impacto.** Si un hilo entra en pánico mientras tiene el mutex, el `Mutex` queda *envenenado* y
**todas** las llamadas siguientes a `.unwrap()` panican también. La app no se recupera: hay que
reiniciarla y pierdes la sesión en curso.

**Arreglo.** Un helper único `fn db(&self) -> Result<MutexGuard<..>, String>` que use
`lock().unwrap_or_else(|e| e.into_inner())`, y devolver error en vez de panico.

---

## E-08 · Módulos de Fase 3 desconectados (código muerto desde la app) — **MEDIA**

**Evidencia.** `permissions.rs` (297 líneas), `vram.rs` (347) y `gaming.rs` (544) se declaran en
`core/src/lib.rs`, tienen tests que pasan… y **cero** referencias en `src-tauri/src/lib.rs` y en
`app/src`. En toda la UI, la única aparición de «vram» es la columna de la ficha de investigación.

**Impacto.** 1.188 líneas que el usuario no puede usar y que **no se ejecutan nunca en producción**.
Peor: dan la impresión de que la funcionalidad existe, que es justo lo que el proyecto dice no hacer.

**Arreglo.** Conectar (5 comandos + 2 vistas) o, si no se va a conectar ya, sacarlas de `lib.rs` para
que no parezcan disponibles. Recomiendo conectar: el trabajo ya está pagado.

---

## E-09 · H: está al 97 % y es el disco del proyecto — **ALTA (operativa)**

**Evidencia.** `df -h /h` → `2.8T total, 2.7T usado, 106G disponible, 97 %`.

**Impacto.** Un `cargo build --release` completo más bundler consume 3–6 GB; el `target/` actual ya
pesa varios GB. Con 106 GB no hay crisis inmediata, pero un `npm install` + build + pruebas de
modelos puede acercarse al límite, y el propio proyecto documenta que H: no es para modelos.

**Arreglo.** `cargo clean` periódico o mover `target/` a otro disco con `CARGO_TARGET_DIR`
(p. ej. `D:\kukia_target`). Verificar antes cuánto ocupa hoy `target/` (no lo he medido para no
tocar nada).

---

## E-10 · El instalador está compilado con código de Fase 3 que no existe en él — **MEDIA**

**Evidencia.** Instalador y EXE tienen los hashes de Fase 2.1 (`a333bbb7…`, `859a8078…`), pero el
árbol de trabajo ya tiene tres módulos nuevos sin compilar. `git status` muestra 6 archivos fuera del
commit.

**Impacto.** Si alguien ejecuta la app instalada creyendo que tiene Fase 3, no la tiene. Y si se
compila ahora, sale una build con módulos que nadie usa.

**Arreglo.** Cerrar la conexión de Fase 3 **antes** de la siguiente compilación, y versionar
(0.1.1 o 0.2.0). Nunca dejar instalador y árbol desincronizados sin decirlo.

---

## E-11 · Catorce carpetas del esqueleto están vacías — **BAJA**

**Evidencia.** `agents/`, `providers/`, `models/`, `memory/`, `tools/`, `terminal/`, `mcp/`,
`research/`, `security/`, `gpu/`, `plugins/`, `config/`, `logs/`, `benchmarks/` → **0 archivos** cada
una.

**Impacto.** Confunden a cualquiera que abra el proyecto: parece modular y no lo es. `config/` y
`logs/` vacías además chocan con `.gitignore` (que las ignora), así que git ni las registra.

**Arreglo.** O se usan (mover ahí lo que corresponda) o se documentan en `README.md` como reservadas
con una nota de una línea. Sin prisa, pero no dejarlas aparentando contenido.

---

## E-12 · Warning de compilación pendiente — **BAJA**

**Evidencia.** `cargo check` → `warning: variable does not need to be mutable --> core\src\gaming.rs:153:9`
(`let mut score;`).

**Impacto.** Ninguno funcional. Importa porque el proyecto presume de build limpio.

**Arreglo.** Quitar el `mut`. Un segundo.

---

## E-13 · Tests que modifican variables de entorno globales — **BAJA**

**Evidencia.** `gaming::tests::laboratorio_crea_las_ocho_carpetas` hace `std::env::set_var`.
`merge` de tests corre en paralelo por defecto; `set_var` es global al proceso.

**Impacto.** Un test puede cambiar el comportamiento de otro que lea la misma variable. Hoy solo hay
un usuario de `KUKIA_GAMING_LAB`, así que no falla; es una bomba de relojería al añadir tests.

**Arreglo.** Guardar y restaurar el valor, o inyectar la ruta como parámetro en vez de leer el
entorno (mejor: `ensure_lab_at(path)`).

---

## E-14 · Rate limit de GitHub sin token — **MEDIA (hipótesis marcada)**

**Evidencia de código.** `gaming.rs` consulta 9 releases + 6 búsquedas + arXiv en `scan_all()`;
`research.rs` hace 3 fuentes. La API pública de GitHub permite **60 peticiones/hora sin token**.

**Estado.** **No medido** — no he ejecutado `scan_all()` para no gastar cuota ni salir a la red en
una auditoría de solo lectura. Lo marco como hipótesis, no como hecho.

**Impacto si se confirma.** Dos barridos seguidos (≈30 peticiones) más la vigilancia cada 6 h pueden
agotar la cuota y devolver `403`, que el radar interpretaría como «sin novedades».

**Arreglo.** Cachear por `ETag`, respetar `X-RateLimit-Remaining`, una sola llamada por repo y día,
y **avisar en la UI** cuando la cuota esté agotada en vez de mostrar cero resultados.

---

## E-15 · `perMachine` exige administrador — **BAJA (ya documentado)**

**Evidencia.** `src-tauri/tauri.conf.json` con `installMode: perMachine`; la instalación real se hizo
con `/S /CURRENTUSER`.

**Impacto.** Un usuario sin admin no puede instalar con doble clic. No rompe nada en tu equipo.

**Arreglo.** Cambiar a `currentUser` o documentar la elevación. Decidir antes de la 1.0.

---

## E-16 · Sin firma de código — **BAJA (ya documentado)**

**Impacto.** SmartScreen avisará en cualquier equipo que no sea el tuyo. Coste real de un
certificado: 200–400 €/año. No es un fallo, es una decisión pendiente.

---

## RESUMEN PRIORIZADO

| Prioridad | Hallazgo | Por qué primero |
|---|---|---|
| 1 | **E-01** permisos no aplicados | Es la base de todo lo que viene después; sin esto, los agentes son peligrosos |
| 2 | **E-03** timeout en comandos | Un `run_command` colgado deja la app inservible y no hay forma de pararlo |
| 3 | **E-02** auditoría muerta | Sin registro no hay confianza ni depuración posible |
| 4 | **E-04 / E-05** bypass y falso negativo | Cierran los dos agujeros que quedarían al arreglar E-01 |
| 5 | **E-08** módulos desconectados | Es trabajo ya hecho que no se usa |
| 6 | **E-07** mutex envenenado | Robustez; un pánico no debe obligar a reiniciar |
| 7 | **E-09** disco al 97 % | Operativo; barato de prevenir |
| 8 | E-06, E-10, E-11, E-12, E-13, E-14, E-15, E-16 | Limpieza, coherencia y decisiones de release |

> **Nada de este informe se ha arreglado**: la auditoría es de solo lectura. Los arreglos están
> descritos para que se autoricen uno a uno.
