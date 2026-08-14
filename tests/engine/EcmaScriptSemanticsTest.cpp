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
#include <string>
#include <variant>
#include <vector>

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

}  // namespace

class EcmaScriptSemanticsTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &SCE::ScriptEngineProvider::getScriptEngine();
    }

    SCE::IScriptEngine *engine_ = nullptr;
};

TEST_F(EcmaScriptSemanticsTest, TheSelectedEngineAnswersWhatEcmaScriptAnswers) {
    const auto cases = loadCases();

    // A floor, not an equality: adding a case must not have to touch this
    // number, but a table that stopped being read must not pass either.
    ASSERT_GE(cases.size(), 55u) << "the shared ECMA-262 table produced only " << cases.size()
                                 << " case(s), so this is not measuring the corpus it claims to";

    std::vector<std::string> failures;

    for (size_t index = 0; index < cases.size(); ++index) {
        const Case &testCase = cases[index];
        const std::string sessionId = "ecma262_case_" + std::to_string(index);
        ASSERT_TRUE(engine_->createSession(sessionId, ""))
            << "could not create a session for [" << testCase.source << "]";

        bool setupOk = true;
        if (!testCase.setup.empty()) {
            auto setupResult = engine_->executeScript(sessionId, testCase.setup).get();
            if (!setupResult.isSuccess()) {
                failures.push_back("[" + testCase.source + "] setup did not run: " + setupResult.getErrorMessage() +
                                   "\n  setup: " + testCase.setup);
                setupOk = false;
            }
        }

        if (setupOk && testCase.asCondition) {
            // The production guard path, not a reimplementation of it: this is
            // what a `cond=` attribute reaches in both engines.
            auto answered = SCE::GuardHelper::evaluateGuard(*engine_, sessionId, testCase.source);
            if (!answered.has_value()) {
                failures.push_back("[" + testCase.source + "] failed to evaluate as a condition (" + testCase.clause +
                                   ")");
            } else if (*answered != testCase.expected.boolean) {
                failures.push_back("[" + testCase.source + "] answered " + (*answered ? "true" : "false") +
                                   ", ECMAScript says " + testCase.expected.describe() + " (" + testCase.clause + ")");
            }
        } else if (setupOk) {
            auto result = engine_->evaluateExpression(sessionId, testCase.source).get();
            if (!result.isSuccess()) {
                failures.push_back("[" + testCase.source + "] failed to evaluate: " + result.getErrorMessage() + " (" +
                                   testCase.clause + ")");
            } else if (!matches(result.getInternalValue(), testCase.expected)) {
                failures.push_back("[" + testCase.source + "] answered " + describe(result.getInternalValue()) +
                                   ", ECMAScript says " + testCase.expected.describe() + " (" + testCase.clause + ")");
            }
        }

        engine_->destroySession(sessionId);
    }

    // Every case is reported, not just the first: a build that answers one
    // group wrong and another right is a different problem from one that
    // answers nothing, and the first failure alone cannot tell them apart.
    EXPECT_TRUE(failures.empty()) << failures.size() << " of " << cases.size()
                                  << " expressions disagree with ECMA-262, evaluated by "
                                  << SCE::ScriptEngineProvider::getEngineName()
                                  << " (SCE_SCRIPT_ENGINE=" << SCE::ScriptEngineProvider::getEngineId()
                                  << ").\nIf this build did not choose that engine on purpose, its CMake"
                                     " cache predates the default: `SCE_SCRIPT_ENGINE` is a cache entry, so"
                                     " an existing build directory keeps whatever it was configured with."
                                     " Reconfigure it (-DSCE_SCRIPT_ENGINE=quickjs) or delete the cache.\n"
                                  << [&failures] {
                                         std::string joined;
                                         for (const auto &failure : failures) {
                                             joined += failure + "\n";
                                         }
                                         return joined;
                                     }();
}
