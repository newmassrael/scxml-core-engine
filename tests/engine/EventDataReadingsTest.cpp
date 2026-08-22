// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief Which reading an arriving `_event.data` payload gets
 *
 * §scxml-B-2-8-1 names four readings and orders them: key-value pairs become
 * named properties, otherwise JSON becomes the corresponding object,
 * otherwise *"if the Processor can interpret the content as a valid XML
 * document, it MUST create the corresponding DOM structure"*, and then the
 * sentence that closes it — *"Otherwise, the Processor MUST treat the content
 * as a space-normalized string literal"*.
 *
 * That final sentence is what this file exists for. A leading `<` is a GUESS
 * about which reading applies, and six of the eight SCE script engines treated
 * the guess as the answer. Measured 2026-08-19, for a payload that opens with
 * `<` and is not a well-formed document, this backend's two engines answered
 * `undefined` (QuickJS, plus an ERROR log line) and nil (Lua) — while its
 * Python and Kotlin siblings answered the string the clause names.
 *
 * It went unnoticed because nothing sent such a payload: the W3C IRP suite's
 * XML payloads are all well-formed. Then the repository filled `_event.data`
 * in at 192 `error.*` raise sites with messages that name the failing
 * construct — `<assign> to detail failed` — so every platform error opens with
 * `<`, and three backends delivered nothing at all.
 *
 * The expectations are not this file's. They live in
 * `tests/ecmascript/event_data_readings.json`, one payload per case with the
 * sentence of the clause that decides it, and the Rust, Go, Python and Kotlin
 * readers ask the same file — a per-backend copy drifts toward whichever
 * backend reads it. What is asked here is the `source` spelling: the author's
 * own ECMAScript, which is what both C++ engines are handed (the Lua one
 * rewrites it itself).
 *
 * Two fixtures, because this backend ships two bindings, and the reasoning is
 * `DomReadSurfaceTest`'s next door: `EventDataReadings` goes through
 * `ScriptEngineProvider` and so measures the engine this build selected, while
 * `EventDataReadingsOnLuaEngine` names `LuaEngine` on purpose — no gate
 * configures `-DSCE_SCRIPT_ENGINE=lua`, so that engine is compiled by every
 * build and run by none, which is exactly how it kept a reading its four
 * siblings had already lost.
 */

#include "SCXMLTypes.h"
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
    /// What the event carried.
    std::string payload;
    /// The author's ECMAScript, asked of the `_event` the binding built.
    std::string source;
    std::string clause;
    /// Exactly one of these carries the answer.
    enum class Kind { Number, Text, Boolean, Empty } kind = Kind::Empty;
    double number = 0.0;
    std::string text;
    bool boolean = false;
};

/// The shared table, read from the path CMake injects.
///
/// Injected rather than resolved from the working directory, so the test reads
/// the same table under ctest, CI and an IDE run.
std::vector<Case> loadCases() {
    std::ifstream file(SCE_EVENT_DATA_CASES_PATH);
    if (!file) {
        // Reported in the test's own words: the floor below says "0 cases",
        // which tells a reader the table was not read where a stack trace
        // would not.
        ADD_FAILURE() << "cannot read the shared table at " << SCE_EVENT_DATA_CASES_PATH;
        return {};
    }
    nlohmann::json table;
    file >> table;

    std::vector<Case> cases;
    for (const auto &entry : table.at("cases")) {
        Case testCase;
        testCase.payload = entry.at("payload").get<std::string>();
        testCase.source = entry.at("source").get<std::string>();
        testCase.clause = entry.at("clause").get<std::string>();

        const auto &expect = entry.at("expect");
        if (expect.contains("number")) {
            testCase.kind = Case::Kind::Number;
            testCase.number = expect.at("number").get<double>();
        } else if (expect.contains("string")) {
            testCase.kind = Case::Kind::Text;
            testCase.text = expect.at("string").get<std::string>();
        } else if (expect.contains("bool")) {
            testCase.kind = Case::Kind::Boolean;
            testCase.boolean = expect.at("bool").get<bool>();
        } else if (expect.contains("empty")) {
            testCase.kind = Case::Kind::Empty;
        } else {
            // A case whose expectation cannot be read is not a case that
            // passes: reading it as "no answer" would let a typo in a key
            // retire a case silently.
            ADD_FAILURE() << "case [" << testCase.source << "] has no readable expectation";
            continue;
        }
        cases.push_back(std::move(testCase));
    }
    return cases;
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
    if (std::holds_alternative<int64_t>(value)) {
        return std::to_string(std::get<int64_t>(value));
    }
    if (std::holds_alternative<double>(value)) {
        return std::to_string(std::get<double>(value));
    }
    if (std::holds_alternative<std::string>(value)) {
        return "\"" + std::get<std::string>(value) + "\"";
    }
    return "object/array";
}

bool matches(const ::ScriptValue &actual, const Case &expected) {
    switch (expected.kind) {
    case Case::Kind::Number:
        // The two engines are entitled to answer a whole number as either —
        // QuickJS has one number type and Lua 5.4 has two.
        if (std::holds_alternative<int64_t>(actual)) {
            return static_cast<double>(std::get<int64_t>(actual)) == expected.number;
        }
        return std::holds_alternative<double>(actual) && std::get<double>(actual) == expected.number;
    case Case::Kind::Text:
        return std::holds_alternative<std::string>(actual) && std::get<std::string>(actual) == expected.text;
    case Case::Kind::Boolean:
        return std::holds_alternative<bool>(actual) && std::get<bool>(actual) == expected.boolean;
    case Case::Kind::Empty:
        return std::holds_alternative<::ScriptUndefined>(actual) || std::holds_alternative<::ScriptNull>(actual);
    }
    return false;
}

std::string expectedOf(const Case &testCase) {
    switch (testCase.kind) {
    case Case::Kind::Number:
        return std::to_string(testCase.number);
    case Case::Kind::Text:
        return "\"" + testCase.text + "\"";
    case Case::Kind::Boolean:
        return testCase.boolean ? "true" : "false";
    case Case::Kind::Empty:
        return "null/undefined";
    }
    return "?";
}

/// Ask every case of one engine and collect what disagreed.
///
/// Every case is reported rather than the first: an engine that drops the
/// fall-through is a different defect from one that runs the payload, and one
/// failure cannot separate them.
std::vector<std::string> disagreements(SCE::IScriptEngine &engine, const std::string &prefix,
                                       const std::vector<Case> &table) {
    std::vector<std::string> failures;
    for (size_t index = 0; index < table.size(); ++index) {
        const Case &testCase = table[index];
        const std::string sessionId = prefix + "_reading_" + std::to_string(index);
        if (!engine.createSession(sessionId, "")) {
            failures.push_back("[" + testCase.source + "] no session");
            continue;
        }
        SCE::SetCurrentEventArgs args;
        args.eventName = "brief";
        args.eventData = testCase.payload;
        args.eventType = "external";
        auto bound = engine.setCurrentEvent(sessionId, args).get();
        if (!bound.status.isSuccess()) {
            failures.push_back("payload \"" + testCase.payload +
                               "\": setCurrentEvent failed: " + bound.status.getErrorMessage());
            engine.destroySession(sessionId);
            continue;
        }
        auto result = engine.evaluateExpression(sessionId, testCase.source).get();
        if (!result.isSuccess()) {
            failures.push_back("payload \"" + testCase.payload + "\": [" + testCase.source +
                               "] did not evaluate: " + result.getErrorMessage() + " (" + testCase.clause + ")");
        } else if (!matches(result.getInternalValue(), testCase)) {
            failures.push_back("payload \"" + testCase.payload + "\": [" + testCase.source + "] answered " +
                               describe(result.getInternalValue()) + ", " + testCase.clause + " says " +
                               expectedOf(testCase));
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

/// A floor, not an equality: adding a case must not have to touch this number,
/// but a table that stopped being read must not pass either.
void assertTableWasRead(const std::vector<Case> &table) {
    ASSERT_GE(table.size(), 8u) << "the shared reading table produced only " << table.size()
                                << " case(s), so this is not measuring the surface it claims to";
}

}  // namespace

TEST(EventDataReadings, TheSelectedEngineReadsEveryPayloadTheClauseNames) {
    const auto table = loadCases();
    assertTableWasRead(table);

    auto &engine = SCE::ScriptEngineProvider::getScriptEngine();
    const auto failures = disagreements(engine, "provider", table);
    EXPECT_TRUE(failures.empty()) << failures.size() << " of " << table.size()
                                  << " readings disagree with W3C SCXML B.2.8.1, "
                                  << "evaluated by " << SCE::ScriptEngineProvider::getEngineName()
                                  << " (SCE_SCRIPT_ENGINE=" << SCE::ScriptEngineProvider::getEngineId() << ").\n"
                                  << joined(failures);
}

#ifdef SCE_ENABLE_LUA
TEST(EventDataReadingsOnLuaEngine, TheLuaEngineReadsEveryPayloadTheClauseNames) {
    const auto table = loadCases();
    assertTableWasRead(table);

    auto &engine = SCE::LuaEngine::instance();
    const auto failures = disagreements(engine, "lua", table);
    EXPECT_TRUE(failures.empty()) << failures.size() << " of " << table.size()
                                  << " readings disagree with W3C SCXML B.2.8.1 on the Lua engine.\n"
                                  << joined(failures);
}

/// The sharper half of the expression case, which the shared table cannot ask
/// because the side effect is spelled in the receiver's own language.
///
/// This engine ran the payload as Lua source until 2026-08-19 — the rung its
/// four siblings had removed two days earlier and this one kept, because no
/// gate selects it. Reading the payload gives back its own text; running it
/// gives back `x` and, on the way, whatever else the sender named.
TEST(EventDataReadingsOnLuaEngine, APayloadThatIsACallLeavesTheSessionAlone) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "lua_payload_call";
    ASSERT_TRUE(engine.createSession(sessionId, ""));
    auto setup = engine.executeScript(sessionId, "breached = false").get();
    ASSERT_TRUE(setup.isSuccess()) << setup.getErrorMessage();

    SCE::SetCurrentEventArgs args;
    args.eventName = "brief";
    args.eventData = "(function() breached = true return 'x' end)()";
    args.eventType = "external";
    auto bound = engine.setCurrentEvent(sessionId, args).get();
    ASSERT_TRUE(bound.status.isSuccess()) << bound.status.getErrorMessage();

    auto result = engine.evaluateExpression(sessionId, "breached").get();
    ASSERT_TRUE(result.isSuccess()) << result.getErrorMessage();
    Case expected;
    expected.kind = Case::Kind::Boolean;
    expected.boolean = false;
    EXPECT_TRUE(matches(result.getInternalValue(), expected))
        << "the payload ran: a host, a peer session or an HTTP sender could write this session's globals "
        << "by naming them in event data";

    engine.destroySession(sessionId);
}
#endif
