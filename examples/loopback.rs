// SPDX-FileCopyrightText: 2018-2022 Joonas Javanainen <joonas.javanainen@gmail.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

extern crate mcp2210;

use mcp2210::{Commands, Mcp2210, SpiMode, SpiTransferSettings};

fn main() {
    //! ##################################################################################
    //! ##                                ⚠️ WARNING ⚠️                                   ##
    //! ## This code sends 0xaa55 on the MCP2210's SPI bus.                             ##
    //! ## If you have a device connected to the SPI bus, ensure this will not harm it. ##
    //! ##################################################################################
    //! #
    //! This code sends 0xaa55 on the MCP2210's SPI bus MOSI pin and asserts that the same
    //! data is simultaneously recieved at the MISO pin. The circuit required for this is
    //! simply a wire between the MOSI and MISO pins of the MCP2210 and no real slave device.

    let devices = mcp2210::scan_devices().expect("Failed to scan devices");
    let device = devices.first().expect("No devices found");
    let mut mcp = Mcp2210::open_device(device).expect("Failed to open device");
    mcp.set_spi_transfer_settings(&SpiTransferSettings {
        bit_rate: 1_000,
        bytes_per_tx: 2,
        spi_mode: SpiMode::Mode0,
        ..Default::default()
    })
    .expect("Failed to set settings");
    let mut buf = Vec::new();
    mcp.spi_transfer_to_end(&[0xaa, 0x55], &mut buf)
        .expect("SPI transfer failed");
    assert_eq!(buf, [0xaa, 0x55]);
}
