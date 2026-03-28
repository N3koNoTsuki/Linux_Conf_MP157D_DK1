// SPDX-License-Identifier: GPL-2.0
//! DS3231 Rust I2C driver (minimal) — FIX __LOG_PREFIX + lecture secondes.

use core::pin::Pin;
use kernel::prelude::*;
use kernel::{bindings, i2c, of};

const REG_SECONDS: u8 = 0x00;

struct Ds3231 {
    _client: kernel::types::ARef<i2c::I2cClient>,
}

// ---------- OF match table ----------
kernel::of_device_table!(
    OF_TABLE,
    MODULE_OF_TABLE,
    <Ds3231 as i2c::Driver>::IdInfo,
    [
        (of::DeviceId::new(c"nekoi2c,ds3231"), ()),
    ]
);

// ---------- I2C + DS3231 helpers ----------
impl Ds3231 {
    #[inline]
    fn raw_client(&self) -> *mut bindings::i2c_client {
        (&*self._client) as *const i2c::I2cClient as *mut bindings::i2c_client
    }

    #[inline]
    fn read_reg_u8(&self, reg: u8) -> Result<u8> {
        let v = unsafe { bindings::i2c_smbus_read_byte_data(self.raw_client(), reg) };
        if v < 0 {
            Err(kernel::error::Error::from_errno(v))
        } else {
            Ok(v as u8)
        }
    }

    #[inline]
    fn bcd2bin(v: u8) -> u8 {
        (v & 0x0f) + ((v >> 4) * 10)
    }

    fn read_seconds(&self) -> Result<u8> {
        let raw = self.read_reg_u8(REG_SECONDS)?;
        Ok(Self::bcd2bin(raw & 0x7f))
    }
}

// ---------- I2C driver impl ----------
impl i2c::Driver for Ds3231 {
    type IdInfo = ();

    const OF_ID_TABLE: Option<of::IdTable<Self::IdInfo>> = Some(&OF_TABLE);

    fn probe(
        dev: &i2c::I2cClient<kernel::device::Core>,
        _id_info: Option<&Self::IdInfo>,
    ) -> Result<Self> {
        pr_info!("ds3231: probe\n");

        let this = Self {
            _client: dev.into(),
        };

        let sec = this.read_seconds()?;
        pr_info!("ds3231: seconds = {}\n", sec);

        Ok(this)
    }

    fn unbind(_dev: &i2c::I2cClient<kernel::device::Core>, _this: Pin<&Self>) {
        pr_info!("ds3231: unbind\n");
    }

    fn shutdown(_dev: &i2c::I2cClient<kernel::device::Core>, _this: Pin<&Self>) {
        pr_info!("ds3231: shutdown\n");
    }
}

impl Drop for Ds3231 {
    fn drop(&mut self) {
        pr_info!("ds3231: drop\n");
    }
}

// ---------- IMPORTANT: module macro in THIS file (fix __LOG_PREFIX) ----------
kernel::module_i2c_driver! {
    type: Ds3231,
    name: "ds3231_rust",
    authors: ["NekoNoTsuki"],
    description: "Rust I2C driver for DS3231 (seconds read test)",
    license: "GPL",
}
