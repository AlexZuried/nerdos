/* BoundaryOS Linker Script
 * Defines memory layout for the kernel
 */

ENTRY(boot)

SECTIONS
{
    . = 1M;

    .boot :
    {
        *(.multiboot)
        *(.text.boot)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata)
        *(.rodata.*)
    }

    .text : ALIGN(4K)
    {
        *(.text)
        *(.text.*)
    }

    .data : ALIGN(4K)
    {
        *(.data)
        *(.data.*)
    }

    .bss : ALIGN(4K)
    {
        __bss_start = .;
        *(COMMON)
        *(.bss)
        *(.bss.*)
        __bss_end = .;
    }

    __kernel_end = .;
    
    /* Stack grows down from here */
    __stack_top = 8M;
}
