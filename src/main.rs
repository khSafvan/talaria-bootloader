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

    let config_path = "\\EFI\\talaria\\boot.conf";
    let config_content = efi_helpers::read_file_to_string(image_handle, config_path);
    let mut config = config::Config::default();
    
    if let Some(content) = &config_content {
        config = config::Config::parse_str(content);
    } else {
        log::warn!("Failed to read boot.conf at {}, auto-detecting...", config_path);
    }
    
    if config.entries.is_empty() {
        log::info!("No boot entries found, generating auto-detect menu...");
        
        let mut idx = 0;
        let mut add_entry = |name: &str, path: &str, color: gui::Color| {
            config.entries.push(gui::BootEntry {
                name: alloc::string::String::from(name),
                icon_path: None,
                icon_data: None,
                kernel_path: Some(alloc::string::String::from(path)),
                initrd_path: None,
                cmdline: None,
                uuid: None,
                index: idx,
                entry_type: 0,
                icon_size: 64,
                color,
                has_color: true,
                sha256: [0; 32],
                has_sha256: false,
            });
            idx += 1;
        };

        if efi_helpers::read_file_bytes(image_handle, "\\EFI\\Microsoft\\Boot\\bootmgfw.efi").is_some() {
            add_entry("Windows Boot Manager", "\\EFI\\Microsoft\\Boot\\bootmgfw.efi", gui::COLOR_BLUE);
        }

        if let Some(files) = efi_helpers::scan_directory(image_handle, "\\EFI\\Linux") {
            for f in files {
                let lower = f.to_lowercase();
                if lower.ends_with(".efi") {
                    let path = alloc::format!("\\EFI\\Linux\\{}", f);
                    let name = f.trim_end_matches(".efi").trim_end_matches(".EFI");
                    add_entry(name, &path, gui::COLOR_GREEN);
                }
            }
        }
        
        if let Some(files) = efi_helpers::scan_directory(image_handle, "\\boot") {
            for f in files {
                let lower = f.to_lowercase();
                if lower.starts_with("vmlinuz") {
                    let path = alloc::format!("\\boot\\{}", f);
                    add_entry(&f, &path, gui::COLOR_ORANGE);
                }
            }
        }
    }
    
    let mut selected_entry = None;
    let mut selected_action = -1;
    
    let gop_handle = uefi::boot::get_handle_for_protocol::<uefi::proto::console::gop::GraphicsOutput>();
    let gop = gop_handle.ok().and_then(|h| uefi::boot::open_protocol_exclusive::<uefi::proto::console::gop::GraphicsOutput>(h).ok());
    
    let pointer_handle = uefi::boot::get_handle_for_protocol::<uefi::proto::console::pointer::Pointer>();
    let pointer = pointer_handle.ok().and_then(|h| uefi::boot::open_protocol_exclusive::<uefi::proto::console::pointer::Pointer>(h).ok());
    
    let use_gui = !config.text_menu && gop.is_some();
    if use_gui {
        // Load icons and background image before starting the GUI
        for entry in &mut config.entries {
            if let Some(path) = &entry.icon_path {
                if let Some(bytes) = efi_helpers::read_file_bytes(image_handle, path) {
                    entry.icon_data = gui::parse_bmp(&bytes);
                }
            }
        }
        
        let mut bg_image = None;
        if let Some(bg_path) = &config.background {
            if let Some(bytes) = efi_helpers::read_file_bytes(image_handle, bg_path) {
                bg_image = gui::parse_bmp(&bytes);
            }
        }
        
        let mut gop_proto = gop.unwrap();
        let mut gui = gui::GuiState {
            gop: Some(&mut *gop_proto),
            pointer,
            entries: config.entries.clone(),
            timeout: config.timeout,
            default_entry: config.default_entry,
            bg_image,
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
            let mut value: u64 = 1;
            let mut data = [0u8; 8];
            if let Ok((var_data, _)) = uefi::runtime::get_variable(&name, &uefi::runtime::VariableVendor::GLOBAL_VARIABLE, &mut data) 
                && var_data.len() == 8 {
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(var_data);
                    value = u64::from_le_bytes(arr) | 1;
            }
            let _ = uefi::runtime::set_variable(&name, &uefi::runtime::VariableVendor::GLOBAL_VARIABLE, flags, &value.to_le_bytes());
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
