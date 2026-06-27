#![no_main]
#![no_std]

extern crate alloc;

pub mod config;
pub mod gui;
pub mod boot;
pub mod efi_helpers;
pub mod crypto;
pub mod text_menu;
pub mod font;
pub mod font_jetbrains;

use log::info;
use uefi::prelude::*;

#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    if let Err(e) = uefi::helpers::init(&mut system_table) {
        return e.status();
    }

    info!("talaria-bootloader (Rust) loading...");

    // Wait for 3 seconds before exiting back to firmware
    system_table.boot_services().stall(3_000_000);

    Status::SUCCESS
}
