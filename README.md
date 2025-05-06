# Talaria

A minimal, fast, modern, graphical UEFI boot manager written entirely in memory-safe **Rust** (`#![no_std]`).

Talaria draws an icon-based boot menu capable of booting **Linux** (EFI stub kernels / Unified Kernel Images) or chainloading other EFI executables (including Windows Boot Manager) with no external dependencies.

## ⚠️ Development Status & Testing

> **Note:** This project began as a rapidly vibe-coded AI pair programming experiment to build a modern, memory-safe boot manager from scratch. 
>
> While the codebase has undergone a rigorous audit and boots successfully inside QEMU emulators, **it has not yet been tested on real, bare-metal hardware.** Expect bugs and proceed with caution.

**Current Testing Status:**

- [x] Rust `no_std` UEFI compilation pipeline (`x86_64-unknown-uefi`)
- [x] Automated QEMU / VirtualBox disk image creation (`build_vdi.sh`)
- [x] UEFI GOP graphical buffer initialization
- [x] `font8x8` text rasterization and rendering
- [x] Complete static analysis, linting, and memory leak patching
- [ ] Bare-metal UEFI firmware testing
- [ ] End-to-end OS kernel handoff (Linux EFI Stub & Windows Boot Manager)
- [ ] Pointer protocol (Mouse/Touchscreen) verification

## Features (Rust Port)

- **Graphical Menu** — modern rendering via the UEFI Graphics Output Protocol (GOP) with a custom zero-allocation rendering engine.
- **True 32-bit ARGB Blending** — seamless, alpha-blended UI components and OS distro icons (BMP) against custom backgrounds.
- **Zero-Flicker Instant Boot** — fully silent and immediate OS handoff when `timeout=0` is configured.
- **Auto-detection** — scans for common Linux and Windows loaders and builds a menu automatically if no config is found.
- **Secure Boot aware** — verifies images through shim's `SHIM_LOCK` protocol when present.

## Requirements

- An **x86_64 UEFI** system
- **Rust Toolchain** (Nightly required for UEFI target compilation)

### Install dependencies

You will need `rustup` installed on your system to compile Talaria.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly
rustup component add rust-src
rustup target add x86_64-unknown-uefi
```

## Building

```bash
cargo build --target x86_64-unknown-uefi --release
```

This produces `talaria-bootloader.efi` inside `target/x86_64-unknown-uefi/release/`.

### Local Testing (QEMU / VirtualBox)

If you'd like to test the bootloader locally without modifying your actual EFI system partition, you can use the provided script to generate a virtual FAT32 drive containing the bootloader and a dummy configuration.

```bash
# 1. Install prerequisites (mtools, qemu, ovmf)
# Ubuntu/Debian: sudo apt install mtools qemu-system-x86 ovmf
# Arch Linux:    sudo pacman -S mtools qemu-base edk2-ovmf

# 2. Build the project and package it into a VirtualBox / QEMU disk image
./build_vdi.sh

# 3. Boot the image in QEMU (using OVMF UEFI firmware)
qemu-system-x86_64 -bios /usr/share/edk2/x64/OVMF.4m.fd -drive file=talaria_test.vdi,format=vdi
```

## Configuration

Config lives at `\EFI\talaria\boot.conf` on the ESP.

**Path rules:** all paths are relative to the **root of the ESP** and use back-slashes, e.g. `\EFI\talaria\vmlinuz-linux`.

### Boot entries (Examples)

Talaria auto-detects how to boot each one from the image itself.

```conf
entry {
    name    = "Arch Linux"
    kernel  = \vmlinuz-linux
    initrd  = \initramfs-linux.img
    cmdline = "root=PARTUUID=... rw quiet"
}

entry {
    name   = "Windows 11"
    kernel = \EFI\Microsoft\Boot\bootmgfw.efi
}
```

## Controls

| Key            | Action                            |
| -------------- | --------------------------------- |
| `Left`/`Right` | Move between boot entries (wraps) |
| `Enter`        | Boot the focused entry            |
| `Esc`          | Boot the default entry            |

## Credits

This repository is a Rust fork of the original [Visor-BootManager](https://github.com/IO-ZetZor/Visor-BootManager) project by **IO-ZetZor**. The original project provided the inspiration and architectural foundation for this UEFI boot manager.
