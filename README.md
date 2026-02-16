## STM32MP157A-DK1 SD Card Setup

### 0. Prerequisites
- Make sure you have these files in the current folder:
  `tf-a-stm32mp157a-dk1.stm32`, `fip.bin`, `rootfs.ext4`, `zImage`, `stm32mp157a-dk1.dtb`
- Identify your SD card device. The examples use `/dev/sda`.

Important: double-check the device name with `lsblk` before you run any `dd` or `mkfs` commands.

### 1. Partition the SD Card
```bash
sudo umount /dev/sda
sudo dd if=/dev/zero of=/dev/sda bs=1M count=128
sudo parted /dev/sda
```

Inside `parted`:
```text
(parted) mklabel gpt
(parted) mkpart fsbl1 0% 4095s
(parted) mkpart fsbl2 4096s 6143s
(parted) mkpart fip 6144s 10239s
(parted) mkpart bootfs 10240s 131071s
(parted) mkpart rootfs 131072s 1179647s
(parted) mkpart data 1179648s 100%
(parted) print
```

Expected output (example):
```text
Model: Generic- SD/MMC/MS PRO (scsi)
Disk /dev/sda: 31,3GB
Sector size (logical/physical): 512B/512B
Partition Table: gpt
Disk Flags: 

Number  Start   End     Size    File system  Name    Flags
 1      1049kB  2097kB  1049kB               fsbl1
 2      2097kB  3146kB  1049kB               fsbl2
 3      3146kB  5243kB  2097kB               fip
 4      5243kB  67,1MB  61,9MB               bootfs
 5      67,1MB  604MB   537MB                rootfs
 6      604MB   31,3GB  30,7GB               data
```

Exit `parted`:
```text
(parted) quit
```

### 2. Format and Flash
```bash
sudo dd if=tf-a-stm32mp157a-dk1.stm32 of=/dev/sda1 bs=1M conv=fdatasync
sudo dd if=tf-a-stm32mp157a-dk1.stm32 of=/dev/sda2 bs=1M conv=fdatasync
sudo dd if=fip.bin of=/dev/sda3 bs=1M conv=fdatasync
sudo mkfs.ext4 -L boot -O ^metadata_csum /dev/sda4
sudo dd if=rootfs.ext4 of=/dev/sda5 bs=4M conv=fdatasync
sudo mkfs.ext4 -L data -E nodiscard /dev/sda6
```

### 3. Copy zImage and stm32mp157a-dk1.dtb
Mount the `bootfs` partition, copy the files, then unmount.
```bash
sudo mount /dev/sda4 /mnt
sudo cp stm32mp157a-dk1.dtb /mnt/
sudo cp zImage /mnt/
sudo umount /dev/sda4
```

### 4. Boot into U-Boot
Insert the SD card into the STM32, then on boot press any key to stop autoboot.

You should see something like:
```text
****************************************************
* WARNING 500mA power supply detected *
* Current too low, use a 3A power supply! *
****************************************************
Net: eth0: ethernet@5800a000
Hit any key to stop autoboot: 0
STM32MP>
```

### 5. U-Boot Environment
```text
setenv ipaddr 192.168.0.100
setenv bootargs 'console=ttySTM0,115200 root=/dev/mmcblk0p5 rootfstype=ext4 rw rootwait'
setenv bootcmd 'mmc dev 0; load mmc 0:4 0xc2000000 zImage; load mmc 0:4 0xc4000000 stm32mp157a-dk1.dtb; bootz 0xc2000000 - 0xc4000000'
saveenv
```

### 6. Login
Credentials:
- User: `root`

### 7. Setup Network for ssh

Set password for root:
```bash
passwd
```

Edit `/etc/network/interfaces`:
```bash
nano /etc/network/interfaces
```

Add at the end of the file:
```text
auto eth0
iface eth0 inet static
    address 192.168.0.100
    netmask 255.255.255.0
    gateway 192.168.0.1
```

Edit `/etc/ssh/sshd_config`:
```bash
nano /etc/ssh/sshd_config
```
Uncomment and set these options:
```text
PermitRootLogin yes
PasswordAuthentication yes
UsePAM no
```

Setup `/root` on `/dev/mmcblk0p6`
Edit `/etc/init.d/rcS`
```bash
nano /etc/init.d/rcS
```
Add at the end of the file
```text
mount -t ext4 -o rw /dev/mmcblk0p6 /root
chmod 700 /root
```


Reboot:
```bash
reboot
```

SSH:
```bash
ssh root@192.168.0.100
```

### 8. Compilation (Toolchain)
Use the toolchain by either exporting its path or using the full prefix `arm-Neko-linux-musleafih`.

Option A: export the toolchain path
```bash
export PATH="/path/to/toolchain/bin:$PATH"
arm-Neko-linux-musleafih-gcc --version
```

Option B: use the full prefix directly
```bash
/path/to/toolchain/bin/arm-Neko-linux-musleafih-gcc --version
```

Example: compile a simple program
```bash
arm-Neko-linux-musleafih-gcc -O2 -Wall -o hello hello.c
```
