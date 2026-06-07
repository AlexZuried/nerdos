# Multiboot2 Header for BoundaryOS
# This file contains the entry point and multiboot2 header

.set ALIGN,    1<<0             # Align loaded modules on page boundaries
.set MEMINFO,  1<<1             # Provide memory map
.set FLAGS,    ALIGN | MEMINFO  # Multiboot2 flags
.set MAGIC,    0xe85250d6       # Multiboot2 magic number
.set ARCH,     0                # i386 protected mode (we'll switch to long mode)

.set HEADER_LENGTH, _header_end - _header_start

.section .multiboot
.align 4
_header_start:
    .long MAGIC                 # Magic number
    .long ARCH                  # Architecture (0 = i386)
    .long HEADER_LENGTH         # Header length
    .long -(MAGIC + ARCH + HEADER_LENGTH)  # Checksum

    # Tag: End tag
    .short 0                    # Type (0 = end)
    .short 0                    # Flags
    .long 8                     # Size
_header_end:

# Stack setup
.section .bss
.align 16
.stack_bottom:
    .skip 16384                 # 16 KB stack
.stack_top:

.section .text.boot
.global boot
.type boot, @function
boot:
    # Set up stack
    mov $.stack_top, %esp
    
    # Check for Multiboot2 magic number in %eax
    cmp $MAGIC, %eax
    jne hang
    
    # Save multiboot info pointer (in %ebx)
    mov %ebx, multiboot_info_ptr
    
    # Jump to Rust kernel main
    extern kernel_main
    call kernel_main
    
    # If kernel_main returns (it shouldn't), hang
hang:
    cli
1:  hlt
    jmp 1b

.size boot, . - boot

# Store multiboot info pointer
.section .data
.align 4
multiboot_info_ptr:
    .long 0
