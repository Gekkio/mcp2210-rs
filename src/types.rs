// SPDX-FileCopyrightText: 2018-2022 Joonas Javanainen <joonas.javanainen@gmail.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use bitflags::bitflags;
use std::fmt;

use crate::utils::{as_bool, as_u16, as_u32};
use crate::{Buffer, MAX_BIT_RATE};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChipStatus {
    pub is_bus_release_pending: bool,
    pub bus_owner: BusOwner,
    pub password_attempt_count: u8,
    pub is_password_guessed: bool,
}

impl ChipStatus {
    pub fn from_buffer(buf: &Buffer) -> Result<ChipStatus, String> {
        Ok(ChipStatus {
            is_bus_release_pending: !as_bool(buf[2])
                .map_err(|v| format!("Invalid is_bus_release_pending value: {:02x}", v))?,
            bus_owner: BusOwner::from_u8(buf[3])
                .map_err(|v| format!("Invalid bus_owner value: {:02x}", v))?,
            password_attempt_count: buf[4],
            is_password_guessed: as_bool(buf[5])
                .map_err(|v| format!("Invalid is_password_guessed value: {:02x}", v))?,
        })
    }
}

bitflags!(
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ChipSelect: u16 {
        const CS0 = 0b0_0000_0001;
        const CS1 = 0b0_0000_0010;
        const CS2 = 0b0_0000_0100;
        const CS3 = 0b0_0000_1000;
        const CS4 = 0b0_0001_0000;
        const CS5 = 0b0_0010_0000;
        const CS6 = 0b0_0100_0000;
        const CS7 = 0b0_1000_0000;
        const CS8 = 0b1_0000_0000;
        const ALL_HIGH = 0b1_1111_1111;
        const ALL_LOW = 0b0_0000_0000;
    }
);

bitflags!(
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct GpioValue: u16 {
        const GP0 = 0b0_0000_0001;
        const GP1 = 0b0_0000_0010;
        const GP2 = 0b0_0000_0100;
        const GP3 = 0b0_0000_1000;
        const GP4 = 0b0_0001_0000;
        const GP5 = 0b0_0010_0000;
        const GP6 = 0b0_0100_0000;
        const GP7 = 0b0_1000_0000;
        const GP8 = 0b1_0000_0000;
        const ALL_HIGH = 0b1_1111_1111;
        const ALL_LOW = 0b0_0000_0000;
    }
);

impl Default for GpioValue {
    fn default() -> GpioValue {
        GpioValue::ALL_HIGH
    }
}

bitflags!(
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct GpioDirection: u16 {
        const GP0DIR = 0b0_0000_0001;
        const GP1DIR = 0b0_0000_0010;
        const GP2DIR = 0b0_0000_0100;
        const GP3DIR = 0b0_0000_1000;
        const GP4DIR = 0b0_0001_0000;
        const GP5DIR = 0b0_0010_0000;
        const GP6DIR = 0b0_0100_0000;
        const GP7DIR = 0b0_1000_0000;
        const GP8DIR = 0b1_0000_0000;
        const ALL_INPUTS = 0b1_1111_1111;
        const ALL_OUTPUTS = 0b0_0000_0000;
    }
);

impl Default for GpioDirection {
    fn default() -> GpioDirection {
        GpioDirection::ALL_INPUTS
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PinMode {
    Gpio = 0x00,
    ChipSelect = 0x01,
    Dedicated = 0x02,
}

impl PinMode {
    fn from_u8(v: u8) -> Result<PinMode, u8> {
        match v {
            0x00 => Ok(PinMode::Gpio),
            0x01 => Ok(PinMode::ChipSelect),
            0x02 => Ok(PinMode::Dedicated),
            _ => Err(v),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BusOwner {
    None,
    UsbBridge,
    ExternalMaster,
}

impl BusOwner {
    fn from_u8(v: u8) -> Result<BusOwner, u8> {
        match v {
            0x00 => Ok(BusOwner::None),
            0x01 => Ok(BusOwner::UsbBridge),
            0x02 => Ok(BusOwner::ExternalMaster),
            _ => Err(v),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct UsbParameters {
    vid: u16,
    pid: u16,
    power_option: UsbPowerOption,
    remote_wakeup_capable: bool,
    requested_current: u8,
}

impl Default for UsbParameters {
    fn default() -> UsbParameters {
        UsbParameters {
            vid: 0x04d8,
            pid: 0x00de,
            power_option: UsbPowerOption::HostPowered,
            remote_wakeup_capable: false,
            requested_current: 50,
        }
    }
}

impl UsbParameters {
    pub fn from_buffer(buf: &Buffer) -> Result<UsbParameters, String> {
        Ok(UsbParameters {
            vid: as_u16(buf[12], buf[13]),
            pid: as_u16(buf[14], buf[15]),
            power_option: UsbPowerOption::from_u8(buf[29] >> 6)
                .map_err(|v| format!("Invalid power_option value: {:02x}", v))?,
            remote_wakeup_capable: buf[29] & 0b10_0000 != 0,
            requested_current: buf[30],
        })
    }
    pub fn write_to_buffer(self, buf: &mut Buffer) {
        buf[4] = self.vid as u8;
        buf[5] = (self.vid >> 8) as u8;
        buf[6] = self.pid as u8;
        buf[7] = (self.pid >> 8) as u8;
        buf[8] = ((self.power_option as u8) << 6)
            | (if self.remote_wakeup_capable {
                0b10_0000
            } else {
                0
            });
        buf[9] = self.requested_current;
    }
}

impl fmt::Debug for UsbParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UsbParameters")
            .field("vid", &format_args!("{:#06x}", self.vid))
            .field("pid", &format_args!("{:#06x}", self.pid))
            .field("power_option", &self.power_option)
            .field("remote_wakeup_capable", &self.remote_wakeup_capable)
            .field(
                "requested_current",
                &format_args!("{} mA", self.requested_current * 2),
            )
            .finish()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UsbPowerOption {
    SelfPowered = 0b01,
    HostPowered = 0b10,
}

impl UsbPowerOption {
    fn from_u8(v: u8) -> Result<UsbPowerOption, u8> {
        match v {
            0b10 => Ok(UsbPowerOption::HostPowered),
            0b01 => Ok(UsbPowerOption::SelfPowered),
            _ => Err(v),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChipSettings {
    pub gp0_mode: PinMode,
    pub gp1_mode: PinMode,
    pub gp2_mode: PinMode,
    pub gp3_mode: PinMode,
    pub gp4_mode: PinMode,
    pub gp5_mode: PinMode,
    pub gp6_mode: PinMode,
    pub gp7_mode: PinMode,
    pub gp8_mode: PinMode,
    pub default_gpio_value: GpioValue,
    pub default_gpio_direction: GpioDirection,
    pub remote_wakeup: bool,
    pub interrupt_mode: InterruptMode,
    pub bus_release: bool,
    pub nvram_access_control: NvramAccessControl,
}

impl ChipSettings {
    pub fn from_buffer(buf: &Buffer) -> Result<ChipSettings, String> {
        Ok(ChipSettings {
            gp0_mode: PinMode::from_u8(buf[4])
                .map_err(|v| format!("Invalid gp0_mode value: {:02x}", v))?,
            gp1_mode: PinMode::from_u8(buf[5])
                .map_err(|v| format!("Invalid gp1_mode value: {:02x}", v))?,
            gp2_mode: PinMode::from_u8(buf[6])
                .map_err(|v| format!("Invalid gp2_mode value: {:02x}", v))?,
            gp3_mode: PinMode::from_u8(buf[7])
                .map_err(|v| format!("Invalid gp3_mode value: {:02x}", v))?,
            gp4_mode: PinMode::from_u8(buf[8])
                .map_err(|v| format!("Invalid gp4_mode value: {:02x}", v))?,
            gp5_mode: PinMode::from_u8(buf[9])
                .map_err(|v| format!("Invalid gp5_mode value: {:02x}", v))?,
            gp6_mode: PinMode::from_u8(buf[10])
                .map_err(|v| format!("Invalid gp6_mode value: {:02x}", v))?,
            gp7_mode: PinMode::from_u8(buf[11])
                .map_err(|v| format!("Invalid gp7_mode value: {:02x}", v))?,
            gp8_mode: PinMode::from_u8(buf[12])
                .map_err(|v| format!("Invalid gp8_mode value: {:02x}", v))?,
            default_gpio_value: GpioValue::from_bits_truncate(as_u16(buf[13], buf[14])),
            default_gpio_direction: GpioDirection::from_bits_truncate(as_u16(buf[15], buf[16])),
            remote_wakeup: buf[17] & 0b10000 != 0,
            interrupt_mode: InterruptMode::from_u8((buf[17] >> 1) & 0b111)
                .map_err(|v| format!("Invalid interrupt_mode value: {:02x}", v))?,
            bus_release: buf[17] & 0b1 == 0,
            nvram_access_control: NvramAccessControl::from_u8(buf[18])
                .map_err(|v| format!("Invalid nvram_access_control value: {:02x}", v))?,
        })
    }
    pub fn write_to_buffer(&self, buf: &mut Buffer) {
        let default_gpio_value = self.default_gpio_value.bits();
        let default_gpio_direction = self.default_gpio_direction.bits();
        buf[4] = self.gp0_mode as u8;
        buf[5] = self.gp1_mode as u8;
        buf[6] = self.gp2_mode as u8;
        buf[7] = self.gp3_mode as u8;
        buf[8] = self.gp4_mode as u8;
        buf[9] = self.gp5_mode as u8;
        buf[10] = self.gp6_mode as u8;
        buf[11] = self.gp7_mode as u8;
        buf[12] = self.gp8_mode as u8;
        buf[13] = default_gpio_value as u8;
        buf[14] = (default_gpio_value >> 8) as u8;
        buf[15] = default_gpio_direction as u8;
        buf[16] = (default_gpio_direction >> 8) as u8;
        buf[17] = (if self.remote_wakeup { 0b10000 } else { 0 })
            | ((self.interrupt_mode as u8) << 1)
            | (if self.bus_release { 0 } else { 0b1 });
        buf[18] = self.nvram_access_control as u8;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum NvramAccessControl {
    #[default]
    None,
    Password = 0x40,
    PermanentlyLocked = 0x80,
}

impl NvramAccessControl {
    fn from_u8(v: u8) -> Result<NvramAccessControl, u8> {
        match v {
            0x00 => Ok(NvramAccessControl::None),
            0x40 => Ok(NvramAccessControl::Password),
            0x80 => Ok(NvramAccessControl::PermanentlyLocked),
            _ => Err(v),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum InterruptMode {
    #[default]
    None = 0b000,
    FallingEdges = 0b001,
    RisingEdges = 0b010,
    LowPulses = 0b011,
    HighPulses = 0b100,
}

impl InterruptMode {
    fn from_u8(v: u8) -> Result<InterruptMode, u8> {
        match v {
            0b100 => Ok(InterruptMode::HighPulses),
            0b011 => Ok(InterruptMode::LowPulses),
            0b010 => Ok(InterruptMode::RisingEdges),
            0b001 => Ok(InterruptMode::FallingEdges),
            0b000 => Ok(InterruptMode::None),
            _ => Err(v),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SpiTransferSettings {
    pub bit_rate: u32,
    pub cs_idle: ChipSelect,
    pub cs_active: ChipSelect,
    pub delay_cs_to_data: u16,
    pub delay_last_data_to_cs: u16,
    pub delay_between_data: u16,
    pub bytes_per_tx: u16,
    pub spi_mode: SpiMode,
}

impl Default for SpiTransferSettings {
    fn default() -> SpiTransferSettings {
        SpiTransferSettings {
            bit_rate: MAX_BIT_RATE,
            cs_idle: ChipSelect::ALL_HIGH,
            cs_active: ChipSelect::ALL_LOW,
            delay_cs_to_data: 0,
            delay_last_data_to_cs: 0,
            delay_between_data: 0,
            bytes_per_tx: 4,
            spi_mode: SpiMode::Mode0,
        }
    }
}

impl SpiTransferSettings {
    pub fn from_buffer(buf: &Buffer) -> Result<SpiTransferSettings, String> {
        Ok(SpiTransferSettings {
            bit_rate: as_u32(buf[4], buf[5], buf[6], buf[7]),
            cs_idle: ChipSelect::from_bits_truncate(as_u16(buf[8], buf[9])),
            cs_active: ChipSelect::from_bits_truncate(as_u16(buf[10], buf[11])),
            delay_cs_to_data: as_u16(buf[12], buf[13]),
            delay_last_data_to_cs: as_u16(buf[14], buf[15]),
            delay_between_data: as_u16(buf[16], buf[17]),
            bytes_per_tx: as_u16(buf[18], buf[19]),
            spi_mode: SpiMode::from_u8(buf[20])
                .map_err(|v| format!("Invalid spi_mode value: {:02x}", v))?,
        })
    }
    pub fn write_to_buffer(&self, buf: &mut Buffer) {
        let cs_idle = self.cs_idle.bits();
        let cs_active = self.cs_active.bits();
        buf[4] = self.bit_rate as u8;
        buf[5] = (self.bit_rate >> 8) as u8;
        buf[6] = (self.bit_rate >> 16) as u8;
        buf[7] = (self.bit_rate >> 24) as u8;
        buf[8] = cs_idle as u8;
        buf[9] = (cs_idle >> 8) as u8;
        buf[10] = cs_active as u8;
        buf[11] = (cs_active >> 8) as u8;
        buf[12] = self.delay_cs_to_data as u8;
        buf[13] = (self.delay_cs_to_data >> 8) as u8;
        buf[14] = self.delay_last_data_to_cs as u8;
        buf[15] = (self.delay_last_data_to_cs >> 8) as u8;
        buf[16] = self.delay_between_data as u8;
        buf[17] = (self.delay_between_data >> 8) as u8;
        buf[18] = self.bytes_per_tx as u8;
        buf[19] = (self.bytes_per_tx >> 8) as u8;
        buf[20] = self.spi_mode as u8;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpiMode {
    Mode0 = 0x00,
    Mode1 = 0x01,
    Mode2 = 0x02,
    Mode3 = 0x03,
}

impl SpiMode {
    fn from_u8(v: u8) -> Result<SpiMode, u8> {
        match v {
            0x00 => Ok(SpiMode::Mode0),
            0x01 => Ok(SpiMode::Mode1),
            0x02 => Ok(SpiMode::Mode2),
            0x03 => Ok(SpiMode::Mode3),
            _ => Err(v),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpiTransferResponse<'a> {
    pub data: &'a [u8],
    pub status: SpiTransferStatus,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpiTransferStatus {
    Started,
    Pending,
    Finished,
}

impl SpiTransferStatus {
    pub(crate) fn from_u8(v: u8) -> Result<SpiTransferStatus, u8> {
        match v {
            0x20 => Ok(SpiTransferStatus::Started),
            0x30 => Ok(SpiTransferStatus::Pending),
            0x10 => Ok(SpiTransferStatus::Finished),
            _ => Err(v),
        }
    }
}
