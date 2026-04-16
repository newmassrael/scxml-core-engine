// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Lua — JNI bridge for Lua 5.4 C API
//
// Thin JNI layer exposing Lua state management and script execution.
// Session isolation and ECMAScript transformation are handled in Kotlin.

#include <jni.h>

extern "C" {
#include "lua.h"
#include "lualib.h"
#include "lauxlib.h"
}

#include <string>
#include <cstring>

// ---------------------------------------------------------------------------
// Helper: Convert jstring to std::string (handles null)
// ---------------------------------------------------------------------------
static std::string jstringToString(JNIEnv *env, jstring jstr) {
    if (!jstr) return "";
    const char *chars = env->GetStringUTFChars(jstr, nullptr);
    std::string result(chars);
    env->ReleaseStringUTFChars(jstr, chars);
    return result;
}

// ---------------------------------------------------------------------------
// Helper: Create jstring from C string (handles null)
// ---------------------------------------------------------------------------
static jstring toJString(JNIEnv *env, const char *str) {
    return str ? env->NewStringUTF(str) : nullptr;
}

// ---------------------------------------------------------------------------
// Helper: Store/retrieve lua_State pointer as Java long
// ---------------------------------------------------------------------------
static inline lua_State *toState(jlong handle) {
    return reinterpret_cast<lua_State *>(handle);
}

static inline jlong toHandle(lua_State *L) {
    return reinterpret_cast<jlong>(L);
}

// JNI package: com.sce.scripting.lua.LuaNative
#define JNI_METHOD(name) Java_com_sce_scripting_lua_LuaNative_##name

extern "C" {

// === State Lifecycle ===

JNIEXPORT jlong JNICALL JNI_METHOD(newState)(JNIEnv *, jclass) {
    lua_State *L = luaL_newstate();
    if (L) luaL_openlibs(L);
    return toHandle(L);
}

JNIEXPORT void JNICALL JNI_METHOD(closeState)(JNIEnv *, jclass, jlong handle) {
    lua_State *L = toState(handle);
    if (L) lua_close(L);
}

// === Script Execution ===

JNIEXPORT jstring JNICALL JNI_METHOD(doString)(JNIEnv *env, jclass, jlong handle, jstring code) {
    lua_State *L = toState(handle);
    if (!L) return toJString(env, "error: null state");

    std::string luaCode = jstringToString(env, code);
    int status = luaL_dostring(L, luaCode.c_str());

    if (status != LUA_OK) {
        const char *err = lua_tostring(L, -1);
        jstring result = toJString(env, err ? err : "unknown error");
        lua_pop(L, 1);
        return result;
    }
    return nullptr;  // success
}

JNIEXPORT jint JNICALL JNI_METHOD(loadAndCall)(JNIEnv *env, jclass, jlong handle,
                                                jstring code, jint nresults) {
    lua_State *L = toState(handle);
    if (!L) return -1;

    std::string luaCode = jstringToString(env, code);
    int status = luaL_loadstring(L, luaCode.c_str());
    if (status != LUA_OK) return status;

    return lua_pcall(L, 0, nresults, 0);
}

// === Stack Operations ===

JNIEXPORT jint JNICALL JNI_METHOD(getTop)(JNIEnv *, jclass, jlong handle) {
    return lua_gettop(toState(handle));
}

JNIEXPORT void JNICALL JNI_METHOD(setTop)(JNIEnv *, jclass, jlong handle, jint index) {
    lua_settop(toState(handle), index);
}

JNIEXPORT void JNICALL JNI_METHOD(pop)(JNIEnv *, jclass, jlong handle, jint n) {
    lua_pop(toState(handle), n);
}

JNIEXPORT jint JNICALL JNI_METHOD(type)(JNIEnv *, jclass, jlong handle, jint index) {
    return lua_type(toState(handle), index);
}

// === Push Operations ===

JNIEXPORT void JNICALL JNI_METHOD(pushNil)(JNIEnv *, jclass, jlong handle) {
    lua_pushnil(toState(handle));
}

JNIEXPORT void JNICALL JNI_METHOD(pushBoolean)(JNIEnv *, jclass, jlong handle, jboolean val) {
    lua_pushboolean(toState(handle), val ? 1 : 0);
}

JNIEXPORT void JNICALL JNI_METHOD(pushInteger)(JNIEnv *, jclass, jlong handle, jlong val) {
    lua_pushinteger(toState(handle), static_cast<lua_Integer>(val));
}

JNIEXPORT void JNICALL JNI_METHOD(pushNumber)(JNIEnv *, jclass, jlong handle, jdouble val) {
    lua_pushnumber(toState(handle), static_cast<lua_Number>(val));
}

JNIEXPORT void JNICALL JNI_METHOD(pushString)(JNIEnv *env, jclass, jlong handle, jstring val) {
    lua_State *L = toState(handle);
    std::string s = jstringToString(env, val);
    lua_pushstring(L, s.c_str());
}

// === Get Operations ===

JNIEXPORT jboolean JNICALL JNI_METHOD(toBoolean)(JNIEnv *, jclass, jlong handle, jint index) {
    return lua_toboolean(toState(handle), index) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jlong JNICALL JNI_METHOD(toInteger)(JNIEnv *, jclass, jlong handle, jint index) {
    return static_cast<jlong>(lua_tointeger(toState(handle), index));
}

JNIEXPORT jdouble JNICALL JNI_METHOD(toNumber)(JNIEnv *, jclass, jlong handle, jint index) {
    return static_cast<jdouble>(lua_tonumber(toState(handle), index));
}

JNIEXPORT jstring JNICALL JNI_METHOD(toJString)(JNIEnv *env, jclass, jlong handle, jint index) {
    const char *str = lua_tostring(toState(handle), index);
    return toJString(env, str);
}

JNIEXPORT jboolean JNICALL JNI_METHOD(isInteger)(JNIEnv *, jclass, jlong handle, jint index) {
    return lua_isinteger(toState(handle), index) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL JNI_METHOD(isNumber)(JNIEnv *, jclass, jlong handle, jint index) {
    return lua_isnumber(toState(handle), index) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL JNI_METHOD(isString)(JNIEnv *, jclass, jlong handle, jint index) {
    return lua_isstring(toState(handle), index) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL JNI_METHOD(isTable)(JNIEnv *, jclass, jlong handle, jint index) {
    return lua_istable(toState(handle), index) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL JNI_METHOD(isNil)(JNIEnv *, jclass, jlong handle, jint index) {
    return lua_isnil(toState(handle), index) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL JNI_METHOD(isBoolean)(JNIEnv *, jclass, jlong handle, jint index) {
    return lua_isboolean(toState(handle), index) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL JNI_METHOD(isFunction)(JNIEnv *, jclass, jlong handle, jint index) {
    return lua_isfunction(toState(handle), index) ? JNI_TRUE : JNI_FALSE;
}

// === Table Operations ===

JNIEXPORT void JNICALL JNI_METHOD(createTable)(JNIEnv *, jclass, jlong handle, jint narr, jint nrec) {
    lua_createtable(toState(handle), narr, nrec);
}

JNIEXPORT void JNICALL JNI_METHOD(setTable)(JNIEnv *, jclass, jlong handle, jint index) {
    lua_settable(toState(handle), index);
}

JNIEXPORT void JNICALL JNI_METHOD(getTable)(JNIEnv *, jclass, jlong handle, jint index) {
    lua_gettable(toState(handle), index);
}

JNIEXPORT void JNICALL JNI_METHOD(setField)(JNIEnv *env, jclass, jlong handle,
                                              jint index, jstring key) {
    std::string k = jstringToString(env, key);
    lua_setfield(toState(handle), index, k.c_str());
}

JNIEXPORT void JNICALL JNI_METHOD(getField)(JNIEnv *env, jclass, jlong handle,
                                              jint index, jstring key) {
    std::string k = jstringToString(env, key);
    lua_getfield(toState(handle), index, k.c_str());
}

JNIEXPORT void JNICALL JNI_METHOD(rawSetI)(JNIEnv *, jclass, jlong handle,
                                            jint index, jlong n) {
    lua_rawseti(toState(handle), index, static_cast<lua_Integer>(n));
}

JNIEXPORT void JNICALL JNI_METHOD(rawGetI)(JNIEnv *, jclass, jlong handle,
                                            jint index, jlong n) {
    lua_rawgeti(toState(handle), index, static_cast<lua_Integer>(n));
}

JNIEXPORT jlong JNICALL JNI_METHOD(rawLen)(JNIEnv *, jclass, jlong handle, jint index) {
    return static_cast<jlong>(lua_rawlen(toState(handle), index));
}

JNIEXPORT jint JNICALL JNI_METHOD(next)(JNIEnv *, jclass, jlong handle, jint index) {
    return lua_next(toState(handle), index);
}

// === Global Variables ===

JNIEXPORT void JNICALL JNI_METHOD(setGlobal)(JNIEnv *env, jclass, jlong handle, jstring name) {
    std::string n = jstringToString(env, name);
    lua_setglobal(toState(handle), n.c_str());
}

JNIEXPORT jint JNICALL JNI_METHOD(getGlobal)(JNIEnv *env, jclass, jlong handle, jstring name) {
    std::string n = jstringToString(env, name);
    return lua_getglobal(toState(handle), n.c_str());
}

// === Error Handling ===

JNIEXPORT jstring JNICALL JNI_METHOD(getError)(JNIEnv *env, jclass, jlong handle) {
    lua_State *L = toState(handle);
    if (lua_gettop(L) > 0 && lua_isstring(L, -1)) {
        const char *err = lua_tostring(L, -1);
        return toJString(env, err);
    }
    return nullptr;
}

// === GC ===

JNIEXPORT void JNICALL JNI_METHOD(gc)(JNIEnv *, jclass, jlong handle) {
    lua_gc(toState(handle), LUA_GCCOLLECT, 0);
}

// === Registry Operations (for Lua callback closures) ===

JNIEXPORT jint JNICALL JNI_METHOD(ref)(JNIEnv *, jclass, jlong handle, jint t) {
    return luaL_ref(toState(handle), t);
}

JNIEXPORT void JNICALL JNI_METHOD(unref)(JNIEnv *, jclass, jlong handle, jint t, jint refVal) {
    luaL_unref(toState(handle), t, refVal);
}

// LUA_REGISTRYINDEX constant for Kotlin side
JNIEXPORT jint JNICALL JNI_METHOD(registryIndex)(JNIEnv *, jclass) {
    return LUA_REGISTRYINDEX;
}

// === Metatable Operations ===

JNIEXPORT jint JNICALL JNI_METHOD(newMetatable)(JNIEnv *env, jclass, jlong handle, jstring name) {
    std::string n = jstringToString(env, name);
    return luaL_newmetatable(toState(handle), n.c_str());
}

JNIEXPORT void JNICALL JNI_METHOD(setMetatable)(JNIEnv *, jclass, jlong handle, jint index) {
    lua_setmetatable(toState(handle), index);
}

JNIEXPORT jboolean JNICALL JNI_METHOD(getMetatable)(JNIEnv *, jclass, jlong handle, jint index) {
    if (lua_getmetatable(toState(handle), index)) {
        lua_pop(toState(handle), 1);
        return JNI_TRUE;
    }
    return JNI_FALSE;
}

// === Lua Type Constants ===

JNIEXPORT jint JNICALL JNI_METHOD(typeNone)(JNIEnv *, jclass)     { return LUA_TNONE; }
JNIEXPORT jint JNICALL JNI_METHOD(typeNil)(JNIEnv *, jclass)      { return LUA_TNIL; }
JNIEXPORT jint JNICALL JNI_METHOD(typeBoolean)(JNIEnv *, jclass)  { return LUA_TBOOLEAN; }
JNIEXPORT jint JNICALL JNI_METHOD(typeNumber)(JNIEnv *, jclass)   { return LUA_TNUMBER; }
JNIEXPORT jint JNICALL JNI_METHOD(typeString)(JNIEnv *, jclass)   { return LUA_TSTRING; }
JNIEXPORT jint JNICALL JNI_METHOD(typeTable)(JNIEnv *, jclass)    { return LUA_TTABLE; }
JNIEXPORT jint JNICALL JNI_METHOD(typeFunction)(JNIEnv *, jclass) { return LUA_TFUNCTION; }

}  // extern "C"
