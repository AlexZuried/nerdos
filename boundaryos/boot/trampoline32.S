# Trampoline32: 32-bit to 64-bit mode switch
# This code transitions from protected mode to long mode

.section .text.trampoline
.code32
.global trampoline32
.type trampoline32, @function

trampoline32:
    # Disable interrupts
    cli
    
    # Load GDT for 64-bit mode
    lgdt gdt64_ptr
    
    # Enable PAE (Physical Address Extension)
    mov %cr0, %eax
    or $(1 << 5), %eax
    mov %eax, %cr0
    
    # Load CR3 with page map level 4 table address
    mov $pml4_table, %eax
    mov %eax, %cr3
    
    # Enable Long Mode (bit 8 in EFER)
    mov $0xC0000080, %ecx
    rdmsr
    or $(1 << 8), %eax
    wrmsr
    
    # Enable paging (set PG bit in CR0)
    mov %cr0, %eax
    or $(1 << 31), %eax
    mov %eax, %cr0
    
    # Far jump to 64-bit code segment
    lgdt gdt64_ptr
    jmp 0x08, $long_mode_entry

.size trampoline32, . - trampoline32

# 64-bit entry point
.code64
long_mode_entry:
    # Set up segment registers
    mov $0, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %ss
    mov %ax, %fs
    mov %ax, %gs
    
    # Set up stack
    mov $stack_top_64, %rsp
    
    # Jump to main kernel entry in 64-bit mode
    extern kernel_main_64
    call kernel_main_64
    
    # Hang if we return
hang64:
    hlt
    jmp hang64

# GDT for 64-bit mode
.align 16
gdt64:
    .quad 0                           # Null descriptor
    .quad 0x00AF9A000000FFFF          # Code segment (selector 0x08)
    .quad 0x00CF92000000FFFF          # Data segment (selector 0x10)

gdt64_ptr:
    .short gdt64_end - gdt64 - 1
    .long gdt64

gdt64_end:

# Page tables (identity mapping for first 4MB)
.align 4096
pml4_table:
    .quad 0x0000000000001003      # Present, writable, points to pdpt

.align 4096
pdpt:
    .quad 0x0000000000002003      # Present, writable, points to pd

.align 4096
pd:
    .quad 0x0000000000003003      # Present, writable, points to pt

.align 4096
pt:
    # Identity map first 4MB (1024 entries * 4KB = 4MB)
    .fill 1024, 8, 0

# 64-bit stack
.section .bss
.align 16
stack_bottom_64:
    .skip 32768                   # 32 KB stack for 64-bit mode
stack_top_64:
