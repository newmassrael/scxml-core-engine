/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * A real number as text, and text as a real number: two seams a C11 machine's
 * core declares and what its deployment links defines (docs/adr/0110, D1, D2).
 *
 * A machine's core never formats or parses a real: no freestanding
 * implementation offers either, and the hosted library's formatter is the
 * largest single cost of an image. Where a value of a real type must become
 * text (the JSON an event carries, a request's text) or text must become a real
 * (a typed field read out of an event's data), the core hands it whole to one
 * of these, as `<log>` hands a real to `_sce_log_real` (`sce/log.h`).
 *
 * The runtime's text unit (backends/c/runtime/text/real_text.c, the library
 * `sce_c_text`) defines both with the hosted C library: the spelling every
 * engine writes (`sce/number_text.h`, ARCHITECTURE.md "JSON Number Text") and
 * `strtod`. A bare-metal deployment links that unit with its C library, or
 * defines them itself. A machine that writes or reads no real references
 * neither, and a machine that may states the `real` requirement
 * (`RuntimeRequirement::Real`, sce-build/src/generator.rs).
 */
#ifndef SCE_REAL_TEXT_H
#define SCE_REAL_TEXT_H

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* `value` as ECMAScript's `Number::toString` spells it, radix 10: the fewest
 * digits that read back as the same double, `NaN`, `Infinity` and `-Infinity`
 * for a value that is not finite. Written into `buf` of `cap` bytes with its
 * NUL: the length, or 0 when it does not fit, and `buf` then holds the empty
 * string, never a cut number. */
size_t _sce_real_text(double value, char *buf, size_t cap);

/* The JSON number the `len` bytes at `text` spell, which the caller has held
 * to the JSON grammar of a number and which need not end in a NUL, read into
 * `*out`: false for one the reader cannot hold, which is refused rather than
 * read as an infinity or a zero. The text unit refuses what `strtod` reports
 * out of range, past the range of a double or below its normal magnitudes. */
bool _sce_real_read(const char *text, size_t len, double *out);

#ifdef __cplusplus
}
#endif

#endif /* SCE_REAL_TEXT_H */
