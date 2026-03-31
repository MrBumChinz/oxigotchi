# Bluetooth Pentest Mode

← [Back to Wiki Home](Home)

---

The Pi Zero 2W's BCM43436B0 chip shares a single UART between WiFi and Bluetooth — they cannot run simultaneously. Oxigotchi cleanly separates them into three operating modes.

## Three Operating Modes

- **RAGE** (default) — WiFi monitor mode, AngryOxide attacking, BT off. The wardriving mode.
- **BT** — Bluetooth offensive: HCI scanning, GATT resolution, BT attacks. WiFi off.
- **SAFE** — WiFi managed mode, BT tethered to phone for internet, no attacks.

Switch via the **PiSugar3 button** (single tap) or the **web dashboard** mode buttons. Transitions happen at the next epoch boundary (~30 seconds) and are managed atomically by `RadioManager` — the lock file prevents partial states.

### RAGE Mode

The bull is hunting. All WiFi attack types active, monitor mode on wlan0mon, BT radio off. This is what you use for wardriving and handshake capture.

### BT Mode

The bull goes Bluetooth hunting. WiFi is fully released, the UART is reclaimed for BT, and a custom patchram is loaded to enable attack-capable firmware. The daemon:

1. Stops AngryOxide and releases wlan0mon
2. Loads the BT patchram via `hciattach` (BCM43430B0 HCD with attack extensions)
3. Runs HCI scanning to discover nearby BT devices
4. Resolves GATT services on discoverable targets
5. Identifies vendor/model via BT device class and manufacturer data
6. Launches BT attacks against selected or auto-targeted devices

**BT Aggression Levels** (BT:1 / BT:2 / BT:3):
- **BT:1** — Passive scanning only, no attacks
- **BT:2** — Scanning + targeted attacks on selected devices
- **BT:3** — Full offensive: scan, enumerate, and attack all reachable devices

The aggression level shows in the e-ink mode indicator (e.g., `BT:2`).

### BT Attack Types

| Attack | What It Does |
|--------|-------------|
| **ATT Fuzz** | Sends malformed ATT (Attribute Protocol) requests to crash or confuse GATT servers |
| **BLE ADV** | Crafted BLE advertisement flooding |
| **KNOB** | Key Negotiation of Bluetooth — forces minimum encryption key length (1 byte) during pairing |
| **L2CAP Fuzz** | Sends malformed L2CAP signaling packets to trigger parser bugs |
| **L2CAP Flood** | Connection flood — opens maximum concurrent L2CAP channels |
| **SMP** | Security Manager Protocol attacks — pairing manipulation and key extraction attempts |

All attacks are implemented in `rust/src/bluetooth/attacks/` and use raw HCI sockets.

### SAFE Mode

The bull is resting. WiFi switches to managed mode, BT tethers to your phone for internet access. This enables:
- **WPA-SEC auto-upload** — captured handshakes upload to wpa-sec for cloud cracking
- **Discord notifications** — webhook fires when handshakes are captured
- **SSH over BT** — if USB isn't connected, BT PAN provides network access to the Pi

## Mode Transitions

When switching modes, the daemon handles full radio teardown and bringup:

**RAGE → BT:**
1. Stop AngryOxide, release wlan0mon
2. `rmmod brcmfmac` (release WiFi SDIO driver)
3. Load BT patchram via `hciattach /dev/ttyAMA0`
4. Power on BT, begin HCI scanning

**BT → RAGE:**
1. Power off BT, detach HCI
2. `modprobe brcmfmac` (reload WiFi driver)
3. Wait for wlan0, create wlan0mon
4. Start AngryOxide

**Any → SAFE:**
1. Release current radio mode
2. Load managed WiFi + BT tethering
3. Connect to configured phone

The `RadioManager` uses a lock file to prevent concurrent mode transitions and ensure clean handoff.

## Bluetooth Tethering (SAFE Mode)

BT tethering activates automatically in SAFE mode:

1. Powers on Bluetooth via `bluetoothctl`
2. Connects to your configured phone via BT PAN
3. Acquires an IP address via DHCP over the BT network interface

## Configuration

Configure your phone's Bluetooth MAC address in `/etc/oxigotchi/config.toml`:

```toml
[bluetooth]
enabled = true
phone_mac = "AA:BB:CC:DD:EE:FF"
```

Replace `AA:BB:CC:DD:EE:FF` with your phone's Bluetooth MAC address. To find it:
- **Android:** Settings → About Phone → Status → Bluetooth address
- **iPhone:** Settings → General → About → Bluetooth

Your phone must be paired with the Pi beforehand. See [docs/BT_TETHERING.md](https://github.com/CoderFX/oxigotchi/blob/main/docs/BT_TETHERING.md) for full pairing and setup instructions.

### Dashboard Controls

The web dashboard's Bluetooth card shows:
- Current BT state (off/scanning/attacking/tethered)
- Discovered devices with vendor identification
- BT aggression level selector (BT:1/BT:2/BT:3)
- Mode toggle buttons (RAGE/BT/SAFE)
- BT visibility toggle (for initial pairing)
