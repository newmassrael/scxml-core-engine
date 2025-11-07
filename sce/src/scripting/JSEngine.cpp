#include "scripting/JSEngine.h"
#include "common/Logger.h"
#include "common/PlatformExecutionHelper.h"
#include "common/UniqueIdGenerator.h"
#include "events/EventRaiserRegistry.h"
#include "events/EventRaiserService.h"
#include "events/IEventDispatcher.h"
#include "quickjs.h"
#include "runtime/StateMachine.h"
#include "scripting/DOMBinding.h"
#include <chrono>
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
    LOG_DEBUG("JSEngine: Starting initialization in constructor...");
    initializeInternal();

    // Initialize EventRaiserService with dependency injection
    initializeEventRaiserService();

    LOG_DEBUG("JSEngine: Constructor completed - fully initialized");
}

JSEngine::~JSEngine() {
    shutdown();
}

void JSEngine::shutdown() {
    LOG_DEBUG("JSEngine: shutdown() called");

    if (shouldStop_) {
        LOG_DEBUG("JSEngine: Already shut down");
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

    LOG_DEBUG("JSEngine: shutdown() - Found {} sessions to clean up", sessionIds.size());

    // Destroy each session via executeAsync
    // WASM: Executes immediately on main thread (synchronous)
    // Native: Executes on worker thread (queued)
    std::vector<std::future<JSResult>> futures;
    for (const auto &sessionId : sessionIds) {
        LOG_DEBUG("JSEngine: shutdown() - Destroying session: {}", sessionId);
        auto future = platformExecutor_->executeAsync([this, sessionId]() {
            destroySessionInternal(sessionId);
            return JSResult::createSuccess();
        });
        futures.push_back(std::move(future));
    }

    // Wait for all session cleanup to complete
    for (auto &future : futures) {
        future.get();
    }

    LOG_DEBUG("JSEngine: shutdown() - All sessions destroyed");

    // Zero Duplication Principle: Platform-specific shutdown logic through Helper
    // Now safe to shutdown worker thread (all QuickJS contexts freed)
    if (platformExecutor_) {
        platformExecutor_->shutdown();
    }

    // W3C SCXML B.2: Reset DOM class ID before freeing runtime
    DOMBinding::resetClassId();

    // Note: Runtime will be freed by PlatformExecutionHelper (shutdown already called)
    runtime_ = nullptr;

    initialized_ = false;
    LOG_DEBUG("JSEngine: Shutdown complete");
}

void JSEngine::reset() {
    LOG_DEBUG("JSEngine: reset() called");

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
        std::vector<std::future<JSResult>> futures;
        for (const auto &sessionId : sessionIds) {
            auto future = platformExecutor_->executeAsync([this, sessionId]() {
                destroySessionInternal(sessionId);
                return JSResult::createSuccess();
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

    // W3C SCXML B.2: Reset DOM class ID for new QuickJS runtime
    DOMBinding::resetClassId();

    // Reinitialize
    initializeInternal();

    LOG_DEBUG("JSEngine: reset() completed");
}

void JSEngine::initializeInternal() {
    LOG_DEBUG("JSEngine: initializeInternal() - Creating platform executor");

    // Zero Duplication Principle: Platform-specific execution logic abstracted through Helper
    // WASM: Synchronous direct execution | Native: Pthread queue execution
    platformExecutor_ = createPlatformExecutor();

    // QuickJS Thread Safety: Wait for executor to create runtime on appropriate thread
    // WASM: Runtime created on main thread (synchronous)
    // Native: Runtime created on worker thread (must wait for initialization)
    platformExecutor_->waitForRuntimeInitialization();
    runtime_ = platformExecutor_->getRuntimePointer();

    if (!runtime_) {
        LOG_ERROR("JSEngine: Failed to get QuickJS runtime from platform executor");
        return;
    }

    initialized_ = true;
    shouldStop_ = false;

    LOG_DEBUG("JSEngine: initializeInternal() completed - runtime and executor ready");
}

// === Session Management ===

bool JSEngine::createSession(const std::string &sessionId, const std::string &parentSessionId) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    auto future = platformExecutor_->executeAsync([this, sessionId, parentSessionId]() {
        bool success = createSessionInternal(sessionId, parentSessionId);
        return success ? JSResult::createSuccess() : JSResult::createError("Failed to create session");
    });
    auto result = future.get();
    return result.isSuccess();
}

bool JSEngine::destroySession(const std::string &sessionId) {
    // Check if JSEngine is already shutdown
    if (shouldStop_.load()) {
        LOG_DEBUG("JSEngine: Already shutdown, skipping destroySession for: {}", sessionId);
        return true;
    }

    // Zero Duplication Principle: Platform-agnostic execution through Helper
    auto future = platformExecutor_->executeAsync([this, sessionId]() {
        bool success = destroySessionInternal(sessionId);
        return success ? JSResult::createSuccess() : JSResult::createError("Failed to destroy session");
    });
    auto result = future.get();
    return result.isSuccess();
}

bool JSEngine::hasSession(const std::string &sessionId) const {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    auto future = const_cast<JSEngine *>(this)->platformExecutor_->executeAsync([this, sessionId]() {
        std::lock_guard<std::mutex> lock(sessionsMutex_);
        bool exists = sessions_.find(sessionId) != sessions_.end();
        return exists ? JSResult::createSuccess() : JSResult::createError("Session not found");
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

// === Session ID Generation ===

uint64_t JSEngine::generateSessionId() const {
    // REFACTOR: Use centralized UniqueIdGenerator for consistency
    return UniqueIdGenerator::generateNumericSessionId();
}

std::string JSEngine::generateSessionIdString(const std::string &prefix) const {
    // REFACTOR: Use centralized UniqueIdGenerator instead of duplicate logic
    return UniqueIdGenerator::generateSessionId(prefix);
}

// === Session Cleanup Hooks ===

void JSEngine::registerEventDispatcher(const std::string &sessionId,
                                       std::shared_ptr<IEventDispatcher> eventDispatcher) {
    if (!eventDispatcher) {
        LOG_WARN("JSEngine: Attempted to register null EventDispatcher for session: {}", sessionId);
        return;
    }

    std::lock_guard<std::mutex> lock(eventDispatchersMutex_);
    eventDispatchers_[sessionId] = eventDispatcher;
    LOG_DEBUG("JSEngine: Registered EventDispatcher for session: {}", sessionId);
}

void JSEngine::unregisterEventDispatcher(const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(eventDispatchersMutex_);
    auto it = eventDispatchers_.find(sessionId);
    if (it != eventDispatchers_.end()) {
        eventDispatchers_.erase(it);
        LOG_DEBUG("JSEngine: Unregistered EventDispatcher for session: {}", sessionId);
    }
}

// === JavaScript Execution ===

std::future<JSResult> JSEngine::executeScript(const std::string &sessionId, const std::string &script) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync(
        [this, sessionId, script]() { return executeScriptInternal(sessionId, script); });
}

std::future<JSResult> JSEngine::evaluateExpression(const std::string &sessionId, const std::string &expression) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync(
        [this, sessionId, expression]() { return evaluateExpressionInternal(sessionId, expression); });
}

std::future<JSResult> JSEngine::validateExpression(const std::string &sessionId, const std::string &expression) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync(
        [this, sessionId, expression]() { return validateExpressionInternal(sessionId, expression); });
}

std::future<JSResult> JSEngine::setVariable(const std::string &sessionId, const std::string &name,
                                            const ScriptValue &value) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync(
        [this, sessionId, name, value]() { return setVariableInternal(sessionId, name, value); });
}

std::future<JSResult> JSEngine::setVariableAsDOM(const std::string &sessionId, const std::string &name,
                                                 const std::string &xmlContent) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync([this, sessionId, name, xmlContent]() {
        // W3C SCXML B.2: Set variable to XML DOM object
        SessionContext *session = getSession(sessionId);
        if (!session || !session->jsContext) {
            return JSResult::createError("Session not found");
        }

        JSContext *ctx = session->jsContext;
        ::JSValue domObject = SCE::DOMBinding::createDOMObject(ctx, xmlContent);

        if (JS_IsException(domObject)) {
            return createErrorFromException(ctx);
        }

        ::JSValue global = JS_GetGlobalObject(ctx);
        int setResult = JS_SetPropertyStr(ctx, global, name.c_str(), domObject);
        JS_FreeValue(ctx, global);

        return (setResult == 0) ? JSResult::createSuccess() : JSResult::createError("Failed to set DOM variable");
    });
}

std::future<JSResult> JSEngine::getVariable(const std::string &sessionId, const std::string &name) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync([this, sessionId, name]() { return getVariableInternal(sessionId, name); });
}

std::future<JSResult> JSEngine::setCurrentEvent(const std::string &sessionId, const std::shared_ptr<Event> &event) {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    return platformExecutor_->executeAsync(
        [this, sessionId, event]() { return setCurrentEventInternal(sessionId, event); });
}

std::future<JSResult> JSEngine::setCurrentEvent(const std::string &sessionId, const std::string &eventName,
                                                const std::string &eventData, const std::string &eventType,
                                                const std::string &sendId, const std::string &origin,
                                                const std::string &originType, const std::string &invokeId) {
    // For AOT engine: Create simple Event object from string parameters
    auto event = std::make_shared<Event>(eventName, eventType);
    if (!eventData.empty()) {
        event->setRawJsonData(eventData);
    }
    // W3C SCXML 5.10.1: Set sendid if provided (test332)
    if (!sendId.empty()) {
        event->setSendId(sendId);
    }
    // W3C SCXML 5.10.1: Set origin if provided (test336)
    if (!origin.empty()) {
        event->setOrigin(origin);
    }
    // W3C SCXML 5.10.1: Set originType if provided (test352)
    if (!originType.empty()) {
        event->setOriginType(originType);
    }
    // W3C SCXML 5.10.1: Set invokeid if provided (test338)
    if (!invokeId.empty()) {
        event->setInvokeId(invokeId);
    }

    // Delegate to Event object version
    return setCurrentEvent(sessionId, event);
}

std::future<JSResult> JSEngine::setupSystemVariables(const std::string &sessionId, const std::string &sessionName,
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
            return JSResult::createSuccess(static_cast<int64_t>(usage.memory_used_size));
        }
        return JSResult::createSuccess(static_cast<int64_t>(0));
    });

    auto result = future.get();
    if (result.isSuccess() && std::holds_alternative<int64_t>(result.value_internal)) {
        return static_cast<size_t>(std::get<int64_t>(result.value_internal));
    }
    return 0;
}

void JSEngine::collectGarbage() {
    // Zero Duplication Principle: Platform-agnostic execution through Helper
    auto future = platformExecutor_->executeAsync([this]() {
        if (runtime_) {
            JS_RunGC(runtime_);
        }
        return JSResult::createSuccess();
    });

    // Wait for completion but ignore result
    future.get();
}

// === Thread-safe Execution Worker ===

// === Internal Implementation (Part 1) ===

bool JSEngine::createSessionInternal(const std::string &sessionId, const std::string &parentSessionId) {
    // Validate session ID is not empty
    if (sessionId.empty()) {
        LOG_ERROR("JSEngine: Session ID cannot be empty");
        return false;
    }

    if (sessions_.find(sessionId) != sessions_.end()) {
        LOG_ERROR("JSEngine: Session already exists: {}", sessionId);
        return false;
    }

    // Runtime is guaranteed to exist in worker thread
    // Create QuickJS context
    JSContext *ctx = JS_NewContext(runtime_);
    if (!ctx) {
        LOG_ERROR("JSEngine: Failed to create context for session: {}", sessionId);
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

    LOG_DEBUG("JSEngine: Created session '{}' - sessions_ map size now: {}", sessionId, sessions_.size());
    return true;
}

bool JSEngine::destroySessionInternal(const std::string &sessionId) {
    LOG_DEBUG("JSEngine: destroySessionInternal() - Destroying session: {}", sessionId);

    auto it = sessions_.find(sessionId);
    if (it == sessions_.end()) {
        LOG_DEBUG("JSEngine: destroySessionInternal() - Session not found: {}", sessionId);
        return false;
    }

    // W3C SCXML 6.2: Cancel delayed events for terminating session
    {
        std::lock_guard<std::mutex> lock(eventDispatchersMutex_);
        auto dispatcherIt = eventDispatchers_.find(sessionId);
        if (dispatcherIt != eventDispatchers_.end()) {
            auto eventDispatcher = dispatcherIt->second.lock();
            if (eventDispatcher) {
                size_t cancelledCount = eventDispatcher->cancelEventsForSession(sessionId);
                LOG_DEBUG("JSEngine: Cancelled {} delayed events for session: {}", cancelledCount, sessionId);
            }
            // Remove the registry entry regardless of dispatcher availability
            eventDispatchers_.erase(dispatcherIt);
        }
    }

    // Clean up session file path mapping
    unregisterSessionFilePath(sessionId);

    if (it->second.jsContext) {
        LOG_DEBUG("JSEngine: destroySessionInternal() - Freeing JSContext for session: {}", sessionId);
        // Force garbage collection before freeing context
        if (runtime_) {
            JS_RunGC(runtime_);
            LOG_DEBUG("JSEngine: destroySessionInternal() - GC completed for session: {}", sessionId);
        }
        JS_FreeContext(it->second.jsContext);
        LOG_DEBUG("JSEngine: destroySessionInternal() - JSContext freed for session: {}", sessionId);
    }

    sessions_.erase(it);
    LOG_DEBUG("JSEngine: Destroyed session '{}' - sessions_ map size now: {}", sessionId, sessions_.size());

    // Clean up EventRaiser from global registry to prevent memory leaks
    auto registry = getEventRaiserRegistry();
    if (registry && registry->hasEventRaiser(sessionId)) {
        bool unregistered = registry->unregisterEventRaiser(sessionId);
        if (unregistered) {
            LOG_DEBUG("JSEngine: Cleaned up EventRaiser for destroyed session: {}", sessionId);
        } else {
            LOG_WARN("JSEngine: Failed to clean up EventRaiser for destroyed session: {}", sessionId);
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
            LOG_DEBUG("JSEngine: Cleaned up state query callback for destroyed session: {}", sessionId);
        }
    }

    LOG_DEBUG("JSEngine: Destroyed session '{}'", sessionId);
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

    // W3C SCXML 5.10: _event is bound lazily on first event (see JSEngineImpl::setCurrentEventInternal)

    // Bind all registered global functions
    {
        std::lock_guard<std::mutex> lock(globalFunctionsMutex_);
        for (const auto &[name, callback] : globalFunctions_) {
            ::JSValue funcName = JS_NewString(ctx, name.c_str());
            ::JSValue func = JS_NewCFunctionData(ctx, globalFunctionWrapper, -1, 0, 1, &funcName);
            JS_SetPropertyStr(ctx, global, name.c_str(), func);
            JS_FreeValue(ctx, funcName);  // Free the string after using it
            LOG_DEBUG("JSEngine: Bound registered global function '{}' to JavaScript context", name);
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
                data: null
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
            var eventProps = ['name', 'type', 'sendid', 'origin', 'origintype', 'invokeid', 'data'];
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
        LOG_ERROR("JSEngine: Failed to setup _event object");
        ::JSValue exception = JS_GetException(ctx);
        const char *errorStr = JS_ToCString(ctx, exception);
        if (errorStr) {
            LOG_ERROR("JSEngine: _event setup error: {}", errorStr);
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
    std::string stateNameStr(stateName);
    bool result = JSEngine::instance().checkStateActive(stateNameStr);

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
    LOG_INFO("SCE console.log: {}", ss.str());
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
        // Get JSEngine instance through static access (SOLID: Dependency Inversion)
        JSEngine::instance().queueInternalEvent(std::string(sessionId), std::string(eventName));
        LOG_DEBUG("JSEngine: Queued internal event '{}' for session '{}'", eventName, sessionId);
    }

    if (sessionId) {
        JS_FreeCString(ctx, sessionId);
    }
    if (eventName) {
        JS_FreeCString(ctx, eventName);
    }

    return JS_UNDEFINED;
}

::JSValue JSEngine::globalFunctionWrapper(JSContext *ctx, JSValue this_val, int argc, JSValue *argv, int magic,
                                          JSValue *func_data) {
    (void)this_val;  // Unused parameter
    (void)magic;     // Unused parameter

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

    LOG_DEBUG("JSEngine: Calling registered global function: {}", funcName);
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

    // W3C SCXML 5.9.2: In() predicate function
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
        LOG_ERROR("JSEngine: Invalid function name or callback for global function registration");
        return false;
    }

    std::lock_guard<std::mutex> lock(globalFunctionsMutex_);
    globalFunctions_[functionName] = std::move(callback);

    LOG_DEBUG("JSEngine: Registered global function: {}", functionName);
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

    LOG_DEBUG("JSEngine: Queued internal event '{}' for session '{}'", eventName, sessionId);
}

void JSEngine::setStateMachine(std::shared_ptr<StateMachine> stateMachine, const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(stateMachinesMutex_);
    if (stateMachine) {
        stateMachines_[sessionId] = stateMachine;  // weak_ptr assignment from shared_ptr
        LOG_DEBUG("JSEngine: StateMachine set for session: {}", sessionId);
    } else {
        auto it = stateMachines_.find(sessionId);
        if (it != stateMachines_.end()) {
            stateMachines_.erase(it);
            LOG_DEBUG("JSEngine: StateMachine removed for session: {}", sessionId);
        }
    }
}

void JSEngine::setStateQueryCallback(StateQueryCallback callback, const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(stateMachinesMutex_);
    if (callback) {
        stateQueryCallbacks_[sessionId] = callback;
        LOG_DEBUG("JSEngine: State query callback set for session: {}", sessionId);
    } else {
        auto it = stateQueryCallbacks_.find(sessionId);
        if (it != stateQueryCallbacks_.end()) {
            stateQueryCallbacks_.erase(it);
            LOG_DEBUG("JSEngine: State query callback removed for session: {}", sessionId);
        }
    }
}

JSContext *JSEngine::getContextForBinding(const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(sessionsMutex_);
    SessionContext *session = getSession(sessionId);
    return session ? session->jsContext : nullptr;
}

// ===================================================================
// INTEGRATED RESULT PROCESSING IMPLEMENTATION
// ===================================================================

bool JSEngine::resultToBool(const JSResult &result) {
    if (!result.success_internal) {
        return false;
    }

    // REUSE: Proven boolean conversion logic from ActionExecutorImpl
    if (std::holds_alternative<bool>(result.value_internal)) {
        return std::get<bool>(result.value_internal);
    } else if (std::holds_alternative<int64_t>(result.value_internal)) {
        return std::get<int64_t>(result.value_internal) != 0;
    } else if (std::holds_alternative<double>(result.value_internal)) {
        return std::get<double>(result.value_internal) != 0.0;
    } else if (std::holds_alternative<std::string>(result.value_internal)) {
        return !std::get<std::string>(result.value_internal).empty();
    }
    return false;
}

std::string JSEngine::resultToString(const JSResult &result, const std::string &sessionId,
                                     const std::string &originalExpression) {
    if (!result.success_internal) {
        return "";
    }

    if (std::holds_alternative<std::string>(result.value_internal)) {
        return result.getValue<std::string>();
    } else if (std::holds_alternative<double>(result.value_internal)) {
        double val = result.getValue<double>();
        // Check if it's an integer value
        if (val == std::floor(val)) {
            return std::to_string(static_cast<int64_t>(val));
        } else {
            // SCXML W3C Compliance: Use ECMAScript-compatible number formatting
            std::ostringstream oss;
            oss << std::noshowpoint << val;
            std::string str = oss.str();

            // Remove trailing zeros after decimal point (JavaScript behavior)
            if (str.find('.') != std::string::npos) {
                str.erase(str.find_last_not_of('0') + 1, std::string::npos);
                if (str.back() == '.') {
                    str.pop_back();
                }
            }
            return str;
        }
    } else if (std::holds_alternative<int64_t>(result.value_internal)) {
        return std::to_string(result.getValue<int64_t>());
    } else if (std::holds_alternative<bool>(result.value_internal)) {
        return result.getValue<bool>() ? "true" : "false";
    } else if (!sessionId.empty() && !originalExpression.empty()) {
        // REUSE: Proven JSON.stringify fallback logic
        std::string stringifyExpr = "JSON.stringify(" + originalExpression + ")";
        auto stringifyResult = SCE::JSEngine::instance().evaluateExpression(sessionId, stringifyExpr).get();
        if (stringifyResult.isSuccess()) {
            return stringifyResult.getValue<std::string>();
        }
        return "[object]";
    }
    return "[conversion_error]";
}

std::vector<std::string> JSEngine::resultToStringArray(const JSResult &result, const std::string &sessionId) {
    // SOLID: Delegate to expression-aware version (Single Responsibility)
    return resultToStringArray(result, sessionId, "");
}

std::vector<std::string> JSEngine::resultToStringArray(const JSResult &result, const std::string &sessionId,
                                                       const std::string &originalExpression) {
    std::vector<std::string> arrayValues;

    LOG_DEBUG("resultToStringArray: Starting with sessionId='{}', originalExpression='{}'", sessionId,
              originalExpression);

    if (!result.success_internal) {
        LOG_DEBUG("resultToStringArray: Result not successful, returning empty array");
        return arrayValues;
    }

    std::string arrayStr;

    // SOLID: Handle all ScriptValue types internally (Single Responsibility)
    if (std::holds_alternative<std::string>(result.value_internal)) {
        arrayStr = std::get<std::string>(result.value_internal);
        LOG_DEBUG("resultToStringArray: Got string result: '{}'", arrayStr);
    } else {
        LOG_DEBUG("resultToStringArray: Result is not string type, attempting JSON.stringify conversion");
        // SOLID: For non-string types, convert to JSON string using proven logic
        // This handles array objects, numbers, booleans, etc.
        if (!sessionId.empty() && !originalExpression.empty()) {
            // Use JSON.stringify for reliable array conversion
            std::string stringifyExpr = "JSON.stringify(" + originalExpression + ")";
            LOG_DEBUG("resultToStringArray: Evaluating stringify expression: '{}'", stringifyExpr);
            auto stringifyResult = SCE::JSEngine::instance().evaluateExpression(sessionId, stringifyExpr).get();
            if (stringifyResult.isSuccess() && std::holds_alternative<std::string>(stringifyResult.value_internal)) {
                arrayStr = std::get<std::string>(stringifyResult.value_internal);
                LOG_DEBUG("resultToStringArray: JSON.stringify succeeded, result: '{}'", arrayStr);
            } else {
                LOG_DEBUG("resultToStringArray: JSON.stringify failed or returned non-string");
                return arrayValues;  // Failed to convert to JSON string
            }
        } else {
            LOG_DEBUG("resultToStringArray: Missing sessionId or originalExpression for non-string type");
            return arrayValues;  // Cannot process non-string types without session context
        }
    }

    LOG_DEBUG("resultToStringArray: Final arrayStr before processing: '{}'", arrayStr);

    // SOLID: Use JSON-based approach for reliable array parsing
    // This correctly handles nested arrays like [[1,2],[3,4]] and all JavaScript types
    if (!arrayStr.empty() && !sessionId.empty()) {
        LOG_DEBUG("resultToStringArray: Processing array using JSON approach");

        try {
            // W3C SCXML B.2 (test 457): Validate that value is actually an array
            // Must check instanceof Array before attempting to iterate
            std::string arrayCheckExpr = originalExpression + " instanceof Array";
            LOG_DEBUG("resultToStringArray: Validating array type with expression: '{}'", arrayCheckExpr);
            auto arrayCheckResult = SCE::JSEngine::instance().evaluateExpression(sessionId, arrayCheckExpr).get();

            if (!arrayCheckResult.isSuccess() || !std::holds_alternative<bool>(arrayCheckResult.value_internal) ||
                !std::get<bool>(arrayCheckResult.value_internal)) {
                LOG_DEBUG(
                    "resultToStringArray: Value is not an array (instanceof Array check failed), returning empty");
                return arrayValues;  // Not an array, caller should check and raise error.execution
            }

            // SCXML W3C Compliance: Use original expression to preserve null/undefined distinction
            std::string setVarExpr = "var _tempArray = " + originalExpression + "; _tempArray.length";
            LOG_DEBUG("resultToStringArray: Evaluating temp variable length expression: '{}'", setVarExpr);
            auto lengthResult = SCE::JSEngine::instance().evaluateExpression(sessionId, setVarExpr).get();

            LOG_DEBUG("resultToStringArray: Length result type index: {}", lengthResult.value_internal.index());

            int64_t arrayLength = 0;
            bool lengthValid = false;

            if (lengthResult.isSuccess()) {
                if (std::holds_alternative<int64_t>(lengthResult.value_internal)) {
                    arrayLength = std::get<int64_t>(lengthResult.value_internal);
                    lengthValid = true;
                    LOG_DEBUG("resultToStringArray: Got int64_t array length: {}", arrayLength);
                } else if (std::holds_alternative<double>(lengthResult.value_internal)) {
                    double doubleLength = std::get<double>(lengthResult.value_internal);
                    arrayLength = static_cast<int64_t>(doubleLength);
                    lengthValid = true;
                    LOG_DEBUG("resultToStringArray: Got double array length: {} -> {}", doubleLength, arrayLength);
                }
            }

            if (lengthValid) {
                // Iterate through array elements using temporary variable approach
                for (int64_t i = 0; i < arrayLength; ++i) {
                    // SCXML W3C: Check for undefined first, then use JSON.stringify for other types
                    std::string typeCheckExpr = "typeof _tempArray[" + std::to_string(i) + "]";
                    auto typeResult = SCE::JSEngine::instance().evaluateExpression(sessionId, typeCheckExpr).get();

                    if (typeResult.isSuccess() && std::holds_alternative<std::string>(typeResult.value_internal)) {
                        std::string typeStr = std::get<std::string>(typeResult.value_internal);

                        if (typeStr == "undefined") {
                            // Preserve undefined values exactly
                            arrayValues.push_back("undefined");
                            LOG_DEBUG("resultToStringArray: Element {} is undefined", i);
                            continue;
                        }
                    }

                    // For non-undefined values, use JSON.stringify
                    std::string elementExpr = "JSON.stringify(_tempArray[" + std::to_string(i) + "])";
                    LOG_DEBUG("resultToStringArray: Element {} expression: '{}'", i, elementExpr);
                    auto elementResult = SCE::JSEngine::instance().evaluateExpression(sessionId, elementExpr).get();

                    if (elementResult.isSuccess() &&
                        std::holds_alternative<std::string>(elementResult.value_internal)) {
                        std::string elementStr = std::get<std::string>(elementResult.value_internal);
                        LOG_DEBUG("resultToStringArray: Element {} result: '{}'", i, elementStr);
                        // Remove quotes for primitive types, keep JSON for complex types
                        if (elementStr.length() >= 2 && elementStr.front() == '"' && elementStr.back() == '"') {
                            // String value - remove quotes
                            arrayValues.push_back(elementStr.substr(1, elementStr.length() - 2));
                        } else {
                            // Non-string value (number, boolean, array, object) - keep as JSON
                            arrayValues.push_back(elementStr);
                        }
                    }
                }
            } else {
                LOG_DEBUG("resultToStringArray: Length evaluation failed - success: {}, error: '{}'",
                          lengthResult.isSuccess(),
                          lengthResult.isSuccess() ? "no error" : lengthResult.errorMessage_internal);
            }
        } catch (const std::exception &e) {
            LOG_ERROR("resultToStringArray: Exception during JSON processing: {}", e.what());
        }
    }

    LOG_DEBUG("resultToStringArray: Returning {} elements", arrayValues.size());
    return arrayValues;
}

void JSEngine::requireSuccess(const JSResult &result, const std::string &operation) {
    if (!result.success_internal) {
        throw std::runtime_error("JSEngine operation failed: " + operation + " - " + result.errorMessage_internal);
    }
}

bool JSEngine::isSuccess(const JSResult &result) noexcept {
    return result.success_internal;
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

void JSEngine::registerInvokeMapping(const std::string &parentSessionId, const std::string &invokeId,
                                     const std::string &childSessionId) {
    std::lock_guard<std::mutex> lock(invokeMappingsMutex_);
    invokeMappings_[parentSessionId][invokeId] = childSessionId;
    LOG_DEBUG("JSEngine: Registered invoke mapping - parent: {}, invoke: {}, child: {}", parentSessionId, invokeId,
              childSessionId);
}

std::string JSEngine::getInvokeSessionId(const std::string &parentSessionId, const std::string &invokeId) const {
    std::lock_guard<std::mutex> lock(invokeMappingsMutex_);

    auto parentIt = invokeMappings_.find(parentSessionId);
    if (parentIt == invokeMappings_.end()) {
        LOG_DEBUG("JSEngine: No invoke mappings found for parent session: {}", parentSessionId);
        return "";
    }

    auto invokeIt = parentIt->second.find(invokeId);
    if (invokeIt == parentIt->second.end()) {
        LOG_DEBUG("JSEngine: Invoke ID '{}' not found in parent session: {}", invokeId, parentSessionId);
        return "";
    }

    LOG_DEBUG("JSEngine: Found invoke mapping - parent: {}, invoke: {}, child: {}", parentSessionId, invokeId,
              invokeIt->second);
    return invokeIt->second;
}

void JSEngine::unregisterInvokeMapping(const std::string &parentSessionId, const std::string &invokeId) {
    std::lock_guard<std::mutex> lock(invokeMappingsMutex_);

    auto parentIt = invokeMappings_.find(parentSessionId);
    if (parentIt != invokeMappings_.end()) {
        parentIt->second.erase(invokeId);

        // Clean up empty parent entries
        if (parentIt->second.empty()) {
            invokeMappings_.erase(parentIt);
        }

        LOG_DEBUG("JSEngine: Unregistered invoke mapping - parent: {}, invoke: {}", parentSessionId, invokeId);
    }
}

std::string JSEngine::getInvokeIdForChildSession(const std::string &childSessionId) const {
    std::lock_guard<std::mutex> lock(invokeMappingsMutex_);

    // W3C SCXML 5.10 test 338: Reverse lookup childSessionId -> invokeId
    // Iterate through all parent sessions to find the invokeId that created this child
    for (const auto &parentEntry : invokeMappings_) {
        for (const auto &invokeEntry : parentEntry.second) {
            if (invokeEntry.second == childSessionId) {
                LOG_DEBUG("JSEngine: Found invokeId '{}' for child session '{}' in parent '{}'", invokeEntry.first,
                          childSessionId, parentEntry.first);
                return invokeEntry.first;
            }
        }
    }

    LOG_DEBUG("JSEngine: No invokeId found for child session: {}", childSessionId);
    return "";
}

void JSEngine::registerSessionFilePath(const std::string &sessionId, const std::string &filePath) {
    std::lock_guard<std::mutex> lock(sessionFilePathsMutex_);
    sessionFilePaths_[sessionId] = filePath;
    LOG_DEBUG("JSEngine: Registered session file path - session: {}, path: {}", sessionId, filePath);
}

std::string JSEngine::getSessionFilePath(const std::string &sessionId) const {
    std::lock_guard<std::mutex> lock(sessionFilePathsMutex_);

    auto it = sessionFilePaths_.find(sessionId);
    if (it == sessionFilePaths_.end()) {
        LOG_DEBUG("JSEngine: No file path found for session: {}", sessionId);
        return "";
    }

    LOG_DEBUG("JSEngine: Found session file path - session: {}, path: {}", sessionId, it->second);
    return it->second;
}

void JSEngine::unregisterSessionFilePath(const std::string &sessionId) {
    std::lock_guard<std::mutex> lock(sessionFilePathsMutex_);

    auto it = sessionFilePaths_.find(sessionId);
    if (it != sessionFilePaths_.end()) {
        sessionFilePaths_.erase(it);
        LOG_DEBUG("JSEngine: Unregistered session file path - session: {}", sessionId);
    }
}

void JSEngine::initializeEventRaiserService() {
    try {
        // Create registry and use JSEngine directly as session manager
        auto registry = std::make_shared<EventRaiserRegistry>();

        // JSEngine implements ISessionManager directly - no adapter needed
        EventRaiserService::initialize(registry, std::shared_ptr<ISessionManager>(this, [](ISessionManager *) {
                                           // Custom deleter that does nothing - JSEngine is a singleton
                                       }));

        LOG_DEBUG("JSEngine: EventRaiserService initialized with dependency injection");
    } catch (const std::exception &e) {
        LOG_ERROR("JSEngine: Failed to initialize EventRaiserService: {}", e.what());
        throw;
    }
}

std::shared_ptr<IEventRaiserRegistry> JSEngine::getEventRaiserRegistry() {
    // Delegate to EventRaiserService for consistency
    try {
        return EventRaiserService::getInstance().getRegistry();
    } catch (const std::exception &e) {
        LOG_ERROR("JSEngine: Failed to get EventRaiserRegistry: {}", e.what());
        // Fallback to static creation for backward compatibility
        static std::shared_ptr<IEventRaiserRegistry> fallbackRegistry = std::make_shared<EventRaiserRegistry>();
        return fallbackRegistry;
    }
}

void JSEngine::clearEventRaiserRegistry() {
    // Check if EventRaiserService is initialized before accessing
    // Prevents "Not initialized" exception during cleanup when tests are skipped
    if (!EventRaiserService::isInitialized()) {
        LOG_DEBUG("JSEngine: EventRaiserService not initialized, skipping registry clear");
        return;
    }

    try {
        EventRaiserService::getInstance().clearAll();
        LOG_DEBUG("JSEngine: EventRaiser registry cleared via EventRaiserService");
    } catch (const std::exception &e) {
        LOG_ERROR("JSEngine: Failed to clear EventRaiser registry: {}", e.what());
        // Fallback to old method for backward compatibility
        auto registry = getEventRaiserRegistry();
        if (registry) {
            auto concreteRegistry = std::dynamic_pointer_cast<EventRaiserRegistry>(registry);
            if (concreteRegistry) {
                concreteRegistry->clear();
                LOG_DEBUG("JSEngine: EventRaiser registry cleared using fallback method");
            }
        }
    }
}

// === Observer Pattern Support (Temporary implementation until Facade refactoring) ===

void JSEngine::addObserver(ISessionObserver *observer) {
    // Temporary implementation - will be delegated to SessionManager after refactoring
    (void)observer;  // Suppress unused parameter warning
    LOG_DEBUG("JSEngine: Observer support not yet implemented in current architecture");
    // TODO: Delegate to internal SessionManager after Facade pattern implementation
}

void JSEngine::removeObserver(ISessionObserver *observer) {
    // Temporary implementation - will be delegated to SessionManager after refactoring
    (void)observer;  // Suppress unused parameter warning
    LOG_DEBUG("JSEngine: Observer support not yet implemented in current architecture");
    // TODO: Delegate to internal SessionManager after Facade pattern implementation
}

// JSEngine internal functions are implemented in JSEngineImpl.cpp

}  // namespace SCE
