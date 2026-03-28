# WiFi Firmware Reverse Engineering

← [Back to Wiki Home](Home)

---

## The Problem with Stock Pwnagotchi

Stock pwnagotchi on a Pi Zero 2W is barely functional. Here's what's actually happening under the hood:

The BCM43436B0 WiFi chip was never designed for packet injection. The nexmon patch that enables monitor mode is essentially duct tape — it forces the firmware into a state Broadcom never intended, and the chip fights back constantly. The PSM (Power Save Mode) watchdog fires every few seconds under injection load, the DPC (Deferred Procedure Call) handler panics when frame queues overflow, and memcpy operations trigger hard faults when the SDIO bus can't keep up. The result: **your WiFi module crashes every 2-5 minutes**. Bettercap tries to send a deauth frame, the firmware panics, the SDIO bus dies, wlan0mon disappears, pwnagotchi restarts, and the cycle repeats.

Most people's pwnagotchis spend more time recovering from crashes than actually capturing handshakes. It looks like a cute hacking toy on the outside, but when you dig into the logs, it's barely working — limping along with constant firmware resets, missing most handshakes because the radio is dead half the time.

And it's not just the WiFi. The crash cascade causes a chain of secondary problems:

- **SSH drops constantly** — You're SSH'd in trying to debug something, the firmware crashes, pwnagotchi restarts, your SSH session dies. Reconnect, wait for boot, crash again. Repeat.
- **`monstop` reloads the entire driver** — Every time pwnagotchi restarts, it calls `modprobe -r brcmfmac && modprobe brcmfmac`, which re-enumerates the SDIO bus. Do this enough times in quick succession and the SDIO bus dies permanently — only a full power cycle recovers it.
- **Restart storms kill the SD card** — Pwnagotchi has `Restart=always` in systemd with no rate limit. Crash → restart → crash → restart, over and over, writing logs and thrashing the SD card each time.
- **Boot takes forever** — On every restart, pwnagotchi re-parses its entire log file backwards using `FileReadBackwards`. With a 10MB log, this takes 30-60 seconds of pure I/O on the slow SD card. Every crash costs you a minute of downtime.
- **Bettercap eats memory** — Written in Go, bettercap uses ~80MB of RAM on a Pi Zero 2W that only has 512MB total. Combined with pwnagotchi's Python, you're constantly near memory pressure.
- **Captures are often junk** — Bettercap saves raw pcap files that may contain incomplete handshakes. You think you captured something, upload it to wpa-sec, and get nothing back. Community tools like `hashie-clean` and `pcap-convert-to-hashcat` exist specifically because this is such a common problem.
- **No real-time control** — Want to whitelist your home WiFi? Edit a TOML file over SSH. Want to see what networks are nearby? Check the tiny e-ink text. Want to download a capture? SCP it manually. The stock web UI shows a PNG of the e-ink display and a config editor. That's it.
- **The "AI" doesn't work** — The original pwnagotchi used reinforcement learning to optimize attacks. The jayofelony fork disabled it because it consumed too many resources and didn't actually improve capture rates. The mood faces that were supposed to reflect AI state just cycle randomly now.

On top of all that, bettercap only supports 2 attack types (deauth and PMKID), while modern tools like AngryOxide support 6 — including CSA, rogue M2, and anonymous reassociation that capture handshakes bettercap simply cannot get.

## What I Did About It

I reverse-engineered the BCM43436B0 firmware — mapped the ROM, found the crash handlers, traced the SDIO bus failures back to their root causes. I built an 8-layer firmware patch:

1. **PSM watchdog threshold** — raised from 5 to 255, preventing premature power-save panics
2. **DPC watchdog threshold** — same treatment, stops the deferred procedure handler from killing the radio
3. **RSSI threshold** — widened to prevent false signal-loss resets
4. **Fatal error wrapper** — intercepts error codes 5, 6, 7 at the firmware level and suppresses them instead of crashing
5. **HardFault recovery** — catches memcpy bus faults that previously killed the SDIO connection
6. **BCOL GTK rekey disable** — prevents a group key rotation that triggers a cascade failure under heavy TX load
7. **DWT watchpoint on wlc_fatal_error** — uses ARM Cortex-M3 hardware debug watchpoint to intercept ALL callers of the crash function, including 5 in read-only ROM that no software patch can reach. When any code — ROM or RAM — tries to crash the firmware, the watchpoint fires a Debug Monitor exception before the crash function executes, and our handler suppresses non-critical errors
8. **RSSI use-after-free fix** — patches the RSSI averaging function that caused rate-2 crashes. The original code read from a stale pointer after TX queue pressure freed the buffer. A NULL check prevents the fault entirely

This is the most thoroughly analyzed WiFi firmware patch for the BCM43436B0 in existence — built on a complete reverse engineering effort that mapped **6,965 functions**, reconstructed **313 fields** of the central WiFi controller structure, traced **24,328 cross-references**, and identified every crash path in the 1 MB firmware. The result: **27,982 injected frames in a 5-minute stress test, zero crashes.** The firmware that used to die every 2 minutes now runs indefinitely.

Then I integrated [AngryOxide](https://github.com/Ragnt/AngryOxide) — a Rust-based attack engine the community has been asking for. Nobody could get it running on the built-in WiFi because the firmware crashes were even worse under AO's heavier injection load. With the patched firmware, it runs flawlessly.

## Impact

**This firmware patch benefits everyone** — not just Oxigotchi users. If you want to keep using stock bettercap in PWN mode, the patched firmware makes that stable too. No more constant crashes and restarts.

The findings are being contributed back to the nexmon project so the broader community benefits. The patch layers are designed to be portable — the threshold adjustments and error wrappers apply to any BCM43436B0 deployment, not just pwnagotchi.

The DWT hardware watchpoint approach (Layer 7) is particularly significant: it's the only known method for intercepting crash paths in read-only ROM on the Cortex-M3 core inside the BCM43436B0. Software patches can only reach RAM-resident functions, but the DWT watchpoint fires on any access to the monitored address — including the 5 ROM callers that no firmware patch can modify.
