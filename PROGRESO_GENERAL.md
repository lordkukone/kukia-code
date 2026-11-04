# KUKIA CODE — PROGRESO GENERAL

**Última actualización:** 01/11/2026 · **Autor:** Xoxete · **revisado 04/11/2026** (verificación independiente, § final)

## Identidad del proyecto

- Código: `H:\KUKIA_CODE` (repo git `master`, sin remoto)
- Laboratorio: `H:\KUKIA_RESEARCH`
- Datos de usuario: `%APPDATA%\KukiaCode` (`kukia.db`, `config\config.json`, `logs\`, `secrets.json`)
- Instalada: `C:\Program Files\KUKIA CODE\kukia-code.exe`
- Stack: Tauri 2 · React 19 · TypeScript · Vite 7 · Rust · SQLite

## Estado actual, en números

| Dato | Valor (01/11/2026) |
|---|---|
| Tests Rust | **98 pasan · 12 ignorados · 0 fallos** |
| Comandos Tauri | 62 |
| Vistas de interfaz | 18 en la barra + estados internos |
| Instalador | NSIS 0.1.0 · SHA-256 `509646014c93dd858c6f6fededccdb883c813423aaa2e6461bdf910faadf088f` |
| Rebuild completo | 2 min 23 s – 3 min 13 s (`npx tauri build`) |
| Instalación | `perMachine`, requiere administrador |

## Fases cerradas

| Fase | Contenido | Prueba |
|---|---|---|
| 1 | Núcleo: proveedores, memoria, GPU, radar, logs | 39 tests |
| 2 | MVP instalable, chat local, SQLite, benchmarks | 94→ tests, instalador NSIS |
| 2.1 | Validación de release, 4 fallos corregidos | informe + benchmark tok/s |
| 3.0 | Permisos aplicados en escritura y comando, timeout con cancelación, `audit_log` conectado | `validacion_fase30.rs` |
| 3A | Permisos por proyecto; tests fuera del log y de la base reales | `validacion_fase3a.rs` |
| 3.1 | VRAM Guard y Gaming Radar conectados a comandos y vistas | `validacion_gaming.rs`, VRAM real |
| 3.2 | Anime ↔ Real con ComfyUI (SDXL, IP-Adapter, img2img) | `validacion_anime_real.rs` |
| 3.3 | Commit, release e instalación verificada | `INFORMES/FASE_3_3_RELEASE.md` |
| 3.4 | Aviso automático de competencia de VRAM + CSP real + etiqueta Fase 3 | `INFORMES/FASE_3_4_AVISO_VRAM.md` |

## Lo que funciona hoy, comprobado en la app instalada

- Chat con Ollama, LM Studio y proveedor cloud (OpenAI-compatible), con streaming.
- Selector de proveedor: `opencode`, `freebuff`, `openai`, personalizado; clave en cascada
  (entorno → `secrets.json` → `auth.json` de OpenCode en solo lectura) y nunca mostrada.
- VRAM Guard midiendo GPUs y motores reales, **con aviso automático** cuando dos motores
  tienen modelos cargados a la vez.
- Gaming Radar con fichas y alertas por nivel.
- Anime ↔ Real contra ComfyUI real, con historial y previsualizaciones.
- Seguridad: política de permisos por agente y por proyecto, timeout y cancelación de comandos,
  auditoría consultable y exportable.
- 9 políticas de agente sembradas en la base al arrancar.

## Lo que NO está

- ~~Tool calling en ningún proveedor~~ → **corregido 04/11/2026: el tool calling existe desde el 03/11** (35 herramientas, bucle verificado).
- Almacenamiento seguro de claves (hoy `secrets.json` en claro, sin DPAPI).
- Coste/presupuesto de nube, router automático local+cloud, kill switch de nube.
- Multi-PC real: el diseño existe (`INFORMES/ARQUITECTURA_MULTI_PC.md`), el worker **no**. El segundo PC ya está cableado y accesible (§ verificación).
- Updater firmado, firma de código, notificaciones del sistema.
- Memoria semántica con embeddings (la búsqueda es `LIKE`).
- Terminal pty real y editor con resaltado. ~~MCP~~ → **corregido 04/11/2026: el cliente MCP (stdio y http) existe desde el 03/11**.

## Mapa de documentos

| Para qué | Dónde |
|---|---|
| Qué está hecho y qué no, por clase | `INFORMES/FUNCIONALIDADES_PENDIENTES.md`, `MATRIZ_FUNCIONALIDADES.md` |
| Auditoría de la ampliación cloud | `INFORMES/AUDITORIA_CLOUD_OMEGA.md` |
| Cómo seguir mañana | `CONTINUAR_TRABAJO.md` |
| Historial de cambios | `CHANGELOG.md` |
| Plan de release | `PLAN_RELEASE.md` |
| Arquitectura y seguridad | `docs/ARQUITECTURA.md`, `docs/SEGURIDAD.md` |

## Verificación independiente — 04/11/2026

Auditoría de solo lectura sobre el equipo real, no sobre estos informes. **No se tocó OpenCode**
(su `auth.json` solo se leyó, como siempre), ni Ollama, ni LM Studio, ni `I:\CEREBRO`, ni ningún driver.

### El binario instalado no es el de la release del 01/11

| Dato | Valor |
|---|---|
| Ruta | `C:\Program Files\KUKIA CODE\kukia-code.exe` |
| Tamaño | **12.371.456 B** |
| Instalado | 01/11/2026 18:01 · **modificado 02/11/2026 21:30** |
| Versión declarada | **0.1.0** (la misma que la release del 01/11) |
| SHA-256 | `a90a7d89f9efc54f98626d3b44370930f52a1d3ef1f3ef3645aede52c8fdb4f5` |

El exe documentado en `PLAN_RELEASE.md` pesaba 9.812.992 B. Este es 2,5 MB mayor, es del 02/11 a las
21:30 y **contiene las herramientas**: dentro del binario aparecen `files_read`, `files_write`,
`mcp_call`, `allow_shell`, `vram://competencia` y `LoRA`. **No es** la build «solo chat» del 01/11.

**El problema de fondo:** tres binarios distintos declaran los tres `0.1.0`, y los `SHA256SUMS` de las
releases solo cubren los instaladores. Mirando la interfaz es imposible saber qué build está instalado.

### Por qué el modelo por defecto no arranca

`config.json` tiene `default_model = kukia-gemma4-q5:latest` (19,13 GB) y `gpu_split = [30, 70]`.
Log del 03/11 a las 08:39:

```
[warn] carga rechazada ollama/kukia-gemma4-q5:latest: archivo 17.82 GB + 12 % + KV(4096) ≈ 20.21 GB · GPU1 (RTX 5060 Ti): 15.45 GB libres → NO cabe
```

Con `[30, 70]` el 70 % se manda a la 5060 Ti (16 GB) y la 5070 (12 GB) queda casi ociosa. **Arreglo
pendiente:** `[50, 50]` (o `[45, 55]`) para usar los 27,9 GB reales, o un modelo que quepa en una sola
tarjeta. `GET 127.0.0.1:11434/api/ps` devuelve vacío: ahora mismo no hay ningún modelo cargado.

### La app está sana (esto descarta un fallo de instalación)

- `logs\kukia.log`: **26 arranques**, el último **04/11/2026 11:32**, con «catálogo universal: 125
  modelos» y liberaciones de VRAM correctas. **Cero errores.**
- `kukia.db`: `integrity_check ok` · **9 políticas** · 7 sesiones · 237 mensajes · **817 memorias** ·
  2 filas de `audit`.
- `entrenador\` con dos datasets del 03/11 (13:46 y 13:48): el entrenador de LoRA ya se usó en ese build.
- `secrets.json` = **21 B** (vacío) y en claro: **C0 sigue abierta**. La clave cloud no sale de ahí
  (`has_cloud_key: true` viene de la cascada entorno → `auth.json` de OpenCode, leído sin modificarlo).

### Red: el segundo PC vuelve a estar cableado

| Dato | Valor |
|---|---|
| Master (este PC) | `KUKIPCWIN11` · Ethernet **192.168.50.1** · Wi-Fi 192.168.1.11 · Tailscale 100.116.82.33 |
| Worker | **DESKTOP-6CK4U6H** · **192.168.50.2** · MAC `D8-BB-C1-97-28-FB` (Micro-Star / MSI) |
| Enlace | cable directo PC-a-PC, **< 1 ms**, 0 % de pérdida |
| Puertos abiertos | 135, 139, 445, **3389 (RDP)**, **5985 (WinRM)** |
| Puertos cerrados | **11434** (Ollama), **1234/1235** (LM Studio), **8791** (worker KUKIA) |

La subred `192.168.50.0/24` son rutas «En vínculo» por el adaptador Ethernet: no pasa por el router. El
worker **no está en Tailscale**. Está encendido y accesible, pero **sin motores de IA y sin
`kukia-worker.exe`**: lo siguiente es **M0-B** (inventario de solo lectura), que necesita credenciales por
WinRM o RDP. **La identidad i7-12700 + RTX 2070 Super no está confirmada:** lo único verificado es una
placa MSI con Windows.

### Hardware del master (medido, no supuesto)

i9-10900K (10C/20T) · 64 GB (2×32 GB @ 3600 MT/s) · ROG MAXIMUS XII APEX · **RTX 5070 12.227 MiB** +
**RTX 5060 Ti 16.311 MiB** (driver 621.01) = **27,9 GB de VRAM** · Windows 11 Pro build 26300 ·
`C:` 94,6 GB libres · `H:` 91,6 GB libres.

> Trampa medida: WMI (`Win32_VideoController`) informa «4,0 GB» por GPU por desbordamiento de
> `AdapterRAM`. Los números válidos son los de `nvidia-smi`.

### Repositorio

`github.com/lordkukone/kukia-code` es público · rama `master` · último commit **`eeccf72`** (03/11/2026
13:53) · tag `prueba-20261103c`. `core/src/catalog.rs` (1.136 líneas) y `core/src/serve.rs` (488) **ya
están versionados**: el incidente del 03/11 no puede repetirse.

### Resultado medido de la carga — 04/11/2026 13:18

Con `gpu_split = [50, 50]` el modelo **sí carga**, y no en una tarjeta: se reparte entre las dos.

| Medición | Valor |
|---|---|
| Tiempo de carga | **52 s** |
| `size` / `size_vram` | 19.649.601.205 B (**18,3 GiB**) — 100 % en VRAM |
| Reparto real (`nvidia-smi`) | RTX 5070 **11.413 MiB** + RTX 5060 Ti **10.687 MiB** |
| Respuesta | `200` · devuelve `OK` con `done_reason: stop` |
| Tras descargarlo | 5070 2.728 MiB · 5060 Ti 226 MiB (como estaba) |

**Hipótesis abierta (no verificada en el código):** el aviso del 03/11 comparaba los 20,21 GB contra
**una sola** GPU (GPU1, 15,45 GB libres), como si el modelo entero tuviera que caber en una tarjeta en
vez de repartirse según `gpu_split`. Con `[50, 50]` el reparto real es 11,4 + 10,7 GB y entra.
Conviene revisar `vram::fits()` para confirmar si el cálculo es por tarjeta o por modelo completo.

### C0 cerrado — claves cifradas con DPAPI (04/11/2026)

| Pieza | Estado |
|---|---|
| Cifrado | `CryptProtectData` por usuario, FFI directa a `crypt32` (**sin dependencias nuevas**) |
| Formato | `secrets.json` pasa a `{"v":2,"dpapi":"<hex>"}` escrito de forma atómica |
| Compatibilidad | sigue leyendo el JSON en claro antiguo, plano y con `providers` |
| Migración | al arrancar (en `setup`): copia `secrets.json.legacy.bak` y reescribe cifrado; si la copia falla, no toca nada |
| API | `core/src/secrets.rs`: `leer`, `buscar`, `guardar`, `borrar`, `migrar`, `esta_cifrado`, `pendiente_de_migrar` |

Pruebas ejecutadas (binario release del 04/11 13:46, con `KUKIA_DATA_DIR` en carpeta temporal):

```
cargo test -p kukia-core --lib secrets    -> 7 pasan (incluye DPAPI real: cifrar y descifrar)
cargo test -p kukia-core --lib buildinfo  -> 2 pasan
cargo build --release -p kukia-code       -> EXIT 0 (3m 26s)
app arrancada con secrets.json en claro   -> {"v": 2, "dpapi": "01000000d08c9ddf..."}
   - lineas del archivo con la clave en claro: 0
   - copia secrets.json.legacy.bak: creada
   - log: "secrets.json migrado a DPAPI"
```

Lo que **no** esta: la vista CLOUD CREDENTIALS (alta, prueba de conexion, revocacion). El almacen
cifra y la API existe, pero la pantalla sigue siendo el texto de Configuracion con el aviso de DPAPI.

### Numero de build visible (04/11/2026)

Antes la cabecera decia `v0.1.0 · Fase 3`, identico en el binario del 01/11, el del 02/11 y el del
03/11: era imposible saber cual se estaba ejecutando. Ahora muestra
`v0.1.0 · Fase 3 · build AAAA-MM-DD HH:MM`, leido del ejecutable en marcha por el comando `app_info`
(`core/src/buildinfo.rs`). Es la fecha de modificacion del binario, no un numero de compilacion
grabado (eso pediria `build.rs`), pero cambia con cada compilacion y cada instalacion y no miente.

### Corregido el binario del 04/11: hay que compilar con `tauri build` (04/11/2026, 15:38)

Al ir a verificar el chat mirando la ventana real (no solo el hash) aparecieron dos fallos del build
publicado como `build-20261104-1346`:

| Sintoma | Causa |
|---|---|
| Ventana de 157x25 en vez de 1440x900 | el ejecutable esta compilado en **modo dev** |
| Dentro se ve "Vaya... no se puede obtener la pagina" | intenta cargar `http://localhost:1420` (`devUrl`) en vez del frontend embebido |

`cargo build --release` **no** produce un binario distribuible en Tauri 2: no activa
`custom-protocol`, de modo que `tauri::generate_context!()` no embebe `dist/` y la webview pide el
servidor de Vite. El comando correcto es `npx tauri build` (o `npm run tauri:build`).

Medido, para que no vuelva a pasar:

| Binario | Ventana inicial | Contenido |
|---|---|---|
| instalado 02/11 21:30 | 1295x735 | interfaz OK |
| release defectuoso `build-20261104-1346` | 157x25 | pagina de error de WebView2 |
| release bueno `build-20261104-1538` | 1295x735 | interfaz completa, cabecera con build |

El 1295x735 no es un fallo: en esta pantalla de 1280x720 es el recorte de los 1440x900 pedidos.

**Regla**: no distribuir nunca un `target/release/kukia-code.exe` que venga de `cargo build`, y
comprobar el binario abriendo la ventana, no solo con el hash. La captura fiable es `PrintWindow`
con `PW_RENDERFULLCONTENT`: `CopyFromScreen` copia lo que se ve en pantalla y devuelve otra ventana.
