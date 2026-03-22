use crate::state::{latest_run, load_state};
use anyhow::Result;
use serde_json::json;

pub async fn execute(run_id: Option<&str>) -> Result<()> {
    let state = if let Some(id) = run_id {
        load_state(id)?
    } else {
        latest_run()?
    };

    match state {
        Some(s) => {
            println!("{}", serde_json::to_string_pretty(&s)?);
        }
        None => {
            let id = run_id.unwrap_or("unknown");
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "run_id": id,
                    "status": "unknown"
                }))?
            );
        }
    }

    Ok(())
}
