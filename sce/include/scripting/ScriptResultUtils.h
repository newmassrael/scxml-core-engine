#pragma once

#include "JSResult.h"
#include <optional>
#include <string>
#include <vector>

namespace SCE {

class IScriptEngine;

/**
 * @brief Engine-agnostic result processing utilities for JSResult
 *
 * Extracted from JSEngine to break circular dependencies and enable
 * pluggable script engines. Uses JSResult's public API only (no friend access).
 */
namespace ScriptResultUtils {

/**
 * @brief Convert JSResult to boolean with W3C SCXML semantics
 * @param result Script engine execution result
 * @return Boolean value following ECMAScript truthy rules
 */
bool resultToBool(const JSResult &result);

/**
 * @brief Convert JSResult to string with optional JSON.stringify fallback
 * @param result Script engine execution result
 * @param engine Optional engine for JSON.stringify (nullptr disables fallback)
 * @param sessionId Session ID for JSON.stringify evaluation
 * @param originalExpression Original expression for complex objects
 * @return String representation or error message
 */
std::string resultToString(const JSResult &result, IScriptEngine *engine = nullptr,
                           const std::string &sessionId = "", const std::string &originalExpression = "");

/**
 * @brief Convert JSResult to string array for SCXML foreach actions
 * @param result Script engine evaluation result of array expression
 * @param engine Optional engine for element evaluation
 * @param sessionId Session for additional evaluation if needed
 * @param originalExpression Original expression for JSON.stringify fallback
 * @return Vector of string representations
 */
std::vector<std::string> resultToStringArray(const JSResult &result, IScriptEngine *engine = nullptr,
                                             const std::string &sessionId = "",
                                             const std::string &originalExpression = "");

/**
 * @brief Check if result represents successful operation
 * @param result Script engine execution result
 * @return true if operation succeeded
 */
bool isSuccess(const JSResult &result) noexcept;

/**
 * @brief Require successful result or throw exception
 * @param result Script engine result to validate
 * @param operation Operation context for error message
 * @throws std::runtime_error if result indicates failure
 */
void requireSuccess(const JSResult &result, const std::string &operation);

/**
 * @brief Extract typed value from JSResult safely
 * @tparam T Target type (bool, int64_t, double, std::string)
 * @param result Script engine execution result
 * @return Optional typed value (nullopt on type mismatch or failure)
 */
template <typename T> std::optional<T> resultToValue(const JSResult &result) {
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
