// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/JSEngine.h"
#include "core/LogMacros.h"
#include "scripting/PlatformExecutionHelper.h"
#include "scripting/ScriptResultUtils.h"
#include "scripting/SessionRegistry.h"
#include "common/UniqueIdGenerator.h"
#include "events/EventRaiserRegistry.h"
#include "events/EventRaiserService.h"
#include "events/IEventDispatcher.h"
#include "quickjs.h"
#include "runtime/StateMachine.h"
#include "scripting/DOMBinding.h"
#include <chrono>
#include <cmath>
#include <cstring>
#include <iostream>
#include <sstream>

namespace SCE {

// Static instance
JSEngine &JSEngine::instance() {
    static JSEngine instance;
    return instance;
}

JSEngine::JSEngine() {
    SCE_LOG_DEBUG("JSEngine: Starting initialization in constructor...");
    initializeInternal();

    // Initialize EventRaiserService with dependency injection
    initializeEventRaiserService();

    SCE_LOG_DEBUG("JSEngine: Constructor completed - fully initialized");
}

JSEngine::~JSEngine() {
    shutdown();
}

void JSEngine::shutdown() {
    SCE_LOG_DEBUG("JSEngine: shutdown() called");

    if (shouldStop_) {
        SCE_LOG_DEBUG("JSEngine: Already shut down");
        return;
    }

    shouldStop_ = true;

    // W3C SCXML + QuickJS Thread Safety: Destroy sessions BEFORE freeing runtime
    // QuickJS contexts must be freed on the same thread where they were created
    std::vector<std::string> sessionIds;
    {
        std::lock_guard<std::mutex> lock(sessionsMutex_);
        for (const auto &[sessionId, _] : sessions_) {
            sessionIds.push_back(sessionId);
        }
    }

    SCE_LOG_DEBUG("JSEngine: shutdown() - Found {} sessions to clean up", sessionIds.size());

    // Destroy each session via executeAsync
    // WASM: Executes immediately on main thread (synchronous)
    // Native: Executes on worker thread (queued)
    std::vector<std::future<ScriptResult>> futures;
    for (const auto &sessionId : sessionIds) {
        SCE_LOG_DEBUG("JSEngine: shutdown() - Destroying session: {}", sessionId);
        auto future = platformExecutor_->executeAsync([this, sessionId]() {
            destroySessionInternal(sessionId);
            return ScriptResult::createSuccess();
        });
        futures.push_back(std::move(future));
    }

    // Wait for all session cleanup to complete
    for (auto &future : futures) {
        future.get();
    }

    SCE_LOG_DEBUG("JSEngine: shutdown() - All sessions destroyed");

    // Zero Duplication Principle: Platform-specific shutdown logic through Helper
    // Now safe to shutdown worker thread (all QuickJS contexts freed)
    if (platformExecutor_) {
        platformExecutor_->shutdown();
    }

    // §scxml-B-2: Reset DOM class ID before freeing runtime
    DOMBinding::resetClassId();

    // Note: Runtime will be freed by PlatformExecutionHelper (shutdown already called)
    runtime_ = nullptr;

    initialized_ = false;
    SCE_LOG_DEBUG("JSEngine: Shutdown complete");
}

void JSEngine::reset() {
    SCE_LOG_DEBUG("JSEngine: reset() called");

    // W3C SCXML + QuickJS Thread Safety: Destroy sessions on worker thread BEFORE stopping it
    std::vector<std::string> sessionIds;
    {
        std::lock_guard<std::mutex> lock(sessionsMutex_);
        for (const auto &[sessionId, _] : sessions_) {
            sessionIds.push_back(sessionId);
        }
    }

    // Destroy each session via executeAsync (executes on worker thread)
    if (platformExecutor_) {
        std::vector<std::future<ScriptResult>> futures;
        for (const auto &sessionId : sessionIds) {
            auto future = platformExecutor_->executeAsync([this, sessionId]() {
                destroySessionInternal(sessionId);
                return ScriptResult::createSuccess();
            });
            futures.push_back(std::move(future));
        }

        // Wait for all session cleanup to complete on worker thread
        for (auto &future : futures) {
            future.get();
        }

        // Zero Duplication Principle: Platform-specific cleanup logic through Helper
        // Now safe to shutdown worker thread (all QuickJS contexts freed)
        platformExecutor_->shutdown();
        platformExecutor_.reset();  // Release unique_ptr
    }

    // Note: Runtime will be freed by PlatformExecutionHelper (shutdown already called)
    runtime_ = nullptr;

    // Clear global functions
    {
        std::lock_guard<std::mutex> lock(globalFunctionsMutex_);
        globalFunctions_.clear();
    }

    // Clear EventRaiser registry
    clearEventRaiserRegistry();

    // Clear SessionRegistry (invoke mappings, file paths, event dispatchers)
    SessionRegistry::instance().reset();

    // §scxml-B-2: Reset DOM class ID for new QuickJS runtime
    DOMBinding::resetClassId();

    // Reinitialize
    initializeInternal();

    SCE_LOG_DEBUG("JSEngine: reset() completed");
}

void JSEngine::initializeInternal() {
    SCE_LOG_DEBUG("JSEngine: initializeInternal() - Creating platform executor");

    // Zero Duplication Principle: Platform-specific execution logic abstracted through Helper
    // WASM: Synchronous direct execution | Native: Pthread queue execution
    platformExecutor_ = createPlatformExecutor();

    // QuickJS Thread Safety: Wait for executor to create runtime on appropriate thread
    // WASM: Runtime created on main thread (synchronous)
    // Native: Runtime created on worker thread (must wait for initialization)
    platformExecutor_->waitForRuntimeInitialization();
    runtime_ = platformExecutor_->getRuntimePointer();

    if (!runtime_) {
        SCE_LOG_ERROR("JSEngine: Failed to get QuickJS runtime from platform executor");
        return;
    }

    initialized_ = true;
    shouldStop_ = false;

    SCE_LOG_DEBUG("JSEngine: initializeInternal() completed - runtime and executor ready");
}

// === Session Management ===

bool JSEngine::createSession(const std::string &sessionId, const std::string &parentSessionId) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    auto future = platformExecutor_->executeAsync([this, sessionId, parentSessionId]() {
        bool success = createSessionInternal(sessionId, parentSessionId);
        return success ? ScriptResult::createSuccess() : ScriptResult::createError("Failed to create session");
    });
    auto result = future.get();
    return result.isSuccess();
}

bool JSEngine::destroySession(const std::string &sessionId) {
    // Check if JSEngine is already shutdown
    if (shouldStop_.load()) {
        SCE_LOG_DEBUG("JSEngine: Already shutdown, skipping destroySession for: {}", sessionId);
        return true;
    }

    // Zero Duplication Principle: Platform-agnostic execution through Helper
    auto future = platformExecutor_->executeAsync([this, sessionId]() {
        bool success = destroySessionInternal(sessionId);
        return success ? ScriptResult::createSuccess() : ScriptResult::createError("Failed to destroy session");
    });
    auto result = future.get();
    return result.isSuccess();
}

bool JSEngine::hasSession(const std::string &sessionId) const {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    auto future = const_cast<JSEngine *>(this)->platformExecutor_->executeAsync([this, sessionId]() {
        std::lock_guard<std::mutex> lock(sessionsMutex_);
        bool exists = sessions_.find(sessionId) != sessions_.end();
        return exists ? ScriptResult::createSuccess() : ScriptResult::createError("Session not found");
    });
    auto result = future.get();
    return result.isSuccess();
}

std::vector<std::string> JSEngine::getActiveSessions() const {
    // Note: This method doesn't use QuickJS, so no platform executor needed
    // Just read sessions_ map directly
    std::vector<std::string> sessions;
    std::lock_guard<std::mutex> lock(sessionsMutex_);
    for (const auto &[sessionId, _] : sessions_) {
        sessions.push_back(sessionId);
    }
    return sessions;
}

std::string JSEngine::getParentSessionId(const std::string &sessionId) const {
    std::lock_guard<std::mutex> lock(sessionsMutex_);

    auto it = sessions_.find(sessionId);
    if (it != sessions_.end()) {
        return it->second.parentSessionId;
    }

    return "";  // Session not found or no parent
}

// === JavaScript Execution ===

std::future<ScriptResult> JSEngine::executeScript(const std::string &sessionId, const std::string &script) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync(
        [this, sessionId, script]() { return executeScriptInternal(sessionId, script); });
}

std::future<ScriptResult> JSEngine::evaluateExpression(const std::string &sessionId, const std::string &expression) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync(
        [this, sessionId, expression]() { return evaluateExpressionInternal(sessionId, expression); });
}

std::future<ScriptResult> JSEngine::validateExpression(const std::string &sessionId, const std::string &expression) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync(
        [this, sessionId, expression]() { return validateExpressionInternal(sessionId, expression); });
}

std::future<ScriptResult> JSEngine::setVariable(const std::string &sessionId, const std::string &name,
                                            const ScriptValue &value) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync(
        [this, sessionId, name, value]() { return setVariableInternal(sessionId, name, value); });
}

std::future<ScriptResult> JSEngine::setVariableAsDOM(const std::string &sessionId, const std::string &name,
                                                 const std::string &xmlContent) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync([this, sessionId, name, xmlContent]() {
        // §scxml-B-2: Set variable to XML DOM object
        SessionContext *session = getSession(sessionId);
        if (!session || !session->jsContext) {
            return ScriptResult::createError("Session not found");
        }

        JSContext *ctx = session->jsContext;
        ::JSValue domObject = SCE::DOMBinding::createDOMObject(ctx, xmlContent);

        if (JS_IsException(domObject)) {
            return createErrorFromException(ctx);
        }

        ::JSValue global = JS_GetGlobalObject(ctx);
        int setResult = JS_SetPropertyStr(ctx, global, name.c_str(), domObject);
        JS_FreeValue(ctx, global);

        return (setResult == 0) ? ScriptResult::createSuccess() : ScriptResult::createError("Failed to set DOM variable");
    });
}

std::future<ScriptResult> JSEngine::getVariable(const std::string &sessionId, const std::string &name) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync([this, sessionId, name]() { return getVariableInternal(sessionId, name); });
}

std::future<ScriptResult> JSEngine::setCurrentEvent(const std::string &sessionId, const std::shared_ptr<Event> &event) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync(
        [this, sessionId, event]() { return setCurrentEventInternal(sessionId, event); });
}

std::future<ScriptResult> JSEngine::setCurrentEvent(const std::string &sessionId,
                                                    const SetCurrentEventArgs &args) {
    // For AOT engine: Create simple Event object from string parameters
    auto event = std::make_shared<Event>(args.eventName, args.eventType);
    if (!args.eventData.empty()) {
        event->setRawJsonData(args.eventData);
    }
    // §scxml-5.10.1: Set sendid if provided (test332)
    if (!args.sendId.empty()) {
        event->setSendId(args.sendId);
    }
    // §scxml-5.10.1: Set origin if provided (test336)
    if (!args.origin.empty()) {
        event->setOrigin(args.origin);
    }
    // §scxml-5.10.1: Set originType if provided (test352)
    if (!args.originType.empty()) {
        event->setOriginType(args.originType);
    }
    // §scxml-5.10.1: Set invokeid if provided (test338)
    if (!args.invokeId.empty()) {
        event->setInvokeId(args.invokeId);
    }

    // Delegate to Event object version
    return setCurrentEvent(sessionId, event);
}

std::future<ScriptResult> JSEngine::setupSystemVariables(const std::string &sessionId, const std::string &sessionName,
                                                     const std::vector<std::string> &ioProcessors) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync([this, sessionId, sessionName, ioProcessors]() {
        return setupSystemVariablesInternal(sessionId, sessionName, ioProcessors);
    });
}

// === Engine Information ===

std::string JSEngine::getEngineInfo() const {
    return "QuickJS Session-based Engine v1.0";
}

size_t JSEngine::getMemoryUsage() const {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    auto future = const_cast<JSEngine *>(this)->platformExecutor_->executeAsync([this]() {
        if (runtime_) {
            JSMemoryUsage usage;
            JS_ComputeMemoryUsage(runtime_, &usage);
            return ScriptResult::createSuccess(static_cast<int64_t>(usage.memory_used_size));
        }
        return ScriptResult::createSuccess(static_cast<int64_t>(0));
    });

    auto result = future.get();
    if (result.isSuccess() && std::holds_alternative<int64_t>(result.getInternalValue())) {
        return static_cast<size_t>(std::get<int64_t>(result.getInternalValue()));
    }
    return 0;
}

void JSEngine::collectGarbage() {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    auto future = platformExecutor_->executeAsync([this]() {
        if (runtime_) {
            JS_RunGC(runtime_);
        }
        return ScriptResult::createSuccess();
    });

    // Wait for completion but ignore result
    future.get();
}

// === Thread-safe Execution Worker ===

// === Internal Implementation (Part 1) ===

bool JSEngine::createSessionInternal(const std::string &sessionId, const std::string &parentSessionId) {
    // Validate session ID is not empty
    if (sessionId.empty()) {
        SCE_LOG_ERROR("JSEngine: Session ID cannot be empty");
        return false;
    }

    if (sessions_.find(sessionId) != sessions_.end()) {
        SCE_LOG_ERROR("JSEngine: Session already exists: {}", sessionId);
        return false;
    }

    // Runtime is guaranteed to exist in worker thread
    // Create QuickJS context
    JSContext *ctx = JS_NewContext(runtime_);
    if (!ctx) {
        SCE_LOG_ERROR("JSEngine: Failed to create context for session: {}", sessionId);
        return false;
    }

    // Setup context
    if (!setupQuickJSContext(ctx, sessionId)) {
        JS_FreeContext(ctx);
        return false;
    }

    // Create session info
    SessionContext session;
    session.jsContext = ctx;
    session.sessionId = sessionId;
    session.parentSessionId = parentSessionId;

    sessions_[sessionId] = std::move(session);

    // §scxml-6.4: Register parent-child relationship in SessionRegistry
    // Enables engine-agnostic parent session lookup for event routing
    if (!parentSessionId.empty()) {
        SessionRegistry::instance().registerParentChild(sessionId, parentSessionId);
    }

    SCE_LOG_DEBUG("JSEngine: Created session '{}' - sessions_ map size now: {}", sessionId, sessions_.size());
    return true;
}

bool JSEngine::destroySessionInternal(const std::string &sessionId) {
    SCE_LOG_DEBUG("JSEngine: destroySessionInternal() - Destroying session: {}", sessionId);

    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        SCE_LOG_DEBUG("JSEngine: destroySessionInternal() - Session not found: {}", sessionId);
        return false;
    }

    // §scxml-6.4: Unregister parent-child relationship
    SessionRegistry::instance().unregisterParentChild(sessionId);
    // §scxml-6.2: Delegate session cleanup to SessionRegistry
    SessionRegistry::instance().cleanupSession(sessionId);

    if (it->second.jsContext) {
        SCE_LOG_DEBUG("JSEngine: destroySessionInternal() - Freeing JSContext for session: {}", sessionId);
        // Force garbage collection before freeing context
        if (runtime_) {
            JS_RunGC(runtime_);
            SCE_LOG_DEBUG("JSEngine: destroySessionInternal() - GC completed for session: {}", sessionId);
        }
        JS_FreeContext(it->second.jsContext);
        SCE_LOG_DEBUG("JSEngine: destroySessionInternal() - JSContext freed for session: {}", sessionId);
    }

    sessions_.erase(it);
    SCE_LOG_DEBUG("JSEngine: Destroyed session '{}' - sessions_ map size now: {}", sessionId, sessions_.size());

    // Clean up EventRaiser from global registry to prevent memory leaks
    auto registry = getEventRaiserRegistry();
    if (registry && registry->hasEventRaiser(sessionId)) {
        bool unregistered = registry->unregisterEventRaiser(sessionId);
        if (unregistered) {
            SCE_LOG_DEBUG("JSEngine: Cleaned up EventRaiser for destroyed session: {}", sessionId);
        } else {
            SCE_LOG_WARN("JSEngine: Failed to clean up EventRaiser for destroyed session: {}", sessionId);
        }
    }

    // Clean up state query callback to prevent dangling pointer access
    // CRITICAL: AOT state machines register lambda callbacks with [this] capture
    // When state machine is destroyed, callback must be removed to prevent ASAN errors
    {
        std::lock_guard<std::mutex> lock(stateMachinesMutex_);
        auto callbackIt = stateQueryCallbacks_.find(sessionId);
        if (callbackIt != stateQueryCallbacks_.end()) {
            stateQueryCallbacks_.erase(callbackIt);
            SCE_LOG_DEBUG("JSEngine: Cleaned up state query callback for destroyed session: {}", sessionId);
        }
    }

    SCE_LOG_DEBUG("JSEngine: Destroyed session '{}'", sessionId);
    return true;
}

JSEngine::SessionContext *JSEngine::getSession(const std::string &sessionId) {
    auto it = sessions_.find(sessionId);
    return (it != sessions_.end()) ? &it->second : nullptr;
}

bool JSEngine::setupQuickJSContext(JSContext *ctx, const std::string &sessionId) {
    // Set engine instance as context opaque for callbacks
    JS_SetContextOpaque(ctx, this);

    // Setup SCXML-specific builtin functions and objects
    setupSCXMLBuiltins(ctx, sessionId);

    return true;
}

// === SCXML-specific Setup ===

void JSEngine::setupSCXMLBuiltins(JSContext *ctx, [[maybe_unused]] const std::string &sessionId) {
    ::JSValue global = JS_GetGlobalObject(ctx);

    // Setup In() function for state checking
    ::JSValue inFunction = JS_NewCFunction(ctx, inFunctionWrapper, "In", 1);
    JS_SetPropertyStr(ctx, global, "In", inFunction);

    // Setup console object
    setupConsoleObject(ctx);

    // NOTE: QuickJS already has Math object built-in, no need to set it up
    // Removing setupMathObject() improves session creation performance by ~10-15%

    // Setup system variables
    setupSystemVariables(ctx);

    // §scxml-5.10: _event is bound lazily on first event (see JSEngineImpl::setCurrentEventInternal)

    // Bind all registered global functions
    {
        std::lock_guard<std::mutex> lock(globalFunctionsMutex_);
        for (const auto &[name, callback] : globalFunctions_) {
            ::JSValue funcName = JS_NewString(ctx, name.c_str());
            ::JSValue func = JS_NewCFunctionData(ctx, globalFunctionWrapper, -1, 0, 1, &funcName);
            JS_SetPropertyStr(ctx, global, name.c_str(), func);
            JS_FreeValue(ctx, funcName);  // Free the string after using it
            SCE_LOG_DEBUG("JSEngine: Bound registered global function '{}' to JavaScript context", name);
        }
    }

    JS_FreeValue(ctx, global);
}

void JSEngine::setupEventObject(JSContext *ctx, const std::string &sessionId) {
    ::JSValue global = JS_GetGlobalObject(ctx);

    // Register native function for error event queueing (SOLID: Interface Segregation)
    ::JSValue queueErrorFunc = JS_NewCFunction(ctx, queueErrorEventWrapper, "_queueErrorEvent", 2);
    JS_SetPropertyStr(ctx, global, "_queueErrorEvent", queueErrorFunc);

    // Create a SCXML W3C compliant read-only _event object using JavaScript
    // This approach uses Object.defineProperty with getters to enforce read-only behavior
    std::string eventSetupCode = R"(
        (function() {
            var sessionId = ')" + sessionId +
                                 R"(';
            // Global event data object that C++ can access directly
            this.__eventData = {
                name: '',
                type: '',
                sendid: '',
                origin: '',
                origintype: '',
                invokeid: '',
                data: null,
                raw: ''  // W3C SCXML testing extension for event data inspection
            };

            // Create the _event object with read-only properties
            var eventObject = {};
            Object.defineProperty(this, '_event', {
                get: function() { return eventObject; },
                set: function(value) {
                    // SCXML W3C Spec: Attempts to modify system variables should fail
                    console.log('SCE Error: Attempt to assign to read-only system variable _event');
                    // Queue error.execution event per SCXML W3C specification
                    _queueErrorEvent(sessionId, 'error.execution');
                    throw new Error('Cannot assign to read-only system variable _event');
                },
                enumerable: true,
                configurable: false
            });

            // Define each property with getter only to make them read-only
            var eventProps = ['name', 'type', 'sendid', 'origin', 'origintype', 'invokeid', 'data', 'raw'];
            for (var i = 0; i < eventProps.length; i++) {
                (function(propName) {
                    Object.defineProperty(_event, propName, {
                        get: function() { return __eventData[propName]; },
                        set: function(value) {
                            // SCXML W3C Spec: Attempts to modify system variables should fail
                            // and place 'error.execution' on internal event queue
                            console.log('SCE Error: Attempt to modify read-only system variable _event.' + propName);
                            // Queue error.execution event per SCXML W3C specification
                            _queueErrorEvent(sessionId, 'error.execution');
                            throw new Error('Cannot modify read-only system variable _event.' + propName);
                        },
                        enumerable: true,
                        configurable: false
                    });
                })(eventProps[i]);
            }

            // C++ directly accesses __eventData, no helper function needed

            return true;
        }).call(this);
    )";

    ::JSValue result =
        JS_Eval(ctx, eventSetupCode.c_str(), eventSetupCode.length(), "<event_setup>", JS_EVAL_TYPE_GLOBAL);
    if (JS_IsException(result)) {
        SCE_LOG_ERROR("JSEngine: Failed to setup _event object");
        ::JSValue exception = JS_GetException(ctx);
        const char *errorStr = JS_ToCString(ctx, exception);
        if (errorStr) {
            SCE_LOG_ERROR("JSEngine: _event setup error: {}", errorStr);
            JS_FreeCString(ctx, errorStr);
        }
        JS_FreeValue(ctx, exception);
    }
    JS_FreeValue(ctx, result);
    JS_FreeValue(ctx, global);
}

void JSEngine::setupConsoleObject(JSContext *ctx) {
    ::JSValue global = JS_GetGlobalObject(ctx);
    ::JSValue consoleObj = JS_NewObject(ctx);

    // Setup console.log function
    ::JSValue logFunction = JS_NewCFunction(ctx, consoleFunctionWrapper, "log", 1);
    JS_SetPropertyStr(ctx, consoleObj, "log", logFunction);

    // Set console in global scope
    JS_SetPropertyStr(ctx, global, "console", consoleObj);
    JS_FreeValue(ctx, global);
}

void JSEngine::setupMathObject(JSContext *ctx) {
    // Add basic Math object support through JavaScript
    const char *mathCode = R"(
        if (typeof Math === 'undefined') {
            Math = {
                max: function() {
                    var max = arguments[0];
                    for (var i = 1; i < arguments.length; i++) {
                        if (arguments[i] > max) max = arguments[i];
                    }
                    return max;
                },
                min: function() {
                    var min = arguments[0];
                    for (var i = 1; i < arguments.length; i++) {
                        if (arguments[i] < min) min = arguments[i];
                    }
                    return min;
                },
                PI: 3.141592653589793,
                abs: function(x) { return x < 0 ? -x : x; },
                floor: function(x) { return Math.floor ? Math.floor(x) : parseInt(x); },
                ceil: function(x) { return Math.ceil ? Math.ceil(x) : parseInt(x) + (x > parseInt(x) ? 1 : 0); }
            };
        }
    )";

    ::JSValue result = JS_Eval(ctx, mathCode, strlen(mathCode), "<math>", JS_EVAL_TYPE_GLOBAL);
    JS_FreeValue(ctx, result);
}

void JSEngine::setupSystemVariables(JSContext *ctx) {
    ::JSValue global = JS_GetGlobalObject(ctx);

    // Setup _sessionid (unique identifier for this session)
    // In a real implementation, this would be provided by the SCXML engine
    std::string sessionId = "session_" + std::to_string(std::chrono::duration_cast<std::chrono::milliseconds>(
                                                            std::chrono::system_clock::now().time_since_epoch())
                                                            .count());
    JS_SetPropertyStr(ctx, global, "_sessionid", JS_NewString(ctx, sessionId.c_str()));

    // Setup _name (from <scxml> element name attribute)
    JS_SetPropertyStr(ctx, global, "_name", JS_NewString(ctx, "RSMStateMachine"));

    // Setup _ioprocessors (Event I/O Processors)
    ::JSValue ioprocessors = JS_NewObject(ctx);
    JS_SetPropertyStr(ctx, global, "_ioprocessors", ioprocessors);

    JS_FreeValue(ctx, global);
}

// === Static callback functions ===

::JSValue JSEngine::inFunctionWrapper(JSContext *ctx, JSValue /*this_val*/, int argc, JSValue *argv) {
    if (argc != 1) {
        JS_ThrowSyntaxError(ctx, "In() function requires exactly one argument");
        return JS_EXCEPTION;
    }

    // Get the state name argument
    const char *stateName = JS_ToCString(ctx, argv[0]);
    if (!stateName) {
        JS_ThrowTypeError(ctx, "In() function argument must be a string");
        return JS_EXCEPTION;
    }

    // SCXML W3C Section 5.9.2: In() predicate function
    // Recover this-engine via context opaque (set in setupQuickJSContext).
    auto *engine = static_cast<JSEngine *>(JS_GetContextOpaque(ctx));
    if (!engine) {
        JS_FreeCString(ctx, stateName);
        return JS_ThrowInternalError(ctx, "JSEngine instance not bound to context");
    }
    std::string stateNameStr(stateName);
    bool result = engine->checkStateActive(stateNameStr);

    JS_FreeCString(ctx, stateName);
    return JS_NewBool(ctx, result);
}

::JSValue JSEngine::consoleFunctionWrapper(JSContext *ctx, JSValue /*this_val*/, int argc, JSValue *argv) {
    std::stringstream ss;

    for (int i = 0; i < argc; i++) {
        if (i > 0) {
            ss << " ";
        }

        const char *str = JS_ToCString(ctx, argv[i]);
        if (str) {
            ss << str;
            JS_FreeCString(ctx, str);
        } else {
            ss << "[object]";
        }
    }

    // Log to our SCE logging system
    // For now, just print to stderr for testing
    SCE_LOG_INFO("SCE console.log: {}", ss.str());
    return JS_UNDEFINED;
}

::JSValue JSEngine::queueErrorEventWrapper(JSContext *ctx, JSValue /*this_val*/, int argc, JSValue *argv) {
    if (argc < 2) {
        return JS_UNDEFINED;
    }

    // Get sessionId from first argument
    const char *sessionId = JS_ToCString(ctx, argv[0]);
    // Get event name from second argument
    const char *eventName = JS_ToCString(ctx, argv[1]);

    if (sessionId && eventName) {
        // Recover this-engine via context opaque (set in setupQuickJSContext).
        auto *engine = static_cast<JSEngine *>(JS_GetContextOpaque(ctx));
        if (engine) {
            engine->queueInternalEvent(std::string(sessionId), std::string(eventName));
            SCE_LOG_DEBUG("JSEngine: Queued internal event '{}' for session '{}'", eventName, sessionId);
        } else {
            SCE_LOG_ERROR("JSEngine: queueErrorEventWrapper called with no engine bound to JSContext");
        }
    }

    if (sessionId) {
        JS_FreeCString(ctx, sessionId);
    }
    if (eventName) {
        JS_FreeCString(ctx, eventName);
    }

    return JS_UNDEFINED;
}

::JSValue JSEngine::globalFunctionWrapper(JSContext *ctx, [[maybe_unused]] JSValue this_val, int argc, JSValue *argv,
                                          [[maybe_unused]] int magic, JSValue *func_data) {
    // 1. Extract function name from func_data[0]
    const char *funcName = JS_ToCString(ctx, func_data[0]);
    if (!funcName) {
        return JS_ThrowTypeError(ctx, "Invalid function data");
    }

    // 2. Get JSEngine instance and find callback in globalFunctions_ map
    JSEngine *engine = static_cast<JSEngine *>(JS_GetContextOpaque(ctx));
    if (!engine) {
        JS_FreeCString(ctx, funcName);
        return JS_ThrowInternalError(ctx, "Engine instance not found in context");
    }

    std::function<ScriptValue(const std::vector<ScriptValue> &)> callback;
    {
        std::lock_guard<std::mutex> lock(engine->globalFunctionsMutex_);
        auto it = engine->globalFunctions_.find(funcName);
        if (it == engine->globalFunctions_.end()) {
            JS_FreeCString(ctx, funcName);
            return JS_ThrowReferenceError(ctx, "Function not found: %s", funcName);
        }
        callback = it->second;
    }

    SCE_LOG_DEBUG("JSEngine: Calling registered global function: {}", funcName);
    JS_FreeCString(ctx, funcName);

    // 3. Convert JSValue arguments to ScriptValue vector
    std::vector<ScriptValue> args;
    args.reserve(argc);
    for (int i = 0; i < argc; ++i) {
        args.push_back(engine->quickJSToJSValue(ctx, argv[i]));
    }

    // 4. Call C++ callback
    try {
        ScriptValue result = callback(args);

        // 5. Convert ScriptValue result back to JSValue
        return engine->jsValueToQuickJS(ctx, result);
    } catch (const std::exception &e) {
        return JS_ThrowInternalError(ctx, "Global function execution failed: %s", e.what());
    }
}

bool JSEngine::checkStateActive(const std::string &stateName) const {
    std::lock_guard<std::mutex> lock(stateMachinesMutex_);

    // §scxml-5.9.2: In() predicate function
    // First check callback-based state queries (for static AOT engines)
    for (const auto &pair : stateQueryCallbacks_) {
        const auto &callback = pair.second;
        if (callback && callback(stateName)) {
            return true;
        }
    }

    // Fall back to StateMachine pointers (for Interpreter engine)
    // RACE CONDITION FIX: Use weak_ptr::lock() to safely access StateMachine
    // W3C Test 530: Prevents heap-use-after-free during invoke exit
    for (const auto &pair : stateMachines_) {
        if (auto sm = pair.second.lock()) {
            if (sm->isStateActive(stateName)) {
                return true;
            }
        }
    }
    return false;
}

bool JSEngine::registerGlobalFunction(const std::string &functionName,
                                      std::function<ScriptValue(const std::vector<ScriptValue> &)> callback) {
    if (functionName.empty() || !callback) {
        SCE_LOG_ERROR("JSEngine: Invalid function name or callback for global function registration");
        return false;
    }

    std::lock_guard<std::mutex> lock(globalFunctionsMutex_);
    globalFunctions_[functionName] = std::move(callback);

    SCE_LOG_DEBUG("JSEngine: Registered global function: {}", functionName);
    return true;
}

void JSEngine::queueInternalEvent(const std::string &sessionId, const std::string &eventName) {
    std::lock_guard<std::mutex> lock(internalEventQueuesMutex_);

    // Create queue for session if it doesn't exist
    if (internalEventQueues_.find(sessionId) == internalEventQueues_.end()) {
        internalEventQueues_[sessionId] = InternalEventQueue{};
    }

    std::lock_guard<std::mutex> queueLock(*internalEventQueues_[sessionId].mutex);
    internalEventQueues_[sessionId].events.push(eventName);

    SCE_LOG_DEBUG("JSEngine: Queued internal event '{}' for session '{}'", eventName, sessionId);
}

void JSEngine::setStateMachine(std::shared_ptr<StateMachine> stateMachine, const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(stateMachinesMutex_);
    if (stateMachine) {
        stateMachines_[sessionId] = stateMachine;  // weak_ptr assignment from shared_ptr
        SCE_LOG_DEBUG("JSEngine: StateMachine set for session: {}", sessionId);
    } else {
        auto it = stateMachines_.find(sessionId);
        if (it != stateMachines_.end()) {
            stateMachines_.erase(it);
            SCE_LOG_DEBUG("JSEngine: StateMachine removed for session: {}", sessionId);
        }
    }
}

void JSEngine::setStateQueryCallback(StateQueryCallback callback, const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(stateMachinesMutex_);
    if (callback) {
        stateQueryCallbacks_[sessionId] = callback;
        SCE_LOG_DEBUG("JSEngine: State query callback set for session: {}", sessionId);
    } else {
        auto it = stateQueryCallbacks_.find(sessionId);
        if (it != stateQueryCallbacks_.end()) {
            stateQueryCallbacks_.erase(it);
            SCE_LOG_DEBUG("JSEngine: State query callback removed for session: {}", sessionId);
        }
    }
}

JSContext *JSEngine::getContextForBinding(const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(sessionsMutex_);
    SessionContext *session = getSession(sessionId);
    return session ? session->jsContext : nullptr;
}

// Static callback for bindNativeObject-created JS functions
static JSValue bindNativeObjectCallback(JSContext *ctx, [[maybe_unused]] JSValueConst this_val, int argc,
                                        JSValueConst *argv, [[maybe_unused]] int magic, JSValue *func_data) {
    // Retrieve the NativeMethod pointer stored as int64 in func_data[0]
    int64_t ptr = 0;
    JS_ToInt64(ctx, &ptr, func_data[0]);
    auto *method = reinterpret_cast<IScriptEngine::NativeMethod *>(ptr);
    if (!method) {
        return JS_ThrowTypeError(ctx, "Invalid native method binding");
    }

    // Convert JS arguments to ScriptValue vector
    std::vector<ScriptValue> args;
    args.reserve(argc);
    for (int i = 0; i < argc; i++) {
        if (JS_IsBool(argv[i])) {
            args.emplace_back(static_cast<bool>(JS_ToBool(ctx, argv[i])));
        } else if (JS_IsString(argv[i])) {
            const char *str = JS_ToCString(ctx, argv[i]);
            args.emplace_back(std::string(str ? str : ""));
            JS_FreeCString(ctx, str);
        } else if (JS_IsNumber(argv[i])) {
            double dVal = 0;
            JS_ToFloat64(ctx, &dVal, argv[i]);
            // Distinguish integer vs float
            auto iVal = static_cast<int64_t>(dVal);
            if (static_cast<double>(iVal) == dVal && !std::isinf(dVal) && !std::isnan(dVal)) {
                args.emplace_back(iVal);
            } else {
                args.emplace_back(dVal);
            }
        } else if (JS_IsNull(argv[i])) {
            args.emplace_back(ScriptNull{});
        } else {
            args.emplace_back(ScriptUndefined{});
        }
    }

    // Call the native method
    ScriptValue result = (*method)(args);

    // Convert ScriptValue result back to JSValue
    return std::visit(
        [ctx](auto &&v) -> JSValue {
            using VT = std::decay_t<decltype(v)>;
            if constexpr (std::is_same_v<VT, bool>) {
                return JS_NewBool(ctx, v);
            } else if constexpr (std::is_same_v<VT, int64_t>) {
                return JS_NewInt64(ctx, v);
            } else if constexpr (std::is_same_v<VT, double>) {
                return JS_NewFloat64(ctx, v);
            } else if constexpr (std::is_same_v<VT, std::string>) {
                return JS_NewString(ctx, v.c_str());
            } else if constexpr (std::is_same_v<VT, ScriptNull>) {
                return JS_NULL;
            } else {
                return JS_UNDEFINED;
            }
        },
        result);
}

bool JSEngine::bindNativeObject(const std::string &sessionId, const std::string &objectName,
                                const std::vector<std::pair<std::string, NativeMethod>> &methods) {
    std::lock_guard<std::mutex> lock(sessionsMutex_);
    SessionContext *session = getSession(sessionId);
    if (!session || !session->jsContext) {
        SCE_LOG_ERROR("JSEngine::bindNativeObject: Session '{}' not found", sessionId);
        return false;
    }

    JSContext *ctx = session->jsContext;
    JSValue global = JS_GetGlobalObject(ctx);
    JSValue obj = JS_NewObject(ctx);

    if (JS_IsException(obj)) {
        SCE_LOG_ERROR("JSEngine::bindNativeObject: Failed to create object '{}' in session '{}'", objectName, sessionId);
        JS_FreeValue(ctx, global);
        return false;
    }

    for (const auto &[methodName, method] : methods) {
        // Store method with session ownership for lifetime management
        auto methodPtr = std::make_unique<NativeMethod>(method);
        NativeMethod *rawPtr = methodPtr.get();
        session->boundMethods.push_back(std::move(methodPtr));

        // Create JS function wrapping the NativeMethod via func_data pointer
        JSValue ptrVal = JS_NewInt64(ctx, reinterpret_cast<int64_t>(rawPtr));
        JSValue func = JS_NewCFunctionData(ctx, bindNativeObjectCallback, 0, 0, 1, &ptrVal);
        JS_FreeValue(ctx, ptrVal);

        if (JS_IsException(func)) {
            SCE_LOG_ERROR("JSEngine::bindNativeObject: Failed to create function for method '{}' in session '{}'",
                          methodName, sessionId);
            JS_FreeValue(ctx, obj);
            JS_FreeValue(ctx, global);
            return false;
        }

        JS_SetPropertyStr(ctx, obj, methodName.c_str(), func);
    }

    JS_SetPropertyStr(ctx, global, objectName.c_str(), obj);
    JS_FreeValue(ctx, global);

    SCE_LOG_DEBUG("JSEngine::bindNativeObject: Bound object '{}' with {} methods in session '{}'", objectName,
                  methods.size(), sessionId);
    return true;
}

// ===================================================================
// INTEGRATED RESULT PROCESSING IMPLEMENTATION
// ===================================================================

bool JSEngine::resultToBool(const ScriptResult &result) {
    return ScriptResultUtils::resultToBool(result);
}

std::string JSEngine::resultToString(const ScriptResult &result, const std::string &sessionId,
                                     const std::string &originalExpression) {
    return ScriptResultUtils::resultToString(result, &JSEngine::instance(), sessionId, originalExpression);
}

std::vector<std::string> JSEngine::resultToStringArray(const ScriptResult &result, const std::string &sessionId) {
    return ScriptResultUtils::resultToStringArray(result, &JSEngine::instance(), sessionId, "");
}

std::vector<std::string> JSEngine::resultToStringArray(const ScriptResult &result, const std::string &sessionId,
                                                       const std::string &originalExpression) {
    return ScriptResultUtils::resultToStringArray(result, &JSEngine::instance(), sessionId, originalExpression);
}

void JSEngine::requireSuccess(const ScriptResult &result, const std::string &operation) {
    ScriptResultUtils::requireSuccess(result, operation);
}

bool JSEngine::isSuccess(const ScriptResult &result) noexcept {
    return ScriptResultUtils::isSuccess(result);
}

bool JSEngine::hasVariable(const std::string &sessionId, const std::string &variableName) const {
    // §scxml-4.6: Check if variable exists in session scope
    std::string checkExpr;
    auto dotPos = variableName.find('.');
    if (dotPos != std::string::npos) {
        // Dotted path (e.g., "obj.nested.value") — build 'in' operator chain:
        //   'obj' in this && 'nested' in obj && 'value' in obj.nested
        // This correctly distinguishes "property doesn't exist" from "property is undefined".
        std::string root = variableName.substr(0, dotPos);
        checkExpr = "'" + root + "' in this";
        size_t segStart = dotPos + 1;
        while (segStart < variableName.size()) {
            size_t nextDot = variableName.find('.', segStart);
            std::string segment = (nextDot != std::string::npos)
                ? variableName.substr(segStart, nextDot - segStart)
                : variableName.substr(segStart);
            std::string parent = variableName.substr(0, segStart - 1);
            checkExpr += " && '" + segment + "' in " + parent;
            segStart = (nextDot != std::string::npos) ? nextDot + 1 : variableName.size();
        }
    } else {
        checkExpr = "'" + variableName + "' in this";
    }
    auto result = const_cast<JSEngine *>(this)->evaluateExpression(sessionId, checkExpr).get();
    if (result.isSuccess() && std::holds_alternative<bool>(result.getInternalValue())) {
        return std::get<bool>(result.getInternalValue());
    }
    return false;
}

bool JSEngine::isVariablePreInitialized(const std::string &sessionId, const std::string &variableName) const {
    std::lock_guard<std::mutex> lock(sessionsMutex_);
    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        return false;
    }
    return it->second.preInitializedVars.find(variableName) != it->second.preInitializedVars.end();
}

// === Invoke Session Management Implementation ===

void JSEngine::initializeEventRaiserService() {
    try {
        auto registry = std::make_shared<EventRaiserRegistry>();
        EventRaiserService::initialize(registry);
        SCE_LOG_DEBUG("JSEngine: EventRaiserService initialized with dependency injection");
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("JSEngine: Failed to initialize EventRaiserService: {}", e.what());
        throw;
    }
}

std::shared_ptr<IEventRaiserRegistry> JSEngine::getEventRaiserRegistry() {
    // Delegate to EventRaiserService for consistency
    try {
        return EventRaiserService::getInstance().getRegistry();
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("JSEngine: Failed to get EventRaiserRegistry: {}", e.what());
        // Fallback to static creation for backward compatibility
        static std::shared_ptr<IEventRaiserRegistry> fallbackRegistry = std::make_shared<EventRaiserRegistry>();
        return fallbackRegistry;
    }
}

void JSEngine::clearEventRaiserRegistry() {
    // Check if EventRaiserService is initialized before accessing
    // Prevents "Not initialized" exception during cleanup when tests are skipped
    if (!EventRaiserService::isInitialized()) {
        SCE_LOG_DEBUG("JSEngine: EventRaiserService not initialized, skipping registry clear");
        return;
    }

    try {
        EventRaiserService::getInstance().clearAll();
        SCE_LOG_DEBUG("JSEngine: EventRaiser registry cleared via EventRaiserService");
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("JSEngine: Failed to clear EventRaiser registry: {}", e.what());
        // Fallback to old method for backward compatibility
        auto registry = getEventRaiserRegistry();
        if (registry) {
            auto concreteRegistry = std::dynamic_pointer_cast<EventRaiserRegistry>(registry);
            if (concreteRegistry) {
                concreteRegistry->clear();
                SCE_LOG_DEBUG("JSEngine: EventRaiser registry cleared using fallback method");
            }
        }
    }
}

// === Observer Pattern Support (Temporary implementation until Facade refactoring) ===

void JSEngine::addObserver([[maybe_unused]] ISessionObserver *observer) {
    // Temporary implementation - will be delegated to SessionManager after refactoring
    SCE_LOG_DEBUG("JSEngine: Observer support not yet implemented in current architecture");
    // TODO: Delegate to internal SessionManager after Facade pattern implementation
}

void JSEngine::removeObserver([[maybe_unused]] ISessionObserver *observer) {
    // Temporary implementation - will be delegated to SessionManager after refactoring
    SCE_LOG_DEBUG("JSEngine: Observer support not yet implemented in current architecture");
    // TODO: Delegate to internal SessionManager after Facade pattern implementation
}

// JSEngine internal functions are implemented in JSEngineImpl.cpp

}  // namespace SCE
