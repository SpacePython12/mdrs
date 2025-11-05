MEMORY
{
    ROM (rx) : ORIGIN = 0, LENGTH = 0x400000
    RAM (rwx) : ORIGIN = 0xFF0000, LENGTH = 0x10000
}

SECTIONS
{
    _stack_top = 0x01000000;
    _stack_bottom = _stack_top - 0x1000;

    .text :
    {
        KEEP(*(.text.ivt))
        KEEP(*(.text.hdr))
        KEEP(*(.text.boot))
        KEEP(*(.text._init))
        KEEP(*(.text._extint))
        KEEP(*(.text._hblank))
        KEEP(*(.text._vblank))
        *(.text .text.* .note.*);
    } > ROM

    .rodata :
    {
        *(.rodata .rodata.*);
        _data_src = .;
    } > ROM

    .data :
    {
        . = ALIGN(4);
        _data_start = .;
        *(.data .data.*);
    } > RAM AT > ROM

    . = ALIGN(4);
    _data_end = .;

    .bss (NOLOAD) :
    {
        . = ALIGN(4);
        _bss_start = .;
        *(.bss .bss.*);
        . = ALIGN(4);
        _bss_end = .;
    } > RAM AT > ROM

    . = ALIGN(16);
    _heap_start = .;
    _heap_end = _stack_bottom;
}