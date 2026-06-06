# =============================================================================
# BoundaryOS Trampoline - Application Processor Startup
# =============================================================================
# DESIGN NOTE: This code is used for starting additional CPUs (APs) in SMP systems.
# The BSP (Bootstrap Processor) runs boot.S, while APs start here.
#
# LAWS: Nothing is Hidden, Nothing is a Magic Incantation
# =============================================================================

.section .boot, "ax"
.code32

.globl trampoline_start
.type trampoline_start, @function

trampoline_start:
    # Disable interrupts
    cli
    
    # Load temporary GDT (same as main GDT for simplicity)
    lgdt trampoline_gdt_desc
    
    # Enable PAE
    movl %cr4, %eax
    orl $(1 << 5), %eax
    movl %eax, %cr4
    
    # Load page tables (physical address passed by BSP)
    movl trampoline_cr3_value, %eax
    movl %eax, %cr3
    
    # Enable Long Mode via EFER
    movl $0xC0000080, %ecx
    rdmsr
    orl $(1 << 8), %eax
    wrmsr
    
    # Enable Paging
    movl %cr0, %eax
    orl $(1 << 31), %eax
    movl %eax, %cr0
    
    # Far jump to 64-bit mode
    leal trampoline_64bit, %eax
    pushl $0x08
    pushl %eax
    lretq

# 64-bit entry for AP
.code64
trampoline_64bit:
    # Clear segment registers
    xorl %eax, %eax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs
    
    # Load AP stack (set by BSP)
    movq trampoline_ap_stack, %rsp
    
    # Jump to AP initialization routine (set by BSP)
    call *trampoline_ap_entry
    
    # Halt if return (should not happen)
    cli
1:  hlt
    jmp 1b

# =============================================================================
# Trampoline GDT (same as main GDT)
# =============================================================================
.align 16
trampoline_gdt:
    .quad 0x0000000000000000
    .quad 0x00AF9A000000FFFF
    .quad 0x00CF92000000FFFF

trampoline_gdt_desc:
    .word 23
    .quad trampoline_gdt

# =============================================================================
# Variables set by BSP before releasing AP
# =============================================================================
.section .data
.align 8

trampoline_cr3_value:
    .long 0
trampoline_ap_stack:
    .quad 0
trampoline_ap_entry:
    .quad 0

# MODULE SIZE: ~0.1k lines
