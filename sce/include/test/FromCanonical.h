// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A value read back from the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5; docs/SCE_TTCN3_LANE.md §10.3): the inverse
// of test/Canonical.h, which a recorded input trace's values are read with
// (`schemas/sce-test-trace.v1.schema.json`).
//
// Each reader takes one form and accepts its one spelling, so a value read
// here and written back by test/Canonical.h is the same bytes:
//
// - an integer is the text of its decimal digits, with no sign but a minus,
//   no leading zero and no `-0`, within the range of the type it is read as;
// - a `bool` is a truth value;
// - a value of an enum is the name its document declares for the variant, or,
//   for an enum of an open set, the text of a number no variant declares;
// - a text is a text of at most its bound in UTF-8 bytes;
// - a byte string is its octets in upper-case hex, two digits each;
// - a record is an object of its fields by their declared ids, an absent
//   field omitted, no field it does not declare;
// - a list is an array of its elements within its bounds, an absent element
//   of a list of `optional<T>` `null`.
//
// A value that does not belong to its type is refused with TraceRefusal,
// naming where it stands (`what`) and why, never converted. Which form a value
// takes is decided by the types its document declares and not by its C++
// type, since a byte string and a list of `uint8` are one C++ type: the
// generated reader of a record (`read<Record>` in the unit of a test
// configuration) calls these by the walk its canonical writer is spelled by
// (`forge::generator::test_template_walk::canonical_form`), and names the type
// each value is read into. C++'s test profile matches no union yet
// (`codegen_matrix::TEST_PROFILE_UNION_BACKENDS`), so no form of one is read
// here. The Rust sibling is `sce_rust_runtime::test::from_canonical`, whose
// refusals these are, word for word. An object that names a member twice,
// which the Rust reader refuses, is not one a SCE::Saved::JsonObject can hold.

#pragma once

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <initializer_list>
#include <limits>
#include <optional>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <vector>

#include "saved/SavedJson.h"
#include "sce/forge/bounded.h"
#include "sce/forge/utf8.h"
#include "test/TraceRefusal.h"

namespace SCE::Test::FromCanonical {

/// A value in the canonical form, as the trace's reader holds it.
using Value = SCE::Saved::JsonValue;

/// An integer as the canonical form spells one: its sign and the decimal
/// digits of its magnitude, with no leading zero.
struct Decimal {
    bool negative = false;
    std::string magnitude;
};

namespace detail {

/// The refusal of the value at @p what, for @p why.
inline TraceRefusal refused(const std::string &what, const std::string &why) {
    return TraceRefusal(what + ": " + why);
}

/// The kind of a JSON value, as a refusal names it.
inline const char *kind(const Value &value) {
    if (value.isNull()) {
        return "null";
    }
    if (value.isBool()) {
        return "a truth value";
    }
    if (value.isNumber()) {
        return "a number";
    }
    if (value.isString()) {
        return "a text";
    }
    if (value.isArray()) {
        return "an array";
    }
    return "an object";
}

/// The refusal of @p value, at @p what, which is not of the form @p form.
inline TraceRefusal notA(const Value &value, const std::string &what, const std::string &form) {
    return refused(what, std::string(kind(value)) + " is not " + form);
}

/// The magnitude of the least and of the greatest 128-bit integer: the range
/// the Rust reader reads an integer's text in before it holds it to its type,
/// so a text of more digits is refused as no integer's on both.
inline const std::string &leastMagnitude() {
    static const std::string magnitude = "170141183460469231731687303715884105728";
    return magnitude;
}

inline const std::string &greatestMagnitude() {
    static const std::string magnitude = "170141183460469231731687303715884105727";
    return magnitude;
}

/// The value of one upper-case hex digit, or none.
inline std::optional<std::uint8_t> hexDigit(char digit) {
    if (digit >= '0' && digit <= '9') {
        return static_cast<std::uint8_t>(digit - '0');
    }
    if (digit >= 'A' && digit <= 'F') {
        return static_cast<std::uint8_t>(digit - 'A' + 10);
    }
    return std::nullopt;
}

/// The octets @p text spells in upper-case hex, two digits each, or none.
inline std::optional<std::vector<std::uint8_t>> upperHex(const std::string &text) {
    if (text.size() % 2 != 0) {
        return std::nullopt;
    }
    std::vector<std::uint8_t> octets;
    octets.reserve(text.size() / 2);
    for (std::size_t at = 0; at < text.size(); at += 2) {
        const auto high = hexDigit(text[at]);
        const auto low = hexDigit(text[at + 1]);
        if (!high || !low) {
            return std::nullopt;
        }
        octets.push_back(static_cast<std::uint8_t>((*high << 4) | *low));
    }
    return octets;
}

/// @p text as the engine's own text, which holds any.
inline std::optional<std::string> heldText(const std::string &text, std::string *) {
    return text;
}

/// @p text as a bounded text, where it is within the bounds.
template <std::size_t Max, std::size_t Min>
std::optional<SCE::Forge::OwnedText<Max, Min>> heldText(const std::string &text, SCE::Forge::OwnedText<Max, Min> *) {
    return SCE::Forge::OwnedText<Max, Min>::make(text);
}

/// @p octets as the engine's own byte string, which holds any.
inline std::optional<std::vector<std::uint8_t>> heldOctets(std::vector<std::uint8_t> octets,
                                                           std::vector<std::uint8_t> *) {
    return octets;
}

/// @p octets as a bounded byte string, where they are within the bounds.
template <std::size_t Max, std::size_t Min>
std::optional<SCE::Forge::OwnedBytes<Max, Min>> heldOctets(std::vector<std::uint8_t> octets,
                                                           SCE::Forge::OwnedBytes<Max, Min> *) {
    return SCE::Forge::OwnedBytes<Max, Min>::make(std::move(octets));
}

}  // namespace detail

/// The integer @p digits spells as the canonical form writes one: an optional
/// minus and decimal digits with no leading zero, never `-0`, within the range
/// of a 128-bit integer. None for any other text.
inline std::optional<Decimal> decimal(const std::string &digits) {
    Decimal read;
    std::size_t at = 0;
    if (!digits.empty() && digits[0] == '-') {
        read.negative = true;
        at = 1;
    }
    read.magnitude = digits.substr(at);
    const std::string &magnitude = read.magnitude;
    if (magnitude.empty()) {
        return std::nullopt;
    }
    for (const char digit : magnitude) {
        if (digit < '0' || digit > '9') {
            return std::nullopt;
        }
    }
    if (magnitude.size() > 1 && magnitude[0] == '0') {
        return std::nullopt;
    }
    if (read.negative && magnitude == "0") {
        return std::nullopt;
    }
    const std::string &limit = read.negative ? detail::leastMagnitude() : detail::greatestMagnitude();
    if (magnitude.size() > limit.size() || (magnitude.size() == limit.size() && magnitude > limit)) {
        return std::nullopt;
    }
    return read;
}

/// @p number as a value of the integer type `Integer`, or none where it is
/// past that type's range.
template <typename Integer> std::optional<Integer> held(const Decimal &number) {
    static_assert(std::is_integral_v<Integer> && !std::is_same_v<Integer, bool>,
                  "an integer of the profile is read into an integer type");
    constexpr unsigned long long WIDEST = std::numeric_limits<unsigned long long>::max();
    unsigned long long count = 0;
    for (const char digit : number.magnitude) {
        const auto value = static_cast<unsigned long long>(digit - '0');
        if (count > (WIDEST - value) / 10) {
            return std::nullopt;
        }
        count = count * 10 + value;
    }
    if (number.negative) {
        if constexpr (std::is_unsigned_v<Integer>) {
            return std::nullopt;
        } else {
            const auto most = static_cast<unsigned long long>(std::numeric_limits<Integer>::max()) + 1ULL;
            if (count > most) {
                return std::nullopt;
            }
            if (count == most) {
                return std::numeric_limits<Integer>::min();
            }
            return static_cast<Integer>(-static_cast<long long>(count));
        }
    }
    if (count > static_cast<unsigned long long>(std::numeric_limits<Integer>::max())) {
        return std::nullopt;
    }
    return static_cast<Integer>(count);
}

/// An integer of the type `Integer`: the text of its decimal digits.
template <typename Integer> Integer integer(const Value &value, const std::string &what) {
    if (!value.isString()) {
        throw detail::notA(value, what, "an integer, which is the text of its decimal digits");
    }
    const std::string &digits = value.asString();
    const auto number = decimal(digits);
    if (!number) {
        throw detail::refused(what, "'" + digits + "' is not the text of an integer's decimal digits");
    }
    const auto read = held<Integer>(*number);
    if (!read) {
        throw detail::refused(what, digits + " is past the range of its type");
    }
    return *read;
}

/// A `bool`.
inline bool boolean(const Value &value, const std::string &what) {
    if (!value.isBool()) {
        throw detail::notA(value, what, "a truth value");
    }
    return value.asBool();
}

/// A text of at most @p max bytes of UTF-8, the bound its type declares, held
/// in `Text`: the engine's own text, or the bounded text of a record's field.
template <typename Text> Text text(const Value &value, std::size_t max, const std::string &what) {
    if (!value.isString()) {
        throw detail::notA(value, what, "a text");
    }
    const std::string &spelled = value.asString();
    const std::string count = std::to_string(spelled.size());
    if (spelled.size() > max) {
        throw detail::refused(what,
                              "a text of " + count + " bytes is past the " + std::to_string(max) + " its type holds");
    }
    if (!SCE::Forge::is_well_formed_utf8(spelled.data(), spelled.size())) {
        throw detail::refused(what, "a text of " + count + " bytes is not well-formed UTF-8");
    }
    auto read = detail::heldText(spelled, static_cast<Text *>(nullptr));
    if (!read) {
        throw detail::refused(what, "a text of " + count + " bytes is past what its type holds");
    }
    return std::move(*read);
}

/// A byte string of at most @p max octets, held in `Octets`: the text of its
/// octets in upper-case hex, two digits each.
template <typename Octets> Octets bytes(const Value &value, std::size_t max, const std::string &what) {
    if (!value.isString()) {
        throw detail::notA(value, what, "a byte string, which is a text of hex");
    }
    const std::string &hex = value.asString();
    auto octets = detail::upperHex(hex);
    if (!octets) {
        throw detail::refused(what, "'" + hex + "' is not octets in upper-case hex, two digits each");
    }
    const std::string count = std::to_string(octets->size());
    if (octets->size() > max) {
        throw detail::refused(what, "a byte string of " + count + " octets is past the " + std::to_string(max) +
                                        " its type holds");
    }
    auto read = detail::heldOctets(std::move(*octets), static_cast<Octets *>(nullptr));
    if (!read) {
        throw detail::refused(what, "a byte string of " + count + " octets is past what its type holds");
    }
    return std::move(*read);
}

/// Each variant of an enum `E` its document declares: its declared name and
/// its value.
template <typename E> using Declared = std::vector<std::pair<std::string, E>>;

/// A value of an enum of a closed set: the variant whose declared name the
/// text is, from @p declared.
template <typename E> E enumerated(const Value &value, const Declared<E> &declared, const std::string &what) {
    if (!value.isString()) {
        throw detail::notA(value, what, "a value of an enum, which is its name");
    }
    const std::string &name = value.asString();
    for (const auto &[declaredName, variant] : declared) {
        if (declaredName == name) {
            return variant;
        }
    }
    throw detail::refused(what, "'" + name + "' names no variant of its enum");
}

/// A value of an enum of an open set (`sce:strict-variants="false"`): the
/// variant whose declared name the text is, or the value @p fromUnderlying
/// makes of the number the text spells, where no variant declares that number.
/// A declared number is written by its variant's name, so its digits are
/// refused as the second spelling of one value.
template <typename E>
E openEnumerated(const Value &value, const Declared<E> &declared, E (*fromUnderlying)(std::underlying_type_t<E>),
                 const std::string &what) {
    if (!value.isString()) {
        throw detail::notA(value, what, "a value of an enum, which is its name");
    }
    const std::string &name = value.asString();
    for (const auto &[declaredName, variant] : declared) {
        if (declaredName == name) {
            return variant;
        }
    }
    const auto number = decimal(name);
    if (!number) {
        throw detail::refused(what, "'" + name + "' names no variant of its enum, and is no number");
    }
    const auto raw = held<std::underlying_type_t<E>>(*number);
    if (!raw) {
        throw detail::refused(what, name + " is past the range of its enum");
    }
    const E read = fromUnderlying(*raw);
    for (const auto &entry : declared) {
        if (entry.second == read) {
            throw detail::refused(what, name + " is the number of a declared variant, which is written by its name");
        }
    }
    return read;
}

/// A record record() has held to its fields: a view of the object the value
/// read is, which field() and optionalField() read. A view held by value, and
/// no reference returned from a call, so the only thing it can point into is
/// the value read, which the caller holds, and never an argument the call was
/// handed as a temporary (the names of the fields, where it stands).
///
/// A view is a reference of its own: binding one a call returned to a
/// reference, as a range-based `for` binds the list it iterates, is checked as
/// binding the reference the call returned (GCC's `-Wdangling-reference`). So
/// a call that returns a view takes by reference only the value the view
/// points into, and every other argument by value: the names of the fields
/// as views of their literals, and where the value stands as a view of its
/// text, which a refusal copies.
class CanonicalRecord {
public:
    explicit CanonicalRecord(const Value &object) : object_(&object) {}

    /// The member @p id of the object, or null where it names none.
    const Value *member(std::string_view id) const {
        return object_->asObject().get(id);
    }

private:
    const Value *object_;
};

/// The elements of a list elements() has held to its bounds: a view of the
/// array the value read is, iterated as the array is, held by value for the
/// reason a CanonicalRecord is.
class CanonicalList {
public:
    explicit CanonicalList(const SCE::Saved::JsonArray &items) : items_(&items) {}

    SCE::Saved::JsonArray::const_iterator begin() const {
        return items_->begin();
    }

    SCE::Saved::JsonArray::const_iterator end() const {
        return items_->end();
    }

    std::size_t size() const {
        return items_->size();
    }

private:
    const SCE::Saved::JsonArray *items_;
};

/// A record whose fields are @p fields: an object, each member of which names
/// one of them. Returns the view of the object, whose fields field() and
/// optionalField() read; only @p value is taken by reference, for the reason a
/// CanonicalRecord gives.
inline CanonicalRecord record(const Value &value, std::initializer_list<std::string_view> fields,
                              std::string_view what) {
    if (!value.isObject()) {
        throw detail::notA(value, std::string(what), "a record, which is an object of its fields");
    }
    for (const std::string &name : value.asObject().keys()) {
        if (std::find(fields.begin(), fields.end(), std::string_view(name)) == fields.end()) {
            throw detail::refused(std::string(what), "'" + name + "' is no field of its record");
        }
    }
    return CanonicalRecord(value);
}

/// The field @p id of the record @p record, whose type is not optional, so it
/// is present. Every parameter is taken by value, so the reference returned
/// points into the value the record views and into no argument.
inline const Value &field(CanonicalRecord record, std::string_view id, std::string_view what) {
    const Value *member = record.member(id);
    if (member == nullptr) {
        throw detail::refused(std::string(what), "the field is absent, and its type is not optional");
    }
    return *member;
}

/// The field @p id of the record @p record, whose type is optional: null where
/// it is absent, which the canonical form writes by omitting it.
inline const Value *optionalField(CanonicalRecord record, std::string_view id) {
    return record.member(id);
}

/// The elements of a list of at least @p min and at most @p max elements, as
/// a view of its array; only @p value is taken by reference, for the reason a
/// CanonicalRecord gives, since the loop that iterates the view binds it.
inline CanonicalList elements(const Value &value, std::size_t min, std::size_t max, std::string_view what) {
    if (!value.isArray()) {
        throw detail::notA(value, std::string(what), "a list, which is an array");
    }
    const SCE::Saved::JsonArray &items = value.asArray();
    if (items.size() < min || items.size() > max) {
        throw detail::refused(std::string(what), "a list of " + std::to_string(items.size()) +
                                                     " elements is outside its bounds, " + std::to_string(min) +
                                                     " to " + std::to_string(max));
    }
    return CanonicalList(items);
}

/// The elements @p items, read, as the bounded list `List` they are read
/// into, whose bound elements() has held them to.
template <typename List> List list(std::vector<typename List::value_type> items, const std::string &what) {
    const std::size_t count = items.size();
    auto read = List::make(std::move(items));
    if (!read) {
        throw detail::refused(what, "a list of " + std::to_string(count) + " elements is past the " +
                                        std::to_string(List::most) + " its type holds");
    }
    return std::move(*read);
}

/// An element of a list of `optional<T>`: null for JSON `null`, which the
/// canonical form writes in the place of an absent element, and the element
/// otherwise.
inline const Value *nullable(const Value &value) {
    return value.isNull() ? nullptr : &value;
}

}  // namespace SCE::Test::FromCanonical
