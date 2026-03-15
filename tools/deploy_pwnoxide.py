#!/usr/bin/env python3
"""PwnOxide Package Deployer — one-command installer for Pi Zero 2W.

Deploys the full Oxagotchi/PwnOxide stack to a Raspberry Pi Zero 2W over SSH
in 13 automated steps. Uploads patched BCM43436B0 v5 firmware, the angryoxide
binary, pwnagotchi plugin, config overlay, mode-switcher script, boot splash
service, and e-ink face PNGs, then applies WiFi stability patches and verifies
the entire deployment before optionally rebooting.

Usage
-----
    python deploy_pwnoxide.py [--dry-run] [--no-reboot]

Command-line flags
------------------
--dry-run
    Run only the preflight step (Step 1) to check connectivity, disk space,
    and service state, then exit without making any changes on the Pi.
--no-reboot
    Perform all deploy and verify steps but skip the final reboot (Step 13).
    Useful when you want to inspect the Pi before restarting.

Deployment steps
----------------
 1. Preflight       — Discover Pi via SSH (tries 192.168.137.8, 10.0.0.2, then
                      pi_client.PI_HOST), check disk space, pwnagotchi service
                      status, existing angryoxide install, wlan0mon presence,
                      usb0 tethering lifeline, and NetworkManager state.
 2. Backup firmware — Back up the stock brcmfmac43436-sdio.bin to .bin.orig if
                      no backup already exists.
 3. Upload firmware — Upload the patched v5 firmware (brcmfmac43436-sdio.v5.bin)
                      via SFTP staging, then sudo cp to /lib/firmware/brcm/.
 4. Upload binary   — Upload the cross-compiled angryoxide aarch64 binary to
                      /usr/local/bin/angryoxide and chmod +x.
 5. Upload plugin   — Upload angryoxide_v2.py as angryoxide.py to the
                      pwnagotchi custom-plugins directory.
 6. Upload config   — Upload angryoxide-v5.toml config overlay to
                      /etc/pwnagotchi/conf.d/.
 7. Upload mode switcher — Upload pwnoxide-mode.sh as /usr/local/bin/pwnoxide-mode
                      (chmod +x) for runtime mode management.
 8. Disable iovars  — Stop and disable the obsolete set-iovars.service that
                      conflicts with v5 firmware.
 9. WiFi fixes      — Apply four stability patches to prevent SDIO crashes on
                      WiFi restarts:
                        a. pwnlib: comment out reload_brcm in
                           stop_monitor_interface to avoid driver unload.
                        b. bettercap-launcher: make reload_brcm conditional,
                           only firing when wlan0/wlan0mon are absent.
                        c. rate-limit: add systemd StartLimitBurst=3 /
                           StartLimitIntervalSec=300 drop-in for pwnagotchi.service.
                        d. cache.py: guard access_point with isinstance(..., dict)
                           to fix TypeError on angryoxide handshakes.
10. Upload faces    — Upload all e-ink-processed PNG files from faces/eink/ to
                      /etc/pwnagotchi/custom-plugins/faces/ on the Pi.
11. Deploy splash   — Upload oxagotchi-splash.py and oxagotchi-splash.service,
                      then systemctl enable the boot/shutdown splash service.
12. Verify          — MD5-compare every deployed file against its local source,
                      check executable permissions, count installed face PNGs,
                      confirm WiFi patches are present, and report service
                      enable states for pwnagotchi and oxagotchi-splash.
13. Reboot          — Reboot the Pi and poll SSH (up to 90 s) until it returns,
                      then wait 30 s for pwnagotchi to start and run post-boot
                      checks: usb0 tethering, NetworkManager, wlan0mon,
                      angryoxide process, pwnagotchi service, firmware dmesg,
                      and wlan0 presence (with rollback hint if missing).

Prerequisites
-------------
- Python 3 with ``paramiko`` installed.
- ``pi_client.py`` in the same directory, exporting PI_HOST, PI_USER, PI_PASS.
- All source files present locally (checked at startup before connecting):
    brcmfmac43436-sdio.v5.bin   — patched BCM43436B0 v5 firmware
    angryoxide                   — cross-compiled aarch64 binary (in
                                   ../../angryoxide-build/target/aarch64-unknown-linux-gnu/release/)
    angryoxide_v2.py             — pwnagotchi plugin
    angryoxide-v5.toml           — config overlay
    pwnoxide-mode.sh             — mode switcher script
    oxagotchi-splash.py          — boot splash script
    oxagotchi-splash.service     — systemd unit for splash
- Face PNGs in faces/eink/ (optional; Step 10 warns and continues if absent).
- Pi reachable over SSH on one of the candidate addresses with sudo access.
- usb0 USB gadget tethering strongly recommended so SSH survives WiFi firmware
  replacement.
"""
import sys, os, time, hashlib, socket
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.chdir(os.path.dirname(os.path.abspath(__file__)))

import paramiko
from pi_client import PI_HOST, PI_USER, PI_PASS

# ---------------------------------------------------------------------------
# Color helpers
# ---------------------------------------------------------------------------
_COLOR = hasattr(sys.stdout, 'isatty') and sys.stdout.isatty()

def _c(code, text):
    return f"\033[{code}m{text}\033[0m" if _COLOR else text

def green(t):  return _c("32", t)
def red(t):    return _c("31", t)
def yellow(t): return _c("33", t)
def bold(t):   return _c("1", t)
def cyan(t):   return _c("36", t)

def ok(msg="OK"):      print(f"  {green('[PASS]')} {msg}")
def warn(msg):          print(f"  {yellow('[WARN]')} {msg}")
def fail(msg):          print(f"  {red('[FAIL]')} {msg}")
def info(msg):          print(f"  {msg}")
def step(n, msg):       print(f"\n{bold(f'[{n}/13]')} {msg}")
def abort(msg):
    fail(msg)
    print(f"\n{red('Aborting.')}")
    sys.exit(1)

# ---------------------------------------------------------------------------
# Constants — file mappings
# ---------------------------------------------------------------------------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
STAGING = "/home/pi/staging"

# (local_path, staging_name, final_dest, chmod)
FILE_MAP = {
    "firmware": (
        os.path.join(SCRIPT_DIR, "brcmfmac43436-sdio.v5.bin"),
        "brcmfmac43436-sdio.v5.bin",
        "/lib/firmware/brcm/brcmfmac43436-sdio.bin",
        None,
    ),
    "angryoxide": (
        os.path.join(SCRIPT_DIR, "..", "..", "angryoxide-build", "target",
                     "aarch64-unknown-linux-gnu", "release", "angryoxide"),
        "angryoxide",
        "/usr/local/bin/angryoxide",
        "+x",
    ),
    "plugin": (
        os.path.join(SCRIPT_DIR, "angryoxide_v2.py"),
        "angryoxide.py",  # renamed on deploy
        "/etc/pwnagotchi/custom-plugins/angryoxide.py",
        None,
    ),
    "config": (
        os.path.join(SCRIPT_DIR, "angryoxide-v5.toml"),
        "angryoxide-v5.toml",
        "/etc/pwnagotchi/conf.d/angryoxide-v5.toml",
        None,
    ),
    "modeswitcher": (
        os.path.join(SCRIPT_DIR, "pwnoxide-mode.sh"),
        "pwnoxide-mode",
        "/usr/local/bin/pwnoxide-mode",
        "+x",
    ),
    "splash_script": (
        os.path.join(SCRIPT_DIR, "oxagotchi-splash.py"),
        "oxagotchi-splash.py",
        "/usr/local/bin/oxagotchi-splash.py",
        "+x",
    ),
    "splash_service": (
        os.path.join(SCRIPT_DIR, "oxagotchi-splash.service"),
        "oxagotchi-splash.service",
        "/etc/systemd/system/oxagotchi-splash.service",
        None,
    ),
}

# Faces directory (eink-processed PNGs)
FACES_LOCAL_DIR = os.path.join(SCRIPT_DIR, "faces", "eink")
FACES_REMOTE_DIR = "/etc/pwnagotchi/custom-plugins/faces"

FW_ORIG = "/lib/firmware/brcm/brcmfmac43436-sdio.bin.orig"
HOSTS_TO_TRY = ["192.168.137.8", "10.0.0.2", PI_HOST]

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
def run(ssh, cmd, timeout=10):
    """Execute a command over SSH and return (stdout, stderr)."""
    stdin, stdout, stderr = ssh.exec_command(cmd, timeout=timeout)
    out = stdout.read().decode().strip()
    err = stderr.read().decode().strip()
    return out, err

def md5_local(path):
    h = hashlib.md5()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()

def try_connect(host, user, password, timeout=5):
    """Try SSH connect to host; return SSHClient or None."""
    try:
        ssh = paramiko.SSHClient()
        ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
        ssh.connect(host, username=user, password=password,
                    timeout=timeout, banner_timeout=timeout)
        return ssh
    except (paramiko.ssh_exception.SSHException, socket.error, OSError):
        return None

def discover_pi():
    """Try each candidate host in order; return (ssh, host) or abort."""
    for host in HOSTS_TO_TRY:
        info(f"Trying {host}...")
        ssh = try_connect(host, PI_USER, PI_PASS)
        if ssh:
            ok(f"Connected to {host}")
            return ssh, host
    abort("Could not reach Pi on any known address.")

# ---------------------------------------------------------------------------
# Steps
# ---------------------------------------------------------------------------

def step1_preflight(dry_run):
    """Preflight: discover Pi, check disk space, check pwnagotchi service."""
    step(1, "Preflight")

    ssh, host = discover_pi()

    out, _ = run(ssh, "df -h /")
    info("Disk space:")
    for line in out.splitlines():
        info(f"  {line}")

    svc_out, _ = run(ssh, "systemctl is-enabled pwnagotchi 2>/dev/null || echo not-found")
    info(f"pwnagotchi service: {svc_out}")

    ao_out, _ = run(ssh, "which angryoxide 2>/dev/null || echo not-installed")
    info(f"angryoxide binary: {ao_out}")

    iface_out, _ = run(ssh, "ip link show wlan0mon 2>/dev/null | head -1 || echo no-wlan0mon")
    info(f"wlan0mon: {iface_out}")

    # Critical: verify usb0 tethering is up — our SSH lifeline
    usb0_out, _ = run(ssh, "ip addr show usb0 2>/dev/null | grep 'inet ' || echo no-usb0")
    if "no-usb0" in usb0_out:
        warn("usb0 interface not found — SSH may rely on WiFi only!")
        warn("If firmware flash fails, Pi could become unreachable.")
        warn("Consider connecting via USB gadget tethering first.")
    else:
        ok(f"usb0 tethering active: {usb0_out.strip()}")

    # Check NetworkManager status (we never touch it, but confirm it's managing usb0)
    nm_out, _ = run(ssh, "systemctl is-active NetworkManager 2>/dev/null || echo inactive")
    info(f"NetworkManager: {nm_out}")

    if dry_run:
        print(f"\n{cyan('--dry-run specified, exiting without changes.')}")
        ssh.close()
        sys.exit(0)

    return ssh, host


def step2_backup_firmware(ssh):
    """Backup original firmware if not already backed up."""
    step(2, "Backup firmware")

    out, _ = run(ssh, f"test -f {FW_ORIG} && echo exists || echo missing")
    if out == "exists":
        ok(f"Backup already exists at {FW_ORIG}")
    else:
        info("Creating firmware backup...")
        _, err = run(ssh, f"sudo cp /lib/firmware/brcm/brcmfmac43436-sdio.bin {FW_ORIG}")
        if err:
            warn(f"Backup may have failed: {err}")
        else:
            ok(f"Backed up to {FW_ORIG}")


def step3_upload_firmware(ssh, sftp):
    """Upload v5 firmware."""
    step(3, "Upload v5 firmware")
    _upload_and_install(ssh, sftp, "firmware")


def step4_upload_binary(ssh, sftp):
    """Upload angryoxide binary."""
    step(4, "Upload angryoxide binary")
    _upload_and_install(ssh, sftp, "angryoxide")


def step5_upload_plugin(ssh, sftp):
    """Upload pwnagotchi plugin."""
    step(5, "Upload plugin")
    _upload_and_install(ssh, sftp, "plugin")


def step6_upload_config(ssh, sftp):
    """Upload config overlay."""
    step(6, "Upload config overlay")
    _upload_and_install(ssh, sftp, "config")


def step7_upload_modeswitcher(ssh, sftp):
    """Upload mode switcher script."""
    step(7, "Upload mode switcher")
    _upload_and_install(ssh, sftp, "modeswitcher")


def _upload_and_install(ssh, sftp, key):
    """Upload a file to staging, then sudo cp to final destination."""
    local, staging_name, dest, chmod = FILE_MAP[key]

    if not os.path.isfile(local):
        abort(f"Local file not found: {local}")

    size = os.path.getsize(local)
    info(f"Local: {local} ({size:,} bytes)")

    staging_path = f"{STAGING}/{staging_name}"

    # Ensure staging dir exists
    run(ssh, f"mkdir -p {STAGING}")

    info(f"Uploading to {staging_path}...")
    sftp.put(local, staging_path)
    ok("Uploaded")

    info(f"Installing to {dest}...")
    # Ensure destination directory exists
    dest_dir = os.path.dirname(dest)
    run(ssh, f"sudo mkdir -p {dest_dir}")

    _, err = run(ssh, f"sudo cp {staging_path} {dest}")
    if err:
        abort(f"Install failed: {err}")
    ok(f"Installed to {dest}")

    if chmod:
        run(ssh, f"sudo chmod {chmod} {dest}")
        ok(f"chmod {chmod}")


def step8_disable_iovars(ssh):
    """Disable the obsolete set-iovars service."""
    step(8, "Disable obsolete set-iovars service")

    run(ssh, "sudo systemctl disable set-iovars.service 2>/dev/null; "
            "sudo systemctl stop set-iovars.service 2>/dev/null")
    ok("set-iovars service disabled/stopped (or was already absent)")


def step9_apply_wifi_fixes(ssh):
    """Apply WiFi stability fixes to prevent SDIO crash on restarts."""
    step(9, "Apply WiFi stability fixes")

    # Fix 1: Comment out reload_brcm in stop_monitor_interface
    out, err = run(ssh, "grep -c '#.*reload_brcm' /usr/bin/pwnlib")
    if '1' in out:
        ok("pwnlib already patched")
    else:
        run(ssh, "sudo sed -i '/stop_monitor_interface/,/^}$/ s/^\\(\\s*reload_brcm\\)/#\\1  # disabled: causes SDIO crash/' /usr/bin/pwnlib")
        ok("Patched pwnlib: disabled reload_brcm in stop_monitor_interface")

    # Fix 2: Make reload_brcm conditional in bettercap-launcher
    out, err = run(ssh, "grep -c 'ip link show wlan0' /usr/bin/bettercap-launcher")
    if out.strip() != '0':
        ok("bettercap-launcher already patched")
    else:
        run(ssh, '''sudo sed -i 's/^reload_brcm$/# Only reload driver if WiFi interface is missing\\nif ! ip link show wlan0 \\&>\\/dev\\/null \\&\\& ! ip link show wlan0mon \\&>\\/dev\\/null; then\\n  reload_brcm\\nfi/' /usr/bin/bettercap-launcher''')
        ok("Patched bettercap-launcher: conditional reload_brcm")

    # Fix 3: Add restart rate limit to pwnagotchi service
    run(ssh, "sudo mkdir -p /etc/systemd/system/pwnagotchi.service.d")
    run(ssh, '''sudo tee /etc/systemd/system/pwnagotchi.service.d/rate-limit.conf > /dev/null << 'EOF'
[Service]
StartLimitBurst=3
StartLimitIntervalSec=300
EOF''')
    run(ssh, "sudo systemctl daemon-reload")
    ok("Added restart rate limit (3 per 5min)")

    # Fix 4: Fix cache.py TypeError
    out, err = run(ssh, "grep -c 'isinstance(access_point, dict)' /home/pi/.pwn/lib/python3.13/site-packages/pwnagotchi/plugins/default/cache.py 2>/dev/null || echo 0")
    if out.strip() != '0':
        ok("cache.py already patched")
    else:
        run(ssh, '''sudo sed -i 's/if self.ready:/if self.ready and isinstance(access_point, dict):/' /home/pi/.pwn/lib/python3.13/site-packages/pwnagotchi/plugins/default/cache.py''')
        ok("Patched cache.py: TypeError fix for AO handshakes")


def step10_upload_faces(ssh, sftp):
    """Upload eink-processed face PNGs to the Pi."""
    step(10, "Upload face PNGs")

    if not os.path.isdir(FACES_LOCAL_DIR):
        warn(f"Faces directory not found: {FACES_LOCAL_DIR}")
        return

    faces = [f for f in os.listdir(FACES_LOCAL_DIR) if f.endswith('.png')]
    if not faces:
        warn("No PNG files found in faces/eink/")
        return

    info(f"Found {len(faces)} face PNGs in {FACES_LOCAL_DIR}")
    run(ssh, f"sudo mkdir -p {FACES_REMOTE_DIR}")

    staging_faces = f"{STAGING}/faces"
    run(ssh, f"mkdir -p {staging_faces}")

    for png in sorted(faces):
        local_path = os.path.join(FACES_LOCAL_DIR, png)
        staging_path = f"{staging_faces}/{png}"
        sftp.put(local_path, staging_path)

    run(ssh, f"sudo cp {staging_faces}/*.png {FACES_REMOTE_DIR}/")
    ok(f"Uploaded and installed {len(faces)} faces to {FACES_REMOTE_DIR}")


def step11_deploy_splash(ssh, sftp):
    """Deploy boot/shutdown splash service."""
    step(11, "Deploy boot splash service")

    _upload_and_install(ssh, sftp, "splash_script")
    _upload_and_install(ssh, sftp, "splash_service")

    run(ssh, "sudo systemctl daemon-reload")
    run(ssh, "sudo systemctl enable oxagotchi-splash.service")
    ok("oxagotchi-splash.service enabled")


def step12_verify(ssh):
    """Verify deployed files with md5sums, permissions, and service state."""
    step(12, "Verify deployment")

    print()
    header = f"  {'File':<20} {'Local MD5':<34} {'Remote MD5':<34} {'Match'}"
    print(header)
    print(f"  {'-'*20} {'-'*34} {'-'*34} {'-'*5}")

    all_ok = True
    for key, (local, _, dest, _) in FILE_MAP.items():
        local_md5 = md5_local(local) if os.path.isfile(local) else "N/A"
        remote_md5_out, _ = run(ssh, f"sudo md5sum {dest} 2>/dev/null")
        remote_md5 = remote_md5_out.split()[0] if remote_md5_out else "N/A"
        match = green("YES") if local_md5 == remote_md5 else red("NO")
        if local_md5 != remote_md5:
            all_ok = False
        print(f"  {key:<20} {local_md5:<34} {remote_md5:<34} {match}")

    print()

    # Permissions check
    for label, path in [("angryoxide", "/usr/local/bin/angryoxide"),
                         ("pwnoxide-mode", "/usr/local/bin/pwnoxide-mode"),
                         ("splash-script", "/usr/local/bin/oxagotchi-splash.py")]:
        out, _ = run(ssh, f"ls -la {path} 2>/dev/null || echo missing")
        info(f"{label}: {out}")

    # Faces check
    out, _ = run(ssh, f"ls {FACES_REMOTE_DIR}/*.png 2>/dev/null | wc -l")
    face_count = out.strip()
    if face_count != '0':
        ok(f"{face_count} face PNGs installed in {FACES_REMOTE_DIR}")
    else:
        warn(f"No face PNGs found in {FACES_REMOTE_DIR}")

    # WiFi fixes check
    out, _ = run(ssh, "grep -c '#.*reload_brcm' /usr/bin/pwnlib 2>/dev/null || echo 0")
    if out.strip() != '0':
        ok("pwnlib reload_brcm patch verified")
    else:
        warn("pwnlib reload_brcm patch NOT applied")

    out, _ = run(ssh, "test -f /etc/systemd/system/pwnagotchi.service.d/rate-limit.conf && echo exists || echo missing")
    if out == "exists":
        ok("pwnagotchi restart rate-limit override present")
    else:
        warn("pwnagotchi restart rate-limit NOT configured")

    # Service checks
    svc_out, _ = run(ssh, "systemctl is-enabled pwnagotchi 2>/dev/null || echo unknown")
    info(f"pwnagotchi service: {svc_out}")

    splash_out, _ = run(ssh, "systemctl is-enabled oxagotchi-splash 2>/dev/null || echo unknown")
    info(f"oxagotchi-splash service: {splash_out}")

    if all_ok:
        ok("All files verified")
    else:
        warn("Some files did not match — check table above")

    return all_ok


def step13_reboot(ssh, host, no_reboot):
    """Reboot the Pi and wait for it to come back."""
    step(13, "Reboot and verify")

    if no_reboot:
        info("--no-reboot specified, skipping reboot.")
        info("Run 'sudo reboot' on the Pi manually when ready.")
        return

    info("Rebooting Pi...")
    try:
        ssh.exec_command("sudo reboot", timeout=5)
    except Exception:
        pass
    ssh.close()

    info("Waiting for Pi to come back (polling every 3s, up to 90s)...")
    time.sleep(10)  # give it a moment to actually go down

    ssh2 = None
    for attempt in range(27):  # ~81s of polling after initial 10s wait
        ssh2 = try_connect(host, PI_USER, PI_PASS, timeout=5)
        if ssh2:
            ok(f"Pi is back (attempt {attempt + 1})")
            break
        time.sleep(3)
    else:
        warn("Pi did not come back after 90s. Check manually.")
        return

    info("Waiting 30s for pwnagotchi to start...")
    time.sleep(30)

    # Post-reboot checks
    # First: verify usb0 survived the reboot (our SSH lifeline)
    out, _ = run(ssh2, "ip addr show usb0 2>/dev/null | grep 'inet ' || echo no-usb0")
    if "no-usb0" not in out:
        ok(f"usb0 tethering survived reboot: {out.strip()}")
    else:
        warn("usb0 not found after reboot — but we got SSH, so some path works")

    # NetworkManager still running (we never touch it)
    out, _ = run(ssh2, "systemctl is-active NetworkManager 2>/dev/null || echo inactive")
    if out == "active":
        ok("NetworkManager still active")
    else:
        warn(f"NetworkManager: {out}")

    out, _ = run(ssh2, "ip link show wlan0mon 2>/dev/null | head -2")
    if "wlan0mon" in out:
        ok("wlan0mon interface found")
    else:
        warn("wlan0mon not found — monitor mode may not be active yet")

    out, _ = run(ssh2, "pgrep angryoxide 2>/dev/null")
    if out:
        ok(f"angryoxide running (PID {out})")
    else:
        warn("angryoxide not running — check pwnagotchi logs")

    # pwnagotchi service health
    out, _ = run(ssh2, "systemctl is-active pwnagotchi 2>/dev/null || echo failed")
    if out == "active":
        ok("pwnagotchi service active")
    else:
        warn(f"pwnagotchi service: {out}")
        info("Check with: journalctl -u pwnagotchi -n 50")

    out, _ = run(ssh2, "dmesg | grep -i 'brcmf\\|firmware' | tail -5")
    if out:
        info("Recent firmware dmesg:")
        for line in out.splitlines():
            info(f"  {line}")

    # Rollback hint if things look bad
    out_wlan, _ = run(ssh2, "ip link show wlan0 2>/dev/null || echo missing")
    if "missing" in out_wlan:
        warn("wlan0 base interface missing — firmware may not have loaded!")
        warn("To rollback: ssh pi 'pwnoxide-mode rollback-fw'")
        warn("Or manually: ssh pi 'sudo cp /lib/firmware/brcm/brcmfmac43436-sdio.bin.orig "
             "/lib/firmware/brcm/brcmfmac43436-sdio.bin && sudo reboot'")

    ssh2.close()

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    dry_run = "--dry-run" in sys.argv
    no_reboot = "--no-reboot" in sys.argv

    print(bold("=" * 60))
    print(bold("  Oxagotchi Deployer — PwnOxide Package"))
    print(bold("=" * 60))

    # Preflight check for local files before even connecting
    missing = []
    for key, (local, _, _, _) in FILE_MAP.items():
        if not os.path.isfile(local):
            missing.append(f"  {key}: {local}")
    if missing and not dry_run:
        print(f"\n{red('Missing local files:')}")
        for m in missing:
            print(m)
        abort("Ensure all files exist before deploying.")

    # Step 1
    ssh, host = step1_preflight(dry_run)

    # Open SFTP for upload steps
    sftp = ssh.open_sftp()

    # Steps 2-7: Upload files
    step2_backup_firmware(ssh)
    step3_upload_firmware(ssh, sftp)
    step4_upload_binary(ssh, sftp)
    step5_upload_plugin(ssh, sftp)
    step6_upload_config(ssh, sftp)
    step7_upload_modeswitcher(ssh, sftp)

    # Step 8
    step8_disable_iovars(ssh)

    # Step 9: WiFi stability fixes
    step9_apply_wifi_fixes(ssh)

    # Step 10: Upload faces
    step10_upload_faces(ssh, sftp)

    # Step 11: Deploy splash service
    step11_deploy_splash(ssh, sftp)

    sftp.close()

    # Step 12: Verify
    step12_verify(ssh)

    # Step 13: Reboot
    step13_reboot(ssh, host, no_reboot)

    print(f"\n{bold(green('=== Oxagotchi deployment complete ==='))}")
    print(f"  Your bull is ready. Run: ssh pi 'pwnoxide-mode status'")


if __name__ == "__main__":
    main()
