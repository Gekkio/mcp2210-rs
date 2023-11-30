// SPDX-FileCopyrightText: 2018-2022 Joonas Javanainen <joonas.javanainen@gmail.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod cmds;
mod types;
mod utils;

pub use crate::cmds::*;
pub use crate::types::*;

use hidapi::{DeviceInfo, HidApi, HidDevice, HidError, HidResult};
use std::cmp::min;
use std::error::Error;
use std::fmt;

pub const FACTORY_VID: u16 = 0x04d8;
pub const FACTORY_PID: u16 = 0x00de;

#[derive(Debug)]
pub enum Mcp2210Error {
    Hid(HidError),
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
        use crate::Mcp2210Error::*;
        match self {
            Hid(err) => fmt::Display::fmt(err, f),
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
        use crate::Mcp2210Error::*;
        match self {
            Hid(err) => Some(err),
            _ => None,
        }
    }
}

const BUFFER_SIZE: usize = 64;

pub type Buffer = [u8; BUFFER_SIZE];

pub const MAX_BIT_RATE: u32 = 12_000_000;

pub struct Mcp2210 {
    device: HidDevice,
}

impl CommandResponse for Mcp2210 {
    fn command_response(&mut self, cmd: &Buffer, res: &mut Buffer) -> HidResult<()> {
        let mut data_to_write = [0; 1 + BUFFER_SIZE];
        data_to_write[0] = 0x00; // HID Report ID. For devices which only support a single report, this must be set to 0x0.
        data_to_write[1..].copy_from_slice(cmd);
        // At this point, length of data_to_write will be 1+BUFFER_SIZE == 65 and responses from the MCP2210 are always
        // BUFFER_SIZE. Therefore, this should only take single reports and these asserts should be good assumptions.
        assert_eq!(self.device.write(&data_to_write)?, data_to_write.len());
        assert_eq!(self.device.read(res)?, BUFFER_SIZE);
        Ok(())
    }
}

impl Mcp2210 {
    /// Converts a HidDevice to a Mcp2210.
    ///
    /// If the passed HidDevice is not actually a MCP2210 device, unexpected things are likely to happen when you
    /// use the Mcp2210 later.
    pub fn from(device: HidDevice) -> Mcp2210 {
        Mcp2210 { device }
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

/// True if the device has the MCP2210's factory Vendor ID (VID) and Product ID (VID).
pub fn is_mcp2210(device_info: &DeviceInfo) -> bool {
    device_info.vendor_id() == FACTORY_VID && device_info.product_id() == FACTORY_PID
}

/// Open the first HID device it finds with the MCP2210's factory Vendor ID (VID) and Product ID (PID).
///
/// When multiple devices with the MCP2210's factory VID and PID are available, then the first one
/// found in the internal device list will be used. There are however no guarantees, which device this
/// will be.
pub fn open_first(hidapi_context: &HidApi) -> Result<Mcp2210, Mcp2210Error> {
    let mcp = hidapi_context
        .open(FACTORY_VID, FACTORY_PID)
        .map_err(Mcp2210Error::Hid)?;
    Ok(Mcp2210::from(mcp))
}
