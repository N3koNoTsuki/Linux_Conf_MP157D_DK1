# Buildroot — Root Filesystem for STM32MP157D-DK1

[Buildroot](https://buildroot.org) is an automated build system that downloads, cross-compiles and packages all software needed for an embedded Linux root filesystem. This directory contains the Buildroot configuration used to produce the `rootfs.ext4` image for the STM32MP157D-DK1 board.

## Files

| File | Description |
|------|-------------|
| `buildroot.config` | Buildroot configuration snapshot (equivalent to a `.config` file). Targets ARM Cortex-A7 with hard-float VFPv4, musl libc, and includes OpenSSH so you can log in over the network after first boot. Generated with **Buildroot 2025.11**. |

## Building the Root Filesystem

### Prerequisites

- The cross-compilation toolchain must be built first. See [../README_TOOLCHAIN.md](../README_TOOLCHAIN.md).
- Approximately 10 GB of free disk space.
- An internet connection (Buildroot will download package sources).

Install host dependencies (Ubuntu/Debian):

```bash
sudo apt install build-essential git wget unzip bc python3 rsync cpio \
    libncurses5-dev libssl-dev flex bison
```

---

### Step 1 — Download Buildroot 2025.11

**Option A — tarball (recommended for reproducibility):**

```bash
wget https://buildroot.org/downloads/buildroot-2025.11.tar.gz
tar xf buildroot-2025.11.tar.gz
cd buildroot-2025.11
```

**Option B — git:**

```bash
git clone https://github.com/buildroot/buildroot.git
cd buildroot
git checkout 2025.11
```

---

### Step 2 — Apply the Configuration

Copy this repository's configuration file into the Buildroot directory and let Buildroot resolve any option changes:

```bash
cp /path/to/this/repo/Buildroot/buildroot.config .config
make olddefconfig
```

If you want to browse or change options (e.g. add packages):

```bash
make menuconfig
```

---

### Step 3 — Build

```bash
make -j$(nproc)
```

Buildroot will download all source packages, cross-compile them for ARM Cortex-A7 and produce the root filesystem image. This typically takes **30 minutes to 2 hours** depending on your machine.

Output file:

```
output/images/rootfs.ext4
```

---

## Key Configuration Highlights

| Option | Value |
|--------|-------|
| Architecture | ARM (Cortex-A7, VFPv4, hard-float) |
| C library | musl |
| Init system | BusyBox init |
| SSH server | OpenSSH (for remote access) |
| Filesystem | ext4 image |

---

## Using the Output

The `rootfs.ext4` image is flashed directly to partition 5 of the SD card. See the root [README.md](../README.md) for the complete SD card setup:

```bash
sudo dd if=output/images/rootfs.ext4 of=/dev/sdX5 bs=4M conv=fdatasync
```

## References

- [Buildroot user manual](https://buildroot.org/downloads/manual/manual.html)
- [Bootlin STM32MP157 Lab Guide](https://bootlin.com/doc/training/embedded-linux/embedded-linux-stm32mp1-labs.pdf) — Root filesystem chapter
