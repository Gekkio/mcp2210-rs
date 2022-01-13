# mcp2210-rs: Microchip MCP2210 library using hiddraw (Linux-only)

Minimum Rust version: 1.31

[MCP2210 datasheet](http://ww1.microchip.com/downloads/en/devicedoc/22288a.pdf)

[![Build Status](https://github.com/Gekkio/mcp2210-rs/workflows/ci/badge.svg)](https://github.com/Gekkio/mcp2210-rs/actions)
[![Latest release on crates.io](https://img.shields.io/crates/v/mcp2210.svg)](https://crates.io/crates/mcp2210)
[![Documentation on docs.rs](https://docs.rs/mcp2210/badge.svg)](https://docs.rs/mcp2210)

```rust
let mut mcp = Mcp2210::open("/dev/hidraw0")?;
mcp.set_spi_transfer_settings(&SpiTransferSettings {
    bit_rate: 1_000_000,
    bytes_per_tx: 4,
    spi_mode: SpiMode::Mode0,
    ..Default::default()
})?;
let mut from_slave = Vec::new();
mcp.spi_transfer_to_end(b"PING", &mut from_slave)?;
handle_response(&from_slave);
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
