ENTRY(_start)

USER_VMA = 0x0000000000400000;

SECTIONS
{
    . = USER_VMA;
    __user_image_start = .;

    .text ALIGN(4K) :
    {
        KEEP(*(.text.entry))
        *(.text .text.*)
    }

    .rodata ALIGN(4K) :
    {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) :
    {
        *(.data .data.*)
    }

    .bss ALIGN(4K) (NOLOAD) :
    {
        __user_bss_start = .;
        *(COMMON)
        *(.bss .bss.*)
        __user_bss_end = .;
    }

    __user_image_end = .;

    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.comment)
        *(.note .note.*)
    }
}
