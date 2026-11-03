# KUKIA CODE — CEREBRO UNIVERSAL DE MODELOS

**Fecha:** 02/11/2026
**Alcance:** auditoría de solo lectura + implementación de la capa común (catálogo, router,
adaptadores y fijado de modelo por herramienta).

---

## 1. Auditoría: lo que YA existía (y no se ha duplicado)

| Pieza | Archivo | Qué hacía |
|---|---|---|
| Chat y streaming | `core/src/providers.rs` | Ollama (API nativa), LM Studio y cloud (OpenAI-compatible), con descubrimiento de puerto de LM Studio |
| Estado y fichas de motor | `core/src/engines.rs` | CC1 estado, CC2 ficha, CC3 parámetros por modelo, CC4 carga con preflight |
| Parámetros avanzados | `core/src/control.rs` | KV cache, contexto, capacidades por GPU, perfiles |
| Descarga de VRAM | `core/src/vram.rs` | Liberar por motor, por GPU y todo, midiendo antes/después |
| Benchmarks | `core/src/lab.rs` | tok/s, prefill, TTFT y picos medidos |
| Cola persistente | `core/src/jobs.rs` | SQ1 (recién terminada) |

**Huecos reales encontrados:**

1. **No había catálogo común.** Cada vista llamaba a su proveedor o a su motor por separado.
2. **Los agentes no tenían modelo.** `Agent` solo guarda prompt y herramientas: el modelo era
   global para toda la app.
3. **ComfyUI no aparecía en ninguna lista de modelos**, aunque se usa por workflow.
4. **No existía enrutado por capacidad.** Se elegía modelo a mano, sin comprobar si servía.
5. **No había registro de qué adaptador soporta qué.** Se trataba a todos los motores igual.

## 2. Lo implementado

### 2.1 Catálogo (`core/src/catalog.rs`, 1024 líneas)

`collect(&cfg)` reúne en paralelo:

- **Ollama** — `/api/tags` + `/api/ps`.
- **LM Studio** — `/api/v1/models` con puerto descubierto.
- **ComfyUI** — `/models/checkpoints` y, si no existe, `CheckpointLoaderSimple` de `/object_info`.
- **Cloud** — `/v1/models` con la clave del proveedor activo.

Y declara, sin rellenar, los motores que **no** publican catálogo: Wan2GP, Maestro y Fish Speech.

Cada modelo lleva: clave `motor::id`, modalidad, capacidades, contexto, tamaño, cuantización,
formato, si está cargado, si es local y si puede chatear. Cuando una capacidad se deduce del
nombre (no la declara el motor), se marca `inferred = true` y la UI lo dice.

### 2.2 Router (`enrutar`, `resolver_en`)

Puntúa cada modelo y **descarta con motivo** en vez de fallar en silencio:

- descarta si no declara la capacidad pedida;
- descarta si el contexto real es menor que el mínimo;
- descarta el cloud si `allow_cloud` no viene autorizado (**por defecto no**);
- descarta modelos sin adaptador de chat cuando se pide texto;
- suma: cargado +25, local +10, contexto ≥32K +6 / ≥8K +3, contexto desconocido con mínimo −5.

### 2.3 Modelo fijado por herramienta (`model_bindings`)

Tabla en SQLite con `scope → (engine, model)`. Si la herramienta tiene modelo fijado, el router
lo respeta aunque el automático prefiera otro. Si ese modelo ya no existe, **avisa en el `note`** y
enruta automático.

### 2.4 Adaptadores declarados con honestidad (`adapters()`)

Tabla pública de qué hace cada motor de verdad: chat, streaming, embeddings, imagen, vídeo, audio,
descarga y estado. Embeddings está marcado **no** en todos: aún no hay adaptador implementado.

### 2.5 Superficie

- **Tauri:** `catalog_list`, `catalog_adapters`, `catalog_route`, `catalog_bind_set`,
  `catalog_binds`, `catalog_bind_clear`, `catalog_chat`.
- **Frontend:** vista **Cerebro** (`app/src/views/Catalog.tsx`), con cobertura por motor, router
  ejecutable, prueba de punta a punta, fijado por herramienta y tabla de adaptadores.

## 3. Medición real en esta máquina (02/11/2026)

```
cargo run -p kukia-core --example catalogo

COBERTURA
  ollama     alcanzable=true  catalogo_api=true  modelos=82
  lmstudio   alcanzable=false catalogo_api=true  modelos=0   (servidor apagado)
  comfyui    alcanzable=false catalogo_api=true  modelos=0   (8188 no responde)
  cloud      alcanzable=true  catalogo_api=true  modelos=43
  wan2gp     alcanzable=false catalogo_api=false  → sin endpoint de catálogo
  maestro    alcanzable=false catalogo_api=false  → sin endpoint de catálogo
  fish       alcanzable=false catalogo_api=false  → sin listado por API

TOTAL ENUMERADO: 125 modelos en 5.001 ms

ROUTER texto  → ollama::aratan/Qwythos-9B-v2-1M-Uncensored-GGUF:Q4_K_M (60 pts)
                43 descartados: los 43 cloud, por no estar autorizado el gasto
ROUTER vision → ollama::aratan/Qwythos-9B-…(declara vision)
ROUTER imagen → sin candidato: ComfyUI apagado, y no se inventa nada
```

## 4. Archivos

**Creados:** `core/src/catalog.rs`, `core/examples/catalogo.rs`, `app/src/views/Catalog.tsx`,
este informe.

**Modificados:** `core/src/lib.rs` (módulo), `core/src/memory.rs` (tabla `model_bindings`),
`src-tauri/src/lib.rs` (7 comandos + registro), `app/src/lib/api.ts` (tipos y métodos),
`app/src/App.tsx` (vista «Cerebro»).

**Backup verificado:** `backups/UNIV_MODELS_20261102_093718/` con SHA-256 comprobado de los cinco
archivos que se iban a tocar.

## 5. Pruebas ejecutadas

```
cargo test --lib catalog  → 10 passed, 0 failed
cargo test --lib          → 124 passed, 0 failed
cargo check -p kukia-code → exit 0
npm run build             → 55 módulos, 370.50 kB, exit 0
cargo run --example catalogo → datos reales de arriba
```

Los 10 tests cubren: clasificación de embedder/coder/visión, lo declarado no es inferido, ComfyUI
como imagen sin chat, preferencia por el cargado, cloud no autorizado, contexto insuficiente,
fijado manual que gana, fijado roto que avisa, y filtrado por capacidad.

## 6. Límites reales (lo que todavía NO hay)

1. **El chat de la app aún no pasa por el router.** El servicio existe y está expuesto, pero la
   vista Chat sigue usando su selector de proveedor/modelo. Enganchar cada herramienta es la
   siguiente fase, una por una.
2. **Contexto de Ollama desconocido en el listado masivo.** `/api/tags` no lo publica y pedirlo por
   modelo son 82 llamadas; se deja `?` y el router penaliza si se exige un mínimo. La ficha
   individual (`engines::card`) sí lo trae.
3. **Embeddings sin adaptador.** Declarado `false` en la tabla de adaptadores.
4. **Enrutado solo bloqueante.** `catalog_chat` no hace streaming todavía.
5. **LM Studio y ComfyUI estaban apagados** en la medición: no se ha podido comprobar su catálogo
   en vivo, solo el código que lo pide.
6. **El cloud se lista, pero no se usa** salvo autorización explícita desde la casilla.
