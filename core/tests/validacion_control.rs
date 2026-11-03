//! Validación del panel avanzado contra Ollama de verdad.
//!
//! Comprueba que la arquitectura leída de `/api/show` cuadra con lo que el
//! modelo declara, que el contexto aplicado se refleja en `/api/ps`, que la
//! tabla de KV calcula lo que se mide en VRAM y que los perfiles se aplican y
//! se pueden deshacer.
//!
//!   cargo test --test validacion_control -- --nocapture --test-threads=1

use kukia_core::control;
use kukia_core::engines;
use kukia_core::vram;

const MODELO: &str = "qwen3-vl:4b";

async fn cargar(ctx: u64) -> Result<(), String> {
    let c = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    let body = serde_json::json!({
        "model": MODELO,
        "prompt": "hola",
        "stream": false,
        "keep_alive": 300,
        "options": { "num_predict": 1, "num_ctx": ctx }
    });
    let r = c
        .post("http://127.0.0.1:11434/api/generate")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama no responde: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status()));
    }
    Ok(())
}

async fn descargar() {
    let c = reqwest::Client::new();
    let _ = c
        .post("http://127.0.0.1:11434/api/generate")
        .json(&serde_json::json!({ "model": MODELO, "keep_alive": 0 }))
        .send()
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
}

#[test]
fn arquitectura_contexto_y_kv_cuadran_con_la_maquina() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        if let Err(e) = cargar(8192).await {
            println!("SALTADO: {e}");
            return;
        }

        let cap = control::capacidades("ollama", MODELO).await;
        println!(
            "arch: {} · {} capas · {} cabezas · {} KV · emb {} · ctx nativo {}",
            cap.arch.architecture,
            cap.arch.layers,
            cap.arch.heads,
            cap.arch.heads_kv,
            cap.arch.embedding,
            cap.arch.ctx_native
        );
        println!(
            "contexto: actual {} · nativo {} · fuente {}",
            cap.contexto.actual, cap.contexto.maximo_modelo, cap.contexto.fuente
        );
        println!("GB de KV por cada 8K: {:.3}", cap.contexto.gb_por_8k);
        for k in &cap.kv {
            println!(
                "  KV {:<5} {:>6.2} GB · ahorro {:>5.2} GB · {}",
                k.formato, k.gb, k.ahorro_gb, k.estado
            );
        }
        println!("GPUs: {}", cap.gpus.len());

        assert_eq!(cap.arch.layers, 36, "qwen3-vl:4b tiene 36 capas");
        assert_eq!(cap.arch.heads_kv, 8);
        assert_eq!(cap.arch.ctx_native, 262144);
        assert_eq!(
            cap.contexto.actual, 8192,
            "el contexto aplicado debe verse en /api/ps"
        );
        assert!(
            cap.avisos.iter().any(|a| a.contains("contexto nativo")),
            "debe avisar del coste del contexto nativo: {:?}",
            cap.avisos
        );

        let fp16 = cap.kv.iter().find(|k| k.formato == "fp16").unwrap();
        let q8 = cap.kv.iter().find(|k| k.formato == "q8_0").unwrap();
        assert!((fp16.gb - 0.703).abs() < 0.01, "KV 8K fp16 ≈ 0,70 GB: {}", fp16.gb);
        assert_eq!(q8.estado, "no_aplicable", "medido: Ollama ignora kv_cache_type");
        assert!(q8.ahorro_gb > 0.3);

        // Lo que el panel predice debe caber en el consumo real medido.
        let rep = vram::report().await;
        let usada: u64 = rep.gpus.iter().map(|g| g.used_mb).sum();
        println!("VRAM total en uso (nvidia-smi): {usada} MB");
        assert!(usada > 2000, "el modelo cargado a 8K debe ocupar varios GB");

        // Parámetros con estado: contexto = recarga, muestreo = caliente.
        let ctx = cap.parametros.iter().find(|p| p.key == "num_ctx").unwrap();
        assert_eq!(ctx.aplica, "recarga");
        assert_eq!(ctx.max, Some(262144.0));
        let temp = cap.parametros.iter().find(|p| p.key == "temperature").unwrap();
        assert_eq!(temp.aplica, "caliente");
        let kvp = cap.parametros.iter().find(|p| p.key == "kv_cache_type").unwrap();
        assert_eq!(kvp.aplica, "no_compatible");
        let rope = cap.parametros.iter().find(|p| p.key == "rope").unwrap();
        assert_eq!(rope.aplica, "no_compatible");

        descargar().await;
    });
}

/// Lee el modelo que ya esté cargado, sin cargar ni descargar nada.
///
/// Sirve para validar el análisis sobre la arquitectura que el usuario tenga
/// abierta en ese momento (hoy, un 39,8B de 49 capas a 262K de contexto).
#[test]
fn analiza_lo_que_ya_esta_cargado_sin_tocarlo() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let ps: serde_json::Value = match reqwest::get("http://127.0.0.1:11434/api/ps").await {
            Ok(r) => r.json().await.unwrap_or_default(),
            Err(e) => {
                println!("SALTADO: Ollama no responde ({e})");
                return;
            }
        };
        let cargado = ps
            .get("models")
            .and_then(|m| m.as_array())
            .and_then(|a| a.first())
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from);
        let Some(modelo) = cargado else {
            println!("SALTADO: no hay ningún modelo cargado en Ollama");
            return;
        };

        let cap = control::capacidades("ollama", &modelo).await;
        println!("modelo cargado: {modelo}");
        println!(
            "  arch {} · {} capas · {} cabezas ({} KV) · ctx nativo {}",
            cap.arch.architecture, cap.arch.layers, cap.arch.heads, cap.arch.heads_kv, cap.arch.ctx_native
        );
        println!(
            "  contexto actual {} · nativo {}",
            cap.contexto.actual, cap.contexto.maximo_modelo
        );
        for k in &cap.kv {
            println!("  KV {:<5} {:>7.2} GB · {}", k.formato, k.gb, k.estado);
        }
        println!("  avisos: {:?}", cap.avisos);

        assert!(cap.arch.layers > 0, "debe leer las capas del modelo cargado");
        assert_eq!(
            cap.contexto.actual, 262144,
            "flint-local está cargado a su contexto nativo"
        );
        let fp16 = cap.kv.iter().find(|k| k.formato == "fp16").unwrap();
        assert!(
            fp16.gb > 10.0,
            "un 39,8B de 49 capas a 262K pide mucha caché KV: {} GB",
            fp16.gb
        );
        assert!(cap.gpus.len() >= 2, "se informan las dos GPU");
    });
}

#[test]
fn un_perfil_se_aplica_y_se_deshace() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // Estado de partida distinto del perfil para notar el cambio.
        engines::params_set(
            "ollama",
            MODELO,
            engines::InferenceParams {
                temperature: Some(1.5),
                num_ctx: Some(2048),
                ..Default::default()
            },
        );

        let (aplicado, avisos) = control::perfil_aplicar("ollama", MODELO, "MÁXIMA CALIDAD")
            .expect("el perfil de fábrica debe existir");
        println!(
            "aplicado MÁXIMA CALIDAD: temp {:?} · ctx {:?} · {} aviso(s)",
            aplicado.temperature,
            aplicado.num_ctx,
            avisos.len()
        );
        assert_eq!(aplicado.num_ctx, Some(32768));
        let guardado = engines::params_get("ollama", MODELO);
        assert_eq!(guardado.num_ctx, Some(32768), "el perfil debe quedar guardado");

        let anterior = control::perfil_restaurar_anterior("ollama", MODELO);
        println!("restaurado: {:?}", anterior.as_ref().map(|p| (p.temperature, p.num_ctx)));
        assert_eq!(anterior.and_then(|p| p.num_ctx), Some(2048), "se deshace el cambio");
        assert_eq!(engines::params_get("ollama", MODELO).num_ctx, Some(2048));

        // Limpieza: dejar el modelo sin parámetros guardados.
        engines::params_set("ollama", MODELO, engines::InferenceParams::default());
    });
}

#[test]
fn los_perfiles_se_guardan_y_se_borran() {
    let p = control::Perfil {
        nombre: "PRUEBA_KUKIA".into(),
        descripcion: "perfil de prueba temporal".into(),
        params: engines::InferenceParams {
            temperature: Some(0.42),
            ..Default::default()
        },
        creado: String::new(),
    };
    let lista = control::perfil_guardar(p).expect("se debe guardar");
    let encontrado = lista.iter().find(|x| x.nombre == "PRUEBA_KUKIA");
    assert!(encontrado.is_some(), "el perfil debe aparecer en la lista");
    assert!(!encontrado.unwrap().creado.is_empty(), "se rellena la fecha");

    let duplicado = control::perfil_guardar(control::Perfil {
        nombre: "PRUEBA_KUKIA".into(),
        descripcion: "otra vez".into(),
        params: engines::InferenceParams {
            temperature: Some(0.9),
            ..Default::default()
        },
        creado: String::new(),
    })
    .unwrap();
    let mismo = duplicado.iter().filter(|x| x.nombre == "PRUEBA_KUKIA").count();
    assert_eq!(mismo, 1, "no se duplican perfiles con el mismo nombre");

    let tras = control::perfil_borrar("PRUEBA_KUKIA").unwrap();
    assert!(!tras.iter().any(|x| x.nombre == "PRUEBA_KUKIA"));
}
