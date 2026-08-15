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
 * ARCHITECTURE.md: Zero Duplication - the same four readers back the Rust
 * `helpers::datamodel_read`, the Go `runtime.ReadDatamodel*`, the Kotlin
 * `DatamodelRead` and the Python `datamodel_read` surface, and the C11
 * template inlines the same rules, so every backend's accessor answers alike.
 * Three hand a value back in a host type; the fourth hands a structured one
 * over as JSON, because there is no host type six languages share for it.
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

    /**
     * @brief Read an array- or object-declared datamodel variable, as JSON
     * text.
     *
     * Why the engine serializes it rather than this helper: every engine SCE
     * can be given carries JSON.stringify — the clause cited in the body is
     * what requires it — and that one serializer is the answer. Walking the
     * ScriptValue tree here would be a
     * second serializer disagreeing with the first, and it would not even
     * agree with itself — the object arrives as an unordered map, so two
     * reads of an unchanged variable could hand a consumer the keys in
     * different orders. What the engine produces is stable for that engine
     * (the Lua family's shared builtin sorts object keys; an ECMAScript
     * engine emits property order), and stability is what a consumer diffing
     * two reads needs. It is the engine's encoding, not a normal form across
     * engines, which is the same shape of promise readInt makes about
     * numeric width. It is also the only implementation six backends can
     * share: C11 has no allocator to build a JSON tree in, and there is no
     * JSON writer the six languages agree through.
     *
     * Why this expression survives either engine family: evaluateExpression
     * takes the ENGINE's language, not the document's — a Lua-backed session
     * is handed Lua. `JSON.stringify(x)` is spelled the same in both, member
     * access and a call, in a language the datamodel clause requires that
     * exact name to exist in.
     *
     * Why the answer is strict: the scalar readers refuse a value of another
     * type and so does this one. A variable declared [...] and later assigned
     * 5 answers nullopt, not "5". The test is the first character of the
     * serializer's output, where JSON's grammar puts the type — [ opens an
     * array and { an object, and nothing else stringifies to either.
     *
     * ARCHITECTURE.md: Zero Duplication - the Rust `read_json`, the Kotlin
     * `readJson`, the Go `ReadDatamodelJson`, the Python `read_json` and the
     * C11 reader inlined in its template all draw the same two lines.
     */
    static std::optional<std::string> readJson(IScriptEngine &engine, const std::string &sessionId,
                                               const std::string &name) {
        // §scxml-5.3: the value a <data> declaration populated into the
        // session, handed over in the encoding §scxml-B-2 already requires
        // the engine to produce. `name` reaches here only for a name the
        // classifier confirmed is a bare identifier — see
        // `analyzer::reachable_as_an_expression`.
        auto result = engine.evaluateExpression(sessionId, "JSON.stringify(" + name + ")").get();
        if (!result.isSuccess()) {
            return std::nullopt;
        }
        const ScriptValue &value = result.getInternalValue();
        if (!std::holds_alternative<std::string>(value)) {
            return std::nullopt;
        }
        const std::string &json = std::get<std::string>(value);
        if (json.empty() || (json.front() != '[' && json.front() != '{')) {
            return std::nullopt;
        }
        return json;
    }
};

}  // namespace SCE
