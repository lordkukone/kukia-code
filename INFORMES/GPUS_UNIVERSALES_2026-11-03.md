# GPUs universales: NVIDIA, AMD e Intel (03/11/2026)

## El problema

Todo el detalle de tarjetas salía de `nvidia-smi` en cinco ficheros. En un equipo
con AMD o Intel no hay `nvidia-smi`, así que la lista de GPU volvía vacía y con ella
se caía todo lo que depende de ella: el panel de VRAM, la guarda de «¿cabe este
modelo?», el reparto por GPU, la temperatura y la potencia. La app arrancaba, pero a
ciegas.

Además había supuestos clavados a esta máquina:

- Los prompts de dos agentes decían «conoces las dos GPUs del equipo: RTX 5070 12 GB
  y RTX 5060 Ti 16 GB».
- Una prueba exigía encontrar una «5070» y una «5060»: en cualquier otro equipo
  fallaba, aunque el código estuviera bien.
- El radar de gaming guarda columnas `rtx5070` / `rtx5060ti` (esto **sigue igual**,
  ver «Lo que no se ha tocado»).

## La solución

Módulo nuevo `core/src/gpus.rs`, sin dependencias nuevas (crates.io responde 403 en
esta máquina, así que se habla con Windows por FFI directa: `dxgi.dll` y `pdh.dll`).

| Dato | Fuente | Fabricantes |
|---|---|---|
| Lista, nombre, VRAM total | DXGI (`EnumAdapters1` + `GetDesc1`) | todos |
| Memoria en uso | `\GPU Adapter Memory(*)\Dedicated Usage` (PDH) | todos |
| Temperatura, potencia, relojes, utilización | `nvidia-smi` | solo NVIDIA |

- Las cifras de uso se emparejan con cada tarjeta por **LUID**, que viaja en el
  nombre de la instancia del contador (`luid_0x00000000_0x0001941d_phys_0`).
- Una iGPU (Intel, APU AMD) no tiene VRAM dedicada: se informa su memoria compartida
  y se marca `compartida`, en vez de un 0 que no sirve para nada.
- Donde el fabricante no publica un dato, el campo queda en 0 y `fuente` lo dice. No
  se rellena con inventos.
- `gpu.rs` y `vram.rs` ahora **componen** sobre esa capa: la lista sale de DXGI y lo
  de NVIDIA se prefiere por ser más preciso (distingue la memoria reservada).

## Medido en esta máquina (03/11/2026)

```
GPU0 NVIDIA GeForce RTX 5070    [nvidia] luid 00000000:0001941d  11943 MB · uso 2829 MB
GPU1 NVIDIA GeForce RTX 5060 Ti [nvidia] luid 00000000:0001c6b0  16050 MB · uso    5 MB

nvidia-smi: 0 = 12227 MiB totales / 2808 MiB usados
            1 = 16311 MiB totales /    0 MiB usados
```

Coinciden. La diferencia de total (11943 frente a 12227 MB) es real y esperada: DXGI
informa la memoria dedicada y `nvidia-smi` incluye la reservada por el driver.

## Fallo encontrado y corregido durante la implementación

DXGI enumeraba **tres** adaptadores: la 5070 aparecía dos veces. La copia tenía LUID
distinto (`00028f50`) pero no existía en los contadores de memoria de Windows: la
crea el driver del monitor virtual. Si se hubiera dejado, la interfaz habría pintado
una GPU fantasma con 0 MB de uso y la guarda de VRAM habría contado una tarjeta que
no puede ejecutar nada.

Se descarta por dos criterios, y queda como prueba automática:

1. LUID repetido → es la misma tarjeta.
2. Mismo nombre y misma VRAM que otra que **sí** tiene contador, y sin contador
   propio → duplicado fantasma. Dos tarjetas físicas iguales aparecen las dos en los
   contadores, así que siguen contando como dos.

## Pruebas ejecutadas

- `cargo test -p kukia-core --lib`: **181 pasan, 0 fallan, 1 ignorada** (antes 174).
  Nuevas: lectura de LUID, identificación de fabricante, normalización de nombres,
  coherencia de la lista, ausencia de duplicados y `nvidia-smi` sin reventar.
- `cargo check -p kukia-core --all-targets`: 0 errores, 0 avisos.

## Lo que no se ha podido probar

- **No hay ninguna tarjeta AMD ni Intel en esta máquina.** El camino DXGI + PDH es
  el estándar de Windows y se ha validado aquí con NVIDIA, pero no puedo afirmar
  «funciona en una RX 7800» sin una delante. Lo que sí está probado es que la
  ausencia de `nvidia-smi` ya no deja la app sin tarjetas ni sin memoria.
- Temperatura, potencia y relojes en AMD/Intel: no hay vía implementada todavía
  (requiere ADL o contadores por motor). Se informan en 0 y `fuente` lo explica.

## Lo que no se ha tocado (a propósito)

- El radar de gaming mantiene sus columnas `rtx5070` / `rtx5060ti` en la base de
  datos. Renombrarlas exige una migración de esquema y no aporta nada al objetivo de
  hoy; queda anotado como deuda.
- `gaming.rs` sigue comparando esas dos tarjetas concretas: es una función de ese
  radar, no del núcleo de inferencia.

## Binarios de esta entrega

Compilados el 03/11/2026 a las 13:35, despues del cambio:

- `kukia-code.exe`: 12.732.416 bytes (antes del cambio: 12.715.008).
- Instalador NSIS: 3.647.391 bytes (antes: 3.638.609).
- `npx tauri build` completo en 3m38s, sin errores.
