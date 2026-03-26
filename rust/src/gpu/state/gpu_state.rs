use serde::{Deserialize, Serialize};

use crate::gpu::runtime::trace::GpuRuntimeSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuMode {
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "observe")]
    Observe,
    #[serde(rename = "optimize")]
    Optimize,
    #[serde(rename = "lab")]
    Lab,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuFeatureState {
    pub mode: GpuMode,
    pub runtime: GpuRuntimeSummary,
}

impl Default for GpuFeatureState {
    fn default() -> Self {
        Self {
            mode: GpuMode::Off,
            runtime: GpuRuntimeSummary::default(),
        }
    }
}
