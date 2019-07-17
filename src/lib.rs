mod cmds;
mod types;
mod utils;

pub use crate::cmds::*;
pub use crate::types::*;

use libudev::Device;
use std::cmp::min;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Mcp2210Error {
    Io(io::Error),
    CommandCode { expected: u8, actual: u8 },
    SubCommandCode { expected: u8, actual: u8 },
    InvalidResponse(String),
    UnknownErrorCode(u8),
    StringSize(usize),
    PayloadSize(usize),
    TransferStatus(SpiTransferStatus),

    // MCP2210 error codes
    EepromWrite,            // 0xFA
    AccessDenied,           // 0xFB
    AccessRejected,         // 0xFC
    AccessDeniedRetry,      // 0xFD
    Unavailable,            // 0xF7
    Busy,                   // 0xF8
    UnknownCommandCode(u8), // 0xF9
}

impl fmt::Display for Mcp2210Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Mcp2210Error::*;
        match self {
            Io(err) => fmt::Display::fmt(err, f),
            CommandCode { expected, actual } => write!(
                f,
                "Invalid command code (expected {:2x}, got {:2x})",
                expected, actual
            ),
            SubCommandCode { expected, actual } => write!(
                f,
                "Invalid sub-command code (expected {:2x}, got {:2x})",
                expected, actual
            ),
            InvalidResponse(response) => write!(f, "Invalid response ({})", response),
            UnknownErrorCode(code) => write!(f, "Unknown error code {:2x}", code),
            StringSize(size) => write!(
                f,
                "String is too long (expected at most 29 UTF-16 encoded u16 values, got {})",
                size
            ),
            PayloadSize(size) => write!(
                f,
                "Payload is too big (expected at most 60 bytes, got {})",
                size
            ),
            TransferStatus(status) => write!(f, "Unexpected SPI transfer status {:?}", status),
            EepromWrite => write!(f, "EEPROM write failure"),
            AccessDenied => write!(f, "Access denied"),
            AccessRejected => write!(f, "Access rejected"),
            AccessDeniedRetry => write!(f, "Access denied, retrying allowed"),
            Unavailable => write!(f, "SPI bus unavailable"),
            Busy => write!(f, "SPI bus busy"),
            UnknownCommandCode(code) => write!(f, "Unknown command code {:2x}", code),
        }
    }
}

impl Error for Mcp2210Error {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use Mcp2210Error::*;
        match self {
            Io(err) => Some(err),
            _ => None,
        }
    }
}

pub type Buffer = [u8; 64];

pub const MAX_BIT_RATE: u32 = 12_000_000;

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
