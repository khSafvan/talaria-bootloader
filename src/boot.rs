#![allow(dead_code)]

use uefi::prelude::*;
use crate::gui::BootEntry;

use alloc::string::String;
use alloc::vec::Vec;
use uefi::proto::device_path::DevicePath;
use uefi::proto::loaded_image::LoadedImage;
use uefi::table::boot::LoadImageSource;

/// Boot a Windows entry by locating its bootmgfw.efi on the specific partition
pub fn boot_windows(image_handle: Handle, system_table: &mut SystemTable<Boot>, entry: &BootEntry) -> Status {
    let kernel_path = match &entry.kernel_path {
        Some(path) => path,
        None => return Status::INVALID_PARAMETER,
    };
    
    let buffer = match crate::efi_helpers::read_file_bytes(system_table, image_handle, kernel_path) {
        Some(b) => b,
        None => return Status::NOT_FOUND,
    };
    
    if entry.has_sha256 {
        if !crate::crypto::verify_hash(&buffer, &entry.sha256) {
            return Status::SECURITY_VIOLATION;
        }
    }
    
    if let Err(e) = crate::crypto::verify_secure_boot(system_table, &buffer) {
        return e;
    }
    
    let bs = system_table.boot_services();
    let device_path_protocol = bs.open_protocol_exclusive::<DevicePath>(image_handle).ok();
    
    let loaded_image_handle = match bs.load_image(image_handle, LoadImageSource::FromBuffer { 
        buffer: &buffer, 
        file_path: device_path_protocol.as_deref() 
    }) {
        Ok(h) => h,
        Err(e) => return e.status(),
    };
    
    drop(device_path_protocol);
    match bs.start_image(loaded_image_handle) {
        Ok(_) => Status::SUCCESS,
        Err(e) => e.status(),
    }
}

/// Boot a Linux entry using the EFI handover protocol or EFISTUB
pub fn boot_linux(image_handle: Handle, system_table: &mut SystemTable<Boot>, entry: &BootEntry) -> Status {
    let kernel_path = match &entry.kernel_path {
        Some(path) => path,
        None => return Status::INVALID_PARAMETER,
    };
    
    let buffer = match crate::efi_helpers::read_file_bytes(system_table, image_handle, kernel_path) {
        Some(b) => b,
        None => return Status::NOT_FOUND,
    };
    
    if entry.has_sha256 {
        if !crate::crypto::verify_hash(&buffer, &entry.sha256) {
            return Status::SECURITY_VIOLATION;
        }
    }
    
    if let Err(e) = crate::crypto::verify_secure_boot(system_table, &buffer) {
        return e;
    }
    
    let bs = system_table.boot_services();
    let device_path_protocol = bs.open_protocol_exclusive::<DevicePath>(image_handle).ok();
    
    let loaded_image_handle = match bs.load_image(image_handle, LoadImageSource::FromBuffer { 
        buffer: &buffer, 
        file_path: device_path_protocol.as_deref() 
    }) {
        Ok(h) => h,
        Err(e) => return e.status(),
    };
    
    if let Some(cmdline) = &entry.cmdline {
        if let Ok(mut loaded_image) = bs.open_protocol_exclusive::<LoadedImage>(loaded_image_handle) {
            let cmdline_utf16: Vec<u16> = cmdline.encode_utf16().chain(core::iter::once(0)).collect();
            let size = cmdline_utf16.len() * 2;
            
            if let Ok(ptr) = bs.allocate_pool(uefi::table::boot::MemoryType::LOADER_DATA, size) {
                unsafe {
                    core::ptr::copy_nonoverlapping(cmdline_utf16.as_ptr() as *const u8, ptr, size);
                    loaded_image.set_load_options(ptr, size as u32);
                }
            }
        }
    }
    
    drop(device_path_protocol);
    match bs.start_image(loaded_image_handle) {
        Ok(_) => Status::SUCCESS,
        Err(e) => e.status(),
    }
}
