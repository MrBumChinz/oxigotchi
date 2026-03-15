#!/bin/bash
# apply-oxigotchi-patches.sh — Reapply oxigotchi core file patches after pwnagotchi updates.
#
# Can be run manually:   sudo /usr/local/bin/apply-oxigotchi-patches.sh
# Or triggered by:       oxigotchi-patches.path systemd unit on package changes
#
# Idempotent: checks each patch before applying. Safe to run repeatedly.

set -euo pipefail

LOG_TAG="oxigotchi-patches"
SITE_PKG="/home/pi/.pwn/lib/python3.13/site-packages/pwnagotchi"
PATCHED=0
SKIPPED=0
FAILED=0

log() { logger -t "$LOG_TAG" "$1"; echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"; }
die() { log "FATAL: $1"; exit 1; }

# ─── Patch 1: pwnlib — comment out reload_brcm in stop_monitor_interface ───
patch_pwnlib() {
    local f="/usr/bin/pwnlib"
    [ -f "$f" ] || { log "SKIP pwnlib: $f not found"; SKIPPED=$((SKIPPED+1)); return; }

    if grep -q '#.*reload_brcm.*disabled.*SDIO' "$f" 2>/dev/null; then
        log "OK   pwnlib: already patched"
        SKIPPED=$((SKIPPED+1))
    else
        # Comment out bare reload_brcm calls inside stop_monitor_interface
        if sed -i '/stop_monitor_interface/,/^}$/ s/^\([[:space:]]*\)reload_brcm\b/\1#reload_brcm  # disabled: causes SDIO crash (oxigotchi)/' "$f"; then
            log "DONE pwnlib: commented out reload_brcm in stop_monitor_interface"
            PATCHED=$((PATCHED+1))
        else
            log "FAIL pwnlib: sed failed"
            FAILED=$((FAILED+1))
        fi
    fi
}

# ─── Patch 2: cache.py — isinstance check for AO handshakes ───
patch_cache() {
    local f="$SITE_PKG/plugins/default/cache.py"
    [ -f "$f" ] || { log "SKIP cache.py: $f not found"; SKIPPED=$((SKIPPED+1)); return; }

    if grep -q 'isinstance(access_point, dict)' "$f" 2>/dev/null; then
        log "OK   cache.py: already patched"
        SKIPPED=$((SKIPPED+1))
    else
        if sed -i 's/if self\.ready:/if self.ready and isinstance(access_point, dict):/' "$f"; then
            log "DONE cache.py: added isinstance check for AO handshakes"
            PATCHED=$((PATCHED+1))
        else
            log "FAIL cache.py: sed failed"
            FAILED=$((FAILED+1))
        fi
    fi
}

# ─── Patch 3: handler.py — CSRF exemption for plugin webhooks ───
patch_handler() {
    local f="$SITE_PKG/ui/web/handler.py"
    [ -f "$f" ] || { log "SKIP handler.py: $f not found"; SKIPPED=$((SKIPPED+1)); return; }

    if grep -q 'csrf\.exempt' "$f" 2>/dev/null; then
        log "OK   handler.py: already patched"
        SKIPPED=$((SKIPPED+1))
    else
        # After the line that sets plugins_with_auth, add CSRF exemption
        if sed -i '/plugins_with_auth = self\.with_auth(self\.plugins)/a\        # Exempt plugin webhooks from CSRF (plugins handle their own auth) [oxigotchi]\n        if hasattr(self._app, '"'"'csrf'"'"'):\n            plugins_with_auth = self._app.csrf.exempt(plugins_with_auth)' "$f"; then
            log "DONE handler.py: added CSRF exemption for plugin webhooks"
            PATCHED=$((PATCHED+1))
        else
            log "FAIL handler.py: sed failed"
            FAILED=$((FAILED+1))
        fi
    fi
}

# ─── Patch 4: server.py — store csrf instance on app ───
patch_server() {
    local f="$SITE_PKG/ui/web/server.py"
    [ -f "$f" ] || { log "SKIP server.py: $f not found"; SKIPPED=$((SKIPPED+1)); return; }

    if grep -q 'app\.csrf = csrf' "$f" 2>/dev/null; then
        log "OK   server.py: already patched"
        SKIPPED=$((SKIPPED+1))
    else
        # Replace CSRFProtect(app) with csrf = CSRFProtect(app); app.csrf = csrf
        if sed -i 's/CSRFProtect(app)/csrf = CSRFProtect(app)\n            app.csrf = csrf/' "$f"; then
            log "DONE server.py: stored csrf instance on app"
            PATCHED=$((PATCHED+1))
        else
            log "FAIL server.py: sed failed"
            FAILED=$((FAILED+1))
        fi
    fi
}

# ─── Patch 5: log.py — session cache for LastSession.parse() ───
patch_log() {
    local f="$SITE_PKG/log.py"
    [ -f "$f" ] || { log "SKIP log.py: $f not found"; SKIPPED=$((SKIPPED+1)); return; }

    if grep -q '_CACHE_FILE' "$f" 2>/dev/null; then
        log "OK   log.py: already patched"
        SKIPPED=$((SKIPPED+1))
    else
        # This patch is too large for sed. Use python to apply it.
        python3 - "$f" <<'PYEOF'
import sys

f = sys.argv[1]
with open(f) as fh:
    code = fh.read()

# Bail if already patched
if '_CACHE_FILE' in code:
    sys.exit(0)

# Add 'import json' if not present
if 'import json' not in code:
    code = code.replace('import hashlib', 'import hashlib\nimport json', 1)

# Insert _CACHE_FILE, _save_cache, _load_cache before the parse method
cache_block = '''
    _CACHE_FILE = '/home/pi/last_session_cache.json'

    def _save_cache(self):
        """Save parsed session data to cache file for fast boot."""
        try:
            import os as _os
            stat = _os.stat(self.path) if _os.path.exists(self.path) else None
            peer_data = None
            if self.last_peer:
                try:
                    peer_data = {
                        'session_id': self.last_peer.session_id(),
                        'channel': 1,
                        'rssi': self.last_peer.rssi,
                        'identity': self.last_peer.identity(),
                        'name': self.last_peer.name(),
                        'pwnd_tot': self.last_peer.pwnd_total(),
                    }
                except Exception:
                    pass
            data = {
                'version': 1,
                'log_mtime': stat.st_mtime if stat else 0,
                'log_size': stat.st_size if stat else 0,
                'last_session_id': self.last_session_id,
                'duration': self.duration,
                'duration_human': self.duration_human,
                'deauthed': self.deauthed,
                'associated': self.associated,
                'handshakes': self.handshakes,
                'epochs': self.epochs,
                'train_epochs': self.train_epochs,
                'peers': self.peers,
                'last_peer': peer_data,
                'min_reward': self.min_reward,
                'max_reward': self.max_reward,
                'avg_reward': self.avg_reward,
            }
            with open(self._CACHE_FILE, 'w') as f:
                json.dump(data, f)
        except Exception as e:
            logging.debug("could not save session cache: %s" % e)

    def _load_cache(self):
        """Load cached session data if cache is valid (log hasn't changed)."""
        try:
            import os as _os
            if not _os.path.isfile(self._CACHE_FILE):
                return False
            stat = _os.stat(self.path) if _os.path.exists(self.path) else None
            if not stat:
                return False
            with open(self._CACHE_FILE, 'r') as f:
                data = json.load(f)
            if data.get('version') != 1:
                return False
            if data.get('log_mtime') != stat.st_mtime or data.get('log_size') != stat.st_size:
                return False
            from pwnagotchi.mesh.peer import Peer as _Peer
            self.last_session_id = data.get('last_session_id', '')
            self.duration = data.get('duration', '')
            self.duration_human = data.get('duration_human', '')
            self.deauthed = data.get('deauthed', 0)
            self.associated = data.get('associated', 0)
            self.handshakes = data.get('handshakes', 0)
            self.epochs = data.get('epochs', 0)
            self.train_epochs = data.get('train_epochs', 0)
            self.peers = data.get('peers', 0)
            self.min_reward = data.get('min_reward', 1000)
            self.max_reward = data.get('max_reward', -1000)
            self.avg_reward = data.get('avg_reward', 0)
            peer_data = data.get('last_peer')
            if peer_data:
                self.last_peer = _Peer({
                    'session_id': peer_data.get('session_id', ''),
                    'channel': peer_data.get('channel', 1),
                    'rssi': peer_data.get('rssi', 0),
                    'identity': peer_data.get('identity', ''),
                    'advertisement': {
                        'name': peer_data.get('name', ''),
                        'pwnd_tot': peer_data.get('pwnd_tot', 0),
                    }
                })
            self.last_saved_session_id = self._get_last_saved_session_id()
            logging.info("loaded session data from cache (skipped log parsing)")
            return True
        except Exception as e:
            logging.debug("could not load session cache: %s" % e)
            return False

'''

# Insert before def parse(
code = code.replace('    def parse(self,', cache_block + '    def parse(self,', 1)

# Patch parse() to use cache: replace the 'else:' branch after 'if skip:'
# We need to add cache loading between 'if skip:' and 'else:'
old_parse = """        if skip:
            logging.debug("skipping parsing of the last session logs ...")
        else:"""
new_parse = """        if skip:
            logging.debug("skipping parsing of the last session logs ...")
        elif self._load_cache():
            logging.debug("session data loaded from cache")
        else:"""
code = code.replace(old_parse, new_parse, 1)

# Add _save_cache() call after _parse_stats() in parse()
code = code.replace(
    '            self._parse_stats()\n        self.parsed = True',
    '            self._parse_stats()\n            self._save_cache()\n        self.parsed = True',
    1
)

with open(f, 'w') as fh:
    fh.write(code)
PYEOF
        if [ $? -eq 0 ]; then
            log "DONE log.py: added session cache to LastSession.parse()"
            PATCHED=$((PATCHED+1))
        else
            log "FAIL log.py: python patch failed"
            FAILED=$((FAILED+1))
        fi
    fi
}

# ─── Main ───
main() {
    log "=== Oxigotchi patch check starting ==="

    # Must be root for writing to /usr/bin and site-packages
    if [ "$(id -u)" -ne 0 ]; then
        die "Must be run as root (use sudo)"
    fi

    patch_pwnlib
    patch_cache
    patch_handler
    patch_server
    patch_log

    log "=== Done: $PATCHED applied, $SKIPPED already ok, $FAILED failed ==="

    if [ "$FAILED" -gt 0 ]; then
        exit 1
    fi

    # If any patches were applied, restart pwnagotchi to pick them up
    if [ "$PATCHED" -gt 0 ]; then
        if systemctl is-active --quiet pwnagotchi 2>/dev/null; then
            log "Restarting pwnagotchi to pick up patches..."
            systemctl restart pwnagotchi
        fi
    fi
}

main "$@"
