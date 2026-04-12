// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/ScriptEngineProvider.h"

// ============================================================================
// Compile-time engine selection
//
// To add a new engine:
//   1. Add #elif defined(SCE_SCRIPT_ENGINE_NEWENGINE) block below
//   2. Define engineInstance(), kEngineName, kEngineId
//   3. Update sce/CMakeLists.txt with option/definition/sources
// ============================================================================

#if defined(SCE_SCRIPT_ENGINE_LUA)

#include "scripting/LuaEngine.h"
namespace {
constexpr const char *kEngineName = "Lua 5.4";
constexpr const char *kEngineId = "lua";
auto &engineInstance() { return SCE::LuaEngine::instance(); }
}  // namespace

#elif defined(SCE_SCRIPT_ENGINE_QUICKJS)

#include "scripting/JSEngine.h"
namespace {
constexpr const char *kEngineName = "QuickJS";
constexpr const char *kEngineId = "quickjs";
auto &engineInstance() { return SCE::JSEngine::instance(); }
}  // namespace

#else
#error "SCE_SCRIPT_ENGINE_* not defined. Set SCE_SCRIPT_ENGINE via CMake (quickjs, lua)."
#endif

namespace SCE {

IScriptEngine &ScriptEngineProvider::getScriptEngine() { return engineInstance(); }

ISessionManager &ScriptEngineProvider::getSessionManager() { return engineInstance(); }

const char *ScriptEngineProvider::getEngineName() { return kEngineName; }

const char *ScriptEngineProvider::getEngineId() { return kEngineId; }

}  // namespace SCE
