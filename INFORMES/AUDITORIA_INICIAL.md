# KUKIA CODE — AUDITORÍA INICIAL (FASE 1)

> Fecha: 01/11/2026
> Estado: **SOLO LECTURA — no se ha instalado ni modificado nada**
> Autor: Xoxete (para Kuki / LordKukone)

---

## 1. HARDWARE DETECTADO

| Componente | Detalle |
|---|---|
| CPU | Intel Core i9-10900K @ 3.70 GHz — 10 núcleos / 20 hilos |
| RAM | 63,9 GB DDR4 |
| GPU 0 | NVIDIA GeForce RTX 5070 — 12.227 MiB (12 GB) |
| GPU 1 | NVIDIA GeForce RTX 5060 Ti — 16.311 MiB (16 GB) |
| Driver NVIDIA | 621.01 |
| CUDA Toolkit (nvcc) | 13.4 (build V13.4.92) |
| Sistema | Windows 11 Pro, build 26300, 64 bits |

**Nota:** no usar las dos GPUs a la vez para modelos grandes (regla ya conocida del CEREBRO).

---

## 2. SOFTWARE Y VERSIONES

### 2.1 Desarrollo
| Herramienta | Versión | Ruta |
|---|---|---|
| Node.js | v24.18.0 | — |
| npm | 11.16.0 | — |
| Bun | 1.4.4 | — |
| Python | 3.12.9 | `C:\Users\kuki\AppData\Local\Programs\Python312` |
| pip | 26.1.2 | — |
| Git | 2.55.0.windows.2 | — |
| Rust (rustc) | 1.98.1 | `C:\Users\kuki\.cargo\bin` |
| Cargo | 1.98.1 | — |
| Rust toolchain | stable-x86_64-pc-windows-**msvc** (default) | `C:\Users\kuki\.rustup` |
| Go | 1.24.5 | `C:\Program Files\Go\bin\go.exe` |
| GCC | 16.2.0 (MSYS2) | `C:\msys64\mingw64\bin\gcc.exe` |
| MSVC (cl) | 14.44.35207 | `D:\VisualStudio2022\VC\Tools\MSVC\...\Hostx64\x64\cl.exe` |
| Windows SDK | 10.0.19041 y 10.0.26100 | `C:\Program Files (x86)\Windows Kits\10` |
| VS Code | 1.143.0 | — |
| .NET SDK | **NO instalado** (solo host runtime) | — |
| Clang | **NO disponible** en `C:\msys64\clang64` | — |

### 2.2 IA local
| Herramienta | Versión | Estado |
|---|---|---|
| Ollama | 0.40.13 | **Activo** — API `http://127.0.0.1:11434` responde |
| Modelos Ollama | ~80 modelos listados | `D:\ollama\models` → **518,7 GB / 352 archivos** |
| LM Studio | 1.19.3 | Instalado; API `:1234` **no responde** (servidor apagado) |
| Modelos LM Studio | 14 GGUF | `C:\Users\kuki\.lmstudio\models` → **196,2 GB** |

Variables de entorno Ollama detectadas:
- `OLLAMA_MODELS=D:\ollama\models`
- `OLLAMA_CONTEXT_LENGTH=32768`
- `OLLAMA_KV_CACHE_TYPE=q4_0`

### 2.3 Herramientas de escritorio / agentes
| Herramienta | Versión | Estado |
|---|---|---|
| OpenCode | 1.19.3 (`opencode-ai@1.19.3`) | Global npm |
| Freebuff Desktop | Electron (app instalada) | Corriendo (5 procesos) |
| n8n | 2.29.11 | Global npm |
| @openai/codex | 0.163.0 | Global npm |
| WebView2 Runtime | 156.0.4314.50 | ✅ (requisito de Tauri) |
| Tauri CLI | **NO instalado** | — |
| Electron (global) | **NO instalado** | — |
| Inno Setup / NSIS / WiX | **NO instalados** | — |

### 2.4 Librerías Python clave ya disponibles
```
PySide6==6.11.1 (+ Addons/Essentials)   FastAPI==0.128.8
Flask==3.1.3                            uvicorn==0.41.0
cryptography==49.0.0                    httpx==0.28.1 / aiohttp==3.13.3
pydantic==2.12.5 (+ settings)           rich==14.3.2
websockets==15.0.1                      requests==3.2.5
torch 2.12.1+cu130 → CUDA 13.0 disponible, 2 GPUs visibles
```
**NO instalados:** PyInstaller, Nuitka (empaquetado Python a EXE).

---

## 3. ESPACIO DE ALMACENAMIENTO

| Unidad | Usado (GB) | Libre (GB) |
|---|---:|---:|
| C: | 1.657,9 | **111,1** |
| D: | 6.940,1 | 511,9 |
| E: | 2.535,0 | 259,4 |
| F: | 3.496,5 | 229,4 |
| G: | 2.530,8 | 263,8 |
| **H: (destino)** | 2.682,9 | **111,6** (total 2.794,5 GB) |
| I: | 3.699,8 | **26,2** ⚠️ |
| J: | 902,2 | **29,4** ⚠️ |
| K: | 934,2 | **18,8** ⚠️ |
| L: | 2.576,2 | 218,3 |
| O: | 1.121,1 | **709,9** |

**Conclusión:** H: tiene 111,6 GB libres → suficiente para código, dependencias y builds (~5-15 GB). **No** para almacenar modelos. Si hace falta más espacio, O: (709,9 GB libres) es la mejor opción.

---

## 4. ESTADO DEL PROYECTO EN H:

- `H:\KUKIA_CODE` → **no existe** (se creará con el informe).
- `H:\Auditoria_KUKIA` → existe (proyecto KUKIA Network previo: wallets, rondas, tools, venv). **No tocar.**
- `H:\Laboratorio Personal Virus Kuki` → existe. **No tocar.**
- `H:\SUPER XOXETE` → existe. **No tocar.**
- `H:\KUKIA_RECOVERY_MEDIA` y `H:\KUKIA_RECOVERY_VM` → existen. **No tocar.**

---

## 5. FRAMEWORK RECOMENDADO

### ✅ Recomendación principal: **Tauri 2 + React/TypeScript**

| Criterio | Tauri 2 | Electron | PySide6 |
|---|---|---|---|
| Requisitos ya cumplidos | WebView2 ✅ + Rust/MSVC ✅ | Node ✅ | PySide6 ✅ |
| Tamaño instalador | ~8-15 MB | ~150-200 MB | ~40-80 MB |
| RAM en reposo | ~60-120 MB | ~200-400 MB | ~80-150 MB |
| Instalador EXE nativo | NSIS/MSI automático | NSIS vía electron-builder | PyInstaller (manual) |
| Seguridad (sandbox) | Alta (Rust + capabilities) | Media | Media |
| Curva de aprendizaje | Media-alta (Rust) | Baja (JS) | Baja (Python) |
| Actualizador firmado | Plugin oficial | electron-updater | Manual |

**Por qué Tauri:** WebView2 ya está instalado, Rust y MSVC ya funcionan, el instalador NSIS sale del propio bundler sin dependencias extra, y el binario es ligero. El núcleo en Rust encaja con procesos, terminal, MCP y conectores.

**Alternativa si se prioriza velocidad de desarrollo:** Electron + React (todo JS/TS, ecosistema enorme), asumiendo +150 MB de instalador.

**Alternativa si se quiere todo Python:** PySide6 (ya instalado), con PyInstaller para el EXE (habría que instalarlo).

---

## 6. ARQUITECTURA PROPUESTA

```
┌─────────────────────────────────────────────────────────┐
│  UI (React + TypeScript + Vite + Tailwind)              │
│  Monaco (editor) · xterm.js (terminal) · chat streaming │
└──────────────────────┬──────────────────────────────────┘
                       │ IPC Tauri (comandos + eventos)
┌──────────────────────▼──────────────────────────────────┐
│  NÚCLEO (Rust)                                          │
│  · Config + keyring (secretos)                          │
│  · Router de modelos (Ollama / LM Studio / cloud)       │
│  · Orquestador de agentes + sistema de permisos         │
│  · Auditoría (SQLite: acciones, diffs, aprobaciones)    │
│  · Memoria persistente (SQLite + embeddings opcional)   │
│  · Cliente MCP (rmcp)                                   │
│  · Servidor local axum (API interna / plugins)          │
│  · Gestor de proyectos y sesiones                       │
└───────┬───────────────────────┬─────────────────────────┘
        │                       │
┌───────▼────────┐     ┌────────▼────────────────────────┐
│ Conectores LLM │     │ Sidecar Python (opcional)       │
│ · Ollama :11434│     │ · ML/embeddings/visión          │
│ · LM Studio    │     │ · FastAPI + PySide6 no requerido│
│ · Cloud (opt.) │     └─────────────────────────────────┘
└────────────────┘
```

**Módulos funcionales:**
1. **Selector universal de modelos** — lista unificada de Ollama, LM Studio y cloud; health-check y fallback.
2. **Agentes especializados** — perfiles con prompt, tools y permisos (lectura, escritura, red, ejecución).
3. **Sistema de permisos** — allowlist por herramienta y por proyecto; confirmación explícita para acciones destructivas; log inmutable.
4. **Memoria persistente** — SQLite con sesiones, hechos, decisiones; búsqueda por texto y (opcional) embeddings.
5. **Terminal integrada** — `portable-pty` + xterm.js, con captura de salida para el agente.
6. **Editor y proyectos** — árbol de archivos, Monaco, diffs antes de aplicar.
7. **MCP** — cliente para servidores locales (filesystem, memory, unreal, etc.).
8. **Investigación autónoma supervisada** — plan → ejecución por pasos → aprobación humana en hitos.
9. **Actualizador seguro** — tauri-plugin-updater con firma minisign.
10. **Instalador EXE** — NSIS (y MSI opcional) generado por Tauri bundler.

---

## 7. COSTES ESTIMADOS

### Gratuitos (todo lo necesario)
- Tauri 2, Rust, Node, Bun, Python, Ollama, LM Studio, SQLite, VS Code: **0 €**
- Modelos locales ya disponibles (Ollama 518,7 GB + LM Studio 196,2 GB): **0 €**
- Instalador NSIS/MSI vía Tauri bundler: **0 €**

### Opcionales
| Concepto | Coste aproximado |
|---|---|
| Certificado de firma de código (evita SmartScreen) | 200–400 €/año |
| APIs cloud (OpenAI/Anthropic/Gemini) | Pago por uso (0–50 €/mes según uso) |
| Repositorio privado / CI | Gratis en GitHub para uso personal |
| Dominio + hosting para updates | 10–30 €/año (o GitHub Releases gratis) |

---

## 8. RIESGOS Y COMPATIBILIDAD

| Riesgo | Nivel | Mitigación |
|---|---|---|
| H: solo 111,6 GB libres | Medio | Proyecto ocupa ~5–15 GB; modelos ya viven en D:/C: |
| Primera build de Tauri descarga crates + NSIS (~1–2 GB) | Bajo | Hacerla una vez y cachear |
| Antivirus/SmartScreen marca el EXE sin firmar | Medio | Firma opcional o distribución con hash publicado |
| LM Studio API apagada por defecto | Bajo | El conector debe detectar y avisar; botón "iniciar servidor" |
| Dos GPUs: no usar ambas para modelos grandes | Conocido | El router de modelos limita a 1 GPU por carga |
| .NET SDK ausente | Bajo | Solo afecta si se quisiera WinUI/WPF; no es el plan |
| PyInstaller/Nuitka ausentes | Bajo | Solo si se empaqueta el sidecar Python; instalar en Fase 2 |
| Clang ausente en MSYS2 | Bajo | MSVC y GCC cubren C/C++ |
| Permisos de administrador para instalar | Bajo | El instalador NSIS pedirá elevación estándar |
| Conflicto con OpenCode/Freebuff | Bajo | KUKIA CODE usará puertos propios (p. ej. 8790) y su propio directorio de datos |

**Compatibilidad verificada:** Windows 11 build 26300 · WebView2 156 · Rust MSVC 1.98 · Node 24 · Python 3.12 · CUDA 13.4/13.0.

---

## 9. PLAN DE DESARROLLO POR FASES

| Fase | Contenido | Entregable | Estimación |
|---|---|---|---|
| **F1** | Auditoría (esta) | `AUDITORIA_INICIAL.md` | ✅ hecha |
| **F2** | Scaffold: repo git, Tauri 2 + React + TS, estructura de carpetas, build "hello world" + instalador NSIS de prueba | EXE instalable vacío | 1 sesión |
| **F3** | Núcleo: config, keyring, logging, auditoría SQLite, servidor local | Core funcional | 2–3 sesiones |
| **F4** | Conectores: Ollama + LM Studio + selector universal + health-check | Chat con modelos locales | 2 sesiones |
| **F5** | UI chat: streaming, markdown, adjuntos, historial | UI usable | 2–3 sesiones |
| **F6** | Agentes + permisos + MCP | Agentes con tools | 3–4 sesiones |
| **F7** | Memoria persistente + proyectos + editor Monaco | Memoria y edición | 2–3 sesiones |
| **F8** | Terminal integrada (pty) | Terminal en la app | 1–2 sesiones |
| **F9** | Investigación autónoma supervisada | Modo research | 2–3 sesiones |
| **F10** | Actualizador firmado + instalador final + documentación | Release 1.0 | 1–2 sesiones |

---

## 10. VERIFICACIONES REALIZADAS (evidencia)

- `nvidia-smi` → 2 GPUs, driver 621.01.
- `nvcc --version` → CUDA 13.4.92.
- `python -c "import torch..."` → torch 2.12.1+cu130, CUDA disponible, 2 GPUs.
- `ollama list` → ~80 modelos; API `/api/version` → 0.40.13.
- LM Studio `:1234/v1/models` → sin respuesta (servidor apagado).
- `npm ls -g` → opencode-ai 1.19.3, n8n 2.29.11, codex 0.163.0, surge 0.44.5.
- `rustup show` → stable-x86_64-pc-windows-msvc.
- `cl.exe` → 14.44.35207 en D:\VisualStudio2022.
- `gcc --version` → 16.2.0 (MSYS2).
- `go version` → go1.24.5 windows/amd64.
- `Get-PSDrive` → tabla de discos incluida arriba.
- `Test-Path H:\KUKIA_CODE` → False (se crea con este informe).

---

## 11. SIGUIENTE PASO

**FASE 2** (a la espera de autorización explícita de Kuki):
1. Crear `H:\KUKIA_CODE` con estructura de carpetas y repo git local.
2. Instalar Tauri CLI (`cargo install tauri-cli` o `npm i -D @tauri-apps/cli`).
3. Scaffold Tauri 2 + React + TypeScript + Vite.
4. Build "hello world" y generar instalador NSIS de prueba.
5. Verificar que el EXE se instala y ejecuta en Windows 11.

**Nada de esto se hará sin la orden de Kuki.**
