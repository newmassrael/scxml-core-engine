#include "scripting/LuaEngine.h"
#include "core/LogMacros.h"
#include "runtime/DataContentHelpers.h"
#include "SCXMLTypes.h"
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

namespace SCE {

// W3C SCXML: Helper to detect undeclared simple variable references in Lua expressions.
// JavaScript throws ReferenceError for undeclared variables; Lua silently returns nil.
// This bridges the semantic gap for simple identifier expressions (e.g., donedata location="foo").
static bool isUndeclaredSimpleVariable(const std::string &expr,
                                       const std::unordered_set<std::string> &declaredVars,
                                       lua_State *L) {
    if (expr.empty()) return false;

    // Check if expression is a simple identifier (variable name)
    if (!std::isalpha(static_cast<unsigned char>(expr[0])) && expr[0] != '_') return false;
    for (size_t i = 1; i < expr.size(); ++i) {
        if (!std::isalnum(static_cast<unsigned char>(expr[i])) && expr[i] != '_') return false;
    }

    // Exclude Lua keywords (true, false, nil, etc.)
    static const std::unordered_set<std::string> luaKeywords = {
        "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto",
        "if",  "in",    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while"};
    if (luaKeywords.count(expr)) return false;

    // If declared via setVariable, it's valid (even if nil)
    if (declaredVars.count(expr) > 0) return false;

    // Check if it's a Lua standard library global (math, string, table, etc.)
    lua_getglobal(L, expr.c_str());
    bool isNil = lua_isnil(L, -1);
    lua_pop(L, 1);

    return isNil;  // Truly undeclared if not a keyword, not declared, and not a Lua global
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
    if (initialized_) return true;
    SCE_LOG_INFO("LuaEngine: Initializing Lua 5.4 scripting engine");
    initialized_ = true;
    return true;
}

void LuaEngine::shutdown() {
    if (!initialized_) return;
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

    {
        std::lock_guard<std::mutex> lock(observerMutex_);
        observers_.clear();
    }

    initialized_ = false;
}

void LuaEngine::reset() {
    SCE_LOG_DEBUG("LuaEngine: reset() called");
    shutdown();

    // Clear SessionRegistry (invoke mappings, file paths, event dispatchers)
    SessionRegistry::instance().reset();

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

    SCE_LOG_DEBUG("LuaEngine: Created session: {} (parent: {})", sessionId,
                  parentSessionId.empty() ? "none" : parentSessionId);

    // Notify observers (copy list to avoid deadlock if observer re-enters LuaEngine)
    {
        std::vector<ISessionObserver *> observersCopy;
        {
            std::lock_guard<std::mutex> obsLock(observerMutex_);
            observersCopy = observers_;
        }
        for (auto *obs : observersCopy) {
            obs->onSessionCreated(sessionId);
        }
    }

    return true;
}

bool LuaEngine::destroySession(const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(sessionMutex_);

    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        SCE_LOG_WARN("LuaEngine: Session not found for destroy: {}", sessionId);
        return false;
    }

    if (it->second->L) {
        lua_close(it->second->L);
    }

    sessions_.erase(it);

    // Remove state query callback
    stateQueryCallbacks_.erase(sessionId);

    SCE_LOG_DEBUG("LuaEngine: Destroyed session: {}", sessionId);

    // Notify observers (copy list to avoid deadlock if observer re-enters LuaEngine)
    {
        std::vector<ISessionObserver *> observersCopy;
        {
            std::lock_guard<std::mutex> obsLock(observerMutex_);
            observersCopy = observers_;
        }
        for (auto *obs : observersCopy) {
            obs->onSessionDestroyed(sessionId);
        }
    }

    return true;
}

bool LuaEngine::hasSession(const std::string &sessionId) const {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    return sessions_.count(sessionId) > 0;
}

// === ISessionManager ===

void LuaEngine::addObserver(ISessionObserver *observer) {
    std::lock_guard<std::mutex> lock(observerMutex_);
    observers_.push_back(observer);
}

void LuaEngine::removeObserver(ISessionObserver *observer) {
    std::lock_guard<std::mutex> lock(observerMutex_);
    observers_.erase(std::remove(observers_.begin(), observers_.end(), observer), observers_.end());
}

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
    if (!L) return nullptr;

    // Open standard libraries
    luaL_openlibs(L);

    // Register DOM metatables (W3C SCXML B.2)
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

    // _scxml_truthy(v): ECMAScript truthiness semantics
    // In JS: 0, "", null, undefined, NaN are falsy. In Lua only nil/false are falsy.
    lua_pushcfunction(L, [](lua_State *Ls) -> int {
        if (lua_isnil(Ls, 1) || (lua_isboolean(Ls, 1) && !lua_toboolean(Ls, 1))) {
            lua_pushboolean(Ls, 0);
            return 1;
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
            lua_pop(Ls, 2); // pop nil + metatable
        }
        // Heuristic: table with consecutive integer keys starting at 0 or 1
        lua_pushboolean(Ls, lua_rawlen(Ls, 1) > 0 ? 1 : 0);
        return 1;
    });
    lua_setglobal(L, "_isArray");

    // _indexOf(obj, value): ECMAScript Array.indexOf / String.indexOf (0-based, returns -1 if not found)
    lua_pushcfunction(L, [](lua_State *Ls) -> int {
        // W3C SCXML B.2: String.prototype.indexOf(searchString)
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
                lua_pushinteger(Ls, i - 1); // 0-based return
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
        int nargs = lua_gettop(Ls) - 1; // -1 for result table

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
            lua_pushnil(Ls); // NaN equivalent
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

    // In(stateId): W3C SCXML 5.9.2 In() predicate
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

    // Register globally registered functions
    {
        std::lock_guard<std::mutex> gfLock(globalFuncMutex_);
        for (auto &[name, func] : globalFunctions_) {
            // Store function pointer in upvalue
            auto *funcPtr = new std::function<ScriptValue(const std::vector<ScriptValue> &)>(func);
            lua_pushlightuserdata(L, funcPtr);
            lua_pushcclosure(L, [](lua_State *Ls) -> int {
                auto *fn = static_cast<std::function<ScriptValue(const std::vector<ScriptValue> &)> *>(
                    lua_touserdata(Ls, lua_upvalueindex(1)));
                // Collect arguments
                int nargs = lua_gettop(Ls);
                std::vector<ScriptValue> args;
                for (int i = 1; i <= nargs; ++i) {
                    // Simple conversion: numbers and strings
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
                // Push result
                std::visit([Ls](auto &&val) {
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
                }, result);
                return 1;
            }, 1);
            lua_setglobal(L, name.c_str());
        }
    }
}

// === Script Execution ===

std::future<ScriptResult> LuaEngine::executeScript(const std::string &sessionId, const std::string &script) {
    auto result = executeScriptInternal(sessionId, script);
    std::promise<ScriptResult> promise;
    promise.set_value(std::move(result));
    return promise.get_future();
}

std::future<ScriptResult> LuaEngine::evaluateExpression(const std::string &sessionId,
                                                          const std::string &expression) {
    auto result = evaluateExpressionInternal(sessionId, expression);
    std::promise<ScriptResult> promise;
    promise.set_value(std::move(result));
    return promise.get_future();
}

std::future<ScriptResult> LuaEngine::validateExpression(const std::string &sessionId,
                                                          const std::string &expression) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        std::promise<ScriptResult> p;
        p.set_value(ScriptResult::createError("Session not found: " + sessionId));
        return p.get_future();
    }

    std::string luaExpr = transformer_.transform(expression);
    std::string wrapped = "return " + luaExpr;

    int status = luaL_loadstring(it->second->L, wrapped.c_str());
    if (status == LUA_OK) {
        lua_pop(it->second->L, 1); // Pop the compiled chunk
    }

    std::promise<ScriptResult> p;
    p.set_value(status == LUA_OK ? ScriptResult::createSuccess(true) :
                ScriptResult::createError("Syntax error: " + std::string(lua_tostring(it->second->L, -1))));
    return p.get_future();
}

ScriptResult LuaEngine::executeScriptInternal(const std::string &sessionId, const std::string &script) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    lua_State *L = it->second->L;
    std::string luaScript = transformer_.transformScript(script);

    SCE_LOG_DEBUG("LuaEngine: Execute script [{}]: {} -> {}", sessionId, script, luaScript);

    int status = luaL_dostring(L, luaScript.c_str());
    return luaResultToScriptResult(L, status);
}

ScriptResult LuaEngine::evaluateExpressionInternal(const std::string &sessionId, const std::string &expression) {
    std::lock_guard<std::mutex> lock(sessionMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    lua_State *L = it->second->L;
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

    SCE_LOG_DEBUG("LuaEngine: Evaluate [{}]: {} -> {}", sessionId, expression, wrapped);

    int status = luaL_dostring(L, wrapped.c_str());
    if (status != LUA_OK) {
        // Try without return (might be a statement like assignment)
        lua_pop(L, 1); // Pop error message
        status = luaL_dostring(L, luaExpr.c_str());
        if (status != LUA_OK) {
            std::string err = lua_tostring(L, -1);
            lua_pop(L, 1);
            return ScriptResult::createError(err);
        }
        return ScriptResult::createSuccess(ScriptUndefined{});
    }

    return luaResultToScriptResult(L, status);
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
    // W3C SCXML 6.4.2: DataModelInitializer skips re-initialization of pre-initialized variables
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
    // W3C SCXML B.2: XML DOM as Lua userdata with getElementsByTagName/getAttribute methods
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
    if (it != sessions_.end()) {
        return it->second->declaredVars.count(variableName) > 0;
    }
    return false;
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

std::future<ScriptResult> LuaEngine::setupSystemVariables(const std::string &sessionId,
                                                            const std::string &sessionName,
                                                            const std::vector<std::string> &ioProcessors) {
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

    // _ioprocessors (table of I/O processors)
    lua_newtable(L);
    for (const auto &proc : ioProcessors) {
        lua_newtable(L);
        lua_pushstring(L, proc.c_str());
        lua_setfield(L, -2, "location");
        lua_setfield(L, -2, proc.c_str());
    }
    // Also set 'scxml' processor with location
    if (std::find(ioProcessors.begin(), ioProcessors.end(), "http://www.w3.org/TR/scxml/#SCXMLEventProcessor") !=
        ioProcessors.end()) {
        lua_getfield(L, -1, "http://www.w3.org/TR/scxml/#SCXMLEventProcessor");
        if (lua_isnil(L, -1)) {
            lua_pop(L, 1);
            lua_newtable(L);
            lua_pushstring(L, "http://www.w3.org/TR/scxml/#SCXMLEventProcessor");
            lua_setfield(L, -2, "location");
            lua_setfield(L, -2, "http://www.w3.org/TR/scxml/#SCXMLEventProcessor");
        } else {
            lua_pop(L, 1);
        }
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

    // Call string overload first (sets up _event table with all metadata fields)
    auto future = setCurrentEvent(sessionId, event->getName(), event->getDataAsString(), event->getType(),
                                  event->getSendId(), event->getOrigin(), event->getOriginType(), event->getInvokeId());
    // Ensure string overload completes before overlaying typed data
    future.get();

    // W3C SCXML 5.10: Override _event.data with typed data when available (avoids JSON parsing)
    if (event->hasTypedData()) {
        std::lock_guard<std::mutex> lock(sessionMutex_);
        auto it = sessions_.find(sessionId);
        if (it != sessions_.end()) {
            lua_State *L = it->second->L;
            lua_getglobal(L, "_event");
            if (lua_istable(L, -1)) {
                pushScriptValue(L, event->getTypedData().value());
                lua_setfield(L, -2, "data");
            }
            lua_pop(L, 1);
        }
    }

    std::promise<ScriptResult> p;
    p.set_value(ScriptResult::createSuccess(true));
    return p.get_future();
}

std::future<ScriptResult> LuaEngine::setCurrentEvent(const std::string &sessionId, const std::string &eventName,
                                                       const std::string &eventData, const std::string &eventType,
                                                       const std::string &sendId, const std::string &origin,
                                                       const std::string &originType, const std::string &invokeId) {
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

    // W3C SCXML B.2: Parse event data as XML DOM, JSON/Lua table, or string
    if (!eventData.empty()) {
        // Check if XML content
        size_t firstNonWS = eventData.find_first_not_of(" \t\r\n");
        bool isXML = firstNonWS != std::string::npos && eventData[firstNonWS] == '<';

        if (isXML) {
            // Parse as DOM object (W3C SCXML B.2)
            LuaDOMBinding::pushDOMObject(L, eventData);
            lua_setfield(L, -2, "data");
        } else {
            // Try to evaluate as Lua expression (for structured data)
            std::string loadExpr = "return " + eventData;
            if (luaL_dostring(L, loadExpr.c_str()) == LUA_OK) {
                lua_setfield(L, -2, "data");
            } else {
                lua_pop(L, 1); // Pop error
                // W3C SCXML B.2 (test 562): Space-normalize plain text content
                std::string normalized = normalizeWhitespace(eventData);
                lua_pushstring(L, normalized.c_str());
                lua_setfield(L, -2, "data");
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
    std::lock_guard<std::mutex> lock(globalFuncMutex_);
    globalFunctions_[functionName] = std::move(callback);

    // Register in all existing sessions
    std::lock_guard<std::mutex> sessLock(sessionMutex_);
    for (auto &[id, ctx] : sessions_) {
        if (ctx->L) {
            auto *funcPtr = new std::function<ScriptValue(const std::vector<ScriptValue> &)>(globalFunctions_[functionName]);
            lua_pushlightuserdata(ctx->L, funcPtr);
            lua_pushcclosure(ctx->L, [](lua_State *Ls) -> int {
                auto *fn = static_cast<std::function<ScriptValue(const std::vector<ScriptValue> &)> *>(
                    lua_touserdata(Ls, lua_upvalueindex(1)));
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
                std::visit([Ls](auto &&val) {
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
                }, result);
                return 1;
            }, 1);
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
    std::visit([L](auto &&val) {
        using T = std::decay_t<decltype(val)>;
        if constexpr (std::is_same_v<T, ScriptUndefined>) {
            lua_pushnil(L);
        } else if constexpr (std::is_same_v<T, ScriptNull>) {
            lua_pushnil(L);
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
                    std::visit([L](auto &&elem) {
                        using E = std::decay_t<decltype(elem)>;
                        if constexpr (std::is_same_v<E, int64_t>) lua_pushinteger(L, elem);
                        else if constexpr (std::is_same_v<E, double>) lua_pushnumber(L, elem);
                        else if constexpr (std::is_same_v<E, std::string>) lua_pushstring(L, elem.c_str());
                        else if constexpr (std::is_same_v<E, bool>) lua_pushboolean(L, elem ? 1 : 0);
                        else lua_pushnil(L);
                    }, val->elements[i]);
                    lua_rawseti(L, -2, static_cast<int>(i + 1));
                }
            }
        } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptObject>>) {
            lua_newtable(L);
            if (val) {
                for (auto &[key, objVal] : val->properties) {
                    std::visit([L](auto &&v) {
                        using V = std::decay_t<decltype(v)>;
                        if constexpr (std::is_same_v<V, int64_t>) lua_pushinteger(L, v);
                        else if constexpr (std::is_same_v<V, double>) lua_pushnumber(L, v);
                        else if constexpr (std::is_same_v<V, std::string>) lua_pushstring(L, v.c_str());
                        else if constexpr (std::is_same_v<V, bool>) lua_pushboolean(L, v ? 1 : 0);
                        else lua_pushnil(L);
                    }, objVal);
                    lua_setfield(L, -2, key.c_str());
                }
            }
        }
    }, value);
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
        default:
            return ScriptUndefined{};
    }
}

}  // namespace SCE
