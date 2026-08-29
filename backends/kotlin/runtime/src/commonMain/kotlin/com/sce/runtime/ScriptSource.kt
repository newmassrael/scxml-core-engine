// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Kotlin Runtime — what a script or expression is written in when it
// crosses into an engine.
//
// The Kotlin sibling of `sce/include/scripting/ScriptSource.h`, and deliberately
// the same shape: `docs/SCE_LUA_TRANSLATION_SEAM.md` argued the seam cannot be a
// one-string signature, C++ landed it on 2026-08-28, and a second spelling of
// the same idea would be two answers to one question.

package com.sce.runtime

/**
 * Which language a string handed to a script engine is written in.
 *
 * The spellings match the `script_engine_language` wire vocabulary
 * (`sce-build/src/manifest.rs`, `SCRIPT_ENGINE_LANGUAGES`) so the manifest a
 * host reads and the tag an engine is handed cannot drift into two names for
 * one answer. [wireName] is that spelling; the enum constants are Kotlin's.
 */
enum class ScriptLanguage(val wireName: String) {
    /**
     * The author's own text, as written in the SCXML document under
     * `datamodel="ecmascript"`. An engine that does not evaluate ECMAScript
     * must lower it (`sce-build`'s ECMAScript frontend) or refuse.
     */
    ECMAScript("ecmascript"),

    /**
     * Lua that `sce-build`'s ECMAScript frontend already produced, via the
     * `to_lua_*` filters. Nothing further is to be rewritten.
     */
    Lua("lua"),
}

/**
 * A script or expression crossing into an engine, with its language.
 *
 * **Two strings, not one.** [text] is what the engine evaluates; [source] is
 * the author's ECMAScript, which is what a diagnostic has to name. They are the
 * same string only when the engine is handed the author's own text.
 *
 * The pairing is not a formatting nicety. This backend's Lua engine runs its
 * undeclared-variable check on the *lowered* text and then builds
 * `ReferenceError: <expr> is not defined` from the *original* — and that
 * message travels out on `_event.data` of `error.execution`, so an entry point
 * that received only lowered Lua would name a language the author never wrote.
 *
 * There is deliberately **no one-argument `lua`**. A caller with no authored
 * ECMAScript to pass must pass the Lua twice, and thereby state that its
 * diagnostics will name Lua.
 *
 * Unlike C++ there is no implicit conversion from `String`: Kotlin has none to
 * give. The `String` overloads on [ScxmlScriptEngine] stay instead, and they
 * mean [ecmascript] — which is what every call site that predates this seam was
 * already saying. A site that SHOULD pass lowered Lua and forgets therefore
 * gets the author's text rewritten by the engine, which is today's behaviour:
 * a missed site stays *diverging* rather than becoming newly wrong, and the
 * ECMA-262 table is what still reports it.
 */
class ScriptSource private constructor(
    /** The language of [text]. */
    val language: ScriptLanguage,
    /** What the engine evaluates. */
    val text: String,
    /**
     * The author's ECMAScript, for every diagnostic and log line that names
     * the expression back to whoever wrote it.
     */
    val source: String,
) {
    companion object {
        /** The author's text, spelled out. */
        fun ecmascript(source: String): ScriptSource =
            ScriptSource(ScriptLanguage.ECMAScript, source, source)

        /**
         * Lua the build-time frontend already produced, paired with the
         * ECMAScript it was lowered from.
         */
        fun lua(lowered: String, source: String): ScriptSource =
            ScriptSource(ScriptLanguage.Lua, lowered, source)
    }

    override fun toString(): String = "ScriptSource(${language.wireName}, ${text})"
}
