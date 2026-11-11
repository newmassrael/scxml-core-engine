// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What every payload reader refuses, and how it names the refusal (docs/adr/0087,
// sections 2.3 and 2.4). The cases live in tests/json_text/payload_refusal.json,
// the table every runtime's reader is held to: each is read here through
// EventPayloadLift.h, and the code, pointer and bound of each refusal are held to
// what the table says. This reader took the first value of a repeated name before
// it held the whole text to the JSON grammar; the cases that repeat one hold it to
// refusing the text instead.

#include "common/EventPayloadLift.h"

#include <cstdint>
#include <fstream>
#include <gtest/gtest.h>
#include <nlohmann/json.hpp>
#include <optional>
#include <sstream>
#include <string>
#include <vector>

namespace {

using SCE::Common::EventPayloadFields;
using SCE::Common::EventPayloadValue;
using SCE::Common::RefusalCode;
using Refusal = SCE::Common::EventPayloadRefusal;

/** `v` read as the type `ty` the table describes, one call per constructor, as a generated reader reads it. */
Refusal readDescribed(const nlohmann::json &ty, const EventPayloadValue &v) {
    if (ty.is_string()) {
        const std::string scalar = ty.get<std::string>();
        if (scalar == "uint8") {
            uint8_t n = 0;
            return v.readInteger(n);
        }
        if (scalar == "uint16") {
            uint16_t n = 0;
            return v.readInteger(n);
        }
        if (scalar == "uint32") {
            uint32_t n = 0;
            return v.readInteger(n);
        }
        if (scalar == "uint64") {
            uint64_t n = 0;
            return v.readWide(n);
        }
        if (scalar == "int8") {
            int8_t n = 0;
            return v.readInteger(n);
        }
        if (scalar == "int16") {
            int16_t n = 0;
            return v.readInteger(n);
        }
        if (scalar == "int32") {
            int32_t n = 0;
            return v.readInteger(n);
        }
        if (scalar == "int64") {
            int64_t n = 0;
            return v.readWide(n);
        }
        if (scalar == "float32") {
            float x = 0.0f;
            return v.readFloating(x);
        }
        if (scalar == "float64") {
            double x = 0.0;
            return v.readFloating(x);
        }
        if (scalar == "bool") {
            bool b = false;
            return v.readBool(b);
        }
        if (scalar == "string") {
            std::string text;
            return v.readName(text);
        }
        ADD_FAILURE() << "the table names no type " << scalar;
        return Refusal(RefusalCode::WrongKind, v.pointer(), "no reader");
    }
    if (ty.is_array()) {
        std::string name;
        Refusal refusal = v.readName(name);
        if (!refusal.empty()) {
            return refusal;
        }
        for (const auto &declared : ty) {
            if (declared.get<std::string>() == name) {
                return {};
            }
        }
        return v.unknownVariant(name, "Described");
    }
    if (ty.contains("optional")) {
        std::optional<int> present;
        return v.readOptional(present,
                              [&](const EventPayloadValue &p, int &) { return readDescribed(ty["optional"], p); });
    }
    if (ty.contains("string")) {
        std::string text;
        return v.readText(ty["string"].get<size_t>(), text);
    }
    if (ty.contains("bytes")) {
        std::vector<uint8_t> bytes;
        return v.readBytes(ty["bytes"].get<size_t>(), bytes);
    }
    if (ty.contains("list")) {
        std::vector<int> items;
        // The lower count, where the case states one (docs/adr/0086, section 15).
        const size_t least = ty.contains("min") ? ty["min"].get<size_t>() : 0u;
        return v.readListWithin(least, ty["max"].get<size_t>(), items, [&](const EventPayloadValue &element, int &) {
            return readDescribed(ty["list"], element);
        });
    }
    if (ty.contains("record")) {
        return v.readRecord(nullptr, [&](const EventPayloadFields &fields) {
            for (const auto &field : ty["record"].items()) {
                EventPayloadValue value;
                Refusal refusal = fields.value(field.key().c_str(), value);
                if (refusal.empty()) {
                    refusal = readDescribed(field.value(), value);
                }
                if (!refusal.empty()) {
                    return refusal;
                }
            }
            return Refusal{};
        });
    }
    if (ty.contains("union")) {
        // The tag's enum is the cases' names; an empty case is null, any other the record
        // it holds (docs/adr/0092, section 4).
        const auto &cases = ty["union"]["cases"];
        return v.readUnion<std::string>(
            ty["union"]["tag"].get<std::string>().c_str(),
            [&](const EventPayloadValue &tag, std::string &name) {
                Refusal refusal = tag.readName(name);
                if (refusal.empty() && !cases.contains(name)) {
                    return tag.unknownVariant(name, "Described");
                }
                return refusal;
            },
            [&](const std::string &name, const EventPayloadValue &object) {
                const auto &record = cases[name];
                return record.is_null() ? Refusal{} : readDescribed(record, object);
            });
    }
    ADD_FAILURE() << "a type of the table names no constructor: " << ty.dump();
    return Refusal(RefusalCode::WrongKind, v.pointer(), "no reader");
}

}  // namespace

TEST(EventPayloadRefusal, EveryCaseOfTheSharedTableIsRefusedWithItsCodePointerAndBound) {
    std::ifstream file(SCE_JSON_PAYLOAD_REFUSAL_TABLE);
    const auto table = nlohmann::json::parse(file);
    const auto &cases = table.at("cases");
    ASSERT_GE(cases.size(), 40u) << "the table was not read from " << SCE_JSON_PAYLOAD_REFUSAL_TABLE;
    for (const auto &c : cases) {
        const std::string name = c.at("name").get<std::string>();
        const std::string data = c.at("data").get<std::string>();
        EventPayloadFields fields;
        // A case that gives a depth is read as a `<sce:lift>` of the record of the one field `v`
        // reads it (docs/adr/0087, section 2.1), its pointer held to `pathBound` bytes, the
        // standard refusal's 256 where the case gives none.
        const uint32_t pathBound = c.contains("pathBound") ? c.at("pathBound").get<uint32_t>() : 256u;
        Refusal refusal = c.contains("depth") ? EventPayloadFields::lift(data, c.at("depth").get<uint32_t>(), pathBound,
                                                                         nullptr, fields)
                                              : EventPayloadFields::decode(data, fields);
        if (refusal.empty()) {
            EventPayloadValue v;
            refusal = fields.value("v", v);
            if (refusal.empty()) {
                refusal = readDescribed(c.at("type"), v);
            }
        }
        if (!c.contains("refusal")) {
            EXPECT_TRUE(refusal.empty()) << name << ": refused (" << refusal.reason() << "), the table reads it";
            continue;
        }
        const auto &want = c.at("refusal");
        ASSERT_FALSE(refusal.empty()) << name << ": read, the table refuses it";
        EXPECT_EQ(Refusal::codeName(refusal.code()), want.at("code").get<std::string>())
            << name << ": " << refusal.reason();
        EXPECT_EQ(refusal.pointer(), want.at("pointer").get<std::string>()) << name << ": " << refusal.reason();
        const std::optional<uint32_t> bound =
            want.at("bound").is_null() ? std::nullopt : std::optional<uint32_t>(want.at("bound").get<uint32_t>());
        EXPECT_EQ(refusal.bound(), bound) << name << ": " << refusal.reason();
    }
}

// The codes are the variants sce:std/lift/refusal_code.scxml declares, by the same names
// and at the same numbers.
TEST(EventPayloadRefusal, EveryCodeIsTheVariantTheStandardEnumDeclares) {
    std::ifstream file(SCE_STDLIB_REFUSAL_CODE);
    std::stringstream read;
    read << file.rdbuf();
    const std::string document = read.str();
    ASSERT_FALSE(document.empty()) << "the enum was not read from " << SCE_STDLIB_REFUSAL_CODE;
    size_t declared = 0;
    for (size_t at = document.find("<sce:variant "); at != std::string::npos;
         at = document.find("<sce:variant ", at + 1)) {
        ++declared;
    }
    // The fourteen codes, tooFew the last (docs/adr/0086, section 15.11).
    EXPECT_EQ(declared, 14u);
    for (unsigned number = 1; number <= 14u; ++number) {
        const auto code = static_cast<RefusalCode>(number);
        const std::string variant = "<sce:variant name=\"" + std::string(Refusal::codeName(code)) + "\" value=\"" +
                                    std::to_string(number) + "\"/>";
        EXPECT_NE(document.find(variant), std::string::npos) << variant;
    }
}

// RFC 6901: `/` before each token, `~` written `~0` and `/` written `~1`, `~` escaped
// first so that `~1` in a name is not read back as `/`.
TEST(EventPayloadRefusal, APointerEscapesTildeAndSolidusInEachToken) {
    EXPECT_EQ(Refusal::pointerBelow("", "items"), "/items");
    EXPECT_EQ(Refusal::pointerBelow("/items", "2"), "/items/2");
    EXPECT_EQ(Refusal::pointerBelow("", "a~b/c"), "/a~0b~1c");
    EXPECT_EQ(Refusal::pointerBelow("", "~1"), "/~01");
    EXPECT_EQ(Refusal::pointerBelow("", ""), "/");
}

// A lift refuses a container past its depth over the brackets alone, before the text is
// read as JSON, and names the container it refused, held to the room of the refusal's
// path by whole tokens (docs/adr/0087, sections 2.1 and 2.4).
TEST(EventPayloadRefusal, ALiftRefusesAContainerPastItsDepthBeforeItReadsTheText) {
    struct Case {
        const char *text;
        uint32_t depth;
        uint32_t pathBound;
        const char *pointer;  // nullptr: no refusal
    };

    const Case cases[] = {
        {R"({"title":"a","count":1})", 1, 256, nullptr},
        {R"({"title":["a"]})", 1, 256, "/title"},
        {R"({"extra":{"a":1}})", 1, 256, "/extra"},
        {"[[1]]", 1, 256, "/0"},
        {"[1,2,[3]]", 1, 256, "/2"},
        {R"({"extra":[[[[[[[)", 1, 256, "/extra"},
        {R"({"title":"[[{\"[","count":1})", 1, 256, nullptr},
        {R"({"title":{"a":1}})", 2, 256, nullptr},
        {R"({"title":"[[)", 1, 256, nullptr},
        {R"({"title":"x","extra":[1]})", 1, 256, "/extra"},
        {R"({"a":{"b":[1]}})", 2, 256, "/a/b"},
        {R"({"a":[0,{"b":[1]}]})", 3, 256, "/a/1/b"},
        {R"({"a/b~c":[1]})", 1, 256, "/a~1b~0c"},
        {"{[1]}", 1, 256, ""},
        {R"({"a":{"abcdef":[1]}})", 2, 4, "/a"},
        {R"({"a":{"bc":[1]}})", 2, 5, "/a/bc"},
    };
    for (const Case &c : cases) {
        const Refusal refusal = EventPayloadFields::liftTooDeep(c.text, c.depth, c.pathBound);
        if (c.pointer == nullptr) {
            EXPECT_TRUE(refusal.empty()) << c.text << ": " << refusal.reason();
            continue;
        }
        EXPECT_EQ(refusal.code(), RefusalCode::TooDeep) << c.text;
        EXPECT_EQ(refusal.pointer(), c.pointer) << c.text;
        EXPECT_EQ(refusal.bound(), std::optional<uint32_t>(c.depth)) << c.text;
    }
}

// A lift reads its record as a delivery's is read, or answers its refusal as data.
TEST(EventPayloadRefusal, ALiftReadsARecordOrAnswersItsRefusal) {
    struct Pair {
        uint8_t v = 0;
        uint8_t w = 0;
    };

    const auto read = [](const EventPayloadFields &fields, Pair &out) -> Refusal {
        Refusal refusal = fields.readInteger("v", out.v);
        return refusal.empty() ? fields.readInteger("w", out.w) : refusal;
    };
    Pair lifted;
    ASSERT_TRUE(EventPayloadFields::liftRecord(R"({"v":1})", "Pair", 1, 256, R"({"w":7})", lifted, read).empty());
    EXPECT_EQ(lifted.v, 1);
    EXPECT_EQ(lifted.w, 7);
    const std::pair<const char *, RefusalCode> refused[] = {
        {" \t", RefusalCode::NotJson},
        {"[[", RefusalCode::TooDeep},
        {"[1]", RefusalCode::NotObject},
        {R"({"v":1,"v":2})", RefusalCode::NotJson},
        {R"({"v":256,"w":1})", RefusalCode::OutOfRange},
        {R"({"w":1})", RefusalCode::Missing},
    };
    for (const auto &[text, code] : refused) {
        Pair untouched{9, 9};
        const Refusal refusal = EventPayloadFields::liftRecord(text, "Pair", 1, 256, nullptr, untouched, read);
        EXPECT_EQ(refusal.code(), code) << text << ": " << refusal.reason();
        EXPECT_FALSE(refusal.empty()) << text;
        EXPECT_EQ(untouched.v, 9) << text << ": a refused lift writes nothing";
    }
}

// A lift reads a text into a value read whole, a union's tag first and then its case's record,
// from the root of the text, or answers its refusal as data (docs/adr/0092, section 6.1 item 5).
TEST(EventPayloadRefusal, ALiftReadsAUnionWholeOrAnswersItsRefusal) {
    enum class Kind { Dot, Circle };

    struct Shape {
        Kind kind = Kind::Dot;
        uint16_t radius = 0;
    };

    const auto read = [](const EventPayloadValue &value, Shape &out) -> Refusal {
        return value.readUnion<Kind>(
            "kind",
            [](const EventPayloadValue &tag, Kind &kind) -> Refusal {
                std::string name;
                Refusal named = tag.readName(name);
                if (!named.empty()) {
                    return named;
                }
                if (name == "dot") {
                    kind = Kind::Dot;
                } else if (name == "circle") {
                    kind = Kind::Circle;
                } else {
                    return tag.unknownVariant(name, "Kind");
                }
                return Refusal{};
            },
            [&](const Kind &kind, const EventPayloadValue &object) -> Refusal {
                out = Shape{kind, 0};
                if (kind == Kind::Dot) {
                    return Refusal{};
                }
                return object.readRecord(nullptr, [&](const EventPayloadFields &record) {
                    return record.readInteger("radius", out.radius);
                });
            });
    };
    Shape lifted;
    ASSERT_TRUE(
        EventPayloadFields::liftValue(R"({"radius":4,"kind":"circle"})", "Shape", 1, 256, lifted, read).empty());
    EXPECT_EQ(lifted.kind, Kind::Circle);
    EXPECT_EQ(lifted.radius, 4);
    ASSERT_TRUE(EventPayloadFields::liftValue(R"({"kind":"dot"})", "Shape", 1, 256, lifted, read).empty());
    EXPECT_EQ(lifted.kind, Kind::Dot);

    const struct {
        const char *text;
        RefusalCode code;
        const char *pointer;
    } refused[] = {
        {" ", RefusalCode::NotJson, ""},
        {"[1]", RefusalCode::NotObject, ""},
        {R"({"radius":4})", RefusalCode::Missing, "/kind"},
        {R"({"kind":"oval"})", RefusalCode::UnknownVariant, "/kind"},
        {R"({"kind":"circle","radius":70000})", RefusalCode::OutOfRange, "/radius"},
    };

    for (const auto &c : refused) {
        Shape untouched{Kind::Circle, 9};
        const Refusal refusal = EventPayloadFields::liftValue(c.text, "Shape", 1, 256, untouched, read);
        EXPECT_EQ(refusal.code(), c.code) << c.text << ": " << refusal.reason();
        EXPECT_EQ(refusal.pointer(), c.pointer) << c.text << ": " << refusal.reason();
        EXPECT_EQ(untouched.radius, 9) << c.text << ": a refused lift writes nothing";
    }
}

// What a reader of an enum field answers for a name its enum does not declare: the
// sentence the generated reader writes, and the code and pointer beside it.
TEST(EventPayloadRefusal, ANameNoVariantDeclaresIsRefusedAsUnknownVariant) {
    const std::string data = R"({"status":"x"})";
    EventPayloadFields fields;
    ASSERT_EQ(EventPayloadFields::decode(data, fields), "");
    const Refusal refusal = fields.unknownVariant("status", "x", "Status");
    EXPECT_EQ(refusal, "'status' (x) is not a variant of Status");
    EXPECT_EQ(refusal.code(), RefusalCode::UnknownVariant);
    EXPECT_EQ(refusal.pointer(), "/status");
    EXPECT_FALSE(refusal.bound().has_value());
}
