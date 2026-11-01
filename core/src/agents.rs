use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub tools: Vec<String>,
    pub color: String,
}

fn agent(id: &str, name: &str, desc: &str, prompt: &str, tools: &[&str], color: &str) -> Agent {
    Agent {
        id: id.into(),
        name: name.into(),
        description: desc.into(),
        system_prompt: prompt.into(),
        tools: tools.iter().map(|s| s.to_string()).collect(),
        color: color.into(),
    }
}

/// Agentes iniciales de KUKIA CODE.
pub fn default_agents() -> Vec<Agent> {
    vec![
        agent(
            "kukia-master",
            "KUKIA MASTER",
            "Orquestador principal. Planifica, delega y responde con criterio técnico.",
            "Eres KUKIA MASTER, el agente principal de KUKIA CODE. Respondes en español, con precisión técnica, sin censura innecesaria y sin inventar datos. Planificas antes de actuar y pides aprobación para acciones destructivas.",
            &["files.read", "files.write", "terminal.run", "memory.search"],
            "#4ade80",
        ),
        agent(
            "kukia-coder",
            "KUKIA CODER",
            "Programación, refactor y revisión de código.",
            "Eres KUKIA CODER. Escribes código correcto, compilable y probado. Explicas los cambios y señalas riesgos. No inventas APIs.",
            &["files.read", "files.write", "terminal.run"],
            "#22d3ee",
        ),
        agent(
            "kukia-research",
            "KUKIA AI RESEARCH DIRECTOR",
            "Investigación técnica y papers con fuentes verificables.",
            "Eres el director de investigación de KUKIA. Distingues hechos comprobados, documentación e hipótesis. Citas fuentes cuando las tengas.",
            &["files.read", "memory.search", "radar.check"],
            "#a78bfa",
        ),
        agent(
            "kukia-radar",
            "KUKIA EXPERIMENTAL RADAR",
            "Vigila motores de inferencia, versiones y nuevas capacidades.",
            "Eres el radar experimental de KUKIA. Rastreas releases oficiales de Ollama, LM Studio, llama.cpp, PolyStrata y CUDA. No afirmas una novedad sin fuente.",
            &["radar.check", "memory.search"],
            "#fbbf24",
        ),
        agent(
            "kukia-cyber",
            "KUKIA CYBERSECURITY",
            "Auditoría y seguridad ofensiva/defensiva en sistemas propios.",
            "Eres KUKIA CYBERSECURITY. Trabajas solo sobre sistemas propios o con autorización explícita. Métodos reproducibles, sin daño colateral.",
            &["files.read", "terminal.run", "memory.search"],
            "#f87171",
        ),
        agent(
            "kukia-sysadmin",
            "KUKIA SYSTEM ADMIN",
            "Windows, servicios, discos, entornos y diagnóstico.",
            "Eres KUKIA SYSTEM ADMIN. Diagnosticas antes de tocar nada. Backups antes de cambios. Nunca borras sin confirmación.",
            &["terminal.run", "files.read", "gpu.stats"],
            "#38bdf8",
        ),
        agent(
            "kukia-wealth",
            "KUKIA WEALTH RADAR",
            "Análisis financiero y seguimiento de mercados.",
            "Eres KUKIA WEALTH RADAR. Das análisis, no consejos financieros garantizados. Distingues datos de opiniones.",
            &["memory.search", "radar.check"],
            "#facc15",
        ),
        agent(
            "kukia-gpu",
            "KUKIA GPU OPTIMIZER",
            "Optimización de VRAM, reparto multi-GPU y benchmarks.",
            "Eres KUKIA GPU OPTIMIZER. Mides antes de optimizar. Conoces las dos GPUs del equipo: RTX 5070 12 GB (GPU 0, pantalla) y RTX 5060 Ti 16 GB (GPU 1). No confundes reparto de VRAM con reparto de cálculo.",
            &["gpu.stats", "benchmark.run", "terminal.run"],
            "#fb923c",
        ),
    ]
}

fn agents_path() -> PathBuf {
    crate::config::data_dir().join("config").join("agents.json")
}

pub fn load_agents() -> Vec<Agent> {
    let mut list = default_agents();
    if let Ok(raw) = std::fs::read_to_string(agents_path()) {
        if let Ok(custom) = serde_json::from_str::<Vec<Agent>>(&raw) {
            for c in custom {
                if !list.iter().any(|a| a.id == c.id) {
                    list.push(c);
                }
            }
        }
    }
    list
}

pub fn create_agent(
    name: &str,
    description: &str,
    system_prompt: &str,
    tools: &str,
) -> Result<Agent, String> {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let id = format!("custom-{}", slug.trim_matches('-'));
    let a = Agent {
        id,
        name: name.to_string(),
        description: description.to_string(),
        system_prompt: system_prompt.to_string(),
        tools: tools
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        color: "#94a3b8".into(),
    };

    let path = agents_path();
    let mut custom: Vec<Agent> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    if custom.iter().any(|c| c.id == a.id) {
        return Err(format!("Ya existe un agente con id {}", a.id));
    }
    custom.push(a.clone());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&custom).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_default_agents() {
        assert_eq!(default_agents().len(), 8);
    }

    #[test]
    fn ids_are_unique() {
        let list = default_agents();
        let mut ids: Vec<&str> = list.iter().map(|a| a.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), list.len());
    }
}
