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
    let mut config = config::Config::default();
    
    if let Some(config_content) = efi_helpers::read_file_to_string(&system_table, image_handle, config_path) {
        match config::Config::parse_str(&config_content) {
            Ok(c) => config = c,
            Err(e) => log::error!("Failed to parse config: {:?}", e),
        }
    } else {
        log::warn!("Failed to read boot.conf at {}, auto-detecting...", config_path);
    }
    
    if config.entries.is_empty() {
        log::info!("No boot entries found, generating auto-detect menu...");
        config.entries.push(gui::BootEntry {
            name: alloc::string::String::from("Windows Boot Manager"),
            icon_path: None,
            kernel_path: Some(alloc::string::String::from("\\EFI\\Microsoft\\Boot\\bootmgfw.efi")),
            initrd_path: None,
            cmdline: None,
            uuid: None,
            index: 0,
            entry_type: 0,
            icon_size: 64,
            color: gui::COLOR_BLUE,
            has_color: true,
            sha256: [0; 32],
            has_sha256: false,
        });
        config.entries.push(gui::BootEntry {
            name: alloc::string::String::from("Linux (Auto)"),
            icon_path: None,
            kernel_path: Some(alloc::string::String::from("\\EFI\\Linux\\BOOTX64.EFI")),
            initrd_path: None,
            cmdline: None,
            uuid: None,
            index: 1,
            entry_type: 0,
            icon_size: 64,
            color: gui::COLOR_WHITE,
            has_color: true,
            sha256: [0; 32],
            has_sha256: false,
        });
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
            timeout: config.timeout,
            default_entry: config.default_entry,
            ..gui::GuiState::new()
        };
        gui.init();
        selected_entry = gui.run(&mut system_table);
        selected_action = gui.action;
    } else {
        if let Some(idx) = text_menu::show_text_menu(&mut system_table, &config.entries, config.timeout, config.default_entry) {
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
        let status = if entry.cmdline.is_some() || entry.initrd_path.is_some() {
            boot::boot_linux(image_handle, &mut system_table, &entry)
        } else {
            boot::boot_windows(image_handle, &mut system_table, &entry)
        };
        
        if status.is_error() {
            log::error!("Failed to boot '{}': {:?}", entry.name, status);
            system_table.boot_services().stall(5_000_000);
            return status;
        }
    }

    Status::SUCCESS
}
