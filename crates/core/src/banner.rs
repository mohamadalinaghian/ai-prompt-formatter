/// Exit codes as specified in Section 4.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Ok = 0,
    Degraded = 10,
    Abort = 1,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// Degradation information recording which stages failed and the resulting impact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Degradation {
    pub failed_stages: Vec<String>,
    pub result: String,
}

impl Degradation {
    pub fn new(failed_stages: Vec<String>, result: impl Into<String>) -> Self {
        Self {
            failed_stages,
            result: result.into(),
        }
    }

    /// Renders the warning banner to prepend to the XML output.
    pub fn render_banner(&self) -> String {
        let stages_str = if self.failed_stages.is_empty() {
            "none".to_string()
        } else {
            self.failed_stages.join(", ")
        };

        format!(
            "⚠️⚠️⚠️ PFMT DEGRADED OUTPUT: READ BEFORE SENDING ⚠️⚠️⚠️\n\
Failed stages: {}\n\
Result: {}\n\
Delete this banner before sending it to the agent.\n\
⚠️⚠️⚠️ ─────────────────────────────────────────── ⚠️⚠️⚠️",
            stages_str, self.result
        )
    }

    /// One-line summary suitable for stderr notification.
    pub fn stderr_status(&self) -> String {
        format!(
            "DEGRADED: failed stages [{}] - {}",
            self.failed_stages.join(", "),
            self.result
        )
    }
}
