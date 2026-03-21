//! Web dashboard module (axum HTTP server).
//!
//! Provides a REST API and embedded HTML dashboard for monitoring
//! and configuring oxigotchi. The axum router shares DaemonState via
//! Arc<Mutex<DaemonState>>.
//!
//! Many types and constants are defined for future endpoints that will be
//! wired in when the corresponding daemon features are connected.

use axum::{
    extract::State,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Shared daemon state (the web server reads/writes this via Arc<Mutex>)
// ---------------------------------------------------------------------------

/// Snapshot of all daemon state that the web server needs access to.
pub struct DaemonState {
    // -- status --
    pub name: String,
    pub uptime_str: String,
    pub epoch: u64,
    pub channel: u8,
    pub aps_seen: u32,
    pub handshakes: u32,
    pub blind_epochs: u32,
    pub mood: f32,
    pub face: String,
    pub status_message: String,
    pub mode: String,

    // -- attacks --
    pub total_attacks: u64,
    pub total_handshakes_attacks: u64,
    pub attack_rate: u32,
    pub deauths_this_epoch: u32,

    // -- captures --
    pub capture_files: usize,
    pub handshake_files: usize,
    pub pending_upload: usize,
    pub total_capture_size: u64,
    pub capture_list: Vec<CaptureEntry>,

    // -- battery --
    pub battery_level: u8,
    pub battery_charging: bool,
    pub battery_voltage_mv: u16,
    pub battery_low: bool,
    pub battery_critical: bool,
    pub battery_available: bool,

    // -- wifi --
    pub wifi_state: String,
    pub wifi_aps_tracked: usize,

    // -- ao --
    pub ao_state: String,
    pub ao_pid: u32,
    pub ao_crash_count: u32,
    pub ao_uptime: String,

    // -- system --
    pub boot_time: Instant,

    // -- action requests from web -> daemon --
    pub pending_mode_switch: Option<String>,
    pub pending_rate_change: Option<u32>,
    pub pending_restart: bool,
}

impl DaemonState {
    /// Create a default state for startup.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            uptime_str: "00:00:00".into(),
            epoch: 0,
            channel: 0,
            aps_seen: 0,
            handshakes: 0,
            blind_epochs: 0,
            mood: 0.5,
            face: "(O_O)".into(),
            status_message: "Booting...".into(),
            mode: "AO".into(),
            total_attacks: 0,
            total_handshakes_attacks: 0,
            attack_rate: 1,
            deauths_this_epoch: 0,
            capture_files: 0,
            handshake_files: 0,
            pending_upload: 0,
            total_capture_size: 0,
            capture_list: Vec::new(),
            battery_level: 100,
            battery_charging: false,
            battery_voltage_mv: 4200,
            battery_low: false,
            battery_critical: false,
            battery_available: false,
            wifi_state: "Down".into(),
            wifi_aps_tracked: 0,
            ao_state: "STOPPED".into(),
            ao_pid: 0,
            ao_crash_count: 0,
            ao_uptime: "N/A".into(),
            boot_time: Instant::now(),
            pending_mode_switch: None,
            pending_rate_change: None,
            pending_restart: false,
        }
    }
}

/// Shared state type used by axum handlers.
pub type SharedState = Arc<Mutex<DaemonState>>;

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

/// System status snapshot returned by /api/status.
#[derive(Debug, Clone, Serialize)]
pub struct StatusResponse {
    pub name: String,
    pub version: String,
    pub uptime: String,
    pub epoch: u64,
    pub channel: u8,
    pub aps_seen: u32,
    pub handshakes: u32,
    pub blind_epochs: u32,
    pub mood: f32,
    pub face: String,
    pub status_message: String,
    pub mode: String,
}

/// Attack stats returned by /api/attacks.
#[derive(Debug, Clone, Serialize)]
pub struct AttackStats {
    pub total_attacks: u64,
    pub total_handshakes: u64,
    pub attack_rate: u32,
    pub deauths_this_epoch: u32,
}

/// Capture info returned by /api/captures.
#[derive(Debug, Clone, Serialize)]
pub struct CaptureInfo {
    pub total_files: usize,
    pub handshake_files: usize,
    pub pending_upload: usize,
    pub total_size_bytes: u64,
    pub files: Vec<CaptureEntry>,
}

/// A single capture file entry.
#[derive(Debug, Clone, Serialize)]
pub struct CaptureEntry {
    pub filename: String,
    pub size_bytes: u64,
}

/// Health response returned by /api/health.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub wifi_state: String,
    pub battery_level: u8,
    pub battery_charging: bool,
    pub battery_available: bool,
    pub uptime_secs: u64,
    pub ao_state: String,
    pub ao_pid: u32,
    pub ao_crash_count: u32,
    pub ao_uptime: String,
}

/// Mode switch request for POST /api/mode.
#[derive(Debug, Clone, Deserialize)]
pub struct ModeSwitch {
    pub mode: String,
}

/// Rate change request for POST /api/rate.
#[derive(Debug, Clone, Deserialize)]
pub struct RateChange {
    pub rate: u32,
}

/// Generic action response.
#[derive(Debug, Clone, Serialize)]
pub struct ActionResponse {
    pub ok: bool,
    pub message: String,
}

/// Config update request for /api/config.
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigUpdate {
    pub name: Option<String>,
    pub attack_rate: Option<u32>,
    pub channel_dwell_ms: Option<u64>,
    pub whitelist_add: Option<String>,
    pub whitelist_remove: Option<String>,
}

/// Battery info returned by /api/battery.
#[derive(Debug, Clone, Serialize)]
pub struct BatteryInfo {
    pub level: u8,
    pub charging: bool,
    pub voltage_mv: u16,
    pub low: bool,
    pub critical: bool,
}

/// WiFi info returned by /api/wifi.
#[derive(Debug, Clone, Serialize)]
pub struct WifiInfo {
    pub state: String,
    pub channel: u8,
    pub aps_tracked: usize,
    pub channels: Vec<u8>,
    pub dwell_ms: u64,
}

/// Bluetooth info returned by /api/bluetooth.
#[derive(Debug, Clone, Serialize)]
pub struct BluetoothInfo {
    pub state: String,
    pub phone_mac: String,
    pub internet_available: bool,
    pub retry_count: u32,
}

/// Recovery/health info returned by /api/recovery.
#[derive(Debug, Clone, Serialize)]
pub struct RecoveryInfo {
    pub state: String,
    pub total_recoveries: u32,
    pub soft_retries: u32,
    pub hard_retries: u32,
    pub diagnostic_count: usize,
}

/// Personality/mood info returned by /api/personality.
#[derive(Debug, Clone, Serialize)]
pub struct PersonalityInfo {
    pub mood: f32,
    pub face: String,
    pub blind_epochs: u32,
    pub total_handshakes: u32,
    pub total_aps_seen: u32,
    pub xp: u64,
    pub level: u32,
}

/// System info returned by /api/system.
#[derive(Debug, Clone, Serialize)]
pub struct SystemInfoResponse {
    pub cpu_temp_c: f32,
    pub mem_used_mb: u32,
    pub mem_total_mb: u32,
    pub cpu_percent: f32,
}

/// Handshake file entry returned by /api/handshakes.
#[derive(Debug, Clone, Serialize)]
pub struct HandshakeEntry {
    pub filename: String,
    pub ssid: String,
    pub size_bytes: u64,
    pub uploaded: bool,
}

// ---------------------------------------------------------------------------
// API route constants
// ---------------------------------------------------------------------------

pub const API_STATUS: &str = "/api/status";
pub const API_ATTACKS: &str = "/api/attacks";
pub const API_CAPTURES: &str = "/api/captures";
pub const API_CONFIG: &str = "/api/config";
pub const API_DISPLAY: &str = "/api/display.png";
pub const API_BATTERY: &str = "/api/battery";
pub const API_WIFI: &str = "/api/wifi";
pub const API_BLUETOOTH: &str = "/api/bluetooth";
pub const API_RECOVERY: &str = "/api/recovery";
pub const API_PERSONALITY: &str = "/api/personality";
pub const API_SYSTEM: &str = "/api/system";
pub const API_HANDSHAKES: &str = "/api/handshakes";
pub const API_HANDSHAKE_DL: &str = "/api/handshakes/:filename";
pub const API_MODE: &str = "/api/mode";
pub const API_RESTART: &str = "/api/restart";
pub const API_SHUTDOWN: &str = "/api/shutdown";
pub const API_WHITELIST: &str = "/api/whitelist";
pub const API_CRACKED: &str = "/api/cracked";
pub const API_HEALTH: &str = "/api/health";
pub const API_RATE: &str = "/api/rate";

// ---------------------------------------------------------------------------
// StatusParams helper (used by main.rs to build StatusResponse)
// ---------------------------------------------------------------------------

/// Parameters for building a [`StatusResponse`].
pub struct StatusParams<'a> {
    pub name: &'a str,
    pub uptime: &'a str,
    pub epoch: u64,
    pub channel: u8,
    pub aps_seen: u32,
    pub handshakes: u32,
    pub blind_epochs: u32,
    pub mood: f32,
    pub face: &'a str,
    pub status_message: &'a str,
    pub mode: &'a str,
}

/// Build a [`StatusResponse`] from a [`StatusParams`] snapshot.
pub fn build_status(p: &StatusParams<'_>) -> StatusResponse {
    StatusResponse {
        name: p.name.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: p.uptime.to_string(),
        epoch: p.epoch,
        channel: p.channel,
        aps_seen: p.aps_seen,
        handshakes: p.handshakes,
        blind_epochs: p.blind_epochs,
        mood: p.mood,
        face: p.face.to_string(),
        status_message: p.status_message.to_string(),
        mode: p.mode.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Axum route handlers
// ---------------------------------------------------------------------------

/// GET / -> dashboard HTML
async fn dashboard_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

/// GET /api/status -> JSON status
async fn status_handler(State(state): State<SharedState>) -> Json<StatusResponse> {
    let s = state.lock().unwrap();
    Json(StatusResponse {
        name: s.name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: s.uptime_str.clone(),
        epoch: s.epoch,
        channel: s.channel,
        aps_seen: s.aps_seen,
        handshakes: s.handshakes,
        blind_epochs: s.blind_epochs,
        mood: s.mood,
        face: s.face.clone(),
        status_message: s.status_message.clone(),
        mode: s.mode.clone(),
    })
}

/// GET /api/captures -> JSON capture list
async fn captures_handler(State(state): State<SharedState>) -> Json<CaptureInfo> {
    let s = state.lock().unwrap();
    Json(CaptureInfo {
        total_files: s.capture_files,
        handshake_files: s.handshake_files,
        pending_upload: s.pending_upload,
        total_size_bytes: s.total_capture_size,
        files: s.capture_list.clone(),
    })
}

/// GET /api/health -> JSON system health
async fn health_handler(State(state): State<SharedState>) -> Json<HealthResponse> {
    let s = state.lock().unwrap();
    Json(HealthResponse {
        wifi_state: s.wifi_state.clone(),
        battery_level: s.battery_level,
        battery_charging: s.battery_charging,
        battery_available: s.battery_available,
        uptime_secs: s.boot_time.elapsed().as_secs(),
        ao_state: s.ao_state.clone(),
        ao_pid: s.ao_pid,
        ao_crash_count: s.ao_crash_count,
        ao_uptime: s.ao_uptime.clone(),
    })
}

/// POST /api/mode -> switch mode
async fn mode_handler(
    State(state): State<SharedState>,
    Json(body): Json<ModeSwitch>,
) -> Json<ActionResponse> {
    let mut s = state.lock().unwrap();
    let new_mode = if body.mode == "toggle" {
        if s.mode == "AO" { "PWN".to_string() } else { "AO".to_string() }
    } else {
        body.mode.to_uppercase()
    };
    s.pending_mode_switch = Some(new_mode.clone());
    Json(ActionResponse {
        ok: true,
        message: format!("Mode switch to {} queued", new_mode),
    })
}

/// POST /api/rate -> change attack rate
async fn rate_handler(
    State(state): State<SharedState>,
    Json(body): Json<RateChange>,
) -> Json<ActionResponse> {
    let rate = body.rate.clamp(1, 3);
    let mut s = state.lock().unwrap();
    s.pending_rate_change = Some(rate);
    Json(ActionResponse {
        ok: true,
        message: format!("Rate change to {} queued", rate),
    })
}

/// POST /api/restart -> restart AO
async fn restart_handler(State(state): State<SharedState>) -> Json<ActionResponse> {
    let mut s = state.lock().unwrap();
    s.pending_restart = true;
    Json(ActionResponse {
        ok: true,
        message: "AO restart queued".into(),
    })
}

// ---------------------------------------------------------------------------
// Router builder
// ---------------------------------------------------------------------------

/// Build the axum router with all routes, sharing daemon state.
pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/", get(dashboard_handler))
        .route(API_STATUS, get(status_handler))
        .route(API_CAPTURES, get(captures_handler))
        .route(API_HEALTH, get(health_handler))
        .route(API_MODE, post(mode_handler))
        .route(API_RATE, post(rate_handler))
        .route(API_RESTART, post(restart_handler))
        .with_state(state)
}

/// Start the axum web server on 0.0.0.0:8080.
/// This function is async and should be spawned as a tokio task.
pub async fn start_server(state: SharedState) {
    let app = build_router(state);
    let listener = match tokio::net::TcpListener::bind("0.0.0.0:8080").await {
        Ok(l) => l,
        Err(e) => {
            log::error!("failed to bind web server on 0.0.0.0:8080: {e}");
            return;
        }
    };
    log::info!("web dashboard listening on http://0.0.0.0:8080");
    if let Err(e) = axum::serve(listener, app).await {
        log::error!("web server error: {e}");
    }
}

// ---------------------------------------------------------------------------
// Embedded dashboard HTML
// ---------------------------------------------------------------------------

pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>oxigotchi</title>
    <style>
        body { font-family: monospace; background: #1a1a2e; color: #e0e0e0; margin: 20px; }
        .card { background: #16213e; border-radius: 8px; padding: 16px; margin: 8px 0; }
        .face { font-size: 48px; text-align: center; padding: 20px; }
        .stat { display: inline-block; margin: 8px 16px; }
        .label { color: #888; font-size: 12px; }
        .value { font-size: 20px; color: #0f0; }
        .warn { color: #ff0; }
        .err { color: #f00; }
        h1 { color: #e94560; }
        h3 { color: #aaa; margin: 0 0 8px 0; }
        .refresh { color: #666; font-size: 10px; }
        .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
        .btn { background: #0a3d62; border: 1px solid #3c6382; color: #e0e0e0;
               padding: 8px 16px; cursor: pointer; border-radius: 4px; font-family: monospace; }
        .btn:hover { background: #3c6382; }
    </style>
</head>
<body>
    <h1 id="name">oxigotchi</h1>
    <!-- Face -->
    <div class="card">
        <div class="face" id="face">(O_O)</div>
        <div style="text-align:center" id="status">Loading...</div>
    </div>
    <!-- Core stats -->
    <div class="card">
        <div class="stat"><div class="label">CH</div><div class="value" id="ch">-</div></div>
        <div class="stat"><div class="label">APS</div><div class="value" id="aps">-</div></div>
        <div class="stat"><div class="label">PWND</div><div class="value" id="pwnd">-</div></div>
        <div class="stat"><div class="label">EPOCH</div><div class="value" id="epoch">-</div></div>
        <div class="stat"><div class="label">UPTIME</div><div class="value" id="uptime">-</div></div>
        <div class="stat"><div class="label">MOOD</div><div class="value" id="mood">-</div></div>
    </div>
    <div class="grid">
    <!-- Battery -->
    <div class="card">
        <h3>Battery</h3>
        <div class="stat"><div class="label">LEVEL</div><div class="value" id="bat_level">-</div></div>
        <div class="stat"><div class="label">STATE</div><div class="value" id="bat_state">-</div></div>
    </div>
    <!-- WiFi -->
    <div class="card">
        <h3>WiFi</h3>
        <div class="stat"><div class="label">STATE</div><div class="value" id="wifi_state">-</div></div>
        <div class="stat"><div class="label">AO</div><div class="value" id="ao_state">-</div></div>
    </div>
    <!-- Captures -->
    <div class="card">
        <h3>Captures</h3>
        <div class="stat"><div class="label">FILES</div><div class="value" id="cap_files">-</div></div>
        <div class="stat"><div class="label">PENDING</div><div class="value" id="cap_pending">-</div></div>
        <div id="cap_list" style="font-size:11px;max-height:150px;overflow-y:auto;margin-top:8px"></div>
    </div>
    <!-- Mode -->
    <div class="card">
        <h3>Mode</h3>
        <div class="stat"><div class="label">CURRENT</div><div class="value" id="mode">-</div></div>
        <button class="btn" onclick="toggleMode()">Toggle AO/PWN</button>
    </div>
    <!-- Actions -->
    <div class="card">
        <h3>Actions</h3>
        <button class="btn" onclick="restartAO()">Restart AO</button>
        <button class="btn" onclick="setRate(1)">Rate 1</button>
        <button class="btn" onclick="setRate(2)">Rate 2</button>
    </div>
    <!-- Health -->
    <div class="card">
        <h3>System Health</h3>
        <div class="stat"><div class="label">AO PID</div><div class="value" id="ao_pid">-</div></div>
        <div class="stat"><div class="label">CRASHES</div><div class="value" id="ao_crashes">-</div></div>
        <div class="stat"><div class="label">AO UP</div><div class="value" id="ao_uptime">-</div></div>
        <div class="stat"><div class="label">SYS UP</div><div class="value" id="sys_uptime">-</div></div>
    </div>
    </div>
    <div class="refresh">Auto-refreshes every 5s</div>
    <script>
        function update() {
            fetch('/api/status')
                .then(r => r.json())
                .then(d => {
                    document.getElementById('name').textContent = d.name + '>';
                    document.getElementById('face').textContent = d.face;
                    document.getElementById('status').textContent = d.status_message;
                    document.getElementById('ch').textContent = d.channel;
                    document.getElementById('aps').textContent = d.aps_seen;
                    document.getElementById('pwnd').textContent = d.handshakes;
                    document.getElementById('epoch').textContent = d.epoch;
                    document.getElementById('uptime').textContent = d.uptime;
                    document.getElementById('mood').textContent = Math.round(d.mood * 100) + '%';
                    document.getElementById('mode').textContent = d.mode;
                })
                .catch(console.error);

            fetch('/api/health')
                .then(r => r.json())
                .then(d => {
                    document.getElementById('wifi_state').textContent = d.wifi_state;
                    document.getElementById('bat_level').textContent = d.battery_available ? d.battery_level + '%' : 'N/A';
                    document.getElementById('bat_state').textContent = d.battery_charging ? 'Charging' : 'Battery';
                    document.getElementById('ao_state').textContent = d.ao_state;
                    document.getElementById('ao_pid').textContent = d.ao_pid || '-';
                    document.getElementById('ao_crashes').textContent = d.ao_crash_count;
                    document.getElementById('ao_uptime').textContent = d.ao_uptime;
                    var h = Math.floor(d.uptime_secs / 3600);
                    var m = Math.floor((d.uptime_secs % 3600) / 60);
                    var s = d.uptime_secs % 60;
                    document.getElementById('sys_uptime').textContent =
                        String(h).padStart(2,'0') + ':' + String(m).padStart(2,'0') + ':' + String(s).padStart(2,'0');
                })
                .catch(console.error);

            fetch('/api/captures')
                .then(r => r.json())
                .then(d => {
                    document.getElementById('cap_files').textContent = d.total_files;
                    document.getElementById('cap_pending').textContent = d.pending_upload;
                    var list = d.files.map(f => f.filename + ' (' + (f.size_bytes/1024).toFixed(1) + 'K)').join('<br>');
                    document.getElementById('cap_list').innerHTML = list || 'No captures yet';
                })
                .catch(console.error);
        }
        function toggleMode() {
            fetch('/api/mode', {method:'POST', headers:{'Content-Type':'application/json'},
                body:JSON.stringify({mode:'toggle'})}).then(update);
        }
        function restartAO() {
            fetch('/api/restart', {method:'POST'}).then(update);
        }
        function setRate(r) {
            fetch('/api/rate', {method:'POST', headers:{'Content-Type':'application/json'},
                body:JSON.stringify({rate:r})}).then(update);
        }
        update();
        setInterval(update, 5000);
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_status() {
        let status = build_status(&StatusParams {
            name: "oxi", uptime: "00:01:23", epoch: 42, channel: 6,
            aps_seen: 10, handshakes: 3, blind_epochs: 2, mood: 0.75,
            face: "(^_^)", status_message: "Having fun!", mode: "AO",
        });
        assert_eq!(status.name, "oxi");
        assert_eq!(status.epoch, 42);
        assert_eq!(status.channel, 6);
        assert_eq!(status.handshakes, 3);
        assert!(!status.version.is_empty());
    }

    #[test]
    fn test_status_serializes() {
        let status = build_status(&StatusParams {
            name: "oxi", uptime: "00:00:00", epoch: 0, channel: 1,
            aps_seen: 0, handshakes: 0, blind_epochs: 0, mood: 0.5,
            face: "(O_O)", status_message: "Booting", mode: "AO",
        });
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"name\":\"oxi\""));
        assert!(json.contains("\"epoch\":0"));
    }

    #[test]
    fn test_api_paths() {
        assert_eq!(API_STATUS, "/api/status");
        assert_eq!(API_ATTACKS, "/api/attacks");
        assert_eq!(API_CAPTURES, "/api/captures");
        assert_eq!(API_CONFIG, "/api/config");
        assert_eq!(API_DISPLAY, "/api/display.png");
        assert_eq!(API_BATTERY, "/api/battery");
        assert_eq!(API_WIFI, "/api/wifi");
        assert_eq!(API_BLUETOOTH, "/api/bluetooth");
        assert_eq!(API_RECOVERY, "/api/recovery");
        assert_eq!(API_PERSONALITY, "/api/personality");
        assert_eq!(API_SYSTEM, "/api/system");
        assert_eq!(API_HANDSHAKES, "/api/handshakes");
        assert_eq!(API_HANDSHAKE_DL, "/api/handshakes/:filename");
        assert_eq!(API_MODE, "/api/mode");
        assert_eq!(API_RESTART, "/api/restart");
        assert_eq!(API_SHUTDOWN, "/api/shutdown");
        assert_eq!(API_WHITELIST, "/api/whitelist");
        assert_eq!(API_CRACKED, "/api/cracked");
        assert_eq!(API_HEALTH, "/api/health");
        assert_eq!(API_RATE, "/api/rate");
    }

    #[test]
    fn test_dashboard_html_contains_elements() {
        assert!(DASHBOARD_HTML.contains("<title>oxigotchi</title>"));
        assert!(DASHBOARD_HTML.contains("/api/status"));
        assert!(DASHBOARD_HTML.contains("/api/health"));
        assert!(DASHBOARD_HTML.contains("/api/captures"));
        assert!(DASHBOARD_HTML.contains("toggleMode"));
        assert!(DASHBOARD_HTML.contains("restartAO"));
    }

    #[test]
    fn test_battery_info_serialize() {
        let info = BatteryInfo {
            level: 75,
            charging: true,
            voltage_mv: 4100,
            low: false,
            critical: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"level\":75"));
        assert!(json.contains("\"charging\":true"));
    }

    #[test]
    fn test_wifi_info_serialize() {
        let info = WifiInfo {
            state: "Monitor".into(),
            channel: 6,
            aps_tracked: 15,
            channels: vec![1, 6, 11],
            dwell_ms: 250,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"state\":\"Monitor\""));
    }

    #[test]
    fn test_personality_info_serialize() {
        let info = PersonalityInfo {
            mood: 0.75,
            face: "(^_^)".into(),
            blind_epochs: 2,
            total_handshakes: 10,
            total_aps_seen: 50,
            xp: 420,
            level: 3,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"level\":3"));
        assert!(json.contains("\"xp\":420"));
    }

    #[test]
    fn test_mode_switch_deserialize() {
        let json = r#"{"mode": "MANU"}"#;
        let ms: ModeSwitch = serde_json::from_str(json).unwrap();
        assert_eq!(ms.mode, "MANU");
    }

    #[test]
    fn test_config_update_deserialize() {
        let json = r#"{"name": "mybot", "attack_rate": 1}"#;
        let update: ConfigUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.name.unwrap(), "mybot");
        assert_eq!(update.attack_rate.unwrap(), 1);
        assert!(update.whitelist_add.is_none());
    }

    #[test]
    fn test_attack_stats_serialize() {
        let stats = AttackStats {
            total_attacks: 100,
            total_handshakes: 5,
            attack_rate: 1,
            deauths_this_epoch: 3,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_attacks\":100"));
    }

    #[test]
    fn test_capture_info_serialize() {
        let info = CaptureInfo {
            total_files: 10,
            handshake_files: 3,
            pending_upload: 2,
            total_size_bytes: 1024000,
            files: vec![],
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"handshake_files\":3"));
    }

    #[test]
    fn test_daemon_state_new() {
        let ds = DaemonState::new("testbot");
        assert_eq!(ds.name, "testbot");
        assert_eq!(ds.mode, "AO");
        assert_eq!(ds.epoch, 0);
        assert!(!ds.pending_restart);
    }

    #[test]
    fn test_health_response_serialize() {
        let health = HealthResponse {
            wifi_state: "Monitor".into(),
            battery_level: 80,
            battery_charging: false,
            battery_available: true,
            uptime_secs: 3600,
            ao_state: "RUNNING".into(),
            ao_pid: 1234,
            ao_crash_count: 0,
            ao_uptime: "01:00:00".into(),
        };
        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"ao_pid\":1234"));
    }

    #[test]
    fn test_rate_change_deserialize() {
        let json = r#"{"rate": 2}"#;
        let rc: RateChange = serde_json::from_str(json).unwrap();
        assert_eq!(rc.rate, 2);
    }

    #[test]
    fn test_action_response_serialize() {
        let resp = ActionResponse {
            ok: true,
            message: "done".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"ok\":true"));
    }

    #[test]
    fn test_build_router_compiles() {
        let state = Arc::new(Mutex::new(DaemonState::new("test")));
        let _router = build_router(state);
        // Just verify it builds without panic
    }
}
