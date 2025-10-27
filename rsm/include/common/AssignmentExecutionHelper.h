#pragma once

#include "common/Logger.h"
#include "scripting/JSEngine.h"
#include <functional>
#include <regex>
#include <string>

namespace RSM {

/**
 * @brief Helper class for W3C SCXML assignment execution logic
 *
 * ARCHITECTURE.md: Zero Duplication Principle
 * - Shared assignment execution strategy between Interpreter and AOT engines
 * - Single Source of Truth for system variable detection and assignment logic
 *
 * W3C SCXML 5.3: Data Model Assignment
 * W3C SCXML 5.10: System Variables (_event, _sessionid, _name, _ioprocessors, _x)
 * W3C SCXML B.2: System Variables are Read-Only (enforced by AssignHelper)
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
     * W3C SCXML 5.10: System variables that require special handling
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
     * Implements W3C SCXML 5.3 assignment semantics with proper JavaScript reference handling.
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
    static bool executeAssignment(JSEngine &jsEngine, const std::string &sessionId, const std::string &location,
                                  const std::string &expr, std::function<void(const std::string &)> errorCallback) {
        // W3C SCXML 5.10: System variable references require direct script execution
        // This preserves JavaScript object references (critical for test 329: Var2 = _event)
        if (isSystemVariableReference(expr)) {
            std::string assignScript = location + " = " + expr + ";";
            LOG_DEBUG("AssignmentExecutionHelper: System variable reference - executing script: {}", assignScript);
            auto scriptResult = jsEngine.executeScript(sessionId, assignScript).get();
            if (!scriptResult.isSuccess()) {
                std::string errorMsg = "System variable assignment failed: " + location + " = " + expr;
                LOG_ERROR("AssignmentExecutionHelper: {}", errorMsg);
                errorCallback(errorMsg);
                return false;
            }
            LOG_DEBUG("AssignmentExecutionHelper: Successfully assigned {} = {} (system variable reference)", location,
                      expr);
            return true;
        }

        // W3C SCXML 5.3: Standard evaluation + assignment strategy
        // Step 1: Evaluate expression
        LOG_DEBUG("AssignmentExecutionHelper: Evaluating expression: {}", expr);
        auto evalResult = jsEngine.evaluateExpression(sessionId, expr).get();
        if (!evalResult.isSuccess()) {
            std::string errorMsg = "Expression evaluation failed: " + expr;
            LOG_ERROR("AssignmentExecutionHelper: {}", errorMsg);
            errorCallback(errorMsg);
            return false;
        }

        // Step 2: Assign to location
        // Simple variable names use setVariable, complex paths use executeScript
        if (std::regex_match(location, std::regex("^[a-zA-Z_][a-zA-Z0-9_]*$"))) {
            // Simple variable name - use setVariable (matches Interpreter ActionExecutorImpl.cpp:160-169)
            LOG_DEBUG("AssignmentExecutionHelper: Simple variable - using setVariable for {}", location);
            auto setResult = jsEngine.setVariable(sessionId, location, evalResult.getInternalValue()).get();
            if (!setResult.isSuccess()) {
                std::string errorMsg = "Variable assignment failed: " + location;
                LOG_ERROR("AssignmentExecutionHelper: {}", errorMsg);
                errorCallback(errorMsg);
                return false;
            }
            LOG_DEBUG("AssignmentExecutionHelper: Successfully assigned {} = {}", location, expr);
            return true;
        } else {
            // Complex path (e.g., "data.field") - use executeScript (matches Interpreter ActionExecutorImpl.cpp:174)
            std::string assignScript = location + " = (" + expr + ");";
            LOG_DEBUG("AssignmentExecutionHelper: Complex path - executing script: {}", assignScript);
            auto scriptResult = jsEngine.executeScript(sessionId, assignScript).get();
            if (!scriptResult.isSuccess()) {
                std::string errorMsg = "Complex path assignment failed: " + location;
                LOG_ERROR("AssignmentExecutionHelper: {}", errorMsg);
                errorCallback(errorMsg);
                return false;
            }
            LOG_DEBUG("AssignmentExecutionHelper: Successfully assigned {} = {} (complex path)", location, expr);
            return true;
        }
    }
};

}  // namespace RSM
