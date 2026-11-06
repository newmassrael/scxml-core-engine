// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief What an `_event` field the Processor left blank reads as
 *
 * §scxml-5.10.1 tells the Processor to leave `sendid`, `origin`, `origintype`
 * and `invokeid` blank in named cases, and §scxml-B-2-8 says what blank is in
 * the ECMAScript data model: "the Processor MUST set the value to ECMAScript
 * undefined". Not the empty string. An engine that lowers ECMAScript to Lua
 * holds undefined as nil.
 *
 * Every engine interface of this backend carries those fields as strings, and
 * the empty string means "none", so the rule lives in the binding that builds
 * `_event`. Until 2026-11-07 the QuickJS binding bound `''` for every blank
 * field, below a comment stating the rule, and nothing failed: W3C tests 333
 * and 339 ask the fields as conditions, where `''` and undefined are both
 * false.
 *
 * The expectations are not this file's. They live in
 * `tests/ecmascript/event_blank_fields.json`, which every backend's reader
 * asks. Each case is asked in the spelling the engine speaks natively: the
 * author's ECMAScript (`source`) for QuickJS, the table's own Lua (`lua`) for
 * an engine whose native language is Lua, so what is measured is the binding
 * and not the lowering.
 *
 * Two fixtures, for the reason `EventDataReadingsTest` gives: one through
 * `ScriptEngineProvider`, which measures the engine this build selected, and
 * one naming `LuaEngine`, which every build compiles and no gate selects.
 */

#include "SCXMLTypes.h"
#include "scripting/ScriptEngineProvider.h"
#include "scripting/ScriptSource.h"
#include <cstdint>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <nlohmann/json.hpp>
#include <string>
#include <variant>
#include <vector>

#ifdef SCE_ENABLE_LUA
#include "scripting/LuaEngine.h"
#endif

#ifdef SCE_ENABLE_QUICKJS
#include "scripting/JSEngine.h"
#endif

namespace {

struct Case {
    std::string group;
    /// The event the binding is handed. An absent member of the table's
    /// `event` is a field the Processor left blank, which the engine
    /// interface carries as the empty string.
    SCE::SetCurrentEventArgs event;
    /// The author's ECMAScript, and its Lua spelling.
    std::string source;
    std::string lua;
    std::string clause;
    bool expected = false;
};

struct Table {
    std::vector<Case> cases;
    /// How many entries the table holds, readable or not.
    size_t declared = 0;
};

/// A member of the table's `event`, or the empty string when it is absent.
std::string fieldOf(const nlohmann::json &event, const char *member) {
    return event.contains(member) ? event.at(member).get<std::string>() : std::string();
}

/// The shared table, read from the path CMake injects, so the test reads the
/// same table under ctest, CI and an IDE run.
Table loadTable() {
    Table table;
    std::ifstream file(SCE_EVENT_BLANK_FIELDS_CASES_PATH);
    if (!file) {
        ADD_FAILURE() << "cannot read the shared table at " << SCE_EVENT_BLANK_FIELDS_CASES_PATH;
        return table;
    }
    nlohmann::json document;
    file >> document;

    const auto &entries = document.at("cases");
    table.declared = entries.size();
    for (const auto &entry : entries) {
        Case testCase;
        testCase.group = entry.at("group").get<std::string>();
        testCase.source = entry.at("source").get<std::string>();
        testCase.lua = entry.at("lua").get<std::string>();
        testCase.clause = entry.at("clause").get<std::string>();

        const auto &event = entry.at("event");
        testCase.event.eventName = event.at("name").get<std::string>();
        testCase.event.eventType = event.at("type").get<std::string>();
        testCase.event.sendId = fieldOf(event, "sendid");
        testCase.event.invokeId = fieldOf(event, "invokeid");
        testCase.event.origin = fieldOf(event, "origin");
        testCase.event.originType = fieldOf(event, "origintype");

        const auto &expect = entry.at("expect");
        if (!expect.contains("bool") || !expect.at("bool").is_boolean()) {
            // A case whose expectation cannot be read is not a case that
            // passes: reading it as "no answer" would let a typo in a key
            // retire a case silently.
            ADD_FAILURE() << "case [" << testCase.source << "] has no readable expectation";
            continue;
        }
        testCase.expected = expect.at("bool").get<bool>();
        table.cases.push_back(std::move(testCase));
    }
    return table;
}

std::string describe(const ::ScriptValue &value) {
    if (std::holds_alternative<::ScriptUndefined>(value)) {
        return "undefined";
    }
    if (std::holds_alternative<::ScriptNull>(value)) {
        return "null";
    }
    if (std::holds_alternative<bool>(value)) {
        return std::get<bool>(value) ? "true" : "false";
    }
    if (std::holds_alternative<std::string>(value)) {
        return "\"" + std::get<std::string>(value) + "\"";
    }
    return "a value that is no boolean";
}

/// The case in the language @p engine evaluates without adapting anything.
SCE::ScriptSource spellingFor(const SCE::IScriptEngine &engine, const Case &testCase) {
    if (engine.nativeLanguage() == SCE::ScriptLanguage::Lua) {
        return SCE::ScriptSource::lua(testCase.lua, testCase.source);
    }
    return SCE::ScriptSource::ecmascript(testCase.source);
}

/// Ask every case of one engine and collect what disagreed.
///
/// Every case is reported rather than the first: a binding that binds `''`
/// for one field and nothing for another is two defects, and one failure
/// cannot separate them.
std::vector<std::string> disagreements(SCE::IScriptEngine &engine, const std::string &prefix,
                                       const std::vector<Case> &cases) {
    std::vector<std::string> failures;
    for (size_t index = 0; index < cases.size(); ++index) {
        const Case &testCase = cases[index];
        const std::string sessionId = prefix + "_blank_" + std::to_string(index);
        const std::string label = testCase.group + " [" + testCase.source + "]";
        if (!engine.createSession(sessionId, "")) {
            failures.push_back(label + ": no session");
            continue;
        }
        auto bound = engine.setCurrentEvent(sessionId, testCase.event).get();
        if (!bound.status.isSuccess()) {
            failures.push_back(label + ": setCurrentEvent failed: " + bound.status.getErrorMessage());
            engine.destroySession(sessionId);
            continue;
        }
        auto result = engine.evaluateExpression(sessionId, spellingFor(engine, testCase)).get();
        if (!result.isSuccess()) {
            failures.push_back(label + ": did not evaluate: " + result.getErrorMessage() + " (" + testCase.clause +
                               ")");
        } else {
            const ::ScriptValue &actual = result.getInternalValue();
            const bool agrees = std::holds_alternative<bool>(actual) && std::get<bool>(actual) == testCase.expected;
            if (!agrees) {
                failures.push_back(label + ": answered " + describe(actual) + ", " + testCase.clause + " says " +
                                   (testCase.expected ? "true" : "false"));
            }
        }
        engine.destroySession(sessionId);
    }
    return failures;
}

std::string joined(const std::vector<std::string> &failures) {
    std::string text;
    for (const auto &failure : failures) {
        text += failure + "\n";
    }
    return text;
}

/// Every entry of the table was read, and the table was not empty: a table
/// that stopped being read, or a case that stopped parsing, must not pass.
void assertTableWasRead(const Table &table) {
    ASSERT_GT(table.declared, 0u) << "the shared blank-field table holds no case, so this measures nothing";
    ASSERT_EQ(table.cases.size(), table.declared)
        << "read " << table.cases.size() << " of the " << table.declared << " cases the shared table holds";
}

}  // namespace

TEST(EventBlankFields, TheSelectedEngineBindsABlankFieldAsUndefined) {
    const auto table = loadTable();
    assertTableWasRead(table);

    auto &engine = SCE::ScriptEngineProvider::getScriptEngine();
    const auto failures = disagreements(engine, "provider", table.cases);
    EXPECT_TRUE(failures.empty()) << failures.size() << " of " << table.cases.size()
                                  << " blank-field cases disagree, evaluated by "
                                  << SCE::ScriptEngineProvider::getEngineName()
                                  << " (SCE_SCRIPT_ENGINE=" << SCE::ScriptEngineProvider::getEngineId() << ").\n"
                                  << joined(failures);
}

#ifdef SCE_ENABLE_LUA
TEST(EventBlankFieldsOnLuaEngine, TheLuaEngineBindsABlankFieldAsNil) {
    const auto table = loadTable();
    assertTableWasRead(table);

    auto &engine = SCE::LuaEngine::instance();
    const auto failures = disagreements(engine, "lua", table.cases);
    EXPECT_TRUE(failures.empty()) << failures.size() << " of " << table.cases.size()
                                  << " blank-field cases disagree on the Lua engine.\n"
                                  << joined(failures);
}

/// The binding's second route: an event whose payload is already a value
/// (`Event::setTypedData`) is bound without the string path, so the table,
/// which reaches the engine through `SetCurrentEventArgs`, never asks it.
TEST(EventBlankFieldsOnLuaEngine, AnEventWithTypedDataBindsItsBlankFieldsAsNil) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "lua_blank_typed";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    auto event = std::make_shared<SCE::Event>("brief", "external");
    event->setInvokeId("worker");
    event->setTypedData(::ScriptValue{int64_t{1}});
    auto bound = engine.setCurrentEvent(sessionId, event).get();
    ASSERT_TRUE(bound.status.isSuccess()) << bound.status.getErrorMessage();

    const std::string lua = "_event.sendid == nil and _event.origin == nil and _event.origintype == nil "
                            "and _event.invokeid == 'worker'";
    auto result = engine.evaluateExpression(sessionId, SCE::ScriptSource::lua(lua, lua)).get();
    ASSERT_TRUE(result.isSuccess()) << result.getErrorMessage();
    const ::ScriptValue &actual = result.getInternalValue();
    EXPECT_TRUE(std::holds_alternative<bool>(actual) && std::get<bool>(actual))
        << "a typed-data event read its blank fields as " << describe(actual)
        << " where nil was due, or lost the invokeid it carried";

    engine.destroySession(sessionId);
}
#endif

#ifdef SCE_ENABLE_QUICKJS
/// Undefined is a value of the field, not an absence of it: `_event` keeps a
/// property for every field of §scxml-5.10.1 (W3C test 330 asks `'sendid' in
/// _event`), and a blank one reads as undefined.
TEST(EventBlankFieldsOnQuickJs, ABlankFieldIsStillAPropertyOfEvent) {
    auto &engine = SCE::JSEngine::instance();
    const std::string sessionId = "quickjs_blank_present";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    SCE::SetCurrentEventArgs args;
    args.eventName = "brief";
    args.eventType = "internal";
    auto bound = engine.setCurrentEvent(sessionId, args).get();
    ASSERT_TRUE(bound.status.isSuccess()) << bound.status.getErrorMessage();

    auto result = engine
                      .evaluateExpression(sessionId, "'sendid' in _event && 'origin' in _event && "
                                                     "'origintype' in _event && 'invokeid' in _event && "
                                                     "typeof _event.sendid === 'undefined'")
                      .get();
    ASSERT_TRUE(result.isSuccess()) << result.getErrorMessage();
    const ::ScriptValue &actual = result.getInternalValue();
    EXPECT_TRUE(std::holds_alternative<bool>(actual) && std::get<bool>(actual))
        << "a blank field is missing from _event or is not undefined: " << describe(actual);

    engine.destroySession(sessionId);
}

/// Clearing the current event leaves no event, so every field that can be
/// blank reads as undefined again, not as the empty string or as the value
/// the previous event carried.
TEST(EventBlankFieldsOnQuickJs, AClearedEventReadsItsBlankFieldsAsUndefined) {
    auto &engine = SCE::JSEngine::instance();
    const std::string sessionId = "quickjs_blank_cleared";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    SCE::SetCurrentEventArgs args;
    args.eventName = "brief";
    args.eventType = "external";
    args.sendId = "s-1";
    args.origin = "sce://scxml/peer-1";
    args.originType = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";
    args.invokeId = "worker";
    auto bound = engine.setCurrentEvent(sessionId, args).get();
    ASSERT_TRUE(bound.status.isSuccess()) << bound.status.getErrorMessage();

    auto cleared = engine.setCurrentEvent(sessionId, std::shared_ptr<SCE::Event>()).get();
    ASSERT_TRUE(cleared.status.isSuccess()) << cleared.status.getErrorMessage();

    auto result = engine
                      .evaluateExpression(sessionId, "_event.sendid === undefined && _event.origin === undefined && "
                                                     "_event.origintype === undefined && "
                                                     "_event.invokeid === undefined && _event.name === ''")
                      .get();
    ASSERT_TRUE(result.isSuccess()) << result.getErrorMessage();
    const ::ScriptValue &actual = result.getInternalValue();
    EXPECT_TRUE(std::holds_alternative<bool>(actual) && std::get<bool>(actual))
        << "a cleared event did not read its blank fields as undefined: " << describe(actual);

    engine.destroySession(sessionId);
}
#endif
