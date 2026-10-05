// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "ScriptResult.h"
#include "ScriptSource.h"
#include "common/JsonText.h"
#include <optional>
#include <string>
#include <variant>
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
 * @brief The text of a scalar script value, or nothing for one that needs an engine
 *
 * A string is itself, a number is its ECMAScript `String(value)`
 * (ARCHITECTURE.md, "JSON Number Text"), a truth value is `true` or `false`, and
 * `undefined` and `null` are the empty string (§scxml-C-1). An array or an object
 * has no text without an engine to stringify it, and returns nothing.
 *
 * Header-only on purpose. Generated code under `datamodel="sce-static"` carries
 * no script engine and its component links no SCE library: it is compiled with
 * the engine's headers and one runtime source. A native `<param>` value reaches
 * its text through here and not through `resultToString`, which is compiled in
 * `ScriptResultUtils.cpp` -- measured 2026-10-06, a component built that way
 * started and the platform's loader stopped it at once on the missing symbol.
 */
inline std::optional<std::string> scalarText(const ScriptValue &value) {
    if (std::holds_alternative<std::string>(value)) {
        return std::get<std::string>(value);
    }
    if (std::holds_alternative<double>(value)) {
        // §scxml-B-1: the data model is ECMAScript, so a number's text is its
        // `String(value)` -- the one spelling every engine writes. Neither
        // iostream's spelling is it: `oss << nan` writes "nan", and the default
        // precision writes pi as `3.14159`.
        return JsonText::numberText(std::get<double>(value));
    }
    if (std::holds_alternative<int64_t>(value)) {
        return std::to_string(std::get<int64_t>(value));
    }
    if (std::holds_alternative<bool>(value)) {
        return std::string(std::get<bool>(value) ? "true" : "false");
    }
    if (std::holds_alternative<ScriptUndefined>(value) || std::holds_alternative<ScriptNull>(value)) {
        // §scxml-C-1: undefined evaluates to the empty string for target
        // expressions, so isUnreachableTarget() works across every script engine.
        return std::string();
    }
    return std::nullopt;
}

/**
 * @brief The text of a value a native expression computed, with no script engine
 *
 * `resultToString(ScriptResult::createSuccess(value))` for a scalar, without the
 * library (see `scalarText`). A value that is not a scalar has no engine to be
 * stringified by here and reads as the marker `resultToString` gives it.
 */
inline std::string valueText(const ScriptValue &value) {
    const auto text = scalarText(value);
    return text ? *text : std::string("[conversion_error]");
}

/**
 * @brief Convert ScriptResult to string with optional JSON.stringify fallback
 * @param result Script engine execution result
 * @param engine Optional engine for JSON.stringify (nullptr disables fallback)
 * @param sessionId Session ID for JSON.stringify evaluation
 * @param originalExpression Original expression for complex objects
 * @return String representation or error message
 */
std::string resultToString(const ScriptResult &result, IScriptEngine *engine = nullptr,
                           const std::string &sessionId = "", const ScriptSource &originalExpression = "");

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
                                             const ScriptSource &originalExpression = "");

/**
 * @brief Extract ScriptValue array elements directly from ScriptResult
 *
 * Foreach array element extraction without string round-trip.
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
                                                  const ScriptSource &originalExpression = "");

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
