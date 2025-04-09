#![allow(dead_code)]

use uefi::prelude::*;
use crate::gui::BootEntry;
use alloc::vec::Vec;

use core::fmt::Write;
use uefi::proto::console::text::{Key, ScanCode};

pub fn show_text_menu(entries: &[BootEntry], timeout: isize, default_entry: usize) -> Option<usize> {
    if entries.is_empty() {
        return None;
    }

    let mut selected = 0;
    let mut ticks: isize = 0;
    let timeout_ticks = timeout * 10; // 100ms per tick for text menu
    
    // Clear screen once
    uefi::system::with_stdout(|stdout| { let _ = stdout.clear(); });
    let mut dirty = true;
    
    loop {
        if dirty {
            uefi::system::with_stdout(|stdout| {
                let _ = stdout.clear();
                let _ = write!(stdout, "Talaria Boot Menu\n\n");
                
                for (i, entry) in entries.iter().enumerate() {
                    if i == selected {
                        let _ = write!(stdout, " > {}\n", entry.name);
                    } else {
                        let _ = write!(stdout, "   {}\n", entry.name);
                    }
                }
            });
            dirty = false;
        }
        
        let key_opt = uefi::system::with_stdin(|stdin| stdin.read_key().unwrap_or(None));
        
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
                    let safe_default = if default_entry < entries.len() { default_entry } else { 0 };
                    return Some(safe_default);
                }
                _ => {}
            }
        } else {
            if timeout_ticks > 0 {
                ticks += 1;
                if ticks >= timeout_ticks {
                    if default_entry < entries.len() {
                        return Some(default_entry);
                    }
                }
            }
        }
        
        // Unconditional 100ms limiter
        uefi::boot::stall(core::time::Duration::from_micros(100_000));
    }
}
