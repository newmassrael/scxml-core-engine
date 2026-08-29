// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/LuaEngine.h"
#include "SCXMLTypes.h"
#include "common/EventDataHelper.h"
#include "core/LogMacros.h"
#include "events/EventRaiserService.h"
#include "runtime/DataContentHelpers.h"
#include "scripting/LuaDOMBinding.h"
#include "scripting/ScriptResult.h"
#include "scripting/SessionRegistry.h"

// sce-build's ECMAScript frontend is reached through `LoweringScope`
// (included by LuaEngine.h), which is where the C surface and its one
// preprocessor branch live. This file therefore has no `#ifdef` for it: a
// build that links no frontend gets a scope that refuses every expression,
// and `acceptsLanguage` declines ECMAScript there rather than accepting it in
// order to refuse it one call at a time.

extern "C" {
#include <lauxlib.h>
#include <lua.h>
#include <lualib.h>
}

#include <cstring>
#include <future>
#include <sstream>

namespace {

// ECMAScript null/undefined sentinel tags for Lua lightuserdata
// Used to preserve null vs undefined distinction in Lua arrays (§scxml-B-2)
char NULL_SENTINEL_TAG;
char UNDEFINED_SENTINEL_TAG;

// Type alias for global function callbacks registered via registerGlobalFunction()
using GlobalFuncCallback = std::function<ScriptValue(const std::vector<ScriptValue> &)>;

// Lua __gc metamethod: destroys GlobalFuncCallback stored as full userdata
int globalFuncGC(lua_State *L) {
    auto *fn = static_cast<GlobalFuncCallback *>(lua_touserdata(L, 1));
    if (fn) {
        fn->~GlobalFuncCallback();
    }
    return 0;
}

// Lua cclosure: invokes a GlobalFuncCallback stored as full userdata upvalue
int globalFuncCall(lua_State *Ls) {
    auto *fn = static_cast<GlobalFuncCallback *>(lua_touserdata(Ls, lua_upvalueindex(1)));
    int nargs = lua_gettop(Ls);
    std::vector<ScriptValue> args;
    for (int i = 1; i <= nargs; ++i) {
        if (lua_isinteger(Ls, i)) {
            args.emplace_back(static_cast<int64_t>(lua_tointeger(Ls, i)));
        } else if (lua_isnumber(Ls, i)) {
            args.emplace_back(lua_tonumber(Ls, i));
        } else if (lua_isstring(Ls, i)) {
            args.emplace_back(std::string(lua_tostring(Ls, i)));
        } else if (lua_isboolean(Ls, i)) {
            args.emplace_back(static_cast<bool>(lua_toboolean(Ls, i)));
        } else {
            args.emplace_back(ScriptUndefined{});
        }
    }
    ScriptValue result = (*fn)(args);
    std::visit(
        [Ls](auto &&val) {
            using T = std::decay_t<decltype(val)>;
            if constexpr (std::is_same_v<T, bool>) {
                lua_pushboolean(Ls, val ? 1 : 0);
            } else if constexpr (std::is_same_v<T, int64_t>) {
                lua_pushinteger(Ls, val);
            } else if constexpr (std::is_same_v<T, double>) {
                lua_pushnumber(Ls, val);
            } else if constexpr (std::is_same_v<T, std::string>) {
                lua_pushstring(Ls, val.c_str());
            } else {
                lua_pushnil(Ls);
            }
        },
        result);
    return 1;
}

// Push a Lua closure that calls a GlobalFuncCallback.
// Uses full userdata with __gc for proper lifetime management (no memory leak).
void pushGlobalFuncClosure(lua_State *L, const GlobalFuncCallback &func) {
    auto *ud = static_cast<GlobalFuncCallback *>(lua_newuserdata(L, sizeof(GlobalFuncCallback)));
    new (ud) GlobalFuncCallback(func);

    if (luaL_newmetatable(L, "SCE.GlobalFunc")) {
        lua_pushcfunction(L, globalFuncGC);
        lua_setfield(L, -2, "__gc");
    }
    lua_setmetatable(L, -2);

    lua_pushcclosure(L, globalFuncCall, 1);
}

}  // anonymous namespace

namespace SCE {

// W3C SCXML: Helper to check if a single identifier is undeclared
static bool isUndeclaredIdentifier(const std::string &name, const std::unordered_set<std::string> &declaredVars,
                                   lua_State *L) {
    // Exclude Lua keywords (true, false, nil, etc.)
    static const std::unordered_set<std::string> luaKeywords = {
        "and", "break", "do",  "else", "elseif", "end",    "false",  "for",  "function", "goto",  "if",
        "in",  "local", "nil", "not",  "or",     "repeat", "return", "then", "true",     "until", "while"};
    if (luaKeywords.count(name)) {
        return false;
    }

    // If declared via setVariable, it's valid (even if nil)
    if (declaredVars.count(name) > 0) {
        return false;
    }

    // Check if it's a Lua standard library global (math, string, table, etc.)
    lua_getglobal(L, name.c_str());
    bool isNil = lua_isnil(L, -1);
    lua_pop(L, 1);

    return isNil;  // Truly undeclared if not a keyword, not declared, and not a Lua global
}

// W3C SCXML: Helper to detect undeclared variable references in Lua expressions.
// JavaScript throws ReferenceError for undeclared variables; Lua silently returns nil.
// Handles both simple identifiers (Var1) and member access (Var1.bar, Var1["key"]).
static bool isUndeclaredSimpleVariable(const std::string &expr, const std::unordered_set<std::string> &declaredVars,
                                       lua_State *L) {
    if (expr.empty()) {
        return false;
    }
    if (!std::isalpha(static_cast<unsigned char>(expr[0])) && expr[0] != '_') {
        return false;
    }

    // Extract base identifier (before first '.' or '[')
    size_t baseEnd = 0;
    while (baseEnd < expr.size() && (std::isalnum(static_cast<unsigned char>(expr[baseEnd])) || expr[baseEnd] == '_')) {
        ++baseEnd;
    }
    if (baseEnd == 0) {
        return false;
    }

    std::string baseName = expr.substr(0, baseEnd);

    // Check if the base identifier is undeclared
    return isUndeclaredIdentifier(baseName, declaredVars, L);
}

// Every string key of the global table, once.
//
// `lua_next` is the only way to enumerate a table and it is fragile in one
// specific way: `lua_tostring` on a NUMBER key rewrites that key in place and
// the traversal then loses its position. So the type is checked before the
// key is read, which also happens to be the filter this wants — a global whose
// name is not a string is not a name any datamodel can spell.
static void forEachGlobalName(lua_State *L, const std::function<void(const char *)> &visit) {
    lua_pushglobaltable(L);
    lua_pushnil(L);
    while (lua_next(L, -2) != 0) {
        if (lua_type(L, -2) == LUA_TSTRING) {
            visit(lua_tostring(L, -2));
        }
        lua_pop(L, 1);  // the value; the key stays for the next step
    }
    lua_pop(L, 1);  // the global table
}

// === Singleton ===

LuaEngine &LuaEngine::instance() {
    static LuaEngine engine;
    return engine;
}

LuaEngine::LuaEngine() {
    initialize();
}

LuaEngine::~LuaEngine() {
    shutdown();
}

// === Engine Lifecycle ===

bool LuaEngine::initialize() {
    if (initialized_) {
        return true;
    }
    SCE_LOG_INFO("LuaEngine: Initializing Lua 5.4 scripting engine");
    initialized_ = true;
    return true;
}

void LuaEngine::shutdown() {
    if (!initialized_) {
        return;
    }
    SCE_LOG_INFO("LuaEngine: Shutting down");

    {
        std::lock_guard<std::mutex> lock(sessionMutex_);
        for (auto &[id, ctx] : sessions_) {
            if (ctx->L) {
                lua_close(ctx->L);
                ctx->L = nullptr;
            }
        }
        sessions_.clear();
        stateQueryCallbacks_.clear();
    }

    {
        std::lock_guard<std::mutex> lock(globalFuncMutex_);
        globalFunctions_.clear();
    }

    // §scxml-B-2: Reset DOM binding state (mirrors JSEngine::shutdown behavior)
    LuaDOMBinding::resetClassId();

    initialized_ = false;
}

void LuaEngine::reset() {
    SCE_LOG_DEBUG("LuaEngine: reset() called");
    shutdown();

    // Clear SessionRegistry (invoke mappings, file paths, event dispatchers)
    SessionRegistry::instance().reset();

    // No translation cache to clear. The lowering the engine now uses is the
    // frontend's, and it is memoised per session by `chunkCache` /
    // `scriptExecCache`, which `shutdown()` above has already dropped with the
    // sessions that owned them.

    initialize();
    SCE_LOG_DEBUG("LuaEngine: reset() completed");
}

bool LuaEngine::isInitialized() const {
    return initialized_;
}

std::string LuaEngine::getEngineInfo() const {
    return "LuaEngine (Lua 5.4)";
}

size_t LuaEngine::getMemoryUsage() const {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    size_t total = 0;
    for (auto &[id, ctx] : sessions_) {
        if (ctx->L) {
            total += static_cast<size_t>(lua_gc(ctx->L, LUA_GCCOUNT, 0)) * 1024;
            total += static_cast<size_t>(lua_gc(ctx->L, LUA_GCCOUNTB, 0));
        }
    }
    return total;
}

void LuaEngine::collectGarbage() {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    for (auto &[id, ctx] : sessions_) {
        if (ctx->L) {
            lua_gc(ctx->L, LUA_GCCOLLECT, 0);
        }
    }
}

// === Session Management ===

bool LuaEngine::createSession(const std::string &sessionId, const std::string &parentSessionId) {
    std::lock_guard<std::mutex> lock(sessionMutex_);

    if (sessions_.count(sessionId)) {
        SCE_LOG_WARN("LuaEngine: Session already exists: {}", sessionId);
        return false;
    }

    auto ctx = std::make_unique<LuaSessionContext>();
    ctx->sessionId = sessionId;
    ctx->parentSessionId = parentSessionId;
    ctx->L = createLuaState();

    if (!ctx->L) {
        SCE_LOG_ERROR("LuaEngine: Failed to create Lua state for session: {}", sessionId);
        return false;
    }

    registerBuiltins(ctx->L, sessionId);

    // Taken here and nowhere else: after the runtime has installed everything
    // it installs and before the document has run anything. Every later
    // reading of the global table is read against this.
    forEachGlobalName(ctx->L, [&ctx](const char *name) { ctx->runtimeGlobals.emplace(name); });

    sessions_[sessionId] = std::move(ctx);

    // §scxml-6.4: Register parent-child relationship in SessionRegistry
    // Enables engine-agnostic parent session lookup for event routing
    if (!parentSessionId.empty()) {
        SessionRegistry::instance().registerParentChild(sessionId, parentSessionId);
    }

    SCE_LOG_DEBUG("LuaEngine: Created session: {} (parent: {})", sessionId,
                  parentSessionId.empty() ? "none" : parentSessionId);

    return true;
}

bool LuaEngine::destroySession(const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(sessionMutex_);

    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        SCE_LOG_WARN("LuaEngine: Session not found for destroy: {}", sessionId);
        return false;
    }

    // §scxml-6.4: Unregister parent-child relationship
    SessionRegistry::instance().unregisterParentChild(sessionId);
    // §scxml-6.2: Delegate session cleanup to SessionRegistry
    SessionRegistry::instance().cleanupSession(sessionId);

    if (it->second->L) {
        lua_close(it->second->L);
    }

    sessions_.erase(it);

    // Remove state query callback
    stateQueryCallbacks_.erase(sessionId);

    // Clean up EventRaiser from global registry to prevent dangling callback access
    try {
        auto registry = EventRaiserService::getInstance().getRegistry();
        if (registry && registry->hasEventRaiser(sessionId)) {
            registry->unregisterEventRaiser(sessionId);
            SCE_LOG_DEBUG("LuaEngine: Cleaned up EventRaiser for destroyed session: {}", sessionId);
        }
    } catch (const std::exception &) {
        // EventRaiserService may not be initialized during early cleanup
    }

    SCE_LOG_DEBUG("LuaEngine: Destroyed session: {}", sessionId);

    return true;
}

bool LuaEngine::hasSession(const std::string &sessionId) const {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    return sessions_.count(sessionId) > 0;
}

// === ISessionManager ===

std::vector<std::string> LuaEngine::getActiveSessions() const {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    std::vector<std::string> result;
    result.reserve(sessions_.size());
    for (auto &[id, ctx] : sessions_) {
        result.push_back(id);
    }
    return result;
}

std::string LuaEngine::getParentSessionId(const std::string &sessionId) const {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it != sessions_.end()) {
        return it->second->parentSessionId;
    }
    return "";
}

// === Lua State Creation and Builtins ===

lua_State *LuaEngine::createLuaState() {
    lua_State *L = luaL_newstate();
    if (!L) {
        return nullptr;
    }

    // Open standard libraries
    luaL_openlibs(L);

    // Register DOM metatables (§scxml-B-2)
    LuaDOMBinding::registerMetatable(L);

    return L;
}

void LuaEngine::registerBuiltins(lua_State *L, const std::string &sessionId) {
    // Store sessionId in Lua registry for access from C callbacks
    lua_pushlightuserdata(L, const_cast<char *>("__session_id"));
    lua_pushstring(L, sessionId.c_str());
    lua_settable(L, LUA_REGISTRYINDEX);

    // Store LuaEngine pointer in registry for callback access
    lua_pushlightuserdata(L, const_cast<char *>("__lua_engine"));
    lua_pushlightuserdata(L, this);
    lua_settable(L, LUA_REGISTRYINDEX);

    // === Register SCXML built-in functions ===

    // §scxml-B-2: null/undefined sentinels for array element preservation
    lua_pushlightuserdata(L, &NULL_SENTINEL_TAG);
    lua_setglobal(L, "_NULL");
    lua_pushlightuserdata(L, &UNDEFINED_SENTINEL_TAG);
    lua_setglobal(L, "_UNDEFINED");

    // §scxml-B-2: `_scxml_truthy`, `_typeof`, `_isArray`, `_indexOf`,
    // `_concat`, `parseInt` and `parseFloat` used to be seven lambdas here,
    // one of six implementations of the same seven meanings. They are in the
    // shared sce/include/scripting/ecma_semantics.lua now, loaded at the end
    // of this function, and the sentinel arms two of them carried moved with
    // them (that file compares against the `_NULL` / `_UNDEFINED` globals set
    // above, which is the same test one lightuserdata address at a time).
    //
    // The drift they were predicted to accumulate had arrived: measured
    // 2026-08-16 against tests/ecmascript/ecma262_semantics.json, Go's
    // `_indexOf` and `_concat` had no Array branch at all, Python called
    // `typeof [1,2,3]` "function", and this copy's `_indexOf` compared with
    // `lua_compare(LUA_OPEQ)` — the coercing comparison — where the clause
    // says `===`.

    // In(stateId): §scxml-5.9.1 In() predicate
    // Uses C++ state query callbacks
    lua_pushcfunction(L, [](lua_State *Ls) -> int {
        const char *stateId = luaL_checkstring(Ls, 1);

        // Get LuaEngine pointer from registry
        lua_pushlightuserdata(Ls, const_cast<char *>("__lua_engine"));
        lua_gettable(Ls, LUA_REGISTRYINDEX);
        auto *engine = static_cast<LuaEngine *>(lua_touserdata(Ls, -1));
        lua_pop(Ls, 1);

        // Get session ID from registry
        lua_pushlightuserdata(Ls, const_cast<char *>("__session_id"));
        lua_gettable(Ls, LUA_REGISTRYINDEX);
        const char *sid = lua_tostring(Ls, -1);
        lua_pop(Ls, 1);

        if (engine && sid) {
            auto it = engine->stateQueryCallbacks_.find(sid);
            if (it != engine->stateQueryCallbacks_.end() && it->second) {
                lua_pushboolean(Ls, it->second(stateId) ? 1 : 0);
                return 1;
            }
            // Check all callbacks (parent sessions might have In() queries)
            for (auto &[callbackSessionId, callback] : engine->stateQueryCallbacks_) {
                if (callback && callback(stateId)) {
                    lua_pushboolean(Ls, 1);
                    return 1;
                }
            }
        }

        lua_pushboolean(Ls, 0);
        return 1;
    });
    lua_setglobal(L, "In");

    // Register globally registered functions (full userdata with __gc — no leak)
    {
        std::lock_guard<std::mutex> gfLock(globalFuncMutex_);
        for (auto &[name, func] : globalFunctions_) {
            pushGlobalFuncClosure(L, func);
            lua_setglobal(L, name.c_str());
        }
    }

    // ECMAScript compatibility: string __add, number/boolean __index, JSON, Object builtins
    luaL_dostring(L, R"LUA(
        -- ECMAScript '+' operator: string + anything = concatenation.
        -- __add is only invoked when Lua cannot natively handle the + operation
        -- (i.e., at least one operand is a non-numeric string). For two numeric
        -- strings like "5"+"3", Lua auto-coerces to numbers (result: 8, not "53")
        -- — a known limitation that does not affect W3C SCXML conformance.
        local mt = getmetatable('')
        if mt then
            mt.__add = function(a, b) return tostring(a) .. tostring(b) end
        end

        -- ECMAScript: property access on non-objects returns undefined (nil), not error
        -- Handles cases like (1).bar → nil, true.foo → nil
        debug.setmetatable(0, {__index = function() return nil end})
        debug.setmetatable(true, {__index = function() return nil end})

        -- `Object.keys` is in the shared semantics file with the rest of the
        -- engine vocabulary, and sorts there for the same reason it sorted
        -- here: a Lua table has no enumeration order to hand back.
    )LUA");

// §scxml-B-2: the ECMAScript operators Lua does not share — `+`, `==` and
// the bitwise family, which coerce their operands where Lua either refuses
// or answers differently. Single Source of Truth at
// sce/include/scripting/ecma_semantics.lua: the code sce-build emits calls
// these by name on every backend, so one definition is what keeps the
// engines from disagreeing about what `==` means.
#include "ecma_semantics_lua.h"
    luaL_dostring(L, ECMA_SEMANTICS_LUA);

// §scxml-B-2: JSON.stringify / JSON.parse (Single Source of Truth)
// Shared with Rust sce-rust-lua via sce/include/scripting/json_builtins.lua
// CMake generates json_builtins_lua.h with the Lua source as a C++ raw string literal
#include "json_builtins_lua.h"
    luaL_dostring(L, JSON_BUILTINS_LUA);
}

// === Script Execution ===

int LuaEngine::loadCachedChunk(lua_State *L, const std::string &code,
                               std::unordered_map<std::string, LuaSessionContext::ChunkCacheEntry> &cache) {
    auto cacheIt = cache.find(code);
    if (cacheIt != cache.end()) {
        auto &entry = cacheIt->second;
        if (entry.ref == LuaSessionContext::CHUNK_COMPILE_FAILED) {
            lua_pushstring(L, entry.error.c_str());
            return LUA_ERRSYNTAX;
        }
        lua_rawgeti(L, LUA_REGISTRYINDEX, entry.ref);
        return LUA_OK;
    }

    int status = luaL_loadstring(L, code.c_str());
    if (status == LUA_OK) {
        lua_pushvalue(L, -1);
        int ref = luaL_ref(L, LUA_REGISTRYINDEX);
        cache[code] = {ref, {}};
    } else {
        std::string err = lua_tostring(L, -1) ? lua_tostring(L, -1) : "Unknown Lua error";
        cache[code] = {LuaSessionContext::CHUNK_COMPILE_FAILED, err};
    }
    return status;
}

// The seam: the ONE step a pre-lowered call skips.
//
// docs/SCE_LUA_TRANSLATION_SEAM.md, "The seam is not 'skip the transformer'":
// evaluating an expression here does five things to the text and only the
// first is the rewrite. Steps 2-5 (the undeclared-variable check, the `return`
// wrapping, the chunk cache, the assignment fallback) belong to EVALUATING LUA,
// not to translating ECMAScript, so both paths converge on the same tail below
// rather than growing a second, simpler implementation. Those four keep their
// own clauses, cited where they are implemented in evaluateExpressionInternal.
std::optional<std::string> LuaEngine::loweredTextOf(const ScriptSource &expression, const LoweringScope &scope) {
    // Already lowered by sce-build's ECMAScript frontend. Running the rewriter
    // over the frontend's own output would shift an index the frontend already
    // made 1-based a second time (`transformArrayIndexing`), an off-by-one with
    // no diagnostic.
    if (expression.language() == ScriptLanguage::Lua) {
        return expression.text();
    }
    // The author's ECMAScript, answered by the frontend's PARSER. The owner's
    // decision on 2026-08-29 was to link the frontend and retire
    // `EcmaScriptToLuaTransformer`, and what decides what the frontend can
    // answer is the SCOPE this asks against.
    //
    // It began empty, which asked "can you answer this without me naming
    // anything?" and so selected exactly the CLOSED expressions — 11 of the 23
    // divergences then declared. `scope` is now the session's own, fed by
    // `<data id>` (`setVariableInternal`) and by what a `<script>` chunk's top
    // level introduces (`executeScriptInternal`), which is the pair the D1
    // ledger's scope census measured as sufficient for 301 of 301 sites. An
    // expression naming a variable the session holds is therefore answered
    // here too: `a && b` yields its left operand, `a == null` equates null and
    // undefined, `!a` is ToBoolean's negation — none of it reachable by a pass
    // that replaces text without knowing where an operand ends.
    //
    // Refusal is a normal answer, and it is now the LAST answer. A name the
    // session has not declared, or text the parser will not read, comes back
    // as nothing and the caller raises `error.execution`; nothing rewrites the
    // text instead. That second translator is what this seam was built to
    // retire, and while it stood, an expression it could not read was answered
    // WRONGLY rather than refused — `-7 % 3` as 2, `5 ^ 3` as 125 — with no
    // diagnostic anywhere.
    return scope.lowerValue(expression.text());
}

std::optional<std::string> LuaEngine::loweredScriptOf(const ScriptSource &script, const LoweringScope &scope) {
    if (script.language() == ScriptLanguage::Lua) {
        return script.text();
    }
    // The same seam as `loweredTextOf`, one grammar up. A `<script>` body is a
    // STATEMENT sequence, and the divergences that outlived the expression
    // seam were all of that shape: `EcmaScriptToLuaTransformer::transformScript`
    // replaces text without a parse, so `continue` becomes `_ = continue` and a
    // `return` lands in statement position. No scope can reach those, because
    // the scope only decides which NAMES resolve.
    //
    // A chunk asks less of the scope than an expression does — `var` bindings
    // are hoisted into the chunk's own frame before anything resolves
    // (`resolve::script`), so a self-contained body is answered even by an
    // empty scope. What it still asks about is the names it only READS, which
    // is why this takes the session's scope like its neighbour rather than a
    // constant.
    //
    // Refusal is the answer here too. A body the parser will not read comes
    // back as nothing and the caller raises `error.execution` rather than
    // handing the text to a pass that would replace `continue` with
    // `_ = continue` and call the result Lua.
    return scope.lowerScript(script.text());
}

// W3C SCXML §scxml-5.9.1: an expression the datamodel cannot evaluate places
// `error.execution` on the internal queue. That is the specification's whole
// answer, and it is the one the engine now gives — the refusal is reported,
// not repaired.
//
// The author's own text, never the lowering: this message travels out on
// `_event.data`, and there is no lowering to name anyway.
//
// The frontend distinguishes fifteen failures and this carries none of them,
// because the C surface it is reached through answers a pointer or null. That
// is the `error-channel` row of `docs/SCE_LUA_TRANSLATION_SEAM.md` and it is
// the next thing this boundary is owed; what it is NOT is a reason to keep
// guessing at the text, which is what the previous answer did.
void LuaEngine::offerToScope(LuaSessionContext &session, const std::string &name) {
    if (name.empty() || !session.offeredToScope.insert(name).second) {
        return;
    }
    session.loweringScope.declare(name);
}

void LuaEngine::offerDocumentGlobalsToScope(LuaSessionContext &session) {
    forEachGlobalName(session.L, [&session](const char *name) {
        if (session.runtimeGlobals.count(name) == 0) {
            offerToScope(session, name);
        }
    });
}

ScriptResult LuaEngine::refusedToLower(const ScriptSource &source, const char *role) {
    return ScriptResult::createError("SCE's ECMAScript frontend does not accept this " + std::string(role) + ": " +
                                     source.source());
}

std::future<ScriptResult> LuaEngine::doExecuteScript(const std::string &sessionId, const ScriptSource &script) {
    auto result = executeScriptInternal(sessionId, script);
    std::promise<ScriptResult> promise;
    promise.set_value(std::move(result));
    return promise.get_future();
}

std::future<ScriptResult> LuaEngine::doEvaluateExpression(const std::string &sessionId,
                                                          const ScriptSource &expression) {
    auto result = evaluateExpressionInternal(sessionId, expression);
    std::promise<ScriptResult> promise;
    promise.set_value(std::move(result));
    return promise.get_future();
}

std::future<ScriptResult> LuaEngine::doValidateExpression(const std::string &sessionId,
                                                          const ScriptSource &expression) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        std::promise<ScriptResult> p;
        p.set_value(ScriptResult::createError("Session not found: " + sessionId));
        return p.get_future();
    }

    auto lowered = loweredTextOf(expression, it->second->loweringScope);
    if (!lowered) {
        std::promise<ScriptResult> refused;
        refused.set_value(refusedToLower(expression, "expression"));
        return refused.get_future();
    }
    std::string wrapped = "return " + *lowered;

    lua_State *L = it->second->L;
    int status = loadCachedChunk(L, wrapped, it->second->chunkCache);

    std::promise<ScriptResult> p;
    if (status == LUA_OK) {
        lua_pop(L, 1);  // Pop the compiled function (validate doesn't execute)
        p.set_value(ScriptResult::createSuccess(true));
    } else {
        std::string err = lua_tostring(L, -1) ? lua_tostring(L, -1) : "Unknown Lua error";
        lua_pop(L, 1);  // Pop the error message to prevent stack leak
        p.set_value(ScriptResult::createError("Syntax error: " + err));
    }
    return p.get_future();
}

ScriptResult LuaEngine::executeScriptInternal(const std::string &sessionId, const ScriptSource &script) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    lua_State *L = it->second->L;

    // Fast path: if this script was successfully executed before in this session,
    // skip transformer and chunk cache lookup entirely. A cached entry from the
    // other language is a miss: the same text lowers differently.
    const uint64_t scopeGeneration = it->second->loweringScope.generation();
    auto &scriptExec = it->second->scriptExecCache;
    auto sit = scriptExec.find(script.text());
    if (sit != scriptExec.end() && sit->second.language == script.language() &&
        sit->second.scopeGeneration == scopeGeneration) {
        lua_rawgeti(L, LUA_REGISTRYINDEX, sit->second.chunkRef);
        int status = lua_pcall(L, 0, LUA_MULTRET, 0);
        return luaResultToScriptResult(L, status);
    }

    // Slow path: first-time execution for this script in this session
    auto loweredScript = loweredScriptOf(script, it->second->loweringScope);
    if (!loweredScript) {
        return refusedToLower(script, "script");
    }
    const std::string &luaScript = *loweredScript;

    // The author's own text on the left, so a log line names what was written
    // rather than what it became.
    SCE_LOG_DEBUG("LuaEngine: Execute script [{}]: {} -> {}", sessionId, script.source(), luaScript);

    int loadStatus = loadCachedChunk(L, luaScript, it->second->chunkCache);
    if (loadStatus != LUA_OK) {
        return luaResultToScriptResult(L, loadStatus);
    }

    // Cache for fast path on subsequent calls
    scriptExec[script.text()] = {it->second->chunkCache.at(luaScript).ref, script.language(), scopeGeneration};

    int status = lua_pcall(L, 0, LUA_MULTRET, 0);

    // §scxml-5.8: a `<script>` that ran has introduced its top-level
    // declarations into the datamodel, so the ECMAScript frontend is told
    // about them — the `declare_chunk` half of the scope, and the half that
    // reaches the variables no `<data id>` names.
    //
    // Only after a successful run, because a chunk that raised declared
    // whatever it reached and this cannot say where it stopped — but that is
    // the ONLY thing the two doors share here, and each is asked of the
    // authority that can answer it.
    //
    // ECMAScript text is asked of the frontend's own parser, which reads the
    // chunk's top level (§scxml-5.8) and is the same reader that will later
    // lower expressions against those names.
    //
    // Lua text has no ECMAScript parse to ask, and it used to be skipped
    // entirely on the reasoning that a generated artifact lowers at build time
    // and never asks this engine for a lowering. That reasoning is about a
    // DOCUMENT, and the engine's contract is about a SESSION: it accepts both
    // languages into one global namespace, so a name the Lua door introduced
    // is a name the ECMAScript door must resolve. Lua's own global table is
    // what can say which names those are.
    //
    // ⚠ The two are NOT one call with a different argument, and neither
    // subsumes the other. `var x;` binds a name to undefined — the parser
    // reports it and Lua's global table does not, because assigning nil is how
    // Lua REMOVES a global. A chunk the frontend's parser refuses declares
    // nothing through `declareChunk` and its globals are still in the table.
    // Collapsing this into one reader would silently drop one of those.
    if (status == LUA_OK) {
        if (script.language() == ScriptLanguage::ECMAScript) {
            it->second->loweringScope.declareChunk(script.text());
        } else {
            offerDocumentGlobalsToScope(*it->second);
        }
    }

    return luaResultToScriptResult(L, status);
}

ScriptResult LuaEngine::evaluateExpressionInternal(const std::string &sessionId, const ScriptSource &expression) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    lua_State *L = it->second->L;

    // Fast path: if this expression was successfully evaluated before in this session,
    // skip transformer, "return" wrapping, and double cache lookup entirely.
    // A cached entry from the other language is a miss: the same text lowers
    // differently, and reusing the wrong chunk would be an off-by-one nobody
    // gets a diagnostic for.
    //
    // An entry lowered against an OLDER scope is a miss for the same reason
    // one step out: the frontend's answer depends on which names it was told
    // about, so a `<script>` that declared one since must be able to change
    // what this text lowers to.
    const uint64_t scopeGeneration = it->second->loweringScope.generation();
    auto &execCache = it->second->exprExecCache;
    auto execIt = execCache.find(expression.text());
    if (execIt != execCache.end() && execIt->second.language == expression.language() &&
        execIt->second.scopeGeneration == scopeGeneration) {
        auto &info = execIt->second;
        lua_rawgeti(L, LUA_REGISTRYINDEX, info.chunkRef);
        int status = lua_pcall(L, 0, LUA_MULTRET, 0);
        if (info.returnsValue) {
            return luaResultToScriptResult(L, status);
        }
        if (status == LUA_OK) {
            return ScriptResult::createSuccess(ScriptUndefined{});
        }
        std::string err = lua_tostring(L, -1) ? lua_tostring(L, -1) : "Unknown Lua error";
        lua_pop(L, 1);
        return ScriptResult::createError(err);
    }

    // Slow path: first-time evaluation for this expression in this session
    auto loweredExpr = loweredTextOf(expression, it->second->loweringScope);
    if (!loweredExpr) {
        return refusedToLower(expression, "expression");
    }
    const std::string &luaExpr = *loweredExpr;

    // W3C SCXML: Detect undeclared simple variable references
    // JavaScript throws ReferenceError for undeclared variables; Lua silently returns nil.
    // For simple identifier expressions (e.g., donedata param location="foo"),
    // check if the variable is declared before evaluating.
    //
    // Checked on the LOWERED text and reported from the AUTHOR'S: this message
    // travels out on `_event.data` of `error.execution`, so naming the lowered
    // Lua here would name a language the author never wrote.
    if (isUndeclaredSimpleVariable(luaExpr, it->second->declaredVars, L)) {
        return ScriptResult::createError("ReferenceError: " + expression.source() + " is not defined");
    }

    // Wrap as return statement to get expression value
    std::string wrapped = "return " + luaExpr;
    auto &cache = it->second->chunkCache;

    SCE_LOG_DEBUG("LuaEngine: Evaluate [{}]: {} -> {}", sessionId, expression.source(), wrapped);

    // Try compiled chunk from cache (or compile + cache on first call).
    // If "return <expr>" compiles, it's a valid expression — runtime errors are returned
    // directly without assignment fallback. Assignment expressions (e.g., "x = 5") fail
    // compilation as "return x = 5" (LUA_ERRSYNTAX) and fall through to the fallback below.
    int loadStatus = loadCachedChunk(L, wrapped, cache);
    if (loadStatus == LUA_OK) {
        int status = lua_pcall(L, 0, LUA_MULTRET, 0);
        if (status == LUA_OK) {
            // Cache for fast path on subsequent calls
            execCache[expression.text()] = {cache.at(wrapped).ref, true, expression.language(), scopeGeneration};
            return luaResultToScriptResult(L, status);
        }
        std::string error = lua_tostring(L, -1) ? lua_tostring(L, -1) : "Unknown Lua error";
        lua_pop(L, 1);
        return ScriptResult::createError(error);
    }

    // Compilation of "return <expr>" failed — try assignment fallback
    std::string firstError = lua_tostring(L, -1) ? lua_tostring(L, -1) : "Unknown Lua error";
    lua_pop(L, 1);

    // §scxml-5.9: Only try statement fallback for assignment-like expressions.
    // Bare keywords like "return" are valid Lua chunks but invalid as JS expressions
    // (JavaScript's eval("return") throws SyntaxError — W3C test 344).
    bool looksLikeAssignment = false;
    for (size_t i = 0; i < luaExpr.size(); ++i) {
        if (luaExpr[i] == '=' &&
            (i == 0 ||
             (luaExpr[i - 1] != '~' && luaExpr[i - 1] != '<' && luaExpr[i - 1] != '>' && luaExpr[i - 1] != '=')) &&
            (i + 1 >= luaExpr.size() || luaExpr[i + 1] != '=')) {
            looksLikeAssignment = true;
            break;
        }
    }

    if (looksLikeAssignment) {
        loadStatus = loadCachedChunk(L, luaExpr, cache);
        if (loadStatus == LUA_OK) {
            int status = lua_pcall(L, 0, LUA_MULTRET, 0);
            if (status == LUA_OK) {
                // Cache for fast path on subsequent calls
                execCache[expression.text()] = {cache.at(luaExpr).ref, false, expression.language(), scopeGeneration};
                return ScriptResult::createSuccess(ScriptUndefined{});
            }
            lua_pop(L, 1);
        } else {
            lua_pop(L, 1);
        }
    }

    return ScriptResult::createError(firstError);
}

ScriptResult LuaEngine::luaResultToScriptResult(lua_State *L, int status) {
    if (status != LUA_OK) {
        std::string err = lua_tostring(L, -1) ? lua_tostring(L, -1) : "Unknown Lua error";
        lua_pop(L, 1);
        return ScriptResult::createError(err);
    }

    // Convert top of stack to ScriptValue
    if (lua_gettop(L) == 0) {
        return ScriptResult::createSuccess(ScriptUndefined{});
    }

    ScriptValue value = luaToScriptValue(L, -1);
    lua_pop(L, 1);
    return ScriptResult::createSuccess(std::move(value));
}

// === Variable Management ===

std::future<ScriptResult> LuaEngine::setVariable(const std::string &sessionId, const std::string &name,
                                                 const ScriptValue &value) {
    auto result = setVariableInternal(sessionId, name, value);
    std::promise<ScriptResult> p;
    p.set_value(std::move(result));
    return p.get_future();
}

std::future<ScriptResult> LuaEngine::getVariable(const std::string &sessionId, const std::string &name) {
    auto result = getVariableInternal(sessionId, name);
    std::promise<ScriptResult> p;
    p.set_value(std::move(result));
    return p.get_future();
}

ScriptResult LuaEngine::setVariableInternal(const std::string &sessionId, const std::string &name,
                                            const ScriptValue &value) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    lua_State *L = it->second->L;
    pushScriptValue(L, value);
    lua_setglobal(L, name.c_str());

    // Track declared variables (Lua nil == undeclared, so we need explicit tracking)
    it->second->declaredVars.insert(name);

    // §scxml-5.3: the same declaration, told to the ECMAScript frontend. This
    // is the `<data id>` half of the scope the D1 ledger's census measured —
    // without it the frontend refuses every expression naming this variable,
    // and refusal is now the engine's last answer rather than the rewriter's
    // cue.
    offerToScope(*it->second, name);

    // Track pre-initialized variables for invoke param/namelist support
    // §scxml-6.4.2: DataModelInitializer skips re-initialization of pre-initialized variables
    it->second->preInitializedVars.insert(name);

    return ScriptResult::createSuccess(true);
}

ScriptResult LuaEngine::getVariableInternal(const std::string &sessionId, const std::string &name) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    lua_State *L = it->second->L;
    lua_getglobal(L, name.c_str());
    ScriptValue value = luaToScriptValue(L, -1);
    lua_pop(L, 1);

    return ScriptResult::createSuccess(std::move(value));
}

std::future<ScriptResult> LuaEngine::setVariableAsDOM(const std::string &sessionId, const std::string &name,
                                                      const std::string &xmlContent) {
    // §scxml-B-2: XML DOM as Lua userdata with getElementsByTagName/getAttribute methods
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        std::promise<ScriptResult> p;
        p.set_value(ScriptResult::createError("Session not found: " + sessionId));
        return p.get_future();
    }

    lua_State *L = it->second->L;
    // A caller that already decided the content is a document, so a refusal
    // leaves the variable unbound exactly as before.
    if (LuaDOMBinding::pushDOMObject(L, xmlContent) == 0) {
        SCE_LOG_ERROR("LuaEngine: <data> content is not a valid XML document, leaving '{}' unbound", name);
        lua_pushnil(L);
    }
    lua_setglobal(L, name.c_str());

    // §scxml-5.3: a `<data>` whose content is XML is still a `<data id>`, and
    // the frontend has to be told about it by the same rule as any other. It
    // was not, and nothing said so while a rewriter answered without resolving
    // names: all 39 DOM reads in the shared table are `var1.<member>`, and
    // every one of them named a variable the frontend had never heard of.
    offerToScope(*it->second, name);

    std::promise<ScriptResult> p;
    p.set_value(ScriptResult::createSuccess(true));
    return p.get_future();
}

bool LuaEngine::hasVariable(const std::string &sessionId, const std::string &variableName) const {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        return false;
    }

    bool isDottedPath = variableName.find('.') != std::string::npos;

    // Check explicit tracking first (simple names only)
    if (!isDottedPath) {
        if (it->second->declaredVars.count(variableName) > 0) {
            return true;
        }
    }

    // Check Lua state: simple globals or dotted path traversal
    lua_State *L = it->second->L;
    if (!L) {
        return false;
    }

    if (!isDottedPath) {
        // Simple variable name — check global table directly
        lua_getglobal(L, variableName.c_str());
        bool exists = !lua_isnil(L, -1);
        lua_pop(L, 1);
        return exists;
    }

    // Dotted path (e.g., "obj.nested.value") — traverse table chain
    std::istringstream stream(variableName);
    std::string segment;
    int pushCount = 0;

    // Get root variable
    if (!std::getline(stream, segment, '.') || segment.empty()) {
        return false;
    }
    lua_getglobal(L, segment.c_str());
    ++pushCount;

    if (lua_isnil(L, -1)) {
        lua_pop(L, pushCount);
        return false;
    }

    // Traverse remaining path segments
    while (std::getline(stream, segment, '.')) {
        if (!lua_istable(L, -1)) {
            lua_pop(L, pushCount);
            return false;
        }
        lua_getfield(L, -1, segment.c_str());
        ++pushCount;
        if (lua_isnil(L, -1)) {
            lua_pop(L, pushCount);
            return false;
        }
    }

    lua_pop(L, pushCount);
    return true;
}

bool LuaEngine::isVariablePreInitialized(const std::string &sessionId, const std::string &variableName) const {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it != sessions_.end()) {
        return it->second->preInitializedVars.count(variableName) > 0;
    }
    return false;
}

// === SCXML System Variables ===

std::future<ScriptResult> LuaEngine::setupSystemVariables(const std::string &sessionId, const std::string &sessionName,
                                                          const std::vector<IOProcessorDescriptor> &ioProcessors) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        std::promise<ScriptResult> p;
        p.set_value(ScriptResult::createError("Session not found: " + sessionId));
        return p.get_future();
    }

    lua_State *L = it->second->L;
    it->second->sessionName = sessionName;

    // _sessionid (read-only system variable)
    lua_pushstring(L, sessionId.c_str());
    lua_setglobal(L, "_sessionid");

    // _name (read-only system variable)
    lua_pushstring(L, sessionName.c_str());
    lua_setglobal(L, "_name");

    // §scxml-C-1-1 / §scxml-C-2-3: _ioprocessors carries one entry per
    // processor the deployment supports, each with a 'location' field holding
    // the address that reaches this session through it. Both the entry names
    // and the locations are decided by `IOProcessorHelper::build`, so this
    // engine's view of `_ioprocessors` is identical to QuickJS's.
    lua_newtable(L);
    for (const auto &processor : ioProcessors) {
        lua_newtable(L);
        lua_pushstring(L, processor.location.c_str());
        lua_setfield(L, -2, "location");
        lua_setfield(L, -2, processor.name.c_str());
    }
    lua_setglobal(L, "_ioprocessors");

    it->second->systemVarsInitialized = true;

    std::promise<ScriptResult> p;
    p.set_value(ScriptResult::createSuccess(true));
    return p.get_future();
}

// === Event Management ===

std::future<SetCurrentEventResult> LuaEngine::setCurrentEvent(const std::string &sessionId,
                                                              const std::shared_ptr<Event> &event) {
    if (!event) {
        std::promise<SetCurrentEventResult> p;
        p.set_value({ScriptResult::createSuccess(true), PayloadReading::Absent});
        return p.get_future();
    }

    // §scxml-5.10: Engine-agnostic ScriptValue pipeline. When typedData is
    // present, `EventRaiserImpl::raiseEventWithPriority` has already parsed the
    // JSON eventData at pipeline entry (sce/src/runtime/EventRaiserImpl.cpp:220).
    // Routing through the 8-arg overload would re-parse eventData via
    // luaL_dostring + jsonStringToScriptValue (~2.2us/event for realistic JSON,
    // benchmark_luaengine SetCurrentEvent* — mesh_open_issues.md Issue 4), only
    // for the post-call overlay to overwrite _event.data. The fast path below
    // sets the table directly and skips the redundant string parse.
    if (event->hasTypedData()) {
        std::lock_guard<std::mutex> lock(sessionMutex_);
        auto it = sessions_.find(sessionId);
        if (it == sessions_.end()) {
            std::promise<SetCurrentEventResult> p;
            p.set_value({ScriptResult::createError("Session not found: " + sessionId), PayloadReading::Absent});
            return p.get_future();
        }

        lua_State *L = it->second->L;

        lua_newtable(L);

        lua_pushstring(L, event->getName().c_str());
        lua_setfield(L, -2, "name");

        lua_pushstring(L, event->getType().c_str());
        lua_setfield(L, -2, "type");

        lua_pushstring(L, event->getSendId().c_str());
        lua_setfield(L, -2, "sendid");

        lua_pushstring(L, event->getOrigin().c_str());
        lua_setfield(L, -2, "origin");

        lua_pushstring(L, event->getOriginType().c_str());
        lua_setfield(L, -2, "origintype");

        lua_pushstring(L, event->getInvokeId().c_str());
        lua_setfield(L, -2, "invokeid");

        pushScriptValue(L, event->getTypedData().value());
        lua_setfield(L, -2, "data");

        lua_setglobal(L, "_event");

        // The typed path never walks §scxml-B-2-8-1's ladder: the value was
        // already a value when it arrived, so there was no reading to choose
        // and nothing that could have been lost. `Structured` rather than
        // `Absent`, because a payload IS present and a host asking "did my
        // data survive" should not be told there was none.
        std::promise<SetCurrentEventResult> p;
        p.set_value({ScriptResult::createSuccess(true), PayloadReading::Structured});
        return p.get_future();
    }

    // No typedData — delegate to string overload's full data parsing path
    // (XML DOM / Lua expression / JSON / plain text, §scxml-B-2).
    return setCurrentEvent(sessionId, SetCurrentEventArgs{event->getName(), event->getDataAsString(), event->getType(),
                                                          event->getSendId(), event->getOrigin(),
                                                          event->getOriginType(), event->getInvokeId()});
}

std::future<SetCurrentEventResult> LuaEngine::setCurrentEvent(const std::string &sessionId,
                                                              const SetCurrentEventArgs &args) {
    const std::string &eventName = args.eventName;
    const std::string &eventData = args.eventData;
    const std::string &eventType = args.eventType;
    const std::string &sendId = args.sendId;
    const std::string &origin = args.origin;
    const std::string &originType = args.originType;
    const std::string &invokeId = args.invokeId;
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        std::promise<SetCurrentEventResult> p;
        p.set_value({ScriptResult::createError("Session not found: " + sessionId), PayloadReading::Absent});
        return p.get_future();
    }

    lua_State *L = it->second->L;

    // Which rung of §scxml-B-2-8-1 the payload ends up on, recorded as the
    // ladder below walks. Only the code that ATTEMPTED a structured read knows
    // whether it attempted one, and that is the difference between prose
    // arriving as text and a payload whose fields have stopped existing.
    PayloadReading reading = PayloadReading::Absent;

    // Create _event table
    lua_newtable(L);

    lua_pushstring(L, eventName.c_str());
    lua_setfield(L, -2, "name");

    lua_pushstring(L, eventType.c_str());
    lua_setfield(L, -2, "type");

    lua_pushstring(L, sendId.c_str());
    lua_setfield(L, -2, "sendid");

    lua_pushstring(L, origin.c_str());
    lua_setfield(L, -2, "origin");

    lua_pushstring(L, originType.c_str());
    lua_setfield(L, -2, "origintype");

    lua_pushstring(L, invokeId.c_str());
    lua_setfield(L, -2, "invokeid");

    // §scxml-B-2-8-1: parse event data as an XML DOM, as JSON, or as a
    // space-normalized string — those three readings and no fourth.
    if (!eventData.empty()) {
        // Whether the content OPENS like a document. It is a guess about which
        // reading applies, not the reading itself: the clause conditions the
        // DOM rung on the content being one — "if the Processor can interpret
        // the content as a valid XML document, it MUST create the corresponding
        // DOM structure" — and closes with "Otherwise, the Processor MUST treat
        // the content as a space-normalized string literal". So a guess that
        // turns out wrong falls through to the rungs below rather than
        // answering nil, which is what this engine did until the repository
        // started sending `error.*` messages that name the failing construct:
        // `<assign> to detail failed` opens with `<` and is not a document.
        size_t firstNonWS = eventData.find_first_not_of(" \t\r\n");
        bool opensLikeXML = firstNonWS != std::string::npos && eventData[firstNonWS] == '<';

        if (opensLikeXML && LuaDOMBinding::pushDOMObject(L, eventData) == 1) {
            lua_setfield(L, -2, "data");
            reading = PayloadReading::Dom;
        } else {
            // §scxml-B-2: JSON becomes the corresponding value.
            //
            // There used to be a rung above this one — `luaL_dostring("return "
            // + eventData)`, running the payload as this engine's own source
            // language before anything looked at it. The 2026-08-17 round
            // removed it from the four engines that had a test lane and left it
            // in the two that did not: this one and the Kotlin Lua engine.
            // Measured 2026-08-19, it still decided all three of the following
            // here:
            //
            //   * `2 + 3` from a host arrived as the number 5, and as the
            //     string "2 + 3" on this backend's OWN QuickJS engine, which
            //     read the clause. One payload, two answers, from two engines
            //     behind one backend.
            //   * a payload that is a call RAN, in the session's own globals.
            //     `_event.data` is the one field a document takes from outside
            //     itself.
            //   * the payload was read in whatever language the receiver
            //     happened to be built from.
            //
            // The sender ships JSON (§scxml-B-2-9), so the two rungs the clause
            // names are the two that are here.
            auto parsed = EventDataHelper::jsonStringToScriptValue(eventData);
            if (parsed.has_value()) {
                pushScriptValue(L, parsed.value());
                lua_setfield(L, -2, "data");
                reading = PayloadReading::Structured;
            } else {
                // §scxml-B-2 (test 562): Space-normalize plain text content
                std::string normalized = normalizeWhitespace(eventData);
                lua_pushstring(L, normalized.c_str());
                lua_setfield(L, -2, "data");
                // Which of the two third-rung readings this is — the clause
                // treats them alike and a host does not. See
                // `SCE::payloadReadingOfText`.
                reading = payloadReadingOfText(eventData);
            }
        }
    } else {
        lua_pushnil(L);
        lua_setfield(L, -2, "data");
    }

    lua_setglobal(L, "_event");

    std::promise<SetCurrentEventResult> p;
    p.set_value({ScriptResult::createSuccess(true), reading});
    return p.get_future();
}

// === Global Function Registration ===

bool LuaEngine::registerGlobalFunction(const std::string &functionName,
                                       std::function<ScriptValue(const std::vector<ScriptValue> &)> callback) {
    // Lock order: sessionMutex_ → globalFuncMutex_ (consistent with createSession → registerBuiltins)
    std::lock_guard<std::mutex> sessLock(sessionMutex_);
    std::lock_guard<std::mutex> gfLock(globalFuncMutex_);
    globalFunctions_[functionName] = std::move(callback);

    // Register in all existing sessions (full userdata with __gc — no leak)
    for (auto &[id, ctx] : sessions_) {
        if (ctx->L) {
            pushGlobalFuncClosure(ctx->L, globalFunctions_[functionName]);
            lua_setglobal(ctx->L, functionName.c_str());
        }
    }

    return true;
}

bool LuaEngine::bindNativeObject(const std::string &sessionId, const std::string &objectName,
                                 const std::vector<std::pair<std::string, NativeMethod>> &methods) {
    std::lock_guard<std::mutex> lock(sessionMutex_);

    auto it = sessions_.find(sessionId);
    if (it == sessions_.end() || !it->second->L) {
        SCE_LOG_ERROR("LuaEngine::bindNativeObject: Session '{}' not found", sessionId);
        return false;
    }

    lua_State *L = it->second->L;
    int stackTop = lua_gettop(L);

    // Create a Lua table to represent the object
    lua_newtable(L);

    for (const auto &[methodName, method] : methods) {
        // Store method with session ownership for lifetime management
        auto methodPtr = std::make_unique<NativeMethod>(method);
        NativeMethod *rawPtr = methodPtr.get();
        it->second->boundMethods.push_back(std::move(methodPtr));

        // Push NativeMethod pointer as light userdata upvalue
        lua_pushlightuserdata(L, rawPtr);
        lua_pushcclosure(
            L,
            [](lua_State *Ls) -> int {
                auto *fn = static_cast<NativeMethod *>(lua_touserdata(Ls, lua_upvalueindex(1)));
                if (!fn) {
                    lua_pushnil(Ls);
                    return 1;
                }

                // Convert Lua arguments to ScriptValue vector
                int nargs = lua_gettop(Ls);
                std::vector<ScriptValue> args;
                args.reserve(nargs);
                for (int i = 1; i <= nargs; ++i) {
                    switch (lua_type(Ls, i)) {
                    case LUA_TBOOLEAN:
                        args.emplace_back(static_cast<bool>(lua_toboolean(Ls, i)));
                        break;
                    case LUA_TNUMBER:
                        if (lua_isinteger(Ls, i)) {
                            args.emplace_back(static_cast<int64_t>(lua_tointeger(Ls, i)));
                        } else {
                            args.emplace_back(lua_tonumber(Ls, i));
                        }
                        break;
                    case LUA_TSTRING:
                        args.emplace_back(std::string(lua_tostring(Ls, i)));
                        break;
                    default:
                        args.emplace_back(ScriptUndefined{});
                        break;
                    }
                }

                // Call the native method
                ScriptValue result = (*fn)(args);

                // Convert ScriptValue result to Lua value
                std::visit(
                    [Ls](auto &&val) {
                        using VT = std::decay_t<decltype(val)>;
                        if constexpr (std::is_same_v<VT, bool>) {
                            lua_pushboolean(Ls, val ? 1 : 0);
                        } else if constexpr (std::is_same_v<VT, int64_t>) {
                            lua_pushinteger(Ls, static_cast<lua_Integer>(val));
                        } else if constexpr (std::is_same_v<VT, double>) {
                            lua_pushnumber(Ls, val);
                        } else if constexpr (std::is_same_v<VT, std::string>) {
                            lua_pushstring(Ls, val.c_str());
                        } else {
                            lua_pushnil(Ls);
                        }
                    },
                    result);
                return 1;
            },
            1);

        // Set the closure as a field on the table: obj[methodName] = closure
        lua_setfield(L, -2, methodName.c_str());

        // Verify stack integrity after each method binding
        if (lua_gettop(L) != stackTop + 1) {
            SCE_LOG_ERROR("LuaEngine::bindNativeObject: Stack corruption after binding method '{}' for object '{}' in "
                          "session '{}'",
                          methodName, objectName, sessionId);
            lua_settop(L, stackTop);
            return false;
        }
    }

    // Set the table as a global: _G[objectName] = obj
    lua_setglobal(L, objectName.c_str());

    SCE_LOG_DEBUG("LuaEngine::bindNativeObject: Bound object '{}' with {} methods in session '{}'", objectName,
                  methods.size(), sessionId);
    return true;
}

void LuaEngine::setStateQueryCallback(StateQueryCallback callback, const std::string &sessionId) {
    stateQueryCallbacks_[sessionId] = std::move(callback);
}

// === Type Conversion ===

void LuaEngine::pushScriptValue(lua_State *L, const ScriptValue &value) {
    std::visit(
        [L](auto &&val) {
            using T = std::decay_t<decltype(val)>;
            if constexpr (std::is_same_v<T, ScriptUndefined>) {
                lua_pushnil(L);
            } else if constexpr (std::is_same_v<T, ScriptNull>) {
                // §scxml-B-2: Push null sentinel to preserve typeof semantics
                lua_pushlightuserdata(L, &NULL_SENTINEL_TAG);
            } else if constexpr (std::is_same_v<T, bool>) {
                lua_pushboolean(L, val ? 1 : 0);
            } else if constexpr (std::is_same_v<T, int64_t>) {
                lua_pushinteger(L, static_cast<lua_Integer>(val));
            } else if constexpr (std::is_same_v<T, double>) {
                lua_pushnumber(L, val);
            } else if constexpr (std::is_same_v<T, std::string>) {
                lua_pushstring(L, val.c_str());
            } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptArray>>) {
                lua_newtable(L);
                if (val) {
                    for (size_t i = 0; i < val->elements.size(); ++i) {
                        // §scxml-B-2: Use undefined sentinel in arrays to prevent nil holes
                        if (std::holds_alternative<ScriptUndefined>(val->elements[i])) {
                            lua_pushlightuserdata(L, &UNDEFINED_SENTINEL_TAG);
                        } else {
                            pushScriptValue(L, val->elements[i]);
                        }
                        lua_rawseti(L, -2, static_cast<int>(i + 1));
                    }
                }
            } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptObject>>) {
                lua_newtable(L);
                if (val) {
                    for (auto &[key, objVal] : val->properties) {
                        pushScriptValue(L, objVal);
                        lua_setfield(L, -2, key.c_str());
                    }
                }
            }
        },
        value);
}

ScriptValue LuaEngine::luaToScriptValue(lua_State *L, int index) {
    switch (lua_type(L, index)) {
    case LUA_TNIL:
        return ScriptUndefined{};
    case LUA_TBOOLEAN:
        return static_cast<bool>(lua_toboolean(L, index));
    case LUA_TNUMBER:
        if (lua_isinteger(L, index)) {
            return static_cast<int64_t>(lua_tointeger(L, index));
        }
        return lua_tonumber(L, index);
    case LUA_TSTRING:
        return std::string(lua_tostring(L, index));
    case LUA_TTABLE: {
        // Check if it's an array (sequential integer keys)
        int len = static_cast<int>(lua_rawlen(L, index));
        if (len > 0) {
            auto arr = std::make_shared<ScriptArray>();
            for (int i = 1; i <= len; ++i) {
                lua_rawgeti(L, index, i);
                arr->elements.push_back(luaToScriptValue(L, -1));
                lua_pop(L, 1);
            }
            return ScriptValue(arr);
        }
        // Object
        auto obj = std::make_shared<ScriptObject>();
        int absIndex = index < 0 ? lua_gettop(L) + index + 1 : index;
        lua_pushnil(L);
        while (lua_next(L, absIndex) != 0) {
            if (lua_isstring(L, -2)) {
                std::string key = lua_tostring(L, -2);
                obj->properties[key] = luaToScriptValue(L, -1);
            }
            lua_pop(L, 1);
        }
        return ScriptValue(obj);
    }
    case LUA_TLIGHTUSERDATA: {
        // §scxml-B-2: Convert null/undefined sentinels back to ScriptValue types
        void *p = lua_touserdata(L, index);
        if (p == &NULL_SENTINEL_TAG) {
            return ScriptNull{};
        }
        return ScriptUndefined{};
    }
    default:
        return ScriptUndefined{};
    }
}

}  // namespace SCE
