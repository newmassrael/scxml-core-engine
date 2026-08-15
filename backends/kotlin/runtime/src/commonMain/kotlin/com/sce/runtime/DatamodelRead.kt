// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * Typed reads of a live datamodel variable.
 *
 * The counterpart to the datamodel initialisation `ensureScriptEngine`
 * performs. These take a value back out in the host's own type, so a generated
 * machine can answer a question about its own datamodel without the caller
 * holding a script engine, a session id and the variable's name spelled as a
 * string.
 *
 * ARCHITECTURE.md: Zero Duplication — the same four readers back the C++
 * `DataModelReadHelper`, the Rust `helpers::datamodel_read`, the Go
 * `ReadDatamodel*` and the Python `datamodel_read` surface, and the C11
 * template inlines the same rules, so every backend's accessor answers alike.
 * Three hand a value back in a host type; the fourth hands a structured one
 * over as JSON, because there is no host type six languages share for it.
 *
 * Why the read goes to the engine rather than to a copy: a `<data>` variable
 * with an initialiser is owned by the script engine for the life of the
 * session — `<assign>` writes there and guards read from there. Anything the
 * generated class kept alongside it would be a second representation of one
 * variable, wrong from the first `<assign>` onwards.
 *
 * Why the answer is nullable: the session may not be initialised yet, the
 * variable may have been assigned a value of another type mid-run, or the
 * engine may refuse. All three mean the same thing to a consumer — the machine
 * cannot answer that right now.
 */
object DatamodelRead {
    private fun current(engine: ScxmlScriptEngine?, sessionId: String?, name: String): Any? {
        if (engine == null || sessionId == null) {
            return null
        }
        return try {
            engine.getVariable(sessionId, name)
        } catch (e: Exception) {
            null
        }
    }

    /**
     * Read an integer-declared datamodel variable.
     *
     * Every whole-valued numeric width is accepted, and that leniency is about
     * engines rather than about types: the three engines Kotlin can be given
     * disagree on how a number comes back (Rhino hands out `Double`, the Lua
     * binding `Long`), and refusing one of them would make the accessor's
     * answer depend on which engine the deployment injected — exactly what a
     * typed accessor exists to hide. A fractional value is a different number
     * and is refused.
     */
    fun readInt(engine: ScxmlScriptEngine?, sessionId: String?, name: String): Long? {
        // §scxml-5.3: the value a `<data>` declaration populated into the
        // session, read back out in the host's own type. Reading, not
        // declaring — the clause's own verb belongs to the generated
        // initialiser.
        return when (val value = current(engine, sessionId, name)) {
            is Long -> value
            is Int -> value.toLong()
            is Short -> value.toLong()
            is Byte -> value.toLong()
            is Double -> if (value == value.toLong().toDouble()) value.toLong() else null
            is Float -> if (value.toDouble() == value.toLong().toDouble()) value.toLong() else null
            else -> null
        }
    }

    /**
     * Read a string-declared datamodel variable.
     *
     * Strict: a number that happens to print as text is not a string, and
     * coercing it would let a consumer read a value the datamodel never held.
     */
    fun readString(engine: ScxmlScriptEngine?, sessionId: String?, name: String): String? {
        // §scxml-5.3: the value a `<data>` declaration populated into the
        // session, read back out in the host's own type.
        return current(engine, sessionId, name) as? String
    }

    /**
     * Read a boolean-declared datamodel variable.
     *
     * Strict, and deliberately not the SCXML truthiness rule: that rule answers
     * a question every value has an answer to. This one answers whether the
     * variable is holding a boolean, and a consumer inspecting a declared flag
     * wants to be told when it is not.
     */
    fun readBool(engine: ScxmlScriptEngine?, sessionId: String?, name: String): Boolean? {
        // §scxml-5.3: the value a `<data>` declaration populated into the
        // session, read back out in the host's own type.
        return current(engine, sessionId, name) as? Boolean
    }

    /**
     * Read an array- or object-declared datamodel variable, as JSON text.
     *
     * Why the engine serializes it rather than this function: every engine
     * SCE can be given carries `JSON.stringify` — the clause cited in the
     * body is what requires it — and that one serializer is the answer.
     * Reflecting over whatever object the engine
     * handed back would be a second serializer disagreeing with the first,
     * and it would have to be written three times over here — a Rhino
     * `NativeArray`, a QuickJS value and a Lua table are three different
     * shapes for one document. What each engine produces is stable for that
     * engine (the Lua builtin sorts object keys; Rhino and QuickJS emit
     * property order), and stability is what a consumer diffing two reads
     * needs. It is the engine's encoding, not a normal form across engines,
     * which is the same shape of promise [readInt] makes about numeric
     * width.
     *
     * Why this expression survives either engine family: [evaluateExpr] takes
     * the ENGINE's language, not the document's — a Lua-backed session is
     * handed Lua. `JSON.stringify(x)` is spelled the same in both, member
     * access and a call, in a language the datamodel clause requires that
     * exact name to exist in. `DatamodelReadJsonTest` puts it to all three
     * engines.
     *
     * Why the answer is strict: the scalar readers refuse a value of another
     * type and so does this one. A variable declared `[…]` and later assigned
     * `5` answers null, not `"5"`. The test is the first character of the
     * serializer's output, where JSON's grammar puts the type — `[` opens an
     * array and `{` an object, and nothing else stringifies to either.
     */
    fun readJson(engine: ScxmlScriptEngine?, sessionId: String?, name: String): String? {
        // §scxml-5.3: the value a `<data>` declaration populated into the
        // session, handed over in the encoding §scxml-B-2 already requires the
        // engine to produce. `name` reaches here only for a name the
        // classifier confirmed is a bare identifier — see
        // `analyzer::reachable_as_an_expression`.
        if (engine == null || sessionId == null) {
            return null
        }
        val json = try {
            engine.evaluateExpr(sessionId, "JSON.stringify($name)") as? String
        } catch (e: Exception) {
            null
        } ?: return null
        return if (json.startsWith("[") || json.startsWith("{")) json else null
    }
}
