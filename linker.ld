/* BoundaryOS Linker Script */
/* DESIGN NOTE: This script lays out the kernel ELF for Multiboot2 compliance */

ENTRY(boot_start)

SECTIONS
{
    /* Load at 1MB - standard Multiboot location */
    . = 1M;

    /* Boot section must be first for GRUB to find Multiboot header */
    .boot :
    {
        KEEP(*(.multiboot))
        *(.boot)
        *(.boot.*)
    }

    /* Text section - executable code */
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
    }

    /* Initialized data */
    .data : ALIGN(4K)
    {
        *(.data)
        *(.data.*)
    }

    /* Uninitialized data (zeroed at runtime) */
    .bss : ALIGN(4K)
    {
        _bss_start = .;
        *(COMMON)
        *(.bss)
        *(.bss.*)
        _bss_end = .;
    }

    /* Kernel heap - grows upward */
    .heap : ALIGN(4K)
    {
        _heap_start = .;
        . += 64M;  /* 64MB initial heap */
        _heap_end = .;
    }

    /* Stack - grows downward from higher address */
    .stack : ALIGN(16M)
    {
        _stack_top = .;
        . += 16M;  /* 16MB stack space */
        _stack_bottom = .;
    }

    /* Discard unwanted sections */
    /DISCARD/ :
    {
        *(.comment)
        *(.note.*)
        *(.eh_frame)
        *(.eh_frame_hdr)
    }

    /* Provide symbols for Rust code */
    PROVIDE(_kernel_start = 1M);
    PROVIDE(_kernel_end = .);
}
