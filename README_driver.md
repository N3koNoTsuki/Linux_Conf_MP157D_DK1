# README for building a rust out of tree module for the STM32MP157D-DK1 board

## 1. Create the workspace

To create driver we need three files : the Makefile, the Kconfig file and the source code in rust. We can create a folder named rust_drivers for our workspace :

```bash
mkdir -p rust_drivers
cd rust_drivers
nano Makefile
```

Then we can add the following content to the Makefile :

```makefile
# kernel directory
KDIR ?= /path/to/your/linux/source
BUILD_DIR ?= build

# make the kernel build the module
all:
    $(MAKE) -C $(KDIR) M=$(PWD) modules
    mkdir -p $(BUILD_DIR)
    cp *.ko $(BUILD_DIR)/ 

clean:
    $(MAKE) -C $(KDIR) M=$(PWD) clean
    rm -rf $(BUILD_DIR)
```

Then we can create the Kconfig file :

```bash
nano Kconfig
```

```Kconfig
rustflags-y += -Cpanic=abort

obj-m += hello_rust.o


hello_rust-rust := hello_rust.rs
```

Finally we can create the source code file :

```rust
// SPDX-License-Identifier: GPL-2.0

use kernel::prelude::*;

module! {
    type: HelloModule,
    name: "hello_rust",
    authors: ["myself"],
    description: "Hello Rust out-of-tree driver",
    license: "GPL",
}

struct HelloModule;

impl kernel::Module for HelloModule {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Hello from an out of tree driver !\n");
        Ok(Self)
    }
}

impl Drop for HelloModule {
    fn drop(&mut self) {
        pr_info!("hello_rust: exit()\n");
    }
}
```

## 2. Build the module

To build the module we can just use the make command at the root of our workspace :

```bash
make
```

Then we can take the genrated .ko file in the build folder and copy it to our board to test it.
If you use the configuration from Neko No Tsuki's [git repository](https://github.com/N3koNoTsuki/Linux_Conf_MP157D_DK1).
You can use the following command to copy the module to your board and test it :

```bash
scp build/hello_rust.ko root@YourBoardIPAddress:~/ 
ssh root@YourBoardIPAddress "insmod ~/hello_rust.ko"
ssh root@YourBoardIPAddress "rmmod ~/hello_rust.ko"
ssh root@YourBoardIPAddress "dmesg | tail -n 2"
```

You should see the following output in the dmesg logs of your board :

```txt
[ 1234.567890] Hello from an out of tree driver !
[ 1234.567891] hello_rust: exit()
```
