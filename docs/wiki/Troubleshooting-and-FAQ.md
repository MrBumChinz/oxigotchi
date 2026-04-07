# Troubleshooting & FAQ

← [Back to Wiki Home](Home)

---

## FAQ

**Does this work on Pi 4 / Pi 3 / Pi Zero W / Pi 5?**
No. The firmware patches are for the BCM43436B0 chip in the Pi Zero 2W only. Other Pi models have different chips. No workaround exists.

**Can I write plugins?**
Yes. Oxigotchi v3 uses Lua 5.4 plugins. Place `.lua` files in `/etc/oxigotchi/plugins/`. Plugins can register indicators on the e-ink display and react to epoch, handshake, crash, and BT events. See [docs/RUSTY_V3.md](https://github.com/CoderFX/oxigotchi/blob/main/docs/RUSTY_V3.md) for the full plugin API.

**Is `sudo apt update && sudo apt upgrade -y` safe?**
Yes. The dangerous packages are held and won't upgrade. See the "Safe apt Upgrades" section below.

**Can I switch back to stock pwnagotchi?**
The legacy pwnagotchi and bettercap services are disabled on first boot. You can re-enable them with `systemctl enable pwnagotchi bettercap`, but the Rust daemon is designed to fully replace them. To fully remove the firmware patch: `sudo pwnoxide-mode rollback-fw`.

**Is this legal?**
These are WiFi security auditing tools for testing your own networks or networks you have explicit permission to test. Use responsibly.

**Are my captures actually crackable?**
Yes — AO validates every capture before saving. No junk pcaps. Every `.pcapng` has a matching `.22000` hashcat-ready file. No need for `hashie-clean` or `pcap-convert-to-hashcat`.

**How do I set up WPA-SEC auto-cracking?**
Get a free API key from [wpa-sec.stanev.org](https://wpa-sec.stanev.org), paste it in the WPA-SEC card on the dashboard and hit Save. Captured handshakes upload automatically when internet is available (SAFE mode with BT tethering). Cracked passwords appear in the Cracked Passwords card.

**The e-ink display is blank or garbled.**
Make sure you have the **Waveshare 2.13" V4** (not V1/V2/V3 — they use different drivers). Check daemon logs: `journalctl -u rusty-oxigotchi | grep -i spi`

**How does XP and leveling work?**
Your bull earns XP passively (+1 per epoch, +1 per AP seen) and actively (+100 per handshake, +15 per association, +10 per deauth, +5 per new AP). The level formula is exponential: `XP needed = level^1.3 * 5`. Early levels fly by (Lv 1 needs 5 XP, Lv 10 needs 99 XP), but high levels are a serious grind (Lv 100 needs 1,990 XP, Lv 500 needs 16,129 XP, Lv 999 needs 39,664 XP per level). Max level is **999** — reaching it takes roughly **1 year** of daily use. Walk through busy areas for faster leveling (more APs = more XP). XP persists across reboots.

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

### E-ink Display

The daemon supports the **Waveshare 2.13" V4** only. Other versions (V1, V2, V3) use different controllers and will not work.

If the display is blank:
- Check SPI is enabled: `raspi-config` → Interface Options → SPI → Enable
- Check daemon logs: `journalctl -u rusty-oxigotchi | grep -i "spi\|display\|eink"`
- Verify the display is properly seated on the GPIO header
