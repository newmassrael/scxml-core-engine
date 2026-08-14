// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IScriptEngine.h"
#include "ISessionManager.h"

namespace SCE {

/**
 * @brief Compile-time configured accessor for the build's script engine
 *
 * Engine selection is determined at build time via CMake SCE_SCRIPT_ENGINE option.
 * All engine-specific dispatch is consolidated in ScriptEngineProvider.cpp.
 *
 * Build configuration:
 *   cmake -DSCE_SCRIPT_ENGINE=quickjs  (default; requires SCE_ENABLE_QUICKJS=ON)
 *   cmake -DSCE_SCRIPT_ENGINE=lua      (requires SCE_ENABLE_LUA=ON)
 *
 * The two choices are not interchangeable: they run different languages. A
 * document declaring `datamodel="ecmascript"` is evaluated by QuickJS as
 * ECMAScript and by LuaEngine as Lua-after-rewriting, so which engine a build
 * selects decides what an expression MEANS. That is why the default is
 * QuickJS: measured against `tests/ecmascript/ecma262_semantics.json`, the
 * lua selection answers 26 of 58 ECMA-262 expressions wrong. Selecting it is
 * still allowed and still builds — `ecmascript_semantics_test` is what stops
 * the wrong answers from being silent.
 *
 * Keep this comment and the cache entry in `sce/CMakeLists.txt` in step. They
 * disagreed once, and a reader then concluded an expression had been checked
 * against ECMAScript when it had not.
 *
 * Adding a new engine:
 *   1. Implement IScriptEngine + ISessionManager
 *   2. Add #elif block in ScriptEngineProvider.cpp
 *   3. Add CMake option/definition in sce/CMakeLists.txt
 *
 * Intent:
 *   This class names the build-configured script engine. It is NOT a
 *   dependency-injection default. Call sites must not branch on
 *   "explicit-engine vs. provider-fallback". Helpers that need to honor
 *   build configuration (AOT test harness, build-config-aware tools)
 *   call getScriptEngine() directly. Runtime constructors that expose an
 *   engine parameter to callers fail when the caller omits it rather
 *   than silently substituting this accessor.
 *
 * Zero overhead: no mutex, no std::function, no runtime factory.
 * Each call resolves to a direct singleton reference at compile time.
 */
class ScriptEngineProvider {
public:
    /// Get the compile-time selected script engine instance
    static IScriptEngine &getScriptEngine();

    /// Get the compile-time selected session manager instance
    static ISessionManager &getSessionManager();

    /// Human-readable engine name for display/logging (e.g., "QuickJS", "Lua 5.4")
    static const char *getEngineName();

    /// CMake option identifier for CLI validation (e.g., "quickjs", "lua")
    static const char *getEngineId();
};

}  // namespace SCE
