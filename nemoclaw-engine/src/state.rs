use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persisted run state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    pub run_id: String,
    pub profile: String,
    pub sandbox_name: String,
    pub inference: InferenceState,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceState {
    pub provider_type: String,
    pub provider_name: String,
    pub endpoint: String,
    pub model: String,
    pub credential_env: Option<String>,
}

/// Get the state directory root: ~/.nemoclaw/state/runs/
pub fn state_dir() -> PathBuf {
    dirs_path().join("state").join("runs")
}

fn dirs_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".nemoclaw")
}

/// Get the directory for a specific run.
pub fn run_dir(run_id: &str) -> PathBuf {
    state_dir().join(run_id)
}

/// Save run state to disk.
pub fn save_state(state: &RunState) -> Result<()> {
    let dir = run_dir(&state.run_id);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create state dir: {}", dir.display()))?;

    let path = dir.join("plan.json");
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)
        .with_context(|| format!("failed to write state: {}", path.display()))?;

    Ok(())
}

/// Load state for a specific run.
pub fn load_state(run_id: &str) -> Result<Option<RunState>> {
    let path = run_dir(run_id).join("plan.json");
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let state: RunState = serde_json::from_str(&content)?;
    Ok(Some(state))
}

/// Find the most recent run (by directory name sort).
pub fn latest_run() -> Result<Option<RunState>> {
    let dir = state_dir();
    if !dir.exists() {
        return Ok(None);
    }

    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    entries.sort();
    entries.reverse();

    for name in entries {
        if let Ok(Some(state)) = load_state(&name) {
            return Ok(Some(state));
        }
    }

    Ok(None)
}

/// Mark a run as rolled back.
pub fn mark_rolled_back(run_id: &str) -> Result<()> {
    let path = run_dir(run_id).join("rolled_back");
    let timestamp = Utc::now().to_rfc3339();
    std::fs::write(&path, timestamp)?;
    Ok(())
}

/// Check if a run has been rolled back.
#[allow(dead_code)]
pub fn is_rolled_back(run_id: &str) -> bool {
    run_dir(run_id).join("rolled_back").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state(run_id: &str) -> RunState {
        RunState {
            run_id: run_id.to_string(),
            profile: "default".into(),
            sandbox_name: "openclaw".into(),
            inference: InferenceState {
                provider_type: "nvidia".into(),
                provider_name: "nvidia-inference".into(),
                endpoint: "https://api.nvidia.com/v1".into(),
                model: "nemotron-3-super".into(),
                credential_env: Some("NVIDIA_API_KEY".into()),
            },
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn state_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let state = test_state("nc-test-roundtrip");

        // Override state dir for test
        let dir = tmp.path().join(&state.run_id);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("plan.json");
        let json = serde_json::to_string_pretty(&state).unwrap();
        std::fs::write(&path, &json).unwrap();

        let loaded: RunState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.run_id, "nc-test-roundtrip");
        assert_eq!(loaded.profile, "default");
        assert_eq!(loaded.inference.provider_type, "nvidia");
    }

    #[test]
    fn run_dir_path() {
        let dir = run_dir("nc-test-123");
        assert!(dir.to_string_lossy().contains("nc-test-123"));
    }

    #[test]
    fn serialize_state() {
        let state = test_state("nc-serialize-test");
        let json = serde_json::to_string_pretty(&state).unwrap();
        assert!(json.contains("nc-serialize-test"));
        assert!(json.contains("nvidia-inference"));
    }
}
