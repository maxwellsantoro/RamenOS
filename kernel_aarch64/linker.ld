ENTRY(_start)

SECTIONS {
  . = 0x40080000;
  .text : {
    *(.text._start)
    *(.text*)
  }
  .rodata : { *(.rodata*) }
  .data : { *(.data*) }
  .bss : {
    *(.bss*)
    *(COMMON)
  }
  . = ALIGN(16);
  __stack_start = .;
  . = . + 0x10000;
  __stack_end = .;
}
