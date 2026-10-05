// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A `sce-static` machine joins strings by writing them into a buffer the
// generator sizes from the bounds the data model declares (docs/SCE_ACCEPTED_SUBSET.md
// §2.15): `SCE_FORGE_CONCAT` writes each part as the wire text of its value, a
// string as itself and an integer as its decimal digits, and returns the buffer.
// The cases below are the ones the sizing relies on: the widest text of each
// integer type fits the bound the generator computes, a join never ends in a
// truncated text, and a part is read where it stands.

#include <sce/forge/wire.h>

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static int expect_text(const char *what, const char *got, const char *want) {
    if (strcmp(got, want) != 0) {
        (void)fprintf(stderr, "%s: wrote `%s`, expected `%s`\n", what, got, want);
        return 1;
    }
    return 0;
}

// The bound the generator computes for the decimal text of a type of `bits`
// bits, so the widest value of each is held to it: the digits of the largest
// magnitude, and a sign for a signed one.
static size_t unsigned_bound(size_t bits) {
    return bits * 30103u / 100000u + 1u;
}

static size_t signed_bound(size_t bits) {
    return unsigned_bound(bits - 1u) + 1u;
}

int main(void) {
    int bad = 0;

    // A string and an integer, as `wait + 'ms'` joins them.
    uint32_t wait = 100u;
    bad |= expect_text("number and unit",
                       SCE_FORGE_CONCAT((char[13]){0}, 13, sce_forge_wire_uint(wait), sce_forge_wire_string("ms")),
                       "100ms");

    // A chain is one join, in order.
    bad |= expect_text("a chain",
                       SCE_FORGE_CONCAT((char[13]){0}, 13, sce_forge_wire_string("a"), sce_forge_wire_uint(wait),
                                        sce_forge_wire_string("b")),
                       "a100b");

    // The widest text of each width fits the bound the generator sizes for it.
    bad |= expect_text("the widest uint32", SCE_FORGE_CONCAT((char[11]){0}, 11, sce_forge_wire_uint(UINT32_MAX)),
                       "4294967295");
    bad |= expect_text("the widest uint64", SCE_FORGE_CONCAT((char[21]){0}, 21, sce_forge_wire_uint(UINT64_MAX)),
                       "18446744073709551615");
    bad |= expect_text("the narrowest int64", SCE_FORGE_CONCAT((char[21]){0}, 21, sce_forge_wire_int(INT64_MIN)),
                       "-9223372036854775808");
    bad |= expect_text("the narrowest int8", SCE_FORGE_CONCAT((char[5]){0}, 5, sce_forge_wire_int(INT8_MIN)), "-128");
    if (unsigned_bound(8u) != 3u || unsigned_bound(16u) != 5u || unsigned_bound(32u) != 10u ||
        unsigned_bound(64u) != 20u || signed_bound(8u) != 4u || signed_bound(16u) != 6u || signed_bound(32u) != 11u ||
        signed_bound(64u) != 20u) {
        (void)fprintf(stderr, "the digit bound the generator computes moved\n");
        bad = 1;
    }

    // Nothing is joined that does not fit: the buffer holds the empty string, never
    // a truncated one. A generator that sizes the buffer from the bounds cannot
    // reach this, and it is what keeps a miscounted bound from reading as a value.
    bad |= expect_text("a part that does not fit",
                       SCE_FORGE_CONCAT((char[4]){0}, 4, sce_forge_wire_string("ab"), sce_forge_wire_string("cd")), "");

    // Parts are read where they stand: an expression with a side effect is
    // evaluated once.
    int calls = 0;
    const char *text = SCE_FORGE_CONCAT((char[8]){0}, 8, sce_forge_wire_string("x"), sce_forge_wire_int(++calls));
    bad |= expect_text("a part is evaluated once", text, "x1");
    if (calls != 1) {
        (void)fprintf(stderr, "a part was evaluated %d times\n", calls);
        bad = 1;
    }

    if (bad != 0) {
        return 1;
    }
    (void)printf("forge concat: ok\n");
    return 0;
}
