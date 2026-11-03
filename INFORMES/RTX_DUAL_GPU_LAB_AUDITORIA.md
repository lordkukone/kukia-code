# RTX DUAL GPU LAB — Auditoría, clasificación y plan

**Fecha:** 02/11/2026
**Alcance:** auditoría de **solo lectura** sobre `G:\descargas`, `nvidia-smi` y los paquetes ZIP.
**No se ha instalado, extraído, ejecutado ni modificado nada.** Ni drivers, ni CUDA, ni juegos, ni Pinokio/ComfyUI/Ollama/LM Studio.

---

## 0. Resumen ejecutivo

1. **De tus 28 proyectos solo 10 están descargados.** Los otros 18 no existen en el equipo: no hay carpeta, ZIP, ni `.git`. No los doy por instalados ni los invento.
2. **Hay dos descargas falsas**: `dlss5-bridge-v1.4.13-pre6.zip` son **9 bytes que dicen "Not Found"** y `OpticalFlowSDK_5.0.zip` es una **página HTML de login**, no un ZIP. `OpticalFlowSDK.exe` son **0 bytes**.
3. **Hallazgo que cambia el proyecto:** tienes descargado **`Neural-coprocessor` (MGPU Bridge)**, que hace exactamente lo que preguntas en el punto 6: **la GPU 1 renderiza el juego y la GPU 2 ejecuta el DLSS Neural Rendering**. No es SLI, no parte el frame: el NR es etapa terminal y por eso se puede externalizar.
4. **Y trae la herramienta que necesitábamos**: `nrcheck.exe` (dentro de `mgpu_bridge_launcher-0.4.0.zip`) prueba si DLSS-NR arranca **en cada tarjeta, sin abrir ningún juego**, y tiene un bench (`--bench --neural <idx> --res 2560x1440`). Es la primera prueba que hay que hacer, y es reversible.
5. **Integridad verificada** en lo importante: `DLSS5-NR-v0.8.0.fxpkg` da SHA-256 `e84bf87f…47957`, **idéntico** al `DLSS5-NR-SHA256SUMS.txt` oficial. `renodx-dlss5.addon64` también coincide con su sidecar.
6. **No hay Frame Generation repartida entre GPUs y no existe forma pública de hacerlo.** Lo que sí se reparte es el **Neural Rendering** (arriba) y, en vídeo offline, la interpolación RIFE. Confundir esto es lo que venden los vídeos de YouTube; el informe lo separa en el punto 5.

---

## 1. Hardware verificado (no de memoria: `nvidia-smi`)

| Dato | Valor real leído |
|---|---|
| GPU 0 | NVIDIA GeForce RTX 5070 — 12.227 MiB |
| GPU 1 | NVIDIA GeForce RTX 5060 Ti — 16.311 MiB |
| Driver | **621.01** (coincide con lo que dijiste) |
| PCIe máximo | **Gen3, ancho máximo x16** en ambas |
| CPU / RAM | i9-10900K / 64 GB (ya comprobado en auditorías previas de KUKIA) |

> Discrepancia a comprobar en el momento de probar: dijiste **x8** por tarjeta; `pcie.link.width.max` dice **x16**. El ancho **efectivo** se lee con `nvidia-smi --query-gpu=pcie.link.width.current --format=csv`. Es un dato importante para el punto 6.

---

## 2. Inventario real de los 28 proyectos

Leyenda: **A** listo para probar · **B** requiere compilación · **C** investigación · **D** exclusivo de un juego · **E** incompatible · **F** no verificado.

| # | Proyecto de tu lista | ¿Descargado? | Qué hay realmente | Clase |
|---|---|---|---|---|
| 1 | laurensanp/**siris** | **NO** | nada | — |
| 2 | Kizzuwatnaa/**DLSS5-Autopilot** | SÍ (probable) | `DLSS5-Autopilot-v2.0.9-win64.zip` (exe compilado, 13 MB) + 3 zips de fuente | A / B |
| 3 | Abhito/**Glasir** | **NO** | nada | — |
| 4 | **GTA-5-LEGACY** Intel XeSS/DLSS/FG | **NO** | nada | — |
| 5 | teemupynnonen/**mgsv-phantom-fx** | **NO** | nada | — |
| 6 | Xchen991/**RTX30-Broadcast-FrameGen** | **NO** | nada | — |
| 7 | SmarterGamers/**DLSS5-Swapper** | SÍ (probable) | `DLSS5-Swapper-Setup-2.2.9.exe` + fuentes | A |
| 8 | **OptiScaler 0.9.4** | SÍ | `OptiScaler-master.zip` (fuente) + `OptiScaler-DLSSNR-v0.1.2.zip` (release con DLLs, 130 MB) | A |
| 9 | beeradmoore/**dlss-swapper 1.2.7.0** | SÍ | portable 120 MB + 3 instaladores | A |
| 10 | Kaldaien/**SpecialK** debug_sessions | **NO** | solo referencias en la doc de MGPU | — |
| 11 | PanosK92/**SpartanEngine** | **NO** | nada | — |
| 12 | gkjohnson/**three-gpu-pathtracer** | **NO** | nada | — |
| 13 | Robodog81/**Path-Tracer** | **NO** | nada | — |
| 14 | bwalid13/**XRay-Render** | **NO** | nada | — |
| 15 | atul-mourya/**rayzee-renderer** | **NO** | nada | — |
| 16 | epochlab/**PATHTRACER** | **NO** | nada | — |
| 17 | edoardo911/**Real-Time-Path-Tracer** | **NO** | nada | — |
| 18 | mdhamm/**DXRDemo** | **NO** | nada | — |
| 19 | nvpro/**vk_gltf_renderer** | **NO** | nada | — |
| 20 | Aerosane/**vk-rt-cuda-raytracing** | **NO** | nada | — |
| 21 | **Ablation neural upscaling** | **NO** | nada | — |
| 22 | Limestudio90/**ReShadeUpscaler** | **NO** | nada | — |
| 23 | **video-upscaler-with-dlss5** | **NO** | nada | — |
| 24 | jackthepunished/**ternoise** | **NO** | nada | — |
| 25 | NVIDIA/**Model-Optimizer 0.47.0** | SÍ | `Model-Optimizer-main.zip` (fuente, 20 MB). **Versión interna no verificada** | B |
| 26 | NVIDIAGameWorks/**Streamline 2.14.16** | **PARCIAL / VERSIÓN DISTINTA** | solo `DLSS 310.8.0 streamline **2.13** dll.zip`. **No es 2.14.16** | A (DLLs) |
| 27 | NVIDIAGameWorks/**rtx-remix 1.6.1** | **NO** | nada | — |
| 28 | harpjo60-lgtm/**rdr2-mac-framegen** | **NO** | nada | — |

### 2.1 Extras encontrados que NO estaban en tu lista (y son más valiosos que la mitad de ella)

| Paquete | Tamaño | Qué es | Clase |
|---|---|---|---|
| `Neural-coprocessor-main ultimo.zip` (+4 versiones) | 13,7 MB | **MGPU Bridge**: DLSS-NR cross-adapter (juego en GPU 0, NR en GPU 1) | **A** |
| `mgpu_bridge_launcher-0.4.0.zip` | 0,95 MB | Launcher + **`nrcheck.exe`** + `nvngx.dll_nrcheck.dll` + `Run-NRBench.ps1` | **A** |
| `mgpu_bridge-0.4.0.zip` / `0.3.1` / `0.2.4` / `1.10` | 0,2–31 MB | Add-on `.addon64` + `mgpu.ini` + `ReShade2.ini` (el 1.10 trae también redist FSR) | A |
| `DLSS5-NR-v0.8.0.fxpkg` | 229,9 MB | Modelo/paquete DLSS-NR con SUM oficial | A (hash OK) |
| `DLSS 310.8.0 streamline 2.13 dll.zip` | 186,7 MB | `nvngx_dlss.dll`, `nvngx_dlssg.dll`, **`nvngx_dlssnr.dll` (166 MB)**, `sl.*` | A |
| `FoveatedDLSS5/renodx-dlss5.addon64` | 2,2 MB | DLSS5 NR con foveación — **una sola GPU** | A (hash OK) |
| `CheekyFoveatedDLSS-0.6.0` (Standalone + UEVR) | 6,6 MB | Alternativa sin ReShade, DLSS SR y NR | A |
| `DLSS5-Feeder-1.16.0-beta.5.zip` | 0,4 MB | Feeder con `.addon64` + host64 compilados | A |
| `RTXNTC_windows_x64_v0.10.0b.zip` | 186,9 MB | Neural Texture Compression: `ntc-cli`, `ntc-explorer`, `ntc-renderer` compilados | **A** |
| `rife-ncnn-vulkan-20221029` | 431,5 MB | Interpolación de frames **en vídeo**, no en tiempo real | A |
| `renodx-main.zip`, `DLSSTweaks-master.zip`, `dlssnr-signature-repair-main.zip` | 21 MB | RenoDX, tweaks de presets, reparación de firma DLSS-NR | B |
| `PresentMon-2.6.0-x64.exe` (dentro de Neural-coprocessor) | — | Medición de latencia/frame pacing | A |
| GPU-Z 2.71, HWiNFO 8.52, GPU Caps Viewer 1.64.3, AIDA64 8.40, CPU-Z 3.01 | — | Monitorización y diagnóstico | A |
| DDU v18.1.5.6 → 18.1.6.2 (**6 copias**) | — | Limpieza de drivers. **NO se va a usar**: cambiar driver está prohibido en tu encargo | — (guardar) |
| 9 zips de juegos (Unicorn Overlord, They Are Billions, RimWorld, SPORE, WorldBox, Big Walk, Another Eden, Creator Chronicles, Pandora's Toybox) | ~37 GB | **Banco de pruebas local** para inyectar ReShade/FG sin tocar nada online | A (como conejillo) |

### 2.2 Integridad — lo comprobado y lo roto

| Archivo | Comprobación | Resultado |
|---|---|---|
| `DLSS5-NR-v0.8.0.fxpkg` | SHA-256 contra `DLSS5-NR-SHA256SUMS.txt` | **OK** — `e84bf87ff991fdb7d0eeb94cebcdd329bd0f62e4592ec667d200ecc64e347957` |
| `FoveatedDLSS5/renodx-dlss5.addon64` | SHA-256 contra su sidecar | **OK** — `bd6efa7848b4fd…dd8a4a9` |
| `dlss5-bridge-v1.4.13-pre6.zip` | contenido real | **ROTO** — 9 bytes, texto `Not Found` |
| `OpticalFlowSDK_5.0.zip` | contenido real | **ROTO** — 12.659 bytes de **HTML de login de GitHub** |
| `OpticalFlowSDK.exe` | tamaño | **ROTO** — 0 bytes |
| `mgpu_bridge_launcher-0.4.0.zip` | SHA-256 calculado (sin SUM oficial a mano) | `e1cbc8da14a2a874c44bcb54cb35ea91…` (anotado para comparar con el release) |

Espacio libre: **G: 267,8 GB** · C: 127,9 GB · H: 101,4 GB · I: 27,8 GB · J: 31,5 GB.
Los 28 proyectos, si se instalaran todos, no llegan a **1,5 GB** en total. El problema nunca fue el disco.

---

## 3. El hallazgo: MGPU Bridge (Neural-coprocessor)

### 3.1 Qué hace, en palabras del propio proyecto

> *"A second GPU runs a game's DLSS Neural Rendering while the first one renders the game."*
> *"Not SLI: nothing is split mid-frame. Neural rendering is a terminal stage. It takes a finished frame and returns a finished frame, so it can be picked up and executed somewhere else."*

Es un **add-on de ReShade** (`.addon64`) que se inyecta en juegos **DX11 y DX12**. Windows renderiza en la tarjeta que tiene la pantalla; el add-on elige **la otra** para el trabajo neuronal.

### 3.2 Requisitos reales (del README, no inventados)

- **ReShade 6.8.0 o superior con soporte de add-ons.** Sin eso, el `.addon64` **no se carga y no da error**. Es la causa nº1 de "no pasa nada".
- **Segunda GPU NVIDIA** con driver actual. **DLSS-NR es característica de la serie 50**: *"It is the SECOND card that runs the neural stage, so that is the one the requirement applies to."*
- `nvngx_dlssnr.dll` en una subcarpeta `mgpu\` **junto al add-on, nunca junto al .exe del juego** (algunos juegos cargan cualquier `nvngx_*.dll` que encuentren y se lo quedan antes de tiempo).
- `mgpu_depth_tap.fx` en la carpeta de shaders de ReShade (autocontenido, no necesita paquetes).
- **Un solo monitor ya está soportado** desde 0.3.0 con `DcompOverlay=1` (por defecto). Ratón, teclado y mando funcionan. Hay carpeta `workarounds/single-display` pero está **obsoleta**.

### 3.3 Encaje con TUS dos tarjetas

Este es el punto importante y hay que decirlo claro:

- La RTX 5070 conduce la pantalla → renderiza el juego.
- La **5060 Ti** (16 GB, serie 50) queda libre → **es la que hace DLSS-NR**. Cumple el requisito por los dos lados.
- El README avisa: *"If your two cards are not evenly matched, experiment with inverting the setup. Putting the stronger card on the DLSS 5 workload can give surprising results."* → **hay que medir las dos configuraciones**, no suponer.
- El proyecto **no comprueba el hardware**: si algo no puede correr, carga, escribe log normal y **no produce nada**. Por eso el primer paso es `nrcheck`, no un juego.

### 3.4 Límite PCIe 3.0 x8

Lo único medido por terceros que consta en el repo: un colaborador con **RTX 4080 SUPER + RTX 5060 Ti** reportó que **el ancho de banda PCIe no era la causa** de un problema de suavidad que veía a carga alta (issue del repo). Eso **descarta el mito**, pero **no** es una medición nuestra ni de tu placa. Nuestra cifra la dará `nrcheck --bench`, que imprime cuántos fps caben en 60/120/144 a la resolución pedida.

---

## 4. Respuesta a la pregunta de doble GPU (los 6 escenarios de tu punto 6)

| Escenario | ¿Existe? | Realidad |
|---|---|---|
| 1. **Multi-GPU nativo** | **No** en DX12 moderno | SLI/CrossFire está muerto; ningún juego de 2026 lo usa |
| 2. **GPU secundaria para Optical Flow** | **No** por API pública | NVIDIA Optical Flow (NvOF) trabaja en el device que renderiza; no hay API para enviarlo a otra GPU. `OpticalFlowSDK_5.0.zip` **está roto**, así que ni se puede auditar su contenido |
| 3. **GPU secundaria para inferencia neuronal** | **SÍ** | Es exactamente **MGPU Bridge** + DLSS-NR. Y también `FoveatedDLSS5` (pero ese es **una sola GPU**, foveando en los bordes) |
| 4. **Frame Generation distribuida** | **No** | DLSS-FG (`nvngx_dlssg.dll`) corre dentro del juego, en la GPU del juego. No hay versión "cross-adapter" de FG |
| 5. **Copia de frames entre GPU** | **SÍ, pero es lo que MGPU NO hace** | MGPU transporta **etapa neuronal**, no frames finales. Lo que sí cruza el bus es la imagen de entrada y el resultado; por eso el coste crece con la resolución y el proyecto añade `NRScale`, segunda pasada más barata y `SRUpscale` para reducirlo |
| 6. **Tecnologías que no permiten repartir** | — | SLI, NvOF, DLSS-FG, DSR, Reflex (mide, no reparte) y cualquier cosa que dependa del swapchain del juego |

**Conclusión honesta:** lo que tú quieres (juego en la 5070, generación en la 5060 Ti) **es real para Neural Rendering y está descargado**. Para **Frame Generation** no existe, y ningún paquete de tu carpeta lo convierte en real. Alternativa legítima y offline: **RIFE** interpola frames de vídeo ya grabado en la 5060 Ti mientras la 5070 hace otra cosa — eso sí reparte trabajo, pero no es jugar.

---

## 5. Diseño del laboratorio propuesto

Raíz: **`G:\RTX_LAB`** (267 GB libres; H: solo tiene 101 GB y ahí vive KUKIA_CODE).

```
G:\RTX_LAB\
├── 00_MANIFIESTO.md              ← qué hay, de dónde salió, hash, cómo se desinstala
├── 01_TELEMETRIA\                ← GPU-Z, HWiNFO, PresentMon (solo lectura)
├── 02_NRCHECK\                   ← la prueba sin juego (primera)
│   ├── nrcheck.exe, nvngx.dll_nrcheck.dll, Verify-NRCheck.ps1
│   └── mgpu\ nvngx_dlssnr.dll    ← del pack 310.8.0, ya verificado
├── 03_MGPU_BRIDGE\               ← add-on 0.4.0 + mgpu.ini + launcher
├── 04_FOVEATED\                  ← renodx-dlss5.addon64 + CheekyFoveated 0.6.0
├── 05_OPTISCALER\                ← release DLSSNR v0.1.2 (aislado)
├── 06_SWAPPER\                   ← DLSS Swapper portable (NO instalador)
├── 07_RTXNTC\                    ← demo NTC compilada
├── 08_RIFE\                      ← interpolación de vídeo
├── 09_BANCO_JUEGOS\              ← una copia de UN juego para inyectar (nunca online)
├── 90_LOGS\                      ← ReShade.log, nrcheck_report.txt, benchmarks
└── 99_BACKUPS\                   ← .ini y configs originales antes de tocar
```

Reglas del laboratorio:

- Cada carpeta es **autónoma** (su config, su log, su desinstalación = borrar la carpeta).
- **Nunca** se escribe en la carpeta de un juego instalado sin permiso explícito; se trabaja sobre **copia** en `09_BANCO_JUEGOS`.
- Nada de DLLs originales sustituidas en juegos con anticheat u online: **jamás**.
- Cada prueba deja un `.md` en `90_LOGS` con: comando, hora, VRAM antes/después, fps, latencia, artefactos y veredicto.

---

## 6. Plan de pruebas ordenado (de riesgo cero hacia arriba)

### Fase A — Telemetría (0 riesgo)
`GPU-Z`, `HWiNFO64`, `PresentMon`. Solo leen sensores.

### Fase B — ¿Puede tu 5060 Ti arrancar DLSS-NR? (**la prueba clave**)
Sin juego, sin ReShade, sin tocar nada instalado:

```powershell
# 1. Extraer SOLO la herramienta a una carpeta nueva
mkdir G:\RTX_LAB\02_NRCHECK
cd G:\RTX_LAB\02_NRCHECK
# desde mgpu_bridge_launcher-0.4.0.zip: payload\tools\nrcheck.exe + nvngx.dll_nrcheck.dll + Run-NRBench.ps1
# y en .\mgpu\ poner nvngx_dlssnr.dll del pack 310.8.0 (ya verificado por hash)

# 2. Comprobar en ambas tarjetas (≈1 minuto)
.\nrcheck.exe

# 3. Medir cuánto aguanta la 5060 Ti a 1440p
.\nrcheck.exe --bench --neural 1 --other 0 --res 2560x1440 --passes 1 --frames 300
```

Salida esperada si todo encaja: para cada adaptador, si DLSS-NR arranca, y `nrbench_result.ini` con el tiempo por pase y si cabe en 60/120/144 fps.
**Si sale que no arranca en la 5060 Ti, el proyecto entero se cae y nos habremos ahorrado días.** Por eso va primero.

### Fase C — MGPU Bridge en un juego de copia (riesgo bajo, reversible)
ReShade 6.8.0 **Add-on build** → `mgpu\` + `.addon64` + `mgpu_depth_tap.fx` → `mgpu.ini` de 0.4.0 → leer `[MGPU][T2] SELECTED adapter[N]` en `ReShade.log`.
Comprobar en este orden: `Registered add-on "MGPU Bridge"` → `SELECTED adapter` apunta a la 5060 Ti → `P4.0 stream REQUESTED` → ventana sin arcoíris.

### Fase D — Medición real
PresentMon + GPU-Z, 3 pasadas por configuración: **NR off**, **NR en la 5060 Ti**, **NR en la 5070 (invertido)**. Registrar fps render, fps presentados, frametime, GPU%, VRAM por tarjeta, temperatura, vatios, latencia y artefactos. Sin esas tres columnas no hay comparación posible.

### Fase E — Complementos
Foveated (una GPU), OptiScaler aislado, DLSS Swapper (solo para comparar versiones de DLL), RTXNTC demo, RIFE para vídeo.

---

## 7. Lo que NO voy a hacer sin tu palabra

- **No** toco drivers (y hay 6 DDU descargados: se quedan guardados).
- **No** cambio BIOS, voltajes, frecuencias, Power Limit, ni perfiles NVInspector.
- **No** modifico CUDA, Python, Pinokio, Ollama, LM Studio, ComfyUI, Wan2GP ni Maestro.
- **No** sustituyo DLLs en juegos instalados ni toco nada online/con anticheat.
- **No** desactivo Defender ni ningún control de seguridad.
- **No** borro ni muevo originales de `G:\descargas`.
- **No** ejecuto `nrcheck.exe` ni ningún binario descargado hasta que me digas que sí (tu propio punto 8).

---

## 8. Aprobación que necesito (di sí o no a cada línea)

1. Crear `G:\RTX_LAB` y extraer ahí **solo** `nrcheck.exe` + `nvngx.dll_nrcheck.dll` + `nvngx_dlssnr.dll` (ficheros verificados) para la **prueba B**.
2. **Ejecutar** `nrcheck.exe` y el bench a 1440p en la 5060 Ti. Es lectura de capacidades, sin juego y sin instalar nada.
3. Elegir **un** juego del banco de copias para la prueba C (recomiendo *They Are Billions* o *WorldBox*: DX11, ligeros, sin anticheat, fáciles de revertir). Nunca uno online.
4. Autorizar ReShade 6.8.0 Add-on **solamente dentro de la copia** de ese juego.

Con el 1 y el 2 ya sabremos si tu sueño es posible en esta máquina. Con el 3 y el 4 lo vemos funcionando.

---

## 9. Riesgos declarados

| Riesgo | Nivel | Mitigación |
|---|---|---|
| DLSS-NR no arranque en la 5060 Ti | Medio | Fase B lo detecta en 1 minuto, sin instalar nada |
| Inyección ReShade inestable (crashes, DWM) | Medio | `DcompSafeMode=3`, trabajar sobre copia, log en `90_LOGS` |
| Artefactos/ghosting del NR inyectado | Medio-alto | El propio proyecto avisa: es ruta inyectada, no la de NVIDIA; se mide y se compara |
| Anticheat detecte el add-on | **Alto en online** | Prohibido: solo copias offline |
| Expectativa de "Frame Generation en la 2ª GPU" | — | **No existe.** El informe lo deja escrito para no vender humo |
| Paquetes de origen dudoso (repos de terceros) | Medio | Hashes verificados cuando hay SUM; ejecución solo con tu permiso, en carpeta aislada |
