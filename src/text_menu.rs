#![allow(dead_code)]

use uefi::prelude::*;
use crate::gui::BootEntry;
use alloc::vec::Vec;

use core::fmt::Write;
use uefi::proto::console::text::{Key, ScanCode};

pub fn show_text_menu(system_table: &mut SystemTable<Boot>, entries: &[BootEntry]) -> Option<usize> {
    if entries.is_empty() {
        return None;
    }

    let mut selected = 0;
    
    loop {
        let _ = system_table.stdout().clear();
        let _ = write!(system_table.stdout(), "Talaria Boot Menu\n\n");
        
        for (i, entry) in entries.iter().enumerate() {
            if i == selected {
                let _ = write!(system_table.stdout(), " > {}\n", entry.name);
            } else {
                let _ = write!(system_table.stdout(), "   {}\n", entry.name);
            }
        }
        
        let mut events = [system_table.stdin().wait_for_key_event()];
        let _ = system_table.boot_services().wait_for_event(&mut events);
        
        if let Ok(Some(key)) = system_table.stdin().read_key() {
            match key {
                Key::Special(ScanCode::UP) => {
                    if selected > 0 { selected -= 1; }
                    else { selected = entries.len() - 1; }
                }
                Key::Special(ScanCode::DOWN) => {
                    if selected < entries.len() - 1 { selected += 1; }
                    else { selected = 0; }
                }
                Key::Printable(u) => {
                    let ch: char = u.into();
                    if ch == '\r' || ch == '\n' {
                        return Some(selected);
                    }
                }
                Key::Special(ScanCode::ESCAPE) => {
                    return Some(0);
                }
                _ => {}
            }
        }
    }
}
