use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

/// Top-level blueprint.yaml structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub version: String,
    #[serde(default)]
    pub requirements: Requirements,
    pub components: Components,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Requirements {
    #[serde(default)]
    pub min_openshell_version: Option<String>,
    #[serde(default)]
    pub min_openclaw_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    pub sandbox: SandboxConfig,
    pub inference: InferenceConfig,
    #[serde(default)]
    pub policy: PolicyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub image: String,
    #[serde(default = "default_sandbox_name")]
    pub name: String,
    #[serde(default)]
    pub forward_ports: Vec<u16>,
}

fn default_sandbox_name() -> String {
    "openclaw".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub profiles: HashMap<String, InferenceProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceProfile {
    pub provider_type: String,
    pub provider_name: String,
    pub endpoint: String,
    pub model: String,
    #[serde(default)]
    pub credential_env: Option<String>,
    #[serde(default)]
    pub credential_default: Option<String>,
    #[serde(default)]
    pub dynamic_endpoint: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyConfig {
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub additions: HashMap<String, serde_yaml::Value>,
}

impl Blueprint {
    /// Resolve the profile, returning an error if not found.
    pub fn resolve_profile(&self, name: &str) -> Result<&InferenceProfile> {
        self.components
            .inference
            .profiles
            .get(name)
            .with_context(|| {
                let available: Vec<&str> =
                    self.components.inference.profiles.keys().map(|s| s.as_str()).collect();
                format!(
                    "profile '{name}' not found. Available: {}",
                    available.join(", ")
                )
            })
    }
}

/// Resolve the blueprint path from env or default.
pub fn blueprint_path() -> PathBuf {
    env::var("NEMOCLAW_BLUEPRINT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Load blueprint.yaml from the resolved path.
pub fn load_blueprint() -> Result<Blueprint> {
    let base = blueprint_path();
    load_blueprint_from(&base)
}

/// Load blueprint.yaml from a specific directory.
pub fn load_blueprint_from(dir: &Path) -> Result<Blueprint> {
    let path = dir.join("blueprint.yaml");
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_yaml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = r#"
version: "0.1.0"
requirements:
  min_openshell_version: "0.1.0"
components:
  sandbox:
    image: "ghcr.io/nvidia/openshell-community/sandboxes/openclaw:latest"
    name: "openclaw"
    forward_ports: [18789]
  inference:
    profiles:
      default:
        provider_type: "nvidia"
        provider_name: "nvidia-inference"
        endpoint: "https://integrate.api.nvidia.com/v1"
        model: "nvidia/nemotron-3-super-120b-a12b"
        credential_env: "NVIDIA_API_KEY"
      vllm:
        provider_type: "openai"
        provider_name: "vllm-local"
        endpoint: "http://localhost:8000/v1"
        model: "nvidia/nemotron-3-nano-30b-a3b"
        credential_env: "OPENAI_API_KEY"
        credential_default: "dummy"
  policy:
    base: "sandboxes/openclaw/policy.yaml"
"#;

    #[test]
    fn parse_blueprint() {
        let bp: Blueprint = serde_yaml::from_str(SAMPLE_YAML).unwrap();
        assert_eq!(bp.version, "0.1.0");
        assert_eq!(bp.components.sandbox.name, "openclaw");
        assert_eq!(bp.components.sandbox.forward_ports, vec![18789]);
        assert_eq!(bp.components.inference.profiles.len(), 2);
    }

    #[test]
    fn resolve_profile_ok() {
        let bp: Blueprint = serde_yaml::from_str(SAMPLE_YAML).unwrap();
        let profile = bp.resolve_profile("default").unwrap();
        assert_eq!(profile.provider_type, "nvidia");
        assert_eq!(profile.model, "nvidia/nemotron-3-super-120b-a12b");
    }

    #[test]
    fn resolve_profile_missing() {
        let bp: Blueprint = serde_yaml::from_str(SAMPLE_YAML).unwrap();
        let err = bp.resolve_profile("nonexistent").unwrap_err();
        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("default"));
    }

    #[test]
    fn default_sandbox_name_is_openclaw() {
        assert_eq!(default_sandbox_name(), "openclaw");
    }
}
