// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

/**
 * @brief Engine-agnostic bindObject template implementation
 *
 * Uses GenericClassBinder<T> with IScriptEngine::bindNativeObject() to support
 * any script engine (QuickJS, Lua, etc.) through the ScriptEngineProvider.
 *
 * Include this header explicitly when using StateMachine::bindObject() with class binding API.
 * Not auto-included from StateMachine.h to avoid coupling all users to scripting headers.
 *
 * ClassBinding.h (QuickJS-specific ClassBinder<T>) remains available for direct QuickJS use.
 */

#include "runtime/StateMachine.h"
#include "scripting/GenericClassBinder.h"

namespace SCE {

template <typename T, typename RegisterFunc>
void StateMachine::bindObject(const std::string &name, T *object, RegisterFunc registerMethods) {
    static_assert(std::is_class_v<T>, "Can only bind class objects");

    // Ensure script environment is initialized
    if (!ensureJSEnvironment()) {
        SCE_LOG_ERROR("StateMachine::bindObject: Failed to initialize script environment");
        return;
    }

    // Create engine-agnostic binder and collect methods via callback
    GenericClassBinder<T> binder(name, object);
    registerMethods(binder);

    // Bind through the StateMachine's injected script engine (QuickJS, Lua, etc.)
    if (!scriptEngine_.bindNativeObject(sessionId_, name, binder.getMethods())) {
        SCE_LOG_ERROR("StateMachine::bindObject: Failed to bind object '{}' in session '{}'", name, sessionId_);
    }
}

}  // namespace SCE
