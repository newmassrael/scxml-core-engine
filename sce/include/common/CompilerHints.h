// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

/**
 * @brief Branch prediction hints, written around a condition
 *
 * `if (SCE_EXPECT_TRUE(cond)) {` marks the branch the condition usually takes,
 * and `if (SCE_EXPECT_FALSE(cond)) {` the one it usually does not. Each wraps
 * the condition in an expression, so it reads the same in a C++17 and a C++20
 * translation unit (docs/adr/0059); the choice of expansion is by compiler,
 * which is the same for every translation unit of one program. A compiler with
 * no `__builtin_expect` gets the condition unchanged.
 *
 * Variadic so a condition with a comma outside parentheses (a template
 * argument list, a braced initializer) is passed whole.
 */

#if defined(__GNUC__) || defined(__clang__)
#define SCE_EXPECT_TRUE(...) __builtin_expect(!!(__VA_ARGS__), 1)
#define SCE_EXPECT_FALSE(...) __builtin_expect(!!(__VA_ARGS__), 0)
#else
#define SCE_EXPECT_TRUE(...) (__VA_ARGS__)
#define SCE_EXPECT_FALSE(...) (__VA_ARGS__)
#endif
