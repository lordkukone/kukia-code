//! Validación real de los proveedores cloud (OpenAI-compatible).
//!
//! Ejecutar con:
//!   cargo test -p kukia-core --test validacion_cloud -- --ignored --nocapture
//!
//! Sale a internet. **No imprime claves**: solo el origen de la credencial y recuentos.
//! Si no hay ninguna clave disponible, los tests hacen `skip` explícito en vez de fallar.

use kukia_core::config::{cloud_credential, AppConfig};
use kukia_core::providers;

#[test]
fn la_configuracion_nunca_contiene_claves() {
    let cfg = AppConfig::default();
    let raw = serde_json::to_string_pretty(&cfg).unwrap();
    for prohibido in ["api_key", "\"key\"", "Bearer ", "sk-"] {
        assert!(
            !raw.contains(prohibido),
            "la configuración no debe contener '{prohibido}'"
        );
    }
}

#[tokio::test]
#[ignore = "consulta el endpoint cloud por red"]
async fn el_proveedor_cloud_activo_responde() {
    let cfg = AppConfig::default();
    let activo = cfg.active_cloud();
    let (clave, origen) = cloud_credential(&activo.id);
    println!(
        "proveedor activo: {} ({}) · base={} · clave desde {} · presente={}",
        activo.label,
        activo.id,
        activo.base_url,
        origen.label(),
        clave.is_some()
    );

    let proveedores = providers::list_providers(&cfg).await;
    for p in &proveedores {
        println!(
            "  [{}] {} · online={} · modelos={} · {}",
            p.id, p.name, p.online, p.models_count, p.note
        );
    }

    let cloud = proveedores
        .iter()
        .find(|p| p.id == "cloud")
        .expect("debe existir el proveedor cloud");
    assert_eq!(cloud.base_url, activo.base_url);

    if clave.is_none() {
        println!("SKIP: no hay credencial para {} en esta máquina", activo.id);
        return;
    }
    assert!(cloud.online, "el proveedor cloud debe responder: {}", cloud.note);
    assert!(
        cloud.models_count > 0,
        "debe listar al menos un modelo en la nube"
    );
}

#[tokio::test]
#[ignore = "consulta el endpoint cloud por red"]
async fn lista_los_modelos_del_proveedor_cloud() {
    let cfg = AppConfig::default();
    let activo = cfg.active_cloud();
    if cloud_credential(&activo.id).0.is_none() {
        println!("SKIP: sin credencial para {}", activo.id);
        return;
    }
    let modelos = providers::list_models("cloud", &cfg).await.expect("lista");
    println!("modelos cloud: {}", modelos.len());
    for m in modelos.iter().take(10) {
        println!("  · {} (provider={})", m.name, m.provider);
    }
    assert!(!modelos.is_empty(), "la nube debe devolver modelos");
    assert!(
        modelos.iter().all(|m| m.provider == "cloud"),
        "todos deben marcarse como cloud"
    );
}
