// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

auto &engineInstance() {
    return SCE::LuaEngine::instance();
}
}  // namespace

#elif defined(SCE_SCRIPT_ENGINE_QUICKJS)

#include "scripting/JSEngine.h"

namespace {
constexpr const char *kEngineName = "QuickJS";
constexpr const char *kEngineId = "quickjs";

auto &engineInstance() {
    return SCE::JSEngine::instance();
}
}  // namespace

#else
#error "SCE_SCRIPT_ENGINE_* not defined. Set SCE_SCRIPT_ENGINE via CMake (quickjs, lua)."
#endif

namespace SCE {

// Handing back a shut-down engine is the same defect as handing back a null
// reference: the caller cannot tell, and the failure surfaces far away. Every
// engine here is a process singleton and `shutdown()` is reachable by any
// holder — the integration fixtures call it from TearDown as a matter of
// course — so "the engine you get is usable" has to be asserted at the
// accessor rather than assumed at each of the dozens of call sites.
//
// The two engines answered this differently until now, and only one of them
// was ever run. LuaEngine tolerates shutdown-then-reuse; JSEngine's
// `shutdown()` joins its worker thread, after which a queued request is never
// serviced and the caller's future waits forever. Measured 2026-08-14 on a
// `-DSCE_SCRIPT_ENGINE=quickjs` build: `DonedataLocalInvokeAotTest` passed
// alone and deadlocked when the interpreter test ran first.
//
// `initialize()` is the interface's own name for this, and on a live engine
// it is an atomic load — the "zero overhead" the class documents is about not
// dispatching through a factory, not about skipping a lifecycle check.
IScriptEngine &ScriptEngineProvider::getScriptEngine() {
    auto &engine = engineInstance();
    engine.initialize();
    return engine;
}

ISessionManager &ScriptEngineProvider::getSessionManager() {
    // Same instance, same invariant: a session manager whose engine is down
    // cannot create a session either.
    auto &engine = engineInstance();
    engine.initialize();
    return engine;
}

const char *ScriptEngineProvider::getEngineName() {
    return kEngineName;
}

const char *ScriptEngineProvider::getEngineId() {
    return kEngineId;
}

}  // namespace SCE
