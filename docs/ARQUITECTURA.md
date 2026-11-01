# KUKIA CODE — Arquitectura

> Versión 0.1.0 (Fase 2) · 01/11/2026

## Decisión tecnológica

**Tauri 2 + React + TypeScript + núcleo Rust + SQLite.**

Motivos (verificados en la auditoría, no supuestos):

1. **WebView2 156 ya está instalado** en el equipo → Tauri no añade runtime propio.
2. **Rust 1.98.1 con toolchain MSVC + MSVC 14.44 + Windows SDK** ya funcionan → compilación nativa sin instalar nada global.
3. **Instalador NSIS/MSI** generado por el bundler oficial de Tauri, sin depender de Inno Setup/WiX preinstalados.
4. Binario final ligero (~10-20 MB) frente a ~150-200 MB de Electron.
5. El núcleo en Rust permite terminal (pty), procesos, GPU y red sin puentes frágiles.
6. **Node 24.18 + npm 11.16 + Vite 7** ya disponibles para la UI.

Descartado: Electron (más peso y RAM para el mismo resultado), PySide6 (peor empaquetado y menos control del núcleo), .NET/WinUI (no hay SDK .NET instalado y no aporta ventaja aquí).

## Capas

```
app/            React + TypeScript + Vite (UI)
core/           crate kukia-core (lógica pura, testeable, sin Tauri)
src-tauri/      crate kukia-code (comandos Tauri, estado, ventana)
```

`core` no conoce Tauri: se puede probar con `cargo test -p kukia-core` y reutilizar en CLI o servicio.

## Módulos del núcleo

| Módulo | Responsabilidad |
|---|---|
| `config` | Configuración por usuario en `%APPDATA%\KukiaCode`, perfil GPU 35/65 |
| `providers` | Ollama (API nativa), LM Studio y cloud (OpenAI-compatible), chat y streaming |
| `memory` | SQLite: sesiones, mensajes, memorias, aprobaciones, benchmarks, compatibilidad, caché del radar |
| `gpu` | `nvidia-smi` (utilización, VRAM, temperatura, consumo, relojes) + `sysinfo` (CPU/RAM) |
| `agents` | 8 agentes iniciales + agentes personalizados (`config/agents.json`) |
| `security` | Ejecución PowerShell con aprobación obligatoria y límite de lectura de archivos |
| `logs` | Log de aplicación en `%APPDATA%\KukiaCode\logs\kukia.log` |
| `radar` | Consulta releases oficiales (GitHub API) y fuentes manuales |

## Flujo de chat

1. La UI llama `chat_stream` con `Channel<ChatEvent>`.
2. El comando clona la configuración y lanza el streaming del proveedor.
3. `core::providers` emite `token`, `done` o `error` por un canal tokio.
4. Un forwarder traduce cada evento al `Channel` de Tauri.
5. La UI acumula tokens y guarda el mensaje final en SQLite.

## Seguridad

- `files.write` y `terminal.run` **exigen** `approved: true`; sin eso devuelven error.
- Cada aprobación se registra en la tabla `approvals` (auditoría inmutable desde la UI).
- Antes de sobrescribir un archivo existente se copia a `%APPDATA%\KukiaCode\backups\`.
- La clave cloud **no se guarda**: se lee de `KUKIA_CLOUD_KEY`.
- Nada se instala ni actualiza automáticamente.

## Datos de usuario

```
%APPDATA%\KukiaCode\
  config\config.json
  config\agents.json
  logs\kukia.log
  backups\
  benchmarks\
  kukia.db
```

## Multi-GPU

- Perfil inicial: GPU 0 (RTX 5070) 35 %, GPU 1 (RTX 5060 Ti) 65 %.
- El reparto se guarda en configuración.
- **No se confunde VRAM con cálculo**: Ollama/llama.cpp aplican offload por capas; LM Studio gestiona su propia VRAM. KUKIA CODE no impone métodos que el motor no soporte.

## Radar de actualizaciones

- `radar_check` consulta GitHub Releases de Ollama y llama.cpp (API pública).
- LM Studio, PolyStrata, CUDA y driver NVIDIA se listan como consulta manual con enlace oficial: no se inventan versiones.
- Los resultados se cachean en `radar_cache`.
- **Nunca descarga ni instala**: solo informa.
