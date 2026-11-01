# KUKIA CODE — FASE 2: ENTREGA

> Fecha: 01/11/2026
> Estado: **en verificación** (este informe se actualiza solo con pruebas reales)

---

## 1. QUÉ SE HA CONSTRUIDO

Aplicación de escritorio **KUKIA CODE 0.1.0** con:

| Componente | Estado | Evidencia |
|---|---|---|
| Estructura modular (`app`, `core`, `src-tauri`, ...) | ✅ creada | 22 carpetas en `H:\KUKIA_CODE` |
| UI React + TypeScript (13 vistas) | ✅ compila | `npm run build` → 45 módulos, sin errores TS |
| Núcleo Rust independiente (`kukia-core`) | ✅ implementado | `core/src/*.rs` |
| Conector Ollama (modelos, chat, streaming) | ✅ implementado | `core/src/providers.rs` |
| Conector LM Studio / OpenAI-compatible | ✅ implementado | `core/src/providers.rs` |
| Selector de modelos y proveedores | ✅ implementado | vista `Modelos` |
| Memoria SQLite (sesiones, mensajes, recuerdos) | ✅ implementado | `core/src/memory.rs` + tests |
| Monitor GPU (nvidia-smi) | ✅ implementado | `core/src/gpu.rs` |
| Benchmarks tokens/s persistidos | ✅ implementado | tabla `benchmarks` |
| Radar de actualizaciones + compatibilidad | ✅ implementado | `core/src/radar.rs`, vista `Radar` |
| KUKIA Experimental AI Radar (arXiv/HF/GitHub + alertas + informes + lab aislado) | ✅ implementado | `core/src/research.rs`, vista `Research` |
| Sistema de logs | ✅ implementado | `%APPDATA%\KukiaCode\logs\kukia.log` |
| Aprobaciones auditadas | ✅ implementado | tabla `approvals` |
| Instalador NSIS | ✅ generado (2,62 MiB) | `target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe` |
| Tests automatizados del núcleo | ✅ 22/22 | `cargo test -p kukia-core` |
| Arranque real de la app | ✅ verificado | ventana «KUKIA CODE», 42,2 MB RAM, cierre limpio |

---

## 2. DECISIÓN TECNOLÓGICA

**Tauri 2 + React + TypeScript + Rust + SQLite.**

Justificación basada en la auditoría real:

- WebView2 156 ya instalado → sin runtime extra.
- Rust 1.98.1 (MSVC) + MSVC 14.44 + Windows SDK ya operativos.
- Bundler NSIS oficial → instalador EXE sin Inno Setup/WiX previos.
- Binario ligero y núcleo testeable sin la UI.

Alternativas descartadas: Electron (150-200 MB y más RAM), PySide6 (empaquetado y control del núcleo peores), .NET/WinUI (sin SDK .NET instalado).

---

## 3. ESTRUCTURA CREADA

```
H:\KUKIA_CODE
├── app/            UI React + TS (13 vistas)
├── core/           crate kukia-core (7 módulos + tests)
├── src-tauri/      crate kukia-code (comandos Tauri, icono, capabilities)
├── agents/         (reservado F6: agentes con tools)
├── providers/      (reservado F4: conectores adicionales)
├── models/         (reservado F4: gestión de modelos)
├── memory/         (reservado F7: memoria semántica)
├── tools/          (reservado F6)
├── terminal/       (reservado F8: pty)
├── mcp/            (reservado F6: cliente MCP)
├── research/       (reservado F9)
├── security/       (reservado F3)
├── gpu/            (reservado F3: perfiles multi-GPU)
├── benchmarks/     (salida de benchmarks)
├── plugins/        (reservado F10)
├── config/         (ejemplos de configuración)
├── logs/           (logs locales de desarrollo)
├── backups/        (backups de archivos escritos por la app)
├── docs/           ARQUITECTURA.md, MANUAL.md, ROADMAP.md
├── tests/          smoke.ps1
├── installer/      README.md (NSIS)
└── INFORMES/       AUDITORIA_INICIAL.md, FASE_2.md
```

---

## 4. PRUEBAS REALIZADAS

### 4.1 Frontend

```
npm run build
→ tsc --noEmit: sin errores
→ vite v7.3.7: 45 módulos transformados
→ dist/index.html + assets (260,63 kB JS, 10,38 kB CSS)
```

### 4.2 Núcleo Rust

```
cargo test -p kukia-core
running 18 tests
...
test result: ok. 18 passed; 0 failed; 0 ignored
```

Cobertura de las pruebas:

- `config`: perfil 35/65, proveedor por defecto, roundtrip JSON.
- `agents`: 8 agentes, ids únicos.
- `memory`: sesiones + mensajes, búsqueda de memoria, tokens/s, auditoría.
- `providers`: prompt de sistema, URLs base, serialización de eventos.
- `gpu`: CPU/RAM detectadas, parser tolerante a ausencia de GPU.
- `security`: bloqueo sin aprobación, rechazo de archivo inexistente.
- `logs`: escritura y lectura del log.
- `radar`: entradas manuales.

### 4.3 Verificación real del conector Ollama

Contra el servidor local `127.0.0.1:11434` (Ollama 0.40.13):

| Modelo | Respuesta | prompt_eval_count | eval_count | total_duration |
|---|---|---|---|---|
| `llama3-ablit:latest` | texto devuelto (rechazo genérico del modelo) | 124 | 21 | 366 ms |
| `xoxete-mistral:latest` | texto devuelto en español | — | 32 | 33.731 ms |

Esto confirma el contrato exacto que usa `core/src/providers.rs`:
`message.content`, `prompt_eval_count`, `eval_count`, `total_duration` (ns).

### 4.4 Verificación real del radar

Consulta directa a las APIs oficiales (misma que hace `core/src/radar.rs`):

```
Ollama latest:    v0.40.13 (2026-10-28)  ← coincide con la versión instalada
llama.cpp latest: v0.8.0   (2026-10-28)
```

### 4.5 Compilación de la aplicación

```
npm run tauri:build
→ vite: 46 módulos, index-r06dv11t.js (267,30 kB)
→ cargo release: Finished en 2m 10s
→ makensis: OK
→ Instalador: H:\KUKIA_CODE\target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe (2,62 MiB)
→ Binario: H:\KUKIA_CODE\target\release\kukia-code.exe (8,6 MB)
```

### 4.6 Arranque real de la aplicación

```
Start-Process kukia-code.exe
→ Proceso vivo: True (PID 1500)
→ Título de ventana: "KUKIA CODE"
→ RAM en reposo: 42,2 MB
→ CloseMainWindow: True → cierre limpio
```

Datos creados al arrancar (verificado en disco):

```
%APPDATA%\KukiaCode\kukia.db          73.728 bytes
%APPDATA%\KukiaCode\logs\kukia.log    "KUKIA CODE arrancando"
```

### 4.7 Smoke test de entrega

```
tests\smoke.ps1 → 10/10 [OK]
Instalador: 2,6 MB · Binario: 8,6 MB
```

---

## 4.6 KUKIA Experimental AI Radar

Añadido a Fase 2:

- Laboratorio: `H:\KUKIA_RESEARCH\AI_EXPERIMENTAL\{discoveries,reports,lab,downloads,benchmarks,alerts,templates,archive}`.
- Fuentes con API pública real: arXiv, Hugging Face, GitHub.
- Ficha de 12 campos por descubrimiento, guardada en SQLite.
- Puntuación heurística transparente y alertas por palabras de alto valor (NVFP4, FP4, expert parallel, MoE, KV cache, speculative decoding…).
- Informe diario en `reports\YYYY-MM-DD.md`.
- Descarga aislada: solo https, máximo 25 MB, a `downloads\`, sin ejecución.
- Vigilancia automática cada 6 h si `research_auto` está activo (solo metadatos; nunca instala).
- Plantilla de ficha en `templates\FICHA_DESCUBRIMIENTO.md`.

## 5. LO QUE NO ESTÁ TERMINADO (honestidad)

- Cliente MCP: **no implementado** (F6).
- Investigación autónoma: **no implementada** (F9).
- Terminal pty integrada: **no implementada** (F8); hay ejecución PowerShell con aprobación.
- Editor con resaltado: **no implementado** (F7); hay editor de texto plano.
- Actualizador firmado: **no implementado** (F10); el radar solo informa.
- Reparto multi-GPU activo: **solo perfil guardado**; cada motor aplica lo que soporta.
- Descarga/gestión de modelos desde la app: **no implementada** (F4).
- Licencias/DRM: **no implementados** (F10).

---

## 6. SEGURIDAD

- `files.write` y `terminal.run` exigen `approved: true` y quedan registrados en SQLite.
- Backup automático antes de sobrescribir un archivo.
- `KUKIA_CLOUD_KEY` se lee del entorno; nunca se escribe en disco.
- No se modifica OpenCode, Freebuff, Pinokio, CUDA, drivers ni modelos.
- No hay auto-actualización ni descargas automáticas.

---

## 7. INSTALADOR

Configurado en `src-tauri/tauri.conf.json`:

- Producto: `KUKIA CODE`
- Identificador: `com.kukia.code`
- Target: `nsis` (instalación perMachine, idiomas ES/EN)
- Icono: `src-tauri/icons/icon.ico`

Salida esperada: `src-tauri\target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe`.

---

## 8. SIGUIENTE FASE

F3: permisos por herramienta, auditoría avanzada y recuperación de tareas interrumpidas.
