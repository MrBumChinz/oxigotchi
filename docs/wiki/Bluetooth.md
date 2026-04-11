# Bluetooth Pentest Mode

← [Back to Wiki Home](Home)

---

The Pi Zero 2W's BCM43436B0 chip uses **two independent buses** — SDIO for WiFi and UART for Bluetooth. BT phone tethering stays connected in RAGE and SAFE modes. Only BT attack mode requires exclusive UART access.

## Three Operating Modes

- **RAGE** — WiFi monitor mode, AngryOxide attacking, BT tether stays connected. The wardriving mode.
- **BT** — Bluetooth offensive: HCI scanning, GATT resolution, BT attacks. WiFi off, phone tether disconnected.
- **SAFE** — WiFi managed mode, BT tethered to phone for internet, no attacks.

Switch via the **PiSugar3 button** (single tap) or the **web dashboard** mode buttons. Transitions are managed atomically by `RadioManager` — the lock file prevents partial states.

### RAGE Mode

The bull is hunting. All WiFi attack types active, monitor mode on wlan0mon. BT phone tethering stays connected — you keep SSH and web dashboard access over BT while wardriving.

### BT Mode

The bull goes Bluetooth hunting. WiFi is fully released, the UART is reclaimed for BT, and a custom patchram is loaded to enable attack-capable firmware. The daemon:

1. Stops AngryOxide and releases wlan0mon
2. Loads the BT patchram (BCM43436B0 HCD with attack extensions)
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

When switching modes, the daemon handles radio teardown and bringup:

**Any → RAGE:**
1. Release previous mode's radio (BT patchram or managed WiFi)
2. Enter WiFi monitor mode, start AngryOxide
3. Reconnect BT tether (auto via `ensure_connected()`)

**Any → BT Attack:**
1. Stop AngryOxide/release WiFi (if in RAGE)
2. Load BT attack patchram — **disconnects phone tethering**
3. Begin HCI scanning and BT attacks

**Any → SAFE:**
1. Release previous mode's radio
2. Switch WiFi to managed mode
3. Ensure BT tether is connected

The `RadioManager` uses a lock file to prevent concurrent mode transitions and ensure clean handoff.

## Bluetooth Tethering (Phone-Initiated)

BT tethering uses D-Bus BlueZ directly (`Network1.Connect("nap")`) and stays connected in RAGE and SAFE modes. Pairing is **phone-initiated** — the Pi is discoverable by default and you pair from your phone's Bluetooth settings like any other device.

**Pairing a new phone:**

1. On your phone, turn **mobile data ON** (hotspot needs a data connection to share)
2. Turn **WiFi OFF** on the phone (so hotspot routes via mobile data)
3. Open the phone's **Bluetooth settings** and tap `oxigotchi` when it appears
4. When the passkey popup shows, confirm it matches the code in the web dashboard and tap **Pair** on the phone

The daemon auto-trusts the new bond (via a D-Bus PropertiesChanged watcher on `Device1.Paired`), calls `Network1.Connect("nap")`, and runs DHCP on `bnep0`. ~1 second after the phone confirms, the tether is live.

**Runtime behavior:**

1. At boot, powers on Bluetooth, asserts `pairable=true` and `discoverable=true`, and connects to the paired phone via D-Bus PAN **before** starting WiFi monitor mode
2. If `bnep0` already exists at boot (from a previous session), adopts it instead of re-connecting
3. Periodically checks BT connection health and auto-reconnects with exponential backoff (30s → 60s → 120s → 300s cap)
4. Only BT offensive mode disconnects phone tethering (web dashboard shows a warning)
5. When returning from BT offensive mode, tether auto-reconnects
6. iOS/Android MAC randomization is handled transparently via BlueZ bonding (IRK exchange)
7. Bus-death recovery: if `bluetoothd` restarts, the daemon re-initializes its D-Bus connection, re-registers Agent1, and re-registers the paired-device watcher on the new connection

## Configuration

In `/etc/oxigotchi/config.toml`:

```toml
[bluetooth]
enabled = true
phone_name = "My Phone"       # Optional display label (used to prefer this device when multiple are paired)
auto_connect = true           # Auto-connect at boot
hide_after_connect = false    # Stay discoverable after a successful tether so a second phone can pair later
```

No MAC address is needed — the daemon auto-discovers paired devices via D-Bus `ObjectManager` and filters to only devices that advertise the NAP UUID (so paired headsets, watches, and smart speakers don't poison auto-connect). See [docs/BT_TETHERING.md](https://github.com/CoderFX/oxigotchi/blob/main/docs/BT_TETHERING.md) for full setup instructions and troubleshooting.

### Dashboard Controls

The web dashboard's Bluetooth card shows:
- Current BT state (off/connecting/tethered)
- **Phone Tethering** section with step-by-step pairing instructions
- **Passkey display** — shown when a phone initiates a pair, so you can verify the 6-digit code matches the phone
- **Disconnect** button (only when the tether is live)
- **Reset pairings** button — forgets every paired BT device in BlueZ
- Discoverable toggle (default ON)
- Mode toggle buttons (RAGE/BT/SAFE)

There is intentionally no "Scan + Pair" button from the Pi side. Pi-initiated outgoing pairs were fragile across MIUI builds; phone-initiated pairing is the reliable path and what actually worked in production.
