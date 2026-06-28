#![allow(dead_code)]

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use log::info;
use uefi::prelude::*;
use uefi::proto::media::file::{File, FileAttribute, FileMode, FileInfo};


pub fn read_file_bytes(image_handle: Handle, path: &str) -> Option<Vec<u8>> {
    let mut fs = uefi::boot::get_image_file_system(image_handle).ok()?;
    let mut root = fs.open_volume().ok()?;
    
    let path_str = path.replace('/', "\\");
    let path_str = path_str.trim_start_matches('\\');
    let path_16 = uefi::CString16::try_from(path_str).ok()?;
    
    let file_handle = root.open(&path_16, FileMode::Read, FileAttribute::empty()).ok()?;
    let mut file = file_handle.into_regular_file()?;
    
    let info = file.get_boxed_info::<FileInfo>().ok()?;
    let size = info.file_size() as usize;
    
    let mut buf = alloc::vec![0; size];
    let read_size = file.read(&mut buf).ok()?;
    buf.truncate(read_size);
    Some(buf)
}

pub fn read_file_to_string(image_handle: Handle, path: &str) -> Option<String> {
    let bytes = read_file_bytes(image_handle, path)?;
    
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let u16_data: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16(&u16_data).ok();
    } else if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let u16_data: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16(&u16_data).ok();
    } else if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        return String::from_utf8(bytes[3..].to_vec()).ok();
    }
    
    String::from_utf8(bytes).ok()
}

pub fn log_msg(msg: &str) {
    info!("{}", msg);
}
pub fn scan_directory(image_handle: Handle, path: &str) -> Option<Vec<String>> {
    let mut fs = uefi::boot::get_image_file_system(image_handle).ok()?;
    let mut root = fs.open_volume().ok()?;
    
    let path_str = path.replace('/', "\\");
    let path_str = path_str.trim_start_matches('\\');
    let path_16 = uefi::CString16::try_from(path_str).ok()?;
    
    let file_handle = root.open(&path_16, FileMode::Read, FileAttribute::empty()).ok()?;
    let mut dir = file_handle.into_directory()?;
    
    let mut results = alloc::vec::Vec::new();
    let mut buf = alloc::vec![0u8; 4096];
    
    while let Ok(Some(info)) = dir.read_entry(&mut buf) {
        if info.attribute().contains(FileAttribute::DIRECTORY) { continue; }
        let filename_cstr16 = info.file_name();
        let s = filename_cstr16.to_string();
        if !s.is_empty() && s != "." && s != ".." {
            results.push(s);
        }
    }
    
    Some(results)
}
