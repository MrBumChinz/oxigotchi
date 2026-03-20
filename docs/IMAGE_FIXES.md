# Oxigotchi v2.0 Image Fixes

All fixes and self-healing mechanisms applied to the oxigotchi v2.0 SD card image.
These transforms are baked into the image — they run once during image prep, not on every boot.

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
| Swap configured to 100 MB at `/var/swap` | Small swap prevents OOM kills without thrashing the SD card |

---

## Display Fix

| Change | Why |
|--------|-----|
| Font changed from `"oxigotchi"` (missing) to `"DejaVuSansMono"` | The custom font file was never shipped; DejaVuSansMono is available system-wide and renders Korean face text correctly |
| pwnlib patched: `is_auto_mode()` forced to return `0` | Ensures pwnagotchi always runs in auto mode — prevents accidental manual mode on boot |

---

## Windows Side

| File | Purpose |
|------|---------|
| `tools/setup_rndis_ip.ps1` | PowerShell script that ensures the Windows RNDIS adapter has `10.0.0.1/24` assigned, so the Pi is reachable at `10.0.0.2` over USB |
