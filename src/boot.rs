#![allow(dead_code)]

use uefi::prelude::*;
use crate::gui::BootEntry;


use alloc::vec::Vec;
use uefi::proto::device_path::DevicePath;
use uefi::proto::loaded_image::LoadedImage;
use uefi::boot::LoadImageSource;

/// Boot a Windows entry by locating its bootmgfw.efi on the specific partition
pub fn boot_windows(image_handle: Handle, entry: &BootEntry) -> Status {
    let kernel_path = match &entry.kernel_path {
        Some(path) => path,
        None => return Status::INVALID_PARAMETER,
    };
    
    let buffer = match crate::efi_helpers::read_file_bytes(image_handle, kernel_path) {
        Some(b) => b,
        None => return Status::NOT_FOUND,
    };
    
    if entry.has_sha256 && !crate::crypto::verify_hash(&buffer, &entry.sha256) {
        return Status::SECURITY_VIOLATION;
    }
    
    if let Err(e) = crate::crypto::verify_secure_boot(&buffer) {
        return e;
    }
    
    let device_path_protocol = uefi::boot::open_protocol_exclusive::<DevicePath>(image_handle).ok();
    
    let loaded_image_handle = match uefi::boot::load_image(image_handle, LoadImageSource::FromBuffer { 
        buffer: &buffer, 
        file_path: device_path_protocol.as_deref() 
    }) {
        Ok(h) => h,
        Err(e) => return e.status(),
    };
    
    drop(device_path_protocol);
    let status = match uefi::boot::start_image(loaded_image_handle) {
        Ok(_) => Status::SUCCESS,
        Err(e) => e.status(),
    };
    let _ = uefi::boot::unload_image(loaded_image_handle);
    status
}

/// Boot a Linux entry using the EFI handover protocol or EFISTUB
pub fn boot_linux(image_handle: Handle, entry: &BootEntry) -> Status {
    let kernel_path = match &entry.kernel_path {
        Some(path) => path,
        None => return Status::INVALID_PARAMETER,
    };
    
    let buffer = match crate::efi_helpers::read_file_bytes(image_handle, kernel_path) {
        Some(b) => b,
        None => return Status::NOT_FOUND,
    };
    
    if entry.has_sha256 && !crate::crypto::verify_hash(&buffer, &entry.sha256) {
        return Status::SECURITY_VIOLATION;
    }
    
    if let Err(e) = crate::crypto::verify_secure_boot(&buffer) {
        return e;
    }
    
    let device_path_protocol = uefi::boot::open_protocol_exclusive::<DevicePath>(image_handle).ok();
    
    let loaded_image_handle = match uefi::boot::load_image(image_handle, LoadImageSource::FromBuffer { 
        buffer: &buffer, 
        file_path: device_path_protocol.as_deref() 
    }) {
        Ok(h) => h,
        Err(e) => return e.status(),
    };
    
    let mut final_cmdline = alloc::string::String::new();
    if let Some(cmdline) = &entry.cmdline {
        final_cmdline.push_str(cmdline);
    }
    
    if let Some(initrd) = &entry.initrd_path {
        if !final_cmdline.is_empty() {
            final_cmdline.push(' ');
        }
        final_cmdline.push_str("initrd=");
        final_cmdline.push_str(&initrd.replace('/', "\\"));
    }
    
    let mut cmdline_ptr: Option<core::ptr::NonNull<u8>> = None;
    if !final_cmdline.is_empty() 
        && let Ok(mut loaded_image) = uefi::boot::open_protocol_exclusive::<LoadedImage>(loaded_image_handle) {
        let cmdline_utf16: Vec<u16> = final_cmdline.encode_utf16().chain(core::iter::once(0)).collect();
        let size = cmdline_utf16.len() * 2;
        
        if let Ok(ptr) = uefi::boot::allocate_pool(uefi::boot::MemoryType::LOADER_DATA, size) {
            unsafe {
                core::ptr::copy_nonoverlapping(cmdline_utf16.as_ptr() as *const u8, ptr.as_ptr(), size);
                loaded_image.set_load_options(ptr.as_ptr(), size as u32);
            }
            cmdline_ptr = Some(ptr);
        }
    }
    
    drop(device_path_protocol);
    let status = match uefi::boot::start_image(loaded_image_handle) {
        Ok(_) => Status::SUCCESS,
        Err(e) => e.status(),
    };
    
    if let Some(ptr) = cmdline_ptr {
        unsafe {
            let _ = uefi::boot::free_pool(ptr);
        }
    }
    let _ = uefi::boot::unload_image(loaded_image_handle);
    
    status
}
