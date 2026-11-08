// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#pragma once

#include "SCXMLTypes.h"
#include "common/JsonText.h"

#include <cmath>
#include <cstdint>
#include <map>
#include <memory>
#include <string>
#include <type_traits>
#include <utility>
#include <variant>

/**
 * @brief The one JSON text of a script value: what a value that leaves the data model is
 * written as
 *
 * The compact form every engine writes (ARCHITECTURE.md, "JSON Text", "JSON Number Text"
 * and "JSON Object Key Order"): `null` for an absent value, a string escaped by
 * `JsonText`, a whole number as its digits, a float as ECMAScript spells it and `null` for
 * one that is not finite, an array in its order, and the members of an object ascending by
 * their keys' bytes at every depth.
 *
 * Header-only, because the text form of a value is asked for where no SCE library is
 * linked: a generated machine under `datamodel="sce-static"` gives a `<param>` its text by
 * `ScriptResultUtils::valueText`. The event JSON a `<send>` carries
 * (`EventDataHelper::buildEventDataJson`) is written by the same functions, so the text a
 * form parameter carries and the value the event carries are one spelling.
 */
namespace SCE::ScriptValueJson {

/**
 * @brief Append the object whose members are `members`, each a key with the JSON text of its
 * value, ascending by the keys' bytes (a `std::map` of `std::string` iterates in that order)
 */
inline void appendObject(std::string &out, const std::map<std::string, std::string> &members) {
    out += '{';
    bool first = true;
    for (const auto &[key, value] : members) {
        if (!first) {
            out += ',';
        }
        first = false;
        out += '"';
        JsonText::appendEscaped(out, key);
        out += "\":";
        out += value;
    }
    out += '}';
}

/** @brief The object `appendObject` writes, as its own text */
inline std::string object(const std::map<std::string, std::string> &members) {
    std::string out;
    appendObject(out, members);
    return out;
}

/** @brief Append the JSON text of `value` */
inline void append(std::string &out, const ScriptValue &value) {
    std::visit(
        [&out](const auto &held) {
            using T = std::decay_t<decltype(held)>;
            if constexpr (std::is_same_v<T, ScriptNull> || std::is_same_v<T, ScriptUndefined>) {
                // JSON has no `undefined`; an absent value is `null`.
                out += "null";
            } else if constexpr (std::is_same_v<T, bool>) {
                out += held ? "true" : "false";
            } else if constexpr (std::is_same_v<T, int64_t>) {
                out += std::to_string(held);
            } else if constexpr (std::is_same_v<T, double>) {
                // JSON has no spelling for a float that is not finite.
                out += std::isfinite(held) ? JsonText::numberText(held) : "null";
            } else if constexpr (std::is_same_v<T, std::string>) {
                out += '"';
                JsonText::appendEscaped(out, held);
                out += '"';
            } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptArray>>) {
                out += '[';
                if (held) {
                    bool first = true;
                    for (const auto &element : held->elements) {
                        if (!first) {
                            out += ',';
                        }
                        first = false;
                        append(out, element);
                    }
                }
                out += ']';
            } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptObject>>) {
                // The properties are held unordered; the text orders them.
                std::map<std::string, std::string> members;
                if (held) {
                    for (const auto &[key, property] : held->properties) {
                        std::string written;
                        append(written, property);
                        members.emplace(key, std::move(written));
                    }
                }
                appendObject(out, members);
            }
        },
        value);
}

/** @brief The JSON text of `value` */
inline std::string text(const ScriptValue &value) {
    std::string out;
    append(out, value);
    return out;
}

}  // namespace SCE::ScriptValueJson
