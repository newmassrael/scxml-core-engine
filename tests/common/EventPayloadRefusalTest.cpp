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
        return v.readList(ty["max"].get<size_t>(), items,
                          [&](const EventPayloadValue &element, int &) { return readDescribed(ty["list"], element); });
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
        Refusal refusal = EventPayloadFields::decode(data, fields);
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
    EXPECT_EQ(declared, 13u);
    for (unsigned number = 1; number <= 13u; ++number) {
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
