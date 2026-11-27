/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * The report of a `<sce:test-vector>` row that does not hold, written once for
 * the C11 sidecars of an algorithm and of a codec (`<stem>_test.h`): a sidecar
 * says what failed, and how it is said is here.
 *
 * A sidecar compiles wherever its unit does, a bare-metal target included, so it
 * includes no hosted header and formats no text with `printf` (docs/adr/0110,
 * D1). A row's line is built with `sce/text.h` into a buffer the sidecar holds
 * and handed to the log seam (`sce/log.h`, D2) at the error level, labelled by
 * the unit whose row failed. The host's seam (`backends/c/runtime/posix/log.c`)
 * writes it to `stderr` as `<unit>: test_vector @SCXML L<line>: <what>`; a
 * target's seam writes it wherever that target writes its log. A program that
 * runs a sidecar links a definition of the seam.
 */
#ifndef SCE_FORGE_TEST_VECTOR_H
#define SCE_FORGE_TEST_VECTOR_H

#include <stddef.h>
#include <stdint.h>

#include "sce/log.h"
#include "sce/text.h"

#ifdef __cplusplus
extern "C" {
#endif

/* The bytes of a report's line, its terminator included. A line longer than
 * this (a field with a very long name) is cut where it reaches the end, and is
 * still reported: the row fails all the same. */
#define SCE_FORGE_TEST_VECTOR_LINE_CAP 192u

/* A report's line begun in `buf` of `cap` bytes, naming the line of the
 * document the row stands on: `test_vector @SCXML L<line>: `, which the row's
 * own account of what failed follows. */
static inline sce_text_writer_t sce_forge_test_vector_line(char *buf, size_t cap, uint32_t source_line) {
    sce_text_writer_t line = sce_text_writer(buf, cap);
    SCE_TEXT_PUT_LITERAL(&line, "test_vector @SCXML L");
    sce_text_put_u64(&line, (uint64_t)source_line);
    SCE_TEXT_PUT_LITERAL(&line, ": ");
    return line;
}

/* `line` handed to the log seam at the error level, labelled by `unit`, the
 * name of the algorithm or the codec whose row failed. */
static inline void sce_forge_test_vector_failed(const char *unit, const sce_text_writer_t *line) {
    _sce_log_write(SCE_LOG_ERROR, unit, sce_text_length(unit), line->buf, line->len);
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_FORGE_TEST_VECTOR_H */
