// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Kotlin Lua — JNI bridge to sce-build's ECMAScript frontend
//
// The C++ engine reaches the same C surface through `SCE::LoweringScope`
// (`sce/src/scripting/LoweringScope.cpp`), and this is the JVM's half of the
// identical contract: the surface is named in exactly ONE translation unit per
// backend, and everything above it talks to a class instead of to `extern "C"`.
//
// Why the JVM engine calls it at all is `docs/SCE_LUA_TRANSLATION_SEAM.md`:
// `EcmaScriptToLuaTransformer` rewrites the author's ECMAScript as TEXT,
// without parsing it, so it cannot say where an operand ends — `a && b` cannot
// yield its left operand and `-7 % 3` cannot truncate. The frontend behind this
// file parses.
//
// Refusal is an answer. Every `sce_lower_*` returns NULL when the frontend will
// not lower the text — it did not parse, or it names something the scope does
// not declare — and each function here turns that into a Java `null` rather
// than into an empty string. An empty string is a legal lowering of an empty
// script, so collapsing the two would make "refused" and "lowered to nothing"
// the same answer.

#include <jni.h>

#include <cstddef>

#include "scripting/SceLowering.h"

namespace {

/// Hand a `sce_lower_*` result to the JVM and release it on the frontend's own
/// allocator.
///
/// This is the half every entry point shares — the refusal check, the copy into
/// a `jstring`, and the free — so no caller here can forget one. A leak on this
/// path would be per EXPRESSION rather than per session.
jstring adopt(JNIEnv *env, char *lowered) {
    if (lowered == nullptr) {
        return nullptr;
    }
    jstring result = env->NewStringUTF(lowered);
    sce_lower_free(lowered);
    return result;
}

/// A scope handle as the JVM carries it.
///
/// `jlong` rather than a pointer-shaped Java type for the same reason
/// `LuaNative` carries `lua_State *` that way: the JVM has no pointer, and a
/// 64-bit integer is the widest thing both sides agree on.
SceLoweringScope *scopeOf(jlong handle) {
    return reinterpret_cast<SceLoweringScope *>(handle);
}

/// A borrowed UTF-8 view of a Java string, released on scope exit.
///
/// `GetStringUTFChars` can return NULL when the VM cannot allocate, and every
/// caller here treats that as a refusal rather than passing NULL into the C
/// surface — which would be a refusal too, but by way of an undefined read.
class Utf8 {
public:
    Utf8(JNIEnv *env, jstring value) : env_(env), value_(value) {
        chars_ = (value == nullptr) ? nullptr : env->GetStringUTFChars(value, nullptr);
    }

    ~Utf8() {
        if (chars_ != nullptr) {
            env_->ReleaseStringUTFChars(value_, chars_);
        }
    }

    Utf8(const Utf8 &) = delete;
    Utf8 &operator=(const Utf8 &) = delete;

    const char *get() const {
        return chars_;
    }

private:
    JNIEnv *env_;
    jstring value_;
    const char *chars_ = nullptr;
};

}  // namespace

extern "C" {

#define JNI_LOWERING(name) Java_com_sce_scripting_lua_SceLoweringNative_##name

JNIEXPORT jlong JNICALL JNI_LOWERING(newScope)(JNIEnv *, jclass) {
    return reinterpret_cast<jlong>(sce_scope_new());
}

JNIEXPORT void JNICALL JNI_LOWERING(freeScope)(JNIEnv *, jclass, jlong handle) {
    sce_scope_free(scopeOf(handle));
}

JNIEXPORT void JNICALL JNI_LOWERING(declare)(JNIEnv *env, jclass, jlong handle, jstring name) {
    Utf8 text(env, name);
    if (text.get() == nullptr) {
        return;
    }
    sce_scope_declare(scopeOf(handle), text.get());
}

JNIEXPORT void JNICALL JNI_LOWERING(declareChunk)(JNIEnv *env, jclass, jlong handle, jstring source) {
    Utf8 text(env, source);
    if (text.get() == nullptr) {
        return;
    }
    sce_scope_declare_chunk(scopeOf(handle), text.get());
}

JNIEXPORT jstring JNICALL JNI_LOWERING(lowerValue)(JNIEnv *env, jclass, jlong handle, jstring source) {
    Utf8 text(env, source);
    if (text.get() == nullptr) {
        return nullptr;
    }
    return adopt(env, sce_lower_value(text.get(), scopeOf(handle)));
}

JNIEXPORT jstring JNICALL JNI_LOWERING(lowerCondition)(JNIEnv *env, jclass, jlong handle, jstring source) {
    Utf8 text(env, source);
    if (text.get() == nullptr) {
        return nullptr;
    }
    return adopt(env, sce_lower_condition(text.get(), scopeOf(handle)));
}

JNIEXPORT jstring JNICALL JNI_LOWERING(lowerScript)(JNIEnv *env, jclass, jlong handle, jstring source) {
    Utf8 text(env, source);
    if (text.get() == nullptr) {
        return nullptr;
    }
    return adopt(env, sce_lower_script(text.get(), scopeOf(handle)));
}

JNIEXPORT jstring JNICALL JNI_LOWERING(lowerLocation)(JNIEnv *env, jclass, jstring source) {
    Utf8 text(env, source);
    if (text.get() == nullptr) {
        return nullptr;
    }
    return adopt(env, sce_lower_location(text.get()));
}

#undef JNI_LOWERING

}  // extern "C"
