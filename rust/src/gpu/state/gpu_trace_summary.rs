use crate::gpu::runtime::trace::GpuRuntimeSummary;

#[derive(Debug, Clone, Default)]
pub struct GpuTraceProjection {
    pub headline: String,
    pub detail: String,
}

impl From<&GpuRuntimeSummary> for GpuTraceProjection {
    fn from(summary: &GpuRuntimeSummary) -> Self {
        Self {
            headline: summary
                .strongest_signal
                .as_ref()
                .map(|s| format!("{s:?}"))
                .unwrap_or_else(|| "NoGpuSignal".into()),
            detail: format!(
                "card0={} renderD128={} submit={}",
                summary.card0_seen, summary.renderd128_seen, summary.vc4_submit_cl_seen
            ),
        }
    }
}
