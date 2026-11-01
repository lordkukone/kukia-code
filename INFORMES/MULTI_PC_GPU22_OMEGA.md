# MULTI-PC OMEGA — NODOS AGNÓSTICOS DE GPU Y LA FUTURA GPU DE 22 GB

**Fecha:** 01/11/2026 · **Autor:** Xoxete · **Proyecto:** `H:\KUKIA_CODE`
**Estado:** **diseño y auditoría. No se ha implementado ni instalado nada.** El segundo PC no se ha tocado.
**Base:** [ARQUITECTURA_MULTI_PC.md](ARQUITECTURA_MULTI_PC.md) (diseño de nodos, seguridad y fases M0–M6).
**Este documento no lo repite: añade lo que faltaba** — detección agnóstica de GPU, la matriz de
compatibilidad por VRAM y el plan de sustitución de la 2070 Super por la futura 22 GB.

---

## 0. LO PRIMERO: UN BLOQUEO MEDIDO, NO SUPUESTO

Antes de diseñar nada, medí la red de este PC:

```
Wi-Fi     Intel Wi-Fi 6 AX201 160MHz    Up            136 Mbps   192.168.1.11/24
Ethernet  Intel Ethernet I225-V         Disconnected    0 bps     (sin IP)
Tailscale (virtual)                     100.116.82.33/32
Puerta de enlace: 192.168.1.1
```

**Estás conectado por Wi-Fi a 136 Mbps y el cable está desconectado.** Eso son ~17 MB/s teóricos y,
en la práctica, con jitter. Un enlace gigabit cableado son ~118 MB/s estables.

| Enlace | Ancho de banda | Sirve para despachar trabajos | Sirve para partir un modelo |
|---|---:|---|---|
| Wi-Fi actual (136 Mbps) | ~17 MB/s | Sí, con margen | **No** |
| Ethernet 1 GbE (cable) | ~118 MB/s | Sí, sobrado | Con matices (§8) |
| 2.5 GbE | ~295 MB/s | Sí | Mejor, pero no arregla la latencia |

**Conclusión:** la primera pieza del multi-PC no es software, es **un cable**. Con 136 Mbps de Wi-Fi el
reparto de tareas funcionará (los mensajes son pequeños) pero cualquier prueba de modelo partido será
una pérdida de tiempo.

---

## 1. ¿MERECE LA PENA? VEREDICTO CON NÚMEROS MEDIDOS

La investigación de hoy confirma lo que el diseño anterior sospechaba, ahora con cifras publicadas:

| Configuración medida | Red | Resultado |
|---|---|---|
| Mac Studio M2 Ultra + DGX Spark, Qwen2.5-7B Q4 | 10 GbE (9,41 Gbps reales) | Local **91,8 tok/s** → con RPC **52,7 tok/s** (**−43 %**) |
| Mismo par, Qwen2.5-72B Q4 | 10 GbE | Local 11,1 → RPC **5,9 tok/s** (**−47 %**) |
| RTX 5060 Ti + 4060 Ti, 27B denso Q4 | **1 GbE, RTT 0,29 ms** | ~23 tok/s prosa / ~35 código; **la red NO es el cuello** (~10 KB por token) |
| Mismo par, 35B-A3B (MoE) | 1 GbE | 51,7 tok/s prosa · 62,2 código · 91–115 con MTP |
| 2×3090 + 4090 + 5090, GLM-4.6 IQ3_XXS | ~500 Mbps | 1 máquina **5 tok/s** → RPC **2 tok/s** |
| GLM-4.5-Air, todo en VRAM vía RPC | LAN | **53 tok/s** frente a 17 con offload a RAM en una sola máquina |

**Regla que se deduce de los datos:** *RPC es para capacidad, no para velocidad.* Ganas cuando el modelo
**no cabe** en un nodo y la alternativa es tirar de RAM; pierdes siempre que el modelo ya cabía.

**Y con tu hardware concreto:**

| Tu caso | ¿Repartir? | Por qué |
|---|---|---|
| Modelo que cabe en la 5060 Ti (16 GB) | **No** | Perderías entre 40 % y 50 % del decode |
| Modelo que no cabe en ninguna de tus dos GPU pero sí sumando 12+16 | **No** | Ya no hace falta red: `split` entre GPU0/GPU1 en la misma máquina es más rápido que por Ethernet |
| Modelo que no cabe en el master ni en el worker pero sí en ambos | **Solo medido y con MoE** | El único caso con cifras decentes a 1 GbE es MoE con pocos parámetros activos |
| **Tareas independientes simultáneas** (LLM + STT + TTS + tests) | **Sí, sin dudarlo** | Paralelismo real; la red solo lleva texto |

**Herramientas descartadas con motivo:**

- **exo**: el más maduro, pero GPU en Linux aún en desarrollo, **sin soporte Windows**, y su caso bueno
  es un clúster homogéneo de Macs con RDMA por Thunderbolt 5. No es tu escenario.
- **Petals**: sin mantenimiento desde 2024, Windows solo vía WSL, y su propio README declara **≤6 tok/s**
  para 70B. Es para swarm por internet, no para LAN.
- **distributed-llama**: vivo y con soporte nativo de Windows, pero **solo 2ⁿ nodos**, cuantización **q40**
  y tensor parallelism (más exigente en red). Sin tabla de rendimiento publicada.
- **llama.cpp RPC**: existe, reparte capas, pero sus autores lo etiquetan **proof-of-concept, "fragile and
  insecure"**, sin versionado de protocolo. Se puede probar; no se puede poner en producción a ciegas.

---

## 2. EL SISTEMA DE NODOS AGNÓSTICO DE GPU (el corazón del encargo)

El requisito «que la GPU pueda sustituirse sin reconstruir la arquitectura» se resuelve con **una huella
de hardware** y **capacidades derivadas**, nunca con nombres de modelo cableados.

### 2.1 Lo que reporta cada nodo (`/inventory`, solo lectura)

| Campo | Fuente | Por qué importa |
|---|---|---|
| `cpu.model`, `cores`, `threads`, `ram_total_mb` | `sysinfo` (ya en `Cargo.toml`) | Presupuesto de trabajos sin GPU |
| `gpus[].name`, `vram_mb` | `nvidia-smi --query-gpu` | Encaje del modelo |
| `gpus[].compute_cap` | `nvidia-smi --query-gpu=compute_cap` | **Decide FP8/FP4/INT8** |
| `gpus[].driver_version` | `nvidia-smi` | Un cambio de driver invalida mediciones |
| `gpus[].pcie_gen`, `pcie_width` | `nvidia-smi --query-gpu=pcie.link.gen.current,pcie.link.width.current` | Detecta cuellos locales |
| `engines[]` | Ollama `/api/version`+`/api/tags`, LM Studio `/api/v1` y `/api/v0/models` | Qué puede ejecutar sin instalar nada |
| `models[]` | `ollama list` + `/api/v0/models` (`quantization`, `max_context_length`, `state`) | Catálogo real, no supuesto |
| `disk[k].free_gb` | `sysinfo` | Un nodo sin disco no puede cachear modelos |

**Medido hoy en el master, como referencia de que estos campos existen de verdad:**

```
gpu0  RTX 5070      12.227 MiB  compute_cap 12.0  driver 621.01
gpu1  RTX 5060 Ti   16.311 MiB  compute_cap 12.0  driver 621.01
```

### 2.2 Huella de hardware: la pieza que permite cambiar la GPU sin tocar código

```
hw_fingerprint = sha256( name | vram_mb | compute_cap | driver_major )
```

- Si cambias la RTX 2070 Super por la de 22 GB, **la huella cambia**.
- El master detecta huella nueva → **invalida los `fits()` cacheados y archiva los benchmarks** de la
  GPU anterior (no los borra) → pide al nodo un `bench` de referencia → actualiza el perfil.
- **Cero migraciones de esquema, cero cambios de código, cero reinstalación.** El nodo se «re-registra»
  solo la próxima vez que late.

### 2.3 Dos niveles de verdad: predicho vs medido

| Nivel | De dónde sale | Qué decide |
|---|---|---|
| **Encaje (predicho)** | VRAM libre × 0,88 (margen 12 %, ya implementado en `vram::fits`) | Si el trabajo *puede* enviarse |
| **Rendimiento (medido)** | `benchmarks` (tabla ya existente) por (huella, modelo, quant, ctx) | **A qué nodo se envía** |

Regla dura: si no hay benchmark para ese par (nodo, modelo), el router lo marca **`sin medir`** y no
puede presentarlo como recomendación. Es la diferencia entre «creo que irá bien» y «va a 23 tok/s».

---

## 3. MATRIZ DE COMPATIBILIDAD POR VRAM

Cálculos con bits por peso nominales de GGUF (`Q4_K_M ≈ 4,85`, `Q5_K_M ≈ 5,67`, `Q6_K ≈ 6,56`,
`Q8_0 ≈ 8,5`, FP16 = 16, NVFP4 ≈ 4,5). **Tamaño real del GGUF varía ±5 %.** KV en `f16` salvo que se diga.

| Modelo | Q4_K_M | Q5_K_M | Q8_0 | ¿8 GB? | ¿12 GB? | ¿16 GB? | ¿22 GB? |
|---|---:|---:|---:|---|---|---|---|
| 8B denso | 4,9 GB | 5,7 | 8,5 | **sí** (+8k KV = 6,1) | sí | sí | sí |
| 14B denso | 8,5 | 9,9 | 14,9 | **no** | **sí** (+8k = 9,8) | sí | sí |
| 24B denso | 14,6 | 17,0 | 25,5 | no | no | **al límite** (+8k = 15,8) | sí |
| 30B MoE (A3B) | 18,2 | 21,3 | 31,9 | no | no | **no** | **sí** (+32k KV q8 ≈ 1,5 → 19,7) |
| 32B denso | 19,4 | 22,7 | 34,0 | no | no | no | **al límite** (+8k KV q8 ≈ 1,0 → 20,4) |
| 40B denso | 24,3 (Q3≈20) | — | — | no | no | no | **no** (Q3 20 GB + KV no cabe) |
| 70B denso | 42 | — | — | no | no | no | **no** |

**El dato incómodo de la columna de 22 GB: la caché KV se come el margen.** Un 32B denso con 64 capas y
8 cabezas KV gasta **256 KB por token** en f16: 8k de contexto = 1 GB, 32k = 8,4 GB. Con 22 GB eso obliga
a **KV cuantizado** (`q8_0` → mitad, `q4_0` → cuarta parte) en cuanto pases de 8k de contexto.

Estados usados en la matriz: **sí** = predicho que cabe con margen · **al límite** = cabe sin contexto
largo o con KV cuantizado · **no** = no cabe.

**Nada de esta tabla está medido en el worker**: es cálculo. Se convierte en medición en el paso M0-B.

---

## 4. LA FUTURA GPU DE 22 GB: QUÉ CAMBIA Y QUÉ NO

No se conoce el modelo. El diseño **no lo necesita**; solo necesita tres ramas:

| Rama | Si la GPU resulta ser… | Consecuencia |
|---|---|---|
| **A** | Turing o Ampere (p. ej. una 2080 Ti de 22 GB o una A5000) | INT8/INT4 por GGUF: bien. **Sin FP8 ni FP4 nativos.** El nodo no podrá usar NVFP4 nunca |
| **B** | Ada (compute 8.9) | FP8 nativo. NVFP4 no |
| **C** | Blackwell (compute 12.x) | FP8 + NVFP4 nativo, como tus dos GPU actuales |

**Qué NO cambia con la GPU nueva:** ni el esquema de la base, ni el protocolo del nodo, ni la cola, ni
la seguridad, ni la UI. Solo cambian tres números del perfil (VRAM, compute, driver) y los benchmarks.

**Qué se gana realmente al pasar de 8 a 22 GB** (el salto que importa, no el número):

| Antes (8 GB) | Después (22 GB) |
|---|---|
| 7–8B Q4 a 8k de contexto | 30B MoE Q4 a 32k, o 32B denso Q4 a 8k |
| Whisper `small`, TTS pequeño, embeddings | Whisper `large`, TTS grande, visión con soltura |
| Nada de 24B+ | 24B Q5 sin apuros |
| **Sigue sin caber** | 40B, 70B y vídeo: **no** |

**Aviso para cuando llegue:** no se instalan drivers ni se toca CUDA hasta identificar el modelo exacto y
que tú lo apruebes. Y si la GPU es de 22 GB por modificación de VRAM (no de fábrica), el `compute_cap` y
el driver deben verificarse **antes** de fiarse de ella para trabajo nocturno.

---

## 5. ASIGNACIÓN DE TAREAS POR CAPACIDAD MEDIDA

| Trabajo | Nodo preferente | Regla |
|---|---|---|
| `llm.chat` grande | **Master** (5060 Ti 16 GB) | El worker nunca recibe modelos >16 GB |
| `llm.chat` 7–8B | El que tenga mejor tok/s medido | Con 22 GB, el worker pasa a ser candidato |
| `embed` | Worker | Barato, constante, no compite con el chat |
| `stt` (Whisper) | Worker | `large` solo si la GPU nueva es rama B/C |
| `tts` | Worker | Voz clonada allí; el master se queda libre |
| `test.run`, `build` | **Worker** | No usa GPU: el mejor uso de un i7-12700 |
| `vision.light` | Worker | 8 GB justos; con 22 GB, sobrado |
| `bench` | Worker | Mediciones limpias, sin ensuciar el master |
| `video`, `flux`, `sdxl` | **Solo master** | Ni 8 ni 22 GB son suficiente para vídeo cómodo |

**Presupuesto por nodo:** el master reserva siempre **2 GB de VRAM** para pantalla y escritorio (medido
hoy: la 5070 usa 11,8 GB de 12,2 con el escritorio y ComfyUI; sin reserva, cualquier trabajo la ahoga).

---

## 6. REQUISITOS DE RED (con los números de §1)

| Requisito | Valor | Motivo |
|---|---|---|
| Enlace del master | **Ethernet 1 GbE como mínimo** | Hoy está en Wi-Fi 136 Mbps y el cable desconectado |
| Enlace del worker | 1 GbE | Idem |
| IP fija o reserva DHCP | Sí, para ambos | Un nodo que cambia de IP rompe la lista manual |
| Puertos | **Uno solo** (p. ej. 8791) en la subred, con regla de firewall | Menos superficie |
| Latencia exigida | **< 2 ms RTT** en LAN | Por encima, el pipeline por capas se degrada |
| MTU | 1500 estándar | No hace falta jumbo frames con cargas de ~10 KB |
| Internet | **No necesario** | El diseño funciona en LAN aislada |

**Alternativa ya instalada:** tienes **Tailscale** (`100.116.82.33`). Sirve para conectar el segundo PC
sin abrir puertos y con cifrado de extremo a extremo, y es útil si el worker acaba en otra casa o por
4G. Pero para dos equipos en la misma mesa, **LAN directa + firewall por subred es más rápido y más
simple**. Tailscale como respaldo, no como camino principal (si cae en relay DERP, la latencia se dispara).

---

## 7. SEGURIDAD: LO QUE SE AÑADE AL DISEÑO ANTERIOR

Se mantiene íntegro el capítulo 4 de [ARQUITECTURA_MULTI_PC.md](ARQUITECTURA_MULTI_PC.md) (token por
nodo, HMAC, bind a subred, auditoría, `policy` que solo se puede reducir). Añado cuatro cosas que la
GPU nueva y el paso del tiempo hacen necesarias:

1. **Versionado de protocolo explícito.** Cada petición lleva `proto: 1`. El nodo rechaza versiones que
   no conoce con un error claro. (Lección directa del fallo de llama.cpp RPC: builds distintos fallan
   en silencio.)
2. **Rotación de token sin cortar el servicio:** dos tokens válidos a la vez durante la rotación, y
   `audit` registra qué token se usó (por huella, no por valor).
3. **El `compute_cap` no autoriza nada.** Que un nodo reporte Blackwell no le da permiso para recibir
   modelos mayores: el límite lo pone la política del master.
4. **Sin shell remoto, jamás.** El worker solo acepta `kind` de una lista cerrada. `test.run` ejecuta
   **comandos predefinidos** (cargo test, npm test), no cadenas arbitrarias que lleguen por la red.

---

## 8. PLAN DE INCORPORACIÓN DEL SEGUNDO PC (sin instalar nada todavía)

| Paso | Qué se hace | Criterio de aceptación | Toca el 2º PC |
|---|---|---|---|
| **M0-A** | Cablear el master a Ethernet y medir con `iperf3` | ≥ 900 Mbps sostenidos, RTT < 2 ms | No |
| **M0-B** | Inventario de lectura del worker (cuando esté encendido) | JSON con CPU/RAM/GPU/VRAM/compute/driver/modelos | Solo lectura |
| **M1** | `kukia-worker.exe` con `/health` y `/inventory` | `curl` desde el master devuelve su JSON real | Instala binario propio |
| **M2** | Token + HMAC + regla de firewall | Sin token → 401; token malo → 401; otra subred → timeout | Config de firewall |
| **M3** | Cola con `worker_id` en `tasks` | Un `stt` termina en el worker y el resultado aparece en el master | Nada |
| **M4** | Cancelación y recuperación | Matar el worker a mitad → `interrupted` en < 30 s, sin trabajo huérfano | Nada |
| **M5** | Vista MULTI-PC Center | CPU/GPU/VRAM/temperatura de ambos + cola + histórico | Nada |
| **M6** | Caso real de punta a punta | Whisper o TTS desde la UI, con tok/s o segundos medidos | Nada |

**Nada de esto se ejecuta hasta que autorices, y el paso M0-B es de solo lectura.**

---

## 9. PLAN DE SUSTITUCIÓN 2070 Super (8 GB) → 22 GB

Procedimiento que **no requiere rediseñar nada**, en este orden:

1. **Identificar** el modelo exacto: `nvidia-smi --query-gpu=name,memory.total,compute_cap,driver_version`.
   Sin este dato no se instala ni un driver.
2. **Comprobar la rama** (§4): Turing/Ampere/Ada/Blackwell. De eso depende qué cuantizaciones son nativas.
3. **Registrar el antes**: guardar el perfil de la 2070 Super en `backups/` con su `hw_fingerprint`.
4. **Instalar driver** solo si hace falta, con punto de restauración y backup previo.
5. **Arrancar el nodo**: detecta huella nueva y se re-registra solo.
6. **Medir** el `bench` de referencia con 2 modelos (uno que quepa en 8 GB y otro que solo quepa en 22).
7. **Actualizar la matriz** con datos medidos, sustituyendo los «predicho» por «medido».
8. **Reasignar tareas** (§5): a partir de aquí el worker puede recibir 30B MoE.

**Reversión:** volver a poner la 2070 Super devuelve la huella anterior; los benchmarks antiguos siguen
archivados y el router vuelve a ellos. Nada que deshacer a mano.

---

## 10. PRUEBAS DE RENDIMIENTO (con umbral, no con opiniones)

| # | Prueba | Métrica | Umbral para dar por bueno |
|---|---|---|---|
| **P1** | Despacho de 20 trabajos vacíos | latencia p95 submit→ack | < 50 ms en LAN cableada |
| **P2** | STT de 60 s de audio | segundos, pico de VRAM | worker ≤ 1,3× el master |
| **P3** | TTS de 500 palabras | segundos, calidad subjetiva 1–5 | ≤ 1,2× el master y calidad ≥ 3 |
| **P4** | LLM en master **mientras** el worker hace STT | tok/s vs LLM solo | **≥ 85 %** del rendimiento en solitario |
| **P5** | `llama.cpp RPC`, 2 nodos, 27B Q4 | tok/s vs master solo | Se **documenta**; entra en producción **solo** si convierte offload de RAM en VRAM |
| **P6** | Inventario | VRAM reportada vs `nvidia-smi` | Diferencia **0 MiB** |
| **P7** | Corte de red a mitad de trabajo | tiempo a `interrupted` | < 30 s, sin duplicar el trabajo al reconectar |

**P5 es la prueba que decide la leyenda.** Los datos de §1 dicen que perderá; si pierde, se escribe y se
descarta. Si gana con un MoE concreto, se documenta el caso exacto.

---

## 11. PLAN DE RECUPERACIÓN

| Fallo | Detección | Respuesta |
|---|---|---|
| Worker apagado | 3 heartbeats perdidos (≈15 s) | Nodo `offline`; trabajos pendientes vuelven a la cola local |
| Worker cae a mitad | Timeout del trabajo | `interrupted` + reintento con `deadline` nuevo; nunca duplicado |
| Red caída | Error de transporte | Reintento con backoff; tras N intentos, degradar a local |
| Master reiniciado | Arranque | `tasks_interrupted` (ya existe) reanuda la cola |
| GPU sustituida | Cambia `hw_fingerprint` | Re-medir; benchmarks viejos archivados |
| Driver actualizado | Cambia `driver_version` | Invalida mediciones de esa GPU (no las borra) |
| Token comprometido | — | Rotación doble + revocación + revisar `audit` |
| Base de datos dañada | `integrity_check` al arrancar | Restaurar backup verificado; el worker no guarda estado crítico |

---

## 12. RIESGOS

| Riesgo | Nivel | Mitigación |
|---|---|---|
| Comprar la GPU de 22 GB sin saber si es Turing modificada | **Alto** | Comprobar `compute_cap` real al recibirla; si es Turing, no habrá FP4 |
| Diseñar para «sumar VRAM» y decepcionarse | **Alto** | Este documento: no se suma; solo capacidad o tareas |
| Wi-Fi como enlace del multi-PC | **Alto** | Cablear antes de M1 |
| Repartir modelos «porque se puede» | Medio | P5 con umbral; si empeora, no entra |
| Worker con 8 GB engañosos | Medio | `fits()` con 12 % de margen antes de enviar |
| Protocolo sin versionar | Medio | `proto: 1` desde el primer día |
| El nodo se convierte en puerta trasera | **Alto** | `kind` cerrado, sin shell arbitrario, permisos en el master |
| Disco: H: con 98,5 GB libres y dos builds | Medio | `CARGO_TARGET_DIR` fuera de H: para el worker |

---

## 13. LO QUE NO SE HARÁ SIN AUTORIZACIÓN EXPRESA

- Instalar nada en el segundo PC.
- Tocar drivers, CUDA o BIOS del master.
- Abrir puertos en el router.
- Mover modelos entre discos.
- Cablear o reconfigurar la red (aunque recomendarlo sí).
- Asumir el modelo de la futura GPU.

---

## 14. RESUMEN EN SEIS LÍNEAS

1. **Estás en Wi-Fi a 136 Mbps con el cable desconectado**: eso es lo primero que hay que arreglar.
2. **No se suma VRAM entre PC.** Medido: partir un modelo que ya cabía **empeora** el decode un 43–47 %.
3. **Sí funciona el reparto de tareas independientes**, y eso es el 90 % del valor real.
4. **La 22 GB no da para 70B**: da para 30B MoE a 32k o 32B denso a 8k, con KV cuantizado.
5. El diseño **no depende del modelo de GPU**: huella de hardware + capacidades medidas, y a correr.
6. **Antes de M1 hay que cablear y medir con `iperf3`.** Sin eso, todo lo demás son cuentas.
