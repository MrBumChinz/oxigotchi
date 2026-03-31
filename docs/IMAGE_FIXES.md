# Oxigotchi v2.0 Image Fixes

All fixes and self-healing mechanisms applied to the Oxigotchi v2.0 SD card image.
These transforms are baked into the image — they run once during image prep, not on every boot.

> **Note:** Most of these fixes exist to work around Python/pwnagotchi/bettercap quirks. **Rusty Oxigotchi v3.0** eliminates the root causes entirely — no Python venv to manage, no bettercap to fight with, no pwnagotchi service to rate-limit. The Rusty image will be drastically simpler: flash a ~2GB image, boot in under 5 seconds, done.

---

## Boot Partition Fixes

| File | Change | Why |
|------|--------|-----|
| `cmdline.txt` | Added `modules-load=dwc2,g_ether` | Load USB gadget modules at boot for RNDIS networking |
| `cmdline.txt` | Added `panic=10` | Auto-reboot 10 seconds after kernel panic instead of hanging |
| `config.txt` | Added `dtparam=watchdog=on` in `[all]` section | Enable hardware watchdog so the Pi reboots if the system hangs |
| `ssh` | Created empty file | Enable SSH server on first boot (Raspberry Pi OS convention) |

---

## System Fixes

| Change | Why |
|--------|-----|
| Default target changed from `graphical.target` to `multi-user.target` | No desktop environment needed — saves RAM and boot time |
| Cloud-init disabled | Prevents slow first-boot provisioning that blocks SSH access |
| SSH password auth enabled via `/etc/ssh/sshd_config.d/99-oxigotchi.conf` | Raspberry Pi OS defaults to key-only auth; password auth needed for initial setup |
| SSH host keys pre-generated | Eliminates first-boot delay waiting for key generation |
| Emergency SSH service installed | Fallback SSH daemon if the main `sshd` fails to start |
| Hostname set to `oxigotchi` | Identifies the device on the network and in logs |

---

## Network Fixes

| Change | Why |
|--------|-----|
| NM USB Gadget (shared) set to dual static IPs: `10.0.0.2/24` and `192.168.137.2/24` | Reachable from both Linux (`10.0.0.1`) and Windows RNDIS (`192.168.137.1`) hosts |
| NM USB Gadget (client) connection disabled | Prevents NetworkManager from fighting over `usb0` with a second profile |
| systemd-networkd `usb0` config removed | NetworkManager handles `usb0` — systemd-networkd config would conflict |
| `usb0-ip.service` disabled | Redundant with NM managing `usb0`; would race with NM on boot |
| `usb0-fallback.service` added as safety net | If NM fails to bring up `usb0`, this service sets the static IP as a last resort |
| `/etc/modules-load.d/modules.conf` | `i2c-dev` and `g_ether` on separate lines (fixes parse issue with single-line format) |

---

## Service Cleanup

| Service | Action | Why |
|---------|--------|-----|
| `ModemManager` | Disabled | No cellular modem — wastes CPU probing USB devices |
| `userconfig` | Disabled | Suppresses the first-boot console setup wizard |
| `pi-helper` | Disabled | Not needed for headless operation |
| `rpi-eeprom-update` | Disabled | Zero 2W has no updatable EEPROM — service errors on boot |
| `apt-daily.timer` / `apt-daily-upgrade.timer` | Disabled | Prevents unattended apt runs that spike CPU and wear the SD card |
| Old `oxagotchi-splash` typo files | Removed | Leftover from a previous naming; caused confusing duplicate services |
| Splash service unit | Fixed `ExecStart` to reference `oxigotchi-splash.py` | Was pointing at the old typo name; splash would fail silently |
| Pwnagotchi drop-in | Fixed `After=` / `Wants=` to reference `oxigotchi-splash.service` | Ensures correct boot ordering with the renamed splash service |

---

## Self-Healing

These mechanisms keep the Pi recoverable without physical access.

| Mechanism | How It Works |
|-----------|-------------|
| **Hardware watchdog** | `dtparam=watchdog=on` + `/etc/watchdog.conf` — if the kernel stops petting the watchdog, the hardware resets the SoC |
| **Kernel panic auto-reboot** | `panic=10` on cmdline — kernel reboots 10 seconds after a panic instead of freezing |
| **bootlog.service** | Writes boot diagnostics to `/boot/firmware/bootlog.txt` (readable from any PC via SD card); auto-restarts SSH if it failed |
| **emergency-ssh.service** | Standalone fallback SSH daemon on a different config — starts if the main `sshd` is down |
| **usb0-fallback.service** | Runs `ip addr add` on `usb0` if NetworkManager failed to assign the static IP |
| **resize-rootfs.service** | Auto-expands the root filesystem to fill the SD card on first boot |

---

## Disk Cleanup

| Item Removed | Space Saved |
|-------------|-------------|
| Rust toolchain (`~/.rustup`, `~/.cargo`) | ~2 GB |
| Go cache (`~/go`) | ~500 MB |
| `.vscode-server` cache | ~200 MB |
| Old swapfile | ~100 MB |
| Backup tar archive | ~500 MB |
| Kismet capture file | ~200 MB |

**Total savings:** ~4 GB (11 GB down to 7.3 GB used)

| Change | Why |
|--------|-----|
| `/tmp` mounted as `tmpfs` | Reduces SD card write wear — temp files live in RAM |
| Swap configured to 100 MB at `/var/swap` | Small swap prevents OOM kills without thrashing the SD card (Rusty won't need swap at all — ~10MB RSS) |

---

## Display Fix

| Change | Why |
|--------|-----|
| Font changed from `"oxigotchi"` (missing) to `"DejaVuSansMono"` | The custom font file was never shipped; DejaVuSansMono is available system-wide and renders Korean face text correctly (Rusty will use `embedded-graphics` fonts — no system font dependency) |
| pwnlib patched: `is_auto_mode()` forced to return `0` | Ensures pwnagotchi always runs in auto mode — prevents accidental manual mode on boot (Rusty has no manual mode concept — always scanning) |

---

## Sprint Fixes (2026-03-21)

All fixes applied in the v2.1 sprint, baked into the image via `tools/bake_v2.sh`.

### Blind Epoch Fix

| Change | Why |
|--------|-----|
| AO plugin feeds captures to pwnagotchi AI epoch tracker | In AO mode, bettercap doesn't see handshakes, so the AI reported blind epochs and negative rewards. The AO plugin now emits `association`, `deauth`, and `handshake` events for every AO capture, feeding the RL model accurate data. |
| Synthetic AP heartbeat injected when AP list empty | Prevents `blind_for` counter from incrementing to `mon_max_blind_epochs` and triggering a false restart. A dummy AP (`AO-active`) is injected into `_access_points` when the monitor interface is up. |

### Peer Error Fix

| Change | Why |
|--------|-----|
| `update_peers` AttributeError suppressed | The `'Array' object has no attribute 'read'` error was a bettercap/pwngrid API response format mismatch. Patched in agent.py to catch the error gracefully. Peer discovery is non-critical. |

### Capture Filename Prefix

| Change | Why |
|--------|-----|
| AO `--name` flag set to `hostname` (defaults to `oxigotchi`) | Captures were named `-DATETIME.pcapng` (empty prefix). Now named `oxigotchi-DATETIME.pcapng` for easy identification. Falls back to `oxigotchi` if hostname lookup fails. |

### Boot Time Optimization

| Change | Before | After | Why |
|--------|--------|-------|-----|
| `usb0-fallback.service` timeout reduced | 30.5s blocking | Non-blocking | Was the single biggest boot bottleneck |
| `fix-ndev.service` merged with wifi-recovery | 10.6s + 5.2s | ~5s combined | Two services both waiting for wlan0 |
| `bt-agent.service` race fixed | 7.5s (includes restart) | ~2s | Eliminated retry loop |
| `bootlog.service` made async | 4.7s blocking | Background | Diagnostic collection doesn't need to block boot |
| Disabled services | Various | Removed | ModemManager, rpi-eeprom-update, cloud-init, etc. |
| **Total boot time** | **~65s** | **~20s** | 3x faster boot |

### Kernel Module Blacklist

| Change | Why |
|--------|-----|
| `blacklist bcm2835_v4l2` in `/etc/modprobe.d/blacklist-camera.conf` | Prevents camera/video kernel modules from loading. Eliminates VCHI service initialization errors in the journal and saves RAM. |

### Handshake Directory Consolidation

| Change | Why |
|--------|-----|
| `/root/handshakes` symlinked to `/etc/pwnagotchi/handshakes/` | AO and bettercap were writing to different directories (17 vs 16 files, 179MB total). Single canonical directory eliminates duplication and confusion. |

### BT-Tether Decoupling

| Change | Why |
|--------|-----|
| Standalone bt-tether daemon, independent of pwnagotchi plugin | The pwnagotchi bt-tether plugin threw "Error with mac address" even when disabled. The standalone daemon handles Bluetooth tethering without any pwnagotchi dependency. Toggled via PiSugar button (single press). |
| bt-tether plugin disabled in config.toml | Eliminates the plugin load error. The standalone daemon handles all BT functionality. |

### WiFi Stability Services

| Service | Type | Purpose |
|---------|------|---------|
| `wlan-keepalive.service` | Persistent daemon | Native C binary (`wlan_keepalive`) sends probe frames every 100ms on wlan0mon to prevent BCM43436B0 SDIO bus idle crashes. Replaced the previous tcpdump-based approach. |
| `wifi-recovery.service` | Boot oneshot | GPIO power-cycles the BCM43436B0 via WL_REG_ON (GPIO 41) if wlan0 fails to appear within 4 seconds of boot. Recovers from SDIO bus death without full power cycle. |

### Image Builder

| Tool | Purpose |
|------|---------|
| `tools/bake_v2.sh` | Reproducible image build script. Mounts a base image via loopback, applies all 20 build steps (plugins, config, faces, tools, services, hostname, dual-IP, blacklists, cleanup), runs full verification, and unmounts. Produces a deterministic image from the repo. |

### Disabled Services

| Service | Why |
|---------|-----|
| `rpi-usb-gadget-ics` | Caused NetworkManager-dispatcher spam in logs |
| `ModemManager` | No cellular modem; wasted CPU probing USB |
| `systemd-networkd` | Conflicts with NetworkManager on usb0 |
| `usb0-ip` | Redundant with NM managing usb0 |
| `cloud-init` | Slow first-boot provisioning not needed |
| `userconfig` | First-boot console wizard not needed headless |
| `pi-helper` | Not needed for headless operation |
| `rpi-eeprom-update` | Zero 2W has no updatable EEPROM |

### Miscellaneous Fixes

| Change | Why |
|--------|-----|
| `/var/lib/.rootfs-expanded` sentinel created | Silences resize-rootfs.service failure on every boot |
| `chmod 644` on all service files | Fixes systemd executable permission warnings |
| `personality.associate = false`, `personality.deauth = false` in AO overlay | Prevents misleading "Associating to AP_NAME" status text when AO handles attacks |
| AO default rate set to 1 | Conservative default. All rates (1-3) stable with v6 firmware patch — stress tested 2026-03-26. |
| Whitelist: `["YourNetwork", "YourNetwork-5G"]` | Home network whitelist in both config.toml and angryoxide-v5.toml overlay |
| rpi-usb-gadget-ics disabled | Causes NM-dispatcher spam in logs |

---

## Windows Side

| File | Purpose |
|------|---------|
| `tools/setup_rndis_ip.ps1` | PowerShell script that ensures the Windows RNDIS adapter has `10.0.0.1/24` assigned, so the Pi is reachable at `10.0.0.2` over USB |
