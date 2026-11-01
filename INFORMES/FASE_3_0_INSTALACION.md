# INSTALACIÓN DE KUKIA CODE CON FASE 3.0 — VERIFICADA

**Fecha:** 01/11/2026 · **Autor:** Xoxete (para Kuki / LordKukone)
**Objetivo:** compilar el instalador con Fase 3.0, reinstalar y comprobar que arranca y siembra
las políticas.

**Resultado: ✅ COMPLETADO Y VERIFICADO.**

---

## 1. Punto de partida (medido antes de tocar nada)

| Dato | Valor |
|---|---|
| App instalada | `C:\Program Files\KUKIA CODE\kukia-code.exe` · 9.063.936 B · 11:0x |
| SHA-256 instalado | `859a8078…fd405dd6` (la de Fase 2.1) |
| Versión | 0.1.0 |
| Proceso | **abierto** (`kukia-code.exe` PID 38744) → se cerró para poder reinstalar |
| Copia de seguridad | `backups/instalado_20261101_previo_f3/kukia-code.exe` · hash **idéntico** al instalado ✅ |
| Espacio | H: 102 GB libres · C: 108 GB libres |
| Herramientas | node 24.18.0 · npm 11.16.0 · rustc 1.98.1 |

---

## 2. Compilación

```
npm run tauri:build
  beforeBuildCommand: npm run build   → tsc + vite OK (293,15 kB js)
  Compiling kukia-core   →  Compiling kukia-code
  Finished `release` profile [optimized] in 3m 12s
  Running makensis → KUKIA CODE_0.1.0_x64-setup.exe (2.77 MiB)
```

| Artefacto | Bytes | SHA-256 |
|---|---:|---|
| `target/release/kukia-code.exe` | 9.695.744 | `f0464cf0dc3810763b79bae3a6ec5143ebb0aa0a1b5887ab4bf4b1427ff4449d` |
| `target/release/bundle/nsis/KUKIA CODE_0.1.0_x64-setup.exe` | 2.905.779 | `7d2bf4fcd80645b45447d7beb50d736528f7834cc977c226b191f6326d583309` |

**Aviso del enlazador:** `linker stdout: Creando biblioteca kukia_code_lib.dll.lib…` — es el aviso
normal al generar la `cdylib`; no es un error.

---

## 3. Instalación

- Configuración: `installMode: perMachine`, idiomas Spanish + English, identificador
  `com.kukia.code`.
- Ejecutada con elevación (Program Files lo exige) y en silencio:

```
Start-Process -FilePath "...\KUKIA CODE_0.1.0_x64-setup.exe" -ArgumentList "/S" -Wait
```

| Dato | Antes | Después |
|---|---|---|
| `kukia-code.exe` | 9.063.936 B | **9.695.744 B** (+631.808) |
| SHA-256 | `859a8078…` | **`7e9b7915b8bac075c966587095944799eb7ffb11e00c059c88a9ca14a663dfc0`** |
| `uninstall.exe` | 80.187 B | regenerado a las 12:45 |
| Versión | 0.1.0 | 0.1.0 |

El hash del exe instalado **no** coincide con el de `target/release` porque el bundler NSIS
parchea el binario (información de tipo de bundle). Es el comportamiento esperado y ya se
documentó en Fase 2.1.

---

## 4. Verificación del arranque

| Prueba | Resultado |
|---|---|
| Proceso vivo tras 14 s | ✅ `kukia-code.exe` PID 48904, ventana abierta |
| Log de aplicación | ✅ nueva línea `KUKIA CODE arrancando` (11:47:07 UTC = 12:47 local) |
| Base real intacta | ✅ `agent_policies` **9** · `sessions` 1 · `audit` 0 |
| Cierre limpio | ✅ cerrada por PID, sin residuos |

---

## 5. Verificación de la siembra de políticas (la prueba importante)

Para no tocar tu base real, se arrancó la app instalada con `KUKIA_DATA_DIR` apuntando a una
carpeta limpia (`H:\KUKIA_CODE\tmp_seed_test`). La app creó todo desde cero:

```
db: H:\KUKIA_CODE\tmp_seed_test\kukia.db  ·  118.784 bytes  ·  17 tablas
agent_policies: 9 filas
  kukia-coder      w=1 sh=1 net=1 ro=I:\CEREBRO
  kukia-cyber      w=1 sh=1 net=1 ro=I:\CEREBRO
  kukia-gaming     w=1 sh=1 net=1 ro=I:\CEREBRO
  kukia-gpu        w=1 sh=1 net=1 ro=I:\CEREBRO
  kukia-master     w=1 sh=1 net=1 ro=I:\CEREBRO
  kukia-radar      w=1 sh=1 net=1 ro=I:\CEREBRO
  kukia-research   w=1 sh=1 net=1 ro=I:\CEREBRO
  kukia-sysadmin   w=1 sh=1 net=1 ro=I:\CEREBRO
  kukia-wealth     w=1 sh=1 net=1 ro=I:\CEREBRO
```

Y sobre tu base real, al arrancar de nuevo: **siguen 9** (la siembra es idempotente, no duplica).

Rutas de escritura sembradas: `C:\Users\kuki;H:\KUKIA_CODE;H:\KUKIA_RESEARCH`.
Ruta de solo lectura: `I:\CEREBRO`.

**Esto demuestra que el código de Fase 3.0 está dentro de la app instalada**, porque la siembra de
políticas es código nuevo de esta fase: con el binario anterior la tabla no se habría rellenado.

---

## 6. Lo que queda pendiente (honesto)

1. **No se han pulsado botones dentro de la ventana.** Está probado que arranca, que escribe su
   log y que siembra políticas; las 18 vistas y los 55 comandos no se han ejercitado uno a uno.
2. **La app no tiene firma de código**: Windows puede mostrar aviso de editor desconocido.
3. **El log de producción sigue recibiendo líneas de los tests** (`[test] linea de prueba kukia`,
   3 apariciones). Es el defecto E-? de la auditoría: los tests escriben en el log real.
4. **Sin commit**: el trabajo sigue sin commitear (13 modificados + 23 nuevos). El instalador ya
   lleva Fase 3, pero el repositorio no tiene ese punto guardado.
5. **Carpeta de prueba sobrante**: `H:\KUKIA_CODE\tmp_seed_test` (118 KB). No se ha borrado nada
   sin permiso; dime y la quito.

---

## 7. Rollback, si algo va mal

```
# volver al exe de Fase 2.1 (ya verificado, hash idéntico al original)
copy "H:\KUKIA_CODE\backups\instalado_20261101_previo_f3\kukia-code.exe" "C:\Program Files\KUKIA CODE\kukia-code.exe"
```

O desinstalar desde `C:\Program Files\KUKIA CODE\uninstall.exe` y reinstalar el artefacto anterior.
Los datos de `%APPDATA%\KukiaCode` no se tocan en ningún caso.
