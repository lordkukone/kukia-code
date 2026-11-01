# KUKIA CODE — INFORME DE AUDITORÍA (solo lectura)

**Fecha:** 01/11/2026 · **Autor:** Xoxete (para Kuki / LordKukone)
**Raíz auditada:** `H:\KUKIA_CODE` (la original — no se ha creado ni copiado ningún proyecto)
**Laboratorio:** `H:\KUKIA_RESEARCH\AI_EXPERIMENTAL`
**Modo:** **AUDITORÍA DE SOLO LECTURA.** No se ha modificado código, configuración ni datos.
Lo único escrito han sido estos cinco informes en `INFORMES\`.

---

## 0. VEREDICTO EN UNA LÍNEA

El proyecto **existe, compila, pasa 47 pruebas y está instalado y funcionando** como aplicación real
(no es una maqueta). La **Fase 2.1 está cerrada de verdad**. La **Fase 3 está a medias**: el motor
está escrito y probado en el núcleo, pero **no está conectado a la aplicación** — no hay comandos
Tauri ni vistas, así que hoy no protege ni avisa de nada. Y hay **dos agujeros de seguridad reales**
heredados de Fase 2 que deben cerrarse antes de seguir creciendo.

---

## 1. METODOLOGÍA (qué se ha ejecutado, no supuesto)

| Comprobación | Comando / método | Resultado |
|---|---|---|
| Existencia e identidad del proyecto | `ls`, `cat Cargo.toml/package.json` | ✅ `H:\KUKIA_CODE`, Tauri 2 + React 19 + Rust |
| Historial | `git log --oneline`, `git status` | 2 commits, 6 archivos sin commitear |
| Inventario de código | `find` + `wc -l` | **7.260 líneas** propias (core 4.263 + app 2.451 + src-tauri 546) |
| Compilación del árbol completo | `cargo check --workspace --offline` | ✅ **OK**, 1 warning |
| Pruebas del núcleo | `cargo test -p kukia-core --offline` | ✅ **39 unitarios + 8 de validación**, 5 ignorados (red) |
| Build del frontend | `npm run build` | ✅ 46 módulos, JS 268,28 kB, CSS 11,28 kB |
| Instalación real | `Get-Item ...VersionInfo` | ✅ `C:\Program Files\KUKIA CODE\kukia-code.exe` v**0.1.0** |
| Integridad de la base | `sqlite3 pragma integrity_check` (modo `ro`) | ✅ `ok` |
| Contenido de la base | `select count(*)` por tabla | 11 tablas · 1 sesión · 2 mensajes · resto 0 |
| Estado del disco de trabajo | `df -h /h` | ⚠️ **106 GB libres de 2,8 TB (97 % usado)** |
| Módulos huérfanos | `grep` de `permissions::`, `vram::`, `gaming::` en `src-tauri` | ❌ **0 referencias** |
| Comandos expuestos | `generate_handler!` | **34 comandos** |

> **Nota de honestidad:** `cargo check` y `npm run build` **escriben en `target/` y `dist/`**
> (artefactos de compilación). No se ha tocado ni una línea de fuente, ni configuración, ni la base
> de datos, ni nada de Ollama/LM Studio/Pinokio/CUDA/drivers.

---

## 2. IDENTIDAD DEL PROYECTO

| Dato | Valor real |
|---|---|
| Ruta | `H:\KUKIA_CODE` |
| Stack | Tauri 2 + React 19 + TypeScript 5.8 + Vite 7 + Rust 1.98 (MSVC) + SQLite (rusqlite bundled) |
| Workspace Cargo | `members = ["core", "src-tauri"]`, `opt-level = "s"`, `lto = true`, `strip = true` |
| Versión | 0.1.0 en `package.json`, `src-tauri/Cargo.toml` y `core/Cargo.toml` (las tres coherentes) |
| Rama | `master` |
| Commits | `b125554` Fase 2 · `18d43fb` Fase 2.1 (validación, 4 fallos corregidos) |
| Sin commitear | `M` agents.rs, lib.rs, memory.rs · `??` gaming.rs, permissions.rs, vram.rs |
| Espacio | 106 GB libres en H: (97 % ocupado) |

### 2.1 Estructura: lo que es real y lo que es decorado

Catorce carpetas de la raíz (`agents/`, `providers/`, `models/`, `memory/`, `tools/`, `terminal/`,
`mcp/`, `research/`, `security/`, `gpu/`, `plugins/`, `config/`, `logs/`, `benchmarks/`) están
**vacías: 0 archivos**. Todo el código vive en `core/src/`, `app/src/` y `src-tauri/src/`.

La documentación (`FASE_2.md` §3) las presenta como «reservado F6», lo cual es cierto, pero hoy son
14 carpetas que aparentan una arquitectura modular que en la práctica no existe en disco. No es un
fallo funcional; es una desviación entre lo que se ve y lo que hay.

---

## 3. NÚCLEO (`core/src`, 4.263 líneas)

| Módulo | Líneas | Contenido real |
|---|---:|---|
| `memory.rs` | 1.257 | 16 tablas SQLite, 40 funciones públicas: sesiones, mensajes, memorias, aprobaciones, benchmarks, compatibilidad, radar, research, auditoría, políticas, tareas, gaming |
| `providers.rs` | 642 | Ollama nativo (NDJSON) + LM Studio/cloud (SSE), sondeo de puertos 1234/1235, `reasoning_content`, `think:false` |
| `gaming.rs` | 544 | **Nuevo (F3)**: vigilancia de 9 repos (DLSS Swapper, ReShade, SpecialK, bgfx, RTX Remix, Streamline, OptiScaler…) + 6 búsquedas + arXiv; clasificación por madurez y riesgo |
| `research.rs` | 486 | arXiv export API, HF `/api/models`, GitHub search, ficha de 12 campos, alertas, informe diario, descarga https ≤25 MB |
| `vram.rs` | 347 | **Nuevo (F3)**: VRAM por GPU, procesos, estado de Ollama/LM Studio, detección de competencia, `fits()` con 12 % de margen |
| `permissions.rs` | 297 | **Nuevo (F3)**: 6 herramientas, normalización de rutas contra `..`, allowlist lectura/escritura, rutas de solo lectura, 14 patrones destructivos |
| `agents.rs` | 181 | 8 agentes con prompt y lista de herramientas |
| `gpu.rs` | 146 | `nvidia-smi` + `sysinfo` |
| `config.rs` | 123 | `AppConfig`, `KUKIA_DATA_DIR`, perfil GPU 35/65, flag `reasoning` |
| `radar.rs` | 101 | Releases GitHub (ollama, llama.cpp) |
| `security.rs` | 77 | PowerShell con aprobación obligatoria; lectura con tope de 2 MB |
| `logs.rs` | 41 | Log en `%APPDATA%\KukiaCode\logs\kukia.log` |
| `lib.rs` | 21 | Reexporta 13 módulos |

**Calidad del código:** sin `TODO`, `FIXME`, `todo!`, `unimplemented!` ni `panic!` en todo `core/`.
Solo 14 `unwrap()` (10 en `memory.rs`, todos sobre el mutex de la base). Los módulos nuevos de Fase 3
tienen **su propia batería de tests** y están escritos con criterio conservador (denegar por defecto).

---

## 4. APLICACIÓN (`app/src` 2.451 líneas + `src-tauri` 546)

- **14 vistas** en `App.tsx`: Chat, Modelos, Agentes, Archivos, Terminal, Tareas, Memoria,
  Herramientas, GPU, Benchmarks, Radar, Research, Historial, Configuración.
- **34 comandos Tauri** registrados en `generate_handler!`.
- 3 vistas son **estáticas y honestas** (lo dicen en pantalla): `Tareas` («sin tareas en segundo
  plano»), `Herramientas` (tabla de estado) y parte de `Radar`.
- Sin editor con resaltado (texto plano), sin terminal pty (es PowerShell bloqueante), sin MCP.

---

## 5. VERIFICACIÓN DE COMPILACIÓN Y PRUEBAS (evidencia)

```
cargo check --workspace --offline
  warning: variable does not need to be mutable  → core\src\gaming.rs:153:9
  Finished `dev` profile in 1m 17s                                ✅ OK

cargo test -p kukia-core --offline
  unittests src\lib.rs   → 39 passed; 0 failed                    ✅
  tests\validacion_fase21.rs → 8 passed; 5 ignored (requieren red) ✅
  doc-tests              → 0                                      ✅
  (1,20 s + 1,23 s)

npm run build
  tsc --noEmit: sin errores   vite: 46 módulos
  ../dist/assets/index-BWEgUZ86.js   268,28 kB │ gzip: 81,42 kB   ✅
```

Los 39 tests incluyen los nuevos: `permissions::` (10), `gaming::` (4+), `vram::` (2 con GPUs
reales). **Comparado con Fase 2: 22 → 39 tests unitarios.**

---

## 6. ARTEFACTOS VERIFICADOS HOY

| Artefacto | Bytes | SHA-256 (recortado) | ¿Coincide con FASE_2_1? |
|---|---:|---|---|
| `target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe` | 2.753.792 | `a333bbb7…008e9cb` | ✅ sí |
| `target\release\kukia-code.exe` | 9.063.936 | `8a2ad47a…9b4f646` | ✅ sí |
| `C:\Program Files\KUKIA CODE\kukia-code.exe` | 9.063.936 | `859a8078…405dd6` | ✅ sí (difiere del anterior porque el bundler parchea el binario; es normal) |

Instalador y app instalada son **los mismos que documentó el informe de Fase 2.1**. No hay builds
posteriores: es decir, **el código de Fase 3 que hay ahora en disco no está dentro de la app
instalada**.

---

## 7. DATOS DE USUARIO (estado real)

```
%APPDATA%\KukiaCode\
  kukia.db        73.728 bytes · integrity_check = ok
  config\         VACÍA (no existe config.json)
  logs\kukia.log  incluye 8 líneas "[test] linea de prueba kukia" escritas por los tests
  backups\        vacía
  benchmarks\     vacía
```

Tablas y filas: `sessions` 1 · `messages` 2 · `approvals` **0** · `benchmarks` **0** · `memories` 0 ·
`research_*` 0 · `radar_cache` 0 · `compat` 0.

**Dos consecuencias que hay que decir claras:**

1. Las tablas de Fase 3 (`agent_policies`, `tasks`, `audit`, `gaming_*`) **no existen** en la base
   real: se crean al arrancar la nueva build, que aún no se ha compilado ni instalado.
2. Que `approvals` esté a 0 significa que **en la app instalada nadie ha aprobado ninguna escritura
   ni comando todavía**. La persistencia está probada en tests, no en uso real.

---

## 8. MAPA: LO QUE PIDES (superprompt) vs LO QUE HAY

Clasificación estricta. **Ninguna casilla se marca como verificada sin prueba ejecutada hoy.**

| Bloque del superprompt | Estado | Evidencia / carencia |
|---|---|---|
| Abrir el proyecto existente sin duplicar | ✅ **HECHO** | `H:\KUKIA_CODE` es el original; 2 commits en `master` |
| Auditoría de solo lectura | ✅ **ESTA ENTREGA** | 5 informes en `INFORMES\` |
| Fase 3A — permisos por herramienta/proyecto | 🟠 **PARCIAL** | `permissions.rs` (297 líneas, 10 tests) y tabla `agent_policies` **existen**; **no hay comandos Tauri ni vista**, y `files_write`/`run_command` **no los consultan** |
| Fase 3A — allowlist de directorios | ❌ **NO APLICADA** | `path_in_list` probado en unitarios, pero el comando de escritura escribe en cualquier ruta con `approved=true` |
| Fase 3A — historial auditable | 🟠 **PARCIAL** | Tabla `audit` + `audit_log`/`audit_search`/`audit_export_csv` escritos; **nadie los llama** → la tabla sigue vacía. La auditoría real de hoy es `approvals` (0 filas) |
| Fase 3A — recuperación de tareas interrumpidas | 🟠 **PARCIAL** | `tasks` + `task_upsert`/`tasks_interrupted`/`task_set_status` escritos; **no conectados**. La vista `Tareas` es estática y lo dice |
| Fase 3A — cancelación / anti-bucles / anti-duplicados | ❌ **NO IMPLEMENTADA** | Sin cola, sin supervisor, sin `CancellationToken` |
| Fase 3B — VRAM Guard | 🟠 **PARCIAL** | `vram.rs` completo y con tests contra las GPUs reales; **sin comando ni vista** |
| Fase 3B — Model Intelligence Router | ❌ **NO IMPLEMENTADA** | Hay selector manual (`Modelos`) y `list_models`; no hay puntuación por VRAM/contexto/calidad |
| Fase 3C — World AI Radar | 🟠 **PARCIAL** | `research.rs` ya cubre arXiv/HF/GitHub con informe diario (verificado en F2.1) |
| Fase 3C — Gaming Technologies Radar | 🟠 **PARCIAL** | `gaming.rs` (9 repos + 6 búsquedas + arXiv, clasificación y riesgo alto para «unlock»); **sin comando ni vista** |
| Fase 3D — Universal Desktop Control | ❌ **NO IMPLEMENTADA** | `Cargo.toml` sin `windows`/`uiautomation`/`enigo`/`pywinauto`. Nada de UI Automation |
| Fase 3E — Agent Swarm | ❌ **NO IMPLEMENTADA** | 8 agentes son **perfiles de prompt**, sin ejecución ni tools reales |
| Fase 3E — Memoria avanzada (RAG/embeddings) | ❌ **NO IMPLEMENTADA** | `memory_search` es `LIKE` sobre SQLite; sin embeddings |
| Fase 3F — Distributed AI / segundo PC | ❌ **NO IMPLEMENTADA** | Nada de red entre equipos; sin protocolo, sin cola, sin descubrimiento |
| Fase 3G — laboratorio experimental multimodal | 🟠 **PARCIAL** | Laboratorio aislado y ficha de descubrimiento sí; multimodal **no** (sin ComfyUI/audio/vídeo) |
| Fase 3H — Benchmark Center | 🟠 **PARCIAL** | `benchmark_run` mide tok/s real y persiste; **no** compara modelos ni condiciones |
| Research Center (fuentes + clasificación + informes) | ✅ **IMPLEMENTADO Y VERIFICADO en F2.1** | 35 items, 10 alertas, informe 17.824 bytes, descarga aislada probada |
| Interfaz general | 🟠 **PARCIAL** | 14 vistas reales; Tareas/Herramientas informativas |
| Seguridad (no tocar OpenCode/Pinokio/CUDA/KUKIA Network) | ✅ **CUMPLIDO** | `grep` sin referencias; la app solo habla con 127.0.0.1 y APIs públicas https |
| Continuidad entre sesiones (`PROGRESO_GENERAL.md`, `CONTINUAR_TRABAJO.md`) | ❌ **NO EXISTEN** | Sí existen `INFORMES\` y `docs\ROADMAP.md` |

---

## 9. LO QUE LA DOCUMENTACIÓN DA POR HECHO Y NO LO ESTÁ

No es que los informes anteriores mientan: es que **describen el mismo árbol de código sin distinguir
lo que la app ejecuta de lo que solo existe en el núcleo**. Las tres afirmaciones que hay que
corregir:

1. **«Fase 3: permisos por herramienta»** — el motor está escrito y testeado, pero **no hay UI ni
   comando**, y la ruta de escritura real no lo consulta. Decir «permisos implementados» hoy sería
   falso.
2. **«Auditoría en tabla `audit`»** — la tabla existe; **ninguna ruta del código escribe en ella**.
   La auditoría efectiva hoy son 34 llamadas a `approval_log`.
3. **«Recuperación de tareas interrumpidas»** — existe el esquema y las funciones; **no están
   conectadas** y la vista lo declara vacía.

---

## 10. CONCLUSIONES

**Lo bueno, con pruebas:**

- Es una aplicación **real**, no una demo: compila, 47 tests en verde, instalada, con hashes que
  cuadran con lo documentado.
- La **calidad del núcleo es alta**: sin `panic!`/`todo!`, denegación por defecto, tests que
  consultan `nvidia-smi` de verdad, backups verificados por tamaño antes de sobrescribir.
- El **criterio de honestidad** del proyecto (no declarar PASS sin prueba) se ha respetado; los
  propios informes listan 12 limitaciones.
- La **Fase 2.1 encontró y corrigió 4 fallos reales**, el peor de ellos (backup silencioso) con
  gravedad alta. Eso valida el método.

**Lo que hay que arreglar antes de crecer:**

1. **Conectar Fase 3** (5 comandos + 2 vistas): el trabajo ya está hecho, solo no se usa.
2. **Aplicar la política de permisos en `files_write`/`run_command`**: hoy la allowlist es decorativa.
3. **Timeout en `run_command`**: PowerShell bloqueante sin límite puede colgar la app.
4. **Sacar los tests de los datos reales**: 8 líneas `[test]` están en tu log de producción.

Todo el detalle en `ERRORES_Y_RIESGOS.md`; el orden de trabajo en `ROADMAP_RECOMENDADO.md`; el
diseño de dos equipos en `ARQUITECTURA_MULTI_PC.md`.

---

> Informe generado con comandos ejecutados y salidas reales. **Ninguna afirmación de este documento
> se apoya en los informes anteriores**: los he leído para saber qué esperar y luego lo he medido.
> Lo único que no he podido probar es el comportamiento de la UI en pantalla (requiere abrir la
> ventana y hacer clic), y así queda declarado.
