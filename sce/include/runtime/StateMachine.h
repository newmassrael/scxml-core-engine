// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "core/EntrySetHelper.h"
#include "core/HierarchicalStateHelper.h"
#include "core/InvokeHelper.h"  // §scxml-6.4: Shared invoke lifecycle logic (Zero Duplication)
#include "core/LogMacros.h"
#include "events/IEventDispatcher.h"
#include "model/IStateNode.h"
#include "model/SCXMLModel.h"
#include "runtime/DataModelInitializer.h"
#include "runtime/HistoryManager.h"
#include "runtime/HistoryStateAutoRegistrar.h"
#include "runtime/IActionExecutor.h"
#include "runtime/IEventRaiser.h"
#include "runtime/IExecutionContext.h"
#include "runtime/InterpreterDocument.h"
#include "runtime/InvokeExecutor.h"
#include "runtime/StateHierarchyManager.h"
#include "runtime/TransitionDescriptorString.h"
#include "scripting/IScriptEngine.h"
#include <atomic>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <set>
#include <string>
#include <thread>
#include <vector>

namespace SCE {

class StateNode;
class TransitionNode;
class ActionExecutorImpl;  // Forward declaration for cached pointer optimization

/**
 * @brief Dummy policy struct for EntryExitHelper template instantiation
 *
 * EntryExitHelper requires a Policy template parameter for type mapping.
 * For Interpreter engine, no state/event enums are needed (runtime strings).
 * This empty struct satisfies template requirements while enabling Zero Duplication.
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Enables Interpreter to share EntryExitHelper with AOT engine
 * - Helper Pattern: Policy-based template design for cross-engine compatibility
 */
struct InterpreterPolicy {
    // Empty struct - Interpreter uses runtime string-based state/event handling
    // No compile-time enums needed (unlike AOT's generated Policy)
};

/**
 * @brief SCXML-based State Machine Implementation
 *
 * This class provides a complete implementation of SCXML state machine
 * with JavaScript integration for guards, actions, and data model.
 *
 * What it does with a document is W3C SCXML Appendix D: its microstep is
 * `SCE::Core::MicrostepAlgorithms`, the procedure the AOT engine runs, handed
 * this machine's parsed model; its macrostep is the appendix's main event loop.
 */
class StateMachine : public std::enable_shared_from_this<StateMachine> {
public:
    /**
     * @brief State transition result structure
     */
    struct TransitionResult {
        bool success = false;
        std::string fromState;
        std::string toState;
        std::string eventName;
        std::string errorMessage;

        TransitionResult() = default;

        TransitionResult(bool s) : success(s) {}

        TransitionResult(bool s, const std::string &from, const std::string &to, const std::string &event)
            : success(s), fromState(from), toState(to), eventName(event) {}
    };

    /**
     * @brief Constructor with explicit script engine injection
     * @param scriptEngine Script engine for expression evaluation and session management
     * @param sessionId Pre-existing session ID (empty = auto-generate)
     */
    explicit StateMachine(IScriptEngine &scriptEngine, const std::string &sessionId = "");

    /**
     * @brief Destructor
     */

    /**
     * @brief Get invoked child state machines for visualization
     *
     * Returns all active child state machines created via invoke elements.
     * Used by visualization tools to display parent-child hierarchies.
     *
     * @return Vector of child StateMachine shared_ptrs (empty if no children)
     */
    std::vector<std::shared_ptr<StateMachine>> getInvokedChildren();

    /**
     * @brief Set session file path for invoke resolution (WASM)
     *
     * Registers the SCXML file path with JSEngine so invoke elements
     * can resolve relative paths like src="file:child.scxml".
     *
     * @param filePath Absolute path to SCXML file (e.g., "/resources/226/test226.scxml")
     */
    void setSessionFilePath(const std::string &filePath);

    /**
     * @brief Declare the inbound BasicHTTP endpoint serving this session
     *
     * §scxml-C-2-3: the Basic HTTP Event I/O Processor entry of
     * `_ioprocessors` holds the address external components post events to.
     * That address belongs to the deployment — SCE is embedded, and whoever
     * runs the HTTP listener chooses where it listens — so the engine takes it
     * from here rather than guessing one. Leaving it unset means no BasicHTTP
     * endpoint is serving the session, and no entry is published.
     *
     * Must be called before the session starts, since `_ioprocessors` is
     * populated once during session setup.
     *
     * @param accessUri URI external components post events to
     */
    void setBasicHttpAccessUri(const std::string &accessUri);

    ~StateMachine();

    /**
     * @brief Load SCXML document from file
     * @param scxmlFile Path to SCXML file
     * @return true if loaded successfully
     */
    bool loadSCXML(const std::string &scxmlFile);

    /**
     * @brief Load SCXML document from string
     * @param scxmlContent SCXML content as string
     * @return true if loaded successfully
     */
    bool loadSCXMLFromString(const std::string &scxmlContent);

    /**
     * @brief Create StateMachine instance from SCXML string (factory method)
     *
     * §scxml-6.4: Hybrid invoke support - AOT parent creates Interpreter child at runtime
     * ARCHITECTURE.md: Hybrid Strategy - contentexpr evaluates to SCXML string, this creates child
     *
     * @param scxmlContent SCXML content as string
     * @param sessionId Optional session ID (for invoke scenarios)
     * @return Shared pointer to StateMachine instance, or nullptr if parsing failed
     *
     * @example Hybrid Invoke Usage (AOT parent + Interpreter child)
     * @code
     * // AOT parent evaluates contentexpr via JSEngine
     * auto scxmlStr = jsEngine.evaluateExpression(sessionId, "Var1").get().getValue<std::string>();
     *
     * // Create Interpreter child from SCXML string
     * auto child = StateMachine::createFromSCXMLString(scxmlStr);
     * if (child) {
     *     child->setCompletionCallback([&engine]() { engine.raise(Event::Done_invoke); });
     *     child->start();
     * }
     * @endcode
     */
    static std::shared_ptr<StateMachine> createFromSCXMLString(const std::string &scxmlContent,
                                                               const std::string &sessionId = "");

    /**
     * @brief Create from SCXML string with explicit engine injection
     */
    static std::shared_ptr<StateMachine> createFromSCXMLString(IScriptEngine &scriptEngine,
                                                               const std::string &scxmlContent,
                                                               const std::string &sessionId = "");

    /**
     * @brief Load pre-parsed SCXML model directly
     *
     * This method is used by StaticCodeGenerator when dynamic invoke is detected.
     * Instead of re-parsing the SCXML file, the already-parsed model is injected directly.
     * This follows the Zero Duplication principle (ARCHITECTURE.md).
     *
     * @param model Pre-parsed SCXML model
     * @return true if loaded successfully
     */
    bool loadModel(std::shared_ptr<SCXMLModel> model);

    /**
     * @brief Start the state machine
     *
     * Enters the initial configuration and runs the macrostep it starts to
     * completion, invokes included.
     *
     * @param autoProcessQueuedEvents If true (default), internal events complete
     * every macrostep, and the main event loop takes external events until the
     * external queue is empty — the ones the initial configuration queued
     * included — before returning; `processEvent` does the same. If false
     * (interactive mode), queued events remain for the host to step through
     * one at a time.
     * @return true if started successfully
     */
    bool start(bool autoProcessQueuedEvents = true);

    /**
     * @brief Stop the state machine
     */
    void stop();

    /**
     * @brief Process an event a host is handing to this machine
     *
     * §scxml-3.12 makes this an external event, so W3C SCXML Appendix D's
     * preliminary step — the `<finalize>` for the invoke it came from, and the
     * autoforward copy to every child that asked for one — runs before
     * transitions are selected for it, exactly as it would had the event come
     * off the external queue. Callers that are themselves the event raiser
     * dispatching a queued event want `processDispatchedEvent` instead: the
     * context is already established there, and an internal event that came
     * through here would be forwarded to children that must never see it.
     *
     * @param eventName Name of the event to process
     * @param eventData Optional event data (JSON string)
     * @return Transition result
     */
    TransitionResult processEvent(const std::string &eventName, const std::string &eventData = "");

    /**
     * @brief Process an event the EventRaiser has already dequeued
     *
     * The entry point for raiser callbacks: it reads the queue category, the
     * origin session and the rest of the §scxml-5.10 metadata from the
     * thread-local context the raiser established around this dispatch,
     * instead of declaring the event external the way `processEvent` does. An
     * internal event reaching the machine this way is therefore kept out of
     * the autoforward set by the queue it was raised onto, which is the
     * distinction W3C SCXML Appendix D draws and not one the event's name
     * could express.
     *
     * @param eventName Name of the event to process
     * @param eventData Optional event data (JSON string)
     * @return Transition result
     */
    TransitionResult processDispatchedEvent(const std::string &eventName, const std::string &eventData = "");

    /**
     * @brief Process an event with origin tracking for W3C SCXML finalize support
     *
     * The event, and the macrostep it starts. In auto mode the main event
     * loop then goes on to the external queue, one event and one macrostep at
     * a time, until that queue is empty or the machine has stopped — so what
     * the event's macrostep sent this session comes back before the call
     * does. In interactive mode the queue is left for the host to step.
     *
     * Handed to a machine whose macrostep is already running on this thread,
     * the event is put on its queue instead — Appendix D processes an event
     * only where its main event loop takes one — and the result says so.
     *
     * @param eventName Name of the event to process
     * @param eventData Optional event data (JSON string)
     * @param originSessionId Session ID that originated this event (for finalize)
     * @param sendId Send ID from failed send element (for error events)
     * @param invokeId Invoke ID from invoked child process (test 338)
     * @return Transition result
     */
    TransitionResult processEvent(const std::string &eventName, const std::string &eventData,
                                  const std::string &originSessionId, const std::string &sendId = "",
                                  const std::string &invokeId = "", const std::string &originType = "");

    /**
     * @brief Get current state ID
     * @return Current state ID, empty if not started
     */
    std::string getCurrentState() const;

    /**
     * @brief Get all currently active states (for hierarchical states)
     * @return Vector of active state IDs
     */
    std::vector<std::string> getActiveStates() const;

    /**
     * @brief Check if state machine is running
     * @return true if running
     */
    bool isRunning() const;

    /**
     * @brief Check if a state is currently active
     * @param stateId State ID to check
     * @return true if state is active
     */
    bool isStateActive(const std::string &stateId) const;

    /**
     * @brief Check if the state machine is currently in a final state
     * @return true if current state is a final state
     */
    bool isInFinalState() const;

    /**
     * @brief Get source state of last executed transition
     *
     * Interactive visualizer support for transition tracking
     * Returns the source state of the most recently executed transition,
     * including eventless transitions.
     *
     * @return Source state ID (empty if no transition executed yet)
     */
    std::string getLastTransitionSource() const;

    /**
     * Position of the transition that fired within its source state's
     * transition list, or -1 when it is not known.
     *
     * The document order `ITransitionNode` is parsed in, so a consumer that
     * walks `state->getTransitions()` — as the visualizer's structure
     * builder does — indexes the same list.
     */
    int getLastTransitionIndex() const;

    /**
     * @brief Get target state of last executed transition
     *
     * Interactive visualizer support for transition tracking
     * Returns the target state of the most recently executed transition,
     * including eventless transitions.
     *
     * @return Target state ID (empty if no transition executed yet)
     */
    std::string getLastTransitionTarget() const;

    /**
     * @brief Get enabled transitions from last event processing
     *
     * §scxml-D-removeConflictingTransitions: Returns all transitions that were enabled before conflict resolution.
     * For parallel states, this includes transitions from all regions that could fire for the event.
     * Interactive visualizer support for showing transition conflict resolution process.
     *
     * @return Vector of enabled transition descriptors (empty if no parallel state processing occurred)
     */
    std::vector<TransitionDescriptorString> getLastEnabledTransitions() const;

    /**
     * @brief Get optimal transition set after conflict resolution
     *
     * §scxml-D-removeConflictingTransitions: Returns transitions selected after applying conflict resolution algorithm.
     * These are the transitions that were actually executed in the last microstep.
     * Interactive visualizer support for showing which transitions were chosen.
     *
     * @return Vector of optimal transition descriptors (empty if no conflict resolution occurred)
     */
    std::vector<TransitionDescriptorString> getLastOptimalTransitions() const;

    /**
     * @brief Restore active states directly without executing onentry actions
     *
     * Time-travel debugging support for InteractiveTestRunner
     * Restores state configuration from snapshot without side effects.
     *
     * ARCHITECTURE.md: Zero Duplication - Uses StateHierarchyManager infrastructure
     * Only for debugging/visualization - NOT for production state machine execution
     *
     * @param states Vector of state IDs to activate (document order preserved)
     */
    void restoreActiveStatesDirectly(const std::vector<std::string> &states);

    /**
     * @brief Check if the initial state of the SCXML model is a final state
     * @return true if the initial state is a final state
     */
    bool isInitialStateFinal() const;

    /**
     * @brief Bind C++ object for script engine access (engine-agnostic via GenericClassBinder)
     * @param name Object name in script scope
     * @param object Pointer to C++ object (must outlive the script engine session)
     * @param registerMethods Callback to register methods via GenericClassBinder
     *
     * Example:
     * @code
     * Hardware hw;
     * sm.bindObject("hardware", &hw, [](auto& binder) {
     *     binder.def("getTemperature", &Hardware::getTemperature)
     *           .def("setTemperature", &Hardware::setTemperature);
     * });
     * @endcode
     */
    template <typename T, typename RegisterFunc>
    void bindObject(const std::string &name, T *object, RegisterFunc registerMethods);

    /**
     * @brief Get current event data (accessible from guards/actions)
     * @return Current event data as JSON string
     */
    std::string getCurrentEventData() const;

    /**
     * @brief Get session ID for SCXML data model access
     * @return Current session ID
     */
    const std::string &getSessionId() const;

    /**
     * @brief Get SCXML model
     * @return SCXML model pointer
     */
    std::shared_ptr<SCXMLModel> getModel() const;

    /**
     * @brief Get state machine statistics
     */
    struct Statistics {
        int totalTransitions = 0;
        int totalEvents = 0;
        int failedTransitions = 0;
        std::string currentState;
        bool isRunning = false;
        /// §scxml-3.12.2: `error.*` events the raiser refused because an error
        /// handler kept raising them. The clause bounds what happens to an
        /// error nobody answers; this is the one a handler answers with the
        /// same failure every time, which nothing in the specification bounds
        /// and which used to mean `processEvent` never came back.
        uint32_t errorCascadeEvents = 0;
        /// The most recent event that count refused, empty while it is zero.
        std::string lastErrorCascadeEvent;
        /// Macrosteps this engine stopped short because their chain was still
        /// going after `MAX_MACROSTEP_MICROSTEPS` microsteps — an eventless
        /// transition still enabled, or an event still on the internal queue.
        /// The clause promises a stable configuration at the end of a
        /// macrostep, and the specification's Principles and Constraints add
        /// that such an end need not exist ("A macrostep may not [terminate]
        /// ... This is currently allowed"). When this count moves, the
        /// configuration `currentState` reports is not a stable one — and
        /// nothing else in this struct says so.
        uint32_t truncatedMacrosteps = 0;
        /// The state the drain was in when that last happened, empty while the
        /// count is zero. One state on the eventless cycle that could not
        /// settle, which is where an author looks first.
        std::string lastTruncatedMacrostepState;
        /// §scxml-B-2-8-1: events delivered with a payload that announced
        /// itself as structure and that the datamodel could not read as one.
        ///
        /// The clause requires the fallback — content the processor cannot
        /// interpret becomes a space-normalized string — and says nothing
        /// about telling anyone. So the document reads `_event.data.field`,
        /// gets nothing, assigns nothing, and the run carries on. Measured
        /// 2026-08-22 on three independent Lua implementations: a payload in
        /// Lua's own table syntax emptied every variable the receiving
        /// transition assigned, including the one that primed the next
        /// session, and no gate anywhere went red.
        ///
        /// Counts only the reading a host can act on. Prose arriving as text
        /// is the ladder working (W3C test 562) and is not counted, because a
        /// diagnostic that fires when nothing is wrong is one nobody reads.
        uint32_t undecodablePayloads = 0;
        /// The name of the most recent event that count refused to read,
        /// empty while it is zero. A count says something was lost; this says
        /// which delivery lost it.
        std::string lastUndecodablePayloadEvent;
        /// §scxml-3.13: external events this machine never looked at — handed
        /// to it after it had stopped, or still on its external queue when it
        /// did.
        ///
        /// Appendix D's main event loop exits when the machine reaches a
        /// top-level final state, and the clause is explicit that the
        /// interpreter is then done. Refusing the event is correct; being
        /// unable to say it happened is what this counts. The queue is the
        /// second place it happens, and not a door: the loop checks for the
        /// final state before it dequeues again, so whatever was waiting there
        /// is never taken. The AOT engine counts both the same way.
        ///
        /// This engine already tells the caller — `processEvent` returns a
        /// `TransitionResult` whose `success` is false and whose
        /// `errorMessage` names the reason. The count exists because the six
        /// generated engines have no return value to carry it, and a host
        /// polling statistics must read the same fact on every backend.
        ///
        /// It is the count that separates the third explanation from the
        /// other two. A host that sent an event and saw nothing move has
        /// three candidates: it was dequeued and matched nothing
        /// (`failedTransitions` / the AOT `discardedExternalEvents`), it was
        /// dequeued and a transition's guard was false (nothing moves), or it
        /// was never dequeued at all (this).
        uint32_t unseenExternalEvents = 0;
        /// The name of the most recent event that count recorded, empty while
        /// it is zero.
        std::string lastUnseenEventName;
    };

    Statistics getStatistics() const;

    /**
     * @brief Register a history state for tracking
     * @param historyStateId ID of the history state
     * @param parentStateId ID of the parent compound state
     * @param type History type (SHALLOW or DEEP)
     * @param defaultStateId Default state if no history available
     * @return true if registration succeeded
     */
    bool registerHistoryState(const std::string &historyStateId, const std::string &parentStateId, HistoryType type,
                              const std::string &defaultStateId = "");

    /**
     * @brief Check if a state ID represents a history state
     * @param stateId State ID to check
     * @return true if it's a history state
     */
    bool isHistoryState(const std::string &stateId) const;

    /**
     * @brief Clear all recorded history (for testing/reset purposes)
     */
    void clearAllHistory();

    /**
     * @brief Get history information for debugging
     * @return Vector of all recorded history entries
     */
    std::vector<HistoryEntry> getHistoryEntries() const;

    /**
     * @brief Set EventDispatcher for delayed events and external targets
     * @param eventDispatcher EventDispatcher instance for event handling
     */
    void setEventDispatcher(std::shared_ptr<IEventDispatcher> eventDispatcher);

    /**
     * @brief §scxml-6.4.3: Set completion callback for top-level final state notification
     *
     * This callback is invoked when the StateMachine reaches a top-level final state,
     * AFTER all onexit handlers have been executed. Used by invoke mechanism to
     * generate done.invoke events per W3C SCXML specification.
     *
     * @param callback Function to call on completion (nullptr to clear)
     */
    using CompletionCallback = std::function<void()>;
    void setCompletionCallback(CompletionCallback callback);

    /**
     * @brief §scxml-5.5 + 6.4.3: JSON payload from the reached top-level
     *        `<final>`'s `<donedata>`. Empty when no donedata was authored,
     *        donedata evaluation failed, or the machine has not reached a
     *        top-level final yet. Mirror of
     *        `StaticExecutionEngine::donedataAtFinal()` on the AOT side;
     *        consumed by `SCXMLInvokeHandler`'s completion callback to
     *        populate `done.invoke.<id>._event.data`.
     */
    const std::string &donedataAtFinal() const {
        return pendingDonedataAtFinal_;
    }

    /**
     * @brief §scxml-5.5 + B.2: Structured donedata paired with
     *        `donedataAtFinal()`. Skipping the JSON round-trip when the
     *        parent is in-process would require an impl-specific
     *        `raiseEventWithPriority` path; the public `IEventRaiser`
     *        5-arg `raiseEvent` used by `SCXMLInvokeHandler` re-hydrates
     *        via `EventDataHelper::jsonStringToScriptValue` inside
     *        `EventRaiserImpl::raiseEventWithPriority`, so callers that
     *        use the interface can rely on `donedataAtFinal()` alone;
     *        this getter exists for parity with the AOT shape and for
     *        future impl-specific fast paths.
     */
    const std::optional<ScriptValue> &typedDonedataAtFinal() const {
        return pendingTypedDonedataAtFinal_;
    }

    /**
     * @brief Set EventRaiser for event processing
     * @param eventRaiser EventRaiser instance for event handling
     */
    void setEventRaiser(std::shared_ptr<IEventRaiser> eventRaiser);

    /**
     * @brief Get EventDispatcher for access by child components
     * @return Current EventDispatcher instance
     */
    std::shared_ptr<IEventDispatcher> getEventDispatcher() const;

    /**
     * @brief Get InvokeExecutor for invoke state capture/restore
     * @return Pointer to InvokeExecutor instance
     */
    InvokeExecutor *getInvokeExecutor() const;

    /**
     * @brief Get the event raiser for queue introspection
     * @return Shared pointer to the event raiser
     */
    std::shared_ptr<IEventRaiser> getEventRaiser() const {
        return eventRaiser_;
    }

    /**
     * @brief Raise an external event on the external event queue
     *
     * Delegates to EventRaiser's raiseExternalEvent for W3C SCXML compliant
     * external event processing. External events are queued and processed
     * at the next macrostep boundary.
     *
     * @param eventName Name of the event
     * @param eventData Optional event data (JSON string)
     * @return true if event was queued successfully, false if no EventRaiser available
     */
    bool raiseExternalEvent(const std::string &eventName, const std::string &eventData);

    /**
     * @brief Get the last error from SCXML loading/parsing
     * @return Error detail string, empty if no error
     */
    const std::string &getLastLoadError() const {
        return lastLoadError_;
    }

    /**
     * @brief Restore state machine from snapshot (complete restoration)
     *
     * Time-travel debugging support for InteractiveTestRunner
     * Handles all restoration requirements internally in correct order:
     * 1. JavaScript environment initialization
     * 2. State configuration restoration
     * 3. Running state activation
     *
     * ARCHITECTURE.md: Zero Duplication - encapsulates restoration lifecycle
     * to prevent temporal coupling and maintain Single Source of Truth
     *
     * @param states Vector of state IDs to activate (document order preserved)
     * @return true if restoration succeeded, false on failure
     *
     * @note Thread Safety: NOT thread-safe. Caller must ensure no concurrent
     *       state machine operations (start/stop/processEvent) during restoration.
     * @note State Validity: Invalid state IDs in snapshot are silently skipped
     *       by StateHierarchyManager. Check logs for restoration warnings.
     * @note JS Environment: Idempotent - safe to call even if JS environment
     *       already initialized (e.g., after start()).
     */
    bool restoreFromSnapshot(const std::vector<std::string> &states);

private:
    /// A transition a selection enabled — one member of Appendix D's
    /// `enabledTransitions`, over this machine's string ids.
    using Transition = SCE::Core::EnabledTransition<std::string, std::string>;

    /// This machine as `SCE::Core::MicrostepAlgorithms` reads it. Defined in
    /// StateMachine.cpp, beside the effects it forwards to.
    struct MicrostepHost;

    /**
     * @brief One macrostep of this machine, from the call that starts it to
     *        its return
     *
     * Nothing a macrostep raises is processed until its main event loop takes
     * it, so the raiser's immediate mode — which hands an event straight back
     * to this machine — is off for the whole step rather than toggled around
     * each block of content. Between macrosteps it is on in auto mode, so an
     * event arriving from another thread (a delayed `<send>` firing, a child
     * session reporting) starts the next macrostep at once.
     */
    class MacrostepScope {
    public:
        explicit MacrostepScope(StateMachine &machine);
        ~MacrostepScope();
        MacrostepScope(const MacrostepScope &) = delete;
        MacrostepScope &operator=(const MacrostepScope &) = delete;

    private:
        StateMachine &machine_;
    };

    /// Whether this machine's macrostep is running on the calling thread. Per
    /// machine rather than a thread-local depth: an autoforward hands a child
    /// its copy through the child's `processEvent`, on this very thread, and
    /// the child's macrostep is its own.
    bool macrostepInProgressOnThisThread() const;

    // Thread-safe: accessed from EventRaiser callback (main thread) and from
    // scheduler threads
    std::atomic<bool> isRunning_{false};
    bool autoProcessQueuedEvents_ = true;  // Interactive mode: the host steps queued events itself
    std::atomic<std::thread::id> macrostepOwner_{};

    /// Set on entering a top-level `<final>`. The entry point that owns the
    /// macrostep finishes the session once the microstep is over.
    bool topLevelFinalReached_ = false;

    // Last executed transition tracking (for interactive visualizer)
    std::string lastTransitionSource_{};
    std::string lastTransitionTarget_{};

    // WHICH transition of that source state fired, by its position in the
    // state's transition list — the order the document declares them in, and
    // the order `getTransitions()` returns.
    //
    // ⚠ Source, target and event do not identify a transition. W3C SCXML
    // lets several share all three and differ only by `cond`, which is the
    // ordinary way to write a branch; measured over this repository's
    // documents, 33 keys of the form `source_event_target` name more than
    // one transition and one names thirteen. Without this the visualizer
    // could say "one of these thirteen fired" and no more.
    //
    // -1 means "not known", which is honest for a restored snapshot: the
    // position is not part of what a snapshot carries.
    int lastTransitionIndex_{-1};

    // What the last microstep's selection found and what survived
    // preemption, for the interactive visualizer.
    std::vector<TransitionDescriptorString> lastEnabledTransitions_{};
    std::vector<TransitionDescriptorString> lastOptimalTransitions_{};
    /// What the selection now running has found, before conflicts are
    /// removed. Published to `lastEnabledTransitions_` only when a microstep
    /// is taken, so an eventless selection that finds nothing does not erase
    /// what the visualizer shows about the event before it.
    std::vector<Transition> selectionCandidates_{};

    /// How many microsteps one macrostep may take before this engine stops
    /// taking them. The clause defines a macrostep as a chain ending where
    /// nothing is enabled by NULL and no internal event is left, and the
    /// specification's Principles and Constraints say that chain need not
    /// exist ("A macrostep may not [terminate] ... This is currently
    /// allowed"), so the ceiling is this engine declining a document the
    /// specification permits — which is why `Statistics::truncatedMacrosteps`
    /// publishes the decline instead of a log line carrying it alone. It is
    /// the AOT engine's `MAX_MACROSTEP_MICROSTEPS`, the same number for the
    /// same reason.
    ///
    /// One budget for the whole inner loop, not one per branch: a document
    /// that alternates an eventless transition with a `<raise>` is one chain,
    /// and budgeting the branches separately leaves it unbounded.
    ///
    /// Ten times the error-cascade depth, and deliberately not equal to it.
    /// This is the backstop; the cascade ceiling is a diagnostic that names
    /// the error a handler keeps failing on, and a backstop that fires first
    /// makes that diagnostic unreachable — measured 2026-08-20, with both at a
    /// hundred a handler that raises one event of its own per link was cut at
    /// fifty links here and the cascade count never moved.
    static constexpr int MAX_MACROSTEP_MICROSTEPS = 1000;

    /// Macrosteps stopped at that ceiling with the chain still going, and the
    /// state the drain was in when it last happened. `macrostepTruncated_`
    /// is cleared where a macrostep starts: the host's call, and each
    /// external event this machine takes off its own queue.
    uint32_t truncatedMacrosteps_ = 0;
    std::string lastTruncatedMacrostepState_;
    bool macrostepTruncated_ = false;
    /// §scxml-B-2-8-1: deliveries whose payload announced structure and could
    /// not be read as one, and the name of the last such event. Reported
    /// through `Statistics::undecodablePayloads` — the count lives here
    /// because the executor binds one event at a time and this is the object a
    /// host holds.
    uint32_t undecodablePayloads_ = 0;
    std::string lastUndecodablePayloadEvent_;
    /// §scxml-3.13: external events refused because this machine had stopped,
    /// or left on its external queue when it did, and the name of the last
    /// one. Reported through `Statistics::unseenExternalEvents`.
    uint32_t unseenExternalEvents_ = 0;
    std::string lastUnseenEventName_;
    /// Microsteps this macrostep has taken, on eventless transitions and on
    /// internal events alike — one count, because a document that alternates
    /// the two is one chain. Only the main event loop spends it: the raiser
    /// holds the queues, and the loop is the one party that takes from them
    /// inside a macrostep.
    uint32_t macrostepMicrostepsTaken_ = 0;

    // SCXML model, and the model as Appendix D reads it
    std::shared_ptr<SCXMLModel> model_;
    std::unique_ptr<InterpreterDocument> document_;

    // Script engine integration
    IScriptEngine &scriptEngine_;
    std::string sessionId_;
    std::string basicHttpAccessUri_;  // Inbound BasicHTTP endpoint, empty when none is deployed
    std::string lastLoadError_;       // Parser error detail from loadSCXMLFromString
    std::string currentEventData_;
    std::string currentOriginSessionId_;  // W3C SCXML Test 252: Track origin for cancelled invoke filtering
    bool jsEnvironmentReady_ = false;

    // Action execution infrastructure
    std::shared_ptr<IActionExecutor> actionExecutor_;
    ActionExecutorImpl *cachedExecutorImpl_ = nullptr;  // Cached pointer to avoid dynamic_pointer_cast
    std::shared_ptr<IExecutionContext> executionContext_;

    // The configuration
    std::unique_ptr<StateHierarchyManager> hierarchyManager_;

    // History state management (SOLID architecture)
    std::unique_ptr<HistoryManager> historyManager_;
    std::unique_ptr<HistoryStateAutoRegistrar> historyAutoRegistrar_;

    // W3C SCXML invoke execution (SOLID architecture)
    std::unique_ptr<InvokeExecutor> invokeExecutor_;

    // Event dispatching for delayed events and external targets
    std::shared_ptr<IEventDispatcher> eventDispatcher_;

    // EventRaiser: holds both of Appendix D's queues
    std::shared_ptr<IEventRaiser> eventRaiser_;

    // §scxml-6.4.3: Completion callback for invoke done.invoke event
    CompletionCallback completionCallback_;

    // §scxml-5.5 + 6.4.3: donedata payload captured when the machine
    // enters a top-level `<final>`, before `completionCallback_` fires,
    // mirroring `StaticExecutionEngine::stashDonedataAtFinal` on the AOT side.
    // Consumed by `SCXMLInvokeHandler`'s completion callback to populate
    // `done.invoke.<id>._event.data` (`donedataAtFinal()` getter).
    std::string pendingDonedataAtFinal_;
    std::optional<ScriptValue> pendingTypedDonedataAtFinal_;

    // §scxml-6.4: Pending invoke execution (deferred until macrostep end)
    // Uses InvokeHelper for shared logic with AOT engine (ARCHITECTURE.md Zero Duplication)
    struct PendingInvoke {
        std::string invokeId;                 // Invoke ID (for InvokeHelper logging)
        std::string state;                    // State ID (matches InvokeHelper template parameter)
        std::shared_ptr<IInvokeNode> invoke;  // Single invoke node (not vector)
    };

    std::vector<PendingInvoke> pendingInvokes_;
    std::recursive_mutex
        pendingInvokesMutex_;  // Recursive: same-thread re-entry during child invoke initialization (W3C 6.4)

    // W3C SCXML: Thread safety for StateHierarchyManager access from JSEngine worker thread
    mutable std::mutex hierarchyManagerMutex_;  // Protects hierarchyManager_ read access

    // Serializes macrosteps started from different threads, and lets the
    // destructor wait for one in progress (ASAN heap-use-after-free fix). A
    // delivery on the thread already running this machine's macrostep never
    // takes it: it is queued instead (`macrostepInProgressOnThisThread`).
    std::mutex processEventMutex_;

    // Statistics
    mutable Statistics stats_;

    // §scxml-5.3: Data model initialization (delegated to DataModelInitializer)
    using DataItemInfo = DataModelInitializer::DataItemInfo;
    std::unique_ptr<DataModelInitializer> dataModelInit_;

    bool initializeFromModel();
    void initializeHistoryManager();
    void initializeHistoryAutoRegistrar();

    bool evaluateCondition(const std::string &condition);

    // ── W3C SCXML Appendix D over the parsed model ───────────────────────
    //
    // What Appendix D does with the answers below is
    // `SCE::Core::MicrostepAlgorithms`; these are the answers.

    /// §scxml-D-selectTransitions, or selectEventlessTransitions for an empty
    /// name — the optimal enabled set, in selection order.
    std::vector<Transition> selectTransitions(const std::string &eventName);
    std::optional<Transition> firstEnabledTransition(const std::string &state, const std::string &eventName);
    void takeMicrostep(const std::vector<Transition> &transitions, const std::string &eventName);
    void enterInitialConfiguration();
    void runMainEventLoop();
    /// The outer loop's one step: take the next external event and start the
    /// macrostep it opens. False when the external queue is empty.
    bool takeNextExternalEvent();
    TransitionResult processTakenEvent(const Core::EventMetadata &event, bool fromExternalQueue);
    void bindCurrentEvent(const Core::EventMetadata &event);
    void applyFinalize(const std::string &originSessionId, const std::string &eventName);
    void autoforward(const Core::EventMetadata &event);
    void finishAtTopLevelFinal();
    TransitionResult refuseUnseen(const std::string &eventName);
    /// Put an event handed to this machine while its macrostep is running on
    /// this thread onto the queue it belongs to, for the main event loop to
    /// take in turn.
    TransitionResult holdDelivery(const Core::EventMetadata &event, bool fromExternalQueue);
    /// Count an external event this machine will never look at.
    void noteUnseenEvent(const std::string &eventName);
    /// Empty both queues at the end of the interpretation: every external
    /// event still there is counted as unseen.
    void recordUnseenQueuedEvents();

    void exitStateInMicrostep(const std::string &state, const std::vector<std::string> &configurationBeforeExit);
    void exitState(const std::string &state);
    void cancelInvokesOf(const std::string &state);
    void enterStateInMicrostep(const std::string &state, bool isDefaultEntry);
    void enterFinalState(const std::string &finalState);
    void raiseInternal(const std::string &eventName, const std::string &eventData,
                       std::optional<ScriptValue> typedData);
    void executeTransitionContent(const Transition &transition);
    void executeHistoryDefaultContent(const std::string &history);
    /// Appendix D's isInFinalState over the configuration as it stands now.
    bool stateIsInFinalState(const std::string &state) const;
    std::vector<std::string> configurationInExitOrder() const;
    TransitionDescriptorString describe(const Transition &transition, const std::string &eventName,
                                        const std::vector<std::string> &configuration);

    /// Record that a macrostep was stopped at `MAX_MACROSTEP_MICROSTEPS` with
    /// its chain still going. Shared by both branches of the main event loop,
    /// so both report the same fact the same way.
    void recordTruncatedMacrostep();

    /// §scxml-3.13: may the macrostep now in progress take another microstep?
    /// Asked before an internal event leaves the queue, so a refusal leaves
    /// it for the next macrostep; publishes the refusal on the way out.
    bool mayTakeMicrostep();

    // IActionNode-based action execution
    bool initializeActionExecutor();
    bool executeActionNodes(const std::vector<std::shared_ptr<SCE::IActionNode>> &actions);
    bool executeExitActions(const std::string &stateId);
    void executeOnEntryActions(const std::string &stateId);

    // JavaScript environment lifecycle (internal use only)
    bool ensureJSEnvironment();
    bool setupJSEnvironment();
    void updateStatistics();

    // Deferred invoke execution for W3C SCXML compliance
    void deferInvokeExecution(const std::string &stateId, const std::vector<std::shared_ptr<IInvokeNode>> &invokes);
    void executePendingInvokes();

    // Helper method to reduce code duplication between isInFinalState() and isInitialStateFinal()
    bool isStateInFinalState(const std::string &stateId) const;

    // §scxml-5.5: donedata, through DoneDataHelper (Zero Duplication)
    bool evaluateDoneData(const std::string &finalStateId, std::string &outEventData,
                          std::optional<ScriptValue> &outTypedData);
};

}  // namespace SCE

// Engine-agnostic bindObject template implementation
// Include StateMachineBindObject.h explicitly when using bindObject() with GenericClassBinder API.
// Not auto-included to avoid coupling all StateMachine users to scripting headers.
