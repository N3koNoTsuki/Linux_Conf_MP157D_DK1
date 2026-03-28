// SPDX-License-Identifier: GPL-2.0
//! DS3231 I2C driver with a misc device interface.
#![allow(non_snake_case)]
#![allow(static_mut_refs)]

use core::{ffi::c_void, mem::size_of, pin::Pin}; 

use kernel::{
    alloc::{flags::GFP_KERNEL, KBox},
    bindings, c_str,
    fs::{File, Kiocb},
    i2c,
    iov::IovIterDest,
    ioctl::_IOR,
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    of,
    prelude::*,
    types::ARef,
};

const REG_SECONDS:              u8 = 0x00;
const REG_MINUTES:              u8 = 0x01;
const REG_HOURS:                u8 = 0x02;
const REG_DAYS:                 u8 = 0x03; 
const REG_DATE:                 u8 = 0x04;
const REG_MONTH_CENTURY:        u8 = 0x05;
const REG_YEAR:                 u8 = 0x06;
const REG_ALARM1_SECONDS:       u8 = 0x07;
const REG_ALARM1_MINUTES:       u8 = 0x08;
const REG_ALARM1_HOURS:         u8 = 0x09;
const REG_ALARM1_DAYS_DATE:     u8 = 0x0A;
const REG_ALARM2_MINUTES:       u8 = 0x0B;
const REG_ALARM2_HOURS:         u8 = 0x0C;
const REG_ALARM2_DAYS_DATE:     u8 = 0x0D;
const REG_CONTROL:              u8 = 0x0E;
const REG_CONTROL_STATUS:       u8 = 0x0F;
const REG_AGING_OFFSET:         u8 = 0x10;
const REG_TEMP_MSB:             u8 = 0x11;
const REG_TEMP_LSB:             u8 = 0x12;
const DS3231_IOCTL_GET_SECONDS: u32 = _IOR::<u8>('d' as u32, 0x01);

kernel::module_i2c_driver! {
    type: Ds3231Driver,
    name: "ds3231",
    authors: ["Quentin"],
    description: "DS3231 I2C driver with misc device bridge",
    license: "GPL",
}

static mut GLOBAL_CLIENT: Option<ARef<i2c::I2cClient>> = None;

#[pin_data]
struct Ds3231File {
    client: ARef<i2c::I2cClient>,
}

#[pin_data(PinnedDrop)]
struct Ds3231Driver {
    #[pin]
    misc: MiscDeviceRegistration<Ds3231File>,
}

kernel::of_device_table!(
    OF_TABLE,
    MODULE_OF_TABLE,
    <Ds3231Driver as i2c::Driver>::IdInfo,
    [
        (of::DeviceId::new(c"nekoi2c,ds3231"), ()),
    ]
);

#[inline]
fn raw_client(client: &ARef<i2c::I2cClient>) -> *mut bindings::i2c_client {
    (&**client) as *const i2c::I2cClient as *mut bindings::i2c_client
}

#[inline]
fn read_reg_u8(client: &ARef<i2c::I2cClient>, reg: u8) -> Result<u8> {
    let value = unsafe { bindings::i2c_smbus_read_byte_data(raw_client(client), reg) };
    if value < 0 {
        Err(Error::from_errno(value))
    } else {
        Ok(value as u8)
    }
}

#[inline]
fn bcd2bin(v: u8) -> u8 {
    (v & 0x0f) + ((v >> 4) * 10)
}

fn read_seconds(client: &ARef<i2c::I2cClient>) -> Result<u8> {
    let raw = read_reg_u8(client, REG_SECONDS)?;
    Ok(bcd2bin(raw & 0x7f))
}

fn read_minutes(client: &ARef<i2c::I2cClient>) -> Result<u8> {
    let raw = read_reg_u8(client, REG_MINUTES)?;
    Ok(bcd2bin(raw & 0x7f))
}

fn read_hours(client: &ARef<i2c::I2cClient>) -> Result<u8> {
    let raw = read_reg_u8(client, REG_HOURS)?;
    Ok(bcd2bin(raw & 0x7f))
}

fn format_hms(hours: u8, minutes: u8, seconds: u8) -> [u8; 9] {
    [
        b'0' + (hours / 10),
        b'0' + (hours % 10),
        b':',
        b'0' + (minutes / 10),
        b'0' + (minutes % 10),
        b':',
        b'0' + (seconds / 10),
        b'0' + (seconds % 10),
        b'\n',
    ]
}

#[vtable]
impl MiscDevice for Ds3231File {
    type Ptr = Pin<KBox<Self>>;

    fn open(_file: &File, _misc: &MiscDeviceRegistration<Self>) -> Result<Self::Ptr> {
        let client = unsafe { GLOBAL_CLIENT.clone() }.ok_or(ENODEV)?;

        KBox::try_pin_init(
            try_pin_init!(Self {
                client: client,
            }),
            GFP_KERNEL,
        )
    }

    fn ioctl(me: Pin<&Self>, _file: &File, cmd: u32, arg: usize) -> Result<isize> {
        match cmd {
            DS3231_IOCTL_GET_SECONDS => {
                let seconds = read_seconds(&me.client)?;
                let res = unsafe {
                    bindings::_copy_to_user(
                        arg as *mut c_void,
                        (&seconds as *const u8).cast::<c_void>(),
                        size_of::<u8>(),
                    )
                };
                if res != 0 {
                    return Err(EFAULT);
                }
                Ok(0)
            }
            _ => Err(ENOTTY),
        }
    }

    fn read_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterDest<'_>,) -> Result<usize> {
        let client = unsafe { GLOBAL_CLIENT.clone() }.ok_or(ENODEV)?;
        let seconds = read_seconds(&client)?;
        let minutes = read_minutes(&client)?;
        let hours = read_hours(&client)?;
        let data = format_hms(hours, minutes, seconds);
        let read = iov.simple_read_from_buffer(kiocb.ki_pos_mut(), &data)?;
        Ok(read)
    }
}

#[pinned_drop]
impl PinnedDrop for Ds3231Driver {
    fn drop(self: Pin<&mut Self>) {
        unsafe { GLOBAL_CLIENT = None };
        pr_info!("ds3231: remove\n");
    }
}

impl i2c::Driver for Ds3231Driver {
    type IdInfo = ();

    const OF_ID_TABLE: Option<of::IdTable<Self::IdInfo>> = Some(&OF_TABLE);

    fn probe(
        dev: &i2c::I2cClient<kernel::device::Core>,
        _id_info: Option<&Self::IdInfo>,
    ) -> impl PinInit<Self, Error> {
        let client: ARef<i2c::I2cClient> = dev.into();
        unsafe { GLOBAL_CLIENT = Some(client.clone()) };

        pr_info!("ds3231: probe\n");

        try_pin_init!(Self {
            misc <- MiscDeviceRegistration::register(MiscDeviceOptions {
                name: c_str!("ds3231"),
            }),
        })
    }

    fn unbind(_dev: &i2c::I2cClient<kernel::device::Core>, _this: Pin<&Self>) {
        pr_info!("ds3231: unbind\n");
    }

    fn shutdown(_dev: &i2c::I2cClient<kernel::device::Core>, _this: Pin<&Self>) {
        pr_info!("ds3231: shutdown\n");
    }
}
 
