// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief An `<assign>` is one operation inside the data model
 *
 * W3C SCXML B.2: an `<assign>` replaces the value at its location with the
 * value its expression produces. `IScriptEngine::assign` runs
 * `<location> = (<expr>)` in the engine, and `AssignmentExecutionHelper` —
 * what the Interpreter and every generated C++ machine call — reaches nothing
 * else, so the value never leaves the engine.
 *
 * It used to, for one shape of location. A bare name evaluated the expression
 * into a `ScriptValue` and wrote that back with `setVariable`, and a
 * `ScriptValue` has no kind for a DOM, a function or an object's identity:
 * `<assign location="v" expr="_event.data"/>` stored an empty object where the
 * event carried a document. Five exact spellings (`_event`, `_sessionid`, ...)
 * were routed around the loss so that W3C test 329's `Var2 = _event` kept its
 * reference, and every other way of writing the same value missed the patch.
 *
 * Each case below asks for a value a round trip through `ScriptValue` loses,
 * at a bare name, which was the location that made the trip. Both engines are
 * named rather than reached through `ScriptEngineProvider`, for the reason
 * `ScriptLanguageSeamTest` gives: the provider is a compile-time choice, so one
 * of them would be compiled by every build and run by none.
 */

#include "common/AssignmentExecutionHelper.h"
#include "scripting/IScriptEngine.h"
#include "scripting/ScriptSource.h"
#include <gtest/gtest.h>
#include <string>
#include <variant>

#ifdef SCE_ENABLE_LUA
#include "scripting/LuaEngine.h"
#endif

#ifdef SCE_ENABLE_QUICKJS
#include "scripting/JSEngine.h"
#endif

namespace {

/// `<assign location expr>` as the Interpreter and the generated machines make
/// it; fails the test with the reported error when the assignment fails.
void assign(SCE::IScriptEngine &engine, const std::string &sessionId, const std::string &location,
            const std::string &expr) {
    std::string error;
    const bool assigned = SCE::AssignmentExecutionHelper::executeAssignment(
        engine, sessionId, SCE::ScriptSource::ecmascript(location), SCE::ScriptSource::ecmascript(expr),
        [&error](const std::string &message) { error = message; });
    EXPECT_TRUE(assigned) << "<assign location=\"" << location << "\" expr=\"" << expr << "\"> failed: " << error;
    EXPECT_TRUE(error.empty()) << "a successful assignment reported an error: " << error;
}

/// Whether @p expression evaluates to `true` in the session.
bool holds(SCE::IScriptEngine &engine, const std::string &sessionId, const std::string &expression) {
    auto result = engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript(expression)).get();
    EXPECT_TRUE(result.isSuccess()) << expression << " did not evaluate: " << result.getErrorMessage();
    const auto &value = result.getInternalValue();
    return std::holds_alternative<bool>(value) && std::get<bool>(value);
}

void runs(SCE::IScriptEngine &engine, const std::string &sessionId, const std::string &script) {
    auto result = engine.executeScript(sessionId, SCE::ScriptSource::ecmascript(script)).get();
    ASSERT_TRUE(result.isSuccess()) << script << ": " << result.getErrorMessage();
}

// A DOM keeps being a DOM, at a bare name and at a member path. This is
// `<assign location="v" expr="_event.data"/>` with the event's document bound
// the way `<data>` content binds one.
void expectADomStaysADom(SCE::IScriptEngine &engine, const std::string &sessionId) {
    ASSERT_TRUE(engine.setVariableAsDOM(sessionId, "doc", "<a><b/><b/></a>").get().isSuccess());
    runs(engine, sessionId, "var holder = {};");

    assign(engine, sessionId, "v", "doc");
    assign(engine, sessionId, "holder.d", "doc");

    EXPECT_TRUE(holds(engine, sessionId, "v.getElementsByTagName('b').length === 2"))
        << "the bare-name location lost the document";
    EXPECT_TRUE(holds(engine, sessionId, "holder.d.getElementsByTagName('b').length === 2"))
        << "the member-path location lost the document";
}

// An object keeps its identity: the location holds the SAME object, which is
// what W3C test 329 asks of `Var2 = _event` and of nothing else.
void expectAnObjectKeepsItsIdentity(SCE::IScriptEngine &engine, const std::string &sessionId) {
    runs(engine, sessionId, "var original = {k: 1};");
    assign(engine, sessionId, "copy", "original");
    EXPECT_TRUE(holds(engine, sessionId, "copy === original"))
        << "the location holds a copy of the object, not the object";
}

// A function is a value a `ScriptValue` cannot hold at all.
void expectAFunctionStaysCallable(SCE::IScriptEngine &engine, const std::string &sessionId) {
    assign(engine, sessionId, "f", "function () { return 7; }");
    EXPECT_TRUE(holds(engine, sessionId, "f() === 7")) << "the function did not survive the assignment";
}

// W3C test 452 writes `expr="new testobject();"`: the statement terminator an
// author may put after an expression is dropped, not a syntax error.
void expectATrailingTerminatorIsDropped(SCE::IScriptEngine &engine, const std::string &sessionId) {
    assign(engine, sessionId, "n", "1 + 1;");
    EXPECT_TRUE(holds(engine, sessionId, "n === 2"));
}

// The expression is evaluated exactly once (ECMA-262 13.4.4: `++count` from 0
// is 1, and leaves `count` at 1).
void expectTheExpressionIsEvaluatedOnce(SCE::IScriptEngine &engine, const std::string &sessionId) {
    runs(engine, sessionId, "var count = 0;");
    assign(engine, sessionId, "once", "++count");
    EXPECT_TRUE(holds(engine, sessionId, "once === 1 && count === 1"))
        << "the expression ran more than once, or its value was not the one stored";
}

// An expression that raises is the assignment's failure, and the location
// keeps what it held.
void expectARaisingExpressionChangesNothing(SCE::IScriptEngine &engine, const std::string &sessionId) {
    runs(engine, sessionId, "var kept = 5;");
    bool reported = false;
    EXPECT_FALSE(
        SCE::AssignmentExecutionHelper::executeAssignment(engine, sessionId, SCE::ScriptSource::ecmascript("kept"),
                                                          SCE::ScriptSource::ecmascript("undefined.invalidProperty"),
                                                          [&reported](const std::string &) { reported = true; }))
        << "an expression that raises was reported as assigned";
    EXPECT_TRUE(reported) << "the failure did not reach the error callback, so no error.execution is raised";
    EXPECT_TRUE(holds(engine, sessionId, "kept === 5")) << "the failed assignment changed the location";
}

// Writing a name nothing declared creates it, and it is a variable afterwards
// even when the value written is undefined.
void expectAnAssignedNameIsDeclared(SCE::IScriptEngine &engine, const std::string &sessionId) {
    assign(engine, sessionId, "fresh", "undefined");
    auto read = engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript("fresh")).get();
    EXPECT_TRUE(read.isSuccess()) << "reading the name the assignment created raised: " << read.getErrorMessage();

    assign(engine, sessionId, "created", "5");
    EXPECT_TRUE(holds(engine, sessionId, "created === 5")) << "a write to an undeclared name did not create it";
}

// Whether `<assign location expr>` was refused: the helper answered false and
// reported the error that becomes error.execution.
bool refused(SCE::IScriptEngine &engine, const std::string &sessionId, const std::string &location,
             const std::string &expr) {
    bool reported = false;
    const bool assigned = SCE::AssignmentExecutionHelper::executeAssignment(
        engine, sessionId, SCE::ScriptSource::ecmascript(location), SCE::ScriptSource::ecmascript(expr),
        [&reported](const std::string &) { reported = true; });
    EXPECT_EQ(assigned, !reported) << location << " = " << expr << ": the answer and the report disagree";
    return !assigned && reported;
}

// An expression that raises at a name nothing declared creates no name: the
// write never happens, as it never did.
void expectARaisingExpressionCreatesNoName(SCE::IScriptEngine &engine, const std::string &sessionId) {
    EXPECT_TRUE(refused(engine, sessionId, "ghost", "undefined.invalidProperty"));
    auto read = engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript("ghost")).get();
    EXPECT_FALSE(read.isSuccess()) << "a failed assignment left the name `ghost` behind";
}

void expectAnAssignmentStaysInTheEngine(SCE::IScriptEngine &engine, const std::string &prefix) {
    const std::string sessionId = prefix + "_assign_stays_in_engine";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    expectADomStaysADom(engine, sessionId);
    expectAnObjectKeepsItsIdentity(engine, sessionId);
    expectAFunctionStaysCallable(engine, sessionId);
    expectATrailingTerminatorIsDropped(engine, sessionId);
    expectTheExpressionIsEvaluatedOnce(engine, sessionId);
    expectARaisingExpressionChangesNothing(engine, sessionId);
    expectAnAssignedNameIsDeclared(engine, sessionId);
    expectARaisingExpressionCreatesNoName(engine, sessionId);

    engine.destroySession(sessionId);
}

}  // namespace

#ifdef SCE_ENABLE_QUICKJS
TEST(AnAssignmentStaysInTheEngine, QuickJsKeepsTheValueItIsGiven) {
    auto &engine = SCE::JSEngine::instance();
    ASSERT_TRUE(engine.initialize());
    expectAnAssignmentStaysInTheEngine(engine, "assign_js");
}

// A write that does not take effect is the assignment's failure: the value is
// not a legal value for that location, which is error.execution. Sloppy
// ECMAScript ignores each of these writes in silence; the assignment makes
// them as strict code does, so each throws, and the location keeps its value.
// (A Lua location has no such state: a global is an entry of the global table
// and a field write either stores or raises.)
TEST(AnAssignmentStaysInTheEngine, QuickJsRefusesAWriteThatDoesNotTakeEffect) {
    auto &engine = SCE::JSEngine::instance();
    ASSERT_TRUE(engine.initialize());
    const std::string sessionId = "assign_js_write_that_does_not_take_effect";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    // A non-writable global.
    EXPECT_TRUE(refused(engine, sessionId, "NaN", "1")) << "a write to NaN was reported as assigned";
    EXPECT_TRUE(holds(engine, sessionId, "NaN !== NaN"));

    // A property of a frozen object.
    runs(engine, sessionId, "var frozen = Object.freeze({k: 1});");
    EXPECT_TRUE(refused(engine, sessionId, "frozen.k", "2")) << "a write to a frozen object was reported as assigned";
    EXPECT_TRUE(holds(engine, sessionId, "frozen.k === 1"));

    // A property with a getter and no setter.
    runs(engine, sessionId, "var guarded = {}; Object.defineProperty(guarded, 'k', {get: function () { return 1; }});");
    EXPECT_TRUE(refused(engine, sessionId, "guarded.k", "2"))
        << "a write to a getter-only property was reported as assigned";
    EXPECT_TRUE(holds(engine, sessionId, "guarded.k === 1"));

    // The writable counterparts still take the value, so the refusals above
    // are about the target and not about strict code refusing every write.
    runs(engine, sessionId, "var open = {k: 1};");
    EXPECT_FALSE(refused(engine, sessionId, "open.k", "2"));
    EXPECT_TRUE(holds(engine, sessionId, "open.k === 2"));

    engine.destroySession(sessionId);
}

// The write is strict and the expression is not: a function the expression
// defines is sloppy code, so called plainly it sees the global object as
// `this`, as it does written anywhere else in the document.
TEST(AnAssignmentStaysInTheEngine, QuickJsLeavesTheExpressionSloppy) {
    auto &engine = SCE::JSEngine::instance();
    ASSERT_TRUE(engine.initialize());
    const std::string sessionId = "assign_js_expression_stays_sloppy";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    runs(engine, sessionId, "var marker = 7;");
    assign(engine, sessionId, "reads", "function () { return this.marker; }");
    EXPECT_TRUE(holds(engine, sessionId, "reads() === 7"))
        << "the function the expression defined is strict code: `this` was not the global object";

    engine.destroySession(sessionId);
}
#endif

#ifdef SCE_ENABLE_LUA
TEST(AnAssignmentStaysInTheEngine, LuaKeepsTheValueItIsGiven) {
    expectAnAssignmentStaysInTheEngine(SCE::LuaEngine::instance(), "assign_lua");
}

// The two halves become one statement, so an engine that accepts both
// languages still refuses a location in one and an expression in the other,
// and says which is which rather than running a statement with a hole in it.
TEST(AnAssignmentStaysInTheEngine, LuaRefusesHalvesInTwoLanguages) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "assign_lua_mixed_languages";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    auto refused =
        engine.assign(sessionId, SCE::ScriptSource::ecmascript("mixed"), SCE::ScriptSource::lua("1", "1")).get();
    ASSERT_FALSE(refused.isSuccess()) << "a location and an expression in two languages were assigned";
    EXPECT_NE(refused.getErrorMessage().find("location is 'ecmascript'"), std::string::npos)
        << "the refusal does not name the location's language: " << refused.getErrorMessage();

    engine.destroySession(sessionId);
}
#endif
