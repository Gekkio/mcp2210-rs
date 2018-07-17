use super::{Buffer, Mcp2210Error};
use std::cmp::min;
use std::io::{self, Read, Write};
use types::*;
use utils::{as_u16, encode_utf16_to_buffer};

pub trait CommandResponse {
    fn command_response(&mut self, cmd: &Buffer, res: &mut Buffer) -> io::Result<()>;
    fn do_command<F>(&mut self, cmd_code: u8, res: &mut Buffer, f: F) -> Result<(), Mcp2210Error>
    where
        F: FnOnce(&mut Buffer) -> (),
    {
        let mut cmd: Buffer = [0; 64];
        cmd[0] = cmd_code;
        f(&mut cmd);
        self.command_response(&cmd, res).map_err(Mcp2210Error::Io)?;
        if cmd_code != res[0] {
            return Err(Mcp2210Error::CommandCode {
                expected: cmd_code,
                actual: res[0],
            });
        }
        match res[1] {
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
        res: &mut Buffer,
        f: F,
    ) -> Result<(), Mcp2210Error>
    where
        F: FnOnce(&mut Buffer) -> (),
    {
        self.do_command(cmd_code, res, |cmd| {
            cmd[1] = sub_cmd_code;
            f(cmd);
        })?;
        if res[2] != sub_cmd_code {
            Err(Mcp2210Error::SubCommandCode {
                expected: sub_cmd_code,
                actual: res[2],
            })
        } else {
            Ok(())
        }
    }
}

impl<T> CommandResponse for T
where
    T: Read + Write + Sized,
{
    fn command_response(&mut self, cmd: &Buffer, res: &mut Buffer) -> io::Result<()> {
        self.write_all(cmd)?;
        self.read_exact(res)
    }
}

pub trait Commands: CommandResponse {
    fn get_chip_status(&mut self) -> Result<ChipStatus, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x10, &mut res, |_| {})?;
        ChipStatus::from_buffer(&res).map_err(Mcp2210Error::InvalidResponse)
    }
    fn cancel_spi_transfer(&mut self) -> Result<ChipStatus, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x11, &mut res, |_| {})?;
        ChipStatus::from_buffer(&res).map_err(Mcp2210Error::InvalidResponse)
    }
    fn get_interrupt_event_counter(&mut self) -> Result<u16, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x12, &mut res, |cmd| {
            cmd[1] = 0xff;
        })?;
        Ok(as_u16(res[4], res[5]))
    }
    fn reset_interrupt_event_counter(&mut self) -> Result<u16, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x12, &mut res, |cmd| {
            cmd[1] = 0x00;
        })?;
        Ok(as_u16(res[4], res[5]))
    }
    fn get_chip_settings(&mut self) -> Result<ChipSettings, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x20, &mut res, |_| {})?;
        ChipSettings::from_buffer(&res).map_err(Mcp2210Error::InvalidResponse)
    }
    fn set_chip_settings(&mut self, settings: &ChipSettings) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x21, &mut res, |cmd| {
            settings.write_to_buffer(cmd);
        })
    }
    fn set_gpio_value(&mut self, value: GpioValue) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x30, &mut res, |cmd| {
            let value = value.bits();
            cmd[4] = value as u8;
            cmd[5] = (value >> 8) as u8;
        })
    }
    fn get_gpio_value(&mut self) -> Result<GpioValue, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x31, &mut res, |_| {})?;
        Ok(GpioValue::from_bits_truncate(as_u16(res[4], res[5])))
    }
    fn set_gpio_direction(&mut self, direction: GpioDirection) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x32, &mut res, |cmd| {
            let direction = direction.bits();
            cmd[4] = direction as u8;
            cmd[5] = (direction >> 8) as u8;
        })
    }
    fn get_gpio_direction(&mut self) -> Result<GpioDirection, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x33, &mut res, |_| {})?;
        Ok(GpioDirection::from_bits_truncate(as_u16(res[4], res[5])))
    }
    fn set_spi_transfer_settings(
        &mut self,
        settings: &SpiTransferSettings,
    ) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x40, &mut res, |cmd| {
            settings.write_to_buffer(cmd);
        })
    }
    fn get_spi_transfer_settings(&mut self) -> Result<SpiTransferSettings, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x41, &mut res, |_| {})?;
        SpiTransferSettings::from_buffer(&res).map_err(Mcp2210Error::InvalidResponse)
    }
    fn spi_transfer<'a>(
        &mut self,
        data: &[u8],
        res: &'a mut Buffer,
    ) -> Result<SpiTransferResponse<'a>, Mcp2210Error> {
        if data.len() > 60 {
            return Err(Mcp2210Error::PayloadSize(data.len()));
        }
        let mosi_len = min(data.len(), 60);
        self.do_command(0x42, res, |cmd| {
            cmd[1] = mosi_len as u8;
            cmd[4..][..mosi_len].copy_from_slice(&data[..mosi_len]);
        })?;
        let miso_len = res[2] as usize;
        Ok(SpiTransferResponse {
            data: &res[4..][..miso_len],
            status: SpiTransferStatus::from_u8(res[3]).map_err(|v| {
                Mcp2210Error::InvalidResponse(format!("Invalid SPI transfer status: {:02x}", v))
            })?,
        })
    }
    fn read_eeprom(&mut self, addr: u8) -> Result<u8, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x50, &mut res, |cmd| {
            cmd[1] = addr;
        })?;
        if res[2] != addr {
            return Err(Mcp2210Error::InvalidResponse(format!(
                "Invalid EEPROM address {:2x}",
                addr
            )));
        }
        Ok(res[3])
    }
    fn write_eeprom(&mut self, addr: u8, data: u8) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x51, &mut res, |cmd| {
            cmd[1] = addr;
            cmd[2] = data;
        })
    }
    fn set_nvram_spi_transfer_settings(
        &mut self,
        settings: &SpiTransferSettings,
    ) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_sub_command(0x60, 0x10, &mut res, |cmd| {
            settings.write_to_buffer(cmd);
        })
    }
    fn set_nvram_chip_settings(
        &mut self,
        settings: &ChipSettings,
        password: Option<&[u8; 8]>,
    ) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_sub_command(0x60, 0x20, &mut res, |cmd| {
            settings.write_to_buffer(cmd);
            if let Some(password) = password {
                cmd[19..27].copy_from_slice(password);
            }
        })
    }
    fn set_nvram_usb_parameters(&mut self, params: &UsbParameters) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_sub_command(0x60, 0x30, &mut res, |cmd| {
            params.write_to_buffer(cmd);
        })
    }
    fn set_nvram_usb_product_name(&mut self, name: &str) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        let size = name.encode_utf16().count();
        if size > 29 {
            return Err(Mcp2210Error::StringSize(size));
        }
        self.do_sub_command(0x60, 0x40, &mut res, |cmd| {
            cmd[4] = (size as u8) * 2 + 2;
            cmd[5] = 0x03;
            encode_utf16_to_buffer(name, &mut cmd[6..]);
        })
    }
    fn set_nvram_usb_vendor_name(&mut self, name: &str) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        let size = name.encode_utf16().count();
        if size > 29 {
            return Err(Mcp2210Error::StringSize(size));
        }
        self.do_sub_command(0x60, 0x50, &mut res, |cmd| {
            cmd[4] = (size as u8) * 2 + 2;
            cmd[5] = 0x03;
            encode_utf16_to_buffer(name, &mut cmd[6..]);
        })
    }
    fn get_nvram_spi_transfer_settings(&mut self) -> Result<SpiTransferSettings, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_sub_command(0x61, 0x10, &mut res, |_| {})?;
        SpiTransferSettings::from_buffer(&res).map_err(Mcp2210Error::InvalidResponse)
    }
    fn get_nvram_chip_settings(&mut self) -> Result<ChipSettings, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_sub_command(0x61, 0x20, &mut res, |_| {})?;
        ChipSettings::from_buffer(&res).map_err(Mcp2210Error::InvalidResponse)
    }
    fn get_nvram_usb_parameters(&mut self) -> Result<UsbParameters, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_sub_command(0x61, 0x30, &mut res, |_| {})?;
        UsbParameters::from_buffer(&res).map_err(Mcp2210Error::InvalidResponse)
    }
    fn get_nvram_usb_product_name(&mut self) -> Result<String, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_sub_command(0x61, 0x40, &mut res, |_| {})?;
        let str_bytes = (res[4] - 2) as usize;
        let str_chars = str_bytes / 2;
        let mut char_buf = [0; 29];
        for (idx, chunk) in res[6..][..str_bytes].chunks(2).enumerate() {
            char_buf[idx] = as_u16(chunk[0], chunk[1]);
        }
        Ok(String::from_utf16_lossy(&char_buf[..str_chars]))
    }
    fn get_nvram_usb_vendor_name(&mut self) -> Result<String, Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_sub_command(0x61, 0x50, &mut res, |_| {})?;
        let str_bytes = (res[4] - 2) as usize;
        let str_chars = str_bytes / 2;
        let mut char_buf = [0; 29];
        for (idx, chunk) in res[6..][..str_bytes].chunks(2).enumerate() {
            char_buf[idx] = as_u16(chunk[0], chunk[1]);
        }
        Ok(String::from_utf16_lossy(&char_buf[..str_chars]))
    }
    fn send_access_password(&mut self, password: &[u8; 8]) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x70, &mut res, |cmd| {
            cmd[4..11].copy_from_slice(password);
        })
    }
    fn request_bus_release(&mut self, ack_value: bool) -> Result<(), Mcp2210Error> {
        let mut res: Buffer = [0; 64];
        self.do_command(0x80, &mut res, |cmd| {
            cmd[1] = if ack_value { 0x01 } else { 0x00 };
        })
    }
}

impl<T> Commands for T
where
    T: CommandResponse,
{
}

#[cfg(test)]
struct TestTx {
    cmd: Buffer,
    res: Buffer,
}

#[cfg(test)]
impl TestTx {
    fn new(res: &[u8]) -> TestTx {
        assert!(res.len() <= 64);
        let mut tx = TestTx {
            cmd: [0; 64],
            res: [0; 64],
        };
        tx.res[..res.len()].copy_from_slice(res);
        tx
    }
}

#[cfg(test)]
impl CommandResponse for TestTx {
    fn command_response(&mut self, cmd: &Buffer, res: &mut Buffer) -> io::Result<()> {
        self.cmd.copy_from_slice(cmd);
        res.copy_from_slice(&self.res);
        Ok(())
    }
}

#[test]
fn test_get_chip_status() {
    let mut tx = TestTx::new(&[0x10, 0x00, 0x01, 0x02, 42, 0x01]);
    let status = tx.get_chip_status().unwrap();
    let mut expected_cmd = [0; 64];
    expected_cmd[0] = 0x10;
    assert_eq!(tx.cmd.as_ref(), expected_cmd.as_ref());
    assert_eq!(status.is_bus_release_pending, false);
    assert_eq!(status.bus_owner, BusOwner::ExternalMaster);
    assert_eq!(status.password_attempt_count, 42);
    assert_eq!(status.is_password_guessed, true);
}

#[test]
fn test_cancel_spi_transfer() {
    let mut tx = TestTx::new(&[0x11, 0x00, 0x00, 0x01, 79, 0x00]);
    let status = tx.cancel_spi_transfer().unwrap();
    let mut expected_cmd = [0; 64];
    expected_cmd[0] = 0x11;
    assert_eq!(tx.cmd.as_ref(), expected_cmd.as_ref());
    assert_eq!(status.is_bus_release_pending, true);
    assert_eq!(status.bus_owner, BusOwner::UsbBridge);
    assert_eq!(status.password_attempt_count, 79);
    assert_eq!(status.is_password_guessed, false);
}
