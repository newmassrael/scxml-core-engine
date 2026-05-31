// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SCXMLTypes.h"
#include "scripting/ISessionLifecycle.h"
#include "scripting/ScriptResult.h"
#include <functional>
#include <future>
#include <memory>
#include <string>
#include <vector>

namespace SCE {

class Event;  // Forward declaration for Event-based setCurrentEvent overload

/**
 * @brief Parameter object for the §scxml-5.10 `setCurrentEvent` boundary.
 *
 * Bundles the seven `_event.*` metadata fields (name + 6 metadata) that every
 * script engine impl must surface before guard evaluation / action execution.
 * The cross-language sibling in `sce-rust-runtime` is `SetCurrentEventArgs`;
 * Kotlin / Python / Go ports mirror the same field set. The `eventType` default
 * follows §scxml-5.10.1 ("internal" for `<raise>`-style events; senders
 * override to "external" / "platform" as needed).
 */
struct SetCurrentEventArgs {
    std::string eventName;
    std::string eventData;
    std::string eventType = "internal";
    std::string sendId;
    std::string origin;
    std::string originType;
    std::string invokeId;
};

/**
 * @brief Script execution engine interface (language-agnostic)
 *
 * Extends ISessionLifecycle with script execution, variable management, and SCXML features.
 * Implementations: QuickJS (JSEngine), future Lua, etc.
 */
class IScriptEngine : public virtual ISessionLifecycle {
public:
    virtual ~IScriptEngine() = default;

    // === Core Script Execution ===

    /**
     * @brief Execute script in the specified session
     * @param sessionId Target session context
     * @param script script code to execute
     * @return Future with execution result
     */
    virtual std::future<ScriptResult> executeScript(const std::string &sessionId, const std::string &script) = 0;

    /**
     * @brief Evaluate expression in the specified session
     * @param sessionId Target session context
     * @param expression expression to evaluate
     * @return Future with evaluation result
     */
    virtual std::future<ScriptResult> evaluateExpression(const std::string &sessionId, const std::string &expression) = 0;

    /**
     * @brief Validate expression syntax without executing
     * @param sessionId Target session context for validation context
     * @param expression expression to validate
     * @return Future with validation result (true if syntax is valid)
     */
    virtual std::future<ScriptResult> validateExpression(const std::string &sessionId, const std::string &expression) = 0;

    // === Variable Management ===

    /**
     * @brief Set a variable in the specified session
     * @param sessionId Target session context
     * @param name Variable name
     * @param value Variable value
     * @return Future indicating success/failure
     */
    virtual std::future<ScriptResult> setVariable(const std::string &sessionId, const std::string &name,
                                              const ScriptValue &value) = 0;

    /**
     * @brief Get a variable from the specified session
     * @param sessionId Target session context
     * @param name Variable name
     * @return Future with variable value or error
     */
    virtual std::future<ScriptResult> getVariable(const std::string &sessionId, const std::string &name) = 0;

    /**
     * @brief Set a variable to an XML DOM object (§scxml-B-2)
     * @param sessionId Target session context
     * @param name Variable name
     * @param xmlContent XML string to parse as DOM
     * @return Future indicating success/failure
     */
    virtual std::future<ScriptResult> setVariableAsDOM(const std::string &sessionId, const std::string &name,
                                                   const std::string &xmlContent) = 0;

    /**
     * @brief Check if a variable exists in the session scope
     * @param sessionId Session identifier
     * @param variableName Variable name to check
     * @return true if variable has been declared (even if value is undefined/nil)
     *
     * §scxml-4.6: Foreach must distinguish between declared and undeclared variables.
     * Engine-agnostic replacement for ECMAScript-specific "'name' in this" check.
     */
    virtual bool hasVariable(const std::string &sessionId, const std::string &variableName) const = 0;

    /**
     * @brief Check if a variable was pre-initialized (set before datamodel initialization)
     * @param sessionId Session identifier
     * @param variableName Variable name to check
     * @return true if variable was pre-initialized (e.g., by invoke data)
     */
    virtual bool isVariablePreInitialized(const std::string &sessionId, const std::string &variableName) const = 0;

    // === SCXML-specific Features ===

    /**
     * @brief Setup SCXML system variables for a session
     * @param sessionId Target session context
     * @param sessionName Human-readable session name
     * @param ioProcessors List of available I/O processors
     * @return Future indicating success/failure
     */
    virtual std::future<ScriptResult> setupSystemVariables(const std::string &sessionId, const std::string &sessionName,
                                                       const std::vector<std::string> &ioProcessors) = 0;

    /**
     * @brief Set current event from Event object (§scxml-5.10)
     * @param sessionId Target session context
     * @param event Event object containing all event fields
     * @return Future indicating success/failure
     */
    virtual std::future<ScriptResult> setCurrentEvent(const std::string &sessionId,
                                                      const std::shared_ptr<Event> &event) = 0;

    /**
     * @brief Set current event from individual fields (§scxml-5.10)
     * @param sessionId Target session context
     * @param args SetCurrentEventArgs bundling eventName + 6 metadata fields
     * @return Future indicating success/failure
     */
    virtual std::future<ScriptResult> setCurrentEvent(const std::string &sessionId,
                                                      const SetCurrentEventArgs &args) = 0;

    // === Global Function Management ===

    /**
     * @brief Register a native function accessible from script
     * @param functionName Name of the function in script context
     * @param callback Native function implementation
     * @return true if registration successful
     */
    virtual bool registerGlobalFunction(const std::string &functionName,
                                        std::function<ScriptValue(const std::vector<ScriptValue> &)> callback) = 0;

    // === Native Object Binding ===

    using NativeMethod = std::function<ScriptValue(const std::vector<ScriptValue> &)>;

    /**
     * @brief Bind a native C++ object as a script-accessible object with methods
     * @param sessionId Session identifier
     * @param objectName Name of the object in script context (e.g., "hardware")
     * @param methods Map of method names to native callbacks
     * @return true if binding successful
     *
     * Creates a script object accessible as objectName.methodName() in the datamodel.
     * Engine-agnostic: JSEngine creates a JS object, LuaEngine creates a Lua table.
     */
    virtual bool bindNativeObject(const std::string &sessionId, const std::string &objectName,
                                  const std::vector<std::pair<std::string, NativeMethod>> &methods) = 0;

    // === Engine Information ===

    /**
     * @brief Get engine name and version information
     * @return Engine information string
     */
    virtual std::string getEngineInfo() const = 0;

    /**
     * @brief Get current memory usage in bytes
     * @return Memory usage in bytes
     */
    virtual size_t getMemoryUsage() const = 0;

    /**
     * @brief Trigger garbage collection
     */
    virtual void collectGarbage() = 0;

    // Session lifecycle (createSession, destroySession, hasSession) inherited from ISessionLifecycle

    // === State Query Callback (§scxml-5.9.1 In() predicate) ===

    using StateQueryCallback = std::function<bool(const std::string &)>;

    /**
     * @brief Set state query callback for In() function integration
     * @param callback Function that checks if a state is active (nullptr to unregister)
     * @param sessionId Session ID to associate with this callback
     */
    virtual void setStateQueryCallback(StateQueryCallback callback, const std::string &sessionId) = 0;

    // === Engine Lifecycle ===

    /**
     * @brief Initialize the script engine
     * @return true if initialization successful
     */
    virtual bool initialize() = 0;

    /**
     * @brief Shutdown the script engine and cleanup all contexts
     */
    virtual void shutdown() = 0;

    /**
     * @brief Check if engine is properly initialized
     * @return true if ready for operations
     */
    virtual bool isInitialized() const = 0;

    /**
     * @brief Reset engine state for test isolation
     *
     * Destroys all sessions, clears registered functions and callbacks,
     * then re-initializes for fresh use. Implementations should also
     * reset shared registries (SessionRegistry, etc.).
     */
    virtual void reset() = 0;
};

}  // namespace SCE