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

    let config_path = "\\talaria\\boot.conf";
    let config_content = match efi_helpers::read_file_to_string(&system_table, image_handle, config_path) {
        Some(c) => c,
        None => {
            log::error!("Failed to read boot.conf at {}", config_path);
            system_table.boot_services().stall(5_000_000);
            return Status::NOT_FOUND;
        }
    };
    
    let config = match config::Config::parse_str(&config_content) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to parse config: {:?}", e);
            system_table.boot_services().stall(5_000_000);
            return Status::INVALID_PARAMETER;
        }
    };
    
    if config.entries.is_empty() {
        log::error!("No boot entries found in config!");
        system_table.boot_services().stall(5_000_000);
        return Status::NOT_FOUND;
    }
    
    let mut selected_entry = None;
    let mut selected_action = -1;
    
    let bs = system_table.boot_services();
    let gop_handle = bs.get_handle_for_protocol::<uefi::proto::console::gop::GraphicsOutput>();
    let gop = gop_handle.ok().and_then(|h| bs.open_protocol_exclusive::<uefi::proto::console::gop::GraphicsOutput>(h).ok());
    
    if let Some(mut gop_proto) = gop {
        let mut gui = gui::GuiState {
            gop: Some(&mut gop_proto),
            entries: config.entries.clone(),
            ..gui::GuiState::new()
        };
        gui.init();
        selected_entry = gui.run(&mut system_table);
        selected_action = gui.action;
    } else {
        if let Some(idx) = text_menu::show_text_menu(&mut system_table, &config.entries) {
            selected_entry = Some(config.entries[idx].clone());
        }
    }
    
    match selected_action {
        gui::TALARIA_ACTION_SHUTDOWN => {
            system_table.runtime_services().reset(uefi::table::runtime::ResetType::SHUTDOWN, Status::SUCCESS, None);
        }
        gui::TALARIA_ACTION_REBOOT | gui::TALARIA_ACTION_FIRMWARE => {
            system_table.runtime_services().reset(uefi::table::runtime::ResetType::COLD, Status::SUCCESS, None);
        }
        _ => {}
    }
    
    if let Some(entry) = selected_entry {
        let status = if entry.os_type == "windows" {
            boot::boot_windows(image_handle, &mut system_table, &entry)
        } else if entry.os_type == "linux" {
            boot::boot_linux(image_handle, &mut system_table, &entry)
        } else {
            log::error!("Unknown OS type: {}", entry.os_type);
            Status::UNSUPPORTED
        };
        
        if status.is_error() {
            log::error!("Failed to boot '{}': {:?}", entry.name, status);
            system_table.boot_services().stall(5_000_000);
            return status;
        }
    }

    Status::SUCCESS
}
