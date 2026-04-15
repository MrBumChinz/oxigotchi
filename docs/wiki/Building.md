# Building & Cross-Compilation

← [Back to Wiki Home](Home)

---

## Updating Without Reflashing

You don't need to reflash your SD card for every release. The oxigotchi binary is a single file — just replace it and restart.

### Option 1: Download the pre-built binary (no build tools needed)

The Pi needs internet (BT tether or USB sharing).

```bash
# On the Pi
curl -L -o /home/pi/oxigotchi https://github.com/CoderFX/oxigotchi/releases/latest/download/oxigotchi
sudo systemctl stop rusty-oxigotchi
sudo cp /home/pi/oxigotchi /usr/local/bin/rusty-oxigotchi
sudo chmod +x /usr/local/bin/rusty-oxigotchi
sudo systemctl start rusty-oxigotchi
sudo /usr/local/bin/apply-oxigotchi-patches.sh
```

That's it. The patch script is idempotent — safe to run on every update.

### Option 2: Build and deploy from your PC

Pick the tab that matches your OS.

#### Windows (via WSL)

**One-time setup:**

1. Install [WSL](https://learn.microsoft.com/en-us/windows/wsl/install) with Ubuntu:
   ```powershell
   wsl --install -d Ubuntu
   ```
2. Inside WSL, install Rust + the cross-compiler:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   rustup target add aarch64-unknown-linux-gnu
   sudo apt update && sudo apt install -y gcc-aarch64-linux-gnu libdbus-1-dev:arm64
   ```
3. Clone the repo (accessible from WSL via `/mnt/c/`):
   ```bash
   git clone https://github.com/CoderFX/oxigotchi.git /mnt/c/oxigotchi
   ```

**Build and deploy:**

```powershell
# From a Windows terminal (PowerShell or CMD)
wsl -d Ubuntu -- bash -lc "source ~/.cargo/env && cd /mnt/c/oxigotchi/rust && cargo build --release --target aarch64-unknown-linux-gnu"
```

Then SCP to the Pi. Find your Pi's IP by checking the RNDIS adapter in `ipconfig` (usually `10.0.0.2`):

```powershell
scp \\wsl.localhost\Ubuntu\mnt\c\oxigotchi\rust\target\aarch64-unknown-linux-gnu\release\oxigotchi pi@10.0.0.2:/home/pi/
ssh pi@10.0.0.2 "sudo systemctl stop rusty-oxigotchi && sudo cp /home/pi/oxigotchi /usr/local/bin/rusty-oxigotchi && sudo systemctl start rusty-oxigotchi && sudo apply-oxigotchi-patches.sh"
```

Or from inside WSL/MSYS2:

```bash
scp /mnt/c/oxigotchi/rust/target/aarch64-unknown-linux-gnu/release/oxigotchi pi@10.0.0.2:/home/pi/
ssh pi@10.0.0.2 'sudo systemctl stop rusty-oxigotchi && sudo cp /home/pi/oxigotchi /usr/local/bin/rusty-oxigotchi && sudo systemctl start rusty-oxigotchi && sudo apply-oxigotchi-patches.sh'
```

#### Linux (native)

**One-time setup:**

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install cross-compiler and D-Bus dev headers
rustup target add aarch64-unknown-linux-gnu
sudo apt update && sudo apt install -y gcc-aarch64-linux-gnu libdbus-1-dev:arm64

# Clone
git clone https://github.com/CoderFX/oxigotchi.git ~/oxigotchi
```

**Build and deploy:**

```bash
cd ~/oxigotchi/rust
cargo build --release --target aarch64-unknown-linux-gnu

# Deploy (Pi connected via USB, typically at 10.0.0.2)
scp target/aarch64-unknown-linux-gnu/release/oxigotchi pi@10.0.0.2:/home/pi/
ssh pi@10.0.0.2 'sudo systemctl stop rusty-oxigotchi && sudo cp /home/pi/oxigotchi /usr/local/bin/rusty-oxigotchi && sudo systemctl start rusty-oxigotchi && sudo apply-oxigotchi-patches.sh'
```

### When to reflash instead

Reflash if:
- You're jumping multiple major versions (e.g. v2.x → v3.x)
- The release notes say "reflash recommended" (new systemd services, filesystem layout changes)
- Your SD card is corrupted or you want a clean slate

For minor version bumps (v3.3.3 → v3.3.4), the binary swap + patch script is all you need.

---

## Building a Release Image

If you want to bake a full SD card image (like the ones on the releases page):

```bash
# Requires WSL (Windows) or native Linux with sudo, losetup, zip
# Also needs a base Raspberry Pi OS Lite (64-bit) image
cd oxigotchi
sudo bash tools/bake_release.sh
```

The script handles everything: mounts the image, installs the binary + config + services + plugins + firmware patches, cleans logs/bonds, shrinks the filesystem, and zips the output. Result: `oxigotchi-v{VERSION}-release.img.zip` on your D: drive (Windows) or in the repo root (Linux).

---

## Advanced: Cargo Configuration

The `.cargo/config.toml` in `rust/` pre-configures the aarch64 linker:

```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

### D-Bus cross-compile (multiarch)

The `dbus` crate needs aarch64 D-Bus headers. On Debian/Ubuntu:

```bash
sudo dpkg --add-architecture arm64
# Add arm64 package source (ports.ubuntu.com for Ubuntu, or deb.debian.org for Debian)
sudo apt update
sudo apt install libdbus-1-dev:arm64
```

### Running tests (host)

Tests run on your host machine (x86_64), not the Pi:

```bash
cd rust
cargo test          # all tests
cargo test bluetooth  # just bluetooth tests
cargo clippy        # lint check
```

---

## Install on Existing Pwnagotchi

If you already have a pwnagotchi running on a Pi Zero 2W:

```bash
git clone https://github.com/CoderFX/oxigotchi.git /home/pi/Oxigotchi
cd /home/pi/Oxigotchi/tools
sudo python3 deploy_pwnoxide.py
```

The deployer is an 18-step automated installer. It backs up your existing firmware, installs the Rust binary, sets up systemd services, migrates your config and captures, and disables legacy pwnagotchi/bettercap services.

---

## Release Profile

The `Cargo.toml` release profile is tuned for Pi Zero 2W:

| Setting | Value | Purpose |
|---------|-------|---------|
| `opt-level` | `"z"` | Optimise for binary size |
| `lto` | `true` | Link-time optimisation |
| `codegen-units` | `1` | Single codegen unit for better optimisation |
| `strip` | `true` | Strip debug symbols |
| `panic` | `"abort"` | Abort on panic (saves unwinding code) |

The resulting binary is ~4.5 MB — compared to 150MB+ for the Python/Go stack it replaces.
