// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "ScriptResult.h"
#include <optional>
#include <string>
#include <vector>

namespace SCE {

class IScriptEngine;

/**
 * @brief Engine-agnostic result processing utilities for ScriptResult
 *
 * Extracted from JSEngine to break circular dependencies and enable
 * pluggable script engines. Uses ScriptResult's public API only (no friend access).
 */
namespace ScriptResultUtils {

/**
 * @brief Convert ScriptResult to boolean with W3C SCXML semantics
 * @param result Script engine execution result
 * @return Boolean value following ECMAScript truthy rules
 */
bool resultToBool(const ScriptResult &result);

/**
 * @brief Convert ScriptResult to string with optional JSON.stringify fallback
 * @param result Script engine execution result
 * @param engine Optional engine for JSON.stringify (nullptr disables fallback)
 * @param sessionId Session ID for JSON.stringify evaluation
 * @param originalExpression Original expression for complex objects
 * @return String representation or error message
 */
std::string resultToString(const ScriptResult &result, IScriptEngine *engine = nullptr,
                           const std::string &sessionId = "", const std::string &originalExpression = "");

/**
 * @brief Convert ScriptResult to string array for SCXML foreach actions
 * @param result Script engine evaluation result of array expression
 * @param engine Optional engine for element evaluation
 * @param sessionId Session for additional evaluation if needed
 * @param originalExpression Original expression for JSON.stringify fallback
 * @return Vector of string representations
 */
std::vector<std::string> resultToStringArray(const ScriptResult &result, IScriptEngine *engine = nullptr,
                                             const std::string &sessionId = "",
                                             const std::string &originalExpression = "");

/**
 * @brief Extract ScriptValue array elements directly from ScriptResult
 *
 * W3C SCXML 4.6: Foreach array element extraction without string round-trip.
 * Preserves type information for objects, arrays, and all primitive types.
 * Falls back to engine evaluation for non-ScriptArray results.
 *
 * @param result Script engine evaluation result of array expression
 * @param engine Script engine for fallback evaluation
 * @param sessionId Session ID for fallback evaluation
 * @param originalExpression Original expression for fallback
 * @return Vector of ScriptValue elements, empty on failure
 */
std::vector<ScriptValue> resultToScriptValueArray(const ScriptResult &result, IScriptEngine *engine = nullptr,
                                                   const std::string &sessionId = "",
                                                   const std::string &originalExpression = "");

/**
 * @brief Check if result represents successful operation
 * @param result Script engine execution result
 * @return true if operation succeeded
 */
bool isSuccess(const ScriptResult &result) noexcept;

/**
 * @brief Require successful result or throw exception
 * @param result Script engine result to validate
 * @param operation Operation context for error message
 * @throws std::runtime_error if result indicates failure
 */
void requireSuccess(const ScriptResult &result, const std::string &operation);

/**
 * @brief Extract typed value from ScriptResult safely
 * @tparam T Target type (bool, int64_t, double, std::string)
 * @param result Script engine execution result
 * @return Optional typed value (nullopt on type mismatch or failure)
 */
template <typename T> std::optional<T> resultToValue(const ScriptResult &result) {
    static_assert(std::is_same_v<T, bool> || std::is_same_v<T, int64_t> || std::is_same_v<T, double> ||
                      std::is_same_v<T, std::string>,
                  "Supported types: bool, int64_t, double, std::string");

    if (!result.isSuccess() || !std::holds_alternative<T>(result.getInternalValue())) {
        return std::nullopt;
    }
    return std::get<T>(result.getInternalValue());
}

}  // namespace ScriptResultUtils
}  // namespace SCE
