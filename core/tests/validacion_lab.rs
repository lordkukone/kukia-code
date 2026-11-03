//! Validación del LABORATORIO EXPERIMENTAL contra el motor real.
//!
//! Usa el modelo que ya esté cargado en Ollama (no carga ni descarga nada) y mide
//! una generación corta para comprobar que las métricas salen de verdad.
//!
//!   cargo test --test validacion_lab -- --nocapture --test-threads=1

use kukia_core::engines::InferenceParams;
use kukia_core::lab;

async fn modelo_cargado() -> Option<String> {
    let v: serde_json::Value = reqwest::get("http://127.0.0.1:11434/api/ps")
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    v.get("models")?
        .as_array()?
        .first()?
        .get("name")?
        .as_str()
        .map(String::from)
}

#[test]
fn el_catalogo_refleja_el_entorno_de_verdad() {
    let c = lab::catalogo();
    println!("experimentos: {}", c.len());
    let por_nivel = |n: &str| c.iter().filter(|e| e.nivel == n).count();
    println!(
        "  estables {} · experimentales {} · muy experimentales {} · incompatibles {}",
        por_nivel("estable"),
        por_nivel("experimental"),
        por_nivel("muy_experimental"),
        por_nivel("incompatible")
    );
    for e in c.iter().filter(|e| e.mecanismo == "variable_servidor") {
        println!(
            "  {} · {}={} · {}",
            e.nombre,
            e.variable.clone().unwrap_or_default(),
            e.valor_actual.clone().unwrap_or_else(|| "(sin definir)".into()),
            e.estado_local
        );
    }
    let activables: Vec<&str> = c.iter().filter(|e| e.activable).map(|e| e.id.as_str()).collect();
    println!("  activables desde KUKIA: {activables:?}");

    assert!(c.len() >= 30, "el catálogo debe cubrir todas las secciones");
    assert!(!activables.is_empty(), "alguna técnica tiene que poder activarse");
    // La KV cuantizada es del servidor: nunca debe aparecer como activable.
    assert!(!c.iter().any(|e| e.id == "kv_q4" && e.activable));
}

#[test]
fn mide_de_verdad_sobre_el_modelo_cargado() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let Some(model) = modelo_cargado().await else {
            println!("SALTADO: no hay ningún modelo cargado en Ollama");
            return;
        };
        println!("modelo: {model}");

        let p = InferenceParams::default();
        let r = lab::benchmark("ollama", &model, &p, "Di una frase corta sobre memoria.", 32).await;
        println!("ok={} · {} tok/s · {} tokens", r.ok, r.tok_s, r.tokens);
        println!(
            "prefill {:?} tok/s · TTFT {:?} ms · ITL {:?} ms · carga {:?} ms · total {} ms",
            r.prefill_tok_s, r.ttft_ms, r.itl_ms, r.carga_ms, r.total_ms
        );
        for g in &r.vram_pico {
            println!("  pico GPU{}: {} MB", g.index, g.pico_mb);
        }
        println!(
            "pico RAM {:.1} GB · temp {:.0} °C · {:.0} W",
            r.ram_pico_gb, r.temp_pico_c, r.potencia_pico_w
        );
        println!("params: {}", r.params_texto);

        assert!(r.ok, "la generación debe funcionar: {}", r.error);
        assert!(r.tokens > 0);
        assert!(r.tok_s > 0.0, "tokens/s debe ser un número medido, no cero");
        assert!(r.prefill_tok_s.unwrap_or(0.0) > 0.0, "Ollama publica prefill");
        assert!(r.ttft_ms.unwrap_or(0.0) > 0.0);
        assert!(!r.vram_pico.is_empty(), "se muestrea la VRAM durante la prueba");
        assert!(
            r.vram_pico.iter().any(|g| g.pico_mb > 512),
            "un modelo grande debe dejar rastro en la VRAM"
        );
        assert!(r.ram_pico_gb > 1.0);
        assert!(r.total_ms > 0.0);
    });
}

#[test]
fn la_comparacion_ab_queda_coherente() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let Some(model) = modelo_cargado().await else {
            println!("SALTADO: no hay modelo cargado");
            return;
        };
        // A y B idénticas: sirve para comprobar que la resta y los campos cuadran
        // sin tocar la configuración de nadie.
        let p = InferenceParams::default();
        let c = lab::comparar("ollama", &model, &p, &p, "Di solo: hola.", 16).await;
        println!(
            "A {:.1} tok/s · B {:.1} tok/s · delta {:+.1} % · {}",
            c.a.tok_s, c.b.tok_s, c.delta_tok_s_pct, c.veredicto
        );
        println!("delta VRAM: {:?}", c.delta_vram_mb);
        assert!(c.a.ok && c.b.ok, "las dos pruebas deben completarse");
        assert!(
            (c.delta_tok_s - (c.b.tok_s - c.a.tok_s)).abs() < 1e-9,
            "el delta debe ser B − A"
        );
        assert_eq!(c.delta_vram_mb.len(), c.b.vram_pico.len());
        assert!(!c.veredicto.is_empty());
    });
}
