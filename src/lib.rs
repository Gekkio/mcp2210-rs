#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate failure;
extern crate libudev;
extern crate nix;

mod types;
mod utils;

pub use types::*;

use libudev::Device;
use std::cmp::min;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use utils::*;

#[derive(Fail, Debug)]
pub enum Mcp2210Error {
    #[fail(display = "IO error ({})", _0)]
    Io(#[cause] io::Error),
    #[fail(display = "Invalid command code (expected {:2x}, got {:2x})", expected, actual)]
    CommandCode { expected: u8, actual: u8 },
    #[fail(display = "Invalid sub-command code (expected {:2x}, got {:2x})", expected, actual)]
    SubCommandCode { expected: u8, actual: u8 },
    #[fail(display = "Invalid response ({})", _0)]
    InvalidResponse(String),
    #[fail(display = "Unknown error code {:2x}", _0)]
    UnknownErrorCode(u8),
    #[fail(
        display = "String is too long (expected at most 29 UTF-16 encoded u16 values, got {})", _0
    )]
    StringSize(usize),
    #[fail(display = "Payload is too big (expected at most 60 bytes, got {})", _0)]
    PayloadSize(usize),
    #[fail(display = "Unexpected SPI transfer status")]
    TransferStatus(SpiTransferStatus),

    // MCP2210 error codes
    #[fail(display = "EEPROM write failure")]
    EepromWrite, // 0xFA
    #[fail(display = "Access denied")]
    AccessDenied, // 0xFB
    #[fail(display = "Access rejected")]
    AccessRejected, // 0xFC
    #[fail(display = "Access denied, retrying allowed")]
    AccessDeniedRetry, // 0xFD
    #[fail(display = "SPI bus unavailable")]
    Unavailable, // 0xF7
    #[fail(display = "SPI bus busy")]
    Busy, // 0xF8
    #[fail(display = "Unknown command code {:2x}", _0)]
    UnknownCommandCode(u8), // 0xF9
}

pub type Buffer = [u8; 64];

pub struct Mcp2210 {
    file: File,
    response_buffer: Buffer,
}

impl Mcp2210 {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Mcp2210, Mcp2210Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(Mcp2210Error::Io)?;
        Ok(Mcp2210 {
            file,
            response_buffer: [0; 64],
        })
    }
    fn do_command<F>(&mut self, cmd_code: u8, f: F) -> Result<(), Mcp2210Error>
    where
        F: FnOnce(&mut Buffer) -> (),
    {
        let mut cmd: Buffer = [0; 64];
        cmd[0] = cmd_code;
        f(&mut cmd);
        self.file.write_all(&cmd).map_err(Mcp2210Error::Io)?;
        self.file
            .read_exact(&mut self.response_buffer)
            .map_err(Mcp2210Error::Io)?;
        if cmd_code != self.response_buffer[0] {
            return Err(Mcp2210Error::CommandCode {
                expected: cmd_code,
                actual: self.response_buffer[0],
            });
        }
        match self.response_buffer[1] {
            0x00 => Ok(()),
            0xf7 => Err(Mcp2210Error::Unavailable),
            0xf8 => Err(Mcp2210Error::Busy),
            0xf9 => Err(Mcp2210Error::UnknownCommandCode(cmd_code)),
            0xfa => Err(Mcp2210Error::EepromWrite),
            0xfb => Err(Mcp2210Error::AccessDenied),
            0xfc => Err(Mcp2210Error::AccessRejected),
            0xfd => Err(Mcp2210Error::AccessDeniedRetry),
            err_code => Err(Mcp2210Error::UnknownErrorCode(err_code)),
        }
    }
    fn do_sub_command<F>(
        &mut self,
        cmd_code: u8,
        sub_cmd_code: u8,
        f: F,
    ) -> Result<(), Mcp2210Error>
    where
        F: FnOnce(&mut Buffer) -> (),
    {
        self.do_command(cmd_code, |cmd| {
            cmd[1] = sub_cmd_code;
            f(cmd);
        })?;
        if self.response_buffer[2] != sub_cmd_code {
            Err(Mcp2210Error::SubCommandCode {
                expected: sub_cmd_code,
                actual: self.response_buffer[2],
            })
        } else {
            Ok(())
        }
    }
    pub fn get_chip_status(&mut self) -> Result<ChipStatus, Mcp2210Error> {
        self.do_command(0x10, |_| {})?;
        ChipStatus::from_buffer(&self.response_buffer).map_err(Mcp2210Error::InvalidResponse)
    }
    pub fn cancel_spi_transfer(&mut self) -> Result<ChipStatus, Mcp2210Error> {
        self.do_command(0x11, |_| {})?;
        ChipStatus::from_buffer(&self.response_buffer).map_err(Mcp2210Error::InvalidResponse)
    }
    pub fn get_interrupt_event_counter(&mut self) -> Result<u16, Mcp2210Error> {
        self.do_command(0x12, |cmd| {
            cmd[1] = 0xff;
        })?;
        Ok(as_u16(self.response_buffer[4], self.response_buffer[5]))
    }
    pub fn reset_interrupt_event_counter(&mut self) -> Result<u16, Mcp2210Error> {
        self.do_command(0x12, |cmd| {
            cmd[1] = 0x00;
        })?;
        Ok(as_u16(self.response_buffer[4], self.response_buffer[5]))
    }
    pub fn get_chip_settings(&mut self) -> Result<ChipSettings, Mcp2210Error> {
        self.do_command(0x20, |_| {})?;
        ChipSettings::from_buffer(&self.response_buffer).map_err(Mcp2210Error::InvalidResponse)
    }
    pub fn set_chip_settings(&mut self, settings: &ChipSettings) -> Result<(), Mcp2210Error> {
        self.do_command(0x21, |cmd| {
            settings.write_to_buffer(cmd);
        })
    }
    pub fn set_gpio_value(&mut self, value: GpioValue) -> Result<(), Mcp2210Error> {
        self.do_command(0x30, |cmd| {
            let value = value.bits();
            cmd[4] = value as u8;
            cmd[5] = (value >> 8) as u8;
        })
    }
    pub fn get_gpio_value(&mut self) -> Result<GpioValue, Mcp2210Error> {
        self.do_command(0x31, |_| {})?;
        Ok(GpioValue::from_bits_truncate(as_u16(
            self.response_buffer[4],
            self.response_buffer[5],
        )))
    }
    pub fn set_gpio_direction(&mut self, direction: GpioDirection) -> Result<(), Mcp2210Error> {
        self.do_command(0x32, |cmd| {
            let direction = direction.bits();
            cmd[4] = direction as u8;
            cmd[5] = (direction >> 8) as u8;
        })
    }
    pub fn get_gpio_direction(&mut self) -> Result<GpioDirection, Mcp2210Error> {
        self.do_command(0x33, |_| {})?;
        Ok(GpioDirection::from_bits_truncate(as_u16(
            self.response_buffer[4],
            self.response_buffer[5],
        )))
    }
    pub fn set_spi_transfer_settings(
        &mut self,
        settings: &SpiTransferSettings,
    ) -> Result<(), Mcp2210Error> {
        self.do_command(0x40, |cmd| {
            settings.write_to_buffer(cmd);
        })
    }
    pub fn get_spi_transfer_settings(&mut self) -> Result<SpiTransferSettings, Mcp2210Error> {
        self.do_command(0x41, |_| {})?;
        SpiTransferSettings::from_buffer(&self.response_buffer)
            .map_err(Mcp2210Error::InvalidResponse)
    }
    pub fn spi_transfer(&mut self, data: &[u8]) -> Result<SpiTransferResponse, Mcp2210Error> {
        if data.len() > 60 {
            return Err(Mcp2210Error::PayloadSize(data.len()));
        }
        let mosi_len = min(data.len(), 60);
        self.do_command(0x42, |cmd| {
            cmd[1] = mosi_len as u8;
            cmd[4..][..mosi_len].copy_from_slice(&data[..mosi_len]);
        })?;
        let miso_len = self.response_buffer[2] as usize;
        Ok(SpiTransferResponse {
            data: &self.response_buffer[4..][..miso_len],
            status: SpiTransferStatus::from_u8(self.response_buffer[3]).map_err(|v| {
                Mcp2210Error::InvalidResponse(format!("Invalid SPI transfer status: {:02x}", v))
            })?,
        })
    }
    pub fn read_eeprom(&mut self, addr: u8) -> Result<u8, Mcp2210Error> {
        self.do_command(0x50, |cmd| {
            cmd[1] = addr;
        })?;
        if self.response_buffer[2] != addr {
            return Err(Mcp2210Error::InvalidResponse(format!(
                "Invalid EEPROM address {:2x}",
                addr
            )));
        }
        Ok(self.response_buffer[3])
    }
    pub fn write_eeprom(&mut self, addr: u8, data: u8) -> Result<(), Mcp2210Error> {
        self.do_command(0x51, |cmd| {
            cmd[1] = addr;
            cmd[2] = data;
        })
    }
    pub fn set_nvram_spi_transfer_settings(
        &mut self,
        settings: &SpiTransferSettings,
    ) -> Result<(), Mcp2210Error> {
        self.do_sub_command(0x60, 0x10, |cmd| {
            settings.write_to_buffer(cmd);
        })
    }
    pub fn set_nvram_chip_settings(
        &mut self,
        settings: &ChipSettings,
        password: Option<&[u8; 8]>,
    ) -> Result<(), Mcp2210Error> {
        self.do_sub_command(0x60, 0x20, |cmd| {
            settings.write_to_buffer(cmd);
            if let Some(password) = password {
                cmd[19..27].copy_from_slice(password);
            }
        })
    }
    pub fn set_nvram_usb_parameters(&mut self, params: &UsbParameters) -> Result<(), Mcp2210Error> {
        self.do_sub_command(0x60, 0x30, |cmd| {
            params.write_to_buffer(cmd);
        })
    }
    pub fn set_nvram_usb_product_name(&mut self, name: &str) -> Result<(), Mcp2210Error> {
        let size = name.encode_utf16().count();
        if size > 29 {
            return Err(Mcp2210Error::StringSize(size));
        }
        self.do_sub_command(0x60, 0x40, |cmd| {
            cmd[4] = (size as u8) * 2 + 2;
            cmd[5] = 0x03;
            encode_utf16_to_buffer(name, &mut cmd[6..]);
        })
    }
    pub fn set_nvram_usb_vendor_name(&mut self, name: &str) -> Result<(), Mcp2210Error> {
        let size = name.encode_utf16().count();
        if size > 29 {
            return Err(Mcp2210Error::StringSize(size));
        }
        self.do_sub_command(0x60, 0x50, |cmd| {
            cmd[4] = (size as u8) * 2 + 2;
            cmd[5] = 0x03;
            encode_utf16_to_buffer(name, &mut cmd[6..]);
        })
    }
    pub fn get_nvram_spi_transfer_settings(&mut self) -> Result<SpiTransferSettings, Mcp2210Error> {
        self.do_sub_command(0x61, 0x10, |_| {})?;
        SpiTransferSettings::from_buffer(&self.response_buffer)
            .map_err(Mcp2210Error::InvalidResponse)
    }
    pub fn get_nvram_chip_settings(&mut self) -> Result<ChipSettings, Mcp2210Error> {
        self.do_sub_command(0x61, 0x20, |_| {})?;
        ChipSettings::from_buffer(&self.response_buffer).map_err(Mcp2210Error::InvalidResponse)
    }
    pub fn get_nvram_usb_parameters(&mut self) -> Result<UsbParameters, Mcp2210Error> {
        self.do_sub_command(0x61, 0x30, |_| {})?;
        UsbParameters::from_buffer(&self.response_buffer).map_err(Mcp2210Error::InvalidResponse)
    }
    pub fn get_nvram_usb_product_name(&mut self) -> Result<String, Mcp2210Error> {
        self.do_sub_command(0x61, 0x40, |_| {})?;
        let str_bytes = (self.response_buffer[4] - 2) as usize;
        let str_chars = str_bytes / 2;
        let mut char_buf = [0; 29];
        for (idx, chunk) in self.response_buffer[6..][..str_bytes].chunks(2).enumerate() {
            char_buf[idx] = as_u16(chunk[0], chunk[1]);
        }
        Ok(String::from_utf16_lossy(&char_buf[..str_chars]))
    }
    pub fn get_nvram_usb_vendor_name(&mut self) -> Result<String, Mcp2210Error> {
        self.do_sub_command(0x61, 0x50, |_| {})?;
        let str_bytes = (self.response_buffer[4] - 2) as usize;
        let str_chars = str_bytes / 2;
        let mut char_buf = [0; 29];
        for (idx, chunk) in self.response_buffer[6..][..str_bytes].chunks(2).enumerate() {
            char_buf[idx] = as_u16(chunk[0], chunk[1]);
        }
        Ok(String::from_utf16_lossy(&char_buf[..str_chars]))
    }
    pub fn send_access_password(&mut self, password: &[u8; 8]) -> Result<(), Mcp2210Error> {
        self.do_command(0x70, |cmd| {
            cmd[4..11].copy_from_slice(password);
        })
    }
    pub fn request_bus_release(&mut self, ack_value: bool) -> Result<(), Mcp2210Error> {
        self.do_command(0x80, |cmd| {
            cmd[1] = if ack_value { 0x01 } else { 0x00 };
        })
    }
    pub fn spi_transfer_to_end(
        &mut self,
        mut data: &[u8],
        buf: &mut Vec<u8>,
    ) -> Result<(), Mcp2210Error> {
        {
            let len = min(data.len(), 60);
            let res = self.spi_transfer(&data[..len])?;
            data = &data[len..];
            if res.status != SpiTransferStatus::Started {
                return Err(Mcp2210Error::TransferStatus(res.status));
            }
        }
        loop {
            let len = min(data.len(), 60);
            match self.spi_transfer(&data[..len]) {
                Ok(res) => {
                    data = &data[len..];
                    buf.extend(res.data);
                    if res.status == SpiTransferStatus::Finished {
                        break;
                    }
                }
                Err(Mcp2210Error::Busy) => (),
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }
}

pub fn scan_devices() -> io::Result<Vec<PathBuf>> {
    scan_devices_with_filter(|d| {
        let vendor_id = d.property_value("ID_VENDOR_ID");
        let model_id = d.property_value("ID_MODEL_ID");
        vendor_id == Some(OsStr::new("04d8")) && model_id == Some(OsStr::new("00de"))
    })
}

pub fn scan_devices_with_filter<F: FnMut(Device) -> bool>(mut f: F) -> io::Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    if let Ok(context) = libudev::Context::new() {
        let mut enumerator = libudev::Enumerator::new(&context)?;
        enumerator.match_subsystem("hidraw")?;
        let devices = enumerator.scan_devices()?;
        for d in devices {
            if let Some(devnode) = d.devnode() {
                if let Some(d) = d.parent() {
                    if let Some(d) = d.parent() {
                        if let Some(d) = d.parent() {
                            if d.property_value("ID_BUS") != Some(OsStr::new("usb")) {
                                continue;
                            }
                            if f(d) {
                                results.push(devnode.to_owned());
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(results)
}
