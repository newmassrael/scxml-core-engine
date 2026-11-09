// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The canonical form of a message's value in the test log
// (docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091, C3 and answer 8): one JSON
// value per value of the profile's types, built from texts, truth values,
// arrays and objects alone, so that the JSON Canonicalization Scheme
// (SCE::Saved::writeCanonical) writes two equal values as the same bytes on
// every runtime.
//
// - an integer of any width is the text of its decimal digits, since an
//   `int64` or a `uint64` is past what a JSON reader that holds numbers as
//   doubles reads exactly;
// - a `bool` is a truth value;
// - a value of an enum is the name its document declares for its variant;
// - a text is a text;
// - a byte string is the text of its octets in upper-case hex, two digits each;
// - a record is an object of its fields by their declared ids, a field that is
//   absent omitted, as TTCN-3's `omit` leaves it out;
// - a list is an array of its elements in order.
//
// The generated unit of a test template over a record spells its `canonical`
// with these, by the types its document declares
// (`forge/cpp/test_template.h.jinja2`): a C++ type alone does not say which
// form a value takes, since a byte string and a list of `uint8` are one C++
// type. C++ lowers no union, so no form of one is written here. The Rust
// sibling is `sce_rust_runtime::test::canonical`.

#pragma once

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <vector>

#include "saved/SavedJson.h"

namespace SCE::Test::Canonical {

/// A value in the canonical form: the JSON value model a saved state is made
/// of, written by SCE::Saved::writeCanonical.
using Value = SCE::Saved::JsonValue;

/// An integer of any width the profile has: the text of its decimal digits.
template <typename Integer, std::enable_if_t<std::is_integral_v<Integer> && !std::is_same_v<Integer, bool>, int> = 0>
Value integer(Integer value) {
    if constexpr (std::is_signed_v<Integer>) {
        return Value(std::to_string(static_cast<long long>(value)));
    } else {
        return Value(std::to_string(static_cast<unsigned long long>(value)));
    }
}

/// A `bool`.
inline Value boolean(bool value) {
    return Value(value);
}

/// A text.
inline Value text(std::string_view value) {
    return Value(std::string(value));
}

/// A byte string: its octets in upper-case hex, two digits each, as TTCN-3
/// writes an `octetstring` (`'00FF'O`) without its quotes and its `O`.
template <typename Octets> Value bytes(const Octets &octets) {
    static constexpr char HEX[] = "0123456789ABCDEF";
    std::string out;
    for (const auto octet : octets) {
        const auto byte = static_cast<std::uint8_t>(octet);
        out += HEX[byte >> 4];
        out += HEX[byte & 0x0Fu];
    }
    return Value(std::move(out));
}

/// A value of a closed enum: the name its document declares for the variant
/// (the unit's `declared_name`). A closed enum holds only its declared
/// numbers, so a value no variant names is not a value of its type, and is
/// refused with std::invalid_argument rather than written.
inline Value enumerated(const char *declared) {
    if (declared == nullptr) {
        throw std::invalid_argument("a value of a closed enum that no variant declares has no canonical form");
    }
    return text(declared);
}

/// A value of an enum of an open set (`sce:strict-variants="false"`): the name
/// its document declares for it, or, for a number no variant declares
/// (`declared_name` gives none), the text of that number's decimal digits. No
/// variant's name is such a text, since a name is an identifier. The TTCN-3
/// export refuses an enum of an open set (docs/SCE_TEST_PROFILE.md §9.5), so
/// only a native log holds this form.
template <typename Integer> Value openEnumerated(const char *declared, Integer number) {
    if (declared != nullptr) {
        return text(declared);
    }
    return integer(number);
}

/// A field of a record: its declared id, and its value in the canonical form,
/// or none where the field is absent.
struct Field {
    std::string id;
    std::optional<Value> value;
};

/// A record: each field by its declared id, in any order, since the scheme
/// sorts an object's members; a field given no value is absent and omitted.
inline Value record(const std::vector<Field> &fields) {
    SCE::Saved::JsonObject members;
    for (const Field &field : fields) {
        if (field.value.has_value()) {
            members.set(field.id, *field.value);
        }
    }
    return Value(std::move(members));
}

/// A list: each of @p elements, in order, in the form @p write gives it.
template <typename Elements, typename Write> Value list(const Elements &elements, Write write) {
    SCE::Saved::JsonArray items;
    for (const auto &element : elements) {
        items.push_back(write(element));
    }
    return Value(std::move(items));
}

/// A field that may be absent: the value @p held holds, in the form @p write
/// gives it, or none where it holds none, which `record` omits.
template <typename Held, typename Write> std::optional<Value> optional(const std::optional<Held> &held, Write write) {
    if (!held.has_value()) {
        return std::nullopt;
    }
    return write(*held);
}

}  // namespace SCE::Test::Canonical
