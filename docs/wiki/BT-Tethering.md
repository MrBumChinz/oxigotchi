# Bluetooth Tethering — Step-by-Step Guide

← [Back to Wiki Home](Home)

This page walks you through setting up Bluetooth tethering on a fresh oxigotchi image. For the deep-dive technical doc (state machines, D-Bus flow, recovery semantics), see [`docs/BT_TETHERING.md`](https://github.com/CoderFX/oxigotchi/blob/master/docs/BT_TETHERING.md).

---

## What Bluetooth tethering gives you

When your Pi is paired to your phone over Bluetooth, the phone shares its mobile data connection with the Pi via the BNEP/PAN profile. Oxigotchi can then:

- **Upload captured handshakes** to [WPA-SEC](https://wpa-sec.stanev.org) for free cloud cracking
- **Fetch cracked passwords** back from WPA-SEC and display them on the dashboard
- **Send Discord webhook notifications** when new handshakes are captured
- Give you **SSH access to the Pi from your phone's browser** at the BT PAN IP — no laptop required for a field walk

BT tether stays connected in **RAGE mode** (WiFi capture active) and **SAFE mode** (managed WiFi). The only time the tether drops is when you switch to **BT offensive mode**, which reclaims the BT radio for custom firmware.

---

## Before you start

On the phone side, you need three things:

1. **Mobile data ON.** The hotspot needs a data connection to share. If mobile data is off, the phone's BNEP service refuses the connect and the Pi gets stuck with `Connection refused` in the logs.
2. **WiFi OFF on the phone.** Android and iOS both route hotspot traffic through whatever the phone's current default connection is. If the phone is on WiFi, the tether will work but you'll be eating your home WiFi, not mobile data. Turn WiFi off on the phone to make sure hotspot actually uses cell.
3. **Bluetooth tethering enabled in the phone's settings.** Location varies by OS:
    - **Android stock / Pixel**: Settings → Network & internet → Hotspot & tethering → Bluetooth tethering
    - **Samsung OneUI**: Settings → Connections → Mobile Hotspot and Tethering → Bluetooth tethering
    - **Xiaomi MIUI**: Settings → Portable hotspot → Bluetooth tethering
    - **iOS**: Settings → Personal Hotspot → **Allow Others to Join** (iOS doesn't have a separate "Bluetooth" toggle — turning on Personal Hotspot enables BT, WiFi, and USB all at once)

On the Pi side, a fresh v3.3.1+ image boots with the BT adapter **discoverable by default** — no config needed. The dashboard's Bluetooth card will show `Discoverable: on` and the adapter will be visible to phones within a few meters.

---

## Step-by-step: pair your phone

Oxigotchi uses **phone-initiated pairing**. You tap `oxigotchi` from your phone's Bluetooth scan list, and the Pi's Agent1 handler auto-confirms on the Pi side. There is intentionally **no** "Scan + Pair" button in the web UI — the Pi-initiated flow was fragile across Android and iOS builds, and phone-initiated is the reliable path.

### 1. Open the phone's Bluetooth settings and scan for devices

On the phone, open Bluetooth settings. If you're on Android, tap **Pair new device** / **Scan**. On iOS, just open the Bluetooth page — it scans automatically.

### 2. Wait for `oxigotchi` to appear in the list, then tap it

The Pi's adapter alias is `oxigotchi` on fresh images. It should appear within 10 seconds. If it doesn't:
- Make sure the Pi is booted and has finished its startup (watch the e-ink display for the idle face)
- Open the web dashboard and check the Bluetooth card — the **Discoverable** toggle should be ON
- Bring the phone physically closer (within a few meters)

### 3. Confirm the passkey on both sides

When you tap `oxigotchi`, the phone shows a popup with a 6-digit passkey. At the same time, the **Phone Tethering** card on the web dashboard shows the same passkey in a highlighted box labeled "Pairing passkey — confirm this matches your phone".

- Verify the two 6-digit numbers match.
- Tap **Pair** on the phone within a few seconds. Some Androids (notably MIUI) auto-dismiss the popup if you don't respond within ~30 seconds.
- There is nothing to click on the web UI side. The Pi's Agent1 auto-confirms, so the only action you take is on the phone.

### 4. That's it

The daemon:
1. Receives the `Paired=true` transition via its D-Bus PropertiesChanged watcher
2. Automatically sets `Trusted=true` on the device so it's a first-class bond
3. Calls `Network1.Connect("nap")` to open the BNEP profile
4. Runs `dhcpcd -4 -n bnep0` to grab an IP lease from the phone's hotspot DHCP
5. Adds a default route through the phone
6. Flips the dashboard's Bluetooth card to `Status: BT OK`, `Connected: true`, `IP: 10.x.x.x`, `Internet: Yes`

All of this happens in about one second after the phone confirms the pair. You should see `BT:C` on the e-ink display (mini status indicator), and the Bluetooth card shows the live IP and internet status.

---

## Confirming the tether works

The web dashboard's **Bluetooth** card shows:

| Field | Expected value |
|---|---|
| **Status** | `BT OK` |
| **Device** | Your phone's alias |
| **IP** | An IP in the phone's hotspot subnet (e.g. `192.168.44.x` on Pixel, `172.20.10.x` on iPhone, or `10.79.x.x` on some carriers) |
| **Internet** | `Yes` |
| **Retries** | `0` |

Or from the command line (plug the Pi into your PC via USB for the initial check):

```bash
ssh pi@10.0.0.2
ip addr show bnep0
ping -c 2 -I bnep0 8.8.8.8
```

You should see a valid IPv4 on `bnep0` and successful ICMP replies from Google's DNS.

---

## If it doesn't work — first step for ANY issue

The single most common cause of pair or tether problems is a **stale one-sided bond**: one side forgot the bond, the other kept it. Before you spend any time on detailed debugging, do this clean-slate reset:

1. **On the Pi**: web dashboard → Bluetooth card → **Reset pairings** → confirm. This forgets every paired device in BlueZ.
2. **On the phone**: Bluetooth settings → tap the info/gear icon next to `oxigotchi` → **Forget This Device** (or "Unpair"). Then toggle Bluetooth off and back on to clear the stack.
3. Pair fresh using the steps above.

This fixes the vast majority of "I was paired but now it doesn't work" reports. Always try it before anything else.

---

## Reading the error hint in the web UI

Starting in v3.3.2, when a PAN connect fails the Bluetooth card shows a red-bordered hint box with a **user-actionable message** from the daemon. The most common hints you'll see:

| Hint | What it means |
|---|---|
| *"Bluetooth tethering isn't enabled on the phone. Turn it on under Portable Hotspot → Bluetooth tethering, and make sure mobile data is ON."* | Self-explanatory — go back to the phone-side checklist |
| *"The phone forgot the pairing. The Pi is removing the stale bond — pair again from the phone's Bluetooth settings."* | One-sided bond. The Pi already cleaned up; just re-pair from the phone. |
| *"Phone isn't responding to connection requests. Try: forget the Pi on the phone → toggle Bluetooth tethering off and back on → pair fresh."* | MIUI-flavor stuck state. Do the clean-slate reset above. |
| *"Phone is out of range or Bluetooth is off. Unlock the phone and bring it closer."* | Page timeout / host down. iOS is especially strict about screen-locked hotspot. |
| *"Phone isn't handing out DHCP leases — enable Bluetooth tethering on the phone"* | BNEP link came up but the phone isn't running DHCP on it. Check the phone's hotspot toggle. |

These hints are generated from `classify_pan_error` in the daemon and are the same information that used to be buried in `journalctl -u rusty-oxigotchi`.

---

## Specific phone quirks

### iOS

- iOS Personal Hotspot behaves differently from Android. There's no separate "Bluetooth tethering" toggle — turning on **Allow Others to Join** enables hotspot on all three transports (BT, WiFi, USB) at once.
- iOS aggressively suspends hotspot services when the screen is locked. **Keep the phone screen unlocked** during the first connect attempt; once the Pi has a lease, you can lock it.
- iOS rotates its Bluetooth MAC address for privacy (random resolvable private addresses). This is handled transparently by BlueZ bonding — the Pi resolves the randomized address using the IRK exchanged during pairing.
- If pair completes (phone says "Connected" and the Pi shows `Trusted: yes`) but the e-ink indicator still says `BT:-`, it usually means Allow Others to Join got turned off or iOS is refusing the PAN profile specifically. Toggle it off and back on.

### MIUI (Xiaomi, Redmi, POCO)

- MIUI auto-dismisses the pair confirmation dialog if you don't respond quickly. Be ready to tap Pair the moment it appears.
- MIUI's Bluetooth tethering toggle sometimes "forgets" to bind to new bonds. If a fresh pair doesn't produce internet, toggle **Portable hotspot → Bluetooth tethering** off and back on. That re-binds the BNEP service to the new bond.
- Battery optimization on MIUI can kill background Bluetooth services. Go to Settings → Battery → Bluetooth share → **No restrictions** to keep hotspot running reliably.

### OneUI (Samsung)

- OneUI's hotspot remembers "allowed devices" across reboots. If you reset pairings on the Pi and re-pair, OneUI sometimes still has the old MAC in its allowed-devices list, which makes the new bond stall. Clear it: Settings → Connections → Mobile Hotspot and Tethering → Mobile Hotspot → tap the Pi in **Allowed devices** → Remove.
- OneUI's "Smart WiFi" feature can interfere — it may automatically switch the phone back to WiFi when it detects a known network, silently breaking the tether. Disable it during field walks.

### GrapheneOS / CalyxOS

- Private-by-default Android forks work fine but have BT turned off in the per-app permissions for system apps by default. Make sure **Bluetooth** is granted to **Settings** and **System UI** before pairing.

---

## FAQ

**Q: Do I need to enter a PIN or password?**
No. Oxigotchi uses SSP (Secure Simple Pairing) numeric comparison. The 6-digit code on the phone and the web dashboard are two independent computations of the same value — if they match, the bond is secure. You tap Pair on the phone and that's it.

**Q: Can I pair multiple phones?**
Yes. Fresh images stay discoverable after a successful tether (the new `hide_after_connect = false` default in v3.3.1+). Pair a second phone the same way. Whichever phone the daemon sees first with a matching NAP UUID will be used for auto-connect. You can bias it with `phone_name` in `/etc/oxigotchi/config.toml` to prefer a specific device.

**Q: What if I want the adapter to go invisible after pairing?**
Set `hide_after_connect = true` in `[bluetooth]` in `/etc/oxigotchi/config.toml` and restart the daemon. The adapter will `hide()` itself after each successful connect.

**Q: Does oxigotchi auto-reconnect if the tether drops?**
Yes. Exponential backoff: 30s → 60s → 120s → 300s (capped at 5 minutes). No retry limit — it keeps trying forever unless you click **Disconnect** in the web UI, which suppresses auto-reconnect until you manually re-enable.

**Q: Does the Pi get internet when plugged into USB?**
Not by default — USB is a point-to-point link. See [Troubleshooting & FAQ](Troubleshooting-and-FAQ#does-oxigotchi-have-internet-when-plugged-in-via-usb) for how to share your PC's internet to the Pi over USB on Windows, macOS, and Linux.

**Q: How do I see why the tether is failing?**
Check the Bluetooth card in the web dashboard — if there's a red hint box, that's the actionable error message. For the raw logs:
```bash
sudo journalctl -u rusty-oxigotchi --since '2 minutes ago' | grep -iE 'bluetooth|PAN|bnep'
sudo journalctl -u bluetooth --since '2 minutes ago' | tail -20
```

---

## See also

- [`docs/BT_TETHERING.md`](https://github.com/CoderFX/oxigotchi/blob/master/docs/BT_TETHERING.md) — full technical reference (D-Bus flow, PropertiesChanged watcher, bus-death recovery, configuration surface)
- [Bluetooth Pentest Mode](Bluetooth) — BT offensive attacks and UART handoff
- [Getting Started](Getting-Started) — fresh-image flash walkthrough
- [Troubleshooting & FAQ](Troubleshooting-and-FAQ) — general troubleshooting
