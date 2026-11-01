# FASE 3.1 — VRAM GUARD Y GAMING RADAR CONECTADOS

**Fecha:** 01/11/2026
**Estado:** conectados a la aplicación y probados en vivo con dos motores cargados.
**Proyecto:** `H:\KUKIA_CODE` (el existente).

---

## 1. Qué faltaba y qué se ha hecho

Los módulos `core/src/vram.rs` y `core/src/gaming.rs` existían pero **no estaban conectados**:
`src-tauri` no los importaba y la UI no tenía vistas. Ahora sí:

| Pieza | Antes | Ahora |
|---|---|---|
| Comandos Tauri | ninguno | `vram_report`, `vram_fits`, `gaming_scan`, `gaming_items`, `gaming_alerts`, `gaming_report`, `gaming_lab_path` |
| Vistas | ninguna | `Vram.tsx`, `Gaming.tsx` + entradas "VRAM Guard" y "Gaming Radar" en el menú |
| Motores vigilados | Ollama y LM Studio | **+ ComfyUI**, que es el que más VRAM se come en esta máquina |

Se añadió ComfyUI al guardián porque sin él la competencia más habitual aquí (Ollama + ComfyUI
en la misma tarjeta) era invisible.

## 2. Prueba con dos motores cargados a la vez (medido)

Procedimiento real: se generó una imagen con ComfyUI (carga SDXL en GPU1) y se cargó
`dolphin-mistral:latest` en Ollama con `keep_alive=10m`.

Estado durante la prueba:

```
nvidia-smi    GPU0 RTX 5070      3384 MiB usados /  8560 libres
              GPU1 RTX 5060 Ti   5642 MiB usados / 10409 libres
Ollama /ps    dolphin-mistral:latest  5,17 GB en VRAM
```

Informe de la guardia:

```
  ollama    online=true  modelos=1 · 5295 MB · 1 modelo(s) cargado(s)
  lmstudio  online=true  modelos=0 ·    0 MB · puerto 1235: 0 cargado(s)
  comfyui   online=true  modelos=1 · 1177 MB · cuda:0 RTX 5060 Ti · 1177 MB de 16283 MB
competencia: true → "COMPETENCIA DE VRAM: 2 motores tienen modelos cargados a la vez:
                     ollama (1), comfyui (1)"
```

**Resultado: la guardia detecta la competencia con los dos motores dentro.** Los dos modelos
acabaron en la misma tarjeta (la 5060 Ti), que es justo el caso peligroso.

## 3. Fallos reales encontrados y corregidos

### 3.1 ComfyUI decía tener menos VRAM de la que tiene → falso "sin competencia"

Con SDXL cargado, ComfyUI publicaba **1.177 MB** mientras `nvidia-smi` marcaba **5.642 MiB** en
esa GPU. El umbral estaba en 1.500 MB, así que la guardia informaba `competencia: false` con dos
motores llenos. **Es un falso negativo, el peor tipo de error para un aviso.**

- Corregido: umbral a **800 MB** (por encima de eso no es solo contexto de torch) y la interfaz
  advierte de que es la cifra que da ComfyUI y que torch reserva más de lo que declara.
- Volvió a pasarse: `competencia: true`.

### 3.2 El Gaming Radar duplicaba fichas dentro del mismo escaneo

Primer escaneo: 39 descubiertos, pero solo **36** se guardaban. Los otros 3 eran el mismo
proyecto salido en *releases* y en *búsqueda*.

- Corregido: `scan_all()` deduplica por id antes de devolver.
- Verificado: 19 de 19 persistidos, y una segunda pasada da **0 nuevos** (idempotente).

### 3.3 nvidia-smi no atribuye MB por proceso en esta máquina

Devuelve `N/A` para casi todo (WDDM). La vista listaba 30 filas con "N/A" sin aportar nada.

- Corregido: se filtran las filas sin memoria y se explica que en esta máquina se mide **por
  motor**, no por proceso.

## 4. Gaming Radar: resultados medidos

| Escaneo | Descubiertos | Nuevos | Alertas | Informe |
|---|---|---|---|---|
| 1.º (sin deduplicar) | 39 | 36 | 20 | — |
| Tras corregir duplicados | 19 | 19 | 3 | 20.725 bytes |

- Fuentes que respondieron: **arXiv 15**, releases de GitHub 4 (dlss-swapper, SpecialK,
  rtx-remix, OptiScaler, Streamline, llama.cpp).
- **Aviso importante:** la búsqueda de GitHub sin token está limitada a ~10 peticiones por
  minuto. El segundo escaneo, hecho pocos minutos después, devolvió **0 resultados de búsqueda**
  y solo 19 fichas en total. No es un fallo del radar: es el límite de la API. Para escaneos
  seguidos hace falta un token de GitHub.
- Informe en `H:\KUKIA_RESEARCH\GAMING_GRAPHICS\reports\2026-11-01.md`.
- Laboratorio creado con sus 8 carpetas (`discoveries`, `reports`, `benchmarks`,
  `compatibility`, `downloads`, `lab`, `alerts`, `archive`).

## 5. Verificación ejecutada

```
cargo test -p kukia-core                          → 56 unit: OK (3 nuevos de VRAM)
cargo test -p kukia-core --test validacion_gaming -- --ignored
                                                  → 2/2 OK (red real + SQLite temporal)
cargo test ... el_informe_incluye_los_tres_motores -- --nocapture
                                                  → competencia detectada con 2 motores
cargo check -p kukia-code                         → OK (comandos registrados)
cd app && npm run build                           → tsc + vite: OK (50 módulos)
```

## 6. Límites honestos

1. **Las vistas se han compilado y tipado, pero no se han pulsado dentro de una ventana Tauri.**
   Lo probado en vivo son las funciones del núcleo que envuelven los comandos (una línea cada
   comando). Falta abrir la app y hacer clic.
2. **ComfyUI no expone el nombre del modelo cargado**: la guardia informa VRAM y lo dice.
3. El umbral de 800 MB para ComfyUI es una **heurística declarada**, no una lectura exacta.
4. La competición se detecta a nivel de motor, **no se atribuye a una GPU concreta** cuando el
   motor no la publica (Ollama no dice en qué tarjeta está).
5. El Gaming Radar depende de APIs públicas sin token: escaneos seguidos devuelven menos.
6. No hay escaneo automático programado del Gaming Radar (solo manual desde la vista). El de
   Research sí corre cada 6 h.

## 7. Siguiente paso natural

- Abrir la app y hacer clic en las dos vistas (verificación de interfaz real).
- Atribuir GPU por motor leyendo `nvidia-smi --query-compute-apps` con `pid` de cada motor
  cuando el driver lo permita, o con `CUDA_VISIBLE_DEVICES` fijado por motor.
- Token de GitHub para que el radar no se quede a medias en escaneos seguidos.
- Aviso emergente (no solo tarjeta) cuando la competencia aparece con un modelo tuyo en marcha.
