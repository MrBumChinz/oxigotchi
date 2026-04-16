Follow-up patch release for v3.3.5. One fix, high-impact.

---

## BT tether internet fix

**Symptom:** BT tether pairs, PAN comes up, phone shows Connected, dashboard shows `BT OK`, `ping -I bnep0 8.8.8.8` succeeds — but general internet on the Pi doesn't work. `sudo apt update`, `curl`, any default-routed traffic silently fails.

**Root cause:** the Pi installs a default route via `usb0 → 10.0.0.1` (the USB gadget host) at metric 0. If the host has **not** configured Internet Connection Sharing (Windows ICS / macOS Internet Sharing / Linux NAT), that route is a black hole — packets get sent to the PC and dropped with no error. Because metric 0 is the highest priority, it wins over the BT tether route (metric 1005 from dhcpcd), so even a working BT tether can't deliver traffic.

Rob [IUIU] reported: *"Installed the update, and bamm I have BT connection. But still no internet tethering from phone (BT) or PC (USB)"*.

**Fix:** the usb0 default route is now installed with `metric 2000`, deliberately above the typical dhcpcd metric. Result:

- BT tether up + ICS off → BT wins automatically, internet works
- BT tether up + ICS on  → BT still wins (lower metric), internet works
- BT tether off + ICS on → usb0 kicks in as fallback, internet works
- BT tether off + ICS off → no internet (expected — neither path has upstream)

Previously only the last case behaved correctly. The middle two both left the Pi internetless despite having a valid upstream available.

---

## Upgrade — binary swap, no reflash

```bash
# On the Pi
curl -L -o /home/pi/oxigotchi https://github.com/CoderFX/oxigotchi/releases/latest/download/oxigotchi
sudo systemctl stop rusty-oxigotchi
sudo cp /home/pi/oxigotchi /usr/local/bin/rusty-oxigotchi
sudo chmod +x /usr/local/bin/rusty-oxigotchi
sudo systemctl start rusty-oxigotchi
```

On boot / usb0 probe, the daemon will replace the old metric-0 default route with the new metric-2000 one automatically. If you're already in the broken state, a one-shot manual fix is:

```bash
sudo ip route del default via 10.0.0.1 dev usb0
```

This restores internet until the new binary re-installs the correct route on next probe.

No config changes required.

---

## Credits

- **Rob [IUIU]** — reported the "BT connection works but no internet" symptom that led to the route-metric diagnosis.

---

## Verification

- All 1070 host tests pass
- Cross-compiled clean for `aarch64-unknown-linux-gnu`
- Verified on-device: `default via bnep0 metric 1005` wins over `default via usb0 metric 2000`, `ping 8.8.8.8` via default route returns in ~80ms, `curl https://www.google.com` returns HTTP 200.
