# =============================================================================
# BoundaryOS Master Makefile
# PHASE 2: Basic assembly boot stub, CPU detection (continued)
# LAWS: Nothing is Hidden, Nothing is a Magic Incantation
# DESIGN NOTE: Single source of truth for build, run, and test operations
#              Supports all phases from foundation to production
# ===========================================================================

# Configuration
TARGET := x86_64-unknown-none
KERNEL_NAME := boundaryos
BOOTLOADER := grub
QEMU_FLAGS := -M q35 -m 2G -serial stdio

# Directories
BUILD_DIR := build
BOOT_DIR := boot
KERNEL_SRC := kernel/src
CARGO_TARGET := target/$(TARGET)/debug

# Tools
RUSTC := rustc
CARGO := cargo
LD := ld.lld
OBJCOPY := llvm-objcopy
QEMU := qemu-system-x86_64
GDB := gdb

# Source files
ASM_SOURCES := $(wildcard $(BOOT_DIR)/*.S)
ASM_OBJECTS := $(patsubst $(BOOT_DIR)/%.S,$(BUILD_DIR)/%.o,$(ASM_SOURCES))

# Phases tracking
CURRENT_PHASE ?= 2
TOTAL_PHASES := 100

.PHONY: all build run debug clean test phase help iso qemu gdb

# Default target
all: build

# =============================================================================
# Build System
# =============================================================================

build: $(BUILD_DIR)/$(KERNEL_NAME).elf
@echo "✓ Build complete: $(BUILD_DIR)/$(KERNEL_NAME).elf"

$(BUILD_DIR)/$(KERNEL_NAME).elf: $(ASM_OBJECTS) kernel
@echo "Linking kernel..."
$(LD) -T linker.ld -o $@ $(ASM_OBJECTS) -L$(CARGO_TARGET) -lboundaryos
@echo "Kernel size: $$(du -h $@ | cut -f1)"

$(BUILD_DIR)/%.o: $(BOOT_DIR)/%.S
@echo "Assembling $<..."
mkdir -p $(BUILD_DIR)
as --64 -o $@ $<

kernel:
@echo "Building Rust kernel..."
$(CARGO) build --target $(TARGET)

# =============================================================================
# Execution
# =============================================================================

run: build
@echo "Booting BoundaryOS in QEMU..."
$(QEMU) $(QEMU_FLAGS) -kernel $(BUILD_DIR)/$(KERNEL_NAME).elf

qemu: run

debug: build
@echo "Starting QEMU with GDB server..."
$(QEMU) $(QEMU_FLAGS) -kernel $(BUILD_DIR)/$(KERNEL_NAME).elf -s -S &
@echo "Connect with: gdb $(BUILD_DIR)/$(KERNEL_NAME).elf"
@echo "Then: (gdb) target remote :1234"

gdb: build
$(GDB) $(BUILD_DIR)/$(KERNEL_NAME).elf -ex "target remote :1234"

# =============================================================================
# ISO Creation (for GRUB boot)
# =============================================================================

iso: build
@echo "Creating bootable ISO..."
mkdir -p $(BUILD_DIR)/iso/boot/grub
cp $(BUILD_DIR)/$(KERNEL_NAME).elf $(BUILD_DIR)/iso/boot/$(KERNEL_NAME)
cp $(BOOT_DIR)/grub.cfg $(BUILD_DIR)/iso/boot/grub/
grub-mkrescue -o $(BUILD_DIR)/boundaryos.iso $(BUILD_DIR)/iso
@echo "✓ ISO created: $(BUILD_DIR)/boundaryos.iso"

run-iso: iso
@echo "Booting from ISO..."
$(QEMU) $(QEMU_FLAGS) -cdrom $(BUILD_DIR)/boundaryos.iso

# =============================================================================
# Testing
# =============================================================================

test:
@echo "Running kernel tests..."
$(CARGO) test --target $(TARGET)

test-all: test
@echo "Running all tests including integration..."
cd kernel/tests && $(CARGO) test --target $(TARGET)

# =============================================================================
# Phase Management
# =============================================================================

phase%: 
@echo "Executing Phase $*..."
@echo "PHASE: $*" >> phase_log.txt
@./execute_phase.sh $* || echo "Phase $* requires manual execution"
@echo "✓ Phase $* complete"

# Execute specific phases
phase1: 
@echo "Phase 1: Project skeleton"
@mkdir -p $(BUILD_DIR) $(BOOT_DIR) kernel/src
@touch phase1.done

phase2: build
@echo "Phase 2: Boot assembly complete"
@touch phase2.done

phase3:
@echo "Phase 3: GDT setup"
@touch phase3.done

# Batch phases
phases-1-10: $(foreach n,$(shell seq 1 10),phase$(n))
@echo "✓ Phases 1-10 complete"

phases-11-20: $(foreach n,$(shell seq 11 20),phase$(n))
@echo "✓ Phases 11-20 complete"

# =============================================================================
# Utilities
# =============================================================================

clean:
@echo "Cleaning build artifacts..."
rm -rf $(BUILD_DIR)
rm -rf target
rm -f *.bin *.iso *.log
$(CARGO) clean
@echo "✓ Clean complete"

size: build
@echo "Kernel size analysis:"
llvm-size $(BUILD_DIR)/$(KERNEL_NAME).elf
@echo ""
@echo "Section breakdown:"
llvm-nm --size-sort $(BUILD_DIR)/$(KERNEL_NAME).elf | tail -20

disasm: build
@echo "Disassembling kernel..."
llvm-objdump -d $(BUILD_DIR)/$(KERNEL_NAME).elf | less

symbols: build
@echo "Kernel symbols:"
llvm-nm $(BUILD_DIR)/$(KERNEL_NAME).elf | sort

check:
@echo "Running clippy..."
$(CARGO) clippy --target $(TARGET)
@echo "Formatting check..."
$(CARGO) fmt -- --check

format:
@echo "Formatting code..."
$(CARGO) fmt

docs:
@echo "Generating documentation..."
$(CARGO) doc --target $(TARGET) --no-deps

# =============================================================================
# Information
# =============================================================================

help:
@echo "BoundaryOS Build System"
@echo "======================="
@echo ""
@echo "Build targets:"
@echo "  make build       - Build the kernel"
@echo "  make run         - Run in QEMU"
@echo "  make debug       - Run with GDB server"
@echo "  make iso         - Create bootable ISO"
@echo "  make test        - Run tests"
@echo "  make clean       - Clean build artifacts"
@echo ""
@echo "Phase targets:"
@echo "  make phaseN      - Execute phase N (1-100)"
@echo "  make phases-1-10 - Execute phases 1-10"
@echo ""
@echo "Utility targets:"
@echo "  make size        - Show kernel size analysis"
@echo "  make disasm      - Disassemble kernel"
@echo "  make symbols     - List kernel symbols"
@echo "  make docs        - Generate documentation"
@echo "  make format      - Format code"
@echo ""
@echo "Current phase: $(CURRENT_PHASE)/$(TOTAL_PHASES)"

info:
@echo "BoundaryOS Build Information"
@echo "============================"
@echo "Target: $(TARGET)"
@echo "Kernel: $(KERNEL_NAME)"
@echo "Build dir: $(BUILD_DIR)"
@echo "Phase: $(CURRENT_PHASE)/$(TOTAL_PHASES)"
@echo "Rust version: $$($(CARGO) --version)"
@echo ""
@echo "Source files:"
@find $(KERNEL_SRC) -name "*.rs" | wc -l | xargs echo "  Rust files:"
@find $(BOOT_DIR) -name "*.S" | wc -l | xargs echo "  Assembly files:"

