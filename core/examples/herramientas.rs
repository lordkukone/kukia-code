//! Diagnóstico de solo lectura del Tool Center.
//!
//! Usa una base temporal para no tocar la del usuario: los contadores de memoria,
//! cola y descubrimientos saldrán a cero, pero las sondas del sistema (GPU, MCP,
//! rutas) son reales.
//!
//! ```text
//! cargo run -p kukia-core --example herramientas
//! ```

use kukia_core::{mcp, memory::Database, tools};

fn main() {
    let tmp = std::env::temp_dir().join("kukia_diag_herramientas.db");
    let _ = std::fs::remove_file(&tmp);
    let db = Database::open(&tmp).expect("base temporal");

    let servidores = mcp::servidores(&db);
    println!("=== SERVIDORES MCP DETECTADOS ({}) ===", servidores.len());
    for s in &servidores {
        println!(
            "  {:<22} transporte={:<6} autorizado={:<5} tools={:<3} {}",
            s.name, s.transport, s.authorized, s.tools, s.note
        );
    }

    let resumen = tools::resumen(&db);
    println!("\n=== RESUMEN ===");
    println!(
        "  total={} core={} mcp={} servidores={} autorizados={} disponibles={} requieren_autorizacion={} sin_comprobar={}",
        resumen.total,
        resumen.core,
        resumen.mcp,
        resumen.servidores_mcp,
        resumen.autorizados,
        resumen.disponibles,
        resumen.requieren_autorizacion,
        resumen.sin_comprobar
    );

    println!("\n=== REGISTRO ===");
    for t in tools::registro(&db) {
        println!(
            "  {:<28} cat={:<14} origen={:<5} estado={:<22} {}",
            t.id, t.category, t.origin, t.state, t.detail
        );
    }

    let _ = std::fs::remove_file(&tmp);
}
