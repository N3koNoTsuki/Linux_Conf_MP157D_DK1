# rust_driver — Out-of-Tree Rust Kernel Modules for DS3231

This directory contains a progression of out-of-tree Rust kernel modules culminating in a full I2C driver for the DS3231 real-time clock, plus a userspace test program. The kernel must have been built with `CONFIG_RUST=y` — see [../kernel/README_conf_dtb.md](../kernel/README_conf_dtb.md).

## Files

| File | Description |
|------|-------------|
| `hello_driver.rs` | Minimal "Hello World" Rust kernel module. Useful to verify that the out-of-tree Rust module build chain works before attempting a real driver. |
| `nekoi2c_driver.rs` | Minimal Rust I2C driver that probes a DS3231 via the OF device table and reads the raw seconds register. A stepping stone to understand the `i2c::Driver` trait. |
| `Ds3231.rs` | **Full production driver.** Registers both an I2C driver and a `/dev/ds3231` misc device. Exposes complete time, date and temperature via `read()` (ASCII string `HH:MM:SS MM/DD/CCYY±TT.FF C\n`) and a rich `ioctl()` API (`DS3231_GET_*` / `DS3231_SET_*`). |
| `Makefile` | Drives the module build, copy to the board via SCP, and compilation of the C test program. See configuration variables below. |
| `test_ioctl.c` | Interactive C userspace program that exercises every SET and GET ioctl command. Prompts you for a date/time, writes it to the RTC, reads it back, and prints the result. Cross-compiled for ARM with the custom toolchain. |
| `ds3231.pdf` | DS3231 datasheet — register map, BCD encoding details, I2C protocol, temperature sensor specs. |

---

## Makefile Configuration Variables

Edit the top of `Makefile` before running any target:

| Variable | Default | Description |
|----------|---------|-------------|
| `KDIR` | `/home/user/linux` | Absolute path to your built Linux kernel source tree |
| `STM_ADDR` | `stm32` | SSH hostname or IP of the board (used by `scp` and `ssh` targets) |
| `TEST_CC` | `armv7-neko-linux-gnueabihf-gcc` | Cross-compiler for the C test program |
| `TEST_CFLAGS` | `-Wall -Wextra -O2` | Compiler flags for the C test program |

Example minimal override:

```bash
make KDIR=/home/neko/linux STM_ADDR=192.168.0.100
```

---

## Prerequisites

1. **Toolchain in PATH** — see [../README_TOOLCHAIN.md](../README_TOOLCHAIN.md):
   ```bash
   export PATH="$HOME/x-tools/arm-Neko-linux-musleafih/bin:$PATH"
   export ARCH=arm
   export CROSS_COMPILE=arm-Neko-linux-musleafih-
   ```

2. **Kernel built with Rust support** — see [../kernel/README_conf_dtb.md](../kernel/README_conf_dtb.md). The kernel source directory pointed to by `KDIR` must already have been built.

3. **Board reachable over SSH** at the address set in `STM_ADDR`.

4. **Module in the active DTB** — the device tree must declare the DS3231 at I2C5 address `0x68` with `compatible = "neko,ds3231"`. See the kernel README for the DTS snippet.

---

## Building and Deploying Ds3231.rs

The default `make` target builds `Ds3231.rs`, copies the `.ko` to the board via SCP, and cleans intermediate build artefacts:

```bash
make KDIR=/path/to/linux STM_ADDR=192.168.0.100
```

What happens internally:
1. `make -C $KDIR M=$(PWD) modules` — kernel builds the `.ko`.
2. Copies `*.ko` into `build/`.
3. SCPs `build/*.ko` to `~/` on the board.
4. Removes local build artefacts (`*.cmd`, `*.o`, `*.mod`, …).

---

## Building and Deploying hello_driver.rs

To switch to `hello_driver.rs`, edit the first line of the Kbuild section in `Makefile`:

```makefile
obj-m += hello_driver.o   # uncomment this
#obj-m += Ds3231.o        # comment this out
```

Then run `make` as normal. The Hello World module is useful to confirm the toolchain / kernel pairing works before debugging a real driver.

---

## Building and Deploying nekoi2c_driver.rs

`nekoi2c_driver.rs` is the minimal I2C driver. To build it add its object to the Kbuild section similarly:

```makefile
obj-m += nekoi2c_driver.o
```

---

## Testing the Full DS3231 Driver

### Load / unload the module

```bash
ssh root@192.168.0.100 "insmod ~/Ds3231.ko"
ssh root@192.168.0.100 "dmesg | tail -5"   # should show "ds3231: probe"
ssh root@192.168.0.100 "rmmod Ds3231"
```

### Read via the misc device

```bash
ssh root@192.168.0.100 "cat /dev/ds3231"
# Output example:
# 14:32:07 04/25/2026+26.00 C
```

### Interactive ioctl test

Build and deploy the C test program (requires the toolchain in PATH):

```bash
make test KDIR=/path/to/linux STM_ADDR=192.168.0.100
```

Then run it on the board:

```bash
ssh -t root@192.168.0.100 "~/test_ioctl"
```

The program will prompt you for each time/date field (seconds, minutes, 12h/24h mode, hours, AM/PM, day, date, month, year), write them to the RTC via ioctl SET commands, read them back via GET commands, and print the complete result.

---

## IOCTL Command Reference

All commands use magic number `'d'` (0x64). Include the same `#define` block as `test_ioctl.c` in your own userspace programs.

| Command | Direction | Type | Description |
|---------|-----------|------|-------------|
| `DS3231_GET_SECONDS` | kernel → user | `u8` | Seconds (0–59) |
| `DS3231_GET_MINUTES` | kernel → user | `u8` | Minutes (0–59) |
| `DS3231_GET_HOURS` | kernel → user | `u8` | Hours (0–23 in 24h, 1–12 in 12h) |
| `DS3231_GET_PM` | kernel → user | `u8` | 0 = AM, 1 = PM (12h mode only) |
| `DS3231_GET_DAYS` | kernel → user | `u8` | Day of week (1–7) |
| `DS3231_GET_DATE` | kernel → user | `u8` | Day of month (1–31) |
| `DS3231_GET_MONTH` | kernel → user | `u8` | Month (1–12) |
| `DS3231_GET_YEAR` | kernel → user | `u16` | Full year (e.g. 2026) |
| `DS3231_GET_TEMP` | kernel ↔ user | `i16` | Temperature in Q4 fixed-point (divide by 4 for °C) |
| `DS3231_SET_SECONDS` | user → kernel | `u8` | Set seconds (0–59) |
| `DS3231_SET_MINUTES` | user → kernel | `u8` | Set minutes (0–59) |
| `DS3231_SET_HOURS` | user ↔ kernel | `u8` | Set hours (range depends on 12h/24h mode) |
| `DS3231_SET_12H` | user ↔ kernel | `u8` | Switch mode: 0 = 24h, 1 = 12h |
| `DS3231_SET_PM` | user ↔ kernel | `u8` | Set AM/PM (12h mode only; returns EPERM in 24h mode) |
| `DS3231_SET_DAYS` | user → kernel | `u8` | Set day of week (1–7) |
| `DS3231_SET_DATE` | user → kernel | `u8` | Set day of month (1–31) |
| `DS3231_SET_MONTH` | user ↔ kernel | `u8` | Set month (1–12) |
| `DS3231_SET_YEAR` | user ↔ kernel | `u16` | Set full year (1900–2099) |

---

## Make Targets Summary

| Target | Action |
|--------|--------|
| `make` | Build module, SCP to board, clean intermediates |
| `make test` | Cross-compile `test_ioctl.c` and SCP to board |
| `make clean` | Full kernel-module clean (removes `build/`) |
| `make buildclean` | Remove local intermediates only (keep `build/`) |
| `make test-clean` | Remove the compiled `test_ioctl` binary |

---

## References

- DS3231 datasheet: `ds3231.pdf` (register map and BCD encoding)
- [Linux kernel Rust bindings — I2C](https://rust.docs.kernel.org/kernel/i2c/index.html)
- [Linux kernel Rust bindings — MiscDevice](https://rust.docs.kernel.org/kernel/miscdevice/index.html)
