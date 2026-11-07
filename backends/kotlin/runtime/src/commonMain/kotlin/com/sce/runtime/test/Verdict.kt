// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The verdicts of the test profile (docs/SCE_TEST_PROFILE.md §5): the four a
// document sets with `<sce:setverdict>` (P12) and `error`, which only the
// runtime sets (P13), in the order ETSI ES 201 873-1 §24.1 gives them.
//
// Cross-language siblings: `sce_rust_runtime::test::Verdict` (Rust),
// `SCE::Test::Verdict` (C++).

package com.sce.runtime.test

/**
 * A test verdict, declared in the order `none < pass < inconclusive < fail < error`.
 *
 * The order is the one ES 201 873-1 §24.1 overwrites by: a verdict only ever
 * becomes worse, so the declared order of the enumeration is the order.
 *
 * [spelling] is the verdict's spelling in the test log and in TTCN-3. It is not
 * `name`, as the other runtimes call it, because a Kotlin enumeration's `name`
 * is the constant's own and cannot be given another value.
 */
enum class Verdict(val spelling: String) {
    /** `none`: no verdict was set. */
    NONE("none"),

    /** `pass`. */
    PASS("pass"),

    /** `inconc`. */
    INCONCLUSIVE("inconc"),

    /** `fail`. */
    FAIL("fail"),

    /** `error`: set only by the runtime (P13). */
    ERROR("error"),
}

/**
 * The worse of [a] and [b]: what `setverdict` leaves a local verdict at
 * (P12, -4 §9.45), and how the test case verdict folds the local ones (P14).
 */
fun worse(a: Verdict, b: Verdict): Verdict = maxOf(a, b)
