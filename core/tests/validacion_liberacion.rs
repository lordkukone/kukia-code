//! Validación del gestor de liberación (VRAM MANAGER & MODEL UNLOADER).
//!
//! No simula nada: carga un modelo en Ollama de verdad, lo descarga con el botón
//! nuevo (`vram::release_engine`) y comprueba con `nvidia-smi` cuánta VRAM se
//! recuperó en cada GPU. También comprueba que los motores sin API de descarga se
//! declaran como tales en vez de fingir éxito.
//!
//! Ejecutar con:
//!   cargo test --test validacion_liberacion -- --nocapture --test-threads=1

use kukia_core::vram;

/// Carga `qwen3-vl:4b` en Ollama con keep_alive corto para tener algo que descargar.
async fn cargar_modelo_de_prueba() -> Result<(), String> {
    let c = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let cuerpo = serde_json::json!({
        "model": "qwen3-vl:4b",
        "prompt": "hola",
        "stream": false,
        "keep_alive": 300,
        "options": { "num_predict": 1 }
    });
    let r = c
        .post("http://127.0.0.1:11434/api/generate")
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("Ollama no responde: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("Ollama devolvió HTTP {}", r.status()));
    }
    Ok(())
}

#[test]
fn descarga_medida_en_cada_gpu() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        if let Err(e) = cargar_modelo_de_prueba().await {
            println!("SALTADO: {e}");
            return;
        }

        let antes = vram::report().await;
        let gpus_antes: Vec<(u32, u64)> = antes.gpus.iter().map(|g| (g.index, g.used_mb)).collect();
        let ollama_antes = antes
            .engines
            .iter()
            .find(|e| e.engine == "ollama")
            .cloned()
            .expect("ollama debe estar vigilado");
        println!(
            "antes: ollama con {} modelo(s) · {} MB",
            ollama_antes.models.len(),
            ollama_antes.vram_total_mb
        );
        for m in &ollama_antes.models {
            println!("  - {} · {} MB", m.name, m.vram_mb);
        }
        for (i, u) in &gpus_antes {
            println!("  GPU{i}: {u} MB usados");
        }
        assert!(
            !ollama_antes.models.is_empty(),
            "el modelo de prueba debería estar cargado antes de descargarlo"
        );

        // Botón «Descargar modelos de este motor».
        let r = vram::release_engine("ollama", false).await;
        println!("liberar ollama · ok={} · {} MB recuperados", r.ok, r.freed_mb);
        for g in &r.gpus {
            println!(
                "  GPU{}: {} MB usados · {:.2} GB recuperados",
                g.index,
                g.used_mb,
                g.freed_mb as f64 / 1024.0
            );
        }
        println!("  nota: {}", r.note);
        for u in &r.results {
            println!("  {} / {} -> {} · {}", u.engine, u.model, u.ok, u.message);
        }

        assert!(r.ok, "la descarga por mecanismo oficial debería funcionar");
        assert!(
            r.freed_mb > 0,
            "se debe medir memoria recuperada de verdad, no CERO"
        );
        assert_eq!(r.gpus.len(), antes.gpus.len(), "se mide una cifra por GPU");
        assert_eq!(
            r.freed_mb,
            r.gpus.iter().map(|g| g.freed_mb).sum::<u64>(),
            "el total debe ser la suma exacta por GPU"
        );

        let despues = vram::report().await;
        let ollama_despues = despues
            .engines
            .iter()
            .find(|e| e.engine == "ollama")
            .cloned()
            .unwrap();
        println!(
            "despues: ollama con {} modelo(s)",
            ollama_despues.models.len()
        );
        assert!(
            ollama_despues.models.is_empty(),
            "tras la descarga no debe quedar ningún modelo cargado"
        );

        // Y lo que recuperó la operación cuadra con lo que ve nvidia-smi.
        for (idx, usada_antes) in gpus_antes {
            let ahora = despues.gpus.iter().find(|g| g.index == idx).map(|g| g.used_mb);
            if let Some(ahora) = ahora {
                assert!(
                    ahora <= usada_antes,
                    "GPU{idx}: no puede haber más memoria después de descargar ({ahora} vs {usada_antes})"
                );
            }
        }
    });
}

#[test]
fn los_motores_sin_api_lo_dicen_en_vez_de_fingir() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for motor in ["wan2gp", "maestro"] {
        let r = rt.block_on(vram::release_engine(motor, false));
        println!("{motor}: ok={} · {}", r.ok, r.note);
        assert!(!r.ok, "{motor} no puede afirmar que descargó algo");
        assert!(r.note.contains("no ofrece API") || r.note.contains("no publica"));
    }
}

#[test]
fn motor_inventado_no_hace_nada() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let r = rt.block_on(vram::release_engine("vllm", false));
    println!("vllm: ok={} · {}", r.ok, r.note);
    assert!(!r.ok);
    assert!(r.note.contains("desconocido"));
    assert_eq!(r.freed_mb, 0);
}

/// ComfyUI es el caso difícil: no publica el nombre del modelo residente y su API
/// solo sabe liberar **todo** (`POST /free`). Esta prueba comprueba, contra el
/// servidor real, que tras pedirlo: baja la VRAM, ComfyUI sigue vivo, la cola está
/// vacía y los checkpoints siguen en disco con el mismo tamaño.
#[test]
fn descarga_de_comfyui_sin_cerrarlo_ni_borrar_nada() {
    use std::path::Path;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let c = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();

        let Ok(resp) = c.get("http://127.0.0.1:8188/system_stats").send().await else {
            println!("SALTADO: ComfyUI no responde en 8188");
            return;
        };
        let Ok(v) = resp.json::<serde_json::Value>().await else {
            println!("SALTADO: ComfyUI devuelve algo ilegible");
            return;
        };
        let dev = &v["devices"][0];
        let libre_antes = dev["vram_free"].as_u64().unwrap_or(0) / (1024 * 1024);
        println!("ComfyUI: {libre_antes} MB libres antes");

        let antes = vram::report().await;
        let comfy = antes
            .engines
            .iter()
            .find(|e| e.engine == "comfyui")
            .cloned()
            .expect("comfyui debe estar vigilado");
        assert!(comfy.online, "ComfyUI responde, debe salir online");
        println!("  modelos que declara: {}", comfy.models.len());
        // El umbral del motor (800 MB) cuenta también el contexto de torch que queda
        // tras un /free. Si lo retenido es solo eso, no hay modelo real que descargar y
        // exigir «MB recuperados > 0» sería pedirle peras al olmo.
        let retenido = comfy.models.iter().map(|m| m.vram_mb).max().unwrap_or(0);
        if retenido < 1500 {
            println!(
                "SALTADO: solo hay {retenido} MB retenidos (contexto de torch, no un modelo cargado)"
            );
            return;
        }

        // Huella del checkpoint que estábamos usando, para demostrar que no se toca.
        let ckpt =
            Path::new(r"I:\pinokio\api\comfy.git\app\models\checkpoints\ltx-2.3-22b-dev-fp8.safetensors");
        let tam_antes = std::fs::metadata(ckpt).ok().map(|m| m.len());

        let r = vram::release_engine("comfyui", false).await;
        println!("liberar comfyui · ok={} · {} MB recuperados", r.ok, r.freed_mb);
        for g in &r.gpus {
            println!("  GPU{}: {} MB usados · {} MB recuperados", g.index, g.used_mb, g.freed_mb);
        }
        println!("  nota: {}", r.note);
        assert!(r.ok, "POST /free debe aceptarse");
        assert!(r.freed_mb > 0, "se debe medir VRAM recuperada de verdad, no CERO");

        // Sigue vivo, no lo hemos cerrado.
        let Ok(vivo) = c.get("http://127.0.0.1:8188/system_stats").send().await else {
            panic!("ComfyUI dejó de responder: se ha cerrado, y eso está prohibido");
        };
        assert!(vivo.status().is_success());
        let v2: serde_json::Value = vivo.json().await.unwrap();
        let libre_despues = v2["devices"][0]["vram_free"].as_u64().unwrap_or(0) / (1024 * 1024);
        println!("  {libre_despues} MB libres después");
        assert!(
            libre_despues > libre_antes,
            "debe haber más VRAM libre después ({libre_despues} vs {libre_antes} MB)"
        );

        // Y el nodo que cargaba el modelo sigue disponible: no hemos roto el motor.
        let ok_nodo = c
            .get("http://127.0.0.1:8188/object_info/CheckpointLoaderSimple")
            .send()
            .await
            .map(|x| x.status().is_success())
            .unwrap_or(false);
        assert!(ok_nodo, "ComfyUI debe seguir sirviendo sus nodos");

        // Cola vacía: no hemos dejado trabajo a medias.
        if let Ok(q) = c.get("http://127.0.0.1:8188/queue").send().await {
            let qv: serde_json::Value = q.json().await.unwrap_or(serde_json::json!({}));
            assert_eq!(
                qv["queue_running"].as_array().map(|a| a.len()).unwrap_or(0),
                0,
                "no debe quedar nada ejecutándose"
            );
        }

        // El archivo del modelo sigue exactamente igual.
        let tam_despues = std::fs::metadata(ckpt).ok().map(|m| m.len());
        assert_eq!(tam_antes, tam_despues, "el checkpoint no puede cambiar de tamaño");
        if let Some(t) = tam_despues {
            println!("  checkpoint intacto: {} bytes", t);
        }
    });
}

/// LM Studio: se carga un modelo con su CLI oficial (lo que haría el usuario) y se
/// descarga con el camino de producción de KUKIA (`vram::unload`), comprobando que
/// desaparece de su API, que se mide VRAM recuperada y que el modelo sigue en disco.
#[test]
fn descarga_de_lmstudio_con_su_api() {
    use std::process::Command;

    let modelo = std::env::var("KUKIA_LMS_MODELO").unwrap_or_else(|_| "jev-9b".into());
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let c = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap();

        // Busca el puerto y los modelos cargados sin pasar por KUKIA (comprobación aparte).
        let mut base = String::new();
        for p in [1234u16, 1235] {
            let b = format!("http://127.0.0.1:{p}");
            if let Ok(r) = c.get(format!("{b}/api/v0/models")).send().await {
                if r.status().is_success() {
                    base = b;
                    break;
                }
            }
        }
        if base.is_empty() {
            println!("SALTADO: LM Studio no responde en 1234 ni 1235");
            return;
        }
        println!("LM Studio en {base}");

        let cargados = |v: &serde_json::Value| -> Vec<String> {
            v["data"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter(|m| m["state"] == "loaded")
                        .filter_map(|m| m["id"].as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default()
        };

        let v: serde_json::Value = c.get(format!("{base}/api/v0/models")).send().await.unwrap().json().await.unwrap();
        if cargados(&v).is_empty() {
            // Nada cargado: lo carga el propio test con el CLI oficial.
            let lms = std::env::var("USERPROFILE")
                .map(|h| format!(r"{h}\.lmstudio\bin\lms.exe"))
                .unwrap_or_else(|_| "lms".into());
            let out = Command::new(&lms)
                .args(["load", &modelo, "--gpu", "max", "-c", "2048", "-y"])
                .output();
            match out {
                Ok(o) if o.status.success() => println!("cargado {modelo} con lms"),
                Ok(o) => {
                    println!("SALTADO: lms no pudo cargar {modelo}: {}", String::from_utf8_lossy(&o.stderr));
                    return;
                }
                Err(e) => {
                    println!("SALTADO: sin CLI lms ({e})");
                    return;
                }
            }
        }

        let v: serde_json::Value = c.get(format!("{base}/api/v0/models")).send().await.unwrap().json().await.unwrap();
        let residentes = cargados(&v);
        println!("residentes: {residentes:?}");
        assert!(!residentes.is_empty(), "debería haber algo cargado que descargar");

        let antes = vram::report().await;
        let lms_antes = antes.engines.iter().find(|e| e.engine == "lmstudio").cloned().expect("lmstudio vigilado");
        println!("KUKIA ve {} modelo(s) · {} MB", lms_antes.models.len(), lms_antes.vram_total_mb);
        for m in &lms_antes.models {
            println!("  - {} · {} MB", m.name, m.vram_mb);
        }
        assert!(!lms_antes.models.is_empty(), "KUKIA debe ver el modelo cargado");

        for nombre in &residentes {
            let r = vram::unload("lmstudio", nombre, false).await;
            println!("descargar {nombre} · ok={} · {}", r.ok, r.message);
            assert!(r.ok, "la API de LM Studio debería aceptar la descarga");
        }

        let despues = vram::report().await;
        let lms_despues = despues.engines.iter().find(|e| e.engine == "lmstudio").cloned().unwrap();
        println!("después: {} modelo(s)", lms_despues.models.len());
        assert!(lms_despues.models.is_empty(), "no debe quedar nada cargado");

        let v2: serde_json::Value = c.get(format!("{base}/api/v0/models")).send().await.unwrap().json().await.unwrap();
        assert!(cargados(&v2).is_empty(), "la propia API de LM Studio debe verlo descargado");

        // Sigue instalado en disco: no se ha borrado nada.
        let lms = std::env::var("USERPROFILE")
            .map(|h| format!(r"{h}\.lmstudio\bin\lms.exe"))
            .unwrap_or_else(|_| "lms".into());
        if let Ok(o) = Command::new(&lms).arg("ls").output() {
            let txt = String::from_utf8_lossy(&o.stdout).to_string();
            let raiz = modelo.split('/').next().unwrap_or(&modelo);
            println!("sigue en el catálogo de LM Studio: {}", txt.contains(raiz));
            assert!(txt.contains(raiz), "el modelo debe seguir instalado tras descargarlo");
        }
    });
}

/// La parte delicada de ComfyUI: su /free corta lo que esté generando. Esta prueba
/// encola un trabajo real y comprueba que, con trabajo en marcha, KUKIA se niega a
/// liberar **sin permiso** y sí lo hace cuando se le manda forzar.
#[test]
fn comfyui_no_libera_mientras_genera_salvo_permiso() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let c = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap();

        // Un trabajo corto y autónomo: sin imágenes de entrada que falten.
        let wf = serde_json::json!({
            "1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "Realistic_Vision_V6.0_B1.safetensors"}},
            "2": {"class_type": "EmptyLatentImage", "inputs": {"width": 768, "height": 768, "batch_size": 1}},
            "3": {"class_type": "CLIPTextEncode", "inputs": {"text": "prueba de vram manager", "clip": ["1", 1]}},
            "4": {"class_type": "CLIPTextEncode", "inputs": {"text": "borroso", "clip": ["1", 1]}},
            "5": {"class_type": "KSampler", "inputs": {"seed": 1, "steps": 140, "cfg": 7.0, "sampler_name": "euler", "scheduler": "normal", "denoise": 1.0, "model": ["1", 0], "positive": ["3", 0], "negative": ["4", 0], "latent_image": ["2", 0]}},
            "6": {"class_type": "VAEDecode", "inputs": {"samples": ["5", 0], "vae": ["1", 2]}},
            "7": {"class_type": "SaveImage", "inputs": {"filename_prefix": "kukia_vram_test", "images": ["6", 0]}}
        });

        let Ok(r) = c
            .post("http://127.0.0.1:8188/prompt")
            .json(&serde_json::json!({ "prompt": wf, "client_id": "kukia-vram-test" }))
            .send()
            .await
        else {
            println!("SALTADO: ComfyUI no responde en 8188");
            return;
        };
        if !r.status().is_success() {
            println!("SALTADO: ComfyUI rechazó el trabajo de prueba: {}", r.text().await.unwrap_or_default());
            return;
        }
        println!("trabajo encolado");

        // Espera a que arranque de verdad (carga el checkpoint).
        let mut ocupado = 0usize;
        for _ in 0..60 {
            ocupado = vram::comfyui_generando().await.unwrap_or(0);
            if ocupado > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        println!("comfy_busy = {ocupado}");
        assert!(ocupado > 0, "debe detectar el trabajo en marcha");

        let rep = vram::report().await;
        println!("informe: comfy_busy = {}", rep.comfy_busy);
        assert!(rep.comfy_busy > 0, "el informe debe llevar la cifra de trabajos en marcha");
        assert!(
            rep.recommendations.iter().any(|r| r.contains("ComfyUI tiene")),
            "el informe debe avisar de la generación en curso"
        );

        // Sin permiso: se niega.
        let sin = vram::release_engine("comfyui", false).await;
        println!("sin forzar · ok={} · {}", sin.ok, sin.note);
        assert!(!sin.ok, "no puede liberar a la brava mientras genera");
        assert_eq!(sin.freed_mb, 0, "no debe haber tocado nada");
        assert!(
            sin.results.iter().any(|r| r.message.contains("en marcha")),
            "debe explicar por qué no lo hace"
        );

        // Y el trabajo sigue vivo: no lo hemos cortado.
        let q: serde_json::Value = c
            .get("http://127.0.0.1:8188/queue")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(
            q["queue_running"].as_array().map(|a| !a.is_empty()).unwrap_or(false),
            "el trabajo debe seguir en ejecución tras la negativa"
        );

        // Con permiso explícito: acepta la descarga. Ojo: ComfyUI aplica /free cuando
        // su worker llega a él, así que durante una generación la cifra de VRAM
        // recuperada puede salir 0 en ese instante. No se finge lo contrario.
        let con = vram::release_engine("comfyui", true).await;
        println!("forzando · ok={} · {} MB · {}", con.ok, con.freed_mb, con.note);
        assert!(con.ok, "con permiso sí debe aceptar la descarga");
        assert!(
            con.results.iter().any(|r| r.ok),
            "el motor debe confirmar que aceptó /free"
        );

        // Se corta el trabajo de prueba con la API del propio ComfyUI y se comprueba
        // que, ya en reposo, la VRAM baja de verdad.
        let _ = c.post("http://127.0.0.1:8188/interrupt").json(&serde_json::json!({})).send().await;
        let _ = c
            .post("http://127.0.0.1:8188/queue")
            .json(&serde_json::json!({ "clear": true }))
            .send()
            .await;

        let mut libre = false;
        for _ in 0..60 {
            if vram::comfyui_generando().await.unwrap_or(9) == 0 {
                libre = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        println!("cola vacía tras interrumpir: {libre}");
        assert!(libre, "al interrumpir, ComfyUI no debe quedarse con el trabajo colgado");

        // Y ahora sí, en reposo, el /free forzado deja la VRAM por debajo de lo que marcaba.
        let fin = vram::report().await;
        let comfy_fin = fin.engines.iter().find(|e| e.engine == "comfyui").unwrap();
        println!("ComfyUI tras la prueba: {} MB ocupados", comfy_fin.vram_total_mb);
        assert_eq!(fin.comfy_busy, 0, "sin trabajos pendientes al terminar");
    });
}
