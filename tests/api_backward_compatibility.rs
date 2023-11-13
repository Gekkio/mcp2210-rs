// Explicit types are used on variables since consumers of this crate may rely on those types

#[test]
#[ignore = "Only testing that this builds. If a device is connected, we don't know if it's safe to use it."]
fn pre_windows_support_examples() {
    use mcp2210::{Commands, Mcp2210, SpiMode, SpiTransferSettings};

    // The scan_devices_with_filter() function used the libudev::Device type. Consumers of this function
    // will likely need to rewrite the closure.

    {
        #[allow(deprecated)]
        let _ = Mcp2210::open("/dev/hidraw0").expect("Failed to open device");
    }

    {
        // If consumers of this crate depend on either the scan_devices() or scan_devices_with_filter()
        // function returning a Vec<PathBuf>, they will break.
        let devices = mcp2210::scan_devices().expect("Failed to scan devices");
        let device = devices.iter().next().expect("No devices found");
        #[allow(deprecated)]
        let mut mcp: Mcp2210 = Mcp2210::open(&device).expect("Failed to open device");
        mcp.set_spi_transfer_settings(&SpiTransferSettings {
            bit_rate: 1_000,
            bytes_per_tx: 2,
            spi_mode: SpiMode::Mode0,
            ..Default::default()
        })
        .expect("Failed to set settings");

        mcp.get_chip_status().unwrap();
        mcp.get_interrupt_event_counter().unwrap();
        mcp.get_chip_settings().unwrap();
        mcp.get_gpio_direction().unwrap();
        mcp.get_gpio_value().unwrap();
        mcp.get_nvram_spi_transfer_settings().unwrap();
        mcp.get_nvram_chip_settings().unwrap();
        mcp.get_nvram_usb_parameters().unwrap();
        mcp.get_nvram_usb_product_name().unwrap();
        mcp.get_nvram_usb_vendor_name().unwrap();

        let mut buf: Vec<u8> = Vec::new();
        mcp.spi_transfer_to_end(&[0xaa, 0x55], &mut buf)
            .expect("SPI transfer failed");
        assert_eq!(buf.len(), 2);
        println!("0x{:02x} 0x{:02x}", buf[0], buf[1]); // prints 0xaa 0x55
    }
}
