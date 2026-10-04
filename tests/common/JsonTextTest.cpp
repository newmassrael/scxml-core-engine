// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A string written into JSON text is written one way on every engine
// (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"). The cases live in
// tests/json_text/string_escape.json. This backend writes JSON text three ways
// — SCE::JsonText, nlohmann, and a script engine's JSON.stringify — and all
// three are held to the table here.
//
// The members of a JSON object written into `_event.data` come in one order on
// every engine (ARCHITECTURE.md, "JSON Object Key Order"). The cases live in
// tests/json_text/object_key_order.json, and the core's writer of them,
// EventDataHelper, is held to it below.

#include "common/JsonText.h"
#include "common/EventDataHelper.h"
#include "common/EventPayloadLift.h"
#include "scripting/ScriptEngineProvider.h"
#include "scripting/ScriptResultUtils.h"

#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <gtest/gtest.h>
#include <limits>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <string>
#include <variant>
#include <vector>

#ifdef SCE_ENABLE_LUA
#include "scripting/LuaEngine.h"
#endif

namespace {

struct Case {
    std::string name;
    std::string text;
    std::string escaped;
};

// A floor, not an equality: adding a case must not have to touch it, but a
// table that stopped being read must not pass either.
constexpr size_t kTableFloor = 8;

std::vector<Case> loadCases() {
    std::ifstream file(SCE_JSON_TEXT_TABLE);
    const auto table = nlohmann::json::parse(file);
    std::vector<Case> cases;
    for (const auto &row : table.at("cases")) {
        cases.push_back({row.at("name").get<std::string>(), row.at("text").get<std::string>(),
                         row.at("escaped").get<std::string>()});
    }
    return cases;
}

// What `JSON.stringify(v)` answers on `engine`, for every case a script
// engine can be handed: a text holding U+0000 cannot, because both engine
// bindings pass a string across as a C string and it ends at the NUL. Chosen
// by that property rather than listed, so a new case is measured unless it has
// it.
std::vector<std::string> stringifyDisagreements(SCE::IScriptEngine &engine, const std::string &prefix,
                                                const std::vector<Case> &cases, size_t &measured) {
    std::vector<std::string> failures;
    measured = 0;
    for (size_t index = 0; index < cases.size(); ++index) {
        const Case &c = cases[index];
        if (c.text.find('\0') != std::string::npos) {
            continue;
        }
        measured++;
        const std::string session = prefix + "_json_text_" + std::to_string(index);
        if (!engine.createSession(session, "")) {
            failures.push_back(c.name + ": no session");
            continue;
        }
        const auto set = engine.setVariable(session, "v", ::ScriptValue{c.text}).get();
        const auto result = engine.evaluateExpression(session, "JSON.stringify(v)").get();
        if (!set.isSuccess()) {
            failures.push_back(c.name + ": the text was not handed to the engine: " + set.getErrorMessage());
        } else if (!result.isSuccess()) {
            failures.push_back(c.name + ": JSON.stringify failed: " + result.getErrorMessage());
        } else if (!std::holds_alternative<std::string>(result.getInternalValue()) ||
                   std::get<std::string>(result.getInternalValue()) != "\"" + c.escaped + "\"") {
            failures.push_back(c.name + ": JSON.stringify did not write the table's form");
        }
        engine.destroySession(session);
    }
    return failures;
}

// A value of the key-order table as the data model holds it. A number in the
// table is a whole number, so the cases do not depend on how an engine spells
// a fraction.
::ScriptValue held(const nlohmann::json &value) {
    if (value.is_null()) {
        return ::ScriptNull{};
    }
    if (value.is_boolean()) {
        return value.get<bool>();
    }
    if (value.is_number_integer()) {
        return value.get<int64_t>();
    }
    if (value.is_string()) {
        return value.get<std::string>();
    }
    if (value.is_array()) {
        auto array = std::make_shared<::ScriptArray>();
        for (const auto &item : value) {
            array->elements.push_back(held(item));
        }
        return array;
    }
    auto object = std::make_shared<::ScriptObject>();
    for (const auto &[key, item] : value.items()) {
        object->properties.emplace(key, held(item));
    }
    return object;
}

std::string joined(const std::vector<std::string> &lines) {
    std::string out;
    for (const auto &line : lines) {
        out += "  " + line + "\n";
    }
    return out;
}

}  // namespace

TEST(JsonText, TheCoreWritesTheOneForm) {
    const auto cases = loadCases();
    ASSERT_GE(cases.size(), kTableFloor) << "the table was not read from " << SCE_JSON_TEXT_TABLE;
    for (const auto &c : cases) {
        EXPECT_EQ(SCE::JsonText::escaped(c.text), c.escaped) << c.name;
        EXPECT_EQ(SCE::JsonText::quoted(c.text), "\"" + c.escaped + "\"") << c.name;
    }
}

// The form is nlohmann's, which is why the core's own writer can match it
// byte for byte; this is the claim ARCHITECTURE.md makes, measured.
TEST(JsonText, NlohmannWritesTheSameForm) {
    const auto cases = loadCases();
    ASSERT_GE(cases.size(), kTableFloor);
    for (const auto &c : cases) {
        EXPECT_EQ(nlohmann::json(c.text).dump(), "\"" + c.escaped + "\"") << c.name;
    }
}

// The params a `<send>` evaluated, a name that repeats collecting its values in
// declaration order (W3C test178), written as `_event.data`.
TEST(JsonText, TheCoreWritesObjectMembersInTheOneOrder) {
    std::ifstream file(SCE_JSON_KEY_ORDER_TABLE);
    const auto table = nlohmann::json::parse(file);
    const auto &cases = table.at("cases");
    ASSERT_GE(cases.size(), kTableFloor) << "the table was not read from " << SCE_JSON_KEY_ORDER_TABLE;
    for (const auto &row : cases) {
        std::map<std::string, std::vector<::ScriptValue>> evaluated;
        for (const auto &pair : row.at("params")) {
            evaluated[pair.at(0).get<std::string>()].push_back(held(pair.at(1)));
        }
        EXPECT_EQ(SCE::EventDataHelper::buildJsonFromTypedParams(evaluated), row.at("data").get<std::string>())
            << row.at("name").get<std::string>();
    }
}

namespace {

struct RealCase {
    std::string name;
    double value;
    std::string text;
};

// The cases of tests/json_text/real_text.json, each value rebuilt from the IEEE
// 754 bit pattern the table gives so that no number parser stands between the
// table and the double.
std::vector<RealCase> loadRealCases() {
    std::ifstream file(SCE_JSON_REAL_TEXT_TABLE);
    const auto table = nlohmann::json::parse(file);
    std::vector<RealCase> cases;
    for (const auto &row : table.at("cases")) {
        const auto bits = static_cast<uint64_t>(std::stoull(row.at("bits").get<std::string>(), nullptr, 16));
        double value = 0.0;
        static_assert(sizeof(value) == sizeof(bits));
        std::memcpy(&value, &bits, sizeof(value));
        cases.push_back({row.at("name").get<std::string>(), value, row.at("text").get<std::string>()});
    }
    return cases;
}

constexpr size_t kRealTableFloor = 40;

// A power of two has a nearer neighbour below it than above, so the nearest
// decimal of the fewest digits is not always one that reads back, and a writer
// that rounds first and checks after writes one digit more (QuickJS writes 2^-44
// as 5.6843418860808015e-14 where the shortest reading back is
// 5.684341886080802e-14).
bool isPowerOfTwo(double value) {
    int exponent = 0;
    return std::frexp(std::fabs(value), &exponent) == 0.5;
}

// What `JSON.stringify(v)` answers on `engine` for each float of the table.
// `heldToShortestOnPowersOfTwo` is false for an engine this tree does not own
// (QuickJS, whose `JSON.stringify` is not a writer of SCE's wire text): a power
// of two it writes with one digit more is accepted when that text reads back as
// the same value, and every other row is still held to the table.
std::vector<std::string> stringifyFloatDisagreements(SCE::IScriptEngine &engine, const std::string &prefix,
                                                     const std::vector<RealCase> &cases,
                                                     bool heldToShortestOnPowersOfTwo) {
    std::vector<std::string> failures;
    for (size_t index = 0; index < cases.size(); ++index) {
        const RealCase &c = cases[index];
        const std::string session = prefix + "_json_real_" + std::to_string(index);
        if (!engine.createSession(session, "")) {
            failures.push_back(c.name + ": no session");
            continue;
        }
        const auto set = engine.setVariable(session, "v", ::ScriptValue{c.value}).get();
        const auto result = engine.evaluateExpression(session, "JSON.stringify(v)").get();
        if (!set.isSuccess()) {
            failures.push_back(c.name + ": the value was not handed to the engine: " + set.getErrorMessage());
        } else if (!result.isSuccess()) {
            failures.push_back(c.name + ": JSON.stringify failed: " + result.getErrorMessage());
        } else if (!std::holds_alternative<std::string>(result.getInternalValue())) {
            failures.push_back(c.name + ": JSON.stringify did not return a string");
        } else if (const auto &written = std::get<std::string>(result.getInternalValue()); written != c.text) {
            const bool readsBack = std::strtod(written.c_str(), nullptr) == c.value;
            if (heldToShortestOnPowersOfTwo || !isPowerOfTwo(c.value) || !readsBack) {
                failures.push_back(c.name + ": JSON.stringify wrote " + written + ", the table's text is " + c.text);
            }
        }
        engine.destroySession(session);
    }
    return failures;
}

}  // namespace

// A float is spelled the way ECMAScript spells it on every engine
// (ARCHITECTURE.md, "JSON Number Text"). This backend writes one through
// SCE::JsonText, the compact JSON writer, the text of an untyped `<param>` (a
// scalar and an element of an array) and a typed payload field, and all five
// are held to the table.
TEST(JsonText, TheCoreSpellsAFloatAsEcmaScriptSpellsIt) {
    const auto cases = loadRealCases();
    ASSERT_GE(cases.size(), kRealTableFloor) << "the table was not read from " << SCE_JSON_REAL_TEXT_TABLE;
    for (const auto &c : cases) {
        EXPECT_EQ(SCE::JsonText::numberText(c.value), c.text) << c.name;
        EXPECT_EQ(SCE::EventDataHelper::scriptValueToJsonString(::ScriptValue{c.value}), c.text)
            << c.name << ": as JSON";
        EXPECT_EQ(SCE::ScriptResultUtils::resultToString(SCE::ScriptResult::createSuccess(::ScriptValue{c.value})),
                  c.text)
            << c.name << ": as an untyped param";
        EXPECT_EQ(SCE::Common::EventPayloadFields::field("x", c.value), "\"x\":" + c.text)
            << c.name << ": as a payload field";
        const auto array = std::make_shared<::ScriptArray>(std::vector<::ScriptValue>{::ScriptValue{c.value}});
        EXPECT_EQ(SCE::ScriptResultUtils::resultToStringArray(SCE::ScriptResult::createSuccess(::ScriptValue{array})),
                  std::vector<std::string>{c.text})
            << c.name << ": as an element of an untyped param's array";
    }
}

// JSON has no spelling for a float that is not finite; the untyped wire text
// gives it the one ECMAScript does.
TEST(JsonText, AFloatThatIsNotFiniteHasNoJsonSpellingAndAWireOne) {
    const struct {
        double value;
        const char *wire;
    } nonFinite[] = {{std::numeric_limits<double>::quiet_NaN(), "NaN"},
                     {std::numeric_limits<double>::infinity(), "Infinity"},
                     {-std::numeric_limits<double>::infinity(), "-Infinity"}};

    for (const auto &c : nonFinite) {
        EXPECT_EQ(SCE::EventDataHelper::scriptValueToJsonString(::ScriptValue{c.value}), "null") << c.wire;
        EXPECT_EQ(SCE::ScriptResultUtils::resultToString(SCE::ScriptResult::createSuccess(::ScriptValue{c.value})),
                  c.wire);
        EXPECT_EQ(SCE::Common::EventPayloadFields::field("x", c.value), "\"x\":null") << c.wire;
    }
}

TEST(JsonText, TheSelectedEngineStringifiesTheOneForm) {
    const auto cases = loadCases();
    ASSERT_GE(cases.size(), kTableFloor);
    size_t measured = 0;
    const auto failures =
        stringifyDisagreements(SCE::ScriptEngineProvider::getScriptEngine(), "provider", cases, measured);
    EXPECT_GE(measured, kTableFloor - 1) << "fewer cases reached the engine than the table holds";
    EXPECT_TRUE(failures.empty()) << SCE::ScriptEngineProvider::getEngineName() << ":\n" << joined(failures);
}

TEST(JsonText, TheSelectedEngineStringifiesAFloatTheSameWay) {
    const auto cases = loadRealCases();
    ASSERT_GE(cases.size(), kRealTableFloor);
    const auto failures =
        stringifyFloatDisagreements(SCE::ScriptEngineProvider::getScriptEngine(), "provider_real", cases, false);
    EXPECT_TRUE(failures.empty()) << SCE::ScriptEngineProvider::getEngineName() << ":\n" << joined(failures);
}

#ifdef SCE_ENABLE_LUA
// Every Lua engine loads the same json_builtins.lua, so the C++ build's Lua
// engine measures the JSON.stringify the Rust, Go, Kotlin, Python and C11 Lua
// engines run.
TEST(JsonTextOnLuaEngine, JsonBuiltinsStringifiesAFloatTheSameWay) {
    const auto cases = loadRealCases();
    ASSERT_GE(cases.size(), kRealTableFloor);
    const auto failures = stringifyFloatDisagreements(SCE::LuaEngine::instance(), "lua_real", cases, true);
    EXPECT_TRUE(failures.empty()) << joined(failures);
}

TEST(JsonTextOnLuaEngine, JsonBuiltinsStringifiesTheOneForm) {
    const auto cases = loadCases();
    ASSERT_GE(cases.size(), kTableFloor);
    size_t measured = 0;
    const auto failures = stringifyDisagreements(SCE::LuaEngine::instance(), "lua", cases, measured);
    EXPECT_GE(measured, kTableFloor - 1) << "fewer cases reached the engine than the table holds";
    EXPECT_TRUE(failures.empty()) << joined(failures);
}
#endif
