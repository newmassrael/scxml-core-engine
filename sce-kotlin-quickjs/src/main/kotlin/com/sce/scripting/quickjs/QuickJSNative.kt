// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin QuickJS — JNI declarations for QuickJS ECMAScript engine

package com.sce.scripting.quickjs

/**
 * JNI bridge to QuickJS ECMAScript engine.
 *
 * Higher-level API than the Lua bridge: values are converted at the JNI boundary
 * rather than exposing raw engine stack/value operations.
 *
 * Each context (JSRuntime + JSContext pair) must be accessed from a single thread.
 */
object QuickJSNative {

    init {
        System.loadLibrary("sce_quickjs_jni")
    }

    // Context lifecycle
    /** Create a new QuickJS runtime + context pair. Returns handle (0 on failure). */
    @JvmStatic external fun createContext(): Long
    /** Destroy a context and free all associated resources. */
    @JvmStatic external fun destroyContext(handle: Long)

    // Script execution

    /** Execute JS code as script. Returns null on success, error message on failure. */
    @JvmStatic external fun eval(handle: Long, code: String): String?

    /**
     * Evaluate JS expression and return typed result string.
     *
     * Protocol: "U"=undefined, "N"=null, "T"=true, "F"=false,
     *           "I<int>", "D<double>", "S<string>", "R<refId>"
     *
     * Returns null on error (call [getLastError] for message).
     */
    @JvmStatic external fun evalExpression(handle: Long, code: String): String?

    /** Evaluate condition. Returns -1=error, 0=false, 1=true. */
    @JvmStatic external fun evalToBoolean(handle: Long, code: String): Int

    // Global variable setters
    @JvmStatic external fun setGlobalString(handle: Long, name: String, value: String)
    @JvmStatic external fun setGlobalInt(handle: Long, name: String, value: Long)
    @JvmStatic external fun setGlobalDouble(handle: Long, name: String, value: Double)
    @JvmStatic external fun setGlobalBoolean(handle: Long, name: String, value: Boolean)
    @JvmStatic external fun setGlobalNull(handle: Long, name: String)
    @JvmStatic external fun setGlobalUndefined(handle: Long, name: String)

    // Error handling

    /** Retrieve and clear last error from evalExpression/evalToBoolean. */
    @JvmStatic external fun getLastError(handle: Long): String?
}
