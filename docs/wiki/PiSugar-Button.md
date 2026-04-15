# PiSugar 3 Button

← [Back to Wiki Home](Home)

---

## Button mappings

| Press | Action |
|-------|--------|
| **Single tap** | Cycle rage level (1 → 2 → 3 → 4 → 5 → 6 → back to 1) |
| **Double tap** | Toggle RAGE ↔ SAFE mode |
| **Long press** | Toggle BT tethering on/off |

Every button event triggers an immediate e-ink partial refresh so you see the result instantly — no waiting for the next display cycle.

> **Level 7 (YOLO) is excluded from the button cycle.** The BCM43436B0 firmware crashes at rate 7 + 500ms dwell + all 13 channels. Use the web dashboard if you want level 7.

---

## How tap detection works

The PiSugar 3 MCU (FM33LC023N) handles debouncing and classifies taps itself. The daemon reads register `0x08` (TAP) each epoch:

| Register value | Event |
|---|---|
| `0x01` | Single tap |
| `0x02` | Double tap |
| `0x03` | Long press |
| `0x00` | No event |

This is **MCU-native detection** — the PiSugar firmware decides what counts as a tap. The daemon does not debounce, time, or count anything itself. Prior to v3.3.1 there was a software `ButtonDebouncer` in the daemon loop that was dropping events on slow loop cycles. It was removed entirely.

Long press comes via the web API (`/api/button`) rather than the TAP register, since the PiSugar firmware only exposes long press through pisugar-server's shell scripts. Both paths feed into the same handler.

---

## Register map (relevant bits)

| Register | Address | Purpose |
|---|---|---|
| CTR1 | `0x02` | Anti-mistouch (`0x08` bit) |
| CTR2 | `0x03` | Soft shutdown enable/state, auto-hibernate |
| Temperature | `0x04` | Board temp: `(value - 40) = °C` |
| TAP | `0x08` | Tap event (single/double/long) |

> **The registers were previously swapped.** Before v3.3.1, the daemon was reading temperature from `0x08` and tap events from `0x04` — the exact opposite of the correct register map. Taps were silently discarded because the temperature register never holds `0x01`, `0x02`, or `0x03`. Fixing the register map was what made the button work at all.

---

## Soft shutdown and the CTR2 latch

Long press gracefully shuts the Pi down via `systemctl poweroff`. The PiSugar MCU monitors CTR2 register `0x03` to cut power after the OS halts.

**The latch bug (fixed in v3.3.1):** CTR2 bit 3 (`CTR2_SOFT_SHUTDOWN_STATE`) latches high when a soft shutdown fires and only resets when power is fully cut. If the Pi rebooted instead of fully powering off (e.g. kernel panic, watchdog), the latch stayed high. On the next boot, the daemon saw the latch set and tried to complete the previous shutdown — preventing normal operation until a cold power cycle.

The fix: on boot the daemon reads CTR2, detects the stale latch, and clears it before doing anything else.

---

## Board temperature in Lua

The PiSugar board temperature is exposed to Lua plugins as `pisugar_temp_c`. You can render it on the e-ink display without poking I2C yourself:

```lua
function on_load(config)
    register_indicator("pisugar_temp", 190, 110)
end

function on_epoch(state)
    local t = state.pisugar_temp_c
    if t then
        set_indicator("pisugar_temp", string.format("%d°C", t))
    end
end
```

The value is `nil` on non-Pi builds (host tests, RAGE-only setups without a PiSugar HAT).

---

## See also

- [Getting Started](Getting-Started) — initial setup and first boot
- [Plugins](Plugins) — full Lua plugin API reference
- [Web Dashboard](Web-Dashboard) — adjust rage levels and mode from the browser
