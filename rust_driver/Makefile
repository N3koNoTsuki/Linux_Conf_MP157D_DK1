# ===== Kbuild part =====
#obj-m += hello_driver.o
obj-m += Ds3231.o
# ===== External build support =====

# Address of the STM32 board for scp and ssh
STM_ADDR ?= stm32

# Kernel directory and build directory
KDIR ?= /home/user/linux
BUILD_DIR ?= build

# Compiler and flags for C test programs
TEST_CC ?= armv7-neko-linux-gnueabihf-gcc
TEST_CFLAGS ?= -Wall -Wextra -O2

# Default target: build the kernel module, copy it to the STM32 board, and clean up
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules
	mkdir -p $(BUILD_DIR)
	cp *.ko $(BUILD_DIR)/
	ssh $(STM_ADDR) "rm -f ~/*.ko"
	scp $(BUILD_DIR)/*.ko $(STM_ADDR):~/
	rm -rf *.cmd *.o *.mod *.ko *.symvers *.order .*.cmd .*.o
	find . -maxdepth 1 -type f -name "*.c" ! -name "test_ioctl.c" -delete
	

test: test_ioctl

test_ioctl: test_ioctl.c
	$(TEST_CC) $(TEST_CFLAGS) $< -o $@
	ssh $(STM_ADDR) "rm -f ~/test_ioctl"
	scp test_ioctl $(STM_ADDR):~/

test-clean:
	rm -f test_ioctl

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
	rm -rf $(BUILD_DIR)

buildclean:
	rm -rf *.cmd *.o *.mod *.ko *.symvers *.order .*.cmd .*.o
	find . -maxdepth 1 -type f -name "*.c" ! -name "test_ioctl.c" -delete
