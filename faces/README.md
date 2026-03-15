# Oxigotchi Bull Faces — Design & Integration Guide

## Display Hardware

- **Screen**: Waveshare 2.13" V4 e-ink
- **Resolution**: 250 x 122 pixels
- **Color**: 1-bit (black and white only, no grayscale)
- **Config**: `ui.display.type = "waveshare_4"`, `ui.invert = true` (white on black), `rotation = 180`

## Face Auto-Switching

Faces change automatically based on the `pwnoxide-mode` state:

- **AO ON** (`pwnoxide-mode ao`): AngryOxide attacks + bull PNG faces (this set)
- **AO OFF** (`pwnoxide-mode pwn`): bettercap attacks + stock Korean text faces

The config overlay `angryoxide-v5.toml` sets `[ui.faces] png = true` and maps all 28 states
to bull PNGs. When the overlay is removed (PWN mode), pwnagotchi falls back to its default
Korean text face strings.

## Bull Window

With name/walkby/bt-tether/display-password elements cleared, the face area is:

```
y=0   PWND 0 (50)                      UP 00:50:21
y=14  ─────────────────────────────────────────────
y=16  ┌──────────────────┐  [speech/status text]
      │                  │  [speech continued  ]
      │    BULL FACE     │
      │    120 x 66      │
      │                  │
y=82  └──────────────────┘
y=72                        Lv 21  Exp|████░░░░|
y=82  8b@e05@at@p191        mem  cpu  freq  temp
                             50%  5%  1.06  52C
y=108 ─────────────────────────────────────────────
y=109 CH9  AP21  WWW:C  CHG100%  A⊘ D⊘       AUTO
```

- **Position**: (0, 16)
- **Size**: 120 x 66 pixels
- **Format**: 1-bit PNG, white background, black art
- **Display inverts** to white-on-black automatically (`ui.invert = true`)

The plugin hides overlapping elements every `on_ui_update` cycle:
`name`, `walkby`, `blitz`, `walkby_status`, `bluetooth`, `display-password`, `ip_display`

## 28 Face Files

### Core Lifecycle

| Face File | State | Trigger | What's Happening |
|-----------|-------|---------|-----------------|
| `awake.png` | AWAKE | `on_starting()`, `on_normal()` | System booting, starting epoch |
| `sleep.png` | SLEEP | `wait(sleeping=True)`, `on_shutdown()` | Idle between epochs |
| `shutdown.png` | SHUTDOWN | `on_shutdown()` | Clean power off |
| `broken.png` | BROKEN | `on_rebooting()` | Crash recovery, forced restart |

### Scanning (alternates each step in wait loop)

| Face File | State | Trigger | What's Happening |
|-----------|-------|---------|-----------------|
| `look_r.png` | LOOK_R | `wait(sleeping=False)`, step % 2 == 0, bad mood | Scanning right, neutral |
| `look_l.png` | LOOK_L | `wait(sleeping=False)`, step % 2 == 1, bad mood | Scanning left, neutral |
| `look_r_happy.png` | LOOK_R_HAPPY | `wait(sleeping=False)`, step % 2 == 0, good mood | Scanning right, happy |
| `look_l_happy.png` | LOOK_L_HAPPY | `wait(sleeping=False)`, step % 2 == 1, good mood | Scanning left, happy |

### Attack Cycle (per-epoch transient events)

| Face File | State | Trigger | What's Happening |
|-----------|-------|---------|-----------------|
| `intense.png` | INTENSE | `on_assoc(ap)` | Sending PMKID association frames |
| `cool.png` | COOL | `on_deauth(sta)` | Sending deauth frames |
| `happy.png` | HAPPY | `on_handshakes(count)` | Captured a handshake |
| `sad.png` | SAD | `on_miss(who)` | AP/client disappeared mid-attack |
| `smart.png` | SMART | `on_free_channel()`, `on_reading_logs()` | Found optimal channel, processing logs |

### Mood States (evaluated at epoch boundary by AI)

| Face File | State | Trigger Condition | What's Happening |
|-----------|-------|-------------------|-----------------|
| `excited.png` | EXCITED | `active_for >= 5 epochs` | Sustained captures, on a roll |
| `bored.png` | BORED | `inactive_for >= 25 epochs` | Nothing happening for a while |
| `sad.png` | SAD | `inactive_for >= 40 epochs` | Long dry spell |
| `angry.png` | ANGRY | `inactive >= 2x sad_threshold` OR `misses >= 2x max` | Very long inactivity or many failures |
| `lonely.png` | LONELY | Stale recon, no peers nearby | No other pwnagotchis around |
| `grateful.png` | GRATEFUL | Active + good friend network (overrides bored/sad/angry) | Has peer support |
| `motivated.png` | MOTIVATED | AI reward is positive | Learning is going well |
| `demotivated.png` | DEMOTIVATED | AI reward is negative | Learning isn't working |

### Social

| Face File | State | Trigger | What's Happening |
|-----------|-------|---------|-----------------|
| `friend.png` | FRIEND | `on_new_peer()` with good friend | Met a known pwnagotchi friend |
| `lonely.png` | LONELY | `on_lost_peer()` | Friend went out of range |

### Data Transfer

| Face File | State | Trigger | What's Happening |
|-----------|-------|---------|-----------------|
| `upload.png` | UPLOAD | `on_uploading(to)` | Sending data to wpa-sec/wigle/etc |
| `debug.png` | DEBUG | `on_custom(text)` | Debug mode active |

### Edge Cases (Oxigotchi plugin extensions)

| Face File | State | How Detected | What's Happening |
|-----------|-------|-------------|-----------------|
| `wifi_down.png` | WIFI_DOWN | `blind_for >= mon_max_blind_epochs` (before restart) | wlan0/wlan0mon interface gone |
| `fw_crash.png` | FW_CRASH | `_try_fw_recovery()` detects `-110` in dmesg | brcmfmac firmware crashed |
| `ao_crashed.png` | AO_CRASHED | `_check_health()` finds dead process | AngryOxide process died |
| `battery_low.png` | BATTERY_LOW | PiSugar plugin: `battery_level < 20%` | Battery getting low |
| `battery_critical.png` | BATTERY_CRITICAL | PiSugar plugin: `battery_level < 15%` | About to shutdown |

## Processing Pipeline

```
Source art (any size)          process_for_eink.py          eink/
  angry.png             --->   auto-crop whitespace    --->   angry.png
  awake.png                    resize to 120x66             awake.png
  ...                          LANCZOS + center pad          ...
  (28 files)                   threshold to 1-bit           (28 files)
                               strip metadata
```

**Steps** (`process_for_eink.py`):

1. **Source**: High-res black & white bull art from ChatGPT (any size)
2. **Auto-crop**: Remove whitespace around the bull
3. **Resize**: Fit to 120x66 maintaining aspect ratio (LANCZOS)
4. **Center**: Pad to exactly 120x66 on white canvas
5. **Threshold**: Convert to 1-bit at 50% threshold (no dithering -- dithering looks bad on e-ink)
6. **Save**: 1-bit PNG to `eink/` directory

Metadata is stripped as a side effect of the 1-bit conversion pipeline -- Pillow's `point()` to
mode `'1'` discards all EXIF/ICC/metadata from the source. The output PNGs contain only raw
pixel data. This prevents leaking ChatGPT generation metadata or GPS coordinates.

## Config Integration

### angryoxide-v5.toml (deployed to `/etc/pwnagotchi/conf.d/`)

```toml
[ui.faces]
png = true
position_x = 0
position_y = 16
look_r = ["/etc/pwnagotchi/custom-plugins/faces/look_r.png"]
look_l = ["/etc/pwnagotchi/custom-plugins/faces/look_l.png"]
look_r_happy = ["/etc/pwnagotchi/custom-plugins/faces/look_r_happy.png"]
look_l_happy = ["/etc/pwnagotchi/custom-plugins/faces/look_l_happy.png"]
sleep = ["/etc/pwnagotchi/custom-plugins/faces/sleep.png"]
awake = ["/etc/pwnagotchi/custom-plugins/faces/awake.png"]
bored = ["/etc/pwnagotchi/custom-plugins/faces/bored.png"]
intense = ["/etc/pwnagotchi/custom-plugins/faces/intense.png"]
cool = ["/etc/pwnagotchi/custom-plugins/faces/cool.png"]
happy = ["/etc/pwnagotchi/custom-plugins/faces/happy.png"]
grateful = ["/etc/pwnagotchi/custom-plugins/faces/grateful.png"]
excited = ["/etc/pwnagotchi/custom-plugins/faces/excited.png"]
motivated = ["/etc/pwnagotchi/custom-plugins/faces/motivated.png"]
demotivated = ["/etc/pwnagotchi/custom-plugins/faces/demotivated.png"]
smart = ["/etc/pwnagotchi/custom-plugins/faces/smart.png"]
lonely = ["/etc/pwnagotchi/custom-plugins/faces/lonely.png"]
sad = ["/etc/pwnagotchi/custom-plugins/faces/sad.png"]
angry = ["/etc/pwnagotchi/custom-plugins/faces/angry.png"]
friend = ["/etc/pwnagotchi/custom-plugins/faces/friend.png"]
broken = ["/etc/pwnagotchi/custom-plugins/faces/broken.png"]
debug = ["/etc/pwnagotchi/custom-plugins/faces/debug.png"]
upload = ["/etc/pwnagotchi/custom-plugins/faces/upload.png"]
```

### Plugins disabled when AO is active

```toml
[main.plugins.walkby]
enabled = false

[main.plugins.display-password]
enabled = false

[main.plugins.bt-tether]
show_detailed_status = false
```

## File Locations

| What | Local (Windows) | Pi |
|------|----------------|-----|
| Source art | `firmware_analysis/faces/*.png` | -- |
| Processing script | `firmware_analysis/faces/process_for_eink.py` | -- |
| E-ink ready PNGs | `firmware_analysis/faces/eink/*.png` | `/etc/pwnagotchi/custom-plugins/faces/` |
| Config overlay | `firmware_analysis/angryoxide-v5.toml` | `/etc/pwnagotchi/conf.d/angryoxide-v5.toml` |
| Plugin | `firmware_analysis/angryoxide_v2.py` | `/etc/pwnagotchi/custom-plugins/angryoxide.py` |
| Mode switcher | `firmware_analysis/pwnoxide-mode.sh` | `/usr/local/bin/pwnoxide-mode` |
| Deployer | `firmware_analysis/deploy_pwnoxide.py` | -- (runs from Windows) |

## Edge Case Faces -- Plugin Implementation

The stock pwnagotchi has no face states for wifi_down, fw_crash, ao_crashed, or battery levels.
These are set by the angryoxide plugin via `_face()` which resolves PNG paths with a text fallback:

```python
# In on_epoch or _check_health:
agent._view.set('face', '/etc/pwnagotchi/custom-plugins/faces/fw_crash.png')
agent._view.set('status', 'Firmware crashed! Recovering...')
```

If a PNG is missing, `_face()` falls back to stock text faces (e.g., `faces.BROKEN`, `faces.ANGRY`).

## Art Style Guide

- **Style**: Black and white vector/logo, high contrast, thick outlines
- **Subject**: Front-facing bull head with horns (sports mascot style)
- **Best results at small size**: Front-facing, centered, symmetrical features
- **Avoid**: Fine detail, thin lines, grayscale gradients, side profiles (lose detail at 120x66)
- **Mood variation**: Change eyes, mouth, nostrils, accessories (glasses, hearts, lightning)
- **Consistency**: Same base bull head shape across all moods -- only the expression changes
