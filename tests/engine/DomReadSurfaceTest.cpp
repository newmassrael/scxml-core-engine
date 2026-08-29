// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief XML in the data model is a DOM structure, not three method names
 *
 * §scxml-B-2-1 obliges the Processor to *"create the corresponding DOM
 * structure"* for a `<data>` element's XML content, and §scxml-B-2-8-1
 * says the same for an arriving `_event.data`. Measured 2026-08-18, what
 * every SCE backend supplied was `getElementsByTagName`, `getAttribute`
 * and a non-standard `getTagName` — the two names the W3C IRP suite
 * reads, plus one — so `d.tagName` answered nil, `d.firstChild` answered
 * nil, and `d.childNodes.length` was a length taken of nil. Nothing
 * refused any of them: 204/204 W3C fixtures passed throughout, because
 * the suite's DOM vocabulary is exactly the part that worked.
 *
 * The expectations are not ours and not this file's. They live in
 * `tests/ecmascript/dom_read_surface.json`, one claim per case with the
 * DOM clause that backs it, and the Kotlin, Rust, Go and Python readers
 * ask the same file — a per-backend copy drifts toward whichever backend
 * reads it, which is the blindness that let seven bindings disagree with
 * one specification. What is asked here is the `source` spelling: the
 * author's own ECMAScript, which is what both C++ engines are handed
 * (the Lua one rewrites it itself).
 *
 * Two fixtures, because this backend ships two bindings:
 *
 *   * `DomReadSurface` goes through `ScriptEngineProvider`, so it
 *     measures the engine this build selected (`SCE_SCRIPT_ENGINE`).
 *   * `DomReadSurfaceOnLuaEngine` names `LuaEngine` on purpose. The
 *     provider is a compile-time choice and no gate configures
 *     `-DSCE_SCRIPT_ENGINE=lua`, so `LuaDOMBinding` is compiled by every
 *     build and run by none of them — naming the engine is what gives
 *     that binding a witness at all.
 */

#include "SCXMLTypes.h"
#include "scripting/ScriptEngineProvider.h"
#include <fstream>
#include <gtest/gtest.h>
#include <map>
#include <nlohmann/json.hpp>
#include <string>
#include <variant>
#include <vector>

#ifdef SCE_ENABLE_LUA
#include "scripting/LuaEngine.h"
#endif

namespace {

struct Case {
    /// The document the expression is asked of, resolved from the table's
    /// `documents` map so a case names it rather than repeating it.
    std::string xml;
    std::string source;
    std::string clause;
    /// Exactly one of these carries the answer; `Empty` means DOM Level 1
    /// Core's null — an element has no nodeValue and a node that is not
    /// the document has no documentElement.
    enum class Kind { Number, Text, Boolean, Empty } kind = Kind::Empty;
    double number = 0.0;
    std::string text;
    bool boolean = false;
};

/// The shared table, read from the path CMake injects.
///
/// Injected rather than resolved from the working directory, so the test
/// reads the same table under ctest, CI and an IDE run — the reasoning
/// `ecmascript_semantics_test` next door already carries.
std::vector<Case> loadCases() {
    std::ifstream file(SCE_DOM_SURFACE_CASES_PATH);
    if (!file) {
        // Reported in the test's own words: the floor below says "0
        // cases", which tells a reader the table was not read where a
        // stack trace would not.
        ADD_FAILURE() << "cannot read the shared table at " << SCE_DOM_SURFACE_CASES_PATH;
        return {};
    }
    nlohmann::json table;
    file >> table;

    std::map<std::string, std::string> documents;
    for (const auto &entry : table.at("documents").items()) {
        documents.emplace(entry.key(), entry.value().get<std::string>());
    }

    std::vector<Case> cases;
    for (const auto &entry : table.at("cases")) {
        Case testCase;
        const auto document = entry.at("document").get<std::string>();
        const auto found = documents.find(document);
        if (found == documents.end()) {
            ADD_FAILURE() << "case names document '" << document << "', which the table lacks";
            continue;
        }
        testCase.xml = found->second;
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
        // The two engines are entitled to answer a whole number as
        // either — QuickJS has one number type and Lua 5.4 has two.
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
/// Every case is reported rather than the first: a binding that answers
/// the methods and none of the properties is a different defect from one
/// that cannot parse the document, and one failure cannot separate them.
std::vector<std::string> disagreements(SCE::IScriptEngine &engine, const std::string &prefix,
                                       const std::vector<Case> &table) {
    std::vector<std::string> failures;
    for (size_t index = 0; index < table.size(); ++index) {
        const Case &testCase = table[index];
        const std::string sessionId = prefix + "_dom_" + std::to_string(index);
        if (!engine.createSession(sessionId, "")) {
            failures.push_back("[" + testCase.source + "] no session");
            continue;
        }
        auto bound = engine.setVariableAsDOM(sessionId, "var1", testCase.xml).get();
        if (!bound.isSuccess()) {
            failures.push_back("[" + testCase.source + "] setVariableAsDOM failed: " + bound.getErrorMessage());
            engine.destroySession(sessionId);
            continue;
        }
        auto result = engine.evaluateExpression(sessionId, testCase.source).get();
        if (!result.isSuccess()) {
            failures.push_back("[" + testCase.source + "] did not evaluate: " + result.getErrorMessage() + " (" +
                               testCase.clause + ")");
        } else if (!matches(result.getInternalValue(), testCase)) {
            failures.push_back("[" + testCase.source + "] answered " + describe(result.getInternalValue()) + ", " +
                               testCase.clause + " says " + expectedOf(testCase));
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

/// A floor, not an equality: adding a case must not have to touch this
/// number, but a table that stopped being read must not pass either.
void assertTableWasRead(const std::vector<Case> &table) {
    ASSERT_GE(table.size(), 30u) << "the shared DOM table produced only " << table.size()
                                 << " case(s), so this is not measuring the surface it claims to";
}

}  // namespace

TEST(DomReadSurface, TheSelectedEngineAnswersDomLevel1Core) {
    const auto table = loadCases();
    assertTableWasRead(table);

    auto &engine = SCE::ScriptEngineProvider::getScriptEngine();
    const auto failures = disagreements(engine, "provider", table);
    EXPECT_TRUE(failures.empty()) << failures.size() << " of " << table.size()
                                  << " reads disagree with DOM Level 1 Core, "
                                  << "evaluated by " << SCE::ScriptEngineProvider::getEngineName()
                                  << " (SCE_SCRIPT_ENGINE=" << SCE::ScriptEngineProvider::getEngineId() << ").\n"
                                  << joined(failures);
}

#ifdef SCE_ENABLE_LUA
/// The receiver of a member access is the whole chain before it.
///
/// This lives here because the DOM traversal is what exposed it — every
/// traversal is a chain — but the property reaches further than the DOM,
/// so it gets a witness that does not depend on a DOM being bound.
///
/// It was written against `EcmaScriptToLuaTransformer`, whose passes
/// found a receiver by scanning word characters backwards and so bound
/// the wrong one: `xs.inner.length` became `xs.#inner`, which is not
/// Lua, and `xs.inner.indexOf(2)` became `xs._indexOf(inner, 2)`, which
/// asked a different receiver and said nothing. That rewriter is retired
/// and the property now belongs to the frontend's parser, where a
/// receiver is a subtree rather than a span of characters.
///
/// ⚠ The SETUP is what carried the retirement, and it is not cosmetic.
/// It used to be `xs = { inner = { 10, 20, 30 } }` — Lua table syntax,
/// handed to the engine as ECMAScript, which only ever ran because the
/// rewriter passed text it could not read through unchanged. A parser
/// refuses it, correctly: it is not ECMAScript. The setup below is the
/// same object written in the language the call claims.
TEST(EcmaScriptMemberChain, AMemberAccessBindsTheWholeChainBeforeIt) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "lua_receiver";
    ASSERT_TRUE(engine.createSession(sessionId, ""));
    auto setup = engine.executeScript(sessionId, "var xs = { inner: [10, 20, 30] };").get();
    ASSERT_TRUE(setup.isSuccess()) << setup.getErrorMessage();

    struct Probe {
        const char *source;
        double expected;
    };

    for (const Probe &probe :
         {Probe{"xs.inner.length", 3}, Probe{"xs.inner.indexOf(20)", 1}, Probe{"xs.inner[0]", 10}}) {
        auto result = engine.evaluateExpression(sessionId, probe.source).get();
        ASSERT_TRUE(result.isSuccess()) << probe.source << ": " << result.getErrorMessage();
        Case expected;
        expected.kind = Case::Kind::Number;
        expected.number = probe.expected;
        EXPECT_TRUE(matches(result.getInternalValue(), expected))
            << probe.source << " answered " << describe(result.getInternalValue()) << ", expected " << probe.expected;
    }
    engine.destroySession(sessionId);
}

TEST(DomReadSurfaceOnLuaEngine, TheSecondBindingAnswersTheSameTable) {
    const auto table = loadCases();
    assertTableWasRead(table);

    // Named rather than provided, because no gate configures this engine
    // as the provider's: without this, `LuaDOMBinding` would be compiled
    // by every build and run by none.
    auto &engine = SCE::LuaEngine::instance();
    const auto failures = disagreements(engine, "lua", table);
    EXPECT_TRUE(failures.empty()) << failures.size() << " of " << table.size()
                                  << " reads disagree with DOM Level 1 Core on LuaEngine.\n"
                                  << joined(failures);
}
#endif
