# Changelog

## [2.0.0] - 2026-03-15

### Added
- **AngryOxide plugin v2.0** (`angryoxide_v2.py`): complete rewrite of the pwnagotchi plugin with 22 API endpoints (9 GET, 13 POST) and a full web dashboard
- **Web dashboard**: mobile-friendly dark-theme control panel with live auto-refresh (5s status, 10s AP list, 30s logs), served at the plugin's webhook root
- **28 bull face PNGs** for e-ink display in `faces/eink/`: awake, angry, bored, broken, cool, debug, demotivated, excited, friend, fw_crash, ao_crashed, battery_low, battery_critical, grateful, happy, intense, lonely, look_l, look_l_happy, look_r, look_r_happy, motivated, sad, shutdown, sleep, smart, upload, wifi_down
- **Mode switcher** (`pwnoxide-mode.sh`): switch between AO mode (AngryOxide + bull faces) and PWN mode (stock bettercap) with watchdog and firmware rollback support
- **Boot/shutdown splash service** (`Oxigotchi-splash.py` + `Oxigotchi-splash.service`): systemd unit that displays bull face on e-ink at boot and shutdown
- **Smart Skip toggle**: auto-whitelist APs that already have captured handshakes, skipping them to focus on new targets
- **Capture file downloads**: individual capture download via `/api/download/capture/:filename` and bulk ZIP download via `/api/download/all`, covering both AO and bettercap handshake directories
- **Discord notifications**: POST `/api/discord-webhook` to configure a Discord webhook URL for capture alerts
- **GPS integration**: automatic `--gpsd` flag when gpsd is detected running on 127.0.0.1:2947
- **Session stats**: live capture count, capture rate per epoch, stable epoch counter, uptime tracking (formatted as Xm/Xh)
- **Log viewer**: `/api/logs` endpoint filters journalctl for angryoxide-related entries, displayed in a monospace log panel on the dashboard
- **Capture type detection**: heuristic classification of captures as PMKID (< 2KB) vs 4-way handshake based on file size
- **AP targeting from dashboard**: nearby networks table sorted by RSSI with one-click "target" button per AP; `/api/targets/add` and `/api/targets/remove` endpoints
- **Whitelist table**: combined view of AO plugin whitelist entries and config.toml whitelist, with MAC/SSID display, source labels, and per-entry remove buttons
- **BT keepalive timer**: UI update cycle suppresses bt-tether status text and overlapping plugin UI elements to keep AO display clean
- **Attack type toggles**: 6 individual attack types (deauth, PMKID, CSA, disassociation, anon reassoc, rogue M2) controllable via dashboard switches with per-toggle descriptions
- **Attack rate control**: 3-level rate selector (quiet/normal/aggressive) with immediate AO restart on change
- **Channel configuration**: custom channel list, autohunt mode, and dwell time (1-30s) slider with apply button
- **State persistence**: runtime config (targets, whitelist, rate, attacks, channels, autohunt, dwell, skip_captured) saved to JSON and restored on plugin load
- **Exponential crash backoff**: restart delay follows 5s * 2^(n-1) up to 300s cap, with automatic reset after 5 minutes of stability
- **Firmware crash recovery**: detects brcmfmac -110 channel set errors and firmware halt via regex on kernel logs, triggers modprobe cycle with interface polling
- **Battery monitoring**: reads PiSugar battery level with critical (< 15%) and low (< 20%) face/status overrides on epoch
- **13-step deployer** (`deploy_pwnoxide.py`): one-command SSH installer covering preflight, firmware backup, v5 firmware upload, angryoxide binary, plugin, config, mode switcher, set-iovars disable, WiFi stability fixes, face PNGs, splash service, verification with MD5 checksums, and reboot with post-boot validation
- **180 unit tests across 30 test classes** (`test_angryoxide.py`): command building, backoff math, capture parsing, whitelist normalization, uptime formatting, all webhook endpoints, health checks, firmware crash patterns, skip-captured logic, state persistence, file downloads, boot/shutdown faces, AP list with captured flags, mode API, face helper, battery level, name removal, UI updates, epoch edge cases, MAC extraction, and more
- **XSS prevention**: `esc()` and `escAttr()` helper functions in dashboard JavaScript to sanitize all user-supplied strings rendered in HTML

### Fixed
- **cache.py TypeError**: deployer patches pwnagotchi's cache.py to guard `isinstance(access_point, dict)` check, preventing TypeError when AO handshake objects are passed instead of dicts
- **WiFi crash on restart**: deployer patches `pwnlib` to comment out `reload_brcm` in `stop_monitor_interface`, preventing SDIO bus crash during bettercap restarts
- **bettercap-launcher crash loop**: deployer patches `bettercap-launcher` to make `reload_brcm` conditional (only runs if wlan0/wlan0mon are both missing), preventing unnecessary driver reloads that trigger firmware faults
- **Path traversal in capture download**: `/api/download/capture/` endpoint uses `os.path.basename()` to strip directory traversal attempts
- **Pwnagotchi restart storm**: deployer adds systemd rate-limit override (3 starts per 5 minutes) to prevent crash loops from exhausting the SD card

### Changed
- **Plugin architecture**: moved from v1 single-process model to v2 with thread-safe locking, process group management (SIGTERM then SIGKILL with timeout), and agent reference caching for webhook-triggered restarts
- **Handshake integration**: captures are now copied to bettercap's handshake directory and trigger `plugins.on('handshake')` events for downstream plugins (wigle, wpa-sec, pwncrack)
- **UI layout**: hides overlapping pwnagotchi UI elements (name, walkby, blitz, bluetooth, display-password, ip_display) when AO is active, overrides bt-tether status text with AO capture count
- **Face system**: PNG-first with text fallback -- checks `/etc/pwnagotchi/custom-plugins/faces/` for PNG files, falls back to stock text faces (ANGRY, BROKEN, etc.) if not found
- **Deployer renamed**: `deploy_pwnoxide.py` replaces earlier single-purpose deployers (deploy_and_patch, deploy_minimal, deploy_fatal_wrapper, etc.) as the canonical installer
- **set-iovars service**: disabled by deployer as obsolete for v5 firmware
