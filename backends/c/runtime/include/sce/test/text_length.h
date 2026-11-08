/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file text_length.h
 * @brief The length a template's length restriction holds a text to
 *        (docs/SCE_TEST_PROFILE.md §6.1, §11.4)
 *
 * ES 201 873-1 B.1.4.1 restricts the length of a `universal charstring` in
 * characters, and SCE holds a text as UTF-8 bytes, so the two counts differ
 * wherever a character takes more than one byte. The C11 port of
 * `sce/include/test/Length.h`. Header-only and allocation-free.
 */

#ifndef SCE_TEST_TEXT_LENGTH_H
#define SCE_TEST_TEXT_LENGTH_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * The TTCN-3 length of the NUL-terminated text `text`: how many characters it
 * holds, each one Unicode code point.
 *
 * A generated template's `length-min` and `length-max` on a `string[N]` position
 * compare this, and not the text's size in bytes, which is what bounds it in SCE
 * (docs/SCE_TEST_PROFILE.md §6.3).
 *
 * Precondition: `text` is well-formed UTF-8 (RFC 3629). Every SCE string value
 * is, by construction: a codec refuses to decode one that is not, and a text the
 * document writes is read as UTF-8. Each character then begins with exactly one
 * byte that is not a continuation byte (`10xxxxxx`), so the characters are
 * counted as those bytes.
 */
static inline size_t sce_test_text_length(const char *text) {
    size_t characters = 0u;
    for (const unsigned char *byte = (const unsigned char *)text; *byte != 0u; ++byte) {
        if ((*byte & 0xC0u) != 0x80u) {
            ++characters;
        }
    }
    return characters;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_TEXT_LENGTH_H */
