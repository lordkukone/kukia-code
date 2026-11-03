//! Imprime el catálogo universal real y una prueba del router.
//!
//! Solo lectura: no carga, descarga ni modifica nada de los motores. Sirve para
//! comprobar en esta máquina qué se puede enumerar de verdad.
//!
//! ```text
//! cargo run -p kukia-core --example catalogo
//! ```

use kukia_core::catalog::{self, Need};
use kukia_core::config;

#[tokio::main]
async fn main() {
    let cfg = config::load_config();
    let report = catalog::collect(&cfg).await;

    println!("=== COBERTURA POR MOTOR ===");
    for c in &report.coverage {
        println!(
            "  {:<10} alcanzable={:<5} catalogo_api={:<5} modelos={:<3} {}",
            c.engine, c.reachable, c.catalog_api, c.models, c.detail
        );
    }

    println!("\n=== MODELOS ENUMERADOS ({}) ===", report.total);
    for m in &report.models {
        println!(
            "  {:<40} motor={:<9} tipo={:<10} ctx={:<8} cargado={:<5} caps=[{}]{}",
            m.id,
            m.engine,
            m.kind,
            m.context_length
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".into()),
            m.loaded,
            m.capabilities.join(","),
            if m.inferred { " (deducido)" } else { "" }
        );
    }

    println!("\n=== ROUTER: texto ===");
    let r = catalog::enrutar(&report.models, &Need::default());
    println!("  {}", r.note);
    if let Some(c) = &r.chosen {
        println!("  elegido: {} ({:.0} pts) · {:?}", c.key, c.score, c.reasons);
    }
    for alt in &r.alternatives {
        println!("  alternativa: {} ({:.0} pts)", alt.key, alt.score);
    }

    println!("\n=== ROUTER: vision ===");
    let need = Need {
        capability: "vision".into(),
        ..Default::default()
    };
    let rv = catalog::enrutar(&report.models, &need);
    println!("  {}", rv.note);
    if let Some(c) = &rv.chosen {
        println!("  elegido: {} ({:.0} pts)", c.key, c.score);
    }

    println!("\n=== ROUTER: imagen ===");
    let ni = Need {
        capability: "imagen".into(),
        ..Default::default()
    };
    let ri = catalog::enrutar(&report.models, &ni);
    println!("  {}", ri.note);
    if let Some(c) = &ri.chosen {
        println!("  elegido: {} ({:.0} pts)", c.key, c.score);
    }

    println!("\nGenerado en {} ms.", report.generated_ms);
}
