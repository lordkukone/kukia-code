# KUKIA CODE — ARQUITECTURA MULTI-PC (MASTER + WORKER)

**Fecha:** 01/11/2026 · **Autor:** Xoxete
**Estado:** **diseño, no implementación.** Nada de esto existe todavía en el código.
**Base técnica:** auditoría de hoy + mediciones reales de Fase 2.1 en tu equipo.

---

## 0. LO PRIMERO: LO QUE **NO** SE PUEDE PROMETER

Antes de dibujar nada, hay cuatro afirmaciones que circulan y que son falsas o sin demostrar. Las
digo ahora para que no se cuelen en el diseño:

| Afirmación | Realidad |
|---|---|
| «Se suma la VRAM de los dos ordenadores» | **No.** La VRAM no se suma por red. Cada equipo ejecuta su parte con **su** memoria. No existe memoria unificada entre PCs |
| «Repartir un modelo entre dos PCs lo acelera» | **Sin demostrar, y probablemente falso** en tu red. Medido en tu propio equipo: repartir entre GPU0 y GPU1 ya penaliza. Entre equipos, cada capa que cruza la red añade latencia por token |
| «Con `llama.cpp RPC` ya está resuelto» | RPC existe y funciona, pero hay que **medirlo** con un modelo pequeño. Gigabit da ~110 MB/s; los pesos no se mueven solo una vez, se sincronizan por capa |
| «El worker puede con Flint 40B NVFP4» | **No.** El RTX 2070 Super es Turing (**compute capability 7.5**): no tiene soporte FP4/NVFP4 (eso es Blackwell, tus 50-series). Y con 8 GB no cabe un 40B ni en Q4. El worker es para modelos **pequeños** y tareas auxiliares |

**Conclusión honesta:** lo que sí funciona y aporta es **reparto de tareas independientes**. El PC
principal hace el LLM; el secundario hace TTS, transcripción, visión o pruebas, **a la vez**. Eso es
paralelismo real, y no depende de sumar memoria.

---

## 1. INVENTARIO DE LOS DOS EQUIPOS

### 1.1 MASTER (existe, verificado hoy)

| Componente | Valor |
|---|---|
| CPU / RAM | i9-10900K (10C/20T) · 63,9 GB |
| GPU0 | RTX 5070 **12.227 MiB** — pantalla y juegos |
| GPU1 | RTX 5060 Ti **16.311 MiB** — inferencia |
| Driver / CUDA | 621.01 · CUDA 13.4 |
| Motores | Ollama 0.40.13 (81 modelos) · LM Studio 1.19.3 (32 modelos, API 1235) |
| Software | KUKIA CODE 0.1.0 instalada, ComfyUI, Fish Speech, Pinokio |
| Disco proyecto | H: 106 GB libres (**97 % usado**, ver E-09) |

### 1.2 WORKER (no conectado — **solo se hará inventario de lectura cuando esté**)

| Componente | Valor previsto |
|---|---|
| CPU / RAM | i7-12700 (12C/20T) · 32 GB DDR4-3600 |
| GPU | RTX 2070 Super **8 GB** — Turing, compute 7.5, **sin FP4/NVFP4** |
| SO | Windows |
| Motores | a decidir: Ollama (GGUF Q4/Q5) es lo sensato para Turing |

**Regla:** no se asume que esté encendido, ni en la misma red, ni con permisos. Primero se inventaría;
después se instala algo, y solo con tu autorización.

**Qué le sienta bien a 8 GB:** modelos 7–8B en Q4 (~4,5 GB), Whisper `small`/`medium`, TTS tipo Fish
Speech o XTTS, embeddings, visión ligera, tests y compilaciones. **Qué no:** 30B/40B, vídeo, Flux.

---

## 2. PRINCIPIOS DEL DISEÑO

1. **Tareas independientes, no modelos partidos.** Es lo único que escala de verdad.
2. **El MASTER manda y guarda la verdad.** La base SQLite del master es la fuente de verdad; el worker
   es **sin estado** (o con estado desechable).
3. **Sin puertos públicos.** Solo LAN, y si es posible solo por la interfaz privada.
4. **Autenticado.** Un token por worker, rotable, comparado en tiempo constante, y **auditado**.
5. **Todo lo remoto pasa por el motor de permisos.** Un trabajo remoto no es una puerta trasera para
   saltarse la allowlist local.
6. **Si el worker no está, el sistema sigue funcionando.** Degradación a local, nunca bloqueo.

---

## 3. COMPONENTES

```
┌──────────────────────────── KUKIA MASTER (tu PC) ────────────────────────────┐
│  KUKIA CODE (UI + core)                                                      │
│   ├── Job Scheduler  ─────── usa la tabla `tasks` (YA EXISTE)                │
│   ├── Worker Registry ────── workers, capacidades, latencia, salud            │
│   ├── permissions::check ─── cada trabajo remoto se autoriza aquí            │
│   ├── audit_log ──────────── cada envío/resultado queda registrado           │
│   └── Resource Monitor ───── GPU local (vram.rs YA EXISTE) + worker (heartbeat)│
└───────────────────────────────┬──────────────────────────────────────────────┘
                                │  LAN · HTTP/JSON · token + HMAC
┌───────────────────────────────▼──────── KUKIA WORKER (i7-12700) ────────────┐
│  kukia-worker.exe (servicio o bandeja)                                       │
│   ├── /health      → uptime, versión, carga                                  │
│   ├── /inventory   → CPU, RAM, GPU, VRAM, modelos instalados (SOLO LECTURA)  │
│   ├── POST /jobs   → {kind, payload, deadline}  → job_id                     │
│   ├── GET  /jobs/{id} → estado, progreso, resultado o error                  │
│   └── POST /jobs/{id}/cancel                                                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 3.1 Tipos de trabajo (`kind`) que el worker aceptará

| kind | Motor | Por qué encaja en 8 GB |
|---|---|---|
| `tts` | Fish Speech / XTTS | Modelo de voz pequeño; no compite con el LLM del master |
| `stt` | Whisper `small`/`medium` | Cabe y va sobrado |
| `embed` | Embeddings locales | Barato y constante |
| `vision.light` | Modelo tipo Qwen-VL pequeño | 8 GB justos, medir |
| `test.run` | Cargo / npm / pytest | **No usa GPU**: es el mejor uso del worker |
| `bench` | Benchmark aislado | Comparaciones limpias sin tocar el master |

**Nunca** `llm.chat` con modelos grandes: el master ya tiene 16 GB y el worker tiene 8.

### 3.2 Formato del trabajo

```json
{
  "id": "tsk_01H...",
  "kind": "tts",
  "payload": { "text": "…", "ref_audio": "sha256:…", "format": "wav" },
  "deadline_s": 300,
  "created_by": "kukia-master",
  "policy": { "allow_network": false, "write_paths": null }
}
```

El `policy` que viaja **no se puede ampliar** en el worker: el worker solo puede reducirlo. Y la
respuesta lleva `job_id`, `status`, `progress`, `result_path|result_inline`, `error`, `metrics`.

---

## 4. SEGURIDAD (no negociable)

| Requisito | Implementación concreta |
|---|---|
| Sin exposición pública | Bind a la IP LAN concreta (`192.168.x.x`), no `0.0.0.0`. Regla de firewall de Windows limitada a la subred |
| Autenticación | Token de 32 bytes por worker, en `%APPDATA%\KukiaCode\config\workers.json` (solo hash). Comparación en tiempo constante |
| Integridad | Firma HMAC-SHA256 del cuerpo con el token; rechazo si difiere |
| Transporte | HTTPS con certificado autofirmado **pineado** por huella en el master. Si se prefiere simple: HTTP en LAN **solo** si el token + HMAC están activos, y avisado en la UI |
| Autorización | El master comprueba `permissions::check` **antes** de enviar; el worker rechaza `write_paths` que no estén en su propia allowlist |
| Auditoría | Cada envío, éxito, fallo y cancelación → `audit_log` con `actor=remote:worker-01` |
| Cancelación | `POST /jobs/{id}/cancel` y **timeout duro** (mismo problema que E-03: nunca un trabajo sin límite) |
| Datos | El worker **no recibe** tu clave cloud, ni rutas de wallets, ni nada de `I:\CEREBRO` |

---

## 5. QUÉ CÓDIGO YA EXISTE Y SE REUTILIZA

No se empieza de cero. Esto ya está escrito y probado y sirve tal cual:

| Pieza existente | Uso en multi-PC |
|---|---|
| `tasks` + `task_upsert`/`task_get`/`task_set_status`/`tasks_interrupted` (`memory.rs`) | Es **la cola de trabajos**. Añadir columnas `worker_id` y `remote` |
| `audit_log` / `audit_search` / `audit_export_csv` | Registro de trabajos remotos (cuando E-02 se arregle) |
| `vram.rs` (`gpus_vram`, `gpu_processes`, `fits`) | Estado local; el worker envía lo mismo en `/inventory` |
| `gpu.rs` + `sysinfo` | Monitor de CPU/RAM; reutilizable en el worker |
| `benchmarks` | Guardar tok/s + VRAM + worker para comparar condiciones |
| `permissions.rs` | Puerta de autorización de trabajos remotos |
| `config.rs` (`KUKIA_DATA_DIR`) | Que el worker guarde sus datos en su propio directorio |

**Falta de verdad:** el agente HTTP del worker, el descubrimiento en LAN, la autenticación, y las
columnas `worker_id` en `tasks`. Eso es lo nuevo, y es menos de lo que parece.

---

## 6. DESCUBRIMIENTO EN LAN

Opciones, de menos a más invasivo:

1. **Lista manual** (recomendada para empezar): tú escribes `192.168.1.50:8791` y el token. Cero
   magia, cero superficie de ataque, funciona siempre.
2. **mDNS/`_kukia._tcp`**: cómodo; anuncia la existencia del worker a toda la red. Aceptable en casa.
3. **Broadcast UDP a 8792**: descubre solo, pero es ruidoso y hay que filtrarlo.

Empieza por la lista manual. El descubrimiento automático es una comodidad, no un requisito, y añade
superficie de ataque.

---

## 7. MEDIR ANTES DE PROMETER (plan de validación)

Tres pruebas, en este orden, con números que se publican o no se publica nada:

| # | Prueba | Qué mide |
|---|---|---|
| 1 | TTS de 500 palabras: local (GPU1 del master) vs worker | Segundos, calidad subjetiva, pico de VRAM |
| 2 | `llama.cpp RPC` con un modelo de **1B** partido entre master y worker | tok/s frente a master solo. **Espero pérdida**; si la hay, se descarta la idea y se documenta |
| 3 | Dos tareas a la vez (LLM en master + TTS en worker) | Latencia del LLM comparada con hacerlas en serie |

**Criterio de éxito del proyecto multi-PC:** la prueba 3 demuestra que hacer dos cosas a la vez tarda
**menos** que hacerlas en serie, y la prueba 2 no se usa en producción porque empeora.

---

## 8. FASES DE IMPLEMENTACIÓN

| Fase | Trabajo | Criterio de aceptación |
|---|---|---|
| **M0** | Inventario de lectura del worker (cuando enciendas) | Informe con CPU/GPU/VRAM/modelos, **sin instalar nada** |
| **M1** | `kukia-worker.exe` con `/health` e `/inventory` | `curl` desde el master devuelve JSON real del worker |
| **M2** | Token + HMAC + firewall de subred | Petición sin token → `401`; token incorrecto → `401`; desde otra subred → timeout |
| **M3** | Cola: `worker_id` en `tasks`, `POST /jobs`, sondeo, timeout | Un trabajo `stt` termina y su resultado aparece en el master |
| **M4** | Cancelación y recuperación al reiniciar | Matar el worker a mitad → el master marca `interrupted`, no se queda colgado |
| **M5** | MULTI-PC Center (vista) | CPU/GPU/VRAM/temperatura de ambos + cola + histórico |
| **M6** | Un caso de uso real de punta a punta | Whisper o TTS en el worker desde la UI, medido |

**Estimación honesta: 3–4 sesiones** para M1–M4 (lo mínimo útil). M5–M6, dos más.

---

## 9. RIESGOS

| Riesgo | Nivel | Mitigación |
|---|---|---|
| RTX 2070 sin FP4: copiar un modelo NVFP4 y que no arranque | **Alto** | Usar GGUF Q4/Q5; `inventory` reporta `compute_capability` y modelos válidos |
| 8 GB engañosos: muchos modelos no caben | Medio | `fits()` con 12 % de margen antes de enviar el trabajo |
| Red gigabit como cuello en cualquier reparto de modelo | Alto | No repartir modelos; solo tareas |
| Un token filtrado da acceso a ejecución | **Alto** | Bind a subred, HMAC, rotación, auditoría, y el worker **nunca** ejecuta comandos arbitrarios: solo `kind` conocidos |
| Worker apagado a mitad de trabajo | Medio | `deadline_s` + `tasks_interrupted` (ya existe) |
| Dos motores en el master mientras el worker también tira de red | Medio | VRAM Guard conectado (Fase 3.0) antes de multi-PC |
| Disco H: al 97 % con dos builds | Medio | `CARGO_TARGET_DIR` fuera de H: |

---

## 10. RESUMEN

- **Sí:** reparto de tareas independientes entre dos equipos, con el master de cerebro y el worker de
  músculo para TTS/STT/embeddings/tests.
- **No:** sumar VRAM, partir modelos entre PCs y esperar que vuele, o meter Flint NVFP4 en una 2070.
- **Ya está hecho** el 60 % de la infraestructura (cola `tasks`, auditoría, VRAM, perfiles).
- **Lo nuevo** es el agente del worker, la autenticación y las columnas de worker en la cola.
- **Antes de todo esto:** Fase 3.0. Sin timeout ni permisos aplicados, repartir trabajo entre dos
  equipos multiplica los fallos en lugar de las capacidades.
