# ===== Kbuild part =====
obj-m += hello_driver.o
obj-m += nekoi2c_driver.o
# ===== External build support =====
KDIR ?= ~/Documents/Cour/Stm_Dk1/kernel/linux_C/
ARCH ?= arm
CROSS_COMPILE ?= arm-Neko-linux-musleabihf-

BUILD_DIR := $(PWD)/output

all:
	mkdir -p $(BUILD_DIR)
	make -C $(KDIR) \
	ARCH=$(ARCH) \
	CROSS_COMPILE=$(CROSS_COMPILE) \
	M=$(PWD) \
	MO=$(BUILD_DIR) \
	modules

clean:
	rm -rf $(BUILD_DIR)
