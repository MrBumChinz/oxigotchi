use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::signals::GpuRuntimeSignal;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuRuntimeSummary {
    pub last_seen_at: Option<DateTime<Utc>>,
    pub card0_seen: bool,
    pub renderd128_seen: bool,
    pub vc4_setup_seen: bool,
    pub vc4_submit_cl_seen: bool,
    pub strongest_signal: Option<GpuRuntimeSignal>,
}

impl GpuRuntimeSummary {
    pub fn classify(&mut self) {
        self.strongest_signal = if self.vc4_submit_cl_seen {
            Some(GpuRuntimeSignal::GpuSubmissionObserved)
        } else if self.renderd128_seen || self.vc4_setup_seen {
            Some(GpuRuntimeSignal::RenderSetupActive)
        } else if self.card0_seen {
            Some(GpuRuntimeSignal::DisplayInspectOnly)
        } else {
            None
        };
    }
}
