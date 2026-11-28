/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * The four memory functions a C11 machine's core calls, declared as its core
 * declares them (docs/adr/0110, D1): `memcpy`, `memmove`, `memset` and
 * `memcmp`, which GCC and Clang emit calls to even under `-ffreestanding`, so
 * every C environment supplies them.
 *
 * They are declared here rather than taken from `<string.h>`, which is not a
 * freestanding header. A declaration of a library function without its header
 * is one C allows (C11 7.1.4p2), and it is compatible with the one a hosted
 * unit's `<string.h>` makes. C++ has no such allowance once the library's own
 * declaration carries an exception specification, so a C++ consumer takes them
 * from `<cstring>`: a C++ translation unit is never a machine's core.
 *
 * A header of its own, apart from the text routines of `sce/text.h` (which
 * includes this one), so that a header that needs a memory function and no
 * text routine, as the ownership contract `sce/sample.h` needs `memset`,
 * brings none of those routines into the units that include it.
 *
 * Block comments, as `sce/portability.h` has them, so the header parses under
 * every standard a consumer may compile it with.
 */
#ifndef SCE_MEM_H
#define SCE_MEM_H

#include <stddef.h>

#if defined(__cplusplus)
#include <cstring>
#else
/* Each name in parentheses, so a library that spells one of them as a
 * function-like macro (a fortified one may) declares the function and not the
 * macro's expansion. */
void *(memcpy)(void *dst, const void *src, size_t n);
void *(memmove)(void *dst, const void *src, size_t n);
void *(memset)(void *dst, int value, size_t n);
int(memcmp)(const void *a, const void *b, size_t n);
#endif

#endif /* SCE_MEM_H */
