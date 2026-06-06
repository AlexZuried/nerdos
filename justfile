# NerdOS Justfile
# Alternative to Makefile using 'just' (https://github.com/casey/just)
#
# Install: cargo install just
# Usage: just <command>

# Default recipe - build the kernel
default: build

# Build configuration
PROFILE := "release"
TARGET := "x86_64-nerdos"
BUILD_DIR := "build"
BOOT_ASM := "src/bootloader/src/boot.asm"
LINKER_SCRIPT := "src/bootloader/linker.ld"
KERNEL_BIN := BUILD_DIR / "nerdos.bin"
KERNEL_ISO := BUILD_DIR / "nerdos.iso"

# QEMU settings
QEMU := "qemu-system-x86_64"
QEMU_MEMORY := "512M"
QEMU_CPUS := "2"
QEMU_ARGS := "-m " + QEMU_MEMORY + " -smp " + QEMU_CPUS + " -serial stdio -cpu qemu64,+x2apic,+fsgsbase -no-reboot -no-shutdown"

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

# Build everything
build: boot kernel
n	@echo "[LINK] {{KERNEL_BIN}}"
	mkdir -p {{BUILD_DIR}}
	ld.lld -n -T {{LINKER_SCRIPT}} -o {{KERNEL_BIN}} \
		{{BUILD_DIR}}/boot.o {{BUILD_DIR}}/libkernel_core.a \
		--gc-sections
	@echo "[DONE] Kernel: $(ls -lh {{KERNEL_BIN}} | awk '{print $5}')"

# Assemble boot stub
boot:
	@echo "[NASM] {{BOOT_ASM}}"
	mkdir -p {{BUILD_DIR}}
	nasm -f elf64 -o {{BUILD_DIR}}/boot.o {{BOOT_ASM}}

# Build kernel Rust code
kernel:
	@echo "[CARGO] Building kernel_core"
	RUSTFLAGS="-C link-arg=-T{{LINKER_SCRIPT}} -C code-model=kernel -C relocation-model=static" \
		cargo build --release -p kernel_core
	mkdir -p {{BUILD_DIR}}
	cp target/release/libkernel_core.a {{BUILD_DIR}}/

# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

# Run in QEMU (BIOS mode)
run: build
	{{QEMU}} {{QEMU_ARGS}} -kernel {{KERNEL_BIN}}

# Run in QEMU (UEFI mode)
run-uefi: iso
	{{QEMU}} {{QEMU_ARGS}} -cdrom {{KERNEL_ISO}} -bios /usr/share/ovmf/OVMF.fd

# Run with KVM acceleration
run-kvm: build
	{{QEMU}} {{QEMU_ARGS}} -enable-kvm -kernel {{KERNEL_BIN}}

# ---------------------------------------------------------------------------
# ISO
# ---------------------------------------------------------------------------

# Build bootable ISO
iso: build
	@echo "[ISO] Building bootable ISO..."
	mkdir -p {{BUILD_DIR}}/iso/boot/grub
	cp {{KERNEL_BIN}} {{BUILD_DIR}}/iso/boot/nerdos.bin
	cp boot/grub/grub.cfg {{BUILD_DIR}}/iso/boot/grub/
	grub-mkrescue -o {{KERNEL_ISO}} {{BUILD_DIR}}/iso
	@echo "[DONE] ISO: $(ls -lh {{KERNEL_ISO}} | awk '{print $5}')"

# ---------------------------------------------------------------------------
# Debug
# ---------------------------------------------------------------------------

# Run with GDB server on port 1234
debug: build
	@echo "[QEMU] GDB server on :1234"
	@echo "[INFO] Run 'just gdb' in another terminal"
	{{QEMU}} {{QEMU_ARGS}} -kernel {{KERNEL_BIN}} -s -S

# Connect GDB
gdb:
	gdb -ex "target remote :1234" \
	    -ex "symbol-file {{KERNEL_BIN}}" \
	    -ex "break kernel_main" \
	    -ex "continue"

# ---------------------------------------------------------------------------
# Development
# ---------------------------------------------------------------------------

# Format all code
fmt:
	cargo fmt --all

# Run linter
lint:
	cargo clippy --all -- -D warnings

# Run tests
test:
	cargo test --all

# Clean build artifacts
clean:
	cargo clean
	rm -rf {{BUILD_DIR}}

# Install dependencies (Ubuntu/Debian)
deps:
	sudo apt-get update
	sudo apt-get install -y nasm qemu-system-x86 grub-common grub-pc-bin xorriso ovmf gdb

# Show project info
info:
	@echo "NerdOS Build System"
	@echo "Profile: {{PROFILE}}"
	@echo "Target:  {{TARGET}}"
	@echo "Build:   {{BUILD_DIR}}"
	@echo ""
	@echo "Available recipes:"
	@just --list
