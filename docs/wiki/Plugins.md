# Lua Plugins

← [Back to Wiki Home](Home)

---

The bull runs Lua 5.4 plugins every loop cycle to draw the e-ink indicators you see — battery level, uptime, mode, AP counter, and everything else on the face screen. Twelve builtins ship with the image, and you can drop your own Lua files into `/etc/oxigotchi/plugins/` to add new indicators without touching Rust.

Plugins are sandboxed. They can read state and draw text at fixed coordinates. They cannot touch files, open sockets, or call shell commands. A broken plugin logs an error and gets skipped — it never crashes the daemon.

## Built-in Plugins

Twelve plugins ship in the default image:

| Plugin | What It Shows | When |
|--------|---------------|------|
| `ao_status` | AngryOxide process state and stats top bar | RAGE mode |
| `aps` | Access points seen counter | RAGE mode |
| `battery` | `BAT=75%` or `CHG=80%` from the PiSugar | always |
| `bt_status` | Short BT tether state (`BT:C` / `BT:-`) | always |
| `bt_summary` | BT mode top bar — devices, attacks, captures, patchram | BT mode |
| `crash` | AO crash counter and firmware health | RAGE mode |
| `ip_display` | Current IP so you know where to point your browser | always |
| `mode` | `RAGE:3`, `BT:2`, or `SAFE` with current level | always |
| `status_msg` | Personality status message (the bull's current mood text) | always |
| `sys_stats` | CPU temp, memory, load | always |
| `uptime` | `UP DD:HH:MM` since boot | always |
| `www` | Web dashboard URL reminder | always |

Each one is a small Lua file under 30 lines. Open `/etc/oxigotchi/plugins/battery.lua` on the Pi and you can read the entire plugin in one screen.

## Plugin Structure

Every plugin is one `.lua` file with two functions the daemon calls — `on_load()` when the plugin is first loaded, and `on_epoch()` on every cycle.

```lua
plugin = {}
plugin.name    = "my_plugin"
plugin.version = "1.0.0"
plugin.author  = "your_name"
plugin.tag     = "user"

function on_load(config)
    register_indicator("my_ind", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
end

function on_epoch(state)
    set_indicator("my_ind", "hello")
end
```

Set `tag = "default"` for builtin plugins shipped in the image, or `tag = "user"` for your own. The dashboard treats them slightly differently — users can disable user plugins freely, builtins have warnings.

## Plugin API

Three functions are exposed to every plugin.

### `register_indicator(name, opts)`

Declares a new display indicator. Must be called from `on_load()` — calling it later is a runtime error.

```lua
register_indicator("battery", {
    x    = 10,              -- x coordinate (0-249)
    y    = 95,              -- y coordinate (0-121)
    font = "small",         -- "small" (6pt) or "large" (8pt)
    label = "BAT",          -- optional prefix printed before value
    modes = {"RAGE", "BT"}, -- optional: restrict to listed modes
})
```

If `modes` is omitted the indicator draws in all three modes. Listing a mode name the daemon doesn't recognise (typo, or a mode that no longer exists) produces a warning and the indicator falls back to "all modes".

### `set_indicator(name, text)`

Updates what the indicator shows. Called from `on_epoch()` — this is the heart of every plugin.

```lua
set_indicator("battery", "CHG=80%")
```

The 2.13" display is tight on space. Strings wider than the column get truncated at render time, so prefer short text (`BAT=75%` rather than `Battery: 75 percent`).

### `log(level, message)`

Writes to the daemon log via `journalctl`. Levels: `"info"`, `"warn"`, `"error"`. Useful for debugging a plugin you're developing.

```lua
log("warn", "battery critical!")
```

## Reading Daemon State

The `state` table passed into `on_epoch()` is a flat snapshot of everything the daemon knows about this cycle. All fields are read-only.

### Timing and Mode

| Field | Type | What It Is |
|-------|------|------------|
| `uptime_secs` | number | Seconds since daemon start |
| `epoch` | number | Loop cycle counter (monotonic) |
| `mode` | string | `"RAGE"`, `"BT"`, or `"SAFE"` |
| `rage_level` | number | 0 if custom/disabled, 1-7 for active level |

### WiFi / AngryOxide

| Field | Type | What It Is |
|-------|------|------------|
| `channel` | number | Current WiFi channel |
| `aps_seen` | number | Total unique APs discovered this session |
| `handshakes` | number | Total handshakes captured this session |
| `captures_total` | number | All capture files on SD card |
| `blind_epochs` | number | Consecutive cycles without new APs (blind-mode detector) |
| `ao_state` | string | `"Running"`, `"Crashed"`, `"Starting"`, etc. |
| `ao_pid` | number | AO process ID (0 if not running) |
| `ao_crash_count` | number | Total AO crashes since boot |
| `ao_uptime_str` | string | AO uptime preformatted |
| `session_captures` | number | Capture files created this session |
| `session_handshakes` | number | Validated handshakes moved to SD this session |

### Battery and PiSugar

| Field | Type | What It Is |
|-------|------|------------|
| `battery_level` | number | 0-100 percentage |
| `battery_charging` | boolean | USB-C plugged into PiSugar |
| `battery_voltage_mv` | number | Raw cell voltage |
| `battery_low` | boolean | Below low threshold (default 20%) |
| `battery_critical` | boolean | Below critical threshold (default 5%) |
| `battery_available` | boolean | PiSugar detected on I2C |
| `pisugar_temp_c` | number | PiSugar board temperature |

### Bluetooth

| Field | Type | What It Is |
|-------|------|------------|
| `bt_connected` | boolean | Phone PAN tether active |
| `bt_short` | string | One-character state: `C`/`-`/`P`/`.`/`!` |
| `bt_ip` | string | IP on the BT interface |
| `bt_internet` | boolean | Internet reachable via BT tether |
| `bt_devices_seen` | number | BT scan hit count (BT mode) |
| `bt_active_attacks` | number | Number of running BT attacks |
| `bt_total_captures` | number | BT attack capture count |
| `bt_patchram_state` | string | `"attack"`, `"stock"`, `"unloaded"`, `"error"` |
| `bt_rage_level` | string | `"Low"`, `"Medium"`, `"High"` |

### Network

| Field | Type | What It Is |
|-------|------|------------|
| `internet_online` | boolean | Any internet reachable |
| `display_ip` | string | Primary IP to show on the dashboard |

### Personality

| Field | Type | What It Is |
|-------|------|------------|
| `mood` | number | -1.0 to +1.0, set by captures and dry spells |
| `face` | string | Current face name |
| `level` | number | XP level |
| `xp` | number | Total XP |
| `status_message` | string | The bull's current dialogue line |
| `joke_active` | boolean | A joke face override is in effect |

### System

| Field | Type | What It Is |
|-------|------|------------|
| `cpu_temp` | number | Pi CPU temperature °C |
| `mem_used_mb` | number | Used RAM |
| `mem_total_mb` | number | Total RAM |
| `cpu_percent` | number | CPU utilization |
| `cpu_freq_ghz` | string | Current CPU clock |
| `fw_crash_suppress` | number | WiFi firmware suppressed crashes |
| `fw_hardfault` | number | WiFi firmware hardfault counter |
| `fw_health` | string | Firmware health state |
| `skip_captured` | boolean | Smart Skip is active this cycle |

## A Real Plugin

Here's the full `battery.lua` — 20 lines, ships with the image:

```lua
plugin = {}
plugin.name    = "battery"
plugin.version = "1.0.0"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("battery", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
end

function on_epoch(state)
    local s
    if not state.battery_available then
        s = "BAT N/A"
    elseif state.battery_charging then
        s = "CHG=" .. state.battery_level .. "%"
    else
        s = "BAT=" .. state.battery_level .. "%"
    end
    set_indicator("battery", s)
end
```

No config. No magic. Read the state, format a string, set the indicator.

## Configuring Plugins

Plugin coordinates and enabled state live in `/etc/oxigotchi/plugins.toml`:

```toml
[plugins.battery]
enabled = true
x = 10
y = 95

[plugins.uptime]
enabled = true
x = 120
y = 95
```

You rarely edit this by hand. The dashboard's **Plugins** card shows every loaded plugin with an enable toggle and coordinate editors — changes apply immediately, no restart. The daemon writes `plugins.toml` for you.

## Writing Your Own

1. Create a `.lua` file in `/etc/oxigotchi/plugins/` on the Pi.
2. Set `plugin.tag = "user"` so the dashboard marks it as yours.
3. Define `on_load` and `on_epoch`.
4. Add an entry to `plugins.toml` with `enabled = true` and x/y.
5. Restart the daemon (`sudo systemctl restart rusty-oxigotchi`) or reload from the dashboard.

Errors in your plugin — syntax errors, nil lookups, infinite loops up to a time budget — are caught and logged. Check `journalctl -u rusty-oxigotchi | grep plugin` and you'll see exactly which line blew up.

## What Plugins Cannot Do

The Lua sandbox blocks anything dangerous:

- **No file I/O** — `io` and `os` (except `os.time`/`os.date`) are gone
- **No `require`** — `package`, `require`, `dofile`, `loadfile`, `load` are blocked
- **No network** — there's no socket library exposed
- **No shell** — no `os.execute`, no `popen`

This is intentional. Plugins read state and write indicators, nothing else. If you need to call a command or hit a URL, the feature belongs in the Rust daemon where it can be written safely, tested, and supervised. Submit a PR or open an issue.

## Related

- **[Web Dashboard](Web-Dashboard)** — The Plugins card manages toggle/position from the browser
- **[Architecture](Architecture)** — Where the Lua tick fits into the main loop
- **[Bull Faces](Bull-Faces)** — The faces plugins can read via `state.face`
