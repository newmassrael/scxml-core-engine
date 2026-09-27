// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A string written into JSON text is written one way on every engine
// (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"). The cases live in
// tests/json_text/string_escape.json. This backend writes JSON text three ways
// — SCE::JsonText, nlohmann, and a script engine's JSON.stringify — and all
// three are held to the table here.

#include "common/JsonText.h"
#include "scripting/ScriptEngineProvider.h"

#include <fstream>
#include <gtest/gtest.h>
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

TEST(JsonText, TheSelectedEngineStringifiesTheOneForm) {
    const auto cases = loadCases();
    ASSERT_GE(cases.size(), kTableFloor);
    size_t measured = 0;
    const auto failures =
        stringifyDisagreements(SCE::ScriptEngineProvider::getScriptEngine(), "provider", cases, measured);
    EXPECT_GE(measured, kTableFloor - 1) << "fewer cases reached the engine than the table holds";
    EXPECT_TRUE(failures.empty()) << SCE::ScriptEngineProvider::getEngineName() << ":\n" << joined(failures);
}

#ifdef SCE_ENABLE_LUA
// Every Lua engine loads the same json_builtins.lua, so the C++ build's Lua
// engine measures the JSON.stringify the Rust, Go, Kotlin, Python and C11 Lua
// engines run.
TEST(JsonTextOnLuaEngine, JsonBuiltinsStringifiesTheOneForm) {
    const auto cases = loadCases();
    ASSERT_GE(cases.size(), kTableFloor);
    size_t measured = 0;
    const auto failures = stringifyDisagreements(SCE::LuaEngine::instance(), "lua", cases, measured);
    EXPECT_GE(measured, kTableFloor - 1) << "fewer cases reached the engine than the table holds";
    EXPECT_TRUE(failures.empty()) << joined(failures);
}
#endif
