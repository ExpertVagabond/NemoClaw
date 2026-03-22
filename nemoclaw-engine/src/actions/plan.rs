use crate::blueprint::{load_blueprint, InferenceProfile};
use crate::protocol::emit_progress;
use crate::shell::openshell_available;
use anyhow::{bail, Result};
use serde_json::json;

pub async fn execute(
    run_id: &str,
    profile_name: &str,
    dry_run: bool,
    endpoint_url: Option<&str>,
) -> Result<()> {
    emit_progress(10, "Validating blueprint");

    let bp = load_blueprint()?;
    let profile = bp.resolve_profile(profile_name)?;

    emit_progress(20, "Checking prerequisites");

    if !openshell_available().await {
        bail!(
            "openshell CLI not found. Install it from https://github.com/nvidia/openshell\n\
             Or set PATH to include the openshell binary."
        );
    }

    let endpoint = resolve_endpoint(profile, endpoint_url);

    let plan = json!({
        "run_id": run_id,
        "profile": profile_name,
        "sandbox": {
            "image": bp.components.sandbox.image,
            "name": bp.components.sandbox.name,
            "forward_ports": bp.components.sandbox.forward_ports,
        },
        "inference": {
            "provider_type": profile.provider_type,
            "provider_name": profile.provider_name,
            "endpoint": endpoint,
            "model": profile.model,
            "credential_env": profile.credential_env,
        },
        "policy_additions": bp.components.policy.additions,
        "dry_run": dry_run,
    });

    println!("{}", serde_json::to_string_pretty(&plan)?);
    emit_progress(100, "Plan complete");

    Ok(())
}

fn resolve_endpoint(profile: &InferenceProfile, override_url: Option<&str>) -> String {
    if let Some(url) = override_url {
        return url.to_string();
    }
    profile.endpoint.clone()
}
