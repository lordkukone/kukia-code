//! Estado de VRAM de solo lectura: qué hay cargado, cuánta VRAM ocupa y si cada
//! modelo sigue instalado en disco. **No descarga nada.**
//!
//! ```text
//! cargo run -p kukia-core --example vram_estado
//! ```

use kukia_core::vram;

#[tokio::main]
async fn main() {
    let r = vram::report().await;

    println!("=== GPUs ===");
    for g in &r.gpus {
        println!(
            "  GPU{} {:<26} total {:>6} MB · usado {:>6} MB · libre {:>6} MB",
            g.index,
            g.name,
            g.total_mb,
            g.used_mb,
            g.free_mb
        );
    }

    println!("\n=== MOTORES ===");
    for e in &r.engines {
        println!(
            "  {:<9} online={:<5} modelos={:<3} VRAM={} MB · {}",
            e.engine,
            e.online,
            e.models.len(),
            e.vram_total_mb,
            e.note
        );
        for m in &e.models {
            let disco = vram::sigue_en_disco(&e.engine, &m.name).await;
            println!(
                "      · {:<52} VRAM {:>6} MB · disco={}",
                m.name,
                m.vram_mb,
                match disco {
                    Some(true) => "sí (intacto)",
                    Some(false) => "no aparece",
                    None => "no comprobable",
                }
            );
        }
    }

    println!("\n=== COMPETENCIA ===");
    println!("  {} {}", r.competition, r.warning);
    for rec in &r.recommendations {
        println!("  · {rec}");
    }
    println!("\n  Medido a las {}.", r.checked_at);
    println!("  (Este ejemplo no ha descargado ningún modelo.)");
}
