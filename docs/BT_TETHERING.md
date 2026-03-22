# Bluetooth Tethering on Pi Zero 2W

> **Note:** In Rusty Oxigotchi v3.0, BT and WiFi monitor mode never run simultaneously. The daemon uses RAGE/SAFE mode cycling — BT is only active in SAFE mode, WiFi monitor is only active in RAGE mode. The PiSugar3 button toggles between them. See [RUSTY_V3.md](RUSTY_V3.md) for details.

## Hardware Limitation: BCM43436B0 Shared UART

The Pi Zero 2W uses a BCM43436B0 combo WiFi/BT chip. WiFi and Bluetooth share a single UART bus. This creates a critical constraint:

**Once WiFi enters monitor mode, the BT UART cannot be initialized.**

Symptoms:
- `hciconfig hci0 up` returns "Connection timed out (110)"
- `bluetoothctl power on` fails with "org.bluez.Error.Failed"
- The adapter shows as `DOWN` and cannot be recovered without a reboot or chip reset

## Boot Sequence (Correct Order)

The daemon MUST set up BT **before** starting WiFi monitor mode:

```
1. Power on BT adapter (hciconfig hci0 up / bluetoothctl power on)
2. Pair and connect to phone (bluetoothctl pair/connect + nmcli)
3. Verify bnep0 interface is up
4. THEN start WiFi monitor mode (iw phy0 interface add wlan0mon type monitor)
5. THEN start AngryOxide
```

The current Rust daemon does this in `boot()` — see `rust/src/main.rs`.

> **Note (v3.0):** This boot sequence only applies to the initial SAFE mode transition, not boot. RAGE is the default boot mode — no BT is started at boot. BT is only powered on when the user switches to SAFE mode via the PiSugar3 button.

## Config

In `/etc/oxigotchi/config.toml`:

```toml
[bluetooth]
enabled = true
phone_mac = "XX:XX:XX:XX:XX:XX"   # REQUIRED — get from bluetoothctl devices
phone_name = "Phone Name"          # Used for scan matching if MAC is missing
auto_pair = true
auto_connect = true
hide_after_connect = true
```

### Getting Your Phone's MAC Address

1. Pair your phone to the Pi manually first (while BT is still up)
2. Run `bluetoothctl devices` to see the MAC
3. Add it to the config as `phone_mac`

Having the MAC address is important — without it, the daemon scans for 10 seconds which may fail if your phone isn't discoverable.

## Recovery: BT Adapter Stuck DOWN

If the BT adapter is stuck in DOWN state (common after WiFi monitor mode was started before BT):

1. **Reboot the Pi** — this is the only reliable way to reset the BCM43436B0 UART
2. The daemon will handle the correct boot order on restart

There is no software-only way to recover the UART once it's timed out. `systemctl restart bluetooth`, `hciconfig hci0 reset`, and `hciattach` all fail.

## For SD Card Image Flashers

When someone flashes a new SD card with the oxigotchi image:

1. The daemon starts with `bluetooth.enabled = true` but no `phone_mac`
2. It will scan for 10 seconds looking for a device matching `phone_name`
3. If no phone is found, BT is skipped and WiFi monitor mode starts normally
4. The user should:
   - SSH into the Pi
   - Run `sudo bluetoothctl` → `power on` → `scan on` → find their phone
   - Note the MAC address
   - Add `phone_mac = "XX:XX:XX:XX:XX:XX"` to `/etc/oxigotchi/config.toml`
   - Reboot

Future improvement: a web dashboard BT pairing wizard that handles this flow.

## Known Issues

- **WiFi + BT coexistence**: Once BT is connected and WiFi enters monitor mode, the BT connection typically stays alive. But if BT drops, it CANNOT be re-established without a reboot.
- **nmcli error**: "No suitable device found (device wlan0 not available)" — this means the BT UART is down, not a WiFi issue. The error message is misleading.
- **10-second scan timeout**: If the phone isn't discoverable during the scan window, pairing fails. Using `phone_mac` bypasses this entirely.

## Python Pwnagotchi Comparison

The Python `bt-tether` plugin handled this by:
1. Running `hciconfig hci0 up` in a retry loop
2. Using `dbus` to manage BlueZ directly (not bluetoothctl)
3. Having its own keepalive mechanism

The Rust daemon uses `bluetoothctl` and `nmcli` CLI commands instead, which is simpler but less resilient to the UART timing issue.
