#![allow(dead_code)]


use crate::gui::BootEntry;


use core::fmt::Write;
use uefi::proto::console::text::{Key, ScanCode};

pub fn show_text_menu(entries: &[BootEntry], timeout: isize, default_entry: usize) -> Option<usize> {
    if entries.is_empty() {
        return None;
    }

    let mut selected = default_entry;
    if selected >= entries.len() {
        selected = 0;
    }
    let mut ticks: isize = 0;
    let mut timeout_ticks = timeout * 10; // 100ms per tick for text menu
    
    // Clear screen once
    uefi::system::with_stdout(|stdout| { let _ = stdout.clear(); });
    let mut dirty = true;
    
    loop {
        if dirty {
            uefi::system::with_stdout(|stdout| {
                let _ = stdout.clear();
                let seconds_left = if timeout_ticks >= 0 {
                    (timeout_ticks - ticks + 9) / 10
                } else { 0 };
                
                if seconds_left > 0 {
                    let _ = write!(stdout, "Talaria Boot Menu (Booting default in {}s)\n\n", seconds_left);
                } else {
                    let _ = write!(stdout, "Talaria Boot Menu\n\n");
                }
                
                for (i, entry) in entries.iter().enumerate() {
                    if i == selected {
                        let _ = writeln!(stdout, " > {}", entry.name);
                    } else {
                        let _ = writeln!(stdout, "   {}", entry.name);
                    }
                }
            });
            dirty = false;
        }
        
        let key_opt = uefi::system::with_stdin(|stdin| stdin.read_key().unwrap_or(None));
        
        if let Some(key) = key_opt {
            timeout_ticks = -1; // abort timeout on input
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
                    return None;
                }
                _ => {}
            }
        } else {
            if timeout_ticks >= 0 {
                if ticks >= timeout_ticks {
                    return Some(selected);
                }
                ticks += 1;
                // Update UI every second (10 ticks = 1s)
                if ticks % 10 == 0 {
                    dirty = true;
                }
            }
        }
        
        // Unconditional 100ms limiter
        uefi::boot::stall(core::time::Duration::from_micros(100_000));
    }
}
