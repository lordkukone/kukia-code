//! Prueba real del Model Hub: búsqueda, manifiesto, GGUF locales y compatibilidad.
//!
//! Solo lectura: busca, lee cabeceras y calcula. No descarga ni instala nada.
//!
//! ```text
//! cargo run -p kukia-core --example hub
//! ```

use kukia_core::hub;

#[tokio::main]
async fn main() {
    println!("=== BÚSQUEDA EN OLLAMA (HTML público) ===");
    match hub::buscar_ollama("qwen3").await {
        Ok(hits) => {
            println!("  {} resultado(s)", hits.len());
            for h in hits.iter().take(6) {
                println!("  · {:<28} {}", h.name, h.url);
            }
        }
        Err(e) => println!("  ERROR: {e}"),
    }

    println!("\n=== MANIFIESTO REAL (registry) ===");
    match hub::manifiesto_ollama("qwen3:8b").await {
        Ok(t) => println!(
            "  {:<14} {:<8} digest={}",
            t.name,
            t.size_gb
                .map(|g| format!("{g:.2} GB"))
                .unwrap_or_else(|| "?".into()),
            t.digest.unwrap_or_default()
        ),
        Err(e) => println!("  ERROR: {e}"),
    }

    println!("\n=== HUGGING FACE (API oficial, GGUF) ===");
    match hub::buscar_hf("qwen3", 5).await {
        Ok(lista) => {
            println!("  {} resultado(s)", lista.len());
            for m in &lista {
                println!(
                    "  · {:<58} ↓{:<10} lic={}",
                    m.id,
                    m.descargas,
                    m.licencia.clone().unwrap_or_else(|| "?".into())
                );
            }
        }
        Err(e) => println!("  ERROR: {e}"),
    }

    println!("\n=== GGUF EN DISCO (solo cabecera) ===");
    let raices = hub::raices_gguf();
    println!("  raíces: {:?}", raices);
    let gguf = hub::gguf_locales(&raices, 8);
    println!("  {} archivo(s) listados", gguf.len());
    for g in &gguf {
        println!(
            "  · {:<52} {:>6.2} GB arch={:<12} v={:<3} quant={} {}",
            g.nombre,
            g.size_gb,
            g.arquitectura.clone().unwrap_or_else(|| "?".into()),
            g.version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".into()),
            g.cuantizacion.clone().unwrap_or_else(|| "?".into()),
            if g.valido {
                "OK".to_string()
            } else {
                format!("INVÁLIDO ({})", g.nota)
            }
        );
    }

    println!("\n=== COMPATIBILIDAD CON ESTE EQUIPO ===");
    let (ram, gpus) = hub::hardware_actual();
    println!("  RAM libre: {ram:.1} GB");
    for g in &gpus {
        println!(
            "  GPU{} {} · {:.1}/{:.1} GB libres",
            g.index, g.name, g.free_gb, g.total_gb
        );
    }
    for (p, q, ctx) in [
        (8.0, "Q4_K_M", 8192u64),
        (32.0, "Q4_K_M", 16384),
        (70.0, "Q4_K_M", 8192),
    ] {
        let c = hub::evaluar(p, q, ctx, ram, gpus.clone());
        println!(
            "\n  {p}B {q} @ {ctx}: {} · pesos {:.1} GB · KV {:.1} GB · total {:.1} GB",
            c.veredicto,
            c.pesos_gb.unwrap_or(0.0),
            c.kv_gb.unwrap_or(0.0),
            c.total_gb.unwrap_or(0.0)
        );
        println!("    {}", c.explicacion);
    }
}
