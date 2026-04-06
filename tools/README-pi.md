# Oxigotchi — Quick Reference

Rusty Oxigotchi is a single Rust binary that captures WPA handshakes via
AngryOxide and renders a personality-driven face on the e-ink display.
Everything runs as a systemd service.

---

## Shortcuts (symlinks in this directory)

| Symlink | Points to | What's inside |
|---------|-----------|---------------|
| `config/` | `/etc/oxigotchi/` | `config.toml`, `plugins/`, `faces/`, `handshakes/` |
| `plugins/` | `/etc/oxigotchi/plugins/` | Lua plugin files (.lua) |
| `faces/` | `/etc/oxigotchi/faces/` | E-ink face PNGs |
| `services/` | `/etc/systemd/system/` | All systemd unit files |
| `rusty-oxigotchi` | `/usr/local/bin/rusty-oxigotchi` | The daemon binary |

---

## Common Tasks

### Check status
```bash
systemctl status rusty-oxigotchi
journalctl -u rusty-oxigotchi -f        # live logs
journalctl -u rusty-oxigotchi -n 100    # last 100 lines
```

### Edit config
```bash
sudo nano ~/config/config.toml
sudo systemctl restart rusty-oxigotchi
```

### Add or edit a Lua plugin
```bash
sudo nano ~/plugins/my_plugin.lua
sudo systemctl restart rusty-oxigotchi
```

### Update the binary (from your PC)
```bash
scp oxigotchi pi@<IP>:/home/pi/oxigotchi
ssh pi@<IP> "sudo cp /home/pi/oxigotchi /usr/local/bin/rusty-oxigotchi && sudo systemctl restart rusty-oxigotchi"
```

### Switch operating mode
```bash
curl -X POST http://localhost:8080/api/mode -d '{"mode":"RAGE"}'   # WiFi capture
curl -X POST http://localhost:8080/api/mode -d '{"mode":"BT"}'     # Bluetooth mode
curl -X POST http://localhost:8080/api/mode -d '{"mode":"SAFE"}'   # BT tether only
```

### Web dashboard
Open `http://<Pi-IP>:8080` in your browser (Pi must be connected via RNDIS or WiFi).

### View captured handshakes
```bash
ls ~/config/handshakes/
ls ~/captures/
```

---

## Key File Locations

| File | Purpose |
|------|---------|
| `/etc/oxigotchi/config.toml` | Main config (mode, BT device, rage level, etc.) |
| `/etc/oxigotchi/plugins/*.lua` | Lua plugins for e-ink indicators |
| `/etc/oxigotchi/faces/*.png` | E-ink face images |
| `/usr/local/bin/rusty-oxigotchi` | Daemon binary |
| `/etc/systemd/system/rusty-oxigotchi.service` | Systemd unit |
| `/home/pi/captures/` | Recent capture staging |
| `/home/pi/exp_stats.json` | Saved XP/level/mood state |
| `/home/pi/.wpa_sec_db` | WPA-sec upload tracking |

---

## Troubleshooting

**E-ink shows nothing:** `sudo systemctl restart rusty-oxigotchi`

**WiFi interface gone:** `sudo systemctl restart wifi-recovery`

**BT tether drops:** Normal reconnect cycle — check `journalctl -u rusty-oxigotchi -n 50`

**SSH lost:** Pi broadcasts mDNS as `oxigotchi.local` — try `ssh pi@oxigotchi.local`
