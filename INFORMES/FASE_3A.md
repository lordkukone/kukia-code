# FASE 3A — PERMISOS POR PROYECTO Y TESTS FUERA DE LOS DATOS REALES

**Fecha:** 01/11/2026 · **Autor:** Xoxete (para Kuki / LordKukone)
**Proyecto:** `H:\KUKIA_CODE` (el existente, sin duplicar nada)
**Estado:** ✅ entregado y verificado.

---

## 1. Qué se ha construido

### 1.1 Permisos por proyecto

Antes solo existía la política **por agente**, con una allowlist global. Eso obliga a elegir entre
una lista demasiado ancha (puede escribir en todo lo que necesites algún día) o demasiado estrecha
(te bloquea en tu propio trabajo). Ahora hay **ámbito de proyecto**:

```
permitido = política del agente  Y  ámbito del proyecto
```

| Regla | Comportamiento |
|---|---|
| El proyecto **recorta** | Escribir o ejecutar fuera de su raíz se deniega aunque el agente tenga permiso global |
| El proyecto **nunca amplía** | Si el agente no puede escribir, el proyecto no lo concede |
| Nace conservador | Escritura sí (dentro de la raíz), **shell no** |
| Solo lectura propia | `read_only_paths` del proyecto bloquea escritura y ejecución en esas rutas |
| Aprobación | Sigue siendo obligatoria con o sin proyecto |

**Piezas:**

| Capa | Archivo | Qué aporta |
|---|---|---|
| Datos | [`core/src/memory.rs`](../core/src/memory.rs) | `struct Project` + tabla `projects` + `project_upsert/get/projects_list` |
| Reglas | [`core/src/permissions.rs`](../core/src/permissions.rs) | `check_scoped` y `check_cwd_in_scope_scoped` |
| Config | [`core/src/config.rs`](../core/src/config.rs) | `active_project` persistido en `config.json` |
| Comandos | [`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) | `projects_list`, `project_create`, `project_update`, `project_activate`, `project_active` |
| UI | [`app/src/views/Security.tsx`](../app/src/views/Security.tsx) | Tarjeta **Proyectos**: alta, activar/desactivar, estado |
| Pruebas | [`core/tests/validacion_fase3a.rs`](../core/tests/validacion_fase3a.rs) | 11 tests |

**Dónde se aplica** (los cinco puntos de control que ya tenía la política del agente):
`files_read`, `files_write`, `run_command`, `run_command_stream` y `audit_export`.

**Auditoría:** `projects.create`, `projects.update`, `projects.activate` quedan en la tabla `audit`
y en el log. `project_update` exige `approved: true`, igual que `policy_save`.

### 1.2 Tests fuera del log y de la base reales

| Problema | Estado |
|---|---|
| `logs::tests::append_and_tail` escribía en `%APPDATA%\KukiaCode\logs\kukia.log` | ✅ corregido: escribe en carpeta temporal |
| Cada `cargo test` dejaba `[test] linea de prueba kukia` en el log de producción | ✅ ya no ocurre (medido) |
| Bases de datos de test | ✅ ya eran temporales; ahora hay un test que lo verifica contra la base real |

**Cambios:**

- `logs::append_to(dir, …)` y `logs::tail_from(dir, …)`: permiten escribir en una carpeta concreta.
- `logs::log_dir()` respeta `KUKIA_LOG_DIR` (pruebas y entornos aislados).
- `logs::path()` para mostrar en la interfaz dónde está el log.

---

## 2. Pruebas ejecutadas (salida real)

```
cargo test -p kukia-core --test validacion_fase3a
running 11 tests
test un_proyecto_nuevo_nace_acotado_a_su_raiz ... ok
test el_proyecto_no_amplia_lo_que_el_agente_no_tiene ... ok
test la_aprobacion_sigue_siendo_obligatoria_con_proyecto ... ok
test el_proyecto_puede_marcar_solo_lectura ... ok
test sin_proyecto_activo_todo_funciona_como_antes ... ok
test la_lectura_fuera_del_proyecto_se_deniega_si_el_proyecto_lo_acota ... ok
test ejecutar_exige_permiso_del_proyecto_y_estar_dentro ... ok
test escribir_fuera_del_proyecto_se_deniega_aunque_el_agente_pueda ... ok
test escribir_en_un_log_temporal_no_toca_el_log_de_la_aplicacion ... ok
test las_bases_de_datos_de_prueba_son_temporales ... ok
test los_proyectos_persisten_y_no_se_duplican ... ok
test result: ok. 11 passed; 0 failed
```

**La prueba del log, medida en bytes:**

```
log real antes de la suite:   1557 bytes · 16 líneas [test] antiguas
log real después de la suite: 1557 bytes · 16 líneas [test] antiguas   ← no creció
(comprobado otra vez ejecutando los 3 tests de logs: 16 antes, 16 después)
```

Y la suite completa:

```
cargo test -p kukia-core
  59 unitarios       OK   (antes 56; +2 de logs, +1 de config)
   8 fase21          OK
   9 fase30          OK
  11 fase3a          OK
   2 gaming/gaming + 3 anime_real  (ignorados por red/ComfyUI)
cd app && npm run build     → tsc + vite OK (296,21 kB)
cargo check -p kukia-code   → OK
```

---

## 3. Comportamiento demostrado con casos concretos

| Caso | Resultado |
|---|---|
| Proyecto `solo-a` activo, agente puede escribir en A y B | escribir en A ✅ · escribir en B ❌ «fuera del proyecto» |
| Agente **sin** permiso de escritura + proyecto que la permite | ❌ «no tiene permiso de escritura» (el proyecto no amplía) |
| Escritura dentro del proyecto sin `approved` | ❌ «falta aprobación explícita» |
| Proyecto con `read_only_paths` en una subcarpeta | ❌ «es de solo lectura en el proyecto» |
| Proyecto sin shell (el valor por defecto) | ❌ «no permite ejecutar comandos» |
| Proyecto con shell y cwd fuera de su raíz | ❌ «ejecución fuera del proyecto» |
| Sin proyecto activo | ✅ manda solo la política del agente (comportamiento anterior intacto) |

---

## 4. Lo que NO se ha hecho (honesto)

1. **No se ha recompilado el instalador.** La app instalada sigue siendo la de Fase 3.0; esto está
   en el código y en los tests, no en tu escritorio.
2. **La UI no se ha pulsado en una ventana real.** La tarjeta de Proyectos compila y tipa, pero no
   se ha hecho clic en «Dar de alta» dentro de la app.
3. **No hay borrado de proyectos.** A propósito: mis reglas dicen no borrar sin tu permiso. Si
   quieres, añado archivar (marcar inactivo sin borrar).
4. **El log de producción conserva 16 líneas `[test]` antiguas** (de las ejecuciones de hoy,
   antes del arreglo). No las he borrado: quitar líneas de un log es tocar datos tuyos.
   Dime y las limpio. Comprobado que no crece más: 16 antes de la suite y 16 después.
5. **El ámbito de proyecto no cubre todavía** ni red ni descargas por proyecto (solo lectura,
   escritura y ejecución de comandos). Se puede añadir cuando haga falta.

---

## 5. Siguiente paso natural

- Recompilar el instalador para que esto llegue a la app instalada (junto con lo de Fase 3.1/3.2,
  que tampoco está).
- Pintar el proyecto activo en la barra superior para no tener que entrar en Seguridad a mirarlo.
- Que las sesiones de chat guarden a qué proyecto pertenecen.
