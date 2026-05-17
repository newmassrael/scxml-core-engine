// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "scripting/IScriptEngine.h"
#include "scripting/ScriptEngineProvider.h"

#include <memory>

namespace SCE::Test {

// Inject the build-configured script engine into a generated AOT state
// machine before initialize(). Compiles to no-op when the state machine
// does not need a script engine (the generated Policy advertises this
// via NEEDS_SCRIPT_ENGINE). Lifetime: aliasing shared_ptr with a no-op
// deleter — the engine is owned by ScriptEngineProvider, this view is
// just the handle the generated emit sites require.
template <typename SM>
inline void inject_build_engine(SM &sm) {
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(
            &SCE::ScriptEngineProvider::getScriptEngine(),
            [](SCE::IScriptEngine *) {}));
    }
}

}  // namespace SCE::Test
