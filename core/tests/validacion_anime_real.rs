//! Validación real del KUKIA ANIME ↔ REAL STUDIO contra ComfyUI vivo.
//!
//! Ejecutar con:
//!   cargo test -p kukia-core --test validacion_anime_real -- --ignored --nocapture
//!
//! Necesita ComfyUI escuchando en 127.0.0.1:8188 con los modelos del manifest.
//! Si no está, los tests fallan con un mensaje claro en vez de fingir éxito.

use kukia_core::anime_real::{self, ConvertRequest};
use serde_json::json;

async fn exigir_comfy() -> String {
    let st = anime_real::status(anime_real::COMFY_DEFAULT).await;
    assert!(
        st.online,
        "ComfyUI no responde en {}: {}",
        anime_real::COMFY_DEFAULT, st.nota
    );
    println!(
        "ComfyUI {} · {} · VRAM libre {} MB / {} MB · cola {}",
        st.version,
        st.device,
        st.vram_free_mb,
        st.vram_total_mb,
        st.en_cola
    );
    anime_real::COMFY_DEFAULT.to_string()
}

#[tokio::test]
#[ignore = "requiere ComfyUI escuchando en 127.0.0.1:8188"]
async fn el_estudio_ve_comfyui_y_sus_plantillas() {
    let base = exigir_comfy().await;
    let caps = anime_real::capabilities(&base).await;
    assert!(!caps.is_empty(), "el manifest debe declarar plantillas");

    let imagenes = anime_real::imagenes_entrada(&base).await;
    println!("imágenes de entrada visibles: {}", imagenes.len());
    assert!(
        !imagenes.is_empty(),
        "ComfyUI no ve ninguna imagen en su carpeta input"
    );

    for c in &caps {
        println!(
            "{:<28} {:<10} {:<12} disponible={:<5} faltan={:?}",
            c.id, c.direccion, c.tipo, c.disponible, c.faltan
        );
    }
    let listas: Vec<&str> = caps.iter().filter(|c| c.disponible).map(|c| c.id.as_str()).collect();
    assert!(!listas.is_empty(), "ninguna plantilla disponible, ¿faltan modelos?");
}

#[tokio::test]
#[ignore = "requiere ComfyUI escuchando en 127.0.0.1:8188"]
async fn la_conversion_por_rust_produce_una_imagen_real() {
    let base = exigir_comfy().await;

    let caps = anime_real::capabilities(&base).await;
    let cap = caps
        .iter()
        .find(|c| c.id == "anime2real_id_sdxl")
        .expect("la plantilla con IPAdapter debe estar en el manifest");
    assert!(
        cap.disponible,
        "anime2real_id_sdxl no disponible, faltan: {:?}",
        cap.faltan
    );

    let req = ConvertRequest {
        template_id: "anime2real_id_sdxl".into(),
        image: "goku_anime_rt.png".into(),
        prompt: String::new(),      // usa el del workflow
        negative: String::new(),    // usa el del workflow
        denoise: Some(0.55),
        seed: Some(4242),
        width: Some(1024),
        height: Some(1024),
    };

    let inicio = std::time::Instant::now();
    let r = anime_real::convertir(&base, &req)
        .await
        .expect("la conversión debe completarse");

    println!(
        "resultado: {} en {:.1}s (medido en Rust {:.1}s) · {} archivo(s) · {} KB",
        r.template_id,
        r.segundos,
        inicio.elapsed().as_secs_f64(),
        r.archivos.len(),
        r.bytes / 1024
    );

    assert_eq!(r.template_id, "anime2real_id_sdxl");
    assert!(!r.prompt_id.is_empty(), "debe devolver el prompt_id de ComfyUI");
    assert!(!r.archivos.is_empty(), "debe descargar al menos una imagen");
    assert!(r.preview.starts_with("data:image/"), "la vista previa debe ser una data URL");
    assert!(
        !r.entrada_preview.is_empty(),
        "debe devolver también la imagen de entrada para el antes/después"
    );

    for ruta in &r.archivos {
        let meta = std::fs::metadata(ruta).expect("el archivo descargado debe existir");
        println!("  {ruta} · {} KB", meta.len() / 1024);
        assert!(meta.len() > 10_000, "{ruta} parece vacío o truncado");
    }

    // El historial debe registrar el trabajo en disco.
    let hist = anime_real::historial(5);
    assert!(!hist.is_empty(), "el historial debe tener al menos una entrada");
    assert_eq!(hist[0]["direccion"], json!("anime2real"));
    println!("historial: {} entrada(s), última {}", hist.len(), hist[0]["fecha"]);
}

#[tokio::test]
#[ignore = "requiere ComfyUI escuchando en 127.0.0.1:8188"]
async fn una_plantilla_inventada_se_rechaza() {
    let base = exigir_comfy().await;
    let req = ConvertRequest {
        template_id: "no_existe".into(),
        image: "x.png".into(),
        prompt: String::new(),
        negative: String::new(),
        denoise: None,
        seed: None,
        width: None,
        height: None,
    };
    let err = anime_real::convertir(&base, &req).await;
    assert!(err.is_err(), "una plantilla desconocida no debe ejecutarse");
    println!("rechazo correcto: {}", err.unwrap_err());
}
