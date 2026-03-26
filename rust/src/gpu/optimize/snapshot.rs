use crate::gpu::runtime::signals::GpuRuntimeSignal;
use crate::gpu::runtime::trace::GpuRuntimeSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotPolicy {
    FlushImmediate,
    CoalesceLight,
    CoalesceAggressive,
}

impl SnapshotPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            SnapshotPolicy::FlushImmediate => "flush_immediate",
            SnapshotPolicy::CoalesceLight => "coalesce_light",
            SnapshotPolicy::CoalesceAggressive => "coalesce_aggressive",
        }
    }

    pub fn threshold(&self) -> u32 {
        match self {
            SnapshotPolicy::FlushImmediate => 1,
            SnapshotPolicy::CoalesceLight => 2,
            SnapshotPolicy::CoalesceAggressive => 3,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotOptimizer {
    pending_updates: u32,
}

impl SnapshotOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_update(&mut self) {
        self.pending_updates = self.pending_updates.saturating_add(1);
    }

    pub fn should_flush(&self, threshold: u32) -> bool {
        self.pending_updates >= threshold
    }

    pub fn policy_for(&self, summary: &GpuRuntimeSummary) -> SnapshotPolicy {
        match summary.strongest_signal {
            Some(GpuRuntimeSignal::GpuSubmissionObserved) => SnapshotPolicy::CoalesceAggressive,
            Some(GpuRuntimeSignal::RenderSetupActive) => SnapshotPolicy::CoalesceLight,
            _ => SnapshotPolicy::FlushImmediate,
        }
    }

    pub fn clear(&mut self) {
        self.pending_updates = 0;
    }
}
