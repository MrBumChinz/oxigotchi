# Oxigotchi Wiki

> A Rust-powered WiFi/BT attack tool for the Pi Zero 2W with an e-ink personality.

---

## New Here?

**[Getting Started](Getting-Started)** — Flash the image, connect to your Pi, take your first walk. 5 minutes from download to capturing handshakes.

---

## Reference

- **[Web Dashboard](Web-Dashboard)** — 26 live cards, REST API, mobile-friendly control panel
- **[Bull Faces Reference](Bull-Faces)** — All 28 faces with trigger conditions and personality logic
- **[Troubleshooting & FAQ](Troubleshooting-and-FAQ)** — Common issues, apt safety, XP system

## Deep Dives

- **[Architecture & Self-Healing](Architecture)** — Daemon design, main loop, crash recovery, module overview
- **[Bluetooth Pentest Mode](Bluetooth)** — BT attacks, UART multiplexing, phone tethering
- **[Capture Pipeline](Capture-Pipeline)** — tmpfs-based capture flow, hashcat conversion, SD card protection
- **[WiFi Firmware Patches](WiFi-Firmware)** — The 8-layer BCM43436B0 firmware patch that eliminated WiFi crashes
- **[RF Classification Pipeline](RF-Classification-Pipeline)** — Real-time 802.11 frame classification via VideoCore IV GPU and CPU
- **[Lua Plugins](Plugins)** — Write your own e-ink indicators in sandboxed Lua 5.4

## Development

- **[Building & Cross-Compilation](Building)** — Rust cross-compile for aarch64, Pi sysroot, deployment
