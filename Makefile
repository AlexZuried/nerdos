# =============================================================================
# BoundaryOS Makefile
# =============================================================================
# DESIGN NOTE: This Makefile provides build, run, debug, and testing targets.
# It follows the phase-based development approach outlined in phase.md
#
# LAWS: Nothing is Hidden (all targets documented)
#       Nothing is a Magic Incantation (every step explicit)
# =============================================================================

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------
ARCH := x86_64
TARGET := $(ARCH)-unknown-none
KERNEL_NAME := boundaryos

# Paths
BOOT_DIR := boot
KERNEL_DIR := kernel/src
BUILD_DIR := target/$(TARGET)/debug
ISO_DIR := isodir

# Tools
RUSTC := rustc
CARGO := cargo
AS := nasm
LD := rust-lld
GRUB := grub-mkrescue
QEMU := qemu-system-x86_64
GDB := gdb

# QEMU Flags
QEMU_FLAGS := -m 2G \
              -cpu qemu64 \
              -drive format=raw,file=$(ISO_DIR)/boundaryos.iso \
              -boot d \
              -serial stdio \
              -no-reboot \
              -no-shutdown

# Debug QEMU Flags
QEMU_DEBUG_FLAGS := $(QEMU_FLAGS) \
                    -s \
                    -S

# -----------------------------------------------------------------------------
# Phony Targets
# -----------------------------------------------------------------------------
.PHONY: all build run debug iso clean help phases test disasm size

# -----------------------------------------------------------------------------
# Default Target
# -----------------------------------------------------------------------------
all: build

# -----------------------------------------------------------------------------
# Build Kernel
# -----------------------------------------------------------------------------
build:
@echo "╔══════════════════════════════════════════════╗"
@echo "║        Building BoundaryOS Kernel            ║"
@echo "╚══════════════════════════════════════════════╝"
$(CARGO) build --target $(TARGET)
@echo ""
@echo "[+] Build complete: $(BUILD_DIR)/$(KERNEL_NAME)"

# -----------------------------------------------------------------------------
# Run in QEMU
# -----------------------------------------------------------------------------
run: iso
@echo "╔══════════════════════════════════════════════╗"
@echo "║     Launching BoundaryOS in QEMU             ║"
@echo "╚══════════════════════════════════════════════╝"
$(QEMU) $(QEMU_FLAGS)

# -----------------------------------------------------------------------------
# Run with GDB Debugging
# -----------------------------------------------------------------------------
debug: iso
@echo "╔══════════════════════════════════════════════╗"
@echo "║  Starting QEMU with GDB server (port 1234)   ║"
@echo "║  In another terminal: gdb -ex 'target remote :1234'  ║"
@echo "╚══════════════════════════════════════════════╝"
$(QEMU) $(QEMU_DEBUG_FLAGS)

# -----------------------------------------------------------------------------
# Create Bootable ISO
# -----------------------------------------------------------------------------
iso: build
@echo "╔══════════════════════════════════════════════╗"
@echo "║        Creating Bootable ISO                 ║"
@echo "╚══════════════════════════════════════════════╝"
mkdir -p $(ISO_DIR)/boot/grub
cp $(BUILD_DIR)/$(KERNEL_NAME) $(ISO_DIR)/boot/$(KERNEL_NAME)
cp $(BOOT_DIR)/grub.cfg $(ISO_DIR)/boot/grub/
$(GRUB) -o $(ISO_DIR)/boundaryos.iso $(ISO_DIR)
@echo ""
@echo "[+] ISO created: $(ISO_DIR)/boundaryos.iso"

# -----------------------------------------------------------------------------
# Clean Build Artifacts
# -----------------------------------------------------------------------------
clean:
@echo "╔══════════════════════════════════════════════╗"
@echo "║        Cleaning Build Artifacts              ║"
@echo "╚══════════════════════════════════════════════╝"
$(CARGO) clean
rm -rf $(ISO_DIR)
@echo "[+] Clean complete"

# -----------------------------------------------------------------------------
# Disassemble Kernel
# -----------------------------------------------------------------------------
disasm: build
@echo "╔══════════════════════════════════════════════╗"
@echo "║        Disassembling Kernel                  ║"
@echo "╚══════════════════════════════════════════════╝"
objdump -d $(BUILD_DIR)/$(KERNEL_NAME) | less

# -----------------------------------------------------------------------------
# Show Binary Size
# -----------------------------------------------------------------------------
size: build
@echo "╔══════════════════════════════════════════════╗"
@echo "║        Kernel Size Analysis                  ║"
@echo "╚══════════════════════════════════════════════╝"
@echo ""
@echo "Section sizes:"
size $(BUILD_DIR)/$(KERNEL_NAME)
@echo ""
@echo "Total binary size:"
ls -lh $(BUILD_DIR)/$(KERNEL_NAME) | awk '{print $$5}'
@echo ""
@echo "Budget: < 100KB total | Target: 100k lines"

# -----------------------------------------------------------------------------
# Run Tests
# -----------------------------------------------------------------------------
test:
@echo "╔══════════════════════════════════════════════╗"
@echo "║        Running Kernel Tests                  ║"
@echo "╚══════════════════════════════════════════════╝"
$(CARGO) test --target $(TARGET)

# -----------------------------------------------------------------------------
# Phase Execution (for phase.md workflow)
# -----------------------------------------------------------------------------
phase%: 
@echo "╔══════════════════════════════════════════════╗"
@echo "║        Executing Phase $*                     ║"
@echo "╚══════════════════════════════════════════════╝"
@echo "[*] Phase $* execution placeholder"
@echo "[*] Add phase-specific commands here"
@echo ""
@echo "[+] Phase $* complete"

# -----------------------------------------------------------------------------
# Install Toolchain Components
# -----------------------------------------------------------------------------
install-toolchain:
@echo "╔══════════════════════════════════════════════╗"
@echo "║     Installing Required Toolchain            ║"
@echo "╚══════════════════════════════════════════════╝"
rustup component add rust-src llvm-tools-preview
rustup target add $(TARGET)
@echo ""
@echo "[+] Toolchain installation complete"

# -----------------------------------------------------------------------------
# Help Target
# -----------------------------------------------------------------------------
help:
@echo "╔══════════════════════════════════════════════╗"
@echo "║           BoundaryOS Makefile Help           ║"
@echo "╚══════════════════════════════════════════════╝"
@echo ""
@echo "Available targets:"
@echo "  all          - Build the kernel (default)"
@echo "  build        - Compile kernel with Cargo"
@echo "  run          - Build and run in QEMU"
@echo "  debug        - Run in QEMU with GDB server"
@echo "  iso          - Create bootable ISO image"
@echo "  clean        - Remove build artifacts"
@echo "  disasm       - Disassemble kernel binary"
@echo "  size         - Show binary size analysis"
@echo "  test         - Run kernel tests"
@echo "  phaseN       - Execute phase N (e.g., make phase1)"
@echo "  install-toolchain - Install required Rust components"
@echo "  help         - Show this help message"
@echo ""
@echo "Examples:"
@echo "  make build           # Build only"
@echo "  make run             # Build and run in QEMU"
@echo "  make debug           # Start GDB debugging session"
@echo "  make phase5          # Execute phase 5"
@echo "  make clean && make   # Clean rebuild"
@echo ""

# =============================================================================
# MODULE SIZE: ~0.2k lines | budget: Mk lines of Tk total
# =============================================================================
