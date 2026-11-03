//! Comprobación real del KUKIA UNIVERSAL REALITY CONVERTER (RC1 + RC2).
//!
//! Uso:
//!   cargo run --example studio_check                      (motores + catálogo)
//!   cargo run --example studio_check -- <archivo>         (+ preflight del archivo)
//!
//! No ejecuta difusión: solo sondea motores, cruza el catálogo y analiza el
//! material con ffprobe/ffmpeg y el VLM local.

use kukia_core::studio;

#[tokio::main]
async fn main() {
    println!("===== MOTORES =====");
    for m in studio::engines::todos().await {
        println!(
            "- {:<8} {:<10} {}  v{}\n    {} · {}\n    capacidades: {}",
            m.nombre,
            if m.online { "ONLINE" } else { "offline" },
            m.url,
            m.version,
            m.detalle,
            m.gpu,
            if m.capacidades.is_empty() {
                "ninguna".to_string()
            } else {
                m.capacidades.join(", ")
            }
        );
    }

    let p = studio::presupuesto();
    println!("\n===== PRESUPUESTO =====");
    for v in &p.vram {
        println!(
            "- GPU{} {}: {} MiB libres de {} (usada {} MiB)",
            v.index, v.nombre, v.libre_mb, v.total_mb, v.usada_mb
        );
    }
    println!("- RAM: {:.1} GB libres de {:.1} GB", p.ram_libre_gb, p.ram_total_gb);

    println!("\n===== CATÁLOGO =====");
    match studio::diagnostico_estilos().await {
        Ok(c) => {
            let mut cuenta = std::collections::BTreeMap::new();
            for e in &c {
                *cuenta.entry(e.estado_real.clone()).or_insert(0) += 1;
                println!(
                    "- {:<34} {:<16} {}{}",
                    e.id,
                    e.estado_real,
                    if e.medido.is_empty() { "sin medir".to_string() } else { e.medido.clone() },
                    if e.faltan.is_empty() {
                        String::new()
                    } else {
                        format!(" · faltan {}", e.faltan.len())
                    }
                );
            }
            println!("\nresumen por estado: {cuenta:?}");
        }
        Err(e) => println!("ERROR: {e}"),
    }

    let args: Vec<String> = std::env::args().collect();
    if let Some(ruta) = args.get(1) {
        println!("\n===== PREFLIGHT: {ruta} =====");
        match studio::preflight(ruta, None, None, 0.30).await {
            Ok(pf) => {
                println!(
                    "- sondeo: {} · {}×{} · {} fps · {:.2} s · audio {} · {} · {} bytes",
                    pf.sondeo.tipo, pf.sondeo.ancho, pf.sondeo.alto, pf.sondeo.fps,
                    pf.sondeo.segundos, pf.sondeo.audio, pf.sondeo.codec, pf.sondeo.bytes
                );
                match &pf.deteccion {
                    Some(d) => println!(
                        "- estilo: {} ({:.0} %) con {} · {}",
                        d.estilo, d.confianza * 100.0, d.modelo, d.motivos
                    ),
                    None => println!("- estilo: sin detección"),
                }
                println!("- planos: {}", pf.planos.len());
                for pl in pf.planos.iter().take(8) {
                    println!("    #{:02} {:.2} → {:.2} s ({:.2} s)", pl.indice, pl.inicio, pl.fin, pl.segundos);
                }
                println!(
                    "- paquete: {} ({}) · plantilla {}",
                    pf.paquete_titulo, pf.paquete_id, pf.plantilla
                );
                println!("- VEREDICTO: {}", pf.veredicto);
                for a in &pf.avisos {
                    println!("    aviso: {a}");
                }
                for f in &pf.faltan {
                    println!("    FALTA: {f}");
                }
                println!(
                    "- VRAM libre {:.1} GB · RAM libre {:.1} GB · intermedios ~{:.2} GB",
                    pf.vram_libre_max_gb, pf.ram_libre_gb, pf.disco_estimado_gb
                );
            }
            Err(e) => println!("ERROR: {e}"),
        }
    }
}
