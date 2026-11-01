use std::io::Write;
use std::path::PathBuf;

fn log_path() -> PathBuf {
    crate::config::data_dir().join("logs").join("kukia.log")
}

pub fn append(level: &str, message: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let line = format!(
        "{} [{}] {}\n",
        chrono::Utc::now().to_rfc3339(),
        level,
        message.replace('\n', " ")
    );
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

pub fn tail(lines: usize) -> Vec<String> {
    let raw = std::fs::read_to_string(log_path()).unwrap_or_default();
    let all: Vec<&str> = raw.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..].iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_tail() {
        append("test", "linea de prueba kukia");
        let t = tail(50);
        assert!(t.iter().any(|l| l.contains("linea de prueba kukia")));
    }
}
