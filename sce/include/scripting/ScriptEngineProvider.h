#pragma once

#include "IScriptEngine.h"
#include "ISessionManager.h"

#if !defined(SCE_SCRIPT_ENGINE_LUA) && !defined(SCE_SCRIPT_ENGINE_QUICKJS)
    #error "SCE_SCRIPT_ENGINE_* not defined. Link sce_runtime or set SCE_SCRIPT_ENGINE via CMake."
#endif

namespace SCE {

/**
 * @brief Compile-time configured provider for script engine instances
 *
 * Engine selection is determined at build time via CMake SCE_SCRIPT_ENGINE option.
 * Default: JSEngine (QuickJS). Alternative: LuaEngine (Lua 5.4).
 *
 * Build configuration:
 *   cmake -DSCE_SCRIPT_ENGINE=quickjs  (default)
 *   cmake -DSCE_SCRIPT_ENGINE=lua      (requires SCE_ENABLE_LUA=ON)
 *
 * Zero overhead: no mutex, no std::function, no runtime factory.
 * Each call resolves to a direct singleton reference at compile time.
 */
class ScriptEngineProvider {
public:
    /**
     * @brief Get the compile-time selected script engine instance
     * @return Reference to JSEngine::instance() or LuaEngine::instance()
     */
    static IScriptEngine &getScriptEngine();

    /**
     * @brief Get the compile-time selected session manager instance
     * @return Reference to the same engine as getScriptEngine() (implements both interfaces)
     */
    static ISessionManager &getSessionManager();

    /**
     * @brief Get the compile-time selected engine name for logging
     * @return "QuickJS" or "Lua 5.4"
     */
    static constexpr const char *getEngineName() {
#ifdef SCE_SCRIPT_ENGINE_LUA
        return "Lua 5.4";
#else
        return "QuickJS";
#endif
    }
};

}  // namespace SCE
