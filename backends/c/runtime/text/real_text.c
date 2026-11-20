/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * The runtime's text unit (docs/adr/0110, D2): a real number as text and text
 * as a real number, the two seams of `sce/real_text.h`, defined with the hosted
 * C library. A machine's core formats and parses no real; a deployment that
 * links this unit beside it gets the spelling every engine writes
 * (`sce/number_text.h`, ARCHITECTURE.md "JSON Number Text") and `strtod`'s
 * reading, and one that does not defines the two itself.
 */
#include "sce/real_text.h"

#include <errno.h>
#include <stdlib.h>
#include <string.h>

#include "sce/number_text.h"

/* The longest JSON number text this unit reads whole: a double's shortest text
 * is at most twenty-five bytes, and a text longer than this one holds more
 * digits than any double tells apart, so it is copied no further than a
 * terminator can stand. */
#define SCE_REAL_TEXT_READ_CAP 512u

size_t _sce_real_text(double value, char *buf, size_t cap) {
    return sce_number_text(value, buf, cap);
}

bool _sce_real_read(const char *text, size_t len, double *out) {
    /* `strtod` reads up to a terminator, which the caller's text has none of
     * where the number ends inside a larger document. */
    char copy[SCE_REAL_TEXT_READ_CAP];
    char *end = NULL;
    if (len == 0u || len >= sizeof(copy)) {
        return false;
    }
    memcpy(copy, text, len);
    copy[len] = '\0';
    errno = 0;
    const double value = strtod(copy, &end);
    if (end != copy + len || errno == ERANGE) {
        return false;
    }
    *out = value;
    return true;
}
