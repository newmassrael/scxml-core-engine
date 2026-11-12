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

#include "scripting/IScriptEngine.h"
#include "scripting/ScriptEngineProvider.h"

#include <atomic>
#include <cstddef>
#include <functional>
#include <future>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Tests {

/// The build's engine, refusing to hold any value under one name.
class AHandedValueRefusingEngine final : public ::SCE::IScriptEngine {
public:
    explicit AHandedValueRefusingEngine(std::string refusedName)
        : inner_(::SCE::ScriptEngineProvider::getScriptEngine()), refusedName_(std::move(refusedName)) {}

    /// How many times a value under the refused name was refused.
    std::size_t refusals() const {
        return refusals_.load();
    }

    // ── The one refusal ─────────────────────────────────────────────────

    std::future<::SCE::ScriptResult> setVariable(const std::string &sessionId, const std::string &name,
                                                 const ::ScriptValue &value) override {
        if (name == refusedName_) {
            ++refusals_;
            std::promise<::SCE::ScriptResult> refused;
            refused.set_value(::SCE::ScriptResult::createError("the double holds no value under '" + name + "'"));
            return refused.get_future();
        }
        return inner_.setVariable(sessionId, name, value);
    }

    // ── The rest, as the build's engine answers it ─────────────────────

    std::optional<::SCE::ScriptSessionFailure> createSessionChecked(const std::string &sessionId,
                                                                    const std::string &parentSessionId = "") override {
        return inner_.createSessionChecked(sessionId, parentSessionId);
    }

    std::future<::SCE::ScriptResult>
    setupSystemVariables(const std::string &sessionId, const std::string &sessionName,
                         const std::vector<::SCE::IOProcessorDescriptor> &ioProcessors) override {
        return inner_.setupSystemVariables(sessionId, sessionName, ioProcessors);
    }

    bool destroySession(const std::string &sessionId) override {
        return inner_.destroySession(sessionId);
    }

    bool hasSession(const std::string &sessionId) const override {
        return inner_.hasSession(sessionId);
    }

    std::future<::SCE::ScriptResult> setVariableAsDOM(const std::string &sessionId, const std::string &name,
                                                      const std::string &xmlContent) override {
        return inner_.setVariableAsDOM(sessionId, name, xmlContent);
    }

    std::future<::SCE::SetCurrentEventResult> setCurrentEvent(const std::string &sessionId,
                                                              const std::shared_ptr<::SCE::Event> &event) override {
        return inner_.setCurrentEvent(sessionId, event);
    }

    std::future<::SCE::SetCurrentEventResult> setCurrentEvent(const std::string &sessionId,
                                                              const ::SCE::SetCurrentEventArgs &args) override {
        return inner_.setCurrentEvent(sessionId, args);
    }

    ::SCE::ScriptLanguage nativeLanguage() const override {
        return inner_.nativeLanguage();
    }

    bool acceptsLanguage(::SCE::ScriptLanguage language) const override {
        return inner_.acceptsLanguage(language);
    }

    std::future<::SCE::ScriptResult> getVariable(const std::string &sessionId, const std::string &name) override {
        return inner_.getVariable(sessionId, name);
    }

    bool hasVariable(const std::string &sessionId, const std::string &variableName) const override {
        return inner_.hasVariable(sessionId, variableName);
    }

    bool isVariablePreInitialized(const std::string &sessionId, const std::string &variableName) const override {
        return inner_.isVariablePreInitialized(sessionId, variableName);
    }

    bool registerGlobalFunction(const std::string &functionName,
                                std::function<::ScriptValue(const std::vector<::ScriptValue> &)> callback) override {
        return inner_.registerGlobalFunction(functionName, std::move(callback));
    }

    bool bindNativeObject(const std::string &sessionId, const std::string &objectName,
                          const std::vector<std::pair<std::string, NativeMethod>> &methods) override {
        return inner_.bindNativeObject(sessionId, objectName, methods);
    }

    std::string getEngineInfo() const override {
        return inner_.getEngineInfo();
    }

    size_t getMemoryUsage() const override {
        return inner_.getMemoryUsage();
    }

    void collectGarbage() override {
        inner_.collectGarbage();
    }

    void setStateQueryCallback(StateQueryCallback callback, const std::string &sessionId) override {
        inner_.setStateQueryCallback(std::move(callback), sessionId);
    }

    bool initialize() override {
        return inner_.initialize();
    }

    bool isInitialized() const override {
        return inner_.isInitialized();
    }

    // The build's engine is the test program's, shared by every test in it:
    // the double does not stop or clear it.
    void shutdown() override {}

    void reset() override {}

protected:
    std::future<::SCE::ScriptResult> doExecuteScript(const std::string &sessionId,
                                                     const ::SCE::ScriptSource &script) override {
        return inner_.executeScript(sessionId, script);
    }

    std::future<::SCE::ScriptResult> doEvaluateExpression(const std::string &sessionId,
                                                          const ::SCE::ScriptSource &expression) override {
        return inner_.evaluateExpression(sessionId, expression);
    }

    std::future<::SCE::ScriptResult> doValidateExpression(const std::string &sessionId,
                                                          const ::SCE::ScriptSource &expression) override {
        return inner_.validateExpression(sessionId, expression);
    }

    std::future<::SCE::ScriptResult> doAssign(const std::string &sessionId, const ::SCE::ScriptSource &location,
                                              const ::SCE::ScriptSource &expr) override {
        return inner_.assign(sessionId, location, expr);
    }

private:
    ::SCE::IScriptEngine &inner_;
    const std::string refusedName_;
    std::atomic<std::size_t> refusals_{0};
};

}  // namespace SCE::Tests
