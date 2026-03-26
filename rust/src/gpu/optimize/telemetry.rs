use crate::gpu::runtime::trace::GpuRuntimeSummary;

#[derive(Debug, Clone, Default)]
pub struct GpuTelemetrySummary {
    pub strongest_signal: Option<String>,
}

impl From<&GpuRuntimeSummary> for GpuTelemetrySummary {
    fn from(summary: &GpuRuntimeSummary) -> Self {
        Self {
            strongest_signal: summary.strongest_signal.as_ref().map(|s| format!("{s:?}")),
        }
    }
}
