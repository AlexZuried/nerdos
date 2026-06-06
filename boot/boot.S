# =============================================================================
# BoundaryOS Boot Assembly - Multiboot2 Entry Point
# =============================================================================
# DESIGN NOTE: This is the first code executed by GRUB. It must:
#   1. Provide a valid Multiboot2 header
#   2. Detect x86_64 capability via CPUID
#   3. Set up minimal paging for long mode
#   4. Enable long mode and jump to 64-bit code
#   5. Load GDT and set up stack before calling kernel_main
#
# LAWS: Nothing is Hidden (all magic numbers documented)
#       Nothing is a Magic Incantation (every step commented)
# =============================================================================

.section .multiboot, "a"
.align 8

# -----------------------------------------------------------------------------
# Multiboot2 Header Constants
# Source: https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html
# -----------------------------------------------------------------------------
.set MULTIBOOT2_MAGIC,          0xE85250D6
.set MULTIBOOT2_ARCH,           0          # Protected mode (we switch to 64-bit)
.set MULTIBOOT2_HEADER_LENGTH,  boot_header_end - boot_start

# Tag types
.set MULTIBOOT_TAG_TYPE_END,               0
.set MULTIBOOT_TAG_TYPE_INFORMATION_REQUEST, 1
.set MULTIBOOT_TAG_TYPE_FRAMEBUFFER,       4
.set MULTIBOOT_TAG_TYPE_MEMORY_MAP,        6

boot_start:
    # Magic number
    .long MULTIBOOT2_MAGIC
    
    # Architecture (0 = protected mode, we switch to 64-bit ourselves)
    .long MULTIBOOT2_ARCH
    
    # Header length
    .long MULTIBOOT2_HEADER_LENGTH
    
    # Checksum (makes total sum zero)
    .long -(MULTIBOOT2_MAGIC + MULTIBOOT2_ARCH + MULTIBOOT2_HEADER_LENGTH)

# -----------------------------------------------------------------------------
# Required Multiboot2 Tags
# -----------------------------------------------------------------------------

# Memory Map Tag - Request memory map from bootloader
.align 8
memory_map_tag:
    .short MULTIBOOT_TAG_TYPE_MEMORY_MAP    # type
    .short 0                                # flags
    .long memory_map_tag_end - memory_map_tag  # size
memory_map_tag_end:

# Framebuffer Tag - Request framebuffer (optional, we also support text mode)
.align 8
framebuffer_tag:
    .short MULTIBOOT_TAG_TYPE_FRAMEBUFFER   # type
    .short 0                                # flags (any depth/width/height OK)
    .long framebuffer_tag_end - framebuffer_tag  # size
    .long 0                                 # width (0 = any)
    .long 0                                 # height (0 = any)
    .long 0                                 # depth (0 = any)
framebuffer_tag_end:

# End Tag
.align 8
end_tag:
    .short MULTIBOOT_TAG_TYPE_END           # type
    .short 0                                # flags
    .long 8                                 # size
boot_header_end:

# =============================================================================
# Entry Point - Called by GRUB
# =============================================================================
.section .boot, "ax"
.code32
.globl boot_start
.type boot_start, @function

boot_start:
    # Disable interrupts during setup
    cli
    
    # Save Multiboot2 info pointer (EBX) and magic (EAX) for later
    movl %ebx, multiboot_info_ptr
    movl %eax, multiboot_magic
    
    # -------------------------------------------------------------------------
    # Step 1: Check for x86_64 support via CPUID
    # Source: Intel SDM Vol 2A, Section 3.2
    # -------------------------------------------------------------------------
    movl $0x80000000, %eax
    cpuid
    cmpl $0x80000001, %eax
    jb no_long_mode_support
    
    movl $0x80000001, %eax
    cpuid
    testl $(1 << 29), %edx      # Test LM (Long Mode) bit
    jz no_long_mode_support
    
    # -------------------------------------------------------------------------
    # Step 2: Enable Physical Address Extension (PAE)
    # Required for long mode. Sets CR4.PAE = 1
    # Source: Intel SDM Vol 3A, Section 4.1.4
    # -------------------------------------------------------------------------
    movl %cr4, %eax
    orl $(1 << 5), %eax         # Set PAE bit (bit 5)
    movl %eax, %cr4
    
    # -------------------------------------------------------------------------
    # Step 3: Load Page Directory Pointer Table (PDPTE) address into CR3
    # We use identity mapping for the first 4GB
    # -------------------------------------------------------------------------
    leal page_tables, %eax
    movl %eax, %cr3
    
    # -------------------------------------------------------------------------
    # Step 4: Enable Long Mode
    # Set EFER.LME = 1 (Extended Feature Enable Register, Long Mode Enable)
    # Source: AMD APM Vol 2, Section 15.1
    # -------------------------------------------------------------------------
    movl $0xC0000080, %ecx      # MSR_EFER
    rdmsr
    orl $(1 << 8), %eax         # Set LME bit (bit 8)
    wrmsr
    
    # -------------------------------------------------------------------------
    # Step 5: Enable Paging
    # Set CR0.PG = 1. This actually enables long mode since LME=1 and PAE=1
    # Source: Intel SDM Vol 3A, Section 4.1.4
    # -------------------------------------------------------------------------
    movl %cr0, %eax
    orl $(1 << 31), %eax        # Set PG bit (bit 31)
    movl %eax, %cr0
    
    # -------------------------------------------------------------------------
    # Step 6: Load 64-bit GDT (needed before far jump)
    # -------------------------------------------------------------------------
    lgdt gdt64_descriptor
    
    # -------------------------------------------------------------------------
    # Step 7: Far Jump to 64-bit code segment
    # This switches the CPU to 64-bit mode
    # Segment selector 0x08 = CODE_SEG (defined in GDT below)
    # -------------------------------------------------------------------------
    leal long_mode_entry, %eax
    pushl $0x08                 # Code segment selector
    pushl %eax
    lretq                       # Far return = far jump in AT&T syntax

no_long_mode_support:
    # Halt if no long mode support
    cli
1:  hlt
    jmp 1b

# =============================================================================
# Long Mode Entry Point (64-bit)
# =============================================================================
.section .text
.code64
.globl long_mode_entry
.type long_mode_entry, @function

long_mode_entry:
    # Clear segment registers (not automatically cleared by far jump)
    xorl %eax, %eax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs
    
    # -------------------------------------------------------------------------
    # Step 8: Set up stack
    # Stack grows downward from _stack_top (defined in linker script)
    # -------------------------------------------------------------------------
    movq $_stack_top, %rsp
    
    # Clear frame pointer for backtracing
    xorq %rbp, %rbp
    
    # -------------------------------------------------------------------------
    # Step 9: Call kernel main
    # Pass Multiboot2 info pointer in RDI (first argument per System V AMD64 ABI)
    # -------------------------------------------------------------------------
    movq multiboot_info_ptr(%rip), %rdi
    call kernel_main
    
    # If kernel_main returns (it shouldn't), halt
    cli
1:  hlt
    jmp 1b

# =============================================================================
# Global Descriptor Table (64-bit)
# Minimal GDT with code and data segments for long mode
# =============================================================================
.section .rodata
.align 16

gdt64_start:
    # Null descriptor (required)
    .quad 0x0000000000000000
    
    # Code segment (flat, 64-bit)
    # Base=0, Limit=0xFFFFFFFF, Type=execute/read, DPL=0, Present=1, L=1, D=0
    .quad 0x00AF9A000000FFFF
    
    # Data segment (flat, 64-bit)
    # Base=0, Limit=0xFFFFFFFF, Type=read/write, DPL=0, Present=1, L=0, D=1
    .quad 0x00CF92000000FFFF
gdt64_end:

gdt64_descriptor:
    .word gdt64_end - gdt64_start - 1
    .quad gdt64_start

# =============================================================================
# Page Tables (Identity Mapping for First 4GB)
# Uses 4-level paging with 4KB pages
# =============================================================================
.section .bss
.align 4096

page_tables:
    # PML4 Table (Level 4) - Entry 0 points to PDPT
    .quad 0x0000000000002003    # Present, Read/Write, points to PDPT at 0x2000
    
    # Pad to 4KB
    .space 4088
    
    # Page Directory Pointer Table (PDPT) - Entry 0 points to PD
    .quad 0x0000000000003003    # Present, Read/Write, points to PD at 0x3000
    
    # Pad to 4KB
    .space 4088
    
    # Page Directory (PD) - Entries 0-511 point to PTs for 0-2GB
    # Each entry maps 1GB (using 2MB pages would be more efficient, but 4KB for clarity)
    .fill 512, 8, 0x0000000000004003  # All point to PTs starting at 0x4000
    
    # Pad to 4KB
    .space 4096
    
    # Page Tables (PT) - 512 entries each mapping 4KB
    # We only need the first few for initial boot
    .fill 4096, 8, 0  # Filled at runtime with actual mappings

# =============================================================================
# Storage for Multiboot Info
# =============================================================================
.section .bss
.align 8

multiboot_info_ptr:
    .quad 0
multiboot_magic:
    .long 0

# =============================================================================
# MODULE SIZE: ~0.3k lines | budget: Mk lines of Tk total
# =============================================================================
