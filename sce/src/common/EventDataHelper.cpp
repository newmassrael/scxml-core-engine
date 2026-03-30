#include "common/EventDataHelper.h"
#include "runtime/JsonUtils.h"
#include <nlohmann/json.hpp>

using json = nlohmann::json;

namespace SCE {

std::string EventDataHelper::buildJsonFromParams(const std::map<std::string, std::vector<std::string>> &params) {
    // W3C SCXML 5.10: Build structured JSON object from params
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

std::string EventDataHelper::buildJsonFromTypedParams(const std::map<std::string, ScriptValue> &typedParams) {
    json eventDataJson = json::object();
    for (const auto &[name, value] : typedParams) {
        std::visit(
            [&eventDataJson, &name](auto &&val) {
                using T = std::decay_t<decltype(val)>;
                if constexpr (std::is_same_v<T, int64_t>) {
                    eventDataJson[name] = val;
                } else if constexpr (std::is_same_v<T, double>) {
                    eventDataJson[name] = val;
                } else if constexpr (std::is_same_v<T, bool>) {
                    eventDataJson[name] = val;
                } else if constexpr (std::is_same_v<T, std::string>) {
                    eventDataJson[name] = val;
                } else if constexpr (std::is_same_v<T, ScriptNull> || std::is_same_v<T, ScriptUndefined>) {
                    eventDataJson[name] = nullptr;
                } else {
                    // ScriptArray/ScriptObject: fall back to string representation
                    eventDataJson[name] = nullptr;
                }
            },
            value);
    }
    return JsonUtils::toCompactString(eventDataJson);
}

// W3C SCXML B.2: Recursive JSON → ScriptValue conversion
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
