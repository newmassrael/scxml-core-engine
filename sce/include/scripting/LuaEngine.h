// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "EcmaScriptToLuaTransformer.h"
#include "IScriptEngine.h"
#include "ISessionManager.h"
#include "LoweringScope.h"
#include <atomic>
#include <climits>
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

    /// Lua, and it owns an adapter for the other one.
    ///
    /// `EcmaScriptToLuaTransformer` is that adapter, which is why this engine
    /// accepts both languages while a QuickJS engine accepts only its own:
    /// ECMAScript is rewritten on the way in, Lua is evaluated as given.
    ScriptLanguage nativeLanguage() const override {
        return ScriptLanguage::Lua;
    }

    bool acceptsLanguage(ScriptLanguage language) const override {
        return language == ScriptLanguage::Lua || language == ScriptLanguage::ECMAScript;
    }

    std::future<ScriptResult> setVariable(const std::string &sessionId, const std::string &name,
                                          const ScriptValue &value) override;
    std::future<ScriptResult> getVariable(const std::string &sessionId, const std::string &name) override;
    std::future<ScriptResult> setVariableAsDOM(const std::string &sessionId, const std::string &name,
                                               const std::string &xmlContent) override;
    bool hasVariable(const std::string &sessionId, const std::string &variableName) const override;
    bool isVariablePreInitialized(const std::string &sessionId, const std::string &variableName) const override;
    std::future<ScriptResult> setupSystemVariables(const std::string &sessionId, const std::string &sessionName,
                                                   const std::vector<IOProcessorDescriptor> &ioProcessors) override;
    std::future<SetCurrentEventResult> setCurrentEvent(const std::string &sessionId,
                                                       const std::shared_ptr<Event> &event) override;
    std::future<SetCurrentEventResult> setCurrentEvent(const std::string &sessionId,
                                                       const SetCurrentEventArgs &args) override;
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

        // The same names, asked of sce-build's ECMAScript frontend rather than
        // of Lua. `declaredVars` answers "may this name be read?"; this
        // answers "may this expression be PARSED?" — the frontend refuses any
        // expression naming something it has not been told about, so the
        // engine's own declarations are what let a lowering happen at all.
        //
        // Per session because a datamodel is: two sessions of one document
        // hold different values under the same names, and a session that
        // borrowed another's names would lower an expression the borrower
        // cannot evaluate.
        LoweringScope loweringScope;
        // Bound native method storage for bindNativeObject lifetime management
        std::vector<std::unique_ptr<NativeMethod>> boundMethods;

        // Lua bytecode cache: compiled chunks stored in Lua registry (keyed by source string)
        // Successful compilations store the registry ref; failed compilations store the error message.
        struct ChunkCacheEntry {
            int ref;            // Lua registry ref (>= 0) or CHUNK_COMPILE_FAILED sentinel
            std::string error;  // Lua error message (only set on compilation failure)
        };

        static constexpr int CHUNK_COMPILE_FAILED = INT_MIN;
        std::unordered_map<std::string, ChunkCacheEntry> chunkCache;

        // Expression execution fast path: maps the incoming expression text
        // directly to a pre-resolved chunk ref, skipping transformer lookup,
        // "return" wrapping attempt, and double cache lookup on repeat calls.
        //
        // The entry records which language the text arrived in, because the
        // key is the INPUT and one string can mean two different chunks:
        // `arr[0]` handed over as ECMAScript is rewritten to `arr[0 + 1]`,
        // while the same string handed over as already-lowered Lua is not.
        // A language mismatch is therefore a miss, not a hit.
        struct ExprExecInfo {
            int chunkRef;       // Lua registry ref for compiled chunk
            bool returnsValue;  // true: "return expr" form; false: assignment (ScriptUndefined)
            ScriptLanguage language = ScriptLanguage::ECMAScript;

            // The scope this text was lowered against. A hit on a stale one is
            // a MISS, because the lowering depended on it: `a && b` is refused
            // by the frontend and rewritten while `a` is unknown, and answered
            // by the frontend once a `<script>` has declared it. Without this,
            // whichever evaluation came first would pin its answer for the
            // life of the session and the later declaration could never reach
            // it. Only the expression cache carries it — `scriptExecCache`
            // holds text `loweredScriptOf` lowers without consulting a scope.
            uint64_t scopeGeneration = 0;
        };

        std::unordered_map<std::string, ExprExecInfo> exprExecCache;

        // Script execution fast path: maps the incoming script text directly to
        // a compiled chunk ref, skipping transformer cache lookup and string
        // copy. Language-tagged for the same reason as ExprExecInfo.
        struct ScriptExecInfo {
            int chunkRef;
            ScriptLanguage language = ScriptLanguage::ECMAScript;
        };

        std::unordered_map<std::string, ScriptExecInfo> scriptExecCache;
    };

    // === IScriptEngine hooks ===
    std::future<ScriptResult> doExecuteScript(const std::string &sessionId, const ScriptSource &script) override;
    std::future<ScriptResult> doEvaluateExpression(const std::string &sessionId,
                                                   const ScriptSource &expression) override;
    std::future<ScriptResult> doValidateExpression(const std::string &sessionId,
                                                   const ScriptSource &expression) override;

    // The seam itself: the one step a pre-lowered call skips.
    //
    // Text that arrives as Lua is already the ECMAScript frontend's output and
    // is passed through untouched; text that arrives as ECMAScript goes through
    // the transformer, this engine's input adapter. Everything the engine does
    // AFTER this — the undeclared-variable check, the `return` wrapping, the
    // chunk cache, the assignment fallback — is common to both paths.
    //
    // The scope is an argument rather than engine state because it is the
    // SESSION'S: it says which names the frontend may resolve, and a caller
    // holding the wrong session's would lower an expression this one cannot
    // evaluate.
    std::string loweredTextOf(const ScriptSource &expression, const LoweringScope &scope);
    std::string loweredScriptOf(const ScriptSource &script);

    // === Internal implementation ===
    ScriptResult executeScriptInternal(const std::string &sessionId, const ScriptSource &script);
    ScriptResult evaluateExpressionInternal(const std::string &sessionId, const ScriptSource &expression);
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

    // Global functions registered via registerGlobalFunction()
    std::mutex globalFuncMutex_;
    std::unordered_map<std::string, std::function<ScriptValue(const std::vector<ScriptValue> &)>> globalFunctions_;

    // Expression transformer
    EcmaScriptToLuaTransformer transformer_;

    // Engine state
    std::atomic<bool> initialized_{false};
};

}  // namespace SCE
