# Kernel — Building Linux and the Device Tree for STM32MP157D-DK1

This directory contains the Linux kernel configuration used to build a kernel with Rust support and DS3231 I2C device-tree integration for the STM32MP157D-DK1 board.

## Files

| File | Description |
|------|-------------|
| `kernel_linux.config` | Full kernel `.config` for Linux 7.0-rc1. Based on `multi_v7_defconfig` with Rust support enabled (`CONFIG_RUST=y`), compiled with crosstool-NG 1.28.0 / gcc 14.3.0. Drop this file on top of a fresh `make multi_v7_defconfig` to reproduce the exact build. |

---

## Step-by-Step Build Guide

### 1. Set Up the Cross-Compilation Environment

The custom toolchain must be built first. See [../README_TOOLCHAIN.md](../README_TOOLCHAIN.md).

```bash
export ARCH=arm
export CROSS_COMPILE=arm-Neko-linux-musleafih-
export PATH="$HOME/x-tools/arm-Neko-linux-musleafih/bin:$PATH"
```

For Rust support you also need `clang`, `bindgen`, `rustc` (rustup-managed, nightly) and `ld.lld`:

```bash
sudo apt install clang lld llvm llvm-dev bindgen
```

---

### 2. Download the Linux Kernel Source

```bash
git clone https://github.com/torvalds/linux.git
cd linux
git checkout v7.0-rc1
```

> You can use a later stable release as long as it is compatible with the board and the Rust bindings used by the driver.

---

### 3. Apply the Kernel Configuration

```bash
make multi_v7_defconfig                               # start from the multi-platform ARMv7 base
cp /path/to/this/repo/kernel/kernel_linux.config .config
make olddefconfig                                     # resolve any new/removed options
```

---

### 4. Build the Kernel

```bash
make -j$(nproc)
```

Output: `arch/arm/boot/zImage`

---

### 5. Create the Custom Device Tree

The DS3231 RTC is connected to I2C5 at address `0x68`. Create a new DTS file that extends the upstream DK1 tree:

```bash
cd arch/arm/boot/dts/st
```

Create `stm32mp157d-dk1.dts`:

```dts
/dts-v1/;
#include "stm32mp157a-dk1.dts"

&i2c5 {
    status = "okay";
    clock-frequency = <100000>;

    ds3231n: ds3231n@68 {
        compatible = "neko,ds3231";
        reg = <0x68>;
        status = "okay";
    };
};
```

Then register it in the Makefile for the `st/` subdirectory. Find the block listing the other `stm32mp157*.dtb` targets and add one line:

```makefile
stm32mp157c-ultra-fly-sbc.dtb \
stm32mp157d-dk1.dtb \          # ← add this line
stm32mp157f-dk2.dtb
```

---

### 6. Build the Device Tree Blob

```bash
cd /path/to/your/linux/source
make -j$(nproc) dtbs
```

Output: `arch/arm/boot/dts/st/stm32mp157d-dk1.dtb`

---

### 7. Copy Files to the SD Card Boot Partition

```bash
# Replace sdX4 with your actual boot partition device
sudo mount /dev/sdX4 /mnt
sudo cp arch/arm/boot/zImage /mnt/
sudo cp arch/arm/boot/dts/st/stm32mp157d-dk1.dtb /mnt/
sudo umount /dev/sdX4
```

---

### 8. Update the U-Boot Boot Command

On first boot, interrupt U-Boot and update `bootcmd` to load the new DTB name:

```
STM32MP> editenv bootcmd
edit: mmc dev 0; load mmc 0:4 0xc2000000 zImage; load mmc 0:4 0xc4000000 stm32mp157d-dk1.dtb; bootz 0xc2000000 - 0xc4000000
STM32MP> saveenv
STM32MP> boot
```

---

### 9. Verify the I2C Device

Once booted, check that the I2C bus with the DS3231 appears:

```bash
i2cdetect -l
```

You should see at least three I2C buses listed. If only two appear, U-Boot is still loading the old DTB — check `bootcmd` again.

---

## References

- [Bootlin STM32MP157 Lab Guide](https://bootlin.com/doc/training/embedded-linux/embedded-linux-stm32mp1-labs.pdf) — Kernel and device tree chapters
- [Linux kernel Rust documentation](https://docs.kernel.org/rust/index.html)
- DS3231 datasheet: `../rust_driver/ds3231.pdf`
