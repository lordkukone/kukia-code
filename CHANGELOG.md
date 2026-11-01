# CHANGELOG — KUKIA CODE

Formato: `[versión] fecha — resumen`. Todo lo listado aquí tiene prueba ejecutada.

## [0.1.0] — 01/11/2026 — Fase 3.4: aviso automático de competencia de VRAM

### Añadido
- `vram::evaluar_competencia()`: función pura que decide si dos motores compiten por VRAM
  (`competing`, `count`, `detalle`, `models`, `message`).
- Vigilante de fondo en `src-tauri/src/lib.rs`: consulta cada 20 s (primera pasada al arrancar),
  emite `vram://competencia` y `vram://competencia_fin` solo cuando cambia la composición.
- Banner global en la interfaz: aparece en cualquier vista, con «Abrir VRAM Guard» y ✕.
- Registro `warn` / `info` en `logs\kukia.log` al aparecer y al resolverse.
- 4 tests nuevos, puros y sin red.

### Cambiado
- `report()` usa la misma función que el vigilante: un solo criterio de competencia.
- Etiqueta de cabecera: de «v0.1.0 · Fase 2» a «v0.1.0 · **Fase 3**».

### Seguridad
- CSP real en `tauri.conf.json` (antes `null`): `default-src 'self'`, imágenes `data:`/`blob:`,
  estilos inline permitidos, IPC explícito.
- **Incidencia:** una CSP más estricta (`script-src 'self'`, `object-src`, `base-uri`,
  `form-action`, `frame-ancestors`) dejaba la ventana en blanco. Retirada y verificada la actual.

### Artefactos
- Instalador NSIS `KUKIA CODE_0.1.0_x64-setup.exe` · SHA-256 `509646014c93dd858c6f6fededccdb883c813423aaa2e6461bdf910faadf088f`
- Exe release `de2d933c…64f09` · exe instalado `e4149cfb…71b7`

---

## [0.1.0] — 01/11/2026 — Fase 3.3: commit + release + instalación

- Commit `2048986`: 51 archivos, +11.147 / −169 (permisos, VRAM, gaming, anime-real, cloud).
- Build 3 min 13 s; instalación verificada; `agent_policies` con 9 filas sembradas.
- Informe: `INFORMES/FASE_3_3_RELEASE.md`.

---

## [0.1.0] — 01/11/2026 — Fase 3.2: Anime ↔ Real

- ComfyUI real, SDXL img2img, IP-Adapter; workflows en `workflows/anime_real/`.
- Conservación de identidad y comparación antes/después. Informe: `INFORMES/FASE_3_2_ANIME_REAL.md`.

---

## [0.1.0] — 01/11/2026 — Fase 3.1: VRAM Guard y Gaming Radar conectados

- Comandos `vram_report`, `vram_fits`, radar de gaming con fichas y alertas por nivel.
- VRAM Guard detecta competencia entre Ollama, LM Studio y ComfyUI. Informe: `INFORMES/FASE_3_1_VRAM_GAMING.md`.

---

## [0.1.0] — 01/11/2026 — Fase 3A y 3.0: seguridad

- Permisos por proyecto (`AgentPolicy` con `read_paths`/`write_paths`, solo-lectura).
- `files_write` y `run_command` aplican la política; timeout con cancelación; `audit_log` conectado.
- Tests fuera del log y de la base de producción (`validacion_fase30.rs`, `validacion_fase3a.rs`).

---

## [0.1.0] — 01/11/2026 — Fase 2.1

- Validación de release, 4 fallos corregidos (backup verificado, puerto LM Studio,
  `reasoning_content`, canal de razonamiento), benchmark tok/s. Commit `18d43fb`.

---

## [0.1.0] — 01/11/2026 — Fase 2

- MVP: Tauri 2 + React 19 + núcleo Rust + SQLite + radar. Commit `b125554`.
