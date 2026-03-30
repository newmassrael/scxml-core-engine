#pragma once

#include "IScriptEngine.h"
#include "ISessionManager.h"
#include "EcmaScriptToLuaTransformer.h"
#include "runtime/ISessionObserver.h"
#include <atomic>
#include <functional>
#include <memory>
#include <mutex>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <vector>

// Forward declaration for Lua state
struct lua_State;

namespace SCE {

class Event;

/**
 * @brief Lua 5.4 implementation of IScriptEngine and ISessionManager
 *
 * Drop-in replacement for JSEngine using Lua instead of QuickJS.
 * Each session gets an isolated lua_State for full variable isolation.
 * ECMAScript expressions from W3C SCXML are automatically transformed to Lua
 * via EcmaScriptToLuaTransformer.
 *
 * Thread safety: All public methods are thread-safe via mutex protection.
 * Lua states are not shared between sessions.
 */
class LuaEngine : public IScriptEngine, public ISessionManager {
public:
    /**
     * @brief Get the global LuaEngine instance (singleton)
     */
    static LuaEngine &instance();

    ~LuaEngine() override;

    // Non-copyable, non-movable
    LuaEngine(const LuaEngine &) = delete;
    LuaEngine &operator=(const LuaEngine &) = delete;

    // === IScriptEngine ===
    std::future<ScriptResult> executeScript(const std::string &sessionId, const std::string &script) override;
    std::future<ScriptResult> evaluateExpression(const std::string &sessionId, const std::string &expression) override;
    std::future<ScriptResult> validateExpression(const std::string &sessionId, const std::string &expression) override;
    std::future<ScriptResult> setVariable(const std::string &sessionId, const std::string &name,
                                          const ScriptValue &value) override;
    std::future<ScriptResult> getVariable(const std::string &sessionId, const std::string &name) override;
    std::future<ScriptResult> setVariableAsDOM(const std::string &sessionId, const std::string &name,
                                                const std::string &xmlContent) override;
    bool hasVariable(const std::string &sessionId, const std::string &variableName) const override;
    bool isVariablePreInitialized(const std::string &sessionId, const std::string &variableName) const override;
    std::future<ScriptResult> setupSystemVariables(const std::string &sessionId, const std::string &sessionName,
                                                    const std::vector<std::string> &ioProcessors) override;
    std::future<ScriptResult> setCurrentEvent(const std::string &sessionId,
                                               const std::shared_ptr<Event> &event) override;
    std::future<ScriptResult> setCurrentEvent(const std::string &sessionId, const std::string &eventName,
                                               const std::string &eventData, const std::string &eventType,
                                               const std::string &sendId, const std::string &origin,
                                               const std::string &originType, const std::string &invokeId) override;
    bool registerGlobalFunction(const std::string &functionName,
                                std::function<ScriptValue(const std::vector<ScriptValue> &)> callback) override;
    bool bindNativeObject(const std::string &sessionId, const std::string &objectName,
                          const std::vector<std::pair<std::string, NativeMethod>> &methods) override;
    void setStateQueryCallback(StateQueryCallback callback, const std::string &sessionId) override;
    std::string getEngineInfo() const override;
    size_t getMemoryUsage() const override;
    void collectGarbage() override;
    bool initialize() override;
    void shutdown() override;
    bool isInitialized() const override;

    // === ISessionLifecycle (shared by IScriptEngine and ISessionManager) ===
    bool createSession(const std::string &sessionId, const std::string &parentSessionId = "") override;
    bool destroySession(const std::string &sessionId) override;
    bool hasSession(const std::string &sessionId) const override;

    // === ISessionManager ===
    void addObserver(ISessionObserver *observer) override;
    void removeObserver(ISessionObserver *observer) override;
    std::vector<std::string> getActiveSessions() const override;
    std::string getParentSessionId(const std::string &sessionId) const override;

private:
    LuaEngine();

    // Per-session Lua context
    struct LuaSessionContext {
        lua_State *L = nullptr;
        std::string sessionId;
        std::string parentSessionId;
        std::string sessionName;
        bool systemVarsInitialized = false;
        std::unordered_set<std::string> preInitializedVars;
        std::unordered_set<std::string> declaredVars;  // Track all declared variables (Lua nil != undeclared)
    };

    // === Internal implementation ===
    ScriptResult executeScriptInternal(const std::string &sessionId, const std::string &script);
    ScriptResult evaluateExpressionInternal(const std::string &sessionId, const std::string &expression);
    ScriptResult setVariableInternal(const std::string &sessionId, const std::string &name, const ScriptValue &value);
    ScriptResult getVariableInternal(const std::string &sessionId, const std::string &name);

    // Lua state management
    lua_State *createLuaState();
    void registerBuiltins(lua_State *L, const std::string &sessionId);
    void pushScriptValue(lua_State *L, const ScriptValue &value);
    ScriptValue luaToScriptValue(lua_State *L, int index);
    ScriptResult luaResultToScriptResult(lua_State *L, int status);

    // State query callbacks for In() predicate
    std::unordered_map<std::string, StateQueryCallback> stateQueryCallbacks_;

    // Session storage
    mutable std::mutex sessionMutex_;
    std::unordered_map<std::string, std::unique_ptr<LuaSessionContext>> sessions_;

    // Observer pattern
    mutable std::mutex observerMutex_;
    std::vector<ISessionObserver *> observers_;

    // Global functions registered via registerGlobalFunction()
    std::mutex globalFuncMutex_;
    std::unordered_map<std::string, std::function<ScriptValue(const std::vector<ScriptValue> &)>> globalFunctions_;

    // Expression transformer
    EcmaScriptToLuaTransformer transformer_;

    // Engine state
    std::atomic<bool> initialized_{false};
};

}  // namespace SCE
