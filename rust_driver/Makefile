# ===== Kbuild part =====
#obj-m += hello_driver.o
obj-m += Ds3231.o
# ===== External build support =====

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
