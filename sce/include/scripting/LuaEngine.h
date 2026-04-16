// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

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
#include <climits>
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

    /**
     * @brief Reset engine state for test isolation
     *
     * Destroys all sessions, clears global functions, state query callbacks,
     * and observers. Re-initializes for fresh use. Mirrors JSEngine::reset().
     */
    void reset() override;

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
        // Bound native method storage for bindNativeObject lifetime management
        std::vector<std::unique_ptr<NativeMethod>> boundMethods;
        // Lua bytecode cache: compiled chunks stored in Lua registry (keyed by source string)
        // Successful compilations store the registry ref; failed compilations store the error message.
        struct ChunkCacheEntry {
            int ref;              // Lua registry ref (>= 0) or CHUNK_COMPILE_FAILED sentinel
            std::string error;    // Lua error message (only set on compilation failure)
        };
        static constexpr int CHUNK_COMPILE_FAILED = INT_MIN;
        std::unordered_map<std::string, ChunkCacheEntry> chunkCache;

        // Expression execution fast path: maps original ECMAScript expression
        // directly to pre-resolved chunk ref, skipping transformer lookup,
        // "return" wrapping attempt, and double cache lookup on repeat calls.
        struct ExprExecInfo {
            int chunkRef;       // Lua registry ref for compiled chunk
            bool returnsValue;  // true: "return expr" form; false: assignment (ScriptUndefined)
        };
        std::unordered_map<std::string, ExprExecInfo> exprExecCache;

        // Script execution fast path: maps original script directly to compiled
        // chunk ref, skipping transformer cache lookup and string copy.
        std::unordered_map<std::string, int> scriptExecCache;
    };

    // === Internal implementation ===
    ScriptResult executeScriptInternal(const std::string &sessionId, const std::string &script);
    ScriptResult evaluateExpressionInternal(const std::string &sessionId, const std::string &expression);
    ScriptResult setVariableInternal(const std::string &sessionId, const std::string &name, const ScriptValue &value);
    ScriptResult getVariableInternal(const std::string &sessionId, const std::string &name);

    // Lua chunk compilation with per-session bytecode caching.
    // On LUA_OK: compiled function is pushed onto the stack.
    // On error: error message is pushed onto the stack.
    int loadCachedChunk(lua_State *L, const std::string &code,
                        std::unordered_map<std::string, LuaSessionContext::ChunkCacheEntry> &cache);

    // Lua state management
    lua_State *createLuaState();
    void registerBuiltins(lua_State *L, const std::string &sessionId);
    static void pushScriptValue(lua_State *L, const ScriptValue &value);
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
