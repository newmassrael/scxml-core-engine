// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

/**
 * @brief C++17/C++20 compatibility shim for branch prediction hints
 *
 * On C++20 compilers, SCE_LIKELY/SCE_UNLIKELY expand to [[likely]]/[[unlikely]].
 * On C++17 compilers, they expand to nothing, allowing SCE to compile without
 * requiring C++20 attribute support.
 */

#if __cplusplus >= 202002L
  #define SCE_LIKELY [[likely]]
  #define SCE_UNLIKELY [[unlikely]]
#else
  #define SCE_LIKELY
  #define SCE_UNLIKELY
#endif
