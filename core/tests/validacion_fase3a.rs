//! Validación de Fase 3A: permisos por proyecto y tests fuera del log/base reales.
//!
//! Ejecutar con:
//!   cargo test -p kukia-core --test validacion_fase3a -- --nocapture
//!
//! No sale a internet y no toca `%APPDATA%`: todas las rutas son temporales.

use kukia_core::logs;
use kukia_core::memory::{AgentPolicy, Database, Project};
use kukia_core::permissions::{self, Tool};

fn dir_temporal(nombre: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("kukia-3a-{nombre}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).expect("carpeta temporal");
    d
}

fn db_temporal(nombre: &str) -> Database {
    let d = dir_temporal(nombre);
    Database::open(&d.join("kukia.db")).expect("base temporal")
}

/// Política de agente que permite escribir y ejecutar en dos carpetas.
fn politica_amplia(a: &std::path::Path, b: &std::path::Path) -> AgentPolicy {
    let mut p = AgentPolicy::default_for("kukia-master");
    p.allow_write = true;
    p.allow_shell = true;
    p.allow_read = true;
    p.write_paths = format!("{};{}", a.display(), b.display());
    p
}

/* ------------------------- 1. Modelo de proyecto ------------------------- */

#[test]
fn un_proyecto_nuevo_nace_acotado_a_su_raiz() {
    let p = Project::new_project("Prueba", r"H:\KUKIA_CODE");
    assert!(!p.id.is_empty(), "debe tener id");
    assert_eq!(p.name, "Prueba");
    assert_eq!(p.root, r"H:\KUKIA_CODE");
    // Lee y escribe dentro de su raíz…
    assert_eq!(p.write_paths, r"H:\KUKIA_CODE");
    assert_eq!(p.read_paths, r"H:\KUKIA_CODE");
    assert!(p.allow_write);
    // …y NO ejecuta comandos hasta que se conceda.
    assert!(!p.allow_shell, "un proyecto nuevo no debe permitir shell por defecto");
    assert!(p.read_only_paths.is_empty());
    assert_eq!(Project::split(&p.write_paths), vec![r"H:\KUKIA_CODE".to_string()]);
}

#[test]
fn los_proyectos_persisten_y_no_se_duplican() {
    let db = db_temporal("persistencia");
    let mut p = Project::new_project("KUKIA CODE", r"H:\KUKIA_CODE");
    assert!(db.project_upsert(&p).unwrap(), "la primera vez es nuevo");
    assert!(!db.project_upsert(&p).unwrap(), "la segunda no duplica");

    let lista = db.projects_list().unwrap();
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].name, "KUKIA CODE");

    // Al actualizar se conserva la fecha de creación.
    let creado = lista[0].created_at.clone();
    p.allow_shell = true;
    p.write_paths = r"H:\KUKIA_CODE;H:\KUKIA_RESEARCH".into();
    assert!(!db.project_upsert(&p).unwrap());
    let guardado = db.project_get(&p.id).unwrap().expect("debe existir");
    assert!(guardado.allow_shell);
    assert_eq!(guardado.write_paths, r"H:\KUKIA_CODE;H:\KUKIA_RESEARCH");
    assert_eq!(guardado.created_at, creado, "created_at no se reescribe");

    assert!(db.project_get("no-existe").unwrap().is_none());
}

/* ------------------- 2. El ámbito de proyecto recorta ------------------- */

#[test]
fn escribir_fuera_del_proyecto_se_deniega_aunque_el_agente_pueda() {
    let a = dir_temporal("dentro");
    let b = dir_temporal("fuera");
    let politica = politica_amplia(&a, &b);
    let proyecto = Project::new_project("solo-a", &a.to_string_lossy());

    // El agente puede escribir en a y en b; el proyecto solo en a.
    let dentro = permissions::check_scoped(
        &politica,
        Some(&proyecto),
        Tool::Write,
        &a.join("ok.txt").to_string_lossy(),
        true,
    );
    assert!(dentro.allowed, "dentro del proyecto debe permitirse: {}", dentro.reason);

    let fuera = permissions::check_scoped(
        &politica,
        Some(&proyecto),
        Tool::Write,
        &b.join("no.txt").to_string_lossy(),
        true,
    );
    assert!(!fuera.allowed, "fuera del proyecto debe denegarse");
    assert!(
        fuera.reason.contains("fuera del proyecto"),
        "el motivo debe explicarlo: {}",
        fuera.reason
    );
}

#[test]
fn el_proyecto_no_amplia_lo_que_el_agente_no_tiene() {
    let a = dir_temporal("sinpermiso");
    let mut politica = politica_amplia(&a, &a);
    politica.allow_write = false; // el agente no puede escribir
    let proyecto = Project::new_project("proyecto", &a.to_string_lossy());

    let r = permissions::check_scoped(
        &politica,
        Some(&proyecto),
        Tool::Write,
        &a.join("x.txt").to_string_lossy(),
        true,
    );
    assert!(!r.allowed, "el proyecto no puede conceder lo que el agente no tiene");
    assert!(r.reason.contains("no tiene permiso de escritura"), "{}", r.reason);

    // Y sin proyecto tampoco puede: no hay atajo.
    let sin = permissions::check_scoped(
        &politica,
        None,
        Tool::Write,
        &a.join("x.txt").to_string_lossy(),
        true,
    );
    assert!(!sin.allowed);
}

#[test]
fn la_aprobacion_sigue_siendo_obligatoria_con_proyecto() {
    let a = dir_temporal("aprobacion");
    let politica = politica_amplia(&a, &a);
    let proyecto = Project::new_project("p", &a.to_string_lossy());
    let r = permissions::check_scoped(
        &politica,
        Some(&proyecto),
        Tool::Write,
        &a.join("x.txt").to_string_lossy(),
        false,
    );
    assert!(!r.allowed);
    assert!(r.reason.contains("aprobación"), "{}", r.reason);
}

#[test]
fn el_proyecto_puede_marcar_solo_lectura() {
    let a = dir_temporal("readonly");
    let politica = politica_amplia(&a, &a);
    let mut proyecto = Project::new_project("congelado", &a.to_string_lossy());
    proyecto.read_only_paths = a.join("congelado").to_string_lossy().to_string();
    std::fs::create_dir_all(a.join("congelado")).unwrap();

    let r = permissions::check_scoped(
        &politica,
        Some(&proyecto),
        Tool::Write,
        &a.join("congelado").join("x.txt").to_string_lossy(),
        true,
    );
    assert!(!r.allowed);
    assert!(r.reason.contains("solo lectura"), "{}", r.reason);
}

#[test]
fn la_lectura_fuera_del_proyecto_se_deniega_si_el_proyecto_lo_acota() {
    let a = dir_temporal("lectura-dentro");
    let b = dir_temporal("lectura-fuera");
    let politica = politica_amplia(&a, &b);
    let proyecto = Project::new_project("solo-a", &a.to_string_lossy());

    let ok = permissions::check_scoped(
        &politica,
        Some(&proyecto),
        Tool::Read,
        &a.join("nota.txt").to_string_lossy(),
        false,
    );
    assert!(ok.allowed, "{}", ok.reason);

    let no = permissions::check_scoped(
        &politica,
        Some(&proyecto),
        Tool::Read,
        &b.join("secreto.txt").to_string_lossy(),
        false,
    );
    assert!(!no.allowed);
    assert!(no.reason.contains("fuera del proyecto"), "{}", no.reason);
}

#[test]
fn ejecutar_exige_permiso_del_proyecto_y_estar_dentro() {
    let a = dir_temporal("shell-dentro");
    let b = dir_temporal("shell-fuera");
    let politica = politica_amplia(&a, &b);
    let mut proyecto = Project::new_project("p", &a.to_string_lossy());

    // El proyecto nace sin shell: se deniega aunque el agente sí pueda.
    let sin_shell = permissions::check_scoped(&politica, Some(&proyecto), Tool::Shell, "echo hola", true);
    assert!(!sin_shell.allowed);
    assert!(sin_shell.reason.contains("no permite ejecutar"), "{}", sin_shell.reason);

    // Se concede: dentro pasa, fuera no.
    proyecto.allow_shell = true;
    let dentro = permissions::check_cwd_in_scope_scoped(&politica, Some(&proyecto), &a.to_string_lossy());
    assert!(dentro.allowed, "{}", dentro.reason);

    let fuera = permissions::check_cwd_in_scope_scoped(&politica, Some(&proyecto), &b.to_string_lossy());
    assert!(!fuera.allowed);
    assert!(fuera.reason.contains("fuera del proyecto"), "{}", fuera.reason);

    // Y el comando en sí también pasa por el ámbito.
    let cmd = permissions::check_scoped(&politica, Some(&proyecto), Tool::Shell, "dir", true);
    assert!(cmd.allowed, "{}", cmd.reason);
}

#[test]
fn sin_proyecto_activo_todo_funciona_como_antes() {
    let a = dir_temporal("sin-proyecto");
    let politica = politica_amplia(&a, &a);
    let r = permissions::check_scoped(
        &politica,
        None,
        Tool::Write,
        &a.join("x.txt").to_string_lossy(),
        true,
    );
    assert!(r.allowed, "sin proyecto debe regir solo la política del agente");
}

/* ------------------- 3. Los tests no tocan el log real ------------------- */

#[test]
fn escribir_en_un_log_temporal_no_toca_el_log_de_la_aplicacion() {
    let real = logs::log_dir().join("kukia.log");
    let tam_antes = std::fs::metadata(&real).map(|m| m.len()).unwrap_or(0);

    let dir = dir_temporal("log");
    logs::append_to(&dir, "test", "esta linea no debe aparecer en el log real").unwrap();
    let contenido = logs::tail_from(&dir, 10);
    assert!(
        contenido.iter().any(|l| l.contains("no debe aparecer")),
        "debe escribirse en la carpeta temporal"
    );

    let tam_despues = std::fs::metadata(&real).map(|m| m.len()).unwrap_or(0);
    assert_eq!(
        tam_antes, tam_despues,
        "el log real ({}) no puede crecer al ejecutar tests",
        real.display()
    );

    let _ = std::fs::remove_file(dir.join("kukia.log"));
    let _ = std::fs::remove_dir(dir);
}

#[test]
fn las_bases_de_datos_de_prueba_son_temporales() {
    let db = db_temporal("aislada");
    let p = Project::new_project("temp", r"H:\KUKIA_CODE");
    db.project_upsert(&p).unwrap();
    assert_eq!(db.projects_list().unwrap().len(), 1);

    // La base real del usuario no debe recibir este proyecto.
    let real = kukia_core::config::data_dir().join("kukia.db");
    if real.exists() {
        let con = rusqlite::Connection::open_with_flags(
            &real,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        );
        if let Ok(c) = con {
            let existe: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='projects'",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if existe > 0 {
                let n: i64 = c
                    .query_row("SELECT COUNT(*) FROM projects WHERE name='temp'", [], |r| r.get(0))
                    .unwrap_or(0);
                assert_eq!(n, 0, "un proyecto de prueba no puede acabar en la base real");
            }
        }
    }
}
