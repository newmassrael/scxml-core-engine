// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/LuaEngine.h"
#include "SCXMLTypes.h"
#include "common/EventDataHelper.h"
#include "core/LogMacros.h"
#include "events/EventRaiserService.h"
#include "runtime/DataContentHelpers.h"
#include "scripting/EcmaScriptToLuaTransformer.h"
#include "scripting/LuaDOMBinding.h"
#include "scripting/ScriptResult.h"
#include "scripting/SessionRegistry.h"

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

    // Clear expression transformation cache
    transformer_.clearCache();

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

    // _scxml_truthy(v): ECMAScript truthiness semantics
    // In JS: 0, "", null, undefined, NaN are falsy. In Lua only nil/false are falsy.
    lua_pushcfunction(L, [](lua_State *Ls) -> int {
        if (lua_isnil(Ls, 1) || (lua_isboolean(Ls, 1) && !lua_toboolean(Ls, 1))) {
            lua_pushboolean(Ls, 0);
            return 1;
        }
        // §scxml-B-2: null/undefined sentinels are falsy
        if (lua_islightuserdata(Ls, 1)) {
            void *p = lua_touserdata(Ls, 1);
            if (p == &NULL_SENTINEL_TAG || p == &UNDEFINED_SENTINEL_TAG) {
                lua_pushboolean(Ls, 0);
                return 1;
            }
        }
        if (lua_isnumber(Ls, 1)) {
            lua_pushboolean(Ls, lua_tonumber(Ls, 1) != 0.0 ? 1 : 0);
            return 1;
        }
        if (lua_isstring(Ls, 1)) {
            size_t len = 0;
            lua_tolstring(Ls, 1, &len);
            lua_pushboolean(Ls, len > 0 ? 1 : 0);
            return 1;
        }
        // Tables, functions, userdata are truthy
        lua_pushboolean(Ls, 1);
        return 1;
    });
    lua_setglobal(L, "_scxml_truthy");

    // _typeof(v): ECMAScript typeof semantics
    lua_pushcfunction(L, [](lua_State *Ls) -> int {
        if (lua_isnil(Ls, 1)) {
            lua_pushstring(Ls, "undefined");
        } else if (lua_islightuserdata(Ls, 1)) {
            // §scxml-B-2: typeof null === "object", typeof undefined === "undefined"
            void *p = lua_touserdata(Ls, 1);
            lua_pushstring(Ls, (p == &NULL_SENTINEL_TAG) ? "object" : "undefined");
        } else if (lua_isboolean(Ls, 1)) {
            lua_pushstring(Ls, "boolean");
        } else if (lua_isnumber(Ls, 1)) {
            lua_pushstring(Ls, "number");
        } else if (lua_isstring(Ls, 1)) {
            lua_pushstring(Ls, "string");
        } else if (lua_isfunction(Ls, 1)) {
            lua_pushstring(Ls, "function");
        } else {
            lua_pushstring(Ls, "object");
        }
        return 1;
    });
    lua_setglobal(L, "_typeof");

    // _isArray(v): ECMAScript instanceof Array check
    lua_pushcfunction(L, [](lua_State *Ls) -> int {
        if (!lua_istable(Ls, 1)) {
            lua_pushboolean(Ls, 0);
            return 1;
        }
        // Check for __is_array marker in metatable
        if (lua_getmetatable(Ls, 1)) {
            lua_pushstring(Ls, "__is_array");
            lua_rawget(Ls, -2);
            if (!lua_isnil(Ls, -1)) {
                lua_pushboolean(Ls, 1);
                return 1;
            }
            lua_pop(Ls, 2);  // pop nil + metatable
        }
        // Heuristic: table with consecutive integer keys starting at 0 or 1
        lua_pushboolean(Ls, lua_rawlen(Ls, 1) > 0 ? 1 : 0);
        return 1;
    });
    lua_setglobal(L, "_isArray");

    // _indexOf(obj, value): ECMAScript Array.indexOf / String.indexOf (0-based, returns -1 if not found)
    lua_pushcfunction(L, [](lua_State *Ls) -> int {
        // §scxml-B-2: String.prototype.indexOf(searchString)
        if (lua_isstring(Ls, 1) && lua_isstring(Ls, 2)) {
            const char *haystack = lua_tostring(Ls, 1);
            const char *needle = lua_tostring(Ls, 2);
            const char *found = strstr(haystack, needle);
            if (found) {
                lua_pushinteger(Ls, static_cast<lua_Integer>(found - haystack));  // 0-based
            } else {
                lua_pushinteger(Ls, -1);
            }
            return 1;
        }
        // Array.prototype.indexOf(searchElement)
        if (!lua_istable(Ls, 1)) {
            lua_pushinteger(Ls, -1);
            return 1;
        }
        int len = static_cast<int>(lua_rawlen(Ls, 1));
        for (int i = 1; i <= len; ++i) {
            lua_rawgeti(Ls, 1, i);
            if (lua_compare(Ls, -1, 2, LUA_OPEQ)) {
                lua_pop(Ls, 1);
                lua_pushinteger(Ls, i - 1);  // 0-based return
                return 1;
            }
            lua_pop(Ls, 1);
        }
        lua_pushinteger(Ls, -1);
        return 1;
    });
    lua_setglobal(L, "_indexOf");

    // _concat(arr, ...): ECMAScript Array.concat
    lua_pushcfunction(L, [](lua_State *Ls) -> int {
        lua_newtable(Ls);
        int outIdx = 1;
        int nargs = lua_gettop(Ls) - 1;  // -1 for result table

        for (int arg = 1; arg <= nargs; ++arg) {
            if (lua_istable(Ls, arg)) {
                int len = static_cast<int>(lua_rawlen(Ls, arg));
                for (int i = 1; i <= len; ++i) {
                    lua_rawgeti(Ls, arg, i);
                    lua_rawseti(Ls, -2, outIdx++);
                }
            } else {
                lua_pushvalue(Ls, arg);
                lua_rawseti(Ls, -2, outIdx++);
            }
        }
        return 1;
    });
    lua_setglobal(L, "_concat");

    // parseInt(str, base): ECMAScript parseInt equivalent
    lua_pushcfunction(L, [](lua_State *Ls) -> int {
        const char *str = luaL_checkstring(Ls, 1);
        int base = static_cast<int>(luaL_optinteger(Ls, 2, 10));
        char *endptr = nullptr;
        long long val = strtoll(str, &endptr, base);
        if (endptr == str) {
            lua_pushnil(Ls);  // NaN equivalent
        } else {
            lua_pushinteger(Ls, static_cast<lua_Integer>(val));
        }
        return 1;
    });
    lua_setglobal(L, "parseInt");

    // parseFloat(str): ECMAScript parseFloat equivalent
    lua_pushcfunction(L, [](lua_State *Ls) -> int {
        const char *str = luaL_checkstring(Ls, 1);
        char *endptr = nullptr;
        double val = strtod(str, &endptr);
        if (endptr == str) {
            lua_pushnil(Ls);
        } else {
            lua_pushnumber(Ls, val);
        }
        return 1;
    });
    lua_setglobal(L, "parseFloat");

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

        -- Object.keys (ECMAScript standard)
        Object = {}
        function Object.keys(t)
            if type(t) ~= "table" then return {} end
            local keys = {}
            for k, _ in pairs(t) do keys[#keys+1] = k end
            table.sort(keys, function(a,b)
                if type(a) == type(b) then return tostring(a) < tostring(b) end
                return type(a) < type(b)
            end)
            return keys
        end
    )LUA");

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

std::future<ScriptResult> LuaEngine::executeScript(const std::string &sessionId, const std::string &script) {
    auto result = executeScriptInternal(sessionId, script);
    std::promise<ScriptResult> promise;
    promise.set_value(std::move(result));
    return promise.get_future();
}

std::future<ScriptResult> LuaEngine::evaluateExpression(const std::string &sessionId, const std::string &expression) {
    auto result = evaluateExpressionInternal(sessionId, expression);
    std::promise<ScriptResult> promise;
    promise.set_value(std::move(result));
    return promise.get_future();
}

std::future<ScriptResult> LuaEngine::validateExpression(const std::string &sessionId, const std::string &expression) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        std::promise<ScriptResult> p;
        p.set_value(ScriptResult::createError("Session not found: " + sessionId));
        return p.get_future();
    }

    std::string luaExpr = transformer_.transform(expression);
    std::string wrapped = "return " + luaExpr;

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

ScriptResult LuaEngine::executeScriptInternal(const std::string &sessionId, const std::string &script) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    lua_State *L = it->second->L;

    // Fast path: if this script was successfully executed before in this session,
    // skip transformer and chunk cache lookup entirely.
    auto &scriptExec = it->second->scriptExecCache;
    auto sit = scriptExec.find(script);
    if (sit != scriptExec.end()) {
        lua_rawgeti(L, LUA_REGISTRYINDEX, sit->second);
        int status = lua_pcall(L, 0, LUA_MULTRET, 0);
        return luaResultToScriptResult(L, status);
    }

    // Slow path: first-time execution for this script in this session
    std::string luaScript = transformer_.transformScript(script);

    SCE_LOG_DEBUG("LuaEngine: Execute script [{}]: {} -> {}", sessionId, script, luaScript);

    int loadStatus = loadCachedChunk(L, luaScript, it->second->chunkCache);
    if (loadStatus != LUA_OK) {
        return luaResultToScriptResult(L, loadStatus);
    }

    // Cache for fast path on subsequent calls
    scriptExec[script] = it->second->chunkCache.at(luaScript).ref;

    int status = lua_pcall(L, 0, LUA_MULTRET, 0);
    return luaResultToScriptResult(L, status);
}

ScriptResult LuaEngine::evaluateExpressionInternal(const std::string &sessionId, const std::string &expression) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    lua_State *L = it->second->L;

    // Fast path: if this expression was successfully evaluated before in this session,
    // skip transformer, "return" wrapping, and double cache lookup entirely.
    auto &execCache = it->second->exprExecCache;
    auto execIt = execCache.find(expression);
    if (execIt != execCache.end()) {
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
    std::string luaExpr = transformer_.transform(expression);

    // W3C SCXML: Detect undeclared simple variable references
    // JavaScript throws ReferenceError for undeclared variables; Lua silently returns nil.
    // For simple identifier expressions (e.g., donedata param location="foo"),
    // check if the variable is declared before evaluating.
    if (isUndeclaredSimpleVariable(luaExpr, it->second->declaredVars, L)) {
        return ScriptResult::createError("ReferenceError: " + expression + " is not defined");
    }

    // Wrap as return statement to get expression value
    std::string wrapped = "return " + luaExpr;
    auto &cache = it->second->chunkCache;

    SCE_LOG_DEBUG("LuaEngine: Evaluate [{}]: {} -> {}", sessionId, expression, wrapped);

    // Try compiled chunk from cache (or compile + cache on first call).
    // If "return <expr>" compiles, it's a valid expression — runtime errors are returned
    // directly without assignment fallback. Assignment expressions (e.g., "x = 5") fail
    // compilation as "return x = 5" (LUA_ERRSYNTAX) and fall through to the fallback below.
    int loadStatus = loadCachedChunk(L, wrapped, cache);
    if (loadStatus == LUA_OK) {
        int status = lua_pcall(L, 0, LUA_MULTRET, 0);
        if (status == LUA_OK) {
            // Cache for fast path on subsequent calls
            execCache[expression] = {cache.at(wrapped).ref, true};
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
                execCache[expression] = {cache.at(luaExpr).ref, false};
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
    LuaDOMBinding::pushDOMObject(L, xmlContent);
    lua_setglobal(L, name.c_str());

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

std::future<ScriptResult> LuaEngine::setCurrentEvent(const std::string &sessionId,
                                                     const std::shared_ptr<Event> &event) {
    if (!event) {
        std::promise<ScriptResult> p;
        p.set_value(ScriptResult::createSuccess(true));
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
            std::promise<ScriptResult> p;
            p.set_value(ScriptResult::createError("Session not found: " + sessionId));
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

        std::promise<ScriptResult> p;
        p.set_value(ScriptResult::createSuccess(true));
        return p.get_future();
    }

    // No typedData — delegate to string overload's full data parsing path
    // (XML DOM / Lua expression / JSON / plain text, §scxml-B-2).
    return setCurrentEvent(sessionId, SetCurrentEventArgs{event->getName(), event->getDataAsString(), event->getType(),
                                                          event->getSendId(), event->getOrigin(),
                                                          event->getOriginType(), event->getInvokeId()});
}

std::future<ScriptResult> LuaEngine::setCurrentEvent(const std::string &sessionId, const SetCurrentEventArgs &args) {
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
        std::promise<ScriptResult> p;
        p.set_value(ScriptResult::createError("Session not found: " + sessionId));
        return p.get_future();
    }

    lua_State *L = it->second->L;

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

    // §scxml-B-2: Parse event data as XML DOM, JSON/Lua table, or string
    if (!eventData.empty()) {
        // Check if XML content
        size_t firstNonWS = eventData.find_first_not_of(" \t\r\n");
        bool isXML = firstNonWS != std::string::npos && eventData[firstNonWS] == '<';

        if (isXML) {
            // Parse as DOM object (§scxml-B-2)
            LuaDOMBinding::pushDOMObject(L, eventData);
            lua_setfield(L, -2, "data");
        } else {
            // Try to evaluate as Lua expression (for structured data like Lua tables)
            std::string loadExpr = "return " + eventData;
            if (luaL_dostring(L, loadExpr.c_str()) == LUA_OK) {
                lua_setfield(L, -2, "data");
            } else {
                lua_pop(L, 1);  // Pop error
                // §scxml-B-2: Try JSON parsing for structured event data
                // JSON syntax ({"key":"value"}) is not valid Lua, requires explicit conversion
                auto parsed = EventDataHelper::jsonStringToScriptValue(eventData);
                if (parsed.has_value()) {
                    pushScriptValue(L, parsed.value());
                    lua_setfield(L, -2, "data");
                } else {
                    // §scxml-B-2 (test 562): Space-normalize plain text content
                    std::string normalized = normalizeWhitespace(eventData);
                    lua_pushstring(L, normalized.c_str());
                    lua_setfield(L, -2, "data");
                }
            }
        }
    } else {
        lua_pushnil(L);
        lua_setfield(L, -2, "data");
    }

    lua_setglobal(L, "_event");

    std::promise<ScriptResult> p;
    p.set_value(ScriptResult::createSuccess(true));
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
