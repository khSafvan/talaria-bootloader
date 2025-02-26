#![allow(dead_code)]

use uefi::prelude::*;
use crate::gui::BootEntry;

/// Boot a Windows entry by locating its bootmgfw.efi on the specific partition
pub fn boot_windows(image_handle: Handle, system_table: &mut SystemTable<Boot>, entry: &BootEntry) -> Status {
    // 1. Locate the Device Path for the Windows bootloader
    // 2. Call LoadImage
    // 3. Call StartImage
    Status::SUCCESS
}

/// Boot a Linux entry using the EFI handover protocol or EFISTUB
pub fn boot_linux(image_handle: Handle, system_table: &mut SystemTable<Boot>, entry: &BootEntry) -> Status {
    // 1. Load the Kernel image into memory
    // 2. Load the Initrd into memory
    // 3. Parse setup_header to find the EFI handover offset
    // 4. Setup linux_efi_handover_t and efi_info
    // 5. Call ExitBootServices to get the final memory map
    // 6. Jump into the Linux kernel entry point
    Status::SUCCESS
}
