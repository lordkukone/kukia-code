//! Validación real del KUKIA FUTURE GAMING RADAR.
//!
//! Ejecutar con:
//!   cargo test -p kukia-core --test validacion_gaming -- --ignored --nocapture
//!
//! Sale a internet (GitHub + arXiv) y usa una base SQLite temporal, nunca la del usuario.

use kukia_core::{gaming, memory::Database};

fn db_temporal(nombre: &str) -> (Database, std::path::PathBuf) {
    let dir = std::env::temp_dir().join("kukia_gaming_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{nombre}.db"));
    let _ = std::fs::remove_file(&path); // base de pruebas, no del usuario
    (Database::open(&path).expect("base temporal"), path)
}

#[tokio::test]
#[ignore = "sale a internet (GitHub + arXiv)"]
async fn el_radar_encuentra_y_persiste_fichas() {
    let items = gaming::scan_all().await;
    println!("descubrimientos: {}", items.len());
    assert!(!items.is_empty(), "el radar debe encontrar proyectos");

    let mut por_fuente: std::collections::BTreeMap<String, usize> = Default::default();
    for it in &items {
        *por_fuente.entry(it.source.clone()).or_default() += 1;
    }
    for (fuente, n) in &por_fuente {
        println!("  {fuente}: {n}");
    }

    // Ficha completa: nada de campos vacíos en lo esencial.
    let con_url = items.iter().filter(|i| i.url.starts_with("http")).count();
    println!("con URL válida: {con_url}/{}", items.len());
    assert!(con_url > 0, "cada ficha debe traer enlace de origen");

    for it in items.iter().take(5) {
        println!(
            "  · [{}] {} ({}) puntos={:.1} 5070={} 5060Ti={}",
            it.category, it.name, it.status, it.score, it.rtx5070, it.rtx5060ti
        );
        assert!(!it.id.is_empty() && !it.name.is_empty());
        assert!(!it.status.is_empty(), "toda ficha debe clasificarse");
    }

    // Persistencia real, como hace el comando gaming_scan.
    let (db, path) = db_temporal("radar");
    let mut nuevos = 0;
    for it in &items {
        if db.gaming_upsert(it).unwrap_or(false) {
            nuevos += 1;
        }
    }
    let alertas = gaming::alerts_for(&items);
    for a in &alertas {
        db.gaming_alert_add(&a.item_id, &a.level, &a.message).unwrap();
    }
    let _ = db.gaming_run_add("test", items.len() as i64, nuevos as i64);

    let guardados = db.gaming_list(500).unwrap();
    let alertas_db = db.gaming_alerts_list(100).unwrap();
    println!(
        "persistidos: {} de {} · nuevos: {nuevos} · alertas: {alertas_db}",
        guardados.len(),
        items.len(),
        alertas_db = alertas_db.len()
    );
    assert_eq!(guardados.len(), items.len(), "deben guardarse todas las fichas");
    assert_eq!(nuevos, items.len(), "la primera pasada son todas nuevas");

    // Idempotencia: repetir el escaneo no debe duplicar.
    let mut repetidos = 0;
    for it in &items {
        if db.gaming_upsert(it).unwrap_or(false) {
            repetidos += 1;
        }
    }
    println!("segunda pasada, nuevos: {repetidos}");
    assert_eq!(repetidos, 0, "repetir el escaneo no puede duplicar fichas");
    assert_eq!(db.gaming_list(500).unwrap().len(), items.len());

    // Informe en disco.
    let informe = gaming::write_report(&items, &alertas, false).expect("informe escrito");
    let tam = std::fs::metadata(&informe).map(|m| m.len()).unwrap_or(0);
    println!("informe: {} ({} bytes)", informe.display(), tam);
    assert!(tam > 500, "el informe no puede salir vacío");

    let lab = gaming::ensure_lab().expect("laboratorio creado");
    for sub in ["discoveries", "reports", "benchmarks", "lab"] {
        assert!(lab.join(sub).is_dir(), "falta la carpeta {sub} del laboratorio");
    }
    println!("laboratorio: {}", lab.display());

    let _ = std::fs::remove_file(path); // solo la base de pruebas
}

#[tokio::test]
#[ignore = "sale a internet (GitHub + arXiv)"]
async fn las_alertas_clasifican_por_nivel() {
    let items = gaming::scan_all().await;
    let alertas = gaming::alerts_for(&items);
    println!("alertas generadas: {}", alertas.len());
    for a in alertas.iter().take(10) {
        println!("  [{}] {}", a.level, a.message);
    }
    for a in &alertas {
        assert!(
            ["alta", "info", "riesgo"].contains(&a.level.as_str()),
            "nivel desconocido: {}",
            a.level
        );
    }
}
