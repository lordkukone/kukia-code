# GREENBOOST WINDOWS — AUDITORÍA Y DISEÑO PARA KUKIA CODE

**Fecha:** 01/11/2026 · **Autor:** Xoxete
**Modo:** auditoría de solo lectura. **No se ha descargado, instalado, compilado ni ejecutado nada.**

---

## 1. QUÉ ES (verificado en la fuente)

`denoflore/greenboost-windows` — port a Windows del GreenBoost original de **Ferran Duarri**.
Mantenido por **Chris Zuger / denoflore**. Licencia **GPL v2**; usa **Microsoft Detours** (MIT).

Mecanismo real, sin adornos:

1. `greenboost.exe` **lanza una aplicación CUDA nueva** (proceso x64 recién creado).
2. Le carga `greenboost_cuda.dll`, que **intercepta** (Detours) las llamadas de **asignación y de
   capacidad** de memoria del driver CUDA.
3. Si una asignación grande no cabe en VRAM, pide al driver **managed memory**: el driver decide
   si vive en VRAM, en RAM del sistema o en disco. El cálculo sigue en la GPU.

**No** hay driver de kernel, **no** hace falta administrador, **no** hace falta CUDA Toolkit para
el paquete portable, y **no** toca procesos que ya estén corriendo: solo los que él arranca.

### Versiones

| Versión | Cómo se obtiene | Qué trae |
|---|---|---|
| **2.4.0 beta 3** | ZIP binario publicado | Lanzador portable + scripts de llama.cpp; política de asignación antigua |
| **2.5.0 experimental** (`main`) | **Compilar**: Visual Studio C++ Build Tools + CMake + Detours 4.0.1 (`build.ps1 -DetoursRoot … -StaticRuntime -Package`) | Asignación adaptativa, control de caché KV, inspección de GPU, librería de ficheros (`FileTensor`) |

**No hay binario publicado de la 2.5.** Descargar la beta 3 no instala el código nuevo.

---

## 2. COMPATIBILIDAD CON TU MÁQUINA (medido hoy)

| Requisito del proyecto | Tu equipo | Veredicto |
|---|---|---|
| Windows 10/11 x64 | Windows 11 Pro | ✅ |
| GPU NVIDIA CUDA | RTX 5070 (**CC 12.0**) + RTX 5060 Ti (**CC 12.0**) | ✅ (serie 5000 soportada) |
| Driver compatible | **621.01** en ambas | ⚠️ **más nuevo que el único probado** |
| Backend CUDA recomendado | Toolkit **13.4** instalado (`C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.4`) | ✅ coincide con su recomendación para RTX 5000 |
| Espacio | H: 98 GB · G: 263 GB libres | ✅ |
| Copia local ya descargada | **Ninguna** (`G:\descargas` sin nada de GreenBoost) | — |

### El dato que más importa

Su informe de pruebas está fechado el **31/10/2026 con driver 610.88**. Tú tienes **621.01**.
Como el port funciona **enganchando funciones del driver CUDA**, un driver más nuevo es
exactamente el escenario donde estos hooks fallan o cambian de comportamiento. No es descartable,
pero **no está probado y no se puede prometer**.

### Tarjetas con pruebas registradas

RTX 3060, 3090, 4090, **5060 Ti**, 5090. **La RTX 5070 no aparece nombrada** (sí la serie 5000 por
arquitectura). La RTX 2000 está "incluida pero sin probar".

---

## 3. QUÉ **NO** HACE (expectativas que hay que matar ya)

1. **No da streaming automático de pesos ni de expertos MoE desde el SSD.** Lo dice su propio
   README: la librería de ficheros **no está conectada** al planificador de llama.cpp. Lanzar
   Ollama o LM Studio a través de GreenBoost **no activa** nada de eso.
2. **No acelera.** La memoria gestionada vive donde el driver quiera; si cae en RAM, va por PCIe.
   Los autores **no han medido inferencia por encima de la VRAM**, solo asignaciones y ficheros.
3. **No suma la VRAM de tus dos GPU** en un solo espacio utilizable.
4. **No arregla la competencia de motores** (el problema que ya avisa el VRAM Guard). Es ortogonal.
5. **No soporta AMD ni Intel.** Solo CUDA.
6. **No es un servidor**: no hay puerto que abrir ni endpoint que consultar. Es un lanzador.

Lo que **sí** puede aportar en tu caso: ejecutar en llama.cpp un GGUF que **no cabe** en 12/16 GB
dejando parte en RAM (más lento, pero arranca), y controlar el KV cache (`-CacheType q8_0/q4_0`)
con ahorros medidos por ellos: 1.024 MiB (f16) → 544 MiB (q8_0) → 288 MiB (q4_0) en un 4B a 32k.

---

## 4. CONFLICTO CON TUS PROPIAS REGLAS (te lo pongo delante, no lo escondo)

Tu encargo dice: *«No sustituir DLL, inyectar código ni interceptar procesos existentes»*.

GreenBoost **es inyección por diseño**: carga una DLL propia dentro de un proceso ajeno y engancha
funciones del driver CUDA. Matices honestos:

- **No** toca procesos existentes ni servicios en marcha: solo los que él lanza como hijo.
- **No** sustituye DLLs del sistema: añade la suya en su propio proceso.
- **No** requiere desactivar Secure Boot ni bajar la seguridad de Windows.

Aun así, la regla dice «inyectar código», y esto inyecta. **La decisión es tuya**, y no voy a
interpretarla a mi favor: si quieres respetar la regla al pie de la letra, GreenBoost no se integra
como motor ejecutable y el módulo se queda en **detección e informe, sin lanzar nada**.

---

## 5. RIESGOS, POR ORDEN DE IMPORTANCIA

| # | Riesgo | Gravedad | Comentario |
|---|---|---|---|
| R1 | Driver **621.01 no probado** por el proyecto | **Alta** | Es el punto exacto donde un hook puede fallar |
| R2 | La regla de «no inyectar» queda incumplida | **Alta (decisión tuya)** | Ver §4 |
| R3 | Rendimiento peor que CPU si el modelo cae en RAM | Media | Nadie lo ha medido por encima de VRAM |
| R4 | Caída del proceso lanzado | Media | Aislado: muere el hijo, no el sistema |
| R5 | **GPL v2** | Media | Si algún día va dentro del instalador de KUKIA, obliga a liberar fuente; hoy KUKIA no lo incluye |
| R6 | 2.5 solo desde código, y exige VS Build Tools + CMake + Detours | Media | Compilar es un paso más que descargar |
| R7 | Sin firma de código | Baja | SmartScreen avisará del binario |
| R8 | GPU0 es tu GPU de pantalla y juegos | Baja | Se elige GPU con `-Gpu N`; usar la 5060 Ti |

**Lo que no puede pasar:** no instala drivers, no toca BIOS/Secure Boot/registro, no modifica CUDA,
Python, Ollama, LM Studio, Pinokio, OpenCode ni Freebuff. Todo vive en su carpeta.

---

## 6. DISEÑO DEL CONECTOR (módulo independiente, apagado por defecto)

Nada de esto está implementado; es el plano.

| Pieza | Ubicación propuesta | Regla |
|---|---|---|
| Detección y estado | `core/src/greenboost.rs` | Solo **lee** rutas y versiones; nunca lanza |
| Comandos | `greenboost_status`, `greenboost_probe` (dry-run: dice qué ejecutaría, sin ejecutar) | `probe` no arranca nada |
| Configuración | `greenboost_enabled: false` por defecto; `greenboost_path`, `greenboost_gpu`, `greenboost_extra_mib` | Activar exige acción explícita |
| Datos | `%APPDATA%\KukiaCode\greenboost\` (perfil, logs y resultados propios) | Aislado del resto |
| Binarios | `G:\greenboost_lab\` (fuera del proyecto y del instalador) | Borrar la carpeta = desinstalado |
| Vista | «GreenBoost (experimental)» con estado, GPU elegida, modelos candidatos y resultados | Sin botones falsos |
| Comparación | Reutiliza el Benchmark Center: mismo GGUF, mismo prompt, `llama.cpp` directo vs `greenboost.exe` | Medir tok/s y VRAM ocupada en ambos |

**No hay puerto**: es un lanzador, no un servidor. Su «aislamiento» es proceso y carpeta, no red.

---

## 7. VALIDACIÓN PROPUESTA (reversible, sin tocar el sistema)

Todo esto **necesita tu autorización** porque implica descargar/compilar y ejecutar un tercero.

### Paso 0 — Control (sin GreenBoost)

1. Con `llama.cpp` que ya tengas: un GGUF que **sí** quepa (p. ej. 8B Q4) → tok/s y VRAM.
2. Un GGUF que **no** quepa (un 30B Q4 en la 5070, o los Flint de 40B) → confirmar el error de OOM.

### Paso 1 — GreenBoost portable (si autorizas)

1. Descargar **solo** el ZIP 2.4.0 beta 3 a `G:\greenboost_lab` (sin admin, sin instalador).
2. Verificar hash del ZIP y anotarlo.
3. `.\install_llama.ps1 -Cuda 13.4` (baja llama.cpp b12019 a `backends\` local).
4. `.\run_llama.ps1 -ListDevices` → comprobar que ve **5060 Ti (GPU 1)**.
5. Repetir los dos casos del Paso 0 con `-Gpu 1`, `-ExtraMiB 0` y `-ExtraMiB 8192`.
6. Medir: tok/s, VRAM de la 5060 Ti, RAM del sistema, estabilidad 10 min, y si el driver 621.01
   aguanta sin cuelgues.

### Criterio de éxito (antes de escribir una línea de conector)

- Carga un modelo que **sin** GreenBoost da OOM, y **no** cuelga el proceso ni el driver.
- Tok/s medidos y comparables, con la VRAM registrada.

Si no cumple las dos, el módulo se queda en «detectado, no recomendado» y no se integra.

### Reversión

1. Cerrar los procesos lanzados.
2. Borrar `G:\greenboost_lab`.
3. Borrar `%APPDATA%\KukiaCode\greenboost` si existe.
4. Verificar que `nvidia-smi` sigue viendo las dos GPU y que Ollama/LM Studio/ComfyUI responden.

---

## 8. LO QUE NECESITO DE TI

1. **¿Autorizas el Paso 1** (descargar el ZIP a `G:\greenboost_lab` y ejecutarlo con llama.cpp,
   sin administrador y sin tocar nada del sistema)?
2. **¿Compilamos la 2.5** o nos quedamos en la beta 3? La 2.5 pide VS Build Tools + CMake +
   Detours 4.0.1.
3. **¿Cómo interpretamos la regla de inyección?** Tres opciones:
   - a) Solo detección e informe, sin ejecutar nada (cumple la regla al pie de la letra).
   - b) Ejecutar solo en el laboratorio, nunca desde KUKIA.
   - c) Integrarlo como motor experimental opcional en KUKIA, apagado por defecto.
4. **¿GPU de pruebas?** Recomiendo la **5060 Ti (GPU 1)** para no cargar la de pantalla.

**Nada descargado. Nada instalado. Nada compilado. Nada ejecutado.**
