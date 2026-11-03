# GESTOR DE VRAM — DESCARGA SELECTIVA

**Fecha:** 02/11/2026
**Estado:** multiselección, medición antes/después, verificación en disco e historial
implementados y compilados.

---

## 1. Auditoría: lo que ya existía (no se ha duplicado)

| Pieza | Estado |
|---|---|
| Informe de VRAM por GPU (total/usado/libre) | Ya existía (`vram::report`) |
| Modelos cargados por motor | Ya existía, con VRAM por modelo vía `size_vram` de Ollama |
| Descarga individual | Ya existía: Ollama con `keep_alive: 0`, LM Studio con `lms unload`, ComfyUI con `POST /free` |
| Descarga por motor / por GPU / global | Ya existía con medición antes/después |
| Procesos en GPU | Ya existía (con la limitación de WDDM) |
| Aviso de competencia de VRAM | Ya existía |
| Vista `Vram.tsx` | Ya existía con botones individuales |

**Lo que faltaba de tu lista:**

1. **Selección múltiple** para descargar varios modelos a la vez, incluso de motores distintos.
2. **Verificación de que el modelo sigue en disco** después de sacarlo de memoria.
3. **Historial persistente** de operaciones.
4. **VRAM por modelo visible en la tabla** con la selección.

## 2. Lo implementado

### `vram::descargar_lote(items)`

- Mide **VRAM antes** (`nvidia-smi`) y **después** de todo el lote.
- Descarga cada modelo por el mecanismo oficial de su motor, con 900 ms de respiro entre uno y otro.
- Devuelve `liberado_mb` medido (solo si baja; nunca negativo).
- Comprueba **qué sigue cargado** de lo pedido.
- Comprueba **si el archivo sigue instalado** (`sigue_en_disco`): Ollama vía `/api/tags`, LM Studio
  vía `/api/v1/models`, ComfyUI → `null` (no publica listado fiable por nombre).
- Si ComfyUI entra en la selección, **avisa** de que su `/free` libera todos los modelos, no uno.

### Historial (`vram_ops`)

Tabla nueva en SQLite: motor, modelo, resultado, MB medidos y nota. Se escribe una fila por modelo
y otra con el total del lote cuando hay VRAM recuperada. Se muestra en la vista con fecha local.

### Interfaz

En «Motores vigilados»: casilla por modelo, casilla de cabecera por motor, contador de
seleccionados, **«Descargar seleccionados»**, «Limpiar», panel de resultado con la verificación en
disco y tabla de historial.

## 3. Medición real (solo lectura)

```
cargo run -p kukia-core --example vram_estado

GPU0 RTX 5070      total 12227 MB · usado  3794 MB · libre  8150 MB
GPU1 RTX 5060 Ti   total 16311 MB · usado   293 MB · libre 15758 MB

ollama   online · 0 modelos cargados
lmstudio sin respuesta en 1234 ni 1235
comfyui  sin respuesta en 8188
wan2gp   sin respuesta en 7861
maestro  sin respuesta en 7860
competencia: false
```

Con ese estado **no había nada que descargar**, así que no se ha ejecutado ninguna descarga real:
cargar un modelo solo para probar habría ocupado VRAM sin que me lo pidieras.

## 4. Archivos

**Creados:** `core/examples/vram_estado.rs`, este informe.
**Modificados:** `core/src/vram.rs` (`Seleccion`, `LoteResult`, `sigue_en_disco`, `descargar_lote`,
historial + 1 test), `core/src/memory.rs` (tabla `vram_ops`), `src-tauri/src/lib.rs`
(`vram_unload_seleccion`, `vram_historial`), `app/src/lib/api.ts`, `app/src/views/Vram.tsx`.

## 5. Pruebas

```
cargo test --lib          → 134 passed, 0 failed  (historial: registra y devuelve lo último)
cargo check -p kukia-code → exit 0
npm run build             → 56 módulos, 398.62 kB, exit 0
cargo run --example vram_estado → el estado real de arriba
```

## 6. Límites reales

1. **La descarga por lotes no se ha ejecutado** contra un modelo vivo: no había ninguno cargado.
2. **LM Studio y ComfyUI estaban apagados**: su ruta ya existía y no se ha podido reconfirmar hoy.
3. **ComfyUI no permite descargar un modelo suelto**: su API solo ofrece liberar todo (`/free`).
   Se avisa antes de ejecutar; no se finge lo contrario.
4. **No se puede atribuir MB liberados a un modelo concreto**: se mide el delta global de VRAM.
   Windows/WDDM retiene memoria que no baja del contador.
5. **No se distingue «generando»**: ningún motor expone un estado fiable de generación por modelo,
   así que no se inventa. Solo cargado / no cargado.
6. **No se borra nada del disco** en ningún caso: solo se consulta si sigue instalado.
