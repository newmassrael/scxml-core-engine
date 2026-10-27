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

#include <cstdint>

namespace SCE {

/**
 * @brief The place in the source a log call was written
 *
 * One SCE type in every language standard, never an alias of
 * `std::source_location`: `ILoggerBackend::log` takes it, so a consumer's
 * C++17 override and SCE's C++20 caller must name the same type, or the
 * virtual they share would mangle two ways (docs/adr/0059).
 *
 * `current()` is filled from the compiler builtins `std::source_location`
 * itself is built on. As a default argument they are evaluated where the
 * call that relies on the default is written, so `current()` named as a
 * default argument reports its caller's location. GCC, Clang and MSVC
 * (VS 2019 16.6 and later) provide these builtins at C++17.
 *
 * `column()` is always 0: GCC has no column builtin, and a value one compiler
 * fills and another leaves out would make the field mean different things.
 */
struct source_location {
    static constexpr source_location current(const char *file = __builtin_FILE(),
                                             const char *function = __builtin_FUNCTION(),
                                             std::uint_least32_t line = __builtin_LINE()) noexcept {
        source_location location;
        location.file_ = file;
        location.function_ = function;
        location.line_ = line;
        return location;
    }

    constexpr source_location() noexcept = default;

    constexpr const char *file_name() const noexcept {
        return file_;
    }

    constexpr const char *function_name() const noexcept {
        return function_;
    }

    constexpr std::uint_least32_t line() const noexcept {
        return line_;
    }

    constexpr std::uint_least32_t column() const noexcept {
        return 0;
    }

private:
    const char *file_ = "";
    const char *function_ = "";
    std::uint_least32_t line_ = 0;
};

}  // namespace SCE
