#![allow(dead_code)]

use sha2::{Sha256, Digest};

pub fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

pub fn verify_hash(data: &[u8], expected: &[u8; 32]) -> bool {
    let actual = sha256_hash(data);
    actual == *expected
}

use uefi::prelude::*;

#[repr(C)]
#[uefi_macros::unsafe_protocol("605dab50-e046-4300-abb6-3dd810dd8b23")]
pub struct ShimLock {
    pub verify: extern "efiapi" fn(buffer: *const u8, size: u32) -> uefi::Status,
}

pub fn verify_secure_boot(buffer: &[u8]) -> Result<(), uefi::Status> {
    let shim_handle = match uefi::boot::get_handle_for_protocol::<ShimLock>() {
        Ok(h) => h,
        Err(_) => return Ok(()), // Shim not present
    };
    
    let shim = match uefi::boot::open_protocol_exclusive::<ShimLock>(shim_handle) {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };
    
    let status = (shim.verify)(buffer.as_ptr(), buffer.len() as u32);
    if status == uefi::Status::SUCCESS {
        Ok(())
    } else {
        Err(status)
    }
}
