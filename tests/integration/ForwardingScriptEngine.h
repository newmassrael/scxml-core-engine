// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A script engine that answers every call as another engine answers it: the
// base of the doubles the integration tests drive a machine over. A double
// derives from it and overrides only the calls it changes or counts, so the
// rest of `IScriptEngine` is spelled once, here, and a call the interface
// gains is forwarded by every double through one edit.
//
// Sessions live in the engine forwarded to (the build's, by default), so a
// test reads a machine's datamodel through that engine whichever double the
// machine ran over.

#pragma once

#include "scripting/IScriptEngine.h"
#include "scripting/ScriptEngineProvider.h"

#include <functional>
#include <future>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Tests {

/// Every call of `IScriptEngine`, answered by `inner`.
class ForwardingScriptEngine : public ::SCE::IScriptEngine {
public:
    /// Forwards to the build's engine (`ScriptEngineProvider`).
    ForwardingScriptEngine() : ForwardingScriptEngine(::SCE::ScriptEngineProvider::getScriptEngine()) {}

    explicit ForwardingScriptEngine(::SCE::IScriptEngine &inner) : inner_(inner) {}

    // ── Sessions ────────────────────────────────────────────────────────

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

    // ── The datamodel and the current event ─────────────────────────────

    std::future<::SCE::ScriptResult> setVariable(const std::string &sessionId, const std::string &name,
                                                 const ::ScriptValue &value) override {
        return inner_.setVariable(sessionId, name, value);
    }

    std::future<::SCE::ScriptResult> setVariableAsDOM(const std::string &sessionId, const std::string &name,
                                                      const std::string &xmlContent) override {
        return inner_.setVariableAsDOM(sessionId, name, xmlContent);
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

    std::future<::SCE::SetCurrentEventResult> setCurrentEvent(const std::string &sessionId,
                                                              const std::shared_ptr<::SCE::Event> &event) override {
        return inner_.setCurrentEvent(sessionId, event);
    }

    std::future<::SCE::SetCurrentEventResult> setCurrentEvent(const std::string &sessionId,
                                                              const ::SCE::SetCurrentEventArgs &args) override {
        return inner_.setCurrentEvent(sessionId, args);
    }

    // ── The rest ────────────────────────────────────────────────────────

    ::SCE::ScriptLanguage nativeLanguage() const override {
        return inner_.nativeLanguage();
    }

    bool acceptsLanguage(::SCE::ScriptLanguage language) const override {
        return inner_.acceptsLanguage(language);
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

    // The engine forwarded to is the test program's, shared by every test in
    // it: a double does not stop or clear it.
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
};

}  // namespace SCE::Tests
