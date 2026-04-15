//! BlueZ D-Bus wrapper module.
//!
//! Provides a clean API over BlueZ D-Bus interactions for PAN tethering,
//! device pairing/trust/removal, adapter scanning, and agent registration.
//!
//! On non-Linux platforms, stub implementations allow compilation and testing.

// ─── Shared types (not platform-gated) ───────────────────────────────────────

/// NAP (Network Access Point) profile UUID for BT PAN tethering.
pub const NAP_UUID: &str = "00001116-0000-1000-8000-00805f9b34fb";

/// Represents an active PAN (Personal Area Network) connection.
#[derive(Debug, Clone)]
pub struct PanConnection {
    /// The network interface name (e.g. "bnep0").
    pub interface: String,
}

/// Actionable PAN connection errors (mapped from D-Bus error strings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanConnectError {
    /// BT tethering not enabled on the phone.
    TetherNotEnabled,
    /// Phone unpaired the Pi — need to remove and re-pair.
    PhoneUnpaired,
    /// Phone out of range or BT off (page timeout, host down).
    PhoneOutOfRange,
    /// Transient radio/stack error — retry silently, no user hint.
    TransientRadioError,
    /// Connection already in progress — retry after short delay.
    PhoneBusy,
    /// Phone not responding (timeout / no reply).
    PhoneNotResponding,
    /// Phone's BNEP layer actively rejected the connection setup (EBADE errno 52).
    /// Corrupted protocol-level security context — full reset required.
    BnepRejected,
    /// Other / unknown error.
    Other(String),
}

impl std::fmt::Display for PanConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TetherNotEnabled => write!(f, "BT tethering not enabled on phone"),
            Self::PhoneUnpaired => write!(f, "Phone unpaired Pi — remove and re-pair"),
            Self::PhoneOutOfRange => write!(f, "Phone out of range or BT off"),
            Self::TransientRadioError => write!(f, "Transient radio error (retrying)"),
            Self::PhoneBusy => write!(f, "Connection busy — retry shortly"),
            Self::PhoneNotResponding => write!(f, "Phone not responding (timeout)"),
            Self::BnepRejected => write!(f, "Phone BNEP rejected (protocol mismatch — full reset required)"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl PanConnectError {
    /// User-facing actionable hint suitable for rendering in the web UI.
    /// Unlike `Display`, these are full sentences that tell the user
    /// exactly what to do on their phone or the Pi.
    pub fn hint(&self) -> String {
        match self {
            Self::TetherNotEnabled => {
                "Bluetooth tethering isn't enabled on the phone. Turn it on under Portable Hotspot → Bluetooth tethering, and make sure mobile data is ON."
                    .into()
            }
            Self::PhoneUnpaired => {
                "The phone forgot the pairing. The Pi is removing the stale bond — pair again from the phone's Bluetooth settings."
                    .into()
            }
            Self::PhoneOutOfRange => {
                "Phone is out of range or Bluetooth is off. Unlock the phone and bring it closer."
                    .into()
            }
            Self::TransientRadioError => String::new(),
            Self::PhoneBusy => {
                "Phone is busy with another Bluetooth operation. The Pi will retry automatically in a few seconds."
                    .into()
            }
            Self::PhoneNotResponding => {
                "Phone isn't responding to connection requests. Try: forget the Pi on the phone → toggle Bluetooth tethering off and back on → pair fresh."
                    .into()
            }
            Self::BnepRejected => {
                "Phone rejected the BNEP connection (protocol mismatch). Full reset required: (1) Forget the Pi on the phone's Bluetooth settings, (2) Toggle BT tethering OFF then ON on the phone, (3) Toggle Bluetooth OFF/ON on the phone, (4) Use 'Reset pairings' on the Pi dashboard, (5) Pair fresh."
                    .into()
            }
            Self::Other(s) => format!("Connect failed: {s}"),
        }
    }
}

/// Classify a D-Bus PAN connect error string into an actionable error type.
///
/// The `PhoneUnpaired` arm is the one that triggers `remove_device` in
/// `connect()` so a stale bond gets cleaned up. ONLY errors that
/// *definitively* indicate the phone rejected the bond should land here:
/// "authentication failed/rejected" is the SSP explicit reject from the
/// phone's security manager — the bond is dead, period.
///
/// Everything else is ambiguous. "Input/output error" (EIO) fires for
/// transient reasons too: phone BT tethering disabled, phone screen locked
/// and stack suspended, momentary range loss, BNEP socket teardown during
/// a radio contention event. Treating these as "unpaired" nukes the bond
/// and forces the user to re-pair from scratch — the exact behavior
/// that burned us on the field walk (2026-04-12).
///
/// Rule: when in doubt, classify as a transient error that retries.
/// Only nuke the bond on an unambiguous SSP authentication reject.
pub fn classify_pan_error(err: &str) -> PanConnectError {
    let lower = err.to_lowercase();
    if lower.contains("create-socket")
        || lower.contains("profile-unavailable")
        || lower.contains("does not exist")
    {
        PanConnectError::TetherNotEnabled
    } else if lower.contains("refused") {
        // "Connection refused" usually means BT tethering isn't enabled
        // on the phone, not that the bond is dead.
        PanConnectError::TetherNotEnabled
    } else if lower.contains("authentication")
        || lower.contains("rejected")
    {
        // Explicit SSP reject — bond is definitely dead.
        PanConnectError::PhoneUnpaired
    } else if lower.contains("page-timeout")
        || lower.contains("host is down")
        || lower.contains("no route")
    {
        // Genuine range/availability issue — show user hint.
        PanConnectError::PhoneOutOfRange
    } else if lower.contains("input/output error")
        || lower.contains("i/o error")
        || lower.contains("connection reset")
        || lower.contains("reset by peer")
    {
        // Transient radio/stack errors — retry silently, no user hint.
        // These fire constantly during normal reconnect churn and are
        // not actionable by the user.
        PanConnectError::TransientRadioError
    } else if lower.contains("busy") || lower.contains("inprogress") || lower.contains("in progress") {
        PanConnectError::PhoneBusy
    } else if lower.contains("noreply") || lower.contains("timed out") || lower.contains("timeout") {
        PanConnectError::PhoneNotResponding
    } else if lower.contains("invalid exchange") {
        // EBADE (errno 52): phone's BNEP layer sent non-success
        // BNEP_SETUP_CONNECTION_RESPONSE. The bond exists on both sides but
        // the BNEP security context is corrupted — retrying won't help.
        // Full clean reset (forget both sides, BT toggle, re-pair) required.
        PanConnectError::BnepRejected
    } else {
        PanConnectError::Other(err.to_string())
    }
}

/// A BlueZ device discovered or paired via D-Bus.
#[derive(Debug, Clone)]
pub struct BlueZDevice {
    /// D-Bus object path, e.g. "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF".
    pub path: String,
    /// MAC address, e.g. "AA:BB:CC:DD:EE:FF".
    pub mac: String,
    /// Human-readable name / alias.
    pub name: String,
    /// Whether the device is paired.
    pub paired: bool,
    /// Whether the device is trusted.
    pub trusted: bool,
    /// Whether the device is currently connected.
    pub connected: bool,
}

/// Events emitted during the pairing / agent flow.
#[derive(Debug, Clone)]
pub enum PairingEvent {
    /// BlueZ asks us to confirm a passkey displayed on both devices.
    ConfirmPasskey { device: String, passkey: u32 },
    /// BlueZ asks us to display a passkey the remote device should enter.
    DisplayPasskey { device: String, passkey: u32 },
    /// Pairing request from a remote device (no passkey).
    RequestConfirmation { device: String },
    /// Pairing completed (success or failure).
    PairingComplete { device: String, success: bool },
    /// A Device1 object transitioned to `Paired=true`. Emitted by the
    /// PropertiesChanged watcher (not by Agent1). Main loop should react by
    /// setting `Trusted=true` on this device so future operations that read
    /// the Trusted flag treat it as a first-class bond.
    DeviceNewlyPaired { device: String },
    /// A Device1 object transitioned to `Connected=true`. The phone
    /// re-established the BT ACL link (e.g. after toggling BT off/on).
    /// The daemon should attempt Network1.Connect to bring up PAN/bnep0,
    /// since ACL reconnect doesn't automatically re-establish PAN.
    DeviceReconnected { device: String },
}

// ─── Linux implementation ────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod inner {
    use super::*;
    use dbus::arg::{RefArg, Variant}; // Variant used in prop_bool/prop_str for reading property maps
    use dbus::blocking::Connection;
    use dbus::channel::MatchingReceiver;
    use dbus_crossroads::Crossroads;
    use log::{info, warn};
    use std::collections::HashMap;
    use std::sync::mpsc::Sender;
    use std::time::Duration;

    /// The D-Bus object path we register our agent at.
    const AGENT_PATH: &str = "/org/bluez/agent/oxigotchi";
    /// Default adapter path.
    const ADAPTER_PATH: &str = "/org/bluez/hci0";

    /// Extract a boolean from a D-Bus property map.
    ///
    /// The `dbus` crate maps D-Bus booleans to `i64` (1 = true, 0 = false).
    /// We also try a string fallback for edge cases.
    fn prop_bool(
        props: &HashMap<String, Variant<Box<dyn RefArg>>>,
        key: &str,
    ) -> Option<bool> {
        props.get(key).and_then(|v| {
            if let Some(n) = v.0.as_i64() {
                return Some(n != 0);
            }
            let s = format!("{:?}", v.0);
            match s.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        })
    }

    /// Extract a string from a D-Bus property map.
    fn prop_str(
        props: &HashMap<String, Variant<Box<dyn RefArg>>>,
        key: &str,
    ) -> Option<String> {
        props.get(key).and_then(|v| v.0.as_str().map(|s| s.to_string()))
    }

    /// Wraps all BlueZ D-Bus interactions.
    pub struct DbusBluez {
        conn: Connection,
        /// Active PAN connection, if any.
        pan: Option<PanConnection>,
        /// Device path of the PAN-connected device.
        pan_device: Option<String>,
    }

    impl DbusBluez {
        /// Connect to the system D-Bus.
        pub fn new() -> Result<Self, String> {
            let conn = Connection::new_system().map_err(|e| format!("D-Bus connect: {e}"))?;
            info!("[dbus] Connected to system bus");
            Ok(Self {
                conn,
                pan: None,
                pan_device: None,
            })
        }

        /// List all paired devices via ObjectManager. Caller is responsible
        /// for any further filtering (e.g. NAP capability for tether selection).
        ///
        /// NOTE: we intentionally do NOT filter on `Trusted` here. Trusted is
        /// a BlueZ authorization flag that gates profile auto-connect when an
        /// agent is absent; we always register Agent1 and auto-accept
        /// AuthorizeService, so Trusted is irrelevant to whether a connect
        /// can succeed. The old filter silently dropped phone-initiated
        /// pairs (Paired=true, Trusted=false until our watcher promotes them)
        /// and required users to manually `bluetoothctl trust MAC`.
        pub fn list_paired_devices(&self) -> Result<Vec<BlueZDevice>, String> {
            let proxy = self.conn.with_proxy("org.bluez", "/", Duration::from_secs(5));
            use dbus::blocking::stdintf::org_freedesktop_dbus::ObjectManager;
            let objects = proxy
                .get_managed_objects()
                .map_err(|e| format!("GetManagedObjects: {e}"))?;

            let mut devices = Vec::new();
            for (path, ifaces) in &objects {
                if let Some(props) = ifaces.get("org.bluez.Device1") {
                    let paired = prop_bool(props, "Paired").unwrap_or(false);
                    if !paired {
                        continue;
                    }
                    let trusted = prop_bool(props, "Trusted").unwrap_or(false);
                    let mac = prop_str(props, "Address").unwrap_or_default();
                    let name = prop_str(props, "Alias").unwrap_or_else(|| mac.clone());
                    let connected = prop_bool(props, "Connected").unwrap_or(false);
                    devices.push(BlueZDevice {
                        path: path.to_string(),
                        mac,
                        name,
                        paired,
                        trusted,
                        connected,
                    });
                }
            }
            info!("[dbus] Found {} paired devices", devices.len());
            Ok(devices)
        }

        /// NAP profile UUID used by ConnectProfile fallback.
        const NAP_UUID: &'static str = "00001116-0000-1000-8000-00805f9b34fb";

        /// Connect to a device's PAN.
        ///
        /// Primary path: `Network1.Connect("nap")` — the standard BlueZ API
        /// for PAN tethering. Returns the BNEP interface name directly.
        ///
        /// Fallback: if Network1 fails with `profile-unavailable` (common on
        /// iOS which doesn't advertise NAP in SDP), try
        /// `Device1.ConnectProfile(NAP_UUID)` which uses a lower-level path
        /// that can succeed when Network1 refuses. The interface name must
        /// then be discovered from sysfs since ConnectProfile doesn't return
        /// it. This is the approach the wsvdmeer Python plugin uses for iOS.
        pub fn connect_pan(&mut self, device_path: &str) -> Result<PanConnection, PanConnectError> {
            let proxy = self.conn.with_proxy(
                "org.bluez",
                device_path,
                Duration::from_secs(30),
            );

            // Best-effort cleanup: BlueZ's PAN plugin can retain stale
            // per-device state after a failed session. Clearing any existing
            // Network1 session before a new Connect helps MIUI devices that
            // otherwise get stuck returning EBADE on every retry.
            let cleanup: Result<(), _> =
                proxy.method_call("org.bluez.Network1", "Disconnect", ());
            if let Err(e) = cleanup {
                info!("[dbus] Network1.Disconnect pre-cleanup ignored: {e}");
            }

            // Try Network1.Connect first (standard path, returns interface name)
            match proxy.method_call::<(String,), _, _, _>(
                "org.bluez.Network1",
                "Connect",
                ("nap",),
            ) {
                Ok((iface_name,)) => {
                    info!("[dbus] PAN connected on {iface_name} via {device_path} (Network1)");
                    Ok(self.commit_pan(device_path, iface_name))
                }
                Err(e) => {
                    let err_str = format!("{e}");
                    info!("[dbus] Network1.Connect error: {err_str}");

                    if err_str.contains("profile-unavailable")
                        || err_str.contains("ProfileUnavailable")
                        || err_str.contains("does not exist")
                    {
                        info!("[dbus] Trying ConnectProfile(NAP_UUID) fallback (iOS path)");
                        return self.connect_pan_via_profile(device_path);
                    }

                    let classified = classify_pan_error(&err_str);
                    if classified == PanConnectError::BnepRejected {
                        info!("[dbus] Trying ConnectProfile(NAP_UUID) fallback after BNEP reject");
                        if let Ok(pan) = self.connect_pan_via_profile(device_path) {
                            return Ok(pan);
                        }
                    }

                    Err(classified)
                }
            }
        }

        /// Fallback PAN connect via Device1.ConnectProfile(NAP_UUID).
        /// Used when Network1.Connect fails (e.g. iOS devices).
        fn connect_pan_via_profile(
            &mut self,
            device_path: &str,
        ) -> Result<PanConnection, PanConnectError> {
            let proxy = self.conn.with_proxy(
                "org.bluez",
                device_path,
                Duration::from_secs(30),
            );

            proxy
                .method_call::<(), _, _, _>(
                    "org.bluez.Device1",
                    "ConnectProfile",
                    (Self::NAP_UUID,),
                )
                .map_err(|e| {
                    let err_str = format!("{e}");
                    info!("[dbus] ConnectProfile(NAP) error: {err_str}");
                    classify_pan_error(&err_str)
                })?;

            info!("[dbus] ConnectProfile(NAP) succeeded, discovering PAN interface...");

            // ConnectProfile doesn't return the interface name — discover it
            // from sysfs. Check immediately first, then poll with increasing
            // delays. Interface typically appears within ~100ms of the call.
            for i in 0..10 {
                if let Some(iface) = crate::bluetooth::BtTether::find_existing_bnep() {
                    info!("[dbus] PAN interface discovered: {iface} via ConnectProfile");
                    return Ok(self.commit_pan(device_path, iface));
                }
                std::thread::sleep(Duration::from_millis(if i == 0 { 100 } else { 500 }));
            }

            warn!("[dbus] ConnectProfile succeeded but no PAN interface appeared");
            Err(PanConnectError::Other(
                "ConnectProfile OK but no bnep interface found".into(),
            ))
        }

        /// Store PAN connection state after a successful connect.
        fn commit_pan(&mut self, device_path: &str, iface: String) -> PanConnection {
            let pc = PanConnection { interface: iface };
            self.pan = Some(pc.clone());
            self.pan_device = Some(device_path.to_string());
            pc
        }

        /// Disconnect the active PAN connection via Network1.Disconnect.
        pub fn disconnect_pan(&mut self) -> Result<(), String> {
            if let Some(dev) = self.pan_device.take() {
                let proxy =
                    self.conn
                        .with_proxy("org.bluez", &dev, Duration::from_secs(10));
                let _: () = proxy
                    .method_call("org.bluez.Network1", "Disconnect", ())
                    .map_err(|e| format!("PAN Disconnect: {e}"))?;
                info!("[dbus] PAN disconnected from {dev}");
            }
            self.pan = None;
            Ok(())
        }

        /// Read the UUIDs property from a Device1 to check for NAP support.
        pub fn get_device_uuids(&self, device_path: &str) -> Result<Vec<String>, String> {
            let proxy = self.conn.with_proxy(
                "org.bluez",
                device_path,
                Duration::from_secs(5),
            );
            use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
            let uuids: Vec<String> = proxy
                .get("org.bluez.Device1", "UUIDs")
                .map_err(|e| format!("Get UUIDs: {e}"))?;
            Ok(uuids)
        }

        /// Check if NAP UUID is present in a device's service list.
        pub fn has_nap_uuid(&self, device_path: &str) -> bool {
            match self.get_device_uuids(device_path) {
                Ok(uuids) => uuids.iter().any(|u| u.contains("1116")),
                Err(_) => false,
            }
        }

        /// Return the PAN interface name if connected (e.g. "bnep0").
        pub fn pan_interface(&self) -> Option<&str> {
            self.pan.as_ref().map(|p| p.interface.as_str())
        }

        /// Pair with a device. Unused by the daemon's normal flow (phone-initiated
        /// pairing handled via Agent1); retained for tests and as a fallback.
        pub fn pair_device(&self, device_path: &str) -> Result<(), String> {
            let proxy = self.conn.with_proxy(
                "org.bluez",
                device_path,
                Duration::from_secs(30),
            );
            let _: () = proxy
                .method_call("org.bluez.Device1", "Pair", ())
                .map_err(|e| format!("Pair: {e}"))?;
            info!("[dbus] Paired with {device_path}");
            Ok(())
        }

        /// Set Trusted=true on a device.
        pub fn trust_device(&self, device_path: &str) -> Result<(), String> {
            let proxy = self.conn.with_proxy(
                "org.bluez",
                device_path,
                Duration::from_secs(5),
            );
            use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
            proxy
                .set("org.bluez.Device1", "Trusted", true)
                .map_err(|e| format!("Trust: {e}"))?;
            info!("[dbus] Trusted {device_path}");
            Ok(())
        }

        /// Read a Device1 boolean property via D-Bus `Properties.Get`.
        /// Returns Err if the device object does not exist or the property is absent.
        pub fn get_device_property_bool(
            &self,
            device_path: &str,
            property: &str,
        ) -> Result<bool, String> {
            let proxy = self.conn.with_proxy(
                "org.bluez",
                device_path,
                Duration::from_secs(5),
            );
            use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
            let value: bool = proxy
                .get("org.bluez.Device1", property)
                .map_err(|e| format!("GetProperty({property}): {e}"))?;
            Ok(value)
        }

        /// Remove a device from BlueZ via Adapter1.RemoveDevice.
        pub fn remove_device(&self, device_path: &str) -> Result<(), String> {
            // Derive adapter path: /org/bluez/hci0/dev_XX -> /org/bluez/hci0
            let adapter = device_path
                .rfind('/')
                .map(|i| &device_path[..i])
                .unwrap_or(ADAPTER_PATH);
            let proxy =
                self.conn
                    .with_proxy("org.bluez", adapter, Duration::from_secs(10));
            let dev_path = dbus::Path::from(device_path);
            let _: () = proxy
                .method_call("org.bluez.Adapter1", "RemoveDevice", (dev_path,))
                .map_err(|e| format!("RemoveDevice: {e}"))?;
            info!("[dbus] Removed {device_path}");
            Ok(())
        }

        /// Disconnect the ACL link to a device via Device1.Disconnect.
        ///
        /// Called after BnepRejected (EBADE) to flush the phone's stale BNEP
        /// state. The phone will auto-reconnect ACL for trusted devices, and
        /// the next Network1.Connect will start with a fresh BNEP context.
        /// Errors are non-fatal — device may already be disconnected.
        pub fn disconnect_device(&self, device_path: &str) {
            let proxy =
                self.conn
                    .with_proxy("org.bluez", device_path, Duration::from_secs(5));
            let result: Result<(), _> =
                proxy.method_call("org.bluez.Device1", "Disconnect", ());
            match result {
                Ok(()) => info!("[dbus] Device1.Disconnect OK for {device_path}"),
                Err(e) => info!("[dbus] Device1.Disconnect ignored: {e}"),
            }
        }

        /// Whether a PAN connection is active.
        pub fn is_connected(&self) -> bool {
            self.pan.is_some()
        }

        /// Ping the bus to verify the D-Bus connection and bluez are alive.
        pub fn is_bus_alive(&self) -> bool {
            let proxy = self.conn.with_proxy(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                Duration::from_secs(2),
            );
            let result: Result<(String,), _> =
                proxy.method_call("org.freedesktop.DBus", "GetNameOwner", ("org.bluez",));
            result.is_ok()
        }

        /// Register our agent with BlueZ's AgentManager1.
        pub fn register_agent(&self) -> Result<(), String> {
            let proxy = self.conn.with_proxy(
                "org.bluez",
                "/org/bluez",
                Duration::from_secs(5),
            );
            let agent = dbus::Path::from(AGENT_PATH);
            let _: () = proxy
                .method_call(
                    "org.bluez.AgentManager1",
                    "RegisterAgent",
                    (agent.clone(), "DisplayYesNo"),
                )
                .map_err(|e| format!("RegisterAgent: {e}"))?;
            let _: () = proxy
                .method_call(
                    "org.bluez.AgentManager1",
                    "RequestDefaultAgent",
                    (agent,),
                )
                .map_err(|e| format!("RequestDefaultAgent: {e}"))?;
            info!("[dbus] Agent registered at {AGENT_PATH}");
            Ok(())
        }

        /// Start BT scanning: set discoverable + pairable, then StartDiscovery.
        pub fn start_scan(&self) -> Result<(), String> {
            let proxy = self.conn.with_proxy(
                "org.bluez",
                ADAPTER_PATH,
                Duration::from_secs(5),
            );
            use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
            let _ = proxy.set("org.bluez.Adapter1", "Discoverable", true);
            let _ = proxy.set("org.bluez.Adapter1", "Pairable", true);
            let _: () = proxy
                .method_call("org.bluez.Adapter1", "StartDiscovery", ())
                .map_err(|e| format!("StartDiscovery: {e}"))?;
            info!("[dbus] Scan started on {ADAPTER_PATH}");
            Ok(())
        }

        /// Stop BT scanning: StopDiscovery, set discoverable + pairable = false.
        pub fn stop_scan(&self) -> Result<(), String> {
            let proxy = self.conn.with_proxy(
                "org.bluez",
                ADAPTER_PATH,
                Duration::from_secs(5),
            );
            let _: () = proxy
                .method_call("org.bluez.Adapter1", "StopDiscovery", ())
                .map_err(|e| format!("StopDiscovery: {e}"))?;
            use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
            let _ = proxy.set("org.bluez.Adapter1", "Discoverable", false);
            let _ = proxy.set("org.bluez.Adapter1", "Pairable", false);
            info!("[dbus] Scan stopped on {ADAPTER_PATH}");
            Ok(())
        }


        /// List ALL devices visible to BlueZ (paired or discovered).
        pub fn list_all_devices(&self) -> Result<Vec<BlueZDevice>, String> {
            let proxy = self.conn.with_proxy("org.bluez", "/", Duration::from_secs(5));
            use dbus::blocking::stdintf::org_freedesktop_dbus::ObjectManager;
            let objects = proxy
                .get_managed_objects()
                .map_err(|e| format!("GetManagedObjects: {e}"))?;

            let mut devices = Vec::new();
            for (path, ifaces) in &objects {
                if let Some(props) = ifaces.get("org.bluez.Device1") {
                    let mac = prop_str(props, "Address").unwrap_or_default();
                    let name = prop_str(props, "Alias").unwrap_or_else(|| mac.clone());
                    let paired = prop_bool(props, "Paired").unwrap_or(false);
                    let trusted = prop_bool(props, "Trusted").unwrap_or(false);
                    let connected = prop_bool(props, "Connected").unwrap_or(false);
                    devices.push(BlueZDevice {
                        path: path.to_string(),
                        mac,
                        name,
                        paired,
                        trusted,
                        connected,
                    });
                }
            }
            info!("[dbus] Found {} total devices", devices.len());
            Ok(devices)
        }

        /// Set up the Agent1 crossroads handler for pairing events.
        ///
        /// BlueZ calls our agent when pairing requires user interaction (passkey
        /// confirmation, authorization, etc.).  We auto-accept all requests
        /// (headless mode) and forward the events through `event_tx` so the web
        /// UI can display the passkey.
        pub fn setup_agent_handler(
            &self,
            event_tx: Sender<PairingEvent>,
        ) -> Result<(), String> {
            let mut cr = Crossroads::new();

            // Synchronous mode (no async runtime needed — blocking D-Bus connection)
            cr.set_async_support(None);

            let token = cr.register("org.bluez.Agent1", |b| {
                b.method(
                    "Release",
                    (),
                    (),
                    |_, tx: &mut Sender<PairingEvent>, (): ()| {
                        info!("[agent] Released — pairing flow ended");
                        let _ = tx.send(PairingEvent::PairingComplete {
                            device: String::new(),
                            success: true,
                        });
                        Ok(())
                    },
                );
                b.method(
                    "Cancel",
                    (),
                    (),
                    |_, tx: &mut Sender<PairingEvent>, (): ()| {
                        info!("[agent] Cancelled — pairing aborted");
                        let _ = tx.send(PairingEvent::PairingComplete {
                            device: String::new(),
                            success: false,
                        });
                        Ok(())
                    },
                );
                b.method(
                    "RequestConfirmation",
                    ("device", "passkey"),
                    (),
                    |_, tx: &mut Sender<PairingEvent>, (device, passkey): (dbus::Path<'static>, u32)| {
                        info!("[agent] RequestConfirmation: passkey={passkey}");
                        let _ = tx.send(PairingEvent::ConfirmPasskey {
                            device: device.to_string(),
                            passkey,
                        });
                        // Auto-accept in headless mode
                        Ok(())
                    },
                );
                b.method(
                    "DisplayPasskey",
                    ("device", "passkey", "entered"),
                    (),
                    |_, tx: &mut Sender<PairingEvent>, (device, passkey, _entered): (dbus::Path<'static>, u32, u16)| {
                        info!("[agent] DisplayPasskey: passkey={passkey}");
                        let _ = tx.send(PairingEvent::DisplayPasskey {
                            device: device.to_string(),
                            passkey,
                        });
                        Ok(())
                    },
                );
                b.method(
                    "RequestAuthorization",
                    ("device",),
                    (),
                    |_, _tx: &mut Sender<PairingEvent>, (_device,): (dbus::Path<'static>,)| {
                        info!("[agent] RequestAuthorization: auto-accepting");
                        Ok(())
                    },
                );
                b.method(
                    "AuthorizeService",
                    ("device", "uuid"),
                    (),
                    |_, _tx: &mut Sender<PairingEvent>, (_device, _uuid): (dbus::Path<'static>, String)| {
                        info!("[agent] AuthorizeService: auto-accepting");
                        Ok(())
                    },
                );
            });

            cr.insert(AGENT_PATH, &[token], event_tx);

            self.conn.start_receive(
                dbus::message::MatchRule::new_method_call(),
                Box::new(move |msg, conn| {
                    let _ = cr.handle_message(msg, conn);
                    true
                }),
            );

            info!("[dbus] Agent1 crossroads handler registered at {AGENT_PATH}");
            Ok(())
        }

        /// Process pending D-Bus messages (dispatches to crossroads handlers).
        pub fn process_messages(&self, timeout: Duration) {
            let _ = self.conn.process(timeout);
        }

        /// Register a watcher for `org.freedesktop.DBus.Properties.PropertiesChanged`
        /// signals on `org.bluez.Device1` objects. When we see a device transition
        /// to `Paired=true`, we send `PairingEvent::DeviceNewlyPaired` so the main
        /// loop can auto-set `Trusted=true` on it.
        ///
        /// This is the mechanism that makes phone-initiated pairing produce a
        /// first-class bond without any user intervention. Without this, the
        /// Agent1-only flow leaves `Trusted=false` on the device and every
        /// subsequent code path that cares about Trusted silently skips it.
        pub fn register_paired_watcher(
            &self,
            tx: Sender<PairingEvent>,
        ) -> Result<(), String> {
            use dbus::arg::RefArg;
            use dbus::blocking::stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged;
            use dbus::message::MatchRule;

            // Bus-level filter: only signals from org.bluez reach us. Without
            // this we'd wake up on every PropertiesChanged on the system bus
            // (systemd, NetworkManager, etc.) and drop them in the callback,
            // which is wasteful on a Pi Zero.
            let rule = MatchRule::new_signal(
                "org.freedesktop.DBus.Properties",
                "PropertiesChanged",
            )
            .with_sender("org.bluez");

            self.conn
                .add_match(rule, move |pc: PropertiesPropertiesChanged, _, msg| {
                    // We only care about Device1 objects on org.bluez.
                    if pc.interface_name != "org.bluez.Device1" {
                        return true;
                    }
                    // Check if `Paired` changed in this update. Log EVERY
                    // transition (true OR false) so field diagnostics can
                    // confirm the watcher is firing — the most common silent
                    // failure mode is the watcher being registered on a dead
                    // connection and never seeing the signal at all.
                    let path = msg
                        .path()
                        .map(|p| p.to_string())
                        .unwrap_or_default();

                    if let Some(variant) = pc.changed_properties.get("Paired") {
                        let paired = variant.0.as_i64().map(|v| v != 0).unwrap_or(false);
                        info!(
                            "[dbus] PropertiesChanged: Paired={} on {}",
                            paired,
                            if path.is_empty() { "<unknown>" } else { &path }
                        );
                        if paired && !path.is_empty() {
                            let _ = tx.send(PairingEvent::DeviceNewlyPaired {
                                device: path.clone(),
                            });
                        }
                    }

                    // Detect ACL reconnect: phone toggled BT off/on and the
                    // link came back. BlueZ auto-connects trusted devices at
                    // the ACL level but does NOT re-establish PAN. Without
                    // this signal the daemon sits in Disconnected waiting for
                    // should_connect() while the phone shows "connected".
                    if let Some(variant) = pc.changed_properties.get("Connected") {
                        let connected = variant.0.as_i64().map(|v| v != 0).unwrap_or(false);
                        info!(
                            "[dbus] PropertiesChanged: Connected={} on {}",
                            connected,
                            if path.is_empty() { "<unknown>" } else { &path }
                        );
                        if connected && !path.is_empty() {
                            let _ = tx.send(PairingEvent::DeviceReconnected {
                                device: path,
                            });
                        }
                    }

                    true
                })
                .map_err(|e| format!("add_match PropertiesChanged: {e}"))?;

            info!("[dbus] PropertiesChanged watcher registered for Device1.Paired");
            Ok(())
        }
    }
}

// ─── Non-Linux stubs ─────────────────────────────────────────────────────────

#[cfg(not(target_os = "linux"))]
mod inner {
    use super::*;
    use std::sync::mpsc::Sender;

    /// Stub BlueZ D-Bus wrapper for non-Linux platforms.
    pub struct DbusBluez;

    impl DbusBluez {
        pub fn new() -> Result<Self, String> {
            Ok(Self)
        }

        pub fn list_paired_devices(&self) -> Result<Vec<BlueZDevice>, String> {
            Ok(vec![])
        }

        pub fn connect_pan(&mut self, _device_path: &str) -> Result<PanConnection, PanConnectError> {
            Err(PanConnectError::Other("not supported on this platform".to_string()))
        }

        pub fn disconnect_pan(&mut self) -> Result<(), String> {
            Ok(())
        }

        pub fn get_device_uuids(&self, _device_path: &str) -> Result<Vec<String>, String> {
            Ok(vec![])
        }

        pub fn has_nap_uuid(&self, _device_path: &str) -> bool {
            false
        }

        pub fn pan_interface(&self) -> Option<&str> {
            None
        }

        pub fn pair_device(&self, _device_path: &str) -> Result<(), String> {
            Err("not supported on this platform".to_string())
        }

        pub fn trust_device(&self, _device_path: &str) -> Result<(), String> {
            Err("not supported on this platform".to_string())
        }

        pub fn get_device_property_bool(
            &self,
            _device_path: &str,
            _property: &str,
        ) -> Result<bool, String> {
            Err("not supported on this platform".to_string())
        }

        pub fn remove_device(&self, _device_path: &str) -> Result<(), String> {
            Err("not supported on this platform".to_string())
        }

        pub fn is_connected(&self) -> bool {
            false
        }

        pub fn disconnect_device(&self, _device_path: &str) {}

        pub fn is_bus_alive(&self) -> bool {
            true
        }

        pub fn register_agent(&self) -> Result<(), String> {
            Ok(())
        }

        pub fn start_scan(&self) -> Result<(), String> {
            Ok(())
        }

        pub fn stop_scan(&self) -> Result<(), String> {
            Ok(())
        }

        pub fn list_all_devices(&self) -> Result<Vec<BlueZDevice>, String> {
            Ok(vec![])
        }

        pub fn register_paired_watcher(
            &self,
            _tx: Sender<PairingEvent>,
        ) -> Result<(), String> {
            Ok(())
        }

        pub fn setup_agent_handler(
            &self,
            _event_tx: Sender<PairingEvent>,
        ) -> Result<(), String> {
            Ok(())
        }

        pub fn process_messages(&self, _timeout: std::time::Duration) {}
    }
}

pub use inner::DbusBluez;

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pan_connection_struct() {
        let pc = PanConnection {
            interface: "bnep0".to_string(),
        };
        assert_eq!(pc.interface, "bnep0");
    }

    #[test]
    fn test_bluez_device_struct() {
        let dev = BlueZDevice {
            path: "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF".to_string(),
            mac: "AA:BB:CC:DD:EE:FF".to_string(),
            name: "iPhone".to_string(),
            paired: true,
            trusted: true,
            connected: false,
        };
        assert!(dev.paired);
        assert!(dev.trusted);
        assert!(!dev.connected);
    }

    #[test]
    fn test_pairing_event_variants() {
        let ev = PairingEvent::ConfirmPasskey {
            device: "iPhone".into(),
            passkey: 123456,
        };
        match ev {
            PairingEvent::ConfirmPasskey { passkey, .. } => assert_eq!(passkey, 123456),
            _ => panic!("wrong variant"),
        }
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn test_stub_creates_ok() {
        let mut dbus = DbusBluez::new().unwrap();
        assert!(dbus.list_paired_devices().unwrap().is_empty());
        assert!(dbus.pan_interface().is_none());
        assert!(!dbus.is_connected());
        assert!(dbus.connect_pan("/org/bluez/hci0/dev_AA").is_err());
        assert!(dbus.get_device_uuids("/org/bluez/hci0/dev_AA").unwrap().is_empty());
        assert!(!dbus.has_nap_uuid("/org/bluez/hci0/dev_AA"));
    }

    #[test]
    fn test_classify_pan_error() {
        assert_eq!(
            classify_pan_error("br-connection-create-socket"),
            PanConnectError::TetherNotEnabled
        );
        assert_eq!(
            classify_pan_error("br-connection-profile-unavailable"),
            PanConnectError::TetherNotEnabled
        );
        assert_eq!(
            classify_pan_error("Authentication Rejected"),
            PanConnectError::PhoneUnpaired
        );
        assert_eq!(
            classify_pan_error("Connection refused"),
            PanConnectError::TetherNotEnabled
        );
        assert_eq!(
            classify_pan_error("br-connection-page-timeout"),
            PanConnectError::PhoneOutOfRange
        );
        assert_eq!(
            classify_pan_error("Host is down"),
            PanConnectError::PhoneOutOfRange
        );
        // IO errors are transient, NOT bond-removal triggers
        assert_eq!(
            classify_pan_error("Input/output error"),
            PanConnectError::TransientRadioError
        );
        assert_eq!(
            classify_pan_error("Connection reset by peer"),
            PanConnectError::TransientRadioError
        );
        assert_eq!(
            classify_pan_error("br-connection-busy"),
            PanConnectError::PhoneBusy
        );
        assert_eq!(
            classify_pan_error("org.freedesktop.DBus.Error.NoReply"),
            PanConnectError::PhoneNotResponding
        );
        assert_eq!(
            classify_pan_error("connect error Invalid exchange (52)"),
            PanConnectError::BnepRejected
        );
        matches!(
            classify_pan_error("something unknown"),
            PanConnectError::Other(_)
        );
    }

    #[test]
    fn test_pan_connect_error_display() {
        assert_eq!(
            PanConnectError::TetherNotEnabled.to_string(),
            "BT tethering not enabled on phone"
        );
        assert_eq!(
            PanConnectError::PhoneUnpaired.to_string(),
            "Phone unpaired Pi — remove and re-pair"
        );
    }
}
