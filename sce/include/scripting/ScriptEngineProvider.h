#pragma once

#include "IScriptEngine.h"
#include "ISessionManager.h"
#include <functional>
#include <mutex>

namespace SCE {

/**
 * @brief Configurable provider for script engine instances
 *
 * Replaces direct JSEngine::instance() calls with a configurable provider pattern.
 * Default: JSEngine singleton (backward-compatible).
 * Can be reconfigured to provide LuaEngine or any IScriptEngine implementation.
 *
 * Thread-safe: All methods are protected by mutex.
 */
class ScriptEngineProvider {
public:
    using EngineFactory = std::function<IScriptEngine &()>;
    using SessionManagerFactory = std::function<ISessionManager &()>;

    /**
     * @brief Get the default script engine instance
     * @return Reference to the configured script engine
     */
    static IScriptEngine &getScriptEngine();

    /**
     * @brief Get the default session manager instance
     * @return Reference to the configured session manager
     */
    static ISessionManager &getSessionManager();

    /**
     * @brief Configure the script engine provider
     * @param factory Function that returns a reference to the desired IScriptEngine
     *
     * Must be called before any getScriptEngine() calls, typically at application startup.
     * Not intended for runtime switching between engines.
     */
    static void setScriptEngineFactory(EngineFactory factory);

    /**
     * @brief Configure the session manager provider
     * @param factory Function that returns a reference to the desired ISessionManager
     */
    static void setSessionManagerFactory(SessionManagerFactory factory);

    /**
     * @brief Reset to default (JSEngine::instance()) — primarily for testing
     */
    static void resetToDefault();

private:
    static EngineFactory &scriptEngineFactory();
    static SessionManagerFactory &sessionManagerFactory();
    static std::mutex &mutex();
};

}  // namespace SCE
