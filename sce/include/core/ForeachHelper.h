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
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once
#include "core/LogMacros.h"
#include "scripting/ScriptResultUtils.h"
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace SCE::Core {

/**
 * @brief Helper for W3C SCXML 4.6 foreach loop variable handling
 *
 * Single Source of Truth for foreach variable setting logic shared between engines.
 * Ensures proper variable declaration and type preservation.
 *
 * @note All methods are templatized on JSEngineType to avoid concrete JSEngine
 * dependency. This keeps ForeachHelper usable from sce_core (header-only)
 * without pulling in scripting/JSEngine.h and its transitive dependencies
 * (QuickJS, threading, etc.). JSEngineType is deduced automatically at call sites.
 *
 * JSEngineType contract (satisfied by SCE::JSEngine):
 * - auto evaluateExpression(sessionId, expr).get() → result with isSuccess(), getInternalValue()
 * - auto executeScript(sessionId, script).get()    → result with isSuccess()
 * - static resultToStringArray(result, sessionId, expr) → std::vector<std::string>
 */
class ForeachHelper {
public:
    /**
     * @brief Sets a loop variable with proper W3C SCXML 4.6 compliance
     *
     * Handles variable declaration and type preservation for foreach loop variables.
     * If the variable doesn't exist, it will be declared with 'var' keyword.
     * This is the Single Source of Truth for foreach variable setting logic,
     * shared between Interpreter and AOT engines.
     *
     * @tparam JSEngineType Script engine type (deduced from jsEngine argument)
     * @param jsEngine Reference to script engine instance
     * @param sessionId Script engine session ID
     * @param varName Variable name to set
     * @param value JavaScript literal value (e.g., "1", "undefined", "null")
     * @return true if successful, false otherwise
     */
    /**
     * @brief Validate that a variable name is a legal ECMAScript identifier
     * W3C SCXML B.2: Legal values for 'item' attribute are legal ECMAScript variable names
     */
    static inline bool isLegalVariableName(const std::string &name) {
        if (name.empty()) return false;
        // Must not be quoted (e.g., 'continue' or "continue")
        if (name.front() == '\'' || name.front() == '"') return false;
        // Must start with letter, underscore, or dollar sign
        if (!std::isalpha(name[0]) && name[0] != '_' && name[0] != '$') return false;
        // Must contain only alphanumeric, underscore, or dollar sign
        for (char c : name) {
            if (!std::isalnum(c) && c != '_' && c != '$') return false;
        }
        return true;
    }

    /**
     * @brief Set a loop variable directly from a ScriptValue (no string round-trip)
     *
     * W3C SCXML 4.6: Preserves type information for objects, arrays, and all primitive types.
     */
    template <typename JSEngineType>
    static inline bool setLoopVariable(JSEngineType &jsEngine, const std::string &sessionId, const std::string &varName,
                                       const ScriptValue &value) {
        try {
            if (!isLegalVariableName(varName)) {
                SCE_LOG_ERROR("W3C FOREACH: Illegal variable name '{}'", varName);
                return false;
            }

            auto setResult = jsEngine.setVariable(sessionId, varName, value).get();
            if (!setResult.isSuccess()) {
                SCE_LOG_ERROR("Failed to set foreach variable '{}'", varName);
                return false;
            }

            SCE_LOG_DEBUG("Set foreach variable: {} (ScriptValue direct)", varName);
            return true;

        } catch (const std::exception &e) {
            SCE_LOG_ERROR("Exception setting foreach variable {}: {}", varName, e.what());
            return false;
        }
    }

    /**
     * @brief Set a loop variable from a string expression (evaluates then sets)
     *
     * Used for index variables and initial "undefined" declarations.
     */
    template <typename JSEngineType>
    static inline bool setLoopVariableFromExpr(JSEngineType &jsEngine, const std::string &sessionId,
                                               const std::string &varName, const std::string &expr) {
        try {
            if (!isLegalVariableName(varName)) {
                SCE_LOG_ERROR("W3C FOREACH: Illegal variable name '{}'", varName);
                return false;
            }

            auto evalResult = jsEngine.evaluateExpression(sessionId, expr).get();
            if (evalResult.isSuccess()) {
                auto setResult = jsEngine.setVariable(sessionId, varName, evalResult.getInternalValue()).get();
                if (setResult.isSuccess()) {
                    SCE_LOG_DEBUG("Set foreach variable: {} = {}", varName, expr);
                    return true;
                }
            }

            // Fallback: try as string literal
            auto setStrResult = jsEngine.setVariable(sessionId, varName, ScriptValue(expr)).get();
            if (!setStrResult.isSuccess()) {
                SCE_LOG_ERROR("Failed to set foreach variable {} = {}", varName, expr);
                return false;
            }
            return true;

        } catch (const std::exception &e) {
            SCE_LOG_ERROR("Exception setting foreach variable {}: {}", varName, e.what());
            return false;
        }
    }

    /**
     * @brief Evaluates a foreach array expression using script engine
     *
     * W3C SCXML 5.4: The 'array' attribute must evaluate to an iterable collection,
     * specifically objects that satisfy instanceof(Array) in ECMAScript.
     * Non-array values (numbers, strings, booleans, objects) must raise error.execution.
     *
     * @tparam JSEngineType Script engine type (deduced from jsEngine argument)
     * @param jsEngine Reference to script engine instance
     * @param sessionId Script engine session ID
     * @param arrayExpr Array expression to evaluate (e.g., "Var3", "[1,2,3]")
     * @return std::optional<std::vector<ScriptValue>> Array element values, or std::nullopt on failure
     */
    template <typename JSEngineType>
    static inline std::optional<std::vector<ScriptValue>>
    evaluateForeachArray(JSEngineType &jsEngine, const std::string &sessionId, const std::string &arrayExpr) {
        // W3C SCXML 4.6: Array expression must be non-empty
        if (arrayExpr.empty()) {
            SCE_LOG_ERROR("Foreach array attribute is missing or empty");
            return std::nullopt;
        }

        auto arrayResult = jsEngine.evaluateExpression(sessionId, arrayExpr).get();

        if (!arrayResult.isSuccess()) {
            SCE_LOG_ERROR("Failed to evaluate array expression: {}", arrayExpr);
            return std::nullopt;
        }

        // W3C SCXML 5.4: Validate that the value is an array
        if (!arrayResult.isArray()) {
            // Empty Lua table converts to ScriptObject with no properties — treat as empty array
            const auto &val = arrayResult.getInternalValue();
            if (std::holds_alternative<std::shared_ptr<ScriptObject>>(val)) {
                auto obj = std::get<std::shared_ptr<ScriptObject>>(val);
                if (obj && obj->properties.empty()) {
                    return std::vector<ScriptValue>{};  // Empty array
                }
            }

            std::string arrayCheckExpr = "(" + arrayExpr + ") instanceof Array";
            auto arrayCheckResult = jsEngine.evaluateExpression(sessionId, arrayCheckExpr).get();
            if (!arrayCheckResult.isSuccess() ||
                !std::holds_alternative<bool>(arrayCheckResult.getInternalValue()) ||
                !std::get<bool>(arrayCheckResult.getInternalValue())) {
                SCE_LOG_ERROR("Foreach array '{}' is not an iterable collection (W3C SCXML 5.4)", arrayExpr);
                return std::nullopt;
            }
        }

        // W3C SCXML 4.6: Extract elements as ScriptValue directly (no string round-trip)
        return ScriptResultUtils::resultToScriptValueArray(arrayResult, &jsEngine, sessionId, arrayExpr);
    }

    /**
     * @brief Sets foreach iteration variables (item and optional index)
     *
     * Uses the shared setLoopVariable logic to ensure consistency between
     * Interpreter and AOT engines (ARCHITECTURE.md: Logic Commonization).
     *
     * @tparam JSEngineType Script engine type (deduced from jsEngine argument)
     * @param jsEngine Reference to script engine instance
     * @param sessionId Script engine session ID
     * @param itemVar Item variable name (e.g., "Var1")
     * @param itemValue Current iteration value (JavaScript literal)
     * @param indexVar Index variable name (empty string if not used)
     * @param indexValue Current iteration index
     * @return true if successful, false on failure
     */
    template <typename JSEngineType>
    static inline bool setForeachIterationVariables(JSEngineType &jsEngine, const std::string &sessionId,
                                                    const std::string &itemVar, const ScriptValue &itemValue,
                                                    const std::string &indexVar, size_t indexValue) {
        // Set item variable directly from ScriptValue (no string round-trip)
        if (!setLoopVariable(jsEngine, sessionId, itemVar, itemValue)) {
            SCE_LOG_ERROR("Failed to set foreach item variable: {}", itemVar);
            return false;
        }

        // Set index variable (if provided)
        if (!indexVar.empty()) {
            if (!setLoopVariable(jsEngine, sessionId, indexVar, ScriptValue(static_cast<int64_t>(indexValue)))) {
                SCE_LOG_ERROR("Failed to set foreach index variable: {}", indexVar);
                return false;
            }
        }
        return true;
    }

    /**
     * @brief Executes a foreach loop without iteration body (for variable declaration only)
     *
     * Used when foreach is used solely for declaring variables without executing actions
     * (W3C SCXML 4.6 allows empty foreach for variable declaration)
     *
     * @tparam JSEngineType Script engine type (deduced from jsEngine argument)
     * @param jsEngine Reference to script engine instance
     * @param sessionId Script engine session ID
     * @param arrayExpr Array expression to evaluate
     * @param itemVar Item variable name
     * @param indexVar Index variable name (empty if not used)
     * @return true if successful, false on failure
     */
    template <typename JSEngineType>
    static inline bool executeForeachWithoutBody(JSEngineType &jsEngine, const std::string &sessionId,
                                                 const std::string &arrayExpr, const std::string &itemVar,
                                                 const std::string &indexVar) {
        auto arrayValuesOpt = evaluateForeachArray(jsEngine, sessionId, arrayExpr);
        if (!arrayValuesOpt.has_value()) {
            return false;
        }
        auto &arrayValues = arrayValuesOpt.value();

        // W3C SCXML 4.6: Declare item and index variables even for empty arrays
        if (!setLoopVariable(jsEngine, sessionId, itemVar, ScriptValue(ScriptUndefined{}))) {
            SCE_LOG_ERROR("Failed to declare foreach item variable: {}", itemVar);
            return false;
        }

        if (!indexVar.empty()) {
            if (!setLoopVariable(jsEngine, sessionId, indexVar, ScriptValue(ScriptUndefined{}))) {
                SCE_LOG_ERROR("Failed to declare foreach index variable: {}", indexVar);
                return false;
            }
        }

        for (size_t i = 0; i < arrayValues.size(); ++i) {
            if (!setForeachIterationVariables(jsEngine, sessionId, itemVar, arrayValues[i], indexVar, i)) {
                return false;
            }
        }
        return true;
    }

    /**
     * @brief Executes a foreach loop with custom action body and W3C 4.6 compliant error handling
     *
     * This is the Single Source of Truth for foreach error handling logic,
     * eliminating code duplication between Interpreter and AOT engines.
     *
     * W3C SCXML 4.6 Compliance:
     * "If the evaluation of any child element of foreach causes an error,
     * the processor MUST cease execution of the foreach element and the block that contains it."
     *
     * @tparam JSEngineType Script engine type (deduced from jsEngine argument)
     * @tparam BodyFunc Callable type (lambda, function pointer, functor) that takes (size_t iteration)
     *                  and returns bool (true = continue, false = stop loop due to error)
     * @param jsEngine Reference to script engine instance
     * @param sessionId Script engine session ID
     * @param arrayExpr Array expression to evaluate (e.g., "Var3", "[1,2,3]")
     * @param itemVar Item variable name (e.g., "Var2")
     * @param indexVar Index variable name (empty string if not used)
     * @param executeBody Lambda/callable that executes iteration actions.
     *                    Return true to continue, false to stop loop (W3C 4.6 error handling)
     * @return true if all iterations succeeded, false if loop was stopped due to error
     * @throws std::runtime_error if array evaluation or variable setting fails
     *
     * @example Interpreter Engine usage:
     * @code
     * bool success = ForeachHelper::executeForeachWithActions(
     *     jsEngine, sessionId, "myArray", "item", "index",
     *     [&](size_t i) {
     *         // Execute nested actions
     *         for (const auto& action : nestedActions) {
     *             if (!action->execute(context)) {
     *                 return false;  // Stop loop on error (W3C 4.6)
     *             }
     *         }
     *         return true;
     *     }
     * );
     * @endcode
     *
     * @example AOT Engine usage (generated code):
     * @code
     * bool success = ::SCE::Core::ForeachHelper::executeForeachWithActions(
     *     jsEngine, sessionId_.value(), "Var3", "Var2", "",
     *     [&](size_t i) {
     *         // Custom C++ generated code
     *         auto result = jsEngine.evaluateExpression(sessionId_.value(), "Var1 + 1").get();
     *         if (!result.isSuccess()) {
     *             SCE_LOG_ERROR("Expression evaluation failed");
     *             return false;  // Stop loop on error (W3C 4.6)
     *         }
     *         jsEngine.setVariable(sessionId_.value(), "Var1", result.getInternalValue());
     *         return true;
     *     }
     * );
     * @endcode
     */
    template <typename JSEngineType, typename BodyFunc>
    static inline bool executeForeachWithActions(JSEngineType &jsEngine, const std::string &sessionId,
                                                 const std::string &arrayExpr, const std::string &itemVar,
                                                 const std::string &indexVar, BodyFunc &&executeBody) {
        // Evaluate array expression — returns ScriptValue elements directly (no string round-trip)
        auto arrayValuesOpt = evaluateForeachArray(jsEngine, sessionId, arrayExpr);
        if (!arrayValuesOpt.has_value()) {
            return false;
        }
        auto &arrayValues = arrayValuesOpt.value();

        // W3C SCXML 4.6: Declare item and index variables BEFORE iteration
        if (!setLoopVariable(jsEngine, sessionId, itemVar, ScriptValue(ScriptUndefined{}))) {
            SCE_LOG_ERROR("Failed to declare foreach item variable: {}", itemVar);
            return false;
        }

        if (!indexVar.empty()) {
            if (!setLoopVariable(jsEngine, sessionId, indexVar, ScriptValue(ScriptUndefined{}))) {
                SCE_LOG_ERROR("Failed to declare foreach index variable: {}", indexVar);
                return false;
            }
        }

        // W3C SCXML 4.6: Execute foreach loop with error handling
        for (size_t i = 0; i < arrayValues.size(); ++i) {
            if (!setForeachIterationVariables(jsEngine, sessionId, itemVar, arrayValues[i], indexVar, i)) {
                return false;
            }

            // Execute body actions for this iteration
            // W3C SCXML 4.6: If body returns false (error), stop loop execution
            if (!executeBody(i)) {
                SCE_LOG_DEBUG("Foreach loop stopped at iteration {} due to error (W3C SCXML 4.6)", i);
                return false;  // Single Source of Truth for W3C 4.6 compliance
            }
        }

        return true;  // All iterations succeeded
    }
};

}  // namespace SCE::Core
