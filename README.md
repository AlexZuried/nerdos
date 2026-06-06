# NerdOS

A from-scratch operating system written in Rust for x86_64, designed for hackers, developers, and anyone who believes their OS should be as hackable as their dotfiles.

> "No GUI by default. Everything in Rust. All config is TOML."

## Features

| Component | Status | Description |
|-----------|--------|-------------|
| **Boot** | Working | GRUB2 Multiboot2 with BIOS/UEFI support |
| **GDT** | Working | Flat segmentation with TSS for double fault handling |
| **IDT** | Working | All 32 CPU exceptions + hardware interrupts |
| **Memory** | Working | Physical bitmap allocator, 4-level paging, heap |
| **Scheduler** | Basic | Round-robin with priority levels, context switching |
| **Syscalls** | Working | Linux-compatible via `syscall`/`sysret` + `int 0x80` |
| **VFS** | Framework | Unified layer with ext4, FAT32, and procfs drivers |
| **Keyboard** | Working | PS/2 scancode set 1, modifier keys, LFN support |
| **VGA** | Working | 80x25 text mode with colors |
| **Serial** | Working | COM1 UART 16550 at 115200 baud for debugging |
| **AHCI** | Framework | SATA controller initialization, DMA read/write |
| **e1000** | Framework | Intel 8254x gigabit ethernet, RX/TX rings |
| **PCI** | Working | Full bus enumeration, BAR decoding |
| **Network** | Framework | Ethernet, ARP, IPv4, ICMP, UDP, TCP, DHCP |
| **Shell** | Working | Interactive shell with 30+ builtin commands |
| **Packages** | Framework | nerdpkg with .deb, .pkg.tar.zst, and git support |

## Philosophy

- **No GUI by default**: Boot straight to a hackable TTY shell. Your OS is a development environment, not a tablet.
- **Everything in Rust**: Kernel is `#![no_std]`. Userland tools use `std`. Script in Rust or shell.
- **All config is TOML**: Human-readable, version-controllable, typed configuration.
- **Compatible with Linux packages**: Install from Debian repos, Arch repos, or build from git.
- **Minimal and auditable**: Dependencies are counted, not imported. Every `unsafe` block is documented.

## Quick Start

### Prerequisites

On Ubuntu/Debian:

```bash
sudo apt-get update
sudo apt-get install -y nasm qemu-system-x86 grub-common grub-pc-bin xorriso ovmf gdb

# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install nightly
rustup default nightly
rustup component add rust-src
rustup target add x86_64-unknown-none
```

On Arch Linux:

```bash
sudo pacman -S nasm qemu-full grub xorriso edk2-ovmf gdb

# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install nightly
rustup default nightly
rustup component add rust-src
rustup target add x86_64-unknown-none
```

### Build

```bash
# Clone the repository
git clone https://github.com/nerdos/nerdos.git
cd nerdos

# Build the kernel binary
make build

# Build a bootable ISO image
make iso
```

### Run in QEMU

```bash
# BIOS mode (fastest for development)
make run

# UEFI mode (requires OVMF firmware)
make run-uefi

# With KVM acceleration (Linux host with hardware virtualization)
make run-kvm

# With GDB debugging
make debug        # Terminal 1: starts QEMU with GDB server on :1234
make gdb          # Terminal 2: connects GDB
```

### Debugging

The kernel serial output is available on COM1. In QEMU, this is redirected to stdio:

```bash
make run
```

You'll see serial output like:

```
[NERDOS] Serial output initialized
[NERDOS] NerdOS v0.1.0
[OK] GDT initialized
[OK] IDT initialized
...
```

## Project Structure

```
nerdos/
├── Cargo.toml                 # Workspace definition
├── Makefile                   # Build system
├── README.md                  # This file
│
├── boot/grub/grub.cfg         # GRUB2 bootloader configuration
│
├── bin/                       # Essential user binaries (future)
├── sbin/                      # System binaries (future)
├── etc/
│   ├── fstab                  # Filesystem mount table
│   ├── hostname               # System hostname
│   ├── hosts                  # Static host entries
│   └── nerdpkg/
│       ├── sources.list       # Package repositories
│       └── nerdpkg.toml       # Package manager config
│
├── usr/
│   ├── bin/                   # User programs
│   ├── lib/                   # Shared libraries
│   └── src/                   # Source code
│
├── var/
│   ├── log/                   # System logs
│   ├── cache/                 # Cached data
│   └── packages/              # Downloaded packages
│
├── home/                      # User home directories
├── tmp/                       # Temporary files
├── opt/                       # Optional software
│
└── src/                       # Kernel source code
    ├── bootloader/
    │   ├── src/boot.asm       # Multiboot2 assembly stub
    │   ├── linker.ld          # Kernel linker script
    │   └── Cargo.toml
    │
    ├── kernel_core/           # Core kernel (no_std)
    │   ├── src/
    │   │   ├── lib.rs         # Kernel main, panic handler
    │   │   ├── gdt.rs         # Global Descriptor Table
    │   │   ├── idt.rs         # Interrupt Descriptor Table
    │   │   ├── interrupts.rs  # PIC interrupt controller
    │   │   ├── memory.rs      # Physical/virtual memory, paging
    │   │   ├── scheduler.rs   # Process scheduler
    │   │   ├── syscall.rs     # System call interface (300+ syscalls)
    │   │   ├── vga.rs         # VGA text mode driver
    │   │   ├── serial.rs      # UART 16550 serial driver
    │   │   ├── tty.rs         # Terminal/keyboard input
    │   │   └── clock.rs       # PIT timer driver
    │   ├── x86_64-nerdos.json # Custom target specification
    │   └── Cargo.toml
    │
    ├── drivers/               # Hardware drivers
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── pci.rs         # PCI bus enumeration
    │   │   ├── ahci.rs        # SATA AHCI controller
    │   │   └── e1000.rs       # Intel ethernet
    │   └── Cargo.toml
    │
    ├── vfs/                   # Virtual File System
    │   ├── src/
    │   │   ├── lib.rs         # VFS core, mount system
    │   │   ├── ext4.rs        # Ext4 filesystem driver
    │   │   ├── fat32.rs       # FAT32 filesystem driver
    │   │   └── procfs.rs      # Proc filesystem
    │   └── Cargo.toml
    │
    ├── net/                   # Network stack
    │   ├── src/
    │   │   ├── lib.rs         # Core types, configuration
    │   │   ├── ethernet.rs    # Ethernet framing
    │   │   ├── arp.rs         # Address Resolution Protocol
    │   │   ├── ipv4.rs        # IPv4 packet handling
    │   │   ├── icmp.rs        # ICMP (ping)
    │   │   ├── udp.rs         # UDP sockets
    │   │   ├── tcp.rs         # TCP sockets
    │   │   ├── dhcp.rs        # DHCP client
    │   │   └── socket.rs      # Socket API
    │   └── Cargo.toml
    │
    ├── pkg/                   # Package manager
    │   ├── src/
    │   │   └── lib.rs         # nerdpkg implementation
    │   └── Cargo.toml
    │
    └── nerdshell/             # Interactive shell
        ├── src/
        │   └── lib.rs         # Shell with 30+ builtins
        └── Cargo.toml
```

## Architecture

### Kernel Design

NerdOS uses a monolithic kernel architecture written in Rust:

```
User Space:     [Shell] [Binaries] [Libraries]
                     |        |
System Calls:   ---- syscall/sysret ----
                     |
Kernel Space:   [Scheduler] [VFS] [Network] [Drivers]
                     |         |       |         |
                 [Memory]   [ext4] [TCP/IP]  [PCI]
                     |
              [GDT] [IDT] [Paging]
                     |
Hardware:    [CPU] [RAM] [Devices]
```

### Memory Layout

```
0xFFFF_8000_0000_0000  Kernel virtual address space start
        |
        |   Kernel code, data, BSS
        |
0xFFFF_8000_0010_0000  Kernel heap (1 MiB)
        |
        |   Heap (Bump allocator initially)
        |
0xFFFF_8000_0020_0000  Kernel stack
        |
        |   ... unused kernel space ...
        |
0xFFFF_FFFE_8000_0000  Recursive PML4 mapping
        |
0x0000_0000_0000_0000  User virtual address space start
        |
        |   User code, data, heap, stack
        |
0x0000_7FFF_FFFF_F000  User stack top
```

### Boot Process

```
1. GRUB2 loads kernel from disk
2. boot.asm sets up stack, saves Multiboot2 info
3. Jump to kernel_main() in Rust
4. Initialize serial output (for early debugging)
5. Initialize VGA text mode
6. Set up GDT (flat segmentation + TSS)
7. Set up IDT (all exception handlers)
8. Initialize Physical Memory Manager from Multiboot2 memory map
9. Set up paging (4-level page tables)
10. Initialize heap allocator
11. Remap PIC (IRQs 32-47)
12. Initialize PIT (1000 Hz timer)
13. Initialize syscall interface (MSR setup)
14. Enable interrupts
15. Initialize scheduler
16. Start shell
```

## Shell Commands

| Command | Description |
|---------|-------------|
| `help` | Show all available commands |
| `ls [path]` | List directory contents |
| `cd [path]` | Change directory |
| `pwd` | Print working directory |
| `cat [file]` | Display file contents |
| `mkdir [dir]` | Create directory |
| `rm [file]` | Remove file |
| `rmdir [dir]` | Remove directory |
| `touch [file]` | Create empty file |
| `ps` | List processes |
| `kill [pid]` | Send signal to process |
| `mount` | Mount filesystem |
| `umount [path]` | Unmount filesystem |
| `dmesg` | Kernel message buffer |
| `free` | Memory usage |
| `uptime` | System uptime |
| `uname [-a]` | System information |
| `ifconfig` | Network interface info |
| `ping [host]` | Network connectivity test |
| `nerdpkg ...` | Package management |
| `env` | Environment variables |
| `export K=V` | Set environment variable |
| `history` | Command history |
| `clear` | Clear screen |
| `echo [text]` | Print text |
| `reboot` | Restart system |
| `halt` | Power off system |

## Package Management

NerdOS uses **nerdpkg** for package management:

```bash
# Install from Debian .deb package
nerdpkg install package.deb

# Install from Arch package
nerdpkg install package.pkg.tar.zst

# Install from git repository
nerdpkg install https://github.com/user/repo

# Search repositories
nerdpkg search vim

# List installed packages
nerdpkg list

# Remove a package
nerdpkg remove package-name

# Update package lists
nerdpkg update

# Upgrade all packages
nerdpkg upgrade
```

Package sources are configured in `/etc/nerdpkg/sources.list` (TOML format).

## Development

### Code Style

- All `unsafe` blocks must have a `// Safety:` comment explaining why they're safe.
- Every module starts with a documentation comment explaining its purpose.
- Use `///` for public API docs, `//!` for module-level docs.
- Keep dependencies minimal. Prefer implementing over importing when the cost is low.

### Running Tests

```bash
# Run host-target tests (for pure Rust code)
make test

# Format code
make format

# Run linter
make lint
```

### Adding a New Driver

1. Add a new module in `src/drivers/src/`.
2. Implement the driver interface (see `src/drivers/src/pci.rs` for examples).
3. Register initialization in `drivers::init()`.
4. Add documentation comments explaining the hardware interface.

### Adding a Filesystem

1. Create a new module in `src/vfs/src/` implementing the `FileSystem` trait.
2. Add filesystem detection in `Vfs::mount()`.
3. Implement `InodeOps` for directory and file operations.

## Security

- **No GUI**: Reduces attack surface. No X11/Wayland vulnerabilities.
- **Memory safety**: Rust's ownership model prevents buffer overflows, use-after-free, and most memory safety bugs.
- **Minimal attack surface**: Small codebase, minimal dependencies.
- **W^X by default**: Pages are either writable or executable, not both (where the CPU supports it).
- **ASLR-ready**: PIE support planned for userland binaries.

## Roadmap

### Phase 1: Core (Current)
- [x] Boot via GRUB2 Multiboot2
- [x] GDT/IDT with exception handling
- [x] Physical and virtual memory management
- [x] Timer-based preemptive scheduler
- [x] Syscall interface (Linux-compatible)
- [x] VGA text mode, serial, keyboard
- [x] Interactive shell with builtins

### Phase 2: Storage & I/O
- [ ] AHCI SATA block device driver
- [ ] ext4 read/write support
- [ ] FAT32 read/write support
- [ ] initrd/ramdisk support
- [ ] Disk partition table parsing (GPT/MBR)

### Phase 3: Networking
- [ ] e1000 packet transmission
- [ ] Full TCP stack with proper state machine
- [ ] DHCP client auto-configuration
- [ ] DNS resolver
- [ ] HTTP client (for package downloads)

### Phase 4: Userspace
- [ ] ELF executable loader
- [ ] Dynamic linker
- [ ] Rust standard library port
- [ ] POSIX shell (dash/ash)
- [ ] Coreutils (busybox-style)
- [ ] Package manager (nerdpkg) full implementation

### Phase 5: Self-Hosting
- [ ] Port Rust compiler (rustc) to NerdOS
- [ ] Port Cargo
- [ ] Git client
- [ ] Build NerdOS on NerdOS

## License

NerdOS is licensed under the MIT License. See `LICENSE` for details.

## Contributing

Contributions are welcome! Areas where help is especially appreciated:

- Filesystem drivers (ext4 write support, BTRFS, ZFS)
- Network stack improvements
- Real hardware testing (we primarily test in QEMU)
- Documentation

## Acknowledgments

NerdOS was inspired by:
- [Phil Opp's Writing an OS in Rust](https://os.phil-opp.com/)
- [Linux Kernel source](https://github.com/torvalds/linux)
- [xv6](https://github.com/mit-pdos/xv6-public) (MIT's teaching OS)
- The Rust embedded community

---

**Built with Rust. For hackers, by hackers.**
