# AUDITORÍA DEL SUPERPROMPT MAESTRO — KUKIA CODE

**Fecha:** 01/11/2026 · **Autor:** Xoxete (para Kuki / LordKukone)
**Raíz auditada:** `H:\KUKIA_CODE` (la original: no se ha creado ni copiado nada)
**Laboratorio:** `H:\KUKIA_RESEARCH`
**Modo:** **AUDITORÍA DE SOLO LECTURA.** No se ha modificado ni una línea de código, ni
configuraciones, ni datos. Lo único escrito es este informe.

---

## 0. VEREDICTO EN UNA LÍNEA

El proyecto es **real y está bastante más avanzado de lo que dice el informe anterior**: las
Fases 3.0, 3.1 y 3.2 están hechas y probadas. Pero hay **dos agujeros que valen más que cualquier
funcionalidad nueva**: la app instalada es de Fase 2.1 (**no lleva nada de Fase 3**) y **todo el
trabajo está sin commitear** (13 archivos modificados + 23 nuevos sobre el último commit).

Y el bloque que el superprompt detalla con más mimo —**KV Cache y parámetros de inferencia**— está
**a cero**: el chat solo envía `temperature`. Ni `num_ctx`, ni `num_gpu`, ni `keep_alive`, ni
`OLLAMA_KV_CACHE_TYPE`.

---

## 1. MEDIDO HOY (comandos ejecutados, no supuestos)

| Comprobación | Resultado |
|---|---|
| Código propio | **10.410 líneas** (core 5.584 · src-tauri 1.288 · app 3.538) + **738** de tests |
| Módulos del núcleo | 14 (`memory` 1.298 · `anime_real` 729 · `providers` 642 · `gaming` 548 · `permissions` 509 · `research` 486 · `vram` 439 · `security` 318 · `agents` 181 · `gpu` 146 · `config` 123 · `radar` 101 · `logs` 41 · `lib` 23) |
| Comandos Tauri registrados | **55** |
| Vistas de UI | **18** + entrada en menú para cada una |
| Suites de test | 4 (`fase21`, `fase30`, `gaming`, `anime_real`) |
| Tests | **56 unitarios en verde** + 22 de integración (8 fase21, 9 fase30, 3 anime_real, 2 gaming) |
| Permisos aplicados | **7 puntos**: lectura (290), escritura (339), comando (455 y 553), directorio de trabajo (469 y 567), exportación (816) |
| Timeout / cancelación | `timeout_secs` en 4 sitios · `cancel` en 16 |
| Auditoría | helper `auditar()` + **21 llamadas** reales |
| Parámetros de Ollama en el código | `num_ctx` **0** · `num_gpu` **0** · `num_batch` **0** · `num_parallel` **0** · `keep_alive` **0** · `kv_cache` **0** · `flash_attention` **0** · `num_predict` **0** · `min_p` **0** |
| Lo único que se envía al motor | `options: { temperature }` |
| PolyStrata | **instalado** (`D:\PolyStrata`, `engine-poly\strata-poly.exe`, `D:\PolyStrata-data`) · puerto **8090 cerrado** |
| Motores vivos ahora | Ollama 11434 ✅ · ComfyUI 8188 ✅ · LM Studio **1235** ✅ (1234 cerrado) |
| Laboratorio | `AI_EXPERIMENTAL` + `GAMING_GRAPHICS` |
| Disco H: | **102 GB libres** de 2,8 TB (97 % ocupado) · proyecto 8,2 GB |
| Git | **2 commits** · 13 modificados · 23 nuevos · último `18d43fb` (Fase 2.1) |
| App instalada | `kukia-code.exe` 9.063.936 B del **11:02** · código actual hasta las **12:29** → **instalada vieja** |
| MCP / pty / Monaco | **ninguna** dependencia en los tres `Cargo.toml`/`package.json` |

---

## 2. MATRIZ DEL SUPERPROMPT (seis clases)

### A — IMPLEMENTADO Y PROBADO HOY (no hay que tocarlo)

| # | Bloque del superprompt | Prueba |
|---|---|---|
| A1 | Proyecto existente sin duplicar (§1) | `H:\KUKIA_CODE`, 2 commits en `master` |
| A2 | Chat Ollama con streaming y tokens/s (§3, §4) | `providers.rs` + 81 modelos reales |
| A3 | Chat LM Studio / OpenAI-compatible (§5) | puerto **1235** respondiendo ahora |
| A4 | Memoria SQLite: sesiones, mensajes, memorias (§13) | 56 tests + base real con `integrity_check ok` |
| A5 | Monitor GPU: VRAM, uso, temperatura, relojes (§7) | `nvidia-smi` detecta 5070 y 5060 Ti |
| A6 | **Permisos por herramienta aplicados de verdad** (§17) | 9 tests de `validacion_fase30` |
| A7 | **Allowlist de rutas + 14 patrones destructivos** (§17) | `el_cerebro_queda_como_solo_lectura`, `escritura_fuera_de_las_rutas_se_deniega` |
| A8 | **Timeout y cancelación de comandos** (§17) | `run_command_stream` + `command_cancel` + 4 usos de `timeout_secs` |
| A9 | **Auditoría conectada y consultable** (§17) | helper `auditar()` con 21 llamadas + vista Seguridad con filtros y export |
| A10 | Radar de releases (§9) | `radar_check` con caché |
| A11 | Research Radar arXiv/HF/GitHub + informe + descarga aislada (§3) | 35 fichas / informe de 17.824 B |
| A12 | **VRAM Guard con 3 motores y aviso de competencia** (§7) | probado con 2 motores: `competencia: true` |
| A13 | **Gaming Radar con comandos, vista e informe** (§15) | 19 fichas, 3 alertas, informe 20.725 B |
| A14 | **Anime ↔ Real Studio, imagen** (§11) | 3 plantillas, 16,7 s por la ruta Rust |
| A15 | Logs de aplicación | `logs_tail` + vista |

### B — IMPLEMENTADO SIN VERIFICAR

| # | Qué | Qué falta |
|---|---|---|
| B1 | Proveedor cloud (`KUKIA_CLOUD_KEY`) | Nunca ejecutado contra un endpoint real |
| B2 | Persistencia de `config.json` en `%APPDATA%\KukiaCode` | Según la auditoría anterior el archivo no existía; **no re-verificado hoy** |

### C — PARCIAL (existe una mitad; la otra es el trabajo real pendiente)

| # | Bloque | Existe | Falta |
|---|---|---|---|
| C1 | **KV Cache** (§4) | nada | Tipo (`f16/q8_0/q4_0`), memoria estimada vs real, recomendación por VRAM |
| C2 | **Parámetros de inferencia** (§4, §5) | `temperature`, `max_tokens` | `num_ctx`, `num_gpu`, `num_batch`, `num_parallel`, `keep_alive`, `top_k/p`, `min_p`, `repeat_penalty`, `seed`, `num_predict`, JSON mode, tools |
| C3 | **AI Engine Center** (§3) | chat y listado por proveedor | Panel unificado con cargar/descargar, VRAM por motor, latencia, logs por motor |
| C4 | **Panel Ollama** (§4) | `list_models`, `ps` en VRAM Guard | Descargar, borrar con confirmación, importar GGUF, Modelfile, manifiestos, blobs, huérfanos, integridad |
| C5 | **Panel LM Studio** (§5) | detección en 1234/1235 y modelos cargados | Offload, capas GPU, flash attention, KV K/V, batch, threads, mmap/mlock, speculative |
| C6 | **GPU Router** (§7) | perfil 35/65 guardado | Reparto real de capas, recomendación por motor, comparación, cuello PCIe |
| C7 | **Auto Optimizer** (§8) | nada | 10 perfiles, simulación, comparación con lo actual |
| C8 | **PolyStrata** (§9) | instalación presente en `D:\PolyStrata` | Arranque, API, modelo, comparación con Ollama/LM Studio |
| C9 | **ComfyUI multimodal** (§10) | solo img2img SDXL anime/real | Vídeo (Wan/LTX/Ditto), audio, FLUX, ControlNet, PuLID/InstantID |
| C10 | **Anime ↔ Real vídeo** (§11) | dos grafos localizados (Ditto, LTX 2.3) | Conversión a formato API y prueba real |
| C11 | **Agentes** (§12) | 8 perfiles con prompt y lista de `tools` | **Las herramientas no se ejecutan**: es texto |
| C12 | **Memoria / RAG** (§13) | `LIKE` sobre SQLite | Embeddings, indexación, búsqueda semántica |
| C13 | **Benchmarks** (§19) | `benchmark_run` mide tok/s y persiste | Comparador entre motores con condiciones documentadas |
| C14 | **Gaming Radar de juegos** (§15) | radar de tecnologías | Detectar juegos y procesos, perfiles DLSS, conflictos |
| C15 | **Herramientas de desarrollo** (§14) | — | Nuitka, NASM, Ghidra, Volatility, Sysinternals en la app |
| C16 | **Interfaz** (§18) | 18 vistas reales | Faltan 8 secciones del listado; algunas vistas son informativas |

### D — NO IMPLEMENTADO

| # | Bloque |
|---|---|
| D1 | Terminal pty real (xterm.js + portable-pty) |
| D2 | Cliente MCP (§14) |
| D3 | Editor con resaltado (Monaco) |
| D4 | Agent Swarm con ejecución y presupuesto de tokens (§12) |
| D5 | Universal Desktop Control (UI Automation) |
| D6 | Multi-PC / workers / cola distribuida (§16) |
| D7 | Descubrimiento de red local |
| D8 | Actualizador firmado y firma de código |
| D9 | `PROGRESO_GENERAL.md`, `CONTINUAR_TRABAJO.md`, `MATRIZ_FUNCIONALIDADES.md`, `CHANGELOG.md`, `PLAN_RELEASE.md` (§22) — **los cinco faltan** |
| D10 | ComfyUI general (Wan, Wan2GP, LTX, FLUX, LoRAs, audio) |
| D11 | Perfiles de optimización y simulación de memoria |

### E — BLOQUEADO O REQUIERE TU AUTORIZACIÓN

| # | Qué | Por qué |
|---|---|---|
| E1 | **Recompilar e instalar la app** (`npm run tauri:build` + instalar) | Cambia el programa instalado del sistema → aprobación expresa. Es lo único que hace visible todo Fase 3 |
| E2 | **Commit del trabajo actual** (36 archivos) | No es destructivo, pero reescribe historia del repo → tu palabra |
| E3 | **Cambiar variables globales** (`OLLAMA_KV_CACHE_TYPE`, `OLLAMA_NUM_PARALLEL`, `OLLAMA_MAX_LOADED_MODELS`) | Requiere backup y reinicio de Ollama; afecta a todas las apps |
| E4 | **Arrancar y usar PolyStrata** | Está instalado pero apagado; es software ajeno al proyecto |
| E5 | **Descargar Flint 40B / modelos grandes** | Solo quedan **102 GB** en H: y hay 81 modelos en Ollama |
| E6 | **Escribir en `I:\CEREBRO\INCREMENTOS.md`** | Sagrado: solo append con backup verificado y tu permiso |
| E7 | Sumar VRAM entre dos PCs (§16) | **Bloqueado técnicamente**: llama.cpp RPC por gigabit será cuello de botella; hay que medirlo antes de prometerlo |

---

## 3. LO QUE EL SUPERPROMPT DA POR HECHO Y NO LO ESTÁ

El superprompt dice en su §2: *«el informe anterior identificó módulos huérfanos de permisos, VRAM
y Gaming Radar… `files_write` y `run_command` no aplicaban la política… faltaba timeout… la
auditoría no estaba conectada»*.

**Eso ya no es cierto.** Se cerró en Fase 3.0 y está probado con 9 tests. Hoy:

- la política se consulta en **7 puntos** antes de leer, escribir o ejecutar;
- los comandos tienen **timeout y cancelación**;
- la auditoría escribe de verdad (**21 llamadas**).

Lo que sí sigue siendo cierto: **la app instalada no lleva nada de esto** y **el trabajo no está
commiteado**.

---

## 4. PLAN PROPUESTO (por valor real, no por el orden del PDF)

| Fase | Contenido | Depende de | Sesiones | Riesgo |
|---|---|---|---|---|
| **F3.3** | Cerrar la deuda: commit + `tauri:build` + instalar | tu autorización (E1, E2) | 0,5 | Bajo |
| **F3.4** | **KV Cache y parámetros de inferencia** (§4, §6): panel con `num_ctx`, KV type, memoria estimada vs real, paralelo, keep_alive y recomendación | F3.3 | 2-3 | Medio (variables globales) |
| **F3.5** | AI Engine Center + panel Ollama completo (descargar, borrar, GGUF, blobs, integridad) | F3.4 | 3-4 | Medio (borrado de modelos) |
| **F3.6** | GPU Router real + Auto Optimizer con simulación | F3.4 | 2-3 | Bajo |
| **F3.7** | PolyStrata: arrancar, validar API, comparar con Ollama y LM Studio | E4 | 1 | Medio |
| **F3.8** | ComfyUI general + vídeo anime↔real (Ditto / LTX 2.3 en formato API) | — | 3-4 | Alto (VRAM) |
| **F3.9** | Benchmark comparador entre motores | F3.7 | 1 | Bajo |
| **F3.10** | Agentes con herramientas reales + MCP | F3.4, F3.3 | 4-5 | **Alto** |
| **F3.11** | Memoria semántica / RAG | F3.4 | 2-3 | Medio |
| **F3.12** | Multi-PC con workers | F3.10 | 4-6 | Alto |
| **F3.13** | Release: firmado, actualizador, documentación §22 | todo | 2 | Medio |

**Total honesto: 25-37 sesiones.** Cualquier cifra menor sería mentira.

### Por qué este orden

1. **F3.3 primero**: sin rebuild, el usuario sigue teniendo Fase 2.1 en el escritorio y todo lo
   hecho no se ve. Y sin commit, un fallo de disco se lleva el trabajo.
2. **F3.4 después**: es lo que más pediste y lo que hoy está a cero; además el KV Cache es la
   palanca real de rendimiento en tus 12/16 GB.
3. **Agentes y Multi-PC al final**: son lo más caro y lo que más se rompe sin permisos y límites ya
   probados.

---

## 5. RIESGOS ABIERTOS (con números)

| Riesgo | Detalle | Mitigación |
|---|---|---|
| **Trabajo sin commitear** | 13 modificados + 23 nuevos, último commit de Fase 2.1 | Commit en F3.3 |
| **App instalada obsoleta** | exe del 11:02; código hasta 12:29 | Recompilar en F3.3 |
| **Disco H: al 97 %** | 102 GB libres, proyecto 8,2 GB, `target/` engorda | Limpiar `target` antes de builds grandes (con permiso) |
| **Ollama con 81 modelos** | Sin panel de gestión, borrar a mano es peligroso | Panel con confirmación en F3.5 |
| **KV Cache global** | Cambiarlo afecta a todo el sistema | F3.4 con backup y verificación |
| **Dos GPUs mal repartidas** | Ollama y ComfyUI acabaron los dos en la 5060 Ti | GPU Router en F3.6 |
| **Búsqueda GitHub sin token** | Escaneos seguidos devuelven menos | Token en configuración |

---

## 6. LO QUE NECESITO DE TI PARA EMPEZAR

1. **¿Apruebas F3.3** (commit + recompilar + instalar el exe nuevo)? Es lo único que hace visible
   todo lo hecho.
2. **¿Apruebas el orden** o prefieres empezar por otra fase?
3. **¿Tengo permiso para tocar variables globales de Ollama** en F3.4 (con backup previo)?

No he modificado nada. Espero tu palabra.
