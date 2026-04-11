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
    ioctl::{_IOR, _IOWR},
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    of,
    prelude::*,
    types::ARef,
};

// Register addresses
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
const REG_CONTROL:              u8 = 0x0E;
const REG_CONTROL_STATUS:       u8 = 0x0F;
const REG_AGING_OFFSET:         u8 = 0x10;
const REG_TEMP_MSB:             u8 = 0x11;
const REG_TEMP_LSB:             u8 = 0x12;

// IOCTL commands
const DS3231_GET_SECONDS:   u32 = _IOR::<u8>('d' as u32, 0x01);
const DS3231_GET_MINUTES:   u32 = _IOR::<u8>('d' as u32, 0x02);
const DS3231_GET_HOURS:     u32 = _IOR::<u8>('d' as u32, 0x03);
const DS3231_GET_PM:        u32 = _IOR::<u8>('d' as u32, 0x03);
const DS3231_GET_DAYS:      u32 = _IOR::<u8>('d' as u32, 0x05);
const DS3231_GET_DATE:      u32 = _IOR::<u8>('d' as u32, 0x06);
const DS3231_GET_MONTH:     u32 = _IOR::<u8>('d' as u32, 0x07);
const DS3231_GET_YEAR:      u32 = _IOR::<u8>('d' as u32, 0x08);
const DS3231_GET_TEMP:      u32 = _IOWR::<i16>('d' as u32, 0x09);



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

fn write_reg_u8(client: &ARef<i2c::I2cClient>, reg: u8, value: u8) -> Result<i32> {
     let res = unsafe { bindings::i2c_smbus_write_byte_data(raw_client(client), reg, value) };
     if res < 0 {
         Err(Error::from_errno(res))
     } else {
         Ok(0)
     }
}

#[inline]
fn bcd2bin(v: u8) -> u8 {
    (v & 0x0f) + ((v >> 4) * 10)
}

fn read_reg(client: &ARef<i2c::I2cClient>, reg: u8) -> Result<u8> {
    let raw: u8 = read_reg_u8(client, reg)?;
    Ok(bcd2bin(raw))
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
        b' ',
    ]
}

fn format_date(century: u8, year: u8, month: u8, day: u8) -> [u8; 11] {
    [
        b'0' + (month / 10),
        b'0' + (month % 10),
        b'/',
        b'0' + (day / 10),
        b'0' + (day % 10),
        b'/',
        b'0' + (century / 10),
        b'0' + (century % 10),
        b'0' + (year / 10),
        b'0' + (year % 10),
        b'\n',
    ]
}

fn decode_temp_q4(temp_msb: u8, temp_lsb: u8) -> i16 {
    let raw10 = (((temp_msb as u16) << 2) | (((temp_lsb as u16) >> 6) & 0x03)) as i16;
    (raw10 << 6) >> 6
}

fn format_temp(temp_q4: i16) -> [u8; 9] {
    let sign: u8 = if temp_q4 < 0 { b'-' } else { b'+' };
    let abs_q4: u16 = if temp_q4 < 0 { (-temp_q4) as u16 } else { temp_q4 as u16 };
    let int_part: u8 = (abs_q4 / 4) as u8;
    let frac: u8 = ((abs_q4 % 4) * 25) as u8;
    [
        sign,
        b'0' + (int_part / 10),
        b'0' + (int_part % 10),
        b'.',
        b'0' + (frac / 10),
        b'0' + (frac % 10),
        b' ',
        b'C',
        b'\n',
    ]
}

fn read_reg_ioctl(client: &ARef<i2c::I2cClient>, reg: u8, arg: usize) -> Result<usize> {
    let value: u8 = read_reg(client, reg)?;
    let res = unsafe {
        bindings::_copy_to_user(
            arg as *mut c_void,
            (&value as *const u8).cast::<c_void>(),
            size_of::<u8>(),
        )
    };
    if res != 0 {
        return Err(EFAULT);
    }
    else {
        Ok(0)
    }
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

            DS3231_GET_SECONDS => {
                Ok(read_reg_ioctl(&me.client, REG_SECONDS, arg)? as isize)
            }

            DS3231_GET_MINUTES => {
                Ok(read_reg_ioctl(&me.client, REG_MINUTES, arg)? as isize)
            }

            DS3231_GET_HOURS => {
                let value: u8 = read_reg_u8(&me.client, REG_HOURS)?;
                // The hours register is a bit more complex due to the 12/24 hour mode and AM/PM bit
                let hours: u8 = value & 0x0f + ((value & if (value & 0x40) != 0 { 0x10 } else { 0x30 }) >> 4) * 10 ; 
                let res = unsafe {
                    bindings::_copy_to_user(
                        arg as *mut c_void,
                        (&hours as *const u8).cast::<c_void>(),
                        size_of::<u8>(),
                    )
                };
                if res != 0 {
                    return Err(EFAULT);
                }
                else {
                    Ok(0)
                }
            }

            DS3231_GET_PM => {
                let value:  u8 = read_reg_u8(&me.client, REG_HOURS)?;
                let PM:     u8 = (value & 0x40) >> 6; // Extract the AM/PM bit
                let res = unsafe {
                    bindings::_copy_to_user(
                        arg as *mut c_void,
                        (&PM as *const u8).cast::<c_void>(),
                        size_of::<u8>(),
                    )
                };
                if res != 0 {
                    return Err(EFAULT);
                }
                else {
                    Ok(0)
                }
            }

            DS3231_GET_DAYS => {
                Ok(read_reg_ioctl(&me.client, REG_DAYS, arg)? as isize)
            }

            DS3231_GET_DATE => {
                Ok(read_reg_ioctl(&me.client, REG_DATE, arg)? as isize)
            }

            DS3231_GET_MONTH => {
                let value: u8 = read_reg_u8(&me.client, REG_MONTH_CENTURY)?;
                let Month: u8 = value & 0x0f + ((value & 0x10) >> 4) * 10 ; 
                let res = unsafe {
                    bindings::_copy_to_user(
                        arg as *mut c_void,
                        (&Month as *const u8).cast::<c_void>(),
                        size_of::<u8>(),
                    )
                };
                if res != 0 {
                    return Err(EFAULT);
                }
                else {
                    Ok(0)
                }
            }

            DS3231_GET_YEAR => {
                let centuryReg: u8 = read_reg_u8(&me.client, REG_MONTH_CENTURY)?;
                let yearReg:    u8 = read_reg_u8(&me.client, REG_YEAR)?;
                let year:       u16 = if (centuryReg & 0x80) != 0 { 2000} else { 1900 } + ((yearReg & 0x0f) + ((yearReg & 0xf0) >> 4) * 10) as u16 ; 
                let res = unsafe {
                    bindings::_copy_to_user(
                        arg as *mut c_void,
                        (&year as *const u16).cast::<c_void>(),
                        size_of::<u16>(),
                    )
                };
                if res != 0 {
                    return Err(EFAULT);
                }
                else {
                    Ok(0)
                }
            }

            DS3231_GET_TEMP => {
                let busy: bool = read_reg_u8(&me.client, REG_CONTROL_STATUS)? & 0x04 == 0x04;
                if !busy {
                   let res = write_reg_u8(&me.client, REG_CONTROL, 0b00111100)?; // Start a temperature conversion
                     if res < 0 {
                          return Err(Error::from_errno(res));
                     }
                }
                let tempLSB: u8 = read_reg_u8(&me.client, REG_TEMP_LSB)?;
                let tempMSB: u8 = read_reg_u8(&me.client, REG_TEMP_MSB)?;
                let temp: i16 = decode_temp_q4(tempMSB, tempLSB);
                let res = unsafe {
                    bindings::_copy_to_user(
                        arg as *mut c_void,
                        (&temp as *const i16).cast::<c_void>(),
                        size_of::<i16>(),
                    )
                };
                if res != 0 {
                    return Err(EFAULT);
                }
                else {
                    Ok(0)
                }
            }

            _ => Err(ENOTTY),
        }
    }

    fn read_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterDest<'_>,) -> Result<usize> {
        let client = unsafe { GLOBAL_CLIENT.clone() }.ok_or(ENODEV)?;
        let seconds = read_reg(&client, REG_SECONDS)?;
        let minutes = read_reg(&client, REG_MINUTES)?;
        let hours = read_reg(&client, REG_HOURS)? & 0x1f;
       // let days = read_reg_u8(&client, REG_DAYS)? & 0x07;
        let date = read_reg(&client, REG_DATE)? & 0x3f;
        let month = read_reg(&client, REG_MONTH_CENTURY)? & 0x1f;
        let year = read_reg(&client, REG_YEAR)?;
        let century = if (read_reg_u8(&client, REG_MONTH_CENTURY)? & 0x80) != 0 { 20 } else { 19 };
        let temp_msb = read_reg_u8(&client, REG_TEMP_MSB)?;
        let temp_lsb = read_reg_u8(&client, REG_TEMP_LSB)?;
        let temp_q4 = decode_temp_q4(temp_msb, temp_lsb);
        let data = format_hms(hours, minutes, seconds);
        let data2 = format_date(century, year, month, date);
        let data3 = format_temp(temp_q4);
        let mut data_all = [0u8; 29];
        data_all[..9].copy_from_slice(&data);
        data_all[9..20].copy_from_slice(&data2);
        data_all[20..].copy_from_slice(&data3);
        let read = iov.simple_read_from_buffer(kiocb.ki_pos_mut(), &data_all)?;
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
 
