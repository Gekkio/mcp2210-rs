extern crate failure;
extern crate mcp2210;

use failure::Error;
use mcp2210::*;

fn main() -> Result<(), Error> {
    let devices = mcp2210::scan_devices()?;
    let device = devices.iter().next().expect("No devices found");
    let mut mcp = Mcp2210::open(&device)?;
    mcp.set_spi_transfer_settings(&SpiTransferSettings {
        bit_rate: 1_000,
        bytes_per_tx: 2,
        spi_mode: SpiMode::Mode0,
        ..Default::default()
    })?;
    let mut buf = Vec::new();
    mcp.spi_transfer_to_end(&[0xaa, 0x55], &mut buf)?;
    assert_eq!(buf.len(), 2);
    println!("0x{:02x} 0x{:02x}", buf[0], buf[1]); // prints 0xaa 0x55
    Ok(())
}
