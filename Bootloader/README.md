# Bootloader — TF-A and U-Boot for STM32MP157A-DK1

This directory holds the two pre-built bootloader binaries that must be flashed onto the SD card before Linux can boot.

## Files

| File | Description |
|------|-------------|
| `tf-a-stm32mp157a-dk1.stm32` | **Trusted Firmware-A (TF-A)** — first-stage bootloader (FSBL). The very first code that runs when the board powers on. It initializes the Cortex-A7 trust zone, sets up clocks, and loads the second-stage bootloader from the FIP. It is written to both FSBL partitions (for redundancy). |
| `fip.bin` | **Firmware Image Package (FIP)** — a container format that bundles the second-stage bootloader (U-Boot) along with its device tree. U-Boot initializes DRAM, provides an interactive shell, and loads the Linux kernel and DTB from the boot partition. |

## Rebuilding from Source

These binaries were produced by following the Bootlin *Embedded Linux — STM32MP157* training. If you need to rebuild them, follow the steps below. The custom toolchain must be built first — see [../README_TOOLCHAIN.md](../README_TOOLCHAIN.md).

### Prerequisites

```bash
export PATH="$HOME/x-tools/arm-Neko-linux-musleafih/bin:$PATH"
export CROSS_COMPILE=arm-Neko-linux-musleafih-
export ARCH=arm
```

---

### Step 1 — Download Trusted Firmware-A

```bash
git clone https://git.trustedfirmware.org/TF-A/trusted-firmware-a.git
cd trusted-firmware-a
git checkout v2.12.0
```

Build TF-A for the STM32MP1 platform (generates the `.stm32` FSBL):

```bash
make PLAT=stm32mp1 ARCH=aarch32 ARM_ARCH_MAJOR=7 \
     STM32MP_SDMMC=1 \
     DTB_FILE_NAME=stm32mp157a-dk1.dtb \
     all
```

Output: `build/stm32mp1/release/tf-a-stm32mp157a-dk1.stm32`

---

### Step 2 — Download U-Boot

```bash
cd ..
git clone https://source.denx.de/u-boot/u-boot.git
cd u-boot
git checkout v2025.01
```

Build U-Boot for the DK1 board:

```bash
make stm32mp157a_dk1_defconfig
make DEVICE_TREE=stm32mp157a-dk1 all
```

Outputs: `u-boot-nodtb.bin`, `u-boot.dtb`

---

### Step 3 — Package into a FIP

Go back into the TF-A directory and generate the FIP that bundles U-Boot:

```bash
cd ../trusted-firmware-a
make PLAT=stm32mp1 ARCH=aarch32 ARM_ARCH_MAJOR=7 \
     STM32MP_SDMMC=1 \
     DTB_FILE_NAME=stm32mp157a-dk1.dtb \
     BL33=../u-boot/u-boot-nodtb.bin \
     BL33_CFG=../u-boot/u-boot.dtb \
     fip
```

Output: `build/stm32mp1/release/fip.bin`

---

## How These Files Are Used

Both binaries are flashed onto the SD card during the partitioning step. See the root [README.md](../README.md) for the full SD card setup.

```bash
# TF-A written to both FSBL partitions (redundant)
sudo dd if=tf-a-stm32mp157a-dk1.stm32 of=/dev/sdX1 bs=1M conv=fdatasync
sudo dd if=tf-a-stm32mp157a-dk1.stm32 of=/dev/sdX2 bs=1M conv=fdatasync

# FIP (U-Boot) written to the fip partition
sudo dd if=fip.bin of=/dev/sdX3 bs=1M conv=fdatasync
```

## References

- [Bootlin STM32MP157 Lab Guide](https://bootlin.com/doc/training/embedded-linux/embedded-linux-stm32mp1-labs.pdf) — Bootloader chapter
- [TF-A documentation for STM32MP1](https://trustedfirmware-a.readthedocs.io/en/latest/plat/stm32mp1.html)
- [U-Boot STM32 documentation](https://docs.u-boot.org/en/latest/board/st/stm32mp1.html)
