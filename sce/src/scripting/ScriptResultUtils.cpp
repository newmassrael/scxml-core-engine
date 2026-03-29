#include "scripting/ScriptResultUtils.h"
#include "core/LogMacros.h"
#include "scripting/IScriptEngine.h"
#include <cmath>
#include <sstream>

namespace SCE::ScriptResultUtils {

bool resultToBool(const ScriptResult &result) {
    return result.toBool();
}

std::string resultToString(const ScriptResult &result, IScriptEngine *engine, const std::string &sessionId,
                           const std::string &originalExpression) {
    if (!result.isSuccess()) {
        return "";
    }

    const auto &value = result.getInternalValue();

    if (std::holds_alternative<std::string>(value)) {
        return result.getValue<std::string>();
    } else if (std::holds_alternative<double>(value)) {
        double val = result.getValue<double>();
        if (val == std::floor(val)) {
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
    } else if (engine && !sessionId.empty() && !originalExpression.empty()) {
        // JSON.stringify fallback using provided engine
        std::string stringifyExpr = "JSON.stringify(" + originalExpression + ")";
        auto stringifyResult = engine->evaluateExpression(sessionId, stringifyExpr).get();
        if (stringifyResult.isSuccess()) {
            return stringifyResult.getValue<std::string>();
        }
        return "[object]";
    }
    return "[conversion_error]";
}

std::vector<std::string> resultToStringArray(const ScriptResult &result, IScriptEngine *engine,
                                             const std::string &sessionId, const std::string &originalExpression) {
    std::vector<std::string> arrayValues;

    SCE_LOG_DEBUG("resultToStringArray: Starting with sessionId='{}', originalExpression='{}'", sessionId,
                  originalExpression);

    if (!result.isSuccess()) {
        SCE_LOG_DEBUG("resultToStringArray: Result not successful, returning empty array");
        return arrayValues;
    }

    const auto &value = result.getInternalValue();
    std::string arrayStr;

    if (std::holds_alternative<std::string>(value)) {
        arrayStr = std::get<std::string>(value);
        SCE_LOG_DEBUG("resultToStringArray: Got string result: '{}'", arrayStr);
    } else {
        SCE_LOG_DEBUG("resultToStringArray: Result is not string type, attempting JSON.stringify conversion");
        if (engine && !sessionId.empty() && !originalExpression.empty()) {
            std::string stringifyExpr = "JSON.stringify(" + originalExpression + ")";
            SCE_LOG_DEBUG("resultToStringArray: Evaluating stringify expression: '{}'", stringifyExpr);
            auto stringifyResult = engine->evaluateExpression(sessionId, stringifyExpr).get();
            if (stringifyResult.isSuccess() && std::holds_alternative<std::string>(stringifyResult.getInternalValue())) {
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
            // W3C SCXML B.2 (test 457): Validate that value is actually an array
            std::string arrayCheckExpr = originalExpression + " instanceof Array";
            SCE_LOG_DEBUG("resultToStringArray: Validating array type with expression: '{}'", arrayCheckExpr);
            auto arrayCheckResult = engine->evaluateExpression(sessionId, arrayCheckExpr).get();

            if (!arrayCheckResult.isSuccess() ||
                !std::holds_alternative<bool>(arrayCheckResult.getInternalValue()) ||
                !std::get<bool>(arrayCheckResult.getInternalValue())) {
                SCE_LOG_DEBUG(
                    "resultToStringArray: Value is not an array (instanceof Array check failed), returning empty");
                return arrayValues;
            }

            // W3C SCXML: Use original expression to preserve null/undefined distinction
            std::string setVarExpr = "var _tempArray = " + originalExpression + "; _tempArray.length";
            SCE_LOG_DEBUG("resultToStringArray: Evaluating temp variable length expression: '{}'", setVarExpr);
            auto lengthResult = engine->evaluateExpression(sessionId, setVarExpr).get();

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
                    std::string typeCheckExpr = "typeof _tempArray[" + std::to_string(i) + "]";
                    auto typeResult = engine->evaluateExpression(sessionId, typeCheckExpr).get();

                    if (typeResult.isSuccess() &&
                        std::holds_alternative<std::string>(typeResult.getInternalValue())) {
                        std::string typeStr = std::get<std::string>(typeResult.getInternalValue());

                        if (typeStr == "undefined") {
                            arrayValues.push_back("undefined");
                            SCE_LOG_DEBUG("resultToStringArray: Element {} is undefined", i);
                            continue;
                        }
                    }

                    std::string elementExpr = "JSON.stringify(_tempArray[" + std::to_string(i) + "])";
                    SCE_LOG_DEBUG("resultToStringArray: Element {} expression: '{}'", i, elementExpr);
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

bool isSuccess(const ScriptResult &result) noexcept {
    return result.isSuccess();
}

void requireSuccess(const ScriptResult &result, const std::string &operation) {
    if (!result.isSuccess()) {
        throw std::runtime_error("Script operation failed: " + operation + " - " + result.getErrorMessage());
    }
}

}  // namespace SCE::ScriptResultUtils
