# KUKIA CODE — FASE 3.4: AVISO AUTOMÁTICO DE COMPETENCIA DE VRAM

**Fecha:** 01/11/2026 · **Autor:** Xoxete

## Qué se pedía

Que KUKIA CODE avise **solo**, sin abrir la vista VRAM Guard, en cuanto dos motores tengan
modelos cargados a la vez.

## Qué se hizo

| Pieza | Archivo | Detalle |
|---|---|---|
| Función pura | `core/src/vram.rs` | `evaluar_competencia(&[EngineState]) -> VramAlert` con `competing`, `count`, `detalle`, `models` y `message` |
| Reutilización | `core/src/vram.rs` | `report()` ahora usa esa función para `competition` y `warning` (un solo criterio, no dos) |
| Vigilante | `src-tauri/src/lib.rs` | Tarea de fondo: consulta cada **20 s**, primera pasada inmediata al arrancar; emite `vram://competencia` y `vram://competencia_fin` solo cuando cambia la firma (motores + nº de modelos) |
| Registro | `logs::append` | `warn` al aparecer, `info` al resolverse |
| Tipos | `app/src/lib/api.ts` | `VramAlert` |
| Interfaz | `app/src/App.tsx` | Escucha los dos eventos; banner global encima del contenido con botón «Abrir VRAM Guard» y ✕ para ocultar |
| Estilo | `app/src/styles.css` | `.global-alert`, franja ámbar con borde de aviso |

Regla de conteo: **solo cuentan los motores con modelos cargados**. Un motor encendido y vacío no compite.

## Pruebas

```
cargo test --release -p kukia-core
  98 pasan · 12 ignorados · 0 fallos
  (4 nuevas, puras, sin red)
  vram::tests::dos_motores_con_modelos_disparan_el_aviso ... ok
  vram::tests::un_solo_motor_no_es_competencia ... ok
  vram::tests::motores_encendidos_pero_vacios_no_compiten ... ok
  vram::tests::el_aviso_cuenta_modelos_de_todos_los_motores ... ok
```

## Prueba en la app instalada (no simulada en consola)

Con ComfyUI reteniendo 1.177 MB (cuenta como modelo residente) se levantó un stub en el puerto
1234 que declara un modelo `loaded` — mismo endpoint que consulta el código real
(`/api/v0/models`), sin gastar VRAM:

| Momento | Log | Pantalla |
|---|---|---|
| 15:11:41 | `[warn] vram guard: COMPETENCIA DE VRAM: 2 motores... lmstudio (1), comfyui (1)` | Banner ámbar visible en Chat, sin haber abierto VRAM Guard |
| 15:14:04 | `[info] vram guard: competencia resuelta` | Banner desaparece solo |

Capturas: `tmp_capturas/aviso_competencia.png` y `tmp_capturas/aviso_fin.png`.

## Hallazgo colateral (y arreglado): la CSP dejaba la ventana en blanco

Al aplicar la CSP de la mejora anterior, la app instalada mostraba **ventana blanca sin contenido**
(1 solo elemento accesible). Diagnóstico:

1. La misma política, inyectada como `<meta>` sobre el `dist` real en Chromium, **pintaba**
   (18 vistas) → el frontend no era el problema.
2. Quitando `script-src 'self'`, `object-src`, `base-uri`, `form-action` y `frame-ancestors` y
   dejando la forma que documenta Tauri, la ventana **volvió a pintar**.

CSP final, probada en la app instalada:

```
default-src 'self'; img-src 'self' data: blob: asset: http://asset.localhost;
style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost
```

**Limitación honesta:** no he aislado cuál de las cuatro directivas retiradas rompía el webview.
La sospechosa principal es `frame-ancestors`, que no es válido en una CSP entregada por `<meta>`
(Chromium lo ignora y solo avisa; Tauri podría estar manejándolo peor). La política actual está
verificada en la app instalada, pero es más laxa de lo que quería.

## Artefactos

| Qué | Valor |
|---|---|
| Instalador | `KUKIA CODE_0.1.0_x64-setup.exe` · 2.934.300 B |
| SHA-256 instalador | `509646014c93dd858c6f6fededccdb883c813423aaa2e6461bdf910faadf088f` |
| SHA-256 exe release | `de2d933c9a6b727ff8fd345fcef05323cfc19feffd3594e165376dfd83764f09` |
| SHA-256 exe instalado | `e4149cfb0b5b4b6d80a23a7162152c4662d5e67ed7239e2a91417c63694e71b7` |
| Backup previo | `backups/instalado_20261101_previo_aviso_vram/` (`f1a79f20…`) |
| Etiqueta de cabecera | pasó de «Fase 2» a «**Fase 3**» |

## Pendiente

- Aislar la directiva exacta que rompía la CSP y recuperar `object-src 'none'` si es posible.
- El aviso es **dentro de la app**: si la ventana está minimizada no hay notificación del sistema
  (requeriría `tauri-plugin-notification`).
- El intervalo es fijo (20 s) y no se puede configurar desde Ajustes.
