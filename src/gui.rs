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

#[derive(Clone)]
pub struct BmpImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>, // AARRGGBB
}

pub fn parse_bmp(data: &[u8]) -> Option<BmpImage> {
    if data.len() < 54 { return None; }
    if &data[0..2] != b"BM" { return None; }

    let pixel_offset = u32::from_le_bytes(data[10..14].try_into().unwrap_or([0;4])) as usize;
    let bpp = u16::from_le_bytes(data[28..30].try_into().unwrap_or([0;2]));
    
    if bpp != 24 && bpp != 32 { return None; }

    let width_raw = i32::from_le_bytes(data[18..22].try_into().unwrap_or([0;4]));
    let width = width_raw.unsigned_abs() as usize;
    let height_raw = i32::from_le_bytes(data[22..26].try_into().unwrap_or([0;4]));
    let top_down = height_raw < 0;
    let height = height_raw.unsigned_abs() as usize;

    if data.len() < pixel_offset { return None; }
    
    let mut pixels = alloc::vec![0; width * height];
    let row_stride = (width * (bpp as usize / 8) + 3) & !3;
    
    for y in 0..height {
        let src_y = if top_down { y } else { height - 1 - y };
        let row_start = pixel_offset + src_y * row_stride;
        
        if row_start + width * (bpp as usize / 8) > data.len() {
            return None;
        }
        
        for x in 0..width {
            let p = row_start + x * (bpp as usize / 8);
            let b = data[p];
            let g = data[p+1];
            let r = data[p+2];
            let a = if bpp == 32 { data[p+3] } else { 255 };
            
            pixels[src_y * width + x] = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }
    }
    
    Some(BmpImage { width, height, pixels })
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
    pub icon_data: Option<BmpImage>,
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

pub struct GuiState<'boot> {
    pub gop: Option<&'boot mut uefi::proto::console::gop::GraphicsOutput>,
    pub pointers: Vec<uefi::boot::ScopedProtocol<uefi::proto::console::pointer::Pointer>>,
    
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
    
    pub bg_image: Option<BmpImage>,
    
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
            pointers: Vec::new(),
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
            cursor_x: -1,
            cursor_y: -1,
            title: None,
            show_title: true,
            show_names: true,
            title_color: COLOR_WHITE,
            name_color: COLOR_WHITE,
            bg_color: COLOR_DARK_BG,
            highlight_color: COLOR_BLUE,
            bg_image: None,
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

    pub fn draw_image(&mut self, x: usize, y: usize, img: &BmpImage) {
        if let Some(buf) = &mut self.backbuffer {
            Self::draw_image_buf(buf, self.screen_width, self.screen_height, x, y, img);
        }
    }

    pub fn draw_image_buf(
        buf: &mut [BltPixel], 
        screen_width: usize, 
        screen_height: usize, 
        x: usize, 
        y: usize, 
        img: &BmpImage
    ) {
        let ex = x.saturating_add(img.width).min(screen_width);
        let ey = y.saturating_add(img.height).min(screen_height);
        
        for py in y..ey {
            for px in x..ex {
                let img_x = px - x;
                let img_y = py - y;
                let pixel = img.pixels[img_y * img.width + img_x];
                let a = (pixel >> 24) as u32;
                
                if a == 0 { continue; }
                
                let r = ((pixel >> 16) & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let b = (pixel & 0xFF) as u8;
                let buf_idx = py * screen_width + px;
                
                if a == 255 {
                    buf[buf_idx] = BltPixel::new(r, g, b);
                } else {
                    let bg = buf[buf_idx];
                    let inv_a = 255 - a;
                    
                    let out_r = ((r as u32 * a + bg.red as u32 * inv_a) / 255) as u8;
                    let out_g = ((g as u32 * a + bg.green as u32 * inv_a) / 255) as u8;
                    let out_b = ((b as u32 * a + bg.blue as u32 * inv_a) / 255) as u8;
                    
                    buf[buf_idx] = BltPixel::new(out_r, out_g, out_b);
                }
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
        if let (Some(bg), Some(buf)) = (&self.bg_image, &mut self.backbuffer) {
            Self::draw_image_buf(buf, self.screen_width, self.screen_height, 0, 0, bg);
        }
        
        let title_text = self.title.clone().unwrap_or(alloc::string::String::from("Talaria Bootloader"));
        if self.show_title {
            let scale = 3;
            let text_w = title_text.len() * 8 * scale;
            let x = (self.screen_width.saturating_sub(text_w)) / 2;
            let y = self.screen_height / 10;
            self.draw_text(x, y, &title_text, self.title_color, scale);
        }

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
            
            let box_color = if is_selected { self.highlight_color } else { Color { r: 50, g: 50, b: 50 } };
            self.fill_rect(start_x, start_y, entry_width, entry_height, box_color);
            
            if let Some(icon) = &self.entries[i].icon_data {
                let ix = start_x + (entry_width.saturating_sub(icon.width)) / 2;
                let iy = start_y + (entry_height.saturating_sub(icon.height)) / 2 - 10;
                if let Some(buf) = &mut self.backbuffer {
                    Self::draw_image_buf(buf, self.screen_width, self.screen_height, ix, iy, icon);
                }
            } else {
                let inner_color = if has_color { entry_color } else { Color { r: 80, g: 80, b: 80 } };
                self.fill_rect(start_x + 5, start_y + 5, entry_width - 10, entry_height - 50, inner_color);
            }
            
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
        
        self.draw_power_buttons();
        self.draw_cursor();
    }
    
    pub fn draw_power_buttons(&mut self) {
        let padding = 10;
        let box_size = 40;
        let mut x = self.screen_width.saturating_sub(padding + box_size);
        let y = self.screen_height.saturating_sub(padding + box_size);
        
        self.fill_rect(x, y, box_size, box_size, COLOR_GREEN);
        self.draw_text(x + 12, y + 12, "F", COLOR_BLACK, 2);
        
        x = x.saturating_sub(padding + box_size);
        self.fill_rect(x, y, box_size, box_size, COLOR_ORANGE);
        self.draw_text(x + 12, y + 12, "R", COLOR_BLACK, 2);
        
        x = x.saturating_sub(padding + box_size);
        self.fill_rect(x, y, box_size, box_size, COLOR_RED);
        self.draw_text(x + 12, y + 12, "S", COLOR_BLACK, 2);
    }
    
    pub fn draw_cursor(&mut self) {
        if self.cursor_x == -1 || self.cursor_y == -1 { return; }
        let cx = self.cursor_x as usize;
        let cy = self.cursor_y as usize;
        
        let cursor_color = Color { r: 200, g: 200, b: 200 };
        let outline = Color { r: 0, g: 0, b: 0 };
        
        self.fill_rect(cx.saturating_sub(2), cy.saturating_sub(2), 5, 5, outline);
        self.fill_rect(cx.saturating_sub(1), cy.saturating_sub(1), 3, 3, cursor_color);
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
            
            let mut any_dx = 0;
            let mut any_dy = 0;
            let mut any_btn = false;
            
            for pointer in &mut self.pointers {
                if let Ok(Some(state)) = pointer.read_state() {
                    input_received = true;
                    any_dx += state.relative_movement[0] as isize;
                    any_dy += state.relative_movement[1] as isize;
                    if state.button[0] {
                        any_btn = true;
                    }
                }
            }
            
            if input_received && (any_dx != 0 || any_dy != 0 || any_btn) {
                    let dx = any_dx;
                    let dy = any_dy;
                    
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
                    
                    if any_btn {
                        let cx = self.cursor_x as usize;
                        let cy = self.cursor_y as usize;
                        
                        // Check power buttons
                        let padding = 10;
                        let box_size = 40;
                        let f_x = self.screen_width.saturating_sub(padding + box_size);
                        let r_x = f_x.saturating_sub(padding + box_size);
                        let s_x = r_x.saturating_sub(padding + box_size);
                        let p_y = self.screen_height.saturating_sub(padding + box_size);
                        
                        if cy >= p_y && cy <= p_y + box_size {
                            if cx >= f_x && cx <= f_x + box_size {
                                self.action = TALARIA_ACTION_FIRMWARE;
                                self.running = false;
                                return None;
                            } else if cx >= r_x && cx <= r_x + box_size {
                                self.action = TALARIA_ACTION_REBOOT;
                                self.running = false;
                                return None;
                            } else if cx >= s_x && cx <= s_x + box_size {
                                self.action = TALARIA_ACTION_SHUTDOWN;
                                self.running = false;
                                return None;
                            }
                        }
                        
                        // Check boot entries
                        let entry_height = 100;
                        let entry_width = 300;
                        let num_entries = self.entries.len();
                        let total_w = num_entries * entry_width + (num_entries.saturating_sub(1)) * 20;
                        let mut start_x = (self.screen_width.saturating_sub(total_w)) / 2;
                        let start_y = (self.screen_height.saturating_sub(entry_height)) / 2;
                        
                        for i in 0..num_entries {
                            if cx >= start_x && cx <= start_x + entry_width && cy >= start_y && cy <= start_y + entry_height {
                                self.selected = i;
                                self.running = false;
                                return Some(self.entries[self.selected].clone());
                            }
                            start_x += entry_width + 20;
                        }
                    }
                }

            if !input_received {
                if timeout_ticks >= 0 {
                    if ticks >= timeout_ticks && !self.entries.is_empty() {
                        self.running = false;
                        return Some(self.entries[self.selected].clone());
                    }
                    ticks += 1;
                    let new_timeout = (timeout_ticks - ticks + 99) / 100;
                    if new_timeout != self.timeout {
                        self.timeout = new_timeout;
                        self.dirty = true;
                    }
                }
            } else {
                if timeout_ticks != -1 {
                    timeout_ticks = -1; // Abort timeout on input
                    self.timeout = -1;
                    self.dirty = true;
                }
            }
            
            // Frame-rate limiter: unconditionally wait 10ms to prevent PCIe bus saturation when input is active
            uefi::boot::stall(core::time::Duration::from_micros(10_000));
        }
        
        None
    }
}
