// SPDX-License-Identifier: GPL-2.0
//! DS3231 RTC I2C driver — exposes time, date and temperature via a misc device.
//!
//! # Architecture
//!
//! The driver registers itself as both an I2C driver (`Ds3231Driver`) and a misc
//! device (`/dev/ds3231`).  User-space can interact through two interfaces:
//!
//! - `read()` — returns a 29-byte ASCII string: `HH:MM:SS MM/DD/CCYY±TT.FF C\n`
//! - `ioctl()` — fine-grained field access via the `DS3231_GET_*` / `DS3231_SET_*` commands
//!
//! # Authors
//! Quentin, Jules
#![allow(non_snake_case)]
#![allow(static_mut_refs)]

// ============================================================================
// Imports
// ============================================================================

use core::{ffi::c_void, mem::size_of, pin::Pin};

use kernel::{
    alloc::{flags::GFP_KERNEL, KBox},
    bindings, c_str,
    fs::{File, Kiocb},
    i2c,
    iov::IovIterDest,
    ioctl::{_IOR, _IOW, _IOWR},
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    of,
    prelude::*,
    types::ARef,
};

// ============================================================================
// Module Declaration
// ============================================================================

kernel::module_i2c_driver! {
    type: Ds3231Driver,
    name: "ds3231",
    authors: ["Quentin, Jules"],
    description: "DS3231 I2C driver with misc device bridge",
    license: "GPL",
}

// ============================================================================
// Register Addresses
// ============================================================================
//
// All registers are on the DS3231 internal map (datasheet §8.2).
// Time/date registers store values in BCD format.

const REG_SECONDS:          u8 = 0x00; // [6:0] BCD seconds (00–59)
const REG_MINUTES:          u8 = 0x01; // [6:0] BCD minutes (00–59)
const REG_HOURS:            u8 = 0x02; // [6] 12/24h, [5] AM/PM or hour-tens
const REG_DAYS:             u8 = 0x03; // [2:0] day of week (1–7)
const REG_DATE:             u8 = 0x04; // [5:0] BCD date (01–31)
const REG_MONTH_CENTURY:    u8 = 0x05; // [7] century, [4:0] BCD month (01–12)
const REG_YEAR:             u8 = 0x06; // [7:0] BCD year (00–99)
const REG_ALARM1_SECONDS:   u8 = 0x07;
const REG_ALARM1_MINUTES:   u8 = 0x08;
const REG_ALARM1_HOURS:     u8 = 0x09;
const REG_ALARM1_DAYS_DATE: u8 = 0x0A;
const REG_ALARM2_MINUTES:   u8 = 0x0B;
const REG_CONTROL:          u8 = 0x0E; // [5] CONV — trigger one-shot temperature conversion
const REG_CONTROL_STATUS:   u8 = 0x0F; // [2] BSY  — temperature conversion in progress
const REG_AGING_OFFSET:     u8 = 0x10;
const REG_TEMP_MSB:         u8 = 0x11; // [7:0] temperature integer part (signed)
const REG_TEMP_LSB:         u8 = 0x12; // [7:6] temperature 0.25 °C steps

// ============================================================================
// IOCTL Commands
// ============================================================================
//
// Magic number: 'd'  (0x64)
// All GET commands copy data to user-space; SET commands read data from user-space.

// GET commands — read a single field from the RTC
const DS3231_GET_SECONDS:   u32 = _IOR::<u8> ('d' as u32, 0x01);    // → u8  seconds (0–59)
const DS3231_GET_MINUTES:   u32 = _IOR::<u8> ('d' as u32, 0x02);    // → u8  minutes (0–59)
const DS3231_GET_HOURS:     u32 = _IOR::<u8> ('d' as u32, 0x03);    // → u8  hours   (0–23 or 1–12)
const DS3231_GET_PM:        u32 = _IOR::<u8> ('d' as u32, 0x04);    // → u8  0 = AM, 1 = PM (12h mode only)
const DS3231_GET_DAYS:      u32 = _IOR::<u8> ('d' as u32, 0x05);    // → u8  day of week (1–7)
const DS3231_GET_DATE:      u32 = _IOR::<u8> ('d' as u32, 0x06);    // → u8  day of month (1–31)
const DS3231_GET_MONTH:     u32 = _IOR::<u8> ('d' as u32, 0x07);    // → u8  month (1–12)
const DS3231_GET_YEAR:      u32 = _IOR::<u16> ('d' as u32, 0x08);   // → u16 full year (e.g. 2025)
const DS3231_GET_TEMP:      u32 = _IOWR::<i16>('d' as u32, 0x09);   // → i16 temp in 0.25 °C units (Q4 fixed-point)

// SET commands — write a single field to the RTC
const DS3231_SET_SECONDS:   u32 = _IOW::<u8>('d' as u32, 0x0A);
const DS3231_SET_MINUTES:   u32 = _IOW::<u8>('d' as u32, 0x0B);
const DS3231_SET_HOURS:     u32 = _IOWR::<u8>('d' as u32, 0x0C);
const DS3231_SET_12H:       u32 = _IOWR::<u8>('d' as u32, 0x0D);
const DS3231_SET_PM:        u32 = _IOWR::<u8>('d' as u32, 0x0E);
const DS3231_SET_DAYS:      u32 = _IOW::<u8>('d' as u32, 0x0F);
const DS3231_SET_DATE:      u32 = _IOW::<u8>('d' as u32, 0x10);
const DS3231_SET_MONTH:     u32 = _IOWR::<u8>('d' as u32, 0x11);
const DS3231_SET_YEAR:      u32 = _IOWR::<u16>('d' as u32, 0x12);


// ============================================================================
// Global State & Structs
// ============================================================================

/// Cached reference to the I2C client, set during `probe` and cleared on `drop`.
/// Shared between the driver and every open misc-device file handle.
static mut GLOBAL_CLIENT: Option<ARef<i2c::I2cClient>> = None;

/// Per-file-handle state — holds a reference to the I2C client for the lifetime
/// of a single open `/dev/ds3231` file descriptor.
#[pin_data]
struct Ds3231File {
    client: ARef<i2c::I2cClient>,
}

/// Top-level driver struct — owns the misc device registration for its lifetime.
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

// ============================================================================
// Low-level I2C Helpers
// ============================================================================

/// Returns the raw kernel `i2c_client` pointer for use with `bindings::*` calls.
#[inline]
fn raw_client(client: &ARef<i2c::I2cClient>) -> *mut bindings::i2c_client {
    (&**client) as *const i2c::I2cClient as *mut bindings::i2c_client
}

/// Reads one raw byte from `reg` via SMBus — value is NOT BCD-decoded.
#[inline]
fn read_reg_u8(client: &ARef<i2c::I2cClient>, reg: u8) -> Result<u8> {
    let value = unsafe { bindings::i2c_smbus_read_byte_data(raw_client(client), reg) };
    if value < 0 {
        Err(Error::from_errno(value))
    } else {
        Ok(value as u8)
    }
}

/// Writes one byte to `reg` via SMBus.
fn write_reg_u8(client: &ARef<i2c::I2cClient>, reg: u8, value: u8) -> Result<i32> {
    let res = unsafe { bindings::i2c_smbus_write_byte_data(raw_client(client), reg, value) };
    if res < 0 {
        Err(Error::from_errno(res))
    } else {
        Ok(0)
    }
}

// ============================================================================
// BCD & Formatting Helpers
// ============================================================================

/// Converts a BCD-encoded byte to its binary equivalent.
#[inline]
fn bcd2bin(v: u8) -> u8 {
    (v & 0x0f) + ((v >> 4) * 10)
}

/// Converts a binary value to its BCD-encoded equivalent.
#[inline]
fn bin2bcd(v: u8) -> u8 {
    ((v / 10) << 4) | (v % 10)
}

/// Reads one register and decodes it from BCD to binary.
fn read_reg(client: &ARef<i2c::I2cClient>, reg: u8) -> Result<u8> {
    let raw: u8 = read_reg_u8(client, reg)?;
    Ok(bcd2bin(raw))
}

/// Reads one register and decodes it from BCD to binary.
fn write_reg(client: &ARef<i2c::I2cClient>, reg: u8, value: u8) -> Result<u8> {
    let bcd: u8 = bin2bcd(value);
    write_reg_u8(client, reg, bcd)?;
    Ok(0)
}

/// Decodes the DS3231 two-register temperature into a Q4 fixed-point i16.
///
/// The MSB holds the signed integer part; the two MSBs of the LSB hold the
/// fractional part in 0.25 °C steps.  The returned value is in units of 0.25 °C,
/// so divide by 4 to get °C (e.g. `104` → `+26.0 °C`).
fn decode_temp_q4(temp_msb: u8, temp_lsb: u8) -> i16 {
    let raw10 = (((temp_msb as u16) << 2) | (((temp_lsb as u16) >> 6) & 0x03)) as i16;
    (raw10 << 6) >> 6
}

/// Formats hours, minutes and seconds as `HH:MM:SS ` (9 bytes, trailing space).
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

/// Formats a date as `MM/DD/CCYY\n` (11 bytes).
///
/// `century` is the two-digit century prefix (e.g. `20` for years 2000–2099).
/// `year` is the two-digit BCD-decoded year offset within the century (00–99).
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

/// Formats a Q4 fixed-point temperature as `±TT.FF C\n` (9 bytes).
fn format_temp(temp_q4: i16) -> [u8; 9] {
    let sign: u8     = if temp_q4 < 0 { b'-' } else { b'+' };
    let abs_q4: u16  = if temp_q4 < 0 { (-temp_q4) as u16 } else { temp_q4 as u16 };
    let int_part: u8 = (abs_q4 / 4) as u8;
    let frac: u8     = ((abs_q4 % 4) * 25) as u8;
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

/// Copies a typed value from a user-space pointer supplied as a raw `usize` address.
///
/// Returns `EFAULT` if the copy fails (bad user pointer or insufficient access).
fn copy_from_user<T>(arg: usize, out: &mut T) -> Result<()> {
    let res = unsafe {
        bindings::_copy_from_user(
            (out as *mut T).cast::<c_void>(),
            arg as *const c_void,
            size_of::<T>(),
        )
    };
    if res != 0 { Err(EFAULT) } else { Ok(()) }
}

/// Copies a typed value to a user-space pointer supplied as a raw `usize` address.
///
/// Returns `EFAULT` if the copy fails (bad user pointer or insufficient access).
fn copy_to_user<T>(arg: usize, value: &T) -> Result<isize> {
    let res = unsafe {
        bindings::_copy_to_user(
            arg as *mut c_void,
            (value as *const T).cast::<c_void>(),
            size_of::<T>(),
        )
    };
    if res != 0 { Err(EFAULT) } else { Ok(0) }
}

/// Reads a register (with BCD decode) and copies the result to user-space.
fn read_reg_ioctl(client: &ARef<i2c::I2cClient>, reg: u8, arg: usize) -> Result<usize> {
    let value: u8 = read_reg(client, reg)?;
    let res = unsafe {
        bindings::_copy_to_user(
            arg as *mut c_void,
            (&value as *const u8).cast::<c_void>(),
            size_of::<u8>(),
        )
    };
    if res != 0 { Err(EFAULT) } else { Ok(0) }
}


// ============================================================================
// MiscDevice Implementation
// ============================================================================

#[vtable]
impl MiscDevice for Ds3231File {
    type Ptr = Pin<KBox<Self>>;

    /// Opens `/dev/ds3231` — acquires a reference to the global I2C client.
    fn open(_file: &File, _misc: &MiscDeviceRegistration<Self>) -> Result<Self::Ptr> {
        let client = unsafe { GLOBAL_CLIENT.clone() }.ok_or(ENODEV)?;
        KBox::try_pin_init(
            try_pin_init!(Self { client: client }),
            GFP_KERNEL,
        )
    }

    /// Handles `ioctl` calls on `/dev/ds3231`.
    ///
    /// Each `DS3231_GET_*` command reads one RTC field and copies it to user-space.
    /// Unknown commands return `ENOTTY`.
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
                // bit6 = 12h mode; in 12h mode bit4 is tens, in 24h mode bits[5:4] are tens
                let hours: u8 = (value & 0x0f) + ((value & if (value & 0x40) != 0 { 0x10 } else { 0x30 }) >> 4) * 10;
                copy_to_user(arg, &hours)
            }

            DS3231_GET_PM => {
                let value: u8 = read_reg_u8(&me.client, REG_HOURS)?;
                let pm: u8 = (value & 0x20) >> 5; // bit5: 0 = AM, 1 = PM (only valid in 12h mode)
                copy_to_user(arg, &pm)
            }

            DS3231_GET_DAYS => {
                Ok(read_reg_ioctl(&me.client, REG_DAYS, arg)? as isize)
            }

            DS3231_GET_DATE => {
                Ok(read_reg_ioctl(&me.client, REG_DATE, arg)? as isize)
            }

            DS3231_GET_MONTH => {
                let value: u8 = read_reg_u8(&me.client, REG_MONTH_CENTURY)?;
                let month: u8 = (value & 0x0f) + ((value & 0x10) >> 4) * 10; // bit4 is tens digit
                copy_to_user(arg, &month)
            }

            DS3231_GET_YEAR => {
                let century_reg: u8 = read_reg_u8(&me.client, REG_MONTH_CENTURY)?;
                let year_reg:    u8 = read_reg_u8(&me.client, REG_YEAR)?;
                let year: u16 = if (century_reg & 0x80) != 0 { 2000 } else { 1900 }
                    + ((year_reg & 0x0f) + ((year_reg & 0xf0) >> 4) * 10) as u16;
                copy_to_user(arg, &year)
            }

            DS3231_GET_TEMP => {
                // Trigger a one-shot conversion if the sensor is idle, then read the result.
                let busy: bool = read_reg_u8(&me.client, REG_CONTROL_STATUS)? & 0x04 == 0x04;
                if !busy {
                    // Set CONV bit (bit5) only — preserve existing CONTROL register settings.
                    let ctrl = read_reg_u8(&me.client, REG_CONTROL)?;
                    write_reg_u8(&me.client, REG_CONTROL, ctrl | 0x20)?;
                }
                let temp_lsb: u8 = read_reg_u8(&me.client, REG_TEMP_LSB)?;
                let temp_msb: u8 = read_reg_u8(&me.client, REG_TEMP_MSB)?;
                let temp: i16 = decode_temp_q4(temp_msb, temp_lsb);
                copy_to_user(arg, &temp)
            }

            DS3231_SET_SECONDS => {
                let mut seconds: u8 = 0;
                copy_from_user(arg, &mut seconds)?;
                if seconds > 59 { return Err(EINVAL); }
                write_reg_u8(&me.client, REG_SECONDS, bin2bcd(seconds))?;
                Ok(0)
            }

            DS3231_SET_MINUTES  =>{
                let mut minutes: u8 = 0;
                copy_from_user(arg, &mut minutes)?;
                if minutes > 59 { return Err(EINVAL); }
                write_reg_u8(&me.client, REG_MINUTES, bin2bcd(minutes))?;
                Ok(0)
            }

            DS3231_SET_HOURS    =>{
                let mut hours: u8 = 0;
                copy_from_user(arg, &mut hours)?;
                let value: u8 = read_reg_u8(&me.client, REG_HOURS)?;
                let format: u8 = value & 0x40;
                if format == 0x40 {
                    // 12h mode: plage valide 1–12, préserver bits [7:5] (mode + AM/PM)
                    if hours < 1 || hours > 12 { return Err(EINVAL); }
                    write_reg_u8(&me.client, REG_HOURS, (value & 0xE0) | (bin2bcd(hours) & 0x1F))?;
                } else {
                    // 24h mode: plage valide 0–23, préserver bits [7:6] (mode)
                    if hours > 23 { return Err(EINVAL); }
                    write_reg_u8(&me.client, REG_HOURS, (value & 0xC0) | (bin2bcd(hours) & 0x3F))?;
                }
                Ok(0)
            }
            
            DS3231_SET_12H => {
                let mut h12: u8 =0;
                copy_from_user(arg, &mut h12)?;
                if h12 > 1 { return Err(EINVAL); }
                let value: u8 = read_reg_u8(&me.client, REG_HOURS)?;
                // 0xBF clears bit6 (12/24h mode bit) before setting it from h12
                write_reg_u8(&me.client, REG_HOURS, (value & 0xBF) | (h12 << 6))?;

                
                Ok(0)
            }

            DS3231_SET_PM       =>{
                let mut pm: u8 = 0;
                copy_from_user(arg, &mut pm)?;
                if pm > 1 { return Err(EINVAL); }
                let value: u8 = read_reg_u8(&me.client, REG_HOURS)?;
                let format: u8 = value & 0x40;
                if format == 0x40 {
                    // 0xDF clears bit5 (AM/PM), then set it from pm variable
                    write_reg_u8(&me.client, REG_HOURS, (value & 0xDF) | (pm << 5))?;
                } else {
                    return Err(EPERM); // AM/PM only meaningful in 12h mode
                }
                
                Ok(0)
            }

            DS3231_SET_DAYS     =>{
                let mut days: u8 = 0;
                copy_from_user(arg, &mut days)?;
                if days < 1 || days > 7 { return Err(EINVAL); }
                write_reg_u8(&me.client, REG_DAYS, bin2bcd(days))?;
                Ok(0)
            }

            DS3231_SET_DATE     =>{
                let mut date: u8 = 0;
                copy_from_user(arg, &mut date)?;
                if date < 1 || date > 31 { return Err(EINVAL); }
                write_reg_u8(&me.client, REG_DATE, bin2bcd(date))?;
                Ok(0)
            }

            DS3231_SET_MONTH    =>{
                let mut month: u8 = 0;
                copy_from_user(arg, &mut month)?;
                if month < 1 || month > 12 { return Err(EINVAL); }
                let value: u8 = read_reg_u8(&me.client, REG_MONTH_CENTURY)?;
                write_reg_u8(&me.client, REG_MONTH_CENTURY, bin2bcd(month) | (value & 0x80))?;
                Ok(0)
            }

            DS3231_SET_YEAR     =>{
                let mut year: u16 = 0;
                let mut yearDiz: u8 = 0;
                copy_from_user(arg, &mut year)?;
                if year > 2099 || year < 1900 { return Err(EINVAL); }
                yearDiz = (year % 100) as u8;
                write_reg_u8(&me.client, REG_YEAR, bin2bcd(yearDiz))?;
                let value: u8 = read_reg_u8(&me.client, REG_MONTH_CENTURY)?;
                // Clear bit7 first, then set it based on century — OR alone can't clear it
                let century_bit: u8 = if (year / 100) == 20 { 0x80 } else { 0x00 };
                write_reg_u8(&me.client, REG_MONTH_CENTURY, (value & 0x7F) | century_bit)?;
                Ok(0)
            }
            _ => Err(ENOTTY),
        }
    }

    /// Reads all time, date and temperature data in one shot.
    ///
    /// Returns a 29-byte ASCII string: `HH:MM:SS MM/DD/CCYY±TT.FF C\n`
    fn read_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterDest<'_>) -> Result<usize> {
        let client = unsafe { GLOBAL_CLIENT.clone() }.ok_or(ENODEV)?;

        let seconds  = read_reg(&client, REG_SECONDS)?;
        let minutes  = read_reg(&client, REG_MINUTES)?;
        // bcd2bin must be applied AFTER masking out the mode/AM-PM bits
        let raw_hours = read_reg_u8(&client, REG_HOURS)?;
        let hours = if (raw_hours & 0x40) != 0 {
            bcd2bin(raw_hours & 0x1F) // 12h mode: mask mode+AM/PM bits
        } else {
            bcd2bin(raw_hours & 0x3F) // 24h mode: mask mode bit only
        };
        let date     = read_reg(&client, REG_DATE)?;
        // Read REG_MONTH_CENTURY once: century bit7 must be masked before bcd2bin
        let raw_mc   = read_reg_u8(&client, REG_MONTH_CENTURY)?;
        let month    = bcd2bin(raw_mc & 0x1F);
        let century  = if (raw_mc & 0x80) != 0 { 20 } else { 19 };
        let year     = read_reg(&client, REG_YEAR)?;
        let temp_msb = read_reg_u8(&client, REG_TEMP_MSB)?;
        let temp_lsb = read_reg_u8(&client, REG_TEMP_LSB)?;
        let temp_q4  = decode_temp_q4(temp_msb, temp_lsb);

        let mut data_all = [0u8; 29];
        data_all[..9].copy_from_slice(&format_hms(hours, minutes, seconds));
        data_all[9..20].copy_from_slice(&format_date(century, year, month, date));
        data_all[20..].copy_from_slice(&format_temp(temp_q4));

        let read = iov.simple_read_from_buffer(kiocb.ki_pos_mut(), &data_all)?;
        Ok(read)
    }
}

// ============================================================================
// I2C Driver Implementation
// ============================================================================

impl i2c::Driver for Ds3231Driver {
    type IdInfo = ();

    const OF_ID_TABLE: Option<of::IdTable<Self::IdInfo>> = Some(&OF_TABLE);

    /// Called by the kernel when the DS3231 device is matched via the OF table.
    ///
    /// Stores the I2C client globally and registers the misc device.
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

    /// Called when the device is unbound from the driver (e.g. manual unbind via sysfs).
    fn unbind(_dev: &i2c::I2cClient<kernel::device::Core>, _this: Pin<&Self>) {
        pr_info!("ds3231: unbind\n");
    }

    /// Called on system shutdown before power-off.
    fn shutdown(_dev: &i2c::I2cClient<kernel::device::Core>, _this: Pin<&Self>) {
        pr_info!("ds3231: shutdown\n");
    }
}

/// Clears the global I2C client reference when the driver is removed.
#[pinned_drop]
impl PinnedDrop for Ds3231Driver {
    fn drop(self: Pin<&mut Self>) {
        unsafe { GLOBAL_CLIENT = None };
        pr_info!("ds3231: remove\n");
    }
}
