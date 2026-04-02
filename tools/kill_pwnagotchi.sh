#!/bin/bash
# Run on Pi: kills pwnagotchi permanently so only rusty-oxigotchi touches the display
set -e

echo "=== Killing pwnagotchi for good ==="

# Stop if running
sudo systemctl stop pwnagotchi 2>/dev/null || true
sudo systemctl stop bettercap 2>/dev/null || true
echo "  Stopped pwnagotchi + bettercap"

# Mask (symlink to /dev/null) — prevents starting even manually
sudo systemctl mask pwnagotchi 2>/dev/null || true
sudo systemctl mask bettercap 2>/dev/null || true
echo "  Masked pwnagotchi + bettercap"

# Kill the splash service that uses pwnagotchi's Python env
sudo systemctl stop oxigotchi-splash 2>/dev/null || true
sudo systemctl disable oxigotchi-splash 2>/dev/null || true
sudo rm -f /etc/systemd/system/oxigotchi-splash.service 2>/dev/null
sudo rm -f /etc/systemd/system/sysinit.target.wants/oxigotchi-splash.service 2>/dev/null
echo "  Removed oxigotchi-splash.service"

# Remove pwnagotchi drop-ins
sudo rm -rf /etc/systemd/system/pwnagotchi.service.d 2>/dev/null
echo "  Removed pwnagotchi drop-ins"

sudo systemctl daemon-reload
echo "  systemctl daemon-reload done"

echo ""
echo "=== Verification ==="
echo "pwnagotchi: $(systemctl is-enabled pwnagotchi 2>/dev/null || echo 'not found')"
echo "bettercap:  $(systemctl is-enabled bettercap 2>/dev/null || echo 'not found')"
echo "splash:     $(systemctl is-enabled oxigotchi-splash 2>/dev/null || echo 'not found')"
echo ""
echo "Done. Only rusty-oxigotchi will touch the e-ink now."
