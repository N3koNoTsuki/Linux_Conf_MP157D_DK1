# README for building the kernel and the device tree blob (dtb) for the STM32MP157D-DK1 board

## Introduction

This document explain how to build the linux kernel and the device tree with a Ds3231 RTC I2C device support for the STM32MP157D-DK1 board. The steps described here are based on the linux kernel version 7.0-rc1, but they should be similar for other versions of the kernel.

[1.once the toolchaine is built](#1once-the-toolchaine-is-built)  
[2.build the kernel](#2build-the-kernel)  
[3.modify the device tree](#3modify-the-device-tree)  
[4.build the dtb](#4build-the-dtb)  
[5.copy the kernel and the dtb to the sd card](#5copy-the-kernel-and-the-dtb-to-the-sd-card)  
[6.insert the sd card into the board and power it on](#6insert-the-sd-card-into-the-board-and-power-it-on)

## 1.once the toolchaine is built

Use the following commands to set up the environment variables for cross-compilation. Make sure to replace the paths and toolchain prefix with the correct values for your setup.

```bash
export ARCH=arm
export PATH="$PATH:/path/to/your/toolchain/bin"
export CROSS_COMPILE=your-toolchain-prefix-
```

## 2.build the kernel

Now we can build the kernel and the device tree we need. First we need to download the linux kernel source code :

```bash
git clone https://github.com/torvalds/linux.git
cd linux
git checkout v7.0-rc1 # You can also use another vertion of the kernel, but make sure to check the compatibility with your board and the features you need. 
```

Once you have the wanted kernel version, you need to build the kernel using the config file provided by Neko No Tsuki's [git repository](https://github.com/N3koNoTsuki/Linux_Conf_MP157D_DK1).

To build the kernel with the Rust support configuration, you need to be sure to have ``clang``, ``bindgen``, ``rustc`` and ``ld.lld`` installed on your system.
If not installed use :

```bash
sudo apt install bindgen clang lld llvm llvm-dev
```

Then you can copy the configuration file and build the linux kernel :

```bash
cd /path/to/your/linux/source
make multi_v7_defconfig
cp /path/to/the/kernel/config .config
make -j$(nproc)
```

## 3.modify the device tree

To do so we need to create a dts file for our board, then we can compile it to get the dtb file

```bash
cd /path/to/your/linux/source/arch/arm/boot/dts/st
```

Here create a new dts file that include the dts file of the stm32mp157a-dk1 board. For example, we can create a file named stm32mp157d-dk1.dts with the following content:

```dts
/dts-v1/;
#include "stm32mp157a-dk1.dts"

&i2c5 {
    status = "okay";
    clock-frequency = <100000>;

    /* Add your I2C devices here */
    ds3231n: ds3231n@68 {
        compatible = "neko,ds3231";
        reg = <0x68>;
        status = "okay";
    };
};
```

Then we need to modify the Makefile to include our new dts file.

```makefile
...
stm32mp157c-osd32mp1-red.dtb \
stm32mp157c-phycore-stm32mp1-3.dtb \
stm32mp157c-ultra-fly-sbc.dtb \
stm32mp157d-dk1.dtb \ # ← add this line
stm32mp157f-dk2.dtb
...
```

## 4.build the dtb

```bash
cd /path/to/your/linux/source
make -j$(nproc) dtbs
```

## 5.copy the kernel and the dtb to the sd card

Connect the sd card to your computer and copy the previously created files to the boot partition of the sd card.

```bash
# Use the lsblk command to find the device name of your sd card here we assume it is /dev/sdX
# If you have followed the previous steps the boot partition sould be /dev/sdX4 so let's mount this partition
sudo mount /dev/sdX4 /tmp # replace /tmp with the path where you want to mount the partition
cp /path/to/your/linux/source/arch/arm/boot/zImage /tmp
cp /path/to/your/linux/source/arch/arm/boot/dts/st/stm32mp157d-dk1.dtb /tmp
sudo umount /dev/sdX4
```

## 6.insert the sd card into the board and power it on

Now the board should boot but without the i2c device, to check if the i2c device is working you can use the following command:

```bash
i2cdetect -l
```

This command should list the i2c buses. If you see more than two buses that mean that your dtb file has been loaded successfully. Otherwise you should check the bootcmd of U-boot to make sure that the correct dtb file is loaded.

if you have someting like this in the bootcmd:

```bash
STM32MP> editenv bootcmd
edit: mmc dev 0; load mmc 0:4 0xc2000000 zImage; load mmc 0:4 0xc4000000 stm32mp157a-dk1.dtb; bootz 0xc2000000 - 0xc4000000
```

You should change the dtb file name to the one you created:

```bash
STM32MP> editenv bootcmd
edit: mmc dev 0; load mmc 0:4 0xc2000000 zImage; load mmc 0:4 0xc4000000 stm32mp157d-dk1.dtb; bootz 0xc2000000 - 0xc4000000
```
