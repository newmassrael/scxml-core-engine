#pragma once

/**
 * @brief QuickJS-specific bindObject template implementation
 *
 * Isolated from StateMachine.h to contain QuickJS dependency (JSContext, ClassBinding).
 * Include this header explicitly when using StateMachine::bindObject() with ClassBinder API.
 * Not auto-included from StateMachine.h to avoid coupling all users to JSEngine/QuickJS headers.
 *
 * When replacing QuickJS with another engine (e.g., Lua), only this file needs to change.
 */

#include "scripting/ClassBinding.h"
#include "scripting/JSEngine.h"

namespace SCE {

template <typename T, typename RegisterFunc>
void StateMachine::bindObject(const std::string &name, T *object, RegisterFunc registerMethods) {
    static_assert(std::is_class_v<T>, "Can only bind class objects");

    // Ensure script environment is initialized
    if (!ensureJSEnvironment()) {
        SCE_LOG_ERROR("StateMachine::bindObject: Failed to initialize JS environment");
        return;
    }

    // Get QuickJS context via JSEngine (QuickJS-specific: requires concrete engine access)
    JSContext *ctx = JSEngine::instance().getContextForBinding(sessionId_);
    if (!ctx) {
        SCE_LOG_ERROR("StateMachine::bindObject: Failed to get JSContext for session {}", sessionId_);
        return;
    }

    // Create binder and register methods via callback
    ClassBinder<T> binder(ctx, name, object);
    registerMethods(binder);

    // Finalize and register to JavaScript global object
    JSValue jsObject = binder.finalize();
    JSValue global = JS_GetGlobalObject(ctx);
    JS_SetPropertyStr(ctx, global, name.c_str(), jsObject);  // Takes ownership of jsObject
    JS_FreeValue(ctx, global);

    SCE_LOG_DEBUG("StateMachine::bindObject: Bound object '{}' to JavaScript", name);
}

}  // namespace SCE
