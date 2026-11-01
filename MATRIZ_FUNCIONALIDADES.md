# KUKIA CODE — MATRIZ DE FUNCIONALIDADES

**Actualizada:** 01/11/2026 (cierre de Fase 3.4) · **Autor:** Xoxete

Clases: **A** implementada y verificada · **B** implementada sin verificar ·
**C** parcial (falta una pieza) · **D** no implementada · **E** bloqueada.

## A — Verificada (prueba ejecutada)

| # | Funcionalidad | Prueba |
|---|---|---|
| A1 | Chat Ollama (modelos, streaming, tokens/s) | tests + 81 modelos reales |
| A2 | Chat LM Studio / OpenAI-compatible | sondeo real, 32 modelos |
| A3 | Proveedor cloud OpenAI-compatible | 43 modelos (OpenCode Zen) y 280 (Freebuff) listados |
| A4 | Clave cloud en cascada, nunca mostrada | entorno → `secrets.json` → `auth.json`; `config.json` sin secretos |
| A5 | Memoria SQLite (sesiones, mensajes, memorias) | `integrity_check ok`, tests |
| A6 | Permisos aplicados en `files_write` y `run_command` | `validacion_fase30.rs` |
| A7 | Permisos por proyecto (allowlist + solo lectura) | `validacion_fase3a.rs` |
| A8 | Timeout y cancelación de comandos | `validacion_fase30.rs` |
| A9 | Auditoría conectada, filtrable y exportable CSV | `audit_*` + vista Seguridad |
| A10 | VRAM Guard con lectura real de GPU y motores | RTX 5070 + 5060 Ti medidas |
| A11 | Aviso automático de competencia de VRAM (sin abrir la vista) | log 15:11:41 y 15:14:04 + capturas |
| A12 | Gaming Radar (fichas + alertas) | `validacion_gaming.rs` |
| A13 | Anime ↔ Real contra ComfyUI real | `validacion_anime_real.rs` + imágenes |
| A14 | Monitor GPU, radar de releases, logs, backups verificados | tests |
| A15 | Instalador NSIS + app instalada que arranca | hash verificado, 9 políticas sembradas |
| A16 | CSP activa en la ventana | `tauri.conf.json`, app parcheada pinta |

## B — Implementada, sin verificar de punta a punta

| # | Funcionalidad | Qué falta |
|---|---|---|
| B1 | Generación cloud real (respuesta completa) | Sin saldo en OpenCode Zen (402) ni clave válida de Freebuff |
| B2 | Streaming cloud | Escrito, nunca respondió con éxito |

## C — Parcial

| # | Funcionalidad | Existe | Falta |
|---|---|---|---|
| C1 | Catálogo de proveedores cloud | 4 de 15+ pedidos | Catálogo `models.dev` (235/8.940) y capacidades |
| C2 | Historial de uso | Tokens por mensaje | Agregado, coste y presupuesto |
| C3 | Conectores OpenCode/Freebuff | Lectura de `auth.json` | Módulo con versión y capacidades detectadas |
| C4 | Benchmark Center | Mide tok/s | Comparación entre motores y condiciones |
| C5 | Model Router | Selector manual | Puntuación automática por VRAM/contexto/coste |
| C6 | Pruebas de release | Instalación comprobada | Actualización conservando datos y desinstalación limpia |
| C7 | Notificación de competencia | Banner interno | Aviso del sistema si la ventana está minimizada |

## D — No implementada

| # | Funcionalidad |
|---|---|
| D1 | Almacenamiento seguro de claves (DPAPI / Credential Manager) |
| D2 | Vista CLOUD CREDENTIALS (alta, prueba, revocación) |
| D3 | Tool calling (ningún proveedor envía `tools`) |
| D4 | Coste estimado antes de enviar, presupuesto y corte |
| D5 | Router automático local+cloud y kill switch de nube |
| D6 | Adaptadores nativos Anthropic / Gemini / Azure |
| D7 | Actualizador firmado y firma de código |
| D8 | Multi-PC real (workers, cola, autenticación) |
| D9 | Memoria semántica con embeddings / RAG |
| D10 | Agent Swarm, Autopilot, self-improve supervisado |
| D11 | Cliente MCP y terminal pty real |
| D12 | KV Cache Manager y parámetros de inferencia por modelo |

## E — Bloqueada (y por qué)

| # | Funcionalidad | Bloqueo |
|---|---|---|
| E1 | AWS Bedrock | Requiere SigV4 y credenciales AWS; sin dependencia y con más superficie de error que valor inmediato |
| E2 | Sumar VRAM entre los dos PCs | No es programación: el enlace de red es el cuello. Solo medible con `llama.cpp RPC` |
| E3 | «Control universal» de cualquier app Windows | Bloqueada como afirmación: canvas propietarios y anti-cheat no se controlan por accesibilidad |

## Recuento

| Clase | Cuántas |
|---|---:|
| A | 16 |
| B | 2 |
| C | 7 |
| D | 12 |
| E | 3 |
