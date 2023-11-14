<!--
SPDX-FileCopyrightText: 2018-2022 Joonas Javanainen <joonas.javanainen@gmail.com>

SPDX-License-Identifier: CC0-1.0
-->

# mcp2210-rs: Microchip MCP2210 library

Minimum Rust version: 1.63

[MCP2210 datasheet](http://ww1.microchip.com/downloads/en/devicedoc/22288a.pdf)

[![Build Status](https://github.com/Gekkio/mcp2210-rs/workflows/ci/badge.svg)](https://github.com/Gekkio/mcp2210-rs/actions)
[![Latest release on crates.io](https://img.shields.io/crates/v/mcp2210.svg)](https://crates.io/crates/mcp2210)
[![Documentation on docs.rs](https://docs.rs/mcp2210/badge.svg)](https://docs.rs/mcp2210)

```rust
use mcp2210::{Commands, Mcp2210, SpiMode, SpiTransferSettings};

fn main() {
    let devices = mcp2210::scan_devices().expect("Failed to scan devices");
    let device = devices.first().expect("No devices found");
    let mut mcp = Mcp2210::open_device(&device).expect("Failed to open device");
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
    assert_eq!(buf.len(), 2);
    println!("0x{:02x} 0x{:02x}", buf[0], buf[1]); // prints 0xaa 0x55
}
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
