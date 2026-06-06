/* =============================================================================
 * BoundaryOS Custom Linker Script
 * PHASE 2: Basic assembly boot stub, CPU detection (continued)
 * LAWS: Nothing is Hidden, Nothing is a Magic Incantation
 * DESIGN NOTE: Multiboot2 compliant ELF layout for bare-metal x86_64
 *              Identity mapping assumed for kernel load address
 * =========================================================================== */

/* Entry point - defined in boot.S */
ENTRY(_start)

/* Kernel load address - identity mapped */
KERNEL_BASE = 0xFFFFFFFF80000000;
PHYSICAL_BASE = 0x00100000;  /* 1MB - above real mode and BIOS areas */

SECTIONS
{
    /* Start at 1MB physical address */
    . = PHYSICAL_BASE;

    /* Multiboot2 header must be first */
    .multiboot2 : ALIGN(8)
    {
        *(.multiboot2)
    }

    /* Boot code (assembly entry points) */
    .boot : ALIGN(4K)
    {
        *(.text.boot)
        *(.text.startup)
    }

    /* Main kernel code */
    .text : ALIGN(4K)
    {
        *(.text)
        *(.text.*)
    }

    /* Read-only data */
    .rodata : ALIGN(4K)
    {
        *(.rodata)
        *(.rodata.*)
        
        /* Exception handling tables */
        __eh_frame_start = .;
        *(.eh_frame)
        __eh_frame_end = .;
        
        __eh_frame_hdr_start = .;
        *(.eh_frame_hdr)
        __eh_frame_hdr_end = .;
    }

    /* Initialized data */
    .data : ALIGN(4K)
    {
        __data_start = .;
        *(.data)
        *(.data.*)
        __data_end = .;
    }

    /* Uninitialized data (zeroed at runtime) */
    .bss : ALIGN(4K)
    {
        __bss_start = .;
        *(.bss)
        *(.bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* Trampoline code for AP startup - must be below 1MB */
    .trampoline : ALIGN(4K)
    {
        __trampoline_start = .;
        *(.trampoline)
        *(.trampoline.*)
        __trampoline_end = .;
    }

    /* Stack space */
    .stack : ALIGN(16)
    {
        __stack_bottom = .;
        . += 32K;  /* Initial boot stack */
        __stack_top = .;
    }

    /* Early heap for boot allocator */
    .heap : ALIGN(4K)
    {
        __heap_start = .;
        . += 2M;  /* Initial heap size - can be expanded */
        __heap_end = .;
    }

    /* Discard unwanted sections */
    /DISCARD/ :
    {
        *(.comment)
        *(.note.*)
        *(.gnu.hash)
        *(.dynsym)
        *(.dynstr)
        *(.dynamic)
        *(.got)
        *(.got.plt)
        *(.igot.plt)
    }

    /* Symbols for kernel introspection */
    .symbols : ALIGN(8)
    {
        __kernel_start = PHYSICAL_BASE;
        __kernel_end = .;
        __kernel_size = __kernel_end - __kernel_start;
    }
}

/* Provide memory region definitions for the kernel */
REGION_ALIAS("BOOT", 0x00100000)
REGION_ALIAS("TEXT", 0x00100000)
REGION_ALIAS("DATA", 0x00100000)
REGION_ALIAS("BSS", 0x00100000)

/* Maximum kernel size check - fail if exceeded */
ASSERT(. <= 0x00200000, "Error: Kernel image exceeds 2MB limit for initial load")

