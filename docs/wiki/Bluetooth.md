# Bluetooth Pentest Mode

← [Back to Wiki Home](Home)

---

The Pi Zero 2W's BCM43436B0 chip shares a single UART between WiFi and Bluetooth — they cannot run simultaneously. Oxigotchi cleanly separates them into two operating modes.

## RAGE / SAFE Mode

- **RAGE** (default) — WiFi monitor mode, AngryOxide attacking, BT off
- **SAFE** — WiFi managed mode, BT tethered to phone for internet, no attacks

Switch via the **PiSugar3 button** (single tap) or the **web dashboard** (RAGE/SAFE buttons). The switch happens at the next epoch boundary (~30 seconds).

In RAGE mode, the bull is hunting — all 6 attack types active, WiFi in monitor mode, BT radio off. This is what you use for wardriving and handshake capture.

In SAFE mode, the bull is resting — WiFi switches to managed mode, BT tethers to your phone for internet access. This enables:
- **WPA-SEC auto-upload** — captured handshakes upload to wpa-sec for cloud cracking
- **Discord notifications** — webhook fires when handshakes are captured
- **SSH over BT** — if USB isn't connected, BT PAN provides network access to the Pi

## Bluetooth Tethering

Bluetooth tethering is built into the Rust daemon and activates automatically in SAFE mode.

When switching from RAGE to SAFE, the daemon:
1. Stops AngryOxide and releases wlan0mon
2. Reloads the `hci_uart` kernel module to reset the shared UART
3. Powers on Bluetooth via `bluetoothctl`
4. Connects to your configured phone via BT PAN
5. Acquires an IP address via DHCP over the BT network interface

The switch back to RAGE reverses this — BT is powered off, the UART is reclaimed for WiFi, monitor mode is re-established, and AngryOxide restarts.

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
- Current BT state (on/off/connecting/connected)
- Connected phone name and MAC
- RAGE/SAFE mode toggle buttons
- BT visibility toggle (for initial pairing)
