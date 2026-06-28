#![allow(dead_code)]

use alloc::vec::Vec;
use crate::gui::{BootEntry, Color};

pub fn parse_color(val: &str) -> Option<Color> {
    let val = val.trim().trim_start_matches('#');
    if val.len() == 6 && val.is_ascii() 
        && let Ok(r) = u8::from_str_radix(&val[0..2], 16)
        && let (Ok(g), Ok(b)) = (u8::from_str_radix(&val[2..4], 16), u8::from_str_radix(&val[4..6], 16)) {
        return Some(Color { r, g, b });
    }
    None
}

pub struct Config<'a> {
    pub timeout: isize,
    pub default_entry: usize,
    pub quiet: bool,
    pub text_menu: bool,
    pub res_w: usize,
    pub res_h: usize,
    pub res_max: bool,
    pub def_cmdline: Option<&'a str>,
    pub show_names: bool,
    pub center_info: bool,
    pub box_radius: usize,
    pub remember_last: bool,
    pub recovery_entries: bool,
    pub mouse: bool,
    pub editor: bool,
    pub theme: Option<&'a str>,
    pub title: Option<&'a str>,
    pub no_title: bool,
    pub font: Option<&'a str>,
    pub background: Option<&'a str>,
    pub bg_color: Color,
    pub fg_color: Color,
    pub highlight_color: Color,
    pub title_color: Color,
    pub name_color: Color,
    pub title_size: usize,
    pub name_size: usize,
    pub icon_size: usize,
    pub icon_spacing: usize,
    pub icon_y: usize,
    pub underline_color: Color,
    pub has_underline_color: bool,
    pub underline_thickness: usize,
    pub underline_length: usize,
    pub power_position: i32,
    pub shutdown_color: Color,
    pub reboot_color: Color,
    pub firmware_color: Color,
    pub has_shutdown_color: bool,
    pub has_reboot_color: bool,
    pub has_firmware_color: bool,
    pub power_icons: bool,
    pub power_icon_size: usize,
    pub shutdown_icon: Option<&'a str>,
    pub reboot_icon: Option<&'a str>,
    pub firmware_icon: Option<&'a str>,
    pub blur: i32,
    pub blur_title: bool,
    pub blur_color: Color,
    pub has_blur_color: bool,
    pub anim_speed: i32,
    pub entries_per_page: usize,
    pub entries: Vec<BootEntry>,
}

impl<'a> Default for Config<'a> {
    fn default() -> Self {
        Self {
            timeout: 5,
            default_entry: 0,
            quiet: false,
            text_menu: false,
            res_w: 0,
            res_h: 0,
            res_max: false,
            def_cmdline: None,
            show_names: true,
            center_info: false,
            box_radius: 12,
            remember_last: false,
            recovery_entries: false,
            mouse: true,
            editor: true,
            theme: None,
            title: None,
            no_title: false,
            font: None,
            background: None,
            bg_color: crate::gui::COLOR_DARK_BG,
            fg_color: crate::gui::COLOR_WHITE,
            highlight_color: crate::gui::COLOR_BLUE,
            title_color: crate::gui::COLOR_WHITE,
            name_color: crate::gui::COLOR_WHITE,
            title_size: 24,
            name_size: 16,
            icon_size: 64,
            icon_spacing: 20,
            icon_y: 0,
            underline_color: crate::gui::COLOR_WHITE,
            has_underline_color: false,
            underline_thickness: 2,
            underline_length: 0,
            power_position: crate::gui::POWER_POS_BOTTOMRIGHT,
            shutdown_color: crate::gui::COLOR_RED,
            reboot_color: crate::gui::COLOR_ORANGE,
            firmware_color: crate::gui::COLOR_GRAY,
            has_shutdown_color: false,
            has_reboot_color: false,
            has_firmware_color: false,
            power_icons: true,
            power_icon_size: 48,
            shutdown_icon: None,
            reboot_icon: None,
            firmware_icon: None,
            blur: 0,
            blur_title: false,
            blur_color: crate::gui::COLOR_BLACK,
            has_blur_color: false,
            anim_speed: 1,
            entries_per_page: 8,
            entries: Vec::new(),
        }
    }
}

impl<'a> Config<'a> {
    pub fn parse_str(buf: &'a str) -> Self {
        let mut config = Config::default();
        let mut in_entry_block = false;
        let mut current_entry = BootEntry {
            name: alloc::string::String::new(),
            icon_path: None,
            icon_data: None,
            kernel_path: None,
            initrd_path: None,
            cmdline: None,
            uuid: None,
            index: 0,
            entry_type: 0,
            icon_size: 64,
            color: crate::gui::COLOR_WHITE,
            has_color: false,
            sha256: [0; 32],
            has_sha256: false,
        };
        
        for line in buf.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let lower = line.to_ascii_lowercase();
            if lower.ends_with('{') && (lower.starts_with("entry") || lower.starts_with("linux") || lower.starts_with("windows")) {
                in_entry_block = true;
                current_entry = BootEntry {
                    name: alloc::string::String::new(),
                    icon_path: None,
                    icon_data: None,
                    kernel_path: None,
                    initrd_path: None,
                    cmdline: None,
                    uuid: None,
                    index: 0,
                    entry_type: 0,
                    icon_size: 64,
                    color: crate::gui::COLOR_WHITE,
                    has_color: false,
                    sha256: [0; 32],
                    has_sha256: false,
                };
                continue;
            }

            if in_entry_block && line == "}" {
                in_entry_block = false;
                current_entry.index = config.entries.len();
                config.entries.push(current_entry.clone());
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let mut value = value.trim();
                
                // Strip quotes if present
                if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                    value = &value[1..value.len()-1];
                }
                
                if in_entry_block {
                    match key {
                        "name" => current_entry.name = alloc::string::String::from(value),
                        "icon" => current_entry.icon_path = Some(alloc::string::String::from(value)),
                        "kernel" => current_entry.kernel_path = Some(alloc::string::String::from(value)),
                        "initrd" => current_entry.initrd_path = Some(alloc::string::String::from(value)),
                        "cmdline" => current_entry.cmdline = Some(alloc::string::String::from(value)),
                        "sha256" => {
                            if value.len() == 64 && value.is_ascii() {
                                let mut hash = [0u8; 32];
                                let mut valid = true;
                                for i in 0..32 {
                                    match u8::from_str_radix(&value[i*2..i*2+2], 16) {
                                        Ok(b) => hash[i] = b,
                                        Err(_) => { valid = false; break; }
                                    }
                                }
                                if valid {
                                    current_entry.sha256 = hash;
                                    current_entry.has_sha256 = true;
                                }
                            }
                        }
                        "color" => {
                            if let Some(c) = parse_color(value) {
                                current_entry.color = c;
                                current_entry.has_color = true;
                            }
                        }
                        _ => {}
                    }
                } else {
                    match key {
                        "timeout" => {
                            if let Ok(v) = value.parse::<isize>() {
                                config.timeout = v;
                            }
                        }
                        "quiet" => {
                            config.quiet = value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes");
                        }
                        "default" => {
                            if let Ok(v) = value.parse::<usize>() {
                                config.default_entry = v;
                            }
                        }
                        "bg_color" => {
                            if let Some(c) = parse_color(value) { config.bg_color = c; }
                        }
                        "fg_color" | "name_color" | "title_color" => {
                            if let Some(c) = parse_color(value) { 
                                config.fg_color = c;
                                config.name_color = c;
                                config.title_color = c;
                            }
                        }
                        "highlight_color" => {
                            if let Some(c) = parse_color(value) { config.highlight_color = c; }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        if in_entry_block {
            current_entry.index = config.entries.len();
            config.entries.push(current_entry);
        }
        
        config
    }
}
