use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuRuntimeSignal {
    DisplayInspectOnly,
    RenderSetupActive,
    GpuSubmissionObserved,
}
