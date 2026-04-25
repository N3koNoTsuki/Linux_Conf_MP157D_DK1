# STM32MP157D-DK1 — Embedded Linux with Rust DS3231 I2C Driver

An embedded Linux project for the STMicroelectronics **STM32MP157D-DK1** development board. It covers the full software stack: custom ARM cross-compilation toolchain, Buildroot root filesystem, Linux kernel with Rust support, custom device tree, and an out-of-tree Rust I2C kernel driver for the DS3231 real-time clock.

---

## Project Overview

The goal is to run a mainline Linux kernel on the STM32MP157D-DK1 (ARM Cortex-A7) and interact with a DS3231 RTC chip connected over I2C, driven entirely by a Rust kernel module. The driver exposes time, date and temperature to userspace via a misc device (`/dev/ds3231`) with both a `read()` ASCII interface and a full `ioctl()` API.

---

## Repository Structure

| Path | Contents |
|------|----------|
| [`Bootloader/`](Bootloader/README.md) | Pre-built TF-A and U-Boot binaries (FSBL + FIP), with instructions to rebuild from source |
| [`Buildroot/`](Buildroot/README.md) | Buildroot 2025.11 configuration for the root filesystem (ARM Cortex-A7, musl, OpenSSH) |
| [`kernel/`](kernel/README_conf_dtb.md) | Linux 7.0-rc1 kernel config with Rust support, and instructions to build the kernel and DS3231 device tree |
| [`rust_driver/`](rust_driver/README_driver.md) | Out-of-tree Rust kernel modules: hello world, minimal I2C driver, and the full DS3231 driver with ioctl API |
| [`README_TOOLCHAIN.md`](README_TOOLCHAIN.md) | Instructions to build the ARM cross-compilation toolchain using crosstool-ng 1.28.0 |

---

## Hardware

- **Board:** STM32MP157D-DK1 (ARM Cortex-A7 @ 650 MHz, 512 MB DDR3)
- **RTC:** DS3231 module connected to the I2C5 bus (address `0x68`)
- **Storage:** microSD card (≥ 4 GB)

---

## References

- Bootlin — *Embedded Linux system development, STM32MP157 Discovery Kit variant*, April 2026: <https://bootlin.com/doc/training/embedded-linux/embedded-linux-stm32mp1-labs.pdf>
- DS3231 datasheet: [`rust_driver/ds3231.pdf`](rust_driver/ds3231.pdf)
- Project repository: <https://github.com/N3koNoTsuki/Linux_Conf_MP157D_DK1>
