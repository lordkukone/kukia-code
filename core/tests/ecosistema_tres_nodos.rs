//! Prueba de integración del ecosistema con los tres nodos reales de esta máquina.
//!
//! Comprueba que KUKIA recoge datos de verdad de sí misma, de OpenCode y de
//! Freebuff, y que el enrutador no promete lo que no puede cumplir.
//!
//! Para OpenCode se puede indicar su servidor:
//! `KUKIA_TEST_OPENCODE_URL=http://127.0.0.1:4600`

use kukia_core::{config, ecosystem};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn el_panel_recoge_los_tres_nodos() {
    let cfg = config::load_config();
    let data = config::data_dir().join("kukia.db");
    let url = std::env::var("KUKIA_TEST_OPENCODE_URL").ok();

    let eco = ecosystem::todo(&cfg, &data, url).await;

    for n in &eco.nodos {
        println!(
            "{}: instalado={} conectado={} v{} modelos={} agentes={} herramientas={} procesos={} · {}",
            n.nombre, n.instalado, n.conectado, n.version, n.modelos, n.agentes, n.herramientas,
            n.procesos, n.detalle
        );
    }
    println!(
        "catálogo: locales={} opencode={} agentes_kukia={} agentes_opencode={} mcp={}",
        eco.catalogo.modelos_locales,
        eco.catalogo.modelos_opencode,
        eco.catalogo.agentes_kukia,
        eco.catalogo.agentes_opencode,
        eco.catalogo.servidores_mcp
    );
    for nota in &eco.notas {
        println!("nota: {nota}");
    }

    assert_eq!(eco.nodos.len(), 3, "deben salir los tres nodos");

    let kukia = eco.nodos.iter().find(|n| n.id == "kukia").expect("nodo kukia");
    assert!(kukia.instalado && kukia.conectado);
    assert!(kukia.modelos > 0, "el catálogo local debe tener modelos");

    let opencode = eco
        .nodos
        .iter()
        .find(|n| n.id == "opencode")
        .expect("nodo opencode");
    assert!(opencode.instalado, "OpenCode debe detectarse en esta máquina");
    assert!(!opencode.version.is_empty(), "debe leer su versión");
    assert!(
        opencode.modelos > 0,
        "debe ver modelos (por API o por CLI): {}",
        opencode.detalle
    );

    let freebuff = eco
        .nodos
        .iter()
        .find(|n| n.id == "freebuff")
        .expect("nodo freebuff");
    assert!(freebuff.instalado, "Freebuff debe detectarse: {}", freebuff.ruta);
    assert!(!freebuff.version.is_empty(), "debe leer su versión");
    assert!(
        freebuff.detalle.contains("proyecto"),
        "debe contar proyectos/hilos: {}",
        freebuff.detalle
    );
    assert!(
        freebuff
            .limitaciones
            .iter()
            .any(|l| l.contains("No publica API local")),
        "debe declarar que no hay API oficial"
    );

    // El enrutador nunca debe declarar automatizable una tarea de Freebuff.
    let r = ecosystem::enrutar("automatiza un workflow largo de investigación", &eco.nodos);
    println!("router: {} · automatizable={} · {}", r.sistema, r.automatizable, r.como);
    if r.sistema == "freebuff" {
        assert!(!r.automatizable);
    }

    let r2 = ecosystem::enrutar("repara este bug y compila", &eco.nodos);
    println!("router: {} · automatizable={}", r2.sistema, r2.automatizable);
    assert_eq!(r2.sistema, "opencode");
}
