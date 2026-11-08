// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The TTCN-3 length of a text on the C11 backend (docs/SCE_TEST_PROFILE.md
// §6.1, §11.4, sce/test/text_length.h): ES 201 873-1 B.1.4.1 counts a universal
// charstring in characters, so a character of two, three or four UTF-8 bytes is
// one, as a character of one byte is.
//
// Each case is the twin of the Rust case of the module `text_length` in
// backends/rust/runtime/tests/a_text_length_is_its_count_of_characters.rs,
// named `text_length_<case>`, and of tests/test_runtime/TextLengthTest.cpp, and
// pins the length apart from the byte length `strlen`, which bounds a text in
// SCE. A text's bytes past ASCII are written as escapes.
//
// Header-only code; nothing here allocates.

#include <sce/test/text_length.h>

#include <stdio.h>
#include <string.h>

static int failures = 0;

#define CHECK(condition, ...)                                                                                          \
    do {                                                                                                               \
        if (!(condition)) {                                                                                            \
            (void)fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                 \
            (void)fprintf(stderr, __VA_ARGS__);                                                                        \
            (void)fputc('\n', stderr);                                                                                 \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

static void holds_characters(const char *text, size_t bytes, size_t characters) {
    CHECK(strlen(text) == bytes, "strlen is %zu, not %zu", strlen(text), bytes);
    CHECK(sce_test_text_length(text) == characters, "the length is %zu, not %zu", sce_test_text_length(text),
          characters);
}

static void text_length_an_empty_text_holds_no_character(void) {
    holds_characters("", 0u, 0u);
}

static void text_length_an_ascii_text_holds_one_character_per_byte(void) {
    holds_characters("started", 7u, 7u);
}

// U+00E9, two bytes: C3 A9.
static void text_length_a_character_of_two_bytes_is_one(void) {
    holds_characters("caf\xC3\xA9", 5u, 4u);
}

// U+20AC, three bytes: E2 82 AC.
static void text_length_a_character_of_three_bytes_is_one(void) {
    holds_characters("\xE2\x82\xAC"
                     "5",
                     4u, 2u);
}

// U+1F600, four bytes: F0 9F 98 80.
static void text_length_a_character_of_four_bytes_is_one(void) {
    holds_characters("a\xF0\x9F\x98\x80"
                     "b",
                     6u, 3u);
}

// One character of each width in a row: ten bytes, four characters.
static void text_length_characters_of_every_width_are_counted_together(void) {
    holds_characters("a\xC3\xA9\xE2\x82\xAC\xF0\x9F\x98\x80", 10u, 4u);
}

int main(void) {
    text_length_an_empty_text_holds_no_character();
    text_length_an_ascii_text_holds_one_character_per_byte();
    text_length_a_character_of_two_bytes_is_one();
    text_length_a_character_of_three_bytes_is_one();
    text_length_a_character_of_four_bytes_is_one();
    text_length_characters_of_every_width_are_counted_together();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("text length: ok");
    return 0;
}
