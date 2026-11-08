/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file verdict.h
 * @brief The verdicts of the test profile (docs/SCE_TEST_PROFILE.md §5)
 *
 * The four a document sets with `<sce:setverdict>` (P12) and `error`, which only
 * the runtime sets (P13), in the order ETSI ES 201 873-1 §24.1 gives them. The
 * C11 port of `sce/include/test/Verdict.h` and `sce_rust_runtime::test::Verdict`.
 * Header-only and allocation-free.
 */

#ifndef SCE_TEST_VERDICT_H
#define SCE_TEST_VERDICT_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * A test verdict, declared in the order `none < pass < inconclusive < fail < error`.
 *
 * The order is the one ES 201 873-1 §24.1 overwrites by: a verdict only ever
 * becomes worse, so the order of the enumerators is the order.
 */
typedef enum sce_test_verdict_e {
    SCE_TEST_VERDICT_NONE = 0,
    SCE_TEST_VERDICT_PASS,
    SCE_TEST_VERDICT_INCONCLUSIVE,
    SCE_TEST_VERDICT_FAIL,
    SCE_TEST_VERDICT_ERROR
} sce_test_verdict_t;

/** The worse of `a` and `b`: what `setverdict` leaves a local verdict at (P12,
 *  -4 §9.45), and how the test case verdict folds the local ones (P14). */
static inline sce_test_verdict_t sce_test_worse(sce_test_verdict_t a, sce_test_verdict_t b) {
    return a < b ? b : a;
}

/** The verdict's spelling in the test log and in TTCN-3. */
static inline const char *sce_test_verdict_name(sce_test_verdict_t verdict) {
    switch (verdict) {
    case SCE_TEST_VERDICT_NONE:
        return "none";
    case SCE_TEST_VERDICT_PASS:
        return "pass";
    case SCE_TEST_VERDICT_INCONCLUSIVE:
        return "inconc";
    case SCE_TEST_VERDICT_FAIL:
        return "fail";
    case SCE_TEST_VERDICT_ERROR:
        return "error";
    }
    return "error";
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_VERDICT_H */
