// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once
#include "core/LogMacros.h"
#include "scripting/IScriptEngine.h"
#include <optional>
#include <string>

namespace SCE::GuardHelper {

/**
 * @brief Evaluates a guard expression using a JavaScript execution engine
 *
 * §scxml-5.9: If a conditional expression cannot be evaluated as a boolean value
 * ('true' or 'false') or if its evaluation causes an error, the SCXML processor MUST
 * treat the expression as if it evaluated to 'false' AND place error.execution in
 * the internal event queue.
 *
 * @param jsEngine Reference to IScriptEngine instance
 * @param sessionId Session ID
 * @param guardExpr Guard expression to evaluate (e.g., "typeof Var4 !== 'undefined'")
 * @return std::optional<bool> - std::nullopt if evaluation failed (caller must raise error.execution),
 *                                true/false if evaluation succeeded
 */
inline std::optional<bool> evaluateGuard(IScriptEngine &jsEngine, const std::string &sessionId,
                                         const ScriptSource &guardExpr) {
    auto guardResult = jsEngine.evaluateExpression(sessionId, guardExpr).get();

    if (!guardResult.isSuccess()) {
        // §scxml-5.9: Evaluation errors → caller must raise error.execution
        // Named by what the author wrote, not by what the engine was handed:
        // a lowered guard in this line points at a line nobody can find.
        SCE_LOG_WARN("W3C SCXML 5.9: Guard evaluation failed: {}", guardExpr.source());
        return std::nullopt;  // Signal evaluation failure
    }

    return guardResult.toBool();
}

}  // namespace SCE::GuardHelper
