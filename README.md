# KUKIA CODE

Asistente de IA de escritorio para Windows 11: **una aplicación propia** que
habla con los modelos que ya tienes instalados en tu máquina, les da
herramientas para trabajar y no manda nada a ningún sitio si tú no lo pides.

Interfaz en **React + TypeScript** sobre **Tauri 2**; núcleo en **Rust**
(`core/`), independiente de la interfaz. Memoria persistente en **SQLite**.

> Copyright © 2026 Xoxete (GitHub: [@lordkukone](https://github.com/lordkukone)).
> **Todos los derechos reservados.** Ver [`LICENSE`](LICENSE).
> La publicación del código no cede ningún derecho: se permite consultarlo y
> estudiarlo, no copiarlo, modificarlo ni redistribuirlo.

---

## Qué hace

### Un agente, no un chat que solo habla

Los modelos que declaran la capacidad `tools` (en la máquina de desarrollo, 60
de 83) reciben el esquema de las herramientas de su agente, las piden, KUKIA las
ejecuta bajo permisos y el resultado vuelve al modelo hasta que contesta.
Máximo 6 pasos por turno, y cada llamada queda auditada.

**35 herramientas en cinco niveles:**

| Nivel | Permiso | Aprobación | Ejemplos |
|---|---|---|---|
| Lectura (18) | `allow_read` | no | leer, listar, buscar por nombre (`*`, `?`), buscar dentro de archivos, árbol, memoria, logs, GPU, sistema, discos, modelos cargados, VRAM |
| Escritura (8) | `allow_write` | **sí** | escribir, añadir, reemplazar (solo si el texto es único), copiar, mover, borrar, crear carpeta, guardar recuerdo |
| Ejecución (6) | `allow_shell` | **sí** | PowerShell (60 s), `git status/diff/log`, procesos, descargar de VRAM |
| Red (2) | `allow_network` | **sí** | descargar https, POST JSON |
| MCP (1) | `allow_tools` | **sí** | llamar a una herramienta de un servidor MCP **autorizado** |

Nada que modifique se ofrece siquiera si no hay aprobación explícita: el modelo
no puede pedir lo que no ve. Todo pasa por una política de permisos con
allowlist de rutas y queda en la tabla `audit`.

### Contexto máximo real, medido

No se pide un número fijo: para cada modelo se calcula el mayor contexto que
cabe de verdad,

```
máximo = min(nativo, (VRAM − margen − pesos) ÷ coste por token)
```

con la caché KV calibrada contra mediciones reales (57,2 KB/token, corrección
×1,45 sobre la fórmula teórica). Si el modelo agotó el presupuesto pensando y no
llegó a responder, se reintenta con más margen en vez de dejar la burbuja vacía.

### Catálogo universal de modelos

Enumera lo que hay en Ollama, LM Studio, el proveedor cloud activo y ComfyUI,
clasifica por capacidad (texto, visión, código, herramientas, embeddings,
imagen) y enruta cada necesidad al modelo que mejor la cubre. El cloud **no** se
usa si no se autoriza. Se puede fijar un modelo por ámbito a mano.

### Lo demás

- Memoria persistente con búsqueda.
- Panel de VRAM con guarda antes de cargar y descarga selectiva.
- Cliente **MCP** (stdio y http) con autorización servidor a servidor.
- Servidor local compatible OpenAI (`/v1`) **solo en loopback** y con token,
  para conectar OpenCode u otros clientes.
- Auditoría de cada herramienta, comando y escritura.
- Radar de versiones y de investigación (arXiv, Hugging Face, GitHub).

## Requisitos

- Windows 11, Node.js 20+, Rust estable, Visual Studio Build Tools (C++).
- Ollama y/o LM Studio locales (opcionales: la app arranca sin ellos).

## Arranque

```powershell
npm install
npm run tauri:dev     # desarrollo
npm run tauri:build   # instalador NSIS en target/release/bundle/nsis
```

Pruebas del núcleo:

```powershell
cargo test -p kukia-core --lib
cargo test -p kukia-core --test validacion_herramientas_chat -- --ignored --nocapture
```

## Estructura

```
core/src/          núcleo Rust: proveedores, catálogo, memoria, permisos,
                   herramientas del chat, VRAM, MCP, radar, GPU
core/tests/        pruebas de integración (incluidas las que hablan con Ollama)
src-tauri/src/     comandos que expone la app
app/src/           interfaz React + TypeScript
docs/              arquitectura, manual y hoja de ruta
INFORMES/          informes de cada fase, con lo probado y lo pendiente
```

## Estado

MVP funcional en uso diario. Regla del proyecto: **nada se declara terminado sin
una prueba ejecutada**, y lo que no se puede comprobar se marca como no
comprobado en vez de darlo por bueno.

Nota de transparencia: el 03/11/2026 dos módulos (`core/src/catalog.rs` y
`core/src/serve.rs`) se perdieron por un error de herramientas y se
reconstruyeron desde su contrato (tipos de la interfaz, usos reales, pruebas
originales y literales del binario). Funcionan y están probados, pero su
implementación interna no es la original.

## Autoría

Diseño, código y documentación: **Xoxete** ([@lordkukone](https://github.com/lordkukone)).
© 2026. Todos los derechos reservados. Ver [`LICENSE`](LICENSE).
