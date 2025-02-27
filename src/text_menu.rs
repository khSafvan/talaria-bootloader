#![allow(dead_code)]

use uefi::prelude::*;
use crate::gui::BootEntry;
use alloc::vec::Vec;

pub fn show_text_menu(system_table: &mut SystemTable<Boot>, entries: &[BootEntry]) -> Option<usize> {
    // Basic fallback menu using UEFI SimpleTextOutput
    // 1. Clear screen
    // 2. Print entries
    // 3. Wait for keystroke (WaitForKey event)
    // 4. Return selected index
    None
}
