#!/usr/bin/env bash
set -e

# Compile the bootloader
echo "Compiling bootloader..."
cargo build --target x86_64-unknown-uefi
EFI_BINARY="target/x86_64-unknown-uefi/debug/talaria-bootloader.efi"

# Clean up old images
rm -f uefi_disk.img uefi_disk.vdi

# 1. Create a blank 64MB file
echo "Allocating 64MB raw image..."
dd if=/dev/zero of=uefi_disk.img bs=1M count=256

# 2. Format it as a FAT32 filesystem
echo "Formatting as FAT32..."
mkfs.vfat -F 32 uefi_disk.img

# 3. Create the standard UEFI directory structure inside the image
echo "Building UEFI directory tree..."
mmd -i uefi_disk.img ::/EFI
mmd -i uefi_disk.img ::/EFI/BOOT
mmd -i uefi_disk.img ::/EFI/talaria
mmd -i uefi_disk.img ::/EFI/talaria/icons
mmd -i uefi_disk.img ::/EFI/talaria/backgrounds

# 4. Copy your Rust bootloader into it as the default boot file
echo "Copying bootloader executable to Virtual Disk..."
mcopy -i uefi_disk.img "$EFI_BINARY" ::/EFI/BOOT/BOOTX64.EFI

# 5. Copy config and assets (if available)
if [ -f "boot.conf" ]; then
    echo "Copying boot.conf..."
    mcopy -i uefi_disk.img boot.conf ::/EFI/talaria/
fi

if [ -d "assets/icons" ]; then
    echo "Copying icons..."
    mcopy -s -i uefi_disk.img assets/icons/*.bmp ::/EFI/talaria/icons/
fi

if [ -d "assets/backgrounds" ]; then
    echo "Copying backgrounds..."
    mcopy -s -i uefi_disk.img assets/backgrounds/*.bmp ::/EFI/talaria/backgrounds/
fi

# 6. Convert to VirtualBox format
echo "Converting to VirtualBox VDI format..."
if command -v vboxmanage &> /dev/null; then
    vboxmanage convertfromraw uefi_disk.img uefi_disk.vdi --format VDI
    echo "Successfully generated uefi_disk.vdi!"
else
    echo "Warning: vboxmanage not found on this system. You will need to run the conversion on your host OS using:"
    echo "vboxmanage convertfromraw uefi_disk.img uefi_disk.vdi --format VDI"
fi
