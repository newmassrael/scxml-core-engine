// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief The language seam's one contract, compiled once.
 *
 * These six entry points and the refusal they share are defined here rather
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

std::future<ScriptResult> IScriptEngine::refuseLanguage(const ScriptSource &code) const {
    std::promise<ScriptResult> promise;
    promise.set_value(ScriptResult::createError(
        std::string("script engine language mismatch: this engine evaluates '") + scriptLanguageName(nativeLanguage()) +
        "' and was handed '" + scriptLanguageName(code.language()) +
        "'. Supply the engine the manifest's script_engine_language names; source: " + code.source()));
    return promise.get_future();
}

}  // namespace SCE
