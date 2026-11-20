/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The seams a C11 machine's core declares and its deployment defines
 * (docs/adr/0110, D2), as the `c11-mcu` gate's image defines them:
 *
 *   - `_sce_clock_now_ms` (sce/clock.h): a counter, which a scenario's
 *     `advance_ms` moves (`sce_mcu_clock_advance`) before the machine's tick.
 *     Nothing else moves it, so a run is the same on every host QEMU runs on.
 *   - `_sce_log_write` and `_sce_log_real` (sce/log.h): a line to the host's
 *     standard output through semihosting. A real is written as the 64 bits it
 *     is, in hexadecimal: the seam formats as it likes, and this one holds no
 *     formatter of reals, so the image weighs none.
 */

#include <stddef.h>
#include <stdint.h>

#include <sce/clock.h>
#include <sce/log.h>

#include "sce_mcu_board.h"

void *memcpy(void *restrict to, const void *restrict from, size_t n);

static uint64_t sce_mcu_clock_ms;

uint64_t _sce_clock_now_ms(void) {
    return sce_mcu_clock_ms;
}

void sce_mcu_clock_advance(uint64_t ms) {
    sce_mcu_clock_ms += ms;
}

/* The word a line of `level` begins with. */
static sce_mcu_text_t sce_mcu_level(sce_log_level_t level) {
    static const sce_mcu_text_t info = SCE_MCU_TEXT("info ");
    static const sce_mcu_text_t warning = SCE_MCU_TEXT("warning ");
    static const sce_mcu_text_t error = SCE_MCU_TEXT("error ");
    switch (level) {
    case SCE_LOG_INFO:
        return info;
    case SCE_LOG_WARNING:
        return warning;
    case SCE_LOG_ERROR:
        return error;
    }
    return error;
}

/* `<level> <label>: ` and nothing more, for a line whose value follows. */
static void sce_mcu_line_head(sce_log_level_t level, const char *label, size_t label_len) {
    static const char between[] = ": ";
    const sce_mcu_text_t word = sce_mcu_level(level);
    sce_mcu_write(word.text, word.len);
    if (label != NULL) {
        sce_mcu_write(label, label_len);
        sce_mcu_write(between, sizeof(between) - 1u);
    }
}

void _sce_log_write(sce_log_level_t level, const char *label, size_t label_len, const char *text, size_t text_len) {
    static const char end[] = "\n";
    sce_mcu_line_head(level, label, label_len);
    if (text != NULL) {
        sce_mcu_write(text, text_len);
    }
    sce_mcu_write(end, sizeof(end) - 1u);
}

void _sce_log_real(sce_log_level_t level, const char *label, size_t label_len, double value) {
    static const char digits[] = "0123456789abcdef";
    static const char lead[] = "real 0x";
    static const char end[] = "\n";
    _Static_assert(sizeof(double) == sizeof(uint64_t), "a double is 64 bits");
    uint64_t bits = 0u;
    (void)memcpy(&bits, &value, sizeof(bits));
    char hex[16];
    for (size_t at = 0u; at < sizeof(hex); ++at) {
        hex[sizeof(hex) - 1u - at] = digits[(bits >> (4u * at)) & 0xFu];
    }
    sce_mcu_line_head(level, label, label_len);
    sce_mcu_write(lead, sizeof(lead) - 1u);
    sce_mcu_write(hex, sizeof(hex));
    sce_mcu_write(end, sizeof(end) - 1u);
}
