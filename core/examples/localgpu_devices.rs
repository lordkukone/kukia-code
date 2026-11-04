//! Que dispositivos usaria KUKIA ahora mismo: GPUs del equipo y equipos por RPC.
//! Es lo mismo que pinta la vista de GPU, sin abrir la ventana.

use kukia_core::localgpu;

fn main() {
    let v = localgpu::dispositivos();
    println!("dispositivos: {}", v.len());
    let mut total = 0u64;
    for d in &v {
        total += d.vram_mb;
        println!(
            "  {:<42} {:>6} MB  {:<5} {:<13} {}",
            d.nombre,
            d.vram_mb,
            if d.remoto { "RPC" } else { "local" },
            if d.disponible { "listo" } else { "sin respuesta" },
            d.detalle
        );
    }
    println!("VRAM total: {:.1} GB", total as f64 / 1024.0);
}
