/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The board of the `c11-mcu` gate (docs/adr/0110, D3): QEMU's `mps2-an386`, a
 * Cortex-M4 with its FPU, run with semihosting. What an image links beside the
 * code it weighs is this and nothing else: the startup (`startup.c`), the four
 * memory functions (`memory.c`), the seams a machine's core declares
 * (`seams.c`) and the semihosting calls (`semihosting.c`). No C library: the
 * image is linked with `-nostdlib -lgcc`, so a call the core makes to anything
 * else is an undefined symbol, which the gate names.
 *
 * Every text written here carries its length, since the image has no `strlen`.
 */

#ifndef SCE_MCU_BOARD_H
#define SCE_MCU_BOARD_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* A text and its length in bytes, with no terminator counted. */
typedef struct sce_mcu_text {
    const char *text;
    size_t len;
} sce_mcu_text_t;

/* A string literal as a text: its bytes and their count. */
#define SCE_MCU_TEXT(literal) {(literal), sizeof(literal) - 1u}

/* The image's program: what the reset handler runs once memory is set up. Its
 * answer is the exit status QEMU ends with, 0 for a pass. */
int sce_mcu_main(void);

/* Write one line, `label: text`, to the host's standard output through
 * semihosting. */
void sce_mcu_report(sce_mcu_text_t label, sce_mcu_text_t text);

/* Write the bytes of `text` and nothing else. */
void sce_mcu_write(const char *text, size_t len);

/* End the run: QEMU exits with `status`, which is the gate's verdict of it. */
_Noreturn void sce_mcu_exit(int status);

/* Move the image's clock, the counter `_sce_clock_now_ms` reads, on by `ms`. */
void sce_mcu_clock_advance(uint64_t ms);

#endif /* SCE_MCU_BOARD_H */
