use anyhow::{bail, Context, Result};
use std::time::Duration;
use tokio::process::Command;
use tracing::debug;

/// Output from a shell command.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Run `openshell` with the given args and return output.
pub async fn run_openshell(args: &[&str]) -> Result<ShellOutput> {
    run_command("openshell", args, None).await
}

/// Run `openshell` and fail on non-zero exit.
#[allow(dead_code)]
pub async fn run_openshell_checked(args: &[&str]) -> Result<ShellOutput> {
    let output = run_openshell(args).await?;
    if output.exit_code != 0 {
        bail!(
            "openshell {} failed (exit {}): {}",
            args.first().unwrap_or(&""),
            output.exit_code,
            output.stderr.trim()
        );
    }
    Ok(output)
}

/// Check if `openshell` CLI is available.
pub async fn openshell_available() -> bool {
    run_command("openshell", &["--version"], Some(Duration::from_secs(5)))
        .await
        .map(|o| o.exit_code == 0)
        .unwrap_or(false)
}

/// Run an arbitrary command with optional timeout.
async fn run_command(
    program: &str,
    args: &[&str],
    timeout: Option<Duration>,
) -> Result<ShellOutput> {
    debug!(program, ?args, "running command");

    let mut cmd = Command::new(program);
    cmd.args(args);

    let timeout = timeout.unwrap_or(Duration::from_secs(120));

    let output = tokio::time::timeout(timeout, cmd.output())
        .await
        .context("command timed out")?
        .with_context(|| format!("failed to execute {program}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    debug!(exit_code, stdout_len = stdout.len(), stderr_len = stderr.len(), "command complete");

    Ok(ShellOutput {
        stdout,
        stderr,
        exit_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_echo() {
        let out = run_command("echo", &["hello"], None).await.unwrap();
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn run_false_returns_nonzero() {
        let out = run_command("false", &[], None).await.unwrap();
        assert_ne!(out.exit_code, 0);
    }

    #[tokio::test]
    async fn openshell_check() {
        // openshell likely not installed in CI, but this shouldn't panic.
        let _available = openshell_available().await;
    }
}
