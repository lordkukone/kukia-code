# KUKIA CODE — Modelo de seguridad

## Principios

1. **Local primero.** Nada sale del equipo salvo las peticiones que tú hagas a un proveedor cloud.
2. **Aprobación explícita** para acciones sensibles.
3. **Auditoría** de cada aprobación en SQLite.
4. **Backup antes de sobrescribir.**
5. **Sin auto-actualización.**

## Operaciones y permisos

| Operación | Permiso | Registro |
|---|---|---|
| Leer archivos | directo (límite 2 MB) | — |
| Listar directorios | directo | — |
| Escribir archivos | `approved: true` obligatorio | `approvals` + backup |
| Ejecutar PowerShell | `approved: true` obligatorio | `approvals` + log |
| Guardar memoria | directo | tabla `memories` |
| Llamar a un modelo | directo (según proveedor configurado) | `logs` |
| Radar de versiones | directo (solo lectura de APIs públicas) | `logs` + `radar_cache` |

## Credenciales

- La clave del proveedor cloud se lee de `KUKIA_CLOUD_KEY`.
- **Nunca** se escribe en `config.json` ni en la base de datos.
- `has_cloud_key` solo indica presencia, no el valor.

## Aislamiento del sistema

KUKIA CODE **no**:

- modifica variables de entorno globales,
- instala ni actualiza Ollama, LM Studio, Pinokio, CUDA o drivers,
- mueve modelos,
- toca KUKIA Network, su ledger o sus wallets,
- ejecuta nada sin aprobación cuando la operación es destructiva.

## Datos en disco

```
%APPDATA%\KukiaCode\config\config.json     configuración (sin secretos)
%APPDATA%\KukiaCode\config\agents.json     agentes personalizados
%APPDATA%\KukiaCode\kukia.db               sesiones, mensajes, memorias, auditoría
%APPDATA%\KukiaCode\logs\kukia.log         log de aplicación
%APPDATA%\KukiaCode\backups\               copias previas a escrituras
```

## Fase 3.0 — permisos aplicados, timeout y auditoría (01/11/2026)

Hasta esta fase la allowlist de rutas existía en el núcleo pero **no se aplicaba**. Ahora:

| Operación | Comprobación antes de tocar nada |
|---|---|
| `files.read` | política `allow_read` + `read_paths` |
| `files.write` | `allow_write` + aprobación + **solo lectura** + allowlist de escritura |
| `terminal.run` | `allow_shell` + aprobación + destructivos + `cwd` dentro de las rutas autorizadas |
| `audit.export` | igual que `files.write` sobre la ruta del CSV |
| `policy.save` | aprobación explícita del usuario |

- Las rutas se comparan **canonicalizadas**: un *junction* de Windows no permite escapar.
- Los comandos destructivos se detectan **por verbo** (`rd /s /q`, `Remove-Item`, `robocopy /MIR`,
  `diskpart`, `git clean -f`…), no por subcadena.
- Todo comando tiene **timeout** (por defecto 60 s, ajustable en la interfaz). Al agotarse se mata
  el **árbol** de procesos (`taskkill /T /F`).
- La salida se recorta a **1 MB** por flujo, drenando el resto para no bloquear al hijo.
- La ejecución devuelve un **id** inmediato y se puede **cancelar** desde la terminal.
- **Toda** lectura, escritura, comando y cambio de permisos queda en la tabla `audit`
  (agente, herramienta, acción, objetivo, aprobación, resultado, error) y se puede filtrar y
  exportar a CSV desde **Seguridad**.
- Las políticas se siembran una sola vez: escritura permitida en el perfil del usuario y en
  `H:\KUKIA_CODE` / `H:\KUKIA_RESEARCH`; `I:\CEREBRO` marcado como **solo lectura**.

Un agente **no puede cambiar su propia política**: `policy.save` exige aprobación del usuario y
queda auditado como `usuario / policy.save`.

## Pendiente (fases futuras)

- Permisos por **proyecto** (hoy son por agente y por ruta). (F3A)
- Firma de código del instalador (F10).
- Cifrado de la base de datos local (opcional, F3).
- Sandbox de plugins (F10).

---

## Fase 3A — Permisos por proyecto y tests aislados

### Ámbito de proyecto

Un **proyecto** es una carpeta raíz con su propio ámbito de permisos (`tabla projects`).
Cuando hay un proyecto activo (se guarda en `config.json` → `active_project`), se aplican
**las dos** políticas a la vez:

```
permitido = política del agente  Y  ámbito del proyecto
```

- El proyecto **nunca amplía** lo que el agente no tiene: si el agente no puede escribir,
  el proyecto no lo concede.
- El proyecto **sí recorta**: escribir o ejecutar fuera de su raíz se deniega aunque el agente
  tenga permiso global sobre esa ruta.
- Un proyecto nuevo nace con escritura permitida **dentro de su raíz** y **shell desactivado**.
- `read_paths` vacío en el proyecto = lectura no acotada por el proyecto (sigue mandando el agente).
- `read_only_paths` del proyecto bloquea escritura y ejecución dentro de esas rutas.
- La aprobación explícita (`approved`) sigue siendo obligatoria, con o sin proyecto.

Razón de diseño: sin esto, la allowlist global es demasiado ancha para trabajar con varios
proyectos a la vez; con esto, activar «KUKIA CODE» impide que un agente escriba en
`H:\KUKIA_RESEARCH` por descuido aunque su política lo permita.

### Comandos

| Comando | Qué hace |
|---|---|
| `projects_list` | lista proyectos |
| `project_create` | alta con nombre y raíz (la raíz debe existir) |
| `project_update` | cambia ámbito; **exige `approved: true`** y queda auditado |
| `project_activate` | activa o desactiva (`null`) y lo guarda en la configuración |
| `project_active` | devuelve el proyecto activo, si lo hay |

Todo cambio de ámbito se registra en `audit` (`projects.create`, `projects.update`,
`projects.activate`) y en el log.

### Tests fuera de los datos reales

- `logs::append_to(dir, …)` y `logs::tail_from(dir, …)` permiten escribir en una carpeta concreta.
  Los tests usan una carpeta temporal y **no** el log de la aplicación.
- `KUKIA_LOG_DIR` permite redirigir el log (pruebas, entornos aislados).
- Las bases de datos de test ya eran temporales (`std::env::temp_dir()`); ahora hay un test que
  lo comprueba explícitamente contra la base real.

Antes de Fase 3A, cada `cargo test` añadía una línea `[test] linea de prueba kukia` al log de
producción. Ya no ocurre: verificado midiendo el tamaño del log antes y después de la suite.
