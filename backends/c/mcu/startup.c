/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The startup of the `c11-mcu` gate's images on QEMU's `mps2-an386` (a Cortex-M4
 * with its single-precision FPU): the vector table the core reads its first stack
 * pointer and its reset handler from, and the reset handler, which enables the
 * FPU, copies `.data` from where the image holds it to RAM, zeroes `.bss`, runs
 * the program and ends the run with its status. The symbols it reads are the
 * linker script's (`mps2_an386.ld`).
 *
 * A fault ends the run with status 3 and says which, so a machine that faults on
 * the target is a red that names itself rather than a run that hangs until the
 * gate's time is up.
 */

#include "sce_mcu_board.h"

extern uint32_t _sidata[];
extern uint32_t _sdata[];
extern uint32_t _edata[];
extern uint32_t _sbss[];
extern uint32_t _ebss[];
extern uint32_t _estack[];

void sce_mcu_reset(void);

/* The Coprocessor Access Control Register: CP10 and CP11 are the FPU. */
#define SCE_MCU_CPACR (*(volatile uint32_t *)0xE000ED88u)

static void sce_mcu_fault(sce_mcu_text_t which) {
    static const sce_mcu_text_t board = SCE_MCU_TEXT("mps2-an386");
    sce_mcu_report(board, which);
    sce_mcu_exit(3);
}

static void sce_mcu_nmi(void) {
    static const sce_mcu_text_t which = SCE_MCU_TEXT("NMI");
    sce_mcu_fault(which);
}

static void sce_mcu_hard_fault(void) {
    static const sce_mcu_text_t which = SCE_MCU_TEXT("hard fault");
    sce_mcu_fault(which);
}

static void sce_mcu_mem_manage(void) {
    static const sce_mcu_text_t which = SCE_MCU_TEXT("memory management fault");
    sce_mcu_fault(which);
}

static void sce_mcu_bus_fault(void) {
    static const sce_mcu_text_t which = SCE_MCU_TEXT("bus fault");
    sce_mcu_fault(which);
}

static void sce_mcu_usage_fault(void) {
    static const sce_mcu_text_t which = SCE_MCU_TEXT("usage fault (an FPU instruction before the FPU was on?)");
    sce_mcu_fault(which);
}

static void sce_mcu_unexpected(void) {
    static const sce_mcu_text_t which = SCE_MCU_TEXT("an exception no handler was written for");
    sce_mcu_fault(which);
}

/* An entry of the vector table: the first is the initial stack pointer, every
   other a handler. A union, since ISO C converts neither one to the other. */
typedef union sce_mcu_vector {
    void *stack;
    void (*handler)(void);
} sce_mcu_vector_t;

/* The sixteen system exceptions of an Armv7-M core; no interrupt is enabled. */
__attribute__((section(".isr_vector"), used)) static const sce_mcu_vector_t sce_mcu_vectors[16] = {
    {.stack = _estack},
    {.handler = sce_mcu_reset},
    {.handler = sce_mcu_nmi},
    {.handler = sce_mcu_hard_fault},
    {.handler = sce_mcu_mem_manage},
    {.handler = sce_mcu_bus_fault},
    {.handler = sce_mcu_usage_fault},
    {.handler = sce_mcu_unexpected},
    {.handler = sce_mcu_unexpected},
    {.handler = sce_mcu_unexpected},
    {.handler = sce_mcu_unexpected},
    {.handler = sce_mcu_unexpected},
    {.handler = sce_mcu_unexpected},
    {.handler = sce_mcu_unexpected},
    {.handler = sce_mcu_unexpected},
    {.handler = sce_mcu_unexpected},
};

void sce_mcu_reset(void) {
    /* Full access to CP10 and CP11 before any floating-point instruction runs:
       the code is built for `-mfloat-abi=hard`. */
    SCE_MCU_CPACR |= 0xFu << 20;
    __asm__ volatile("dsb\n\tisb" ::: "memory");
    for (uint32_t *from = _sidata, *to = _sdata; to < _edata;) {
        *to++ = *from++;
    }
    for (uint32_t *to = _sbss; to < _ebss;) {
        *to++ = 0u;
    }
    sce_mcu_exit(sce_mcu_main());
}
