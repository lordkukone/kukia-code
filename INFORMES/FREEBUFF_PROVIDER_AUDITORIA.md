# FREEBUFF COMO PROVEEDOR EN KUKIA CODE — AUDITORÍA Y PLAN

Fecha: 01/11/2026 · Modo: **solo lectura**. No se ha modificado ni una línea de la app.

---

## 1. LO QUE YA EXISTE (verificado en el código)

Freebuff **no hay que inventarlo**: ya está en el catálogo de proveedores cloud.

| Pieza | Archivo | Estado |
|---|---|---|
| Entrada de catálogo | `core/src/config.rs:21-25` | `id: "freebuff"`, `label: "Freebuff"`, `base_url: https://freebuff.com/api/v1/paid` |
| Resolución de clave | `core/src/config.rs:84` `cloud_credential()` | entorno → `secrets.json` → `auth.json` de OpenCode (solo lectura) |
| Listado de modelos cloud | `core/src/providers.rs:247` | `GET {base}/models` con Bearer |
| Chat no streaming | `core/src/providers.rs:339` `chat_openai()` | OpenAI-compatible, `Authorization: Bearer` |
| Chat streaming | `core/src/providers.rs:471` `stream_openai()` | SSE |
| Selector + Base URL | `app/src/views/Settings.tsx` | selector de proveedor y URL editables |
| Test de base URL | `core/src/providers.rs:672` (test) | ya cubre `freebuff → https://freebuff.com/api/v1/paid` |

**Conclusión:** el 70 % del conector está hecho. Lo que falta es la puerta para meter la clave y la honestidad de capacidades.

---

## 2. ENDPOINT — MEDIDO HOY, NO SUPUESTO

Peticiones reales ejecutadas desde esta máquina:

| Petición | Resultado |
|---|---|
| `GET https://freebuff.com/api/v1/paid/models` | **HTTP 200**, `{"object":"list", ...}` |
| `GET .../api/v1/paid/models` con `Authorization: Bearer <clave>` | **HTTP 200**, idéntico (el catálogo es público) |
| `GET https://freebuff.com/api/v1/paid` | HTTP 404 (HTML de la web) |
| `GET https://freebuff.com/api/v1/models` | HTTP 404 |
| `POST .../api/v1/paid/chat/completions` **sin clave** | **HTTP 401** `{"error":{"message":"Invalid API key","type":"authentication_error","code":"invalid_api_key"}}` |

**Lecturas:**

1. El *surface* es **OpenAI-compatible de verdad**: misma forma de error (`error.message` / `type` / `code`), `/models` y `/chat/completions`. No hace falta adaptador propietario.
2. **La inferencia exige clave**; el catálogo no. Eso permite refrescar modelos en la UI sin gastar un céntimo.
3. La URL base **correcta es la que ya tiene KUKIA**. Si se apunta a `/api/v1/paid` "a pelo" (sin `/models`) da 404, así que no hay que tocar la base, sólo llamar a las rutas hijas.

---

## 3. CATÁLOGO REAL: 280 MODELOS Y SUS CAPACIDADES

`GET /models` devuelve 280 entradas con esta forma:

```json
{"id":"deepseek-v4.1-flash","object":"model","owned_by":"freebuff",
 "capabilities":{"chat":true,"streaming":true,"toolCalls":true}}
```

Cifras exactas de hoy:

- **280** modelos, **280** con `chat: true`.
- **1** con `streaming: true`: `deepseek-v4.1-flash`.
- **9** con `toolCalls: true`: `deepseek-v4.1-flash`, `claude-haiku-5-5`, `gpt-6-luna`, `gemini-3.8-flash`, `gpt-6.1-sol`, `gemini-4-argon`, `qwen3.8-max`, `minimax-m3`, `claude-fable-5-1`.
- **No** hay campo de precio, de límite de contexto ni de cuota. Sólo `capabilities`, `id`, `object`, `owned_by`.

### El problema que esto destapa en KUKIA

Ahora mismo `list_models("cloud")` en `providers.rs` hace esto:

- `capabilities` → `vec!["texto"]` fijo. **Miente**: etiqueta igual a un modelo sin streaming y a uno con tool calling.
- `context` → lee `context_length`, campo que Freebuff **no envía** → siempre `null`, correcto por accidente.
- `chat_stream` se usa para todo el proveedor cloud. **Con 279 de 280 modelos sin streaming**, eso puede dar error o colgarse en el primero que no lo soporte.

Esto es exactamente lo que la regla del proyecto prohíbe: presentar como disponible algo que no se ha comprobado.

---

## 4. CREDENCIALES: ESTADO ACTUAL

```
%APPDATA%\KukiaCode\secrets.json  → existe
  claves de primer nivel: ['freebuff']    ← la entrada existe
  valor de 'freebuff': VACÍO (longitud 0) ← pero no hay clave dentro
```

- `KUKIA_CLOUD_KEY`, `KUKIA_CLOUD_KEY_FREEBUFF`, `KUKIA_CLOUD_KEY_OPENCODE`, `KUKIA_CLOUD_KEY_OPENAI` → **ninguna definida**.
- `~/.local/share/opencode/auth.json` → contiene `opencode` y `opencode-go` (fuente de solo lectura; KUKIA nunca la escribe).

Es decir: la ranura está puesta y vacía. **Falta la puerta para llenarla desde la app.** Hoy sólo se puede haciendo `echo` a mano en un JSON.

### Riesgo de seguridad detectado

`secrets.json` se guarda en **texto plano**. Cualquier proceso del usuario puede leerlo. El prompt pide DPAPI / Credential Manager: eso es una dependencia nueva (`windows` crate en `core`), así que va como fase aparte y con tu visto bueno.

---

## 5. QUÉ DICE LA WEB (Y DÓNDE HAY QUE SEPARAR CONCEPTOS)

Investigación en fuentes públicas:

- **freebuff.com no publica documentación de esta API.** `/api/v1/paid` sin ruta hija da 404 y no hay página de docs con el contrato.
- El **CLI gratuito de Freebuff** (GitHub OAuth, financiado con anuncios) es **otra cosa**: su backend es de agent-runs, no expone `/v1/chat/completions`, y llamarlo directamente está prohibido y puede banear la cuenta.
- Existen proxies de terceros que imitan el CLI para sacar un endpoint OpenAI-compatible. **No vamos a usar eso**: roza el ToS y no es una API oficial.
- La **Developer API de pago** sí es la vía legítima y es la que ya apunta KUKIA. La clave se crea desde la cuenta de Freebuff (Escritorio → **Account → Developer API**); la app puede escribirla en el `.env` del proyecto.

**Resumen honesto:** la clave de esta API **no es gratuita**; las llamadas consumen saldo del propietario. El catálogo es gratis de consultar; la inferencia, no.

---

## 6. PLAN PROPUESTO (5 fases, cada una verificable)

### Fase 1 — Puerta de la clave desde Ajustes *(la que desbloquea todo)*
- Comando Tauri `cloud_set_key(provider_id, clave)` → escribe **solo** en `%APPDATA%\KukiaCode\secrets.json`.
- Campo en Configuración con `type="password"`, botón *Guardar clave*, botón *Borrar clave*.
- La clave **nunca** vuelve a la UI, ni a logs, ni a la config: la UI solo recibe `has_cloud_key: true/false`.
- Validación al guardar: `GET /models` y devolver sólo `HTTP 200 ✓` o el código de error.
- Verificación: guardar clave → reiniciar la app → `has_cloud_key` sigue en `true` y el proveedor sale `online`.

### Fase 2 — Catálogo honesto
- `ModelInfo` pasa a llevar `streaming` y `tool_calls` leídos de `capabilities`.
- `context` se queda `null` en Freebuff en vez de inventarse.
- El selector muestra el badge real: `chat`, `+stream`, `+tools`.
- Verificación: test unitario con el JSON real de Freebuff y comparación de conteos (280 / 1 / 9).

### Fase 3 — Inferencia que respeta capacidades
- `chat_stream` en proveedor cloud: si el modelo declara `streaming:false`, se usa `chat()` y se emite el texto troceado localmente para que la UI no note diferencia.
- Traducción de errores: `401` → "clave inválida o caducada"; `402`/`429` → "sin saldo o límite alcanzado"; nunca volcar el body crudo.
- Verificación: una petición real con un modelo sin streaming y otra con `deepseek-v4.1-flash`.

### Fase 4 — Consumo y aviso de coste
- Antes de enviar con proveedor cloud, aviso claro: **operación de pago**.
- Contador de tokens por proveedor y por día (ya existe `memory.rs` con benchmarks/logs donde encaja).
- **No** se muestran precios en euros: Freebuff no los publica en el catálogo y no se van a inventar.

### Fase 5 — Endurecer el secreto (opcional, decide tú)
- Cifrar `secrets.json` con DPAPI (`CryptProtectData`, ligado al usuario de Windows).
- Implica añadir el crate `windows` a `core`. Si no lo quieres, se queda en texto plano y lo dejamos escrito en el informe.

---

## 7. RIESGOS

| Riesgo | Nivel | Mitigación |
|---|---|---|
| Clave en texto plano en `%APPDATA%` | medio | Fase 5 (DPAPI) o permisos NTFS al archivo |
| Gasto de saldo sin querer al probar | alto | Probar sólo `GET /models` (gratis) hasta que tú lo autorices; aviso en Fase 4 |
| 279 modelos sin streaming real | medio | Fase 3, degradar a no-streaming |
| Endpoint sin documentación pública: puede cambiar | medio | Fijar la base en config (ya está) y no cablear rutas nuevas |
| Confundir esto con el CLI gratuito | alto | Ya separado en el informe; nunca tocar tokens del CLI |

---

## 8. LO QUE NECESITO DE TI

1. **La clave** (si aún no la has creado): Escritorio de Freebuff → **Account → Developer API**. Pégala en el campo que construiré en la Fase 1, no aquí en el chat.
2. **¿DPAPI sí o no?** (Fase 5).
3. **Autorización** para tocar `config.rs`, `providers.rs`, `src-tauri/src/lib.rs`, `Settings.tsx` y `api.ts`.

Sin esos tres puntos no se toca código. El estado actual no rompe nada: el proveedor está en el catálogo y simplemente aparece "sin clave".
