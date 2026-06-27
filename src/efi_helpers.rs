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
    
    let path_16 = uefi::CString16::try_from(path).ok()?;
    
    let file_handle = root.open(&path_16, FileMode::Read, FileAttribute::empty()).ok()?;
    let mut file = file_handle.into_regular_file()?;
    
    let info = file.get_boxed_info::<FileInfo>().ok()?;
    let size = info.file_size() as usize;
    
    let mut buf = alloc::vec![0; size];
    let read_size = file.read(&mut buf).ok()?;
    buf.truncate(read_size);
    Some(buf)
}

pub fn read_file_to_string(system_table: &SystemTable<Boot>, image_handle: Handle, path: &str) -> Option<String> {
    let bytes = read_file_bytes(system_table, image_handle, path)?;
    String::from_utf8(bytes).ok()
}

pub fn log_msg(msg: &str) {
    info!("{}", msg);
}
