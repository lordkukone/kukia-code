# KUKIA CODE

Asistente de IA de escritorio para Windows 11, independiente de OpenCode y Freebuff.

- Interfaz propia (React + TypeScript) sobre **Tauri 2**
- Núcleo en **Rust** (`core/`) sin dependencias de la UI
- Memoria persistente en **SQLite**
- Conectores **Ollama** y **LM Studio** + APIs OpenAI-compatible
- Monitor **GPU** (VRAM, uso, temperatura, consumo, relojes)
- **KUKIA Technology Radar**: releases oficiales y matriz de compatibilidad
- **KUKIA Experimental AI Radar**: arXiv, Hugging Face y GitHub, alertas, informes diarios y laboratorio aislado en `H:\KUKIA_RESEARCH\AI_EXPERIMENTAL`
- Instalador **NSIS** (`KUKIA CODE_0.1.0_x64-setup.exe`)

## Estado

**Fase 2 — MVP funcional.** Ver `INFORMES/FASE_2.md` para lo probado y lo pendiente.
No se declara terminado nada que no tenga prueba o verificación real.

## Arranque rápido

```powershell
cd H:\KUKIA_CODE
npm install
npm run tauri:dev
```

Compilar instalador:

```powershell
npm run tauri:build
```

## Documentación

- `docs/ARQUITECTURA.md` — decisiones técnicas y módulos
- `docs/MANUAL.md` — uso de la aplicación
- `docs/ROADMAP.md` — fases
- `INFORMES/AUDITORIA_INICIAL.md` — auditoría del equipo
- `INFORMES/FASE_2.md` — entrega de esta fase

## Reglas del proyecto

- Trabajo exclusivo en `H:\KUKIA_CODE`.
- No se modifica OpenCode, Freebuff, Pinokio, CUDA, drivers ni modelos.
- No se borra nada sin confirmación.
- Escrituras y comandos requieren aprobación explícita y quedan auditados.
- Copia de seguridad antes de sobrescribir archivos.
