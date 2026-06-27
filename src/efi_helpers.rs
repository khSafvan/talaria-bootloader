#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use log::{info, warn};
use uefi::prelude::*;
use uefi::proto::media::file::{File, FileAttribute, FileMode, FileInfo};
use uefi::proto::media::fs::SimpleFileSystem;

pub fn read_file_bytes(system_table: &SystemTable<Boot>, image_handle: Handle, path: &str) -> Option<Vec<u8>> {
    let mut fs = system_table.boot_services().get_image_file_system(image_handle).ok()?;
    let mut root = fs.open_volume().ok()?;
    
    let mut path_buf = [0u16; 256];
    let path_16 = uefi::CStr16::from_str_with_buf(path, &mut path_buf).ok()?;
    
    let file_handle = root.open(path_16, FileMode::Read, FileAttribute::empty()).ok()?;
    let mut file = file_handle.into_regular_file()?;
    
    let mut info_buf = [0u8; 128];
    let info = file.get_info::<FileInfo>(&mut info_buf).ok()?;
    let size = info.file_size() as usize;
    
    let mut buf = alloc::vec![0; size];
    file.read(&mut buf).ok()?;
    Some(buf)
}

pub fn read_file_to_string(system_table: &SystemTable<Boot>, image_handle: Handle, path: &str) -> Option<String> {
    let bytes = read_file_bytes(system_table, image_handle, path)?;
    String::from_utf8(bytes).ok()
}

pub fn log_msg(msg: &str) {
    info!("{}", msg);
}
