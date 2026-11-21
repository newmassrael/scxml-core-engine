// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The expression language's `round(x)` and `floor(x)` as a C11 machine's core
// and the forge kinds it calls compute them (`sce/forge/algorithm.h`,
// docs/adr/0110 D1), held to C99's `llround` and `floor` for every value an
// `int64_t` holds, and to a saturating conversion where C99 leaves the answer
// unspecified. The hosted library is the oracle here and nowhere in the core.

#include <sce/forge/algorithm.h>

#include <float.h>
#include <math.h>
#include <stdio.h>

static int failures = 0;

static void expect(int ok, const char *what, double x) {
    if (!ok) {
        (void)fprintf(stderr, "FAIL: %s, at %.17g\n", what, x);
        ++failures;
    }
}

static void expect_c99(double x) {
    expect(sce_forge_real_round(x) == (int64_t)llround(x), "round is llround", x);
    expect(sce_forge_real_floor(x) == (int64_t)floor(x), "floor is floor", x);
}

// The halves, which `floor(x + 0.5)` gets wrong on both sides of zero, the
// largest double below one half, the smallest magnitudes, and both sides of 2^52,
// past which every double is whole.
static void a_whole_part_is_c99s(void) {
    static const double values[] = {
        0.0,
        -0.0,
        0.5,
        -0.5,
        1.5,
        -1.5,
        2.5,
        -2.5,
        0.49999999999999994,
        -0.49999999999999994,
        DBL_TRUE_MIN,
        -DBL_TRUE_MIN,
        1.0 - DBL_EPSILON / 2.0,
        -(1.0 - DBL_EPSILON / 2.0),
        3.7,
        -3.7,
        1e15 + 0.5,
        -(1e15 + 0.5),
        4503599627370495.5,
        -4503599627370495.5,
        4503599627370496.0,
        -4503599627370496.0,
        4503599627370497.0,
        -4503599627370497.0,
        9223372036854774784.0,
        -9223372036854775808.0,
    };
    for (size_t i = 0u; i < sizeof(values) / sizeof(values[0]); ++i) {
        expect_c99(values[i]);
    }
    for (int k = -4000; k <= 4000; ++k) {
        expect_c99((double)k * 0.125);
        expect_c99((double)k * 0.1);
    }
}

static void past_int64_the_answer_saturates(void) {
    expect(sce_forge_real_round(NAN) == 0 && sce_forge_real_floor(NAN) == 0, "NaN is 0", NAN);
    expect(sce_forge_real_round(INFINITY) == INT64_MAX && sce_forge_real_floor(INFINITY) == INT64_MAX,
           "an infinity is the upper end", INFINITY);
    expect(sce_forge_real_round(-INFINITY) == INT64_MIN && sce_forge_real_floor(-INFINITY) == INT64_MIN,
           "a negative infinity is the lower end", -INFINITY);
    expect(sce_forge_real_round(9223372036854775808.0) == INT64_MAX, "2^63 is the upper end", 9223372036854775808.0);
    expect(sce_forge_real_floor(-1e19) == INT64_MIN, "a value below -2^63 is the lower end", -1e19);
}

int main(void) {
    a_whole_part_is_c99s();
    past_int64_the_answer_saturates();
    if (failures != 0) {
        (void)fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    (void)printf("forge real: round and floor answer as C99's, saturating past int64_t\n");
    return 0;
}
