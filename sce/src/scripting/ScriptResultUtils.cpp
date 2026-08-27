// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/ScriptResultUtils.h"
#include "core/LogMacros.h"
#include "scripting/IScriptEngine.h"
#include "scripting/ScriptDialect.h"
#include <cmath>
#include <sstream>

namespace SCE::ScriptResultUtils {

bool resultToBool(const ScriptResult &result) {
    return result.toBool();
}

std::string resultToString(const ScriptResult &result, IScriptEngine *engine, const std::string &sessionId,
                           const ScriptSource &originalExpression) {
    if (!result.isSuccess()) {
        return "";
    }

    const auto &value = result.getInternalValue();

    if (std::holds_alternative<std::string>(value)) {
        return result.getValue<std::string>();
    } else if (std::holds_alternative<double>(value)) {
        double val = result.getValue<double>();
        // §scxml-B-1: the data model is ECMAScript, so a number's text is its
        // `String(value)`. The three non-finite spellings are ECMAScript's, not
        // iostream's — `oss << nan` writes "nan", which is a C++ fact about a
        // value the document wrote as `NaN`.
        //
        // The magnitude guard is not decoration. `std::floor(inf) == inf`, so
        // an infinity used to take the integer branch below and reach
        // `static_cast<int64_t>(inf)`, which is undefined behaviour — measured
        // as INT64_MIN, i.e. a `<param>` carrying `-9223372036854775808` where
        // the document sent Infinity. Every finite double above 2^63 casts the
        // same way. The four ported runtimes (Rust, Go, Python, Kotlin) already
        // carry this bound; this is the original catching up with its ports.
        if (std::isnan(val)) {
            return "NaN";
        }
        if (std::isinf(val)) {
            return val > 0 ? "Infinity" : "-Infinity";
        }
        if (val == std::floor(val) && std::fabs(val) < 1e15) {
            return std::to_string(static_cast<int64_t>(val));
        } else {
            // W3C SCXML: Use ECMAScript-compatible number formatting
            std::ostringstream oss;
            oss << std::noshowpoint << val;
            std::string str = oss.str();

            if (str.find('.') != std::string::npos) {
                str.erase(str.find_last_not_of('0') + 1, std::string::npos);
                if (str.back() == '.') {
                    str.pop_back();
                }
            }
            return str;
        }
    } else if (std::holds_alternative<int64_t>(value)) {
        return std::to_string(result.getValue<int64_t>());
    } else if (std::holds_alternative<bool>(value)) {
        return result.getValue<bool>() ? "true" : "false";
    } else if (std::holds_alternative<ScriptUndefined>(value)) {
        // §scxml-C-1: undefined evaluates to empty string for target expressions
        // Ensures isUnreachableTarget() works correctly across all script engines
        return "";
    } else if (std::holds_alternative<ScriptNull>(value)) {
        return "";
    } else if (engine && !sessionId.empty() && !originalExpression.text().empty()) {
        // JSON.stringify fallback using provided engine. Composed through the
        // dialect table rather than spelled inline: the wrapper has to be in
        // the same language as the expression it wraps, and on a pre-lowered
        // path there is no rewriter left to repair a mismatch.
        auto stringifyResult =
            engine->evaluateExpression(sessionId, ScriptDialect::stringify(originalExpression)).get();
        if (stringifyResult.isSuccess()) {
            return stringifyResult.getValue<std::string>();
        }
        return "[object]";
    }
    return "[conversion_error]";
}

std::vector<std::string> resultToStringArray(const ScriptResult &result, IScriptEngine *engine,
                                             const std::string &sessionId, const ScriptSource &originalExpression) {
    std::vector<std::string> arrayValues;

    SCE_LOG_DEBUG("resultToStringArray: Starting with sessionId='{}', originalExpression='{}'", sessionId,
                  originalExpression.source());

    if (!result.isSuccess()) {
        SCE_LOG_DEBUG("resultToStringArray: Result not successful, returning empty array");
        return arrayValues;
    }

    const auto &value = result.getInternalValue();
    std::string arrayStr;

    // Direct ScriptArray extraction (engine-agnostic, works with both QuickJS and Lua)
    if (std::holds_alternative<std::shared_ptr<ScriptArray>>(value)) {
        auto arr = std::get<std::shared_ptr<ScriptArray>>(value);
        if (arr) {
            for (const auto &elem : arr->elements) {
                arrayValues.push_back(std::visit(
                    [](const auto &v) -> std::string {
                        using T = std::decay_t<decltype(v)>;
                        if constexpr (std::is_same_v<T, std::string>) {
                            return v;
                        } else if constexpr (std::is_same_v<T, int64_t>) {
                            return std::to_string(v);
                        } else if constexpr (std::is_same_v<T, double>) {
                            std::ostringstream oss;
                            oss << std::noshowpoint << v;
                            return oss.str();
                        } else if constexpr (std::is_same_v<T, bool>) {
                            return v ? "true" : "false";
                        } else {
                            return "undefined";
                        }
                    },
                    elem));
            }
            SCE_LOG_DEBUG("resultToStringArray: Extracted {} elements directly from ScriptArray", arrayValues.size());
            return arrayValues;
        }
    }

    if (std::holds_alternative<std::string>(value)) {
        arrayStr = std::get<std::string>(value);
        SCE_LOG_DEBUG("resultToStringArray: Got string result: '{}'", arrayStr);
    } else {
        SCE_LOG_DEBUG("resultToStringArray: Result is not string type, attempting JSON.stringify conversion");
        if (engine && !sessionId.empty() && !originalExpression.text().empty()) {
            const ScriptSource stringifyExpr = ScriptDialect::stringify(originalExpression);
            SCE_LOG_DEBUG("resultToStringArray: Evaluating stringify expression: '{}'", stringifyExpr.source());
            auto stringifyResult = engine->evaluateExpression(sessionId, stringifyExpr).get();
            if (stringifyResult.isSuccess() &&
                std::holds_alternative<std::string>(stringifyResult.getInternalValue())) {
                arrayStr = std::get<std::string>(stringifyResult.getInternalValue());
                SCE_LOG_DEBUG("resultToStringArray: JSON.stringify succeeded, result: '{}'", arrayStr);
            } else {
                SCE_LOG_DEBUG("resultToStringArray: JSON.stringify failed or returned non-string");
                return arrayValues;
            }
        } else {
            SCE_LOG_DEBUG("resultToStringArray: Missing engine, sessionId or originalExpression for non-string type");
            return arrayValues;
        }
    }

    SCE_LOG_DEBUG("resultToStringArray: Final arrayStr before processing: '{}'", arrayStr);

    if (!arrayStr.empty() && engine && !sessionId.empty()) {
        SCE_LOG_DEBUG("resultToStringArray: Processing array using JSON approach");

        try {
            // §scxml-B-2 (test 457): Validate that value is actually an array
            const ScriptSource arrayCheckExpr = ScriptDialect::isArray(originalExpression);
            SCE_LOG_DEBUG("resultToStringArray: Validating array type with expression: '{}'", arrayCheckExpr.source());
            auto arrayCheckResult = engine->evaluateExpression(sessionId, arrayCheckExpr).get();

            if (!arrayCheckResult.isSuccess() || !std::holds_alternative<bool>(arrayCheckResult.getInternalValue()) ||
                !std::get<bool>(arrayCheckResult.getInternalValue())) {
                SCE_LOG_DEBUG(
                    "resultToStringArray: Value is not an array (instanceof Array check failed), returning empty");
                return arrayValues;
            }

            // W3C SCXML: Use original expression to preserve null/undefined
            // distinction. The bind and the read are two statements now rather
            // than one semicolon-joined string: `var x = e; x.length` is an
            // ECMAScript-only shape, and the bind is the half whose spelling
            // differs (Lua has no `var`).
            const ScriptSource tempName = ScriptDialect::temporary("_tempArray", originalExpression.language());
            const ScriptSource bindExpr = ScriptDialect::bindTemporary("_tempArray", originalExpression);
            SCE_LOG_DEBUG("resultToStringArray: Binding temp array with: '{}'", bindExpr.source());
            (void)engine->executeScript(sessionId, bindExpr).get();
            auto lengthResult = engine->evaluateExpression(sessionId, ScriptDialect::lengthOf(tempName)).get();

            int64_t arrayLength = 0;
            bool lengthValid = false;

            if (lengthResult.isSuccess()) {
                const auto &lengthValue = lengthResult.getInternalValue();
                if (std::holds_alternative<int64_t>(lengthValue)) {
                    arrayLength = std::get<int64_t>(lengthValue);
                    lengthValid = true;
                    SCE_LOG_DEBUG("resultToStringArray: Got int64_t array length: {}", arrayLength);
                } else if (std::holds_alternative<double>(lengthValue)) {
                    double doubleLength = std::get<double>(lengthValue);
                    arrayLength = static_cast<int64_t>(doubleLength);
                    lengthValid = true;
                    SCE_LOG_DEBUG("resultToStringArray: Got double array length: {} -> {}", doubleLength, arrayLength);
                }
            }

            if (lengthValid) {
                for (int64_t i = 0; i < arrayLength; ++i) {
                    // W3C SCXML: Check for undefined first, then use JSON.stringify
                    const ScriptSource element = ScriptDialect::elementAt(tempName, i);
                    auto typeResult = engine->evaluateExpression(sessionId, ScriptDialect::typeOf(element)).get();

                    if (typeResult.isSuccess() && std::holds_alternative<std::string>(typeResult.getInternalValue())) {
                        std::string typeStr = std::get<std::string>(typeResult.getInternalValue());

                        if (typeStr == "undefined") {
                            arrayValues.push_back("undefined");
                            SCE_LOG_DEBUG("resultToStringArray: Element {} is undefined", i);
                            continue;
                        }
                    }

                    const ScriptSource elementExpr = ScriptDialect::stringify(element);
                    SCE_LOG_DEBUG("resultToStringArray: Element {} expression: '{}'", i, elementExpr.source());
                    auto elementResult = engine->evaluateExpression(sessionId, elementExpr).get();

                    if (elementResult.isSuccess() &&
                        std::holds_alternative<std::string>(elementResult.getInternalValue())) {
                        std::string elementStr = std::get<std::string>(elementResult.getInternalValue());
                        SCE_LOG_DEBUG("resultToStringArray: Element {} result: '{}'", i, elementStr);
                        if (elementStr.length() >= 2 && elementStr.front() == '"' && elementStr.back() == '"') {
                            arrayValues.push_back(elementStr.substr(1, elementStr.length() - 2));
                        } else {
                            arrayValues.push_back(elementStr);
                        }
                    }
                }
            } else {
                SCE_LOG_DEBUG("resultToStringArray: Length evaluation failed - success: {}, error: '{}'",
                              lengthResult.isSuccess(),
                              lengthResult.isSuccess() ? "no error" : lengthResult.getErrorMessage());
            }
        } catch (const std::exception &e) {
            SCE_LOG_ERROR("resultToStringArray: Exception during JSON processing: {}", e.what());
        }
    }

    SCE_LOG_DEBUG("resultToStringArray: Returning {} elements", arrayValues.size());
    return arrayValues;
}

std::vector<ScriptValue> resultToScriptValueArray(const ScriptResult &result, IScriptEngine *engine,
                                                  const std::string &sessionId,
                                                  const ScriptSource &originalExpression) {
    // §scxml-4.6: <foreach> array element extraction without string round-trip,
    // preserving type information for objects, arrays, and all primitive types.
    std::vector<ScriptValue> values;

    if (!result.isSuccess()) {
        return values;
    }

    const auto &value = result.getInternalValue();

    // Direct ScriptArray extraction — preserves all types including objects and nested arrays
    if (std::holds_alternative<std::shared_ptr<ScriptArray>>(value)) {
        auto arr = std::get<std::shared_ptr<ScriptArray>>(value);
        if (arr) {
            for (const auto &elem : arr->elements) {
                values.push_back(elem);
            }
            SCE_LOG_DEBUG("resultToScriptValueArray: Extracted {} elements directly from ScriptArray", values.size());
            return values;
        }
    }

    // Fallback: use engine to extract elements by index
    if (engine && !sessionId.empty() && !originalExpression.text().empty()) {
        // Get array length
        auto lengthResult = engine->evaluateExpression(sessionId, ScriptDialect::lengthOf(originalExpression)).get();

        int64_t arrayLength = 0;
        if (lengthResult.isSuccess()) {
            const auto &lv = lengthResult.getInternalValue();
            if (std::holds_alternative<int64_t>(lv)) {
                arrayLength = std::get<int64_t>(lv);
            } else if (std::holds_alternative<double>(lv)) {
                arrayLength = static_cast<int64_t>(std::get<double>(lv));
            }
        }

        for (int64_t i = 0; i < arrayLength; ++i) {
            auto elemResult =
                engine->evaluateExpression(sessionId, ScriptDialect::elementAt(originalExpression, i)).get();
            if (elemResult.isSuccess()) {
                values.push_back(elemResult.getInternalValue());
            } else {
                values.emplace_back(ScriptUndefined{});
            }
        }
        SCE_LOG_DEBUG("resultToScriptValueArray: Extracted {} elements via engine fallback", values.size());
    }

    return values;
}

bool isSuccess(const ScriptResult &result) noexcept {
    return result.isSuccess();
}

void requireSuccess(const ScriptResult &result, const std::string &operation) {
    if (!result.isSuccess()) {
        throw std::runtime_error("Script operation failed: " + operation + " - " + result.getErrorMessage());
    }
}

}  // namespace SCE::ScriptResultUtils
