use crate::state::{latest_run, load_state};
use anyhow::Result;
use serde_json::json;
use std::process::Command;

/// Query Docker for live container state.
fn docker_status(container: &str) -> Option<serde_json::Value> {
    let output = Command::new("docker")
        .args(["inspect", "--format", "{{.State.Status}}|{{.State.StartedAt}}|{{.Config.Image}}|{{.State.Pid}}", container])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = raw.trim().split('|').collect();
    if parts.len() < 4 {
        return None;
    }

    Some(json!({
        "container": container,
        "status": parts[0],
        "started_at": parts[1],
        "image": parts[2],
        "pid": parts[3],
    }))
}

/// Read the openclaw config from inside the container.
fn container_config(container: &str) -> Option<serde_json::Value> {
    let output = Command::new("docker")
        .args(["exec", container, "cat", "/sandbox/.openclaw/openclaw.json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    serde_json::from_slice(&output.stdout).ok()
}

pub async fn execute(run_id: Option<&str>) -> Result<()> {
    // First try saved state
    let state = if let Some(id) = run_id {
        load_state(id)?
    } else {
        latest_run()?
    };

    // Always query live Docker state
    let container = "nemoclaw-sandbox";
    let docker = docker_status(container);
    let config = container_config(container);

    // Extract model/provider from config
    let model = config.as_ref()
        .and_then(|c| c.get("agents"))
        .and_then(|a| a.get("defaults"))
        .and_then(|d| d.get("model"))
        .and_then(|m| m.get("primary"))
        .and_then(|p| p.as_str())
        .unwrap_or("unknown");

    let provider = config.as_ref()
        .and_then(|c| c.get("models"))
        .and_then(|m| m.get("providers"))
        .and_then(|p| p.as_object())
        .and_then(|o| o.keys().next().map(|k| k.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let endpoint = config.as_ref()
        .and_then(|c| c.get("models"))
        .and_then(|m| m.get("providers"))
        .and_then(|p| p.as_object())
        .and_then(|o| o.values().next())
        .and_then(|v| v.get("baseUrl"))
        .and_then(|u| u.as_str())
        .unwrap_or("unknown");

    match (state, &docker) {
        (Some(s), Some(d)) => {
            // Merge saved state with live Docker state
            let merged = json!({
                "run_id": s.run_id,
                "status": d["status"],
                "profile": s.profile,
                "sandbox_name": s.sandbox_name,
                "container": d,
                "inference": {
                    "provider": provider,
                    "model": model,
                    "endpoint": endpoint,
                    "saved": s.inference,
                },
                "timestamp": s.timestamp,
            });
            println!("{}", serde_json::to_string_pretty(&merged)?);
        }
        (None, Some(d)) => {
            // No saved state but container is running
            let live = json!({
                "run_id": run_id.unwrap_or("live"),
                "status": d["status"],
                "container": d,
                "inference": {
                    "provider": provider,
                    "model": model,
                    "endpoint": endpoint,
                },
            });
            println!("{}", serde_json::to_string_pretty(&live)?);
        }
        (Some(s), None) => {
            // Saved state exists but container is not running
            let stale = json!({
                "run_id": s.run_id,
                "status": "stopped",
                "profile": s.profile,
                "sandbox_name": s.sandbox_name,
                "inference": s.inference,
                "timestamp": s.timestamp,
                "note": "Container not running. Start with: docker run -d --name nemoclaw-sandbox -p 18789:18789 nemoclaw:latest"
            });
            println!("{}", serde_json::to_string_pretty(&stale)?);
        }
        (None, None) => {
            let id = run_id.unwrap_or("unknown");
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "run_id": id,
                    "status": "no_sandbox",
                    "note": "No sandbox running and no saved state. Start with: docker run -d --name nemoclaw-sandbox -p 18789:18789 nemoclaw:latest"
                }))?
            );
        }
    }

    Ok(())
}
