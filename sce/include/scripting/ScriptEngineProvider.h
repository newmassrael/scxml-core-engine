// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IScriptEngine.h"
#include "ISessionManager.h"

namespace SCE {

/**
 * @brief Compile-time configured provider for script engine instances
 *
 * Engine selection is determined at build time via CMake SCE_SCRIPT_ENGINE option.
 * All engine-specific dispatch is consolidated in ScriptEngineProvider.cpp.
 *
 * Build configuration:
 *   cmake -DSCE_SCRIPT_ENGINE=quickjs  (default)
 *   cmake -DSCE_SCRIPT_ENGINE=lua      (requires SCE_ENABLE_LUA=ON)
 *
 * Adding a new engine:
 *   1. Implement IScriptEngine + ISessionManager
 *   2. Add #elif block in ScriptEngineProvider.cpp
 *   3. Add CMake option/definition in sce/CMakeLists.txt
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
