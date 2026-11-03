# Reparación de OpenCode + recuperación de Xoxete + integración en KUKIA CODE

Fecha: 2026-11-02 · Máquina: i9-10900K · 64 GB · RTX 5070 12 GB · RTX 5060 Ti 16 GB
Ámbito tocado: `H:\Xoxete`, `H:\KUKIA_CODE`. **No se ha tocado** `C:\` (salvo leer), `I:\CEREBRO`, KUKIA Network, Ollama, LM Studio, ComfyUI ni drivers.

---

## 1. Qué estaba roto (con causa real, reproducida)

### 1.1 El lanzador apuntaba a un ejecutable que ya no existe

`H:\Xoxete\Abrir-Xoxete.cmd` lanzaba:

```
C:\Users\kuki\AppData\Local\Programs\@opencode-aidesktop\OpenCode.exe
```

Esa carpeta **no existe**. La app real está en:

```
C:\XOXETE_GENERAL\05_APLICACION\SUPER_2.2.5\OpenCode.exe   (246.614.920 bytes, 30/10/2026)
```

Conclusión: doble clic en el lanzador habitual = nada. No era «OpenCode roto», era un acceso directo muerto.

### 1.2 El puerto 28894 estaba secuestrado por un servicio huérfano

La app 2.2.5 arranca su propio servidor (`opencode-cli.exe serve --service`) en `127.0.0.1:28894`. Al pedirle trabajo, la respuesta textual era:

```
Error: Managed service port 28894 on 127.0.0.1 is already in use by another process.
       Configure another port with `opencode service set port <port>` and start the service again.
```

El puerto lo ocupaba un `serve --service` **sin proceso padre vivo** (huérfano de una ejecución anterior). Efecto visible: la ventana no podía levantar su servicio y no había forma de crear chats.

### 1.3 Los plugins de Xoxete no cargaban (y uno hacía trabajo de más)

Contrato real, extraído del binario (`strings` sobre `opencode.exe` 1.19.3):

- 1.19.3: `Plugin … must default export an object with server()`
- 2.2.5: `Plugin must export a default definition with an id and an effect or setup function`

Y, además, 1.19.3 **invoca como plugin toda función exportada del fichero**. De ahí venían los errores del registro:

| Fichero | Error real en 1.19.3 | Error real en 2.2.5 |
|---|---|---|
| `xoxete-router.ts` | `(s \|\| "").toLowerCase is not a function` — se llamaba `recomendar()` con el objeto de contexto | no cargaba |
| `xoxete-web.ts` | `buscar()` se ejecutaba **como plugin** al arrancar: una búsqueda real en Internet en cada inicio | no cargaba |
| `xoxete-learning.ts` | `must default export an object with server()` | no cargaba |
| `security-agent.ts` | cargaba (export con nombre) | `Plugin must export a default definition with an id and an effect or setup function` |

Consecuencia directa: en el CLI, **la memoria automática de Xoxete estaba desconectada** aunque el agente respondiera.

### 1.4 La 2.2.5 no termina de activar plugins si hay ficheros en `opencode\plugin`

Síntoma en la interfaz: sesiones visibles pero **sin modelos ni agentes**, y el botón Enviar sin efecto. La API devolvía, tras 5 s exactos:

```
503 ServiceUnavailableError · "Location plugins did not finish activating" · service: plugin
```

Bisección hecha (servicio oficial reiniciado en cada paso, sondeando `/api/provider`):

| Estado de `xdg-config\opencode\plugin` | `/api/provider` |
|---|---|
| Sin carpeta plugin | **200 en 5–8 ms** |
| Un plugin trivial (`export default async () => ({})`) | 503 tras 5.005 ms |
| Un plugin trivial `{ id, effect }` | 503 tras 5.006 ms |
| `xoxete-learning.ts` solo, `{ id, effect }` | 503 tras 5.006 ms |
| `xoxete-learning.ts` solo, `{ id, server, effect, setup }` | 503 tras 5.005 ms |

Es decir: **no es culpa del contenido de los plugins**; con cualquier fichero en esa carpeta, la activación de plugins de la 2.2.5 no termina. Aviso de honestidad: en una prueba con el plugin trivial en un XDG de `C:` la misma petición devolvió 200 una vez, así que el fallo se comporta de forma **inestable**; lo reproducible al 100 % es lo contrario (carpeta sin plugins → 200 siempre).

### 1.5 El CLI se quedaba colgado esperando entrada

`opencode run "…"` lanzado desde un shell con stdin no interactivo se quedaba 180 s tras `init`. Con `< /dev/null` responde siempre. Es el clásico «se queda pensando» del CLI, no un fallo del modelo.

---

## 2. Qué se ha reparado

### 2.1 Lanzador (backup + reescritura)

`H:\Xoxete\Abrir-Xoxete.cmd` ahora apunta al ejecutable real y usa un XDG propio sin carpeta de plugins:

```
XDG_CONFIG_HOME=H:\Xoxete\xdg-config-app
XDG_DATA_HOME=H:\Xoxete\xdg-data        (mismas sesiones que el CLI)
start "" "C:\XOXETE_GENERAL\05_APLICACION\SUPER_2.2.5\OpenCode.exe"
```

Backup del original: `backups\reparacion_opencode_20261102_182308\Abrir-Xoxete.cmd.antes`.

### 2.2 Servicio huérfano

Cerrado **con el mecanismo oficial** (`opencode-cli.exe service stop`), no a mano: el puerto quedó libre y el servicio se puede arrancar limpio. Después de las pruebas lo he dejado **parado y el puerto libre**, para que lo levante la propia app al abrirla.

### 2.3 Plugins: una sola exportación válida para las dos versiones

Los cuatro ficheros exportan ahora **exactamente un** objeto por defecto:

```ts
export default { id: "…", server: Fabrica, effect: Fabrica, setup: Fabrica }
```

- `server` lo exige 1.19.3; `effect`/`setup` los exige 2.2.5.
- No queda ninguna función exportada más: se acabaron las llamadas espurias (`recomendar`, `buscar`) y la búsqueda web en cada arranque.
- La factoría ya **no espera** a `client.app.log` ni ejecuta Python durante la activación (era otra forma de dejar la activación pendiente).

Ficheros modificados (con backup previo y hashes en `backups\reparacion_opencode_20261102_182308\SHA256SUMS.txt`):

- `xdg-config\opencode\plugin\xoxete-learning.ts`
- `xdg-config\opencode\plugin\xoxete-router.ts`
- `xdg-config\opencode\plugin\xoxete-web.ts`
- `xdg-config\opencode\plugin\security-agent.ts` (copia previa: `security-agent.ts.antes-de-adaptar`)

### 2.4 Configuración de la app (sin duplicar la del CLI)

`H:\Xoxete\xdg-config-app\opencode\` contiene **hardlinks** (`mklink /H`) a `opencode.json`, `AGENTS.md` y `AGENTS_AMPLIACION.md` del XDG del CLI, más copia de `agents\` y `command\`, y **sin** carpeta `plugin\`. Editar una config edita la otra: no hay deriva.

---

## 3. Pruebas reales ejecutadas

### CLI 1.19.3 (`opencode-original\opencode.exe`, con los 4 plugins)

| Prueba | Resultado |
|---|---|
| `run` sin stdin redirigido | se cuelga >180 s (reproducido) |
| `run` con `< /dev/null` | responde en **30 s** («PRUEBA full») y **38 s** («VERIFICACION») |
| `run` final tras restaurar los plugins | **136 s** → «CLI FINAL OK» |
| Registro de plugins | `herramienta buscar_web disponible`, `router cargado · catálogo…`, `plugin cargado · motor localizado · memoria automática activa` — **sin un solo `failed to load plugin`** |
| Memoria automática | `memoria inyectada · n=2 · ids=4,3` y `n=1 · ids=3` (recuerdos reales) |
| Agente y modelo | `agent=xoxete`, `model=ollama/xoxete-14b:latest` en el log de cada sesión |

### App 2.2.5 (mismo servidor que carga la ventana)

| Prueba | Resultado |
|---|---|
| Servicio | 28894 escuchando, `HTTP 200` en `/` |
| Emparejamiento | enlace `opencode pair` válido, entra en la UI |
| `/api/provider`, `/api/model`, `/api/agent` | **200 en 15 ms** con la config sin plugins |
| Contenido de `/api/agent` | agente **`xoxete`**, modelo `xoxete-14b:latest`, proveedor `ollama` |
| Crear chat nuevo | sesión `ses_e627800eeffeQx1dQlYgHYtArI` creada desde la interfaz |
| Responder en el chat | «CHAT DESDE LA APP OK» y «APP ESTABLE OK» |
| Historial | la sesión «PUENTE XOXETE» carga con su mensaje y su tiempo (1m 14s) |

### Escritura de memoria desde KUKIA CODE (prueba controlada)

- Antes: **8** recuerdos. Escritura vía `core::xoxete::guardar` → «Guardado como memoria #10».
- Después: **9** recuerdos (`experiencia` 2 → 3) y `search "KUKIA CODE"` devuelve `#10`.
- Misma base original (`learning\memoria_aprendizaje.db`, 65.536 bytes). No se ha copiado ni duplicado nada.

---

## 4. Xoxete dentro de KUKIA CODE (parte C)

Módulo nuevo: `core/src/xoxete.rs`

- Detecta la instalación real (`KUKIA_XOXETE_ROOT` o `H:\Xoxete`): config (con BOM, como está), agente y modelo por defecto, instrucciones y si existen, los 4 plugins esperados, y los binarios con su **versión real** (`1.19.3` los dos CLI).
- Motor de memoria por su CLI oficial (`stats`, `search`, `add`), con **tope de 10 s** y matado del proceso si se pasa: un motor atascado no congela la interfaz.
- Lectura libre; la escritura es una acción explícita del usuario (la vista pide confirmación y avisa de que escribe en la base original).
- Nunca duplica la memoria ni el cerebro.

Comandos Tauri: `xoxete_estado`, `xoxete_memorias`, `xoxete_recordar` (en `src-tauri/src/lib.rs`).
Interfaz: `app/src/views/Xoxete.tsx` + entrada «Xoxete 💜» en el menú + tipos y métodos en `app/src/lib/api.ts`.

Comprobaciones de compilación:

```
cargo check -p kukia-code            → Finished (sin avisos de error)
cargo test  -p kukia-core --lib xoxete → 4 passed (incluye estado() real: 1,53 s)
npx tsc --noEmit                     → rc=0
npx vite build                       → ✓ built in 4.15s
```

Datos reales que devuelve hoy `xoxete_estado()`:

```
raiz H:\Xoxete · config_ok true · agente xoxete · modelo ollama/xoxete-14b:latest
motor: disponible, 8 recuerdos (conocimiento 3, error 2, experiencia 2, solucion 1)
plugins: iocs.json, security-agent.ts, sentinel_*.py, skill_validator.py, xoxete-*.ts
binarios: CLI oficial 1.19.3 · CLI Evil 1.19.3
```

---

## 5. Copias y protecciones

- `backups\reparacion_opencode_20261102_182308\` — `opencode.json`, los 4 plugins originales, `AGENTS_AMPLIACION.md`, `SHA256SUMS.txt`, `Abrir-Xoxete.cmd.antes` y `security-agent.ts.antes-de-adaptar`.
- La carpeta `plugin\` se movió temporalmente durante la bisección y se **restauró íntegra**.
- No se ha borrado ningún modelo, sesión, base de datos ni configuración original.
- Los directorios de prueba que creé (`oc_cfg_test`, `oc_plugins_test`, `plugin_pruebas`) se eliminaron; los procesos que arranqué para probar se pararon por PID.

---

## 6. Límites que quedan (no maquillados)

1. **En la app 2.2.5 los plugins no se activan.** Cualquier fichero en `opencode\plugin` deja la activación pendiente (503). Por eso la GUI usa una config sin plugins: chat, modelos, agentes e historial funcionan, pero **sin** memoria automática ni herramientas propias de Xoxete. El cerebro completo está en el CLI 1.19.3 (`Abrir-Evil-Xoxete.cmd` u `opencode-original\opencode.exe`).
2. **`micode` y `opencode-plugin-openspec`** (npm) fallaron al cargar en la última ejecución del CLI (`failed to load plugin`), aunque en ejecuciones anteriores sí cargaron. Son plugins de terceros, no del cerebro de Xoxete.
3. **La ventana Electron real no se ha abierto** en esta sesión: se ha probado el mismo servidor que la ventana carga (28894) más su API y su interfaz. La app debe levantar su servicio al abrirse con el lanzador nuevo.
4. La 2.2.5 mostró un comportamiento **inestable** en la prueba con `C:` (200 una vez). Lo reproducible es: sin plugins → 200; con plugins → 503.

---

## 7. Cómo usarlo

- **App de escritorio:** doble clic en `H:\Xoxete\Abrir-Xoxete.cmd`.
- **CLI con el cerebro completo:** `H:\Xoxete\Abrir-Evil-Xoxete.cmd` (o `opencode-original\opencode.exe run "…" < /dev/null`).
- **KUKIA CODE:** pestaña **Xoxete** — estado real, plugins, binarios, búsqueda de recuerdos y alta de recuerdos con confirmación.
