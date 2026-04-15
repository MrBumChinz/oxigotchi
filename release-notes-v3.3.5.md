# v3.3.5

Reliability and security fix release. No new features — every change here fixes a real bug or closes a real attack surface.

---

## Bluetooth tether — ACL reconnect fix

**Symptom:** Phone's Bluetooth settings show `oxigotchi — Connected`, but the web dashboard says `Status: Disconnected` or no IP. Reported on Samsung OneUI; can affect any Android.

**Root cause:** BlueZ automatically re-establishes the ACL (radio) link when a trusted device comes back into range (e.g. after phone BT toggle or walking back in range). But BlueZ does *not* re-establish the BNEP/PAN profile on top of that ACL link — that's the daemon's job. Previously the daemon only tried PAN reconnect on its health-check timer (up to 5 minutes later). So the ACL was up, the phone thought it was connected, but bnep0 never came back.

**Fix:** The D-Bus `PropertiesChanged` watcher now also watches `Connected=true` transitions on paired devices. When BlueZ fires that event (ACL up), the daemon immediately calls `Network1.Connect("nap")` instead of waiting for the next health check. If the PAN connect fails (phone not ready yet), `retry_count` is reset to 0 so the next health check retries without backoff.

---

## Security fixes (codex audit)

Three high-severity findings from a full codebase audit:

### Shell injection in ao.rs (CVE-class)

AngryOxide is launched via `script -qc "<command>"`. The command string was built by concatenating user-controlled values (interface name, output path, config path) directly into a shell string. A crafted interface name or path containing `"`, `` ` ``, or `$()` could execute arbitrary commands as root.

**Fix:** All arguments are POSIX single-quote wrapped — each token becomes `'<value>'` with any embedded single-quotes escaped as `'\''`. The shell sees the values as literals regardless of their content.

### Path traversal in face pack API

`POST /api/face_pack` accepted a `name` field that was used to construct a filesystem path (`/etc/oxigotchi/face_packs/<name>/`). A name like `../../etc/passwd` or `../systemd/system/` would resolve outside the face_packs directory.

**Fix:** The name field is now validated before use — rejects any value containing `..`, `/`, `\`, or control characters.

### Capture loss on copy failure

The capture move pipeline (tmpfs → SD card) previously deleted source files before verifying both the `.pcapng` and `.22000` copies succeeded. A mid-copy error (full disk, permission denied) left the source deleted and the destination incomplete — the handshake was lost.

**Fix:** Both copies must complete successfully before either source file is deleted (transactional move). If either copy fails, both source files are kept for the next epoch's retry.

---

## Stability fixes

- **UTF-8 boundary panics:** Three sites used byte-offset string slicing on SSID names and log lines. Any non-ASCII character (accented names, emoji SSIDs) caused a panic. Fixed with a `safe_truncate` helper that always splits on char boundaries.
- **Integer overflow in AP/attack tracking:** `hit_count` and `epochs_since_visit` used wrapping addition. On long-running sessions these would wrap to 0 and corrupt priority scoring. Fixed with `saturating_add`.
- **Stale SSID entry accumulation:** The SSID resolver's `scan_directory` now prunes entries for files that no longer exist, preventing unbounded map growth on long sessions.
- **Capture refresh skip:** `refresh_bssid_info` now skips files that already have metadata, so re-scanning a directory with 1000 captures doesn't re-stat every file every epoch.
- **Cleanup sort stability:** `cleanup()` now sorts captures by mtime (oldest-first eviction), not by arbitrary filesystem order.

---

## Dashboard

- **Discoverable toggle** description updated: *"Only needed to pair an additional phone — your already-paired phone reconnects automatically."* The toggle still works the same way; the label now accurately describes when you actually need it.

---

## Wiki

- **BT Tethering page:** Added "Adding a second phone" section explaining why your existing phone reconnects without discoverable on (it uses the cached MAC), and the exact steps to pair a second phone. FAQ updated to reference the new section.
- **Building page:** Rewritten with separate Windows/WSL and Linux paths, "Updating Without Reflashing" section at the top (binary swap — no reflash needed for minor bumps), and full image baking instructions.

---

## Upgrade notes

Binary swap — no reflash needed:

```bash
# On the Pi
curl -L -o /home/pi/oxigotchi https://github.com/CoderFX/oxigotchi/releases/latest/download/oxigotchi
sudo systemctl stop rusty-oxigotchi
sudo cp /home/pi/oxigotchi /usr/local/bin/rusty-oxigotchi
sudo chmod +x /usr/local/bin/rusty-oxigotchi
sudo systemctl start rusty-oxigotchi
```

No config changes required. All fixes are behavioural.

---

## Verification

- All host tests pass
- Cross-compiled clean for `aarch64-unknown-linux-gnu`
- ACL reconnect fix verified on Pi Zero 2W: BT toggle on phone → bnep0 back within ~1 second
