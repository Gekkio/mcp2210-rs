#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate failure;
extern crate libudev;
extern crate nix;

mod cmds;
mod types;
mod utils;

pub use cmds::*;
pub use types::*;

use libudev::Device;
use std::cmp::min;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

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
}

impl CommandResponse for Mcp2210 {
    fn command_response(&mut self, cmd: &Buffer, res: &mut Buffer) -> io::Result<()> {
        self.file.command_response(cmd, res)
    }
}

impl Mcp2210 {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Mcp2210, Mcp2210Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(Mcp2210Error::Io)?;
        Ok(Mcp2210 { file })
    }
    pub fn spi_transfer_to_end(
        &mut self,
        mut data: &[u8],
        buf: &mut Vec<u8>,
    ) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        {
            let len = min(data.len(), 60);
            let res = self.spi_transfer(&data[..len], &mut res)?;
            data = &data[len..];
            if res.status != SpiTransferStatus::Started {
                return Err(Mcp2210Error::TransferStatus(res.status));
            }
        }
        loop {
            let len = min(data.len(), 60);
            match self.spi_transfer(&data[..len], &mut res) {
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
