# Ecosistema tridireccional: KUKIA CORE + OpenCode + Freebuff

Fecha: 02/11/2026 · Proyecto: `H:\KUKIA_CODE` · Backup previo verificado en
`BACKUPS/PUENTE_OPENCODE_20261102_103153` (hashes origen = copia, OK).

## Lo que se ha implementado

| Archivo | Qué hace |
|---|---|
| `core/src/bridge.rs` (nuevo) | Puente KUKIA ↔ OpenCode: descubrimiento real (versión, catálogo, agentes, MCP, config), conversación por API y generador del fragmento `opencode.json`. |
| `core/src/serve.rs` (nuevo) | Servidor HTTP compatible OpenAI, **solo loopback**, con token obligatorio: `/v1/models`, `/v1/chat/completions` (normal y SSE), `/health`. |
| `core/src/ecosystem.rs` (nuevo) | Los tres nodos con capacidades reales, catálogo compartido y enrutador de tareas. |
| `src-tauri/src/lib.rs` | 12 comandos nuevos: `puente_*`, `kukia_server_*`, `ecosistema_*`. |
| `app/src/views/Bridge.tsx` (nuevo) | Pantalla del puente OpenCode (estado, petición con modelo/agente, servidor KUKIA y fragmento). |
| `app/src/views/Ecosystem.tsx` (nuevo) | Panel **ECOSISTEMA IA — INTEGRACIONES** con las tres tarjetas, catálogo y enrutador. |
| `app/src/App.tsx`, `app/src/lib/api.ts` | Navegación y tipos/cliente de los comandos nuevos. |
| `core/tests/puente_bidireccional.rs` (nuevo) | Integración: KUKIA sirve sus modelos por HTTP compatible OpenAI. |
| `core/tests/puente_opencode.rs` (nuevo) | Integración: KUKIA pide respuesta al servidor real de OpenCode. |
| `core/tests/ecosistema_tres_nodos.rs` (nuevo) | Integración: los tres nodos devuelven datos reales. |

## Hechos comprobados en esta máquina

### OpenCode 1.19.3
- `opencode models` → **186** modelos en 8 proveedores (`opencode`, `ollama`, `opencode-go`,
  `lmstudio`, `bonsai`, `llamaserver`, `endy-cybersec`, `cyber-preview`).
- Servidor propio: `opencode serve --port N --hostname 127.0.0.1` → OpenAPI 3.1 con **162 rutas**
  (evidencia: `INFORMES/evidencia/opencode_openapi.json`, 469 KB).
- Con servidor vivo: `GET /api/model` → **213** modelos · `GET /api/agent` → **30** agentes.
- Conversación verificada: `POST /api/session` + `POST /session/{id}/message` con `parts`
  → respuesta real de Xoxete (`qwen3-vl:4b`), texto **"PUENTE OK"** en 27 s / 355 tokens.

### Freebuff 0.0.192
- Ejecutable: `C:\Users\kuki\AppData\Local\Programs\@codebufffreebuff-desktop\Freebuff.exe`.
- Estado real medido: **6 procesos**, **7 proyectos**, **13 hilos**, **1800 mensajes**
  (SQLite en solo lectura), **6 servidores MCP** compartidos, **2 skills** de cuenta.
- Puertos locales `127.0.0.1:34147` y `:34148` responden `401 missing or invalid token`:
  **no se usan** (no hay API oficial; prohibido usar vías privadas).

### KUKIA CORE
- Catálogo local: **125** modelos que pueden chatear, **9** agentes, **16** herramientas,
  **6** servidores MCP.
- Servidor `/v1` probado de verdad: 125 modelos servidos, 401 sin token, 404 con modelo
  inexistente y un chat completo con respuesta no vacía.

## Limitaciones declaradas (no se finge nada)

1. **Freebuff no se puede pilotar desde KUKIA**: no publica API local documentada.
   El panel lo dice y el enrutador marca esas tareas como `automatizable=false`.
2. **La configuración de OpenCode no se toca**: KUKIA genera el fragmento para `opencode.json`
   y el usuario decide si lo pega.
3. **`/api/session/{id}/prompt` (v2) devuelve 500** en 1.19.3; se usa la ruta clásica
   `/session/{id}/message`, que sí funciona.
4. **El CLI de OpenCode se cuelga sin terminal** (`run` y `agent list`; comprobado: 20 s sin
   terminar). Por eso los agentes se piden por API y hay un timeout corto con aviso.
   `opencode models` sí responde (2 s).
5. **No se comparten credenciales** entre nodos, y el servidor de KUKIA exige token propio.

## Pruebas ejecutadas

```
cargo test --lib                    → 145 passed, 0 failed, 1 ignored
cargo test --test puente_bidireccional  → 1 passed (125 modelos, chat real)
cargo test --test puente_opencode       → 1 passed ("PUENTE OK")
cargo test --test ecosistema_tres_nodos → 1 passed (tres nodos con datos reales)
cargo check -p kukia-code           → exit 0
npm run build                       → 58 módulos, 416 kB, exit 0
npm run tauri:build                 → release en 4m 41s
```

Ejecutable nuevo: `target/release/kukia-code.exe` (12.156.928 bytes, 02/11 11:38).
Instalador: `target/release/bundle/nsis/KUKIA CODE_0.1.0_x64-setup.exe` (3,34 MiB).

## Cómo usarlo

1. Abre KUKIA CODE → **Ecosistema IA**: verás las tres tarjetas con lo verificado y lo que no hay.
2. **Puente OpenCode** → *Arrancar opencode serve* → elige proveedor/modelo/agente → *Preguntar*.
3. **Puente OpenCode** → *Abrir servidor de KUKIA* → *Generar fragmento* → pégalo en
   `opencode.json` si quieres que OpenCode use los modelos de KUKIA.
