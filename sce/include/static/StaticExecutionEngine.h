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
#include "common/SceClock.h"
#include "common/SendHelper.h"
#include "common/SendSchedulingHelper.h"
#include "core/AOTEventQueue.h"
#include "core/EventMatchingHelper.h"
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
#include <optional>
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

/**
 * @brief What the engine did with one event it offered to the active configuration
 *
 * This used to be a bare `bool` meaning "the configuration changed", which
 * answers false for two unrelated outcomes: an event no transition matched at
 * all, and a targetless internal transition that ran its actions in place.
 * Only the first is the discard §scxml-3.1.2 describes, and a
 * count keyed off the old bool would have reported a handled event as one, so
 * the two facts are spelled apart rather than inferred from each other.
 *
 * The Interpreter's `StateMachine::TransitionResult` is the same distinction on
 * the other engine; this is the AOT side, kept to two bools because the
 * generated engine carries no `std::string` on this path.
 */
struct EventOutcome {
    /// Whether any transition matched the event.
    bool selected = false;
    /// False for a targetless internal transition, which leaves the
    /// configuration alone after running its actions.
    bool configurationChanged = false;
};
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
     * @brief How many links an `error.*` chain may have before the engine
     *        stops feeding it — see `errorCascadeEvents()`
     *
     * §scxml-3.12.2 says what to do with an error event nothing matches. It
     * does not say what to do when something *does* match it and that handler
     * fails too: the failure raises the same error, the same transition
     * answers it, and the machine has no way out. Nothing in the specification
     * bounds that, so the number is this engine's to choose, and it matches
     * the ceiling `EventProcessingAlgorithms::checkEventlessTransitions` uses
     * for the sibling case of a macrostep that cannot finish — decided the
     * same way for the same reason.
     *
     * A hundred links is far past any repair strategy a document plausibly
     * spells (a handler that tries a fallback, then a second one, is three)
     * and far short of a number a host would wait through.
     */
    static constexpr uint32_t MAX_ERROR_CASCADE_DEPTH = 100;

    /**
     * @brief How many microsteps one macrostep may take before this engine
     *        stops taking them — see `truncatedMacrosteps()`
     *
     * The specification defines a macrostep as a chain of microsteps ending in
     * a configuration where nothing is enabled by NULL and the internal queue
     * is empty, and its Principles and Constraints say in as many words that
     * such a chain need not exist: *"A microstep always terminates. A
     * macrostep may not. A macrostep that does not terminate may be said to
     * consist of an infinitely long sequence of microsteps. This is currently
     * allowed."*
     *
     * So the ceiling is not conformance — it is this engine declining a
     * document the specification permits, which is exactly why the decline has
     * to be visible.
     *
     * One budget for the whole inner loop, not one per branch. Appendix D's
     * loop takes a microstep on an eventless transition *or* on an internal
     * event, and a document alternating the two is one chain, not two:
     * budgeting the branches separately leaves that chain unbounded, which is
     * what a per-call counter on the eventless branch alone did here until
     * 2026-08-20.
     *
     * Ten times `MAX_ERROR_CASCADE_DEPTH`, and deliberately not equal to it.
     * This is the backstop; the cascade ceiling is a diagnostic that names the
     * error a handler keeps failing on, and a backstop that fires first makes
     * that diagnostic unreachable. Measured 2026-08-20: with both at a
     * hundred, a handler that raises one event of its own per link — two
     * microsteps a link, which is what a document that logs before it fails
     * looks like — was cut at fifty links by this ceiling and
     * `errorCascadeEvents()` never moved. The factor of ten is the headroom
     * that keeps the specific report reachable for a handler raising up to
     * eight events a link; a busier one is cut here instead, which is coarser
     * but still reported.
     */
    static constexpr uint32_t MAX_MACROSTEP_MICROSTEPS = 1000;

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

            executeOnEntryChain(entryChain, newState);

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
            executeOnEntryChain(entryChain, newState);

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
     * @return what the event did — see EventOutcome
     */
    EventOutcome executeTransition(Event event) {
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
    template <typename PostTransitionFn>
    EventOutcome executeTransition(Event event, PostTransitionFn &&postTransition) {
        State oldState = currentState_;
        std::vector<State> preTransitionStates = getActiveStates();
        if (!policy_.processTransition(currentState_, event, *this)) {
            return EventOutcome{};
        }

        // §scxml-3.13: Self-transitions (target = source) exit and re-enter the state
        // §scxml-3.13: Targetless transitions consume event only (no exit/enter)
        bool isSelfTransition = (oldState == currentState_);
        bool needsHierarchicalHandling =
            (oldState != currentState_) || (isSelfTransition && !policy_.lastTransitionIsTargetless_);

        if (!needsHierarchicalHandling) {
            // §scxml-3.13: Targetless transition - execute actions without state change
            policy_.executeTransitionActions(*this);
            // W3C SCXML Appendix D's main event loop returns to
            // `selectEventlessTransitions()` after EVERY microstep and drains
            // the internal queue in the same inner loop, without asking whether
            // the microstep moved the machine. Returning here instead ended the
            // macrostep at a transition that ran content: whatever that content
            // enabled was never walked, and whatever it raised stayed on the
            // queue, so the host was handed a configuration the clause calls
            // unstable with nothing anywhere saying so. This is the same work
            // the state-changing path below does, in the same order.
            postTransition();
            checkEventlessTransitions();
            // The transition itself moved nothing; the chain it opened may
            // have. This asks the machine rather than the transition, so a
            // chain that reaches a top-level `<final>` still notifies the
            // parent — see this struct's contract for why the two facts are
            // spelled apart.
            return EventOutcome{/*selected=*/true, /*configurationChanged=*/currentState_ != oldState};
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
        return EventOutcome{/*selected=*/true, /*configurationChanged=*/true};
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
     * Takes the carrier rather than the bare event because an external event
     * is more than its name here: §scxml-D-mainEventLoop's preliminary step
     * reads the metadata to decide which `<invoke>` a `<finalize>` belongs to
     * and what an `autoforward` child is handed, so a door that only knew the
     * name could not run it.
     *
     * @param eventWithMeta Event, plus whatever the host said about it
     */
    void processEventImpl(const EventWithMetadata &eventWithMeta) {
        // §scxml-3.13: a host call is one turn as well as one macrostep
        // boundary, so the `<onentry>` this event's transition runs arms its
        // `<send delay>`s against a single instant — see `beginTurn()`.
        TurnGuard turn(*this);

        // A host call is a macrostep boundary, so the previous macrostep's
        // ceiling stops applying here — see `truncatedMacrosteps()`. Recorded
        // in this entry point as well as at the external dequeue because this
        // one does not go through that queue: it hands the event straight to
        // `executeTransition`, so a machine left inside an endless chain would
        // otherwise never get another budget from a host that drives it this
        // way.
        macrostepTruncated_ = false;
        macrostepMicrostepsTaken_ = 0;

        // §scxml-D-mainEventLoop: the preliminary step is owed to the event,
        // not to the queue. This door bypasses `processNextExternalEvent`, so
        // it has to run the step itself or an `autoforward` child never sees
        // what the host delivered and a `<finalize>` never runs for a child
        // event a transport relayed in this way.
        const Event event = eventWithMeta.event;
        applyExternalEventPreamble(eventWithMeta);

        const EventOutcome outcome = executeTransition(event, [this] { runMainEventLoop(); });
        // §scxml-3.1.2: this entry point takes an event straight from the host,
        // so it is the same fact `processNextExternalEvent` records — unlike
        // the Rust and Go engines, whose `process_event` enqueues and lets the
        // dequeue do it. Recorded in both places so a host calling
        // `processEvent` reads the same count on every backend.
        recordEventOutcome(event, outcome);
        const bool stateChanged = outcome.configurationChanged;
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

    /**
     * @brief Record what an external event did, for the host that queued it
     *
     * §scxml-3.1.2: "If no transition matches in any state, the event is
     * discarded." Discarding it is the rule; being unable to say so is not part
     * of the rule. The host is the one party that cannot see the outcome — a
     * discard leaves the configuration exactly as a self transition does — and
     * it is the party that got the event wrong.
     *
     * Called for external events only. An internal `<raise>` that matches
     * nothing is discarded too, but both ends of that are inside the document.
     */
    void recordEventOutcome(Event event, const EventOutcome &outcome) {
        if (outcome.selected) {
            return;
        }
        ++discardedExternalEvents_;
        lastDiscardedEvent_ = event;
        hasDiscardedEvent_ = true;
        SCE_LOG_DEBUG("AOT: no transition matched external event '{}'; discarded", policy_.getEventName(event));
    }

    /**
     * @brief §scxml-3.12.2: record an `error.*` event no transition answered
     *
     * The internal-queue twin of `recordEventOutcome` above, and deliberately a
     * separate count rather than the same one. That count stops at the external
     * queue because an author's unmatched `<raise>` has both ends inside the
     * document; the sender of an error event is this engine, so the same
     * reasoning does not reach it — the host never wrote the document and
     * cannot see the failure in the configuration.
     *
     * An author's own `<raise>` that matches nothing is still not counted:
     * the name is what separates the two, and the clause reserves the
     * `error.` prefix for the processor's own errors.
     */
    void recordInternalEventOutcome(Event event, const EventOutcome &outcome) {
        // §scxml-3.12.2: error events go on the internal queue and "are
        // ignored if no transition is found that matches them". Cited in the
        // body rather than the doc comment because the ledger's C++ resolver
        // binds a citation to the symbol enclosing it.
        if (outcome.selected) {
            return;
        }
        if (!SCE::Core::EventMatchingHelper::isErrorEvent(policy_.getEventName(event))) {
            return;
        }
        ++unhandledErrorEvents_;
        lastUnhandledError_ = event;
        hasUnhandledError_ = true;
        SCE_LOG_DEBUG("AOT: no transition matched error event '{}'; unhandled", policy_.getEventName(event));
    }

    State currentState_;
    SCE::Core::EventQueueManager<EventWithMetadata>
        internalQueue_;  // §scxml-3.13: Internal event queue (high priority)
    SCE::Core::EventQueueManager<EventWithMetadata> externalQueue_;  // §scxml-3.13: External event queue (low priority)
    bool isRunning_ = false;
    // Whether `tick()` has ever run. A machine whose policy declares
    // NEEDS_EVENT_SCHEDULER has delayed events only `tick()` can deliver, so a
    // host driving it with `step()` alone waits forever with nothing said.
    // Once `tick()` has run the host owns a clock and its `step()` calls are
    // its own business, so the count below stops.
    bool tickHasRun_ = false;
    // Macrosteps taken on a scheduler-driven machine before any `tick()`.
    uint32_t unattendedSchedulerSteps_ = 0;
    // §scxml-3.1.2: external events no transition matched, and the most recent
    // of them. `hasDiscardedEvent_` is separate because the zero value of the
    // generated `Event` enum is a real event and cannot stand in for "none".
    uint32_t discardedExternalEvents_ = 0;
    Event lastDiscardedEvent_{};
    bool hasDiscardedEvent_ = false;
    // §scxml-3.12.2: `error.*` events this engine raised that no transition
    // matched, and the most recent of them. `hasUnhandledError_` is separate
    // for the same reason as above: the zero value of the generated `Event`
    // enum is a real event and cannot stand in for "none".
    uint32_t unhandledErrorEvents_ = 0;
    Event lastUnhandledError_{};
    bool hasUnhandledError_ = false;
    // §scxml-3.12.2: the drain is executing a transition an `error.*` event
    // selected, which is the state in which a newly raised error is a link in
    // a chain rather than a first failure; how long that chain is; and what
    // the engine refused because of it. See `errorCascadeEvents()`.
    bool handlingErrorEvent_ = false;
    uint32_t errorCascadeDepth_ = 0;
    uint32_t errorCascadeEvents_ = 0;
    Event lastErrorCascadeEvent_{};
    bool hasErrorCascadeEvent_ = false;
    // Macrosteps stopped at `MAX_MACROSTEP_MICROSTEPS` with the chain still
    // going, and the state the drain was in when that last happened.
    // `hasTruncatedMacrostep_` is separate because the zero value of the
    // generated `State` enum is a real state. `macrostepTruncated_` exists
    // because the drain is reached more than once per macrostep, and without
    // it each caller would get a fresh budget; it is cleared where the
    // algorithm starts a macrostep, the external dequeue.
    // `macrostepMicrostepsTaken_` is that budget, and it lives here rather
    // than in either drain because Appendix D's inner loop is one loop: the
    // eventless branch and the internal-event branch take turns inside a
    // single macrostep, so a counter local to either bounds only half of it.
    uint32_t truncatedMacrosteps_ = 0;
    State lastTruncatedMacrostepState_{};
    bool hasTruncatedMacrostep_ = false;
    bool macrostepTruncated_ = false;
    uint32_t macrostepMicrostepsTaken_ = 0;
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

    /// §scxml-6.2.2: where this engine reads "now" from — see `ISceClock`.
    std::shared_ptr<SCE::ISceClock> clock_ = std::make_shared<SCE::MonotonicClock>();

    /// The reading `clock_` gave when the current turn began, empty between
    /// turns. See `beginTurn()`.
    std::optional<uint64_t> turnNowMs_;

    /**
     * @brief §scxml-3.13: what time it is, for everything this turn arms or judges
     *
     * The clause executes a microstep's executable content as one unit and a
     * macrostep as a chain of those, so "now" is a property of the turn the
     * engine is in rather than of the statement asking for it. Between turns
     * there is no turn for it to be a property of, and the host's queries
     * (`timeUntilNextScheduled()`, `nowMs()`) read the clock live.
     */
    uint64_t schedNowMs() const {
        return turnNowMs_.has_value() ? *turnNowMs_ : clock_->elapsedMs();
    }

    /**
     * @brief Open a turn: take the single clock reading everything inside uses
     *
     * Returns whether this call is the one that opened it, which `endTurn()`
     * needs so a nested entry point (`tick()` delegating to `step()`,
     * `processEvent()` doing the same) does not close the outer turn early.
     *
     * §scxml-6.2.2 makes a delay the wait the DOCUMENT asks for — "how long the
     * processor should wait before dispatching the message". Time the host
     * spent descheduled between two statements of one microstep is not part of
     * any delay the document wrote, so it must not reach the deadline. Reading
     * the clock per statement instead was two defects at once, both measured on
     * the sibling backends this engine shares its scheduler shape with:
     *
     * - Two `<send delay>`s executed by one `<onentry>` took a reading each, so
     *   a host descheduled between them by more than the gap between their
     *   delays got the later send's deadline first — and the engine then
     *   dispatched them in that order, so the document's `<cancel>` arrived
     *   after the event it named. Which of two events the author ordered
     *   arrives first became a fact about the host's scheduler.
     * - The dispatch loop in `tick()` re-read it on every pass, so a tick slow
     *   enough to cross the next deadline dispatched that entry too, then the
     *   one after it — the engine chasing deadlines its own slowness created,
     *   in a loop the host cannot get between.
     *
     * Neither is reachable from a clock that is read once per turn.
     */
    bool beginTurn() {
        if (turnNowMs_.has_value()) {
            return false;
        }
        turnNowMs_ = clock_->elapsedMs();
        return true;
    }

    /// Close a turn opened by `beginTurn()`.
    void endTurn(bool opened) {
        if (opened) {
            turnNowMs_.reset();
        }
    }

    /**
     * @brief RAII pairing of `beginTurn()` / `endTurn()`
     *
     * A guard rather than two bare calls because every entry point below has
     * early returns, and one that skipped `endTurn()` would leave the engine
     * frozen at the instant that call began — every later deadline computed
     * from a clock that had stopped.
     */
    class TurnGuard {
    public:
        explicit TurnGuard(StaticExecutionEngine &engine) : engine_(engine), opened_(engine.beginTurn()) {}

        ~TurnGuard() {
            engine_.endTurn(opened_);
        }

        TurnGuard(const TurnGuard &) = delete;
        TurnGuard &operator=(const TurnGuard &) = delete;

    private:
        StaticExecutionEngine &engine_;
        bool opened_;
    };

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
     * An `error.*` event raised while an error handler is running is refused
     * once the chain reaches `MAX_ERROR_CASCADE_DEPTH` — see
     * `errorCascadeEvents()` for why the engine is the one that has to stop
     * it. Only the engine's own error events are refused: an author's
     * `<raise>` inside an error handler is the document doing its job and
     * rides the queue like any other.
     *
     * @param metadata Complete event metadata including all §scxml-5.10.1 fields
     */
    void raise(EventWithMetadata metadata) {
        // §scxml-3.12.2 names the error events this refuses; the clause itself
        // is silent on a handler that fails, which is why the ceiling is a
        // choice this engine documents rather than a rule it implements.
        if (handlingErrorEvent_ && SCE::Core::EventMatchingHelper::isErrorEvent(policy_.getEventName(metadata.event))) {
            ++errorCascadeDepth_;
            if (errorCascadeDepth_ >= MAX_ERROR_CASCADE_DEPTH) {
                ++errorCascadeEvents_;
                lastErrorCascadeEvent_ = metadata.event;
                hasErrorCascadeEvent_ = true;
                if (errorCascadeEvents_ == 1) {
                    SCE_LOG_ERROR("AOT: an error handler has raised an error {} times over; refusing to feed the "
                                  "chain - the document's error handling is failing",
                                  MAX_ERROR_CASCADE_DEPTH);
                }
                return;
            }
        }
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
        uint64_t fireTimeMs = schedNowMs() + static_cast<uint64_t>(delay.count());
        return scheduler_.scheduleEventAt(event, fireTimeMs, sendId, eventData);
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
        return scheduler_.hasReadyEvents(schedNowMs());
    }

    /**
     * @brief How long until this machine next needs `tick()`
     *
     * `std::chrono::milliseconds::zero()` means something is due now;
     * `std::nullopt` means the scheduler is empty and no clock-driven wake-up
     * is owed.
     *
     * `NEEDS_EVENT_SCHEDULER` tells a host *which* entry point to drive the
     * machine with. This tells it *when*, and a host that cannot ask has only
     * one move left: pick a polling interval. That guess is not free in either
     * direction — measured on a document whose `<send delay="200ms">` is
     * cancelled by a 100 ms signal, a 1 ms interval spends 180 wasted ticks to
     * be on time, a 500 ms one fires 300 ms late, and a 250 ms one steps over
     * both deadlines at once. An interval cannot straddle two deadlines it was
     * never told about.
     *
     * The answer feeds a host loop directly — a `condition_variable::wait_for`,
     * an event-loop timeout, a frame budget.
     */
    std::optional<std::chrono::milliseconds> timeUntilNextScheduled() const {
        auto next = scheduler_.nextFireTime();
        if (!next.has_value()) {
            return std::nullopt;
        }
        uint64_t now = schedNowMs();
        if (*next <= now) {
            return std::chrono::milliseconds::zero();
        }
        return std::chrono::milliseconds(static_cast<int64_t>(*next - now));
    }

    // ════════════════════════════════════════
    // Clock (§scxml-6.2.2)
    // ════════════════════════════════════════

    /**
     * @brief Where this engine reads "now" from — see `ISceClock`
     *
     * Never null: an engine that was never given one reads a `MonotonicClock`.
     */
    const std::shared_ptr<SCE::ISceClock> &clock() const {
        return clock_;
    }

    /**
     * @brief Install the `ISceClock` this engine measures its deadlines against
     *
     * Must be called before `initialize()`: the entry configuration's
     * `<onentry>` can arm delayed sends, and swapping the clock under deadlines
     * already computed from another one would leave the queue holding two
     * incomparable time bases. That is a programming error rather than a
     * recoverable condition, so it throws.
     */
    void setClock(std::shared_ptr<SCE::ISceClock> clock) {
        if (!clock) {
            throw std::invalid_argument(
                "StaticExecutionEngine::setClock requires a clock; pass a MonotonicClock for the default");
        }
        if (isRunning_) {
            throw std::logic_error("StaticExecutionEngine::setClock must be called before initialize(): this "
                                   "machine has already armed its entry configuration against the previous "
                                   "clock, and deadlines from two clocks do not compare");
        }
        clock_ = std::move(clock);
    }

    /**
     * @brief Move this engine's clock forward by `ms` and run what that made due
     *
     * The host-owned twin of `tick()`: `tick()` asks a clock that moves on its
     * own what time it is, this one *sets* what time it is and then ticks. A
     * machine driven exclusively through here has no dependency on the load of
     * the machine it runs on — the same sequence of calls produces the same
     * configuration every time, which is what a simulation, a replay and a
     * deterministic test each need.
     *
     * Throws unless a `ManualClock` is installed, because that is the only kind
     * of clock a host can move. Calling it against the monotonic default is a
     * programming error, not a no-op: the caller believes it owns time and it
     * does not, so the events it is waiting for would arrive on a schedule it
     * did not choose.
     */
    void advanceTimeMs(uint64_t ms) {
        auto manual = std::dynamic_pointer_cast<SCE::ManualClock>(clock_);
        if (!manual) {
            throw std::logic_error("StaticExecutionEngine::advanceTimeMs needs a ManualClock; this machine's "
                                   "time is not the host's to move. Call setClock(std::make_shared<ManualClock>()) "
                                   "before initialize(), or drive it with tick() and timeUntilNextScheduled()");
        }
        manual->advance(ms);
        tick();
    }

    /**
     * @brief This engine's current reading of its clock, in milliseconds
     *
     * The absolute counterpart of `timeUntilNextScheduled()`'s relative answer.
     * A host owning time through a `ManualClock` uses it to say where in the
     * run it is; a host on the wall clock uses it to correlate the engine's
     * deadlines with its own log.
     */
    uint64_t nowMs() const {
        return schedNowMs();
    }

    /**
     * @brief Macrosteps run on a scheduler-driven machine before any `tick()`
     *
     * Non-zero means the host is driving with `step()` a machine whose policy
     * declares `NEEDS_EVENT_SCHEDULER`: the delayed events sitting in the
     * scheduler have had no opportunity to fire. It stops counting once
     * `tick()` has run, so a host that mixes the two reads its start-up and
     * nothing after. Always 0 for a machine with no delayed send, whatever the
     * host calls.
     *
     * A test harness can assert on it; a supervising host can log it. Either
     * way the wiring mistake becomes something a program can see, which is what
     * a `step()`-only loop otherwise never offers.
     */
    uint32_t unattendedSchedulerSteps() const {
        return unattendedSchedulerSteps_;
    }

    /**
     * @brief External events this engine discarded because nothing matched them
     *
     * §scxml-3.1.2: "If no transition matches in any state, the event is
     * discarded." Discarding is what the clause requires. This is the part the
     * clause does not cover: the host that fed the event in cannot otherwise
     * tell that outcome from a handled one, because a self transition, a
     * targetless internal transition and a discard all leave the configuration
     * alone. Comparing the count across a drive turns "the machine ignored what
     * I sent" into something the program can see.
     *
     * The Interpreter has answered this all along — `StateMachine::processEvent`
     * returns a `TransitionResult` whose `success` is false, and
     * `getStatistics().failedTransitions` counts them. This is the AOT side of
     * the same question, so a document moving from one engine to the other
     * keeps it.
     *
     * External events only: an internal `<raise>` that matches nothing is
     * discarded too, but both ends of that are inside the document.
     */
    uint32_t discardedExternalEvents() const {
        return discardedExternalEvents_;
    }

    /**
     * @brief The most recent event `discardedExternalEvents()` counted
     *
     * `std::nullopt` while that count is zero — the generated `Event` enum's
     * zero value is a real event and cannot stand in for "none". A count says
     * something went nowhere; this says which thing did, which is the question
     * a host debugging a stalled supervisor actually has.
     */
    std::optional<Event> lastDiscardedEvent() const {
        if (!hasDiscardedEvent_) {
            return std::nullopt;
        }
        return lastDiscardedEvent_;
    }

    /**
     * @brief `error.*` events this engine raised that no transition answered
     *
     * §scxml-3.12.2 requires the processor to signal its own failures as
     * `error.*` events on the internal queue, and says in the same breath that
     * "they are ignored if no transition is found that matches them". Being
     * ignored is the clause. Being unable to say it happened is not, and the
     * difference matters to exactly one party: the host, which did not write the
     * document, cannot see the failure anywhere in the configuration, and is the
     * only one positioned to do something about it. A supervisor driving a
     * machine whose `<assign>` silently fails every round reads `isRunning() ==
     * true` and a plausible state forever.
     *
     * The sibling of `discardedExternalEvents()`, and deliberately a separate
     * count. That one stops at the external queue because an author's unmatched
     * `<raise>` has both ends inside the document; an error event's sender is
     * the engine, so the same reasoning does not reach it. An author's `<raise>`
     * that matches nothing is still not counted here.
     *
     * An error the document *did* answer is not counted either — the document
     * dealt with it, and its handling is visible in the configuration the host
     * can already read. What this counts is only the silent case.
     *
     * The Interpreter has answered this all along, through
     * `getLastStateMachineError()` and the message it passes to
     * `raiseEvent("error.execution", msg)`. This is the AOT side of the same
     * question, so a document moving from one engine to the other keeps it.
     */
    uint32_t unhandledErrorEvents() const {
        return unhandledErrorEvents_;
    }

    /**
     * @brief The most recent `error.*` event `unhandledErrorEvents()` counted
     *
     * `std::nullopt` while that count is zero — the generated `Event` enum's
     * zero value is a real event and cannot stand in for "none".
     *
     * Which error it was narrows a silent failure from "something in this
     * machine is broken" to a class: `error.execution` is the document's own
     * executable content failing, `error.communication` is a `<send>` or
     * `<invoke>` that could not reach its target — two different repairs, and a
     * count alone separates neither.
     */
    std::optional<Event> lastUnhandledError() const {
        if (!hasUnhandledError_) {
            return std::nullopt;
        }
        return lastUnhandledError_;
    }

    /**
     * @brief `error.*` events refused because an error handler kept raising them
     *
     * §scxml-3.12.2 says an unmatched error event is ignored, and
     * `unhandledErrorEvents()` is that case. This is its opposite and its worse
     * half: the document *does* match the error, and the handler fails the same
     * way every time. The failure raises `error.execution`, the same transition
     * answers it, and the drain never empties. Nothing in the clause covers it —
     * it bounds what happens to an error nobody wants, not an error everybody
     * wants and nobody can handle.
     *
     * Left to run, that is not a hang: it is a core at 100% forever. Measured
     * 2026-08-19 on a two-line document, the Python engine turned 37,000 links a
     * second while its configuration never moved and `isRunning()` stayed true —
     * the exact reading an unattended supervisor takes as healthy. So the engine
     * stops feeding the chain after `MAX_ERROR_CASCADE_DEPTH` links and says how
     * often it had to.
     *
     * A document that fails five hundred times cleanly counts zero here: the
     * chain is measured from *handler to handler*, not from failure to failure,
     * and any other internal event resets it. Nothing is discarded that a
     * working document would have processed.
     */
    uint32_t errorCascadeEvents() const {
        return errorCascadeEvents_;
    }

    /**
     * @brief The most recent `error.*` event `errorCascadeEvents()` refused
     *
     * `std::nullopt` while that count is zero — the generated `Event` enum's
     * zero value is a real event and cannot stand in for "none".
     *
     * Which error it was names the repair: `error.execution` is a handler whose
     * own executable content fails, `error.communication` one that answers an
     * unreachable target by talking to it again.
     */
    std::optional<Event> lastErrorCascadeEvent() const {
        if (!hasErrorCascadeEvent_) {
            return std::nullopt;
        }
        return lastErrorCascadeEvent_;
    }

    /**
     * @brief Macrosteps stopped short because their chain was still going
     *        after `MAX_MACROSTEP_MICROSTEPS` microsteps
     *
     * The specification says a macrostep ends in a configuration where nothing
     * is enabled by NULL and no internal event is left, and its Principles and
     * Constraints add that a macrostep *may not terminate* and that this "is
     * currently allowed". A document with a cyclic eventless transition is
     * therefore not malformed, and neither is one whose `<raise>` answers
     * itself; both are documents whose macrostep is infinite, and an engine
     * that runs either to the letter never returns.
     *
     * Both are counted here, because they are the same fact to a host: the
     * macrostep it just drove did not reach a stable configuration. Which
     * chain it was is what `lastTruncatedMacrostepState()` points at.
     *
     * This engine does not run it to the letter. It stops, and this count is
     * how a host learns that it did — because every other reading says the
     * opposite: `getCurrentState()` answers, `isRunning()` is true, and the
     * call returned at once. The configuration behind those answers is *not*
     * the stable one the clause promises; it is wherever the hundredth
     * microstep happened to land, and the document has more to do that this
     * engine will not do.
     *
     * Until 2026-08-20 this engine answered the case a third way again: the
     * ceiling called `stop()`, so the same document that merely paused
     * elsewhere came back dead here — and its off-by-one meant a chain of
     * ninety-nine microsteps that settled on its own was stopped too. The
     * ceiling is now on microsteps *taken* and is only counted when a
     * transition was still enabled after them, so a document whose chain ends
     * at the ceiling passes through untouched.
     */
    uint32_t truncatedMacrosteps() const {
        return truncatedMacrosteps_;
    }

    /**
     * @brief The state this engine was in when it last stopped a macrostep
     *        that way
     *
     * `std::nullopt` while `truncatedMacrosteps()` is zero — the generated
     * `State` enum's zero value is a real state and cannot stand in for
     * "none".
     *
     * Which state it was is the whole repair: an endless chain is a closed
     * walk through the state graph, and this names one state on it — the
     * source of the transition that was refused, or the state the drain was
     * standing in when it stopped taking internal events. The count alone says
     * a document somewhere cannot settle; this says where to look.
     */
    std::optional<State> lastTruncatedMacrostepState() const {
        if (!hasTruncatedMacrostep_) {
            return std::nullopt;
        }
        return lastTruncatedMacrostepState_;
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
    /// `pollInterval` is a ceiling on the wait between ticks, not the interval
    /// actually slept: a nearer scheduler deadline shortens it, so a caller
    /// passing an interval coarser than the document's delays no longer steps
    /// over them.
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

            // Sleep until the next deadline, or `pollInterval`, whichever comes
            // first. The scheduler's own answer wins whenever it is nearer:
            // sleeping past a deadline is what turns a coarse interval into a
            // document that behaves differently, and waking on it costs nothing.
            auto nextDue = timeUntilNextScheduled();
            std::this_thread::sleep_for(nextDue.has_value() ? std::min(pollInterval, *nextDue) : pollInterval);

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
     * @param pathChild The child of @p state the entry set already holds, when
     *        @p state is merely an ANCESTOR of the entry target. Such a state
     *        is entered without its default initial child; `nullopt` means
     *        @p state is the target itself and takes its defaults. See
     *        `integration_resources/ancestor_entry_is_not_default_entry/`.
     */
    void executeOnEntry(State state, std::optional<State> pathChild = std::nullopt) {
        // Call through policy instance (works for both static and non-static)
        policy_.executeEntryActions(state, *this, pathChild);
    }

    /// Enter a whole root-to-target chain, giving every link BEFORE the target
    /// the next one as its `pathChild`. One place, because the entry-chain
    /// walks in this engine owe the same rule and a chain walked with `nullopt`
    /// throughout puts two children of one compound state in the configuration.
    ///
    /// The chain does not stop at @p target: `buildEntryChain` appends the
    /// target's own default initial descendants, and names only
    /// `getInitialChild`, leaving the intermediate levels of a DEEP `initial`
    /// to `executeEntryActions`. Everything from the target onwards therefore
    /// takes its defaults — treating that tail as an ancestor chain suppresses
    /// exactly the descent it was appended to trigger.
    template <typename Chain> void executeOnEntryChain(const Chain &entryChain, State target) {
        bool reachedTarget = false;
        for (std::size_t i = 0; i < entryChain.size(); ++i) {
            if (entryChain[i] == target) {
                reachedTarget = true;
            }
            const std::optional<State> pathChild =
                (!reachedTarget && i + 1 < entryChain.size()) ? std::optional<State>(entryChain[i + 1]) : std::nullopt;
            SCE_LOG_DEBUG("AOT executeOnEntryChain: entering {}", static_cast<int>(entryChain[i]));
            executeOnEntry(entryChain[i], pathChild);
        }
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
                if (macrostepTruncated_) {
                    // Either branch may have spent the last of the budget.
                    // Without this the loop turns forever on a chain that is
                    // no longer being drained: the queue stays non-empty
                    // precisely because the drain refused it.
                    break;
                }
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
            //
            // Not when this macrostep was already stopped at the ceiling: the
            // queue is non-empty because the drain refused it, so looping back
            // is a spin that takes no microstep, logs nothing, and never ends.
            // Falling through to the external dequeue instead is what keeps a
            // machine inside an endless chain reachable at all — the event
            // that rescues it is on that queue, and §scxml-3.13's priority
            // would otherwise hold it behind a chain that never ends.
            if (!macrostepTruncated_ && internalQueue_.hasEvents()) {
                continue;
            }

            if (!processNextExternalEvent()) {
                break;
            }
        }
    }

    /**
     * @brief Drain the internal event queue (§scxml-C-1, high priority)
     *
     * Bounded by the same macrostep budget the eventless branch spends, and
     * for the same reason: a `<raise>` answered by a transition that raises
     * again is a macrostep that never ends, exactly as a cyclic eventless
     * transition is. Until 2026-08-20 this branch had no ceiling in any of the
     * seven engines here, so that document did not return at all.
     */
    void processInternalQueue() {
        if (macrostepTruncated_) {
            // The eventless branch of this same macrostep already ran out of
            // budget. Draining now would hand the chain a second one.
            return;
        }
        SCE_LOG_DEBUG("AOT processInternalQueue: Starting internal queue processing");
        // §scxml-3.13: Process internal queue first (high priority)
        SCE::Core::AOTEventQueue<EventWithMetadata> internalAdapter(internalQueue_);
        SCE::Core::EventProcessingAlgorithms::processInternalEventQueue(
            internalAdapter,
            [this](const EventWithMetadata &eventWithMeta) {
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

                // §scxml-3.12.2: the processor raises `error.*` into this queue
                // and the clause says they "are ignored if no transition is
                // found that matches them". Ignoring them is the clause;
                // staying silent about it is not. `discardedExternalEvents()`
                // deliberately stops at the external queue because an unmatched
                // `<raise>` has both ends inside the document — but the sender
                // of an error event is this engine, so that reasoning does not
                // reach it. The host never wrote the document, cannot see the
                // failure in the configuration, and is the only party able to
                // act on it.
                //
                // The selection runs first and unconditionally: it is what
                // processes every internal event, and folding it into the
                // condition below would skip it for everything that is not an
                // error.
                // An error raised from here on is raised *by an error
                // handler*, which is the one situation the engine cannot leave
                // to the document: the handler that failed is the same one
                // that will answer the failure. The flag is what `raise()`
                // reads to tell that apart from a first failure, and it is
                // cleared before anything else can run so a chain cannot be
                // attributed to the wrong event.
                const bool isError = SCE::Core::EventMatchingHelper::isErrorEvent(policy_.getEventName(event));
                // The chain is not ended by the drain doing something else. An
                // earlier draft reset the depth on every non-error event,
                // which reads as the careful choice and is the opposite: a
                // handler that raises its own event before failing — a
                // document that logs, then fails, which is most of them —
                // leaves the queue alternating `tick, error, tick, error…`,
                // and each `tick` put the ceiling back out of reach. The count
                // needs no such guard, because it only ever rises while an
                // error handler is running.
                handlingErrorEvent_ = isError;
                const EventOutcome outcome = executeTransition(event);
                handlingErrorEvent_ = false;
                recordInternalEventOutcome(event, outcome);
                if (outcome.selected) {
                    // Appendix D: the loop turn that selects nothing takes no
                    // microstep, so it spends no budget. Only a turn that
                    // answered the event moved the machine, and only those are
                    // what a ceiling on microsteps can be counted in.
                    ++macrostepMicrostepsTaken_;
                }
                return true;  // Continue processing
            },
            [this] {
                if (macrostepMicrostepsTaken_ < MAX_MACROSTEP_MICROSTEPS) {
                    return true;
                }
                // Work is still queued one microstep past the budget, so this
                // is the case the specification calls a macrostep that does
                // not terminate. Refusing here leaves the event on the queue,
                // which is where the next macrostep will find it, and the
                // count says the configuration a host reads now is not a
                // stable one.
                recordTruncatedMacrostep(currentState_);
                SCE_LOG_ERROR("StaticExecutionEngine: macrostep still going after {} microsteps; stopped draining "
                              "the internal queue",
                              MAX_MACROSTEP_MICROSTEPS);
                return false;
            });
        if (macrostepTruncated_) {
            // The chain was refused rather than finished, so the queue is not
            // empty and the error-cascade depth below belongs to a chain that
            // is still standing.
            return;
        }
        // The queue emptied, so the chain — refused or merely finished — is
        // over. A machine whose next macrostep starts a new one starts it from
        // zero, and the count of what was refused stays where the host reads it.
        errorCascadeDepth_ = 0;
    }

    /**
     * @brief Run the preliminary step every external event gets before
     *        transition selection (§scxml-D-mainEventLoop)
     *
     * Appendix D binds this step to *an external event the machine is about to
     * process*, not to the queue it happened to arrive on:
     *
     *     externalEvent = externalQueue.dequeue()
     *     datamodel["_event"] = externalEvent
     *     for state in configuration:
     *         for inv in state.invoke:
     *             if inv.invokeid == externalEvent.invokeid:
     *                 applyFinalize(inv, externalEvent)
     *             if inv.autoforward:
     *                 send(inv.id, externalEvent)
     *     enabledTransitions = selectTransitions(externalEvent)
     *
     * This engine has two doors an external event can come through — the
     * queue drain and `processEvent()`, which hands the event straight to
     * `executeTransition` — and the step belongs to both. It lives in one
     * function for the reason ARCHITECTURE.md gives for every shared helper:
     * two copies are two chances for one of them to stop running, and this
     * seam has already produced that defect once. `processEvent(Event, const
     * EventMetadata&)` used to be the door that skipped
     * `populatePolicyFromMetadata`, so a host handed over a payload the
     * document could never see. Measured 2026-08-21: the same door was still
     * skipping `<finalize>` and autoforward, so an `autoforward` child saw
     * nothing a host delivered this way while the identical machine driven
     * through the queue forwarded it.
     */
    void applyExternalEventPreamble(const EventWithMetadata &eventWithMeta) {
        currentEventInvokeId_ = eventWithMeta.invokeId;
        SCE::Common::EventMetadataHelper::populatePolicyFromMetadata<StatePolicy, Event>(policy_, eventWithMeta);

        // §scxml-6.5: Execute finalize BEFORE processing child events
        if constexpr (SCE::Core::HasFinalize<StatePolicy, EventWithMetadata, StaticExecutionEngine<StatePolicy>>) {
            policy_.executeFinalizeForChildEvent(eventWithMeta, *this);
        }

        // §scxml-D-mainEventLoop: autoforward belongs to the same preliminary
        // step as `<finalize>` above — both run against the external event
        // this macrostep is about to select transitions for, and both run
        // before that selection. §scxml-6.4.2 fixes the position in prose as
        // well: the parent forwards "at the point at which it removes it from
        // the external event queue".
        //
        // Forwarding where the event is *enqueued* instead is a different
        // algorithm, not an earlier one. `raiseExternal` runs inside whatever
        // executable content produced the event, so a transition body that
        // queues two events hands the child both of them before the parent
        // has processed either — the child runs a whole event ahead and the
        // two sessions stop agreeing on what has happened. Run-to-completion
        // is a property of the loop's shape, so the forward has to live where
        // the event is taken up, which is what this function is.
        //
        // ARCHITECTURE.md Zero Duplication: Policy handles child forwarding (forwardToAutoforwardChildren)
        //
        // SCE_MESH.md §mesh-9.6.5: this is the "parent runtime observes each
        // external event" point the section names, and the copy handed to the
        // policy is verbatim — §scxml-6.4 requires an exact copy, so the child
        // must see the same `_event.data`, `_event.origin`, `_event.sendid`,
        // `_event.origintype` and `_event.invokeid` the parent sees. Only
        // `target` is withheld (a routing decision owned by the originating
        // `<send>`; inheriting it would bounce the copy back onto the mesh
        // instead of delivering it to the child) — see ForwardedEvent.h.
        if constexpr (SCE::Core::HasAutoforward<StatePolicy, StaticExecutionEngine>) {
            ::SCE::Common::ForwardedEvent forwarded;
            forwarded.name = policy_.getEventName(eventWithMeta.event);
            forwarded.data = eventWithMeta.data;
            forwarded.origin = eventWithMeta.origin;
            forwarded.sendId = eventWithMeta.sendId;
            forwarded.type = eventWithMeta.type;
            forwarded.originType = eventWithMeta.originType;
            forwarded.invokeId = eventWithMeta.invokeId;
            SCE_LOG_DEBUG("AOT applyExternalEventPreamble: Autoforwarding external event '{}'", forwarded.name);
            policy_.forwardToAutoforwardChildren(forwarded, *this);
        }
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
        // §scxml-D-mainEventLoop: taking an event off the external queue is
        // where a macrostep begins, so it is where the previous one's ceiling
        // stops applying. A machine left inside an endless chain gets a full
        // budget for each event it is given, and each refusal is counted
        // separately — which is what tells a host that spins once from one
        // that spins on everything.
        //
        // Here and not at the entry to `runMainEventLoop`, which reads like
        // the more general boundary and is not one: a machine whose chain was
        // refused would spend a whole budget re-walking it before it ever
        // looked at the event the host sent to get it out. The refused events
        // stay queued either way — this is where the budget that drains them
        // comes back.
        macrostepTruncated_ = false;
        macrostepMicrostepsTaken_ = 0;
        {
            Event event = eventWithMeta.event;
            applyExternalEventPreamble(eventWithMeta);
            recordEventOutcome(event, executeTransition(event));
        }
        return true;
    }

    /**
     * @brief Publish a macrostep this engine stopped short, from whichever
     *        branch of the main event loop's inner loop ran out of budget
     *
     * One function, two callers, for the reason the budget is one number: a
     * host reads a macrostep that did not reach a stable configuration, and
     * the branch it died in is a detail of the document, not of the contract.
     * Two copies of this would be two chances for one of them to stop setting
     * the flag that keeps the same chain from being handed a second budget.
     */
    void recordTruncatedMacrostep(State state) {
        ++truncatedMacrosteps_;
        lastTruncatedMacrostepState_ = state;
        hasTruncatedMacrostep_ = true;
        macrostepTruncated_ = true;
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
        if (macrostepTruncated_) {
            // This macrostep was already stopped at the ceiling. Re-entering
            // the drain would hand the same chain a second budget, which is
            // the runaway the ceiling exists to refuse.
            return;
        }
        // Microsteps taken, not loop turns: the turn that finds nothing
        // enabled is how a macrostep ends, and counting it would spend the
        // budget on the proof that no budget was needed. Counting turns is
        // what made a chain of ninety-nine microsteps that settled on its own
        // read as a runaway here, and the verdict called `stop()`. The count
        // lives on the engine because the macrostep does — see
        // `macrostepMicrostepsTaken_`.

        // §scxml-3.13: Use shared algorithm (Single Source of Truth)
        // Note: Eventless transitions can raise new internal events, use internal queue
        SCE::Core::AOTEventQueue<EventWithMetadata> adapter(internalQueue_);

        while (true) {
            State oldState = currentState_;
            std::vector<State> preTransitionStates = getActiveStates();  // §scxml-3.11: Capture before transition
            SCE_LOG_DEBUG("AOT checkEventlessTransitions: Microstep {}, currentState={}", macrostepMicrostepsTaken_,
                          static_cast<int>(currentState_));

            // Call processTransition with default event for eventless transitions
            if (policy_.processTransition(currentState_, Event(), *this)) {
                if (macrostepMicrostepsTaken_ == MAX_MACROSTEP_MICROSTEPS) {
                    // The chain is still going one microstep past the budget,
                    // so this is the case the specification's Principles and
                    // Constraints call a macrostep that does not terminate.
                    // Refuse the microstep rather than take it, and publish
                    // the refusal: the configuration left behind is not a
                    // stable one and only this counter says so. The machine
                    // keeps running — the specification allows the document, so
                    // declining to run it forever is a fact to report, not
                    // grounds to kill a session whose other states still work.
                    //
                    // `processTransition` only selects for a non-parallel
                    // machine — the exit / body / entry chain is
                    // `handleHierarchicalTransition`'s below — so putting
                    // `currentState_` back is what makes the refusal exact.
                    // A parallel policy runs `executeMicrostep` inside the
                    // call instead, so there the microstep is already taken
                    // and the ceiling holds one microstep later; the count
                    // means the same thing either way.
                    if constexpr (!StatePolicy::HAS_PARALLEL_STATES) {
                        currentState_ = oldState;
                    }
                    recordTruncatedMacrostep(currentState_);
                    SCE_LOG_ERROR(
                        "StaticExecutionEngine: macrostep still going after {} microsteps; stopped taking them",
                        MAX_MACROSTEP_MICROSTEPS);
                    break;
                }
                ++macrostepMicrostepsTaken_;
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
                } else if (policy_.lastTransitionIsTargetless_) {
                    // §scxml-3.13: a transition with no `target` exits and
                    // enters nothing and runs its content in place. The
                    // configuration is unchanged by definition, so a loop that
                    // continued only on a changed configuration selected this
                    // transition and then dropped it — the content never ran,
                    // and the chain ended one microstep early. Running it here
                    // is the same decision `executeTransition` makes for the
                    // event-driven case, read off the same policy flag.
                    policy_.executeTransitionActions(*this);
                } else {
                    // §scxml-3.13: a self transition with a target exits and
                    // re-enters its state, which is work even though the
                    // configuration ends where it started. The ceiling above is
                    // what stops the chain it can open; leaving early instead
                    // skipped the exit and entry the clause requires.
                    if constexpr (!StatePolicy::HAS_PARALLEL_STATES) {
                        handleHierarchicalTransition(actualSourceState, currentState_, preTransitionStates,
                                                     [this] { policy_.executeTransitionActions(*this); });
                    }
                }
            } else {
                // §scxml-3.13: No eventless transition available - stable configuration reached
                // Internal events will be processed by caller (runMainEventLoop or step)
                break;
            }
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
        // §scxml-3.13: entering the initial configuration is one turn, and the
        // `<onentry>` handlers it runs arm their `<send delay>`s against one
        // instant — see `beginTurn()` for what reading the clock per `<send>`
        // did to two of them.
        TurnGuard turn(*this);

        isRunning_ = true;

        // §scxml-5.3: Initialize datamodel before any state entry
        // This ensures error.execution events are raised immediately if initialization fails
        if constexpr (SCE::Core::HasDataModelInit<StatePolicy, StaticExecutionEngine>) {
            policy_.initializeDataModel(*this);
        }

        // §scxml-3.3: Use HierarchicalStateHelper for correct entry order
        auto entryChain = SCE::Core::HierarchicalStateHelper<StatePolicy>::buildEntryChain(currentState_);

        // Execute entry actions from root to leaf (ancestor first).
        //
        // Every link here takes its defaults — this is deliberately NOT
        // `executeOnEntryChain`. §scxml-D-addAncestorStatesToEnter is about a
        // state on the way to a target somebody NAMED; this chain is the
        // opposite, a default descent whose leaf the policy has already
        // resolved. Measured 2026-08-15: passing the next link here suppressed
        // `s1`'s deep `initial="s11p112 s11p122"` and W3C test364 failed.
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
        // §scxml-3.13: one host call, one reading. The macrostep below can
        // enter a state whose `<onentry>` arms several `<send delay>`s, and
        // they are one instant's worth of executable content however long the
        // host takes to run them — see `beginTurn()`.
        TurnGuard turn(*this);

        // A machine with delayed sends hands `step()` a queue it cannot reach:
        // `runMainEventLoop()` never consults the scheduler, so the event is
        // neither delivered nor refused. Say it once — the host is driving with
        // the wrong call, and every later macrostep would repeat the same word.
        if constexpr (::SCE::Core::NeedsEventScheduler<StatePolicy>) {
            if (!tickHasRun_) {
                ++unattendedSchedulerSteps_;
                if (unattendedSchedulerSteps_ == 1) {
                    SCE_LOG_ERROR("AOT step: this machine has delayed sends and no tick() has run; "
                                  "delayed events will never fire - drive it with tick()");
                }
            }
        }

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
        // §scxml-D-mainEventLoop: a host event with nothing said about it is
        // still an external event, so it is carried the same way the queue
        // carries one. The empty carrier is the point, not an omission — the
        // fields it clears are the previous event's, and leaving them standing
        // is what let a stale `_event.data` be read as this event's.
        EventWithMetadata carried;
        carried.event = event;
        processEventImpl(carried);
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
        // §scxml-5.10: the metadata has to reach the `_event` binding, not
        // merely be stored. `currentEventMetadata_` is read by the invoke
        // finalize path; what fills `_event.data` / `.origin` / `.sendid` is
        // the policy's pending fields, and those were populated only by the
        // QUEUE drain — so a host that called this overload handed over a
        // payload the document could never see. Measured 2026-08-16: five
        // backends deliver a host payload on their equivalent call and this
        // one silently dropped it, with `_event.data` left at whatever the
        // previous dequeue had put there.
        //
        // Carried rather than applied here, so this door and the queue drain
        // reach `applyExternalEventPreamble` with the same shape and cannot
        // answer differently about what a metadata field means. A relayed
        // child event arrives through here with its `invokeid` intact, which
        // is what §scxml-6.5 matches a `<finalize>` on.
        EventWithMetadata carried;
        carried.event = event;
        carried.data = metadata.data;
        carried.origin = metadata.originSessionId;
        carried.sendId = metadata.sendId;
        carried.type = metadata.type;
        carried.originType = metadata.originType;
        carried.invokeId = metadata.invokeId;
        carried.typedData = metadata.typedData;
        processEventImpl(carried);
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
     *
     * Draining without a macrostep between entries is precisely what makes it
     * unsuitable for a plain loop: everything past its deadline lands on the
     * external queue together, so a `<cancel>` executed by the first one's
     * transitions can no longer reach the rest. `tick()` promotes them one at
     * a time for that reason.
     */
    void pumpScheduledEvents() {
        TurnGuard turn(*this);
        std::string eventData;
        Event event;
        while (scheduler_.popReadyEvent(schedNowMs(), event, eventData)) {
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
        // §scxml-3.13: one turn, one reading. Everything below judges due
        // against the instant this tick began, and everything the macrosteps
        // below arm is measured from it — so a tick dispatches what was due
        // when the host called it, and cannot be extended by how long it takes
        // to run (see `beginTurn()`).
        TurnGuard turn(*this);

        // Recorded before the running check: a host that calls `tick()` owns a
        // clock whatever the engine's lifecycle says, and the count exists to
        // find hosts that never call it at all.
        tickHasRun_ = true;
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

        // §scxml-6.2: dispatch the ready scheduled events, earliest deadline
        // first and one macrostep apart — not `pumpScheduledEvents()`, which
        // drains them together by design.
        //
        // `<cancel>` drops an event that has not been dispatched yet, and a
        // host that woke late holds several past their deadlines. Promoting
        // them together makes every later one undroppable before the earlier
        // one's transitions have run, which is how a settle timer — arm a long
        // `<send delay>`, cancel it when the short signal arrives first —
        // delivers the event it was told to cancel. Measured 2026-08-19 on the
        // Rust, Go and Python backends alike; C++ shares the shape.
        //
        // "Due" is judged against the instant this tick began, not against a
        // clock re-read on every pass: a tick that chased its own slowness
        // would dispatch entries the host had not yet reached, in a loop the
        // host cannot get between (see `beginTurn()`).
        {
            std::string eventData;
            Event event;
            while (scheduler_.popReadyEvent(schedNowMs(), event, eventData)) {
                raiseExternal(event, eventData);
                // The macrostep this event drives may `<cancel>` a later one,
                // so the queue is re-consulted after it rather than before.
                runMainEventLoop();
                if (!isRunning_ || isInFinalState()) {
                    break;
                }
            }
        }

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
