mod actions;
mod blueprint;
mod protocol;
mod shell;
mod state;

use actions::{apply, plan, rollback, status};
use clap::{Parser, Subcommand};
use protocol::emit_run_id;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "nemoclaw-engine", version, about = "NemoClaw blueprint orchestrator")]
struct Cli {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Validate blueprint and generate a deployment plan
    Plan {
        #[arg(long, default_value = "default")]
        profile: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        endpoint_url: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create sandbox and configure inference
    Apply {
        #[arg(long, default_value = "default")]
        profile: String,
        #[arg(long)]
        plan: Option<String>,
        #[arg(long)]
        endpoint_url: Option<String>,
    },
    /// Report current run state
    Status {
        #[arg(long)]
        run_id: Option<String>,
    },
    /// Stop and remove sandbox
    Rollback {
        #[arg(long)]
        run_id: String,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let run_id = protocol::generate_run_id();
    emit_run_id(&run_id);

    let result = match cli.action {
        Action::Plan {
            profile,
            dry_run,
            endpoint_url,
            json: _,
        } => plan::execute(&run_id, &profile, dry_run, endpoint_url.as_deref()).await,
        Action::Apply {
            profile,
            plan: plan_path,
            endpoint_url,
        } => {
            apply::execute(
                &run_id,
                &profile,
                plan_path.as_deref(),
                endpoint_url.as_deref(),
            )
            .await
        }
        Action::Status { run_id: rid } => status::execute(rid.as_deref()).await,
        Action::Rollback { run_id: rid } => rollback::execute(&rid).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
