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

ScriptValue
EventDataHelper::buildScriptValueFromParams(const std::map<std::string, std::vector<ScriptValue>> &typedParams) {
    auto obj = std::make_shared<ScriptObject>();
    for (const auto &[name, values] : typedParams) {
        if (values.empty()) {
            continue;
        }
        // A name that was sent twice is an Array of its values in document
        // order; one that was sent once is the value itself. Wrapping the
        // single case would change what every existing document reads.
        if (values.size() == 1) {
            obj->properties[name] = values.front();
            continue;
        }
        auto arr = std::make_shared<ScriptArray>();
        for (const auto &value : values) {
            arr->elements.push_back(value);
        }
        obj->properties[name] = arr;
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

std::string
EventDataHelper::buildJsonFromTypedParams(const std::map<std::string, std::vector<ScriptValue>> &typedParams) {
    json eventDataJson = json::object();
    for (const auto &[name, values] : typedParams) {
        if (values.empty()) {
            continue;
        }
        if (values.size() == 1) {
            eventDataJson[name] = scriptValueToJson(values.front());
            continue;
        }
        json arr = json::array();
        for (const auto &value : values) {
            arr.push_back(scriptValueToJson(value));
        }
        eventDataJson[name] = arr;
    }
    return JsonUtils::toCompactString(eventDataJson);
}

// Merged rather than "typed wins": the two maps do not carry the same shape.
// Both hold a VECTOR per name now — a document may write `<param name="x">`
// more than once and W3C test178 requires every value to arrive — but only
// `stringParams` is filled on the paths that never reach a script engine, so
// choosing either wholesale loses something: the string map turns `expr="42"`
// into `"42"`, which an ECMAScript receiver compares against 42 and finds
// unequal (measured 2026-08-14: the autoforward fixture's
// `_event.data.value === 42` read false end to end), and the typed map is
// simply absent for a literal-only send.
static json mergeParams(const std::map<std::string, std::vector<std::string>> &stringParams,
                        const std::map<std::string, std::vector<ScriptValue>> &typedParams) {
    // One occurrence is the value; more than one is an Array of them in
    // document order. W3C test 178 sends a name twice and requires both pairs
    // delivered, and an object cannot hold one name twice.
    const auto publish = [](const std::vector<ScriptValue> &values) {
        if (values.size() == 1) {
            return scriptValueToJson(values.front());
        }
        json arr = json::array();
        for (const auto &value : values) {
            arr.push_back(scriptValueToJson(value));
        }
        return arr;
    };

    json eventDataJson = json::object();
    for (const auto &[name, values] : stringParams) {
        if (values.empty()) {
            continue;
        }
        // The typed values are preferred whenever there is one per occurrence.
        // This arm used to be reachable only for a single param, so a repeated
        // name published the STRING vector while the typed values sat unused
        // in the map beside it — `<param expr="1"/>` twice arrived as
        // `["1","1"]` and `=== 1` read false.
        auto typed = typedParams.find(name);
        if (typed != typedParams.end() && typed->second.size() == values.size()) {
            eventDataJson[name] = publish(typed->second);
            continue;
        }
        if (values.size() > 1) {
            eventDataJson[name] = values;
            continue;
        }
        eventDataJson[name] = json(values[0]);
    }

    // A name that produced a typed value without a stringified one still
    // belongs in the payload; dropping it would make the presence of a sibling
    // param decide whether this one is delivered.
    for (const auto &[name, values] : typedParams) {
        if (stringParams.find(name) == stringParams.end() && !values.empty()) {
            eventDataJson[name] = publish(values);
        }
    }
    return eventDataJson;
}

std::string EventDataHelper::buildEventDataJson(const std::map<std::string, std::vector<std::string>> &stringParams,
                                                const std::map<std::string, std::vector<ScriptValue>> &typedParams) {
    return JsonUtils::toCompactString(mergeParams(stringParams, typedParams));
}

std::string EventDataHelper::buildEventDataJson(const std::string &data,
                                                const std::map<std::string, std::vector<std::string>> &stringParams,
                                                const std::map<std::string, std::vector<ScriptValue>> &typedParams) {
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
