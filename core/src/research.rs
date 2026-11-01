use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discovery {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub source: String,
    pub url: String,
    pub published: String,
    pub summary: String,
    pub license: String,
    pub windows: String,
    pub gpu: String,
    pub vram: String,
    pub expected: String,
    pub measured: String,
    pub risks: String,
    pub recommendation: String,
    pub status: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub item_id: String,
    pub level: String,
    pub message: String,
}

pub fn lab_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("KUKIA_RESEARCH_DIR") {
        return PathBuf::from(custom);
    }
    PathBuf::from("H:\\KUKIA_RESEARCH\\AI_EXPERIMENTAL")
}

pub fn ensure_lab() -> Result<PathBuf, String> {
    let dir = lab_dir();
    for sub in ["discoveries", "reports", "lab", "downloads", "benchmarks", "alerts", "templates", "archive"] {
        std::fs::create_dir_all(dir.join(sub)).map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("KUKIA-CODE/0.1 (experimental-radar)")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/* ----------------------------- Evaluación ----------------------------- */

const HIGH_VALUE: &[&str] = &[
    "nvfp4", "fp4", "fp8", "speculative decoding", "tensor parallel", "expert parallel",
    "flash attention", "paged attention", "kv cache", "cuda graphs", "moe", "mixture of experts",
    "quantization", "quantization-aware", "multi-gpu", "multi gpu", "offload", "context length",
];

fn matched_keywords(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    HIGH_VALUE
        .iter()
        .copied()
        .filter(|k| lower.contains(k))
        .collect()
}

fn days_since(iso: &str) -> Option<i64> {
    let dt = chrono::DateTime::parse_from_rfc3339(iso).ok()?;
    Some((chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days())
}

fn evaluate(
    mut d: Discovery,
    downloads: u64,
    stars: u64,
) -> Discovery {
    let hits = matched_keywords(&format!("{} {}", d.title, d.summary));
    let mut score = 0.30_f64;
    if !hits.is_empty() {
        score += 0.20;
    }
    if downloads > 10_000 || stars > 1_000 {
        score += 0.20;
    }
    if let Some(days) = days_since(&d.published) {
        if days <= 30 {
            score += 0.20;
        } else if days > 180 {
            score -= 0.15;
            d.risks = "posible abandono (sin actualización en >180 días)".into();
        }
    }
    if d.license.is_empty() || d.license == "desconocida" {
        score -= 0.05;
        if d.risks.is_empty() {
            d.risks = "licencia no declarada; verificar antes de usar".into();
        }
    }
    d.score = score.clamp(0.0, 1.0);
    d.recommendation = if score >= 0.6 {
        "probar en laboratorio aislado".into()
    } else {
        "vigilar; sin evidencia suficiente para probar".into()
    };
    d.status = "candidato".into();
    d
}

pub fn alerts_for(items: &[Discovery]) -> Vec<Alert> {
    let mut out = Vec::new();
    for item in items {
        let text = format!("{} {}", item.title, item.summary).to_lowercase();
        let high = text.contains("nvfp4") || text.contains("expert parallel") || text.contains("fp4");
        let interesting = matched_keywords(&text);
        if high {
            out.push(Alert {
                item_id: item.id.clone(),
                level: "alta".into(),
                message: format!("Posible salto de rendimiento/VRAM: {}", item.title),
            });
        } else if !interesting.is_empty() {
            out.push(Alert {
                item_id: item.id.clone(),
                level: "info".into(),
                message: format!(
                    "Coincide con [{}]: {}",
                    interesting.join(", "),
                    item.title
                ),
            });
        }
    }
    out
}

/* ----------------------------- arXiv ----------------------------- */

fn tag_text(block: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    if let Some(start) = block.find(&open) {
        let rest = &block[start + open.len()..];
        if let Some(end) = rest.find(&close) {
            return rest[..end].split_whitespace().collect::<Vec<_>>().join(" ");
        }
    }
    String::new()
}

pub async fn scan_arxiv() -> Vec<Discovery> {
    let url = "http://export.arxiv.org/api/query?search_query=cat:cs.LG+AND+(quantization+OR+moe+OR+inference)&sortBy=submittedDate&sortOrder=descending&max_results=10";
    let Ok(resp) = client().get(url).send().await else {
        return Vec::new();
    };
    let Ok(xml) = resp.text().await else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for block in xml.split("<entry>").skip(1) {
        let block = block.split("</entry>").next().unwrap_or("");
        let title = tag_text(block, "title");
        if title.is_empty() {
            continue;
        }
        let id_url = tag_text(block, "id");
        let summary = tag_text(block, "summary");
        let published = tag_text(block, "published");
        let id = id_url
            .rsplit('/')
            .next()
            .unwrap_or(&title)
            .to_string();
        let d = Discovery {
            id: format!("arxiv:{id}"),
            kind: "paper".into(),
            title,
            source: "arXiv".into(),
            url: id_url,
            published,
            summary: summary.chars().take(600).collect(),
            license: "arXiv (ver licencia del paper)".into(),
            windows: "no aplica (paper)".into(),
            gpu: "no aplica (paper)".into(),
            vram: "no aplica (paper)".into(),
            expected: "por determinar tras leer el paper".into(),
            measured: "sin medir".into(),
            risks: String::new(),
            recommendation: String::new(),
            status: String::new(),
            score: 0.0,
        };
        out.push(evaluate(d, 0, 0));
    }
    out
}

/* ----------------------------- Hugging Face ----------------------------- */

pub async fn scan_huggingface() -> Vec<Discovery> {
    let url = "https://huggingface.co/api/models?sort=lastModified&direction=-1&limit=15";
    let Ok(resp) = client().get(url).send().await else {
        return Vec::new();
    };
    let Ok(items) = resp.json::<serde_json::Value>().await else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(arr) = items.as_array() {
        for m in arr {
            let id = m["id"].as_str().unwrap_or("");
            if id.is_empty() {
                continue;
            }
            let downloads = m["downloads"].as_u64().unwrap_or(0);
            let likes = m["likes"].as_u64().unwrap_or(0);
            let tags: Vec<String> = m["tags"]
                .as_array()
                .map(|a| a.iter().filter_map(|t| t.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let license = tags
                .iter()
                .find(|t| t.starts_with("license:"))
                .map(|t| t.trim_start_matches("license:").to_string())
                .unwrap_or_else(|| "desconocida".into());
            let d = Discovery {
                id: format!("hf:{id}"),
                kind: "modelo".into(),
                title: id.to_string(),
                source: "Hugging Face".into(),
                url: format!("https://huggingface.co/{id}"),
                published: m["lastModified"].as_str().unwrap_or("").to_string(),
                summary: format!(
                    "pipeline: {} · descargas: {} · likes: {} · tags: {}",
                    m["pipeline_tag"].as_str().unwrap_or("n/d"),
                    downloads,
                    likes,
                    tags.iter().take(8).cloned().collect::<Vec<_>>().join(", ")
                ),
                license: license.clone(),
                windows: "desconocida (requiere prueba)".into(),
                gpu: "por verificar en RTX 5070 / 5060 Ti".into(),
                vram: "por medir".into(),
                expected: "por evaluar".into(),
                measured: "sin medir".into(),
                risks: String::new(),
                recommendation: String::new(),
                status: String::new(),
                score: 0.0,
            };
            out.push(evaluate(d, downloads, likes));
        }
    }
    out
}

/* ----------------------------- GitHub ----------------------------- */

pub async fn scan_github() -> Vec<Discovery> {
    let url = "https://api.github.com/search/repositories?q=llm+inference+quantization&sort=updated&order=desc&per_page=10";
    let Ok(resp) = client().get(url).send().await else {
        return Vec::new();
    };
    let Ok(v) = resp.json::<serde_json::Value>().await else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(arr) = v["items"].as_array() {
        for r in arr {
            let full = r["full_name"].as_str().unwrap_or("");
            if full.is_empty() {
                continue;
            }
            let stars = r["stargazers_count"].as_u64().unwrap_or(0);
            let license = r["license"]["spdx_id"].as_str().unwrap_or("desconocida");
            let d = Discovery {
                id: format!("gh:{full}"),
                kind: "repo".into(),
                title: full.to_string(),
                source: "GitHub".into(),
                url: r["html_url"].as_str().unwrap_or("").to_string(),
                published: r["updated_at"].as_str().unwrap_or("").to_string(),
                summary: r["description"]
                    .as_str()
                    .unwrap_or("sin descripción")
                    .chars()
                    .take(400)
                    .collect(),
                license: license.to_string(),
                windows: "desconocida (requiere prueba)".into(),
                gpu: "por verificar".into(),
                vram: "por medir".into(),
                expected: "por evaluar".into(),
                measured: "sin medir".into(),
                risks: String::new(),
                recommendation: String::new(),
                status: String::new(),
                score: 0.0,
            };
            out.push(evaluate(d, 0, stars));
        }
    }
    out
}

/* ----------------------------- Orquestación ----------------------------- */

pub async fn scan_all() -> Vec<Discovery> {
    let (a, h, g) = tokio::join!(scan_arxiv(), scan_huggingface(), scan_github());
    let mut out: Vec<Discovery> = Vec::new();
    for d in a.into_iter().chain(h).chain(g) {
        if !out.iter().any(|x| x.id == d.id) {
            out.push(d);
        }
    }
    out
}

pub async fn scan_one(source: &str) -> Vec<Discovery> {
    match source {
        "arxiv" => scan_arxiv().await,
        "huggingface" => scan_huggingface().await,
        "github" => scan_github().await,
        _ => scan_all().await,
    }
}

/* ----------------------------- Informe ----------------------------- */

pub fn write_report(items: &[Discovery], alerts: &[Alert]) -> Result<PathBuf, String> {
    let dir = ensure_lab()?;
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let path = dir.join("reports").join(format!("{date}.md"));
    let mut md = String::new();
    md.push_str(&format!("# KUKIA EXPERIMENTAL AI RADAR — {date}\n\n"));
    md.push_str(&format!("Descubrimientos: {}\nAlertas: {}\n\n", items.len(), alerts.len()));

    md.push_str("## Alertas\n\n");
    if alerts.is_empty() {
        md.push_str("Sin alertas.\n\n");
    } else {
        for a in alerts {
            md.push_str(&format!("- **[{}]** {} (`{}`)\n", a.level, a.message, a.item_id));
        }
        md.push('\n');
    }

    md.push_str("## Descubrimientos\n\n");
    for d in items {
        md.push_str(&format!("### {} ({})\n\n", d.title, d.source));
        md.push_str(&format!("- Fuente: {}\n", d.url));
        md.push_str(&format!("- Fecha: {}\n", d.published));
        md.push_str(&format!("- Tipo: {}\n", d.kind));
        md.push_str(&format!("- Licencia: {}\n", d.license));
        md.push_str(&format!("- Windows: {}\n", d.windows));
        md.push_str(&format!("- GPU: {}\n", d.gpu));
        md.push_str(&format!("- VRAM: {}\n", d.vram));
        md.push_str(&format!("- Mejora esperada: {}\n", d.expected));
        md.push_str(&format!("- Medido: {}\n", d.measured));
        let risks = if d.risks.is_empty() { "sin evaluar" } else { d.risks.as_str() };
        md.push_str(&format!("- Riesgos: {risks}\n"));
        md.push_str(&format!("- Recomendación: {}\n", d.recommendation));
        md.push_str(&format!("- Puntuación heurística: {:.2}\n\n", d.score));
    }
    md.push_str("\n> Generado automáticamente. Nada se ha instalado ni modificado en el sistema.\n");
    std::fs::write(&path, md).map_err(|e| e.to_string())?;
    Ok(path)
}

/* ----------------------------- Descarga aislada ----------------------------- */

pub async fn fetch_public(url: &str) -> Result<PathBuf, String> {
    if !url.starts_with("https://") {
        return Err("Solo se permiten URLs https".into());
    }
    let dir = ensure_lab()?.join("downloads");
    let name = url
        .rsplit('/')
        .next()
        .unwrap_or("descarga")
        .split('?')
        .next()
        .unwrap_or("descarga")
        .replace(|c: char| !c.is_alphanumeric() && c != '.' && c != '-' && c != '_', "_");
    let dest = dir.join(format!("{}_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"), name));

    let resp = client().get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    if let Some(len) = resp.content_length() {
        if len > 25_000_000 {
            return Err(format!("Archivo demasiado grande ({len} bytes, límite 25 MB)"));
        }
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > 25_000_000 {
        return Err("Archivo demasiado grande (límite 25 MB)".into());
    }
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    let _ = std::fs::write(
        dir.join("registro_descargas.log"),
        format!("{} {} -> {}\n", now_iso(), url, dest.display()),
    );
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_detected() {
        let hits = matched_keywords("NVFP4 quantization for MoE inference");
        assert!(hits.contains(&"nvfp4"));
        assert!(hits.contains(&"moe"));
    }

    #[test]
    fn high_value_generates_alert() {
        let d = Discovery {
            id: "x".into(),
            kind: "paper".into(),
            title: "NVFP4 kernels".into(),
            source: "arXiv".into(),
            url: String::new(),
            published: now_iso(),
            summary: String::new(),
            license: String::new(),
            windows: String::new(),
            gpu: String::new(),
            vram: String::new(),
            expected: String::new(),
            measured: String::new(),
            risks: String::new(),
            recommendation: String::new(),
            status: String::new(),
            score: 0.0,
        };
        let alerts = alerts_for(&[d]);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, "alta");
    }

    #[test]
    fn evaluation_rewards_recent_and_popular() {
        let d = Discovery {
            id: "x".into(),
            kind: "modelo".into(),
            title: "quantization model".into(),
            source: "HF".into(),
            url: String::new(),
            published: now_iso(),
            summary: "moe".into(),
            license: "mit".into(),
            windows: String::new(),
            gpu: String::new(),
            vram: String::new(),
            expected: String::new(),
            measured: String::new(),
            risks: String::new(),
            recommendation: String::new(),
            status: String::new(),
            score: 0.0,
        };
        let e = evaluate(d, 50_000, 0);
        assert!(e.score >= 0.89, "score inesperado: {}", e.score);
        assert!(e.recommendation.contains("probar"));
    }

    #[test]
    fn lab_dirs_created() {
        let p = ensure_lab().unwrap();
        assert!(p.join("reports").exists());
        assert!(p.join("downloads").exists());
    }
}
