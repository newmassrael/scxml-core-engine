// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce_forge_runtime — optional
// A value that may be absent (docs/adr/0055) is a `std::optional<T>` on this
// backend: absent is `std::nullopt`, a present value is the `T` it holds, a
// test is `has_value()`, a read the build has proven present is `*x`, and
// `x === y` is the `==` the standard library defines over two optionals, which
// is true of two absent values and of two present values that are equal. What
// the language says beyond those four is here, one helper each:
//
// - `otherwise`: `x ?? d`, which evaluates `d` only when `x` is absent;
// - `mapPresent`: what a present value becomes, an absent one staying absent,
//   which holds a present text to the bound of the place it is written to;
// - `presentText`: a present value as a `<log>` shows it.

#pragma once

#include <charconv>
#include <optional>
#include <string>
#include <type_traits>
#include <utility>

namespace SCE::Forge {
namespace Detail {

/// Whether `T` is a `std::optional` of something.
template <typename T> struct IsOptional : std::false_type {};

template <typename T> struct IsOptional<std::optional<T>> : std::true_type {};

}  // namespace Detail

/// `value ?? fallback` (docs/adr/0055): the value `value` holds when it is
/// present, and what `fallback` answers when it is absent. `value` is read once,
/// and `fallback` is called only when `value` is absent, so a failure it would
/// record (an overflow of a checked operation it computes) is recorded only
/// then. A fallback of `T` answers a `T`; a fallback that may itself be absent
/// answers an optional value, absent when both are.
template <typename T, typename F> auto otherwise(const std::optional<T> &value, F &&fallback) {
    using Fallback = std::decay_t<std::invoke_result_t<F>>;
    if constexpr (Detail::IsOptional<Fallback>::value || std::is_same_v<Fallback, std::nullopt_t>) {
        if (value.has_value()) {
            return std::optional<T>(value);
        }
        return std::optional<T>(std::forward<F>(fallback)());
    } else {
        if (value.has_value()) {
            return T(*value);
        }
        return static_cast<T>(std::forward<F>(fallback)());
    }
}

/// What `map` makes of the value `value` holds, as an optional value: absent
/// where `value` is absent, and `map` is not called then, so a failure it would
/// record (a text past the bound of the place it is written to) is recorded only
/// for a present value.
template <typename T, typename F>
auto mapPresent(const std::optional<T> &value, F &&map)
    -> std::optional<std::decay_t<std::invoke_result_t<F, const T &>>> {
    if (!value.has_value()) {
        return std::nullopt;
    }
    return std::forward<F>(map)(*value);
}

/// A present value of an optional as a `<log>` shows it: a `bool` as `true` or
/// `false`, a number as the shortest text that reads back as it, and a text,
/// or the name an enum's `sceLogName` gives its value, as itself. An absent
/// value is shown as `null` by the caller, which knows it is absent.
template <typename V> std::string presentText(const V &value) {
    if constexpr (std::is_same_v<V, bool>) {
        return value ? "true" : "false";
    } else if constexpr (std::is_arithmetic_v<V>) {
        char text[64];
        const auto written = std::to_chars(text, text + sizeof(text), value);
        return std::string(text, written.ptr);
    } else {
        return std::string(value);
    }
}

}  // namespace SCE::Forge
