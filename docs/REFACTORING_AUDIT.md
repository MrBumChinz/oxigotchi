# Pwnagotchi (jayofelony fork) Refactoring Audit

**Date:** 2026-03-21
**Source:** `/c/msys64/home/user/pwnagotchi/pwnagotchi/`
**Installed:** `/home/pi/.pwn/lib/python3.13/site-packages/pwnagotchi/` (26 MB)
**Pi service status:** Failed (start-limit-hit) at time of audit

---

## Size Breakdown (on Pi)

| Directory | Size | Notes |
|-----------|------|-------|
| ui/hw/ | 15 MB | 99 display driver files + libs/ |
| ui/web/ | 5.5 MB | Web UI static assets (jQuery Mobile icons, etc.) |
| locale/ | 3.8 MB | 186 languages, 184 .po + 184 .mo files |
| plugins/ | 1.2 MB | Default plugins |
| ai/ | 40 KB | epoch.py (249 lines) + reward.py (28 lines) |
| mesh/ | 44 KB | Peer-to-peer mesh networking |
| **Total** | **26 MB** | |

---

## 1. Dead Code Inventory

### Priority 1 (Highest Impact -- Cut First)

#### 1a. `ai/reward.py` -- COMPLETELY DEAD (28 lines)
- **File:** `pwnagotchi/ai/reward.py`
- **What:** `RewardFunction.__call__()` computes a float reward from epoch stats
- **Status:** The import was already commented out in `epoch.py` (line 8). The `self._reward` instantiation is commented out (line 64). The call at `epoch.py:177` is commented out. **Nothing calls this file.**
- **On Pi:** Still deployed at 28 lines, still importable, still parsed by Python
- **Action:** Delete `reward.py` entirely. Remove the REMOVED comments in `epoch.py`.
- **RAM savings:** Trivial (module object + class), but removes a confusing artifact

#### 1b. `log.py` LastSession reward/training fields -- DEAD COMPUTATION
- **File:** `pwnagotchi/log.py` lines 23, 45-48, 80-85, 115-132, 172, 204-209, 236-240
- **What:** `TRAINING_TOKEN`, `train_epochs`, `min_reward`, `max_reward`, `avg_reward` -- all parsed from log lines and cached
- **Status:** No reward is ever logged anymore (epoch.py line 177 removed the `reward=` output). `train_epochs` is always 0 (no training token ever written). These fields are parsed, cached, and serialized to JSON cache file -- all for values that are permanently zero.
- **Consumers:** `cli.py:31-37` prints "trained for 0, average reward:0 (min:1000 max:-1000)" every manual mode start. The grid.py already stripped these from the upload payload.
- **Action:** Remove all reward/training fields from LastSession. Fix the cli.py log message to drop the training/reward line.
- **RAM savings:** ~200 bytes per session, but removes wasted CPU on every log parse

#### 1c. `voice.py` dead methods -- NEVER CALLED
- **File:** `pwnagotchi/voice.py`
- **Dead methods:**
  - `on_motivated(reward)` (line 63) -- never called. View.on_motivated exists but nothing calls it since AI removal
  - `on_demotivated(reward)` (line 70) -- same, never triggered
  - `on_last_session_tweet(last_session)` (line 226) -- never called by anything. Twitter plugin was removed years ago
  - `on_downloading(name)` (line 210) -- never called by any code path
- **Action:** Remove all four methods. Remove corresponding View methods (`on_motivated`, `on_demotivated` in `view.py:327-335`). Keep `on_uploading` -- plugins use it.

#### 1d. `epoch.py` dead fields
- **File:** `pwnagotchi/ai/epoch.py`
- **Dead fields:**
  - `non_overlapping_channels` (line 58) -- dict `{1:0, 6:0, 11:0}` initialized but never read or written to by anything. Was used by the AI observation vectors. Only reference in entire codebase is the initialization.
- **Action:** Remove `non_overlapping_channels`

#### 1e. Switcher plugin AI hooks -- DEAD HOOK NAMES
- **File:** `pwnagotchi/plugins/default/switcher.py` line 139-141
- **What:** Registers hooks for `ai_ready`, `ai_policy`, `ai_training_start`, `ai_training_step`, `ai_training_end`, `ai_best_reward`, `ai_worst_reward`, `free_channel`
- **Status:** These plugin events are never fired by any code. The AI is gone. `free_channel` was part of the AI's channel selection -- nothing fires `plugins.on('free_channel', ...)` anywhere in the codebase.
- **Action:** Remove these hook names from the methods list. Plugin will still work for the remaining valid hooks.

### Priority 2 (Moderate Impact)

#### 2a. Display type normalization -- BLOATED (~290 lines)
- **File:** `pwnagotchi/utils.py` lines 216-507
- **What:** A massive if/elif chain (187 comparisons) to normalize display type strings. Each is 2-3 lines.
- **Problem:** This is ~42% of utils.py (692 lines total). Could be a dictionary lookup in ~5 lines.
- **Action:** Replace with a `DISPLAY_ALIASES = {}` dict and a 5-line lookup function. Saves ~285 lines.
- **RAM savings:** String internment + bytecode for 187 branch comparisons

#### 2b. 186 locale directories -- MOSTLY UNUSED
- **File:** `pwnagotchi/locale/` -- 3.8 MB on disk
- **What:** Translations for 186 languages. Most have only the `.po` source and compiled `.mo`.
- **Status:** Most users run `lang = "en"`. Only a handful of languages have complete translations. Many are empty stubs or have only the "neural network is ready" line commented out.
- **Action (conservative):** Ship only the ~10 most popular locales. Others can be a separate installable package.
- **Disk savings:** ~3.5 MB (significant on Pi SD)

#### 2c. 99 hardware display drivers -- 15 MB
- **File:** `pwnagotchi/ui/hw/` -- 99 Python files + libs/ directory
- **What:** Individual driver files for every Waveshare, Pimoroni, DFRobot, etc. display
- **Problem:** Each device only uses ONE driver. The other 98 are dead weight.
- **Action:** Long-term, make drivers installable per-device. Short-term, no change needed as they're only imported on demand.
- **Disk savings:** Potentially ~14 MB but requires architectural change

#### 2d. `defaults.toml` references torch plugin repo
- **File:** `pwnagotchi/defaults.toml` line 19
- **What:** `custom_plugin_repos` includes `pwnagotchi-torch-plugins` -- a repo of plugins that depend on torch/AI, which is removed
- **Action:** Remove this repo URL from defaults

### Priority 3 (Minor/Cleanup)

#### 3a. `fix_services.py` description mentions "brain"
- **File:** `pwnagotchi/plugins/default/fix_services.py` line 24
- **What:** `__description__` says "Fix blindness, firmware crashes and brain not being loaded"
- **Status:** "Brain" was the AI model. No brain exists anymore.
- **Action:** Update description string

#### 3b. `log.py` comment references "tensorflow"
- **File:** `pwnagotchi/log.py` line 350
- **What:** Comment says `# disable scapy and tensorflow logging`
- **Status:** tensorflow was removed. The code only disables scapy logging.
- **Action:** Fix comment

#### 3c. View `on_motivated`/`on_demotivated` -- DEAD UI PATHS
- **File:** `pwnagotchi/ui/view.py` lines 327-335
- **What:** Two methods to show motivated/demotivated faces. They accept a `reward` parameter.
- **Status:** Nothing calls these methods since the AI reward system was removed. The corresponding faces exist but are never shown through this path (though `MOTIVATED` face is still used by `on_new_peer` for good friends).
- **Action:** Remove both methods from View. Keep `faces.MOTIVATED` and `faces.DEMOTIVATED` as they're referenced elsewhere.

#### 3d. `example.py` plugin
- **File:** `pwnagotchi/plugins/default/example.py` (131 lines)
- **What:** Example plugin showing all callbacks. Ships with the deployed package.
- **Status:** Should be disabled by default (it is), but it's deployed on Pi
- **Action:** Move to a docs/ or examples/ directory, don't deploy

#### 3e. Locale `.po` files with dead "neural network" translations
- **Files:** ~40 locale `.po` files contain commented-out `#~ msgid "The neural network is ready."` lines
- **Action:** No action needed (they're comments), but strip them if doing a locale cleanup

---

## 2. RAM Savings Estimate

| Item | Estimated Savings | Notes |
|------|-------------------|-------|
| Remove reward.py | ~2 KB | Module + class object |
| Remove LastSession reward/training fields | ~1 KB | 5 attributes * session count |
| Remove dead Voice methods | ~1 KB | 4 method objects + string constants |
| Display alias dict vs if/elif | ~5-10 KB | Bytecode reduction |
| Locale pruning (186 -> 10) | 0 KB runtime | Only loaded locale uses RAM |
| **Total runtime** | **~10-15 KB** | |

The real win is **disk space** (3.8 MB locale, 15 MB hw drivers) and **code clarity**, not RAM. The pwnagotchi process is not currently running so RSS could not be measured.

---

## 3. Things to NOT Touch (Look Dead but Needed)

### 3a. `epoch.py` `Epoch` class -- STILL ACTIVE
The `pwnagotchi/ai/epoch.py` Epoch class looks like AI residue because it lives in the `ai/` package, but it is the **core heartbeat** of the system:
- `Automata.__init__()` creates `self._epoch = Epoch(config)` (automata.py:13)
- Every main loop iteration calls `agent.next_epoch()` -> `self._epoch.next()`
- All mood logic (sad, bored, angry, excited) depends on epoch counters
- Peer bond factors computed in `observe()` drive the mood system
- The `_epoch_data` dict is passed to plugins via `plugins.on('epoch', ...)`
- Session-stats plugin charts are built from epoch data

**Recommendation:** Move `epoch.py` out of `ai/` to `pwnagotchi/epoch.py` since it has nothing to do with AI anymore.

### 3b. `on_uploading` in Voice/View -- STILL USED
Multiple plugins call `display.on_uploading()`: ohcapi.py, wpa-sec.py, wigle.py

### 3c. Grid plugin + grid.py -- FUNCTIONAL
The grid plugin and `pwnagotchi/grid.py` communicate with `pwngrid-peer` (running on port 8666, confirmed via `ps aux` -- 20 MB RSS). This handles:
- Peer-to-peer mesh discovery
- Handshake reporting to opwngrid.xyz
- Inbox messaging between units
- Advertisement data broadcasting

**Status:** Functional and actively used. The grid.py module has already been cleaned of AI fields (lines 65-66 show REMOVED comments).

### 3d. `utils.py` functions: `download_file`, `unzip`, `extract_from_pcap`, `StatusFile`, `md5`
All actively used:
- `download_file` + `unzip`: used by `plugins/cmd.py` (plugin installer) and `auto-update.py`
- `extract_from_pcap` + `WifiInfo`: used by `grid.py` plugin and `wigle.py`
- `StatusFile`: used by 6 files (grid, session-stats, ohcapi, wigle, auto_backup, auto-update)
- `md5`: used by `plugins/cmd.py`

### 3e. Bettercap client (`bettercap.py`) -- ESSENTIAL
Even in AO mode, the `Client` class is inherited by `Agent` (for MRO). The `StubClient` overrides its methods. The websocket/retry logic is used in PWN mode. Do not remove.

### 3f. `automata.py` mood system -- CORE FEATURE
The entire mood state machine (sad, bored, angry, excited, grateful, lonely) is the personality of the device. All counters in epoch.py feed this. Not dead code.

---

## 4. Priority Recommendations Summary

| Priority | Action | Lines Saved | Disk Saved |
|----------|--------|-------------|------------|
| P1 | Delete `ai/reward.py` | 28 | 1 KB |
| P1 | Strip reward/training from `log.py` + `cli.py` | ~30 | 0 |
| P1 | Remove dead Voice/View methods | ~20 | 0 |
| P1 | Remove AI hooks from switcher.py | ~2 | 0 |
| P1 | Remove `non_overlapping_channels` from epoch.py | 1 | 0 |
| P1 | Remove torch plugin repo from defaults.toml | 1 | 0 |
| P2 | Refactor display normalization to dict | ~285 | 0 |
| P2 | Move epoch.py out of ai/ package | 0 | 0 |
| P2 | Prune locales to top 10 | 0 | ~3.5 MB |
| P3 | Update fix_services description | 0 | 0 |
| P3 | Fix tensorflow comment in log.py | 0 | 0 |
| P3 | Move example.py out of deployment | 131 | ~5 KB |

**Start with P1** -- all are safe, surgical removals with zero risk of breaking functionality.
The display dict refactor (P2) is the biggest code quality win.
Locale pruning (P2) is the biggest disk space win.
