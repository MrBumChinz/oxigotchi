#!/bin/bash
# Find pwnagotchi remnants that write faces to e-ink on boot/shutdown
# Run on the Pi: bash /home/pi/find_pwn_remnants.sh

set -e
echo "=== Searching for pwnagotchi face strings on disk ==="

echo ""
echo "--- Files containing 'powered off' or shutdown faces ---"
grep -rl "powered off\|Zzz\|zzZ\|(-_-)\|(⇀_⇀)\|ZzzZzzZzz" /usr/local/ /etc/ /opt/ /home/ /var/ /lib/systemd/ 2>/dev/null || echo "(none found)"

echo ""
echo "--- Files containing pwnagotchi display/face writes ---"
grep -rl "set_image\|display\.set\|epd\.display\|view\.update\|ui\.set.*face\|_agent_frame\|_state_frame" /usr/local/ /etc/ /opt/ /home/ /var/ 2>/dev/null || echo "(none found)"

echo ""
echo "--- Pwnagotchi-related systemd services ---"
systemctl list-unit-files 2>/dev/null | grep -i "pwn\|bettercap\|pwnagotchi" || echo "(none found)"

echo ""
echo "--- Active pwnagotchi processes ---"
ps aux 2>/dev/null | grep -i "pwn\|bettercap" | grep -v grep || echo "(none running)"

echo ""
echo "--- Pwnagotchi Python packages ---"
pip3 list 2>/dev/null | grep -i pwn || echo "(none found)"
pip list 2>/dev/null | grep -i pwn || echo "(none found)"

echo ""
echo "--- Pwnagotchi-related files in systemd ---"
ls -la /etc/systemd/system/*pwn* /lib/systemd/system/*pwn* /etc/systemd/system/*bettercap* 2>/dev/null || echo "(none found)"

echo ""
echo "--- ExecStart/ExecStop in pwnagotchi services ---"
for f in /etc/systemd/system/*pwn* /lib/systemd/system/*pwn* 2>/dev/null; do
    if [ -f "$f" ]; then
        echo ">>> $f"
        grep -E "ExecStart|ExecStop|ExecStartPre|ExecStopPost" "$f" 2>/dev/null
    fi
done

echo ""
echo "--- Shutdown/halt hooks ---"
ls -la /lib/systemd/system-shutdown/ /usr/lib/systemd/system-shutdown/ 2>/dev/null || echo "(no shutdown hooks dir)"
for f in /lib/systemd/system-shutdown/* /usr/lib/systemd/system-shutdown/* 2>/dev/null; do
    if [ -f "$f" ]; then
        echo ">>> $f"
        head -20 "$f"
    fi
done

echo ""
echo "--- rc.local or other boot scripts ---"
cat /etc/rc.local 2>/dev/null || echo "(no rc.local)"

echo ""
echo "--- Cron jobs mentioning pwnagotchi ---"
crontab -l 2>/dev/null | grep -i pwn || echo "(no cron)"
ls /etc/cron.d/*pwn* 2>/dev/null || echo "(no cron.d entries)"

echo ""
echo "=== Done. Review above to find what's writing the face ==="
