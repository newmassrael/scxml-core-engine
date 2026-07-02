// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin QuickJS — JNI bridge for QuickJS ECMAScript engine
//
// Higher-level JNI layer compared to Lua bridge: since QuickJS is value-based
// (not stack-based), we convert between JSValue and JNI types at the boundary.
// Session isolation and W3C SCXML builtins are handled in Kotlin.

#include "quickjs.h"
#include <jni.h>

#include <climits>
#include <cmath>
#include <cstring>
#include <string>

// ---------------------------------------------------------------------------
// Session handle: wraps JSRuntime + JSContext pair
// One runtime per session for thread safety (parallel test execution).
// ---------------------------------------------------------------------------
struct QJSSession {
    JSRuntime *rt;
    JSContext *ctx;
    int nextRefId;
    std::string lastError;
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
static inline QJSSession *toSession(jlong handle) {
    return reinterpret_cast<QJSSession *>(handle);
}

static inline jlong toHandle(QJSSession *session) {
    return reinterpret_cast<jlong>(session);
}

static std::string jstringToString(JNIEnv *env, jstring jstr) {
    if (!jstr) {
        return "";
    }
    const char *chars = env->GetStringUTFChars(jstr, nullptr);
    std::string result(chars);
    env->ReleaseStringUTFChars(jstr, chars);
    return result;
}

static jstring toJString(JNIEnv *env, const char *str) {
    return str ? env->NewStringUTF(str) : nullptr;
}

// Extract error message from pending exception (clears it)
static std::string getExceptionMessage(JSContext *ctx) {
    JSValue exc = JS_GetException(ctx);
    const char *str = JS_ToCString(ctx, exc);
    std::string msg = str ? str : "unknown error";
    JS_FreeCString(ctx, str);

    // Append stack trace if available
    if (JS_IsObject(exc)) {
        JSValue stack = JS_GetPropertyStr(ctx, exc, "stack");
        if (!JS_IsUndefined(stack)) {
            const char *stackStr = JS_ToCString(ctx, stack);
            if (stackStr && strlen(stackStr) > 0) {
                msg += "\n";
                msg += stackStr;
            }
            JS_FreeCString(ctx, stackStr);
        }
        JS_FreeValue(ctx, stack);
    }

    JS_FreeValue(ctx, exc);
    return msg;
}

// JNI package: com.sce.scripting.quickjs.QuickJSNative
#define JNI_METHOD(name) Java_com_sce_scripting_quickjs_QuickJSNative_##name

extern "C" {

// === Context Lifecycle ===

JNIEXPORT jlong JNICALL JNI_METHOD(createContext)(JNIEnv *, jclass) {
    auto *session = new QJSSession();
    session->rt = JS_NewRuntime();
    if (!session->rt) {
        delete session;
        return 0;
    }

    // W3C SCXML tests are lightweight; 32MB memory limit is generous
    JS_SetMemoryLimit(session->rt, 32 * 1024 * 1024);
    JS_SetMaxStackSize(session->rt, 1024 * 1024);

    session->ctx = JS_NewContext(session->rt);
    if (!session->ctx) {
        JS_FreeRuntime(session->rt);
        delete session;
        return 0;
    }

    session->nextRefId = 1;
    return toHandle(session);
}

JNIEXPORT void JNICALL JNI_METHOD(destroyContext)(JNIEnv *, jclass, jlong handle) {
    auto *session = toSession(handle);
    if (!session) {
        return;
    }
    if (session->ctx) {
        JS_FreeContext(session->ctx);
    }
    if (session->rt) {
        JS_FreeRuntime(session->rt);
    }
    delete session;
}

// === Script Execution ===

// Execute JS code as script. Returns null on success, error message on failure.
JNIEXPORT jstring JNICALL JNI_METHOD(eval)(JNIEnv *env, jclass, jlong handle, jstring code) {
    auto *session = toSession(handle);
    if (!session) {
        return toJString(env, "error: null context");
    }

    std::string js = jstringToString(env, code);
    JSValue result = JS_Eval(session->ctx, js.c_str(), js.length(), "<eval>", JS_EVAL_TYPE_GLOBAL);

    if (JS_IsException(result)) {
        std::string err = getExceptionMessage(session->ctx);
        return toJString(env, err.c_str());
    }

    JS_FreeValue(session->ctx, result);
    return nullptr;  // success
}

// Evaluate expression and return typed result string.
//
// Encoding protocol:
//   "U"         — undefined
//   "N"         — null
//   "T"         — true
//   "F"         — false
//   "I" + int   — integer (decimal, fits in int64)
//   "D" + float — double (%.17g format)
//   "S" + str   — string (rest of string is the value)
//   "R" + id    — reference (stored in JS __sce_refs[id])
//   null        — error (call getLastError)
JNIEXPORT jstring JNICALL JNI_METHOD(evalExpression)(JNIEnv *env, jclass, jlong handle, jstring code) {
    auto *session = toSession(handle);
    if (!session) {
        return nullptr;
    }

    std::string js = jstringToString(env, code);
    JSValue result = JS_Eval(session->ctx, js.c_str(), js.length(), "<expr>", JS_EVAL_TYPE_GLOBAL);

    if (JS_IsException(result)) {
        session->lastError = getExceptionMessage(session->ctx);
        return nullptr;
    }

    JSContext *ctx = session->ctx;
    std::string encoded;

    if (JS_IsUndefined(result)) {
        encoded = "U";
    } else if (JS_IsNull(result)) {
        encoded = "N";
    } else if (JS_IsBool(result)) {
        encoded = JS_ToBool(ctx, result) ? "T" : "F";
    } else if (JS_IsNumber(result)) {
        int tag = JS_VALUE_GET_TAG(result);
        if (tag == JS_TAG_INT) {
            // 32-bit integer path
            int32_t ival = JS_VALUE_GET_INT(result);
            encoded = "I" + std::to_string(ival);
        } else {
            // Float64 path — check if representable as integer
            double dval;
            JS_ToFloat64(ctx, &dval, result);
            if (!std::isnan(dval) && !std::isinf(dval) && dval == std::floor(dval) && dval >= -9007199254740992.0 &&
                dval <= 9007199254740992.0) {
                encoded = "I" + std::to_string(static_cast<int64_t>(dval));
            } else {
                char buf[64];
                snprintf(buf, sizeof(buf), "%.17g", dval);
                encoded = std::string("D") + buf;
            }
        }
    } else if (JS_IsString(result)) {
        const char *str = JS_ToCString(ctx, result);
        encoded = "S";
        if (str) {
            encoded += str;
        }
        JS_FreeCString(ctx, str);
    } else {
        // Object, array, function, BigInt, Symbol → store as reference
        JSValue global = JS_GetGlobalObject(ctx);
        JSValue refs = JS_GetPropertyStr(ctx, global, "__sce_refs");
        if (JS_IsObject(refs)) {
            int refId = session->nextRefId++;
            // JS_SetPropertyUint32 takes ownership of the value
            JS_SetPropertyUint32(ctx, refs, static_cast<uint32_t>(refId), JS_DupValue(ctx, result));
            encoded = "R" + std::to_string(refId);
        } else {
            // Registry missing or corrupted — encode as "undefined"
            session->lastError = "__sce_refs registry is not an object";
            encoded = "U";
        }
        JS_FreeValue(ctx, refs);
        JS_FreeValue(ctx, global);
    }

    JS_FreeValue(ctx, result);
    return env->NewStringUTF(encoded.c_str());
}

// Evaluate condition expression, return boolean directly.
// Returns -1 on error, 0 for false, 1 for true.
JNIEXPORT jint JNICALL JNI_METHOD(evalToBoolean)(JNIEnv *env, jclass, jlong handle, jstring code) {
    auto *session = toSession(handle);
    if (!session) {
        return -1;
    }

    std::string js = jstringToString(env, code);
    JSValue result = JS_Eval(session->ctx, js.c_str(), js.length(), "<cond>", JS_EVAL_TYPE_GLOBAL);

    if (JS_IsException(result)) {
        session->lastError = getExceptionMessage(session->ctx);
        return -1;
    }

    int boolResult = JS_ToBool(session->ctx, result);
    JS_FreeValue(session->ctx, result);
    return boolResult;
}

// === Global Variable Setters ===

JNIEXPORT void JNICALL JNI_METHOD(setGlobalString)(JNIEnv *env, jclass, jlong handle, jstring name, jstring value) {
    auto *session = toSession(handle);
    if (!session) {
        return;
    }

    std::string n = jstringToString(env, name);
    std::string v = jstringToString(env, value);

    JSValue global = JS_GetGlobalObject(session->ctx);
    // JS_SetPropertyStr takes ownership of the JSValue
    JS_SetPropertyStr(session->ctx, global, n.c_str(), JS_NewString(session->ctx, v.c_str()));
    JS_FreeValue(session->ctx, global);
}

JNIEXPORT void JNICALL JNI_METHOD(setGlobalInt)(JNIEnv *env, jclass, jlong handle, jstring name, jlong value) {
    auto *session = toSession(handle);
    if (!session) {
        return;
    }

    std::string n = jstringToString(env, name);

    JSValue global = JS_GetGlobalObject(session->ctx);
    JS_SetPropertyStr(session->ctx, global, n.c_str(), JS_NewInt64(session->ctx, static_cast<int64_t>(value)));
    JS_FreeValue(session->ctx, global);
}

JNIEXPORT void JNICALL JNI_METHOD(setGlobalDouble)(JNIEnv *env, jclass, jlong handle, jstring name, jdouble value) {
    auto *session = toSession(handle);
    if (!session) {
        return;
    }

    std::string n = jstringToString(env, name);

    JSValue global = JS_GetGlobalObject(session->ctx);
    JS_SetPropertyStr(session->ctx, global, n.c_str(), JS_NewFloat64(session->ctx, value));
    JS_FreeValue(session->ctx, global);
}

JNIEXPORT void JNICALL JNI_METHOD(setGlobalBoolean)(JNIEnv *env, jclass, jlong handle, jstring name, jboolean value) {
    auto *session = toSession(handle);
    if (!session) {
        return;
    }

    std::string n = jstringToString(env, name);

    JSValue global = JS_GetGlobalObject(session->ctx);
    JS_SetPropertyStr(session->ctx, global, n.c_str(), JS_NewBool(session->ctx, value));
    JS_FreeValue(session->ctx, global);
}

JNIEXPORT void JNICALL JNI_METHOD(setGlobalNull)(JNIEnv *env, jclass, jlong handle, jstring name) {
    auto *session = toSession(handle);
    if (!session) {
        return;
    }

    std::string n = jstringToString(env, name);

    JSValue global = JS_GetGlobalObject(session->ctx);
    JS_SetPropertyStr(session->ctx, global, n.c_str(), JS_NULL);
    JS_FreeValue(session->ctx, global);
}

JNIEXPORT void JNICALL JNI_METHOD(setGlobalUndefined)(JNIEnv *env, jclass, jlong handle, jstring name) {
    auto *session = toSession(handle);
    if (!session) {
        return;
    }

    std::string n = jstringToString(env, name);

    // Delete the property to make it truly undefined
    JSValue global = JS_GetGlobalObject(session->ctx);
    JS_SetPropertyStr(session->ctx, global, n.c_str(), JS_UNDEFINED);
    JS_FreeValue(session->ctx, global);
}

// === Error Handling ===

JNIEXPORT jstring JNICALL JNI_METHOD(getLastError)(JNIEnv *env, jclass, jlong handle) {
    auto *session = toSession(handle);
    if (!session) {
        return toJString(env, "null context");
    }

    if (!session->lastError.empty()) {
        jstring msg = toJString(env, session->lastError.c_str());
        session->lastError.clear();
        return msg;
    }
    return nullptr;
}

}  // extern "C"
