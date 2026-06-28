#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use uefi::proto::console::gop::{BltOp, BltPixel};

#[derive(Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub const COLOR_BLACK: Color = Color { r: 0x00, g: 0x00, b: 0x00 };
pub const COLOR_WHITE: Color = Color { r: 0xFF, g: 0xFF, b: 0xFF };
pub const COLOR_GRAY: Color = Color { r: 0x80, g: 0x80, b: 0x80 };
pub const COLOR_BLUE: Color = Color { r: 0x4A, g: 0x90, b: 0xD9 };
pub const COLOR_RED: Color = Color { r: 0xD9, g: 0x4A, b: 0x4A };
pub const COLOR_GREEN: Color = Color { r: 0x4A, g: 0xD9, b: 0x6E };
pub const COLOR_ORANGE: Color = Color { r: 0xD9, g: 0x8A, b: 0x4A };
pub const COLOR_DARK_BG: Color = Color { r: 0x1A, g: 0x1A, b: 0x2E };

pub struct Icon {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    pub scaled_size: usize,
    pub scaled: Option<Vec<u32>>,
}

pub const TALARIA_ACTION_BOOT: i32 = 0;
pub const TALARIA_ACTION_SHUTDOWN: i32 = 1;
pub const TALARIA_ACTION_REBOOT: i32 = 2;
pub const TALARIA_ACTION_FIRMWARE: i32 = 3;

pub const POWER_POS_BOTTOMRIGHT: i32 = 0;
pub const POWER_POS_BOTTOMLEFT: i32 = 1;
pub const POWER_POS_TOPRIGHT: i32 = 2;
pub const POWER_POS_TOPLEFT: i32 = 3;

#[derive(Clone)]
pub struct BootEntry {
    pub name: String,
    pub icon_path: Option<String>,
    pub kernel_path: Option<String>,
    pub initrd_path: Option<String>,
    pub cmdline: Option<String>,
    pub uuid: Option<String>,
    pub index: usize,
    pub entry_type: i32,
    pub icon_size: usize,
    pub color: Color,
    pub has_color: bool,
    pub sha256: [u8; 32],
    pub has_sha256: bool,
}

// Note: In a complete Rust port, the GuiState struct would hold UEFI protocol handles
// and vectors for entries and caches. We will define it as we port the GUI rendering.

pub struct GuiState<'boot> {
    pub gop: Option<&'boot mut uefi::proto::console::gop::GraphicsOutput>,
    pub pointer: Option<uefi::boot::ScopedProtocol<uefi::proto::console::pointer::Pointer>>,
    
    pub screen_width: usize,
    pub screen_height: usize,
    pub bpp: usize,
    pub pixels_per_scanline: usize,
    pub backbuffer: Option<Vec<BltPixel>>,
    
    pub entries: Vec<BootEntry>,
    pub selected: usize,
    pub per_page: usize,
    pub running: bool,
    pub action: i32,
    pub focus: i32,
    
    pub timeout: isize,
    pub default_entry: usize,
    pub cursor_x: isize,
    pub cursor_y: isize,
    
    pub title: Option<String>,
    pub show_title: bool,
    pub show_names: bool,
    pub title_color: Color,
    pub name_color: Color,
    pub bg_color: Color,
    pub highlight_color: Color,
    
    pub scene_cache: Option<Vec<BltPixel>>,
    pub scene_valid: bool,
    pub dirty: bool,
}

impl<'boot> Default for GuiState<'boot> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'boot> GuiState<'boot> {
    pub fn new() -> Self {
        Self {
            gop: None,
            pointer: None,
            screen_width: 0,
            screen_height: 0,
            bpp: 4,
            pixels_per_scanline: 0,
            backbuffer: None,
            entries: Vec::new(),
            selected: 0,
            per_page: 8,
            running: false,
            action: 0,
            focus: 0,
            timeout: 5,
            default_entry: 0,
            cursor_x: -1, // -1 implies hidden until moved
            cursor_y: -1,
            title: None,
            show_title: true,
            show_names: true,
            title_color: COLOR_WHITE,
            name_color: COLOR_WHITE,
            bg_color: COLOR_DARK_BG,
            highlight_color: COLOR_BLUE,
            scene_cache: None,
            scene_valid: false,
            dirty: true,
        }
    }

    pub fn init(&mut self) -> uefi::Status {
        if let Some(ref gop) = self.gop {
            let mode = gop.current_mode_info();
            self.screen_width = mode.resolution().0;
            self.screen_height = mode.resolution().1;
            self.pixels_per_scanline = mode.stride();
            
            let size = self.screen_width * self.screen_height;
            self.backbuffer = Some(alloc::vec![BltPixel::new(0, 0, 0); size]);
            self.scene_cache = Some(alloc::vec![BltPixel::new(0, 0, 0); size]);
        }
        uefi::Status::SUCCESS
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        if x >= self.screen_width || y >= self.screen_height {
            return;
        }
        
        if let Some(buf) = &mut self.backbuffer {
            let pixel = BltPixel::new(color.r, color.g, color.b);
            
            let ex = x.saturating_add(w).min(self.screen_width);
            let ey = y.saturating_add(h).min(self.screen_height);
            
            for j in y..ey {
                let start = j * self.screen_width + x;
                let end = start + (ex - x);
                buf[start..end].fill(pixel);
            }
        }
    }

    pub fn draw_char(&mut self, x: usize, y: usize, ch: char, color: Color, scale: usize) {
        use font8x8::UnicodeFonts;
        if let Some(glyph) = font8x8::BASIC_FONTS.get(ch) {
            for (r, row) in glyph.iter().enumerate() {
                for c in 0..8 {
                    if (*row & (1 << c)) != 0 {
                        self.fill_rect(x + c * scale, y + r * scale, scale, scale, color);
                    }
                }
            }
        }
    }

    pub fn draw_text(&mut self, x: usize, y: usize, text: &str, color: Color, scale: usize) {
        let mut cur_x = x;
        for ch in text.chars() {
            self.draw_char(cur_x, y, ch, color, scale);
            cur_x += 8 * scale;
        }
    }
    
    pub fn draw(&mut self) {
        self.fill_rect(0, 0, self.screen_width, self.screen_height, self.bg_color);
        
        // Draw the main Title
        let title_text = self.title.clone().unwrap_or(alloc::string::String::from("Talaria Bootloader"));
        if self.show_title {
            let scale = 3;
            let text_w = title_text.len() * 8 * scale;
            let x = (self.screen_width.saturating_sub(text_w)) / 2;
            let y = self.screen_height / 10;
            self.draw_text(x, y, &title_text, self.title_color, scale);
        }

        // Draw boot entries
        let entry_height = 100;
        let entry_width = 300;
        let padding = 20;
        
        let num_entries = self.entries.len();
        let total_w = num_entries * entry_width + (num_entries.saturating_sub(1)) * padding;
        let mut start_x = (self.screen_width.saturating_sub(total_w)) / 2;
        let start_y = (self.screen_height.saturating_sub(entry_height)) / 2;
        
        for i in 0..num_entries {
            let is_selected = i == self.selected;
            let has_color = self.entries[i].has_color;
            let entry_color = self.entries[i].color;
            let entry_name = self.entries[i].name.clone();
            
            // Draw box (highlight if selected)
            let box_color = if is_selected { self.highlight_color } else { Color { r: 50, g: 50, b: 50 } };
            self.fill_rect(start_x, start_y, entry_width, entry_height, box_color);
            
            // Draw internal colored block or OS hint
            let inner_color = if has_color { entry_color } else { Color { r: 80, g: 80, b: 80 } };
            self.fill_rect(start_x + 5, start_y + 5, entry_width - 10, entry_height - 50, inner_color);
            
            // Draw entry title text
            if self.show_names {
                let scale = 2;
                let text_w = entry_name.len() * 8 * scale;
                let text_x = start_x + (entry_width.saturating_sub(text_w)) / 2;
                let text_y = start_y + entry_height - 35;
                let text_color = if is_selected { Color { r: 255, g: 255, b: 255 } } else { self.name_color };
                self.draw_text(text_x, text_y, &entry_name, text_color, scale);
            }
            
            start_x += entry_width + padding;
        }
        
        // Draw timeout countdown if active
        if self.timeout > 0 {
            let mut buf = alloc::string::String::new();
            use core::fmt::Write;
            let _ = write!(&mut buf, "Booting in {}s...", self.timeout);
            
            let scale = 2;
            let text_w = buf.len() * 8 * scale;
            let text_x = (self.screen_width.saturating_sub(text_w)) / 2;
            let text_y = self.screen_height - (self.screen_height / 10);
            self.draw_text(text_x, text_y, &buf, COLOR_WHITE, scale);
        }
    }
    
    pub fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        
        if let (Some(gop), Some(buf), Some(cache)) = (self.gop.as_mut(), &self.backbuffer, &mut self.scene_cache) {
            if !self.scene_valid {
                let op = BltOp::BufferToVideo {
                    buffer: buf,
                    src: uefi::proto::console::gop::BltRegion::Full,
                    dest: (0, 0),
                    dims: (self.screen_width, self.screen_height),
                };
                let _ = gop.blt(op);
                cache.copy_from_slice(buf);
                self.scene_valid = true;
                return;
            }
            
            let mut min_y = self.screen_height;
            let mut max_y = 0;
            
            for y in 0..self.screen_height {
                let start = y * self.screen_width;
                let end = start + self.screen_width;
                
                // Compare pixels by raw bytes
                let changed = unsafe {
                    let buf_ptr = buf[start..end].as_ptr() as *const u8;
                    let cache_ptr = cache[start..end].as_ptr() as *const u8;
                    core::slice::from_raw_parts(buf_ptr, self.screen_width * 4) != core::slice::from_raw_parts(cache_ptr, self.screen_width * 4)
                };
                
                if changed {
                    if y < min_y { min_y = y; }
                    if y > max_y { max_y = y; }
                }
            }
            
            if min_y <= max_y {
                let height = max_y - min_y + 1;
                let start_idx = min_y * self.screen_width;
                let end_idx = (max_y + 1) * self.screen_width;
                
                let op = BltOp::BufferToVideo {
                    buffer: &buf[start_idx..end_idx],
                    src: uefi::proto::console::gop::BltRegion::Full,
                    dest: (0, min_y),
                    dims: (self.screen_width, height),
                };
                let _ = gop.blt(op);
                
                cache[start_idx..end_idx].copy_from_slice(&buf[start_idx..end_idx]);
            }
        }
    }
    
    pub fn run(&mut self) -> Option<BootEntry> {
        self.running = true;
        let mut ticks: isize = 0;
        let mut timeout_ticks = self.timeout * 100;
        
        while self.running {
            if self.dirty {
                self.draw();
            }
            self.flush();
            let mut input_received = false;
            
            let key_opt = uefi::system::with_stdin(|stdin| stdin.read_key().unwrap_or(None));
            
            if let Some(key) = key_opt {
                input_received = true;
                match key {
                    uefi::proto::console::text::Key::Special(uefi::proto::console::text::ScanCode::LEFT) | 
                    uefi::proto::console::text::Key::Special(uefi::proto::console::text::ScanCode::UP) => {
                        if self.selected > 0 { self.selected -= 1; }
                        else if !self.entries.is_empty() { self.selected = self.entries.len() - 1; }
                        self.dirty = true;
                    }
                    uefi::proto::console::text::Key::Special(uefi::proto::console::text::ScanCode::RIGHT) |
                    uefi::proto::console::text::Key::Special(uefi::proto::console::text::ScanCode::DOWN) => {
                        if self.selected < self.entries.len().saturating_sub(1) { self.selected += 1; }
                        else { self.selected = 0; }
                        self.dirty = true;
                    }
                    uefi::proto::console::text::Key::Printable(u) => {
                        let ch: char = u.into();
                        match ch {
                            '\r' | '\n' => {
                                if !self.entries.is_empty() {
                                    self.running = false;
                                    return Some(self.entries[self.selected].clone());
                                }
                            }
                            's' | 'S' => {
                                self.action = TALARIA_ACTION_SHUTDOWN;
                                self.running = false;
                            }
                            'r' | 'R' => {
                                self.action = TALARIA_ACTION_REBOOT;
                                self.running = false;
                            }
                            'f' | 'F' => {
                                self.action = TALARIA_ACTION_FIRMWARE;
                                self.running = false;
                            }
                            _ => {}
                        }
                    }
                    uefi::proto::console::text::Key::Special(uefi::proto::console::text::ScanCode::ESCAPE) => {
                        self.running = false;
                        return None;
                    }
                    _ => {}
                }
            }
            
            if let Some(pointer) = &mut self.pointer 
                && let Ok(Some(state)) = pointer.read_state() {
                    input_received = true;
                    
                    let dx = state.relative_movement[0] as isize;
                    let dy = state.relative_movement[1] as isize;
                    
                    if self.cursor_x == -1 {
                        self.cursor_x = (self.screen_width / 2) as isize;
                        self.cursor_y = (self.screen_height / 2) as isize;
                    }
                    
                    let max_x = (self.screen_width as isize - 1).max(0);
                    let max_y = (self.screen_height as isize - 1).max(0);
                    self.cursor_x = (self.cursor_x + dx).clamp(0, max_x);
                    self.cursor_y = (self.cursor_y + dy).clamp(0, max_y);
                    if dx != 0 || dy != 0 {
                        self.dirty = true;
                    }
                    
                    if state.button[0] && !self.entries.is_empty() {
                        self.running = false;
                        return Some(self.entries[self.selected].clone());
                    }
                }

            if !input_received {
                if timeout_ticks >= 0 {
                    if ticks >= timeout_ticks && !self.entries.is_empty() {
                        self.running = false;
                        return Some(self.entries[self.selected].clone());
                    }
                    ticks += 1;
                }
            } else {
                timeout_ticks = -1; // Abort timeout on input
            }
            
            // Frame-rate limiter: unconditionally wait 10ms to prevent PCIe bus saturation when input is active
            uefi::boot::stall(core::time::Duration::from_micros(10_000));
        }
        
        None
    }
}
