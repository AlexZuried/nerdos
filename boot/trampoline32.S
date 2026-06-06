# =============================================================================
# BoundaryOS Trampoline - 32-bit to 64-bit Mode Switch
# PHASE 2: Basic assembly boot stub, CPU detection (continued)
# LAWS: Nothing is Hidden, Nothing is a Magic Incantation
# MODULE SIZE: ~0.15k lines | budget: 5k lines of 100k total
# DESIGN NOTE: Separate trampoline for AP startup and BSP fallback
#              This code runs during SMP initialization or if main boot fails
# =============================================================================

.code32
.section .trampoline, "ax"
.globl trampoline_start
.type trampoline_start, @function

trampoline_start:
    # Disable interrupts
    cli

    # Load temporary GDT for transition
    lgdt trampoline_gdt_desc

    # Enable protected mode (set CR0.PE)
    mov %cr0, %eax
    or $1, %eax
    mov %eax, %cr0

    # Far jump to flush pipeline and load CS
    ljmp $0x08, $trampoline_pm

trampoline_pm:
    # Set up segment registers
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    # Enable PAE
    mov %cr4, %eax
    or $0x20, %eax
    mov %eax, %cr4

    # Load page table base (CR3) - set by caller
    # eax should contain physical address of PML4
    mov %edi, %cr3

    # Enable long mode (EFER.LME)
    mov $0xC0000080, %ecx
    rdmsr
    or $0x100, %eax
    wrmsr

    # Enable paging (CR0.PG)
    mov %cr0, %eax
    or $0x80000000, %eax
    mov %eax, %cr0

    # Far jump to 64-bit mode
    ljmp $0x08, $trampoline_64

.code64
trampoline_64:
    # Set up segment registers for 64-bit
    xor %ax, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    # Set up stack (passed in esi)
    mov %esi, %rsp

    # Jump to C/Rust entry point
    call trampoline_entry_64

    # Should never return
    cli
    hlt
    jmp .

# Temporary GDT for transition
.align 8
trampoline_gdt:
    .quad 0                               # Null descriptor
    .quad 0x00af9a000000ffff              # Code segment
    .quad 0x00cf92000000ffff              # Data segment

trampoline_gdt_desc:
    .word 0x17                            # Limit (3 entries - 1)
    .long trampoline_gdt                  # Base address

.section .data
.globl trampoline_ready
trampoline_ready:
    .long 0                               # Flag indicating trampoline is ready

