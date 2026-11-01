# KUKIA CODE — Manual de uso (v0.1.0)

## Requisitos

- Windows 11 x64
- WebView2 (ya presente en el equipo)
- Para modelos locales: Ollama en `127.0.0.1:11434` y/o LM Studio en `127.0.0.1:1234`

## Desarrollo

```powershell
cd H:\KUKIA_CODE
npm install
npm run tauri:dev
```

## Compilar instalador

```powershell
cd H:\KUKIA_CODE
npm run tauri:build
```

Salida esperada:

```
src-tauri\target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe
```

## Vistas

| Vista | Qué hace |
|---|---|
| Chat | Conversaciones con streaming, agente seleccionable, historial en SQLite |
| Modelos | Detecta Ollama / LM Studio / cloud y permite elegir modelo por proveedor |
| Agentes | 8 agentes iniciales + creación de agentes personalizados |
| Archivos | Explorador, lectura y escritura con aprobación + backup automático |
| Terminal | PowerShell con aprobación obligatoria y registro de auditoría |
| Memoria | Búsqueda y alta de recuerdos persistentes |
| GPU | Métricas en vivo (VRAM, uso, temperatura, consumo, relojes) con histórico |
| Benchmarks | Mide tokens/s reales contra el modelo activo |
| Radar | Releases oficiales + matriz de compatibilidad verificada |
| Historial | Conversaciones guardadas |
| Configuración | Modelo por defecto, temperatura, reparto GPU 35/65, cloud |
| Herramientas | Estado real de cada herramienta (implementado / pendiente) |
| Tareas | Estado real: no hay tareas en segundo plano en esta fase |

## Seguridad

- Escritura de archivos y comandos requieren casilla de aprobación.
- Cada aprobación queda registrada en la base de datos.
- Copia de seguridad automática antes de sobrescribir.
- La clave cloud se lee de `KUKIA_CLOUD_KEY` (nunca se escribe en disco).

## Limitaciones conocidas (Fase 2)

- Sin cliente MCP todavía.
- Sin investigación autónoma.
- Sin actualizador firmado.
- Sin editor con resaltado de sintaxis (Monaco llegará en F7).
- El reparto multi-GPU es un perfil guardado: cada motor decide cómo aplicarlo.
