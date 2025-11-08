// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <map>
#include <memory>
#include <optional>
#include <queue>
#include <string>
#include <vector>

#ifdef __EMSCRIPTEN__
#include <emscripten/val.h>
#endif

#include "model/TransitionNode.h"
#include "runtime/StateMachine.h"
#include "runtime/StateSnapshot.h"

namespace SCE::W3C {

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

    std::shared_ptr<StateMachine> stateMachine_;
    SnapshotManager snapshotManager_;

    // Initial snapshot captured after initialize() for true reset functionality
    std::optional<StateSnapshot> initialSnapshot_;

    int currentStep_;
    std::string lastTransitionSource_;
    std::string lastTransitionTarget_;
    std::string lastEventName_;

    // Pending external events queue (for step-by-step execution)
    std::queue<EventSnapshot> pendingEvents_;

    // Event execution history for accurate state restoration via replay
    // W3C SCXML 3.13: All processed events stored to enable time-travel debugging
    std::vector<EventSnapshot> executedEvents_;
};

}  // namespace SCE::W3C
