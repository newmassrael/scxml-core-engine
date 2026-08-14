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
 * ARCHITECTURE.md: Zero Duplication — the same three coercions back the C++
 * `DataModelReadHelper`, the Rust `helpers::datamodel_read` and the Go
 * `ReadDatamodel*` surface, so every backend's accessor answers alike.
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
}
