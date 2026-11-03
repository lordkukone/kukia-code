# KUKIA MODEL HUB — INFORME

**Fecha:** 02/11/2026
**Estado:** descubrimiento real (Ollama + Hugging Face), GGUF locales con lectura de cabecera,
evaluación de compatibilidad con el hardware real y descarga de Ollama con progreso medido.

---

## 1. Auditoría previa

| Pieza | Estado encontrado | Decisión |
|---|---|---|
| `engines.rs` | Estado, fichas, parámetros y carga con preflight | **Reutilizado** para «instalados» |
| `catalog.rs` | Catálogo universal + router | **Reutilizado**: el hub alimenta a ese catálogo |
| `vram.rs` | Reporte de VRAM y liberación | **Reutilizado** (no se duplica) |
| `lab.rs` | Benchmarks medidos | **Reutilizado** |
| `jobs.rs` | Cola persistente con progreso | **Reutilizado** para la descarga |
| `research.rs` | Escaneo de Hugging Face | Se solapa; el hub usa la API oficial con filtro GGUF |

No existía nada de: búsqueda de modelos, lectura de GGUF locales, evaluación de compatibilidad,
ni descarga desde la app.

## 2. Comprobación de endpoints (antes de diseñar)

```
https://ollama.com/api/search?q=llama   → HTTP 404  {"error":"path \"/api/search\" not found"}
https://registry.ollama.ai/v2/library/qwen3/tags/list → 404 page not found
https://registry.ollama.ai/v2/library/qwen3/manifests/latest → 200 (manifiesto con capas y tamaños)
https://ollama.com/search?q=qwen3       → 200 (77.728 bytes de HTML con enlaces /library/…)
https://huggingface.co/api/models?search=qwen3&filter=gguf → 200 (JSON completo)
POST http://127.0.0.1:11434/api/pull    → responde (500 con modelo inexistente)
```

Por eso: **la búsqueda de Ollama se hace leyendo su HTML público** (declarado en la UI), los
tamaños salen del **manifiesto del registry**, y Hugging Face va por su **API oficial**.

## 3. Lo implementado (`core/src/hub.rs`, ~600 líneas)

- **`buscar_ollama(q)`** — 12 resultados reales para «qwen3», sin duplicados.
- **`manifiesto_ollama(modelo)`** — tamaño real y digest: `qwen3:8b` → **4.87 GB**.
- **`buscar_hf(q, limite)`** — API oficial con licencia, descargas, fecha y pipeline.
- **`gguf_locales(raíces, max)`** — recorre G:\, J:\ y el directorio de datos, lee la **cabecera
  GGUF** (magic, versión, `general.architecture`) y detecta partes de archivos divididos.
  Marca como **inválido** lo que no es un GGUF: se encontraron archivos de 15 y 105 bytes con
  «Entry not found» y «awaiting a review» que parecían modelos.
- **`evaluar(params, quant, ctx, ram, gpus)`** — pesos = params × bits/8, KV estimada,
  1,5 GB de margen. Veredictos: COMPATIBLE / CON LIMITACIONES (reparto o offload) /
  NO COMPATIBLE / DESCONOCIDO. **Nunca suma las dos VRAM como una sola**.
- **`pull_ollama(job, modelo)`** — `POST /api/pull` en streaming, escribiendo el progreso real
  (`completed/total`) en la fila del trabajo.

Trabajo de cola nuevo: `model_pull`. Comandos: `hub_buscar_ollama`, `hub_buscar_hf`,
`hub_manifest`, `hub_gguf`, `hub_compat`, `hub_pull`. Vista: **MODEL HUB** con cuatro pestañas.

## 4. Medición real

```
=== BÚSQUEDA EN OLLAMA ===
  12 resultado(s): qwen3.8, qwen3.8-flash-next, qwen3.5, qwen3.6, qwen3, qwen3-coder, qwen3-vl…
=== MANIFIESTO ===
  qwen3:8b  4.87 GB  digest=sha256:05a61d37…
=== HUGGING FACE ===
  5 resultado(s), licencias apache-2.0, descargas de 3,1 M a 7,6 M
=== GGUF EN DISCO ===
  8 archivos, de los cuales 3 INVÁLIDOS:
    mmproj-Qwen2.5-VL-32B-Q8_0.gguf       15 bytes  «Entry not found»
    sentinel-r2.1-Q8_0.gguf              105 bytes  «awaiting a review»
  válidos: ltx-2.5-22b…Q4_K_M (11.38 GB, arch=ltxv), Qwen2.5-VL-32B…Q3_K_S (5.89 GB, qwen2vl),
           Underdog-Saluki-27B…IQ2 (7.78 GB, qwen35), y su mmproj (clip)
=== COMPATIBILIDAD (hardware de ahora: RAM 38.5 GB, GPU0 8.3 libre, GPU1 15.6 libre) ===
  8B  Q4_K_M @ 8K   → COMPATIBLE        7.4 GB, cabe en GPU0
  32B Q4_K_M @ 16K  → CON LIMITACIONES  offload: 14.4 GB en GPU + 15.3 GB en RAM
  70B Q4_K_M @ 8K   → NO COMPATIBLE     pide 53.3 GB
```

El primer cálculo daba 70B como «compatible con limitaciones» porque la condición de offload era
demasiado optimista. Se corrigió a `resto = total − VRAM_útil` y ahora el veredicto es correcto.

## 5. Archivos

**Creados:** `core/src/hub.rs`, `core/examples/hub.rs`, `app/src/views/ModelHub.tsx`, este informe.
**Modificados:** `core/src/lib.rs`, `core/src/jobs.rs` (tipo `model_pull`), `src-tauri/src/lib.rs`
(6 comandos + despacho), `app/src/lib/api.ts`, `app/src/App.tsx` (vista «Model Hub»).

## 6. Pruebas

```
cargo test --lib          → 133 passed, 0 failed   (6 nuevos de hub)
cargo check -p kukia-code → exit 0
npm run build             → 56 módulos, 393.76 kB, exit 0
cargo run --example hub   → la medición de arriba
```

Los 6 tests cubren: bits por cuantización, 8B Q4 cabe en la 5060, 70B no cabe, sin parámetros →
DESCONOCIDO, cabecera GGUF leída de un archivo sintético, y un archivo que no es GGUF.

## 7. Límites reales (lo que NO hay)

1. **LM Studio sin descargas**: no se ha implementado `lms download` porque no hay comando
   documentado verificado en esta versión. La pestaña LM Studio del hub no existe todavía; sus
   modelos se ven en el catálogo universal.
2. **Importación a Ollama/LM Studio**: solo se detectan y analizan GGUF; no se crea el modelo con
   `ollama create` ni se registra en LM Studio.
3. **Descarga de GGUF de Hugging Face**: no implementada. Buscar y calcular compatibilidad sí.
4. **Sin verificación de hash** de lo descargado (Ollama lo hace por su cuenta con digests).
5. **Sin reanudación propia ni borrado**: borrar modelos no se toca (regla del proyecto).
6. **Búsqueda de Ollama por HTML**: si cambian la página, se rompe. Está aislada en una función y
   con mensaje de error explícito.
7. **KV estimada**: aproximación por parámetros, no lectura de la arquitectura real.
8. **Sin secciones de Favoritos, Historial de actualizaciones ni «Actualizaciones»** del diseño
   original: no se han inventado.
9. La descarga no se ha lanzado de verdad (habría bajado GB sin pedirlo): el camino está
   implementado y encolado, pero **no probado end-to-end contra un pull real**.
