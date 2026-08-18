// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @brief The engine this build selected answers what ECMA-262 says
 *
 * The C++ backend does not translate expressions. A generated state machine
 * carries the author's ECMAScript verbatim — `safeEvaluateGuard(engine,
 * session, "turns + 1 >= max_turns")` — and so does the Interpreter, so for
 * C++ the whole of `datamodel="ecmascript"` is whatever
 * `ScriptEngineProvider` hands back. That makes the engine, not the code
 * generator, the thing that has to be measured against the language.
 *
 * Which is why this reads `ScriptEngineProvider::getScriptEngine()` rather
 * than naming an engine: the assertion is "the engine this build ships is an
 * ECMAScript engine", and it has to be re-answered by every configuration of
 * `SCE_SCRIPT_ENGINE` rather than by the one a test author happened to pick.
 * `ECMAScriptComplianceTest` next door names `JSEngine` directly, so it
 * passes on a build that never runs `JSEngine`.
 *
 * The expectations are not ours. They live in
 * `tests/ecmascript/ecma262_semantics.json`, shared with
 * `sce-build/tests/ecmascript_semantics.rs`, and each one cites the ECMA-262
 * clause it comes from. A per-engine copy of a table drifts toward the engine
 * that reads it; one table cannot.
 */

#include "SCXMLTypes.h"
#include "common/GuardHelper.h"
#include "scripting/ScriptEngineProvider.h"
#include <cmath>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <nlohmann/json.hpp>
#include <set>
#include <string>
#include <variant>
#include <vector>

#ifdef SCE_ENABLE_LUA
#include "scripting/LuaEngine.h"
#endif

namespace {

/// The shape `expect` takes in the shared table: exactly one of these keys.
struct Answer {
    enum class Kind { Bool, Number, String, Empty } kind;
    bool boolean = false;
    double number = 0.0;
    std::string text;

    std::string describe() const {
        switch (kind) {
        case Kind::Bool:
            return boolean ? "true" : "false";
        case Kind::Number:
            return std::to_string(number);
        case Kind::String:
            return "\"" + text + "\"";
        case Kind::Empty:
            return "null/undefined";
        }
        return "?";
    }
};

struct Case {
    std::string group;
    std::string setup;
    std::string source;
    bool asCondition = false;
    Answer expected;
    std::string clause;
};

Answer parseAnswer(const nlohmann::json &expect, const std::string &source) {
    Answer answer{};
    if (expect.contains("bool")) {
        answer.kind = Answer::Kind::Bool;
        answer.boolean = expect.at("bool").get<bool>();
    } else if (expect.contains("number")) {
        answer.kind = Answer::Kind::Number;
        answer.number = expect.at("number").get<double>();
    } else if (expect.contains("string")) {
        answer.kind = Answer::Kind::String;
        answer.text = expect.at("string").get<std::string>();
    } else if (expect.contains("empty")) {
        answer.kind = Answer::Kind::Empty;
    } else {
        // A case whose expectation cannot be read is not a case that passes.
        // Reading it as "no answer" would let a typo in a key name retire a
        // case silently, which is the failure mode the shared table exists to
        // remove.
        ADD_FAILURE() << "case [" << source << "] names no expected answer";
    }
    return answer;
}

std::vector<Case> loadCases() {
    std::ifstream file(SCE_ECMA262_CASES_PATH);
    if (!file.is_open()) {
        // Returning nothing rather than throwing keeps the verdict in the
        // test's own words: the floor below reports "0 cases", which says
        // the table was not read, where a stack trace would not.
        ADD_FAILURE() << "cannot read the shared table at " << SCE_ECMA262_CASES_PATH;
        return {};
    }

    nlohmann::json table;
    file >> table;

    std::vector<Case> cases;
    for (const auto &entry : table.at("cases")) {
        Case testCase;
        testCase.group = entry.value("group", std::string{});
        testCase.setup = entry.at("setup").get<std::string>();
        testCase.source = entry.at("source").get<std::string>();
        testCase.asCondition = entry.at("form").get<std::string>() == "condition";
        testCase.clause = entry.at("clause").get<std::string>();
        testCase.expected = parseAnswer(entry.at("expect"), testCase.source);
        cases.push_back(std::move(testCase));
    }
    return cases;
}

/// What the engine answered, in the same words the table uses.
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
    if (std::holds_alternative<std::shared_ptr<::ScriptArray>>(value)) {
        return "[array]";
    }
    if (std::holds_alternative<std::shared_ptr<::ScriptObject>>(value)) {
        return "[object]";
    }
    return "[unknown]";
}

/// An engine may hold a whole number as an integer or as a double, and
/// ECMA-262 has one Number type — so both spellings answer a `number` case.
bool matches(const ::ScriptValue &actual, const Answer &expected) {
    switch (expected.kind) {
    case Answer::Kind::Bool:
        return std::holds_alternative<bool>(actual) && std::get<bool>(actual) == expected.boolean;
    case Answer::Kind::Number:
        if (std::holds_alternative<int64_t>(actual)) {
            return static_cast<double>(std::get<int64_t>(actual)) == expected.number;
        }
        if (std::holds_alternative<double>(actual)) {
            return std::abs(std::get<double>(actual) - expected.number) < 1e-9;
        }
        return false;
    case Answer::Kind::String:
        return std::holds_alternative<std::string>(actual) && std::get<std::string>(actual) == expected.text;
    case Answer::Kind::Empty:
        return std::holds_alternative<::ScriptUndefined>(actual) || std::holds_alternative<::ScriptNull>(actual);
    }
    return false;
}

/// One case that answered something other than what the language says.
///
/// Carries which case it was, not only the sentence about it: the second
/// engine's answers are compared against a declared list, and a list keyed on
/// a rendered message would have to be rewritten whenever the wording moved.
struct Disagreement {
    size_t caseIndex;
    std::string message;
};

/// Ask every case of one engine and collect what disagreed.
///
/// Every case is reported, not just the first: an engine that answers one
/// group wrong and another right is a different problem from one that answers
/// nothing, and the first failure alone cannot tell them apart.
///
/// The session id carries a prefix because two engines are asked in one
/// process and a session id is only unique within the engine that owns it.
std::vector<Disagreement> disagreements(SCE::IScriptEngine &engine, const std::string &prefix,
                                        const std::vector<Case> &cases) {
    std::vector<Disagreement> failures;

    for (size_t index = 0; index < cases.size(); ++index) {
        const Case &testCase = cases[index];
        const std::string sessionId = prefix + "_ecma262_case_" + std::to_string(index);
        if (!engine.createSession(sessionId, "")) {
            failures.push_back({index, "[" + testCase.source + "] no session"});
            continue;
        }

        bool setupOk = true;
        if (!testCase.setup.empty()) {
            auto setupResult = engine.executeScript(sessionId, testCase.setup).get();
            if (!setupResult.isSuccess()) {
                failures.push_back({index, "[" + testCase.source + "] setup did not run: " +
                                               setupResult.getErrorMessage() + "\n  setup: " + testCase.setup});
                setupOk = false;
            }
        }

        if (setupOk && testCase.asCondition) {
            // The production guard path, not a reimplementation of it: this is
            // what a `cond=` attribute reaches in both engines.
            auto answered = SCE::GuardHelper::evaluateGuard(engine, sessionId, testCase.source);
            if (!answered.has_value()) {
                failures.push_back(
                    {index, "[" + testCase.source + "] failed to evaluate as a condition (" + testCase.clause + ")"});
            } else if (*answered != testCase.expected.boolean) {
                failures.push_back({index, "[" + testCase.source + "] answered " + (*answered ? "true" : "false") +
                                               ", ECMAScript says " + testCase.expected.describe() + " (" +
                                               testCase.clause + ")"});
            }
        } else if (setupOk) {
            auto result = engine.evaluateExpression(sessionId, testCase.source).get();
            if (!result.isSuccess()) {
                failures.push_back({index, "[" + testCase.source + "] failed to evaluate: " + result.getErrorMessage() +
                                               " (" + testCase.clause + ")"});
            } else if (!matches(result.getInternalValue(), testCase.expected)) {
                failures.push_back({index, "[" + testCase.source + "] answered " + describe(result.getInternalValue()) +
                                               ", ECMAScript says " + testCase.expected.describe() + " (" +
                                               testCase.clause + ")"});
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

std::vector<std::string> messagesOf(const std::vector<Disagreement> &failures) {
    std::vector<std::string> messages;
    messages.reserve(failures.size());
    for (const auto &failure : failures) {
        messages.push_back(failure.message);
    }
    return messages;
}

/// A case, named the way the declared list names one.
///
/// Source and clause together, because a source can appear twice: `!a` is
/// asked of `0` under 12.5.9 and of `null` under 7.1.2, and one of those two
/// answers correctly.
std::string identify(const Case &testCase) {
    return testCase.source + " | " + testCase.clause;
}

/// The cases the `lua` selection is declared to answer differently.
///
/// Read from a file rather than written here so that the sentence "this
/// engine is not an ECMAScript engine" is a list a reader can check against
/// the clauses it cites, and so that closing one is a deletion from data
/// rather than an edit to a test.
std::vector<std::string> loadDeclaredDivergences() {
    std::ifstream file(SCE_LUA_DIVERGENCES_PATH);
    if (!file.is_open()) {
        ADD_FAILURE() << "cannot read the declared divergences at " << SCE_LUA_DIVERGENCES_PATH;
        return {};
    }
    nlohmann::json document;
    file >> document;

    std::vector<std::string> declared;
    for (const auto &entry : document.at("divergences")) {
        declared.push_back(entry.at("source").get<std::string>() + " | " + entry.at("clause").get<std::string>());
    }
    return declared;
}

/// A floor, not an equality: adding a case must not have to touch this
/// number, but a table that stopped being read must not pass either.
void assertTableWasRead(const std::vector<Case> &cases) {
    ASSERT_GE(cases.size(), 55u) << "the shared ECMA-262 table produced only " << cases.size()
                                 << " case(s), so this is not measuring the corpus it claims to";
}

}  // namespace

TEST(EcmaScriptSemantics, TheSelectedEngineAnswersWhatEcmaScriptAnswers) {
    const auto cases = loadCases();
    assertTableWasRead(cases);

    auto &engine = SCE::ScriptEngineProvider::getScriptEngine();
    const auto failures = messagesOf(disagreements(engine, "provider", cases));

    EXPECT_TRUE(failures.empty()) << failures.size() << " of " << cases.size()
                                  << " expressions disagree with ECMA-262, evaluated by "
                                  << SCE::ScriptEngineProvider::getEngineName()
                                  << " (SCE_SCRIPT_ENGINE=" << SCE::ScriptEngineProvider::getEngineId()
                                  << ").\nIf this build did not choose that engine on purpose, its CMake"
                                     " cache predates the default: `SCE_SCRIPT_ENGINE` is a cache entry, so"
                                     " an existing build directory keeps whatever it was configured with."
                                     " Reconfigure it (-DSCE_SCRIPT_ENGINE=quickjs) or delete the cache.\n"
                                  << joined(failures);
}

#ifdef SCE_ENABLE_LUA
/// The other engine this backend ships answers the same table.
///
/// Named rather than provided, for the reason `DomReadSurfaceOnLuaEngine`
/// carries next door: `SCE_SCRIPT_ENGINE` is a compile-time choice and no
/// gate configures `lua`, so the engine reached by that selection would be
/// compiled by every build and measured by none — while remaining a listed,
/// validated value of the cache entry that a consumer may choose.
///
/// What it measures is the emission in `EcmaScriptToLuaTransformer`, not the
/// semantics: `ecma_semantics.lua` is a shared runtime asset that this engine
/// already loads and that the code generator's Lua backends already call, so
/// a disagreement here says the transformer wrote a bare Lua operator where
/// the shared definition of the ECMAScript one was in scope.
///
/// The verdict has two sides, because a one-sided one rots. Asking only
/// "nothing new disagrees" lets the declared list keep claiming a divergence
/// that has since been repaired, and the list is the only written account of
/// what this engine is not; asking only "the declared ones still disagree"
/// lets a new one arrive unremarked, which is how the count reached 44 while
/// a comment said 26.
TEST(EcmaScriptSemanticsOnLuaEngine, TheSecondEngineDivergesExactlyWhereItIsDeclaredTo) {
    const auto cases = loadCases();
    assertTableWasRead(cases);

    const auto declared = loadDeclaredDivergences();
    ASSERT_FALSE(declared.empty()) << "the declared divergences were not read from " << SCE_LUA_DIVERGENCES_PATH;

    auto &engine = SCE::LuaEngine::instance();
    const auto failures = disagreements(engine, "lua", cases);

    std::set<std::string> declaredSet(declared.begin(), declared.end());
    std::set<std::string> observed;
    std::vector<std::string> undeclared;
    for (const auto &failure : failures) {
        const std::string name = identify(cases[failure.caseIndex]);
        observed.insert(name);
        if (declaredSet.count(name) == 0) {
            undeclared.push_back(failure.message);
        }
    }

    EXPECT_TRUE(undeclared.empty())
        << undeclared.size() << " expression(s) disagree with ECMA-262 on LuaEngine without being declared to.\n"
        << "Either the transformer lost an answer it used to give, or a new case reached a rewriting pass that\n"
        << "cannot read it. If it is the second, `tests/ecmascript/lua_engine_divergences.json` is where it is\n"
        << "written down, with what closing it would take.\n"
        << joined(undeclared);

    // Names in the list that answer no case at all: a divergence declared
    // against a source or clause the shared table no longer carries would
    // otherwise sit there forever, describing nothing.
    std::set<std::string> named;
    for (const auto &testCase : cases) {
        named.insert(identify(testCase));
    }

    std::vector<std::string> stale;
    for (const auto &entry : declared) {
        if (named.count(entry) == 0) {
            stale.push_back(entry + "  (no such case in the shared table)");
        } else if (observed.count(entry) == 0) {
            stale.push_back(entry + "  (now agrees with ECMA-262)");
        }
    }

    EXPECT_TRUE(stale.empty()) << stale.size()
                               << " declared divergence(s) no longer describe this engine. Remove them from\n"
                                  "`tests/ecmascript/lua_engine_divergences.json`: a list that keeps a repaired\n"
                                  "entry understates the engine as surely as a missing entry overstates it.\n"
                               << joined(stale);
}
#endif
