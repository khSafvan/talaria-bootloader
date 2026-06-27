#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use log::{info, warn};
use uefi::prelude::*;
use uefi::proto::media::file::{File, FileAttribute, FileMode, FileInfo};
use uefi::proto::media::fs::SimpleFileSystem;

pub fn read_file_to_string(system_table: &SystemTable<Boot>, path: &str) -> Option<String> {
    // 1. Open SimpleFileSystem protocol
    // 2. Open root directory
    // 3. Open file `path`
    // 4. Read to Vec<u8>
    // 5. Convert to String (handling UTF-8/UTF-16)
    None
}

pub fn log_msg(msg: &str) {
    info!("{}", msg);
}
