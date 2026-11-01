//! Validación de Fase 3.0 — permisos aplicados y auditoría real.
//!
//! Estas pruebas comprueban la cadena completa que usa el puente Tauri:
//! política del agente → `permissions::check` → decisión, y que toda decisión
//! queda registrada en la tabla `audit`.
//!
//! Son locales y deterministas: no salen a la red ni tocan nada fuera de `%TEMP%`.

use kukia_core::{
    memory::{AgentPolicy, Database},
    permissions::{self, check, check_cwd_in_scope, Tool},
};
use std::path::PathBuf;

fn db_temporal(nombre: &str) -> Database {
    let dir = std::env::temp_dir().join("kukia_fase30");
    std::fs::create_dir_all(&dir).expect("crear dir temporal");
    let path: PathBuf = dir.join(format!("{nombre}.db"));
    let _ = std::fs::remove_file(&path);
    Database::open(&path).expect("abrir sqlite")
}

/* --------------------------- Semilla de políticas --------------------------- */

#[test]
fn las_politicas_se_siembran_y_son_idempotentes() {
    let db = db_temporal("semilla");
    let lista = db.policies_list().unwrap();
    assert!(!lista.is_empty(), "debe haber políticas sembradas");
    println!("políticas sembradas: {}", lista.len());

    // Cualquier agente desconocido devuelve una política conservadora, no un error.
    let maestro = db.policy_get("kukia-master").unwrap();
    assert_eq!(maestro.agent, "kukia-master");
    assert!(maestro.allow_write, "el maestro debe poder escribir");
    assert!(!maestro.write_paths.is_empty(), "debe tener rutas");
    println!("rutas del maestro: {}", maestro.write_paths);

    // Sembrar otra vez no duplica ni pisa lo que el usuario haya cambiado.
    let mut cambiada = maestro.clone();
    cambiada.allow_download = true;
    db.policy_set(&cambiada).unwrap();
    let creadas = db.seed_policies().unwrap();
    assert_eq!(creadas, 0, "no debe volver a sembrar si ya hay políticas");
    assert!(db.policy_get("kukia-master").unwrap().allow_download);
}

#[test]
fn el_cerebro_queda_como_solo_lectura() {
    let db = db_temporal("cerebro");
    let p = db.policy_get("kukia-master").unwrap();
    assert!(
        p.read_only_paths.to_lowercase().contains("cerebro"),
        "I:\\CEREBRO debe estar marcado como solo lectura: {}",
        p.read_only_paths
    );
    if std::path::Path::new(r"I:\CEREBRO").is_dir() {
        let c = check(&p, Tool::Write, r"I:\CEREBRO\prueba.txt", true);
        assert!(!c.allowed, "escribir en el CEREBRO debe estar prohibido");
        assert!(c.reason.contains("solo lectura"), "{}", c.reason);
        println!("OK CEREBRO protegido -> {}", c.reason);
    } else {
        println!("AVISO: I:\\CEREBRO no está montado; comprobación parcial");
    }
}

/* --------------------------- Escritura --------------------------- */

#[test]
fn escritura_fuera_de_las_rutas_se_deniega() {
    let db = db_temporal("fuera");
    let p = db.policy_get("kukia-master").unwrap();
    let c = check(&p, Tool::Write, r"C:\Windows\System32\malo.dll", true);
    assert!(!c.allowed, "no debe poder escribir en Windows");
    assert!(c.reason.contains("fuera"), "{}", c.reason);

    // Y queda auditado como denegado.
    db.audit_log(
        "kukia-master",
        "files.write",
        "escribir archivo",
        r"C:\Windows\System32\malo.dll",
        true,
        "denegado",
        &c.reason,
    )
    .unwrap();
    let filas = db.audit_search("", "files.write", None, 10).unwrap();
    assert_eq!(filas.len(), 1);
    assert_eq!(filas[0].result, "denegado");
    assert!(
        filas[0].approved,
        "el usuario sí aprobó; lo que denegó fue la política"
    );
    println!("OK denegado y auditado: {}", filas[0].error);
}

#[test]
fn escritura_dentro_de_las_rutas_pasa() {
    let db = db_temporal("dentro");
    let p = db.policy_get("kukia-master").unwrap();
    let destino = std::env::temp_dir().join("kukia_fase30_dentro.txt");
    let c = check(&p, Tool::Write, destino.to_str().unwrap(), true);
    assert!(c.allowed, "debería permitir escribir en el perfil: {}", c.reason);

    // Sin aprobación explícita, no.
    let sin = check(&p, Tool::Write, destino.to_str().unwrap(), false);
    assert!(!sin.allowed);
    assert!(sin.reason.contains("aprobaci"), "{}", sin.reason);
}

#[test]
fn politica_sin_rutas_deniega_todo() {
    let mut p = AgentPolicy::default_for("prueba");
    p.allow_write = true;
    p.write_paths = String::new();
    let c = check(&p, Tool::Write, r"C:\cualquier.txt", true);
    assert!(!c.allowed);
    assert!(c.reason.contains("ninguna ruta"), "{}", c.reason);
}

/* --------------------------- Terminal --------------------------- */

#[test]
fn comando_destructivo_exige_permiso_de_escritura() {
    let mut p = AgentPolicy::default_for("prueba");
    p.allow_shell = true;
    p.allow_write = false;
    let c = check(&p, Tool::Shell, "rd /s /q H:\\datos", true);
    assert!(!c.allowed, "un borrado no puede ejecutarse sin permiso de escritura");
    assert!(c.reason.contains("destructivo"), "{}", c.reason);
    println!("OK destructivo bloqueado -> {}", c.reason);

    // Un comando normal sí pasa con aprobación.
    let normal = check(&p, Tool::Shell, "Get-Date", true);
    assert!(normal.allowed, "{}", normal.reason);
}

#[test]
fn el_terminal_solo_corre_en_rutas_autorizadas() {
    let db = db_temporal("cwd");
    let p = db.policy_get("kukia-master").unwrap();

    let dentro = check_cwd_in_scope(&p, r"H:\KUKIA_CODE");
    assert!(dentro.allowed, "{}", dentro.reason);

    let fuera = check_cwd_in_scope(&p, r"C:\Windows\System32");
    assert!(!fuera.allowed);
    assert!(fuera.reason.contains("fuera"), "{}", fuera.reason);
}

/* --------------------------- Auditoría --------------------------- */

#[test]
fn la_auditoria_se_filtra_y_se_exporta() {
    let db = db_temporal("auditoria");
    db.audit_log("kukia-master", "files.write", "escribir archivo", r"H:\a.txt", true, "ok", "")
        .unwrap();
    db.audit_log("kukia-master", "terminal.run", "ejecutar comando", "H:\\ > dir", true, "ok", "")
        .unwrap();
    db.audit_log("kukia-coder", "files.read", "leer archivo", r"H:\b.txt", false, "ok", "")
        .unwrap();

    let todas = db.audit_search("", "", None, 100).unwrap();
    assert_eq!(todas.len(), 3);

    let solo_terminal = db.audit_search("", "terminal.run", None, 100).unwrap();
    assert_eq!(solo_terminal.len(), 1);
    assert_eq!(solo_terminal[0].agent, "kukia-master");

    let denegadas = db.audit_search("", "", Some(false), 100).unwrap();
    assert_eq!(denegadas.len(), 1);
    assert_eq!(denegadas[0].agent, "kukia-coder");

    let por_texto = db.audit_search("b.txt", "", None, 100).unwrap();
    assert_eq!(por_texto.len(), 1);

    let csv = std::env::temp_dir().join("kukia_fase30_auditoria.csv");
    let filas = db.audit_export_csv(csv.to_str().unwrap()).unwrap();
    assert_eq!(filas, 3);
    let contenido = std::fs::read_to_string(&csv).unwrap();
    assert!(contenido.starts_with("id;fecha;agente;"));
    assert!(contenido.contains("terminal.run"));
    println!("OK CSV con {filas} filas en {}", csv.display());
}

/* --------------------------- Combinación completa --------------------------- */

#[test]
fn la_politica_manda_sobre_la_aprobacion() {
    // Aprobación sin política no sirve de nada; política sin aprobación tampoco.
    let db = db_temporal("combinada");
    let mut p = db.policy_get("kukia-master").unwrap();
    p.allow_write = false;
    db.policy_set(&p).unwrap();
    let p = db.policy_get("kukia-master").unwrap();

    let destino = std::env::temp_dir().join("kukia_fase30_combinada.txt");
    let c = check(&p, Tool::Write, destino.to_str().unwrap(), true);
    assert!(!c.allowed, "sin allow_write no debe escribir aunque se apruebe");
    assert!(c.reason.contains("no tiene permiso"), "{}", c.reason);

    assert_eq!(permissions::Tool::parse("files.write"), Some(Tool::Write));
    assert_eq!(permissions::Tool::parse("terminal.run"), Some(Tool::Shell));
}
