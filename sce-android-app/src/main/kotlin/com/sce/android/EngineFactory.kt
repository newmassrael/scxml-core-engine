// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// SCE Android — Engine factory for all 3 script engine backends

package com.sce.android

import com.sce.runtime.ScxmlScriptEngine
import com.sce.scripting.RhinoScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import com.sce.scripting.quickjs.QuickJSScriptEngine

enum class EngineType(val label: String) {
    RHINO("Rhino"),
    LUA("Lua 5.4"),
    QUICKJS("QuickJS");
}

object EngineFactory {

    fun create(type: EngineType): ScxmlScriptEngine = when (type) {
        EngineType.RHINO -> RhinoScriptEngine()
        EngineType.LUA -> LuaScriptEngine()
        EngineType.QUICKJS -> QuickJSScriptEngine()
    }
}
