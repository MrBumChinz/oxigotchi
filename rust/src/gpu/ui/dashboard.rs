use crate::gpu::state::gpu_state::GpuFeatureState;

#[derive(Debug, Clone, Default)]
pub struct GpuDashboardSummary {
    pub mode: String,
    pub strongest_signal: Option<String>,
}

impl From<&GpuFeatureState> for GpuDashboardSummary {
    fn from(state: &GpuFeatureState) -> Self {
        Self {
            mode: format!("{:?}", state.mode),
            strongest_signal: state
                .runtime
                .strongest_signal
                .as_ref()
                .map(|s| format!("{s:?}")),
        }
    }
}
