# Bluetooth Tethering on Pi Zero 2W

> **Note (v3.3):** BT tethering is phone-initiated. The Pi is discoverable by
> default, you pair from your phone's Bluetooth settings, the daemon auto-
> trusts the new bond and connects PAN via D-Bus BlueZ. No web-UI "Scan +
> Pair" dance, no MAC address in config, no nmcli.

## Hardware: BCM43436B0 Dual-Bus Architecture

The Pi Zero 2W's BCM43436B0 combo chip uses **two independent buses**:
- **WiFi** — SDIO bus (parallel, high-bandwidth)
- **Bluetooth** — UART bus (serial, HCI protocol)

Because the buses are independent, BT tethering can stay alive while WiFi is
in monitor mode (RAGE). The only exception is **BT offensive mode**, which
requires exclusive UART access to load custom firmware — that disconnects
phone tethering.

## Pairing flow (what users actually do)

On a fresh image, the Pi boots discoverable. To connect your phone:

1. On your phone, turn **mobile data ON** — hotspot needs a data connection
   to share with the Pi.
2. Turn **WiFi OFF** on the phone so its tethering routes via mobile data.
3. Open the phone's **Bluetooth settings** and tap `oxigotchi` when it
   appears in the nearby-devices list.
4. When the pair popup shows a 6-digit code, confirm it matches the code
   shown in the web dashboard (Phone Tethering card) and tap **Pair** on
   the phone within a few seconds.
5. Done. The daemon auto-trusts the bond, calls `Network1.Connect("nap")`,
   runs DHCP on bnep0, and adds the default route. The whole thing takes
   about 1 second after the phone confirms.

No buttons to press on the web UI during pairing. No scan step. The Pi
stays discoverable by default so you can pair a second phone later without
toggling anything.

## What the daemon does under the hood

1. **Boot**: power on BT adapter, assert `pairable=true` and
   `discoverable=true` on the adapter via D-Bus.
2. **Register Agent1** so incoming pair requests get auto-confirmed in
   NoInputNoOutput mode (the phone's user confirms the passkey; the Pi
   auto-accepts on its side).
3. **Register a PropertiesChanged watcher** on `org.bluez.Device1` that
   fires the moment a new bond transitions to `Paired=true`. The main
   loop catches the event and calls `trust_device()` on that path so the
   bond ends up `Paired=true, Trusted=true` without any user intervention.
4. **Auto-connect loop** — every epoch, if a paired device exists and the
   adapter isn't already tethered, the daemon calls
   `org.bluez.Network1.Connect("nap")`, runs `dhcpcd` on the returned
   interface (`bnep0`), and flips the daemon state to `Connected`.
5. **Auto-adoption** — if `bnep0` already exists at boot (from a previous
   session, or because a system service manages the tether outside the
   daemon), the daemon adopts it instead of re-connecting.
6. **Bus-death recovery** — all of the above routes through `ensure_dbus()`
   which health-checks the existing D-Bus connection (`is_bus_alive`),
   tears down and re-creates on `bluetoothd` restart, and re-registers
   Agent1 + the PropertiesChanged watcher on the new connection.

## Boot order

The daemon sets up BT tethering **before** starting WiFi monitor mode at
boot so there's no SDIO↔UART coexistence weirdness:

```
1. Power on BT adapter
2. Assert pairable + discoverable (via bluetoothctl, not btmgmt — btmgmt hangs without a TTY)
3. Connect to the paired phone via Network1.Connect("nap") OR adopt existing bnep0
4. Run dhcpcd on bnep0
5. THEN start WiFi monitor mode and AngryOxide
```

## Config

In `/etc/oxigotchi/config.toml`:

```toml
[bluetooth]
enabled = true
phone_name = "Phone Name"          # Optional display label
auto_connect = true                # Auto-connect to paired phone at boot
# Stay discoverable even after a successful tether so users can add a second
# phone later without toggling anything in the web UI. Set to true if you
# prefer the adapter to go invisible once a tether is live.
hide_after_connect = false
```

No MAC address is needed. The daemon picks the best candidate via
`ObjectManager` and prefers devices that:

1. Match `phone_name` as a substring (case-insensitive)
2. Are currently connected
3. Advertise the NAP UUID (`00001116-0000-1000-8000-00805f9b34fb`) — paired
   headsets, watches, and smart speakers are filtered out so they can't
   starve auto-connect

## Auto-reconnect

If the BT connection drops, the daemon reconnects with exponential
backoff:

- **Schedule**: 30s → 60s → 120s → 300s (caps at 5 minutes)
- **No max retry limit** — keeps trying indefinitely
- Reconnect runs every epoch in all modes (RAGE, BT, SAFE)
- If the user explicitly disconnects via the dashboard, auto-reconnect is
  paused until manually re-enabled

## Dashboard Controls

The web dashboard's **Phone Tethering** section shows:

- **Instructions** for the 4-step phone-initiated pairing flow
- **Passkey display** — when the phone initiates a pair and Agent1 gets a
  `RequestConfirmation`, the 6-digit code shows here so you can verify it
  matches your phone before tapping Pair
- **Disconnect** button — manually drops the tether and suppresses
  auto-reconnect until re-enabled
- **Reset pairings** button — nuclear option, forgets every paired device
  in BlueZ and starts fresh

There is intentionally **no** "Scan for Devices" or "Pair from Pi" button.
The Pi-initiated outgoing pair flow used to exist but was fragile across
different Android and MIUI builds. The phone-initiated flow is simpler
and reliable.

## Troubleshooting

### First step for ANY pair issue: clean slate on both sides

The single most common cause of pair/tether problems is a stale one-sided
bond: one side forgot the bond, the other kept it. Before you spend any
time debugging, do this:

1. **On the Pi**: web dashboard → Bluetooth card → **Reset pairings** →
   confirm. This forgets every paired device in BlueZ.
2. **On the phone**: go to Bluetooth settings → tap the info/gear next to
   `oxigotchi` → **Forget This Device** (or "Unpair"). Then toggle
   Bluetooth off and back on.
3. Pair fresh from the phone side (scan, tap `oxigotchi`, confirm
   passkey, tap Pair).

This fixes the vast majority of "I was paired but now it doesn't work"
reports. Try it before reading anything else in this section.

### "I paired but the Pi never connects" (and Reset pairings didn't fix it)

- **Check that mobile data is actually ON on the phone**. Without it the
  phone's BNEP service refuses the connection with `Connection refused`.
- **Check that the phone's "Bluetooth tethering" toggle is ON** (usually
  under Portable Hotspot settings). On iOS this is **Settings → Personal
  Hotspot → Allow Others to Join**.
- **Toggle Bluetooth tethering OFF then ON** on the phone — some MIUI
  builds need this to re-bind the BNEP service to a fresh bond.
- **Keep the phone screen unlocked** during the first connect attempt,
  especially on iOS. iOS suspends hotspot services aggressively when the
  screen is off.
- **Reboot the phone entirely**. Sounds drastic, but when the phone's BT
  stack is in a stuck state from previous failed attempts, a full power
  cycle is the only reliable fix.

### "I see `oxigotchi` in my phone's BT list, tap it, then nothing happens"

- Watch the phone screen for a passkey confirmation popup. If it appears,
  tap **Pair** within a few seconds. Some Androids (notably MIUI) auto-
  dismiss the popup if you don't respond.
- If no popup appears, the Pi may already have a stale bond for this
  phone. Click **Reset pairings** in the web UI and try again.

### "Phone says connected, the Pi trusted the phone, but the e-ink still shows BT:-"

The daemon's "BT:C" indicator comes from `self.bluetooth.state ==
Connected`, which only happens after `Network1.Connect("nap")` succeeds
and a `bnep` interface is up. If Paired+Trusted is true on both sides but
the BT indicator stays "-", it means the PAN profile never opened. Most
common causes:

- iOS: Personal Hotspot → **Allow Others to Join** is off.
- Android: Bluetooth tethering toggle is off in Portable Hotspot
  settings, OR mobile data is off.
- Stale one-sided bond — do the clean-slate reset at the top of this
  section.

You can confirm by querying the daemon:

```bash
curl -s http://127.0.0.1:8080/api/bluetooth
```

If `connected: false` and `ip: ""` while the phone says "Connected",
that's the Pi-side PAN failing. Check the daemon log for the actual
error:

```bash
sudo journalctl -u rusty-oxigotchi --since '2 minutes ago' | grep -iE 'Network1|PAN|bnep'
sudo journalctl -u bluetooth --since '2 minutes ago' | tail -20
```

### "Pair works but bond doesn't survive reboot"

- This was a real bug in v3.3.0 and earlier: `list_paired_devices` filtered
  out `Paired=true, Trusted=false` bonds, and phone-initiated pairs produce
  exactly that state. Fixed in v3.3.1 — the PropertiesChanged watcher
  auto-trusts the moment Paired becomes true.

### "I want the tether to stop being discoverable after it connects"

Set `hide_after_connect = true` in `/etc/oxigotchi/config.toml` under
`[bluetooth]`. Restart the daemon. The adapter will `hide()` itself after
a successful `Network1.Connect`.

## Mode Transitions and BT Tethering

### RAGE Mode (BT tether stays connected)
BT tethering remains active during RAGE mode. The daemon auto-reconnects
each epoch if the connection drops. WiFi monitor mode and BT PAN coexist
on independent buses.

### BT Offensive Mode (BT tether disconnects)
Switching to BT offensive mode disconnects phone tethering — the UART is
reclaimed for custom firmware. A warning appears in the web dashboard.
When returning from BT offensive mode to RAGE or SAFE, the daemon
automatically reconnects BT tethering.

### SAFE Mode (BT tether active)
BT tethering is fully active. WiFi is in managed mode (no monitor, no
attacks).

## Python Pwnagotchi Comparison

The Python `bt-tether` plugin required a hardcoded MAC address in config,
used retry loops, and had its own keepalive daemon. The Rust version:

- No MAC address needed — auto-discovers via `ObjectManager` and filters
  by NAP UUID
- Exponential backoff reconnect instead of hammer-loops
- Phone-initiated pairing (no fragile Pi-initiated Pair() with state
  machines)
- Auto-trust on PropertiesChanged so phone-initiated bonds are first-class
- Transparent iOS/Android MAC randomization via BlueZ bonding IRK exchange
- Automatic bnep0 adoption if the interface already exists at boot
- Bus-death recovery routes through `ensure_dbus` so Agent1 and the
  watcher are always re-registered after a `bluetoothd` restart
