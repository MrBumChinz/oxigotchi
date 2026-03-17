# Oxigotchi E-Ink Display Specification

**Hardware:** Waveshare 2.13" V4 (250×122 pixels, 1-bit, partial refresh)
**Orientation:** Landscape, 250px wide × 122px tall
**Colors:** Black (0xFF rendered) on white (0x00 rendered) — inverted internally

---

## Display Layout — Shared Elements

Both modes share the same hardware layout grid:

```
┌──────────────────────────────────────────────────────────┐
│ CH 00    APS 0 (00)                         UP 00:00:00  │  ← Top bar (Y=0..13)
├──────────────────────────────────────────────────────────┤  ← line1 (Y=14)
│                                                          │
│  [NAME]              [STATUS TEXT]                        │  ← Y=20 zone
│                                                          │
│  [FACE]                                                  │  ← Y=16 or Y=34 zone
│                                                          │
│                                                          │
│                                                          │
│  [FRIEND FACE + NAME]                                    │  ← Y=92..94
├──────────────────────────────────────────────────────────┤  ← line2 (Y=108)
│ PWND 0 (00)                                        AUTO  │  ← Bottom bar (Y=109+)
└──────────────────────────────────────────────────────────┘
```

### Element Positions (Waveshare 2.13" V4)

| Element | Position | Font | Notes |
|---------|----------|------|-------|
| channel | (0, 0) | Bold 10pt + Medium 10pt | "CH 00" |
| aps | (28, 0) | Bold 10pt + Medium 10pt | "APS 0 (00)" |
| uptime | (185, 0) | Bold 10pt + Medium 10pt | "UP HH:MM:SS" |
| line1 | Y=14, full width | — | Horizontal divider |
| name | (5, 20) | Bold 10pt | Mode-dependent (see below) |
| status | (125, 20) | Medium (custom font) | Wrapping text, max 20 chars/line |
| face | (0, Y) | Huge 35pt or PNG | Y depends on mode (see below) |
| friend_name | (0, 92) | BoldSmall 9pt | "▌▌▌│ PeerName 5 (12)" |
| line2 | Y=108, full width | — | Horizontal divider |
| shakes | (0, 109) | Bold 10pt + Medium 10pt | "PWND 0 (00)" |
| mode | (225, 109) | Bold 10pt | "AUTO" or "MANU" |

### Font Sizes (Waveshare V4 override)

```
fonts.setup(10, 9, 10, 35, 25, 9)
         Bold BoldSmall Medium Huge BoldBig Small
```

- **Huge** (35pt Bold): Face text in PWN mode
- **Bold** (10pt Bold): Name, labels, mode indicator
- **Medium** (10pt): Values, channel, APS, uptime
- **BoldSmall** (9pt Bold): Friend name
- **Small** (9pt): Plugin status elements (walkby, AO capture count)

---

## AO Mode (AngryOxide)

**Activated by:** `bettercap.disabled = true` in config overlay (`angryoxide-v5.toml`)
**Switched via:** `pwnoxide-mode ao`

### Boot Sequence

| Time | Display State | What Happens |
|------|--------------|--------------|
| T=0 | **Black screen** | Pi powers on, kernel loading |
| T=5-8s | **Bull AWAKE face (centered, full refresh)** | `oxigotchi-splash.service` runs before pwnagotchi. Renders `awake.png` centered on display via full EPD refresh. Writes to both RAM banks so image persists through partial refreshes. |
| T=8-11s | Bull face persists | `pwnagotchi-splash-delay.conf` adds 3s `ExecStartPre=/bin/sleep 3` before pwnagotchi starts. Splash stays visible. |
| T=11-15s | **Bull SLEEP face + "Initializing..."** | Pwnagotchi starts, `view.on_starting()` sets SLEEP face + version text. EPD partial refresh begins. |
| T=15-45s | **Bull SLEEP face + "Reading logs..."** | `LastSession.parse()` runs (or loads from cache in ~1s). Face = SLEEP or SMART. |
| T=45-60s | **Bull AWAKE face + "Ready"** | Monitor mode up, AO started, first epoch begins. |

### Steady State Display

```
┌──────────────────────────────────────────────────────────┐
│ CH *     APS 3 (12)                         UP 01:23:45  │
├──────────────────────────────────────────────────────────┤
│                      Sniffing around...                   │
│  ┌────────────┐                                          │
│  │            │                                          │
│  │  BULL PNG  │                                          │
│  │  (120×66)  │                                          │
│  │            │                                          │
│  └────────────┘                                          │
│                                                          │
├──────────────────────────────────────────────────────────┤
│ PWND 5 (23)                                        AUTO  │
└──────────────────────────────────────────────────────────┘
```

**Key differences from PWN mode:**
- **No name label** — the `name` element is empty (`''`), nothing renders at (5, 20)
- **No cursor blink** — cursor animation disabled
- **Face at Y=16** — 2px below line1, almost touching the top bar. Bull PNG gets more vertical space.
- **Face is PNG** — 120×66 pixel 1-bit bull head, rendered at (0, 16) via `canvas.paste()`
- **AO capture count** — plugin adds a LabeledValue showing capture count (bottom area)

### Face → Mood Mapping (AO Mode)

All faces are bull head PNGs at `/etc/pwnagotchi/custom-plugins/faces/`:

| Event | Face PNG | When |
|-------|----------|------|
| Starting | `awake.png` | Boot, initialization |
| Normal/Idle | `awake.png` | Default idle state |
| Sleeping | `sleep.png` | Between recon cycles |
| Looking (good mood) | `look_r_happy.png` / `look_l_happy.png` | Waiting, alternating L/R |
| Looking (neutral) | `look_r.png` / `look_l.png` | Waiting, alternating L/R |
| Association | `intense.png` | Sending PMKID assoc frame |
| Deauth | `cool.png` | Sending deauth frame |
| Handshake captured | `happy.png` | New handshake file detected |
| New peer | `awake.png` / `cool.png` / `friend.png` | Mesh peer discovered |
| Lost peer | `lonely.png` | Mesh peer lost |
| Good friend | `motivated.png` / `friend.png` / `happy.png` | Known peer with high bond |
| Bored | `bored.png` | No activity for bored_num_epochs |
| Sad | `sad.png` | No activity for sad_num_epochs |
| Angry | `angry.png` | Extended inactivity + no friends |
| Motivated | `motivated.png` | Positive reward trend |
| Demotivated | `demotivated.png` | Negative reward trend |
| Excited | `excited.png` | Sustained activity (excited_num_epochs) |
| Grateful | `grateful.png` | Sad/bored but has good friends |
| Smart | `smart.png` | Reading logs, free channel found |
| Uploading | `upload.png` | Uploading to wpa-sec |
| Rebooting | `broken.png` | System reboot triggered |
| Debug | `debug.png` | Custom debug message |
| Shutdown | `sleep.png` | Graceful shutdown |
| FW crash | `fw_crash.png` | AO plugin: firmware crash detected |
| AO crashed | `ao_crashed.png` | AO plugin: AO process died |
| Battery low | `battery_low.png` | Battery plugin: < 20% |
| Battery critical | `battery_critical.png` | Battery plugin: < 5% |
| WiFi down | `wifi_down.png` | Monitor interface disappeared |

### Bull Face PNG Specs

- **Size:** 120×66 pixels
- **Mode:** 1-bit grayscale (black and white only)
- **Format:** PNG, non-interlaced
- **Background:** White (transparent areas converted to white)
- **Rendering:** `Image.open()` → RGBA → alpha→white → colorize if inverted → convert to '1' → `canvas.paste()` at (0, 16)
- **28 faces total** covering all mood states + diagnostic states

### Shutdown Sequence (AO Mode)

| Time | Display State |
|------|--------------|
| T=0 | **Bull SLEEP face + "Zzz..."** | `view.on_shutdown()` called, display frozen |
| T=0-5s | Display frozen (no more updates) | Pwnagotchi stopping |
| T=5s | **Bull SHUTDOWN face (centered, full refresh)** | `oxigotchi-splash.service` ExecStop renders `shutdown.png` |
| T=5-10s | Shutdown face persists | System powering off, display retains last image |

---

## PWN Mode (Pwnagotchi / Bettercap)

**Activated by:** removing config overlay (no `bettercap.disabled` key)
**Switched via:** `pwnoxide-mode pwn`

### Boot Sequence

| Time | Display State | What Happens |
|------|--------------|--------------|
| T=0 | **Black screen** | Pi powers on, kernel loading |
| T=5-8s | **Nothing** | `oxigotchi-splash.service` detects no AO overlay → exits immediately. No splash shown. |
| T=8-15s | **Korean SLEEP face + "Pwnagotchi>" + "Initializing..."** | Pwnagotchi starts, `view.on_starting()`. EPD Clear() → fresh white canvas → partial refresh begins. |
| T=15-45s | **Korean SLEEP/SMART face + "Reading logs..."** | `LastSession.parse()` runs. |
| T=45-60s | **Korean AWAKE face + "Pwnagotchi>" + "Ready"** | Bettercap API ready, monitor mode up, first epoch. |

### Steady State Display

```
┌──────────────────────────────────────────────────────────┐
│ CH 06    APS 5 (18)                         UP 00:45:12  │
├──────────────────────────────────────────────────────────┤
│  Pwnagotchi> █        Sniffing around...                 │
│                                                          │
│  (◕‿‿◕)                                                │
│                                                          │
│                                                          │
│                                                          │
│  ▌▌▌│ buddy 3 (15)                                      │
├──────────────────────────────────────────────────────────┤
│ PWND 3 (18)                                        AUTO  │
└──────────────────────────────────────────────────────────┘
```

**Key differences from AO mode:**
- **Name label visible** — "Pwnagotchi>" at (5, 20), Bold 10pt, with blinking cursor (█)
- **Cursor blinks** — `_refresh_handler` toggles "█" suffix on name at ui.fps rate
- **Face at Y=34** — below the name, leaving 2px gap (name ends ~Y=32)
- **Face is Korean text** — Unicode emoticons rendered with Huge font (35pt Bold DejaVuSansMono)
- **PNG mode OFF** — `ui.faces.png = false`, all face values are strings like `(◕‿‿◕)`
- **No AO plugin UI elements** — no capture count, no AO status

### Face → Mood Mapping (PWN Mode)

All faces are Korean Unicode text rendered with Huge 35pt font:

| Event | Face Text | Variants |
|-------|-----------|----------|
| Starting | `(◕‿‿◕)` | AWAKE |
| Normal/Idle | `(◕‿‿◕)` | AWAKE |
| Sleeping | `(⇀‿‿↼)` | `(≖‿‿≖)`, `(－_－)` |
| Looking R | `( ⚆_⚆)` | |
| Looking L | `(☉_☉ )` | |
| Looking R happy | `( ◕‿◕)` | `( ≧◡≦)` |
| Looking L happy | `(◕‿◕ )` | `(≧◡≦ )` |
| Association | `(°▃▃°)` | `(°ロ°)` — INTENSE |
| Deauth | `(⌐■_■)` | COOL |
| Handshake | `(•‿‿•)` | `(^‿‿^)`, `(^◡◡^)` — HAPPY |
| New peer (first) | AWAKE or COOL | Random |
| New peer (friend) | MOTIVATED/FRIEND/HAPPY | Random |
| New peer (normal) | EXCITED/HAPPY/SMART | Random |
| Lost peer | `(ب__ب)` | `(｡•́︿•̀｡)`, `(︶︹︺)` — LONELY |
| Bored | `(-__-)` | `(—__—)` |
| Sad | `(╥☁╥ )` | `(╥﹏╥)`, `(ಥ﹏ಥ)` |
| Angry | `(-_-')` | `(⇀__⇀)`, `` (`___´) `` |
| Motivated | `(☼‿‿☼)` | `(★‿★)`, `(•̀ᴗ•́)` |
| Demotivated | `(≖__≖)` | `(￣ヘ￣)`, `(¬､¬)` |
| Excited | `(ᵔ◡◡ᵔ)` | `(✜‿‿✜)` |
| Grateful | `(^‿‿^)` | |
| Smart | `(✜‿‿✜)` | |
| Friend | `(♥‿‿♥)` | `(♡‿‿♡)`, `(♥‿♥ )`, `(♥ω♥ )` |
| Uploading | `(1__0)` | `(1__1)`, `(0__1)` |
| Rebooting | `(☓‿‿☓)` | BROKEN |
| Debug | `(#__#)` | |
| Shutdown | `(⇀‿‿↼)` | SLEEP |

### Shutdown Sequence (PWN Mode)

| Time | Display State |
|------|--------------|
| T=0 | **Korean SLEEP face + "Zzz..."** | `view.on_shutdown()`, display frozen |
| T=0-5s | Display frozen | Pwnagotchi stopping |
| T=5s | **Nothing new** | Splash service exits (no AO overlay). Display retains last Korean face. |
| T=5-10s | Korean face persists | System powers off, e-ink retains last image indefinitely |

---

## Mode Switching Behavior

### AO → PWN (`pwnoxide-mode pwn`)

1. Overlay moved: `angryoxide-v5.toml` → `angryoxide-v5.toml.disabled`
2. Bettercap service enabled and started
3. Pwnagotchi restarted
4. On restart:
   - Config loads without overlay → `bettercap.disabled` absent → `_ao_mode = False`
   - `ui.faces.png = false` (defaults.toml) → Korean text faces
   - `name` = "Pwnagotchi>" with cursor blink
   - `face` position = (0, 34) — below name
   - Splash service detects no overlay → does nothing on next boot

### PWN → AO (`pwnoxide-mode ao`)

1. Overlay moved: `angryoxide-v5.toml.disabled` → `angryoxide-v5.toml`
2. Bettercap service disabled and stopped
3. Pwnagotchi restarted
4. On restart:
   - Config loads overlay → `bettercap.disabled = true` → `_ao_mode = True`
   - `ui.faces.png = true` (overlay) → bull PNG faces
   - `name` = empty string, no cursor blink
   - `face` position = (0, 16) — near top, no name above
   - Splash service detects overlay → shows bull on next boot

---

## Rules & Constraints

### No Overlap Rule
- **AO mode:** No name rendered. Face at Y=16. Status at (125, 20). No conflict.
- **PWN mode:** Name at Y=20 (ends ~Y=32). Face at Y=34. 2px gap. No overlap.
- **Friend area:** Y=92-94, well below face zone. No conflict in either mode.
- **Plugin elements** (walkby status, AO capture count): Must be placed at Y ≥ 82 and ≤ 107 to avoid face and bottom bar.

### Bull Faces — Never in PWN Mode
- Splash service checks for overlay file → skips if PWN mode
- Config overlay disabled → `png = false` → faces.py loads Korean text defaults
- components.py Text widget: if PNG load fails, falls back to text rendering
- No bull PNG path should appear in defaults.toml — only in the overlay

### Korean Faces — Never in AO Mode
- Config overlay sets `png = true` + all 28 face paths to PNG files
- faces.py `load_from_config()` overwrites all globals with PNG paths
- Text widget sees `png = True` → loads PNG file instead of rendering text

### Display Refresh
- **Partial refresh** for all normal updates (fast, no full-screen flicker)
- **Full refresh** only for splash service (boot/shutdown) — writes to both EPD RAM banks
- Splash full refresh ensures image survives pwnagotchi's `epd.Clear()` + `displayPartBaseImage()`

### Cursor Behavior
- **AO mode:** Cursor disabled (`_ao_mode` check in `_refresh_handler`)
- **PWN mode:** Cursor blinks at `ui.fps` rate — toggles " █" suffix on name

### Status Text Position
- Always at (125, 20) in both modes
- Max 20 characters per line, wrapping enabled
- In AO mode, status text has the full width since no name is at (5, 20)
- In PWN mode, name "Pwnagotchi> █" occupies ~(5-120, 20), status starts at (125, 20)
