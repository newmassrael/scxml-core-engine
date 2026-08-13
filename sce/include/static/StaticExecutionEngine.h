// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/CompilerHints.h"
#include "common/EventMetadataHelper.h"
#include "common/EventTypeHelper.h"
#include "common/ForwardedEvent.h"
#include "common/IOProcessorHelper.h"
#include "common/SCXMLConstants.h"
#include "common/SendHelper.h"
#include "common/SendSchedulingHelper.h"
#include "core/AOTEventQueue.h"
#include "core/EventMetadata.h"
#include "core/EventProcessingAlgorithms.h"
#include "core/EventQueueManager.h"
#include "core/HierarchicalStateHelper.h"
#include "core/HistoryHelper.h"
#include "core/LogMacros.h"
#include "core/StatePolicyConcepts.h"
#include "events/EventDescriptor.h"
#include <algorithm>
#include <any>
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

/// §scxml-C-2: HTTP send request data for BasicHTTPEventProcessor callback.
/// Matches Kotlin HttpSendRequest — transport-agnostic data struct passed to onHttpSend callback.
struct HttpSendRequest {
    std::string target;
    std::string eventName;
    std::string content;
    std::map<std::string, std::vector<std::string>> params;
    std::string sendId;
};

/// Mesh send callback signature: (target, eventName, data, sendId, invokeId)
/// → accepted. Kept as raw fields so StaticExecutionEngine.h has zero
/// mesh-layer includes — the generated TransportRouter's wireTo() lambda
/// builds the wire-format envelope from these fields, keeping CBOR / UUID /
/// envelope concerns in the mesh layer where they belong.
///
/// `invokeId` carries `_event.invokeid` verbatim from the triggering event's
/// metadata (§scxml-5.10.1). The mesh lambda parses it as a UUID and
/// stamps `MeshEnvelope.invoke_id` only for patterns that carry correlation
/// (`RpcReply`); for every other pattern the invokeId is ignored. This
/// mirrors §scxml-6.4.1's auto-propagation of `_event.invokeid` from
/// child-to-parent sends and extends the same semantics to mesh-rpc replies.
using MeshSendCallback =
    std::function<bool(const std::string &target, const std::string &eventName, const std::string &data,
                       const std::string &sendId, const std::string &invokeId)>;

/// Mesh-rpc invoke callback signature: (target, fieldSuffix, invokeId, data) → accepted.
///
/// Fires when a state with `<invoke type="sce:mesh-rpc">` is entered. The
/// generated TransportRouter installs a lambda that constructs a
/// `MeshEnvelope` with a fresh UUID v7 as `invoke_id`, registers a deliver
/// callback in the `InvokeCorrelation` table keyed on that UUID, optionally
/// arms a deadline timer, and dispatches the request to the matching
/// transport. Raw fields (no mesh-layer types) preserve the "engine has zero
/// mesh includes" invariant in the same way `MeshSendCallback` does.
///
/// * `target`       — SCXML `src="#..."` (e.g. `"#motor"`)
/// * `fieldSuffix`  — codegen-generated identifier (e.g. `"invoke_0"`)
/// * `invokeId`     — SCXML-side invoke id, used to raise
///                    `done.invoke.<invokeId>` / `error.invoke.<invokeId>`
/// * `data`         — JSON-encoded param payload (reserved `_mesh_*`
///                    names already stripped by the parser)
using MeshInvokeCallback = std::function<bool(const std::string &target, const std::string &fieldSuffix,
                                              const std::string &invokeId, const std::string &data)>;

/// Mesh-rpc cancel callback signature: (target, fieldSuffix) → accepted.
///
/// Fires from onexit when a state with a pending mesh-rpc invoke is left
/// before the reply arrives. SCE_MESH.md §mesh-9.5: `<cancel>` semantics for
/// mesh-rpc erase the correlation entry without raising `done`/`error`.
/// Takes `(target, fieldSuffix)` — the router's `active_invokes_` map
/// translates the pair to the UUID of the latest registration so the
/// correlation table and deadline scheduler can be cleared together.
using MeshCancelCallback = std::function<bool(const std::string &target, const std::string &fieldSuffix)>;

/// SCXML remote-invoke start callback signature: (target, invokeId, data) → accepted.
///
/// Fires from onentry when a state with `<invoke type="scxml" src="#peer">`
/// is entered and the classifier marked it as remote-mesh (SCE_MESH.md §mesh-9.6).
/// Returns true when the TransportRouter accepted the wire-14 `InvokeStart`
/// envelope for outbound dispatch; false when no callback is installed
/// (document rendered without TransportRouter wiring) so the generated code
/// can fall through to the transport-absent local raise per §mesh-9.6 line 1396.
/// * `target`         — deploy.yaml machine name (e.g. `"worker_session_f"`)
/// * `invokeIdString` — SCXML-side invoke id (W3C 3.12.1 `stateid.ptr.index`),
///                      preserved so the eventual `error.execution` raise
///                      can surface it via `_event.invokeid` when wire-15
///                      `InvokeStarted` / wire-20 `InvokeError` reply.
/// * `data`           — opaque payload bytes the parent wants the child to
///                      receive at session creation (reserved for the §mesh-9.6.2
///                      inner `{src, params, content, namelist, autoforward}`
///                      CBOR map once wires 15-19 activate consumers).
using ScxmlInvokeStartCallback =
    std::function<bool(const std::string &target, const std::string &invokeIdString, const std::string &data)>;

/// SCXML remote-invoke parent-event callback signature: wire-17 `ParentEvent`
/// per SCE_MESH.md §mesh-9.6.2. Fires from the parent engine's autoforward path
/// (`forwardToAutoforwardChildren` in invoke_methods.jinja2) when an
/// external event must be forwarded to an active remote invoke's child.
/// Returns true when the TransportRouter accepted the wire-17 envelope for
/// outbound dispatch; false when no callback is installed (no remote
/// autoforward authored for this machine).
///
/// * `target`         — child's deploy.yaml machine name
/// * `invokeIdString` — SCXML-side invoke id (identifies the child session)
/// * `eventName`      — event being forwarded (per §scxml-6.4 verbatim)
/// * `data`           — event data payload (JSON-encoded when present)
/// * `sendId`         — original sendId (preserved per §mesh-9.6.3); empty when
///                      the forwarded event had no explicit sendid.
using ScxmlInvokeParentEventCallback =
    std::function<bool(const std::string &target, const std::string &invokeIdString, const std::string &eventName,
                       const std::string &data, const std::string &sendId)>;

/// SCXML remote-invoke cancel callback signature: wire-19 `InvokeCancel`
/// per SCE_MESH.md §mesh-9.6.2. Fires when the parent exits the invoking state
/// of a still-active remote invoke. Returns true when the TransportRouter
/// accepted the wire-19 envelope for outbound dispatch; false when no
/// callback is installed.
using ScxmlInvokeCancelCallback = std::function<bool(const std::string &target, const std::string &invokeIdString)>;

/**
 * @brief Template-based SCXML execution engine for static code generation
 *
 * This engine implements the core SCXML execution semantics (event queue management,
 * entry/exit actions, transitions) while delegating state-specific logic to the
 * StatePolicy template parameter.
 *
 * Key SCXML standards implemented:
 * - Internal event queue with FIFO ordering (§scxml-3.13)
 * - Entry/exit action execution (§scxml-3.8, 3.9)
 * - Event processing loop (§scxml-D-mainEventLoop)
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
    static_assert(
        requires(StatePolicy p) {
            { p.lastTransitionIsInternal_ } -> std::convertible_to<bool>;
        }, "StatePolicy must have member: mutable bool lastTransitionIsInternal_");
    static_assert(
        requires(StatePolicy p) {
            { p.lastTransitionIsTargetless_ } -> std::convertible_to<bool>;
        }, "StatePolicy must have member: mutable bool lastTransitionIsTargetless_");
    static_assert(
        requires(StatePolicy p) {
            { p.lastTransitionSourceState_ } -> std::convertible_to<typename StatePolicy::State>;
        }, "StatePolicy must have member: mutable State lastTransitionSourceState_");
#endif

public:
    using State = typename StatePolicy::State;
    using Event = typename StatePolicy::Event;

    /**
     * @brief Event with metadata for §scxml-5.10 compliance
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
        std::string origin;                    // §scxml-5.10.1: _event.origin
        std::string sendId;                    // §scxml-5.10.1: _event.sendid
        std::string type;                      // §scxml-5.10.1: _event.type
        std::string originType;                // §scxml-5.10.1: _event.origintype
        std::string invokeId;                  // §scxml-5.10.1: _event.invokeid
        std::string target;                    // §scxml-C-2: HTTP POST target URL
        std::optional<ScriptValue> typedData;  // §scxml-5.5: Engine-agnostic typed event data
        // NL→IR Item C1 Path A (EventSchema native lowering): typed
        // `_event.data` payload riding with the event through the queue — a
        // generated per-event payload struct, type-erased in std::any (the
        // C++-idiomatic twin of the Rust runtime's statically-typed
        // EventWithMetadata<E,P>.payload and the Go `TypedPayload any`
        // carrier). The generated policy's populateTypedPayload() any_casts it
        // into its typed pending<Event>Payload_ field, which the natively-
        // lowered transition guards read (no script engine). Empty for every
        // event raised without a typed payload, against which the native
        // guards' tag check fails. The name↔type pairing is enforced at the
        // single generated raise<Event>() inject seam.
        std::any typedPayload;

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
     * ARCHITECTURE.md: Extract duplicate code from the event-queue drains
     * §scxml-3.13: Compute LCA and execute hierarchical exit/entry
     *
     * @param oldState State before transition
     * @param newState State after transition
     * @param preTransitionStates Active states before transition (for history recording)
     */
    // C++17-compatible named empty callable (replaces decltype([] {}))
    struct NoOpAction {
        void operator()() const {}
    };

    template <typename TransitionActionFn = NoOpAction>
    void handleHierarchicalTransition(State oldState, State newState, const std::vector<State> &preTransitionStates,
                                      TransitionActionFn &&transitionAction = {}) {
        SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Transition {} -> {}", static_cast<int>(oldState),
                      static_cast<int>(newState));

        // §scxml-3.13: Determine LCA based on transition type
        std::optional<State> lca;
        if (policy_.lastTransitionIsInternal_) {
            // §scxml-3.13: Internal transitions whose target is NOT a proper descendant behave as external
            bool isSelfTransition = (oldState == newState);
            bool isProperDescendant =
                !isSelfTransition &&
                SCE::Core::HierarchicalStateHelper<StatePolicy>::isDescendantOf(newState, oldState);

            // §scxml-3.13: Check if source is compound state (test 533)
            // Parallel states and atomic states are NOT compound - internal transitions from them behave as external
            bool isSourceCompound = StatePolicy::isCompoundState(oldState);

            if (isProperDescendant && isSourceCompound) {
                // §scxml-3.13: Internal transition to proper descendant in compound state - source is LCA (don't
                // exit source)
                lca = oldState;  // Source is the LCA - don't exit it
                SCE_LOG_DEBUG(
                    "AOT handleHierarchicalTransition: Internal transition (proper descendant, compound source) "
                    "- source {} is LCA",
                    static_cast<int>(oldState));
            } else {
                // §scxml-3.13: Non-compound source or non-descendant - behaves as external
                // Use normal LCA calculation, then target==LCA check handles exit/re-entry
                lca = SCE::Core::HierarchicalStateHelper<StatePolicy>::findLCA(oldState, newState);
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Internal transition (non-compound source or "
                              "non-descendant) - behaves as "
                              "external, LCA={}",
                              lca.has_value() ? static_cast<int>(lca.value()) : -1);
            }
        } else {
            // §scxml-3.13: External transition - find LCA normally
            lca = SCE::Core::HierarchicalStateHelper<StatePolicy>::findLCA(oldState, newState);
        }

        if (lca.has_value()) {
            // §scxml-3.13: First exit any active descendants of oldState (deepest first)
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

            // §scxml-3.13: Exit states from oldState up to (but not including) LCA
            auto exitChain = SCE::Core::HierarchicalStateHelper<StatePolicy>::buildExitChain(oldState, lca.value());
            for (const auto &state : exitChain) {
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Hierarchical exit state {}", static_cast<int>(state));
                executeOnExit(state, preTransitionStates);
            }

            // §scxml-3.10 (test 579): Ancestor transition (target == LCA)
            // When transitioning to self or ancestor, the target must also be exited and re-entered
            // This is how Interpreter handles internal self-transitions to satisfy W3C 5.9.2
            bool isTargetActive = std::find(preTransitionStates.begin(), preTransitionStates.end(), newState) !=
                                  preTransitionStates.end();
            if (newState == lca.value() && isTargetActive) {
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Ancestor/self transition - exit target {} (W3C 3.10)",
                              static_cast<int>(newState));
                executeOnExit(newState, preTransitionStates);
            }

            // §scxml-3.13: Execute transition actions AFTER exit, BEFORE entry
            SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Executing transition actions");
            transitionAction();

            // §scxml-3.13: Enter states from LCA down to newState (including initial children)
            std::vector<State> entryChain;

            // §scxml-3.10: If target == LCA (ancestor/self transition), enter full subtree from target
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

            // §scxml-3.11: Update currentState to deepest entered state
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

            // §scxml-3.13: Execute transition actions AFTER exit, BEFORE entry
            SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Executing transition actions (no LCA)");
            transitionAction();

            // Enter full hierarchy from root to newState
            auto entryChain = SCE::Core::HierarchicalStateHelper<StatePolicy>::buildEntryChain(newState, policy_);
            for (const auto &state : entryChain) {
                SCE_LOG_DEBUG("AOT handleHierarchicalTransition: Entry state {} (from root)", static_cast<int>(state));
                executeOnEntry(state);
            }

            // §scxml-3.11: Update currentState to deepest entered state
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
     * @param event Event to process
     * @return true if a hierarchical state change occurred
     */
    bool executeTransition(Event event) {
        return executeTransition(event, [] {});
    }

    /**
     * @brief Execute a state transition with hierarchical exit/entry handling
     *
     * §scxml-3.13: Single Source of Truth for transition execution across all
     * processing paths (event queues, direct processEvent, eventless transitions).
     *
     * Callers customize one axis of variation via a template callback:
     * - postTransition: work after hierarchical handling, before eventless check
     *   (queue path: nothing; direct path: runMainEventLoop)
     *
     * W3C SCXML Appendix D: who performs exit/entry is *not* an axis of
     * variation, because it is fixed by the document's shape rather than by the
     * caller. A policy with parallel states routes every event — external and
     * eventless alike — through `executeMicrostep`, which owns the whole
     * exit → transition content → entry sequence and maintains `activeStates_`
     * itself. Any exit/entry this function adds on top of that is a second
     * application: `executeExitActions` removes its argument from the
     * configuration, so re-exiting `oldState` after the microstep has already
     * re-entered it drops that region's leaf while leaving the region's
     * ancestors active. The region then holds no atomic state and can never
     * fire again. `checkEventlessTransitions` already states this invariant for
     * its own path; expressing it once, here, is what keeps the two paths from
     * disagreeing.
     *
     * @tparam PostTransitionFn Callable() for post-hierarchical work
     * @param event Event to process
     * @param postTransition Post-hierarchical work before eventless check
     * @return true if a hierarchical state change occurred
     */
    template <typename PostTransitionFn> bool executeTransition(Event event, PostTransitionFn &&postTransition) {
        State oldState = currentState_;
        std::vector<State> preTransitionStates = getActiveStates();
        if (!policy_.processTransition(currentState_, event, *this)) {
            return false;
        }

        // §scxml-3.13: Self-transitions (target = source) exit and re-enter the state
        // §scxml-3.13: Targetless transitions consume event only (no exit/enter)
        bool isSelfTransition = (oldState == currentState_);
        bool needsHierarchicalHandling =
            (oldState != currentState_) || (isSelfTransition && !policy_.lastTransitionIsTargetless_);

        if (!needsHierarchicalHandling) {
            // §scxml-3.13: Targetless transition - execute actions without state change
            policy_.executeTransitionActions(*this);
            return false;
        }

        // §scxml-3.13: State transition requires hierarchical exit/entry
        if constexpr (!StatePolicy::HAS_PARALLEL_STATES) {
            handleHierarchicalTransition(oldState, currentState_, preTransitionStates,
                                         [this] { policy_.executeTransitionActions(*this); });
        }
        // W3C SCXML Appendix D: a parallel policy needs no exit/entry here —
        // `executeMicrostep` has already exited, run the transition content and
        // entered. See this function's contract above for why adding to it
        // costs a region its leaf. What it does not do is settle
        // `currentState_`, which is the next statement's job.
        else {
            resolveCurrentStateToLeaf();
        }
        postTransition();
        checkEventlessTransitions();
        return true;
    }

    /**
     * @brief Settle `currentState_` on an atomic state
     *
     * `executeMicrostep` leaves `currentState_` on the last transition target
     * it processed, and a target may be a compound state. The configuration is
     * then right and this one field is not: `getCurrentState()` names a state
     * the machine is *within* rather than the atomic state it is *in*.
     *
     * Measured 2026-08-13 on a two-region `<parallel>` whose transition
     * targets a compound state: the active set was
     * `[run | counter | drive | within | outer | a]` and `getCurrentState()`
     * answered `outer`. `sce_rust_runtime`'s `resolve_current_state_to_leaf`
     * and the Go engine's `resolveCurrentStateToLeaf` both answer `a`, so C++
     * was the one backend of three disagreeing on a public accessor that 105
     * files in this repository read.
     *
     * The descent reads the CONFIGURATION rather than recomputing initial or
     * history children, which the other two backends do. That is deliberate:
     * `activeStates_` is what the microstep actually entered, so a descent
     * through it cannot disagree with what happened. Recomputing can — the
     * generated microstep builds its entry chain from plain initial children
     * while `getInitialOrHistoryChild` is history-aware, and the two answers
     * part company exactly when a `<history>` is involved.
     */
    void resolveCurrentStateToLeaf() {
        // A region holds one atomic state, so this descends once per level.
        // The bound is a cycle guard, not a depth policy: a parent chain that
        // loops would otherwise hang here rather than report anything.
        constexpr int MAX_DESCENTS = 50;
        const std::vector<State> active = getActiveStates();
        for (int depth = 0; depth < MAX_DESCENTS; ++depth) {
            if (!StatePolicy::isCompoundState(currentState_)) {
                return;
            }
            const auto child = std::find_if(active.begin(), active.end(), [this](State candidate) {
                const auto parent = StatePolicy::getParent(candidate);
                return parent.has_value() && parent.value() == currentState_;
            });
            // A compound state with no active child is not a configuration
            // this can repair, so it is left as it is rather than guessed at.
            if (child == active.end()) {
                return;
            }
            currentState_ = *child;
        }
        SCE_LOG_ERROR("AOT resolveCurrentStateToLeaf: exceeded {} descents from a compound state — "
                      "the parent chain does not terminate",
                      MAX_DESCENTS);
    }

    /**
     * @brief Shared implementation for processEvent overloads
     *
     * §scxml-3.13: External event processing with full macrostep completion.
     *
     * @param event Event to process
     */
    void processEventImpl(Event event) {
        bool stateChanged = executeTransition(event, [this] { runMainEventLoop(); });
        // §scxml-6.4: Notify parent only when the machine has globally
        // terminated. `isInFinalState()` adds the parent-presence check to
        // the structural `StatePolicy::isFinalState`, excluding a regional
        // `<final>` inside a `<parallel>` whose sibling regions may still be
        // running — the done.invoke contract in §scxml-6.4 fires at
        // top-level-final only.
        if (stateChanged && isInFinalState() && completionCallback_) {
            completionCallback_();
        }
    }

    State currentState_;
    SCE::Core::EventQueueManager<EventWithMetadata>
        internalQueue_;  // §scxml-3.13: Internal event queue (high priority)
    SCE::Core::EventQueueManager<EventWithMetadata> externalQueue_;  // §scxml-3.13: External event queue (low priority)
    bool isRunning_ = false;
    std::function<void()> completionCallback_;                 // §scxml-6.4: Callback for done.invoke
    std::function<void(const HttpSendRequest &)> onHttpSend_;  // §scxml-C-2: BasicHTTP callback
    MeshSendCallback onMeshSend_;                              // SCE Mesh: cross-machine <send> callback
    MeshInvokeCallback onMeshInvoke_;  // SCE Mesh §mesh-9.5: <invoke type="sce:mesh-rpc"> entry hook
    MeshCancelCallback onMeshCancel_;  // SCE Mesh §mesh-9.5: mesh-rpc exit / cancel hook
    ScxmlInvokeStartCallback
        onScxmlInvokeStart_;  // SCE Mesh §mesh-9.6.2 wire-14: <invoke type="scxml" src="#peer"> entry hook
    ScxmlInvokeParentEventCallback
        onScxmlInvokeParentEvent_;                   // SCE Mesh §mesh-9.6.2 wire-17: autoforward outbound hook
    ScxmlInvokeCancelCallback onScxmlInvokeCancel_;  // SCE Mesh §mesh-9.6.2 wire-19: remote invoke cancel hook
    // SCE Mesh §mesh-16.5 (rule 12) — `<parallel>` partition role hooks. Set
    // by the derived SM ctor when codegen materializes a Root or NonRoot
    // tracker / sender for a hosted `<parallel>`. The Policy invokes
    // them via `triggerParallelRegionLocalComplete` /
    // `triggerParallelRegionRemoteSend` from the parallel-final
    // dispatcher (`mesh/cpp/parallel_final.jinja2`); no-op when the
    // SM was generated without partition context (single-partition
    // builds leave both unset). Both take `string&` (not `string_view`)
    // so the closure's `[this]` capture can stash the values into
    // member containers without lifetime caveats.
    std::function<void(const std::string &parallel_id, const std::string &region_id)> onParallelRegionLocalComplete_;
    std::function<void(const std::string &parallel_id, const std::string &region_id, const std::string &donedata)>
        onParallelRegionRemoteSend_;
    std::string currentEventInvokeId_;     // SCE Mesh §mesh-9.5: invokeId of event being processed
    SCE::PullScheduler<Event> scheduler_;  // §scxml-6.2: Delayed event scheduler

    // §scxml-5.5 + 6.4.3: donedata payload stashed at top-level <final> entry.
    // Consumed by:
    //   - local invoke completion callback (invoke_methods.jinja2) to populate
    //     `done.invoke.<id>._event.data`;
    //   - SCE Mesh ChildSessionAdapter::getDonedata() (§mesh-9.6.2 wire-18) to carry
    //     the payload to the parent peer.
    // Shared single source of truth — no local vs remote divergence. Empty
    // string / nullopt when the top-level final had no `<donedata>` child.
    std::string pendingDonedataAtFinal_;
    std::optional<ScriptValue> pendingTypedDonedataAtFinal_;

protected:
    StatePolicy policy_;  // Policy instance for stateful policies

public:
    /**
     * @brief Raise an internal event with metadata (§scxml-3.13)
     *
     * Places event on the internal queue with FIFO ordering.
     * Internal events have higher priority than external events.
     *
     * @param metadata Complete event metadata including all §scxml-5.10.1 fields
     */
    void raise(EventWithMetadata metadata) {
        // §scxml-3.13: Enqueue event with metadata
        internalQueue_.raise(std::move(metadata));
    }

    /**
     * @brief Raise an external event (§scxml-3.13, 6.2)
     *
     * External events are placed at the back of the external event queue.
     * They are processed after all internal events have been consumed.
     *
     * Used by:
     * - <send> without target (§scxml-6.2)
     * - <send> with external targets (not #_internal)
     * - <send target="#_parent"> from child state machines (§scxml-6.2)
     *
     * §scxml-3.13 (test189): External queue has lower priority than internal queue.
     *
     * @param event Event to raise externally
     * @param eventData Optional event data as JSON string (§scxml-5.10)
     */
    void raiseExternal(Event event, const std::string &eventData = "", const std::string &origin = "",
                       const std::string &target = "") {
        // §scxml-3.13: Enqueue event with metadata (origin, data, sendid, type, originType, target)
        // Delegates to the full-metadata overload so that SCE Mesh target
        // routing and §scxml-6.4 autoforward both see the event. Prior
        // to this delegation the simple (datamodel="null") codepath dropped
        // the target attribute, which meant mesh-declared targets silently
        // hit the external queue instead of the mesh callback.
        EventWithMetadata meta(event, eventData, origin, "", "external", SCE::Constants::SCXML_EVENT_PROCESSOR_TYPE);
        meta.target = target;
        raiseExternal(meta);
    }

    /**
     * @brief Raise external event by name (§scxml-6.4)
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
     * @brief Raise an autoforwarded external event carrying its source
     *        `_event` fields (§scxml-6.4 exact-copy contract)
     *
     * The receiving end of `SCE::Common::ForwardedEvent`: the event crosses
     * the machine boundary by name because the sender's `Event` enum is a
     * different type (local invoke child) or lives on another device
     * (SCE_MESH.md §9.6.5 wire-17). Unknown names degrade silently — a child
     * is not required to declare every event its parent forwards
     * (§scxml-6.4).
     *
     * @param forwarded Source event's name and `_event` fields
     */
    void raiseExternal(const ::SCE::Common::ForwardedEvent &forwarded) {
        auto event = policy_.getEventFromName(forwarded.name);
        if (!event) {
            SCE_LOG_DEBUG("AOT raiseExternal: Forwarded event '{}' not in Event enum, ignoring", forwarded.name);
            return;
        }
        // `target` stays default-constructed: the forwarded copy is delivered
        // to this machine, never re-routed to the original event's target.
        EventWithMetadata meta(*event, forwarded.data, forwarded.origin, forwarded.sendId, forwarded.type,
                               forwarded.originType, forwarded.invokeId);
        raiseExternal(meta);
    }

    /**
     * @brief Raise external event with full metadata (§scxml-6.4.1)
     *
     * Used for child-to-parent communication where invokeid must be preserved.
     * §scxml-6.4.1 (test338): Events from child to parent must include invokeid.
     *
     * @param eventWithMetadata Event with metadata (including invokeid)
     */
    void raiseExternal(const EventWithMetadata &eventWithMetadata) {
        // SCE Mesh: route cross-machine targets through the mesh callback before
        // they reach the external queue. Matches the §scxml-C-2 BasicHTTP
        // split — HTTP targets are dispatched via performHttpSend(), mesh
        // targets via performMeshSend(). Applications that do not wire a mesh
        // transport leave onMeshSend_ unset and the event falls through to
        // the external queue (legacy behavior, preserves W3C conformance).
        if (::SCE::SendHelper::isMeshTarget(eventWithMetadata.target)) {
            // SCE_MESH.md §mesh-9.5: the metadata's invokeId is authoritative
            // when set (send.jinja2 full-metadata path calls
            // engine.currentEventInvokeId() explicitly). The engine-level
            // field fills the gap for the simple raiseExternal(Event, ...)
            // overload (datamodel="null" <send>) which constructs
            // EventWithMetadata without invokeId. Stamping the engine
            // field into the metadata struct was rejected: it would leak
            // invokeId through self-sends that should carry empty
            // _event.invokeid per §scxml-5.10.1.
            const auto &invokeId =
                eventWithMetadata.invokeId.empty() ? currentEventInvokeId_ : eventWithMetadata.invokeId;
            if (performMeshSend(eventWithMetadata.target, policy_.getEventName(eventWithMetadata.event),
                                eventWithMetadata.data, eventWithMetadata.sendId, invokeId)) {
                return;  // handled by mesh transport — do not enqueue locally
            }
        }

        // §scxml-C-1: an event whose target names an accessible session —
        // spelled here as that session's published location — must go to
        // that session's external queue. Without this the address resolved
        // to nothing and the event landed back in the SENDER's queue — a
        // parent answering `_event.origin` sent to itself, which no
        // assertion in the corpus notices because test336 and test350 both
        // send to the session they already are.
        if constexpr (SCE::Core::HasChildSessionDelivery<StatePolicy>) {
            const std::string childSession =
                SCE::IOProcessorHelper::sessionIdFromScxmlLocation(eventWithMetadata.target);
            if (!childSession.empty()) {
                SCE::Common::ForwardedEvent addressed{policy_.getEventName(eventWithMetadata.event),
                                                      eventWithMetadata.data,
                                                      eventWithMetadata.origin,
                                                      eventWithMetadata.sendId,
                                                      eventWithMetadata.type,
                                                      eventWithMetadata.originType,
                                                      eventWithMetadata.invokeId};
                if (policy_.deliverToChildSession(childSession, addressed)) {
                    return;  // delivered to the addressed session
                }
            }
        }

        // Normal internal/external queue processing
        SCE_LOG_DEBUG("AOT raiseExternal: Enqueuing external event with metadata (event={}, invokeId='{}')",
                      static_cast<int>(eventWithMetadata.event), eventWithMetadata.invokeId);

        externalQueue_.raise(eventWithMetadata);

        // §scxml-5.10.1: Mark next event as external for _event.type (test331)
        if constexpr (SCE::Core::HasExternalEventFlag<StatePolicy>) {
            policy_.nextEventIsExternal_ = true;
        }
    }

    /**
     * @brief Schedule an event for delayed delivery (§scxml-6.2)
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
     * @brief Cancel a scheduled event (§scxml-6.3)
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
     * @brief Run state machine until completion or timeout (§scxml-6.2)
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

            // §scxml-6.2: Poll scheduler and process events
            tick();
        }

        SCE_LOG_DEBUG("AOT runUntilCompletion: Exiting loop, isInFinalState()={}, getCurrentState()={}",
                      isInFinalState(), static_cast<int>(getCurrentState()));
        return true;  // Reached final state
    }

protected:
    /**
     * @brief Execute entry actions for a state (§scxml-3.8)
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
     * @brief Execute exit actions for a state (§scxml-3.9)
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
     * @brief The §scxml-D-mainEventLoop outer loop
     *
     * The only place this engine's entry points (`initialize`, `step`, `tick`,
     * `processEventImpl`) express macrostep semantics. Appendix D names the
     * external queue exactly once per iteration and it is *after*
     * `invoke(inv)`:
     *
     * ```
     * while running:
     *     while running and not macrostepDone:      # eventless + internal only
     *         ... selectEventlessTransitions() / internalQueue.dequeue() ...
     *     for state in statesToInvoke.sort(entryOrder):
     *         for inv in state.invoke.sort(documentOrder):
     *             invoke(inv)
     *     statesToInvoke.clear()
     *     if not internalQueue.isEmpty(): continue
     *     externalEvent = externalQueue.dequeue()
     * ```
     *
     * Folding the external drain into the macrostep-completion loop instead is
     * a different algorithm, not a shorter one. The invoked children do not
     * exist yet while that drain runs, so everything `<onentry>` queued for
     * this session on the way in is consumed with no `autoforward` child to
     * receive it — and there is no later point at which it is delivered. One
     * external event per iteration for the same reason: a state entered by
     * event N's transition must have its invokes started before N+1 comes off
     * the queue.
     *
     * §scxml-3.13 (test189): the internal queue (`#_internal` target) keeps its
     * priority over the external one — the inner loop drains it to exhaustion
     * before the outer loop looks at an external event at all.
     */
    void runMainEventLoop() {
        while (true) {
            // §scxml-D-mainEventLoop: complete the macrostep on eventless
            // transitions and internal events alone.
            while (true) {
                checkEventlessTransitions();
                if (!internalQueue_.hasEvents()) {
                    break;
                }
                processInternalQueue();
            }

            if (!isRunning_ || isInFinalState()) {
                break;
            }

            // §scxml-6.4: invokes for states entered during this macrostep.
            if constexpr (SCE::Core::HasInvokeSupport<StatePolicy, StaticExecutionEngine>) {
                policy_.executePendingInvokes(*this);
            }

            // §scxml-D-mainEventLoop: invoking may have raised internal error
            // events (and a child that completed synchronously may already
            // have raised `done.invoke`); handle them before touching the
            // external queue.
            if (internalQueue_.hasEvents()) {
                continue;
            }

            if (!processNextExternalEvent()) {
                break;
            }
        }
    }

    /**
     * @brief Drain the internal event queue (§scxml-C-1, high priority)
     */
    void processInternalQueue() {
        SCE_LOG_DEBUG("AOT processInternalQueue: Starting internal queue processing");
        // §scxml-3.13: Process internal queue first (high priority)
        SCE::Core::AOTEventQueue<EventWithMetadata> internalAdapter(internalQueue_);
        SCE::Core::EventProcessingAlgorithms::processInternalEventQueue(
            internalAdapter, [this](const EventWithMetadata &eventWithMeta) {
                Event event = eventWithMeta.event;
                currentEventInvokeId_ = eventWithMeta.invokeId;
                SCE::Common::EventMetadataHelper::populatePolicyFromMetadata<StatePolicy, Event>(policy_,
                                                                                                 eventWithMeta);

                SCE_LOG_DEBUG("AOT processInternalQueue: Processing internal event, currentState={}",
                              static_cast<int>(currentState_));

                // §scxml-3.7: Stop processing events if TOP-LEVEL final state reached
                // (Zero Duplication: same top-level-final predicate as tick() — encapsulated
                // in isInFinalState() to keep regional `<final>` inside a `<parallel>`
                // from mis-terminating the queue drain.)
                if (isInFinalState()) {
                    SCE_LOG_DEBUG(
                        "AOT processInternalQueue: Top-level final state {} reached, stopping event processing",
                        static_cast<int>(currentState_));
                    return false;
                }
                if (StatePolicy::isFinalState(currentState_)) {
                    SCE_LOG_DEBUG("AOT processInternalQueue: Non-top-level final state {} (inside parallel/compound), "
                                  "continue processing done.state events",
                                  static_cast<int>(currentState_));
                }

                executeTransition(event);
                return true;  // Continue processing
            });
    }

    /**
     * @brief Take exactly one event off the external queue (§scxml-D-mainEventLoop)
     *
     * Runs the preliminary `<finalize>` / autoforward step against it, then
     * selects transitions. Returns false when the queue was empty.
     *
     * One event, not a drain: Appendix D returns to the top of the outer loop
     * after each external event, so a state entered by this event's transition
     * gets its invokes started before the next one is dequeued.
     */
    bool processNextExternalEvent() {
        if (!externalQueue_.hasEvents()) {
            return false;
        }
        const EventWithMetadata eventWithMeta = externalQueue_.pop();
        {
            Event event = eventWithMeta.event;
            currentEventInvokeId_ = eventWithMeta.invokeId;
            SCE::Common::EventMetadataHelper::populatePolicyFromMetadata<StatePolicy, Event>(policy_, eventWithMeta);

            // §scxml-6.5: Execute finalize BEFORE processing child events
            if constexpr (SCE::Core::HasFinalize<StatePolicy, EventWithMetadata, StaticExecutionEngine<StatePolicy>>) {
                policy_.executeFinalizeForChildEvent(eventWithMeta, *this);
            }

            // §scxml-D-mainEventLoop: autoforward belongs to the same
            // preliminary step as `<finalize>` above — both run against the
            // event this drain has just removed from the external queue, and
            // both run before transition selection. §scxml-6.4.2 fixes the
            // position in prose as well: the parent forwards "at the point at
            // which it removes it from the external event queue".
            //
            // Forwarding where the event is *enqueued* instead is a different
            // algorithm, not an earlier one. `raiseExternal` runs inside
            // whatever executable content produced the event, so a transition
            // body that queues two events hands the child both of them before
            // the parent has processed either — the child runs a whole event
            // ahead and the two sessions stop agreeing on what has happened.
            // Run-to-completion is a property of this loop's shape, so the
            // forward has to live in the loop.
            //
            // ARCHITECTURE.md Zero Duplication: Policy handles child forwarding (forwardToAutoforwardChildren)
            //
            // SCE_MESH.md §mesh-9.6.5: this is the "parent runtime observes
            // each external event" point the section names, and the copy
            // handed to the policy is verbatim — §scxml-6.4 requires an exact
            // copy, so the child must see the same `_event.data`,
            // `_event.origin`, `_event.sendid`, `_event.origintype` and
            // `_event.invokeid` the parent sees. Only `target` is withheld (a
            // routing decision owned by the originating `<send>`; inheriting
            // it would bounce the copy back onto the mesh instead of
            // delivering it to the child) — see ForwardedEvent.h.
            if constexpr (SCE::Core::HasAutoforward<StatePolicy, StaticExecutionEngine>) {
                ::SCE::Common::ForwardedEvent forwarded;
                forwarded.name = policy_.getEventName(eventWithMeta.event);
                forwarded.data = eventWithMeta.data;
                forwarded.origin = eventWithMeta.origin;
                forwarded.sendId = eventWithMeta.sendId;
                forwarded.type = eventWithMeta.type;
                forwarded.originType = eventWithMeta.originType;
                forwarded.invokeId = eventWithMeta.invokeId;
                SCE_LOG_DEBUG("AOT processNextExternalEvent: Autoforwarding dequeued external event '{}'",
                              forwarded.name);
                policy_.forwardToAutoforwardChildren(forwarded, *this);
            }

            executeTransition(event);
        }
        return true;
    }

    /**
     * @brief Check for eventless transitions (§scxml-3.13)
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

        // §scxml-3.13: Use shared algorithm (Single Source of Truth)
        // Note: Eventless transitions can raise new internal events, use internal queue
        SCE::Core::AOTEventQueue<EventWithMetadata> adapter(internalQueue_);

        while (iterations++ < MAX_ITERATIONS) {
            State oldState = currentState_;
            std::vector<State> preTransitionStates = getActiveStates();  // §scxml-3.11: Capture before transition
            SCE_LOG_DEBUG("AOT checkEventlessTransitions: Iteration {}, currentState={}", iterations,
                          static_cast<int>(currentState_));

            // Call processTransition with default event for eventless transitions
            if (policy_.processTransition(currentState_, Event(), *this)) {
                // §scxml-3.4: For parallel states, use actual transition source state
                State actualSourceState = policy_.lastTransitionSourceState_;
                SCE_LOG_DEBUG("AOT checkEventlessTransitions: Transition taken from {} to {} (actual source: {})",
                              static_cast<int>(oldState), static_cast<int>(currentState_),
                              static_cast<int>(actualSourceState));
                if (oldState != currentState_) {
                    // W3C SCXML Appendix D: For parallel states, executeMicrostep already handled exit/transition/entry
                    // Only call handleHierarchicalTransition for non-parallel state machines
                    if constexpr (!StatePolicy::HAS_PARALLEL_STATES) {
                        // ARCHITECTURE.MD: Zero Duplication - use shared helper
                        // §scxml-3.13: Pass transition action callback for correct execution order
                        // §scxml-3.4: Use actualSourceState for correct hierarchical exit/entry
                        handleHierarchicalTransition(actualSourceState, currentState_, preTransitionStates,
                                                     [this] { policy_.executeTransitionActions(*this); });
                    } else {
                        SCE_LOG_DEBUG(
                            "AOT checkEventlessTransitions: Parallel state machine - executeMicrostep handled "
                            "all transitions");
                    }

                    // §scxml-3.13: Internal events are processed AFTER stable configuration is reached
                    // Continue loop to check for more eventless transitions first
                } else {
                    // Transition taken but state didn't change - stop
                    break;
                }
            } else {
                // §scxml-3.13: No eventless transition available - stable configuration reached
                // Internal events will be processed by caller (runMainEventLoop or step)
                break;
            }
        }

        if (iterations >= MAX_ITERATIONS) {
            // Eventless transition loop detected
            SCE_LOG_ERROR(
                "StaticExecutionEngine: Eventless transition loop detected after {} iterations - stopping state "
                "machine",
                MAX_ITERATIONS);
            stop();
        }

        // §scxml-3.13: Check if we reached a top-level final state after eventless transitions
        // For parallel states, check if any active state is a top-level final state
        if constexpr (StatePolicy::HAS_PARALLEL_STATES) {
            auto activeStates = getActiveStates();
            for (const auto &state : activeStates) {
                if (StatePolicy::isFinalState(state) && StatePolicy::getParent(state) == std::nullopt) {
                    SCE_LOG_INFO(
                        "AOT checkEventlessTransitions: Reached top-level final state {}, halting processing (W3C "
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
     * @brief Initialize state machine (§scxml-3.2)
     *
     * Performs the initial configuration:
     * 1. Enter initial state (with hierarchical entry from root to leaf)
     * 2. Execute entry actions (may raise internal events)
     * 3. Process internal event queue
     * 4. Check for eventless transitions
     */
    void initialize() {
        isRunning_ = true;

        // §scxml-5.3: Initialize datamodel before any state entry
        // This ensures error.execution events are raised immediately if initialization fails
        if constexpr (SCE::Core::HasDataModelInit<StatePolicy, StaticExecutionEngine>) {
            policy_.initializeDataModel(*this);
        }

        // §scxml-3.3: Use HierarchicalStateHelper for correct entry order
        auto entryChain = SCE::Core::HierarchicalStateHelper<StatePolicy>::buildEntryChain(currentState_);

        // Execute entry actions from root to leaf (ancestor first)
        for (const auto &state : entryChain) {
            executeOnEntry(state);
        }

        // §scxml-D-mainEventLoop: hand over to the outer loop. The macrostep
        // completes on eventless transitions and internal events, then the
        // invokes for the states just entered run (only those in
        // entered-and-not-exited states; cancellation is handled during state
        // exits), and only then is anything taken off the external queue — so
        // an `autoforward` child is live for every event `<onentry>` queued on
        // the way in.
        SCE_LOG_DEBUG("AOT initialize: After entry actions, entering main event loop");
        runMainEventLoop();
        SCE_LOG_DEBUG("AOT initialize: Main event loop settled - stable configuration reached");

        // §scxml-6.4: Invoke completion callback if top-level final after initialization.
        // Child state machines may reach the machine-done state immediately (e.g.,
        // initial="subFinal") and must notify parent. Regional `<final>` inside a
        // `<parallel>` is excluded by `isInFinalState()` because the machine as a
        // whole is still running while sibling regions are active.
        if (isInFinalState() && completionCallback_) {
            SCE_LOG_DEBUG(
                "AOT initialize: Reached top-level final state during initialization, invoking completion callback");
            // §scxml-3.9: Execute onexit actions for final state before notifying parent
            std::vector<State> activeStates = getActiveStates();
            executeOnExit(currentState_, activeStates);
            completionCallback_();
        }
    }

    /**
     * @brief Step the state machine (process pending events)
     *
     * §scxml-6.4: For parent-child communication, parents must explicitly
     * step child state machines after sending events to ensure synchronous processing.
     *
     * This method processes all pending events in both internal and external queues.
     */
    void step() {
        runMainEventLoop();

        // §scxml-6.4: Invoke completion callback only at top-level final.
        // The bare structural `StatePolicy::isFinalState` would mis-fire on a
        // regional `<final>` inside a `<parallel>`; `isInFinalState()` carries
        // the parent-presence check that distinguishes "machine done" from
        // "one region done".
        if (isInFinalState() && completionCallback_) {
            SCE_LOG_DEBUG("AOT step: Invoking completion callback for done.invoke");
            completionCallback_();
        }
    }

    /**
     * @brief Read `_event.invokeid` for the event currently being processed
     *        (§scxml-5.10.1)
     *
     * Templates that emit cross-boundary dispatches (e.g. mesh `<send>`) call
     * this to auto-propagate the incoming event's invokeId onto the outgoing
     * envelope — the same §scxml-6.4.1 auto-propagation semantics that
     * already govern child-to-parent sends, extended to mesh-rpc replies.
     *
     * Returns an empty string when the Policy was generated without the
     * `pendingEventInvokeId_` field (`datamodel="null"` machines that never
     * touch `_event.invokeid`), keeping non-metadata policies lean.
     */
    [[nodiscard]] const std::string &currentEventInvokeId() const {
        return currentEventInvokeId_;
    }

    /**
     * @brief Process an external event (§scxml-3.13)
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
        if (!isRunning_) {
            return;
        }
        processEventImpl(event);
    }

    /**
     * @brief Process an external event with metadata (§scxml-5.10)
     *
     * External events with metadata support originSessionId for invoke finalize.
     * Used when events come from child sessions via invoke.
     *
     * @param event External event to process
     * @param metadata Event metadata (originSessionId, etc.)
     */
    void processEvent(Event event, const SCE::Core::EventMetadata &metadata) {
        if (!isRunning_) {
            return;
        }
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
     * @brief Get all active states (§scxml-3.11)
     *
     * For simple state machines (no parallel), returns vector with single current state hierarchy.
     * For parallel state machines, returns all active states across all parallel regions.
     *
     * Used by history recording logic and parallel completion checks.
     *
     * @return Vector of currently active states
     */
    std::vector<State> getActiveStates() const {
        // §scxml-3.4: For parallel state machines, use policy's activeStates_ tracking
        if constexpr (StatePolicy::HAS_PARALLEL_STATES) {
            if constexpr (SCE::Core::HasActiveStates<StatePolicy>) {
                return policy_.getActiveStates();
            }
        }

        // §scxml-3.11: For non-parallel, use shared HistoryHelper for full active hierarchy (Zero Duplication
        // Principle) Returns [currentState, parent, grandparent, ...] for proper history recording
        return ::SCE::Core::HistoryHelper::getActiveHierarchy(currentState_,
                                                              [](State s) { return StatePolicy::getParent(s); });
    }

    /**
     * @brief Check whether this session has ended (§scxml-3.7 / §scxml-6.4)
     *
     * True only when `currentState_` is a `<final>` **and** has no parent,
     * i.e. its parent is the `<scxml>` element. §scxml-D-enterStates sets
     * `running = false` for a `<final>` only when `isSCXMLElement(s.parent)`;
     * a nested one queues `done.state.<parent>` and the machine carries on.
     * The structural question — "is this state a `<final>` element" — is
     * `StatePolicy::isFinalState`, and it is not the completion criterion on
     * its own.
     *
     * Every "machine is done" decision keys on this predicate: the scheduler
     * short-circuit in `tick()`, the queue-processing bail-out in
     * `runMainEventLoop()`, `runUntilCompletion()`, and external
     * `done.invoke` notification.
     *
     * See SCE_MESH.md §mesh-16.5 for the concrete case that motivated
     * the parent check: a `<parallel>` whose local region has reached its
     * regional `<final>` ahead of a remote sibling's wire-21 arrival still
     * needs the scheduler pumped so the barrier-timeout event can fire.
     *
     * @return true if `currentState_` is a top-level `<final>`
     */
    bool isInFinalState() const {
        return StatePolicy::isFinalState(currentState_) && !StatePolicy::getParent(currentState_).has_value();
    }

    /**
     * @brief Stash donedata evaluated at top-level `<final>` entry
     *        (§scxml-5.5 + 6.4.3).
     *
     * Called by generated entry actions (`entry_exit_actions.jinja2`) after
     * `DoneDataHelper::evaluateParams` / `evaluateContent` / `emitContentLiteral`
     * has produced the payload for the reached top-level `<final>`. Shared
     * between the local invoke completion path (read by
     * `invoke_methods.jinja2`'s completionCallback to populate
     * `done.invoke.<id>._event.data`) and the SCE Mesh worker (read by
     * `ChildSessionAdapter::getDonedata()` to ship wire-18 per §mesh-9.6.2). Called
     * at most once per invocation — the final state is terminal.
     */
    void stashDonedataAtFinal(std::string data, std::optional<ScriptValue> typedData) {
        pendingDonedataAtFinal_ = std::move(data);
        pendingTypedDonedataAtFinal_ = std::move(typedData);
    }

    /// §scxml-5.5 + 6.4.3: JSON/literal string payload from the reached
    /// top-level `<final>`'s `<donedata>`. Empty when no donedata was authored
    /// or the machine has not reached a top-level final yet. Consumed by both
    /// local invoke completion and SCE Mesh §mesh-9.6.2 wire-18.
    const std::string &donedataAtFinal() const {
        return pendingDonedataAtFinal_;
    }

    /// §scxml-5.5 + B.2: Structured donedata (engine-agnostic ScriptValue)
    /// paired with `donedataAtFinal()`. `setPolicyMetadata` consumes this to
    /// populate `_event.data` without a JSON round-trip when the child is
    /// local; on the wire-18 path the parent's `setPolicyMetadata` re-parses
    /// `data` via `EventDataHelper::jsonStringToScriptValue`.
    const std::optional<ScriptValue> &typedDonedataAtFinal() const {
        return pendingTypedDonedataAtFinal_;
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
     * @brief Drain the delayed-send scheduler onto the external queue
     *        (§scxml-6.2).
     *
     * Pops every scheduled event whose delay has elapsed (wall-clock)
     * and raises it onto the external queue via `raiseExternal`. Does
     * **not** run `step()` afterwards — the caller is expected to
     * follow up with `step()` when it wants the raised events to
     * drive transitions.
     *
     * **Canonical polling API is `tick()`** — it is parallel-aware
     * (short-circuits on `isInFinalState()`, which requires the final's
     * parent to be absent, not on the bare structural
     * `StatePolicy::isFinalState`), calls this method internally, and then
     * performs a full macrostep. Normal polling loops should call
     * `tick()`.
     *
     * `pumpScheduledEvents()` is retained as a public hook for
     * fine-grained callers that need the scheduler drained *without*
     * a follow-up microstep — e.g. harnesses that interleave
     * scheduler-only pulses with explicit `processEvent()` /
     * `step()` sequencing to exercise a specific ordering. If you
     * are writing a plain tick loop, prefer `tick()`.
     */
    void pumpScheduledEvents() {
        std::string eventData;
        Event event;
        while (scheduler_.popReadyEvent(event, eventData)) {
            raiseExternal(event, eventData);
        }
    }

    /**
     * @brief Tick scheduler and process ready internal events (§scxml-6.2)
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

        // §scxml-6.4 — top-level final only: a regional `<final>`
        // inside a `<parallel>` is *not* a terminator for the machine
        // as a whole, so we must not short-circuit the scheduler pump
        // when only a region has completed. `isInFinalState()` encodes the
        // parent-presence check that the structural `StatePolicy::isFinalState`
        // deliberately omits; see SCE_MESH.md §mesh-16.5 for the
        // barrier-timeout case that surfaces this.
        if (isInFinalState()) {
            if (completionCallback_) {
                SCE_LOG_DEBUG("AOT tick: Invoking completion callback for already-final state");
                completionCallback_();
            }
            return;
        }

        // §scxml-6.2: Check for ready scheduled events and raise them
        pumpScheduledEvents();

        // §scxml-6.4: Tick child state machines to process their events
        // Children need to run independently during parent's event loop
        if constexpr (SCE::Core::HasChildTick<StatePolicy, StaticExecutionEngine>) {
            policy_.tickChildren(*this);
        }

        // Zero Duplication: Delegate event processing + completion callback to step()
        // step() handles: runMainEventLoop() + completionCallback_. §scxml-6.4's
        // invokes are part of that loop and run there, ahead of the external
        // dequeue rather than after it.
        step();
    }

    /**
     * @brief Set completion callback for done.invoke event generation (§scxml-6.4)
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
     * @brief Set HTTP send callback for BasicHTTPEventProcessor (§scxml-C-2)
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
     * @brief Dispatch BasicHTTP send via callback (§scxml-C-2)
     *
     * Called by AOT-generated code for BasicHTTPEventProcessor sends.
     * Delegates to onHttpSend_ callback set by test harness or application.
     * Matches Kotlin StateMachineEngine.performHttpSend() pattern.
     */
    void performHttpSend(const std::string &target, const std::string &eventName, const std::string &content,
                         const std::map<std::string, std::vector<std::string>> &params, const std::string &sendId) {
        if (onHttpSend_) {
            onHttpSend_(HttpSendRequest{target, eventName, content, params, sendId});
        }
    }

    /**
     * @brief Register a SCE Mesh send callback (cross-machine transport)
     *
     * Mirrors setHttpSendCallback(). The callback receives raw field values
     * (target, eventName, data, sendId) for every external
     * <send target="#<machine>"/>. It must return true
     * when the send was accepted by the transport and false to fall through
     * to the external queue (legacy W3C behavior, useful for targets that
     * the transport does not recognize).
     *
     * Applications typically do not call this directly — the generated
     * TransportRouter's wireTo() method registers itself here.
     */
    void setMeshSendCallback(MeshSendCallback callback) {
        onMeshSend_ = std::move(callback);
    }

    /**
     * @brief Dispatch a mesh send via the registered callback
     *
     * @return true if the callback accepted the send (do not re-enqueue);
     *         false if no callback is wired or it declined the target
     *         (caller should fall back to the external queue).
     */
    bool performMeshSend(const std::string &target, const std::string &eventName, const std::string &data,
                         const std::string &sendId, const std::string &invokeId) {
        if (onMeshSend_) {
            return onMeshSend_(target, eventName, data, sendId, invokeId);
        }
        return false;
    }

    /**
     * @brief Register a mesh-rpc invoke callback (SCE_MESH.md §mesh-9.5)
     *
     * Installed by the generated TransportRouter ctor when any target's
     * `invoke_sites` is non-empty. The callback receives (target,
     * fieldSuffix, invokeId, data) and returns true on successful
     * dispatch. Applications typically do not call this directly.
     */
    void setMeshInvokeCallback(MeshInvokeCallback callback) {
        onMeshInvoke_ = std::move(callback);
    }

    /**
     * @brief Dispatch a mesh-rpc invoke via the registered callback
     *
     * Emitted from the generated onentry block for states with
     * `<invoke type="sce:mesh-rpc">`. Returns false when no callback is
     * installed — that is a deployment-time error (mesh-rpc document
     * rendered without TransportRouter wiring) and the generated code
     * raises `error.execution` per §scxml-6.4.1 graceful-degrade
     * semantics.
     */
    bool performMeshInvoke(const std::string &target, const std::string &fieldSuffix, const std::string &invokeId,
                           const std::string &data) {
        if (onMeshInvoke_) {
            return onMeshInvoke_(target, fieldSuffix, invokeId, data);
        }
        return false;
    }

    /**
     * @brief Register a mesh-rpc cancel callback (SCE_MESH.md §mesh-9.5)
     *
     * Installed alongside `setMeshInvokeCallback`. Applications typically
     * do not call this directly.
     */
    void setMeshCancelCallback(MeshCancelCallback callback) {
        onMeshCancel_ = std::move(callback);
    }

    /**
     * @brief Dispatch a mesh-rpc cancel via the registered callback
     *
     * Emitted from the generated onexit block for states with
     * `<invoke type="sce:mesh-rpc">`. Returns false if no callback is
     * installed or there is no active invoke to cancel — both cases are
     * benign (nothing to clean up).
     */
    bool performMeshCancel(const std::string &target, const std::string &fieldSuffix) {
        if (onMeshCancel_) {
            return onMeshCancel_(target, fieldSuffix);
        }
        return false;
    }

    /**
     * @brief Register the SCXML remote-invoke start callback (SCE_MESH.md §mesh-9.6.2 wire 14)
     *
     * Installed by the generated TransportRouter ctor when any target machine
     * has a distinct-peer `<invoke type="scxml" src="#peer">` entry classified
     * by `classify_remote_scxml_invokes`. Applications typically do not call
     * this directly.
     */
    void setScxmlInvokeStartCallback(ScxmlInvokeStartCallback callback) {
        onScxmlInvokeStart_ = std::move(callback);
    }

    /**
     * @brief Dispatch an SCXML remote-invoke start via the registered callback
     *
     * Emitted from the generated onentry block for states with
     * `<invoke type="scxml" src="#peer">` whose `src` refers to a deploy.yaml
     * peer. Returns false when no callback is installed — the document was
     * rendered without TransportRouter wiring for the remote peer; the caller
     * (codegen) falls through to the transport-absent local
     * `error.execution` raise carrying SESSION_F_TRANSPORT_UNAVAILABLE per
     * SCE_MESH.md §mesh-9.6 line 1396.
     */
    bool performScxmlInvokeStart(const std::string &target, const std::string &invokeIdString,
                                 const std::string &data) {
        if (onScxmlInvokeStart_) {
            return onScxmlInvokeStart_(target, invokeIdString, data);
        }
        return false;
    }

    /**
     * @brief Register the SCXML remote-invoke parent-event callback
     *        (SCE_MESH.md §mesh-9.6.2 wire 17, autoforward).
     *
     * Installed by the generated TransportRouter ctor when any target machine
     * hosts a remote `<invoke autoforward="true">`. Applications typically
     * do not call this directly.
     */
    void setScxmlInvokeParentEventCallback(ScxmlInvokeParentEventCallback callback) {
        onScxmlInvokeParentEvent_ = std::move(callback);
    }

    /**
     * @brief Dispatch an SCXML remote autoforward via the registered callback.
     *
     * Emitted from `forwardToAutoforwardChildren` (invoke_methods.jinja2)
     * when the parent is autoforwarding an external event to a remote child
     * session. Returns false when no callback is installed (authoring error
     * — caller should log but not crash).
     */
    bool performScxmlInvokeParentEvent(const std::string &target, const std::string &invokeIdString,
                                       const std::string &eventName, const std::string &data,
                                       const std::string &sendId) {
        if (onScxmlInvokeParentEvent_) {
            return onScxmlInvokeParentEvent_(target, invokeIdString, eventName, data, sendId);
        }
        return false;
    }

    /**
     * @brief Register the SCXML remote-invoke cancel callback
     *        (SCE_MESH.md §mesh-9.6.2 wire 19).
     *
     * Installed by the generated TransportRouter ctor for every remote
     * invoke target. Fires when the parent exits the invoking state before
     * the child reaches `<final>`, so the worker can tear down the child
     * session cleanly per §scxml-6.4.
     */
    void setScxmlInvokeCancelCallback(ScxmlInvokeCancelCallback callback) {
        onScxmlInvokeCancel_ = std::move(callback);
    }

    /**
     * @brief Dispatch an SCXML remote invoke cancel via the registered callback.
     *
     * Returns false when no callback is installed (authoring error) or when
     * the TransportRouter declined the cancel (no matching active invoke).
     * Both cases are benign — the parent's `activeInvokes_` entry is cleared
     * regardless.
     */
    bool performScxmlInvokeCancel(const std::string &target, const std::string &invokeIdString) {
        if (onScxmlInvokeCancel_) {
            return onScxmlInvokeCancel_(target, invokeIdString);
        }
        return false;
    }

    /**
     * @brief Register the local-region completion callback (SCE_MESH.md §mesh-16.5).
     *
     * Installed by the derived SM ctor when the codegen materialized a Root
     * tracker for a hosted `<parallel>`. The closure dispatches on `parallel_id`
     * to the matching `ParallelCompletionTracker.onLocalRegionComplete(region)`
     * member of the SM, which fires `done.state.<parallel>` on threshold.
     * Applications do not call this directly — the SM ctor wires it.
     */
    void
    setParallelRegionLocalCompleteCallback(std::function<void(const std::string &, const std::string &)> callback) {
        onParallelRegionLocalComplete_ = std::move(callback);
    }

    /**
     * @brief Invoke the local-region completion hook (SCE_MESH.md §mesh-16.5).
     *
     * Called from the generated `mesh/cpp/parallel_final.jinja2` Root branch
     * when a region hosted in this partition enters its `<final>`. No-op when
     * no callback is installed (single-partition or non-Root builds).
     */
    void triggerParallelRegionLocalComplete(const std::string &parallel_id, const std::string &region_id) {
        if (onParallelRegionLocalComplete_) {
            onParallelRegionLocalComplete_(parallel_id, region_id);
        }
    }

    /**
     * @brief Register the remote-region wire-21 send callback (SCE_MESH.md §mesh-16.5).
     *
     * Installed by the derived SM ctor when the codegen materialized a NonRoot
     * sender for a hosted `<parallel>`. The closure builds the
     * `ParallelRegionDone` envelope (wire 21, `subject = parallel_id +
     * "/" + region_id`) and routes it through the SM-side wire-21 callback
     * registered by the generated TransportRouter ctor. Applications do not
     * call this directly — the SM ctor wires it.
     */
    void setParallelRegionRemoteSendCallback(
        std::function<void(const std::string &, const std::string &, const std::string &)> callback) {
        onParallelRegionRemoteSend_ = std::move(callback);
    }

    /**
     * @brief Invoke the remote-region wire-21 send hook (SCE_MESH.md §mesh-16.5).
     *
     * Called from the generated `mesh/cpp/parallel_final.jinja2` NonRoot branch
     * when a region hosted in this partition enters its `<final>`. The closure
     * is responsible for failing loudly when no transport-side callback was
     * installed by the TransportRouter — a missing wire-up must surface as a
     * fatal exception rather than a silent drop (SCE_MESH.md §mesh-14).
     */
    void triggerParallelRegionRemoteSend(const std::string &parallel_id, const std::string &region_id,
                                         const std::string &donedata) {
        if (onParallelRegionRemoteSend_) {
            onParallelRegionRemoteSend_(parallel_id, region_id, donedata);
        }
    }

    /**
     * @brief Get access to policy for parameter passing (§scxml-6.4)
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
