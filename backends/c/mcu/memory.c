/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The four memory functions a C11 machine's core may call (docs/adr/0110, D1):
 * GCC and Clang emit calls to them even under `-ffreestanding`, so every C
 * environment supplies them, and a deployment brings its own. These are the
 * `c11-mcu` gate's, written plainly, a byte at a time: the gate weighs the
 * machine, not these, and a word-at-a-time copy would only move bytes from one
 * figure to another.
 *
 * Built with `-fno-tree-loop-distribute-patterns` beside the gate's flags: GCC
 * otherwise recognises each loop below as the function it is in and replaces it
 * with a call to that function, which recurses without end.
 */

#include <stddef.h>

void *memcpy(void *restrict to, const void *restrict from, size_t n);
void *memmove(void *to, const void *from, size_t n);
void *memset(void *to, int value, size_t n);
int memcmp(const void *a, const void *b, size_t n);

void *memcpy(void *restrict to, const void *restrict from, size_t n) {
    unsigned char *t = to;
    const unsigned char *f = from;
    for (size_t at = 0u; at < n; ++at) {
        t[at] = f[at];
    }
    return to;
}

void *memmove(void *to, const void *from, size_t n) {
    unsigned char *t = to;
    const unsigned char *f = from;
    if (t < f) {
        for (size_t at = 0u; at < n; ++at) {
            t[at] = f[at];
        }
    } else {
        for (size_t at = n; at > 0u; --at) {
            t[at - 1u] = f[at - 1u];
        }
    }
    return to;
}

void *memset(void *to, int value, size_t n) {
    unsigned char *t = to;
    for (size_t at = 0u; at < n; ++at) {
        t[at] = (unsigned char)value;
    }
    return to;
}

int memcmp(const void *a, const void *b, size_t n) {
    const unsigned char *x = a;
    const unsigned char *y = b;
    for (size_t at = 0u; at < n; ++at) {
        if (x[at] != y[at]) {
            return x[at] < y[at] ? -1 : 1;
        }
    }
    return 0;
}
