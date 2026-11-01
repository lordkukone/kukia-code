# KUKIA CODE — CÓMO CONTINUAR

**Actualizado:** 01/11/2026 · **Autor:** Xoxete
Lee esto antes de tocar nada. Está escrito para que otra sesión (u otra IA) no rompa lo que ya funciona.

## 0. Reglas que no se negocian

1. `I:\CEREBRO` es **solo lectura**. Nunca borrar, renombrar ni sobrescribir.
2. Antes de editar configuración o binarios: **backup + hash verificado**. Si el backup falla, parar.
3. No borrar modelos, ni tocar KUKIA Network, ni CUDA/drivers, ni Pinokio, ni OpenCode, ni Ollama.
4. Nada se declara terminado sin prueba ejecutada y con su salida pegada.
5. `tmp_capturas/`, `logs/`, `output/`, `tmp_seed_test/` **no** se commitean.

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
