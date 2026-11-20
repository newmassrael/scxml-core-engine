/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The memory of QEMU's `mps2-an386` (Arm's AN386 on an MPS2 board, a Cortex-M4
 * with its FPU), as the `c11-mcu` gate links an image for it:
 *
 *   ZBT SSRAM1     at 0x00000000, 4 MiB: the code, the constants and the
 *                  initial values of `.data`. The core reads its vector table
 *                  from address 0 at reset.
 *   ZBT SSRAM2/3   at 0x20000000, 4 MiB: `.data`, `.bss` and the stack.
 *
 * The startup (`startup.c`) copies `.data` from `_sidata` to `_sdata`.._edata,
 * zeroes `_sbss`.._ebss, and takes its first stack pointer from `_estack`.
 *
 * Sections are not collected (`--gc-sections` is not passed): every function of
 * every object the gate links is in the image, so a call any of them makes to a
 * function nothing defines is an undefined symbol at the link, used or not.
 */

ENTRY(sce_mcu_reset)

MEMORY
{
    SSRAM1 (rx)   : ORIGIN = 0x00000000, LENGTH = 4M
    SSRAM23 (rwx) : ORIGIN = 0x20000000, LENGTH = 4M
}

/* The stack: the deepest a scenario's machine has been measured to need is the
   gate's to find out, so this is generous rather than tight. */
_sce_mcu_stack_size = 64K;

SECTIONS
{
    .isr_vector :
    {
        KEEP(*(.isr_vector))
    } > SSRAM1

    .text :
    {
        *(.text .text.*)
        *(.glue_7) *(.glue_7t) *(.vfp11_veneer) *(.v4_bx)
    } > SSRAM1

    .rodata : ALIGN(4)
    {
        *(.rodata .rodata.*)
    } > SSRAM1

    .ARM.extab :
    {
        *(.ARM.extab* .gnu.linkonce.armextab.*)
    } > SSRAM1

    .ARM.exidx :
    {
        *(.ARM.exidx* .gnu.linkonce.armexidx.*)
    } > SSRAM1

    _sidata = LOADADDR(.data);

    .data : ALIGN(4)
    {
        _sdata = .;
        *(.data .data.*)
        . = ALIGN(4);
        _edata = .;
    } > SSRAM23 AT > SSRAM1

    .bss (NOLOAD) : ALIGN(4)
    {
        _sbss = .;
        *(.bss .bss.* COMMON)
        . = ALIGN(4);
        _ebss = .;
    } > SSRAM23

    .stack (NOLOAD) : ALIGN(8)
    {
        . = . + _sce_mcu_stack_size;
        . = ALIGN(8);
        _estack = .;
    } > SSRAM23
}
