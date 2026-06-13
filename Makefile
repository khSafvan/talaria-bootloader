# Minimal EFI Boot Manager Makefile
# Targets x86_64 UEFI systems

ARCH = x86_64
TARGET = visor_x64.efi
BUILD_DIR = build
# -fno-asynchronous-unwind-tables / -fno-unwind-tables: stop GCC emitting
# .eh_frame DWARF unwind data.  In a freestanding -shared EFI link binutils
# trips over it (".eh_frame_hdr refers to overlapping FDEs"), and UEFI has no
# use for C++-style unwinding anyway.
# -fvisibility=hidden: CRITICAL.  gnu-efi's crt0 self-relocator (_relocate)
# only applies R_X86_64_RELATIVE relocs — it does NOT process JUMP_SLOT or
# GLOB_DAT.  Without hidden visibility our own functions are default-visibility
# (preemptible) in this -shared link, so every intra-image call is routed
# through a PLT whose GOT slots get JUMP_SLOT relocs that are never resolved.
# crt0 even calls efi_main through that PLT, so the first call faults into
# garbage before any output.  Hidden visibility makes the linker bind these
# calls directly (PC-relative), leaving only RELATIVE relocs.  (gnu-efi's own
# library is built this way, which is why libefi calls were never affected.)
# -DGNU_EFI_USE_MS_ABI: CRITICAL.  UEFI firmware uses the Microsoft x64 calling
# convention (args in RCX/RDX/R8/R9); GCC defaults to System V (RDI/RSI/...).
# This code calls firmware function pointers DIRECTLY (e.g.
# ST->ConOut->OutputString(...)), which is only correct when gnu-efi's EFIAPI
# expands to __attribute__((ms_abi)) -- i.e. when this macro is defined.
# Without it every firmware call passed args in the wrong registers, so the
# firmware dereferenced garbage and faulted (observed: #UD inside OutputString).
# NOTE: the prebuilt crt0/libgnuefi are SysV-built and call efi_main via SysV,
# so efi_main must NOT be EFIAPI (see main.c).  Our own functions are never
# EFIAPI, so they stay SysV and remain mutually consistent.
EFI_CFLAGS = -ffreestanding -fno-stack-protector -fno-strict-aliasing \
             -fno-asynchronous-unwind-tables -fno-unwind-tables \
             -fpic -fshort-wchar -fvisibility=hidden -DGNU_EFI_USE_MS_ABI \
             -mno-red-zone -Wall -Wextra -O2 -I include

# --- Toolchain (override on the command line if needed) ----------------------
# Prefer the cross-prefixed gcc when present, otherwise the native gcc.  The
# origin check lets `make CC=...` or an exported CC win while still replacing
# make's built-in default of `cc`.
ifeq ($(origin CC),default)
CC := $(shell command -v x86_64-linux-gnu-gcc 2>/dev/null || command -v gcc 2>/dev/null || echo cc)
endif
OBJCOPY ?= objcopy

# --- gnu-efi location auto-detection -----------------------------------------
# Distros disagree on where gnu-efi lives:
#   Arch / Debian / Ubuntu : headers /usr/include/efi, libs /usr/lib
#   Fedora / openSUSE       : libs and crt0/lds under /usr/lib64/gnuefi
# Each of these is overridable: `make GNU_EFI_INC=/path CRT0=/path/crt0.o ...`
GNU_EFI_INC ?= $(firstword $(wildcard /usr/include/efi /usr/local/include/efi))
CRT0        ?= $(firstword $(wildcard \
                   /usr/lib/crt0-efi-x86_64.o \
                   /usr/lib64/gnuefi/crt0-efi-x86_64.o \
                   /usr/lib/gnuefi/crt0-efi-x86_64.o))
GNU_EFI_LIB ?= $(dir $(CRT0))
# gnu-efi ships its own linker script; prefer ours but fall back to the system
# one if a distro needs it.  Ours (visor_x86_64.lds) is committed.
LDS  = visor_x86_64.lds
VERS = efi.vers

# FIX: crt0-efi-x86_64.o, -lefi, -lgnuefi give the binary a proper EFI
#      entry-point stub plus the gnu-efi runtime helpers.
# FIX: -z notext — binutils >= 2.39 defaults to -z text, which rejects the
#      crt0 PC-relative relocation against the linker-defined ImageBase symbol
#      ("relocation R_X86_64_PC32 ... can not be used when making a shared
#      object").  gnu-efi resolves these at runtime, so text relocs are fine.
# --version-script=efi.vers localizes ALL symbols so the linker emits no PLT/GOT
# (no JUMP_SLOT/GLOB_DAT relocs).  Without it, crt0 calls _relocate through an
# unrelocated PLT slot and faults before relocation runs.  See efi.vers.
EFI_LDFLAGS = -nostdlib -znocombreloc -z notext -T $(LDS) -shared \
              -Bsymbolic -Wl,--version-script=$(VERS) -L $(GNU_EFI_LIB) \
              $(CRT0) -lefi -lgnuefi

SRCS = main.c efi_helpers.c gui.c config.c linux_boot.c windows_boot.c \
       png_decoder.c font_jetbrains.c
OBJS = $(SRCS:.c=.o)

# Font baking: regenerate font_jetbrains.c from any TTF and pixel size.
#   make bakefont FONT=/path/to/Some.ttf FONT_PX=64
# The baked C file is committed, so a normal build does NOT require Python.
FONT    ?= /usr/share/fonts/TTF/JetBrainsMono-Regular.ttf
FONT_PX ?= 64

.PHONY: all clean install bakefont check-env

all: check-env $(TARGET)

# Fail early with a helpful message if gnu-efi is not installed.
check-env:
	@test -n "$(GNU_EFI_INC)" || { \
	  echo "ERROR: gnu-efi headers not found (looked in /usr/include/efi)."; \
	  echo "  Install gnu-efi:  Arch: pacman -S gnu-efi | Debian/Ubuntu: apt install gnu-efi"; \
	  echo "                    Fedora: dnf install gnu-efi | openSUSE: zypper in gnu-efi-devel"; \
	  echo "  Or override:  make GNU_EFI_INC=/path/to/efi CRT0=/path/to/crt0-efi-x86_64.o"; \
	  exit 1; }
	@test -n "$(CRT0)" || { \
	  echo "ERROR: crt0-efi-x86_64.o not found. Install gnu-efi or set CRT0=..."; exit 1; }

bakefont:
	python3 tools/bake_font.py "$(FONT)" $(FONT_PX) jetbrains font_jetbrains.c

$(TARGET): $(OBJS)
	@mkdir -p $(BUILD_DIR)
	$(CC) $(EFI_LDFLAGS) -o $(BUILD_DIR)/visor.so $(OBJS)
	# Modern binutils dropped the efi-app-x86_64 BFD target, so convert the
	# gnu-efi ELF shared object to PE/COFF (pei-x86-64) and stamp the EFI
	# Application subsystem (10) instead of using --target=efi-app-x86_64.
	$(OBJCOPY) -j .text -j .sdata -j .data -j .rodata -j .dynamic \
		   -j .dynsym  -j .dynstr -j .rel* -j .reloc \
		   -O pei-x86-64 --subsystem=10 $(BUILD_DIR)/visor.so $(TARGET)

%.o: %.c
	$(CC) $(EFI_CFLAGS) \
	      -I $(GNU_EFI_INC) \
	      -I $(GNU_EFI_INC)/$(ARCH) \
	      -c $< -o $@

clean:
	rm -f $(OBJS) $(TARGET) $(BUILD_DIR)/*

# Build then hand off to install.sh, which locates the ESP and copies assets.
install: $(TARGET)
	./install.sh
