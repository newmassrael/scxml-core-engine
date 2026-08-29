// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SCXMLTypes.h"
#include "common/IOProcessorHelper.h"
// §scxml-B-2-8-1: `PayloadReading` travels out of `setCurrentEvent` below and
// is counted by engines that include no `scripting/` header, so it lives with
// the event metadata rather than here — see the note in that file.
#include "core/PayloadReading.h"
#include "scripting/ISessionLifecycle.h"
#include "scripting/ScriptResult.h"
#include "scripting/ScriptSource.h"
#include <functional>
#include <future>
#include <memory>
#include <string>
#include <vector>

namespace SCE {

class Event;  // Forward declaration for Event-based setCurrentEvent overload

/**
 * @brief What `setCurrentEvent` answers: whether it bound, and which rung.
 *
 * The rung used to be discarded one line after it was decided — the generated
 * Rust binding was literally `let _ = se.set_current_event(...)`. It is
 * returned rather than left to a "ask me afterwards" accessor because an
 * accessor can drift out of step with the binding it describes, and a decision
 * handed back by the function that made it cannot.
 */
struct SetCurrentEventResult {
    ScriptResult status;
    PayloadReading reading = PayloadReading::Absent;
};

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
    //
    // The public entry points are NON-VIRTUAL on purpose. They hold the one
    // contract the language seam adds — an engine handed a language it cannot
    // evaluate REFUSES rather than tries — so that contract lives at a single
    // site instead of being boilerplate a third engine can forget. What an
    // engine implements is the `do*` hook in the protected section below.
    //
    // Each takes a ScriptSource: the text to evaluate, the language that text
    // is written in, and the author's ECMAScript behind it. ONE entry point
    // per operation, with no string overload beside it — a bare string
    // converts implicitly and means the author's own ECMAScript, which is what
    // every call site predating the seam hands over. A second overload would
    // only make a string literal ambiguous between the two readings.
    // See docs/SCE_LUA_TRANSLATION_SEAM.md.

    /**
     * @brief Execute script in the specified session
     * @param sessionId Target session context
     * @param script script code, tagged with the language it is written in
     * @return Future with execution result, or a refusal when this engine does
     *         not evaluate that language
     */
    std::future<ScriptResult> executeScript(const std::string &sessionId, const ScriptSource &script);

    /**
     * @brief Evaluate expression in the specified session
     * @param sessionId Target session context
     * @param expression expression, tagged with the language it is written in
     * @return Future with evaluation result, or a refusal when this engine does
     *         not evaluate that language
     */
    std::future<ScriptResult> evaluateExpression(const std::string &sessionId, const ScriptSource &expression);

    /**
     * @brief Validate expression syntax without executing
     * @param sessionId Target session context for validation context
     * @param expression expression, tagged with the language it is written in
     * @return Future with validation result (true if syntax is valid)
     */
    std::future<ScriptResult> validateExpression(const std::string &sessionId, const ScriptSource &expression);

    // === The language this engine speaks ===

    /**
     * @brief The language this engine evaluates without adapting anything.
     *
     * The engine-side mirror of the manifest's `script_engine_language`: a
     * host that supplied the wrong kind of engine can be told so, instead of
     * discovering it as a syntax error in a language nobody wrote.
     */
    virtual ScriptLanguage nativeLanguage() const = 0;

    /**
     * @brief Whether this engine may be handed text written in @p language.
     *
     * True for `nativeLanguage()` always, and additionally for any language
     * the engine can LOWER into it — `LuaEngine` accepts ECMAScript where
     * `sce-build`'s ECMAScript frontend is linked, and only there. Answering
     * false is what turns a wrong-language call into a refusal rather than an
     * attempt.
     */
    virtual bool acceptsLanguage(ScriptLanguage language) const = 0;

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
     *
     * The descriptors arrive fully resolved from `IOProcessorHelper::build`.
     * An implementation files each one under its name with its location and
     * invents neither, so `_ioprocessors` reads identically whichever engine
     * backs the session.
     *
     * @param sessionId Target session context
     * @param sessionName Human-readable session name
     * @param ioProcessors Entries to publish in `_ioprocessors`
     * @return Future indicating success/failure
     */
    virtual std::future<ScriptResult> setupSystemVariables(const std::string &sessionId, const std::string &sessionName,
                                                           const std::vector<IOProcessorDescriptor> &ioProcessors) = 0;

    /**
     * @brief Set current event from Event object (§scxml-5.10)
     * @param sessionId Target session context
     * @param event Event object containing all event fields
     * @return Future carrying success/failure AND which §scxml-B-2-8-1 rung
     *         the payload got — see `PayloadReading`. An implementation that
     *         binds no payload reports `Absent`; one that cannot tell the
     *         rungs apart must not guess, because a wrong `Undecodable` is a
     *         host chasing a payload that arrived intact.
     */
    virtual std::future<SetCurrentEventResult> setCurrentEvent(const std::string &sessionId,
                                                               const std::shared_ptr<Event> &event) = 0;

    /**
     * @brief Set current event from individual fields (§scxml-5.10)
     * @param sessionId Target session context
     * @param args SetCurrentEventArgs bundling eventName + 6 metadata fields
     * @return Future carrying success/failure AND the §scxml-B-2-8-1 rung.
     */
    virtual std::future<SetCurrentEventResult> setCurrentEvent(const std::string &sessionId,
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

protected:
    // === What an engine implements ===
    //
    // Reached only after the public entry point above has established that
    // this engine accepts `code.language()`, so an implementation may treat
    // that as a precondition rather than re-checking it.

    virtual std::future<ScriptResult> doExecuteScript(const std::string &sessionId, const ScriptSource &script) = 0;

    virtual std::future<ScriptResult> doEvaluateExpression(const std::string &sessionId,
                                                           const ScriptSource &expression) = 0;

    virtual std::future<ScriptResult> doValidateExpression(const std::string &sessionId,
                                                           const ScriptSource &expression) = 0;

    /**
     * @brief The refusal a wrong-language call gets.
     *
     * Names both languages and the author's own text, because the reader who
     * has to act on it is the one who chose the engine — a host that obeyed
     * the manifest's `script_engine_language`, or a build that selected
     * `SCE_SCRIPT_ENGINE`. It is deliberately not phrased as a syntax error:
     * the text is well-formed, it is simply in the other language.
     */
    std::future<ScriptResult> refuseLanguage(const ScriptSource &code) const;
};

}  // namespace SCE