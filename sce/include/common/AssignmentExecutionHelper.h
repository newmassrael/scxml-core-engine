// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "core/LogMacros.h"
#include "scripting/IScriptEngine.h"
#include <functional>
#include <regex>
#include <string>

namespace SCE {

/**
 * @brief Helper class for W3C SCXML assignment execution logic
 *
 * ARCHITECTURE.md: Zero Duplication Principle
 * - Shared assignment execution strategy between Interpreter and AOT engines
 * - Single Source of Truth for system variable detection and assignment logic
 *
 * §scxml-5.4: <assign> element execution (modify the data model)
 * §scxml-5.10: System Variables (_event, _sessionid, _name, _ioprocessors, _x)
 * §scxml-B-2: System Variables are Read-Only (enforced by AssignHelper)
 *
 * Usage Pattern:
 * 1. Validate location with AssignHelper::isValidLocation() (system variable protection)
 * 2. Call AssignmentExecutionHelper::executeAssignment() with location and expression
 * 3. Handle error via callback (Interpreter: eventRaiser, AOT: engine.raise)
 */
class AssignmentExecutionHelper {
public:
    /**
     * @brief Check if expression is a system variable reference
     *
     * §scxml-5.10: System variables that require special handling
     * to preserve JavaScript object reference semantics.
     *
     * @param expr Expression to check
     * @return true if expr is a system variable reference (_event, _sessionid, etc.)
     */
    static bool isSystemVariableReference(const std::string &expr) {
        return expr == "_sessionid" || expr == "_event" || expr == "_name" || expr == "_ioprocessors" || expr == "_x";
    }

    /**
     * @brief Execute assignment with appropriate strategy based on expression type
     *
     * ARCHITECTURE.md: Zero Duplication - Single Source of Truth for assignment execution
     * Implements §scxml-5.4 <assign> semantics with proper JavaScript reference handling.
     *
     * Strategy:
     * 1. System variable reference (e.g., "Var2 = _event") → executeScript (preserves references)
     * 2. Simple variable + simple expression → evaluateExpression + setVariable
     * 3. Complex path (e.g., "data.field") → executeScript (handles nested access)
     *
     * @param jsEngine JSEngine instance
     * @param sessionId Session identifier
     * @param location Target variable (must be valid per AssignHelper::isValidLocation)
     * @param expr Expression to evaluate and assign
     * @param errorCallback Called on error with error message
     * @return true if assignment succeeded, false otherwise
     */
    static bool executeAssignment(IScriptEngine &jsEngine, const std::string &sessionId, const ScriptSource &location,
                                  const ScriptSource &expr, std::function<void(const std::string &)> errorCallback) {
        // Both parameters are ScriptSource because BOTH cross the boundary as
        // executable text: the location is not merely a name here, it is glued
        // in front of `=` and run as a script. The shape questions below —
        // is this a system variable, is this a simple name — are asked of
        // `source()`, because they are questions about what the AUTHOR wrote:
        // §scxml-5.10 names `_event`, not whatever a lowering spells it.
        //
        // §scxml-5.10: System variable references require direct script execution
        // This preserves JavaScript object references (critical for test 329: Var2 = _event)
        if (isSystemVariableReference(expr.source())) {
            const ScriptSource assignScript =
                ScriptSourceBuilder(expr.language()).add(location).add(" = ").add(expr).add(";").build();
            SCE_LOG_DEBUG("AssignmentExecutionHelper: System variable reference - executing script: {}",
                          assignScript.source());
            auto scriptResult = jsEngine.executeScript(sessionId, assignScript).get();
            if (!scriptResult.isSuccess()) {
                std::string errorMsg =
                    "System variable assignment failed: " + location.source() + " = " + expr.source();
                SCE_LOG_ERROR("AssignmentExecutionHelper: {}", errorMsg);
                errorCallback(errorMsg);
                return false;
            }
            SCE_LOG_DEBUG("AssignmentExecutionHelper: Successfully assigned {} = {} (system variable reference)",
                          location.source(), expr.source());
            return true;
        }

        // §scxml-5.4: Standard evaluation + assignment strategy
        // Step 1: Evaluate expression
        SCE_LOG_DEBUG("AssignmentExecutionHelper: Evaluating expression: {}", expr.source());
        auto evalResult = jsEngine.evaluateExpression(sessionId, expr).get();
        if (!evalResult.isSuccess()) {
            std::string errorMsg = "Expression evaluation failed: " + expr.source();
            SCE_LOG_ERROR("AssignmentExecutionHelper: {}", errorMsg);
            errorCallback(errorMsg);
            return false;
        }

        // Step 2: Assign to location
        // Simple variable names use setVariable, complex paths use executeScript
        if (std::regex_match(location.source(), std::regex("^[a-zA-Z_][a-zA-Z0-9_]*$"))) {
            // Simple variable name - use setVariable (matches Interpreter ActionExecutorImpl.cpp:160-169)
            // The engine is handed a NAME here, not text to evaluate, and a
            // bare identifier is the same name in either language.
            SCE_LOG_DEBUG("AssignmentExecutionHelper: Simple variable - using setVariable for {}", location.source());
            auto setResult = jsEngine.setVariable(sessionId, location.source(), evalResult.getInternalValue()).get();
            if (!setResult.isSuccess()) {
                std::string errorMsg = "Variable assignment failed: " + location.source();
                SCE_LOG_ERROR("AssignmentExecutionHelper: {}", errorMsg);
                errorCallback(errorMsg);
                return false;
            }
            SCE_LOG_DEBUG("AssignmentExecutionHelper: Successfully assigned {} = {}", location.source(), expr.source());
            return true;
        } else {
            // Complex path (e.g., "data.field") - use executeScript (matches Interpreter ActionExecutorImpl.cpp:174)
            const ScriptSource assignScript =
                ScriptSourceBuilder(expr.language()).add(location).add(" = (").add(expr).add(");").build();
            SCE_LOG_DEBUG("AssignmentExecutionHelper: Complex path - executing script: {}", assignScript.source());
            auto scriptResult = jsEngine.executeScript(sessionId, assignScript).get();
            if (!scriptResult.isSuccess()) {
                std::string errorMsg = "Complex path assignment failed: " + location.source();
                SCE_LOG_ERROR("AssignmentExecutionHelper: {}", errorMsg);
                errorCallback(errorMsg);
                return false;
            }
            SCE_LOG_DEBUG("AssignmentExecutionHelper: Successfully assigned {} = {} (complex path)", location.source(),
                          expr.source());
            return true;
        }
    }
};

}  // namespace SCE
