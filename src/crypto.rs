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
