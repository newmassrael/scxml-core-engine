// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <map>
#include <memory>
#include <optional>
#include <queue>
#include <string>
#include <unordered_map>
#include <vector>

#ifdef __EMSCRIPTEN__
#include <emscripten/val.h>
#endif

#include "model/TransitionNode.h"
#include "runtime/StateMachine.h"
#include "runtime/StateSnapshot.h"

namespace SCE::W3C {

/**
 * @brief Information about detected sub-SCXML file from static analysis
 *
 * W3C SCXML 6.3: Invoke elements with static src attributes are analyzed
 * at loadSCXML() time to enable visualization regardless of child execution timing.
 */
struct SubSCXMLInfo {
    std::string parentStateId;  // State containing the invoke element
    std::string invokeId;       // Invoke element ID
    std::string srcPath;        // Resolved file path (absolute)
#ifdef __EMSCRIPTEN__
    emscripten::val structure;  // Pre-parsed SCXML structure for visualization
#endif
};

/**
 * @brief Interactive test runner for step-by-step SCXML execution visualization
 *
 * Provides fine-grained control over state machine execution for debugging
 * and educational purposes. Supports forward/backward stepping, state
 * introspection, and execution history.
 *
 * Architecture:
 * - Wraps StateMachine with snapshot-based time-travel debugging
 * - Exposes JavaScript-friendly API via Emscripten bindings
 * - Maintains execution history for backward stepping
 */
class InteractiveTestRunner {
public:
    InteractiveTestRunner();
    ~InteractiveTestRunner();

    /**
     * @brief Load SCXML from file or string content
     *
     * @param scxmlSource Path to SCXML file or SCXML content string
     * @param isFilePath If true, treat as file path; otherwise, as SCXML content
     * @return true if loaded and initialized successfully
     */
    bool loadSCXML(const std::string &scxmlSource, bool isFilePath = true);

    /**
     * @brief Initialize state machine to initial configuration
     *
     * Must be called after loadSCXML before stepping.
     * Captures initial snapshot for reset functionality.
     *
     * @return true if initialization succeeded
     */
    bool initialize();

    /**
     * @brief Execute one microstep (process next event or eventless transition)
     *
     * W3C SCXML 3.13: A microstep processes one event and executes enabled transitions.
     * Captures snapshot after execution for backward stepping.
     *
     * @return true if step executed, false if in final state
     */
    bool stepForward();

    /**
     * @brief Restore previous execution state (backward step)
     *
     * Restores state machine to previous snapshot.
     *
     * @return true if restored successfully, false if at beginning
     */
    bool stepBackward();

    /**
     * @brief Reset to initial state
     *
     * Restores state machine to initial snapshot (after initialize()).
     */
    void reset();

    /**
     * @brief Raise external event for next step
     *
     * Queues an event to be processed on next stepForward().
     *
     * @param eventName Event name
     * @param eventData Optional event data (JSON string)
     */
    void raiseEvent(const std::string &eventName, const std::string &eventData = "");

    /**
     * @brief Remove event from internal queue by index
     *
     * W3C SCXML 3.13: Removes event from internal queue at specified index.
     * Triggers history branching: removes all snapshots after current step.
     *
     * @param index Zero-based index in internal queue
     * @return true if removed successfully, false if index out of range
     */
    bool removeInternalEvent(int index);

    /**
     * @brief Remove event from external queue by index
     *
     * W3C SCXML 3.13: Removes event from external queue at specified index.
     * Triggers history branching: removes all snapshots after current step.
     *
     * @param index Zero-based index in external queue
     * @return true if removed successfully, false if index out of range
     */
    bool removeExternalEvent(int index);

    /**
     * @brief Get current active states
     *
     * @return Vector of active state IDs
     */
    std::vector<std::string> getActiveStates() const;

    /**
     * @brief Get current step number
     *
     * @return Current execution step (0 = initial configuration)
     */
    int getCurrentStep() const {
        return currentStep_;
    }

    /**
     * @brief Check if state machine is in final state
     */
    bool isInFinalState() const;

    /**
     * @brief Get last transition information
     *
     * Returns JavaScript object with:
     * - source: Source state ID
     * - target: Target state ID
     * - event: Event name
     * - id: Transition ID (for W3C spec reference)
     *
     * @return JavaScript object or null if no transition
     */
#ifdef __EMSCRIPTEN__
    emscripten::val getLastTransition() const;
#else
    std::string getLastTransition() const;
#endif

    /**
     * @brief Get event queue state
     *
     * Returns JavaScript object with:
     * - internal: Array of internal queue events
     * - external: Array of external queue events
     *
     * @return JavaScript object with queue contents
     */
    emscripten::val getEventQueue() const;

    /**
     * @brief Get data model snapshot
     *
     * Returns JavaScript object with all data model variables.
     *
     * @return JavaScript object {var1: value1, var2: value2, ...}
     */
#ifdef __EMSCRIPTEN__
    emscripten::val getDataModel() const;
#else
    std::string getDataModel() const;
#endif

    /**
     * @brief Get SCXML structure for visualization
     *
     * Returns JavaScript object with:
     * - states: Array of state objects {id, type, initial, children}
     * - transitions: Array of transition objects {source, target, event, guard}
     * - initial: Initial state ID
     *
     * @return JavaScript object with SCXML structure
     */
#ifdef __EMSCRIPTEN__
    emscripten::val getSCXMLStructure() const;
#else
    std::string getSCXMLStructure() const;
#endif

    /**
     * @brief Get W3C specification references for current test
     *
     * Returns JavaScript object mapping transition IDs to W3C spec sections.
     *
     * @return JavaScript object {transitionId: "3.13", ...}
     */
#ifdef __EMSCRIPTEN__
    emscripten::val getW3CReferences() const;
#else
    std::string getW3CReferences() const;
#endif

    /**
     * @brief Preload SCXML file content for invoke resolution (WASM)
     *
     * In WASM environment, there's no filesystem access. This method allows
     * JavaScript to preload sub-SCXML files so invoke with src="file:..." works.
     *
     * @param filename Relative filename (e.g., "test226sub1.scxml")
     * @param content Full SCXML content string
     * @return true if preloaded successfully
     *
     * Example usage from JavaScript:
     *   const subContent = await fetch("../../resources/226/test226sub1.scxml").then(r => r.text());
     *   runner.preloadFile("test226sub1.scxml", subContent);
     */
    bool preloadFile(const std::string &filename, const std::string &content);

    /**
     * @brief Set base directory path for resolving relative invoke paths (WASM)
     *
     * Tells the engine where the parent SCXML file is located, so it can
     * resolve relative paths in invoke src="file:...".
     *
     * @param basePath Base directory path (e.g., "../../resources/226/")
     */
    void setBasePath(const std::string &basePath);

    /**
     * @brief Get invoked child state machines for visualization
     *
     * Returns JavaScript object with child state machine information:
     * {
     *   children: [
     *     {
     *       sessionId: "child_session_id",
     *       activeStates: ["state1", "state2"],
     *       structure: { states: [...], transitions: [...] },
     *       isInFinalState: false
     *     }
     *   ]
     * }
     *
     * @return JavaScript object with child information
     */
#ifdef __EMSCRIPTEN__
    emscripten::val getInvokedChildren() const;
#else
    std::string getInvokedChildren() const;
#endif

    /**
     * @brief Get statically detected sub-SCXML structures for visualization
     *
     * Analyzes parent SCXML at load time to find invoke elements with static src,
     * verifies files exist, parses structures, and caches for visualization.
     *
     * Returns JavaScript array of objects:
     * [
     *   {
     *     parentStateId: "s0",
     *     invokeId: "child1",
     *     srcPath: "/resources/226/test226sub1.scxml",
     *     structure: { states: [...], transitions: [...], initial: "..." }
     *   }
     * ]
     *
     * @return JavaScript array of sub-SCXML information
     */
#ifdef __EMSCRIPTEN__
    emscripten::val getSubSCXMLStructures() const;
#endif

private:
    /**
     * @brief Capture current state machine state as snapshot
     */
    void captureSnapshot();

    /**
     * @brief Restore state machine from snapshot
     *
     * @param snapshot Snapshot to restore
     * @return true if restored successfully
     */
    bool restoreSnapshot(const StateSnapshot &snapshot);

    /**
     * @brief Extract data model from JSEngine
     *
     * @return Map of variable names to serialized values
     */
    std::map<std::string, std::string> extractDataModel() const;

    /**
     * @brief Restore data model to JSEngine
     *
     * @param dataModel Map of variable names to serialized values
     */
    void restoreDataModel(const std::map<std::string, std::string> &dataModel);

    /**
     * @brief Extract event queues from state machine
     *
     * @param outInternal Internal queue events
     * @param outExternal External queue events
     */
    void extractEventQueues(std::vector<EventSnapshot> &outInternal, std::vector<EventSnapshot> &outExternal) const;

    /**
     * @brief Restore event queues to state machine
     *
     * @param internal Internal queue events
     * @param external External queue events
     */
    void restoreEventQueues(const std::vector<EventSnapshot> &internal, const std::vector<EventSnapshot> &external);

    /**
     * @brief Analyze parent SCXML for static invoke elements
     *
     * W3C SCXML 6.3: Finds invoke elements with static src, verifies file existence,
     * parses child SCXML structures, and caches for visualization.
     *
     * @param parentModel Parent SCXML model to analyze
     */
    void analyzeSubSCXML(std::shared_ptr<SCXMLModel> parentModel);

    /**
     * @brief Convert Type enum to string for visualization
     *
     * Zero Duplication: Single implementation used by getSCXMLStructure,
     * getInvokedChildren, and buildStructureFromModel.
     *
     * @param type State type enum value
     * @return String representation ("atomic", "compound", "parallel", etc.)
     */
    static std::string typeToString(SCE::Type type);

#ifdef __EMSCRIPTEN__
    /**
     * @brief Build structure object from SCXML model for visualization
     *
     * Extracts states, transitions, and initial state from model into
     * JavaScript-friendly object format.
     *
     * @param model SCXML model to extract structure from
     * @return JavaScript object with structure information
     */
    emscripten::val buildStructureFromModel(std::shared_ptr<SCXMLModel> model) const;

    /**
     * @brief Serialize action nodes to JavaScript array for visualization
     *
     * W3C SCXML 3.7: Converts executable content (assign, raise, foreach, log)
     * into JavaScript-friendly format for action visualization.
     *
     * @param actions Vector of action nodes to serialize
     * @return JavaScript array of action objects
     */
    emscripten::val serializeActions(const std::vector<std::shared_ptr<IActionNode>> &actions) const;
#endif

    std::shared_ptr<StateMachine> stateMachine_;
    SnapshotManager snapshotManager_;

    // W3C SCXML 6.2: Event infrastructure for send/invoke support
    std::shared_ptr<IEventRaiser> eventRaiser_;
    std::shared_ptr<IEventScheduler> scheduler_;
    std::shared_ptr<IEventDispatcher> eventDispatcher_;

    // Initial snapshot captured after initialize() for true reset functionality
    std::optional<StateSnapshot> initialSnapshot_;

    int currentStep_;
    std::string lastTransitionSource_;
    std::string lastTransitionTarget_;
    std::string lastEventName_;

    // Track previous active states for transition source detection
    std::set<std::string> previousActiveStates_;

    // W3C SCXML 3.13: UI events now managed by EventRaiser's external queue (Zero Duplication)
    // pendingEvents_ removed - EventRaiser owns all event queue management

    // Event execution history for accurate state restoration via replay
    // W3C SCXML 3.13: All processed events stored to enable time-travel debugging
    std::vector<EventSnapshot> executedEvents_;

    // WASM file preloading support (for invoke src="file:..." resolution)
    std::string basePath_;                                         // Base directory for resolving relative paths
    std::unordered_map<std::string, std::string> preloadedFiles_;  // filename -> content

    // W3C SCXML 6.3: Static sub-SCXML analysis results (populated at loadSCXML time)
    std::vector<SubSCXMLInfo> subScxmlStructures_;
};

}  // namespace SCE::W3C
