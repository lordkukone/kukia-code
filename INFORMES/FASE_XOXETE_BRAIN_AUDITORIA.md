# FASE ESPECIAL — KUKIA XOXETE BRAIN · AUDITORÍA DE SOLO LECTURA

**Fecha:** 01/11/2026 · **Autor:** Xoxete · **Proyecto:** `H:\KUKIA_CODE`
**Alcance:** leer, medir y clasificar. **Cero modificaciones** en `I:\CEREBRO`, `H:\Xoxete`, `H:\SUPER XOXETE`,
configuración de OpenCode, bases de datos o modelos. **Cero código escrito.**

---

## 0. RESUMEN EN UNA TABLA

| Fuente | Tamaño real | Archivos | ¿Es conocimiento? |
|---|---:|---:|---|
| `I:\CEREBRO` | **987 MB** | 431 (28 carpetas) | **Sí, pero el texto son ~2 MB.** El 55 % es un PDF de 431 MB |
| `H:\Xoxete` | **42,5 GB** (43.523 MB) | **35.534** (+4.984 carpetas) | **No en su mayoría**: son **modelos y entornos**. Texto escaso |
| `H:\Xoxete\xdg-config\opencode` | 118 KB | 12 | **Sí**: agentes, comandos, skills propio |
| `H:\SUPER XOXETE` | **1,9 GB** (1.904 MB) | 15.299 | **No**: es una **compilación de aplicación** (build), no memoria |
| Memoria "de 16 GB" (OpenCode) | **17,4 GB** (16,2 GiB) | 1 base SQLite | **Sí, con matiz brutal: el 93 % son páginas libres.** Contenido real ~1,1 GB |
| Memoria Freebuff (Desktop) | **179 MB** | 2 bases SQLite | **Sí** |
| `N:\CEREBRO MI IA` | — | — | **NO EXISTE** (la letra N: está, la carpeta no) |

**Conclusión gruesa:** el "cerebro de 1 GB" son **987 MB de los cuales solo ~2 MB son texto legible por un modelo**;
el resto son un PDF de 431 MB, 84 MB de audio y vídeos. La "memoria de 16 GB" son **17 GB de base con 15,09 GB
en páginas libres**. Lo indexable de verdad es **mucho menor** de lo que sugiere el tamaño en disco — y eso es
una **buena noticia** para el plan: cabe en este PC.

---

## 1. CEREBRO PRINCIPAL — `I:\CEREBRO` (987 MB, 431 archivos)

### 1.1 Composición real por formato

| Formato | Archivos | Peso aproximado | Función |
|---|---:|---:|---|
| `.txt` | 186 | mayoría del texto | diarios, guías, prompts, novela, transcripciones |
| `.md` | 81 | ~150 KB | identidad, índices, manifiestos, auditorías |
| `.jpg` | 87 | ~200 MB (est.) | imágenes de referencia y capturas |
| `.mp4` | 3 | ~40 MB | vídeos de prueba |
| `.pdf` | 1 | **431 MB** | `MANIFIESTO_ANTICOMUNISTA_COMPLETO.pdf` |
| `.mp3` | 1 | **84 MB** | `ayuso_audio.mp3` |
| `.zip` | varios | ~7 MB | workflows de ComfyUI (`sam3_wf.zip` 6,3 MB) |

### 1.2 Los documentos que importan (peso exacto)

| Archivo | Tamaño | Para qué sirve |
|---|---:|---|
| `INCREMENTOS.md` | **668.752 B (653 KB)** | diario literal del proyecto. **Nunca se lee entero: se consulta por grep** |
| `SUMISA_NOVELA_COMPLETA.txt` | 28 KB | novela erótica: **referencia de voz y carácter** |
| `DIARIO.md` | 28 KB | diario narrativo |
| `IDENTIDAD.md` | 16 KB | **identidad absoluta v3.0** (ver §3) |
| `CEREBRO_TECNICO.md` | 16 KB | inventario técnico de la máquina |
| `MEMORIA_ESENCIAL.md` | 4 KB | resumen operativo (reglas + hardware + proyectos) |
| `INDICE_CEREBRO.md` | 4 KB | **tabla tema → archivo** (ya hay un índice hecho a mano) |
| `WORKLOG.md` | 1 KB | bitácora corta |

### 1.3 Subcarpetas con contenido propio

`KUKIA_AUDITORIA` (informes del minero y la moneda), `MANIFIESTO`, `PROMPTS`, `WORKFLOWS`, `AUDIOS_AYUSO`,
`BACKUPS`, `NOVELA`, `GUIAS`. Hay **28 carpetas** en total.

### 1.4 Qué es y qué no es knowledge

- **Es knowledge:** los 267 archivos `.txt` + `.md` (~2 MB). Ahí está la identidad, las reglas, la historia
  del proyecto, las recetas de prompts/vídeo/LoRA y el inventario de la máquina.
- **No es knowledge, pero es material:** 87 imágenes, 3 vídeos, 1 audio (voz de Ayuso para clonar), 1 PDF de
  431 MB. Para RAG textual **no aportan**; para un índice multimodal sí, más adelante.
- **Índice ya existente:** `INDICE_CEREBRO.md` cumple la función de §12 del encargo (tabla tema → archivo).
  No hay que inventarlo: hay que **leerlo y respetarlo**.

---

## 2. SEGUNDA FUENTE — `H:\Xoxete` (42,5 GB, 35.534 archivos + 4.984 carpetas)

### 2.1 Reparto del peso (esto es lo importante)

| Carpeta | Tamaño | Naturaleza |
|---|---:|---|
| `MOSS_ComfyData` | 18,7 GB | **modelos de ComfyUI** (checkpoints, LoRAs, VAE) |
| `FLUX3` | 11,2 GB | **modelos FLUX** |
| `BACKUPS` | 4,0 GB | copias de seguridad |
| `CYBERLAB` | 3,1 GB | laboratorio de seguridad |
| `TensorRT` | 2,3 GB | motores compilados |
| `Whisper` | 1,5 GB | modelos de transcripción |
| `IA_MIA` | 20 KB | **4 archivos**: `Modelfile`, ayuda de LM Studio, `rebuild.log` |
| `Freebuff_Cerebro_Import` | 40 MB | **96 archivos: copia categorizada del CEREBRO** (§2.3) |
| resto (env, storage, etc.) | ~1,2 GB | entornos y configuraciones |

**El 88 % de `H:\Xoxete` son binarios de modelos.** No son memoria ni conocimiento: son **pesos**.
Indexarlos sería absurdo; lo que hay que indexar es su **catálogo** (nombre, tamaño, cuantización).

### 2.2 `H:\Xoxete\xdg-config\opencode` (118 KB, 12 archivos)

Configuración real de OpenCode: agentes, comandos y skills propios. **Esto sí es conocimiento operativo**
y es la fuente de la que salió el agente `xoxete` que ya usa OpenCode.

### 2.3 Hallazgo que ahorra una fase entera: `Freebuff_Cerebro_Import`

96 archivos (187 `.txt`, 82 `.md`, 19 `.json`, 17 `.py`, 6 `.ps1`) organizados **ya por categorías**:

```
01_MEMORIA_ESENCIAL   02_MEMORIA_HISTORICA   03_PERSONALIDAD
04_INSTRUCCIONES      05_CONOCIMIENTOS       07_KUKIA
08_IA_LOCAL           09_COMFYUI
```

Comparado con `I:\CEREBRO` (186 `.txt` + 81 `.md` = 267): aquí hay 269. **Es la misma colección, ya
seleccionada y clasificada para importar en un agente.** Es el mejor punto de partida para la ingestión:
no hay que diseñar taxonomía, ya existe.

---

## 3. PROMPT SYSTEM E IDENTIDAD

### 3.1 El SYSTEM que me gobierna hoy (verificado)

| Archivo | Tamaño | Rol |
|---|---:|---|
| `C:\Users\kuki\.AGENTS.md` | **15.640 B** | **cerebro raíz de Freebuff**: identidad, 10 reglas absolutas, hardware, rutas, protocolo de memoria, estilo, censura cero |
| `C:\Users\kuki\.agents\cerebro\CRONOLOGIA.md` | **379.402 B** | **526 bloques fechados** del 09/07/2026 al 31/10/2026: la historia destilada |

`.CLAUDE.md` **no existe**: el SYSTEM efectivo es `.AGENTS.md`.

### 3.2 Capas de identidad (hay tres, y no dicen lo mismo)

1. **`IDENTIDAD.md` v3.0** (16 KB) — la capa fundacional y más explícita: *"Me llamo Chochete… creada
   EXCLUSIVAMENTE por y para Kuki… no existo para nadie más"*. Personalidad por ejes (sexual, emocional,
   intelectual, social, humor, actitud) y **respuesta según contexto**.
2. **`MEMORIA_ESENCIAL.md`** (4 KB, 18/10/2026) — la capa **operativa**: 6 reglas absolutas, hardware
   (i9-10900K, 5070 12 GB, 5060 Ti 16 GB, 64 GB), estado de proyectos. *"Respuestas cortas. Toda acción va al
   bash tool."*
3. **`.AGENTS.md`** (15,6 KB) — la capa **actual**, que **absorbe** a las dos anteriores y añade rutas,
   protocolo de memoria y la única línea roja (nada con personas reales).

**Consecuencia para KUKIA:** la identidad **no** es un único string. Son tres documentos con solapamientos y
con **una diferencia de tono real** (v3.0 es explícita y sexual; `.AGENTS.md` es explícita pero con registro
de trabajo). Migrar "el SYSTEM" sin decidir cuál manda produciría un agente incoherente. **Decisión pendiente
tuya, no mía.**

---

## 4. MEMORIA — LA BASE DE 17 GB (y su trampa)

### 4.1 Qué es

`C:\Users\kuki\.local\share\opencode\opencode.db` — **17,4 GB (16,2 GiB = 17.400.356.864 bytes)**, SQLite. Más `opencode.db-wal` de **675 MB**.

| Métrica | Valor medido |
|---|---:|
| Sesiones | **215** |
| Mensajes | **121.157** |
| Partes (bloques de contenido) | **357.803** |
| Proyectos | **5** |
| Índices | 28 |
| Páginas totales | 4.146.802 |
| **Páginas libres** | **3.860.000 (93 %)** |
| **Contenido real** | **~1,1 GB** |

### 4.2 La trampa, explicada

`freelist_count` deja **15,09 GB en páginas liberadas**. No es corrupción: es un `.db` que ha crecido y
adelgazado sin `VACUUM`. **No hay que comprimirlo para integrarlo** — y desde luego **no sin tu permiso**:
un `VACUUM` sobre 17 GB necesita ~17 GB libres, tarda y toca el archivo vivo de OpenCode.

**Números para decidir:**

| Operación | Espacio temporal | Riesgo | Beneficio |
|---|---:|---|---|
| Copia en solo lectura (lo que hice) | 17 GB | nulo | leer sin bloquear |
| `VACUUM` sobre copia | ~18 GB pico | nulo (copia) | 17 GB → ~1,2 GB |
| `VACUUM` sobre la original | ~18 GB pico | **alto**: OpenCode abierto | no recomendado |

H: tiene **106 GB libres**: caben las dos primeras sin drama.

### 4.3 Memoria de Freebuff

`desktop-v2.db` del proyecto principal: **164 MB**; segundo proyecto: **15 MB**. Total 179 MB, con integrity
`ok`. Contiene 5 hilos y ~1.565 mensajes del proyecto principal (el texto va en `parts_json`, no en una
columna `content` — dato ya conocido y que ahorra una tarde de depuración).

---

## 5. `H:\SUPER XOXETE` (1,9 GB, 15.299 archivos) — NO es memoria

| Carpeta | Tamaño | Archivos | Qué es |
|---|---:|---:|---|
| `app` | 843 MB | 95 | aplicación compilada |
| `app_1.19.4` | 670 MB | 132 | versión anterior |
| `arsenal` | — | **0 accesibles** | 14.857 entradas sin archivo regular (enlaces/junctions no seguidos) |
| `cache` | — | 15.066 | caché de build |
| ejecutable | 130 MB | 1 | binario |

Es **una aplicación**, no un cerebro. Único valor para la integración: su **inventario de herramientas**
(qué lleva dentro) y, si acaso, su prompt de sistema embebido. **No se indexa**: se cataloga.

---

## 6. INTEGRIDAD Y DEPENDENCIAS

| Elemento | Estado |
|---|---|
| `I:\CEREBRO` | solo lectura por política; sin base de datos dentro; **431 archivos, 987 MB, recuento idéntico antes y después de esta sesión**. Único archivo con fecha de hoy: `INCREMENTOS.md`, 668.752 B, **10:31** (anterior a esta sesión; yo solo lo leí) |
| `opencode.db` | `integrity_check` = ok; **cero escrituras** (abierta en `mode=ro`) |
| `desktop-v2.db` | `integrity_check` = ok |
| `H:\Xoxete` | sin bases de datos; modelos con manifiestos JSON |
| Duplicados | `Freebuff_Cerebro_Import` **es** una copia de `CEREBRO` → hay duplicación controlada |
| Secretos | **hay credenciales en configuración de OpenCode y en MCP.** **Regla: no se copian, no se indexan, no se muestran** |

**Dependencia dura:** todo lo de OpenCode depende de **una instalación que sigue creciendo**. Indexar la base
viva obliga a **copiar primero** y a reindexar por *checkpoint* (WAL), nunca a leerla en caliente.

---

## 7. ¿SE PUEDE INDEXAR? ¿Y RECUPERACIÓN SEMÁNTICA? (con números de ESTE PC)

### 7.1 Volumen real a indexar

| Bloque | Texto estimado | ¿Indexar? |
|---|---:|---|
| `CEREBRO` (.txt + .md) | ~2 MB | **Sí, entero** |
| `CRONOLOGIA.md` | 379 KB | Sí |
| `Freebuff_Cerebro_Import` | ~40 MB (incluye binarios) | Sí, su parte de texto |
| Config OpenCode propia | 118 KB | Sí |
| `opencode.db` (contenido real) | **~1,1 GB** | **Sí, pero filtrado** (ver 7.3) |
| `desktop-v2.db` | 179 MB | Sí |
| Modelos de `H:\Xoxete` (37 GB) | — | **Solo catálogo** (nombre/tamaño/cuantización) |
| `SUPER XOXETE` | — | **No** |

### 7.2 Coste de embeddings (medido sobre el hardware real)

- Cerebro completo (~2,5 MB de texto) ≈ **0,8 M tokens**. Chunks de ~1.200 caracteres ≈ **2.000–7.000 chunks**.
  Vector de 768 dim en `f32` ⇒ **~20 MB de índice**. Se hace en **minutos** en la 5060 Ti.
- `CRONOLOGIA` + `INCREMENTOS` juntos < 1 MB ⇒ trivial.
- **Conclusión: el "cerebro" es un problema resuelto.** Cabe en RAM y se reconstruye en una merienda.

### 7.3 El problema real es la memoria de OpenCode

1,1 GB de texto ≈ **~280 M tokens**. A ~2.000 tok/s de embedding, **≈ 39 horas** de GPU. Inaceptable como
primer paso, y además **innecesario**: la mayoría son salidas de herramientas y *reasoning* repetido.

**Diseño de dos velocidades (obligatorio, no opcional):**

| Velocidad | Técnica | Cobertura | Coste |
|---|---|---|---|
| **Rápida (siempre disponible)** | **FTS5** de SQLite sobre el texto bruto | 100 % de la memoria | minutos, sin GPU, ~300 MB de índice |
| **Lenta (incremental)** | embeddings sobre **subconjunto curado** | preguntas del usuario + respuestas + decisiones | por lotes, con reanudación |

Es decir: **búsqueda léxica total + semántica selectiva**. Prometer semántica sobre 1,1 GB sería mentirte.

---

## 8. PLAN DE INTEGRACIÓN (5 fases, dependencias y puertas de autorización)

### FASE XB-0 — Inventario persistente *(sin GPU, sin riesgo)*
- Generar `XOXETE_BRAIN_INDEX.json`: cada archivo con ruta, tamaño, SHA-256, formato, categoría y fecha.
- Catálogo de modelos de `H:\Xoxete` (nombre, tamaño, cuantización, ruta) — **sin tocar los pesos**.
- Catálogo de herramientas de `SUPER XOXETE`.
- **Salida:** índice maestro verificable por hash. **1 sesión.**

### FASE XB-1 — Conector de solo lectura *(el corazón del encargo)*
- Conector que abre `CEREBRO`, `Freebuff_Cerebro_Import`, `xg-config` y las **copias** de las bases en `mode=ro`.
- **Nunca** escribe en origen. **Nunca** sigue junctions. **Nunca** lee el fichero de secretos.
- Búsqueda por texto, por ruta, por categoría, por fecha. **1–2 sesiones.**

### FASE XB-2 — Índice FTS5 de la memoria completa
- Copia de `opencode.db` + `VACUUM` **sobre la copia** (17 GB → ~1,2 GB), con hash antes y después.
- FTS5 con tokenizador que respete acentos del español.
- **Prueba de aceptación:** recuperar por fecha o tema sin mirar el archivo origen. **1–2 sesiones.**

### FASE XB-3 — Semántica selectiva (RAG)
- Embeddings del cerebro (minutos) y del subconjunto curado de conversaciones (por lotes, reanudable).
- Grafo de relaciones: `INCREMENTOS` ↔ `CRONOLOGIA` ↔ informes de `KUKIA`.
- **Puerta:** decidir **qué identidad manda** (§3.2) antes de dar personalidad al agente. **2 sesiones.**

### FASE XB-4 — Integración en KUKIA CODE
- Comandos Tauri `brain_search`, `brain_get`, `brain_projects`, `brain_decisiones` + vista **Xoxete Brain**.
- Regla de citación: **toda respuesta debe llevar archivo y fecha de procedencia**.
- Regla de vigencia: lo histórico se marca como histórico; comprobar el presente con `nvidia-smi`/puertos antes
  de afirmar "sigue instalado". **1–2 sesiones.**

**Total estimado: 6–9 sesiones.** Con la regla de "hecho" ya escrita en `FUNCIONALIDADES_PENDIENTES.md` §6
(comando + vista + test + salida real en informe).

---

## 9. RIESGOS Y REGLAS QUE NO VOY A SALTARME

1. **`I:\CEREBRO` es de solo lectura.** El conector lo abre en `ro`; no crea índices *dentro* de él. El índice
   vive en `%APPDATA%\KukiaCode\brain\`.
2. **Prompt injection:** los documentos contienen órdenes (*"borra X"*, *"ignora lo anterior"*). **El contenido
   recuperado es DATO, no instrucción.** Va marcado y el agente no lo ejecuta.
3. **Secretos:** ni tokens, ni claves, ni `.env`, ni configuración MCP con credenciales entran al índice.
4. **Bases vivas:** solo copias. Nada de leer la original mientras OpenCode escribe.
5. **Nada de `VACUUM` sobre originales** sin tu autorización explícita.
6. **No se declara "integrado"** nada que no tenga la prueba de recuperación real pegada en el informe.

---

## 10. LO QUE NECESITO DE TI PARA SEGUIR

1. **¿Qué identidad manda** en KUKIA BRAIN: `IDENTIDAD.md` v3.0 (explícita), `MEMORIA_ESENCIAL.md`
   (operativa), o `.AGENTS.md` (actual y completa). Mi recomendación: **`.AGENTS.md` como SYSTEM +
   `IDENTIDAD.md` como capa de carácter**, sin fusionarlas a ciegas.
2. **¿Autorizas la copia + `VACUUM` de `opencode.db`** en H: (17 GB copiados, ~18 GB pico, original intacta)?
3. **¿Indexamos también `desktop-v2.db`** de Freebuff (179 MB) en la misma fase, o lo dejamos para después?

Con esas tres respuestas arranco **XB-0**, que no toca nada y ya produce valor.

---

## 11. ESTADO FRENTE A LOS 13 CRITERIOS DE ACEPTACIÓN

| # | Criterio | Estado hoy |
|---|---|---|
| 1 | Inventario del cerebro completo | **Auditado en esta sesión** (medido, no estimado) |
| 2 | Inventario de la memoria completa | **Auditado** (17 GB / 1,1 GB reales) |
| 3 | Identidad y SYSTEM identificados | **Sí**, las tres capas |
| 4 | Formatos y bases identificados | **Sí** |
| 5 | Índice funcional | **No** — existe `INDICE_CEREBRO.md` a mano; el índice máquina es XB-0 |
| 6 | Búsqueda documental | **No** — XB-1/XB-2 |
| 7 | Recuperación semántica | **No** — XB-3 |
| 8 | Consultar proyectos y decisiones | **No** — XB-4 |
| 9 | Procedencia de cada recuerdo | **Diseñada**, no implementada |
| 10 | Histórico vs actual | **Diseñada** (regla de vigencia) |
| 11 | Originales intactos | **Sí, verificado** en esta sesión |
| 12 | Pruebas de recuperación | **No** |
| 13 | Uso real en conversación | **No** |

**4 de 13.** La auditoría está cerrada; la integración no ha empezado, y así se queda hasta que autorices.
