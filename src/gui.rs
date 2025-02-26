#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

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
    pub gop: Option<&'boot mut uefi::proto::console::gop::GraphicsOutput<'boot>>,
    pub screen_width: usize,
    pub screen_height: usize,
    pub bpp: usize,
    pub pixels_per_scanline: usize,
    pub backbuffer: Option<Vec<u32>>,
    
    pub entries: Vec<BootEntry>,
    pub selected: usize,
    pub per_page: usize,
    pub running: bool,
    pub action: i32,
    pub focus: i32,
    
    pub title: Option<String>,
    pub show_title: bool,
    pub show_names: bool,
    pub title_color: Color,
    pub name_color: Color,
    pub bg_color: Color,
    pub highlight_color: Color,
    
    pub scene_cache: Option<Vec<u32>>,
    pub scene_valid: bool,
}

impl<'boot> GuiState<'boot> {
    pub fn new() -> Self {
        Self {
            gop: None,
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
            title: None,
            show_title: true,
            show_names: true,
            title_color: COLOR_WHITE,
            name_color: COLOR_WHITE,
            bg_color: COLOR_DARK_BG,
            highlight_color: COLOR_BLUE,
            scene_cache: None,
            scene_valid: false,
        }
    }

    pub fn init(&mut self) -> uefi::Status {
        // Here we would use the UEFI boot services to locate the GOP protocol
        // and initialize our backbuffer.
        uefi::Status::SUCCESS
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        if let Some(buf) = &mut self.backbuffer {
            let pixel = ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32);
            let ex = (x + w).min(self.screen_width);
            let ey = (y + h).min(self.screen_height);
            
            for j in y..ey {
                let start = j * self.screen_width + x;
                let end = start + (ex - x);
                buf[start..end].fill(pixel);
            }
        }
    }
    
    pub fn run(&mut self) -> Option<BootEntry> {
        self.running = true;
        // Basic stub for the main GUI loop.
        // In reality, this waits for user input and updates the screen.
        None
    }
}
