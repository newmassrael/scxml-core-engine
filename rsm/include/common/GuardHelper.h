// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once
#include "common/Logger.h"
#include "scripting/JSEngine.h"
#include <optional>
#include <string>

namespace SCE::GuardHelper {

/**
 * @brief Evaluates a guard expression using JSEngine
 *
 * W3C SCXML 5.9: If a conditional expression cannot be evaluated as a boolean value
 * ('true' or 'false') or if its evaluation causes an error, the SCXML processor MUST
 * treat the expression as if it evaluated to 'false' AND place error.execution in
 * the internal event queue.
 *
 * @param jsEngine Reference to JSEngine instance
 * @param sessionId JSEngine session ID
 * @param guardExpr Guard expression to evaluate (e.g., "typeof Var4 !== 'undefined'")
 * @return std::optional<bool> - std::nullopt if evaluation failed (caller must raise error.execution),
 *                                true/false if evaluation succeeded
 */
inline std::optional<bool> evaluateGuard(JSEngine &jsEngine, const std::string &sessionId,
                                         const std::string &guardExpr) {
    auto guardResult = jsEngine.evaluateExpression(sessionId, guardExpr).get();

    if (!JSEngine::isSuccess(guardResult)) {
        // W3C SCXML 5.9: Evaluation errors → caller must raise error.execution
        LOG_WARN("W3C SCXML 5.9: Guard evaluation failed: {}", guardExpr);
        return std::nullopt;  // Signal evaluation failure
    }

    return JSEngine::resultToBool(guardResult);
}

}  // namespace SCE::GuardHelper
