# KUKIA CODE — AUDITORÍA OMEGA CLOUD (solo lectura)

**Fecha:** 01/11/2026 · **Autor:** Xoxete
**Alcance:** ampliación «Cloud AI + OpenCode + Freebuff + instalador EXE»
**Método:** lectura de código, inspección de configuración instalada y consultas locales. **Cero modificaciones.**

---

## 0. QUÉ SE HA INSPECCIONADO (y cómo)

| Objeto | Qué se miró |
|---|---|
| `H:\KUKIA_CODE` | estructura, git, `package.json`, `tauri.conf.json`, `core/src/*.rs`, `core/tests/*`, `src-tauri/src/lib.rs`, `app/src/**` |
| `target/release/bundle/nsis` | instalador generado, tamaño, SHA-256 |
| `C:\Program Files\KUKIA CODE` | binario instalado + desinstalador |
| `%APPDATA%\KukiaCode` | datos de usuario, `secrets.json`, base |
| OpenCode 1.19.3 | `auth.json` (**solo nombres de proveedor**), `opencode.json`, caché `models.json` |
| Freebuff | carpeta de configuración (sin leer credenciales) |

Comandos usados: `git`, `ls`, `find`, `grep` y `python` en modo lectura. No se ha escrito ni borrado nada del proyecto.

---

## 1. ESTADO REAL MEDIDO

| Dato | Valor hoy |
|---|---|
| Rama / commits | `master` · 2 commits (`18d43fb`, `b125554`) · todo el trabajo de fases 2–3 **sin commitear** |
| Tests Rust | **106** (`#\[test\]`/`#\[tokio::test\]`) |
| Comandos Tauri expuestos | **61** |
| Vistas de interfaz | **19** |
| Proveedores cloud en el catálogo de KUKIA | **4** (`opencode`, `freebuff`, `openai`, `custom`) |
| Protocolo cloud soportado | **solo OpenAI-compatible** (`/chat/completions`, `/models`) |
| Tool calling | **no implementado** en ningún proveedor (ni local ni cloud) |
| Almacenamiento de clave | `secrets.json` en **texto plano** + variable de entorno + `auth.json` de OpenCode (solo lectura) |
| DPAPI / Credential Manager | **no** (ningún crate `keyring`/`windows` en Cargo.toml) |
| Instalador | `target/release/bundle/nsis/KUKIA CODE_0.1.0_x64-setup.exe` · **2.931.239 B** · SHA-256 `bd4ab2a047f43b2a5c002f2d6ef6646f8201dd1d1117ec22a6dc0f60e00cfe67` |
| Modo de instalación | `perMachine` (NSIS, pide administrador) |
| Updater / firma de código | **no** (`tauri-plugin-updater` ausente) |
| CSP de la ventana | **`null`** (sin política de seguridad de contenido) |
| Espacio libre | C: 127,2 GB · G: 263,7 GB · H: 98,1 GB · J: 29,4 GB |

---

## 2. ORGANISMOS EXTERNOS: QUÉ HAY DISPONIBLE PARA REUTILIZAR

### 2.1 OpenCode (instalado, 1.19.3)

| Dato | Valor |
|---|---|
| Credenciales presentes | **2**: `opencode`, `opencode-go` (campos `type`, `key`) |
| Proveedores configurados en `opencode.json` | **9**: `llamaserver`, `bonsai`, `lmstudio`, `omniroute`, `ollama`, `endy-cybersec`, `cyber-preview`, `opencode`, `opencode-go` |
| Modelo por defecto | `ollama/xoxete-14b:latest` |
| Modelos declarados | 40 (ollama) + 27 (lmstudio) + 2 + 1 + 1 + 1 + 1 |
| Caché de registro | `~/.cache/opencode/models.json` · **5.963.825 B** |

### 2.2 El hallazgo importante: registro `models.dev` ya en disco

La caché de OpenCode contiene el registro público de modelos:

- **235 proveedores**
- **8.940 modelos**
- Cada modelo trae: `cost` (entrada, salida, caché, **tramos por contexto**), `limit` (contexto y salida), `modalities` (texto, imagen, audio, vídeo, pdf), `tool_call`, `reasoning`, `attachment`, `temperature`.

Ejemplo real extraído (`aihubmix/gemini-2.5-pro`): coste 1,25/10 USD por millón, contexto 1.048.576, modalidades de entrada `text,image,audio,video,pdf`, `tool_call: true`.

**Esto es exactamente lo que pide la sección 1 de la ampliación** (catálogo, capacidades reales, precios, límites). No hay que inventarlo ni scrapearlo: se puede leer del disco o descargar el registro propio. Es la pieza que convierte «indicador de coste antes de enviar» de promesa en algo calculable.

### 2.3 Freebuff

Instalado (esta misma app). **No guarda una clave de API en claro reutilizable**: en la búsqueda anterior solo aparecieron `apiHost` y un `.env.ejemplo` vacío. El endpoint `https://freebuff.com/api/v1/paid` responde **401** sin clave y **280 modelos** con ella. Requiere que el usuario cree su clave en Cuenta → Developer API.

---

## 3. MATRIZ DE LA AMPLIACIÓN (clases A/B/C/D/E)

| # | Funcionalidad pedida | Clase | Estado real |
|---|---|---|---|
| 1 | Chat cloud OpenAI-compatible (OpenAI, OpenRouter, DeepSeek, xAI, Mistral, Groq, Cerebras, Together, Fireworks, Perplexity) | **A (parcial)** | El adaptador genérico funciona; probado listado real con OpenCode Zen (**43 modelos**) y Freebuff (**280**). Generación no probada por falta de saldo/clave |
| 2 | Streaming cloud | **B** | `stream_openai` escrito; nunca ejecutado con éxito contra un endpoint real |
| 3 | Catálogo de proveedores extensible | **C** | 4 de los 15+ pedidos; sin capacidades ni precios |
| 4 | Capacidades reales por modelo (visión/audio/razonamiento/contexto) | **D** | No existe. **Datos disponibles** en el registro `models.dev` |
| 5 | Tool calling | **D** | Ningún proveedor envía `tools`. Bloquea agentes cloud |
| 6 | JSON / salida estructurada | **D** | Existe `format: json` en Ollama; no en cloud |
| 7 | Coste estimado antes de enviar | **D** | Nada. Precios disponibles pero no leídos |
| 8 | Presupuesto y control de gasto | **D** | Nada |
| 9 | Historial de uso, tokens in/out, gasto | **C** | `memory.rs` guarda tokens por mensaje; no hay agregado ni coste |
| 10 | Detección de modelos retirados/incompatibles | **D** | — |
| 11 | Anthropic (protocolo `/v1/messages`) | **D** | Requiere adaptador propio |
| 12 | Google Gemini (`generateContent`) | **D** | Requiere adaptador propio |
| 13 | Azure OpenAI | **C** | Sirve el genérico si se pone base URL con `api-version`; falta cabecera `api-key` |
| 14 | AWS Bedrock | **D (bloqueada sin firma)** | Requiere SigV4 (`aws-sigv4`) y región; hoy no hay dependencia |
| 15 | CLOUD CREDENTIALS (alta/prueba/revoca) | **D** | Solo un texto estático en Configuración que dice dónde está `secrets.json` |
| 16 | Almacenamiento seguro (DPAPI / Credential Manager) | **D** | `secrets.json` en claro |
| 17 | No mostrar claves / ocultar en logs | **A** | Verificado: `config.json` sin `sk-`, la clave nunca se imprime, solo su origen |
| 18 | Bloqueo total de la nube | **D** | No hay interruptor; solo elección de proveedor |
| 19 | Aprobación antes de gasto | **D** | — |
| 20 | Router local+cloud por tarea/calidad/coste/privacidad | **D** | Solo selector manual |
| 21 | Conectores OpenCode / Freebuff | **C** | «Conector» = leer `auth.json` en solo lectura. No hay módulo, ni versión, ni capacidades |
| 22 | Instalador NSIS | **A** | Existe y está instalado; SHA-256 arriba |
| 23 | Desinstalador | **A** | `uninstall.exe` presente |
| 24 | Actualización controlada / reparación / volver atrás | **D** | Sin updater ni firma |
| 25 | Detección de hardware/Ollama/LM Studio en el instalador | **D** | El instalador no detecta nada |
| 26 | No incluir claves ni modelos en el EXE | **A** | Verificado: el EXE no contiene credenciales; los modelos viven fuera |
| 27 | Pruebas de instalación limpia / actualización / desinstalación | **C** | Instalación probada; actualización y desinstalación **no** |
| 28 | CSP de la ventana | **D** | `"csp": null` |

**Recuento:** A 6 (2 parciales) · B 1 · C 5 · D 15 · E 1.

---

## 4. RIESGOS DE SEGURIDAD (antes de añadir 15 proveedores)

| # | Riesgo | Severidad | Por qué |
|---|---|---|---|
| S1 | **Clave en texto plano** en `%APPDATA%\KukiaCode\secrets.json` | **Alta** | Con 15 claves de 15 proveedores, un archivo legible por cualquier proceso del usuario es un almacén de dinero. Debe ir a DPAPI (`CryptProtectData`) o Credential Manager |
| S2 | **`csp: null`** | **Alta** | La ventana puede cargar/ejecutar lo que llegue; con claves en memoria y salida de modelo en pantalla, es la vía clásica de exfiltración |
| S3 | Reutilizar `auth.json` de OpenCode | **Media** | Es cómodo y no copia nada, pero acopla KUKIA a un archivo que OpenCode puede rotar o borrar; si desaparece, la nube se queda muda sin avisar |
| S4 | Sin control de gasto | **Media** | Un agente en bucle con un modelo de 15 USD/M tokens gasta sin techo |
| S5 | Envío de datos privados a la nube sin decisión explícita | **Media** | No hay interruptor global ni aviso por proveedor |
| S6 | `has_cloud_key` persistido en `config.json` | **Baja** | Se recalcula al cargar (mitigado), pero el campo puede quedar obsoleto si se borra `secrets.json` |
| S7 | Instalador `perMachine` | **Baja** | Cada instalación/actualización pide administrador |
| S8 | Sin firma de código | **Baja-media** | Windows SmartScreen avisará; la actualización no es verificable criptográficamente |

---

## 5. QUÉ SE REUTILIZA (no hay que escribirlo de cero)

| Pieza existente | Para qué sirve en esta ampliación |
|---|---|
| `providers::join_api` | Empalme de URL sin duplicar `/v1` — probado con 4 formatos |
| `config::active_cloud` + catálogo | Base del catálogo extensible (4 → 235 entradas) |
| `config::cloud_credential` + `KeySource` | Cadena de resolución de claves con origen auditable |
| `chat_openai` / `stream_openai` | Cubren ~11 de los 15 proveedores pedidos sin tocar código |
| Registro `models.dev` (235/8.940) | Catálogo, precios, límites, modalidades, `tool_call`, `reasoning` |
| `providers.rs` normalización de respuesta | Reutilizable para adaptadores nativos |
| `memory.rs` (tokens por mensaje) | Base para el historial de gasto |
| `logs.rs` | Registro de auditoría de llamadas cloud |
| Tauri 2 + NSIS ya funcionando | Base del instalador; solo falta versión, updater y pruebas |
| 106 tests | Red de seguridad para no romper el chat al añadir adaptadores |

---

## 6. PLAN POR FASES (con dependencias y tamaño)

### FASE C0 — Credenciales protegidas (base de todo)

- Migrar de `secrets.json` en claro a **DPAPI por usuario** (`CryptProtectData`) con sobre cifrado; mantener lectura del archivo antiguo para migrar sin perder nada.
- Vista **CLOUD CREDENTIALS**: alta, prueba de conexión, revocación, «necesita configuración».
- Nunca devolver la clave: solo `presente/no presente`, origen y últimos 4 caracteres opcionales (a decidir).
- **Depende de:** nada. **Bloquea a:** todo lo demás.
- Riesgo: bajo. Sesiones estimadas: **1**.

### FASE C1 — Catálogo y capacidades reales

- Ingesta del registro `models.dev` (lectura local primero; descarga propia después, con hash).
- Ficha por modelo: coste (con tramos), contexto, modalidades, `tool_call`, `reasoning`.
- Sustituir el catálogo de 4 por catálogo + proveedores del registro.
- **Depende de:** C0 (para no mezclar claves con catálogo). Sesiones: **1**.

### FASE C2 — Adaptadores por protocolo

- Genérico OpenAI-compatible (ya está) · **Anthropic** (`/v1/messages`, `x-api-key`, `anthropic-version`) · **Gemini** (`generateContent`, `?key=`) · **Azure** (`api-key` + `api-version`).
- **Bedrock**: requiere `aws-sigv4`; lo dejo para el final y avisando de lo que implica.
- Normalizador único de respuesta → mismo `ChatResponse` para los cuatro.
- Tests con servidor local simulado (sin red, sin claves).
- **Depende de:** C1. Sesiones: **2**.

### FASE C3 — Router local+cloud y kill switch

- Puntuación por tarea/calidad/velocidad/coste/privacidad/contexto/herramientas.
- **Bloqueo total de nube** (interruptor real, no cosmético).
- Aprobación explícita antes de proveedor de pago sin presupuesto autorizado.
- Etiqueta visible: «esta tarea la está haciendo X».
- **Depende de:** C1, C2. Sesiones: **2**.

### FASE C4 — Uso, gasto y presupuesto

- Tabla `usage` (proveedor, modelo, tokens in/out, coste estimado, fecha, tarea).
- Presupuesto por proveedor y global; aviso y corte.
- Historial y exportación CSV.
- Coste estimado **antes** de enviar (usando precios de C1).
- **Depende de:** C1. Sesiones: **1–2**.

### FASE C5 — Tool calling (prerrequisito de agentes)

- Enviar `tools`, parsear `tool_calls`, ejecutar bajo la política de permisos ya cerrada en Fase 3.0/3A.
- **Depende de:** C2 (por proveedor) y de la política de permisos (ya hecha). Sesiones: **2**.

### FASE C6 — Release instalable

- Versión y número de compilación visibles; `CHANGELOG`.
- **CSP** real en lugar de `null`.
- Pruebas: instalación limpia, actualización conservando `%APPDATA%`, desinstalación sin borrar datos, arranque sin modelos y sin Internet.
- Informe de tests + SHA-256 del instalador.
- Updater firmado: **requiere clave de firma tuya** → autorización expresa.
- **Depende de:** todo lo anterior. Sesiones: **1–2**.

### Total estimado: **9–11 sesiones**, en el orden C0 → C1 → C2 → C4 → C3 → C5 → C6.

---

## 7. LO QUE NO HARÍA TODAVÍA (y por qué)

- **Añadir Anthropic/Gemini/Bedrock antes de C0**: serían tres claves más en un archivo de texto plano. Primero la caja fuerte.
- **Bedrock**: exige SigV4 y credenciales de AWS; es el proveedor con más superficie de error y menos valor inmediato para ti.
- **Updater firmado sin clave de firma**: daría una falsa sensación de seguridad.
- **Tool calling cloud antes de C5**: los agentes ejecutarían en la nube con herramientas que aún no distinguen local de remoto.

---

## 8. LO QUE NECESITO DE TI PARA EMPEZAR

1. **Autorización de la Fase C0** (credenciales DPAPI + vista CLOUD CREDENTIALS).
2. Decisión sobre **dónde poner tus claves**: por la app (recomendado, DPAPI) o por variable de entorno.
3. Si quieres que el registro de modelos se **descargue** (necesita Internet) o se lea del que ya existe en disco.
4. Para la fase C6: si habrá **clave de firma** o el instalador seguirá sin firmar.

**Nada de esto se ha implementado.** No se ha modificado ningún archivo del proyecto en esta auditoría.
