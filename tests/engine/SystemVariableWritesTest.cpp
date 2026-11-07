// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief Which writes to a system variable a script engine refuses
 *
 * §scxml-5.10: "The Processor MUST cause any attempt to change the value of a
 * system variable to fail and MUST place the error 'error.execution' on the
 * internal event queue." A refused write is a script that fails, which the
 * Processor turns into `error.execution`; so each case runs its write as a
 * script, asks whether the script failed, and reads back the value the write
 * would have changed.
 *
 * Until 2026-11-07 the Lua engines answered `_event.name = 'x'` by changing
 * `_event.name`: they bound `_event` as a plain table and the system variables
 * as plain globals. W3C tests 322, 326 and 346 `<assign>` only to a whole
 * system variable, which the Processor refuses before an engine is asked.
 *
 * The expectations are not this file's. They live in
 * `tests/ecmascript/system_variable_writes.json`, which every backend's reader
 * asks, with the rule and its reasons in its `about`. Each case is asked in
 * the spelling the engine speaks natively, as `EventBlankFieldsTest` asks its
 * table.
 *
 * Two fixtures, for the reason `EventDataReadingsTest` gives: one through
 * `ScriptEngineProvider`, which measures the engine this build selected, and
 * one naming `LuaEngine`, which every build compiles and no gate selects.
 */

#include "SCXMLTypes.h"
#include "common/IOProcessorHelper.h"
#include "scripting/ScriptEngineProvider.h"
#include "scripting/ScriptSource.h"
#include <cstdint>
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
    std::string group;
    std::string write;
    std::string writeLua;
    bool refused = false;
    std::string after;
    std::string afterLua;
    std::string clause;
};

struct Table {
    std::string sessionId;
    std::string sessionName;
    std::vector<SCE::IOProcessorDescriptor> ioProcessors;
    SCE::SetCurrentEventArgs event;
    std::vector<Case> cases;
    /// How many entries the table holds, readable or not.
    size_t declared = 0;
};

/// The shared table, read from the path CMake injects, so the test reads the
/// same table under ctest, CI and an IDE run.
Table loadTable() {
    Table table;
    std::ifstream file(SCE_SYSTEM_VARIABLE_WRITES_CASES_PATH);
    if (!file) {
        ADD_FAILURE() << "cannot read the shared table at " << SCE_SYSTEM_VARIABLE_WRITES_CASES_PATH;
        return table;
    }
    nlohmann::json document;
    file >> document;

    const auto &session = document.at("session");
    table.sessionId = session.at("sessionid").get<std::string>();
    table.sessionName = session.at("name").get<std::string>();
    for (const auto &[name, location] : session.at("ioprocessors").items()) {
        table.ioProcessors.push_back({name, location.get<std::string>()});
    }

    const auto &event = document.at("event");
    table.event.eventName = event.at("name").get<std::string>();
    table.event.eventType = event.at("type").get<std::string>();
    table.event.sendId = event.at("sendid").get<std::string>();
    table.event.origin = event.at("origin").get<std::string>();
    table.event.originType = event.at("origintype").get<std::string>();
    table.event.invokeId = event.at("invokeid").get<std::string>();
    table.event.eventData = event.at("data").get<std::string>();

    const auto &entries = document.at("cases");
    table.declared = entries.size();
    for (const auto &entry : entries) {
        if (!entry.contains("refused") || !entry.at("refused").is_boolean()) {
            // A case whose expectation cannot be read is not a case that
            // passes: reading it as "not refused" would let a typo in a key
            // retire a case silently.
            ADD_FAILURE() << "a case has no readable `refused`: " << entry.dump();
            continue;
        }
        Case testCase;
        testCase.group = entry.at("group").get<std::string>();
        testCase.write = entry.at("write").get<std::string>();
        testCase.writeLua = entry.at("write_lua").get<std::string>();
        testCase.refused = entry.at("refused").get<bool>();
        testCase.after = entry.at("after").get<std::string>();
        testCase.afterLua = entry.at("after_lua").get<std::string>();
        testCase.clause = entry.at("clause").get<std::string>();
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

bool nativeLua(const SCE::IScriptEngine &engine) {
    return engine.nativeLanguage() == SCE::ScriptLanguage::Lua;
}

SCE::ScriptSource spelling(const SCE::IScriptEngine &engine, const std::string &source, const std::string &lua) {
    return nativeLua(engine) ? SCE::ScriptSource::lua(lua, source) : SCE::ScriptSource::ecmascript(source);
}

/// Ask every case of one engine and collect what disagreed. Every case is
/// reported rather than the first, so one run names every write that landed.
std::vector<std::string> disagreements(SCE::IScriptEngine &engine, const Table &table) {
    std::vector<std::string> failures;
    for (const Case &testCase : table.cases) {
        const std::string label = testCase.group + " [" + testCase.write + "]";
        // Every case in a session of the table's own id, so `_sessionid` reads
        // as the table says; one at a time, so the id is never held twice.
        if (!engine.createSession(table.sessionId, "")) {
            failures.push_back(label + ": no session");
            continue;
        }
        auto system = engine.setupSystemVariables(table.sessionId, table.sessionName, table.ioProcessors).get();
        auto bound = engine.setCurrentEvent(table.sessionId, table.event).get();
        if (!system.isSuccess() || !bound.status.isSuccess()) {
            failures.push_back(label + ": the session's system variables or its event could not be bound: " +
                               system.getErrorMessage() + " " + bound.status.getErrorMessage());
            engine.destroySession(table.sessionId);
            continue;
        }

        auto written = engine.executeScript(table.sessionId, spelling(engine, testCase.write, testCase.writeLua)).get();
        if (written.isSuccess() == testCase.refused) {
            failures.push_back(label + ": " + (testCase.refused ? "ran" : "was refused: " + written.getErrorMessage()) +
                               ", where " + testCase.clause + (testCase.refused ? " refuses it" : " allows it"));
        }

        auto after =
            engine.evaluateExpression(table.sessionId, spelling(engine, testCase.after, testCase.afterLua)).get();
        if (!after.isSuccess()) {
            failures.push_back(label + ": the value afterwards could not be read: " + after.getErrorMessage());
        } else {
            const ::ScriptValue &actual = after.getInternalValue();
            if (!(std::holds_alternative<bool>(actual) && std::get<bool>(actual))) {
                failures.push_back(label + ": afterwards [" + (nativeLua(engine) ? testCase.afterLua : testCase.after) +
                                   "] answered " + describe(actual) + ", so the write " +
                                   (testCase.refused ? "landed although the script was refused" : "did not land"));
            }
        }
        engine.destroySession(table.sessionId);
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
    ASSERT_GT(table.declared, 0u) << "the shared system-variable table holds no case, so this measures nothing";
    ASSERT_EQ(table.cases.size(), table.declared)
        << "read " << table.cases.size() << " of the " << table.declared << " cases the shared table holds";
}

}  // namespace

TEST(SystemVariableWrites, TheSelectedEngineRefusesAWriteToASystemVariable) {
    const auto table = loadTable();
    assertTableWasRead(table);

    auto &engine = SCE::ScriptEngineProvider::getScriptEngine();
    const auto failures = disagreements(engine, table);
    EXPECT_TRUE(failures.empty()) << failures.size() << " of " << table.cases.size()
                                  << " system-variable cases disagree, evaluated by "
                                  << SCE::ScriptEngineProvider::getEngineName()
                                  << " (SCE_SCRIPT_ENGINE=" << SCE::ScriptEngineProvider::getEngineId() << ").\n"
                                  << joined(failures);
}

#ifdef SCE_ENABLE_LUA
TEST(SystemVariableWritesOnLuaEngine, TheLuaEngineRefusesAWriteToASystemVariable) {
    const auto table = loadTable();
    assertTableWasRead(table);

    auto &engine = SCE::LuaEngine::instance();
    const auto failures = disagreements(engine, table);
    EXPECT_TRUE(failures.empty()) << failures.size() << " of " << table.cases.size()
                                  << " system-variable cases disagree on the Lua engine.\n"
                                  << joined(failures);
}

/// A system variable read WHOLE reaches the host with its contents: a
/// read-only view's own table is empty, so a raw copy of it would hand
/// `<data id="Var1" expr="_ioprocessors"/>` (W3C test 326) an empty object.
TEST(SystemVariableWritesOnLuaEngine, ASystemVariableReadWholeCarriesItsContents) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "lua_system_whole";
    ASSERT_TRUE(engine.createSession(sessionId, ""));
    ASSERT_TRUE(engine.setupSystemVariables(sessionId, "machine", {{"scxml", "sce://scxml/lua_system_whole"}})
                    .get()
                    .isSuccess());
    SCE::SetCurrentEventArgs event;
    event.eventName = "brief";
    event.eventType = "external";
    ASSERT_TRUE(engine.setCurrentEvent(sessionId, event).get().status.isSuccess());

    auto processors =
        engine.evaluateExpression(sessionId, SCE::ScriptSource::lua("_ioprocessors", "_ioprocessors")).get();
    ASSERT_TRUE(processors.isSuccess()) << processors.getErrorMessage();
    const auto &whole = processors.getInternalValue();
    ASSERT_TRUE(std::holds_alternative<std::shared_ptr<::ScriptObject>>(whole)) << "_ioprocessors is no object";
    const auto &entries = std::get<std::shared_ptr<::ScriptObject>>(whole)->properties;
    ASSERT_EQ(entries.count("scxml"), 1u) << "_ioprocessors read whole lost its `scxml` entry";
    const auto &entry = entries.at("scxml");
    ASSERT_TRUE(std::holds_alternative<std::shared_ptr<::ScriptObject>>(entry)) << "the `scxml` entry is no object";
    const auto &fields = std::get<std::shared_ptr<::ScriptObject>>(entry)->properties;
    ASSERT_EQ(fields.count("location"), 1u) << "the `scxml` entry read whole lost its `location`";
    EXPECT_EQ(describe(fields.at("location")), "\"sce://scxml/lua_system_whole\"");

    auto current = engine.evaluateExpression(sessionId, SCE::ScriptSource::lua("_event", "_event")).get();
    ASSERT_TRUE(current.isSuccess()) << current.getErrorMessage();
    ASSERT_TRUE(std::holds_alternative<std::shared_ptr<::ScriptObject>>(current.getInternalValue()))
        << "_event is no object";
    const auto &eventFields = std::get<std::shared_ptr<::ScriptObject>>(current.getInternalValue())->properties;
    ASSERT_EQ(eventFields.count("name"), 1u) << "_event read whole lost its `name`";
    EXPECT_EQ(describe(eventFields.at("name")), "\"brief\"");

    engine.destroySession(sessionId);
}

/// A variable a document names — `<data id>`, `<assign location>` — is set
/// through the engine rather than by a script, and when that name is a system
/// variable's the write is refused there too, as an error the engine returns:
/// an unprotected global write would have aborted the process on the refusal.
TEST(SystemVariableWritesOnLuaEngine, SettingASystemVariableByNameIsRefusedAsAnError) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "lua_system_by_name";
    ASSERT_TRUE(engine.createSession(sessionId, ""));
    ASSERT_TRUE(engine.setupSystemVariables(sessionId, "machine", {}).get().isSuccess());

    auto refused = engine.setVariable(sessionId, "_sessionid", ::ScriptValue{std::string("x")}).get();
    EXPECT_FALSE(refused.isSuccess()) << "setting _sessionid by name was accepted";

    auto kept = engine
                    .evaluateExpression(sessionId, SCE::ScriptSource::lua("_sessionid == 'lua_system_by_name'",
                                                                          "_sessionid === 'lua_system_by_name'"))
                    .get();
    ASSERT_TRUE(kept.isSuccess()) << kept.getErrorMessage();
    EXPECT_TRUE(std::holds_alternative<bool>(kept.getInternalValue()) && std::get<bool>(kept.getInternalValue()))
        << "_sessionid changed: " << describe(kept.getInternalValue());

    auto allowed = engine.setVariable(sessionId, "v", ::ScriptValue{int64_t{1}}).get();
    EXPECT_TRUE(allowed.isSuccess()) << "setting a document's own variable was refused: " << allowed.getErrorMessage();

    engine.destroySession(sessionId);
}
#endif
