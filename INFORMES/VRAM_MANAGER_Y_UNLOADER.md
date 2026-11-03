# KUKIA CODE — VRAM MANAGER & MODEL UNLOADER

**Fecha:** 01/11/2026 · **Fase:** auditoría + implementación + prueba real
**Backup previo verificado:** `BACKUPS/VRAM_MANAGER_20261101_221719` (tamaño y SHA-256 comprobados en origen y copia)

---

## 1. Auditoría de capacidades reales (medida, no supuesta)

Sondas ejecutadas contra los motores vivos de esta máquina:

| Motor | Puerto detectado | ¿Expone modelos cargados? | ¿Descarga oficial? | Mecanismo |
|---|---|---|---|---|
| **Ollama** | 11434 | **Sí** — `/api/ps` con `size_vram` por modelo | **Sí** | `POST /api/generate` con `keep_alive: 0` |
| **LM Studio** | **1235** (no 1234) | Sí — `/api/v0/models` (`state: loaded`) | **Sí** | `POST /api/v1/models/unload` con `instance_id`, y `lms unload` como respaldo |
| **ComfyUI** | 8188 | **No** — no publica el nombre del checkpoint | **Sí, pero global** | `POST /free` (`unload_models`, `free_memory`) |
| **Wan2GP** | 7861 (`KUKIA_WAN2GP`) | **No** | **No** | No hay API: solo se libera cerrando el proceso |
| **Maestro** | 7860 (`KUKIA_MAESTRO`) | **No** | **No** | No hay API: solo se libera cerrando el proceso |

Comprobaciones literales de la auditoría:

```
puertos en escucha: 1235 (LM Studio), 8188 (ComfyUI), 11434 (Ollama)
8189 /system_stats -> 000   → el segundo ComfyUI está apagado
7861 y 7860         → nadie escucha: Wan2GP y Maestro están apagados
POST 8188/free      -> 200  y la VRAM no cambió (2379 / 212 MB): no había nada cargado
```

### El hallazgo que cambia el diseño

Ollama tenía `qwen3-vl:4b` cargado y **repartido entre las dos GPU**:

```
/api/ps:  qwen3-vl:4b · size_vram 18.24 GB
nvidia-smi antes:  GPU0 11.528 MB · GPU1 9.661 MB
descarga oficial:  GPU0   212 MB · GPU1   212 MB   → 18,6 GB liberados
```

Conclusión dura: **ningún motor local permite descargar solo la parte de un modelo
que vive en una GPU**. Los botones «LIBERAR VRAM RTX 5070» y «LIBERAR VRAM RTX 5060 Ti»
existen, pero no fingen selectividad: descargan los modelos enteros y **miden después
cuánto se recuperó en cada tarjeta**. La interfaz lo dice antes de confirmar.

---

## 2. Lo implementado

### Núcleo (`core/src/vram.rs`)

- `release_engine(motor)` — descarga todos los modelos de un motor por su mecanismo
  oficial y devuelve `ReleaseReport` con VRAM **antes/después por GPU**.
- `release_gpu(gpu)` — libera una GPU concreta con la nota honesta de arriba.
- `release_everything()` — ambas GPU de una vez.
- `recuperable_mb(engines)` — estimación de lo que se recuperaría ahora mismo
  (usa `size_vram` si el motor lo publica; si no, el tamaño del archivo como suelo).
- `motor_sin_api()` — Wan2GP y Maestro entran en la vigilancia **declarando** que no
  publican su inventario ni tienen descarga por API.
- `comparar()` usa `saturating_sub`: si otro proceso carga memoria mientras se mide,
  se informa 0 recuperado, nunca un negativo sin sentido.

### Comandos Tauri (`src-tauri/src/lib.rs`)

`vram_release_engine`, `vram_release_gpu`, `vram_release_everything`.
Todos escriben en el log de KUKIA qué se descargó y cuántos MB se recuperaron.

### Panel (`app/src/views/Vram.tsx`)

- **Por GPU:** botón «Liberar esta GPU» en cada tarjeta, con su barra de uso.
- **Por motor:** «Descargar todo» en la fila de Ollama, LM Studio y ComfyUI;
  Wan2GP y Maestro muestran `sin API` con la razón en el tooltip.
- **Por modelo:** el «Expulsar» que ya existía, intacto.
- **Confirmación previa** con: qué modelos se descargan, GPU afectada, **VRAM estimada
  a recuperar** y consecuencias (los motores siguen abiertos, los modelos recargan solos,
  no se cierra ningún proceso).
- **Tarjeta «Última liberación»** con antes / ahora / recuperado por GPU y los fallos
  concretos si los hubo.

### Seguridad respetada

No se cierra ningún proceso · no se borra ningún archivo del disco · no se descarga
nada sin pulsar el botón · no se interrumpe una inferencia en curso · la Guardia sigue
solo vigilando y avisando.

---

## 3. Prueba real (no simulada)

`cargo test --test validacion_liberacion` carga `qwen3-vl:4b` en Ollama, pulsa el
liberador y mide:

```
antes:  ollama con 1 modelo · 17.397 MB
        qwen3-vl:4b · 17.397 MB
        GPU0: 11.222 MB · GPU1: 9.653 MB
liberar ollama · ok=true · 18.499 MB recuperados
        GPU0: 2.149 MB usados  · 8,85 GB recuperados
        GPU1:   212 MB usados  · 9,22 GB recuperados
        ollama / qwen3-vl:4b -> true · descargado de VRAM (keep_alive 0)
después: ollama con 0 modelos
```

Asserts que pasan: la descarga funciona, **se mide memoria de verdad (> 0)**, el total
es la suma exacta por GPU, no queda ningún modelo cargado y ninguna GPU acaba con más
memoria que antes.

Los otros dos tests confirman que Wan2GP/Maestro y un motor inventado **no fingen éxito**:

```
wan2gp:  ok=false · no ofrece API para descargar modelos: solo libera su VRAM al cerrar el proceso
maestro: ok=false · idem
vllm:    ok=false · motor desconocido: KUKIA no conoce un mecanismo de descarga para él
```

### Verificación de compilación

- `cargo test --lib vram` → **15 tests OK** (incluye 5 nuevos del gestor).
- `cargo check` en `src-tauri` → limpio.
- `npm run build` (tsc + vite) → limpio, 52 módulos.

---

## 4. Limitaciones que quedan escritas, no escondidas

1. **Sin descarga selectiva por GPU** en ningún motor: se libera el modelo entero.
2. **ComfyUI no dice qué checkpoint tiene cargado**: se informa la VRAM retenida y se
   descarga con `/free`, que es global.
3. **Wan2GP y Maestro no tienen API de descarga**: el panel lo declara en vez de ofrecer
   un botón que fallaría. Su VRAM solo se libera cerrando el proceso, y KUKIA no cierra
   procesos de Windows.
4. **`/free` responde 200 aunque no haya nada cargado**: el éxito del HTTP no es prueba
   de que se liberara memoria; por eso todas las operaciones se miden con `nvidia-smi`.
5. **LM Studio no publica la VRAM del modelo cargado**: se usa su tamaño de archivo como
   estimación, marcada como tal, y la cifra real siempre es la de `nvidia-smi`.
