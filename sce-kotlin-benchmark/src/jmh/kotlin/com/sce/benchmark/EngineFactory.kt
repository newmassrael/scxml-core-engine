// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// Benchmark engine factory — creates ScxmlScriptEngine instances by type name

package com.sce.benchmark

import com.sce.runtime.ScxmlScriptEngine
import com.sce.scripting.RhinoScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import com.sce.scripting.quickjs.QuickJSScriptEngine

object EngineFactory {

    fun create(type: String): ScxmlScriptEngine = when (type) {
        "rhino" -> RhinoScriptEngine()
        "lua" -> LuaScriptEngine()
        "quickjs" -> QuickJSScriptEngine()
        else -> throw IllegalArgumentException("Unknown engine type: $type")
    }
}
