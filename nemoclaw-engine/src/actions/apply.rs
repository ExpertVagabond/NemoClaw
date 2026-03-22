use crate::blueprint::load_blueprint;
use crate::protocol::emit_progress;
use crate::shell::{openshell_available, run_openshell};
use crate::state::{save_state, InferenceState, RunState};
use anyhow::{bail, Result};
use chrono::Utc;
use std::env;

pub async fn execute(
    run_id: &str,
    profile_name: &str,
    _plan_path: Option<&str>,
    endpoint_url: Option<&str>,
) -> Result<()> {
    let bp = load_blueprint()?;
    let profile = bp.resolve_profile(profile_name)?;

    if !openshell_available().await {
        bail!("openshell CLI not found");
    }

    let endpoint = endpoint_url
        .map(|s| s.to_string())
        .unwrap_or_else(|| profile.endpoint.clone());

    let sandbox_name = &bp.components.sandbox.name;
    let sandbox_image = &bp.components.sandbox.image;

    // Step 1: Create sandbox
    emit_progress(20, "Creating OpenClaw sandbox");

    let port_args: Vec<String> = bp
        .components
        .sandbox
        .forward_ports
        .iter()
        .map(|p| format!("--forward={p}"))
        .collect();

    let mut create_args = vec![
        "sandbox",
        "create",
        "--from",
        sandbox_image,
        "--name",
        sandbox_name,
    ];
    let port_refs: Vec<&str> = port_args.iter().map(|s| s.as_str()).collect();
    create_args.extend(port_refs);

    let output = run_openshell(&create_args).await?;
    if output.exit_code != 0 {
        if output.stderr.contains("already exists") {
            eprintln!("Sandbox '{sandbox_name}' already exists, reusing");
        } else {
            bail!(
                "Failed to create sandbox: {}",
                output.stderr.trim()
            );
        }
    }

    // Step 2: Configure provider
    emit_progress(50, "Configuring inference provider");

    let credential = profile
        .credential_env
        .as_deref()
        .and_then(|key| env::var(key).ok())
        .or_else(|| profile.credential_default.clone())
        .unwrap_or_default();

    let mut provider_args = vec![
        "provider".to_string(),
        "create".to_string(),
        "--name".to_string(),
        profile.provider_name.clone(),
        "--type".to_string(),
        profile.provider_type.clone(),
    ];

    if !credential.is_empty() {
        provider_args.push("--credential".to_string());
        provider_args.push(format!("OPENAI_API_KEY={credential}"));
    }
    if !endpoint.is_empty() {
        provider_args.push("--config".to_string());
        provider_args.push(format!("OPENAI_BASE_URL={endpoint}"));
    }

    let provider_refs: Vec<&str> = provider_args.iter().map(|s| s.as_str()).collect();
    let output = run_openshell(&provider_refs).await?;
    if output.exit_code != 0 && !output.stderr.contains("already exists") {
        bail!("Failed to configure provider: {}", output.stderr.trim());
    }

    // Step 3: Set inference route
    emit_progress(70, "Setting inference route");

    let output = run_openshell(&[
        "inference",
        "set",
        "--provider",
        &profile.provider_name,
        "--model",
        &profile.model,
    ])
    .await?;

    if output.exit_code != 0 {
        bail!("Failed to set inference route: {}", output.stderr.trim());
    }

    // Step 4: Save state
    emit_progress(85, "Saving run state");

    let state = RunState {
        run_id: run_id.to_string(),
        profile: profile_name.to_string(),
        sandbox_name: sandbox_name.to_string(),
        inference: InferenceState {
            provider_type: profile.provider_type.clone(),
            provider_name: profile.provider_name.clone(),
            endpoint: endpoint.clone(),
            model: profile.model.clone(),
            credential_env: profile.credential_env.clone(),
        },
        timestamp: Utc::now(),
    };
    save_state(&state)?;

    println!("Sandbox '{sandbox_name}' is ready.");
    println!(
        "Inference: {} -> {} @ {endpoint}",
        profile.provider_name, profile.model
    );
    emit_progress(100, "Apply complete");

    Ok(())
}
