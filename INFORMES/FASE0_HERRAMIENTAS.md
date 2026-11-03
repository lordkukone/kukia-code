# Fase 0 — Herramientas y capacidades reales (verificado 02/11/2026)

Todo lo que aparece como «disponible» se ha comprobado ejecutándolo o llamando a
su API en esta máquina. Lo que no responde se marca como tal, no se da por hecho.

## 1. Herramientas internas de KUKIA CODE

Registradas en `core/src/tools.rs` (`BASE`), expuestas por 143 comandos Tauri:

| id | Función | Permiso |
|---|---|---|
| `files.list` | Listar directorios | lectura |
| `files.read` | Leer archivo | lectura |
| `files.write` | Escribir archivo | escritura (allowlist + aprobación) |
| `terminal.run` | Ejecutar comando | ejecución |
| `memory.search` | Buscar en memoria | lectura |
| `memory.add` | Guardar recuerdo | escritura |
| `providers.list` | Listar proveedores | red |
| `gpu.stats` | Métricas GPU | lectura |
| `benchmark.run` | Medir modelo | red |
| `radar.check` | Radar de versiones | red |
| `research.scan` | Escanear fuentes | red |
| `research.autonomous` | Investigación autónoma | red |
| `model.catalog` | Catálogo universal | red |
| `jobs.queue` | Cola de trabajos | ejecución |
| `mcp.client` | Cliente MCP | herramientas externas |
| `security.policy` | Políticas por agente | lectura |

La escritura pasa por `core/src/permissions.rs`: política + aprobación explícita
+ ruta dentro de la allowlist, con comprobación de junctions para no escaparse.

## 2. Servidores MCP configurados (`~/.agents/mcp.json`)

| Servidor | Ejecutable | Estado real |
|---|---|---|
| filesystem | `npx.cmd` (existe) | configurado, **sin autorizar en KUKIA** (0 concesiones) |
| memory | `npx.cmd` | configurado, sin autorizar |
| sequential-thinking | `npx.cmd` | configurado, sin autorizar |
| fetch | `uvx.exe` (existe) | configurado, sin autorizar |
| time | `uvx.exe` | configurado, sin autorizar |
| sqlite | `uvx.exe` + BD copia (existe) | configurado, sin autorizar |

`mcp_grants` = 0 filas y `mcp_tools` = 0 filas: dentro de KUKIA ningún servidor
MCP está autorizado ni descubierto todavía. Es un permiso que no se concede solo.

## 3. APIs y servicios locales (probados con curl, timeout 3 s)

| Servicio | Puerto | Estado |
|---|---|---|
| Ollama | 11434 | **200** — versión 0.40.13 |
| KUKIA core | 8300 | **activo** (404 en `/`, es normal) |
| LM Studio | 1234 / 1235 | **cerrado** (000) |
| llama.cpp / llamaserver | 8080 / 8081 | cerrado |
| KoboldCpp | 5001 | cerrado, y **no se encontró ejecutable** |
| ComfyUI | 8188 / 8189 | cerrado |
| omniroute / bonsai | 20128 / 8060 | cerrado |

## 4. Binarios y librerías

| Herramienta | Estado |
|---|---|
`nvidia-smi` | disponible (`C:\WINDOWS\system32`)
`ollama` | disponible, 0.40.13
`python` | 3.12.9
`node` / `npx` | disponibles
`uvx` | disponible
`git` | disponible
`magick` | disponible
`rg` | **no instalado** (se usa grep)
`gguf` (Python) | disponible — lectura de metadatos GGUF probada (JEV-9B: `qwen35.context_length = 262144`)
`transformers` 5.13.0, `safetensors` 0.8.0, `numpy` 2.5.3, `psutil` 7.2.2 | disponibles
`llama-server.exe` | presente en LM Studio (`2.23.1 … 2.67.0`) y en `J:\models\maestro_ckpts\llm\bin`
`llama-gguf-split.exe` | presente (solo parte, no volcado de metadatos)
KoboldCpp | **no encontrado**

## 5. Datos y memoria

- BD de KUKIA: `%APPDATA%\KukiaCode\kukia.db` — 23 tablas; `memories` vacía,
  `agent_policies` 9, `approvals` 2, `messages` 207.
- Para el contexto **no hace falta** la memoria: los datos salen del motor.

## 6. Herramientas que se usarán para el contexto máximo

1. `GET /api/show` de Ollama → arquitectura, `*.context_length`, `num_ctx` del Modelfile.
2. `GET /api/tags` → tamaño real en disco.
3. `GET /api/ps` → contexto con el que está cargado y `size_vram` (detecta offload).
4. Parser propio de cabecera GGUF (`tools/gguf_contexto.py`) → contexto nativo de
   los modelos que **no** están en Ollama (LM Studio, M.2, descargas).
5. `nvidia-smi` → VRAM real por tarjeta antes y después.
6. Python → cálculo de caché KV con la fórmula ya validada en `core/src/control.rs`.
7. `OLLAMA_KV_CACHE_TYPE` (registro de Windows) → formato real de la KV (q4_0).

## 7. Limitaciones declaradas

- LM Studio está cerrado: su contexto (`/api/v0/models` → `max_context_length`)
  no se puede leer ahora. Los GGUF suyos sí se han leído del disco.
- Sin llama.cpp ni KoboldCpp en marcha, no hay segundo motor que validar en caliente.
- El contexto «estable» solo se puede afirmar **midiendo**; el cálculo de KV es
  una estimación con la arquitectura real, no una medición de carga.
