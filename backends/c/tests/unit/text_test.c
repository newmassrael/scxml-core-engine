// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The text routines a C11 machine's core calls in place of the hosted library
// (`sce/text.h`, docs/adr/0110 D1), held to the answers of the routines they
// stand in for: each comparison to `strcmp` and `strncmp` by its sign, each
// length to `strlen`, the last place of a character to `strrchr`, and the text
// of an integer to `printf`'s, at both ends of every width. The hosted library
// is the oracle here and nowhere in the core.

#include <sce/text.h>

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static int failures = 0;

static void expect(int ok, const char *what) {
    if (!ok) {
        (void)fprintf(stderr, "FAIL: %s\n", what);
        ++failures;
    }
}

static int sign(int v) {
    return (v > 0) - (v < 0);
}

static void the_comparisons_agree_with_the_hosted_ones(void) {
    static const char *const texts[] = {"", "a", "ab", "abc", "abd", "b", "\xff", "a\xff", "A"};
    const size_t n = sizeof(texts) / sizeof(texts[0]);
    for (size_t i = 0u; i < n; ++i) {
        expect(sce_text_length(texts[i]) == strlen(texts[i]), "sce_text_length is strlen");
        for (size_t j = 0u; j < n; ++j) {
            expect(sign(sce_text_compare(texts[i], texts[j])) == sign(strcmp(texts[i], texts[j])),
                   "sce_text_compare orders as strcmp does, bytes read unsigned");
            for (size_t k = 0u; k < 4u; ++k) {
                expect(sign(sce_text_compare_n(texts[i], texts[j], k)) == sign(strncmp(texts[i], texts[j], k)),
                       "sce_text_compare_n orders as strncmp does");
            }
        }
    }
    expect(sce_text_starts_with("file:x.scxml", "file:", 5u), "a prefix is found");
    expect(!sce_text_starts_with("fil", "file:", 5u), "a text shorter than the prefix does not start with it");
}

static void the_last_place_and_a_byte_are_found(void) {
    const char *name = "a.b.c";
    expect(sce_text_find_last(name, '.') == strrchr(name, '.'), "the last dot is strrchr's");
    expect(sce_text_find_last(name, 'x') == NULL, "a character the text lacks is not found");
    expect(sce_text_find_last("", '.') == NULL, "the empty text holds nothing");
    const char held[4] = {'a', '\0', 'b', 'c'};
    expect(sce_text_find_byte(held, '\0', sizeof(held)) == held + 1, "a NUL is a byte like any other");
    expect(sce_text_find_byte(held, 'z', sizeof(held)) == NULL, "a byte the span lacks is not found");
    char copy[8];
    expect(sce_text_copy(copy, "abc") == copy && strcmp(copy, "abc") == 0, "a copy holds the text and its NUL");
}

static void the_text_of_an_integer_is_printfs(void) {
    char got[SCE_TEXT_DECIMAL_CAP];
    char want[32];
    static const uint64_t unsigned_values[] = {0u, 1u, 9u, 10u, 4294967295u, 4294967296u, UINT64_MAX};
    for (size_t i = 0u; i < sizeof(unsigned_values) / sizeof(unsigned_values[0]); ++i) {
        (void)snprintf(want, sizeof(want), "%" PRIu64, unsigned_values[i]);
        expect(sce_text_of_u64(unsigned_values[i], got, sizeof(got)) == strlen(want) && strcmp(got, want) == 0,
               "the digits of a uint64 are printf's");
    }
    static const int64_t signed_values[] = {0, -1, 1, INT32_MIN, INT32_MAX, INT64_MIN, INT64_MAX};
    for (size_t i = 0u; i < sizeof(signed_values) / sizeof(signed_values[0]); ++i) {
        (void)snprintf(want, sizeof(want), "%" PRId64, signed_values[i]);
        expect(sce_text_of_i64(signed_values[i], got, sizeof(got)) == strlen(want) && strcmp(got, want) == 0,
               "the digits of an int64 are printf's, its sign leading");
    }
    expect(sce_text_of_u32(UINT32_MAX, got, sizeof(got)) == 10u && strcmp(got, "4294967295") == 0,
           "the digits of the largest uint32");
    expect(sce_text_of_i32(INT32_MIN, got, sizeof(got)) == 11u && strcmp(got, "-2147483648") == 0,
           "the digits of the least int32, whose magnitude no int32 holds");
    expect(sce_text_of_u32(0u, got, sizeof(got)) == 1u && strcmp(got, "0") == 0, "zero is one digit");
    // A buffer one byte short of the text and its NUL holds the empty string.
    char short_buf[3] = {'x', 'x', 'x'};
    expect(sce_text_of_i64(-12, short_buf, sizeof(short_buf)) == 0u && short_buf[0] == '\0',
           "a text that does not fit is refused whole, never cut");
    expect(sce_text_of_i64(-1, short_buf, sizeof(short_buf)) == 2u && strcmp(short_buf, "-1") == 0,
           "a text that fits exactly is written");
    expect(sce_text_of_u64(1u, got, 0u) == 0u, "a buffer of no bytes holds nothing");
}

static void a_writer_keeps_what_fits_as_snprintf_does(void) {
    char buf[16];
    char want[16];
    sce_text_writer_t w = sce_text_writer(buf, sizeof(buf));
    sce_text_put_text(&w, "id.");
    sce_text_put_u64(&w, 42u);
    sce_text_put_char(&w, '.');
    sce_text_put_i64(&w, -5);
    (void)snprintf(want, sizeof(want), "id.%u.%d", 42u, -5);
    expect(!w.cut && strcmp(buf, want) == 0 && w.len == strlen(want), "a text that fits is written whole");

    // What `snprintf` keeps of a text one byte too long for eight bytes: the
    // first seven and the NUL.
    char eight[8];
    w = sce_text_writer(eight, sizeof(eight));
    sce_text_put_text(&w, "done.inv");
    expect(w.cut && strcmp(eight, "done.in") == 0, "a text that does not fit is cut as snprintf cuts it, and says so");

    w = sce_text_writer(buf, sizeof(buf));
    sce_text_put_text(&w, "\\u");
    sce_text_put_hex(&w, 0x1fu, 4u);
    (void)snprintf(want, sizeof(want), "\\u%04x", 0x1fu);
    expect(!w.cut && strcmp(buf, want) == 0, "four hexadecimal digits, zeros leading, as %04x writes them");

    char wide[24];
    w = sce_text_writer(wide, sizeof(wide));
    sce_text_put_hex(&w, UINT64_C(0xdeadbeefcafe), 1u);
    expect(strcmp(wide, "deadbeefcafe") == 0, "a value wider than the width is written whole");

    w = sce_text_writer(NULL, 0u);
    sce_text_put_text(&w, "x");
    expect(w.cut && w.len == 0u, "a writer over no buffer keeps nothing and says it cut");
}

// A text written within a bound is what `%.<n>s` writes of it: at most the
// bound, and no more than the text holds.
static void a_text_within_a_bound_is_what_a_precision_writes(void) {
    static const char *const texts[] = {"", "ab", "abcdef"};
    for (size_t i = 0u; i < sizeof(texts) / sizeof(texts[0]); ++i) {
        for (size_t max = 0u; max < 8u; ++max) {
            char buf[16];
            char want[16];
            sce_text_writer_t w = sce_text_writer(buf, sizeof(buf));
            sce_text_put_text(&w, "[");
            sce_text_put_within(&w, texts[i], max);
            sce_text_put_text(&w, "]");
            (void)snprintf(want, sizeof(want), "[%.*s]", (int)max, texts[i]);
            expect(!w.cut && strcmp(buf, want) == 0, "a text within a bound is what %.*s writes");
        }
    }
}

int main(void) {
    the_comparisons_agree_with_the_hosted_ones();
    the_last_place_and_a_byte_are_found();
    the_text_of_an_integer_is_printfs();
    a_writer_keeps_what_fits_as_snprintf_does();
    a_text_within_a_bound_is_what_a_precision_writes();
    if (failures != 0) {
        (void)fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    (void)printf("text: every routine answers as the hosted one it stands in for\n");
    return 0;
}
