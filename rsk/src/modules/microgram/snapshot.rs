use super::{Microgram, load_all};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Ecosystem snapshot — serializable state of all micrograms + metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub timestamp: String,
    pub micrograms: Vec<Microgram>,
    pub total_tests: usize,
    pub all_pass: bool,
}

/// Save ecosystem state to a JSON snapshot file
pub fn snapshot_save(dir: &Path, out: &Path) -> Result<Snapshot, String> {
    let all = load_all(dir)?;
    let mut total_tests = 0;
    let mut all_pass = true;
    for mg in &all {
        let r = mg.test();
        total_tests += r.total;
        if r.failed > 0 {
            all_pass = false;
        }
    }

    let snap = Snapshot {
        timestamp: chrono::Utc::now().to_rfc3339(),
        micrograms: all,
        total_tests,
        all_pass,
    };

    let json = serde_json::to_string_pretty(&snap).map_err(|e| format!("Serialize: {e}"))?;
    std::fs::write(out, json).map_err(|e| format!("Write: {e}"))?;
    Ok(snap)
}

/// Restore ecosystem from a snapshot file
pub fn snapshot_restore(snap_path: &Path, dir: &Path) -> Result<usize, String> {
    let content = std::fs::read_to_string(snap_path).map_err(|e| format!("Read: {e}"))?;
    let snap: Snapshot = serde_json::from_str(&content).map_err(|e| format!("Parse: {e}"))?;

    if !dir.exists() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Mkdir: {e}"))?;
    }

    let mut count = 0;
    for mg in &snap.micrograms {
        let yaml = serde_yaml::to_string(mg).map_err(|e| format!("YAML: {e}"))?;
        let path = dir.join(format!("{}.yaml", mg.name));
        std::fs::write(&path, yaml).map_err(|e| format!("Write: {e}"))?;
        count += 1;
    }

    Ok(count)
}
