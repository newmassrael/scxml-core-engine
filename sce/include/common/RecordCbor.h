// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE::Common::RecordCbor — the record codec: a record a COSE operation of the
// standard host signs or seals by name crosses as its deterministic CBOR, and
// one it verified or opened is read back from those bytes (docs/adr/0109, D9;
// docs/adr/0101, section 2; docs/SCE_HOST_FEATURES.md, "Signing and sealing").
//
// A machine holds a record in its own types and writes it, as every payload,
// as JSON. The two forms are one tree (docs/adr/0101, section 2.4), the leaves
// related one by one by the record's type, and this header is that one map,
// both ways: an integer a JSON number (one of 64 bits the text of its digits)
// and a CBOR integer; a float a number and the shortest float that holds it;
// a text a text; a byte string its Latin-1 text and a CBOR byte string; an enum
// the name of its variant and the integer of its number; a record an object of
// every field (`null` for one absent) and a map of the fields present keyed by
// their ids; a union an object of its tag beside the case's fields and a map of
// one entry, the case's name to its record; a list an array, an absent element
// `null` in both.
//
// The type is the description the build writes of it
// (`sce_build::forge::payload_record::describe`), JSON text, each record's
// fields in the order their keys encode, so this header sorts nothing. Every
// CBOR item is written and read by the forge runtime's items
// (`sce/forge/cbor.h`). Mirrors `sce_rust_runtime::record_cbor`, rule for rule,
// so the two lanes write the same bytes and refuse the same bytes.
//
// The description of a record names each field by its id, whether it is
// optional, its type and what a map that lacks its key reads (docs/adr/0044;
// SCE_FORGE.md section 4.6.1, "A record whose schema grew"): `null` for a
// field of the first revision that is not optional, which such a map is
// refused for (CborRequiredKeyMissing), and for an optional field, which such a
// map leaves absent; and for a field a later revision added, the hexadecimal
// text of the deterministic CBOR of its `sce:default`, or `80` (the empty list)
// for a list, which such a map reads as the value those bytes hold, read by the
// reader of the field's own value. The bytes are the ones the plan of the codec
// ADR 0101 prints for the same schema holds for the field, which its decode
// reads the same way, so the two read a record an earlier revision wrote alike.
// A default that is no value of its field's type, or that bytes follow, is a
// description the codec cannot read, refused before any byte of a record is.
//
// A decode reads the profile of definite lengths and refuses what is no value
// of the record (docs/adr/0101, section 3.3): an item of another type, a key
// given twice, a field of the first revision that is not optional missing, a
// value outside its type, a variant no enum or union declares, and bytes after
// the record. It also refuses what the machine's payload holds no spelling of:
// a float that is not finite and a text that carries U+0000.
//
// Header-only, as the payload lift is: a generated machine compiles against it
// with no runtime library.

#pragma once

#include "common/Latin1Bytes.h"
#include "common/ScriptValueJson.h"
#include "saved/SavedJson.h"
#include "sce/forge/cbor.h"
#include "sce/forge/codec.h"
#include "sce/forge/span.h"

#include <cerrno>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace SCE::Common::RecordCbor {

namespace detail {

using ::SCE::Saved::JsonArray;
using ::SCE::Saved::JsonNumber;
using ::SCE::Saved::JsonObject;
using ::SCE::Saved::JsonValue;

/// Why a record has no encoding, or why bytes are no value of a record: the
/// strict RFC 6901 pointer to the place, and what was refused there.
struct Refused {
    std::string pointer;
    std::string reason;

    [[nodiscard]] std::string sentence() const {
        if (pointer.empty()) {
            return "the record " + reason;
        }
        return "'" + pointer + "' " + reason;
    }
};

/// What a CBOR read refused, as a refusal names it.
inline Refused codecRefusal(const std::string &pointer, ::SCE::Forge::CodecError kind) {
    return Refused{pointer, "is no value of its type (codec error " + std::to_string(static_cast<int>(kind)) + ")"};
}

struct Shape;

/// A field of a record, as the description names it.
struct Field {
    std::string id;
    bool optional = false;
    std::shared_ptr<Shape> shape;
    /// What a map that lacks the field's key reads: for a field a later revision of its schema added, the value its
    /// default's bytes hold; none where such a map is refused, or, for an optional field, reads it absent.
    std::optional<JsonValue> lacked;
};

/// A case of a union: its variant's name, and its record's fields where it
/// holds one.
struct Case {
    std::string name;
    bool holdsRecord = false;
    std::vector<Field> fields;
};

/// A variant of an enum: its name, and its number as a CBOR head writes it:
/// major type 0 and the number, or major type 1 and `-1 - number`.
struct Variant {
    std::string name;
    bool negative = false;
    std::uint64_t argument = 0;
};

/// A type, as the description names it.
struct Shape {
    enum class Kind { Unsigned, Signed, Float, Bool, Text, Bytes, Enum, Record, Union, List, Nullable };
    Kind kind = Kind::Bool;
    /// The width of a number.
    std::uint32_t bits = 0;
    /// The bound of a text, a byte string or a list.
    std::uint64_t max = 0;
    /// The lower count of a list.
    std::uint64_t min = 0;
    std::vector<Variant> variants;
    std::vector<Field> fields;
    std::string tag;
    std::vector<Case> cases;
    /// The element of a list, or the value of an element that may be absent.
    std::shared_ptr<Shape> of;
};

/// A whole number at or above zero a description spells.
inline std::uint64_t count(const JsonValue &value) {
    if (!value.isNumber()) {
        throw Refused{"", "has a description whose count is no number"};
    }
    const std::string &text = value.asNumber().text;
    if (text.empty() || text.find_first_not_of("0123456789") != std::string::npos) {
        throw Refused{"", "has a description whose count `" + text + "` is no count"};
    }
    return std::strtoull(text.c_str(), nullptr, 10);
}

/// The sign and the magnitude of a whole number written as `text`, or none.
inline std::optional<std::pair<bool, std::uint64_t>> wholeNumber(const std::string &text) {
    const bool negative = !text.empty() && text[0] == '-';
    const std::string digits = negative ? text.substr(1) : text;
    if (digits.empty() || digits.size() > 20 || digits.find_first_not_of("0123456789") != std::string::npos) {
        return std::nullopt;
    }
    errno = 0;
    const unsigned long long magnitude = std::strtoull(digits.c_str(), nullptr, 10);
    if (errno == ERANGE) {
        return std::nullopt;
    }
    return std::make_pair(negative, static_cast<std::uint64_t>(magnitude));
}

inline std::shared_ptr<Shape> shapeOf(const JsonValue &value);

inline JsonValue readValue(const Shape &shape, ::SCE::Forge::SceCursor &c, const std::string &at);

/// The bytes the lowercase hexadecimal text `text` spells, two digits a byte; none for a text of an odd length or a
/// character that is no such digit.
inline std::optional<std::vector<std::uint8_t>> hexBytes(const std::string &text) {
    if (text.size() % 2 != 0) {
        return std::nullopt;
    }
    const auto digit = [](char c) -> int {
        if (c >= '0' && c <= '9') {
            return c - '0';
        }
        if (c >= 'a' && c <= 'f') {
            return c - 'a' + 10;
        }
        return -1;
    };
    std::vector<std::uint8_t> bytes;
    bytes.reserve(text.size() / 2);
    for (std::size_t at = 0; at < text.size(); at += 2) {
        const int high = digit(text[at]);
        const int low = digit(text[at + 1]);
        if (high < 0 || low < 0) {
            return std::nullopt;
        }
        bytes.push_back(static_cast<std::uint8_t>((high << 4) | low));
    }
    return bytes;
}

/// The value the bytes `hex` spell hold, read by the reader of a value of `shape`, which the field `id` reads where
/// a map lacks its key: a default is held to every rule a written value is held to, and bytes after the value are
/// refused, as after a record.
inline JsonValue readDefault(const Shape &shape, const std::string &id, const std::string &hex) {
    const auto bytes = hexBytes(hex);
    if (!bytes) {
        throw Refused{"", "has a description whose default of `" + id + "` (`" + hex +
                              "`) is no lowercase hexadecimal text"};
    }
    ::SCE::Forge::SceCursor cursor(bytes->data(), bytes->size());
    JsonValue value;
    try {
        value = readValue(shape, cursor, "");
    } catch (const Refused &why) {
        throw Refused{"", "has a description whose default of `" + id + "` is no value of its type: " + why.reason};
    }
    if (cursor.remaining() != 0) {
        throw Refused{"", "has a description whose default of `" + id + "` is followed by " +
                              std::to_string(cursor.remaining()) + " bytes that are no part of it"};
    }
    return value;
}

inline std::vector<Field> fieldsOf(const JsonValue &value) {
    if (!value.isArray()) {
        throw Refused{"", "has a description whose record's fields are no array"};
    }
    std::vector<Field> fields;
    for (const JsonValue &field : value.asArray()) {
        if (!field.isArray() || field.asArray().size() != 4 || !field.asArray()[0].isString() ||
            !field.asArray()[1].isBool()) {
            throw Refused{"", "has a description whose field is not an id, an optional mark, a type and what a map "
                              "that lacks its key reads"};
        }
        const JsonArray &described = field.asArray();
        const std::string &id = described[0].asString();
        const bool optional = described[1].asBool();
        std::shared_ptr<Shape> shape = shapeOf(described[2]);
        std::optional<JsonValue> lacked;
        if (described[3].isString()) {
            // A missing key of an optional field is how a writer of every revision spells it absent, so it reads no
            // default.
            if (optional) {
                throw Refused{"", "has a description whose optional field `" + id +
                                      "` names a default, and a map that lacks it reads it absent"};
            }
            lacked = readDefault(*shape, id, described[3].asString());
        } else if (!described[3].isNull()) {
            throw Refused{"", "has a description whose default of `" + id + "` is neither a text nor null"};
        }
        fields.push_back(Field{id, optional, std::move(shape), std::move(lacked)});
    }
    return fields;
}

inline std::shared_ptr<Shape> shapeOf(const JsonValue &value) {
    if (!value.isObject() || value.asObject().size() != 1) {
        throw Refused{"", "has a description whose type is not an object of one member"};
    }
    const std::string &kind = value.asObject().keys()[0];
    const JsonValue &argument = value.asObject().valueAt(0);
    auto shape = std::make_shared<Shape>();
    const auto width = [&]() {
        const std::uint64_t bits = count(argument);
        if (bits != 8 && bits != 16 && bits != 32 && bits != 64) {
            throw Refused{"", "has a description whose width " + std::to_string(bits) + " is no width"};
        }
        return static_cast<std::uint32_t>(bits);
    };
    if (kind == "uint") {
        shape->kind = Shape::Kind::Unsigned;
        shape->bits = width();
    } else if (kind == "int") {
        shape->kind = Shape::Kind::Signed;
        shape->bits = width();
    } else if (kind == "float") {
        shape->kind = Shape::Kind::Float;
        shape->bits = width();
        if (shape->bits != 32 && shape->bits != 64) {
            throw Refused{"", "has a description whose float is of no width a float has"};
        }
    } else if (kind == "bool") {
        shape->kind = Shape::Kind::Bool;
    } else if (kind == "text") {
        shape->kind = Shape::Kind::Text;
        shape->max = count(argument);
    } else if (kind == "bytes") {
        shape->kind = Shape::Kind::Bytes;
        shape->max = count(argument);
    } else if (kind == "enum") {
        shape->kind = Shape::Kind::Enum;
        const JsonValue *variants = argument.isObject() ? argument.asObject().get("variants") : nullptr;
        if (variants == nullptr || !variants->isArray()) {
            throw Refused{"", "has a description whose enum names no variants"};
        }
        for (const JsonValue &variant : variants->asArray()) {
            if (!variant.isArray() || variant.asArray().size() != 2 || !variant.asArray()[0].isString() ||
                !variant.asArray()[1].isNumber()) {
                throw Refused{"", "has a description whose variant is not a name and a number"};
            }
            const auto number = wholeNumber(variant.asArray()[1].asNumber().text);
            if (!number || (number->first && number->second == 0)) {
                throw Refused{"", "has a description whose variant's number is no whole number"};
            }
            Variant read;
            read.name = variant.asArray()[0].asString();
            read.negative = number->first;
            read.argument = number->first ? number->second - 1 : number->second;
            shape->variants.push_back(std::move(read));
        }
    } else if (kind == "record") {
        shape->kind = Shape::Kind::Record;
        shape->fields = fieldsOf(argument);
    } else if (kind == "union") {
        shape->kind = Shape::Kind::Union;
        const JsonValue *tag = argument.isObject() ? argument.asObject().get("tag") : nullptr;
        const JsonValue *cases = argument.isObject() ? argument.asObject().get("cases") : nullptr;
        if (tag == nullptr || !tag->isString() || cases == nullptr || !cases->isArray()) {
            throw Refused{"", "has a description whose union names no tag or no cases"};
        }
        shape->tag = tag->asString();
        for (const JsonValue &entry : cases->asArray()) {
            if (!entry.isArray() || entry.asArray().size() != 2 || !entry.asArray()[0].isString()) {
                throw Refused{"", "has a description whose case is not a name and its fields"};
            }
            Case read;
            read.name = entry.asArray()[0].asString();
            if (!entry.asArray()[1].isNull()) {
                read.holdsRecord = true;
                read.fields = fieldsOf(entry.asArray()[1]);
            }
            shape->cases.push_back(std::move(read));
        }
    } else if (kind == "list") {
        shape->kind = Shape::Kind::List;
        const JsonValue *min = argument.isObject() ? argument.asObject().get("min") : nullptr;
        const JsonValue *max = argument.isObject() ? argument.asObject().get("max") : nullptr;
        const JsonValue *of = argument.isObject() ? argument.asObject().get("of") : nullptr;
        if (min == nullptr || max == nullptr || of == nullptr) {
            throw Refused{"", "has a description whose list names no counts or no element"};
        }
        shape->min = count(*min);
        shape->max = count(*max);
        shape->of = shapeOf(*of);
    } else if (kind == "nullable") {
        shape->kind = Shape::Kind::Nullable;
        shape->of = shapeOf(argument);
    } else {
        throw Refused{"", "has a description that names `" + kind + "`, no kind of type"};
    }
    return shape;
}

inline std::shared_ptr<Shape> readDescription(std::string_view description) {
    JsonValue value;
    try {
        value = ::SCE::Saved::parseJson(description);
    } catch (const ::SCE::Saved::JsonError &why) {
        throw Refused{"", std::string("has a description that is not JSON (") + why.what() + ")"};
    }
    return shapeOf(value);
}

/// `token` below `pointer`, as RFC 6901 joins reference tokens.
inline std::string below(const std::string &pointer, const std::string &token) {
    std::string escaped;
    for (const char c : token) {
        if (c == '~') {
            escaped += "~0";
        } else if (c == '/') {
            escaped += "~1";
        } else {
            escaped += c;
        }
    }
    return pointer + "/" + escaped;
}

/// The least magnitude below zero and the greatest at or above zero a number
/// of `bits` bits holds, `negativeLimit` zero for an unsigned one.
inline void limits(const Shape &shape, std::uint64_t &negativeLimit, std::uint64_t &positiveLimit) {
    const bool isSigned = shape.kind == Shape::Kind::Signed;
    if (shape.bits == 64) {
        positiveLimit = isSigned ? static_cast<std::uint64_t>(INT64_MAX) : UINT64_MAX;
        negativeLimit = isSigned ? static_cast<std::uint64_t>(INT64_MAX) + 1u : 0u;
        return;
    }
    const std::uint64_t span = std::uint64_t{1} << (isSigned ? shape.bits - 1 : shape.bits);
    positiveLimit = span - 1;
    negativeLimit = isSigned ? span : 0u;
}

inline const JsonValue *member(const JsonObject &object, const std::string &name) {
    return object.get(name);
}

inline void writeValue(const Shape &shape, const JsonValue &value, const std::string &at, ::SCE::Forge::SceSink &w);

inline void writeRecord(const std::vector<Field> &fields, const JsonObject &members, const std::string &at,
                        ::SCE::Forge::SceSink &w) {
    std::vector<std::pair<const Field *, const JsonValue *>> present;
    for (const Field &field : fields) {
        const JsonValue *value = member(members, field.id);
        if (value == nullptr || value->isNull()) {
            if (field.optional) {
                continue;
            }
            if (value == nullptr) {
                throw Refused{below(at, field.id), "is missing"};
            }
        }
        present.emplace_back(&field, value);
    }
    (void)::SCE::Forge::Cbor::write_map_head(w, present.size());
    for (const auto &[field, value] : present) {
        (void)::SCE::Forge::Cbor::write_text(w, field->id);
        writeValue(*field->shape, *value, below(at, field->id), w);
    }
}

inline void writeValue(const Shape &shape, const JsonValue &value, const std::string &at, ::SCE::Forge::SceSink &w) {
    namespace Cbor = ::SCE::Forge::Cbor;
    const auto wrong = [&](const char *what) { return Refused{at, std::string("is not ") + what}; };
    switch (shape.kind) {
    case Shape::Kind::Unsigned:
    case Shape::Kind::Signed: {
        // A 64-bit integer is written as the text of its digits, every other
        // one as a number.
        const std::string *text = nullptr;
        if (value.isNumber()) {
            text = &value.asNumber().text;
        } else if (value.isString()) {
            text = &value.asString();
        }
        std::optional<std::pair<bool, std::uint64_t>> number;
        if (text != nullptr) {
            number = wholeNumber(*text);
        }
        if (!number) {
            throw wrong("a whole number");
        }
        std::uint64_t negativeLimit = 0;
        std::uint64_t positiveLimit = 0;
        limits(shape, negativeLimit, positiveLimit);
        // `-0` is zero.
        const bool belowZero = number->first && number->second != 0;
        const bool fits = belowZero ? number->second <= negativeLimit : number->second <= positiveLimit;
        if (!fits) {
            throw Refused{at, "(" + *text + ") does not fit the width its type declares"};
        }
        if (belowZero) {
            (void)Cbor::write_head(w, Cbor::kMajorNegative, number->second - 1);
        } else {
            (void)Cbor::write_uint(w, number->second);
        }
        return;
    }
    case Shape::Kind::Float: {
        if (!value.isNumber()) {
            throw wrong("a number");
        }
        double x = std::strtod(value.asNumber().text.c_str(), nullptr);
        if (shape.bits == 32) {
            x = static_cast<double>(static_cast<float>(x));
        }
        if (!std::isfinite(x)) {
            throw Refused{at, "is not a finite number"};
        }
        (void)Cbor::write_float(w, x);
        return;
    }
    case Shape::Kind::Bool:
        if (!value.isBool()) {
            throw wrong("a truth value");
        }
        (void)Cbor::write_bool(w, value.asBool());
        return;
    case Shape::Kind::Text:
        if (!value.isString()) {
            throw wrong("a text");
        }
        if (value.asString().size() > shape.max) {
            throw Refused{at, "is " + std::to_string(value.asString().size()) + " bytes, past the " +
                                  std::to_string(shape.max) + " its type declares"};
        }
        (void)Cbor::write_text(w, value.asString());
        return;
    case Shape::Kind::Bytes: {
        if (!value.isString()) {
            throw wrong("a byte string");
        }
        std::vector<std::uint8_t> bytes;
        const std::string why = ::SCE::Latin1Bytes::bytesOf(value.asString(), bytes);
        if (!why.empty()) {
            throw Refused{at, why};
        }
        if (bytes.size() > shape.max) {
            throw Refused{at, "is " + std::to_string(bytes.size()) + " bytes, past the " + std::to_string(shape.max) +
                                  " its type declares"};
        }
        (void)Cbor::write_bytes(w, ::SCE::Forge::Span<const std::uint8_t>(bytes.data(), bytes.size()));
        return;
    }
    case Shape::Kind::Enum: {
        if (!value.isString()) {
            throw wrong("the name of a variant");
        }
        for (const Variant &variant : shape.variants) {
            if (variant.name == value.asString()) {
                (void)Cbor::write_head(w, variant.negative ? Cbor::kMajorNegative : Cbor::kMajorUnsigned,
                                       variant.argument);
                return;
            }
        }
        throw Refused{at, "(" + value.asString() + ") names no variant"};
    }
    case Shape::Kind::Record:
        if (!value.isObject()) {
            throw wrong("a record");
        }
        writeRecord(shape.fields, value.asObject(), at, w);
        return;
    case Shape::Kind::Union: {
        if (!value.isObject()) {
            throw wrong("a union");
        }
        const JsonValue *variant = member(value.asObject(), shape.tag);
        if (variant == nullptr || !variant->isString()) {
            throw Refused{below(at, shape.tag), "is not the name of a case"};
        }
        for (const Case &c : shape.cases) {
            if (c.name == variant->asString()) {
                (void)Cbor::write_variant(w, c.name);
                if (c.holdsRecord) {
                    writeRecord(c.fields, value.asObject(), at, w);
                } else {
                    (void)Cbor::write_map_head(w, 0);
                }
                return;
            }
        }
        throw Refused{below(at, shape.tag), "(" + variant->asString() + ") names no case"};
    }
    case Shape::Kind::List: {
        if (!value.isArray()) {
            throw wrong("a list");
        }
        const JsonArray &items = value.asArray();
        if (items.size() < shape.min || items.size() > shape.max) {
            throw Refused{at, "holds " + std::to_string(items.size()) + " elements, outside the " +
                                  std::to_string(shape.min) + " to " + std::to_string(shape.max) +
                                  " its type declares"};
        }
        (void)Cbor::write_array_head(w, items.size());
        for (std::size_t index = 0; index < items.size(); ++index) {
            writeValue(*shape.of, items[index], below(at, std::to_string(index)), w);
        }
        return;
    }
    case Shape::Kind::Nullable:
        if (value.isNull()) {
            (void)Cbor::write_null(w);
        } else {
            writeValue(*shape.of, value, at, w);
        }
        return;
    }
}

/// The value of a CBOR read, or the refusal at `at` of what it refused.
template <typename T> T took(::SCE::Forge::Decoded<T> read, const std::string &at) {
    if (!read) {
        throw codecRefusal(at, read.error());
    }
    return std::move(*read);
}

inline JsonValue readValue(const Shape &shape, ::SCE::Forge::SceCursor &c, const std::string &at);

inline JsonObject readRecord(const std::vector<Field> &fields, ::SCE::Forge::SceCursor &c, const std::string &at) {
    namespace Cbor = ::SCE::Forge::Cbor;
    const std::uint64_t entries = took(Cbor::read_map_len(c), at);
    std::vector<std::optional<JsonValue>> held(fields.size());
    for (std::uint64_t entry = 0; entry < entries; ++entry) {
        const std::string key = took(Cbor::read_text(c, std::nullopt), at);
        bool declared = false;
        for (std::size_t index = 0; index < fields.size(); ++index) {
            if (fields[index].id != key) {
                continue;
            }
            declared = true;
            if (held[index]) {
                throw codecRefusal(below(at, key), ::SCE::Forge::CodecError::CborDuplicateKey);
            }
            held[index] = readValue(*fields[index].shape, c, below(at, key));
            break;
        }
        if (!declared) {
            (void)took(Cbor::skip(c), at);
        }
    }
    JsonObject members;
    for (std::size_t index = 0; index < fields.size(); ++index) {
        if (held[index]) {
            members.set(fields[index].id, std::move(*held[index]));
        } else if (fields[index].optional) {
            members.set(fields[index].id, JsonValue(nullptr));
        } else if (fields[index].lacked) {
            // A map a writer of a revision before the field existed wrote (docs/adr/0044).
            members.set(fields[index].id, *fields[index].lacked);
        } else {
            throw codecRefusal(below(at, fields[index].id), ::SCE::Forge::CodecError::CborRequiredKeyMissing);
        }
    }
    return members;
}

inline JsonValue readValue(const Shape &shape, ::SCE::Forge::SceCursor &c, const std::string &at) {
    namespace Cbor = ::SCE::Forge::Cbor;
    switch (shape.kind) {
    case Shape::Kind::Unsigned: {
        std::uint64_t negativeLimit = 0;
        std::uint64_t positiveLimit = 0;
        limits(shape, negativeLimit, positiveLimit);
        const std::uint64_t n = took(Cbor::read_uint_upto(c, positiveLimit), at);
        if (shape.bits == 64) {
            return JsonValue(std::to_string(n));
        }
        return JsonValue(JsonNumber{std::to_string(n)});
    }
    case Shape::Kind::Signed: {
        std::uint64_t negativeLimit = 0;
        std::uint64_t positiveLimit = 0;
        limits(shape, negativeLimit, positiveLimit);
        const std::int64_t least = shape.bits == 64 ? INT64_MIN : -static_cast<std::int64_t>(negativeLimit);
        const std::int64_t n = took(Cbor::read_int_within(c, least, static_cast<std::int64_t>(positiveLimit)), at);
        if (shape.bits == 64) {
            return JsonValue(std::to_string(n));
        }
        return JsonValue(JsonNumber{std::to_string(n)});
    }
    case Shape::Kind::Float: {
        const double x =
            shape.bits == 32 ? static_cast<double>(took(Cbor::read_float32(c), at)) : took(Cbor::read_float(c), at);
        if (!std::isfinite(x)) {
            throw Refused{at, "is a float that is not finite, which no payload spells"};
        }
        return JsonValue(JsonNumber{::SCE::JsonText::numberText(x)});
    }
    case Shape::Kind::Bool:
        return JsonValue(took(Cbor::read_bool(c), at));
    case Shape::Kind::Text: {
        std::string text = took(Cbor::read_text(c, shape.max), at);
        if (text.find('\0') != std::string::npos) {
            throw Refused{at, "carries U+0000, which no text holds"};
        }
        return JsonValue(std::move(text));
    }
    case Shape::Kind::Bytes:
        return JsonValue(::SCE::Latin1Bytes::textOf(took(Cbor::read_bytes(c, shape.max), at)));
    case Shape::Kind::Enum: {
        const Cbor::Head head = took(Cbor::read_head(c), at);
        if (head.major != Cbor::kMajorUnsigned && head.major != Cbor::kMajorNegative) {
            throw codecRefusal(at, ::SCE::Forge::CodecError::CborWrongType);
        }
        const bool negative = head.major == Cbor::kMajorNegative;
        for (const Variant &variant : shape.variants) {
            if (variant.negative == negative && variant.argument == head.value) {
                return JsonValue(variant.name);
            }
        }
        throw codecRefusal(at, ::SCE::Forge::CodecError::UndeclaredEnumValue);
    }
    case Shape::Kind::Record:
        return JsonValue(readRecord(shape.fields, c, at));
    case Shape::Kind::Union: {
        if (took(Cbor::read_map_len(c), at) != 1) {
            throw codecRefusal(at, ::SCE::Forge::CodecError::CborUnionEntries);
        }
        const std::string name = took(Cbor::read_text(c, std::nullopt), at);
        for (const Case &cased : shape.cases) {
            if (cased.name != name) {
                continue;
            }
            JsonObject members;
            if (cased.holdsRecord) {
                members = readRecord(cased.fields, c, below(at, name));
            } else {
                (void)took(Cbor::read_empty_case(c), at);
            }
            members.set(shape.tag, JsonValue(name));
            return JsonValue(std::move(members));
        }
        throw codecRefusal(at, ::SCE::Forge::CodecError::CborUnknownVariant);
    }
    case Shape::Kind::List: {
        const std::uint64_t items = took(Cbor::read_array_len(c), at);
        if (items < shape.min || items > shape.max) {
            throw codecRefusal(at, ::SCE::Forge::CodecError::CborOutOfRange);
        }
        JsonArray read;
        for (std::uint64_t index = 0; index < items; ++index) {
            read.push_back(readValue(*shape.of, c, below(at, std::to_string(index))));
        }
        return JsonValue(std::move(read));
    }
    case Shape::Kind::Nullable:
        if (took(Cbor::take_null(c), at)) {
            return JsonValue(nullptr);
        }
        return readValue(*shape.of, c, at);
    }
    throw Refused{at, "has a type the codec does not read"};
}

}  // namespace detail

/// The deterministic CBOR of the record `recordJson` holds, the record's JSON
/// as a machine writes it, under the description `description` of its type;
/// none, with `refusal` saying why, for a description the codec cannot read
/// and a record that is no value of its type (a list below its lower count, a
/// value past a bound, a variant its enum does not declare).
inline std::optional<std::vector<std::uint8_t>> encode(const std::string &recordJson, std::string_view description,
                                                       std::string &refusal) {
    try {
        const auto shape = detail::readDescription(description);
        detail::JsonValue value;
        try {
            value = ::SCE::Saved::parseJson(recordJson);
        } catch (const ::SCE::Saved::JsonError &why) {
            throw detail::Refused{"", std::string("is not JSON (") + why.what() + ")"};
        }
        std::vector<std::uint8_t> bytes;
        ::SCE::Forge::VectorSink sink(bytes);
        detail::writeValue(*shape, value, "", sink);
        return bytes;
    } catch (const detail::Refused &refused) {
        refusal = refused.sentence();
        return std::nullopt;
    }
}

/// `encode`, as the byte string's Latin-1 text, the text a request's byte
/// string crosses as (SCE Accepted Subset §2.12).
inline std::optional<std::string> encodeText(const std::string &recordJson, std::string_view description,
                                             std::string &refusal) {
    auto bytes = encode(recordJson, description, refusal);
    if (!bytes) {
        return std::nullopt;
    }
    return ::SCE::Latin1Bytes::textOf(*bytes);
}

/// `encodeText` of the record a machine holds as `value`, the value its record
/// `<param>` evaluates to, written as its JSON first.
inline std::optional<std::string> encodeValue(const ScriptValue &value, std::string_view description,
                                              std::string &refusal) {
    return encodeText(::SCE::ScriptValueJson::text(value), description, refusal);
}

/// The record `bytes` hold, the deterministic CBOR or any encoding of it within
/// the profile of definite lengths, under the description `description` of its
/// type, as the JSON a machine reads a record from; none, with `refusal` saying
/// why, for bytes that are no value of the record, bytes after it included.
inline std::optional<std::string> decode(const std::vector<std::uint8_t> &bytes, std::string_view description,
                                         std::string &refusal) {
    try {
        const auto shape = detail::readDescription(description);
        ::SCE::Forge::SceCursor cursor(bytes.data(), bytes.size());
        const detail::JsonValue value = detail::readValue(*shape, cursor, "");
        if (cursor.remaining() != 0) {
            throw detail::Refused{"", "is followed by " + std::to_string(cursor.remaining()) +
                                          " bytes that are no part of it"};
        }
        return ::SCE::Saved::writeJson(value);
    } catch (const detail::Refused &refused) {
        refusal = refused.sentence();
        return std::nullopt;
    }
}

/// The completion `data` of a verification or an opening whose invoke names a
/// record, with the bytes it carries read as that record: what the machine's
/// lift of the completion reads (docs/adr/0109, D9). `field` is the result's
/// byte string (`payload`, `plaintext`), `answered` its truth field that says
/// the bytes are there (`valid`, `opened`), `description` the record's type,
/// and `failure` the completion the operation's failure `not_a_record` is,
/// which the build writes.
///
/// A completion whose `error` is not empty, or whose `answered` is not `true`,
/// carries no record: `field` is `null`, the rest as it came. One that
/// answered has `field` the record its bytes hold. One whose bytes are no
/// value of the record is `failure`, whole. A completion that is no JSON
/// object, or that carries no text under `field`, is handed on as it came, for
/// the lift to refuse as it refuses any data its record does not fit.
inline std::string openCompletion(const std::string &data, std::string_view field, std::string_view answered,
                                  std::string_view description, std::string_view failure) {
    detail::JsonValue parsed;
    try {
        parsed = ::SCE::Saved::parseJson(data);
    } catch (const ::SCE::Saved::JsonError &) {
        return data;
    }
    if (!parsed.isObject()) {
        return data;
    }
    const detail::JsonObject &members = parsed.asObject();
    const detail::JsonValue *error = members.get("error");
    const bool failed = error != nullptr && !(error->isString() && error->asString().empty());
    const detail::JsonValue *answer = members.get(answered);
    const bool answeredTrue = answer != nullptr && answer->isBool() && answer->asBool();
    const detail::JsonValue *bytesText = members.get(field);
    if (bytesText == nullptr) {
        return data;
    }
    detail::JsonObject rewritten;
    for (std::size_t at = 0; at < members.size(); ++at) {
        rewritten.set(members.keys()[at], members.valueAt(at));
    }
    if (failed || !answeredTrue) {
        rewritten.set(std::string(field), detail::JsonValue(nullptr));
        return ::SCE::Saved::writeJson(detail::JsonValue(std::move(rewritten)));
    }
    if (!bytesText->isString()) {
        return data;
    }
    std::vector<std::uint8_t> bytes;
    if (!::SCE::Latin1Bytes::bytesOf(bytesText->asString(), bytes).empty()) {
        return std::string(failure);
    }
    std::string refusal;
    const auto record = decode(bytes, description, refusal);
    if (!record) {
        return std::string(failure);
    }
    rewritten.set(std::string(field), ::SCE::Saved::parseJson(*record));
    return ::SCE::Saved::writeJson(detail::JsonValue(std::move(rewritten)));
}

}  // namespace SCE::Common::RecordCbor
