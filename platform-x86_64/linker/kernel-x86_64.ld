ENTRY(_start)

KERNEL_VMA = 0xffffffff80000000;
BOOT_STACK_SIZE = 256K;

SECTIONS
{
    . = KERNEL_VMA;
    __kernel_start = .;

    .limine_requests ALIGN(8) : AT(ADDR(.limine_requests) - KERNEL_VMA)
    {
        KEEP(*(.limine_requests_start))
        KEEP(*(.limine_requests))
        KEEP(*(.limine_requests_end))
    }

    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_VMA)
    {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_VMA)
    {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_VMA)
    {
        *(.data .data.*)
    }

    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_VMA)
    {
        __bss_start = .;
        *(COMMON)
        *(.bss .bss.*)
        __bss_end = .;
    }

    .boot_stack ALIGN(16) (NOLOAD) : AT(ADDR(.boot_stack) - KERNEL_VMA)
    {
        __ngos_boot_stack_bottom = .;
        . += BOOT_STACK_SIZE;
        __ngos_boot_stack_top = .;
    }

    __kernel_end = .;

    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.comment)
        *(.note .note.*)
    }
}
