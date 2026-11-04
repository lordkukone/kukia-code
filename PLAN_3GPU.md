# PLAN 3 GPU (2 PCs) — Fase 0: reconocimiento

Fecha: 04/11/2026 · Rama: `feature/3gpu-local` · **Sin código todavía**, como pediste.

## 1. Lo que ya existe y se reutiliza

| Necesidad | Dónde | Qué hace hoy |
|---|---|---|
| GPUs locales | `core/src/vram.rs:92 gpus_vram()` · `core/src/gpus.rs:116 tarjetas()`, `:172 nvidia_smi()` | Lee las 2 GPUs con nvidia-smi + WMI (total/uso/libre/utilización) |
| Estado de motores | `core/src/engines.rs:112 EngineStatus` | Ollama 11434 fijo; LM Studio sondea 1234/1235 y lee `lms server status` |
| Estimación previa | `core/src/engines.rs preflight()` | Ollama: tamaño de `/api/tags` + 12 % + KV. LM Studio: `lms load --estimate-only` |
| ¿Cabe? | `core/src/vram.rs:327 fits()` | Criterio **una GPU**, 12 % de margen |
| Chat streaming | `core/src/providers.rs:117 base()`, `join_api()`, `ChatEvent` | Ollama, LM Studio y cloud OpenAI-compatible |
| Servidor OpenAI | `core/src/serve.rs` (`start/status/stop`, token, loopback) | KUKIA ya **sirve** `/v1`; aquí hace falta el lado cliente |
| GGUF en disco | `core/src/hub.rs:233 raices_gguf()`, `:357 gguf_locales()` | Rutas fijas + `%APPDATA%\KukiaCode\models`, cabecera GGUF leída |
| Config | `core/src/config.rs AppConfig` | JSON en `%APPDATA%\KukiaCode\config\config.json`, sin claves |
| Comandos UI | `src-tauri/src/lib.rs:2903 generate_handler![…]` | Lista plana (`vram_*`, `gpu_stats`, `studio_*`…) |
| Parada de procesos | `core/src/security.rs:75` (`taskkill`), `core/src/entrenador.rs:677` (python + streaming) | Patrón reutilizable |
| Vistas | `app/src/views/Gpu.tsx` (131), `Models.tsx` (567) | Ahí va el selector de perfil |

## 2. Huecos reales

1. **Cero llama.cpp**: no hay una sola referencia a `llama-server`, `rpc-server`, `--rpc` ni `-ngl` en `core/`, `src-tauri/` ni `app/`.
2. **No hay supervisor de procesos largos**: todos los `Command::new` son de un tiro; falta dueño del hijo, parada limpia y reinicio.
3. **`fits()` y `preflight()` razonan por GPU suelta**, no por la suma de un perfil de 2/3 GPUs.
4. **La VRAM del i7 es invisible**: `nvidia-smi` es local y `rpc-server` no publica VRAM.
5. **`gpu_split: [u32;2]`** es de Ollama; llama.cpp necesita `--tensor-split` de 3 valores.
6. **`raices_gguf()` está cableado** a `G:\descargas`, `J:\models`, `H:\models`, `C:\Users\kuki\models`.
7. **No hay nada de WinRM/remoto en el código**: M0-B sigue bloqueado por credenciales.

## 3. Arquitectura propuesta

```
core/src/localgpu/
  mod.rs      Perfil (solo_i9 | tres_gpus), estado público, reparto de VRAM
  proceso.rs  lanzar/parar/supervisar llama-server (hijo + log por stderr + parada /T)
  rpc.rs      sonda TCP al rpc-server del i7 (10 s máximo) y arranque remoto opcional
  flags.rs    construcción pura de argumentos -> testeable sin red ni binarios
```

Perfil `tres_gpus`:

```
llama-server -m <gguf> -ngl 99 --tensor-split 12,16,8 -c 8192 \
  --host 127.0.0.1 --port 8080 --rpc 192.168.50.2:50052
```

Perfil `solo_i9`: mismo comando **sin** `--rpc` y con `--tensor-split` de 2 valores.

Consumo: `providers.rs` gana la rama `"llamacpp"` en `base()` (`http://127.0.0.1:{puerto}` de
config); `join_api()` ya resuelve `/v1/chat/completions` y el streaming existente sirve tal cual.

## 4. Config nueva (todo con `#[serde(default)]`)

`AppConfig::load_config()` hace `unwrap_or_default()` si el JSON no parsea: **un campo sin
`default` borraría la configuración del usuario**. Por eso todo entra con valor por defecto.

```json
"local_gpu": {
  "llama_server": "", "rpc_server": "", "modelo": "", "perfil": "solo_i9",
  "puerto": 8080, "ctx": 8192, "ngl": 99, "tensor_split": [50, 50, 0],
  "rpc_host": "192.168.50.2:50052", "vram_i7_gb": 8, "rutas_gguf": [],
  "timeout_rpc_s": 10, "auto_arranque": false
}
```

Ningún secreto: son rutas, puertos y un número de GB declarado.

## 5. Comandos Tauri y UI

`localgpu_status` · `localgpu_start` · `localgpu_stop` · `localgpu_probe_rpc` ·
`localgpu_ggufs` (reutiliza `hub::gguf_locales`).

En `Gpu.tsx`, tarjeta «3 GPUs (LAN)»: perfil, estado, últimas líneas de stderr, botón parar y
aviso explícito cuando el i7 no responde y se cae a 2 GPUs.

## 6. Riesgos y límites (sin adornos)

- El RPC de llama.cpp es **experimental**: activaciones por capa viajan por red. Con 2,5 Gbps
  medidos (185–267 MB/s) los tokens/s bajan frente a 2 GPUs locales.
- 28 + 8 = 36 GB **no** son 36 GB útiles: hay que dejar margen y el KV de contexto. Un 70B Q4
  (~40 GB) no entra; el objetivo realista es ~30–33 GB (70B Q3, o 32B con contexto largo).
- La VRAM del i7 se guarda como **dato declarado** hasta que exista agente o WinRM; el preflight
  lo dirá con esa palabra.
- `rpc-server` no tiene autenticación: sólo LAN directa y hay que decidir el firewall del i7.
- Parar mal deja VRAM ocupada: la parada va con `taskkill /PID /T` y verificación posterior.

## 7. Fases y criterios

| Fase | Entrega | Criterio de aceptación |
|---|---|---|
| 1 | `localgpu/` + comandos + config | Arranca y para llama-server; sonda del i7 en ≤10 s; sin zombis |
| 2 | proveedor `llamacpp` + selector + fallback | Chat con streaming por llama-server; sin i7 sigue en 2 GPUs |
| 3 | medición real 2 vs 3 GPUs | Tabla de tokens/s y límites, con el modelo de prueba |

Cada fase: backup `.zip` + SHA-256 en `CAMBIOS.md`, `cargo test` completo en verde y prueba real
ejecutada (no vale «debería»).

## 8. Datos que necesito antes de la Fase 1

1. **Ruta de llama.cpp** en el i9 (`llama-server.exe` y `rpc-server.exe`): ¿ya descargado? ¿qué build y CUDA?
2. **Arranque del `rpc-server` en el i7**: WinRM está abierto pero sin credenciales. ¿Me las das,
   lo arrancas tú con un `.bat`, o lo dejamos como paso manual documentado?
3. **Firewall del i7**: ¿autorizas abrir 50052 en la LAN directa?
4. **GGUF de prueba**: cuál y dónde está (para medir de verdad).
5. **Puerto 8080**: ¿libre o prefieres otro?
6. **¿La UI debe arrancar llama-server** o sólo conectar con uno ya encendido?

## 9. Estado

✅ Fase 0 entregada (este plan). ⚠️ 6 decisiones pendientes arriba. ❌ Nada de código tocado.
Siguiente paso: tu OK a la Fase 1 y las respuestas a los 6 puntos.
