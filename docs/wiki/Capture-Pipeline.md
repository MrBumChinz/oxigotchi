# Capture Pipeline

← [Back to Wiki Home](Home)

---

Captures flow through a RAM-based pipeline that protects the SD card from write wear:

```
AO (angryoxide)
 │  writes to /tmp/ao_captures/ (tmpfs, 150MB RAM)
 │  files: capture-TIMESTAMP.pcapng + capture.kismet
 ▼
Daemon (every loop cycle)
 │  1. Scans /tmp/ao_captures/ for new .pcapng files
 │  2. Converts to .22000 (hashcat-ready) via hcxpcapngtool
 │  3. Moves validated captures to /home/pi/captures/ (SD card)
 │  4. Deletes junk from tmpfs (failed conversions, empty captures)
 ▼
SD card: /home/pi/captures/
   capture-2026-03-24_12-00-00.pcapng  (original)
   capture-2026-03-24_12-00-00.22000   (hashcat-ready)
```

## Key Details

- `/tmp` is a 150MB tmpfs (RAM). AO's `.kismet` tracking file can grow to 20-60MB during long sessions, plus pcapng files at ~1-5MB each. The 150MB limit gives comfortable headroom.
- Only the last 30 seconds of captures are at risk on a sudden reboot — everything processed in a prior cycle is safe on SD.
- The `.22000` companion file is hashcat-ready. Every capture on SD has one.
- The dashboard's "Captures" card shows file count, handshake count, pending uploads, and total size. Individual files can be downloaded from the dashboard.

## /tmp Overflow Warning

If `/tmp` fills up, AO crashes because it can't write. The daemon detects this and restarts AO, but the real fix is to ensure `/tmp` doesn't fill. The 150MB default handles typical sessions (several hours). For marathon sessions (12+ hours), the `.kismet` file may grow large — the buffer-cleaner timer runs every 5 minutes and helps, but extremely long sessions in dense environments may need monitoring.

## WPA-SEC Auto-Upload

Captured handshakes can automatically upload to [wpa-sec.stanev.org](https://wpa-sec.stanev.org) for free cloud cracking.

Setup:
1. Get a free API key from wpa-sec
2. Paste the key in the WPA-SEC card on the web dashboard
3. Hit Save

Uploads happen automatically when internet is available (SAFE mode with BT tethering). Cracked passwords appear in the Cracked Passwords dashboard card.

## File Formats

| Extension | Format | Purpose |
|-----------|--------|---------|
| `.pcapng` | Packet capture (next-gen) | Original capture from AngryOxide, contains full 802.11 frames |
| `.22000` | Hashcat hash format | Extracted WPA handshake/PMKID, ready for hashcat cracking |
| `.kismet` | Kismet tracking DB | AO's internal tracking file for AP/client state (lives in tmpfs, not saved to SD) |
