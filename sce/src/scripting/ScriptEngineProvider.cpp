#include "scripting/ScriptEngineProvider.h"
#include "scripting/JSEngine.h"

namespace SCE {

IScriptEngine &ScriptEngineProvider::getScriptEngine() {
    std::lock_guard<std::mutex> lock(mutex());
    auto &factory = scriptEngineFactory();
    if (factory) {
        return factory();
    }
    // Default: JSEngine singleton (backward-compatible)
    return JSEngine::instance();
}

ISessionManager &ScriptEngineProvider::getSessionManager() {
    std::lock_guard<std::mutex> lock(mutex());
    auto &factory = sessionManagerFactory();
    if (factory) {
        return factory();
    }
    // Default: JSEngine singleton (backward-compatible)
    return JSEngine::instance();
}

void ScriptEngineProvider::setScriptEngineFactory(EngineFactory factory) {
    std::lock_guard<std::mutex> lock(mutex());
    scriptEngineFactory() = std::move(factory);
}

void ScriptEngineProvider::setSessionManagerFactory(SessionManagerFactory factory) {
    std::lock_guard<std::mutex> lock(mutex());
    sessionManagerFactory() = std::move(factory);
}

void ScriptEngineProvider::resetToDefault() {
    std::lock_guard<std::mutex> lock(mutex());
    scriptEngineFactory() = nullptr;
    sessionManagerFactory() = nullptr;
}

ScriptEngineProvider::EngineFactory &ScriptEngineProvider::scriptEngineFactory() {
    static EngineFactory factory;
    return factory;
}

ScriptEngineProvider::SessionManagerFactory &ScriptEngineProvider::sessionManagerFactory() {
    static SessionManagerFactory factory;
    return factory;
}

std::mutex &ScriptEngineProvider::mutex() {
    static std::mutex mtx;
    return mtx;
}

}  // namespace SCE
