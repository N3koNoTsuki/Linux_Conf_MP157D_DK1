//! Hello World Rust kernel module.
use kernel::prelude::*;

module! {
    type: HelloDriver,
    name: "hello_driver",
    description: "Hello World Rust module",
    license: "GPL",
}

struct HelloDriver;

impl kernel::Module for HelloDriver {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Hello World from Rust kernel module!\n");
        Ok(Self)
    }
}

impl Drop for HelloDriver {
    fn drop(&mut self) {
        pr_info!("Goodbye!\n");
    }
}

