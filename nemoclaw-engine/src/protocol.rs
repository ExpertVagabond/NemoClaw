use chrono::Utc;

/// Emit a progress line to stdout in the NemoClaw protocol format.
pub fn emit_progress(percent: u8, label: &str) {
    println!("PROGRESS:{percent}:{label}");
}

/// Emit the run ID to stdout.
pub fn emit_run_id(run_id: &str) {
    println!("RUN_ID:{run_id}");
}

/// Generate a NemoClaw run ID: nc-YYYYMMDD-HHMMSS-<8 hex chars>
pub fn generate_run_id() -> String {
    let now = Utc::now();
    let hex: String = uuid::Uuid::new_v4().to_string()[..8].to_string();
    format!("nc-{}-{hex}", now.format("%Y%m%d-%H%M%S"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_format() {
        let id = generate_run_id();
        assert!(id.starts_with("nc-"));
        assert_eq!(id.len(), 27); // nc-YYYYMMDD-HHMMSS-8hex
    }
}
