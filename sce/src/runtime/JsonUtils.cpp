// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/JsonUtils.h"
#include "common/JsonText.h"
#include "core/LogMacros.h"
#include <chrono>
#include <cmath>

namespace SCE {

namespace {

// The compact form every engine writes (ARCHITECTURE.md, "JSON Text", "JSON
// Number Text" and "JSON Object Key Order"): a string escaped by `JsonText`, a
// float as ECMAScript spells it, the members of an object in byte order (`json`'s
// object is a `std::map`, which iterates in that order). nlohmann's own `dump`
// is the same text for everything but a float, which it writes as `1e-05` and
// `5.0`.
void appendCompact(std::string &out, const json &value) {
    switch (value.type()) {
    case json::value_t::null:
        out += "null";
        break;
    case json::value_t::boolean:
        out += value.get<bool>() ? "true" : "false";
        break;
    case json::value_t::number_integer:
        out += std::to_string(value.get<json::number_integer_t>());
        break;
    case json::value_t::number_unsigned:
        out += std::to_string(value.get<json::number_unsigned_t>());
        break;
    case json::value_t::number_float: {
        // JSON has no spelling for a float that is not finite.
        const double number = value.get<double>();
        out += std::isfinite(number) ? JsonText::numberText(number) : "null";
        break;
    }
    case json::value_t::string:
        out += '"';
        JsonText::appendEscaped(out, value.get_ref<const std::string &>());
        out += '"';
        break;
    case json::value_t::array: {
        out += '[';
        bool first = true;
        for (const auto &item : value) {
            if (!first) {
                out += ',';
            }
            first = false;
            appendCompact(out, item);
        }
        out += ']';
        break;
    }
    case json::value_t::object: {
        out += '{';
        bool first = true;
        for (const auto &member : value.items()) {
            if (!first) {
                out += ',';
            }
            first = false;
            out += '"';
            JsonText::appendEscaped(out, member.key());
            out += "\":";
            appendCompact(out, member.value());
        }
        out += '}';
        break;
    }
    default:
        // A binary value or a discarded one: nothing here builds either, and
        // nlohmann's own text is as good an answer as any.
        out += value.dump();
        break;
    }
}

}  // namespace

std::optional<json> JsonUtils::parseJson(const std::string &jsonString, std::string *errorOut) {
    if (jsonString.empty()) {
        if (errorOut) {
            *errorOut = "Empty JSON string";
        }
        return std::nullopt;
    }

    try {
        return json::parse(jsonString);
    } catch (const json::parse_error &e) {
        if (errorOut) {
            *errorOut = e.what();
        }
        SCE_LOG_DEBUG("JsonUtils: Failed to parse JSON: {}", e.what());
        return std::nullopt;
    }
}

std::string JsonUtils::toCompactString(const json &value) {
    std::string out;
    appendCompact(out, value);
    return out;
}

std::string JsonUtils::toPrettyString(const json &value) {
    return value.dump(2);  // indent with 2 spaces
}

std::string JsonUtils::getString(const json &object, const std::string &key, const std::string &defaultValue) {
    if (!object.is_object() || !object.contains(key)) {
        return defaultValue;
    }

    const auto &value = object[key];
    if (!value.is_string()) {
        return defaultValue;
    }

    return value.get<std::string>();
}

int JsonUtils::getInt(const json &object, const std::string &key, int defaultValue) {
    if (!object.is_object() || !object.contains(key)) {
        return defaultValue;
    }

    const auto &value = object[key];
    if (!value.is_number_integer()) {
        return defaultValue;
    }

    return value.get<int>();
}

bool JsonUtils::hasKey(const json &object, const std::string &key) {
    return object.is_object() && object.contains(key) && !object[key].is_null();
}

json JsonUtils::createTimestampedObject() {
    json object = json::object();
    object["timestamp"] =
        std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch())
            .count();
    return object;
}

}  // namespace SCE