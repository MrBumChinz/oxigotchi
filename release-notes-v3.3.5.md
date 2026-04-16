Reliability, security, and hardware-safety fix release. No new features — every change here fixes a real bug or closes a real attack surface.

---

## Bluetooth tethering

### MIUI/Xiaomi "Invalid exchange" fix

Xiaomi (MIUI / HyperOS) devices were hitting BNEP rejection (errno 52) on every tether connect attempt, causing a tight reconnect loop and eventually destroying the BlueZ pairing database. Fixed by:

- EBADE classified as `BnepRejected` with a clear 5-step hint shown in the dashboard.
- `Device1.Disconnect` forced after rejection tears down the stale ACL link; phone auto-reconnects with a fresh BNEP context.
- `Network1.Disconnect` pre-cleanup before each `Connect` call clears stale BlueZ PAN plugin state.
- `ConnectProfile(NAP_UUID)` second-path fallback after EBADE.
- `DeviceReconnected` loop closed: the ACL signal fired by `Connect` itself no longer triggers an immediate reconnect that bypasses backoff.
- NAP UUID fallback for phones that hide the NAP profile until tethering is toggled active.

### ACL reconnect fix

**Symptom:** Phone's Bluetooth settings show `oxigotchi — Connected`, but the web dashboard says `Status: Disconnected` with no IP. Common on Samsung OneUI but not MIUI-specific.

**Root cause:** BlueZ automatically re-establishes the ACL link when a trusted device returns to range, but does *not* re-establish BNEP/PAN on top — that's the daemon's job. Previously the daemon only tried PAN reconnect on its health-check timer (up to 5 minutes later).

**Fix:** The D-Bus `PropertiesChanged` watcher now watches `Connected=true` transitions on paired devices. When BlueZ fires that event, the daemon immediately calls `Network1.Connect("nap")`.

### Carrier + internet handling

- Periodic internet re-probe (every 2 min) when PAN is up but the phone had no carrier at connect time — no longer stuck "BT OK / no internet".
- WPA-SEC upload gate skips attempts while internet is known offline; pending uploads resume automatically when connectivity returns.
- Transient radio-error hints are suppressed in the web UI (they fire constantly during normal reconnect churn).

### Phone compatibility

| Phone type | Primary path | Fallback |
|---|---|---|
| Android (Pixel, Samsung, OnePlus) | `Network1.Connect("nap")` | — |
| Xiaomi / MIUI / HyperOS | Primary with pre-cleanup, retry | `ConnectProfile(NAP_UUID)` on EBADE |
| iPhone / iPad | `ConnectProfile(NAP_UUID)` (iOS never advertised NAP in SDP) | — |

See the [BT-Tethering wiki](https://github.com/CoderFX/oxigotchi/wiki/BT-Tethering) for per-OS setup instructions and the iOS-specific section for Personal Hotspot caveats.

---

## Security fixes (codex audit)

Three high-severity findings from a full codebase audit:

### Shell injection in `ao.rs`

AngryOxide was launched via `script -qc "<command>"`. The command string was built by concatenating user-controlled values (interface name, output path, config path) directly into a shell string. A crafted interface name or path containing `"`, `` ` ``, or `$()` could execute arbitrary commands as root.

**Fix:** All arguments are POSIX single-quote wrapped — each token becomes `'<value>'` with any embedded single-quotes escaped as `'\''`. The shell sees the values as literals regardless of their content.

### Path traversal in face pack API

`POST /api/face_pack` accepted a `name` field that was used to construct a filesystem path (`/etc/oxigotchi/face_packs/<name>/`). A name like `../../etc/passwd` or `../systemd/system/` would resolve outside the face_packs directory.

**Fix:** The name field is now validated before use — rejects any value containing `..`, `/`, `\`, or control characters.

### Capture loss on copy failure

The capture move pipeline (tmpfs → SD card) previously deleted source files before verifying both the `.pcapng` and `.22000` copies succeeded. A mid-copy error (full disk, permission denied) left the source deleted and the destination incomplete — the handshake was lost.

**Fix:** Both copies must complete successfully before either source file is deleted (transactional move).

---

## WiFi recovery safety

Ends the "wlan0mon MAC all zeros / RSSI -100 everywhere" symptom. Both soft and hard recovery paths were taking actions that could not be undone without a physical power cycle (USB + battery unplug). Soft recovery is now a safe `iw`-only monitor restart; hard recovery surfaces **FwCrash** and logs that a power cycle is required instead of making the state worse.

- Old recovery helpers removed entirely so future code cannot accidentally re-introduce the unsafe operations.
- `GiveUp` log suppression: the recovery state machine emitted `GiveUp` on every health-check tick after exhaustion, spamming logs forever. Now emits once, then stays silent until WiFi returns to healthy.
- Release image no longer installs `wifi-recovery`, `fix-ndev`, or `wifi-watchdog` services — the Rust daemon supersedes them safely.

---

## Stability fixes

- **UTF-8 boundary panics:** three sites used byte-offset string slicing on SSID names and log lines. Any non-ASCII character (accented names, emoji SSIDs) caused a panic. Fixed with a `safe_truncate` helper that always splits on char boundaries.
- **Integer overflow in AP/attack tracking:** `hit_count` and `epochs_since_visit` used wrapping addition. On long-running sessions these would wrap to 0 and corrupt priority scoring. Fixed with `saturating_add`.
- **Stale SSID entry accumulation:** the SSID resolver's `scan_directory` now prunes entries for files that no longer exist, preventing unbounded map growth on long sessions.
- **Capture refresh skip:** `refresh_bssid_info` now skips files that already have metadata, so re-scanning a directory with 1000 captures doesn't re-stat every file every iteration.
- **Cleanup sort stability:** `cleanup()` now sorts captures by mtime (oldest-first eviction), not by arbitrary filesystem order.
- **Reset pairings:** simplified the reset flow in the web UI; clears error hint and state atomically instead of racing with a refresh-and-read chain.

---

## Dashboard

- **Discoverable toggle** description updated: *"Only needed to pair an additional phone — your already-paired phone reconnects automatically."*
- Transient BT radio errors no longer surface as scary user-facing hints.

---

## Wiki / Docs

- **BT Tethering page:** added "Adding a second phone" section explaining why your existing phone reconnects without discoverable on (cached MAC), and exact steps to pair a second phone. MIUI BNEP reset procedure added.
- **Building page:** rewritten with separate Windows/WSL and Linux paths, "Updating Without Reflashing" section at the top (binary swap — no reflash needed for minor bumps), and full image baking instructions.
- **PiSugar 3 button page:** new page documenting button mappings, the CTR2 latch fix, and the v3.3.1 MCU-register rewrite.
- **Troubleshooting:** new entry for the WiFi zombie-state symptom with manual recovery steps for users on older images.

---

## Upgrade — binary swap, no reflash

```bash
# On the Pi
curl -L -o /home/pi/oxigotchi https://github.com/CoderFX/oxigotchi/releases/latest/download/oxigotchi
sudo systemctl stop rusty-oxigotchi
sudo cp /home/pi/oxigotchi /usr/local/bin/rusty-oxigotchi
sudo chmod +x /usr/local/bin/rusty-oxigotchi
sudo systemctl disable --now wifi-recovery fix-ndev wifi-watchdog 2>/dev/null
sudo systemctl start rusty-oxigotchi
```

The `systemctl disable` line removes the three legacy services that were doing the unsafe WiFi recovery from outside the daemon. `Unit ... does not exist` errors are fine — means your image never had them.

No config changes required. All fixes are behavioural.

---

## Credits

- **MrBumChinz [BASI]** — found and reported three on-device issues: missing `wlan-keepalive` binary on deployed Pi (the keepalive for WiFi idle-crash prevention), duplicate `spi0-2cs` entries in `config.txt` from repeated bake runs, and `eth0` unconfigured on Ethernet HAT setups. Solid field testing.
- **Atomicek1234 [ABI]** — reported the "wlan0mon MAC all zeros / RSSI -100" symptom that led to the WiFi recovery rewrite.

---

## Verification

- All 1070 host tests pass
- Cross-compiled clean for `aarch64-unknown-linux-gnu`
- BT tether verified on-device (Xiaomi 13T) post-fix
- ACL reconnect verified on Pi Zero 2W: BT toggle on phone → bnep0 back within ~1 second
