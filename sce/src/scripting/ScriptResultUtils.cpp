// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/ScriptResultUtils.h"
#include "common/JsonText.h"
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

    // §scxml-B-1 and §scxml-C-1: a scalar is its own text, with no engine.
    // `scalarText` writes it (header-only, so generated code without a script
    // engine reaches it without this library), and this function adds only the
    // engine fallback for what is not a scalar.
    // An integer-valued double used to be cast to `int64_t` here, which is
    // undefined for an infinity and for every finite double above 2^63;
    // `numberText` has no cast.
    if (const auto text = scalarText(value)) {
        return *text;
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
                            return JsonText::numberText(v);
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
