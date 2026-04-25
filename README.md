# STM32MP157A-DK1 — Embedded Linux with Rust DS3231 I2C Driver

A complete embedded Linux project for the STMicroelectronics **STM32MP157A-DK1** development board: custom ARM cross-compilation toolchain, Buildroot root filesystem, Linux 7.0 kernel with Rust support, custom device tree, and an out-of-tree Rust I2C kernel driver for the DS3231 real-time clock with a full ioctl API.

---

## Hardware Required

| Item | Details |
|------|---------|
| STM32MP157A-DK1 | ARM Cortex-A7 @ 650 MHz, 512 MB DDR3 |
| DS3231 RTC module | I2C, address `0x68`, connected to I2C5 on the board |
| microSD card | ≥ 4 GB |
| Serial console cable | 115200 baud, 3.3 V TTL — or use the board's built-in ST-LINK USB virtual COM |
| Ethernet cable | For network/SSH access after first boot |

---

## Repository Structure

| Path | Contents |
|------|----------|
| [`Bootloader/`](Bootloader/README.md) | Pre-built TF-A and U-Boot binaries; instructions to rebuild from source |
| [`Buildroot/`](Buildroot/README.md) | Buildroot 2025.11 config for ARM Cortex-A7 with musl and OpenSSH |
| [`kernel/`](kernel/README_conf_dtb.md) | Linux 7.0-rc1 config with Rust support; instructions to build the kernel and DS3231 device tree |
| [`rust_driver/`](rust_driver/README_driver.md) | Out-of-tree Rust modules: hello world, minimal I2C driver, and the full DS3231 driver with ioctl API |
| [`README_TOOLCHAIN.md`](README_TOOLCHAIN.md) | Step-by-step guide to build the ARM cross-compilation toolchain with crosstool-ng 1.28.0 |

---

## Complete Step-by-Step Setup Guide

> Everything below is done on a Linux x86_64 host. Each section tells you exactly what to download.

---

### Step 0 — Host Dependencies

Install the packages needed across all steps (Ubuntu / Debian):

```bash
sudo apt update
sudo apt install \
    build-essential git wget curl unzip bc \
    autoconf bison flex texinfo help2man gawk libtool-bin \
    libncurses5-dev libssl-dev gettext python3 python3-pip rsync cpio \
    parted e2fsprogs fdisk \
    clang lld llvm llvm-dev \
    qemu-user minicom
```

Install a Rust nightly toolchain (required to build the kernel with Rust support):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly
cargo install bindgen-cli
```

> Disk space estimate: toolchain ~9 GB during build (recoverable), Buildroot ~10 GB, kernel ~5 GB. Allow at least **30 GB** free.

---

### Step 1 — Cross-Compilation Toolchain

> **Download:** `crosstool-ng` from GitHub — approximately 5 MB clone.

Full instructions: [README_TOOLCHAIN.md](README_TOOLCHAIN.md)

```bash
# 1. Clone and check out the tested release
git clone https://github.com/crosstool-ng/crosstool-ng
cd crosstool-ng
git checkout crosstool-ng-1.28.0

# 2. Build crosstool-ng itself (local install, no root needed)
./bootstrap
./configure --enable-local
make

# 3. Start from a Cortex-A5 sample and tune it
./ct-ng list-samples          # find the arm-cortex_a5 sample name
./ct-ng arm-cortex_a5-linux-gnueabihf   # or the exact name shown above

# 4. Open menuconfig and apply these settings:
./ct-ng menuconfig
```

Settings to change in `menuconfig`:

| Menu | Option | Value |
|------|--------|-------|
| Path and misc options | Try features marked EXPERIMENTAL | ✓ enabled |
| Target options | Emit assembly for CPU | `cortex-a7` |
| Target options | Use specific FPU | `vfpv4` |
| Target options | Floating point | `hardware (FPU)` |
| Toolchain options | Tuple's vendor string | `Neko` |
| Toolchain options | Tuple's alias | `arm-Neko-linux` |
| Operating System | Version of linux | ≤ 6.12 (closest available) |
| C-library | C library | `musl` |
| C compiler | Version of gcc | `14.3.0` |
| Debug facilities | *(all)* | disabled |

```bash
# 5. Build (30 min – several hours depending on your machine)
./ct-ng build

# 6. Add the toolchain to your PATH (add to ~/.bashrc to make it permanent)
export PATH="$HOME/x-tools/arm-Neko-linux-musleafih/bin:$PATH"
export ARCH=arm
export CROSS_COMPILE=arm-Neko-linux-musleafih-

# 7. Verify
arm-Neko-linux-musleafih-gcc --version
```

**Output of this step:** `$HOME/x-tools/arm-Neko-linux-musleafih/` — the complete ARM cross-compilation toolchain.

---

### Step 2 — Bootloader (TF-A + U-Boot)

> **Download:** Trusted Firmware-A and U-Boot source — each a few hundred MB.

Full instructions: [Bootloader/README.md](Bootloader/README.md)

The pre-built binaries are already in `Bootloader/`. Skip to Step 3 if you want to use them directly.

To rebuild from source:

```bash
# --- TF-A ---
git clone https://git.trustedfirmware.org/TF-A/trusted-firmware-a.git
cd trusted-firmware-a && git checkout v2.12.0

make PLAT=stm32mp1 ARCH=aarch32 ARM_ARCH_MAJOR=7 \
     STM32MP_SDMMC=1 \
     DTB_FILE_NAME=stm32mp157a-dk1.dtb \
     all
# → build/stm32mp1/release/tf-a-stm32mp157a-dk1.stm32

# --- U-Boot ---
cd ..
git clone https://source.denx.de/u-boot/u-boot.git
cd u-boot && git checkout v2025.01

make stm32mp157a_dk1_defconfig
make DEVICE_TREE=stm32mp157a-dk1 all
# → u-boot-nodtb.bin, u-boot.dtb

# --- Package into FIP ---
cd ../trusted-firmware-a
make PLAT=stm32mp1 ARCH=aarch32 ARM_ARCH_MAJOR=7 \
     STM32MP_SDMMC=1 \
     DTB_FILE_NAME=stm32mp157a-dk1.dtb \
     BL33=../u-boot/u-boot-nodtb.bin \
     BL33_CFG=../u-boot/u-boot.dtb \
     fip
# → build/stm32mp1/release/fip.bin
```

**Output of this step:**
- `tf-a-stm32mp157a-dk1.stm32` — first-stage bootloader (FSBL)
- `fip.bin` — second-stage bootloader (U-Boot) in a FIP container

---

### Step 3 — Root Filesystem (Buildroot)

> **Download:** Buildroot 2025.11 — tarball is about 10 MB; package downloads add up to several hundred MB.

Full instructions: [Buildroot/README.md](Buildroot/README.md)

```bash
# 1. Download Buildroot
wget https://buildroot.org/downloads/buildroot-2025.11.tar.gz
tar xf buildroot-2025.11.tar.gz
cd buildroot-2025.11

# 2. Apply the repository's configuration
cp /path/to/this/repo/Buildroot/buildroot.config .config
make olddefconfig

# 3. Build (30 min – 2 hours)
make -j$(nproc)
```

**Output of this step:** `output/images/rootfs.ext4` — the complete root filesystem image.

---

### Step 4 — Linux Kernel and Device Tree

> **Download:** Linux kernel source (v7.0-rc1) — the clone is several GB; allow 10–20 minutes.

Full instructions: [kernel/README_conf_dtb.md](kernel/README_conf_dtb.md)

```bash
# 1. Clone the kernel
git clone https://github.com/torvalds/linux.git
cd linux
git checkout v7.0-rc1

# 2. Apply the repository's kernel config
make multi_v7_defconfig
cp /path/to/this/repo/kernel/kernel_linux.config .config
make olddefconfig

# 3. Build the kernel (~20 min on a modern machine)
make -j$(nproc)
# → arch/arm/boot/zImage

# 4. Create the custom device tree (once, follow the instructions in kernel/README_conf_dtb.md)
#    Then build the DTB:
make -j$(nproc) dtbs
# → arch/arm/boot/dts/st/stm32mp157d-dk1.dtb
```

**Output of this step:**
- `arch/arm/boot/zImage` — compressed kernel image
- `arch/arm/boot/dts/st/stm32mp157d-dk1.dtb` — device tree blob including the DS3231

---

### Step 5 — Partition and Flash the SD Card

> **You need:** All outputs from Steps 1–4, plus an SD card. Replace `/dev/sdX` with your actual device (check with `lsblk`).

```bash
# Identify your SD card — do NOT skip this
lsblk
```

> **Warning:** The following commands are destructive. Double-check the device name.

```bash
# Unmount all partitions and zero the first 128 MB
sudo umount /dev/sdX* 2>/dev/null || true
sudo dd if=/dev/zero of=/dev/sdX bs=1M count=128

# Partition the card (GPT, 6 partitions)
sudo parted /dev/sdX
(parted) mklabel gpt
(parted) mkpart fsbl1   0%      4095s
(parted) mkpart fsbl2   4096s   6143s
(parted) mkpart fip     6144s   10239s
(parted) mkpart bootfs  10240s  131071s
(parted) mkpart rootfs  131072s 1179647s
(parted) mkpart data    1179648s 100%
(parted) print
(parted) quit
```

Expected partition table:

```
Number  Start    End      Size     File system  Name    Flags
 1      1049kB   2097kB   1049kB                fsbl1
 2      2097kB   3146kB   1049kB                fsbl2
 3      3146kB   5243kB   2097kB                fip
 4      5243kB   67.1MB   61.9MB                bootfs
 5      67.1MB   604MB    537MB                 rootfs
 6      604MB    (end)    (rest)                data
```

```bash
# Flash the bootloaders
sudo dd if=Bootloader/tf-a-stm32mp157a-dk1.stm32 of=/dev/sdX1 bs=1M conv=fdatasync
sudo dd if=Bootloader/tf-a-stm32mp157a-dk1.stm32 of=/dev/sdX2 bs=1M conv=fdatasync
sudo dd if=Bootloader/fip.bin                     of=/dev/sdX3 bs=1M conv=fdatasync

# Format boot partition and flash the root filesystem
sudo mkfs.ext4 -L boot -O ^metadata_csum /dev/sdX4
sudo dd if=/path/to/buildroot/output/images/rootfs.ext4 of=/dev/sdX5 bs=4M conv=fdatasync
sudo mkfs.ext4 -L data -E nodiscard /dev/sdX6

# Copy the kernel and DTB to the boot partition
sudo mount /dev/sdX4 /mnt
sudo cp /path/to/linux/arch/arm/boot/zImage                          /mnt/
sudo cp /path/to/linux/arch/arm/boot/dts/st/stm32mp157d-dk1.dtb     /mnt/
sudo umount /dev/sdX4
```

---

### Step 6 — First Boot and U-Boot Setup

Insert the SD card and power on the board. Open a serial terminal (115200 8N1) and press any key when you see the countdown.

```text
Hit any key to stop autoboot:  0
STM32MP>
```

Set the IP address and boot command:

```
setenv ipaddr 192.168.0.100
setenv bootargs 'console=ttySTM0,115200 root=/dev/mmcblk0p5 rootfstype=ext4 rw rootwait'
setenv bootcmd 'mmc dev 0; load mmc 0:4 0xc2000000 zImage; load mmc 0:4 0xc4000000 stm32mp157d-dk1.dtb; bootz 0xc2000000 - 0xc4000000'
saveenv
boot
```

---

### Step 7 — Login and Network Setup

Login as `root` (no password on the first boot). Set a password immediately:

```bash
passwd
```

Configure a static IP for Ethernet. Edit `/etc/network/interfaces`:

```bash
nano /etc/network/interfaces
```

Add at the end:

```
auto eth0
iface eth0 inet static
    address 192.168.0.100
    netmask 255.255.255.0
    gateway 192.168.0.1
```

Enable SSH root login. Edit `/etc/ssh/sshd_config`:

```bash
nano /etc/ssh/sshd_config
```

Uncomment and set:

```
PermitRootLogin yes
PasswordAuthentication yes
UsePAM no
```

Mount the data partition persistently as `/root`. Edit `/etc/init.d/rcS`:

```bash
nano /etc/init.d/rcS
```

Add at the end:

```bash
mount -t ext4 -o rw /dev/mmcblk0p6 /root
chmod 700 /root
```

Reboot and connect over SSH:

```bash
reboot
# (on your host)
ssh root@192.168.0.100
```

---

### Step 8 — Build and Deploy the DS3231 Rust Driver

> **Download:** Nothing new — uses the kernel source from Step 4.

Full instructions: [rust_driver/README_driver.md](rust_driver/README_driver.md)

```bash
cd rust_driver/

# Edit Makefile:
#   KDIR     → absolute path to your linux source from Step 4
#   STM_ADDR → your board IP or SSH hostname

# Build the module, copy it to the board, and run the test
make KDIR=/path/to/linux STM_ADDR=192.168.0.100

# Load the module on the board
ssh root@192.168.0.100 "insmod ~/Ds3231.ko && dmesg | tail -5"

# Read time/date/temperature
ssh root@192.168.0.100 "cat /dev/ds3231"
# Example output: 14:32:07 04/25/2026+26.00 C
```

To set the date/time interactively via ioctl:

```bash
# Build and copy the C test program
make test KDIR=/path/to/linux STM_ADDR=192.168.0.100

# Run it on the board
ssh -t root@192.168.0.100 "~/test_ioctl"
```

---

## Hardware Wiring — DS3231 to STM32MP157A-DK1

Connect the DS3231 module to the I2C5 bus on the board's Arduino-compatible expansion connector:

| DS3231 pin | STM32MP157A-DK1 |
|------------|-----------------|
| VCC | 3.3 V (CN6 pin 4 or any 3.3 V header pin) |
| GND | GND (any GND header pin) |
| SDA | I2C5_SDA (Arduino D4 — CN9) |
| SCL | I2C5_SCL (Arduino D5 — CN9) |

The DS3231 I2C address is **0x68**.

---

## Cross-Compilation Quick Reference

Once the toolchain is in your PATH, these three environment variables are all you need for any cross-compilation (kernel, U-Boot, out-of-tree modules, userspace programs):

```bash
export PATH="$HOME/x-tools/arm-Neko-linux-musleafih/bin:$PATH"
export ARCH=arm
export CROSS_COMPILE=arm-Neko-linux-musleafih-
```

Compile a standalone C program:

```bash
arm-Neko-linux-musleafih-gcc -O2 -Wall -o hello hello.c
```

---

## References

- Bootlin — *Embedded Linux system development, STM32MP157 Discovery Kit variant*, April 2026: <https://bootlin.com/doc/training/embedded-linux/embedded-linux-stm32mp1-labs.pdf>
- crosstool-ng documentation: <https://crosstool-ng.github.io/docs/>
- Buildroot user manual: <https://buildroot.org/downloads/manual/manual.html>
- Linux kernel Rust documentation: <https://docs.kernel.org/rust/index.html>
- DS3231 datasheet: [`rust_driver/ds3231.pdf`](rust_driver/ds3231.pdf)
- Project repository: <https://github.com/N3koNoTsuki/Linux_Conf_MP157D_DK1>
