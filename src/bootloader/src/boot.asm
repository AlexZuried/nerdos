; NerdOS Bootloader Stub
; 
; This is the initial assembly stub that GRUB2 loads via Multiboot2.
; Its job is minimal:
;   1. Set up a basic stack
;   2. Save the Multiboot2 information pointer
;   3. Jump to the Rust kernel_main function
;
; We rely on GRUB2 to have already:
;   - Loaded us into memory
;   - Set up initial identity paging (if needed)
;   - Enabled the A20 line
;   - Switched to protected mode (i386) or long mode (x86_64)
;
; For x86_64, we assume GRUB2 has already entered long mode for us.
; This is the standard behavior when using a Multiboot2-compliant
; bootloader with a 64-bit ELF kernel.

[BITS 64]

; ---------------------------------------------------------------------------
; Multiboot2 Header
; ---------------------------------------------------------------------------
; The Multiboot2 header must be within the first 32768 bytes of the file.
; We place it at the very beginning with a small jump over it.

section .multiboot2_header
align 8

; Multiboot2 magic number
MB2_MAGIC   equ 0xE85250D6
MB2_ARCH    equ 0          ; 0 = i386 protected mode (GRUB handles long mode)
MB2_LENGTH  equ header_end - header_start
MB2_CHECKSUM equ -(MB2_MAGIC + MB2_ARCH + MB2_LENGTH)

header_start:
    dd MB2_MAGIC
    dd MB2_ARCH
    dd MB2_LENGTH
    dd MB2_CHECKSUM

    ; Information request tag (ask GRUB for specific info)
    dw 1                    ; Type = information request
    dw 0                    ; Flags
    dd 24                   ; Size
    dd 4                    ; Request basic mem info
    dd 5                    ; Request BIOS boot device
    dd 6                    ; Request memory map
    dd 9                    ; Request ELF sections

    ; Address tag (optional, for relocatable kernels)
    ; dw 2                    ; Type = address
    ; dw 0                    ; Flags
    ; dd 24                   ; Size
    ; dd header_start         ; Header address
    ; dd _start               ; Load address
    ; dd 0                    ; Load end address (0 = use ELF)
    ; dd 0                    ; BSS end address (0 = use ELF)

    ; Entry address tag
    dw 3                    ; Type = entry address
    dw 0                    ; Flags
    dd 12                   ; Size
    dd _start               ; Entry point

    ; Console flags tag
    ; dw 4                    ; Type = console flags
    ; dw 0                    ; Flags
    ; dd 12                   ; Size
    ; dd 0                    ; Console required

    ; Framebuffer tag (request linear framebuffer)
    ; dw 5                    ; Type = framebuffer
    ; dw 0                    ; Flags
    ; dd 20                   ; Size
    ; dd 1024                 ; Preferred width
    ; dd 768                  ; Preferred height
    ; dd 32                   ; Preferred depth

    ; Module alignment tag
    dw 6                    ; Type = module alignment
    dw 0                    ; Flags
    dd 8                    ; Size

    ; EFI boot services tag (request GRUB not to exit boot services)
    ; This lets us handle EFI ourselves if needed.
    ; dw 7                    ; Type = EFI boot services
    ; dw 0                    ; Flags
    ; dd 8                    ; Size

    ; EFI 64-bit entry tag
    ; dw 9                    ; Type = EFI x86_64 entry
    ; dw 0                    ; Flags
    ; dd 12                   ; Size
    ; dd _start               ; Entry point for EFI

    ; Relocatable tag (mark kernel as relocatable)
    ; dw 10                   ; Type = relocatable
    ; dw 0                    ; Flags
    ; dd 24                   ; Size
    ; dd 0x100000             ; Min address
    ; dd 0xFFFFFFFF80000000   ; Max address
    ; dd 0x1000               ; Alignment
    ; dd 1                    ; Preference (1 = prefer high)

    ; End tag
    dw 0                    ; Type = end
    dw 0                    ; Flags
    dd 8                    ; Size

header_end:

; ---------------------------------------------------------------------------
; BSS Section
; ---------------------------------------------------------------------------
section .bss
align 16

; Initial kernel stack (16 KiB).
; We need a stack before we can call any Rust code.
stack_bottom:
    resb 16384              ; 16 KiB stack
stack_top:

; ---------------------------------------------------------------------------
; Data Section
; ---------------------------------------------------------------------------
section .data

; ---------------------------------------------------------------------------
; Text Section (Entry Point)
; ---------------------------------------------------------------------------
section .text
align 8

global _start
extern kernel_main          ; Defined in kernel_core/src/lib.rs

_start:
    ; GRUB2 has loaded us into memory and jumped here.
    ; At this point:
    ;   - We are in long mode (x86_64)
    ;   - Paging is enabled (identity mapping by GRUB)
    ;   - Interrupts are disabled
    ;   - EAX = Multiboot2 magic (0x36d76289)
    ;   - EBX = Physical address of Multiboot2 info structure
    
    ; ----------------------------------------------------------------
    ; Set up the stack
    ; ----------------------------------------------------------------
    ; Use our pre-allocated stack. The stack grows downward,
    ; so we start at stack_top.
    mov rsp, stack_top
    
    ; ----------------------------------------------------------------
    ; Save Multiboot2 registers
    ; ----------------------------------------------------------------
    ; The Multiboot2 spec says:
    ;   EAX = magic number (0x36d76289)
    ;   EBX = physical address of Multiboot2 info structure
    ;
    ; We need to save these before calling any functions that
    ; might clobber them. On x86_64, we pass arguments in registers
    ; per the System V AMD64 ABI:
    ;   RDI = first argument
    ;   RSI = second argument
    
    ; Zero the upper 32 bits of RBX (it came from EBX).
    mov ebx, ebx            ; This zero-extends to RBX
    
    ; Save the magic number and info pointer as arguments.
    ; RDI = multiboot_magic (from EAX)
    ; RSI = multiboot_info_ptr (from EBX)
    mov edi, eax            ; Magic number -> RDI
    mov rsi, rbx            ; Info pointer -> RSI
    
    ; ----------------------------------------------------------------
    ; Clear the BSS section
    ; ----------------------------------------------------------------
    ; The BSS should be zeroed. GRUB may or may not do this for us,
    ; so we do it ourselves to be safe.
    ;
    ; Note: In a full implementation, we'd iterate over the ELF
    ; sections and zero any SHT_NOBITS sections. For simplicity,
    ; we assume the linker script defines __bss_start and __bss_end.
    
    ; ----------------------------------------------------------------
    ; Call the Rust kernel_main
    ; ----------------------------------------------------------------
    ; kernel_main(uint32_t multiboot_magic, usize multiboot_info_ptr)
    call kernel_main
    
    ; ----------------------------------------------------------------
    ; kernel_main should never return
    ; ----------------------------------------------------------------
    ; If it does, something went very wrong. Halt the CPU.
    cli                     ; Disable interrupts
.halt:
    hlt                     ; Halt until next interrupt
    jmp .halt               ; Loop forever if we wake up

; ---------------------------------------------------------------------------
; Utility Functions
; ---------------------------------------------------------------------------

; Halt the CPU indefinitely.
global halt_forever
halt_forever:
    cli
    hlt
    jmp halt_forever

; Read a byte from an I/O port.
; Arguments: RDI = port
; Returns: AL = value
global inb
inb:
    mov dx, di
    in al, dx
    ret

; Write a byte to an I/O port.
; Arguments: RDI = port, RSI = value
global outb
outb:
    mov dx, di
    mov ax, si
    out dx, al
    ret

; Read a 32-bit value from an I/O port.
global inl
inl:
    mov dx, di
    in eax, dx
    ret

; Write a 32-bit value to an I/O port.
global outl
outl:
    mov dx, di
    mov eax, esi
    out dx, eax
    ret

; ---------------------------------------------------------------------------
; Long Mode GDT (temporary, kernel will set up its own)
; ---------------------------------------------------------------------------
; We don't need a GDT here because GRUB2 has already set one up.
; The kernel's GDT initialization will happen in kernel_main.
; 
; However, if we ever need to reload segments before reaching
; kernel_main, we can do so here.

; ---------------------------------------------------------------------------
; End of boot.asm
; ---------------------------------------------------------------------------
