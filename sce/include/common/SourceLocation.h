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
 * @brief C++17/C++20 compatibility shim for std::source_location
 *
 * On C++20 compilers with library support, this aliases std::source_location.
 * On C++17 compilers, this provides a lightweight stub with the same API
 * that returns empty/zero values, allowing SCE to compile without requiring
 * C++20 standard library features.
 */

#if __has_include(<source_location>) && __cplusplus >= 202002L
#include <source_location>

namespace SCE {
using source_location = std::source_location;
}
#else
#include <cstdint>

namespace SCE {
struct source_location {
    static constexpr source_location current() noexcept {
        return {};
    }

    constexpr const char *file_name() const noexcept {
        return "";
    }

    constexpr const char *function_name() const noexcept {
        return "";
    }

    constexpr uint_least32_t line() const noexcept {
        return 0;
    }

    constexpr uint_least32_t column() const noexcept {
        return 0;
    }
};
}  // namespace SCE
#endif
