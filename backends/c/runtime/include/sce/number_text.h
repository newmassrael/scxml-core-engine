/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * How a 64-bit real is written into JSON text or the text of an untyped
 * `<param>` (ARCHITECTURE.md, "JSON Number Text"): as ECMAScript's
 * `Number::toString` writes it, the one spelling every engine writes, so the same
 * value is the same bytes whichever engine wrote it.
 *
 * Header-only and dependent on nothing outside the C library and this include
 * directory, because both of its callers must be able to include it without the
 * other: the typed payload an event carries (`sce/event_payload.h`, which a
 * machine with no `sce-static` datamodel includes) and the `sce-static` wire
 * writer (`sce/forge/wire.h`). tests/json_text/real_text.json holds the cases.
 */
#ifndef SCE_NUMBER_TEXT_H
#define SCE_NUMBER_TEXT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Whether the decimal `digits` (`k` of them) times ten to `exponent` — the
 * exponent of the first digit — reads back as `magnitude`. The text handed to
 * `strtod` has no radix character (the digits as one integer, scaled by the
 * exponent of the last), so a locale that spells the radix with a comma cannot
 * change what it reads. */
static inline bool sce_number_reads_back(const char *digits, int k, int exponent, double magnitude) {
    char text[64];
    (void)snprintf(text, sizeof(text), "%.*se%d", k, digits, exponent - (k - 1));
    return strtod(text, NULL) == magnitude;
}

/* A 64-bit real as ECMAScript's `Number::toString` spells it, radix 10: the
 * fewest digits that read back as the same double, in decimal notation when
 * `1e-6 <= |x| < 1e21` and as `d[.ddd]e[+-]n` otherwise, no fraction on a whole
 * value and `0` for either zero. A value that is not finite is `NaN`, `Infinity`
 * or `-Infinity`, as `String(x)` has it; a JSON writer, which has none of those,
 * tests for them first.
 *
 * `printf` has no shortest form, so the digits are the first precision at which
 * the rounded decimal reads back, and when that rounding does not, the decimal
 * one unit above it: below a power of two the neighbouring double is half as far,
 * so the nearest decimal can fall outside the interval that reads as the value
 * (2^-44 is `5.684341886080802e-14`, not `...801e-14`).
 *
 * Written into `buf` of `cap` bytes. The length, or 0 when it did not fit, and
 * `buf` then holds the empty string, never a truncated number. */
static inline size_t sce_number_text(double value, char *buf, size_t cap) {
    char out[64];
    size_t n = 0u;
    if (cap == 0u) {
        return 0u;
    }
    if (value != value) {
        n = (size_t)snprintf(out, sizeof(out), "NaN");
    } else if (value - value != 0.0) {
        n = (size_t)snprintf(out, sizeof(out), "%s", value > 0.0 ? "Infinity" : "-Infinity");
    } else if (value == 0.0) {
        n = (size_t)snprintf(out, sizeof(out), "0");
    } else {
        const bool negative = value < 0.0;
        const double magnitude = negative ? -value : value;
        char digits[24] = {0};
        int k = 0;
        int exponent = 0;
        for (int precision = 0; precision <= 16; ++precision) {
            char scientific[48];
            (void)snprintf(scientific, sizeof(scientific), "%.*e", precision, magnitude);
            const char *p = scientific;
            k = 0;
            for (; *p != '\0' && *p != 'e' && *p != 'E'; ++p) {
                if (*p >= '0' && *p <= '9') {
                    digits[k++] = *p;
                }
            }
            exponent = *p != '\0' ? (int)strtol(p + 1, NULL, 10) : 0;
            if (sce_number_reads_back(digits, k, exponent, magnitude)) {
                break;
            }
            /* The decimal one unit above, a carry out of the first digit
             * leaving a one and zeros. */
            char above[24];
            int above_exponent = exponent;
            int i = k - 1;
            memcpy(above, digits, (size_t)k);
            while (i >= 0 && above[i] == '9') {
                above[i--] = '0';
            }
            if (i >= 0) {
                above[i] = (char)(above[i] + 1);
            } else {
                above[0] = '1';
                above_exponent++;
            }
            if (sce_number_reads_back(above, k, above_exponent, magnitude)) {
                memcpy(digits, above, (size_t)k);
                exponent = above_exponent;
                break;
            }
        }
        /* The value is 0.<digits> * 10^point: the specification's `k` and `n`. */
        const int point = exponent + 1;
        if (negative) {
            out[n++] = '-';
        }
        if (k <= point && point <= 21) {
            memcpy(out + n, digits, (size_t)k);
            n += (size_t)k;
            for (int i = k; i < point; ++i) {
                out[n++] = '0';
            }
        } else if (0 < point && point <= 21) {
            memcpy(out + n, digits, (size_t)point);
            n += (size_t)point;
            out[n++] = '.';
            memcpy(out + n, digits + point, (size_t)(k - point));
            n += (size_t)(k - point);
        } else if (-6 < point && point <= 0) {
            out[n++] = '0';
            out[n++] = '.';
            for (int i = 0; i < -point; ++i) {
                out[n++] = '0';
            }
            memcpy(out + n, digits, (size_t)k);
            n += (size_t)k;
        } else {
            const int power = point - 1;
            out[n++] = digits[0];
            if (k > 1) {
                out[n++] = '.';
                memcpy(out + n, digits + 1, (size_t)(k - 1));
                n += (size_t)(k - 1);
            }
            out[n++] = 'e';
            out[n++] = power < 0 ? '-' : '+';
            n += (size_t)snprintf(out + n, sizeof(out) - n, "%d", power < 0 ? -power : power);
        }
    }
    if (n >= cap) {
        buf[0] = '\0';
        return 0u;
    }
    memcpy(buf, out, n);
    buf[n] = '\0';
    return n;
}

#endif /* SCE_NUMBER_TEXT_H */
