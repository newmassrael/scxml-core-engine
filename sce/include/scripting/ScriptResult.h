// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SCXMLTypes.h"
#include <climits>
#include <cmath>
#include <cstdio>
#include <string>
#include <typeinfo>
#include <variant>

namespace SCE {

// ScriptValue is now defined in SCXMLTypes.h

/**
 * @brief JavaScript execution result
 */
struct ScriptResult {
private:
    // PRIVATE: Legacy API completely blocked - no direct access
    bool success_internal = false;
    ScriptValue value_internal = ScriptUndefined{};
    std::string errorMessage_internal;

public:
    // ONLY ALLOW: Access only through integrated API
    // Constructors only used through static factory methods
    static ScriptResult createSuccess(const ScriptValue &val = ScriptUndefined{}) {
        ScriptResult result;
        result.success_internal = true;
        result.value_internal = val;
        return result;
    }

    static ScriptResult createError(const std::string &error) {
        ScriptResult result;
        result.success_internal = false;
        result.errorMessage_internal = error;
        return result;
    }

    // LEGACY API COMPLETELY BLOCKED: Legacy fields completely removed
    // success, value, errorMessage fields no longer exist

    // ONLY THESE METHODS ALLOWED: Only integrated API can be used
    bool isSuccess() const {
        return success_internal;
    }

    bool isError() const {
        return !success_internal;
    }

    std::string getErrorMessage() const {
        return errorMessage_internal;
    }

    template <typename T> T getValue() const {
        // Debug logging for getValue calls
        std::string type_name = typeid(T).name();
        std::string variant_type = "unknown";
        std::string variant_value = "unknown";

        // Determine current variant type and value
        if (std::holds_alternative<ScriptUndefined>(value_internal)) {
            variant_type = "ScriptUndefined";
            variant_value = "undefined";
        } else if (std::holds_alternative<ScriptNull>(value_internal)) {
            variant_type = "ScriptNull";
            variant_value = "null";
        } else if (std::holds_alternative<bool>(value_internal)) {
            variant_type = "bool";
            variant_value = std::get<bool>(value_internal) ? "true" : "false";
        } else if (std::holds_alternative<int64_t>(value_internal)) {
            variant_type = "int64_t";
            variant_value = std::to_string(std::get<int64_t>(value_internal));
        } else if (std::holds_alternative<double>(value_internal)) {
            variant_type = "double";
            variant_value = std::to_string(std::get<double>(value_internal));
        } else if (std::holds_alternative<std::string>(value_internal)) {
            variant_type = "string";
            variant_value = "\"" + std::get<std::string>(value_internal) + "\"";
        } else if (std::holds_alternative<std::shared_ptr<ScriptArray>>(value_internal)) {
            variant_type = "ScriptArray";
            variant_value = "[array]";
        } else if (std::holds_alternative<std::shared_ptr<ScriptObject>>(value_internal)) {
            variant_type = "ScriptObject";
            variant_value = "[object]";
        }

        // Direct type match - fastest path
        if (std::holds_alternative<T>(value_internal)) {
            return std::get<T>(value_internal);
        }

        // SCXML W3C compliance: Support automatic numeric type conversion
        // JavaScript numbers can be accessed as both double and int64_t
        if constexpr (std::is_same_v<T, double>) {
            // Request double: convert from int64_t if needed
            if (std::holds_alternative<int64_t>(value_internal)) {
                int64_t int_val = std::get<int64_t>(value_internal);
                return static_cast<double>(int_val);
            }
        } else if constexpr (std::is_same_v<T, int64_t>) {
            // Request int64_t: convert from double if it's a whole number
            if (std::holds_alternative<double>(value_internal)) {
                double d = std::get<double>(value_internal);
                const double llong_min_d = static_cast<double>(LLONG_MIN);
                const double llong_max_d = static_cast<double>(LLONG_MAX);
                if (d == floor(d) && d >= llong_min_d && d <= llong_max_d) {
                    return static_cast<int64_t>(d);
                }
            }
        }

        // No conversion possible - return default value
        return T{};
    }

    /**
     * @brief Get value as array (returns nullptr if not an array)
     */
    std::shared_ptr<ScriptArray> getArray() const {
        if (std::holds_alternative<std::shared_ptr<ScriptArray>>(value_internal)) {
            return std::get<std::shared_ptr<ScriptArray>>(value_internal);
        }
        return nullptr;
    }

    /**
     * @brief Get value as object (returns nullptr if not an object)
     */
    std::shared_ptr<ScriptObject> getObject() const {
        if (std::holds_alternative<std::shared_ptr<ScriptObject>>(value_internal)) {
            return std::get<std::shared_ptr<ScriptObject>>(value_internal);
        }
        return nullptr;
    }

    /**
     * @brief Get array element by index
     */
    ScriptValue getArrayElement(size_t index) const {
        auto arr = getArray();
        if (arr && index < arr->elements.size()) {
            return arr->elements[index];
        }
        return ScriptUndefined{};
    }

    /**
     * @brief Get object property by key
     */
    ScriptValue getObjectProperty(const std::string &key) const {
        auto obj = getObject();
        if (obj) {
            auto it = obj->properties.find(key);
            if (it != obj->properties.end()) {
                return it->second;
            }
        }
        return ScriptUndefined{};
    }

    /**
     * @brief Check if value is an array
     */
    bool isArray() const {
        return std::holds_alternative<std::shared_ptr<ScriptArray>>(value_internal);
    }

    /**
     * @brief Check if value is an object
     */
    bool isObject() const {
        return std::holds_alternative<std::shared_ptr<ScriptObject>>(value_internal);
    }

    std::string getValueAsString() const {
        return std::visit(
            [](const auto &v) -> std::string {
                using T = std::decay_t<decltype(v)>;
                if constexpr (std::is_same_v<T, std::string>) {
                    return v;
                } else if constexpr (std::is_same_v<T, bool>) {
                    return v ? "true" : "false";
                } else if constexpr (std::is_same_v<T, int64_t>) {
                    return std::to_string(v);
                } else if constexpr (std::is_same_v<T, double>) {
                    return std::to_string(v);
                } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptArray>>) {
                    std::string result = "[";
                    for (size_t i = 0; i < v->elements.size(); ++i) {
                        if (i > 0) {
                            result += ",";
                        }
                        result += std::visit(
                            [](const auto &elem) -> std::string {
                                using ElemT = std::decay_t<decltype(elem)>;
                                if constexpr (std::is_same_v<ElemT, std::string>) {
                                    return "\"" + elem + "\"";
                                } else if constexpr (std::is_same_v<ElemT, bool>) {
                                    return elem ? "true" : "false";
                                } else if constexpr (std::is_same_v<ElemT, int64_t>) {
                                    return std::to_string(elem);
                                } else if constexpr (std::is_same_v<ElemT, double>) {
                                    return std::to_string(elem);
                                } else {
                                    return "null";
                                }
                            },
                            v->elements[i]);
                    }
                    result += "]";
                    return result;
                } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptObject>>) {
                    return "[object Object]";
                } else {
                    return "undefined";
                }
            },
            value_internal);
    }

    /**
     * @brief ECMAScript truthy conversion (ECMA-262 7.1.2 ToBoolean)
     *
     * Converts the result value to boolean using JavaScript truthiness rules:
     * - bool: direct value
     * - int64_t: true if non-zero
     * - double: true if non-zero
     * - string: true if non-empty
     * - undefined/null/error: false
     *
     * @return false if result is an error or value is falsy
     */
    bool toBool() const {
        if (!success_internal) {
            return false;
        }
        if (std::holds_alternative<bool>(value_internal)) {
            return std::get<bool>(value_internal);
        } else if (std::holds_alternative<int64_t>(value_internal)) {
            return std::get<int64_t>(value_internal) != 0;
        } else if (std::holds_alternative<double>(value_internal)) {
            return std::get<double>(value_internal) != 0.0;
        } else if (std::holds_alternative<std::string>(value_internal)) {
            return !std::get<std::string>(value_internal).empty();
        }
        return false;
    }

    // Internal value accessor for friend classes only
    const ScriptValue &getInternalValue() const {
        return value_internal;
    }

    // Engine-agnostic: No friend class dependency — all engines use public API
    // (createSuccess, createError, isSuccess, getInternalValue, getValue, etc.)
};

}  // namespace SCE
