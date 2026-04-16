// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Lua — JNI declarations for Lua 5.4 C API

package com.sce.scripting.lua

/**
 * JNI bridge to Lua 5.4 C API.
 *
 * Each lua_State is represented as a Long handle (native pointer).
 * Thread safety: each state must be accessed from a single thread at a time.
 */
object LuaNative {

    init {
        System.loadLibrary("sce_lua_jni")
    }

    // Type constants (initialized from C side)
    @JvmStatic external fun typeNone(): Int
    @JvmStatic external fun typeNil(): Int
    @JvmStatic external fun typeBoolean(): Int
    @JvmStatic external fun typeNumber(): Int
    @JvmStatic external fun typeString(): Int
    @JvmStatic external fun typeTable(): Int
    @JvmStatic external fun typeFunction(): Int

    // State lifecycle
    @JvmStatic external fun newState(): Long
    @JvmStatic external fun closeState(handle: Long)

    // Script execution
    /** Execute Lua code. Returns null on success, error message on failure. */
    @JvmStatic external fun doString(handle: Long, code: String): String?
    /** Load and call Lua code. Returns Lua status code (0 = LUA_OK). */
    @JvmStatic external fun loadAndCall(handle: Long, code: String, nresults: Int): Int

    // Stack operations
    @JvmStatic external fun getTop(handle: Long): Int
    @JvmStatic external fun setTop(handle: Long, index: Int)
    @JvmStatic external fun pop(handle: Long, n: Int)
    @JvmStatic external fun type(handle: Long, index: Int): Int

    // Push operations
    @JvmStatic external fun pushNil(handle: Long)
    @JvmStatic external fun pushBoolean(handle: Long, value: Boolean)
    @JvmStatic external fun pushInteger(handle: Long, value: Long)
    @JvmStatic external fun pushNumber(handle: Long, value: Double)
    @JvmStatic external fun pushString(handle: Long, value: String)

    // Get operations
    @JvmStatic external fun toBoolean(handle: Long, index: Int): Boolean
    @JvmStatic external fun toInteger(handle: Long, index: Int): Long
    @JvmStatic external fun toNumber(handle: Long, index: Int): Double
    @JvmStatic external fun toJString(handle: Long, index: Int): String?
    @JvmStatic external fun isInteger(handle: Long, index: Int): Boolean
    @JvmStatic external fun isNumber(handle: Long, index: Int): Boolean
    @JvmStatic external fun isString(handle: Long, index: Int): Boolean
    @JvmStatic external fun isTable(handle: Long, index: Int): Boolean
    @JvmStatic external fun isNil(handle: Long, index: Int): Boolean
    @JvmStatic external fun isBoolean(handle: Long, index: Int): Boolean
    @JvmStatic external fun isFunction(handle: Long, index: Int): Boolean

    // Table operations
    @JvmStatic external fun createTable(handle: Long, narr: Int, nrec: Int)
    @JvmStatic external fun setTable(handle: Long, index: Int)
    @JvmStatic external fun getTable(handle: Long, index: Int)
    @JvmStatic external fun setField(handle: Long, index: Int, key: String)
    @JvmStatic external fun getField(handle: Long, index: Int, key: String)
    @JvmStatic external fun rawSetI(handle: Long, index: Int, n: Long)
    @JvmStatic external fun rawGetI(handle: Long, index: Int, n: Long)
    @JvmStatic external fun rawLen(handle: Long, index: Int): Long
    @JvmStatic external fun next(handle: Long, index: Int): Int

    // Global variables
    @JvmStatic external fun setGlobal(handle: Long, name: String)
    @JvmStatic external fun getGlobal(handle: Long, name: String): Int

    // Error handling
    @JvmStatic external fun getError(handle: Long): String?

    // GC
    @JvmStatic external fun gc(handle: Long)

    // Registry operations
    @JvmStatic external fun ref(handle: Long, t: Int): Int
    @JvmStatic external fun unref(handle: Long, t: Int, refVal: Int)
    @JvmStatic external fun registryIndex(): Int

    // Metatable operations
    @JvmStatic external fun newMetatable(handle: Long, name: String): Int
    @JvmStatic external fun setMetatable(handle: Long, index: Int)
    /** Returns true if value at index has a metatable (pops the metatable internally). */
    @JvmStatic external fun getMetatable(handle: Long, index: Int): Boolean
}
