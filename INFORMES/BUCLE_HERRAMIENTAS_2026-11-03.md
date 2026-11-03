# Herramientas en el chat — 03/11/2026

Los modelos que declaran la capacidad `tools` **usan las herramientas de KUKIA
dentro del chat**: se les ofrece el esquema, se ejecutan sus llamadas bajo la
política del agente y el resultado vuelve al modelo hasta que contesta (máximo 6
pasos por turno). En esta máquina lo declaran **60 de 83** modelos.

## Catálogo (35 herramientas, 5 niveles)

### Lectura — `allow_read`, sin aprobación (18)

| Herramienta | Qué hace |
|---|---|
| `files_list` | lista una carpeta con tipo y tamaño |
| `files_read` | lee un archivo de texto |
| `files_read_many` | lee varios de una vez, separados |
| `files_find` | busca por nombre con `*` y `?`, en profundidad |
| `files_search` | busca un texto y devuelve `archivo:línea: contenido` |
| `files_info` | ficha: tipo, tamaño, fecha, líneas |
| `dir_tree` | árbol hasta N niveles |
| `memory_search` | busca en la memoria SQLite |
| `sessions_list` | conversaciones guardadas |
| `tools_list` | registro de herramientas con estado comprobado |
| `mcp_list` | servidores MCP, autorización y nº de herramientas |
| `logs_read` | últimas líneas del registro |
| `gpu_stats` | VRAM, uso, temperatura y potencia reales |
| `sys_info` | SO, CPU, núcleos, RAM y GPU |
| `disk_usage` | espacio libre por unidad |
| `models_loaded` | modelos cargados en Ollama con su contexto |
| `vram_report` | quién ocupa la VRAM y qué se puede liberar |
| `time_now` | fecha y hora local |

### Escritura — `allow_write` + aprobación (8)

`files_write`, `files_append`, `files_replace` (exige texto único),
`files_copy`, `files_move`, `files_delete` (carpeta con contenido exige
`recursivo`; nunca una raíz de unidad), `mkdir`, `memory_add`.

### Ejecución — `allow_shell` + aprobación (6)

`terminal_run` (PowerShell, 60 s), `git_status`, `git_diff`, `git_log`,
`processes_list`, `vram_unload`.

### Red — `allow_network` + aprobación (2)

`web_fetch` (solo https, texto limpio), `web_post` (JSON).

### MCP — `allow_tools` + aprobación (1)

`mcp_call`: solo con el servidor **autorizado** en KUKIA; si no, se rechaza
explicando que no se lanza un proceso de terceros sin permiso.

## Interfaz

Dos casillas en la barra del chat: **Herramientas** (encendida de serie) y
**Acciones** (apagada). Las acciones exigen además el permiso del agente en
Seguridad, y cada llamada queda en la tabla `audit` con su resultado.

## Pruebas ejecutadas

| Prueba | Resultado |
|---|---|
| `cargo check --all-targets` | **0 errores, 0 avisos** |
| `cargo test -p kukia-core --lib` | ver salida del día (incluye esquemas por nivel, comodines, búsquedas, escritura completa, git, procesos, red y MCP) |
| `npm run build` | OK |
| `cargo test --test validacion_chat_responde -- --ignored` | el modelo razona y **responde** |
| `cargo test --test validacion_herramientas_chat -- --ignored` | pide `files_read`, KUKIA lo ejecuta y **contesta con lo leído** |

Salida real del bucle:

```
--- herramientas pedidas ---
  files_read({"ruta":"C:\\Users\\kuki\\AppData\\Local\\Temp\\kukia_bucle_herramientas.txt"})
--- resultados ---
  files_read ok=true → la clave secreta es 4711
--- respuesta ---
La clave contenida en el archivo es **4711**.
```

## Incidente del mismo día (importante)

Un script de parcheo mío descartó el texto anterior a cada literal de
`ChatRequest` y **truncó** `core/src/catalog.rs` (950 → 180 líneas) y
`core/src/serve.rs` (600 → 85). Ambos eran *untracked* y sin copia: no había
backup en `backups/`, ni objeto en git, ni copia de sombra de `H:`.

Se reconstruyeron desde el contrato que sí sobrevivió (tipos de `api.ts`, usos de
`lib.rs`/`ecosystem.rs`, los tests originales del router y los literales del
binario de las 10:44). **No son idénticos a los originales**: `collect()` es
nuevo.

### Recomendación

`catalog.rs` y `serve.rs` siguen fuera de git. Conviene `git add` de `core/src/`
completo para que esto no pueda repetirse.
