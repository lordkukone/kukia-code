# KUKIA CODE — Roadmap

| Fase | Contenido | Estado |
|---|---|---|
| F1 | Auditoría del equipo | ✅ completada (`INFORMES/AUDITORIA_INICIAL.md`) |
| F2 | MVP: Tauri + React, núcleo Rust, Ollama/LM Studio, memoria SQLite, GPU, logs, tests, radar | ✅ esta entrega |
| F3 | Núcleo ampliado: permisos por herramienta, auditoría avanzada, recuperación de tareas | pendiente |
| F4 | Conectores completos: PolyStrata, llama.cpp, descarga/gestión de modelos | pendiente |
| F3A | Permisos por proyecto (ámbito que recorta) y tests fuera del log/base reales | ✅ verificado (`INFORMES/FASE_3A.md`) |
| F3.1 | VRAM Guard y Gaming Radar conectados a la app: comandos, vistas y prueba con dos motores cargados | ✅ verificado (`INFORMES/FASE_3_1_VRAM_GAMING.md`) |
| F3.2 | KUKIA ANIME ↔ REAL STUDIO: anime→real y real→anime por ComfyUI real, capacidades comprobadas y antes/después | ✅ verificado (`docs/ANIME_REAL.md`) |
| F4.5 | KUKIA Experimental AI Radar (arXiv/HF/GitHub, alertas, informes, laboratorio aislado) | ✅ integrado en F2 |
| F5 | UI avanzada: Monaco, markdown enriquecido, adjuntos, atajos | pendiente |
| F6 | Agentes con tools reales + cliente MCP | pendiente |
| F7 | Memoria semántica (embeddings), proyectos, editor | pendiente |
| F8 | Terminal pty integrada (xterm.js + portable-pty) | pendiente |
| F9 | Investigación autónoma supervisada con hitos de aprobación | pendiente |
| F10 | Actualizador firmado, licencias modulares, release 1.0 | pendiente |

## Fase 2 — alcance entregado

- [x] Estructura modular completa
- [x] UI oscura con 13 vistas
- [x] Núcleo Rust separado (`kukia-core`)
- [x] Conector Ollama (modelos, chat, streaming, tokens/s)
- [x] Conector LM Studio / OpenAI-compatible
- [x] Selector de modelos y proveedores
- [x] Memoria SQLite (sesiones, mensajes, recuerdos)
- [x] Monitor GPU con histórico
- [x] Benchmarks persistidos
- [x] Radar de releases + matriz de compatibilidad
- [x] Sistema de logs y aprobaciones
- [x] Tests unitarios del núcleo
- [x] Instalador NSIS configurado
- [ ] Verificación de instalación en limpio (se documenta en FASE_2.md)
