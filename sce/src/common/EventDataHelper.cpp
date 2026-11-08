// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "common/EventDataHelper.h"
#include "common/JsonText.h"
#include "common/ScriptValueJson.h"
#include "runtime/JsonUtils.h"
#include <cstdint>
#include <map>
#include <nlohmann/json.hpp>
#include <string>
#include <vector>

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

// Each value a param name holds, as the one JSON text a value has (`ScriptValueJson`):
// one occurrence is the value; more than one is an Array of them in document order.
// W3C test 178 sends a name twice and requires both pairs delivered, and an object
// cannot hold one name twice.
static std::string publishedValues(const std::vector<ScriptValue> &values) {
    if (values.size() == 1) {
        return ScriptValueJson::text(values.front());
    }
    std::string out = "[";
    for (size_t i = 0; i < values.size(); ++i) {
        if (i > 0) {
            out += ',';
        }
        ScriptValueJson::append(out, values[i]);
    }
    out += ']';
    return out;
}

// The text values a param name holds, as JSON strings: one occurrence is the string,
// more than one an Array of them in document order.
static std::string publishedTexts(const std::vector<std::string> &values) {
    if (values.size() == 1) {
        return JsonText::quoted(values.front());
    }
    std::string out = "[";
    for (size_t i = 0; i < values.size(); ++i) {
        if (i > 0) {
            out += ',';
        }
        out += JsonText::quoted(values[i]);
    }
    out += ']';
    return out;
}

std::string
EventDataHelper::buildJsonFromTypedParams(const std::map<std::string, std::vector<ScriptValue>> &typedParams) {
    std::map<std::string, std::string> members;
    for (const auto &[name, values] : typedParams) {
        if (values.empty()) {
            continue;
        }
        members[name] = publishedValues(values);
    }
    return ScriptValueJson::object(members);
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
//
// Each member is the JSON text of its value, keyed by its name, so the object is
// written by the same functions that give a value its text (`ScriptValueJson`), the
// one a `<param>`'s text form is read through too (`ScriptResultUtils::valueText`).
static std::map<std::string, std::string>
mergeParams(const std::map<std::string, std::vector<std::string>> &stringParams,
            const std::map<std::string, std::vector<ScriptValue>> &typedParams) {
    std::map<std::string, std::string> eventDataJson;
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
            eventDataJson[name] = publishedValues(typed->second);
            continue;
        }
        eventDataJson[name] = publishedTexts(values);
    }

    // A name that produced a typed value without a stringified one still
    // belongs in the payload; dropping it would make the presence of a sibling
    // param decide whether this one is delivered.
    for (const auto &[name, values] : typedParams) {
        if (stringParams.find(name) == stringParams.end() && !values.empty()) {
            eventDataJson[name] = publishedValues(values);
        }
    }
    return eventDataJson;
}

std::string EventDataHelper::buildEventDataJson(const std::map<std::string, std::vector<std::string>> &stringParams,
                                                const std::map<std::string, std::vector<ScriptValue>> &typedParams) {
    return ScriptValueJson::object(mergeParams(stringParams, typedParams));
}

std::string EventDataHelper::buildEventDataJson(const std::string &data,
                                                const std::map<std::string, std::vector<std::string>> &stringParams,
                                                const std::map<std::string, std::vector<ScriptValue>> &typedParams) {
    // Composed here rather than by re-parsing the params JSON at the call
    // site: a parse can fail, and the send path that needs this runs inside an
    // event target whose caller does not expect an exception.
    std::map<std::string, std::string> eventDataJson = mergeParams(stringParams, typedParams);
    eventDataJson["data"] = JsonText::quoted(data);
    return ScriptValueJson::object(eventDataJson);
}

std::string EventDataHelper::scriptValueToJsonString(const ScriptValue &value) {
    // §scxml-B-2-9: when data has to leave the ECMAScript data model — as it does for
    // the BasicHTTP Event I/O Processor — it is serialized to JSON, which reconstructs
    // the value in full rather than falling back to a lossy platform format. One
    // writer, the header-only one a `<param>`'s text form is read through too.
    return ScriptValueJson::text(value);
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
