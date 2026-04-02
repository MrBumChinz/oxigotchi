#!/bin/bash
set -e

IMG="/mnt/d/oxigotchi-release.img"
BIN="/mnt/c/msys64/home/gelum/oxigotchi/rust/target/aarch64-unknown-linux-gnu/release/oxigotchi"
MNT="/mnt/pi_root"

# Clean up any stale loop devices
for dev in /dev/loop*; do
    if sudo losetup "$dev" 2>/dev/null | grep -q oxigotchi; then
        echo "Cleaning stale loop: $dev"
        sudo umount "$MNT" 2>/dev/null || true
        sudo losetup -d "$dev" 2>/dev/null || true
    fi
done

# Set up loop device
LOOP=$(sudo losetup -fP --show "$IMG")
echo "Loop: $LOOP"
sleep 1

# Mount rootfs
sudo mkdir -p "$MNT"
sudo mount "${LOOP}p2" "$MNT"
echo "Mounted"

# Replace binary
sudo cp "$BIN" "$MNT/usr/local/bin/rusty-oxigotchi"
SIZE=$(stat -c%s "$MNT/usr/local/bin/rusty-oxigotchi")
echo "Binary replaced: $SIZE bytes"

# Quick verification
MOOD=$(strings "$MNT/usr/local/bin/rusty-oxigotchi" | grep -c "Mooooood" || true)
DISP=$(strings "$MNT/usr/local/bin/rusty-oxigotchi" | grep -c "Battery Critical" || true)
echo "Mooooood: $MOOD matches"
echo "display_name: $DISP matches"

# Unmount
sudo umount "$MNT"
sudo losetup -d "$LOOP"
echo "Done - image updated"
