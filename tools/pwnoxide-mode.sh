#!/bin/bash
# pwnoxide-mode — switch between AngryOxide and bettercap attack modes
# Usage: pwnoxide-mode ao|pwn|status|rollback-fw
#
# Safety: mode switches include a watchdog — if pwnagotchi doesn't come
# back within 45s, the config change is auto-reverted so the Pi stays
# reachable. Never touches NetworkManager or usb0.

set -euo pipefail

OVERLAY="/etc/pwnagotchi/conf.d/angryoxide-v5.toml"
DISABLED="${OVERLAY}.disabled"
FW_PATH="/lib/firmware/brcm/brcmfmac43436-sdio.bin"
FW_ORIG="${FW_PATH}.orig"
WATCHDOG_TIMEOUT=45

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

check_usb0() {
  if ip link show usb0 > /dev/null 2>&1; then
    echo "  usb0: UP (SSH lifeline OK)"
  else
    echo "  WARNING: usb0 is DOWN — SSH may depend on WiFi!"
    echo "  Proceeding anyway, but be aware you may lose connectivity."
  fi
}

# Restart pwnagotchi with watchdog: if the service doesn't come back
# healthy within $WATCHDOG_TIMEOUT seconds, revert $1 -> $2 and restart again.
safe_restart() {
  local FROM="$1"   # current overlay location (what we just set)
  local TO="$2"     # where to revert to if restart fails

  sudo systemctl restart pwnagotchi

  echo "Waiting up to ${WATCHDOG_TIMEOUT}s for pwnagotchi to come back..."
  local elapsed=0
  while [ $elapsed -lt $WATCHDOG_TIMEOUT ]; do
    sleep 5
    elapsed=$((elapsed + 5))
    if systemctl is-active --quiet pwnagotchi; then
      echo "  pwnagotchi is active (${elapsed}s)"
      return 0
    fi
    echo "  not yet... (${elapsed}s)"
  done

  # Service didn't come back — revert
  echo ""
  echo "WATCHDOG: pwnagotchi failed to start within ${WATCHDOG_TIMEOUT}s!"
  echo "  Reverting config change..."
  if [ -f "$FROM" ]; then
    sudo mv "$FROM" "$TO"
  elif [ -f "$TO" ]; then
    : # already in reverted state
  fi
  echo "  Restarting pwnagotchi with reverted config..."
  sudo systemctl restart pwnagotchi
  echo "  Config reverted. Check 'journalctl -u pwnagotchi -n 50' for errors."
  return 1
}

# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

case "${1:-status}" in
  ao)
    check_usb0
    if [ -f "$DISABLED" ]; then
      sudo mv "$DISABLED" "$OVERLAY"
      echo "AngryOxide mode ENABLED"
      echo "Restarting pwnagotchi (with watchdog)..."
      if ! safe_restart "$OVERLAY" "$DISABLED"; then
        echo "ERROR: Mode switch to AO failed, reverted to pwn mode."
        exit 1
      fi
    elif [ -f "$OVERLAY" ]; then
      echo "AngryOxide mode already active"
    else
      echo "ERROR: overlay not found at $OVERLAY or $DISABLED"
      exit 1
    fi
    ;;
  pwn)
    check_usb0
    if [ -f "$OVERLAY" ]; then
      sudo mv "$OVERLAY" "$DISABLED"
      echo "AngryOxide mode DISABLED (bettercap attacks restored)"
      echo "Restarting pwnagotchi (with watchdog)..."
      if ! safe_restart "$DISABLED" "$OVERLAY"; then
        echo "ERROR: Mode switch to pwn failed, reverted to AO mode."
        exit 1
      fi
    elif [ -f "$DISABLED" ]; then
      echo "AngryOxide mode already disabled"
    else
      echo "ERROR: overlay not found at $OVERLAY or $DISABLED"
      exit 1
    fi
    ;;
  rollback-fw)
    echo "=== Firmware Rollback ==="
    check_usb0
    if [ ! -f "$FW_ORIG" ]; then
      echo "ERROR: No backup firmware found at $FW_ORIG"
      echo "  (backup is created during deploy — nothing to roll back to)"
      exit 1
    fi
    echo "Current firmware: $(md5sum $FW_PATH 2>/dev/null | cut -d' ' -f1)"
    echo "Backup firmware:  $(md5sum $FW_ORIG 2>/dev/null | cut -d' ' -f1)"
    echo ""
    echo "This will restore the original firmware and reboot."
    echo "Press Ctrl-C within 5 seconds to cancel..."
    sleep 5
    sudo cp "$FW_ORIG" "$FW_PATH"
    echo "Firmware restored from backup."
    # Also disable AO overlay since original firmware won't support it
    if [ -f "$OVERLAY" ]; then
      sudo mv "$OVERLAY" "$DISABLED"
      echo "AO config overlay disabled (original firmware doesn't support AO)."
    fi
    echo "Rebooting in 3 seconds..."
    sleep 3
    sudo reboot
    ;;
  status)
    if [ -f "$OVERLAY" ]; then
      echo "Mode: AngryOxide (AO active, bettercap attacks off)"
      if pgrep -f angryoxide > /dev/null 2>&1; then
        PID=$(pgrep -f angryoxide | head -1)
        UPTIME=$(ps -o etimes= -p $PID 2>/dev/null | tr -d ' ')
        echo "  AO PID: $PID (uptime: ${UPTIME}s)"
      else
        echo "  AO: not running"
      fi
    elif [ -f "$DISABLED" ]; then
      echo "Mode: Pwnagotchi (bettercap attacks, AO disabled)"
    else
      echo "Mode: Unknown (overlay not found)"
    fi
    # Interfaces
    if ip link show wlan0mon > /dev/null 2>&1; then
      echo "  wlan0mon: UP"
    else
      echo "  wlan0mon: DOWN"
    fi
    check_usb0
    # Firmware info
    if [ -f "$FW_ORIG" ]; then
      echo "  Firmware backup: exists (rollback available)"
    else
      echo "  Firmware backup: none"
    fi
    # pwnagotchi service
    if systemctl is-active --quiet pwnagotchi; then
      echo "  pwnagotchi: active"
    else
      echo "  pwnagotchi: inactive/failed"
    fi
    ;;
  *)
    echo "Usage: pwnoxide-mode {ao|pwn|status|rollback-fw}"
    echo "  ao          — Enable AngryOxide, disable bettercap attacks"
    echo "  pwn         — Disable AngryOxide, enable bettercap attacks"
    echo "  status      — Show current mode, interfaces, services"
    echo "  rollback-fw — Restore original firmware and reboot"
    exit 1
    ;;
esac
