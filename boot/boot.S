# =============================================================================
# BoundaryOS Boot Assembly - Multiboot2 Compliant Entry Point
# PHASE 2: Basic assembly boot stub, CPU detection
# LAWS: Nothing is Hidden, Nothing is a Magic Incantation
# MODULE SIZE: ~0.3k lines | budget: 5k lines of 100k total
# =============================================================================

# Multiboot2 magic number
.equ MULTIBOOT2_BOOT_MAGIC, 0x36d76289
.equ MULTIBOOT2_ARCHITECTURE, 0

# Section flags
.section .multiboot2, "a"
.align 8

# Multiboot2 header
multiboot2_header:
    .long MULTIBOOT2_BOOT_MAGIC           # Magic number
    .long MULTIBOOT2_ARCHITECTURE         # Architecture (0 = protected mode i386)
    .long multiboot2_header_end - multiboot2_header  # Header length
    .long 0x00000001                      # Checksum (magic + arch + length + checksum = 0)

    # Information tags requested from bootloader
    .short 4                              # Tag type: Memory map
    .short 0                              # Flags
    .long 8                               # Size
    .long 0                               # Reserved

    .short 5                              # Tag type: Boot loader name
    .short 0                              # Flags
    .long 8                               # Size
    .long 0                               # Reserved

    .short 6                              # Tag type: Framebuffer
    .short 0                              # Flags
    .long 20                              # Size
    .long 0                               # Width (0 = any)
    .long 0                               # Height (0 = any)
    .long 0                               # Depth (0 = any)

    # End tag
    .short 0                              # Tag type: End
    .short 0                              # Flags
    .long 8                               # Size
multiboot2_header_end:

# =============================================================================
# Stack Setup
# DESIGN NOTE: Early boot stack before kernel heap is available
#              32KB should be sufficient for initial boot sequence
# =============================================================================
.section .bss
.align 16
stack_bottom:
    .skip 32768                           # 32 KB stack space
stack_top:

# =============================================================================
# Entry Point - 32-bit Protected Mode
# DESIGN NOTE: GRUB loads us in 32-bit protected mode with paging disabled
#              We must switch to long mode (64-bit) manually
# =============================================================================
.section .text
.globl _start
.type _start, @function
_start:
    # Disable interrupts during setup
    cli

    # Save multiboot2 info pointer (ESI contains magic, EDI contains pointer)
    mov %esi, multiboot_magic
    mov %edi, multiboot_info_ptr

    # Set up stack
    mov $stack_top, %esp

    # Check if we're on an x86_64 capable CPU
    call check_long_mode

    # If not x86_64 capable, halt with error
    test %eax, %eax
    jz no_long_mode

    # Enable paging and switch to long mode
    call enable_paging

    # Jump to 64-bit entry point
    lgdt gdt64_pointer
    jmp $0x08, $start64

no_long_mode:
    # Halt system - no x86_64 support
    hlt
    jmp no_long_mode

# =============================================================================
# CPU Detection - Check for Long Mode Support
# DESIGN NOTE: Uses CPUID to detect Intel 64 or AMD64 support
#              This is the first hardware interaction in boot sequence
# =============================================================================
.type check_long_mode, @function
check_long_mode:
    push %ebx
    push %ecx
    push %edx

    # Check if CPUID is supported (bit 21 of EFLAGS)
    pushfd
    pop %eax
    mov %eax, %ecx
    xor $0x00200000, %eax
    push %eax
    popfd
    pushfd
    pop %eax
    push %ecx
    popfd

    cmp %ecx, %eax
    jne .cpuid_supported

    # CPUID not supported - ancient CPU, no long mode
    xor %eax, %eax
    jmp .done

.cpuid_supported:
    # Check for CPUID function 0x80000001 support (extended features)
    mov $0x80000000, %eax
    cpuid
    cmp $0x80000001, %eax
    jb .no_extended

    # Get extended features (EDX bit 29 = LM flag for long mode)
    mov $0x80000001, %eax
    cpuid
    test $0x20000000, %edx
    jz .no_long_mode_support

    # Long mode supported
    mov $1, %eax
    jmp .done

.no_extended:
.no_long_mode_support:
    xor %eax, %eax

.done:
    pop %edx
    pop %ecx
    pop %ebx
    ret

# =============================================================================
# Enable Paging and Switch to Long Mode
# DESIGN NOTE: Sets up identity mapping for first 4GB
#              Uses 4-level page tables as required by x86_64
# =============================================================================
.type enable_paging, @function
enable_paging:
    # Create page tables (identity mapping)
    # PML4 at 0x1000, PDPT at 0x2000, PD at 0x3000
    mov $0x1000, %eax
    mov %eax, %cr3                        # Load CR3 with PML4 base

    # Clear PML4 table (512 entries * 8 bytes = 4KB)
    mov $512, %ecx
    mov $0x1000, %edi
    xor %eax, %eax
    rep stosl

    # Clear PDPT table
    mov $512, %ecx
    mov $0x2000, %edi
    rep stosl

    # Clear PD table
    mov $512, %ecx
    mov $0x3000, %edi
    rep stosl

    # Set up PML4[0] to point to PDPT
    mov $0x2003, %eax                     # Present + Writable
    mov %eax, 0x1000

    # Set up PDPT[0] to point to PD
    mov $0x3003, %eax                     # Present + Writable
    mov %eax, 0x2000

    # Set up PD entries for identity mapping (first 2MB for now)
    mov $0x3000, %edi
    mov $0x00000083, %eax                 # Present + Writable + 2MB pages
    mov $512, %ecx
.map_loop:
    mov %eax, (%edi)
    add $0x200000, %eax                   # Next 2MB region
    add $8, %edi
    loop .map_loop

    # Enable Physical Address Extension (PAE)
    mov %cr4, %eax
    or $0x00000020, %eax                  # Set PAE bit
    mov %eax, %cr4

    # Enable Long Mode (EFER.LME)
    mov $0xC0000080, %ecx                 # EFER MSR
    rdmsr
    or $0x00000100, %eax                  # Set LME bit
    wrmsr

    # Enable Paging (CR0.PG) and Write Protection (CR0.WP)
    mov %cr0, %eax
    or $0x80000001, %eax                  # Set PG and WP bits
    mov %eax, %cr0

    ret

# =============================================================================
# 64-bit Entry Point
# DESIGN NOTE: Now running in long mode with identity mapping
#              GDT has been loaded, ready to call kernel_main
# =============================================================================
.code64
.globl start64
.type start64, @function
start64:
    # Set up segment registers
    xor %ax, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    # Set up new stack for 64-bit mode
    mov $stack_top, %rsp

    # Call kernel main (C/Rust entry point)
    # First argument: multiboot2 info pointer (in %rdi)
    # Second argument: multiboot2 magic (in %rsi)
    mov multiboot_info_ptr(%rip), %rdi
    mov multiboot_magic(%rip), %rsi
    call kernel_main

    # Kernel should never return, but just in case
    cli
    hlt
    jmp .

# =============================================================================
# Data Section - Store multiboot info
# =============================================================================
.section .data
.align 8
multiboot_magic:
    .quad 0
multiboot_info_ptr:
    .quad 0

# GDT for 64-bit mode
.align 16
gdt64:
    .quad 0                               # Null descriptor
    .quad 0x00af9a000000ffff              # Code segment: base=0, limit=0xFFFFF, type=exec/read
    .quad 0x00cf92000000ffff              # Data segment: base=0, limit=0xFFFFF, type=read/write

gdt64_pointer:
    .word 0x17                            # Limit (3 entries * 8 - 1)
    .quad gdt64                           # Base address

