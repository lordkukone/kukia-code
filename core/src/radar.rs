use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub source: String,
    pub version: String,
    pub published: String,
    pub url: String,
    pub note: String,
}

/// Consulta releases oficiales en GitHub y deja constancia de las fuentes
/// que no exponen API pública (se marcan como consulta manual).
pub async fn check() -> Vec<UpdateInfo> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("KUKIA-CODE/0.1 (radar)")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let repos = [
        ("Ollama", "ollama/ollama"),
        ("llama.cpp", "ggml-org/llama.cpp"),
    ];

    let mut out = Vec::new();
    for (label, repo) in repos {
        let url = format!("https://api.github.com/repos/{repo}/releases/latest");
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
                Ok(v) => out.push(UpdateInfo {
                    source: label.into(),
                    version: v["tag_name"].as_str().unwrap_or("").to_string(),
                    published: v["published_at"].as_str().unwrap_or("").to_string(),
                    url: v["html_url"].as_str().unwrap_or("").to_string(),
                    note: "GitHub Releases (oficial)".into(),
                }),
                Err(e) => out.push(manual(label, format!("respuesta ilegible: {e}"))),
            },
            Ok(resp) => out.push(manual(
                label,
                format!("GitHub HTTP {} (¿límite de peticiones?)", resp.status()),
            )),
            Err(e) => out.push(manual(label, format!("sin conexión: {e}"))),
        }
    }

    out.push(UpdateInfo {
        source: "LM Studio".into(),
        version: "—".into(),
        published: "".into(),
        url: "https://lmstudio.ai/changelog".into(),
        note: "changelog oficial (consulta manual: no expone API)".into(),
    });
    out.push(UpdateInfo {
        source: "PolyStrata".into(),
        version: "—".into(),
        published: "".into(),
        url: "".into(),
        note: "sin API pública verificada; revisar fuente oficial antes de integrar".into(),
    });
    out.push(UpdateInfo {
        source: "CUDA Toolkit".into(),
        version: "—".into(),
        published: "".into(),
        url: "https://developer.nvidia.com/cuda-toolkit".into(),
        note: "consulta manual (NVIDIA no publica release feed JSON)".into(),
    });
    out.push(UpdateInfo {
        source: "Driver NVIDIA".into(),
        version: "—".into(),
        published: "".into(),
        url: "https://www.nvidia.com/Download/index.aspx".into(),
        note: "consulta manual".into(),
    });

    out
}

fn manual(source: &str, note: String) -> UpdateInfo {
    UpdateInfo {
        source: source.into(),
        version: "—".into(),
        published: "".into(),
        url: "".into(),
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_entry_builds() {
        let m = manual("X", "sin datos".into());
        assert_eq!(m.source, "X");
        assert_eq!(m.version, "—");
    }
}
