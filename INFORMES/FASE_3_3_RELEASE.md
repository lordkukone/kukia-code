# KUKIA CODE — FASE 3.3 (commit + release + instalación)

**Fecha:** 01/11/2026 · **Autor:** Xoxete

---

## 1. COMMIT

| Dato | Valor |
|---|---|
| Hash | `2048986` |
| Mensaje | `Fase 3 cerrada: permisos aplicados, comandos con timeout y auditoria real` |
| Archivos | **51** · +11.147 / −169 |
| Rama | `master` (sin remoto configurado → nada que empujar) |

Incluye: `permissions.rs`, `vram.rs`, `gaming.rs`, `anime_real.rs`, `validacion_fase30.rs`, `validacion_fase3a.rs`, `validacion_cloud.rs`, `validacion_gaming.rs`, `validacion_anime_real.rs`, las 4 vistas nuevas (Seguridad, VRAM Guard, Gaming Radar, Anime ↔ Real), selector cloud, workflows de ComfyUI, `docs/` e informes.

**Fuera del commit a propósito** (basura temporal, no proyecto): `tmp_capturas/`, `tmp_seed_test/`, `logs/`, `output/` (16 MB de imágenes generadas) y `capturar_ventana.ps1` (herramienta de captura).

---

## 2. TESTS ANTES DE EMPAQUETAR

```
cargo test --release -p kukia-core
  65 passed · 3 ignored (anime_real, salen a ComfyUI)
  1 passed · 2 ignored (cloud, necesitan red/clave)
  8 passed · 5 ignored (fase3a)
  9 passed            (fase30)
 11 passed            (fase21)
  0 passed · 2 ignored (gaming, salen a GitHub/arXiv)
→ 94 pasan · 12 ignorados · 0 fallos

npm run build (tsc --noEmit && vite build)
→ 50 módulos · index-MY4nb9cT.js 296,98 kB (88,50 gzip) · ok
```

---

## 3. COMPILACIÓN

| Dato | Valor |
|---|---|
| Comando | `npx tauri build` |
| Tiempo | **3 min 13 s** |
| Resultado | `target/release/kukia-code.exe` — 9.795.584 B |
| SHA-256 exe | `47754464a1bc435e53c3daba37d47e9293d865ad3f0ce195722b931e84f08339` |
| Instalador | `KUKIA CODE_0.1.0_x64-setup.exe` — 2.930.665 B |
| SHA-256 instalador | `21564960f7c95af321ff358a6b7fed20ccde654b6d060dfccbefabeed318e17a` |

---

## 4. BACKUP PREVIO (regla del proyecto)

| Qué | SHA-256 |
|---|---|
| Instalado antes de F3.3 (copia en `backups/instalado_20261101_previo_f33/`) | `558c96ddac80d88986c44b7eb71bf4d63dccd0a4f1392cd99bfbb7782592687b` |
| Instalado en `C:\Program Files` antes del cambio | idéntico (`558c96dd…`) → backup verificado |

---

## 5. INSTALACIÓN

| Intento | Resultado |
|---|---|
| 1 · `"…setup.exe" /S` elevado | **no se ejecutó** (el wrapper no reportó; verificado en disco: seguía el binario viejo) |
| 2 · mismo comando | **fallo de parseo**: PowerShell interpretó `/S` como división |
| 3 · `Start-Process … -ArgumentList '/S' -Wait` | **instalado**, exit 0 |

| Dato tras instalar | Valor |
|---|---|
| `C:\Program Files\KUKIA CODE\kukia-code.exe` | 9.795.584 B · 15:23 |
| SHA-256 instalado | `f1a79f20fa28c7a8efbdee0417c4421c5c028ab99cb58735cb5bc1ec94608ec2` |
| Diferencia con el release | **3 bytes** de 9.795.584 (offsets 7.820.962-64) → mismo binario, ruido de empaquetado |
| Versión de recursos | `FileVersion 0.1.0` · `ProductVersion 0.1.0` |
| Desinstalador | `uninstall.exe` actualizado (15:28) |

---

## 6. VERIFICACIÓN EN VIVO (app instalada)

- Proceso arrancado: `kukia-code.exe` PID 43612 · ventana 1293×685 maximizada.
- Base `%APPDATA%\KukiaCode\kukia.db`: **políticas sembradas** → `agent_policies` con **9 filas**; 18 tablas presentes (`projects`, `audit`, `approvals`, `gaming_*`, `research_*`, `tasks`…).
- `audit` con 0 filas: correcto, nadie ha ejecutado todavía una acción auditada.
- Interfaz: navegación con **Seguridad, VRAM Guard, Gaming Radar, Anime ↔ Real** y contador **3/3 proveedores**.
- VRAM Guard con lectura real del 01/11 a las **15:30:11**: RTX 5070 11,5/11,9 GB (0,1 GB libres), RTX 5060 Ti 13,8/15,9 GB (1,9 GB libres) y aviso de **competencia de VRAM** con LM Studio + ComfyUI cargados a la vez.

---

## 7. PENDIENTE / LIMITACIONES HONESTAS

1. **La cabecera sigue diciendo «v0.1.0 · Fase 2»**: es una cadena fija en la interfaz, no refleja Fase 3. Cosmético, pero engaña.
2. **Sin firma de código ni updater**: SmartScreen avisará; la actualización no es verificable ni reversible desde la app.
3. **`csp: null`** en `tauri.conf.json`: sin política de contenido (riesgo S2 de la auditoría cloud).
4. Instalación **perMachine**: cada actualización pide administrador.
5. No probado: actualización sobre instalación existente conservando `%APPDATA%` (hoy se ha instalado encima, pero sin test automatizado) ni desinstalación limpia.
6. El commit **no incluye** los borradores de captura ni `output/`; si quieres histórico visual, hay que versionarlo aparte.
