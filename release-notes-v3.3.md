## What's New in v3.3

### Bluetooth PAN Tethering (major fix)

BT tethering now works reliably with Android phones. The root cause was a conflicting `bt-agent` service running `NoInputNoOutput` capability, which prevented proper SSP pairing and link key storage. Fixed by:

- **Agent conflict prevention** — `rusty-oxigotchi.service` now declares `Conflicts=bt-agent.service`, ensuring only our `DisplayYesNo` Agent1 handles pairing
- **Proper link key storage** — SSP pairing with passkey confirmation produces authenticated bonds that persist across reboots
- **NAP readiness wait** — polls device UUIDs for up to 15s before attempting PAN connection, preventing premature connect failures
- **Actionable error messages** — new `PanConnectError` enum maps D-Bus errors to clear diagnostics: "BT tethering not enabled on phone", "Phone unpaired Pi", "Connection busy", etc.
- **PhoneBusy retry** — automatically retries PAN connection up to 3 times with 3s delay on `br-connection-busy`
- **Existing connection adoption** — on startup, detects and adopts an existing `bnep0` interface instead of failing with "already in progress"
- **Auto-discoverable** — Pi makes itself discoverable when no paired devices exist, enabling phone-initiated pairing

### Wall-Clock Timers

Replaced all epoch-count-based scheduling with wall-clock `WallTimer`s. Mood ticks, passive XP, display refresh, and BT reconnect now fire at consistent real-time intervals regardless of epoch frequency.

### RAGE Presets Refactored

All 7 levels re-tuned based on hardware stress testing with BT PAN active:

| Level | Name | Rate | Dwell | Channels |
|-------|------|------|-------|----------|
| 1 | Chill | 1 | 2000ms | 1,6,11 |
| 2 | Lurk | 1 | 2000ms | All 11 |
| 3 | Prowl | 2 | 2000ms | All 11 |
| 4 | Hunt | 2 | 1000ms | All 11 |
| 5 | RAGE | 3 | 1000ms | All 11 |
| 6 | FURY | 3 | 500ms | All 11 |
| 7 | YOLO | 5 | 500ms | All 11 |

Levels 1-6 are validated stable with BT tethering active. YOLO pushes past the tested envelope. Previously, 3 out of 7 levels were rate 1 — now the full range is usable.

### WiFi + BT Coexistence

- **BT carrier-lost backoff** — when `bnep0` disappears, uses exponential backoff (30/60/120/300s) instead of instant retry, which was hammering the combo chip's coexistence arbiter and causing firmware crashes
- **Firmware health monitor fixed** — crash counter addresses were TODO stubs, now reading real counters for preemptive crash detection
- **Rate cap removed** — rate 3 with BT PAN active is stable once the flapping fix is in; the coexistence rate cap was unnecessary dead code
- **Dashboard channel fix** — preset display showed 13 channels, now correctly shows 11

### Personality & Display

- Fresh-install mood starts at 100% (was 50%)
- Fixed double mood boost from joke messages
- Per-AP mood boost with configurable cap
- Display full refresh now gated by both partial count AND 180s minimum interval
- Removed stale Sleep-phase comments and spurious `next_phase` call

### Image

- Bake script clears runtime state (`state.json`, AO state) for clean first-boot defaults

## Upgrade

Flash `oxigotchi-v3.3.0-release.img.zip` to your SD card. Default login: `pi` / `raspberry`.

After boot, pair your phone via the web dashboard (BT scan + pair button) or from your phone's Bluetooth settings — look for "oxigotchi". Enable **Bluetooth tethering** on your phone for internet sharing.
