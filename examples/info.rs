//! This example reads out all information and state from the MCP2210 and writes it to the console.

extern crate mcp2210;

use mcp2210::*;

fn main() {
    let devices = mcp2210::scan_devices().expect("Failed to scan devices");
    let device = devices.iter().next().expect("No devices found");
    let mut mcp = Mcp2210::open(&device).expect("Failed to open device");
    println!("Current Chip Status");
    println!("===================");
    println!("{:#?}", mcp.get_chip_status().expect("Failed to read chip status"));
    println!("Interrupt event counter: {}", mcp.get_interrupt_event_counter().expect("Failed to read interrupt count"));
    println!("{:#?}", mcp.get_chip_settings().expect("Failed to read chip settings"));
    println!("GPIO directions (inputs): {:#?}", mcp.get_gpio_direction().expect("Failed to read GPIO directions"));
    println!("GPIO values: {:#?}", mcp.get_gpio_value().expect("Failed to read GPIO values"));
    println!();
    println!("NVRAM settings");
    println!("==============");
    println!("{:#?}", mcp.get_nvram_spi_transfer_settings().expect("Failed to read NVRAM SPI transfer settings"));
    println!("{:#?}", mcp.get_nvram_chip_settings().expect("Failed to read NVRAM chip settings"));
    println!("{:#?}", mcp.get_nvram_usb_parameters().expect("Failed to read NVRAM USB parameters"));
    println!("Product name: {:?}", mcp.get_nvram_usb_product_name().expect("Failed to read NVRAM USB product name"));
    println!("Vendor name: {:?}", mcp.get_nvram_usb_vendor_name().expect("Failed to read NVRAM USB vendor name"));
}
