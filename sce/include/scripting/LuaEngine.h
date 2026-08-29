// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IScriptEngine.h"
#include "ISessionManager.h"
#include "LoweringScope.h"
#include <atomic>
#include <climits>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
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
 * ECMAScript expressions from W3C SCXML are lowered to Lua by `sce-build`'s
 * ECMAScript frontend, reached through the session's `LoweringScope`. Text the
 * frontend refuses is REFUSED here too — see `loweredTextOf`.
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

    /// Lua, and — where a frontend is linked — the language that frontend
    /// lowers into it.
    ///
    /// This engine used to own a text-rewriting adapter and so accepted
    /// ECMAScript unconditionally. It no longer has one: what accepts the
    /// second language is `sce-build`'s ECMAScript frontend, linked beside
    /// `lua54` by `sce/CMakeLists.txt`, and Lua is still evaluated as given.
    ScriptLanguage nativeLanguage() const override {
        return ScriptLanguage::Lua;
    }

    /// A build with no frontend is a LUA engine, and says so.
    ///
    /// `SCE_HAS_LOWERING_FFI` is set on the same two lines that link the
    /// frontend, so this cannot claim a lowering route the image does not
    /// carry. The wasm build is the one that reaches neither line, and
    /// answering `true` there would accept every ECMAScript expression in
    /// order to fail it one refusal at a time — a capability announced and
    /// then withdrawn per call. `IScriptEngine`'s three entry points already
    /// refuse a language this rejects, with the language named, which is the
    /// clearer failure and the earlier one.
    bool acceptsLanguage(ScriptLanguage language) const override {
        if (language == ScriptLanguage::Lua) {
            return true;
        }
#ifdef SCE_HAS_LOWERING_FFI
        return language == ScriptLanguage::ECMAScript;
#else
        return false;
#endif
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

        // The global names this session was BORN with — Lua's own standard
        // library plus everything `registerBuiltins` installs.
        //
        // They are the RUNTIME's names, not the author's, and the difference
        // is what makes a later reading of the global table mean something: a
        // name absent from here and present in `_G` was introduced by the
        // document. Without the baseline, telling the frontend what the
        // session holds would also tell it about `string` and `math`, and SCE's
        // ECMAScript datamodel would start answering `string.rep('a', 3)` —
        // Lua's standard library leaking out through the datamodel's door.
        std::unordered_set<std::string> runtimeGlobals;

        // The names already offered to `loweringScope`.
        //
        // The scope counts OFFERS, so an offer it already holds still moves
        // `generation()` and invalidates every lowering cached against it.
        // Offering each name once keeps a re-reading of the global table free
        // for a session whose names have stopped changing, which is every
        // session after its first few macrosteps.
        std::unordered_set<std::string> offeredToScope;

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
            // it. `ScriptExecInfo` carries the same field for the same
            // reason — both lowerings now ask the scope.
            uint64_t scopeGeneration = 0;
        };

        std::unordered_map<std::string, ExprExecInfo> exprExecCache;

        // Script execution fast path: maps the incoming script text directly to
        // a compiled chunk ref, skipping transformer cache lookup and string
        // copy. Language-tagged for the same reason as ExprExecInfo.
        struct ScriptExecInfo {
            int chunkRef;
            ScriptLanguage language = ScriptLanguage::ECMAScript;

            // As `ExprExecInfo::scopeGeneration`. A chunk hoists its own `var`
            // bindings, so its lowering asks the scope only about the names it
            // READS — but that is enough to change: `x = a + 1;` is refused
            // and rewritten while `a` is unknown, and parsed once a `<data
            // id>` has declared it.
            uint64_t scopeGeneration = 0;
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
    // is passed through untouched; text that arrives as ECMAScript is offered
    // to that frontend. Everything the engine does AFTER this — the
    // undeclared-variable check, the `return` wrapping, the chunk cache, the
    // assignment fallback — is common to both paths.
    //
    // NOTHING is returned when the frontend refuses, and refusal is the
    // engine's answer rather than a second translator's cue. It used to be the
    // cue: `EcmaScriptToLuaTransformer` rewrote the text without a parse, and
    // `-7 % 3` answering 2 and `5 ^ 3` answering 125 are what that cost. A
    // caller turns nothing into `error.execution` (§scxml-5.9.1), which is the
    // answer the specification already has for an expression the datamodel
    // cannot evaluate.
    //
    // The scope is an argument rather than engine state because it is the
    // SESSION'S: it says which names the frontend may resolve, and a caller
    // holding the wrong session's would lower an expression this one cannot
    // evaluate.
    std::optional<std::string> loweredTextOf(const ScriptSource &expression, const LoweringScope &scope);
    std::optional<std::string> loweredScriptOf(const ScriptSource &script, const LoweringScope &scope);

    // One name, offered to the frontend once.
    //
    // Every door that puts a name into a session's global namespace comes
    // through here, so "which names does this session hold" has one answer
    // rather than one per door.
    static void offerToScope(LuaSessionContext &session, const std::string &name);

    // Every name the DOCUMENT has put in the global table, offered to the
    // frontend.
    //
    // The engine's own setters know the names they write and say so directly.
    // A `<script>` chunk does not: its assignments happen inside Lua. For a
    // chunk that arrived as ECMAScript the frontend's own parser answers
    // (`declareChunk`), and for one that arrived as LUA there is no ECMAScript
    // parse to ask — so the session's global table is asked instead, which is
    // the authority both doors ultimately write to.
    //
    // Reading it is what keeps the two languages ONE session: the engine
    // accepts both into the same namespace, and a name the Lua door
    // introduced is a name the ECMAScript door must be able to resolve. It
    // could not, until this: `arr = {10, 20, 30}` as Lua left `arr[1]` as
    // ECMAScript unresolvable, and the rewriter hid it by never resolving
    // names at all.
    static void offerDocumentGlobalsToScope(LuaSessionContext &session);

    // The refusal above, as the boundary reports it.
    //
    // One place, because all three entry points refuse for the same reason and
    // a message per site is three places for the reason to drift apart. The
    // GRAMMAR the text was read as is the one thing they do not share, so it
    // is the one thing passed in: a caller knows whether it asked for an
    // expression or for a script, and deriving it from the text would be a
    // guess where the call site holds the fact.
    static ScriptResult refusedToLower(const ScriptSource &source, const char *role);

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

    // Engine state
    std::atomic<bool> initialized_{false};
};

}  // namespace SCE
