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
    let loaded_image_handle = match bs.load_image(image_handle, LoadImageSource::FromBuffer { buffer: &buffer, file_path: None }) {
        Ok(h) => h,
        Err(e) => return e.status(),
    };
    
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
    let loaded_image_handle = match bs.load_image(image_handle, LoadImageSource::FromBuffer { buffer: &buffer, file_path: None }) {
        Ok(h) => h,
        Err(e) => return e.status(),
    };
    
    if let Some(cmdline) = &entry.cmdline {
        if let Ok(mut loaded_image) = bs.open_protocol_exclusive::<LoadedImage>(loaded_image_handle) {
            let mut cmdline_utf16: Vec<u16> = cmdline.encode_utf16().chain(core::iter::once(0)).collect();
            unsafe {
                loaded_image.set_load_options(cmdline_utf16.as_mut_ptr() as *mut u8, (cmdline_utf16.len() * 2) as u32);
            }
            // Memory leak of cmdline_utf16 is avoided because UEFI copies or we just leak it intentionally since we're booting.
            // Actually, we must ensure it lives long enough. Let's leak it.
            core::mem::forget(cmdline_utf16);
        }
    }
    
    match bs.start_image(loaded_image_handle) {
        Ok(_) => Status::SUCCESS,
        Err(e) => e.status(),
    }
}
