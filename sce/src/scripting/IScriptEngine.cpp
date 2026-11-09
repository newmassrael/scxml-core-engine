// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief The language seam's one contract, compiled once.
 *
 * These entry points and the refusal they share are defined here rather
 * than inline in `IScriptEngine.h` for two reasons, and the second is measured:
 *
 *  1. The check they perform is a CONTRACT, not an engine detail. Every
 *     implementation reaches its own work through the same gate, so the gate
 *     belongs in one translation unit — the same reasoning that made the
 *     public entry points non-virtual in the first place.
 *  2. `IScriptEngine.h` is included by every generated state machine. Carrying
 *     these bodies inline changed GCC 13's inlining decisions in those
 *     translation units and surfaced the known `-Wmaybe-uninitialized` false
 *     positive in `std::variant`'s move constructor
 *     (`W3CTestRunner_Test561.cpp`, which compiled clean at 8023a18b41 and
 *     failed under `-Werror` with the bodies in the header). The repo already
 *     names that false positive at `tests/CMakeLists.txt:221`. Suppressing the
 *     warning on one more target would have hidden a header that had simply
 *     grown code it did not need to carry.
 *
 * Lives in `sce_base` rather than `sce_scripting` because it is engine-
 * agnostic — it touches `ScriptResult` value types and nothing else — and
 * because `sce_base` is what every consumer of the interface links.
 */

#include "scripting/IScriptEngine.h"

namespace SCE {

bool IScriptEngine::createSession(const std::string &sessionId, const std::string &parentSessionId) {
    return !createSessionChecked(sessionId, parentSessionId).has_value();
}

std::future<ScriptResult> IScriptEngine::executeScript(const std::string &sessionId, const ScriptSource &script) {
    if (!acceptsLanguage(script.language())) {
        return refuseLanguage(script);
    }
    return doExecuteScript(sessionId, script);
}

std::future<ScriptResult> IScriptEngine::evaluateExpression(const std::string &sessionId,
                                                            const ScriptSource &expression) {
    if (!acceptsLanguage(expression.language())) {
        return refuseLanguage(expression);
    }
    return doEvaluateExpression(sessionId, expression);
}

std::future<ScriptResult> IScriptEngine::validateExpression(const std::string &sessionId,
                                                            const ScriptSource &expression) {
    if (!acceptsLanguage(expression.language())) {
        return refuseLanguage(expression);
    }
    return doValidateExpression(sessionId, expression);
}

std::future<ScriptResult> IScriptEngine::assign(const std::string &sessionId, const ScriptSource &location,
                                                const ScriptSource &expr) {
    if (!acceptsLanguage(location.language())) {
        return refuseLanguage(location);
    }
    if (!acceptsLanguage(expr.language())) {
        return refuseLanguage(expr);
    }
    // The two halves become one statement, so they have to be one language.
    // `ScriptSourceBuilder` would drop the odd half and build a statement with
    // a hole in it; refusing here names both halves instead.
    if (location.language() != expr.language()) {
        std::promise<ScriptResult> promise;
        promise.set_value(ScriptResult::createError(
            std::string("an assignment's location is '") + scriptLanguageName(location.language()) +
            "' and its expression '" + scriptLanguageName(expr.language()) +
            "'; both halves of one statement are one language. location: " + location.source() +
            ", expression: " + expr.source()));
        return promise.get_future();
    }
    return doAssign(sessionId, location, expr);
}

namespace {

/// The value expression as the operand of `=`: one trailing `;`, and the
/// whitespace around it, dropped. W3C's own corpus writes
/// `expr="new testobject();"` (test 452), an author copying a statement into
/// an attribute that takes an expression, and `sce-build`'s frontend accepts
/// and drops that `;` when it parses an expression
/// (`ecmascript::parser::parse_expression`). `;` ends a statement in Lua as
/// well, so the same rule serves the lowered half.
std::string asOperand(const std::string &expression) {
    static constexpr const char *kSpace = " \t\r\n\f\v";
    auto end = expression.find_last_not_of(kSpace);
    if (end != std::string::npos && expression[end] == ';') {
        end = end == 0 ? std::string::npos : expression.find_last_not_of(kSpace, end - 1);
    }
    return end == std::string::npos ? std::string() : expression.substr(0, end + 1);
}

}  // namespace

ScriptSource IScriptEngine::assignmentOperand(const ScriptSource &expr) {
    return expr.language() == ScriptLanguage::Lua ? ScriptSource::lua(asOperand(expr.text()), asOperand(expr.source()))
                                                  : ScriptSource::ecmascript(asOperand(expr.source()));
}

ScriptSource IScriptEngine::assignmentStatement(const ScriptSource &location, const ScriptSource &expr) {
    return ScriptSourceBuilder(expr.language())
        .add(location)
        .add(" = (")
        .add(assignmentOperand(expr))
        .add(");")
        .build();
}

std::future<ScriptResult> IScriptEngine::refuseLanguage(const ScriptSource &code) const {
    std::promise<ScriptResult> promise;
    promise.set_value(ScriptResult::createError(
        std::string("script engine language mismatch: this engine evaluates '") + scriptLanguageName(nativeLanguage()) +
        "' and was handed '" + scriptLanguageName(code.language()) +
        "'. Supply the engine the manifest's script_engine_language names; source: " + code.source()));
    return promise.get_future();
}

}  // namespace SCE
