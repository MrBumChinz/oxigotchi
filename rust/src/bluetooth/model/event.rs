use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::config::BtMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BtEventSource {
    Bluez,
    Btmon,
    Controller,
    OxiGotchi,
    Rf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BtSeverity {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BtEventKind {
    StackUp,
    StackDown,
    ModeChanged,
    DeviceNew,
    DeviceUpdate,
    DeviceLost,
    SummaryTick,
    ControllerSnapshot,
    ControllerProbeOk,
    ControllerProbeFail,
    CoexOverlapStart,
    CoexOverlapStop,
    CoexContentionSuspected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtEvent<T> {
    pub version: u16,
    pub ts: DateTime<Utc>,
    pub event: BtEventKind,
    pub source: BtEventSource,
    pub mode: BtMode,
    pub severity: BtSeverity,
    pub payload: T,
}
