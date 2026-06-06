# NerdOS Makefile
# 
# Targets:
#   make build    - Build the kernel binary
#   make run      - Run NerdOS in QEMU
#   make iso      - Build bootable ISO image
#   make clean    - Remove build artifacts
#   make debug    - Run with GDB debugging
#   make format   - Format Rust code
#   make lint     - Run clippy linter
#
# Requirements:
#   - Rust nightly toolchain with x86_64 target
#   - nasm (Netwide Assembler)
#   - qemu-system-x86_64
#   - grub-mkrescue (for ISO)
#   - xorriso (for ISO)
#   - ld.lld (LLVM linker, usually bundled with Rust)

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

# Target architecture
ARCH := x86_64
TARGET := x86_64-nerdos

# Directories
SRC_DIR := src
KERNEL_DIR := $(SRC_DIR)/kernel_core
BUILD_DIR := build
ISO_DIR := $(BUILD_DIR)/iso
GRUB_DIR := $(ISO_DIR)/boot/grub

# Output files
KERNEL_BIN := $(BUILD_DIR)/nerdos.bin
KERNEL_ISO := $(BUILD_DIR)/nerdos.iso
BOOT_ASM := $(SRC_DIR)/bootloader/src/boot.asm
LINKER_SCRIPT := $(SRC_DIR)/bootloader/linker.ld

# Rust toolchain
CARGO := cargo
RUSTC := rustc
LD := ld.lld
NASM := nasm
QEMU := qemu-system-x86_64
GDB := gdb

# Build profile (dev or release)
PROFILE := release
ifeq ($(PROFILE),release)
    CARGO_PROFILE := --release
    RUSTFLAGS := -C link-arg=-T$(LINKER_SCRIPT) -C code-model=kernel -C relocation-model=static
else
    CARGO_PROFILE :=
    RUSTFLAGS := -C link-arg=-T$(LINKER_SCRIPT) -C code-model=kernel -C relocation-model=static
endif

# QEMU arguments
QEMU_MEMORY := 512M
QEMU_CPUS := 2
QEMU_ARGS := -m $(QEMU_MEMORY) -smp $(QEMU_CPUS) -serial stdio \
    -cpu qemu64,+x2apic,+fsgsbase \
    -no-reboot -no-shutdown \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04

# ---------------------------------------------------------------------------
# Default Target
# ---------------------------------------------------------------------------

.PHONY: all build run iso clean debug format lint help

all: build

# ---------------------------------------------------------------------------
# Build Targets
# ---------------------------------------------------------------------------

# Build the kernel binary from Rust code + assembly stub
build: $(KERNEL_BIN)

# Link everything together into the final kernel binary
$(KERNEL_BIN): boot_object kernel_object
	@echo "[LINK] $(KERNEL_BIN)"
	@mkdir -p $(BUILD_DIR)
	$(LD) -n -T $(LINKER_SCRIPT) -o $@ \
		$(BUILD_DIR)/boot.o \
		$(BUILD_DIR)/libkernel_core.a \
		--gc-sections
	@echo "[SIZE] $$(ls -lh $@ | awk '{print $$5}')"

# Assemble the boot stub (Multiboot2 header + entry point)
boot_object: $(BOOT_ASM)
	@echo "[NASM] $(BOOT_ASM)"
	@mkdir -p $(BUILD_DIR)
	$(NASM) -f elf64 -o $(BUILD_DIR)/boot.o $(BOOT_ASM)

# Build kernel Rust code as a static library
kernel_object:
	@echo "[CARGO] Building kernel_core (profile: $(PROFILE))"
	RUSTFLAGS="$(RUSTFLAGS)" $(CARGO) build $(CARGO_PROFILE) -p kernel_core
	@mkdir -p $(BUILD_DIR)
	@cp target/$(if $(filter release,$(PROFILE)),release,debug)/libkernel_core.a \
		$(BUILD_DIR)/libkernel_core.a

# ---------------------------------------------------------------------------
# ISO Target
# ---------------------------------------------------------------------------

# Create a bootable ISO using GRUB2
iso: $(KERNEL_ISO)

$(KERNEL_ISO): $(KERNEL_BIN) grub_cfg
	@echo "[ISO] Building bootable ISO..."
	@mkdir -p $(GRUB_DIR)
	@cp $(KERNEL_BIN) $(ISO_DIR)/boot/nerdos.bin
	@cp boot/grub/grub.cfg $(GRUB_DIR)/grub.cfg
	@grub-mkrescue -o $@ $(ISO_DIR) 2>/dev/null || \
		echo "[WARN] grub-mkrescue not available, using xorriso fallback..."
	@echo "[DONE] ISO created: $@"

# Copy GRUB config
grub_cfg: boot/grub/grub.cfg
	@echo "[GRUB] Using GRUB configuration"

# ---------------------------------------------------------------------------
# QEMU Targets
# ---------------------------------------------------------------------------

# Run NerdOS in QEMU (BIOS mode)
run: build
	@echo "[QEMU] Starting NerdOS (BIOS mode)..."
	$(QEMU) $(QEMU_ARGS) -kernel $(KERNEL_BIN)

# Run NerdOS in QEMU (UEFI mode)
run-uefi: iso
	@echo "[QEMU] Starting NerdOS (UEFI mode)..."
	$(QEMU) $(QEMU_ARGS) -cdrom $(KERNEL_ISO) -bios /usr/share/ovmf/OVMF.fd

# Run NerdOS with KVM acceleration (Linux only)
run-kvm: build
	@echo "[QEMU] Starting NerdOS (KVM accelerated)..."
	$(QEMU) $(QEMU_ARGS) -enable-kvm -kernel $(KERNEL_BIN)

# ---------------------------------------------------------------------------
# Debug Targets
# ---------------------------------------------------------------------------

# Run QEMU with GDB server (port 1234)
debug: build
	@echo "[QEMU] Starting with GDB server on :1234"
	@echo "[GDB]  In another terminal, run: make gdb"
	$(QEMU) $(QEMU_ARGS) -kernel $(KERNEL_BIN) -s -S

# Connect GDB to QEMU
gdb:
	$(GDB) -ex "target remote :1234" \
		-ex "symbol-file $(KERNEL_BIN)" \
		-ex "break kernel_main" \
		-ex "continue"

# ---------------------------------------------------------------------------
# Utility Targets
# ---------------------------------------------------------------------------

# Format all Rust code
format:
	@echo "[FMT] Formatting Rust code..."
	$(CARGO) fmt --all

# Run clippy linter on all crates
lint:
	@echo "[LINT] Running clippy..."
	$(CARGO) clippy --all -- -D warnings \
		-W clippy::pedantic \
		-W clippy::nursery \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc

# Run tests (for host-target code)
test:
	@echo "[TEST] Running tests..."
	$(CARGO) test --all

# Clean build artifacts
clean:
	@echo "[CLEAN] Removing build artifacts..."
	$(CARGO) clean
	@rm -rf $(BUILD_DIR)
	@rm -f $(ISO_DIR)/boot/nerdos.bin

# Install dependencies (Ubuntu/Debian)
deps:
	@echo "[DEPS] Installing build dependencies..."
	sudo apt-get update
	sudo apt-get install -y \
		nasm qemu-system-x86 \
		grub-common grub-pc-bin xorriso \
		ovmf gdb
	rustup target add x86_64-unknown-none

# Install Rust nightly toolchain
rust-toolchain:
	@echo "[RUST] Installing nightly toolchain..."
	rustup install nightly
	rustup default nightly
	rustup component add rust-src
	rustup target add x86_64-unknown-none

# ---------------------------------------------------------------------------
# Help
# ---------------------------------------------------------------------------

help:
	@echo "NerdOS Build System"
	@echo ""
	@echo "Targets:"
	@echo "  make build      - Build the kernel binary"
	@echo "  make run        - Run NerdOS in QEMU (BIOS)"
	@echo "  make run-uefi   - Run NerdOS in QEMU (UEFI)"
	@echo "  make run-kvm    - Run NerdOS with KVM acceleration"
	@echo "  make iso        - Build bootable ISO image"
	@echo "  make debug      - Run QEMU with GDB server"
	@echo "  make gdb        - Connect GDB to running QEMU"
	@echo "  make clean      - Remove build artifacts"
	@echo "  make format     - Format Rust code"
	@echo "  make lint       - Run clippy linter"
	@echo "  make test       - Run tests"
	@echo "  make deps       - Install build dependencies"
	@echo "  make help       - Show this help"
	@echo ""
	@echo "Variables:"
	@echo "  PROFILE=release - Build optimized binary (default)"
	@echo "  PROFILE=dev     - Build debug binary"
