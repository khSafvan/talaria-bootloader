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
fn main() -> Status {
    let image_handle = uefi::boot::image_handle();
    
    info!("talaria-bootloader (Rust) loading...");

    let config_path = "\\talaria\\boot.conf";
    let config_content = efi_helpers::read_file_to_string(image_handle, config_path);
    let mut config = config::Config::default();
    
    if let Some(content) = &config_content {
        config = config::Config::parse_str(content);
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
    
    let gop_handle = uefi::boot::get_handle_for_protocol::<uefi::proto::console::gop::GraphicsOutput>();
    let gop = gop_handle.ok().and_then(|h| uefi::boot::open_protocol_exclusive::<uefi::proto::console::gop::GraphicsOutput>(h).ok());
    
    let pointer_handle = uefi::boot::get_handle_for_protocol::<uefi::proto::console::pointer::Pointer>();
    let pointer = pointer_handle.ok().and_then(|h| uefi::boot::open_protocol_exclusive::<uefi::proto::console::pointer::Pointer>(h).ok());
    
    if let Some(mut gop_proto) = gop {
        let mut gui = gui::GuiState {
            gop: Some(&mut *gop_proto),
            pointer: pointer,
            entries: config.entries.clone(),
            timeout: config.timeout,
            default_entry: config.default_entry,
            ..gui::GuiState::new()
        };
        let _ = gui.init();
        selected_entry = gui.run();
        selected_action = gui.action;
    } else {
        if let Some(idx) = text_menu::show_text_menu(&config.entries, config.timeout, config.default_entry) {
            selected_entry = Some(config.entries[idx].clone());
        }
    }
    
    match selected_action {
        gui::TALARIA_ACTION_SHUTDOWN => {
            uefi::runtime::reset(uefi::runtime::ResetType::SHUTDOWN, Status::SUCCESS, None);
        }
        gui::TALARIA_ACTION_REBOOT => {
            uefi::runtime::reset(uefi::runtime::ResetType::COLD, Status::SUCCESS, None);
        }
        gui::TALARIA_ACTION_FIRMWARE => {
            let flags = uefi::runtime::VariableAttributes::NON_VOLATILE 
                      | uefi::runtime::VariableAttributes::BOOTSERVICE_ACCESS 
                      | uefi::runtime::VariableAttributes::RUNTIME_ACCESS;
            let name = uefi::CString16::try_from("OsIndications").unwrap();
            let _ = uefi::runtime::set_variable(&name, &uefi::runtime::VariableVendor::GLOBAL_VARIABLE, flags, &1u64.to_le_bytes());
            uefi::runtime::reset(uefi::runtime::ResetType::COLD, Status::SUCCESS, None);
        }
        _ => {}
    }
    
    if let Some(entry) = selected_entry {
        let status = if entry.cmdline.is_some() || entry.initrd_path.is_some() {
            boot::boot_linux(image_handle, &entry)
        } else {
            boot::boot_windows(image_handle, &entry)
        };
        
        if status.is_error() {
            log::error!("Failed to boot '{}': {:?}", entry.name, status);
            uefi::boot::stall(core::time::Duration::from_micros(5_000_000));
            return status;
        }
    }

    Status::SUCCESS
}
