#!/usr/bin/env bash
set -e

# Cargo passes the path to the compiled .efi binary as the first argument
EFI_BINARY=$1

if [ -z "$EFI_BINARY" ]; then
    # If run manually, compile first and set the binary path
    echo "Compiling the bootloader..."
    cargo build --target x86_64-unknown-uefi
    EFI_BINARY="target/x86_64-unknown-uefi/debug/talaria-bootloader.efi"
fi

# Set up the mock EFI System Partition (ESP)
echo "Setting up ESP environment..."
mkdir -p esp/EFI/BOOT
mkdir -p esp/talaria

# Copy our newly compiled bootloader into the standard fallback location
cp "$EFI_BINARY" esp/EFI/BOOT/BOOTX64.EFI

# Safely copy configuration or assets (if they exist) into the ESP
if [ -f "boot.conf" ]; then
    cp boot.conf esp/talaria/
fi

# Resolve the correct OS-agnostic OVMF path
if [ -f "/usr/share/edk2/x64/OVMF.4m.fd" ]; then
    OVMF="/usr/share/edk2/x64/OVMF.4m.fd"
elif [ -f "/usr/share/edk2/x64/OVMF.fd" ]; then
    OVMF="/usr/share/edk2/x64/OVMF.fd"
elif [ -f "/usr/share/ovmf/OVMF.fd" ]; then
    OVMF="/usr/share/ovmf/OVMF.fd"
    # Standard path for Debian / Ubuntu
    OVMF="/usr/share/ovmf/OVMF.fd"
else
    echo "Error: OVMF firmware not found!"
    echo "Please install QEMU and OVMF:"
    echo "  - Ubuntu/Debian: sudo apt install qemu-system-x86 ovmf"
    echo "  - Arch Linux: sudo pacman -S qemu-base edk2-ovmf"
    exit 1
fi

echo "Starting QEMU..."
qemu-system-x86_64 \
    -bios "$OVMF" \
    -drive format=raw,file=fat:rw:esp \
    -net none \
     \
    -m 256 -nographic
