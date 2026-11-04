# KUKIA CODE — CÓMO CONTINUAR

**Actualizado:** 01/11/2026 · **Autor:** Xoxete · **revisado 04/11/2026** (ver §0.1)
Lee esto antes de tocar nada. Está escrito para que otra sesión (u otra IA) no rompa lo que ya funciona.

## 0. Reglas que no se negocian

1. `I:\CEREBRO` es **solo lectura**. Nunca borrar, renombrar ni sobrescribir.
2. Antes de editar configuración o binarios: **backup + hash verificado**. Si el backup falla, parar.
3. No borrar modelos, ni tocar KUKIA Network, ni CUDA/drivers, ni Pinokio, ni OpenCode, ni Ollama.
4. Nada se declara terminado sin prueba ejecutada y con su salida pegada.
5. `tmp_capturas/`, `logs/`, `output/`, `tmp_seed_test/` **no** se commitean.
6. **OpenCode no se toca.** Su `auth.json` se lee en solo lectura para la cascada de claves; no se
   modifica, no se reinstala y no se le cambia la configuración. Lo mismo con Ollama y LM Studio.

## 0.1 Estado verificado el 04/11/2026 (leer antes de tocar nada)

Auditoría de solo lectura sobre el equipo real. El detalle completo está en `PROGRESO_GENERAL.md`
(§ «Verificación independiente»); aquí va solo lo accionable.

| # | Hallazgo | Qué hacer |
|---|---|---|
| V1 | El exe instalado es del **02/11 21:30**, 12.371.456 B, SHA-256 `a90a7d89…fdb4f5`, y **declara 0.1.0** igual que la release del 01/11 (que pesaba 9.812.992 B) | Poner número de build visible (o subir a 0.2.0) y añadir el hash del exe interno a `SHA256SUMS` |
| V2 | `default_model` = `kukia-gemma4-q5` (19,13 GB) con `gpu_split [30,70]` → **carga rechazada**: 20,21 GB en 15,45 GB libres | **Hecho 04/11**: `gpu_split = [50, 50]` (backup en `backups/config_2026-11-04/`). Carga en **52 s**, repartido 11.413 + 10.687 MiB |
| V7 | `vram::fits()` parece comparar el modelo **entero** contra una sola GPU, no contra el reparto | **Sin verificar en código**: comprobar si el cálculo debe ser por tarjeta |
| V3 | La app **no está rota**: 26 arranques, último 04/11 11:32, 0 errores, `integrity_check ok`, 817 memorias | Nada; usar V1/V2 como causa del síntoma «solo va el chat» |
| V4 | `secrets.json` sigue en claro (21 B) | **C0**, sin novedad: sigue siendo lo primero |
| V5 | El worker **vuelve a estar cableado**: `DESKTOP-6CK4U6H` = `192.168.50.2`, < 1 ms, RDP+WinRM abiertos | **M0-B**: inventario de solo lectura (necesita credenciales). No tiene Ollama, LM Studio ni `kukia-worker.exe` (11434/1234/1235/8791 cerrados) |
| V6 | Este documento y `PROGRESO_GENERAL.md` decían que no había tool calling ni MCP; el build del 03/11 los tiene | Corregido en `PROGRESO_GENERAL.md`; falta revisar D3 y D11 de `MATRIZ_FUNCIONALIDADES.md` |

**No hacer sin autorización:** reinstalar o reconfigurar OpenCode, tocar `auth.json`, borrar
`%APPDATA%\KukiaCode`, abrir puertos en el router, ni instalar nada en el worker.

## 1. Comandos que funcionan (Windows, Git Bash)

```bash
cd /h/KUKIA_CODE

# Pruebas del núcleo (98 pasan, 12 ignorados por salir a Internet/ComfyUI)
cargo test --release -p kukia-core

# Typecheck + build del frontend
npm run build

# Release completo + instalador NSIS  (~3 min)
npx tauri build

# Instalar (requiere administrador; /S directo falla en PowerShell)
Start-Process "H:\KUKIA_CODE\target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe" -ArgumentList '/S' -Wait

# Ver la ventana instalada y capturarla / pulsar un elemento por nombre accesible
powershell -NoProfile -ExecutionPolicy Bypass -File capturar_ventana.ps1 -Salida "x.png"
powershell -NoProfile -ExecutionPolicy Bypass -File capturar_ventana.ps1 -Pulsar "VRAM Guard" -Salida "x.png"
```

**Trampa aprendida:** `"…setup.exe" /S` con elevación se parsea como PowerShell y explota en el `/`.
Hay que usar `Start-Process … -ArgumentList '/S' -Wait`.

## 2. Trampa gorda: la CSP

`src-tauri/tauri.conf.json` tiene una CSP **que funciona**:

```
default-src 'self'; img-src 'self' data: blob: asset: http://asset.localhost;
style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost
```

Añadir `script-src 'self'`, `object-src 'none'`, `base-uri 'self'`, `form-action 'none'` o
`frame-ancestors 'none'` **dejó la ventana en blanco** (webview sin contenido, 1 elemento accesible).
Probado: la misma política en Chromium sí pinta, así que el fallo es del lado de Tauri/wry.
Si alguien quiere endurecerla, que lo haga **una directiva por build** y verificando con
`capturar_ventana.ps1 -Listar` (debe devolver decenas de elementos, no 1).

## 3. Trampa: el hash del exe instalado no coincide con el del release

Siempre difieren **3 bytes** (offsets ~7.820.962). Mismo tamaño y misma versión: es ruido de
empaquetado, no otra compilación. No perseguirlo.

## 4. Qué hacer a continuación, por orden

### C0 — Credenciales protegidas (lo primero, bloquea lo demás)

**Estado 04/11/2026: el cifrado está hecho.** `secrets.json` va cifrado con DPAPI
(`core/src/secrets.rs`), se migra solo al arrancar y deja `secrets.json.legacy.bak`.
Verificado en el binario release del 04/11. **Queda pendiente la vista CLOUD CREDENTIALS.**

- `secrets.json` está en claro en `%APPDATA%\KukiaCode`. Migrar a **DPAPI** (`CryptProtectData`)
  manteniendo lectura del formato viejo para migrar sin perder nada.
- Vista **CLOUD CREDENTIALS**: alta, prueba de conexión, revocación, «necesita configuración».
- Nunca devolver la clave a la interfaz; solo `presente/no presente` y origen.
- Detalle completo en `INFORMES/AUDITORIA_CLOUD_OMEGA.md` §6.

### C1 — Catálogo real de modelos

Ya está en disco el registro `models.dev`: `~/.cache/opencode/models.json`
(**235 proveedores, 8.940 modelos** con coste, límites, modalidades, `tool_call`, `reasoning`).
Leerlo (o descargarlo propio con hash) y montar la ficha por modelo. Ver §2.2 de la auditoría.

### Después

C2 adaptadores (Anthropic/Gemini/Azure) → C4 uso y coste → C3 router + kill switch →
C5 tool calling → C6 release con updater firmado.

## 5. Estado de la deuda técnica conocida

| Tema | Estado |
|---|---|
| `audit` con 0 filas | Normal: no hay acciones ejecutadas todavía |
| Notificación si la app está minimizada | No existe: el banner es interno (haría falta `tauri-plugin-notification`) |
| Intervalo del vigilante de VRAM | Fijo en 20 s, no configurable |
| Instalación `perMachine` | Pide administrador en cada actualización |
| Firma de código / updater | No hay |
| Test de actualización conservando `%APPDATA%` | No automatizado |
| Presupuesto/coste de nube | No existe |

## 6. Cómo verificar que la app sigue viva tras un cambio

```bash
# 1. proceso
tasklist | grep -i kukia-code
# 2. árbol de accesibilidad (debe haber muchos elementos, no 1)
powershell -NoProfile -ExecutionPolicy Bypass -File capturar_ventana.ps1 -Listar -Salida "check.png"
# 3. log del núcleo
tail -20 "$APPDATA/KukiaCode/logs/kukia.log"
# 4. base sembrada
python -c "import sqlite3,os;c=sqlite3.connect(os.path.expandvars(r'%APPDATA%\\KukiaCode\\kukia.db'));print(c.execute('select count(*) from agent_policies').fetchone())"
```
