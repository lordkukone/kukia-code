# KUKIA CODE — ECOSISTEMA UNIVERSAL DE HERRAMIENTAS (F6 + F9)

**Fecha:** 02/11/2026
**Estado:** cliente MCP implementado (con permiso por servidor), investigación autónoma
implementada, registro y TOOL CENTER funcionando con datos reales.

---

## 1. Auditoría previa (lo que había y no se ha duplicado)

| Pieza | Estado encontrado |
|---|---|
| `files.list/read/write`, `terminal.run`, `memory.search/add`, `providers.list`, `gpu.stats`, `benchmark.run`, `radar.check` | Implementadas y funcionando |
| `mcp.client` | **No existía**. La carpeta `mcp/` estaba vacía |
| `research.autonomous` | **No existía**. Solo `research.scan` (arXiv/HF/GitHub) |
| Vista Herramientas | Una tabla **estática** con 12 filas escritas a mano en el cliente |
| Permisos | `permissions.rs` ya tenía `Tool` con lectura/escritura/ejecución/red/descargas/herramientas externas |

Conclusión: no había cliente MCP, ni registro real, ni investigación autónoma, y la vista
mentía por omisión al mostrar «pendiente F6/F9» como texto fijo.

## 2. MCP client (`core/src/mcp.rs`, ~460 líneas)

**Descubre** servidores en `.agents/mcp.json` (proyecto y padre), `~/.agents/mcp.json` y
`%APPDATA%\KukiaCode\config\mcp.json`. No modifica ninguno.

**Transportes:** `stdio` (JSON-RPC por líneas, handshake `initialize` + `notifications/initialized`)
y `http` (POST JSON-RPC, con respuesta JSON o SSE).

**Operaciones:** listar servidores, autorizar/revocar, conectar y listar herramientas
(`tools/list`), ejecutar una herramienta (`tools/call`), y guardar el resultado en SQLite
(`mcp_grants`, `mcp_tools`).

**Seguridad, que es lo importante aquí:**

- Un servidor `stdio` es un proceso de terceros (`npx`, `uvx`). **No se lanza sin autorización
  explícita guardada en base de datos**, y la UI pide confirmación mostrando el comando exacto.
- `mcp_call` vuelve a comprobar el permiso en el servidor, no solo en la interfaz.
- `kill_on_drop` + timeout de 25 s: un servidor colgado no deja procesos vivos.
- Los `.cmd`/`.bat` de Windows se ejecutan vía `cmd /C` (si no, `CreateProcess` falla).
- Sin ventana de consola (`CREATE_NO_WINDOW`).

## 3. Investigación autónoma (`research::autonomous`)

Objetivo → palabras clave (sin palabras vacías) → lectura de arXiv, Hugging Face y GitHub →
puntuación por cobertura de palabras + puntuación del radar → guardado de lo nuevo → informe
Markdown con enlaces en `H:\KUKIA_RESEARCH\AI_EXPERIMENTAL\reports\auto_<fecha>.md`.

Incluye detección de **contradicciones o duplicados** (mismo título en fuentes distintas) y
**propuestas de integración** cuando un hallazgo habla de MCP, tools, agentes o SDKs.

No instala, no descarga modelos, no ejecuta binarios, no cambia permisos. Los textos de las
fuentes se tratan como datos, nunca como instrucciones.

Disponible también como trabajo de cola: `research_auto` (con objetivo y límite).

## 4. Registro de herramientas (F4) y TOOL CENTER (F6)

`core/src/tools.rs` construye el registro: 16 herramientas del núcleo + las herramientas MCP
descubiertas, con categoría, permiso, origen y estado. Donde hay una sonda barata y segura, se
comprueba de verdad:

- `gpu.stats` → nvidia-smi real
- `jobs.queue` → cola real
- `memory.*` → número de recuerdos
- `security.policy` → políticas
- `mcp.client` → servidores/autorizados/herramientas
- el resto → «se comprueba al ejecutarlo (necesita motor o aprobación)»

La vista **Herramientas** es ahora el TOOL CENTER con tres pestañas: registro filtrable,
servidores MCP (autorizar/revocar, descubrir, ver y ejecutar) e investigación autónoma con su
informe.

## 5. Medición real en esta máquina

```
cargo run -p kukia-core --example herramientas

SERVIDORES MCP DETECTADOS (6)   ← leídos de C:\Users\kuki\.agents\mcp.json
  fetch · filesystem · memory · sequential-thinking · sqlite · time
  transporte=stdio · autorizado=false en los 6 · tools=0

RESUMEN
  total=16  core=16  mcp=0  servidores=6  autorizados=0
  disponibles=16  requieren_autorizacion=0  sin_comprobar=0

gpu.stats → GPU0 RTX 5070 12227 MB · GPU1 RTX 5060 Ti 16311 MB
security.policy → 9 políticas de agente
```

## 6. Archivos

**Creados:** `core/src/mcp.rs`, `core/src/tools.rs`, `core/examples/herramientas.rs`,
este informe.
**Modificados:** `core/src/lib.rs` (módulos), `core/src/memory.rs` (tablas `mcp_grants` y
`mcp_tools`), `core/src/research.rs` (autónoma), `core/src/jobs.rs` (tipo `research_auto`),
`src-tauri/src/lib.rs` (9 comandos nuevos + despacho del trabajo),
`app/src/lib/api.ts` (tipos y métodos), `app/src/views/Tools.tsx` (TOOL CENTER),
`app/src/App.tsx` (pasa `notify`).

## 7. Pruebas ejecutadas

```
cargo test --lib        → 127 passed, 0 failed   (3 nuevos de tools)
cargo check -p kukia-code → exit 0
npm run build           → 55 módulos, 380.71 kB, exit 0
cargo run --example herramientas → los datos reales de arriba
```

## 8. Límites reales

1. **No se ha conectado ningún servidor MCP de verdad**: los 6 están sin autorizar y ejecutarlos
   lanzaría `npx`/`uvx` (descarga de paquetes). El código de conexión y de `tools/call` está
   implementado y compila, pero **no está probado contra un servidor vivo**. Es lo primero que hay
   que validar cuando autorices uno.
2. **MCP sin streamable-http con sesión**: soporta POST simple y SSE de una línea. Servidores que
   exijan `Mcp-Session-Id` en cabeceras no funcionarán todavía.
3. **No hay prompts/recursos MCP** (`prompts/list`, `resources/list`): solo herramientas.
4. **Investigación autónoma sin ejecución programada**: se lanza a mano o desde la cola. No hay
   planificador temporal todavía.
5. **Descubrimiento continuo (F11)**: el radar `radar.check` sigue existiendo, pero no hay un
   bucle que proponga herramientas nuevas solo.
6. **Tool Router (F5)**: el registro informa; aún no elige herramienta automáticamente por tarea.
