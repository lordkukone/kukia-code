# KUKIA CODE — TRASPASO COMPLETO (para otra IA) · FASE 2.1 CERRADA

Fecha: 01/11/2026 · Autor: Xoxete (para Kuki / LordKukone)
Estado: **Fase 2.1 validada. App instalada y funcionando en el equipo de Kuki.**

---

## 1. QUÉ ES KUKIA CODE

Aplicación de escritorio Windows 11, independiente de OpenCode y Freebuff:

- Interfaz propia (React) con 14 vistas.
- Núcleo de IA en Rust con selector universal de modelos.
- Conectores: Ollama (API nativa), LM Studio y cloud OpenAI-compatible.
- Memoria persistente en SQLite.
- Monitor GPU real (nvidia-smi) y benchmarks de tokens/s.
- Radar de actualizaciones (GitHub releases) y radar experimental de IA (arXiv, Hugging Face, GitHub).
- Sistema de aprobaciones con auditoría y backups.
- Instalador EXE (NSIS).

**Stack:** Tauri 2 + React 19 + TypeScript + Vite 7 + Rust + SQLite (rusqlite bundled).

---

## 2. UBICACIONES

| Qué | Dónde |
|---|---|
| Proyecto | `H:\KUKIA_CODE` |
| Laboratorio | `H:\KUKIA_RESEARCH\AI_EXPERIMENTAL` |
| Datos de usuario | `%APPDATA%\KukiaCode` (config, logs, backups, kukia.db) |
| App instalada | `C:\Program Files\KUKIA CODE\kukia-code.exe` |
| Informe de fase | `H:\KUKIA_CODE\INFORMES\FASE_2_1_VALIDACION.md` |
| Auditoría inicial | `H:\KUKIA_CODE\INFORMES\AUDITORIA_INICIAL.md` |
| Docs | `docs\ARQUITECTURA.md`, `docs\MANUAL.md`, `docs\ROADMAP.md`, `docs\SEGURIDAD.md` |
| Backups de código | `H:\KUKIA_CODE\backups\` |

---

## 3. ARTEFACTOS Y HASHES (reconstrucción final, 01/11/2026 11:02:01)

```
Instalador NSIS : H:\KUKIA_CODE\target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe
                  2.753.792 bytes
                  SHA256 A333BBB775FE08F67013F648469BC36597752CC8A818A9E39F5CEB289008E9CB

Binario release : H:\KUKIA_CODE\target\release\kukia-code.exe
                  9.063.936 bytes
                  SHA256 8A2AD47AC9057403EB7D7F31491DD30F30515350F483A3C493D7EF43B9F4F646

App instalada   : C:\Program Files\KUKIA CODE\kukia-code.exe
                  versión 0.1.0
                  SHA256 859A80782D325C2D456DFC9EB45CA93961A680EE99194DADE0D9EC66FD405DD6
```

El hash del instalado difiere del de `target\release` porque el bundler parchea el binario con la
información de bundle NSIS. Es normal en Tauri.

---

## 4. PRUEBAS EJECUTADAS (evidencia real)

| Prueba | Resultado |
|---|---|
| `npm run build` (tsc --noEmit + vite) | PASS — 46 módulos, JS 267,30 kB, CSS 11,28 kB |
| `cargo test -p kukia-core` | PASS — 22/22 unitarios |
| Tests de validación locales | PASS — 8/8 |
| Tests de validación con red | PASS — 5/5 |
| `npm run tauri:build` | PASS — 3m 40s |
| Arranque del EXE de release | PASS — ventana «KUKIA CODE», 42,2 MB RAM |
| Instalación NSIS | PASS — código 0 |
| Arranque de la app instalada | PASS — PID vivo, 45 MB RAM, cierre limpio |
| Persistencia tras reinicio | PASS — kukia.db 73.728 bytes intacta |
| Ollama | PASS — 81 modelos + chat real |
| LM Studio | PASS — 32 modelos + chat real `"OK"` en 531 ms |
| Cloud | NOT TESTED — sin `KUKIA_CLOUD_KEY` |
| GPU | PASS — RTX 5070 + RTX 5060 Ti, driver 621.01 |
| Research | PASS — 35 items, 10 alertas, informe 17.824 bytes |
| Seguridad | PASS — bloqueos, auditoría y backup verificados |
| Secretos en disco/logs | PASS — 0 coincidencias |

---

## 5. FALLOS ENCONTRADOS Y CORREGIDOS (4)

### 5.1 Backup silencioso — ALTA

`files_write` usaba `let _ = std::fs::copy(...)`. Si el backup fallaba, **el archivo se sobrescribía
igual**. Corregido: ahora la escritura se cancela si el directorio no se crea, si la copia falla o si
los tamaños no coinciden. Log: `backup verificado: <ruta>`.
Backup del archivo: `backups\lib.rs.20261101_104101.bak`.

### 5.2 LM Studio en otro puerto — MEDIA

El conector apuntaba fijo a `127.0.0.1:1234`; LM Studio estaba en **1235** y la app lo daba por
apagado. Corregido: sondeo de 1234 y 1235 con `/v1/models`, usando el que responda.
Backup: `backups\providers.rs.20261101_104722.bak`.

### 5.3 Respuesta vacía con modelos de razonamiento — ALTA

Los modelos con cadena de pensamiento devuelven `reasoning_content` y dejan `content` vacío; la app
mostraba nada con 16 tokens generados. Corregido: `content` vacío → `reasoning_content`
(OpenAI/LM Studio) o `thinking` (Ollama).

### 5.4 Pensamiento mezclado con la respuesta — BAJA

Nuevo evento `ChatEvent::Reasoning` en streaming; el chat lo muestra plegado («Pensamiento del
modelo (N caracteres)»). Interruptor nuevo en Configuración (`AppConfig.reasoning`):

- **ON por defecto** (orden de Kuki: «razonar tiene que razonar»).
- OFF: `think: false` en Ollama; `chat_template_kwargs.enable_thinking=false` +
  `reasoning_effort: none` en OpenAI-compatible, con reintento si el servidor devuelve 400.

---

## 6. RENDIMIENTO (medido, no supuesto)

`tests\bench_tok_s.ps1`, modelo `llama3:latest` (8B, 4,34 GB):

| Escenario | Generación |
|---|---|
| VRAM ocupada por LM Studio (2 modelos cargados) | **0,13 tok/s** |
| VRAM libre, opciones por defecto | **83,41 tok/s** |
| VRAM libre, `num_batch=1024` + `num_ctx=4096` + `num_gpu=99` | 82,75 tok/s |
| VRAM libre, mismas + recarga | 83,65 tok/s |

**Conclusión:** el cuello de botella era la **VRAM compartida entre dos motores**. Liberarla multiplicó
el rendimiento ×640. Las opciones avanzadas no aportan cuando el modelo cabe entero en la GPU; incluso
empeoran el arranque al forzar recarga.

Reglas derivadas:

1. Un solo motor con modelos cargados a la vez (Ollama **o** LM Studio).
2. El modelo entero en una GPU (`num_gpu=99`), sin offload a CPU.
3. No repartir un modelo entre GPU0 y GPU1.
4. Flash attention y KV cache q8: sin ganancia en esta máquina (ya activos).
5. Pendiente de medir: **speculative decoding** con modelo borrador (única vía clara para subir tok/s
   sin tocar calidad).

VRAM tras limpieza: GPU0 2.052/12.227 MiB · GPU1 11/16.311 MiB.

---

## 7. ENTORNO DEL EQUIPO

- Windows 11 Pro build 26300 x64. CPU i9-10900K, 64 GB RAM.
- GPU0 RTX 5070 12 GB (pantalla/juegos) · GPU1 RTX 5060 Ti 16 GB. Driver 621.01.
- CUDA 13.4 · PyTorch 2.12.1+cu130 · Rust 1.98.1 MSVC · MSVC 14.44 · Node 24.18 · npm 11.16.
- WebView2 Runtime 156.0.4314.50 (imprescindible para Tauri).
- Ollama 0.40.13 (81 modelos, ~519 GB) · LM Studio 1.19.3 (**32 modelos, 519,70 GB**, API en 1235).
- Discos: H: 111 GB libres · C: 111 · D: 512 · I: 26 · J: 29 · K: 19 · O: 710.

---

## 8. MÓDULOS DEL NÚCLEO (`core/src`)

- `config.rs` — AppConfig (default_provider, default_model, temperature, gpu_split [35,65], theme,
  cloud_base_url, cloud_model, has_cloud_key, research_auto, **reasoning**).
- `providers.rs` — Ollama (`/api/tags`, `/api/chat` NDJSON), LM Studio/cloud (`/v1/models`,
  `/v1/chat/completions` SSE), detección de puerto, `reasoning_content`, ChatEvent (token/reasoning/done/error).
- `memory.rs` — SQLite: sessions, messages, memories, approvals, benchmarks, compat, radar_cache,
  research_items, research_alerts, research_runs.
- `gpu.rs` — nvidia-smi + sysinfo.
- `agents.rs` — 8 agentes + personalizados en `config/agents.json`.
- `security.rs` — PowerShell solo con `approved=true`; lectura con límite de 2 MB.
- `logs.rs` — `%APPDATA%\KukiaCode\logs\kukia.log`.
- `radar.rs` — releases GitHub (ollama, llama.cpp) + fuentes manuales.
- `research.rs` — arXiv export API, HF `/api/models`, GitHub search; ficha de 12 campos; alertas;
  informe diario; descarga https ≤25 MB al laboratorio.

---

## 9. COMANDOS TAURI (camelCase desde JS)

```
get_system_info, config_get, config_set,
list_providers, list_models, chat, chat_stream (Channel<ChatEvent>),
gpu_stats, sessions_list, session_create, session_rename, messages_list, message_add,
memory_add, memory_search, agents_list, agent_create,
files_list, files_read, files_write (approved), run_command (approved),
benchmark_run, benchmarks_list, approvals_list,
radar_check, compat_add, compat_list,
research_scan, research_items, research_alerts, research_report, research_fetch, research_lab_path,
logs_tail
```

---

## 10. SEGURIDAD

- `files_write` y `run_command` exigen `approved=true`; sin ello devuelven error antes de tocar nada.
- Toda aprobación se registra en la tabla `approvals` (acción, detalle, estado, fecha).
- Backup verificado antes de sobrescribir; si falla, se cancela la escritura.
- `KUKIA_CLOUD_KEY` solo por variable de entorno; nunca en disco ni en logs.
- No hay autoactualización, ni instalación de software, ni modificación de Ollama/LM Studio/Pinokio/
  CUDA/drivers/modelos.
- No se toca KUKIA Network, ledger ni wallets.

---

## 11. LIMITACIONES REALES (sin maquillar)

1. Cliente MCP: no implementado (F6).
2. Investigación autónoma con ejecución: no (F9); el radar descubre e informa, no ejecuta.
3. Terminal pty (xterm.js): no; es ejecución PowerShell con aprobación.
4. Editor con resaltado: no; texto plano.
5. Actualizador firmado: no; el radar solo informa.
6. Reparto multi-GPU 35/65: perfil guardado, no reparto activo.
7. Gestión/descarga de modelos desde la app: no (F4).
8. Firma de código: no (SmartScreen avisará).
9. Cloud sin probar (sin clave).
10. Vistas «Tareas» y «Herramientas» son informativas, no funcionales (y lo dicen).
11. `jev-9b` genera basura con prompts cortos y `max_tokens` bajo: defecto del modelo.
12. Speculative decoding sin probar.

---

## 12. CÓMO COMPILAR

```powershell
cd H:\KUKIA_CODE
npm install
npm run tauri:dev     # desarrollo
npm run tauri:build   # instalador NSIS
cargo test -p kukia-core
powershell -File tests\smoke.ps1
powershell -File tests\instalador.ps1
powershell -File tests\bench_tok_s.ps1 <modelo>
```

---

## 13. REGLAS INNEGOCIABLES DEL PROYECTO

- Trabajar solo en `H:\KUKIA_CODE` y `H:\KUKIA_RESEARCH`.
- No modificar OpenCode, Freebuff, Pinokio, Ollama, LM Studio, Python, CUDA, drivers ni modelos.
- No borrar nada sin confirmación. Copiar → verificar → preguntar.
- Backup verificado antes de modificar archivos.
- No instalar dependencias globales ni servicios de pago.
- No tocar KUKIA Network, wallets, ledger ni tokenomics.
- No declarar nada operativo sin prueba real.

---

## 14. SIGUIENTE FASE PROPUESTA (F3)

1. Permisos por herramienta y por proyecto (allowlist de rutas escribibles, política por agente).
2. Auditoría consultable con filtros y exportación.
3. Recuperación de tareas interrumpidas con estado persistente.
4. **Guardia de VRAM**: avisar o impedir dos motores con modelos cargados (mayor ganancia medida).
5. Speculative decoding con medición antes/después.
