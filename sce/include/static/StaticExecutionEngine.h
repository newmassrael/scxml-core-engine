// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/EventMetadataHelper.h"
#include "common/EventTypeHelper.h"
#include "core/HierarchicalStateHelper.h"
#include "core/HistoryHelper.h"
#include "core/LogMacros.h"
#include "core/StatePolicyConcepts.h"
#include "common/SCXMLConstants.h"
#include "common/SendHelper.h"
#include "common/SendSchedulingHelper.h"
#include "core/EventMetadata.h"
#include "core/EventProcessingAlgorithms.h"
#include "core/AOTEventQueue.h"
#include "core/EventQueueManager.h"
#include "events/EventDescriptor.h"
// W3C SCXML C.2: BasicHTTP Event I/O Processor support
// SCE_ENABLE_HTTP is defined only by sce_runtime (PUBLIC), never by sce_core standalone.
// HttpSendHelper.h requires sce_runtime linkage (CppHttplibClient.cpp / EmscriptenFetchClient.cpp).
#ifdef SCE_ENABLE_HTTP
#include "static/HttpSendHelper.h"
#endif
#include <chrono>
#include <cstdint>
#include <functional>
#include <map>
#include <memory>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

namespace SCE::Static {

/// W3C SCXML C.2: HTTP send request data for BasicHTTPEventProcessor callback.
/// Matches Kotlin HttpSendRequest — transport-agnostic data struct passed to onHttpSend callback.
struct HttpSendRequest {
    std::string target;
    std::string eventName;
    std::string content;
    std::map<std::string, std::vector<std::string>> params;
    std::string sendId;
};

/**
 * @brief Template-based SCXML execution engine for static code generation
 *
 * This engine implements the core SCXML execution semantics (event queue management,
 * entry/exit actions, transitions) while delegating state-specific logic to the
 * StatePolicy template parameter.
 *
 * Key SCXML standards implemented:
 * - Internal event queue with FIFO ordering (W3C SCXML 3.12.1)
 * - Entry/exit action execution (W3C SCXML 3.7, 3.8)
 * - Event processing loop (W3C SCXML D.1)
 *
 * @tparam StatePolicy Policy class providing state-specific implementations.
 *         Must satisfy SCE::Core::EventNamingPolicy concept (C++20) or duck typing (C++17).
 *         See core/StatePolicyConcepts.h for the full interface contract.
 */
#if __cpp_concepts >= 202002L
template <SCE::Core::EventNamingPolicy StatePolicy> class StaticExecutionEngine {
#else
template <typename StatePolicy> class StaticExecutionEngine {
#endif
    // ── Compile-time verification of required member variables ──
    // StatePolicy is generated as a struct (public members), verified via requires expression.
#if __cpp_concepts >= 202002L
    static_assert(requires(StatePolicy p) { { p.lastTransitionIsInternal_ } -> std::convertible_to<bool>; },
                  "StatePolicy must have member: mutable bool lastTransitionIsInternal_");
    static_assert(requires(StatePolicy p) { { p.lastTransitionIsTargetless_ } -> std::convertible_to<bool>; },
                  "StatePolicy must have member: mutable bool lastTransitionIsTargetless_");
    static_assert(requires(StatePolicy p) {
                      { p.lastTransitionSourceState_ } -> std::convertible_to<typename StatePolicy::State>;
                  },
                  "StatePolicy must have member: mutable State lastTransitionSourceState_");
#endif

public:
    using State = typename StatePolicy::State;
    using Event = typename StatePolicy::Event;

    /**
     * @brief Event with metadata for W3C SCXML 5.10 compliance
     *
     * Wraps Event enum with metadata (origin, sendid, data, type) to support
     * _event.origin, _event.sendid, _event.data, _event.type fields.
     *
     * @example Constructor Pattern (required for nested template types)
     * @code
     * // Create event with data and sendId
     * engine.raise(EventWithMetadata(Event::Error_execution, errorMsg, "", sendId));
     *
     * // Create external event with all metadata
     * externalQueue_.raise(EventWithMetadata(
     *     Event::Foo,         // event
     *     "data",             // data
     *     "#_internal",       // origin
     *     sendId,             // sendId
     *     "external"          // type
     * ));
     * @endcode
     */
    struct EventWithMetadata {
        Event event;
        std::string data;
        std::string origin;      // W3C SCXML 5.10.1: _event.origin
        std::string sendId;      // W3C SCXML 5.10.1: _event.sendid
        std::string type;        // W3C SCXML 5.10.1: _event.type
        std::string originType;  // W3C SCXML 5.10.1: _event.origintype
        std::string invokeId;    // W3C SCXML 5.10.1: _event.invokeid
        std::string target;      // W3C SCXML C.2: HTTP POST target URL
        std::optional<ScriptValue> typedData;  // W3C SCXML 5.5: Engine-agnostic typed event data

        // Default constructor for aggregate initialization
        EventWithMetadata() = default;

        // Constructor with positional parameters (event, data, origin, sendId, type, originType, invokeId, target)
        EventWithMetadata(Event e, const std::string &d = "", const std::string &o = "", const std::string &s = "",
                          const std::string &t = "", const std::string &ot = "", const std::string &i = "",
                          const std::string &tgt = "")
            : event(e), data(d), origin(o), sendId(s), type(t), originType(ot), invokeId(i), target(tgt) {}
    };

private:
    /**
     * @brief Handle hierarchical exit and entry for state transition
     *
     * @details
     * ARCHITECTURE.md: Extract duplicate code from processEventQueues
     * W3C SCXML 3.12: Compute LCA and execute hierarchical exit/entry
     *
     * @param oldState State before transition
     * @param newState State after transition
     * @param preTransitionStates Active states before transition (for history recording)
     */
    // C++17-compatible named empty callable (replaces decltype([] {}))
    struct NoOpAction { void operator()() const {} };

    template <typename TransitionActionFn = NoOpAction>
    void handleHierarchicalTransition(State oldState, State newState, const std::vector<State> &preTransitionStates,
                                      TransitionActionFn &&transitionAction = {}) {
        SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Transition {} -> {}", static_cast<int>(oldState),
                  static_cast<int>(newState));

        // W3C SCXML 5.9.2: Determine LCA based on transition type
        std::optional<State> lca;
        if (policy_.lastTransitionIsInternal_) {
            // W3C SCXML 5.9.2: Internal transitions whose target is NOT a proper descendant behave as external
            bool isSelfTransition = (oldState == newState);
            bool isProperDescendant =
                !isSelfTransition &&
                SCE::Core::HierarchicalStateHelper<StatePolicy>::isDescendantOf(newState, oldState);

            // W3C SCXML 3.13: Check if source is compound state (test 533)
            // Parallel states and atomic states are NOT compound - internal transitions from them behave as external
            bool isSourceCompound = StatePolicy::isCompoundState(oldState);

            if (isProperDescendant && isSourceCompound) {
                // W3C SCXML 3.13: Internal transition to proper descendant in compound state - source is LCA (don't
                // exit source)
                lca = oldState;  // Source is the LCA - don't exit it
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Internal transition (proper descendant, compound source) "
                          "- source {} is LCA",
                          static_cast<int>(oldState));
            } else {
                // W3C SCXML 3.13/5.9.2: Non-compound source or non-descendant - behaves as external
                // Use normal LCA calculation, then target==LCA check handles exit/re-entry
                lca = SCE::Core::HierarchicalStateHelper<StatePolicy>::findLCA(oldState, newState);
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Internal transition (non-compound source or "
                          "non-descendant) - behaves as "
                          "external, LCA={}",
                          lca.has_value() ? static_cast<int>(lca.value()) : -1);
            }
        } else {
            // W3C SCXML 3.12: External transition - find LCA normally
            lca = SCE::Core::HierarchicalStateHelper<StatePolicy>::findLCA(oldState, newState);
        }

        if (lca.has_value()) {
            // W3C SCXML 3.13: First exit any active descendants of oldState (deepest first)
            std::vector<State> descendantsToExit;
            for (const auto &activeState : preTransitionStates) {
                if (activeState != oldState &&
                    SCE::Core::HierarchicalStateHelper<StatePolicy>::isDescendantOf(activeState, oldState)) {
                    descendantsToExit.push_back(activeState);
                }
            }
            // Sort by state enum value (proxy for document order - deeper states have higher values)
            std::sort(descendantsToExit.begin(), descendantsToExit.end(),
                      [](State a, State b) { return static_cast<int>(a) > static_cast<int>(b); });

            for (const auto &descendant : descendantsToExit) {
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Exit descendant {} of oldState {}",
                          static_cast<int>(descendant), static_cast<int>(oldState));
                executeOnExit(descendant, preTransitionStates);
            }

            // W3C SCXML 3.13: Exit states from oldState up to (but not including) LCA
            auto exitChain = SCE::Core::HierarchicalStateHelper<StatePolicy>::buildExitChain(oldState, lca.value());
            for (const auto &state : exitChain) {
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Hierarchical exit state {}", static_cast<int>(state));
                executeOnExit(state, preTransitionStates);
            }

            // W3C SCXML 3.10 (test 579): Ancestor transition (target == LCA)
            // When transitioning to self or ancestor, the target must also be exited and re-entered
            // This is how Interpreter handles internal self-transitions to satisfy W3C 5.9.2
            bool isTargetActive = std::find(preTransitionStates.begin(), preTransitionStates.end(), newState) !=
                                  preTransitionStates.end();
            if (newState == lca.value() && isTargetActive) {
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Ancestor/self transition - exit target {} (W3C 3.10)",
                          static_cast<int>(newState));
                executeOnExit(newState, preTransitionStates);
            }

            // W3C SCXML 3.13: Execute transition actions AFTER exit, BEFORE entry
            SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Executing transition actions");
            transitionAction();

            // W3C SCXML 3.13: Enter states from LCA down to newState (including initial children)
            std::vector<State> entryChain;

            // W3C SCXML 3.10: If target == LCA (ancestor/self transition), enter full subtree from target
            if (newState == lca.value()) {
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Ancestor/self transition - enter target {} and its "
                          "initial children (W3C 3.10)",
                          static_cast<int>(newState));
                // Build full entry chain from root, then keep only states at/below LCA
                auto fullChain = SCE::Core::HierarchicalStateHelper<StatePolicy>::buildEntryChain(newState, policy_);
                for (const auto &s : fullChain) {
                    // Include state if it's at or below LCA (check if LCA is ancestor of s or s == LCA)
                    if (s == lca.value() ||
                        SCE::Core::HierarchicalStateHelper<StatePolicy>::isDescendantOf(s, lca.value())) {
                        entryChain.push_back(s);
                    }
                }
            } else {
                // Normal case: enter from LCA's child down to newState
                entryChain =
                    SCE::Core::HierarchicalStateHelper<StatePolicy>::buildEntryChainFromParent(newState, lca.value());
            }

            for (const auto &state : entryChain) {
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Hierarchical entry state {}", static_cast<int>(state));
                executeOnEntry(state);
            }

            // W3C SCXML 3.11: Update currentState to deepest entered state
            if (!entryChain.empty()) {
                currentState_ = entryChain.back();
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Updated currentState_ to {}",
                          static_cast<int>(currentState_));
            }
        } else {
            // No LCA (top-level transition) - exit all ancestors of oldState
            SCE_LOG_DEBUG("AOT handleHierarchicalTransition: No LCA (top-level transition)");

            State current = oldState;
            while (true) {
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Exit state {} (to root)", static_cast<int>(current));
                executeOnExit(current, preTransitionStates);

                auto parent = StatePolicy::getParent(current);
                if (!parent.has_value()) {
                    break;  // Reached root
                }
                current = parent.value();
            }

            // W3C SCXML 3.13: Execute transition actions AFTER exit, BEFORE entry
            SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Executing transition actions (no LCA)");
            transitionAction();

            // Enter full hierarchy from root to newState
            auto entryChain = SCE::Core::HierarchicalStateHelper<StatePolicy>::buildEntryChain(newState, policy_);
            for (const auto &state : entryChain) {
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Entry state {} (from root)", static_cast<int>(state));
                executeOnEntry(state);
            }

            // W3C SCXML 3.11: Update currentState to deepest entered state
            if (!entryChain.empty()) {
                currentState_ = entryChain.back();
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Updated currentState_ to {}",
                          static_cast<int>(currentState_));
            }
        }
    }

    /**
     * @brief Execute a state transition with default handlers (for event queue processing)
     *
     * W3C SCXML Appendix D: For parallel states, executeMicrostep already handles
     * exit/entry, so no parallel handler is needed.
     *
     * @param event Event to process
     * @return true if a hierarchical state change occurred
     */
    bool executeTransition(Event event) {
        return executeTransition(event, [](auto, const auto &) {}, [] {});
    }

    /**
     * @brief Execute a state transition with hierarchical exit/entry handling
     *
     * W3C SCXML 3.12/3.13: Single Source of Truth for transition execution across all
     * processing paths (event queues, direct processEvent, eventless transitions).
     *
     * Callers customize two axes of variation via template callbacks:
     * - onParallelTransition: how parallel states handle exit/entry (queue path: executeMicrostep
     *   already handled; direct path: explicit executeOnExit/executeOnEntry)
     * - postTransition: work after hierarchical handling, before eventless check
     *   (queue path: nothing; direct path: processEventQueues)
     *
     * @tparam ParallelHandlerFn Callable(State oldState, vector<State> preStates) for parallel states
     * @tparam PostTransitionFn Callable() for post-hierarchical work
     * @param event Event to process
     * @param onParallelTransition Parallel exit/entry handler
     * @param postTransition Post-hierarchical work before eventless check
     * @return true if a hierarchical state change occurred
     */
    template <typename ParallelHandlerFn, typename PostTransitionFn>
    bool executeTransition(Event event, ParallelHandlerFn &&onParallelTransition,
                           PostTransitionFn &&postTransition) {
        State oldState = currentState_;
        std::vector<State> preTransitionStates = getActiveStates();
        if (!policy_.processTransition(currentState_, event, *this)) {
            return false;
        }

        // W3C SCXML 3.13: Self-transitions (target = source) exit and re-enter the state
        // W3C SCXML 5.9.2: Targetless transitions consume event only (no exit/enter)
        bool isSelfTransition = (oldState == currentState_);
        bool needsHierarchicalHandling =
            (oldState != currentState_) || (isSelfTransition && !policy_.lastTransitionIsTargetless_);

        if (!needsHierarchicalHandling) {
            // W3C SCXML 3.4: Targetless transition - execute actions without state change
            policy_.executeTransitionActions(*this);
            return false;
        }

        // W3C SCXML 3.12: State transition requires hierarchical exit/entry
        if constexpr (!StatePolicy::HAS_PARALLEL_STATES) {
            handleHierarchicalTransition(oldState, currentState_, preTransitionStates,
                                         [this] { policy_.executeTransitionActions(*this); });
        } else {
            // W3C SCXML Appendix D: Parallel states - caller provides context-specific handling
            onParallelTransition(oldState, preTransitionStates);
        }
        postTransition();
        checkEventlessTransitions();
        return true;
    }

    /**
     * @brief Shared implementation for processEvent overloads
     *
     * W3C SCXML 3.12: External event processing with full macrostep completion.
     *
     * @param event Event to process
     */
    void processEventImpl(Event event) {
        bool stateChanged = executeTransition(
            event,
            [this](State oldState, const std::vector<State> &preTransitionStates) {
                executeOnExit(oldState, preTransitionStates);
                executeOnEntry(currentState_);
            },
            [this] { processEventQueues(); });
        // W3C SCXML 6.4: Notify parent if reached final state after macrostep
        if (stateChanged && isInFinalState() && completionCallback_) {
            completionCallback_();
        }
    }

    State currentState_;
    SCE::Core::EventQueueManager<EventWithMetadata>
        internalQueue_;  // W3C SCXML C.1: Internal event queue (high priority)
    SCE::Core::EventQueueManager<EventWithMetadata>
        externalQueue_;  // W3C SCXML C.1: External event queue (low priority)
    bool isRunning_ = false;
    std::function<void()> completionCallback_;  // W3C SCXML 6.4: Callback for done.invoke
    std::function<void(const HttpSendRequest &)> onHttpSend_;  // W3C SCXML C.2: BasicHTTP callback
    SCE::PullScheduler<Event> scheduler_;       // W3C SCXML 6.2: Delayed event scheduler

protected:
    StatePolicy policy_;  // Policy instance for stateful policies

public:
    /**
     * @brief Raise an internal event with metadata (W3C SCXML C.1)
     *
     * Places event on the internal queue with FIFO ordering.
     * Internal events have higher priority than external events.
     *
     * @param metadata Complete event metadata including all W3C SCXML 5.10.1 fields
     */
    void raise(EventWithMetadata metadata) {
        // W3C SCXML C.1: Enqueue event with metadata
        internalQueue_.raise(std::move(metadata));
    }

    /**
     * @brief Raise an external event (W3C SCXML C.1, 6.2)
     *
     * External events are placed at the back of the external event queue.
     * They are processed after all internal events have been consumed.
     *
     * Used by:
     * - <send> without target (W3C SCXML 6.2)
     * - <send> with external targets (not #_internal)
     * - <send target="#_parent"> from child state machines (W3C SCXML 6.2)
     *
     * W3C SCXML C.1 (test189): External queue has lower priority than internal queue.
     *
     * @param event Event to raise externally
     * @param eventData Optional event data as JSON string (W3C SCXML 5.10)
     */
    void raiseExternal(Event event, const std::string &eventData = "", const std::string &origin = "") {
        // W3C SCXML C.1: Enqueue event with metadata (origin, data, sendid, type, originType)
        // W3C SCXML 5.10.1: Set originType to SCXML Event I/O Processor for parent-child communication
        externalQueue_.raise(
            EventWithMetadata(event, eventData, origin, "", "external", SCE::Constants::SCXML_EVENT_PROCESSOR_TYPE));

        // W3C SCXML 5.10.1: Mark next event as external for _event.type (test331)
        if constexpr (SCE::Core::HasExternalEventFlag<StatePolicy>) {
            policy_.nextEventIsExternal_ = true;
        }
    }

    /**
     * @brief Raise external event by name (W3C SCXML 6.4.6)
     *
     * Used for autoforward - converts event name string to Event enum and raises.
     * If event name doesn't match any enum value, silently ignores (child may not have that event).
     *
     * @param eventName Event name string (e.g., "childToParent")
     * @param eventData Optional event data
     */
    void raiseExternal(const std::string &eventName, const std::string &eventData = "") {
        // Convert event name to Event enum using Policy's getEventFromName() (O(n) if-chain)
        // ARCHITECTURE.md: Generated code provides efficient event name lookup
        if (auto event = policy_.getEventFromName(eventName)) {
            raiseExternal(*event, eventData);
        } else {
            SCE_LOG_DEBUG("AOT raiseExternal: Event '{}' not found in Event enum, ignoring", eventName);
        }
    }

    /**
     * @brief Raise external event with full metadata (W3C SCXML 6.4.1)
     *
     * Used for child-to-parent communication where invokeid must be preserved.
     * W3C SCXML 6.4.1 (test338): Events from child to parent must include invokeid.
     *
     * @param eventWithMetadata Event with metadata (including invokeid)
     */
    void raiseExternal(const EventWithMetadata &eventWithMetadata) {
#ifdef SCE_ENABLE_HTTP
        // W3C SCXML C.2: Delegate BasicHTTP sends to HttpSendHelper
        if (HttpSendHelper::isHttpSend(eventWithMetadata.originType, eventWithMetadata.target)) {
            HttpSendHelper::sendHttpPost(policy_.getEventName(eventWithMetadata.event), eventWithMetadata.data,
                                         eventWithMetadata.target, eventWithMetadata.sendId, *this);
            return;
        }
#endif

        // Normal internal/external queue processing
        SCE_LOG_DEBUG("AOT raiseExternal: Enqueuing external event with metadata (event={}, invokeId='{}')",
                  static_cast<int>(eventWithMetadata.event), eventWithMetadata.invokeId);

        // W3C SCXML 6.4.6: Autoforward - forward external events to children with autoforward=true
        // ARCHITECTURE.md Zero Duplication: Policy handles child forwarding (forwardToAutoforwardChildren)
        SCE_LOG_DEBUG("AOT raiseExternal: About to check autoforward capability");
        if constexpr (SCE::Core::HasAutoforward<StatePolicy, StaticExecutionEngine>) {
            SCE_LOG_DEBUG("AOT raiseExternal: Policy has autoforward capability");
            const std::string eventName = policy_.getEventName(eventWithMetadata.event);
            policy_.forwardToAutoforwardChildren(eventName, *this);
        } else {
            SCE_LOG_DEBUG("AOT raiseExternal: Policy does NOT have autoforward capability");
        }

        externalQueue_.raise(eventWithMetadata);

        // W3C SCXML 5.10.1: Mark next event as external for _event.type (test331)
        if constexpr (SCE::Core::HasExternalEventFlag<StatePolicy>) {
            policy_.nextEventIsExternal_ = true;
        }
    }

    /**
     * @brief Schedule an event for delayed delivery (W3C SCXML 6.2)
     *
     * Used by AOT-generated code for <send delay="..."> elements.
     *
     * @param event Event to schedule
     * @param delay Delay before delivery
     * @param sendId Optional sendid for cancellation
     * @param eventData Optional event data JSON
     * @return The sendid assigned to this event
     */
    std::string scheduleEvent(Event event, std::chrono::milliseconds delay, const std::string &sendId = "",
                              const std::string &eventData = "") {
        return scheduler_.scheduleEvent(event, delay, sendId, eventData);
    }

    /**
     * @brief Cancel a scheduled event (W3C SCXML 6.2.5)
     *
     * @param sendId Send ID to cancel
     * @return true if event was cancelled
     */
    bool cancelEvent(const std::string &sendId) {
        return scheduler_.cancelEvent(sendId);
    }

    /**
     * @brief Check if scheduler has ready events
     *
     * @return true if events are ready to fire
     */
    bool hasReadyEvents() const {
        return scheduler_.hasReadyEvents();
    }

    /**
     * @brief Run state machine until completion or timeout (W3C SCXML 6.2)
     *
     * Convenience API for running state machines with delayed send operations.
     * Internally polls the event scheduler and processes events until the state
     * machine reaches a final state or the timeout expires.
     *
     * This is the recommended API for simple use cases where you just want to
     * run the state machine to completion without manually managing the tick() loop.
     *
     * ARCHITECTURE NOTE: Polling Design Trade-off
     * - Uses sleep-based polling loop (pollInterval) for timer checks
     * - Trade-off: Simplicity and zero threading overhead vs. precise timer interrupts
     * - Rationale: "You don't pay for what you don't use" - no background threads, no timers
     * - Latency: Maximum delay = pollInterval (default 10ms) between event ready and processing
     * - For precise timing control: Use explicit tick() calls in custom event loop
     * - See ARCHITECTURE.md "All-or-Nothing Strategy" for AOT engine philosophy
     *
     * @param timeout Maximum time to wait for completion
     * @param pollInterval Interval between tick() calls (default: 10ms)
     * @return true if state machine reached final state, false if timeout
     *
     * @example Simple usage
     * @code
     * sm.initialize();
     * bool success = sm.runUntilCompletion(std::chrono::seconds(3));
     * if (success) {
     *     bool pass = (sm.getCurrentState() == SM::State::Pass);
     * }
     * @endcode
     */
    bool runUntilCompletion(std::chrono::milliseconds timeout,
                            std::chrono::milliseconds pollInterval = std::chrono::milliseconds(10)) {
        // W3C SCXML: If already stopped but reached final state during initialize(), return true
        // This handles tests like 580 where eventless transitions complete during initialization
        if (!isRunning_) {
            return isInFinalState();
        }

        auto startTime = std::chrono::steady_clock::now();

        while (!isInFinalState()) {
            // Check for timeout
            if (std::chrono::steady_clock::now() - startTime > timeout) {
                return false;  // Timeout
            }

            // Sleep briefly to allow scheduled events to become ready
            std::this_thread::sleep_for(pollInterval);

            // W3C SCXML 6.2: Poll scheduler and process events
            tick();
        }

        SCE_LOG_DEBUG("AOT runUntilCompletion: Exiting loop, isInFinalState()={}, getCurrentState()={}", isInFinalState(),
                  static_cast<int>(getCurrentState()));
        return true;  // Reached final state
    }

protected:
    /**
     * @brief Execute entry actions for a state (W3C SCXML 3.7)
     *
     * Entry actions are executable content that runs when entering a state.
     * This includes <onentry> blocks which may contain <raise>, <assign>, etc.
     *
     * Supports both static (stateless) and non-static (stateful) policies.
     * Static methods can also be called through an instance in C++.
     *
     * @param state State being entered
     */
    void executeOnEntry(State state) {
        // Call through policy instance (works for both static and non-static)
        policy_.executeEntryActions(state, *this);
    }

    /**
     * @brief Execute exit actions for a state (W3C SCXML 3.8)
     *
     * Exit actions are executable content that runs when exiting a state.
     * This includes <onexit> blocks.
     *
     * Supports both static (stateless) and non-static (stateful) policies.
     * Static methods can also be called through an instance in C++.
     *
     * @param state State being exited
     */
    void executeOnExit(State state, const std::vector<State> &activeStatesBeforeTransition) {
        // Call through policy instance with pre-transition active states
        policy_.executeExitActions(state, *this, activeStatesBeforeTransition);
    }

    /**
     * @brief Process both internal and external event queues (W3C SCXML D.1 Algorithm)
     *
     * Processes all queued internal and external events in priority order.
     * Internal events are processed first (high priority), then external events.
     *
     * W3C SCXML C.1 (test189): Internal queue (#_internal target) has higher
     * priority than external queue (no target or external targets).
     *
     * Uses shared EventProcessingAlgorithms for W3C-compliant processing.
     * This ensures Interpreter and AOT engines use identical logic.
     *
     * Supports both static (stateless) and non-static (stateful) policies.
     * Static methods can also be called through an instance in C++.
     */
    void processEventQueues() {
        SCE_LOG_DEBUG("AOT processEventQueues: Starting internal queue processing");
        // W3C SCXML C.1: Process internal queue first (high priority)
        SCE::Core::AOTEventQueue<EventWithMetadata> internalAdapter(internalQueue_);
        SCE::Core::EventProcessingAlgorithms::processInternalEventQueue(
            internalAdapter, [this](const EventWithMetadata &eventWithMeta) {
                Event event = eventWithMeta.event;
                SCE::Common::EventMetadataHelper::populatePolicyFromMetadata<StatePolicy, Event>(policy_,
                                                                                                 eventWithMeta);

                SCE_LOG_DEBUG("AOT processEventQueues: Processing internal event, currentState={}",
                          static_cast<int>(currentState_));

                // W3C SCXML 5.4.1: Stop processing events if TOP-LEVEL final state reached
                if (StatePolicy::isFinalState(currentState_)) {
                    auto parent = StatePolicy::getParent(currentState_);
                    if (!parent.has_value()) {
                        SCE_LOG_DEBUG("AOT processEventQueues: Top-level final state {} reached, stopping event processing",
                                  static_cast<int>(currentState_));
                        return false;
                    }
                    SCE_LOG_DEBUG("AOT processEventQueues: Non-top-level final state {} (inside parallel/compound), "
                              "continue processing done.state events",
                              static_cast<int>(currentState_));
                }

                executeTransition(event);
                return true;  // Continue processing
            });

        // W3C SCXML C.1: Process external queue second (low priority)
        SCE::Core::AOTEventQueue<EventWithMetadata> externalAdapter(externalQueue_);
        SCE::Core::EventProcessingAlgorithms::processInternalEventQueue(
            externalAdapter, [this](const EventWithMetadata &eventWithMeta) {
                Event event = eventWithMeta.event;
                SCE::Common::EventMetadataHelper::populatePolicyFromMetadata<StatePolicy, Event>(policy_,
                                                                                                 eventWithMeta);

                // W3C SCXML 6.5: Execute finalize BEFORE processing child events
                if constexpr (SCE::Core::HasFinalize<StatePolicy, EventWithMetadata,
                                                     StaticExecutionEngine<StatePolicy>>) {
                    policy_.executeFinalizeForChildEvent(eventWithMeta, *this);
                }

                executeTransition(event);
                return true;  // Continue processing
            });
    }

    /**
     * @brief Check for eventless transitions (W3C SCXML 3.13)
     *
     * Eventless transitions have no event attribute and are evaluated
     * immediately after entering a state. They are checked after all
     * internal events have been processed.
     *
     * Uses shared EventProcessingAlgorithms for W3C-compliant processing.
     * This ensures Interpreter and AOT engines use identical logic.
     *
     * Uses iteration instead of recursion to prevent stack overflow
     * and includes loop detection to prevent infinite cycles.
     */
    void checkEventlessTransitions() {
        SCE_LOG_DEBUG("AOT checkEventlessTransitions: Starting");
        static const int MAX_ITERATIONS = 100;  // Safety limit
        int iterations = 0;

        // W3C SCXML 3.13: Use shared algorithm (Single Source of Truth)
        // Note: Eventless transitions can raise new internal events, use internal queue
        SCE::Core::AOTEventQueue<EventWithMetadata> adapter(internalQueue_);

        while (iterations++ < MAX_ITERATIONS) {
            State oldState = currentState_;
            std::vector<State> preTransitionStates = getActiveStates();  // W3C SCXML 3.11: Capture before transition
            SCE_LOG_DEBUG("AOT checkEventlessTransitions: Iteration {}, currentState={}", iterations,
                      static_cast<int>(currentState_));

            // Call processTransition with default event for eventless transitions
            if (policy_.processTransition(currentState_, Event(), *this)) {
                // W3C SCXML 3.4: For parallel states, use actual transition source state
                State actualSourceState = policy_.lastTransitionSourceState_;
                SCE_LOG_DEBUG("AOT checkEventlessTransitions: Transition taken from {} to {} (actual source: {})",
                          static_cast<int>(oldState), static_cast<int>(currentState_),
                          static_cast<int>(actualSourceState));
                if (oldState != currentState_) {
                    // W3C SCXML Appendix D: For parallel states, executeMicrostep already handled exit/transition/entry
                    // Only call handleHierarchicalTransition for non-parallel state machines
                    if constexpr (!StatePolicy::HAS_PARALLEL_STATES) {
                        // ARCHITECTURE.MD: Zero Duplication - use shared helper
                        // W3C SCXML 3.13: Pass transition action callback for correct execution order
                        // W3C SCXML 3.4: Use actualSourceState for correct hierarchical exit/entry
                        handleHierarchicalTransition(actualSourceState, currentState_, preTransitionStates,
                                                     [this] { policy_.executeTransitionActions(*this); });
                    } else {
                        SCE_LOG_DEBUG("AOT checkEventlessTransitions: Parallel state machine - executeMicrostep handled "
                                  "all transitions");
                    }

                    // W3C SCXML C.1: Internal events are processed AFTER stable configuration is reached
                    // Continue loop to check for more eventless transitions first
                } else {
                    // Transition taken but state didn't change - stop
                    break;
                }
            } else {
                // W3C SCXML C.1: No eventless transition available - stable configuration reached
                // Internal events will be processed by caller (processEventQueues or step)
                break;
            }
        }

        if (iterations >= MAX_ITERATIONS) {
            // Eventless transition loop detected
            SCE_LOG_ERROR("StaticExecutionEngine: Eventless transition loop detected after {} iterations - stopping state "
                      "machine",
                      MAX_ITERATIONS);
            stop();
        }

        // W3C SCXML 3.13: Check if we reached a top-level final state after eventless transitions
        // For parallel states, check if any active state is a top-level final state
        if constexpr (StatePolicy::HAS_PARALLEL_STATES) {
            auto activeStates = getActiveStates();
            for (const auto &state : activeStates) {
                if (StatePolicy::isFinalState(state) && StatePolicy::getParent(state) == std::nullopt) {
                    SCE_LOG_INFO("AOT checkEventlessTransitions: Reached top-level final state {}, halting processing (W3C "
                             "SCXML 3.13)",
                             static_cast<int>(state));
                    currentState_ = state;  // W3C SCXML: Update currentState_ for getCurrentState()
                    SCE_LOG_DEBUG("AOT checkEventlessTransitions: After update, getCurrentState() = {}",
                              static_cast<int>(getCurrentState()));
                    stop();
                    break;
                }
            }
        } else {
            // For non-parallel states, check currentState_
            if (StatePolicy::isFinalState(currentState_) && StatePolicy::getParent(currentState_) == std::nullopt) {
                SCE_LOG_INFO("AOT checkEventlessTransitions: Reached top-level final state {}, halting processing (W3C "
                         "SCXML 3.13)",
                         static_cast<int>(currentState_));
                stop();
            }
        }
    }

public:
    StaticExecutionEngine() : currentState_(StatePolicy::initialState()) {}

    /**
     * @brief Initialize state machine (W3C SCXML 3.2)
     *
     * Performs the initial configuration:
     * 1. Enter initial state (with hierarchical entry from root to leaf)
     * 2. Execute entry actions (may raise internal events)
     * 3. Process internal event queue
     * 4. Check for eventless transitions
     */
    void initialize() {
        isRunning_ = true;

        // W3C SCXML 5.3: Initialize datamodel before any state entry
        // This ensures error.execution events are raised immediately if initialization fails
        if constexpr (SCE::Core::HasDataModelInit<StatePolicy, StaticExecutionEngine>) {
            policy_.initializeDataModel(*this);
        }

        // W3C SCXML 3.3: Use HierarchicalStateHelper for correct entry order
        auto entryChain = SCE::Core::HierarchicalStateHelper<StatePolicy>::buildEntryChain(currentState_);

        // Execute entry actions from root to leaf (ancestor first)
        for (const auto &state : entryChain) {
            executeOnEntry(state);
        }

        // W3C SCXML C.1: Macrostep completion loop
        // Process eventless transitions and internal events until stable configuration
        SCE_LOG_DEBUG("AOT initialize: After entry actions, starting macrostep completion loop");
        while (true) {
            // Process eventless transitions until stable
            checkEventlessTransitions();

            // Check if there are internal events to process
            if (!internalQueue_.hasEvents() && !externalQueue_.hasEvents()) {
                // Truly stable - no eventless transitions and no events
                break;
            }

            // Process internal/external events (may raise more events or cause transitions)
            processEventQueues();
        }
        SCE_LOG_DEBUG("AOT initialize: Macrostep completion loop finished - stable configuration reached");

        // W3C SCXML 6.4: Execute pending invokes after macrostep completes (ARCHITECTURE.md Zero Duplication)
        // Only invokes in entered-and-not-exited states execute (cancellation handled during state exits)
        if constexpr (SCE::Core::HasInvokeSupport<StatePolicy, StaticExecutionEngine>) {
            policy_.executePendingInvokes(*this);

            // W3C SCXML 6.4: Process done.invoke events raised by immediately-completed children
            // Child state machines may reach final state during initialization and raise done.invoke
            // These events must be processed to allow parent transitions (e.g., s1 -> pass)
            SCE_LOG_DEBUG("AOT initialize: Processing events raised by completed invokes");
            processEventQueues();
            checkEventlessTransitions();
        }

        // W3C SCXML 6.4: Invoke completion callback if in final state after initialization
        // Child state machines may reach final state immediately (e.g., initial="subFinal")
        // and must notify parent to generate done.invoke event
        if (isInFinalState() && completionCallback_) {
            SCE_LOG_DEBUG("AOT initialize: Reached final state during initialization, invoking completion callback");
            // W3C SCXML 3.8: Execute onexit actions for final state before notifying parent
            std::vector<State> activeStates = getActiveStates();
            executeOnExit(currentState_, activeStates);
            completionCallback_();
        }
    }

    /**
     * @brief Step the state machine (process pending events)
     *
     * W3C SCXML 6.4: For parent-child communication, parents must explicitly
     * step child state machines after sending events to ensure synchronous processing.
     *
     * This method processes all pending events in both internal and external queues.
     */
    void step() {
        processEventQueues();
        checkEventlessTransitions();

        // W3C SCXML 6.4: Invoke completion callback if in final state
        if (isInFinalState() && completionCallback_) {
            SCE_LOG_DEBUG("AOT step: Invoking completion callback for done.invoke");
            completionCallback_();
        }
    }

    /**
     * @brief Process an external event (W3C SCXML 3.12)
     *
     * External events are processed after all internal events have been
     * consumed. Each external event triggers a macrostep.
     *
     * Supports both static (stateless) and non-static (stateful) policies.
     * Static methods can also be called through an instance in C++.
     *
     * @param event External event to process
     */
    void processEvent(Event event) {
        if (!isRunning_) return;
        processEventImpl(event);
    }

    /**
     * @brief Process an external event with metadata (W3C SCXML 5.10)
     *
     * External events with metadata support originSessionId for invoke finalize.
     * Used when events come from child sessions via invoke.
     *
     * @param event External event to process
     * @param metadata Event metadata (originSessionId, etc.)
     */
    void processEvent(Event event, const SCE::Core::EventMetadata &metadata) {
        if (!isRunning_) return;
        policy_.currentEventMetadata_ = metadata;
        processEventImpl(event);
    }

    /**
     * @brief Get current state
     * @return Current active state
     */
    State getCurrentState() const {
        return currentState_;
    }

    /**
     * @brief Get all active states (W3C SCXML 3.11)
     *
     * For simple state machines (no parallel), returns vector with single current state hierarchy.
     * For parallel state machines, returns all active states across all parallel regions.
     *
     * Used by history recording logic and parallel completion checks.
     *
     * @return Vector of currently active states
     */
    std::vector<State> getActiveStates() const {
        // W3C SCXML 3.4: For parallel state machines, use policy's activeStates_ tracking
        if constexpr (StatePolicy::HAS_PARALLEL_STATES) {
            if constexpr (SCE::Core::HasActiveStates<StatePolicy>) {
                return policy_.getActiveStates();
            }
        }

        // W3C SCXML 3.11: For non-parallel, use shared HistoryHelper for full active hierarchy (Zero Duplication
        // Principle) Returns [currentState, parent, grandparent, ...] for proper history recording
        return ::SCE::Core::HistoryHelper::getActiveHierarchy(currentState_,
                                                        [](State s) { return StatePolicy::getParent(s); });
    }

    /**
     * @brief Check if in a final state (W3C SCXML 3.3)
     * @return true if current state is final
     */
    bool isInFinalState() const {
        return StatePolicy::isFinalState(currentState_);
    }

    /**
     * @brief Check if state machine is running
     * @return true if running (not stopped or completed)
     */
    bool isRunning() const {
        return isRunning_;
    }

    /**
     * @brief Stop state machine execution
     */
    void stop() {
        isRunning_ = false;
    }

    /**
     * @brief Tick scheduler and process ready internal events (W3C SCXML 6.2)
     *
     * For single-threaded AOT engines with delayed send support.
     * This method polls the event scheduler and processes any ready scheduled events.
     * Should be called periodically in a polling loop to allow delayed sends to fire
     * at the correct time.
     *
     * Implementation: Checks scheduler for ready events, raises them to external queue,
     * then processes event queues and checks for eventless transitions.
     */
    void tick() {
        if (!isRunning_) {
            return;
        }

        // W3C SCXML 6.4: If already in final state, notify parent and return
        if (isInFinalState()) {
            if (completionCallback_) {
                SCE_LOG_DEBUG("AOT tick: Invoking completion callback for already-final state");
                completionCallback_();
            }
            return;
        }

        // W3C SCXML 6.2: Check for ready scheduled events and raise them
        std::string eventData;
        Event event;
        while (scheduler_.popReadyEvent(event, eventData)) {
            raiseExternal(event, eventData);
        }

        // W3C SCXML 6.4: Tick child state machines to process their events
        // Children need to run independently during parent's event loop
        if constexpr (SCE::Core::HasChildTick<StatePolicy, StaticExecutionEngine>) {
            policy_.tickChildren(*this);
        }

        // Zero Duplication: Delegate event processing + completion callback to step()
        // step() handles: processEventQueues() + checkEventlessTransitions() + completionCallback_
        step();

        // W3C SCXML 6.4: Execute pending invokes after stable configuration is reached
        // Macrostep has completed - entered-and-not-exited states ready for invoke execution
        if constexpr (SCE::Core::HasInvokeSupport<StatePolicy, StaticExecutionEngine>) {
            policy_.executePendingInvokes(*this);
        }
    }

    /**
     * @brief Set completion callback for done.invoke event generation (W3C SCXML 6.4)
     *
     * This callback is invoked when the state machine reaches a final state.
     * Used by parent to generate done.invoke.{id} events.
     *
     * @param callback Function to call on completion (nullptr to clear)
     */
    void setCompletionCallback(std::function<void()> callback) {
        completionCallback_ = callback;
    }

    /**
     * @brief Set HTTP send callback for BasicHTTPEventProcessor (W3C SCXML C.2)
     *
     * Matches Kotlin StateMachineEngine.onHttpSend pattern.
     * Generated code calls performHttpSend() which delegates to this callback.
     * The test harness or application provides the actual HTTP transport.
     *
     * @param callback Function receiving HttpSendRequest (nullptr to clear)
     */
    void setHttpSendCallback(std::function<void(const HttpSendRequest &)> callback) {
        onHttpSend_ = std::move(callback);
    }

    /**
     * @brief Dispatch BasicHTTP send via callback (W3C SCXML C.2)
     *
     * Called by AOT-generated code for BasicHTTPEventProcessor sends.
     * Delegates to onHttpSend_ callback set by test harness or application.
     * Matches Kotlin StateMachineEngine.performHttpSend() pattern.
     */
    void performHttpSend(const std::string &target, const std::string &eventName,
                         const std::string &content,
                         const std::map<std::string, std::vector<std::string>> &params,
                         const std::string &sendId) {
        if (onHttpSend_) {
            onHttpSend_(HttpSendRequest{target, eventName, content, params, sendId});
        }
    }

    /**
     * @brief Get access to policy for parameter passing (W3C SCXML 6.4)
     *
     * Used by parent state machines to pass invoke parameters to child state machines.
     * Allows setting datamodel variables before calling initialize().
     *
     * @return Reference to policy instance
     */
    StatePolicy &getPolicy() {
        return policy_;
    }
};

}  // namespace SCE::Static
