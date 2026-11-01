# Instalador

El instalador se genera con el bundler oficial de Tauri (NSIS).

```powershell
cd H:\KUKIA_CODE
npm run tauri:build
```

Salida:

```
src-tauri\target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe
```

## Características

- Instalación por máquina (`perMachine`) con elevación estándar.
- Desinstalación limpia desde Configuración → Aplicaciones.
- **No** instala ni modifica Ollama, LM Studio, Pinokio, CUDA, Python ni controladores.
- Los datos del usuario viven en `%APPDATA%\KukiaCode` y no se tocan al desinstalar (se pueden borrar manualmente si el usuario quiere).

## Firma (opcional, futura)

Para evitar avisos de SmartScreen hace falta un certificado de firma de código
(200–400 €/año). Se configurará en `tauri.conf.json` → `bundle.windows.certificateThumbprint`
cuando exista. No se incluye ningún certificado en este repositorio.
