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

## Pendiente (fases futuras)

- Permisos por herramienta y por proyecto (F3).
- Firma de código del instalador (F10).
- Cifrado de la base de datos local (opcional, F3).
- Sandbox de plugins (F10).
