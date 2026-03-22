use crate::protocol::emit_progress;
use crate::shell::run_openshell;
use crate::state::{load_state, mark_rolled_back};
use anyhow::{bail, Result};

pub async fn execute(run_id: &str) -> Result<()> {
    let state = load_state(run_id)?;
    let state = match state {
        Some(s) => s,
        None => bail!("run '{run_id}' not found"),
    };

    let sandbox_name = &state.sandbox_name;

    // Step 1: Stop sandbox
    emit_progress(30, &format!("Stopping sandbox {sandbox_name}"));
    let output = run_openshell(&["sandbox", "stop", sandbox_name]).await?;
    if output.exit_code != 0 {
        eprintln!(
            "Warning: stop failed (may already be stopped): {}",
            output.stderr.trim()
        );
    }

    // Step 2: Remove sandbox
    emit_progress(60, &format!("Removing sandbox {sandbox_name}"));
    let output = run_openshell(&["sandbox", "remove", sandbox_name]).await?;
    if output.exit_code != 0 {
        eprintln!(
            "Warning: remove failed: {}",
            output.stderr.trim()
        );
    }

    // Step 3: Mark rolled back
    emit_progress(90, "Cleaning up run state");
    mark_rolled_back(run_id)?;

    emit_progress(100, "Rollback complete");
    Ok(())
}
