// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "common/EventDataHelper.h"
#include "runtime/JsonUtils.h"
#include <nlohmann/json.hpp>

using json = nlohmann::json;

namespace SCE {

std::string EventDataHelper::buildJsonFromParams(const std::map<std::string, std::vector<std::string>> &params) {
    // §scxml-5.10: Build structured JSON object from params
    json eventDataJson = json::object();

    // Add parameters (W3C SCXML: Support duplicate param names - Test 178)
    for (const auto &param : params) {
        if (param.second.size() == 1) {
            // Single value: store as string
            eventDataJson[param.first] = param.second[0];
        } else if (param.second.size() > 1) {
            // Multiple values: store as array (duplicate param names)
            eventDataJson[param.first] = param.second;
        }
        // Empty vector: skip (should not happen in normal operation)
    }

    return JsonUtils::toCompactString(eventDataJson);
}

ScriptValue EventDataHelper::buildScriptValueFromParams(const std::map<std::string, ScriptValue> &typedParams) {
    auto obj = std::make_shared<ScriptObject>();
    for (const auto &[name, value] : typedParams) {
        obj->properties[name] = value;
    }
    return obj;
}

// §scxml-B-2: Recursive ScriptValue → JSON conversion (inverse of jsonToScriptValue)
// Note: ScriptUndefined → null (JSON lacks undefined); roundtrip yields ScriptNull
static json scriptValueToJson(const ScriptValue &value) {
    return std::visit(
        [](auto &&val) -> json {
            using T = std::decay_t<decltype(val)>;
            if constexpr (std::is_same_v<T, ScriptNull> || std::is_same_v<T, ScriptUndefined>) {
                return nullptr;
            } else if constexpr (std::is_same_v<T, bool>) {
                return val;
            } else if constexpr (std::is_same_v<T, int64_t>) {
                return val;
            } else if constexpr (std::is_same_v<T, double>) {
                return val;
            } else if constexpr (std::is_same_v<T, std::string>) {
                return val;
            } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptArray>>) {
                json arr = json::array();
                if (val) {
                    for (const auto &elem : val->elements) {
                        arr.push_back(scriptValueToJson(elem));
                    }
                }
                return arr;
            } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptObject>>) {
                json obj = json::object();
                if (val) {
                    for (const auto &[key, prop] : val->properties) {
                        obj[key] = scriptValueToJson(prop);
                    }
                }
                return obj;
            }
            return nullptr;
        },
        value);
}

std::string EventDataHelper::buildJsonFromTypedParams(const std::map<std::string, ScriptValue> &typedParams) {
    json eventDataJson = json::object();
    for (const auto &[name, value] : typedParams) {
        eventDataJson[name] = scriptValueToJson(value);
    }
    return JsonUtils::toCompactString(eventDataJson);
}

// Merged rather than "typed wins": the two maps do not carry the same shape.
// `stringParams` holds a VECTOR per name, because a document may write
// `<param name="x">` more than once and W3C test 178 requires every value to
// arrive. `typedParams` holds ONE value per name, so choosing it wholesale
// publishes the last `<param>` and silently drops the earlier ones — while
// choosing the string map wholesale turns `expr="42"` into `"42"`, which is
// what an ECMAScript receiver compares against 42 and finds unequal (measured
// 2026-08-14: the autoforward fixture's `_event.data.value === 42` read false
// end to end).
static json mergeParams(const std::map<std::string, std::vector<std::string>> &stringParams,
                        const std::map<std::string, ScriptValue> &typedParams) {
    json eventDataJson = json::object();
    for (const auto &[name, values] : stringParams) {
        if (values.empty()) {
            continue;
        }
        if (values.size() > 1) {
            eventDataJson[name] = values;
            continue;
        }
        auto typed = typedParams.find(name);
        eventDataJson[name] = typed != typedParams.end() ? scriptValueToJson(typed->second) : json(values[0]);
    }

    // A name that produced a typed value without a stringified one still
    // belongs in the payload; dropping it would make the presence of a sibling
    // param decide whether this one is delivered.
    for (const auto &[name, value] : typedParams) {
        if (stringParams.find(name) == stringParams.end()) {
            eventDataJson[name] = scriptValueToJson(value);
        }
    }
    return eventDataJson;
}

std::string EventDataHelper::buildEventDataJson(const std::map<std::string, std::vector<std::string>> &stringParams,
                                                const std::map<std::string, ScriptValue> &typedParams) {
    return JsonUtils::toCompactString(mergeParams(stringParams, typedParams));
}

std::string EventDataHelper::buildEventDataJson(const std::string &data,
                                                const std::map<std::string, std::vector<std::string>> &stringParams,
                                                const std::map<std::string, ScriptValue> &typedParams) {
    // Composed here rather than by re-parsing the params JSON at the call
    // site: `json::parse` throws, and the send path that needs this runs
    // inside an event target whose caller does not expect an exception.
    json eventDataJson = mergeParams(stringParams, typedParams);
    eventDataJson["data"] = data;
    return JsonUtils::toCompactString(eventDataJson);
}

std::string EventDataHelper::scriptValueToJsonString(const ScriptValue &value) {
    // §scxml-B-2-9: when data has to leave the ECMAScript data model — as it does for
    // the BasicHTTP Event I/O Processor — it is serialized to JSON, which reconstructs
    // the value in full rather than falling back to a lossy platform format.
    return JsonUtils::toCompactString(scriptValueToJson(value));
}

// §scxml-B-2: Recursive JSON → ScriptValue conversion
static ScriptValue jsonToScriptValue(const json &j) {
    if (j.is_null()) {
        return ScriptNull{};
    } else if (j.is_boolean()) {
        return j.get<bool>();
    } else if (j.is_number_integer()) {
        return j.get<int64_t>();
    } else if (j.is_number_float()) {
        return j.get<double>();
    } else if (j.is_string()) {
        return j.get<std::string>();
    } else if (j.is_array()) {
        auto arr = std::make_shared<ScriptArray>();
        for (const auto &elem : j) {
            arr->elements.push_back(jsonToScriptValue(elem));
        }
        return arr;
    } else if (j.is_object()) {
        auto obj = std::make_shared<ScriptObject>();
        for (auto &[key, val] : j.items()) {
            obj->properties[key] = jsonToScriptValue(val);
        }
        return obj;
    }
    return ScriptUndefined{};
}

std::optional<ScriptValue> EventDataHelper::jsonStringToScriptValue(const std::string &jsonStr) {
    // Skip non-JSON strings early to avoid exception overhead from nlohmann::json::parse
    size_t pos = jsonStr.find_first_not_of(" \t\r\n");
    if (pos == std::string::npos) {
        return std::nullopt;
    }
    char first = jsonStr[pos];
    if (first != '{' && first != '[' && first != '"' && first != 't' && first != 'f' && first != 'n' &&
        !std::isdigit(first) && first != '-') {
        return std::nullopt;
    }

    auto parsed = JsonUtils::parseJson(jsonStr);
    if (!parsed.has_value()) {
        return std::nullopt;
    }
    return jsonToScriptValue(parsed.value());
}

}  // namespace SCE
