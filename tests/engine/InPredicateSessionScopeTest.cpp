// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief In() answers for the session it is evaluated in, and no other
 *
 * §scxml-5.9.1 defines In(stateId) over the evaluating session's own
 * configuration. Both script engines are one instance for every session, and
 * both answered from other sessions' configurations: JSEngine asked every
 * registered state query and returned true if any said yes, and LuaEngine did
 * the same whenever the evaluating session had no query of its own. So In('s')
 * was true in one session whenever any other — a parent, an invoked child, an
 * unrelated machine — was in a state called `s` (measured 2026-09-24 by reading
 * both engines; the JSEngine test that exercised In() evaluated it in a session
 * other than the machine's and relied on exactly this).
 *
 * `LuaEngine` and `JSEngine` are named rather than reached through
 * `ScriptEngineProvider`, as `ScriptLanguageSeamTest` next door explains: the
 * provider is a compile-time choice, so one of the two would be compiled by
 * every build and run by none.
 */

#include "scripting/IScriptEngine.h"
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

/// `In(stateId)` evaluated in `sessionId`, as a boolean; fails the test when
/// the expression does not evaluate to one.
bool inState(SCE::IScriptEngine &engine, const std::string &sessionId, const std::string &stateId) {
    auto result = engine.evaluateExpression(sessionId, "In('" + stateId + "')").get();
    EXPECT_TRUE(result.isSuccess()) << "In('" << stateId << "') in " << sessionId;
    EXPECT_TRUE(std::holds_alternative<bool>(result.getInternalValue()))
        << "In('" << stateId << "') in " << sessionId << " is not a boolean";
    return std::holds_alternative<bool>(result.getInternalValue()) && std::get<bool>(result.getInternalValue());
}

/// Three sessions of one engine: `inS` is in state `s`, `elsewhere` is in
/// state `t`, and `unqueried` registered no state query at all.
void expectEachSessionAnswersForItself(SCE::IScriptEngine &engine, const std::string &prefix) {
    const std::string inS = prefix + "_in_s";
    const std::string elsewhere = prefix + "_elsewhere";
    const std::string unqueried = prefix + "_unqueried";
    ASSERT_TRUE(engine.createSession(inS, ""));
    ASSERT_TRUE(engine.createSession(elsewhere, ""));
    ASSERT_TRUE(engine.createSession(unqueried, ""));

    engine.setStateQueryCallback([](const std::string &state) { return state == "s"; }, inS);
    engine.setStateQueryCallback([](const std::string &state) { return state == "t"; }, elsewhere);

    EXPECT_TRUE(inState(engine, inS, "s"));
    EXPECT_FALSE(inState(engine, inS, "t")) << "another session's state is not this one's";

    EXPECT_TRUE(inState(engine, elsewhere, "t"));
    EXPECT_FALSE(inState(engine, elsewhere, "s")) << "another session's state is not this one's";

    EXPECT_FALSE(inState(engine, unqueried, "s")) << "a session with no state query is in no state";
    EXPECT_FALSE(inState(engine, unqueried, "t")) << "a session with no state query is in no state";

    engine.setStateQueryCallback(nullptr, inS);
    engine.setStateQueryCallback(nullptr, elsewhere);
    engine.destroySession(inS);
    engine.destroySession(elsewhere);
    engine.destroySession(unqueried);
}

}  // namespace

#ifdef SCE_ENABLE_QUICKJS
TEST(InPredicateSessionScope, QuickJsAnswersForTheEvaluatingSession) {
    expectEachSessionAnswersForItself(SCE::JSEngine::instance(), "in_scope_js");
}
#endif

#ifdef SCE_ENABLE_LUA
TEST(InPredicateSessionScope, LuaAnswersForTheEvaluatingSession) {
    expectEachSessionAnswersForItself(SCE::LuaEngine::instance(), "in_scope_lua");
}
#endif
