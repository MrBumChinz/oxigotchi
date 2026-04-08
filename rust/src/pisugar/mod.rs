//! PiSugar 3 battery monitoring via I2C.
//!
//! Reads battery level, charging status, and handles button presses.
//! On non-Pi platforms, provides a mock implementation.

// ---------------------------------------------------------------------------
// I2C register constants for PiSugar 3
// ---------------------------------------------------------------------------

/// PiSugar 3 battery I2C address.
pub const I2C_ADDR_BATTERY: u16 = 0x57;
/// PiSugar 3 RTC I2C address.
pub const I2C_ADDR_RTC: u16 = 0x68;
/// Register: battery level percentage (0-100).
pub const REG_BATTERY_LEVEL: u8 = 0x2A;
/// Register: battery voltage high byte (mV, big-endian).
pub const REG_VOLTAGE_HIGH: u8 = 0x22;
/// Register: battery voltage low byte (mV, big-endian).
pub const REG_VOLTAGE_LOW: u8 = 0x23;
/// Register: CTR1 — charging status (lower bits RO) + power control (upper bits R/W).
/// Also used as REG_CHARGE_STATUS (lower 3 bits).
pub const REG_CTR1: u8 = 0x02;
/// Alias: charge status is in the lower bits of CTR1.
pub const REG_CHARGE_STATUS: u8 = 0x02;
/// Register: CTR2 — soft shutdown enable/state, auto-hibernate.
pub const REG_CTR2: u8 = 0x03;
/// Register: board temperature (value - 40 = °C).
pub const REG_TEMPERATURE: u8 = 0x04;
/// Register: TAP — button tap type (2-bit: 0=none, 1=single, 2=double, 3=long).
/// Write 0 to clear after reading.
pub const REG_TAP: u8 = 0x08;
/// Register: delay shutdown countdown (value × 2 seconds).
pub const REG_DELAY_SHUTDOWN: u8 = 0x09;
/// Register: write protection gate (0x29 = unlock, anything else = lock).
pub const REG_WRITE_PROTECT: u8 = 0x0B;
/// Register: auto-shutdown battery level threshold (undocumented — verify on Pi).
pub const REG_AUTO_SHUTDOWN: u8 = 0x19;

// Charge status bit flags (lower bits of CTR1 / register 0x02)
/// Bit 0: power cable connected.
pub const CHARGE_FLAG_POWER_CONNECTED: u8 = 0x01;
/// Bit 1: charging in progress.
pub const CHARGE_FLAG_CHARGING: u8 = 0x02;
/// Bit 2: charge complete / full.
pub const CHARGE_FLAG_FULL: u8 = 0x04;

// CTR1 control bit masks (upper bits of register 0x02)
/// Bit 3: anti-mistouch — prevents accidental power button activation.
pub const CTR1_ANTI_MISTOUCH: u8 = 0x08;
/// Bit 4: auto-restore output on USB power reconnect.
pub const CTR1_AUTO_RESTORE: u8 = 0x10;

// CTR2 bit masks (register 0x03)
/// Bit 3: soft shutdown state (RO) — set by MCU when power button long-pressed.
pub const CTR2_SOFT_SHUTDOWN_STATE: u8 = 0x08;
/// Bit 4: soft shutdown enable.
pub const CTR2_SOFT_SHUTDOWN_ENABLE: u8 = 0x10;
/// Bit 6: auto-hibernate.
pub const CTR2_AUTO_HIBERNATE: u8 = 0x40;

// ---------------------------------------------------------------------------
// Parsing helpers (pure functions, testable on any platform)
// ---------------------------------------------------------------------------

/// Parse battery level from register 0x2A.
/// Clamps to 0-100 range.
pub fn parse_battery_level(raw: u8) -> u8 {
    raw.min(100)
}

/// Parse voltage from two register bytes (0x22 high, 0x23 low) in big-endian mV.
pub fn parse_voltage_mv(high: u8, low: u8) -> u16 {
    u16::from_be_bytes([high, low])
}

/// Parse charge state from register 0x02 bit flags.
pub fn parse_charge_state(flags: u8) -> ChargeState {
    if flags & CHARGE_FLAG_FULL != 0 {
        ChargeState::Full
    } else if flags & CHARGE_FLAG_CHARGING != 0 {
        ChargeState::Charging
    } else if flags & CHARGE_FLAG_POWER_CONNECTED != 0 {
        // Power connected but not charging (e.g. maintenance/done)
        ChargeState::Full
    } else {
        ChargeState::Discharging
    }
}

/// Parse TAP register value (0x08) into a button action.
/// The register uses a 2-bit value: 0=none, 1=single, 2=double, 3=long.
pub fn parse_tap_event(value: u8) -> Option<ButtonAction> {
    match value & 0x03 {
        1 => Some(ButtonAction::SingleTap),
        2 => Some(ButtonAction::DoubleTap),
        3 => Some(ButtonAction::LongPress),
        _ => None,
    }
}

/// Battery charging state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeState {
    Discharging,
    Charging,
    Full,
    Unknown,
}

/// PiSugar button actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    SingleTap,
    DoubleTap,
    LongPress,
}

/// Battery status snapshot.
#[derive(Debug, Clone)]
pub struct BatteryStatus {
    /// Battery level as percentage (0-100).
    pub level: u8,
    /// Charging state.
    pub charge_state: ChargeState,
    /// Battery voltage in millivolts.
    pub voltage_mv: u16,
    /// Whether the battery is below critical threshold.
    pub critical: bool,
    /// Whether the battery is below low threshold.
    pub low: bool,
}

impl Default for BatteryStatus {
    fn default() -> Self {
        Self {
            level: 100,
            charge_state: ChargeState::Unknown,
            voltage_mv: 4200,
            critical: false,
            low: false,
        }
    }
}

/// PiSugar I2C configuration.
#[derive(Debug, Clone)]
pub struct PiSugarConfig {
    /// I2C bus number (usually 1 on Pi).
    pub i2c_bus: u8,
    /// I2C device address for PiSugar 3 battery.
    pub i2c_addr: u16,
    /// Battery level below which to trigger low warning.
    pub low_threshold: u8,
    /// Battery level below which to trigger critical/shutdown.
    pub critical_threshold: u8,
    /// Poll interval in seconds.
    pub poll_interval_secs: u64,
    /// Whether to auto-shutdown on critical battery.
    pub auto_shutdown: bool,
    /// Auto-shutdown level to write to register 0x19 (0 = disabled).
    pub auto_shutdown_level: u8,
}

impl Default for PiSugarConfig {
    fn default() -> Self {
        Self {
            i2c_bus: 1,
            i2c_addr: I2C_ADDR_BATTERY,
            low_threshold: 20,
            critical_threshold: 5,
            poll_interval_secs: 30,
            auto_shutdown: true,
            auto_shutdown_level: 5,
        }
    }
}

// ---------------------------------------------------------------------------
// I2C backend (real on aarch64, stub on other platforms)
// ---------------------------------------------------------------------------

/// Errors from I2C operations.
#[derive(Debug)]
pub enum I2cError {
    /// I2C bus could not be opened.
    BusOpen(String),
    /// I2C device not found at address.
    DeviceNotFound(u16),
    /// Read/write failed.
    IoError(String),
}

impl std::fmt::Display for I2cError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            I2cError::BusOpen(e) => write!(f, "I2C bus open failed: {e}"),
            I2cError::DeviceNotFound(addr) => write!(f, "I2C device not found at 0x{addr:02X}"),
            I2cError::IoError(e) => write!(f, "I2C I/O error: {e}"),
        }
    }
}

/// Read a single register from the PiSugar over I2C.
#[cfg(target_arch = "aarch64")]
fn i2c_read_register(bus: u8, addr: u16, register: u8) -> Result<u8, I2cError> {
    let mut i2c = rppal::i2c::I2c::with_bus(bus).map_err(|e| I2cError::BusOpen(e.to_string()))?;
    i2c.set_slave_address(addr)
        .map_err(|e| I2cError::IoError(e.to_string()))?;
    let mut buf = [0u8; 1];
    i2c.write_read(&[register], &mut buf)
        .map_err(|e| I2cError::IoError(e.to_string()))?;
    Ok(buf[0])
}

#[cfg(not(target_arch = "aarch64"))]
fn i2c_read_register(_bus: u8, _addr: u16, _register: u8) -> Result<u8, I2cError> {
    Err(I2cError::DeviceNotFound(0x57))
}

/// Write a single register to the PiSugar over I2C.
#[cfg(target_arch = "aarch64")]
fn i2c_write_register(bus: u8, addr: u16, register: u8, value: u8) -> Result<(), I2cError> {
    let mut i2c = rppal::i2c::I2c::with_bus(bus).map_err(|e| I2cError::BusOpen(e.to_string()))?;
    i2c.set_slave_address(addr)
        .map_err(|e| I2cError::IoError(e.to_string()))?;
    i2c.write(&[register, value])
        .map_err(|e| I2cError::IoError(e.to_string()))?;
    Ok(())
}

#[cfg(not(target_arch = "aarch64"))]
fn i2c_write_register(_bus: u8, _addr: u16, _register: u8, _value: u8) -> Result<(), I2cError> {
    Err(I2cError::DeviceNotFound(0x57))
}

/// Probe whether an I2C device is present at the given address.
#[cfg(target_arch = "aarch64")]
fn i2c_probe(bus: u8, addr: u16) -> bool {
    let Ok(mut i2c) = rppal::i2c::I2c::with_bus(bus) else {
        return false;
    };
    if i2c.set_slave_address(addr).is_err() {
        return false;
    }
    // Try reading register 0x00 -- if the device ACKs, it is present
    let mut buf = [0u8; 1];
    i2c.write_read(&[0x00], &mut buf).is_ok()
}

#[cfg(not(target_arch = "aarch64"))]
fn i2c_probe(_bus: u8, _addr: u16) -> bool {
    false
}

// ---------------------------------------------------------------------------
// PiSugar manager
// ---------------------------------------------------------------------------

/// PiSugar manager.
pub struct PiSugar {
    pub config: PiSugarConfig,
    pub status: BatteryStatus,
    pub available: bool,
    /// Board temperature in °C (register 0x04 value - 40).
    pub temperature_c: i8,
    /// Whether a shutdown sequence is in progress.
    pub shutting_down: bool,
}

impl PiSugar {
    /// Create a new PiSugar manager with the given configuration.
    pub fn new(config: PiSugarConfig) -> Self {
        Self {
            config,
            status: BatteryStatus::default(),
            available: false,
            temperature_c: 0,
            shutting_down: false,
        }
    }

    /// Probe for PiSugar on I2C bus.
    pub fn probe(&mut self) -> bool {
        self.available = i2c_probe(self.config.i2c_bus, self.config.i2c_addr);
        if self.available && self.config.auto_shutdown && self.config.auto_shutdown_level > 0 {
            let _ = self.set_auto_shutdown_level(self.config.auto_shutdown_level);
        }
        self.available
    }

    /// Read-modify-write a register with write protection unlock/re-lock.
    /// Always re-locks (0x0B=0x00) even on error.
    fn write_protected(
        &self, bus: u8, addr: u16, register: u8, modify: impl FnOnce(u8) -> u8,
    ) -> Result<(), I2cError> {
        i2c_write_register(bus, addr, REG_WRITE_PROTECT, 0x29)?;
        let current = match i2c_read_register(bus, addr, register) {
            Ok(v) => v,
            Err(e) => {
                let _ = i2c_write_register(bus, addr, REG_WRITE_PROTECT, 0x00);
                return Err(e);
            }
        };
        let new_val = modify(current);
        let result = i2c_write_register(bus, addr, register, new_val);
        let _ = i2c_write_register(bus, addr, REG_WRITE_PROTECT, 0x00);
        result
    }

    /// Configure CTR1 and CTR2 control registers on boot.
    /// Non-fatal: logs warnings on failure.
    pub fn configure_registers(&mut self) {
        if !self.available { return; }
        let bus = self.config.i2c_bus;
        let addr = self.config.i2c_addr;

        if let Err(e) = self.write_protected(bus, addr, REG_CTR1, |val| {
            (val & !CTR1_ANTI_MISTOUCH) | CTR1_AUTO_RESTORE
        }) {
            log::warn!("pisugar: CTR1 configure failed: {e}");
        }

        if let Err(e) = self.write_protected(bus, addr, REG_CTR2, |val| {
            (val | CTR2_SOFT_SHUTDOWN_ENABLE) & !CTR2_AUTO_HIBERNATE
        }) {
            log::warn!("pisugar: CTR2 configure failed: {e}");
        }

        log::info!("pisugar: control registers configured");
    }

    /// Read battery status from I2C registers.
    /// On non-Pi or if not available, just applies thresholds to current status.
    pub fn read_status(&mut self) -> &BatteryStatus {
        if self.available {
            // Read battery level (register 0x2A)
            if let Ok(raw_level) =
                i2c_read_register(self.config.i2c_bus, self.config.i2c_addr, REG_BATTERY_LEVEL)
            {
                self.status.level = parse_battery_level(raw_level);
            }

            // Read voltage (registers 0x22-0x23, big-endian mV)
            if let Ok(v_high) =
                i2c_read_register(self.config.i2c_bus, self.config.i2c_addr, REG_VOLTAGE_HIGH)
            {
                if let Ok(v_low) =
                    i2c_read_register(self.config.i2c_bus, self.config.i2c_addr, REG_VOLTAGE_LOW)
                {
                    self.status.voltage_mv = parse_voltage_mv(v_high, v_low);
                }
            }

            // Read charging status (register 0x02)
            if let Ok(flags) =
                i2c_read_register(self.config.i2c_bus, self.config.i2c_addr, REG_CHARGE_STATUS)
            {
                self.status.charge_state = parse_charge_state(flags);
            }

            // Read board temperature (register 0x04, value - 40 = °C)
            // Clamp to valid range to avoid i8 overflow on corrupted reads
            if let Ok(raw) = i2c_read_register(self.config.i2c_bus, self.config.i2c_addr, REG_TEMPERATURE) {
                self.temperature_c = (raw as i16 - 40).clamp(-40, 125) as i8;
            }
        }

        // Apply thresholds
        self.status.low = self.status.level <= self.config.low_threshold;
        self.status.critical = self.status.level <= self.config.critical_threshold;
        &self.status
    }

    /// Read and clear a tap event from the PiSugar MCU TAP register.
    pub fn read_tap_event(&mut self) -> Option<ButtonAction> {
        if !self.available { return None; }
        let value = i2c_read_register(self.config.i2c_bus, self.config.i2c_addr, REG_TAP).ok()?;
        let action = parse_tap_event(value)?;
        // Clear the TAP register (requires write protection unlock)
        if self.write_protected(self.config.i2c_bus, self.config.i2c_addr, REG_TAP, |_| 0x00).is_err() {
            return None; // Clear failed — skip, retry next iteration
        }
        Some(action)
    }

    /// Check if the PiSugar MCU has signaled a soft shutdown (power button long-press).
    pub fn check_soft_shutdown(&mut self) -> bool {
        if self.shutting_down || !self.available { return false; }
        let ctr2 = match i2c_read_register(self.config.i2c_bus, self.config.i2c_addr, REG_CTR2) {
            Ok(v) => v,
            Err(_) => return false,
        };
        if ctr2 & CTR2_SOFT_SHUTDOWN_STATE == 0 { return false; }
        log::info!("pisugar: power button soft shutdown detected");
        // Boot-loop guard: charging at low battery = skip shutdown
        if self.status.level < 10 && self.status.charge_state == ChargeState::Charging {
            log::info!("pisugar: charging at {}% — skipping shutdown (boot-loop guard)", self.status.level);
            return false;
        }
        true
    }

    /// Execute a graceful shutdown sequence via the PiSugar hardware.
    pub fn pisugar_shutdown(&mut self) {
        self.shutting_down = true;
        // Write 15 × 2 = 30 second hardware safety net
        if let Err(e) = self.write_protected(self.config.i2c_bus, self.config.i2c_addr, REG_DELAY_SHUTDOWN, |_| 15) {
            log::warn!("pisugar: failed to write delay timer: {e}");
        }
        #[cfg(unix)]
        {
            log::info!("pisugar: executing shutdown -h now");
            let _ = std::process::Command::new("sudo").args(["shutdown", "-h", "now"]).spawn();
        }
    }

    /// Set the auto-shutdown battery level on the PiSugar hardware.
    pub fn set_auto_shutdown_level(&self, level: u8) -> Result<(), I2cError> {
        i2c_write_register(self.config.i2c_bus, self.config.i2c_addr, REG_WRITE_PROTECT, 0x29)?;
        let result = i2c_write_register(self.config.i2c_bus, self.config.i2c_addr, REG_AUTO_SHUTDOWN, level);
        let _ = i2c_write_register(self.config.i2c_bus, self.config.i2c_addr, REG_WRITE_PROTECT, 0x00);
        result
    }

    /// Update battery level (for testing or mock data).
    pub fn set_level(&mut self, level: u8) {
        self.status.level = level.min(100);
        self.status.low = level <= self.config.low_threshold;
        self.status.critical = level <= self.config.critical_threshold;
    }

    /// Check if shutdown should be triggered.
    pub fn should_shutdown(&self) -> bool {
        self.config.auto_shutdown && self.status.critical
    }

    /// Display string for battery level.
    pub fn display_str(&self) -> String {
        if !self.available {
            return "BAT N/A".to_string();
        }
        let label = match self.status.charge_state {
            ChargeState::Charging | ChargeState::Full => "CHG",
            _ => "BAT",
        };
        format!("{}={}%", label, self.status.level)
    }
}

impl Default for PiSugar {
    fn default() -> Self {
        Self::new(PiSugarConfig::default())
    }
}

// ---------------------------------------------------------------------------
// Button action mapping
// ---------------------------------------------------------------------------

/// Mapped button actions for the PiSugar custom button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappedAction {
    /// Cycle rage level 1→2→3→4→5→6→1 (skips 7/YOLO — crashes firmware).
    CycleRageLevel,
    /// Toggle BT PAN tethering on/off.
    ToggleBtTether,
    /// Toggle between RAGE and SAFE mode.
    ToggleRageSafe,
}

/// Map a raw button action to a semantic daemon action.
pub fn map_button_action(action: ButtonAction) -> MappedAction {
    match action {
        ButtonAction::SingleTap => MappedAction::CycleRageLevel,
        ButtonAction::DoubleTap => MappedAction::ToggleBtTether,
        ButtonAction::LongPress => MappedAction::ToggleRageSafe,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Register constant tests =====

    #[test]
    fn test_i2c_address_constants() {
        assert_eq!(I2C_ADDR_BATTERY, 0x57);
        assert_eq!(I2C_ADDR_RTC, 0x68);
    }

    #[test]
    fn test_register_addresses() {
        assert_eq!(REG_BATTERY_LEVEL, 0x2A);
        assert_eq!(REG_VOLTAGE_HIGH, 0x22);
        assert_eq!(REG_VOLTAGE_LOW, 0x23);
        assert_eq!(REG_CTR1, 0x02);
        assert_eq!(REG_CHARGE_STATUS, 0x02);
        assert_eq!(REG_CTR1, REG_CHARGE_STATUS);
        assert_eq!(REG_CTR2, 0x03);
        assert_eq!(REG_TEMPERATURE, 0x04);
        assert_eq!(REG_TAP, 0x08);
        assert_eq!(REG_DELAY_SHUTDOWN, 0x09);
        assert_eq!(REG_WRITE_PROTECT, 0x0B);
        assert_eq!(REG_AUTO_SHUTDOWN, 0x19);
    }

    #[test]
    fn test_charge_flag_constants() {
        assert_eq!(CHARGE_FLAG_POWER_CONNECTED, 0x01);
        assert_eq!(CHARGE_FLAG_CHARGING, 0x02);
        assert_eq!(CHARGE_FLAG_FULL, 0x04);
        assert_eq!(
            CHARGE_FLAG_POWER_CONNECTED & CHARGE_FLAG_CHARGING & CHARGE_FLAG_FULL,
            0
        );
    }

    #[test]
    fn test_tap_values() {
        assert_eq!(0u8 & 0x03, 0);
        assert_eq!(1u8 & 0x03, 1);
        assert_eq!(2u8 & 0x03, 2);
        assert_eq!(3u8 & 0x03, 3);
    }

    // ===== Battery level parsing tests =====

    #[test]
    fn test_parse_battery_level_normal() {
        assert_eq!(parse_battery_level(75), 75);
        assert_eq!(parse_battery_level(0), 0);
        assert_eq!(parse_battery_level(100), 100);
    }

    #[test]
    fn test_parse_battery_level_clamps_overflow() {
        assert_eq!(parse_battery_level(120), 100);
        assert_eq!(parse_battery_level(255), 100);
    }

    #[test]
    fn test_parse_battery_level_boundaries() {
        assert_eq!(parse_battery_level(1), 1);
        assert_eq!(parse_battery_level(99), 99);
        assert_eq!(parse_battery_level(101), 100);
    }

    // ===== Voltage conversion tests =====

    #[test]
    fn test_parse_voltage_mv_typical_values() {
        assert_eq!(parse_voltage_mv(0x10, 0x68), 4200);
        assert_eq!(parse_voltage_mv(0x0C, 0xE4), 3300);
    }

    #[test]
    fn test_parse_voltage_mv_zero() {
        assert_eq!(parse_voltage_mv(0x00, 0x00), 0);
    }

    #[test]
    fn test_parse_voltage_mv_max() {
        assert_eq!(parse_voltage_mv(0xFF, 0xFF), 65535);
    }

    #[test]
    fn test_parse_voltage_mv_big_endian_order() {
        assert_eq!(parse_voltage_mv(0x01, 0x00), 256);
        assert_eq!(parse_voltage_mv(0x00, 0x01), 1);
    }

    // ===== Charge state decoding tests =====

    #[test]
    fn test_parse_charge_state_discharging() {
        assert_eq!(parse_charge_state(0x00), ChargeState::Discharging);
    }

    #[test]
    fn test_parse_charge_state_charging() {
        assert_eq!(
            parse_charge_state(CHARGE_FLAG_CHARGING | CHARGE_FLAG_POWER_CONNECTED),
            ChargeState::Charging
        );
        assert_eq!(
            parse_charge_state(CHARGE_FLAG_CHARGING),
            ChargeState::Charging
        );
    }

    #[test]
    fn test_parse_charge_state_full() {
        assert_eq!(
            parse_charge_state(CHARGE_FLAG_FULL | CHARGE_FLAG_POWER_CONNECTED),
            ChargeState::Full
        );
        assert_eq!(parse_charge_state(CHARGE_FLAG_FULL), ChargeState::Full);
    }

    #[test]
    fn test_parse_charge_state_power_only() {
        assert_eq!(
            parse_charge_state(CHARGE_FLAG_POWER_CONNECTED),
            ChargeState::Full
        );
    }

    #[test]
    fn test_parse_charge_state_full_overrides_charging() {
        assert_eq!(
            parse_charge_state(CHARGE_FLAG_FULL | CHARGE_FLAG_CHARGING),
            ChargeState::Full
        );
    }

    // ===== TAP event parsing tests =====

    #[test]
    fn test_parse_tap_event_none() {
        assert_eq!(parse_tap_event(0x00), None);
    }

    #[test]
    fn test_parse_tap_event_single() {
        assert_eq!(parse_tap_event(0x01), Some(ButtonAction::SingleTap));
    }

    #[test]
    fn test_parse_tap_event_double() {
        assert_eq!(parse_tap_event(0x02), Some(ButtonAction::DoubleTap));
    }

    #[test]
    fn test_parse_tap_event_long() {
        assert_eq!(parse_tap_event(0x03), Some(ButtonAction::LongPress));
    }

    #[test]
    fn test_parse_tap_event_masks_upper_bits() {
        assert_eq!(parse_tap_event(0x04), None);
        assert_eq!(parse_tap_event(0x05), Some(ButtonAction::SingleTap));
        assert_eq!(parse_tap_event(0x80), None);
        assert_eq!(parse_tap_event(0xFF), Some(ButtonAction::LongPress));
        assert_eq!(parse_tap_event(0xFE), Some(ButtonAction::DoubleTap));
    }

    // ===== New field defaults =====

    #[test]
    fn test_pisugar_default_new_fields() {
        let ps = PiSugar::default();
        assert_eq!(ps.temperature_c, 0);
        assert!(!ps.shutting_down);
    }

    #[test]
    fn test_parse_temperature() {
        assert_eq!((0x49_u8 as i16 - 40).clamp(-40, 125) as i8, 33);
        assert_eq!((0x28_u8 as i16 - 40).clamp(-40, 125) as i8, 0);
        assert_eq!((0x00_u8 as i16 - 40).clamp(-40, 125) as i8, -40);
        assert_eq!((0x7D_u8 as i16 - 40).clamp(-40, 125) as i8, 85);
        assert_eq!((0xA8_u8 as i16 - 40).clamp(-40, 125) as i8, 125); // overflow clamped
        assert_eq!((0xFF_u8 as i16 - 40).clamp(-40, 125) as i8, 125); // overflow clamped
    }

    #[test]
    fn test_read_tap_event_unavailable() {
        let mut ps = PiSugar::default();
        assert_eq!(ps.read_tap_event(), None);
    }

    #[test]
    fn test_check_soft_shutdown_unavailable() {
        let mut ps = PiSugar::default();
        assert!(!ps.check_soft_shutdown());
    }

    #[test]
    fn test_check_soft_shutdown_already_shutting_down() {
        let mut ps = PiSugar::default();
        ps.shutting_down = true;
        assert!(!ps.check_soft_shutdown());
    }

    // ===== Existing battery/pisugar tests =====

    #[test]
    fn test_default_battery() {
        let ps = PiSugar::default();
        assert_eq!(ps.status.level, 100);
        assert!(!ps.status.critical);
        assert!(!ps.status.low);
        assert!(!ps.available);
    }

    #[test]
    fn test_set_level_thresholds() {
        let mut ps = PiSugar::default();
        ps.set_level(15);
        assert!(ps.status.low);
        assert!(!ps.status.critical);

        ps.set_level(3);
        assert!(ps.status.low);
        assert!(ps.status.critical);
    }

    #[test]
    fn test_should_shutdown() {
        let mut ps = PiSugar::default();
        ps.set_level(3);
        assert!(ps.should_shutdown());

        ps.config.auto_shutdown = false;
        assert!(!ps.should_shutdown());
    }

    #[test]
    fn test_display_str_unavailable() {
        let ps = PiSugar::default();
        assert_eq!(ps.display_str(), "BAT N/A");
    }

    #[test]
    fn test_display_str_available() {
        let mut ps = PiSugar::default();
        ps.available = true;
        ps.status.level = 75;
        ps.status.charge_state = ChargeState::Discharging;
        assert_eq!(ps.display_str(), "BAT=75%");

        ps.status.charge_state = ChargeState::Charging;
        assert_eq!(ps.display_str(), "CHG=75%");
    }

    #[test]
    fn test_set_level_clamps() {
        let mut ps = PiSugar::default();
        ps.set_level(150);
        assert_eq!(ps.status.level, 100);
    }

    #[test]
    fn test_read_status_applies_thresholds() {
        let mut ps = PiSugar::default();
        ps.status.level = 10;
        ps.read_status();
        assert!(ps.status.low);
        assert!(!ps.status.critical);
    }

    #[test]
    fn test_charge_states() {
        assert_ne!(ChargeState::Charging, ChargeState::Discharging);
        assert_ne!(ChargeState::Full, ChargeState::Unknown);
    }

    #[test]
    fn test_battery_level_zero() {
        let mut ps = PiSugar::default();
        ps.set_level(0);
        assert!(ps.status.critical);
        assert!(ps.status.low);
        assert!(ps.should_shutdown());
    }

    #[test]
    fn test_battery_level_100() {
        let mut ps = PiSugar::default();
        ps.set_level(100);
        assert!(!ps.status.critical);
        assert!(!ps.status.low);
        assert!(!ps.should_shutdown());
    }

    #[test]
    fn test_battery_display_str_charging_full() {
        let mut ps = PiSugar::default();
        ps.available = true;
        ps.status.level = 100;
        ps.status.charge_state = ChargeState::Full;
        assert_eq!(ps.display_str(), "CHG=100%");
    }

    // ===== Button action mapping tests =====

    #[test]
    fn test_single_tap_maps_to_cycle_rage() {
        assert_eq!(map_button_action(ButtonAction::SingleTap), MappedAction::CycleRageLevel);
    }

    #[test]
    fn test_double_tap_maps_to_bt_tether() {
        assert_eq!(map_button_action(ButtonAction::DoubleTap), MappedAction::ToggleBtTether);
    }

    #[test]
    fn test_long_press_maps_to_rage_safe() {
        assert_eq!(map_button_action(ButtonAction::LongPress), MappedAction::ToggleRageSafe);
    }

    // ===== Config defaults test =====

    #[test]
    fn test_config_defaults() {
        let cfg = PiSugarConfig::default();
        assert_eq!(cfg.i2c_bus, 1);
        assert_eq!(cfg.i2c_addr, 0x57);
        assert_eq!(cfg.low_threshold, 20);
        assert_eq!(cfg.critical_threshold, 5);
        assert_eq!(cfg.auto_shutdown_level, 5);
        assert!(cfg.auto_shutdown);
    }

    // ===== I2C error display =====

    #[test]
    fn test_i2c_error_display() {
        let e = I2cError::BusOpen("permission denied".into());
        assert!(e.to_string().contains("permission denied"));

        let e = I2cError::DeviceNotFound(0x57);
        assert!(e.to_string().contains("0x57"));

        let e = I2cError::IoError("timeout".into());
        assert!(e.to_string().contains("timeout"));
    }

    #[test]
    fn test_probe_not_available_on_non_pi() {
        let mut ps = PiSugar::default();
        assert!(!ps.probe());
        assert!(!ps.available);
    }

    #[test]
    fn test_set_auto_shutdown_level_non_pi() {
        let ps = PiSugar::default();
        assert!(ps.set_auto_shutdown_level(10).is_err());
    }
}
