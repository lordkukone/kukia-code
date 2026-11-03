# Reparación de OpenCode — diagnóstico y resultado

Fecha: 02/11/2026 · OpenCode 1.19.3 · base `C:\Users\kuki\.local\share\opencode\opencode.db` (17,4 GB)

## 1. Diagnóstico: por qué no se podían crear chats

No era el modelo, ni la red, ni la configuración. Era **desincronización de esquema** entre la
base de datos y el binario instalado. El orden exacto en que aparecieron los fallos:

| # | Error real (del log con `--print-logs --log-level DEBUG`) | Capa |
|---|---|---|
| 1 | `SQLiteError: no such table: session_input` | falta una tabla entera |
| 2 | `SQLiteError: no such table: session_context_epoch` | falta otra tabla |
| 3 | `FOREIGN KEY constraint failed` al insertar en `session_message` | la FK apunta a `session_v2`, tabla que **este binario no conoce** |
| 4 | `SessionRunnerModel.ModelUnavailableError: Model unavailable: ollama/qwen3-vl:4b` | resolución de modelo (ruta nueva) |
| 5 | `SessionRunnerModel.UnsupportedApiError: Unsupported API for opencode/glm-5.3-flash-free: native` | resolución de modelo (ruta nueva) |

**Causa de fondo:** la base fue migrada por **otra compilación** de OpenCode (la que creó
`session_inbox`, `session_v2`, `session_part`, `job`, `schedule`…). El binario instalado (19/10)
solo conoce `session_input`, `session_context_epoch` y `session_entry`, y las consulta con
`PRAGMA foreign_keys=ON`. Resultado: cualquier chat moría con 500 antes de generar nada.

Comprobaciones que lo demuestran:

- De las 38 migraciones que conoce el binario, **0 estaban sin aplicar**: el runner no iba a
  arreglarlo solo, porque su tabla de control decía que todo estaba hecho.
- El binario menciona `session_input` **36 veces** y `session_inbox` **0 veces**.
- `/api/session` (v2) escribe la sesión en `session`, pero `session_message` tiene su FK hacia
  `session_v2` → violación en cada mensaje.

## 2. Reparación aplicada (solo aditiva, sin borrar ni renombrar nada)

Backup verificable: `BACKUPS/OPENCODE_SESSION_INPUT_20261102_115400`
- `esquema_completo.sql` (17.827 B) · SHA-256 `cabb93a64adf2c845f08c73f5cc8d9a9208b6f648d5c1865b88aded5d19b5a93`
- `estado.json` con las 29 tablas y sus recuentos (120.459 `session_message`, 357.823 `part`…)

Cambios (DDL extraído del propio binario, no inventado):

1. `session_input` — esquema final (`admitted_seq`, `promoted_seq`) + 3 índices.
2. `session_context_epoch` — con `replacement_seq` y `revision`.
3. `session_entry` + índices `session_entry_session_idx`, `session_entry_session_type_idx`,
   `session_entry_time_created_idx`.
4. `data_migration` — tabla de control que el binario espera (se crea vacía; sus 38 migraciones
   ya constan aplicadas en `migration`).
5. Dos triggers de compatibilidad **KUKIA** (borrables en cualquier momento):
   - `kukia_session_mirror_v2`: al insertar en `session`, replica la fila en `session_v2`.
   - `kukia_session_mirror_v2_upd`: mantiene título, coste, tokens y `time_updated` en sincronía.

Se comprobó con FK activada que un mensaje entra correctamente en `session_message`.

## 3. Resultado medido

| Prueba | Antes | Después |
|---|---|---|
| `POST /api/session/{id}/prompt` | **500** `no such table: session_input` | **200** |
| Errores SQLite en el log | en cada mensaje | **0** |
| Mensaje de usuario persistido | no | **sí** (`session_message`) |
| Chat por ruta clásica `/session/{id}/message` | funcionaba | **`"CLASICO OK"`** (138 s) |
| `opencode run` (CLI) | colgado, 0 bytes | **sigue colgado** |

`PRAGMA quick_check` → `ok`. Tablas: 29 → 33. Ninguna tabla perdida.

## 4. Lo que sigue roto y por qué (no es la base de datos)

El **runner nuevo (v2)** de este build no resuelve ningún modelo disponible en la máquina:

- proveedores `native` (opencode, opencode-go) → `UnsupportedApiError: native`;
- proveedores locales (`ollama`, `lmstudio`) → `ModelUnavailableError`.

Probado también en el laboratorio (`H:\KUKIA_RESEARCH\oc_lab`) declarando a mano
`capabilities: {input:["text"], output:["text"]}` para `qwen3-vl:4b`: **mismo error**. No es
configuración, es el build.

Consecuencia: `opencode run` y la ruta v2 no pueden chatear; la **ruta clásica sí** y es la que
usa KUKIA CODE (`/session/{id}/message`). Para que el CLI y la interfaz nueva vuelvan a chatear
hay que alinear versiones (actualizar `opencode-ai`), y eso **implica instalar software**: queda
pendiente de autorización expresa.

## 5. Cómo revertirlo (sin pérdida de datos)

```sql
DROP TRIGGER IF EXISTS kukia_session_mirror_v2;
DROP TRIGGER IF EXISTS kukia_session_mirror_v2_upd;
DROP TABLE IF EXISTS session_input;
DROP TABLE IF EXISTS session_context_epoch;
DROP TABLE IF EXISTS session_entry;
DROP TABLE IF EXISTS data_migration;
```

Ninguna de las seis contiene datos originales tuyos: se crearon vacías en esta reparación.

## 6. Inventario del cerebro XOXETE (Fase 2, rutas verificadas)

| Componente | Ubicación real | Estado |
|---|---|---|
| KUKIA CORE | `H:\KUKIA_CODE` | operativo · 149 pruebas en verde |
| Laboratorio | `H:\KUKIA_RESEARCH` | operativo |
| Xoxete (voz/identidad) | `H:\Xoxete`, `I:\CEREBRO\SUMISA_NOVELA_COMPLETA.txt` | presente |
| Config OpenCode | `C:\Users\kuki\.config\opencode\opencode.json` | legible · modelo `ollama/xoxete-14b:latest` · agente `xoxete` |
| Agentes OpenCode | `~/.config/opencode/agent` (2 ficheros) + 28 declarados que reporta la API | 30 por API |
| Plugin Xoxete | `~/.config/opencode/plugin/xoxete-learning.ts` | presente |
| MCP compartidos | `C:\Users\kuki\.agents\mcp.json` | 6 servidores |
| Memoria Freebuff | `~/.config/freebuff-desktop/projects/*/desktop-v2.db` | 7 proyectos · 1800 mensajes (solo lectura) |
| Modelos vídeo | `J:\models\video` | presente |
| Descargas | `G:\descargas` | presente |

## 7. Pendiente

- Fases 3–7 (integraciones Evil/Unsecured/Zero Censura, selector unificado, memoria compartida):
  **no empezadas a propósito**. La orden era reparar primero, y el último tramo (runner v2) depende
  de una decisión tuya sobre versiones.
- 4 sesiones de prueba creadas en tu base durante el diagnóstico (`ses_e64188f3…`, `ses_e641a175…`,
  `ses_e6414d31…`, `ses_e6413e7d…`, `ses_e64165c2…`, `ses_e6411664…`). No se han borrado: borrar
  requiere tu confirmación.
