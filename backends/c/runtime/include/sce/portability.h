/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael */
/*
 * SCE C runtime — spellings that differ between C and C++.
 *
 * Every public header under `sce/`, and every header the C11 backend
 * generates, wraps its declarations in `#ifdef __cplusplus extern "C"`.
 * That wrapper is a contract: the same generated state machine an MCU
 * compiles as C11 is linkable from a C++ application on the AP side.
 * A contract of that shape is worth exactly what the compiler enforces,
 * and a single C-only keyword anywhere in the header voids it for the
 * whole file — `extern "C"` only fixes linkage, it does not make C
 * syntax parse as C++.
 *
 * `_Static_assert` is that keyword. C++ spells the same declaration
 * `static_assert`, so a header writing the C form stops compiling the
 * moment a C++ translation unit includes it, and it fails with a parse
 * error at the assertion rather than a diagnostic naming the real
 * problem. Deleting the assertions would trade a loud failure for a
 * silent one: the layout invariants they pin (slot-state discriminants,
 * bitmap widths, handle offsets) are what keep a generated pool header
 * and its consumers agreeing on wire layout.
 *
 * So the spelling is centralised here instead. `SCE_STATIC_ASSERT` is
 * correct in both languages, which means the invariants stay checked on
 * every consumer rather than only on the ones that speak C.
 *
 * This header includes nothing. It is safe in freestanding mode and
 * therefore usable from both `sce_c_runtime` and `sce_forge_runtime_c`,
 * whose policy (SCE_FORGE.md 2.1) admits only freestanding-safe headers.
 *
 * Comments here are block comments rather than the `//` form its
 * siblings use. Unlike them, this header carries a pre-C99 lowering
 * below, and a `//` comment would make the header itself unparseable
 * under `-std=c89 -Wpedantic`, leaving that lowering unreachable in
 * exactly the configuration it exists to serve.
 */

#ifndef SCE_PORTABILITY_H
#define SCE_PORTABILITY_H

/*
 * Compile-time assertion, spelled for whichever language is compiling.
 *
 * Three lowerings, in the order the preprocessor should prefer them:
 *
 *   * C++ (any standard from C++11): `static_assert`. C++17 and later
 *     also admit the single-argument form, but the two-argument form
 *     compiles under every C++ standard SCE targets and keeps the
 *     message text, so it is the one used.
 *
 *   * C11 and later: `_Static_assert`, the keyword this macro exists
 *     to stop headers from writing directly.
 *
 *   * Anything older: no keyword exists, so the check lowers to a
 *     negative-array-size typedef. Every conforming C compiler must
 *     diagnose an array declared with a negative bound, so the
 *     invariant is still verified rather than quietly dropped. What is
 *     lost is the message text, which no pre-C11 lowering can carry;
 *     the typedef name embeds the source line so the diagnostic still
 *     points at the assertion that failed.
 *
 * The last arm is a safety net, not a support claim: `sce_c_runtime`
 * and `sce_forge_runtime_c` both declare `c_std_11`. It exists because
 * an invariant that silently stops being checked is worse than one that
 * was never written — a consumer compiling this header on an older
 * toolchain gets a hard error at the drifted assertion instead of a
 * build that passes while the layout contract has already broken.
 */
#if defined(__cplusplus)
#define SCE_STATIC_ASSERT(cond, msg) static_assert(cond, msg)
#elif defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
#define SCE_STATIC_ASSERT(cond, msg) _Static_assert(cond, msg)
#else
/* Two levels of indirection so `__LINE__` expands before pasting. */
#define SCE_STATIC_ASSERT_PASTE_(a, b) a##b
#define SCE_STATIC_ASSERT_PASTE(a, b) SCE_STATIC_ASSERT_PASTE_(a, b)
#define SCE_STATIC_ASSERT(cond, msg)                                                                                   \
    typedef char SCE_STATIC_ASSERT_PASTE(sce_static_assert_line_, __LINE__)[(cond) ? 1 : -1]
#endif

#endif /* SCE_PORTABILITY_H */
