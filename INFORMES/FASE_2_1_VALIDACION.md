# KUKIA CODE — FASE 2.1: VALIDACIÓN FINAL Y AUDITORÍA DE RELEASE

**Fecha:** 01/11/2026 · **Hora de cierre:** 11:20 (local)
**Proyecto:** `H:\KUKIA_CODE` · **Laboratorio:** `H:\KUKIA_RESEARCH\AI_EXPERIMENTAL`
**Versión:** 0.1.0 (package.json · src-tauri · kukia-core, las tres a 0.1.0)
**Ejecutado por:** Xoxete · **Supervisado por:** Kuki (LordKukone)

---

## 0. RESUMEN EJECUTIVO

| Prueba | Resultado |
|---|---|
| Reconstrucción con Research incluido | **PASS** |
| `npm run build` (tsc + vite) | **PASS** |
| `cargo test -p kukia-core` | **PASS** — 22/22 unitarios |
| Pruebas de validación nuevas | **PASS** — 8/8 locales + 5/5 de red |
| `npm run tauri:build` | **PASS** |
| Arranque real del EXE | **PASS** |
| Instalación NSIS | _ver §8_ |
| Ollama (listar + chat real) | **PASS** |
| LM Studio (listar + chat real) | **PASS** (tras corregir 2 fallos) |
| GPU 0 / GPU 1 | **PASS** |
| Research (3 fuentes + informe + descarga) | **PASS** |
| Seguridad (bloqueos + auditoría + backup) | **PASS** (1 fallo corregido) |
| Fuga de claves en disco/logs | **PASS** |
| Rendimiento tok/s | **PASS** — 83,4 tok/s con VRAM libre |

**Se han encontrado y corregido 4 fallos reales.** Todos con backup verificado del archivo tocado.

---

## 1. FALLOS ENCONTRADOS Y CORREGIDOS

### 1.1 Backup silencioso en `files_write` — GRAVEDAD ALTA

`src-tauri/src/lib.rs` copiaba el archivo a `backups` con `let _ = std::fs::copy(...)`. Si la copia
fallaba (permisos, disco lleno, archivo bloqueado), el error se descartaba y **el archivo se
sobrescribía sin copia de seguridad**. Viola la regla del proyecto «backup verificado antes de
sobrescribir».

Ahora: si no se puede crear el directorio, si falla la copia o si los tamaños no coinciden, **la
escritura se cancela** y se devuelve el error. Se registra `backup verificado: <ruta>` en el log.

Backup: `backups\lib.rs.20261101_104101.bak` (15.786 bytes, SHA-256 idéntico).

### 1.2 LM Studio en otro puerto no se detectaba — GRAVEDAD MEDIA

El conector apuntaba fijo a `127.0.0.1:1234`, pero LM Studio estaba escuchando en **1235**. La app
lo mostraba como «servidor apagado» aunque estuviera funcionando con 32 modelos.

Ahora se sondean `1234` y `1235` con una petición corta a `/v1/models` y se usa el que responda, sin
tocar la configuración de LM Studio.

Backup: `backups\providers.rs.20261101_104722.bak` (17.683 bytes, SHA-256 idéntico).

### 1.3 Respuesta vacía con modelos de razonamiento — GRAVEDAD ALTA

Los modelos con cadena de pensamiento (DeepSeek-R1, JEV, QwQ…) devuelven el texto en
`reasoning_content` y dejan `content` vacío. La app mostraba **nada** aunque el modelo hubiera
generado tokens. Reproducido en real: `respuesta: ""` con 16 tokens generados.

Ahora `content` vacío cae a `reasoning_content` (OpenAI/LM Studio) y a `thinking` (Ollama).

### 1.4 El pensamiento se mezclaba con la respuesta — GRAVEDAD BAJA

Se ha añadido el evento `ChatEvent::Reasoning`, canal separado en streaming, y en el chat se muestra
plegado en un desplegable «Pensamiento del modelo (N caracteres)». Además, interruptor en
Configuración:

- **Activado (por defecto):** los modelos razonan a pleno rendimiento (orden de Kuki: «razonar tiene
  que razonar»).
- **Desactivado:** se pide al motor que no piense (`think: false` en Ollama,
  `chat_template_kwargs.enable_thinking=false` + `reasoning_effort: none` en OpenAI-compatible), con
  reintento automático si el servidor rechaza esos campos.

---

## 2. PRUEBAS DEL NÚCLEO

```
cargo test -p kukia-core
  unittests src\lib.rs ................ 22 passed; 0 failed
  tests\validacion_fase21.rs .......... 8 passed; 0 failed; 5 ignored (red)
  doc-tests ........................... 0
```

Pruebas de red ejecutadas aparte (`--ignored --nocapture`): **5/5 PASS**

| Test | Evidencia real |
|---|---|
| `proveedor_ollama_lista_y_responde` | 81 modelos · chat contestado |
| `proveedor_lmstudio_lista` | 32 modelos |
| `proveedor_lmstudio_chat_real` | `"OK"` · 15 prompt / 2 completion · 531 ms |
| `research_escaneo_real_tres_fuentes` | 35 items · 10 alertas · informe 17.824 bytes |
| `research_descarga_aislada_respeta_limite` | LICENSE 1.058 bytes en laboratorio |

Pruebas locales (8/8 PASS): bloqueo de PowerShell sin aprobación, ejecución con aprobación, cwd
inexistente, límite de lectura 2 MB, lectura normal, auditoría en SQLite, persistencia de
sesiones/mensajes/memorias, rechazo de `http://` en descargas.

---

## 3. PROVEEDORES

### 3.1 Ollama — PASS

```
/api/version → 0.40.13
/api/tags    → 81 modelos
chat real    → llama3-ablit:latest · prompt 121 · completion 21
```

### 3.2 LM Studio — PASS

```
lms status        → Server: ON (port 1235)
lms ls            → 32 modelos · 519,70 GB
/v1/models        → 32 modelos
chat real         → dirty-muse-writer-v01 · "OK" · 15 prompt / 2 completion · 531 ms
```

Fallos encontrados durante la prueba (documentados en §1.2 y §1.3): puerto distinto y
`reasoning_content`.

### 3.3 Cloud — NOT TESTED

Sin `KUKIA_CLOUD_KEY` en el entorno. No se ha configurado ninguna clave.

---

## 4. RENDIMIENTO (tokens/s)

Medición real con `tests\bench_tok_s.ps1` sobre `llama3:latest`, GPU1 vacía:

| Escenario | Generación | Prompt | Carga |
|---|---|---|---|
| VRAM ocupada por LM Studio (2 modelos) | **0,13 tok/s** | — | — |
| VRAM libre, opciones por defecto | **83,41 tok/s** | 2.255 tok/s | 0,01 s |
| VRAM libre, `num_batch=1024` + `num_ctx=4096` + `num_gpu=99` | 82,75 tok/s | 329 tok/s | 9,31 s |
| VRAM libre, mismas + recarga | 83,65 tok/s | 2.214 tok/s | 0,01 s |

**Conclusión medida, no supuesta:** el cuello de botella era la **VRAM compartida entre dos motores**.
Liberar la GPU multiplicó el rendimiento por ~640. Las opciones avanzadas (batch, contexto, capas)
no aportan nada cuando el modelo ya cabe entero en la GPU; incluso empeoran el arranque porque
fuerzan recarga.

**Reglas prácticas que se derivan de la medición:**

1. **Un solo motor con modelos cargados a la vez.** Ollama y LM Studio peleándose por 16 GB es lo
   peor que se puede hacer.
2. **El modelo debe caber entero en una GPU** (`num_gpu=99`), sin offload a CPU.
3. **No mezclar GPU0 y GPU1 para un mismo modelo**: el reparto entre tarjetas distintas penaliza.
4. Flash attention y KV cache q8 no dan ganancia aquí (ya activos por defecto).
5. Para subir tok/s **sin perder inteligencia** queda una vía pendiente de medir: **speculative
   decoding** con modelo borrador (LM Studio lo soporta). No se ha probado aún; se propone para Fase 3
   con medición antes/después.

VRAM tras la limpieza: GPU0 **2.052 / 12.227 MiB** · GPU1 **11 / 16.311 MiB**.

---

## 5. GPU

```
0, NVIDIA GeForce RTX 5070,   12227 MiB total · 2052 MiB usados · driver 621.01
1, NVIDIA GeForce RTX 5060 Ti, 16311 MiB total ·   11 MiB usados · driver 621.01
```

- GPU0 sigue siendo la principal de pantalla/juegos.
- El perfil 35/65 **no se presenta como reparto real**: la vista dice «Reparto multi-GPU (perfil)» y
  avisa de que cada motor aplica solo lo que soporta.
- No se ha tocado ningún voltaje, frecuencia, límite ni driver.

---

## 6. RESEARCH — PASS

```
descubrimientos: 35   (arXiv 10 · Hugging Face 15 · GitHub 10)
alertas: 10
informe: H:\KUKIA_RESEARCH\AI_EXPERIMENTAL\reports\2026-11-01.md (17.824 bytes)
descarga: H:\KUKIA_RESEARCH\AI_EXPERIMENTAL\downloads\20261101_094322_LICENSE (1.058 bytes)
```

Muestras: `[arXiv] A Unified Hessian-Guided Framework for Post-Training Quantization` (0,70) ·
`[HF] axteam/gemma-4-E2B-it-NVFP4` (0,70) · `[GitHub] NVIDIA/Model-Optimizer` (0,90).

Aislamiento verificado: solo https (rechazo de `http://` probado), límite 25 MB por cabecera y por
tamaño real, destino único en el laboratorio, registro en `registro_descargas.log`, y ningún código
que ejecute lo descargado.

---

## 7. SEGURIDAD

| Prueba | Método | Resultado |
|---|---|---|
| Escritura sin aprobación | `files_write` retorna error antes de tocar disco | PASS |
| PowerShell sin aprobación | ejecución real del test | **PASS** |
| Operación autorizada registrada | tabla `approvals` con 2 filas reales | **PASS** |
| Sobrescritura con backup verificado | corregido en §1.1 | **PASS** |
| Claves fuera de disco | solo `std::env::var("KUKIA_CLOUD_KEY")` | **PASS** |
| Secretos en `%APPDATA%\KukiaCode` | búsqueda de `sk-`, `api_key`, `Bearer ` | **PASS** (0) |
| Permisos no eludibles desde la UI | checkbox obligatorio + botón deshabilitado | **PASS** (código) |

---

## 8. INSTALADOR — PASS

```
Instalador : H:\KUKIA_CODE\target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe
             2.753.792 bytes · 01/11/2026 11:02:01
             SHA256 A333BBB775FE08F67013F648469BC36597752CC8A818A9E39F5CEB289008E9CB
Binario    : H:\KUKIA_CODE\target\release\kukia-code.exe
             9.063.936 bytes · 01/11/2026 11:02:01
             SHA256 8A2AD47AC9057403EB7D7F31491DD30F30515350F483A3C493D7EF43B9F4F646
Instalado  : C:\Program Files\KUKIA CODE\kukia-code.exe
             versión 0.1.0
             SHA256 859A80782D325C2D456DFC9EB45CA93961A680EE99194DADE0D9EC66FD405DD6
```

Nota: el hash del ejecutable instalado **no** coincide con el de `target\release` porque el bundler
parchea el binario con la información de bundle (`Info Patching … with bundle type information:
nsis`) antes de empaquetarlo. Es el comportamiento normal de Tauri, no una alteración.

### 8.1 Instalación y arranque

```
Instalación silenciosa /S /CURRENTUSER → código de salida 0
Ejecutable localizado → C:\Program Files\KUKIA CODE\kukia-code.exe (versión 0.1.0)

Primer arranque
  Proceso vivo .................. True (PID 46876)
  Título de ventana ............. "KUKIA CODE"
  RAM en reposo ................. 45 MB
  kukia.db ...................... 73.728 bytes (intacta)
  kukia.log ..................... 520 → 585 bytes (añade "KUKIA CODE arrancando")
  CloseMainWindow ............... True → cierre limpio

Segundo arranque (persistencia)
  Proceso vivo .................. True (PID 34200)
  Título ........................ "KUKIA CODE"
  kukia.db tras reabrir ......... 73.728 bytes
```

- `%APPDATA%\KukiaCode` se crea y se conserva entre arranques.
- La base SQLite mantiene su tamaño e integridad tras cerrar y volver a abrir.
- La configuración persiste (no se regenera desde cero).
- **No se ha desinstalado**: la app queda instalada y operativa. Desinstalar requiere autorización
  explícita de Kuki. `deleteAppDataOnUninstall` no está activado, así que un desinstalador no
  borraría los datos del usuario, pero no se ejecuta sin permiso.

---

## 9. PROBLEMAS PENDIENTES

1. `installMode: perMachine` exige administrador (se instala en modo usuario).
2. Sin firma de código → SmartScreen avisará en otros equipos.
3. Sin actualizador firmado (el radar informa, no actualiza).
4. El reparto 35/65 es un perfil guardado, no un reparto activo.
5. MCP y research autónomo pendientes (F6 y F9).
6. `jev-9b` genera basura con prompts cortos: defecto del modelo, no del conector.
7. Speculative decoding sin probar.

---

## 10. RECOMENDACIÓN PARA FASE 3

1. **Permisos por herramienta y por proyecto**: allowlist de rutas escribibles y política por agente.
2. **Auditoría consultable** con filtros y exportación.
3. **Recuperación de tareas interrumpidas** con estado persistente.
4. **Guardia de VRAM**: avisar (o impedir) cuando dos motores tienen modelos cargados a la vez.
   Es la mejora de rendimiento más grande medida hasta ahora.
5. **Speculative decoding** con medición real antes/después.

---

> Informe generado con evidencia de ejecución. Nada se declara PASS sin prueba.
> Los cuatro fallos encontrados están documentados con su corrección y su backup.
