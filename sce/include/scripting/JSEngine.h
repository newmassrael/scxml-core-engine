// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "../SCXMLTypes.h"
#include "IScriptEngine.h"
#include "ISessionManager.h"
#include "ScriptResult.h"
#include "events/IEventRaiserRegistry.h"
#include "quickjs.h"
#include "runtime/ISessionObserver.h"
#include <atomic>
#include <condition_variable>
#include <future>
#include <memory>
#include <mutex>
#include <queue>
#include <set>
#include <string>
#include <thread>
#include <unordered_map>
#include <unordered_set>
#include <vector>

// Forward declarations for QuickJS
struct JSRuntime;
struct JSContext;

// Note: JSValue is typedef'd in quickjs.h (depending on config: uint64_t or struct JSValue*)
// Do NOT forward declare here as it conflicts with the typedef

// JSValueConst is defined by QuickJS, no need to redefine

namespace SCE {
class StateMachine;

/**
 * @brief Thread-safe session-based JavaScript engine
 *
 * Global singleton that manages multiple isolated JavaScript contexts (sessions).
 * Each session has its own variable space, event context, and system variables.
 * All JavaScript execution happens in a single background thread for QuickJS thread safety.
 */
class JSEngine : public IScriptEngine, public ISessionManager {
public:
    /**
     * @brief Get the global JSEngine instance
     */
    static JSEngine &instance();

    /**
     * @brief Get the global EventRaiserRegistry instance
     * @return Shared pointer to the singleton registry
     */
    static std::shared_ptr<IEventRaiserRegistry> getEventRaiserRegistry();

    /**
     * @brief Clear the global EventRaiserRegistry for test isolation
     *
     * This method should only be used in test environments to ensure
     * clean state between test cases and prevent cross-test interference.
     */
    static void clearEventRaiserRegistry();

    /**
     * @brief Reset the JavaScript engine for test isolation
     * Reinitializes the engine after shutdown, allowing fresh start between tests
     */
    void reset() override;

    /**
     * @brief Shutdown the JavaScript engine and cleanup all sessions
     */
    void shutdown() override;

    // === Lifecycle Management ===

    bool initialize() override { return true; }

    /**
     * @brief Check if engine is properly initialized
     * @return true if ready for operations
     */
    bool isInitialized() const override {
        return initialized_.load();
    }

    // === Session Management ===

    /**
     * @brief Create a new JavaScript session with isolated context
     * @param sessionId Unique identifier for the session
     * @param parentSessionId Optional parent session for hierarchical contexts
     * @return true if session created successfully
     */
    bool createSession(const std::string &sessionId, const std::string &parentSessionId = "") override;

    /**
     * @brief Destroy a JavaScript session and cleanup its context
     * @param sessionId Session to destroy
     * @return true if session destroyed successfully
     */
    bool destroySession(const std::string &sessionId) override;

    // === Observer Pattern Support (ISessionManager extension) ===

    /**
     * @brief Add observer for session lifecycle events
     * @param observer Observer to be notified of session events
     */
    void addObserver(ISessionObserver *observer) override;

    /**
     * @brief Remove observer from session lifecycle events
     * @param observer Observer to be removed
     */
    void removeObserver(ISessionObserver *observer) override;

    /**
     * @brief Check if a session exists
     * @param sessionId Session to check
     * @return true if session exists
     */
    bool hasSession(const std::string &sessionId) const override;

    /**
     * @brief Get list of all active sessions
     * @return Vector of session IDs
     */
    std::vector<std::string> getActiveSessions() const override;

    /**
     * @brief Get parent session ID for a given session
     * @param sessionId Session to get parent for
     * @return Parent session ID or empty string if no parent
     */
    std::string getParentSessionId(const std::string &sessionId) const override;

    // === Thread-safe JavaScript Execution ===

    /**
     * @brief Execute JavaScript script in the specified session
     * @param sessionId Target session
     * @param script JavaScript code to execute
     * @return Future with execution result
     */
    std::future<ScriptResult> executeScript(const std::string &sessionId, const std::string &script) override;

    /**
     * @brief Evaluate JavaScript expression in the specified session
     * @param sessionId Target session
     * @param expression JavaScript expression to evaluate
     * @return Future with evaluation result
     */
    std::future<ScriptResult> evaluateExpression(const std::string &sessionId, const std::string &expression) override;

    // === Session-specific Variable Management ===

    /**
     * @brief Set a variable in the specified session
     * @param sessionId Target session
     * @param name Variable name
     * @param value Variable value
     * @return Future indicating success/failure
     */
    std::future<ScriptResult> setVariable(const std::string &sessionId, const std::string &name, const ScriptValue &value) override;

    /**
     * @brief Set a variable to an XML DOM object (W3C SCXML B.2)
     * @param sessionId Target session
     * @param name Variable name
     * @param xmlContent XML string to parse as DOM
     * @return Future indicating success/failure
     */
    std::future<ScriptResult> setVariableAsDOM(const std::string &sessionId, const std::string &name,
                                           const std::string &xmlContent) override;

    /**
     * @brief Get a variable from the specified session
     * @param sessionId Target session
     * @param name Variable name
     * @return Future with variable value or error
     */
    std::future<ScriptResult> getVariable(const std::string &sessionId, const std::string &name) override;

    /**
     * @brief Check if a variable was pre-initialized (set before datamodel initialization)
     * @param sessionId Session identifier
     * @param variableName Variable name to check
     * @return true if variable was pre-initialized (e.g., by invoke data)
     */
    bool hasVariable(const std::string &sessionId, const std::string &variableName) const override;
    bool isVariablePreInitialized(const std::string &sessionId, const std::string &variableName) const override;

    // === SCXML-specific Features ===

    /**
     * @brief Set current event object in JavaScript context (W3C SCXML 5.10)
     *
     * Overload 1: For Interpreter engine with Event objects
     * @param sessionId Target session
     * @param event Event object with full metadata (name, type, data, sendid, origin, etc.)
     * @return Future indicating success/failure
     */
    std::future<ScriptResult> setCurrentEvent(const std::string &sessionId, const std::shared_ptr<Event> &event) override;

    /**
     * @brief Set current event object in JavaScript context (W3C SCXML 5.10)
     *
     * Overload 2: For AOT engine with string literals
     * @param sessionId Target session
     * @param eventName Event name (compile-time constant)
     * @param eventData Event data as JSON string (compile-time constant)
     * @param eventType Event type (default: "internal")
     * @param sendId Send ID for events triggered by <send> (W3C SCXML 5.10.1, test332)
     * @param origin Origin URL for bidirectional communication (W3C SCXML 5.10.1, test336)
     * @param invokeId Invoke ID for child-to-parent events (W3C SCXML 6.4.1, test338)
     * @return Future indicating success/failure
     */
    std::future<ScriptResult> setCurrentEvent(const std::string &sessionId, const std::string &eventName,
                                          const std::string &eventData = "", const std::string &eventType = "internal",
                                          const std::string &sendId = "", const std::string &origin = "",
                                          const std::string &originType = "", const std::string &invokeId = "") override;

    /**
     * @brief Setup SCXML system variables for a session
     * @param sessionId Target session
     * @param sessionName Human-readable session name
     * @param ioProcessors List of available I/O processors
     * @return Future indicating success/failure
     */
    std::future<ScriptResult> setupSystemVariables(const std::string &sessionId, const std::string &sessionName,
                                               const std::vector<std::string> &ioProcessors) override;

    /**
     * @brief Register a native function accessible from JavaScript
     * @param functionName Name of the function in JavaScript
     * @param callback Native function implementation
     * @return true if registration successful
     */
    bool registerGlobalFunction(const std::string &functionName,
                                std::function<ScriptValue(const std::vector<ScriptValue> &)> callback) override;

    bool bindNativeObject(const std::string &sessionId, const std::string &objectName,
                          const std::vector<std::pair<std::string, NativeMethod>> &methods) override;

    /**
     * @brief Set the StateMachine instance for In() function integration
     * @param stateMachine Pointer to the StateMachine instance
     * @param sessionId Session ID to associate with this state machine
     */
    void setStateMachine(std::shared_ptr<StateMachine> stateMachine, const std::string &sessionId);

    /**
     * @brief Set state query callback for In() function integration (for static engines)
     * @param callback Function that checks if a state is active
     * @param sessionId Session ID to associate with this callback
     */
    void setStateQueryCallback(StateQueryCallback callback, const std::string &sessionId) override;

    /**
     * @brief Get JSContext for C++ object binding (thread-safe)
     *
     * Provides safe access to JSContext for ClassBinding infrastructure.
     * Must be called from the same thread as JSEngine operations.
     *
     * @param sessionId Session identifier
     * @return JSContext pointer or nullptr if session not found
     */
    JSContext *getContextForBinding(const std::string &sessionId);

    // === Engine Information ===

    /**
     * @brief Get engine name and version
     */
    std::string getEngineInfo() const override;

    /**
     * @brief Get current memory usage in bytes
     */
    size_t getMemoryUsage() const override;

    /**
     * @brief Trigger garbage collection
     */
    void collectGarbage() override;

    /**
     * @brief Validate JavaScript expression syntax without executing it
     * @param sessionId Target session for context
     * @param expression JavaScript expression to validate
     * @return Future with validation result (true if syntax is valid)
     */
    std::future<ScriptResult> validateExpression(const std::string &sessionId, const std::string &expression) override;

    // === INTEGRATED RESULT PROCESSING API ===

    /**
     * @brief Convert ScriptResult to boolean with W3C SCXML semantics
     *
     * ARCHITECTURAL DECISION: Integrated directly into JSEngine to eliminate
     * scattered type conversion logic across the codebase.
     *
     * @param result JSEngine execution result
     * @return Boolean value following W3C SCXML conversion rules
     * @throws std::runtime_error on conversion failure
     */
    static bool resultToBool(const ScriptResult &result);

    /**
     * @brief Convert ScriptResult to string with JSON.stringify fallback
     *
     * Integrates the proven conversion logic from ActionExecutorImpl
     * directly into the engine for consistent access.
     *
     * @param result JSEngine execution result
     * @param sessionId Session ID for JSON.stringify evaluation (optional)
     * @param originalExpression Original expression for complex objects (optional)
     * @return String representation or error message
     */
    static std::string resultToString(const ScriptResult &result, const std::string &sessionId = "",
                                      const std::string &originalExpression = "");

    /**
     * @brief Convert ScriptResult to string array for SCXML foreach actions
     *
     * @param result JSEngine evaluation result of array expression
     * @param sessionId Session for additional evaluation if needed
     * @return Vector of string representations
     */
    static std::vector<std::string> resultToStringArray(const ScriptResult &result, const std::string &sessionId);

    /**
     * @brief Convert ScriptResult to string array with original expression context
     *
     * SOLID: Expression-aware version that can re-evaluate with JSON.stringify
     * This maintains Single Responsibility while providing complete functionality
     *
     * @param result JSEngine evaluation result
     * @param sessionId Session for additional evaluation if needed
     * @param originalExpression Original expression for JSON.stringify fallback
     * @return Vector of string representations
     */
    static std::vector<std::string> resultToStringArray(const ScriptResult &result, const std::string &sessionId,
                                                        const std::string &originalExpression);

    /**
     * @brief Extract typed value from ScriptResult safely
     *
     * @tparam T Target type (bool, int64_t, double, std::string)
     * @param result JSEngine execution result
     * @return Optional typed value (nullopt on type mismatch or failure)
     */

    /**
     * @brief Require successful result or throw exception
     *
     * @param result JSEngine result to validate
     * @param operation Operation context for error message
     * @throws std::runtime_error if result indicates failure
     */
    static void requireSuccess(const ScriptResult &result, const std::string &operation);

    /**
     * @brief Check if result represents successful operation
     *
     * @param result JSEngine execution result
     * @return true if operation succeeded
     */
    static bool isSuccess(const ScriptResult &result) noexcept;

    /**
     * @brief Extract typed value from ScriptResult safely (template implementation)
     *
     * @tparam T Target type (bool, int64_t, double, std::string)
     * @param result JSEngine execution result
     * @return Optional typed value (nullopt on type mismatch or failure)
     */
    template <typename T> static std::optional<T> resultToValue(const ScriptResult &result) {
        static_assert(std::is_same_v<T, bool> || std::is_same_v<T, int64_t> || std::is_same_v<T, double> ||
                          std::is_same_v<T, std::string>,
                      "Supported types: bool, int64_t, double, std::string");

        if (!result.isSuccess() || !std::holds_alternative<T>(result.getInternalValue())) {
            return std::nullopt;
        }
        return std::get<T>(result.getInternalValue());
    }

    // === DEPRECATED: Direct result.value access is architecturally forbidden ===
    // Use the resultToXXX methods above instead

private:
    JSEngine();  // Complete initialization in constructor
    ~JSEngine();

    // Non-copyable, non-movable
    JSEngine(const JSEngine &) = delete;
    JSEngine &operator=(const JSEngine &) = delete;
    JSEngine(JSEngine &&) = delete;
    JSEngine &operator=(JSEngine &&) = delete;

    // === Internal Types ===

    struct SessionContext {
        JSContext *jsContext = nullptr;
        std::string sessionId;
        std::string parentSessionId;
        std::shared_ptr<Event> currentEvent;
        std::string sessionName;
        std::vector<std::string> ioProcessors;
        std::unordered_set<std::string>
            preInitializedVars;  // Variables set before datamodel initialization (e.g., invoke data)
        // SOLID: Single Responsibility - session management includes invoke relationships
        std::shared_ptr<class IEventRaiser> eventRaiser;
        // W3C SCXML 5.10: Track _event object initialization for lazy binding
        bool eventObjectInitialized = false;
        // Bound native method storage for bindNativeObject lifetime management
        std::vector<std::unique_ptr<NativeMethod>> boundMethods;
    };

    struct ExecutionRequest {
        enum Type {
            EXECUTE_SCRIPT,
            EVALUATE_EXPRESSION,
            VALIDATE_EXPRESSION,
            SET_VARIABLE,
            GET_VARIABLE,
            SET_CURRENT_EVENT,
            SETUP_SYSTEM_VARIABLES,
            CREATE_SESSION,
            DESTROY_SESSION,
            HAS_SESSION,
            GET_ACTIVE_SESSIONS,
            GET_MEMORY_USAGE,
            COLLECT_GARBAGE,
            SHUTDOWN_ENGINE
        };

        Type type;
        std::string sessionId;
        std::string code;                       // for EXECUTE_SCRIPT, EVALUATE_EXPRESSION
        std::string variableName;               // for SET_VARIABLE, GET_VARIABLE
        ScriptValue variableValue;              // for SET_VARIABLE
        bool isDOMObject = false;               // for SET_VARIABLE: XML DOM object (W3C SCXML B.2)
        std::shared_ptr<Event> event;           // for SET_CURRENT_EVENT
        std::string sessionName;                // for SETUP_SYSTEM_VARIABLES
        std::vector<std::string> ioProcessors;  // for SETUP_SYSTEM_VARIABLES
        std::string parentSessionId;            // for CREATE_SESSION
        std::promise<ScriptResult> promise;

        ExecutionRequest(Type t, const std::string &sid) : type(t), sessionId(sid) {}
    };

    // === QuickJS Management ===
    JSRuntime *runtime_ = nullptr;
    std::unordered_map<std::string, SessionContext> sessions_;
    mutable std::mutex sessionsMutex_;

    // Session registry operations delegated to SessionRegistry singleton
    // (invoke mappings, file paths, event dispatchers moved to SessionRegistry)

    // === Platform-Specific Execution ===
    // Zero Duplication Principle: Platform logic abstracted through PlatformExecutionHelper
    // WASM: Synchronous execution | Native: Pthread queue execution
    // See ARCHITECTURE.md Platform Execution Abstraction
    std::unique_ptr<class PlatformExecutionHelper> platformExecutor_;
    std::atomic<bool> shouldStop_{false};
    std::atomic<bool> initialized_{false};

    // === Global Functions ===
    std::unordered_map<std::string, std::function<ScriptValue(const std::vector<ScriptValue> &)>> globalFunctions_;
    std::mutex globalFunctionsMutex_;
    // === StateMachine Integration ===
    // RACE CONDITION FIX: Use weak_ptr to prevent heap-use-after-free (W3C Test 530)
    // Worker threads can safely check validity with lock() before accessing StateMachine
    std::unordered_map<std::string, std::weak_ptr<StateMachine>> stateMachines_;  // sessionId -> weak_ptr<StateMachine>
    // === StateMachine Integration (Callback-based for static engines) ===
    std::unordered_map<std::string, StateQueryCallback> stateQueryCallbacks_;  // sessionId -> callback
    mutable std::mutex stateMachinesMutex_;

    // === Internal Event System ===
    struct InternalEventQueue {
        std::queue<std::string> events;
        std::unique_ptr<std::mutex> mutex;

        InternalEventQueue() : mutex(std::make_unique<std::mutex>()) {}

        InternalEventQueue(InternalEventQueue &&other) noexcept
            : events(std::move(other.events)), mutex(std::move(other.mutex)) {}

        InternalEventQueue &operator=(InternalEventQueue &&other) noexcept {
            if (this != &other) {
                events = std::move(other.events);
                mutex = std::move(other.mutex);
            }
            return *this;
        }
    };

    std::unordered_map<std::string, InternalEventQueue> internalEventQueues_;  // sessionId -> event queue
    mutable std::mutex internalEventQueuesMutex_;

    // === Internal Methods ===
    void executionWorker();
    void processExecutionRequest(std::unique_ptr<ExecutionRequest> request);
    void initializeInternal();            // Common initialization logic
    void initializeEventRaiserService();  // EventRaiserService initialization

    // QuickJS helpers
    ScriptResult executeScriptInternal(const std::string &sessionId, const std::string &script);
    ScriptResult evaluateExpressionInternal(const std::string &sessionId, const std::string &expression);
    ScriptResult validateExpressionInternal(const std::string &sessionId, const std::string &expression);
    ScriptResult setVariableInternal(const std::string &sessionId, const std::string &name, const ScriptValue &value);
    ScriptResult getVariableInternal(const std::string &sessionId, const std::string &name);
    ScriptResult setCurrentEventInternal(const std::string &sessionId, const std::shared_ptr<Event> &event);
    ScriptResult setupSystemVariablesInternal(const std::string &sessionId, const std::string &sessionName,
                                          const std::vector<std::string> &ioProcessors);

    // Context management
    bool createSessionInternal(const std::string &sessionId, const std::string &parentSessionId);
    bool destroySessionInternal(const std::string &sessionId);
    SessionContext *getSession(const std::string &sessionId);

    // QuickJS setup
    bool setupQuickJSContext(JSContext *ctx, const std::string &sessionId);
    void setupSCXMLBuiltins(JSContext *ctx, const std::string &sessionId);
    void setupEventObject(JSContext *ctx, const std::string &sessionId);
    void setupConsoleObject(JSContext *ctx);
    void setupMathObject(JSContext *ctx);
    void setupSystemVariables(JSContext *ctx);

    // Static callback functions for QuickJS
    static JSValue inFunctionWrapper(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);

    // Helper method for In() function
    bool checkStateActive(const std::string &stateName) const;

    static JSValue consoleFunctionWrapper(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);
    static JSValue queueErrorEventWrapper(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);

    // Global function wrapper for C++ registered functions
    static JSValue globalFunctionWrapper(JSContext *ctx, JSValue this_val, int argc, JSValue *argv, int magic,
                                         JSValue *func_data);

    // SCXML W3C Compliance - Read-only system variables
    void queueInternalEvent(const std::string &sessionId, const std::string &eventName);

    // Type conversion
    ScriptValue quickJSToJSValue(JSContext *ctx, JSValue qjsValue);
    JSValue jsValueToQuickJS(JSContext *ctx, const ScriptValue &value);

    // Error handling
    ScriptResult createErrorFromException(JSContext *ctx);

    // === Internal EventRaiser Management (Private) ===
    /**
     * @brief Register EventRaiser for a session (Internal use only)
     *
     * Enables ParentEventTarget to send events to parent sessions by
     * providing access to their EventRaiser instances.
     * Only accessible by friend classes (StateMachine).
     *
     * @param sessionId Target session
     * @param eventRaiser EventRaiser instance for this session
     */
};

}  // namespace SCE
