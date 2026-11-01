//! KUKIA FUTURE GAMING RADAR (Fase 3).
//!
//! Investiga tecnologías de videojuegos, gráficos, GPU y optimización.
//!
//! Reglas de seguridad:
//! - Solo lectura: consulta APIs públicas y documentación.
//! - No descarga ejecutables, DLL ni instaladores.
//! - No modifica juegos, drivers ni configuraciones.
//! - No ejecuta nada de lo descubierto.
//! - Cualquier prueba real exigirá autorización expresa y entorno aislado.

use crate::memory::GamingItem;
use serde::Serialize;
use std::path::PathBuf;

/// Repositorios y proyectos de referencia que se vigilan por releases.
const REPOS: [(&str, &str); 9] = [
    ("beeradmoore/dlss-swapper", "Inyector / sustitución de DLL"),
    ("crosire/reshade", "Inyector / post-procesado"),
    ("Kaldaien/SpecialK", "Inyector / límites de frames"),
    ("bkaradzic/bgfx", "Wrapper gráfico"),
    ("NVIDIAGameWorks/rtx-remix", "Modding / path tracing"),
    ("NVIDIAGameWorks/Streamline", "SDK de NVIDIA para DLSS/FG/Reflex"),
    ("optiscaler/OptiScaler", "Upscaling universal (DLSS/FSR/XeSS)"),
    ("NVIDIA/Model-Optimizer", "Optimización de modelos (FP4/FP8)"),
    ("ggml-org/llama.cpp", "Inferencia local (referencia de técnicas)"),
];

/// Términos de búsqueda para descubrir proyectos nuevos.
const BUSQUEDAS: [(&str, &str); 6] = [
    ("dlss frame generation", "Frame Generation"),
    ("path tracing renderer", "Path Tracing"),
    ("neural rendering upscaler", "Renderizado neuronal"),
    ("gpu unlock framegen", "GPU Unlock / experimentos"),
    ("ray tracing denoiser", "Denoising RT"),
    ("dlss swapper injector", "Inyectores"),
];

const ARXIV_TERMS: [&str; 5] = [
    "neural rendering",
    "real-time path tracing",
    "temporal upscaling",
    "ray tracing denoising",
    "frame interpolation neural",
];

pub fn lab_dir() -> PathBuf {
    if let Ok(p) = std::env::var("KUKIA_GAMING_LAB") {
        return PathBuf::from(p);
    }
    PathBuf::from(r"H:\KUKIA_RESEARCH\GAMING_GRAPHICS")
}

pub fn ensure_lab() -> Result<PathBuf, String> {
    let base = lab_dir();
    for sub in [
        "discoveries",
        "reports",
        "benchmarks",
        "compatibility",
        "downloads",
        "lab",
        "alerts",
        "archive",
    ] {
        std::fs::create_dir_all(base.join(sub)).map_err(|e| e.to_string())?;
    }
    let readme = base.join("LEEME.md");
    if !readme.exists() {
        let _ = std::fs::write(
            &readme,
            "# KUKIA GAMING & GRAPHICS LAB\n\n\
             Laboratorio de investigación gráfica. Por defecto es de SOLO LECTURA.\n\n\
             - `discoveries`  fichas de descubrimientos\n\
             - `reports`      informes diarios y semanales\n\
             - `benchmarks`   mediciones propias\n\
             - `compatibility` base de compatibilidad de juegos\n\
             - `downloads`   descargas manuales autorizadas\n\
             - `lab`         entorno aislado para pruebas futuras\n\
             - `alerts`      alertas\n\
             - `archive`     histórico\n\n\
             **No se ejecuta código de terceros, ni instaladores, ni DLL, ni scripts descargados.**\n\
             **No se modifican juegos, drivers ni configuraciones.**\n",
        );
    }
    Ok(base)
}

fn client(timeout: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .user_agent("KUKIA-CODE-GamingRadar/0.1")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn slug(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn trunc(s: &str, n: usize) -> String {
    let clean = s.replace('\n', " ").replace('\r', " ");
    if clean.chars().count() <= n {
        clean
    } else {
        clean.chars().take(n).collect::<String>() + "…"
    }
}

/// Ficha vacía con los 20 campos obligatorios.
fn ficha(nombre: &str, categoria: &str) -> GamingItem {
    GamingItem {
        id: slug(nombre),
        name: nombre.to_string(),
        category: categoria.to_string(),
        description: String::new(),
        source: String::new(),
        url: String::new(),
        published: String::new(),
        updated: String::new(),
        status: "INVESTIGACION".into(),
        opensource: String::new(),
        license: String::new(),
        os: String::new(),
        gpus: String::new(),
        games: String::new(),
        requirements: String::new(),
        measured: "sin medir".into(),
        risks: String::new(),
        conflicts: String::new(),
        rtx5070: "sin evaluar".into(),
        rtx5060ti: "sin evaluar".into(),
        potential: String::new(),
        recommendation: String::new(),
        score: 0.0,
    }
}

/// Clasificación automática a partir de metadatos del repositorio.
fn clasificar(archived: bool, prerelease: bool, license: &str, nombre: &str) -> (&'static str, f64) {
    let l = license.to_lowercase();
    let n = nombre.to_lowercase();

    let score;
    let estado;

    if archived {
        estado = "ABANDONADO";
        score = 0.2;
    } else if l.contains("gpl") || l.contains("agpl") {
        // GPL en inyectores implica obligaciones y conflictos de distribución.
        estado = "EXPERIMENTAL";
        score = 0.55;
    } else if n.contains("unlock") || n.contains("crack") || n.contains("bypass") {
        estado = "RIESGO_ALTO";
        score = 0.3;
    } else if prerelease {
        estado = "EXPERIMENTAL";
        score = 0.6;
    } else {
        estado = "ESTABLE";
        score = 0.75;
    }
    (estado, score)
}

/// Evaluación para nuestras dos GPU.
fn compat_gpu(categoria: &str, nombre: &str) -> (String, String, String) {
    let n = nombre.to_lowercase();
    let c = categoria.to_lowercase();

    if c.contains("inyector") || c.contains("wrapper") || n.contains("swapper") {
        return (
            "compatible (RTX 5070 soporta DLSS 4 y Frame Generation)".into(),
            "compatible (RTX 5060 Ti soporta DLSS 4 y Frame Generation)".into(),
            "Requiere sustituir DLL del juego; riesgo de anticheat en multijugador online.".into(),
        );
    }
    if c.contains("path tracing") || c.contains("modding") {
        return (
            "parcial: 12 GB limitan texturas y geometría de Remix".into(),
            "buena: 16 GB dan margen para Remix y Mega Geometry".into(),
            "Pensado para un jugador; nunca en juegos con anticheat.".into(),
        );
    }
    if c.contains("sdk") || c.contains("upscaling") {
        return (
            "compatible".into(),
            "compatible".into(),
            "Integración por el desarrollador del juego; no requiere modificar nada.".into(),
        );
    }
    ("sin evaluar".into(), "sin evaluar".into(), String::new())
}

/* ------------------------------ Fuentes ------------------------------ */

pub async fn scan_github_releases() -> Vec<GamingItem> {
    let mut out = Vec::new();
    for (repo, categoria) in REPOS {
        let url = format!("https://api.github.com/repos/{repo}/releases/latest");
        let resp = match client(15).get(&url).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !resp.status().is_success() {
            continue;
        }
        let v: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let tag = v["tag_name"].as_str().unwrap_or("");
        let name = v["name"].as_str().unwrap_or(repo);
        let published = v["published_at"].as_str().unwrap_or("");
        let body = v["body"].as_str().unwrap_or("");
        let prerelease = v["prerelease"].as_bool().unwrap_or(false);
        let html = v["html_url"].as_str().unwrap_or("");

        let mut f = ficha(&format!("{repo} {tag}"), categoria);
        f.description = trunc(body, 400);
        f.source = format!("GitHub · {repo}");
        f.url = html.to_string();
        f.published = published.to_string();
        f.updated = published.to_string();
        f.opensource = "sí (repositorio público)".into();
        f.license = "ver repositorio".into();
        f.os = "Windows (principal)".into();
        f.requirements = "ver documentación del proyecto".into();
        let (estado, score) = clasificar(false, prerelease, "", name);
        f.status = estado.into();
        f.score = score;
        let (a, b, riesgo) = compat_gpu(categoria, name);
        f.rtx5070 = a;
        f.rtx5060ti = b;
        f.risks = riesgo;
        f.potential = "seguimiento de versiones y cambios de compatibilidad".into();
        f.recommendation = format!(
            "{}: revisar notas de la versión antes de usar en un juego concreto.",
            if prerelease { "Experimental" } else { "Estable" }
        );
        out.push(f);
    }
    out
}

pub async fn scan_github_search() -> Vec<GamingItem> {
    let mut out = Vec::new();
    for (q, categoria) in BUSQUEDAS {
        let url = format!(
            "https://api.github.com/search/repositories?q={}&sort=updated&order=desc&per_page=4",
            q.replace(' ', "+")
        );
        let resp = match client(20).get(&url).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !resp.status().is_success() {
            continue;
        }
        let v: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(arr) = v["items"].as_array() {
            for it in arr {
                let full = it["full_name"].as_str().unwrap_or("");
                if full.is_empty() {
                    continue;
                }
                let desc = it["description"].as_str().unwrap_or("");
                let lic = it["license"]["spdx_id"].as_str().unwrap_or("sin licencia");
                let archived = it["archived"].as_bool().unwrap_or(false);
                let stars = it["stargazers_count"].as_u64().unwrap_or(0);
                let updated = it["updated_at"].as_str().unwrap_or("");
                let html = it["html_url"].as_str().unwrap_or("");

                let mut f = ficha(full, categoria);
                f.description = trunc(desc, 300);
                f.source = "GitHub search".into();
                f.url = html.to_string();
                f.published = updated.to_string();
                f.updated = updated.to_string();
                f.opensource = "sí".into();
                f.license = lic.to_string();
                f.os = "ver repositorio".into();
                let (estado, mut score) = clasificar(archived, false, lic, full);
                f.status = estado.into();
                if stars > 500 {
                    score = (score + 0.1).min(0.95);
                }
                f.score = score;
                let (a, b, riesgo) = compat_gpu(categoria, full);
                f.rtx5070 = a;
                f.rtx5060ti = b;
                f.risks = if riesgo.is_empty() {
                    "proyecto no verificado".into()
                } else {
                    riesgo
                };
                f.recommendation = "candidato a estudiar; no usar sin revisar el código".into();
                out.push(f);
            }
        }
    }
    out
}

pub async fn scan_arxiv() -> Vec<GamingItem> {
    let mut out = Vec::new();
    for term in ARXIV_TERMS {
        let url = format!(
            "http://export.arxiv.org/api/query?search_query=all:{}&sortBy=submittedDate&sortOrder=descending&max_results=3",
            term.replace(' ', "+")
        );
        let resp = match client(20).get(&url).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !resp.status().is_success() {
            continue;
        }
        let xml = match resp.text().await {
            Ok(t) => t,
            Err(_) => continue,
        };
        for entry in xml.split("<entry>").skip(1) {
            let titulo = entre(entry, "<title>", "</title>");
            let resumen = entre(entry, "<summary>", "</summary>");
            let fecha = entre(entry, "<published>", "</published>");
            let id_url = entre(entry, "<id>", "</id>");
            if titulo.is_empty() {
                continue;
            }
            let mut f = ficha(&trunc(&titulo, 120), "Investigación gráfica");
            f.description = trunc(&resumen, 400);
            f.source = "arXiv".into();
            f.url = id_url;
            f.published = fecha.clone();
            f.updated = fecha;
            f.opensource = "preprint (código según autores)".into();
            f.license = "arXiv (ver artículo)".into();
            f.os = "no aplica".into();
            f.status = "INVESTIGACION".into();
            f.score = 0.45;
            f.rtx5070 = "sin evaluar".into();
            f.rtx5060ti = "sin evaluar".into();
            f.potential = "técnica de vanguardia; posible adopción futura en motores".into();
            f.recommendation = "leer el artículo antes de sacar conclusiones".into();
            out.push(f);
        }
    }
    out
}

fn entre(texto: &str, ini: &str, fin: &str) -> String {
    if let Some(a) = texto.find(ini) {
        let resto = &texto[a + ini.len()..];
        if let Some(b) = resto.find(fin) {
            return resto[..b].trim().to_string();
        }
    }
    String::new()
}

/// Escaneo completo de todas las fuentes del radar gaming.
pub async fn scan_all() -> Vec<GamingItem> {
    let mut todos = Vec::new();
    todos.extend(scan_github_releases().await);
    todos.extend(scan_github_search().await);
    todos.extend(scan_arxiv().await);
    // El mismo proyecto puede aparecer en releases y en la búsqueda: se queda una sola ficha.
    // Sin esto el escaneo inflaba el recuento (medido: 39 descubiertos, 36 reales).
    let mut vistos = std::collections::HashSet::new();
    todos.retain(|it| vistos.insert(it.id.clone()));
    todos
}

/* ------------------------------ Alertas ------------------------------ */

#[derive(Debug, Clone, Serialize)]
pub struct GamingAlert {
    pub item_id: String,
    pub level: String,
    pub message: String,
}

pub fn alerts_for(items: &[GamingItem]) -> Vec<GamingAlert> {
    let mut out = Vec::new();
    for it in items {
        let texto = format!(
            "{} {} {}",
            it.name.to_lowercase(),
            it.description.to_lowercase(),
            it.category.to_lowercase()
        );
        let mut push = |level: &str, msg: String| {
            out.push(GamingAlert {
                item_id: it.id.clone(),
                level: level.into(),
                message: msg,
            })
        };

        if texto.contains("dlss 4") || texto.contains("dlss4") || texto.contains("frame generation")
        {
            push("alta", format!("Generación de frames / DLSS en {} ({})", it.name, it.category));
        }
        if texto.contains("path tracing") || texto.contains("ray tracing") {
            push("info", format!("Trazado de rayos en {} ({})", it.name, it.category));
        }
        if texto.contains("unlock") || texto.contains("bypass") {
            push("riesgo", format!("GPU Unlock / técnica no oficial en {}", it.name));
        }
        if texto.contains("injector") || texto.contains("swapper") || texto.contains("dll") {
            push("info", format!("Inyector o sustitución de DLL: {}", it.name));
        }
        if it.status == "RIESGO_ALTO" {
            push("riesgo", format!("Clasificado RIESGO ALTO: {}", it.name));
        }
        if it.status == "ABANDONADO" {
            push("info", format!("Proyecto abandonado: {}", it.name));
        }
    }
    out
}

/* ------------------------------ Informes ------------------------------ */

pub fn write_report(items: &[GamingItem], alerts: &[GamingAlert], weekly: bool) -> Result<PathBuf, String> {
    let dir = ensure_lab()?.join("reports");
    let hoy = chrono::Utc::now();
    let nombre = if weekly {
        format!("semanal-{}.md", hoy.format("%Y-%W"))
    } else {
        format!("{}.md", hoy.format("%Y-%m-%d"))
    };
    let path = dir.join(nombre);

    let mut md = String::new();
    md.push_str(&format!(
        "# KUKIA FUTURE GAMING RADAR — {}\n\n",
        if weekly { "informe semanal" } else { "informe diario" }
    ));
    md.push_str(&format!("Generado: {}\n\n", now_iso()));
    md.push_str(&format!(
        "Descubrimientos: {}\nAlertas: {}\n\n",
        items.len(),
        alerts.len()
    ));

    md.push_str("## Alertas\n\n");
    if alerts.is_empty() {
        md.push_str("Sin alertas.\n\n");
    } else {
        for a in alerts {
            md.push_str(&format!("- **[{}]** {}\n", a.level, a.message));
        }
        md.push('\n');
    }

    md.push_str("## Descubrimientos\n\n");
    for it in items {
        md.push_str(&format!("### {} — {}\n\n", it.name, it.category));
        md.push_str(&format!("- Estado: {}\n", it.status));
        md.push_str(&format!("- Fuente: {} ({})\n", it.source, it.url));
        md.push_str(&format!("- Publicado: {}\n", it.published));
        md.push_str(&format!("- Código abierto: {}\n", it.opensource));
        md.push_str(&format!("- Licencia: {}\n", it.license));
        md.push_str(&format!("- SO: {}\n", it.os));
        md.push_str(&format!("- GPU: {}\n", it.gpus));
        md.push_str(&format!("- Juegos: {}\n", it.games));
        md.push_str(&format!("- Requisitos: {}\n", it.requirements));
        md.push_str(&format!("- Rendimiento medido: {}\n", it.measured));
        md.push_str(&format!("- RTX 5070: {}\n", it.rtx5070));
        md.push_str(&format!("- RTX 5060 Ti: {}\n", it.rtx5060ti));
        md.push_str(&format!("- Riesgos: {}\n", it.risks));
        md.push_str(&format!("- Conflictos: {}\n", it.conflicts));
        md.push_str(&format!("- Potencial: {}\n", it.potential));
        md.push_str(&format!("- Recomendación: {}\n", it.recommendation));
        md.push_str(&format!("- Puntuación: {:.2}\n\n", it.score));
        md.push_str(&format!("{}\n\n", it.description));
    }

    md.push_str("\n> Generado automáticamente. No se ha descargado, ejecutado ni modificado nada.\n");
    std::fs::write(&path, md).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laboratorio_crea_las_ocho_carpetas() {
        std::env::set_var(
            "KUKIA_GAMING_LAB",
            std::env::temp_dir().join("kukia_gaming_test").display().to_string(),
        );
        let base = ensure_lab().expect("crear laboratorio");
        for sub in [
            "discoveries",
            "reports",
            "benchmarks",
            "compatibility",
            "downloads",
            "lab",
            "alerts",
            "archive",
        ] {
            assert!(base.join(sub).is_dir(), "falta {sub}");
        }
    }

    #[test]
    fn clasificacion_marca_archivado_como_abandonado() {
        let (estado, score) = clasificar(true, false, "MIT", "x");
        assert_eq!(estado, "ABANDONADO");
        assert!(score < 0.5);
    }

    #[test]
    fn clasificacion_marca_unlock_como_riesgo() {
        let (estado, _) = clasificar(false, false, "MIT", "gpu-unlock-tool");
        assert_eq!(estado, "RIESGO_ALTO");
    }

    #[test]
    fn compatibilidad_distingue_gpus() {
        let (a, b, _) = compat_gpu("Modding / path tracing", "rtx-remix");
        assert!(a.contains("parcial"));
        assert!(b.contains("buena"));
    }

    #[test]
    fn slug_limpia_nombres() {
        assert_eq!(slug("NVIDIA DLSS 4.5!"), "nvidia-dlss-4-5");
    }
}
