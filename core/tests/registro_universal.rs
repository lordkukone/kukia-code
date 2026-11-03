//! Detección real del registro universal en esta máquina.
//!
//! No afirma qué hay instalado: lo imprime y comprueba la coherencia de los
//! estados (nadie puede estar «conectado» sin estar «detectado», etc.).

use kukia_core::registry;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn el_registro_detecta_lo_que_hay() {
    let inv = registry::inventario().await;

    println!(
        "sistemas={} instalados={} conectados={} operativos={}",
        inv.total, inv.instalados, inv.conectados, inv.operativos
    );
    for s in &inv.sistemas {
        println!(
            "{:<9} {:<14} inst={:<5} det={:<5} con={:<5} op={:<5} v{:<12} {}",
            s.id,
            s.categoria,
            s.instalado,
            s.detectado,
            s.conectado,
            s.operativo,
            if s.version.is_empty() { "-" } else { &s.version },
            s.detalle
        );
        if let Some(e) = &s.error {
            println!("            error: {e}");
        }
    }
    println!("no detectados: {:?}", inv.no_detectados);

    assert!(inv.total >= 15, "debe cubrir la lista pedida");
    for s in &inv.sistemas {
        if s.conectado {
            assert!(s.detectado, "{} conectado sin detectar", s.id);
            assert!(s.instalado, "{} conectado sin instalado", s.id);
            assert!(
                !s.endpoint.is_empty(),
                "{} conectado sin endpoint probado",
                s.id
            );
        }
        if s.operativo {
            assert!(!s.version.is_empty(), "{} operativo sin versión", s.id);
        }
        assert!(
            !s.capacidades.is_empty() && (!s.limitaciones.is_empty() || s.id == "kukia"),
            "{} debe declarar capacidades y límites",
            s.id
        );
    }

    // KUKIA siempre está: es el propio programa.
    let kukia = inv.sistemas.iter().find(|s| s.id == "kukia").unwrap();
    assert!(kukia.instalado, "KUKIA debe detectarse a sí mismo");
    assert!(!kukia.version.is_empty(), "KUKIA debe declarar su versión");
}
