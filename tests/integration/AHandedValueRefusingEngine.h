// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The build's script engine with one variable it cannot hold, for the drivers
// of tests/integration/a_cpp_child_takes_a_handed_value_in_place_of_its_data.scxml.
//
// §scxml-6.4 + §scxml-5.3 (docs/adr/0098): a value an invoke hands its child
// is the initial value of the child's top-level <data> of that name, so a
// value the child's session cannot hold is that <data>'s initialization
// error: error.execution, and the variable left unbound. No value the build's
// engine is handed by a document fails to be held, so the double refuses
// `setVariable` of one name and passes everything else to the build's engine,
// which keeps every session. A parent hands its child the engine it was given
// on both C++ channels, so the child's session is asked of the double too.

#pragma once

#include "integration/ForwardingScriptEngine.h"

#include <atomic>
#include <cstddef>
#include <future>
#include <string>
#include <utility>

namespace SCE::Tests {

/// The build's engine, refusing to hold any value under one name.
class AHandedValueRefusingEngine final : public ForwardingScriptEngine {
public:
    explicit AHandedValueRefusingEngine(std::string refusedName) : refusedName_(std::move(refusedName)) {}

    /// How many times a value under the refused name was refused.
    std::size_t refusals() const {
        return refusals_.load();
    }

    std::future<::SCE::ScriptResult> setVariable(const std::string &sessionId, const std::string &name,
                                                 const ::ScriptValue &value) override {
        if (name == refusedName_) {
            ++refusals_;
            std::promise<::SCE::ScriptResult> refused;
            refused.set_value(::SCE::ScriptResult::createError("the double holds no value under '" + name + "'"));
            return refused.get_future();
        }
        return ForwardingScriptEngine::setVariable(sessionId, name, value);
    }

private:
    const std::string refusedName_;
    std::atomic<std::size_t> refusals_{0};
};

}  // namespace SCE::Tests
