// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Kotlin Lua — JNI declarations for sce-build's ECMAScript frontend

package com.sce.scripting.lua

/**
 * JNI bridge to `sce-build`'s ECMAScript frontend, the C surface declared in
 * `sce/include/scripting/SceLowering.h`.
 *
 * The other half is `backends/kotlin/lua/src/main/cpp/lowering_jni.cpp`, which
 * is compiled into `sce_lua_jni` — the same shared library [LuaNative] loads —
 * because the frontend is linked beside `lua54` rather than shipped on its own.
 * That is the C++ engine's arrangement too (`cmake/SCEBuildLowering.cmake`
 * builds one staticlib and both backends link it), so the two engines cannot
 * lower against different frontends.
 *
 * Nothing outside [LoweringScope] should call this. The handle discipline —
 * one scope per session, freed exactly once — is that class's, and a second
 * caller holding a raw `Long` is a second place for a double free.
 *
 * ## Refusal is a null
 *
 * Every `lower*` answers `null` when the frontend will not lower the text: it
 * did not parse, or it names something the scope has not been told about. That
 * is a normal answer rather than an error, and it is deliberately distinct from
 * the empty string, which is what an empty script legitimately lowers to.
 */
internal object SceLoweringNative {

    init {
        // The same library [LuaNative] loads. Loading it twice is a no-op, and
        // naming it here rather than depending on `LuaNative` having been
        // touched first keeps the two objects independent: a caller that only
        // wants a lowering should not have to have created a Lua state.
        System.loadLibrary("sce_lua_jni")
    }

    /** Open a scope with nothing declared. Released with [freeScope]. */
    @JvmStatic external fun newScope(): Long

    /** Release a scope. A handle of 0 is accepted. */
    @JvmStatic external fun freeScope(handle: Long)

    /** Declare one name, as a `<data id>` does (W3C SCXML §scxml-5.3). */
    @JvmStatic external fun declare(handle: Long, name: String)

    /**
     * Declare whatever a chunk's top level introduces, as a document-level
     * `<script>` does at load time (W3C SCXML §scxml-5.8).
     */
    @JvmStatic external fun declareChunk(handle: Long, source: String)

    /** The frontend's Lua for a value expression, or null if it refuses. */
    @JvmStatic external fun lowerValue(handle: Long, source: String): String?

    /**
     * The frontend's Lua for a condition — the result is a Lua BOOLEAN.
     *
     * A separate entry point from [lowerValue] rather than a wrapper over it,
     * because §scxml-5.9 truthiness is not Lua's: `0`, `''` and `NaN` are false
     * in ECMAScript and true in Lua, so a guard has to arrive already wrapped
     * in the shared truthiness helper. This is the same `to_lua_guard` the
     * build-time filter applies.
     */
    @JvmStatic external fun lowerCondition(handle: Long, source: String): String?

    /** The frontend's Lua for a statement sequence, or null if it refuses. */
    @JvmStatic external fun lowerScript(handle: Long, source: String): String?

    /**
     * The frontend's Lua for an assignment TARGET, or null if it refuses.
     *
     * No scope: a location names what it WRITES, and a write is how this
     * datamodel's globals come into existence, so the target is not resolved
     * against what the document has already declared.
     */
    @JvmStatic external fun lowerLocation(source: String): String?
}
