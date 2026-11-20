/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The board's output and its end, through Arm semihosting: a `bkpt 0xAB` with the
 * operation in r0 and its argument in r1, which QEMU answers when it is run with
 * `-semihosting-config enable=on,target=native` (the `c11-mcu` gate does). The
 * operations used are the ones every semihosting host implements:
 *
 *   SYS_WRITEC (0x03)         one character to the host's console;
 *   SYS_EXIT_EXTENDED (0x20)  end with a reason and a status, which QEMU exits
 *                             with when the reason is ADP_Stopped_ApplicationExit;
 *   SYS_EXIT (0x18)           end with a reason alone, after the above, for a host
 *                             that does not know it: QEMU then exits 0 for an
 *                             application exit and 1 for any other reason.
 */

#include "sce_mcu_board.h"

enum {
    SCE_MCU_SYS_WRITEC = 0x03,
    SCE_MCU_SYS_EXIT = 0x18,
    SCE_MCU_SYS_EXIT_EXTENDED = 0x20,
};

/* The reasons a run ends for (ADP_Stopped_*). */
#define SCE_MCU_APPLICATION_EXIT 0x20026u
#define SCE_MCU_RUN_TIME_ERROR 0x20023u

static uint32_t sce_mcu_semihost(uint32_t operation, const void *argument) {
    register uint32_t r0 __asm__("r0") = operation;
    register const void *r1 __asm__("r1") = argument;
    __asm__ volatile("bkpt 0xAB" : "+r"(r0) : "r"(r1) : "memory");
    return r0;
}

void sce_mcu_write(const char *text, size_t len) {
    for (size_t at = 0u; at < len; ++at) {
        (void)sce_mcu_semihost(SCE_MCU_SYS_WRITEC, &text[at]);
    }
}

void sce_mcu_report(sce_mcu_text_t label, sce_mcu_text_t text) {
    static const char between[] = ": ";
    static const char end[] = "\n";
    sce_mcu_write(label.text, label.len);
    sce_mcu_write(between, sizeof(between) - 1u);
    sce_mcu_write(text.text, text.len);
    sce_mcu_write(end, sizeof(end) - 1u);
}

_Noreturn void sce_mcu_exit(int status) {
    const uint32_t block[2] = {SCE_MCU_APPLICATION_EXIT, (uint32_t)status};
    (void)sce_mcu_semihost(SCE_MCU_SYS_EXIT_EXTENDED, block);
    /* A host that does not know the extended exit returns here. The reason is
       passed in r1 itself on a 32-bit target, not through a block. */
    (void)sce_mcu_semihost(SCE_MCU_SYS_EXIT,
                           (const void *)(uintptr_t)(status == 0 ? SCE_MCU_APPLICATION_EXIT : SCE_MCU_RUN_TIME_ERROR));
    for (;;) {
    }
}
