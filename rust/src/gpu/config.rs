use serde::{Deserialize, Serialize};

use crate::gpu::state::gpu_state::GpuMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuFeatureConfig {
    pub enabled: bool,
    pub mode: GpuMode,
    #[serde(default)]
    pub runtime: GpuRuntimeConfig,
    #[serde(default)]
    pub optimize: GpuOptimizeConfig,
    #[serde(default)]
    pub ui: GpuUiConfig,
    #[serde(default)]
    pub lab: GpuLabConfig,
}

impl Default for GpuFeatureConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: GpuMode::Observe,
            runtime: GpuRuntimeConfig::default(),
            optimize: GpuOptimizeConfig::default(),
            ui: GpuUiConfig::default(),
            lab: GpuLabConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRuntimeConfig {
    #[serde(default = "default_true")]
    pub trace_enabled: bool,
    #[serde(default = "default_true")]
    pub capture_summary_only: bool,
    #[serde(default = "default_signal_window")]
    pub signal_window_sec: u64,
    #[serde(default)]
    pub summary_source: String,
}

fn default_true() -> bool { true }
fn default_signal_window() -> u64 { 30 }

impl Default for GpuRuntimeConfig {
    fn default() -> Self {
        Self {
            trace_enabled: true,
            capture_summary_only: true,
            signal_window_sec: 30,
            summary_source: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuOptimizeConfig {
    pub snapshot_coalescing_enabled: bool,
    pub telemetry_enabled: bool,
    pub batching_enabled: bool,
}

impl Default for GpuOptimizeConfig {
    fn default() -> Self {
        Self {
            snapshot_coalescing_enabled: true,
            telemetry_enabled: true,
            batching_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuUiConfig {
    pub dashboard_enabled: bool,
    pub eink_summary_enabled: bool,
    pub eink_min_interval_sec: u64,
}

impl Default for GpuUiConfig {
    fn default() -> Self {
        Self {
            dashboard_enabled: true,
            eink_summary_enabled: true,
            eink_min_interval_sec: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuLabConfig {
    pub gles_probe_enabled: bool,
    pub deep_graphics_enabled: bool,
    pub compute_experiments_enabled: bool,
    pub require_explicit_flag: bool,
}

impl Default for GpuLabConfig {
    fn default() -> Self {
        Self {
            gles_probe_enabled: false,
            deep_graphics_enabled: false,
            compute_experiments_enabled: false,
            require_explicit_flag: true,
        }
    }
}
