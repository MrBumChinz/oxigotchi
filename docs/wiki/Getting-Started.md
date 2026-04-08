# Getting Started

← [Back to Wiki Home](Home)

---

## What You Need

- **Pi Zero 2W** with a **Waveshare 2.13" e-ink V4** display and a **PiSugar 3** battery
- A microSD card (8GB+)
- A USB cable for initial setup (micro-USB to your PC)
- An Android phone with Bluetooth tethering (optional but recommended for walkabouts)

## 1. Flash the Image

Download `oxigotchi-v3.3.0-release.img.zip` from the [latest release](https://github.com/CoderFX/oxigotchi/releases/latest).

Flash it to your microSD card using [balenaEtcher](https://etcher.balena.io/) or Raspberry Pi Imager ("Use custom" → select the .img.zip).

Insert the card into your Pi and power it on.

## 2. First Boot

The Pi boots in about 5 seconds. You'll see:

1. The e-ink display shows a boot face
2. WiFi monitor mode starts automatically
3. AngryOxide begins scanning for targets

On first boot, the filesystem auto-expands to fill your SD card. This happens once and takes a few extra seconds.

## 3. Connect to Your Pi

Plug the Pi into your PC via USB. It appears as a network adapter (RNDIS/USB Gadget).

**SSH in:**
```
ssh pi@10.0.0.2
```
Password: `raspberry`

> **Windows users:** If `10.0.0.2` doesn't work, try `192.168.137.2`. Check your network adapters for the RNDIS device.

## 4. Open the Dashboard

In your browser, go to:

```
http://10.0.0.2:8080
```

You'll see the full dashboard with your bull's face, live stats, attack controls, and capture history. Everything auto-refreshes.

## 5. Configure Your Whitelist

**Important:** Add your home WiFi network to the whitelist so your bull doesn't attack it.

In the dashboard, find the **Whitelist** card and add your network name (SSID). Or edit the config directly:

```bash
sudo nano /etc/oxigotchi/config.toml
```

Change the whitelist line:
```toml
whitelist = ["YourHomeWiFi", "YourHomeWiFi-5G"]
```

Restart the daemon: `sudo systemctl restart rusty-oxigotchi`

## 6. Take Your First Walk

Unplug the USB cable, grab your Pi, and go for a walk. The bull is already scanning and attacking in RAGE mode (level 1 — gentle by default).

**What's happening:**
- The bull scans WiFi channels and discovers access points
- It sends deauth frames to force devices to reconnect
- When devices reconnect, it captures WPA handshakes
- Handshakes are saved to the SD card automatically

**What you'll see on the e-ink display:**
- The bull's face changes based on mood (captures = happy, dry spell = bored)
- Stats update: APs seen, handshakes captured, current channel
- Battery level from the PiSugar

## 7. Check Your Captures

After your walk, plug back in and open the dashboard. The **Recent Captures** card shows your handshakes. You can:

- **Download** individual .pcapng or .22000 files
- **Crack** them locally with hashcat: `hashcat -m 22000 capture.22000 wordlist.txt`
- **Upload** to [WPA-SEC](https://wpa-sec.stanev.org) for free cloud cracking (set your API key in the WPA-SEC card)

## 8. Set Up Phone Tethering (Optional)

For untethered walks with internet access (auto-upload captures, Discord notifications, SSH from your phone):

1. Open the dashboard → **Phone Tethering** card
2. Tap **Scan for Devices**
3. Select your phone and tap **Pair**
4. Confirm the passkey on both devices
5. On your phone, enable **Bluetooth tethering** (Settings → Network → Bluetooth tethering)

The Pi auto-connects to your phone on every boot. You can access the dashboard from your phone's browser at the BT PAN IP.

## 9. Crank Up the Aggression

The default RAGE level is 1 (Chill) — safe and quiet. Once you're comfortable, slide it up:

| Level | Name | What Changes |
|-------|------|-------------|
| 1 | Chill | Rate 1, 3 channels — minimal |
| 2 | Lurk | Full 11 channels |
| 3 | Prowl | Rate 2 — more deauths |
| 4 | Hunt | Faster channel hopping |
| 5 | RAGE | Rate 3 — aggressive |
| 6 | FURY | Fast dwell — max validated |
| 7 | YOLO | Rate 5 — may crash, auto-recovers |

Levels 1-6 are all stable, even with BT phone tethering active.

## PiSugar3 Buttons

The PiSugar 3 has two buttons on the edge and a reset button near the magnets. The **power button** handles power on/off. The **custom button** is mapped to quick-access actions:

| Gesture | Action | Details |
|---------|--------|---------|
| **Single tap** | Cycle rage level | Rotates through levels 1-6 (Chill → FURY). Skips level 7 (YOLO) for stability. |
| **Double tap** | Toggle BT tethering | Connect or disconnect Bluetooth PAN to your phone. |
| **Long press** | Toggle RAGE ↔ SAFE | Switch between attack mode and safe/upload mode. |

**Power button** (the other edge button):
- **Long press** — Clean shutdown with e-ink confirmation. Hardware cuts power after 30 seconds as a safety net.
- **Plugging in USB-C** — Auto-boots the Pi (even from fully off).

All actions are also available from the web dashboard.

## Next Steps

- **[Web Dashboard](Web-Dashboard)** — Full guide to all 26 dashboard cards
- **[Bull Faces](Bull-Faces)** — Learn what each face means
- **[Bluetooth Mode](Bluetooth)** — Switch to BT attack mode for Bluetooth pentesting
- **[Troubleshooting](Troubleshooting-and-FAQ)** — Common issues and fixes
