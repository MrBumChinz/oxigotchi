# Troubleshooting & FAQ

← [Back to Wiki Home](Home)

---

## FAQ

**Does this work on Pi 4 / Pi 3 / Pi Zero W / Pi 5?**
No. The firmware patches are for the BCM43436B0 chip in the Pi Zero 2W only. Other Pi models have different chips. No workaround exists.

**Can I write plugins?**
Yes. The bull runs Lua 5.4 plugins for every e-ink indicator. Drop a `.lua` file into `/etc/oxigotchi/plugins/`, add it to `plugins.toml`, and the dashboard Plugins card picks it up. See the [Lua Plugins](Plugins) wiki page for the API, state fields, and a working example.

**Is `sudo apt update && sudo apt upgrade -y` safe?**
Yes. The dangerous packages are held and won't upgrade. See the "Safe apt Upgrades" section below.

**Can I switch back to stock pwnagotchi?**
The legacy pwnagotchi and bettercap services are masked (not just disabled) in the release image — they cannot be re-enabled with `systemctl enable`. The Rust daemon fully replaces both. If you want stock pwnagotchi back, reflash a stock image.

**Is this legal?**
These are WiFi security auditing tools for testing your own networks or networks you have explicit permission to test. Use responsibly.

**Are my captures actually crackable?**
Yes — AO validates every capture before saving. No junk pcaps. Every `.pcapng` has a matching `.22000` hashcat-ready file. No need for `hashie-clean` or `pcap-convert-to-hashcat`.

**How do I set up WPA-SEC auto-cracking?**
Get a free API key from [wpa-sec.stanev.org](https://wpa-sec.stanev.org), paste it in the WPA-SEC card on the dashboard and hit Save. Captured handshakes upload automatically when internet is available (SAFE mode with BT tethering). Cracked passwords appear in the Cracked Passwords card.

**The e-ink display is blank or garbled.**
Make sure you have the **Waveshare 2.13" V4** (not V1/V2/V3 — they use different drivers and are not supported out of the box). See the [E-ink Display](#e-ink-display) section below for a sketch of what hypothetical V3 support would look like. Check daemon logs: `journalctl -u rusty-oxigotchi | grep -i spi`

**How does XP and leveling work?**
Your bull earns XP passively (+1 every 30 seconds) and actively (+100 per handshake, +2 per new AP, +1 per deauth, +1 per association). The level formula is quadratic: `XP needed = max(1, level² / 330)`. Early levels fly by (Lv 1-18 need just 1 XP each), mid-levels are moderate (Lv 100 needs 30 XP, Lv 500 needs 757 XP), but the endgame is a grind (Lv 999 needs 3,024 XP per level). Max level is **999**. Walk through busy areas for faster leveling (more APs = more XP). XP persists across reboots.

**Can I change the attack rate?**
The dashboard has a **RAGE Slider** with 7 levels (Chill through YOLO). Each level changes exactly one variable from the previous. Levels 1-6 are stress-test-validated stable, even with BT PAN active. Level 7 (YOLO) deliberately pushes past the tested-stable envelope — the daemon auto-recovers if AO crashes. In BT mode, an equivalent **BT aggression level** (BT:1/BT:2/BT:3) controls scanning and attack intensity.

**Does scanning more channels help?**
Yes. We use channels 1-11 (the legal 2.4 GHz set). Channels 1, 6, and 11 are where 95% of APs live, so they remain the best default for efficiency, but scanning all 11 is fully stable even at rate 3 with BT active. Autohunt mode (which scans all channels then locks onto active ones) is the recommended approach.

**How long does the battery last?**
With PiSugar 3 (1200mAh): 3-4 hours active. The bull face warns at 20% and 15%.

## Safe apt Upgrades

The following packages are held and won't upgrade:

| Held Package | Why |
|-------------|-----|
| `linux-image-*` | Kernel pinned to 6.12.62 (nexmon compatibility) |
| `firmware-brcm80211`, `firmware-nexmon` | Protects patched WiFi firmware |
| `brcmfmac-nexmon-dkms` | Prevents nexmon module rebuild |
| `libpcap-dev`, `libpcap0.8-dev` | AO dependency version lock |

A dpkg hook (`/etc/apt/apt.conf.d/99-protect-firmware`) backs up the patched firmware before any apt operation. If a package update somehow overwrites it, `verify-oxigotchi` auto-restores from the `.pre-apt` backup.

After any upgrade, run `sudo verify-oxigotchi` to confirm nothing broke. To see what's held: `apt-mark showhold`.

## Common Issues

### RNDIS Driver (Windows)

Windows needs a USB gadget driver to see the Pi as a network device over USB. Download and run [rpi-usb-gadget-driver-setup.exe](https://github.com/jayofelony/pwnagotchi/releases) before connecting. macOS and Linux don't need this.

If Windows shows "Unknown USB device":
1. Open Device Manager
2. Find the unknown device under "Other devices"
3. Right-click → Update driver → Browse → point to the downloaded driver
4. The Pi should appear as a RNDIS Ethernet device at `10.0.0.2`

### SSH Connection

The Pi is accessible at `10.0.0.2` over the USB data port (the micro USB port closest to the center, not the edge).

```bash
ssh pi@10.0.0.2
# Default password: raspberry
```

If SSH times out:
- Make sure you're using the **data** port (center), not the power-only port (edge)
- Wait 5-10 seconds after power-on for boot to complete
- Check that the RNDIS driver is installed (Windows)
- Try `ping 10.0.0.2` to verify network connectivity

### Does oxigotchi have internet when plugged in via USB?

**Not by default.** USB gadget mode gives the Pi a point-to-point link to your computer (`usb0` → `10.0.0.2` on macOS/Linux, or `192.168.137.2` on Windows RNDIS). That's enough for SSH and the web dashboard, but it isn't a route to the wider internet.

For the Pi to reach the internet via USB, **your computer has to share its own connection onto the USB adapter**:

- **Windows**: Enable Internet Connection Sharing (ICS) on your real network adapter (Ethernet or WiFi). Right-click the adapter → Properties → Sharing tab → "Allow other network users to connect through this computer's Internet connection" → pick the RNDIS/USB Ethernet adapter from the dropdown. Windows assigns the Pi an IP in `192.168.137.x` and routes traffic through NAT automatically.
- **macOS**: System Settings → General → Sharing → Internet Sharing → share your primary connection ("From") to the USB Ethernet adapter ("To computers using"). Turn on the switch. macOS handles NAT transparently.
- **Linux**: Manual iptables MASQUERADE + `sysctl -w net.ipv4.ip_forward=1`. See the Arch wiki's "Internet sharing" page for the exact rules; there's no GUI toggle on most distros.

Without any of those, USB is just SSH and dashboard access. The normal way oxigotchi gets internet in the field is **Bluetooth tethering to a phone** (see the [Bluetooth wiki page](Bluetooth)). That works stand-alone, no PC required.

### BT tether up but no internet (≤ v3.3.5 only)

**Symptom:** phone paired, dashboard shows `BT OK` with a valid IP on `bnep0`, `ping -I bnep0 8.8.8.8` succeeds — but `sudo apt update`, `curl`, and any default-routed traffic silently fail. `ip route show default` shows two default routes.

**Cause:** on pre-v3.3.6 images the Pi installs its `usb0` default route at metric 0 (highest priority). If the host has not configured Internet Connection Sharing, that route is a black hole — all default-routed packets go to the PC and are dropped with no error, even when BT tether is up.

**Fixed in v3.3.6:** the `usb0` default route is now installed with metric 2000, so BT tether (dhcpcd metric 1005) wins automatically.

**Quick fix on older images** (one-shot, clears until next reboot/probe):

```bash
sudo ip route del default via 10.0.0.1 dev usb0
```

After this, `ip route show default` should list only the `bnep0` default, and `ping 8.8.8.8` works without needing `-I bnep0`. Update to v3.3.6+ for a permanent fix.

### WiFi chip in a zombie state after a crash — the Broken face

**This section is the answer to "my bull shows the Broken face `(X_X)`"** — v3.3.6+ pins the message `ZOMBIE - UNPLUG USB+BATT 10s` on the e-ink status line when this state is reached. Follow the procedure below.

**Symptoms:**
- E-ink shows the **Broken face `(X_X)`** with status `ZOMBIE - UNPLUG USB+BATT 10s` (v3.3.6+)
- Or on older builds: `wlan0mon` exists but has MAC `00:00:00:00:00:00`, every AP shows RSSI `-100`, the dashboard reports "0 APs" forever, capture stops
- `dmesg` shows `mmc1: error -22 whilst initialising SDIO card`
- Happens after a firmware crash on older versions (≤ v3.3.4) or rarely after unusual crashes on v3.3.5+

**Why it used to happen:** previous recovery code ran `modprobe -r brcmfmac` or toggled the WiFi chip's power pin when the driver wedged. Neither is reversible on the Pi Zero 2W from software — the interface comes back but without a working radio, and only a true power cut restores it.

**Fixed in v3.3.5 / v3.3.6:** soft recovery no longer touches the kernel module, hard recovery no longer toggles the power pin. A true firmware crash surfaces the **Broken face** on the e-ink with the unmissable `ZOMBIE - UNPLUG USB+BATT 10s` sticky message. Three background shell services that did the same thing from outside the daemon (`wifi-recovery`, `fix-ndev`, `wifi-watchdog`) have been removed from the release image.

#### Recovery procedure (the Broken face is telling you to do this)

**A reboot will NOT work.** `sudo reboot` keeps PiSugar powering the Pi continuously — the WiFi chip never loses power, so its corrupted SDIO state survives. **You must physically cut all power.**

**Important:** PiSugar is a UPS. Unplugging USB alone leaves the battery powering everything. Tapping the PiSugar power button just triggers a graceful shutdown — the MCU stays alive and holds power ready to return. You need a **real** power cut.

**Step-by-step:**

1. **`sudo shutdown -h now`** — clean shutdown first (protects the SD card)
2. **Unplug USB** (data + power)
3. **Cut PiSugar power** — one of:
   - **Slide the hardware power switch on the PiSugar 3 HAT to OFF** (easiest if your version has the switch — look on the side of the HAT)
   - **Or disconnect the battery** by unplugging the small JST connector between the battery pack and the HAT
4. **Wait 30–60 seconds.** Not 10. The WiFi chip has internal decoupling capacitors that can hold state for 5–10s, and the corrupted SDIO bit stays set until they fully drain. 30s minimum is a safe margin.
5. **Reconnect in order:** battery / switch ON first, then USB
6. Wait for boot. Face should return to normal within 30–60s if the chip came back clean.

If the Broken face persists after a proper 30-60s power cut, the chip may be in a deeper state that needs either a longer soak (try 2–3 minutes) or a reflash.

#### Verify it worked

```bash
ssh pi@10.0.0.2
ip link show wlan0       # should show real MAC, not 00:00:00:00:00:00
dmesg | grep brcmfmac    # should NOT show 'error -22 whilst initialising SDIO card'
```

If `wlan0` exists with a real MAC, the chip is back. The daemon will clear the sticky zombie message and the face will return to normal automatically.

#### Prevent recurrence

- **Update to v3.3.6+** — binary-only update is enough, no reflash required. See [Building](Building).
- **If you can't update yet:** disable the three legacy services manually (they do the banned operations from shell):
   ```bash
   sudo systemctl disable --now wifi-recovery fix-ndev wifi-watchdog
   ```
   Some images never installed them; `Unit ... does not exist` errors are fine.

### E-ink Display

The daemon supports the **Waveshare 2.13" V4** only. Other versions (V1, V2, V3) use different controllers and will not work out of the box.

If the display is blank:
- Check SPI is enabled: `raspi-config` → Interface Options → SPI → Enable
- Check daemon logs: `journalctl -u rusty-oxigotchi | grep -i "spi\|display\|eink"`
- Verify the display is properly seated on the GPIO header

#### Hypothetical V3 Support

> **Note:** This section is hypothetical. The project maintainer does not own a Waveshare 2.13" V3 panel and cannot test this path. The code below does not exist yet — this is a rough sketch of what adding V3 would involve for anyone who wants to try it. Pull requests welcome from anyone with actual V3 hardware.

The V3 and V4 panels have the **same physical dimensions** (122×250 pixels), the **same SPI wiring**, and the **same GPIO pins** (BCM 17/25/8/24) — so hardware-wise they're interchangeable. The problem is that they use **different controllers**:

| Panel | Controller | Init | LUT | Full Refresh |
|-------|-----------|------|-----|-------------|
| V4 | SSD1680 | ~10 commands | Built-in (send 0xF7/0xFF) | ~2 seconds |
| V3 | SSD1675B | ~20 commands | Manual upload (70 bytes, separate full+partial tables) | ~15 seconds |

Adding V3 support would require a new driver file alongside the existing SSD1680 one:

1. **Create `rust/src/display/driver_v3.rs`** — port the `waveshare_epd/epd2in13_V3.py` init sequence, the two 70-byte LUT tables (`LUT_FULL_UPDATE` and `LUT_PARTIAL_UPDATE`), and the refresh functions to Rust. Mirror the structure of the existing `Ssd1680Driver`.
2. **Add a driver enum in `display/mod.rs`:**
   ```rust
   pub enum EpdDriver {
       V4(Ssd1680Driver<RppalHal>),
       V3(Ssd1675bDriver<RppalHal>),
   }
   ```
3. **Branch on `config.display_type`** when instantiating the driver. The `display_type = "waveshare_3"` value is already accepted by the config parser — it just has no driver behind it.
4. **Adjust the display refresh budget** — V3's 15-second full refresh will change the 180-second full-refresh gate in `display/mod.rs`. The partial refresh rhythm will also feel different.
5. **Test on real hardware** — all the timing, waveform, and ghosting behavior can only be validated on an actual V3 panel.

An alternative path is to replace the in-tree driver with the [`epd-waveshare`](https://crates.io/crates/epd-waveshare) crate, which supports both V3 and V4. That trades our hardened custom driver (180s full-refresh gate, ghosting recovery, BUSY timeout) for a third-party implementation — probably not worth it unless multiple panel versions need to be supported.
