// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael/scxml-core-engine
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "scripting/IScriptEngine.h"
#include <cmath>
#include <cstdint>
#include <optional>
#include <string>
#include <variant>

namespace SCE {

/**
 * @brief Typed reads of a live datamodel variable.
 *
 * The counterpart to DataModelInitHelper, which puts a declared <data>
 * variable into the session. This one takes a value back out in the host's
 * own type, so a generated machine can answer a question about its own
 * datamodel without the caller holding a script engine, a session id and the
 * variable's name spelled as a string.
 *
 * ARCHITECTURE.md: Zero Duplication - the same three coercions back the Rust
 * `helpers::datamodel_read`, the Go `runtime.ReadDatamodel*` and the Kotlin
 * `DatamodelRead` surface, so every backend's accessor answers alike.
 *
 * Why the read goes to the engine rather than to a copy: a <data> variable
 * with an initializer is owned by the script engine for the life of the
 * session — <assign> writes there and guards read from there. Anything the
 * generated class kept alongside it would be a second representation of one
 * variable, wrong from the first <assign> onwards.
 *
 * Why the answer is optional: the session may not be initialized yet, the
 * variable may have been assigned a value of another type mid-run, or the
 * engine may refuse. All three mean the same thing to a consumer — the
 * machine cannot answer that right now.
 */
class DataModelReadHelper {
public:
    /**
     * @brief Read an integer-declared datamodel variable.
     *
     * A whole-valued double is accepted as well as an int64_t, and that
     * leniency is about engines rather than about types: Lua 5.2-family
     * bindings have no integer subtype at all, so the same authored 40
     * crosses back as an integer from one engine and a double from another.
     * Refusing the second would make the accessor's answer depend on which
     * engine the deployment injected, which is exactly what a typed accessor
     * exists to hide. A fractional value is a different number and is
     * refused.
     */
    static std::optional<int64_t> readInt(IScriptEngine &engine, const std::string &sessionId,
                                          const std::string &name) {
        // §scxml-5.3: the value a <data> declaration populated into the
        // session, read back out in the host's own type. Reading, not
        // declaring — the clause's own verb belongs to DataModelInitHelper.
        auto result = engine.getVariable(sessionId, name).get();
        if (!result.isSuccess()) {
            return std::nullopt;
        }
        const ScriptValue &value = result.getInternalValue();
        if (std::holds_alternative<int64_t>(value)) {
            return std::get<int64_t>(value);
        }
        if (std::holds_alternative<double>(value)) {
            const double d = std::get<double>(value);
            if (std::isfinite(d) && d == std::floor(d) && d >= static_cast<double>(INT64_MIN) &&
                d <= static_cast<double>(INT64_MAX)) {
                return static_cast<int64_t>(d);
            }
        }
        return std::nullopt;
    }

    /**
     * @brief Read a string-declared datamodel variable.
     *
     * Strict: a number that happens to print as text is not a string, and
     * coercing it would let a consumer read a value the datamodel never held.
     */
    static std::optional<std::string> readString(IScriptEngine &engine, const std::string &sessionId,
                                                 const std::string &name) {
        // §scxml-5.3: the value a <data> declaration populated into the
        // session, read back out in the host's own type.
        auto result = engine.getVariable(sessionId, name).get();
        if (!result.isSuccess()) {
            return std::nullopt;
        }
        const ScriptValue &value = result.getInternalValue();
        if (std::holds_alternative<std::string>(value)) {
            return std::get<std::string>(value);
        }
        return std::nullopt;
    }

    /**
     * @brief Read a boolean-declared datamodel variable.
     *
     * Strict, and deliberately not the SCXML truthiness rule: that rule
     * answers a question every value has an answer to. This one answers
     * whether the variable is holding a boolean, and a consumer inspecting a
     * declared flag wants to be told when it is not.
     */
    static std::optional<bool> readBool(IScriptEngine &engine, const std::string &sessionId, const std::string &name) {
        // §scxml-5.3: the value a <data> declaration populated into the
        // session, read back out in the host's own type.
        auto result = engine.getVariable(sessionId, name).get();
        if (!result.isSuccess()) {
            return std::nullopt;
        }
        const ScriptValue &value = result.getInternalValue();
        if (std::holds_alternative<bool>(value)) {
            return std::get<bool>(value);
        }
        return std::nullopt;
    }
};

}  // namespace SCE
