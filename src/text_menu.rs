#![allow(dead_code)]

use uefi::prelude::*;
use crate::gui::BootEntry;
use alloc::vec::Vec;

use core::fmt::Write;
use uefi::proto::console::text::{Key, ScanCode};

pub fn show_text_menu(system_table: &mut SystemTable<Boot>, entries: &[BootEntry], timeout: isize, default_entry: usize) -> Option<usize> {
    if entries.is_empty() {
        return None;
    }

    let mut selected = 0;
    let mut ticks: isize = 0;
    let timeout_ticks = timeout * 10; // 100ms per tick for text menu
    
    // Clear screen once
    let _ = system_table.stdout().clear();
    let mut dirty = true;
    
    loop {
        if dirty {
            let _ = system_table.stdout().clear();
            let _ = write!(system_table.stdout(), "Talaria Boot Menu\n\n");
            
            for (i, entry) in entries.iter().enumerate() {
                if i == selected {
                    let _ = write!(system_table.stdout(), " > {}\n", entry.name);
                } else {
                    let _ = write!(system_table.stdout(), "   {}\n", entry.name);
                }
            }
            dirty = false;
        }
        
        let key_opt = system_table.stdin().read_key().unwrap_or(None);
        
        if let Some(key) = key_opt {
            ticks = 0; // reset on input
            match key {
                Key::Special(ScanCode::UP) => {
                    if selected > 0 { selected -= 1; }
                    else { selected = entries.len() - 1; }
                    dirty = true;
                }
                Key::Special(ScanCode::DOWN) => {
                    if selected < entries.len() - 1 { selected += 1; }
                    else { selected = 0; }
                    dirty = true;
                }
                Key::Printable(u) => {
                    let ch: char = u.into();
                    if ch == '\r' || ch == '\n' {
                        return Some(selected);
                    }
                }
                Key::Special(ScanCode::ESCAPE) => {
                    return Some(default_entry);
                }
                _ => {}
            }
        } else {
            system_table.boot_services().stall(100_000); // 100ms
            if timeout_ticks > 0 {
                ticks += 1;
                if ticks >= timeout_ticks {
                    if default_entry < entries.len() {
                        return Some(default_entry);
                    }
                }
            }
        }
    }
}
