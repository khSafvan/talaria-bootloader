# Talaria Bootloader

<p align="center">
  <em>A minimal, fast, and modern graphical UEFI boot manager written entirely in memory-safe Rust.</em>
</p>

---

## ⚡ Overview

Talaria is a lightweight, zero-dependency UEFI boot manager built on a `#![no_std]` Rust architecture. It provides a stunning, icon-based graphical boot menu capable of launching Linux (EFI stub kernels / Unified Kernel Images) or chainloading other EFI executables (like the Windows Boot Manager). 

This repository is a fully audited, oxidized Rust fork of the original [Visor-BootManager](https://github.com/IO-ZetZor/Visor-BootManager) by **IO-ZetZor**.

## ✨ Features

- **Blazing Fast Handoff:** Engineered with a custom, zero-allocation rendering engine that minimizes heap fragmentation and UEFI API overhead.
- **True 32-bit ARGB Blending:** Supports seamless, alpha-blended UI components and high-quality OS distro icons (`.bmp`) overlaid dynamically on custom backgrounds.
- **Zero-Flicker Instant Boot:** Delivers a fully silent, immediate OS handoff when `timeout=0` is configured—no screen flashing or UI initialization artifacts.
- **Smart Auto-Detection:** Automatically scans the EFI System Partition (ESP) for common Linux and Windows loaders, constructing a boot menu dynamically if no configuration file is present.
- **Secure Boot Integration:** Actively checks for and integrates with the `shim` `SHIM_LOCK` protocol, ensuring secure OS verification chains remain unbroken.
- **Mouse & Touchpad Support:** Fully interactive GUI using the UEFI `SimplePointer` protocol.

## 🛠️ Prerequisites

To compile Talaria from source, you need a standard `x86_64` UEFI system and the Rust Nightly toolchain.

```bash
# Install rustup if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Configure the Nightly toolchain and required components
rustup default nightly
rustup component add rust-src
rustup target add x86_64-unknown-uefi
```

## 🚀 Building & Installation

Compile the bootloader in release mode to strip debug symbols and maximize execution speed:

```bash
cargo build --target x86_64-unknown-uefi --release
```

The resulting EFI binary will be located at:
`target/x86_64-unknown-uefi/release/talaria-bootloader.efi`

### Deployment

1. Rename the compiled `.efi` file to `BOOTX64.EFI` (or register it manually via `efibootmgr`).
2. Place it in your EFI System Partition (ESP) under `\EFI\BOOT\BOOTX64.EFI`.
3. Create the configuration directory at `\EFI\talaria\`.

## ⚙️ Configuration

Configuration is managed via a simple text file located at `\EFI\talaria\boot.conf` on your ESP. 

> **Note:** All paths in the configuration file must be relative to the **root** of the ESP and utilize Windows-style backslashes (`\`).

### Example `boot.conf`

```conf
# Global Settings
timeout = 5
default = 0
background = \EFI\talaria\bg.bmp

# Arch Linux (EFI Stub)
entry {
    name    = "Arch Linux"
    icon    = \EFI\talaria\arch.bmp
    kernel  = \vmlinuz-linux
    initrd  = \initramfs-linux.img
    cmdline = "root=PARTUUID=xxxx-xxxx rw quiet"
}

# Windows 11 (Chainload)
entry {
    name   = "Windows 11"
    icon   = \EFI\talaria\win.bmp
    kernel = \EFI\Microsoft\Boot\bootmgfw.efi
}
```

## ⌨️ Controls

| Input Device | Action |
| --- | --- |
| **Mouse/Touchpad** | Move the cursor and click to select an entry. |
| **Left / Right** | Cycle between available boot entries. |
| **Enter** | Boot the currently highlighted entry. |
| **Escape** | Abort the timeout and boot the default entry immediately. |

## 🧪 Testing Locally (QEMU)

For development and safe local testing without modifying your host system's EFI partition, you can use the provided virtualization scripts:

```bash
# 1. Install prerequisites (Debian/Ubuntu example)
sudo apt install mtools qemu-system-x86 ovmf

# 2. Build the project and package it into a virtual disk image
./build_vdi.sh

# 3. Boot the image in QEMU
qemu-system-x86_64 -bios /usr/share/edk2/x64/OVMF.4m.fd -drive file=talaria_test.vdi,format=vdi
```

## 🤝 Credits & Acknowledgements

* **Visor-BootManager:** This project is a Rust fork and architectural modernization of the original [Visor-BootManager](https://github.com/IO-ZetZor/Visor-BootManager) by [IO-ZetZor](https://github.com/IO-ZetZor). The original C codebase provided the core inspiration and structural foundation.
* **uefi-rs:** Leveraging the fantastic [uefi-rs](https://github.com/rust-osdev/uefi-rs) crate for memory-safe UEFI bindings.

## 📄 License

This project is open-source and distributed under the standard MIT License.
