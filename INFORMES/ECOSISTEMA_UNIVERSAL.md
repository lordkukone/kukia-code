# KUKIA UNIVERSAL ECOSYSTEM — registro de sistemas

Fase añadida el 02/11/2026 sobre el puente tridireccional ya entregado
(ver `INFORMES/PUENTE_TRIDIRECCIONAL.md`).

## Qué se añadió

| Archivo | Qué hace |
|---|---|
| `core/src/registry.rs` (nuevo) | 18 sistemas declarados y detección genérica por **proceso + puerto + sonda HTTP + ruta**. Sin lógica por programa: añadir uno es añadir una fila a `SPECS`. |
| `app/src/views/Universal.tsx` (nuevo) | Pantalla «KUKIA UNIVERSAL ECOSYSTEM»: estados, capacidades y límites reales por sistema, con filtros. |
| `src-tauri/src/lib.rs` | Comando `ecosistema_universal` (sondeo en paralelo, sin arrancar ni modificar nada). |
| `core/tests/registro_universal.rs` (nuevo) | Detección real en esta máquina + coherencia de estados. |

## Estados (no se saltan)

`instalado` (ruta o proceso) → `detectado` (además puerto abierto) → `conectado` (su API
contesta a la sonda) → `operativo` (conectado y con versión leída).

## Inventario medido

`sistemas=18 · instalados=10 · conectados=1 · operativos=1`

| Sistema | Estado real medido |
|---|---|
| KUKIA CORE | instalado · v0.1.0 · su `/v1` parado |
| OpenCode | instalado · 1 proceso · sin servidor |
| Freebuff | instalado · 6 procesos |
| n8n **2.29.11** | instalado (`~/.n8n`, 0 workflows, 0 ejecuciones) · sin arrancar |
| Ollama **0.40.13** | **operativo** · 3 procesos · API responde en 11434 |
| LM Studio | instalado · parado |
| ComfyUI / Wan2GP / Fish Speech | instalados en `I:\pinokio` · parados |
| Pinokio | instalado en `I:\` · parado |
| Dify, Flowise, AnythingLLM, Open WebUI, Maestro, RVC, Qwen3-TTS, LTX | **no detectados** |

## Tres falsos positivos que la prueba destapó y se corrigieron

1. **KUKIA no se detectaba a sí misma** (no tiene ruta declarada) → caso propio con su versión.
2. **`python.exe` contaba como ComfyUI: 15 procesos falsos.** Un intérprete compartido no prueba
   nada; ahora manda el puerto.
3. **`J:\models\video` hacía creer que LTX estaba instalado.** Es un almacén de modelos, no una
   instalación.

## Lo que NO está implementado de la lista OMEGA (y por qué)

- **Ejecución de workflows n8n**: hace falta su API con credenciales y está parado (0 workflows).
  Solo se detecta y se lee su base en solo lectura.
- **Dify / Flowise / AnythingLLM / Open WebUI**: no están instalados en este equipo. No hay
  integración que probar y no se ha escrito una fachada vacía.
- **RVC / Qwen3-TTS / LTX / Maestro**: instalación o API no verificadas aquí.
- **Agent Swarm, Plugin Hub, Self-Healing, Code Factory, Benchmark Arena ampliado**: no se han
  tocado en esta fase; el registro universal es su base. Sin detección real, cualquiera de esos
  sería decorado.

## Pruebas de esta fase

```
cargo test --lib                     → 149 passed, 0 failed, 1 ignored
cargo test --test registro_universal → 1 passed (18 sistemas, estados coherentes)
cargo check -p kukia-code            → exit 0
npm run build                        → 59 módulos, 420 kB, exit 0
npm run tauri:build                  → release en ~5 min
```

Ejecutable: `target/release/kukia-code.exe` (12.231.680 bytes, 11:47).
Instalador: `target/release/bundle/nsis/KUKIA CODE_0.1.0_x64-setup.exe` (3,37 MiB).
