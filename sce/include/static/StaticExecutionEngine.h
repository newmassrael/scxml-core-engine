// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/CompilerHints.h"
#include "common/EventMetadataHelper.h"
#include "common/EventTypeHelper.h"
#include "common/ForwardedEvent.h"
#include "common/IOProcessorHelper.h"
#include "common/MeshEndpoint.h"
#include "common/NoParent.h"
#include "common/SCXMLConstants.h"
#include "common/SceClock.h"
#include "common/SendHelper.h"
#include "common/SendSchedulingHelper.h"
#include "core/AOTEventQueue.h"
#include "core/ConfigurationHelper.h"
#include "core/ConflictResolutionHelper.h"
// docs/adr/0084: what another thread has for this engine, applied by its owner.
#include "core/EngineInbox.h"
#include "core/EntrySetHelper.h"
#include "core/EventMatchingHelper.h"
#include "core/EventMetadata.h"
#include "core/EventNameResolver.h"
#include "core/EventProcessingAlgorithms.h"
#include "core/EventQueueManager.h"
#include "core/ExternalEventBudget.h"
#include "core/HierarchicalStateHelper.h"
#include "core/HistoryHelper.h"
#include "core/HostInvokeLedger.h"
#include "core/MacrostepCeilings.h"
#include "core/MicrostepAlgorithms.h"
#include "core/ParallelTransitionHelper.h"
// §scxml-6.2.5: the request/reply shape a host-declared Event I/O Processor is
// dispatched with. A `core/` header for the same reason `PayloadReading.h` is
// one — it is a fact about a `<send>`, not about any engine's interface, and
// the Interpreter can grow the same door without moving the types.
#include "core/HostProcessor.h"
#include "core/LogMacros.h"
// §scxml-B-2-8-1: the rung a delivered payload got, which this engine counts.
// A `core/` header on purpose — this engine includes nothing from `scripting/`
// because the policy owns the script-engine handle, and the reading is a fact
// about the event rather than about that interface.
#include "core/PayloadReading.h"
#include "core/StatePolicyConcepts.h"
// docs/adr/0084, D2 as amended at step 10: the tree's directory of sessions, which
// hands an input that names a session to that session's inbox when it is posted.
#include "core/TreeSessions.h"
#include "events/EventDescriptor.h"
#include <algorithm>
#include <any>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <iterator>
#include <map>
#include <memory>
#include <optional>
#include <set>
#include <stdexcept>
#include <string>
#include <thread>
#include <utility>
#include <variant>
#include <vector>

namespace SCE::Saved {
/// The door a saved state goes through into an engine (`saved/SavedEngine.h`): it reads
/// what a save writes and sets what a restore stands the engine on, which no host
/// should be able to do to a machine that is running.
struct EngineAccess;
}  // namespace SCE::Saved

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
 * @brief The Mesh router a child session is given by its parent (SCE_MESH.md,
 *        Child Sessions in Mesh; docs/adr/0043)
 *
 * The child's endpoint attached to that one session; the C++ twin of the Rust
 * runtime's `ChildMeshRouter`. What a host installs on a machine it starts with
 * `setMeshSendCallback()`, the parent installs on the child it starts, before
 * the child initializes, since an `<onentry>` of the child's initial state may
 * already send: the child's `<send target="#<peer>">` goes to send(). What
 * arrives for the child waits here until the parent drives the child, which
 * takes it with takeCalls() before the child's macrostep. When the session ends
 * the parent says so with detach(), after the child has stopped.
 *
 * Raw fields and `ForwardedEvent`, never a mesh-layer type, so this header keeps
 * its zero mesh includes, as `MeshSendCallback` does. A child's
 * `<invoke type="sce:mesh-rpc">` is not served through it: the request has no
 * invoker in a child session and raises `error.execution` there.
 *
 * Threading (ARCHITECTURE.md "Engine Threading"): every member is called on the
 * thread that owns the session's tree, which is the thread its parent is driven
 * on. `takeCalls` is therefore the tree's own pull of what the router holds, an
 * inbox of the router's making: a router whose transport receives on a thread of
 * its own guards what it holds against that thread itself, and hands it over
 * only when asked.
 */
class IChildMeshRouter {
public:
    virtual ~IChildMeshRouter() = default;

    /**
     * @brief A `<send target="#<peer>">` of the child, in `MeshSendCallback`'s
     *        fields
     *
     * @return Whether the router took it. One it did not take raises
     *         `error.execution` in the child, as a send no router is there to
     *         take does; it never lands on the child's own queue.
     */
    virtual bool send(const std::string &target, const std::string &eventName, const std::string &data,
                      const std::string &sendId, const std::string &invokeId) = 0;

    /**
     * @brief What arrived for the child since it was last asked, oldest first,
     *        each with the `_event` the endpoint built for it (for a peer's
     *        event, the one SCE_MESH.md gives a Mesh-delivered event)
     */
    virtual std::vector<::SCE::Common::ForwardedEvent> takeCalls() = 0;

    /**
     * @brief The session this router was attached to has ended
     *
     * It reached a top-level `<final>`, or its parent left the invoking state
     * and cancelled it. Called once, after the child has stopped, so a send the
     * child made while it was being stopped has been handed over.
     */
    virtual void detach() = 0;

    /**
     * @brief The router of a child this child starts, asked as the parent's
     *        factory is asked (`ChildMeshRouters`)
     *
     * None by default: a child whose router answers none starts children with
     * no router, whose peer sends raise `error.execution` in them.
     */
    virtual std::shared_ptr<IChildMeshRouter> childRouter(const std::string &invokeId, const std::string &document) {
        (void)invokeId;
        (void)document;
        return nullptr;
    }
};

/// The factory of the routers of a machine's children, registered with
/// `registerChildMeshRouters()`: asked, each time an invocation starts a child,
/// with the invoke's id and the stem of the child's document (empty for a child
/// written inline). None leaves the child with no router.
using ChildMeshRouters =
    std::function<std::shared_ptr<IChildMeshRouter>(const std::string &invokeId, const std::string &document)>;

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
 * Threading: the rule is ARCHITECTURE.md "Engine Threading" (docs/adr/0084).
 * This engine runs no loop of its own; its owner is the thread that drives it.
 * A run is entered (`initialize`, `enterAt`, a restore) on any thread, and the
 * first macrostep a host drives after that (`step`, `tick`, `processEvent`,
 * `pumpScheduledEvents`, `advanceTimeMs`) makes the driving thread the owner,
 * until `stop` or the next run. The owner-thread operations are those, raising
 * an external event (`raiseExternal`), ending or cancelling a host-run
 * invocation (`completeHostInvoke`, `failHostInvoke`, `completeHostInvokeIn`,
 * `failHostInvokeIn`, `endHostInvokeIn`, `cancelHostInvoke`), applying a Mesh
 * delivery (`applyMeshCalls`) and `stop`. Another thread uses `inbox()`
 * instead, the one thread-safe handle. A debug build checks each owner-thread
 * operation and throws `std::logic_error` naming the inbox when it is called
 * off the owner.
 *
 * @tparam StatePolicy Policy class providing state-specific implementations.
 *         Must satisfy SCE::Core::EventNamingPolicyTrait, which the class body asserts.
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

template <typename StatePolicy> class StaticExecutionEngine {
    static_assert(SCE::Core::EventNamingPolicyTrait<StatePolicy>::value,
                  "StatePolicy must be an event naming policy: a base state policy (document tree, initial and "
                  "history targets, final states) whose events convert to and from their names "
                  "(EventNamingPolicyTrait)");

public:
    /// The policy this engine runs: the document's tables and the datamodel it holds.
    using StatePolicyType = StatePolicy;
    using State = typename StatePolicy::State;
    using Event = typename StatePolicy::Event;
    /// A `<history>` pseudo-state. Never a member of the configuration, only
    /// of target lists, which is why it is not a State.
    using History = typename StatePolicy::History;
    /// One member of Appendix D's `enabledTransitions`, as the policy's
    /// selection answers it.
    using TransitionInfo = SCE::Core::EnabledTransition<State, History>;

    /// One transition the engine took, as its row of the emitted transition
    /// table names it (docs/adr/0089, docs/SCE_TRANSITION_TABLE.md): `source`
    /// is the policy's own State, the type the table's `from` holds, and
    /// `index` the table's `index` type, so a pair compares with a row
    /// (`row.from`, `row.index`) without a conversion.
    struct TransitionTaken {
        /// The state the transition is written in: the `from` of its row.
        State source;
        /// Its position among `source`'s own transitions in document order:
        /// the `index` of its row.
        std::uint32_t index;

        friend bool operator==(const TransitionTaken &a, const TransitionTaken &b) {
            return a.source == b.source && a.index == b.index;
        }

        friend bool operator!=(const TransitionTaken &a, const TransitionTaken &b) {
            return !(a == b);
        }
    };

    /// What `setTransitionObserver` takes.
    using TransitionObserver = std::function<void(const TransitionTaken &)>;

    /// How many links an `error.*` chain may have before the engine stops
    /// feeding it — see `errorCascadeEvents()`. The number, and why it is that
    /// one, is `Core::MAX_ERROR_CASCADE_DEPTH`, shared with the Interpreter.
    static constexpr uint32_t MAX_ERROR_CASCADE_DEPTH = Core::MAX_ERROR_CASCADE_DEPTH;

    /// How many microsteps one macrostep may take before this engine stops
    /// taking them — see `truncatedMacrosteps()`. The number, and why it is that
    /// one, is `Core::MAX_MACROSTEP_MICROSTEPS`, shared with the Interpreter.
    static constexpr uint32_t MAX_MACROSTEP_MICROSTEPS = Core::MAX_MACROSTEP_MICROSTEPS;

    /**
     * @brief Event with metadata for §scxml-5.10 compliance
     *
     * Wraps Event enum with metadata (origin, sendid, data, type) to support
     * _event.origin, _event.origintype, _event.sendid, _event.data, _event.type fields.
     *
     * @example Constructor Pattern (required for nested template types)
     * @code
     * // Create event with data and sendId
     * engine.raise(EventWithMetadata(Event::Error_execution, errorMsg, std::nullopt, sendId));
     *
     * // Create external event a session sent, with the pair the SCXML
     * // processor writes for it (docs/adr/0090)
     * externalQueue_.raise(EventWithMetadata(
     *     Event::Foo,                                      // event
     *     "data",                                          // data
     *     IOProcessorHelper::scxmlOrigin(senderSessionId), // origin
     *     sendId,                                          // sendId
     *     "external"                                       // type
     * ));
     * @endcode
     */
    struct EventWithMetadata {
        Event event;
        // §scxml-5.10: `_event.name` is the name an event ARRIVED under. The event is
        // carried as the enum member of a name the document writes, and §scxml-3.12.1
        // lets `request.new` take a transition on `request`, so the member alone loses
        // the longer name. Set by a door that resolved a name through a prefix; empty
        // means the member's own name IS the name, which is every event the document
        // itself raises or sends.
        std::string name;
        std::string data;
        // §scxml-5.10.1: `_event.origin` and `_event.origintype`, one value the
        // delivering processor wrote (docs/adr/0090); none for an event no
        // processor stamped, which reads both fields as the empty string.
        std::optional<Core::EventOrigin> origin;
        // The id of the session that sent the event, written by the processor
        // that delivered it from that session: the SCXML processor for a local
        // session, the Mesh transport for a remote child or parent; empty for
        // any other sender. Not an `_event` field: `<finalize>` and a remote
        // child's admission find the child by it, never by the origin's address.
        std::string sender;
        std::string sendId;                    // §scxml-5.10.1: _event.sendid
        std::string type;                      // §scxml-5.10.1: _event.type
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
        // §scxml-6.4: the token of the host-run invocation this event
        // completes, set only by completeHostInvoke() after it confirmed the
        // invocation was still running. A host invoke's `done.invoke.<id>`
        // that arrives without one did not come through that check, so the
        // engine refuses it. Not script-visible: `_event` has no field for it.
        std::optional<uint64_t> hostInvokeToken;

        // Default constructor for aggregate initialization
        EventWithMetadata() = default;

        // Constructor with positional parameters (event, data, origin, sendId, type, invokeId, target).
        // The origin is a pair or none, never a string, so an argument list written for
        // the string fields cannot bind to it.
        EventWithMetadata(Event e, const std::string &d = "", std::optional<Core::EventOrigin> o = std::nullopt,
                          const std::string &s = "", const std::string &t = "", const std::string &i = "",
                          const std::string &tgt = "")
            : event(e), data(d), origin(std::move(o)), sendId(s), type(t), invokeId(i), target(tgt) {}
    };

private:
    /// What a door made of a delivery just taken off a queue (docs/adr/0086,
    /// section 4.2).
    enum class Admission {
        /// Its data fits its type, or the document reads none of it: it is taken.
        Admitted,
        /// It is refused whole, and the `error.execution` that carries the
        /// refusal is bound and taken in its place.
        InPlace,
        /// It is refused whole, and the document declares no `error.execution`
        /// to take in its place.
        Refused
    };

public:
    // ── What another thread may post (docs/adr/0084) ─────────────────────
    //
    // The set is closed: it is the inputs that have a caller on another thread,
    // each carrying exactly what the owner-thread operation that applies it
    // takes, because the owner applies a posted input by calling that operation
    // (`applyInboxInput`). A posted input and a direct call cannot differ.

    /// An external event, applied with `raiseExternal(const EventWithMetadata &)`.
    struct InboxExternalEvent {
        EventWithMetadata event;
    };

    /// An external event named by text, applied with
    /// `raiseExternal(const ForwardedEvent &)`: resolved against the document's
    /// names when it is applied, and dropped then if no event of the document
    /// could take it.
    struct InboxExternalEventByName {
        ::SCE::Common::ForwardedEvent event;
    };

    /// The end of a host-run invocation's start. An empty `sessionId` ends one of
    /// this machine; a session id ends one of that session of the tree, this
    /// machine's own included, and the handle hands it to that session's inbox
    /// when it is posted (`Inbox::post`). The session it reached applies it with
    /// `completeHostInvoke` or `failHostInvoke`. The token check those make is
    /// made when the input is applied, so an ending posted for a start that was
    /// cancelled or started again by then raises nothing, as a direct one would
    /// (see `completeHostInvoke`).
    struct InboxHostInvokeEnding {
        std::string sessionId;
        ::SCE::HostInvokeEnding ending;
    };

    // SCE_MESH.md §9.6: what the transport of a child session running on another
    // device delivers for this machine, its parent, on the transport's own thread.
    // Each is applied with the policy's member of the same name, which a machine
    // with no such child does not have; there the input is applied as nothing,
    // as the envelope it stands for would have been dropped.

    /// Wire-15: the child that `invokeId` started runs as `childSessionId`.
    /// Applied with the policy's `onInvokeStarted`.
    struct InboxChildSessionStarted {
        std::string invokeId;
        std::string childSessionId;
        /// The child session's Mesh endpoint the wire carried
        /// (`common/MeshEndpoint.h`), empty when it carried none.
        std::string endpoint;
    };

    /// Wire-16: `event` comes from the child session `childSessionId`. Applied
    /// with `raiseExternal` when the policy's `hasActiveChildSession` still
    /// answers for that session, and dropped otherwise: the parent has left the
    /// invoking state, and an event of a cancelled child is ignored.
    struct InboxChildSessionEvent {
        std::string childSessionId;
        EventWithMetadata event;
    };

    /// Wire-18: the child `invokeId` started reached its final state with
    /// `doneData`. Applied with the policy's `onInvokeDone`.
    struct InboxChildSessionDone {
        std::string invokeId;
        std::string doneData;
    };

    /// The peer named `peer` is gone. Applied with the policy's
    /// `failScxmlRemoteInvokesForPeer`.
    struct InboxPeerLost {
        std::string peer;
    };

    /// One input of this engine's inbox.
    using InboxInput =
        std::variant<InboxExternalEvent, InboxExternalEventByName, InboxHostInvokeEnding, InboxChildSessionStarted,
                     InboxChildSessionEvent, InboxChildSessionDone, InboxPeerLost>;

    /**
     * @brief The handle another thread posts to this engine through (`inbox()`)
     *
     * Copyable and safe from any thread, as is every copy of it, and valid after
     * the engine is gone (its posts are refused then). The address of what is
     * posted is resolved when it is posted (ARCHITECTURE.md "Engine Threading";
     * docs/adr/0084, D2 as amended at step 10): an ending that names a session goes
     * to that session's own inbox, found in the tree's directory, and every other
     * input to this engine's.
     */
    class Inbox {
    public:
        /**
         * @brief Post @p input; whether it was taken
         *
         * An `InboxHostInvokeEnding` that names a session is posted to that
         * session's inbox, and refused, answering false, when the tree holds no
         * session of that id: it never started, or it has ended. That refusal
         * reached no inbox, so no inbox counts it (`refusedInboxPosts`).
         */
        bool post(InboxInput input) const {
            if (const auto *ended = std::get_if<InboxHostInvokeEnding>(&input);
                ended != nullptr && !ended->sessionId.empty()) {
                return sessions_->endHostInvokeIn(ended->sessionId, ended->ending) ==
                       ::SCE::HostInvokeEndOutcome::Delivered;
            }
            return own_.post(std::move(input));
        }

        /// Install the waker a post to any session of the tree calls; see
        /// `EngineInboxState::wakeWith`.
        void wakeWith(std::function<void()> waker) const {
            own_.wakeWith(std::move(waker));
        }

    private:
        friend StaticExecutionEngine;

        Inbox(::SCE::Core::EngineInbox<InboxInput> own, std::shared_ptr<::SCE::Core::TreeSessions> sessions) noexcept
            : own_(std::move(own)), sessions_(std::move(sessions)) {}

        ::SCE::Core::EngineInbox<InboxInput> own_;
        std::shared_ptr<::SCE::Core::TreeSessions> sessions_;
    };

private:
    // ── Appendix D over the generated policy ─────────────────────────────
    //
    // The policy answers what only the document knows: which of a state's
    // transitions an event enables, what a transition's content is, what a
    // state's onentry and onexit do, what a history recorded. What Appendix D
    // does with those answers is `SCE::Core::MicrostepAlgorithms`, written once
    // for this engine and the Interpreter; `MicrostepHost` is how this engine
    // hands it the policy. A machine with a `<parallel>` used to take a second
    // microstep written into its generated code while every other machine took
    // an LCA walk here, and the two disagreed about eventless selection, about
    // the order transition content runs in, and about how many targets a
    // transition has.

    /// This engine as `SCE::Core::MicrostepAlgorithms` reads it — that struct
    /// states what each member answers. The document is the policy's static
    /// tables; the history values and the configuration are run-time state,
    /// which is why this holds the engine.
    struct MicrostepHost {
        using State = typename StatePolicy::State;
        using History = typename StatePolicy::History;

        StaticExecutionEngine &engine;

        std::optional<State> parentOf(const State &s) const {
            return StatePolicy::getParent(s);
        }

        bool isCompound(const State &s) const {
            return SCE::Core::HierarchicalStateHelper<StatePolicy>::isTransitionDomainCandidate(s);
        }

        bool isParallel(const State &s) const {
            return StatePolicy::isParallelState(s);
        }

        std::vector<State> childStates(const State &s) const {
            return StatePolicy::getChildStates(s);
        }

        std::vector<SCE::Core::EntryTarget<State, History>> initialTargets(const State &s) const {
            return StatePolicy::getInitialTargets(s);
        }

        State historyParent(const History &h) const {
            return StatePolicy::getHistoryParent(h);
        }

        std::optional<std::vector<State>> historyValue(const History &h) const {
            return engine.policy_.historyValue(h);
        }

        std::vector<SCE::Core::EntryTarget<State, History>> historyDefaultTargets(const History &h) const {
            return StatePolicy::getHistoryDefaultTargets(h);
        }

        int documentOrder(const State &s) const {
            return StatePolicy::getDocumentOrder(s);
        }

        std::vector<State> configuration() const {
            return engine.getActiveStates();
        }

        std::optional<TransitionInfo> firstEnabledTransition(const State &s, Event event) {
            return engine.policy_.firstEnabledTransition(s, event, engine);
        }

        void exitState(const State &s, const std::vector<State> &configurationBeforeExit) {
            engine.policy_.executeExitActions(s, engine, configurationBeforeExit);
        }

        void executeTransitionContent(const TransitionInfo &t) {
            engine.policy_.executeTransitionActions(t.source, t.transitionIndex, engine);
        }

        void enterState(const State &s, bool isDefaultEntry) {
            engine.policy_.executeEntryActions(s, engine, isDefaultEntry);
            // §scxml-D-enterStates: a <final> whose parent is the <scxml>
            // element ends the run. Recorded as the run's terminal state, so
            // "where did it end" is a fact the engine keeps rather than a
            // reading of the configuration.
            if (StatePolicy::isFinalState(s) && !StatePolicy::getParent(s).has_value()) {
                engine.terminalState_ = s;
            }
        }

        void executeHistoryDefaultContent(const History &h) {
            engine.policy_.executeHistoryDefaultContent(h, engine);
        }
    };

    /**
     * @brief Appendix D's selectTransitions, or its selectEventlessTransitions
     *        when @p event is `Event()`
     *
     * @return The optimal enabled transition set, in selection order
     */
    std::vector<TransitionInfo> selectTransitions(Event event) {
        policy_.bindCurrentEvent(event, *this);
        MicrostepHost host{*this};
        return SCE::Core::MicrostepAlgorithms::selectTransitions(host, event);
    }

    /**
     * @brief Appendix D's microstep: exit, run the transitions' content, enter
     */
    void microstep(const std::vector<TransitionInfo> &transitions) {
        // docs/adr/0089: the selected set, before the first exit, each
        // transition once and in selection order, as the Python engine tells it.
        if (transitionObserver_) {
            for (const TransitionInfo &transition : transitions) {
                transitionObserver_(
                    TransitionTaken{transition.source, static_cast<std::uint32_t>(transition.transitionIndex)});
            }
        }
        MicrostepHost host{*this};
        const auto entry = SCE::Core::MicrostepAlgorithms::microstep(host, transitions);
        settleCurrentState(transitions, entry.statesToEnter);
    }

    void enterStates(const std::vector<SCE::Core::EntryTransition<State, History>> &transitions) {
        MicrostepHost host{*this};
        const auto entry = SCE::Core::MicrostepAlgorithms::enterStates(host, transitions);
        settleCurrentState(transitions, entry.statesToEnter);
    }

    /**
     * @brief Point `currentState_` at the atomic state the last transition
     *        brought the machine into
     *
     * Without a `<parallel>` the configuration is one chain and the entry set
     * ends on its atomic state. With one, `currentState_` is the first state
     * the last transition names, dereferenced, then down through the
     * configuration to an atomic state — the answer every backend gives.
     *
     * @tparam Transitions Anything whose elements carry `targets` as written:
     *         the microstep's enabled set, or the initial transition
     */
    template <typename Transitions>
    void settleCurrentState(const Transitions &transitions, const std::vector<State> &entered) {
        if (entered.empty()) {
            return;
        }
        if constexpr (!StatePolicy::HAS_PARALLEL_STATES) {
            currentState_ = entered.back();
        } else {
            for (auto it = transitions.rbegin(); it != transitions.rend(); ++it) {
                const std::vector<State> targets =
                    SCE::Core::EntrySetAlgorithms::getEffectiveTargetStates(it->targets, MicrostepHost{*this});
                if (!targets.empty()) {
                    currentState_ = targets.front();
                    resolveCurrentStateToLeaf();
                    return;
                }
            }
        }
    }

    /**
     * @brief Select, and take the microstep if anything was selected
     *
     * A delayed send to `#_parent` used to be drained here, from a queue of
     * its own beside the engine's. It is an entry in the engine's queue now,
     * delivered by `tick()` like every other delayed send (§scxml-6.2), so
     * `<cancel>` reaches it and it carries its payload.
     */
    std::vector<TransitionInfo> selectAndTakeMicrostep(Event event) {
        std::vector<TransitionInfo> enabled = selectTransitions(event);
        if (!enabled.empty()) {
            microstep(enabled);
        }
        return enabled;
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
     * @brief Offer one event to the configuration and take the microstep it
     *        selects
     *
     * §scxml-3.13: Single Source of Truth for transition execution across all
     * processing paths (event queues, direct processEvent, eventless transitions).
     *
     * Callers customize one axis of variation via a template callback:
     * - postTransition: work after the microstep, before the eventless check
     *   (queue path: nothing; direct path: runMainEventLoop)
     *
     * @tparam PostTransitionFn Callable() for post-microstep work
     * @param event Event to process
     * @param postTransition Post-microstep work before the eventless check
     * @return what the event did — see EventOutcome
     */
    template <typename PostTransitionFn>
    EventOutcome executeTransition(Event event, PostTransitionFn &&postTransition) {
        const State oldState = currentState_;
        const std::vector<TransitionInfo> taken = selectAndTakeMicrostep(event);
        if (taken.empty()) {
            return EventOutcome{};
        }

        // W3C SCXML Appendix D's main event loop returns to
        // `selectEventlessTransitions()` after EVERY microstep and drains the
        // internal queue in the same inner loop, without asking whether the
        // microstep moved the machine — a targetless transition that only ran
        // content opens a chain as surely as one that entered a state.
        postTransition();
        checkEventlessTransitions();

        // A microstep of targetless transitions moved nothing itself; the chain
        // it opened may have. This asks the machine rather than the transition,
        // so a chain that reaches a top-level `<final>` still notifies the
        // parent — see EventOutcome's contract for why the two facts are
        // spelled apart.
        const bool entered =
            std::any_of(taken.begin(), taken.end(), [](const TransitionInfo &t) { return !t.isTargetless(); });
        return EventOutcome{/*selected=*/true, /*configurationChanged=*/entered || currentState_ != oldState};
    }

    /**
     * @brief Settle `currentState_` on an atomic state
     *
     * A transition may target a compound state, or a `<parallel>`, and
     * `getCurrentState()` must still name the atomic state the machine is
     * *in* rather than one it is *within*. Measured 2026-08-13 on a two-region
     * `<parallel>` whose transition targets a compound state: the active set
     * was `[run | counter | drive | within | outer | a]` and
     * `getCurrentState()` answered `outer`, while `sce_rust_runtime`'s
     * `resolve_current_state_to_leaf` and the Go engine's
     * `resolveCurrentStateToLeaf` both answer `a`.
     *
     * The descent reads the CONFIGURATION rather than recomputing initial or
     * history children: the configuration is what the microstep actually
     * entered, so a descent through it cannot disagree with what happened.
     * Under a `<parallel>` it takes the first region in document order, so
     * the answer does not depend on the order states happened to become
     * active.
     */
    void resolveCurrentStateToLeaf() {
        // A region holds one atomic state, so this descends once per level.
        // The bound is a cycle guard, not a depth policy: a parent chain that
        // loops would otherwise hang here rather than report anything.
        constexpr int MAX_DESCENTS = 50;
        const std::vector<State> active = getActiveStates();
        for (int depth = 0; depth < MAX_DESCENTS; ++depth) {
            if (!StatePolicy::isCompoundState(currentState_) && !StatePolicy::isParallelState(currentState_)) {
                return;
            }
            const std::vector<State> children = StatePolicy::getChildStates(currentState_);
            const auto child = std::find_if(children.begin(), children.end(), [&active](State candidate) {
                return std::find(active.begin(), active.end(), candidate) != active.end();
            });
            // A compound state with no active child is not a configuration
            // this can repair, so it is left as it is rather than guessed at.
            if (child == children.end()) {
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

        // docs/adr/0084, D2: the host's call is an arrival, and what other
        // threads posted before it arrived first, so it is applied first, in the
        // order the inbox took it. When that applied anything, the host's event
        // joins the external queue behind it, through the one door every
        // external enqueue takes, and the main event loop takes them all in
        // order, this event's preliminary step and outcome with the rest
        // (`processNextExternalEvent`). When it applied nothing, the event is
        // handled here directly, as it always was.
        const bool finalBefore = isInFinalState();
        if (drainInbox() > 0) {
            raiseExternal(eventWithMeta);
            runMainEventLoop();
            // §scxml-6.4: a parent is told at a top-level final only, and, as
            // below, only when this call is what reached it.
            if (!finalBefore && isInFinalState() && completionCallback_) {
                completionCallback_();
            }
            return;
        }

        // §scxml-D-mainEventLoop: the preliminary step is owed to the event,
        // not to the queue. This door bypasses `processNextExternalEvent`, so
        // it has to run the step itself or an `autoforward` child never sees
        // what the host delivered and a `<finalize>` never runs for a child
        // event a transport relayed in this way.
        const Event event = eventWithMeta.event;
        std::optional<EventWithMetadata> inPlace;
        const Admission admission = applyExternalEventPreamble(eventWithMeta, inPlace);
        if (admission == Admission::Refused) {
            // Nothing is taken in its place, and it is not dropped unseen: it is
            // counted as a discarded external event, the hook every lane shares
            // for an external event that moved nothing (docs/adr/0086, section 4.2).
            recordEventOutcome(event, EventOutcome{});
            return;
        }

        EventOutcome outcome;
        if (admission == Admission::InPlace) {
            // The `error.execution` the door took in the refused delivery's
            // place is answered as one off the internal queue is (docs/adr/0086,
            // section 4.2), and the loop runs after it as after the host's own
            // event, with one spent.
            outcome = answerInPlace(inPlace->event);
            if (outcome.selected) {
                runMainEventLoop(1);
            }
        } else {
            // The host's own event is the first of this invocation's budget: it is
            // handled here, directly, and the loop that follows starts with one spent
            // (ARCHITECTURE.md "External-Event Budget", rule 1).
            outcome = executeTransition(event, [this] { runMainEventLoop(1); });
            // §scxml-3.1.2: this entry point takes an event straight from the host,
            // so it is the same fact `processNextExternalEvent` records — unlike
            // the Rust and Go engines, whose `process_event` enqueues and lets the
            // dequeue do it. Recorded in both places so a host calling
            // `processEvent` reads the same count on every backend.
            recordEventOutcome(event, outcome);
        }
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
    // §scxml-D-enterStates: the top-level <final> the run ended in; set by
    // MicrostepHost::enterState, cleared when a run starts.
    std::optional<State> terminalState_;
    // §scxml-D-exitInterpreter has run for this run: the configuration is
    // empty. Cleared when a run starts.
    bool interpreterExited_ = false;
    // §scxml-D-enterStates, late binding: the states whose `<data>` this run
    // has already bound — `s.isFirstEntry` is false for exactly these. The
    // rule lives here, once, and the generated entry code asks it
    // (claimLateBindingFirstEntry); cleared when a run starts.
    std::set<State> lateBoundStates_;
    SCE::Core::EventQueueManager<EventWithMetadata>
        internalQueue_;  // §scxml-3.13: Internal event queue (high priority)
    SCE::Core::EventQueueManager<EventWithMetadata> externalQueue_;  // §scxml-3.13: External event queue (low priority)
    // docs/adr/0084, D2 as amended at step 10: the directory of the tree's sessions,
    // shared as the tree's invokers are (`shareTreeWith`), and this session's entry
    // in it, made when a run starts (`enterTree`) and removed when the run ends
    // (`closeInbox`) or the engine goes. Declared before the inbox, so an engine
    // that goes closes its inbox before it leaves the directory, as an ended run does.
    std::shared_ptr<::SCE::Core::TreeSessions> treeSessions_ = std::make_shared<::SCE::Core::TreeSessions>();
    ::SCE::Core::TreeSessions::Membership treeMembership_;
    // docs/adr/0084: what other threads posted, which the owner applies at each
    // macrostep boundary (`drainInbox`) and before it puts an event of its own on
    // the external queue (`applyHeldBeforeExternalEnqueue`), and the record of who
    // the owner is. Open from construction, so what is posted before the first run
    // is kept for it.
    ::SCE::Core::EngineInboxOwner<InboxInput> inbox_;
    // A drain of the inbox is applying what it held; the owner's only.
    bool drainingInbox_ = false;
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
    // §scxml-B-2-8-1: deliveries whose payload announced structure and that the
    // datamodel could not read as one, and the most recent of them.
    // `hasUndecodablePayload_` is separate for the same reason as above.
    uint32_t undecodablePayloads_ = 0;
    Event lastUndecodablePayload_{};
    bool hasUndecodablePayload_ = false;
    // §scxml-3.13: external events this machine never looked at, because it
    // had already stopped, and the most recent of them. `hasUnseenEvent_` is
    // separate for the same reason as above.
    uint32_t unseenExternalEvents_ = 0;
    Event lastUnseenEvent_{};
    bool hasUnseenEvent_ = false;
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
    // The budget of one invocation of the main event loop, the invocations this
    // engine handed back with an external event still queued, and the event at
    // the head of the queue when that last happened — see
    // `truncatedEventChains()`. The Interpreter holds the same class, so the
    // number and the arithmetic are written once.
    Core::ExternalEventBudget<Event> externalEventBudget_;
    std::function<void()> completionCallback_;                 // §scxml-6.4: Callback for done.invoke
    std::function<void(const HttpSendRequest &)> onHttpSend_;  // §scxml-C-2: BasicHTTP callback
    // docs/adr/0089: told each transition the engine takes, by its row of the
    // emitted transition table; empty until a host sets one.
    TransitionObserver transitionObserver_;
    // §scxml-6.2.5: handlers for the Event I/O Processor types this host
    // declared it serves, keyed by the `type` a `<send>` names. A map rather
    // than one callback because the identifier set is open and a host may
    // serve several — which is also why the request carries its own type.
    std::map<std::string, ::SCE::HostSendHandler> hostProcessors_;
    // §scxml-6.4.1: handlers that RUN each `<invoke type>` this host declared.
    // A second map rather than a second use of the one above, because
    // delivering an event is not the same capability as running a process with
    // a lifecycle — see `core/HostProcessor.h`'s invoker half.
    //
    // docs/adr/0045: shared by every session of the tree this machine is the root
    // of. The set of invocable types is the processor's, not a session's
    // (§scxml-6.4.1), so a parent hands this registry to each child it starts or
    // resumes (`shareTreeWith`), and an invoker registered afterwards
    // serves the child too, since it is the tree's own registry the child holds.
    std::shared_ptr<::SCE::TreeInvokers> invokers_ = std::make_shared<::SCE::TreeInvokers>();

    // Every host-run invocation of this session that was started and has neither
    // ended nor been cancelled, and every one a restore re-armed and has yet to
    // start (docs/adr/0045), with the token of each start. Held by the engine rather
    // than by the generated machine because "is this one still running?" is the
    // question every ending and the exit chain (`cancelHostInvoke`) have to answer,
    // and answering it in each backend's template would be the same bookkeeping
    // written once per language. The bookkeeping itself is the shared layer's
    // (docs/adr/0084, D5): this engine hands it its scheduler to arm and drop
    // deadlines, and raises what each operation leaves it to raise.
    Core::HostInvokeLedger hostInvokes_;
    // docs/adr/0045: the children a restore resumed, whose routers are asked of the
    // factory once the host has registered it (`oweChildMesh`), each as the call that
    // asks.
    std::vector<std::function<void()>> owedChildMeshes_;
    // docs/adr/0045: what a save reads and a restore sets is this engine's own state.
    friend struct ::SCE::Saved::EngineAccess;
    // How many ids this machine has generated for a `<send idlocation>`: the
    // number the last one carried. Counted per machine, not per process, so a
    // machine's ids do not depend on what else ran beside it. See
    // nextAutoSendId().
    uint64_t autoSendSeq_ = 0;
    // See refusedHostInvokeCompletions().
    uint64_t refusedHostInvokeCompletions_ = 0;
    MeshSendCallback onMeshSend_;      // SCE Mesh: cross-machine <send> callback
    MeshInvokeCallback onMeshInvoke_;  // SCE Mesh §mesh-9.5: <invoke type="sce:mesh-rpc"> entry hook
    MeshCancelCallback onMeshCancel_;  // SCE Mesh §mesh-9.5: mesh-rpc exit / cancel hook
    ScxmlInvokeStartCallback
        onScxmlInvokeStart_;  // SCE Mesh §mesh-9.6.2 wire-14: <invoke type="scxml" src="#peer"> entry hook
    ScxmlInvokeParentEventCallback
        onScxmlInvokeParentEvent_;                   // SCE Mesh §mesh-9.6.2 wire-17: autoforward outbound hook
    ScxmlInvokeCancelCallback onScxmlInvokeCancel_;  // SCE Mesh §mesh-9.6.2 wire-19: remote invoke cancel hook
    // The factory of the routers of this machine's children (SCE_MESH.md,
    // Child Sessions in Mesh): registered by the host on a machine it starts,
    // and on a child by the router its parent gave it, which answers for the
    // child's children.
    ChildMeshRouters childMeshRouters_;
    // The router this machine was given as a child session; empty for a
    // machine its host started, for a child given none, and once detached.
    std::shared_ptr<IChildMeshRouter> installedMesh_;
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
    std::string currentEventInvokeId_;  // SCE Mesh §mesh-9.5: invokeId of event being processed
    // §scxml-5.10.1: whether the event being processed was taken off the
    // external queue. Set where the event is dequeued — the one moment the
    // queue is known — and read when `_event` is bound, so interleaved
    // internal and external events each carry their own `_event.type`.
    bool currentEventFromExternalQueue_ = false;
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

    // ── The owner and the inbox (ARCHITECTURE.md "Engine Threading") ─────

    /**
     * @brief Refuse @p operation, an owner-thread operation, on a thread that
     *        does not own this engine (debug builds)
     *
     * Checked before the operation changes anything, so a refused call leaves
     * the engine as it was. A release build does not check: the rule holds
     * there by the host keeping it, as it does for any other C++ object
     * confined to one thread.
     */
    void requireOwner(const char *operation) const {
#ifndef NDEBUG
        if (!inbox_.state().isOwnerHere()) {
            refuseOffOwner(operation);
        }
#else
        (void)operation;
#endif
    }

    /**
     * @brief The calling thread drives a macrostep of @p operation: it becomes
     *        the owner when the engine has none, and is refused when another
     *        thread owns it (debug builds)
     */
    void driveOnThisThread(const char *operation) {
#ifndef NDEBUG
        if (!inbox_.state().claimOwner()) {
            refuseOffOwner(operation);
        }
#else
        (void)operation;
#endif
    }

    [[noreturn]] static void refuseOffOwner(const char *operation) {
        throw std::logic_error(std::string(operation) +
                               " is an owner-thread operation of this engine and was called from a thread that does "
                               "not own it (ARCHITECTURE.md \"Engine Threading\"): call it from the thread driving "
                               "the engine, or post the input from this thread through the engine's inbox "
                               "(`inbox().post`)");
    }

    /**
     * @brief The owner's half of the inbox (docs/adr/0084)
     *
     * Applies, on the owner, what other threads posted, in the order the inbox
     * took it, each by the owner-thread operation of the same name
     * (`applyInboxInput`). Called at each macrostep boundary, and before the
     * owner puts an event of its own on the external queue
     * (`applyHeldBeforeExternalEnqueue`). What the inbox held when this was
     * called, so posters that never stop cannot hold the owner here; what they
     * post meanwhile is applied at the next drain.
     *
     * One drain at a time. Applying a held input reaches the external queue
     * through `raiseExternal`, which asks for the held inputs first, and so can
     * a handler the input runs (a host-run invocation's ending raising
     * `done.invoke`, a cancel handler raising an event of its own). A drain
     * started there would put the inputs posted after the one being applied on
     * the queue ahead of it. The drain already applying them takes them in
     * order, so a drain inside a drain applies nothing, and an applied event is
     * pushed as it is.
     *
     * @return How many inputs were applied: none inside a drain.
     */
    std::size_t drainInbox() {
        if (drainingInbox_) {
            return 0;
        }
        drainingInbox_ = true;

        struct Draining {
            bool &flag;

            ~Draining() {
                flag = false;
            }
        } draining{drainingInbox_};

        return inbox_.state().drain([this](InboxInput &&input) { applyInboxInput(std::move(input)); });
    }

    /**
     * @brief Before the owner puts an event of its own on the external queue,
     *        apply what the inbox holds (docs/adr/0084, D2)
     *
     * The external queue takes events in the order they arrived. What another
     * thread posted before this event has arrived first, so it joins the queue
     * first, and not only at the macrostep's boundary: W3C test 201's BasicHTTP
     * send waits for the listener, whose thread posts `event1`, and the
     * `timeout` the document sends itself next must come second. Only while a
     * run is going, as the boundary drains only then: before a run is entered,
     * what was posted is kept for that run, as `inbox()` says.
     *
     * Not asked for an internal raise (`raise`): the internal queue is the
     * running macrostep's own, and arrival order is the external queue's.
     */
    void applyHeldBeforeExternalEnqueue() {
        if (isRunning_) {
            drainInbox();
        }
    }

    /**
     * @brief Close the inbox and apply what it still holds
     *
     * A run that ended, or was stopped, takes no more posts. What was posted
     * before is applied as a direct call would be now: an event joins the
     * external queue of a machine that will not dequeue it, and an ending finds
     * its invocation over.
     */
    void closeInbox() {
        inbox_.state().close();
        drainInbox();
        // docs/adr/0084, D2 as amended at step 10: the session has ended, so it
        // leaves the tree's directory, after what its inbox held was applied; a post
        // that names it from now on is refused where it is posted.
        treeMembership_.leave();
    }

    /**
     * @brief Enter this session in the tree's directory as its run starts, before
     *        the run's first macrostep (docs/adr/0084, D2 as amended at step 10)
     *
     * From then on an ending posted for this session, through this engine's
     * handle or any other session's of the tree, is posted to this inbox. A
     * machine that holds no session id is named by no post and enters nothing.
     */
    void enterTree() {
        const std::string sessionId = ownSessionId();
        if (sessionId.empty()) {
            treeMembership_.leave();
            return;
        }
        treeMembership_ =
            treeSessions_->enter(sessionId, [own = inbox_.handle()](const ::SCE::HostInvokeEnding &ending) {
                return own.post(InboxHostInvokeEnding{std::string(), ending});
            });
    }

    /// Apply one posted @p input by the operation a direct call on the owner makes.
    void applyInboxInput(InboxInput &&input) {
        if (const auto *external = std::get_if<InboxExternalEvent>(&input)) {
            raiseExternal(external->event);
            return;
        }
        if (const auto *named = std::get_if<InboxExternalEventByName>(&input)) {
            raiseExternal(named->event);
            return;
        }
        if (const auto *ended = std::get_if<InboxHostInvokeEnding>(&input)) {
            // An ending in this inbox is this session's own: one that names a
            // session was handed to that session's inbox when it was posted
            // (`Inbox::post`).
            const ::SCE::HostInvokeEnding &ending = ended->ending;
            if (ending.done) {
                (void)completeHostInvoke(ending.processorType, ending.invokeId, ending.token, ending.data,
                                         ending.origin);
            } else {
                (void)failHostInvoke(ending.processorType, ending.invokeId, ending.token, ending.data, ending.origin);
            }
            return;
        }
        applyChildSessionInput(input);
    }

    /// Apply a posted input of a child session on another device (SCE_MESH.md
    /// §9.6) with the policy's member of the same name. A machine that runs no
    /// such child has nothing to apply it to.
    void applyChildSessionInput(InboxInput &input) {
        if constexpr (SCE::Core::HasRemoteChildSessionsTrait<StatePolicy, StaticExecutionEngine>::value) {
            if (const auto *started = std::get_if<InboxChildSessionStarted>(&input)) {
                policy_.onInvokeStarted(started->invokeId, started->childSessionId, started->endpoint);
            } else if (const auto *child = std::get_if<InboxChildSessionEvent>(&input)) {
                if (policy_.hasActiveChildSession(child->childSessionId)) {
                    raiseExternal(child->event);
                }
            } else if (const auto *done = std::get_if<InboxChildSessionDone>(&input)) {
                policy_.onInvokeDone(done->invokeId, done->doneData, *this);
            } else if (const auto *lost = std::get_if<InboxPeerLost>(&input)) {
                policy_.failScxmlRemoteInvokesForPeer(lost->peer, *this);
            }
        } else {
            (void)input;
        }
    }

protected:
    StatePolicy policy_;  // Policy instance for stateful policies

public:
    /**
     * @brief This engine's inbox: the handle a thread that does not own the
     *        engine posts external events and host-run invocations' endings
     *        through (docs/adr/0084)
     *
     * Thread-safe, as is every copy of it; everything else of the engine that
     * changes its state is an owner-thread operation (ARCHITECTURE.md "Engine
     * Threading"). What is posted is applied by the owner, in the order the inbox
     * took it, at its next macrostep boundary, before the external queue is
     * consulted, or sooner, before it puts an event of its own on the external
     * queue, which takes events in the order they arrived. `post` answers
     * whether the inbox took the input: it refuses one once the run has ended
     * or been stopped (until the next `initialize` or `enterAt`), and when the
     * heap refuses the memory for it. It is unbounded otherwise (docs/adr/0084,
     * D2), and never refuses a post for contention. A post taken before the
     * first run is applied by that run. An ending that names a session of the
     * tree is posted to that session's inbox instead, and refused when the tree
     * holds no such session (`Inbox::post`). `wakeWith` installs what each post
     * to any session of the tree calls, so a host that drives the engine itself
     * knows when to drive it.
     */
    Inbox inbox() const {
        return Inbox(inbox_.handle(), treeSessions_);
    }

    /// How many posts this engine's inbox refused: closed, or out of memory. A
    /// post that returned `false` to a caller who could not act on it is still
    /// counted here, by the inbox it was posted to; one that named a session the
    /// tree does not hold reached no inbox and is not.
    uint64_t refusedInboxPosts() const {
        return inbox_.state().refusedPosts();
    }

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
        // §scxml-3.13: Enqueue event with metadata. The inbox is not applied
        // first, as it is before an external enqueue (docs/adr/0084, D2): the
        // internal queue is the running macrostep's own, and arrival order is
        // the external queue's.
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
     * @param origin The reply address the processor that delivers the event
     *        names, in its own terms (docs/adr/0090); none for a host that
     *        names no party, and then `_event.origin` and `_event.origintype`
     *        both read empty
     */
    void raiseExternal(Event event, const std::string &eventData = "",
                       std::optional<Core::EventOrigin> origin = std::nullopt, const std::string &target = "") {
        requireOwner("raiseExternal");
        // §scxml-3.13: Enqueue event with metadata (origin, data, sendid, type, target)
        // Delegates to the full-metadata overload so that SCE Mesh target
        // routing and §scxml-6.4 autoforward both see the event. Prior
        // to this delegation the simple (datamodel="null") codepath dropped
        // the target attribute, which meant mesh-declared targets silently
        // hit the external queue instead of the mesh callback.
        EventWithMetadata meta(event, eventData, std::move(origin), "", "external");
        meta.target = target;
        raiseExternal(meta);
    }

    /**
     * @brief Raise external event by name (§scxml-6.4)
     *
     * Used for autoforward - converts event name string to Event enum and raises.
     * The name is delivered as the event `resolveEventByName` answers (§scxml-3.12.1):
     * the document's own name for it, or the longest of the document's names that is
     * a token prefix of it. A name no transition could match is silently ignored
     * (child may not have that event).
     *
     * @param eventName Event name string (e.g., "childToParent")
     * @param eventData Optional event data
     * @param origin The reply address the host names for the event, in its own
     *        terms (docs/adr/0090); none when it names no party
     */
    void raiseExternal(const std::string &eventName, const std::string &eventData = "",
                       std::optional<Core::EventOrigin> origin = std::nullopt) {
        requireOwner("raiseExternal");
        if (auto event = resolveEventByName(eventName)) {
            EventWithMetadata meta(*event, eventData, std::move(origin), "", "external");
            meta.name = arrivalNameOf(*event, eventName);
            raiseExternal(meta);
        } else {
            SCE_LOG_DEBUG("AOT raiseExternal: Event '{}' matches no event of the document, ignoring", eventName);
        }
    }

    /**
     * @brief The event an event that arrives by name is delivered as (§scxml-3.12.1)
     *
     * `Policy::getEventFromName` is the exact table (O(n) if-chain) the generated code
     * writes; the rule that an arriving name is the longest of the document's names that
     * is a token prefix of it is written once, in `SCE::Core::resolveArrivingEventName`.
     */
    [[nodiscard]] std::optional<Event> resolveEventByName(const std::string &name) const {
        return ::SCE::Core::resolveArrivingEventName(
            name, [this](const std::string &exact) { return policy_.getEventFromName(exact); });
    }

    /**
     * @brief What `EventWithMetadata::name` is for an event that arrived as `name` and was
     *        delivered as `event` (§scxml-5.10)
     *
     * Empty when `event` is the document's own name for it, so an event the document
     * raises or sends carries no copy of a name its member already holds; `name` when
     * `resolveEventByName` had to cut it back to a shorter one.
     */
    [[nodiscard]] std::string arrivalNameOf(Event event, const std::string &name) const {
        return name == policy_.getEventName(event) ? std::string() : name;
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
        requireOwner("raiseExternal");
        auto event = resolveEventByName(forwarded.name);
        if (!event) {
            SCE_LOG_DEBUG("AOT raiseExternal: Forwarded event '{}' matches no event of the document, ignoring",
                          forwarded.name);
            return;
        }
        // `target` stays default-constructed: the forwarded copy is delivered
        // to this machine, never re-routed to the original event's target.
        EventWithMetadata meta(*event, forwarded.data, forwarded.origin, forwarded.sendId, forwarded.type,
                               forwarded.invokeId);
        meta.sender = forwarded.sender;
        meta.name = arrivalNameOf(*event, forwarded.name);
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
        requireOwner("raiseExternal");
        // A Mesh session endpoint — the origin a remote child's event or its
        // parent's carries — is a target, so a reply sent to it reaches that
        // session (docs/adr/0090). Resolved by the one that wrote it: the
        // policy for the children this machine runs, the child's transport
        // adapter (its Mesh send) for the parent that invoked it.
        if (::SCE::Common::isMeshSessionEndpoint(eventWithMetadata.target)) {
            if (!deliverToMeshEndpoint(eventWithMetadata)) {
                // §scxml-C-1: a target the processor cannot dispatch to is
                // error.communication on the sender's internal queue.
                if (auto error = policy_.getEventFromName("error.communication")) {
                    raise(EventWithMetadata(*error,
                                            "<send target='" + eventWithMetadata.target +
                                                "'> names no Mesh session this machine reaches",
                                            std::nullopt, eventWithMetadata.sendId));
                }
            }
            return;
        }
        // SCE Mesh: route cross-machine targets through the mesh callback before
        // they reach the external queue. Matches the §scxml-C-2 BasicHTTP
        // split — HTTP targets are dispatched via performHttpSend(), mesh
        // targets via performMeshSend(). A machine whose host wired no mesh
        // transport leaves onMeshSend_ unset; that is a configuration fault of
        // the host and raises error.execution, as on every other backend
        // (SCE_MESH.md, Errors) — it is not a send to the sender's own queue,
        // which is a peer's event answered by the sender. No W3C test names a
        // `#<peer>` target (`#_internal`, `#_parent` and the session and invoke
        // addresses all start `#_`), so no conformance depends on the old route.
        if (::SCE::SendHelper::isMeshTarget(eventWithMetadata.target)) {
            if (!onMeshSend_) {
                refuseMeshSend(eventWithMetadata.target, "no Mesh router is registered for this peer");
                return;
            }
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
        if constexpr (SCE::Core::HasChildSessionDeliveryTrait<StatePolicy>::value) {
            const std::string childSession =
                SCE::IOProcessorHelper::sessionIdFromScxmlLocation(eventWithMetadata.target);
            if (!childSession.empty()) {
                SCE::Common::ForwardedEvent addressed{policy_.getEventName(eventWithMetadata.event),
                                                      eventWithMetadata.data,
                                                      eventWithMetadata.origin,
                                                      eventWithMetadata.sender,
                                                      eventWithMetadata.sendId,
                                                      eventWithMetadata.type,
                                                      eventWithMetadata.invokeId};
                if (policy_.deliverToChildSession(childSession, addressed)) {
                    return;  // delivered to the addressed session
                }
            }
        }

        // docs/adr/0084, D2: every event this machine puts on its own external
        // queue comes through here (a `<send>` to itself, a host-run
        // invocation's `done.invoke`, a delayed send released by `tick`, a
        // child's report, the host's own call), so this is where the external
        // queue keeps arrival order: what other threads posted before this event
        // is applied first, and this event joins the queue behind it. Applying a
        // posted event comes back here inside the drain, and is pushed as it is.
        applyHeldBeforeExternalEnqueue();

        SCE_LOG_DEBUG("AOT raiseExternal: Enqueuing external event with metadata (event={}, invokeId='{}')",
                      static_cast<int>(eventWithMetadata.event), eventWithMetadata.invokeId);

        externalQueue_.raise(eventWithMetadata);
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
     * @param senderSessionId The sending session — this session for a `<send>` —
     *        whose published location `_event.origin` is when the event arrives
     *        (§scxml-C-1); empty for a machine that holds no session id
     * @return The sendid assigned to this event
     */
    std::string scheduleEvent(Event event, std::chrono::milliseconds delay, const std::string &sendId = "",
                              const std::string &eventData = "", const std::string &senderSessionId = "") {
        uint64_t fireTimeMs = schedNowMs() + static_cast<uint64_t>(delay.count());
        return scheduler_.scheduleEventAt(event, fireTimeMs, sendId, eventData, senderSessionId);
    }

    /**
     * @brief The event a session's `<send>` puts on an external queue
     *
     * The SCXML processor writes the pair of its own type and the sender's
     * published location (`scxmlOriginOf`) where it builds the event, and sets the
     * sender's id beside it (docs/adr/0090, section 2). A machine that holds no
     * session id sends an event with no origin.
     */
    EventWithMetadata sentByASession(Event event, const std::string &eventData, const std::string &sendId,
                                     const std::string &senderSessionId) const {
        EventWithMetadata meta(event, eventData, ::SCE::IOProcessorHelper::scxmlOriginOf(senderSessionId), sendId,
                               "external");
        meta.sender = senderSessionId;
        return meta;
    }

    /**
     * @brief `sentByASession` for a delivery that leaves this machine by name
     *        (a child, a session, the parent)
     */
    ::SCE::Common::ForwardedEvent forwardedByASession(const std::string &eventName, const std::string &eventData,
                                                      const std::string &sendId,
                                                      const std::string &senderSessionId) const {
        return ::SCE::Common::ForwardedEvent{
            eventName,  eventData, ::SCE::IOProcessorHelper::scxmlOriginOf(senderSessionId), senderSessionId, sendId,
            "external", ""};
    }

    /**
     * @brief Deliver a delayed event whose wait is over (§scxml-6.2)
     *
     * With the metadata the immediate send stamps — its sendid (§scxml-5.10.1)
     * and its origin (§scxml-C-1) — which the simple `raiseExternal` overload
     * would leave blank: a delayed send is the same event, only later.
     */
    void raiseScheduled(Event event, const std::string &eventData, const std::string &sendId,
                        const std::string &senderSessionId) {
        raiseExternal(sentByASession(event, eventData, sendId, senderSessionId));
    }

    /**
     * @brief Schedule a delayed send whose target is not this session's own
     *        external queue (§scxml-6.2)
     *
     * A `delay` postpones a send; it does not change where the send goes. The
     * generated send site resolves the target when the send is made and hands
     * it over here, so the delivery at the end of the delay reaches it — in
     * the same queue as every other delayed send, which is what lets one
     * `<cancel sendid>` (§scxml-6.3) reach it.
     *
     * @param event This machine's spelling of the event, used for the
     *        internal queue; a child or a parent resolves `route.eventName`
     * @return The sendid assigned to this event
     */
    std::string scheduleRoutedEvent(Event event, std::chrono::milliseconds delay, const std::string &sendId,
                                    const std::string &eventData, const std::string &senderSessionId,
                                    ::SCE::ScheduledRoute route) {
        uint64_t fireTimeMs = schedNowMs() + static_cast<uint64_t>(delay.count());
        return scheduler_.scheduleRoutedAt(event, fireTimeMs, sendId, eventData, senderSessionId, std::move(route));
    }

    /**
     * @brief Deliver a delayed send whose wait is over to the target it named
     *
     * §scxml-C-1: "If the SCXML Processor cannot dispatch the event to the
     * target, it MUST place the error error.communication on the internal
     * event queue of the session that attempted to send the event." For a
     * delayed send the dispatch is this delivery, so an invocation that has
     * ended in the meantime is reported here, not when the send was made.
     */
    void deliverScheduled(Event event, const std::string &eventData, const std::string &sendId,
                          const std::string &senderSessionId, const ::SCE::ScheduledRoute &route) {
        if (!deliverRouted(event, eventData, sendId, senderSessionId, route)) {
            if (auto communicationError = policy_.getEventFromName("error.communication")) {
                raise(EventWithMetadata(*communicationError,
                                        "<send> named a target that was no longer there when its delay elapsed",
                                        std::nullopt, sendId));
            }
        }
    }

    /**
     * @brief Deliver a send to the target a route names, now
     *
     * The one delivery both a send made at once and a delayed send whose wait
     * is over go through (§scxml-6.2.4: a delay postpones a send, it does not
     * change where it goes). It answers whether the target was there to take
     * the event; the caller reports a `false` as error.communication
     * (§scxml-C-1).
     */
    bool deliverRouted(Event event, const std::string &eventData, const std::string &sendId,
                       const std::string &senderSessionId, const ::SCE::ScheduledRoute &route) {
        using Kind = ::SCE::ScheduledRoute::Kind;
        if (route.kind == Kind::InternalQueue) {
            // An event on the internal queue is an internal event, whose origin
            // the processor leaves blank — no pair, and no sender.
            EventWithMetadata meta(event, eventData, std::nullopt, sendId);
            meta.name = arrivalNameOf(event, route.eventName);
            raise(std::move(meta));
            return true;
        }
        if (route.kind == Kind::ExternalQueue) {
            // §scxml-5.10: this session's own external queue, for an event sent
            // under a name that is not its member's own (a computed one).
            EventWithMetadata meta = sentByASession(event, eventData, sendId, senderSessionId);
            meta.name = arrivalNameOf(event, route.eventName);
            raiseExternal(std::move(meta));
            return true;
        }
        if (route.kind == Kind::MeshPeer) {
            // SCE_MESH.md §mesh-9.5: the delivery an immediate `#<peer>` send
            // makes — raiseExternal hands a peer-named event to the Mesh router
            // — with the invokeid captured when the send was made (§mesh-10.7).
            // The router reports a peer it cannot reach itself (§mesh-16.7).
            EventWithMetadata meta = sentByASession(event, eventData, sendId, senderSessionId);
            meta.name = arrivalNameOf(event, route.eventName);
            meta.invokeId = route.meshInvokeId;
            meta.target = route.address;
            raiseExternal(std::move(meta));
            return true;
        }
        const SCE::Common::ForwardedEvent forwarded =
            forwardedByASession(route.eventName, eventData, sendId, senderSessionId);
        if (route.kind == Kind::Invocation) {
            if constexpr (SCE::Core::HasInvocationDeliveryTrait<StatePolicy>::value) {
                return policy_.deliverToInvocation(route.address, forwarded);
            } else {
                return false;
            }
        }
        if (route.kind == Kind::Session) {
            if constexpr (SCE::Core::HasChildSessionDeliveryTrait<StatePolicy>::value) {
                return policy_.deliverToChildSession(route.address, forwarded);
            } else {
                return false;
            }
        }
        if constexpr (SCE::Core::HasParentDeliveryTrait<StatePolicy>::value) {
            return policy_.deliverToParent(forwarded);
        } else {
            return false;
        }
    }

    /**
     * @brief Deliver an event whose target is a Mesh session endpoint
     *        (`common/MeshEndpoint.h`), now
     *
     * The endpoint is resolved by the side that wrote it, by its text: a child
     * session this machine runs on another device answers at the endpoint its
     * transport reported when it started (`deliverToMeshEndpoint` of the
     * policy), and the session that invoked this one answers at the endpoint it
     * sent with its request, which this machine's transport adapter holds and
     * takes through the Mesh send (`ChildSessionAdapter`). The event keeps the
     * origin and the sender it was sent with.
     *
     * @return Whether a session answered at the endpoint and took the event
     */
    bool deliverToMeshEndpoint(const EventWithMetadata &eventWithMetadata) {
        const std::string eventName = eventWithMetadata.name.empty()
                                          ? std::string(policy_.getEventName(eventWithMetadata.event))
                                          : eventWithMetadata.name;
        if constexpr (SCE::Core::HasMeshEndpointDeliveryTrait<StatePolicy, StaticExecutionEngine>::value) {
            const ::SCE::Common::ForwardedEvent forwarded{eventName,
                                                          eventWithMetadata.data,
                                                          eventWithMetadata.origin,
                                                          eventWithMetadata.sender,
                                                          eventWithMetadata.sendId,
                                                          "external",
                                                          eventWithMetadata.invokeId};
            if (policy_.deliverToMeshEndpoint(eventWithMetadata.target, forwarded, *this)) {
                return true;
            }
        }
        return performMeshSend(eventWithMetadata.target, eventName, eventWithMetadata.data, eventWithMetadata.sendId,
                               eventWithMetadata.invokeId);
    }

    /// What became of a `<send>` handed to [sendToTarget].
    enum class TargetSendOutcome {
        Sent,         ///< delivered, or scheduled to be when its delay elapses
        Unsupported,  ///< the value is not a target: the caller raises error.execution
        Unreachable,  ///< nothing the value names is there: the caller raises error.communication
    };

    /**
     * @brief Send to a target value read at run time (§scxml-6.2.4, §scxml-C-1)
     *
     * A `targetexpr` is a target: whatever value it yields is routed as the same
     * value written in `target` is — `SendHelper::classifyTarget` reads it, for
     * the Interpreter as for this engine — sent at once or, with a delay, from
     * the scheduler through [deliverRouted] when it comes due. A session the
     * value names that is not there now is reported now; an invocation or a
     * session that ends while a delayed send waits is reported when it comes
     * due.
     *
     * @param event This machine's spelling of the event, for its own queues
     * @param eventName The event's name, for another session to resolve
     * @param ownSessionId The sending session, which a session target may name
     * @param senderSessionId The session the delivered event names as its sender,
     *        whose published location is its origin (§scxml-C-1)
     */
    TargetSendOutcome sendToTarget(Event event, const std::string &eventName, const std::string &target,
                                   const std::string &ownSessionId, std::chrono::milliseconds delay,
                                   const std::string &sendId, const std::string &eventData,
                                   const std::string &senderSessionId) {
        using Kind = ::SCE::SendHelper::SendTarget::Kind;
        using RouteKind = ::SCE::ScheduledRoute::Kind;
        const bool delayed = delay.count() > 0;
        // §scxml-C-1: a Mesh session endpoint — the origin of an event a remote
        // child or its parent sent — is a target (docs/adr/0090). Its session is
        // looked for when the event is delivered, at once or when the delay is
        // over, through the same delivery a `#<peer>` send's deadline takes.
        if (::SCE::Common::isMeshSessionEndpoint(target)) {
            if (delayed) {
                scheduleRoutedEvent(
                    event, delay, sendId, eventData, senderSessionId,
                    ::SCE::ScheduledRoute{RouteKind::MeshPeer, eventName, target, currentEventInvokeId_});
                return TargetSendOutcome::Sent;
            }
            EventWithMetadata meta = sentByASession(event, eventData, sendId, senderSessionId);
            meta.name = arrivalNameOf(event, eventName);
            meta.target = target;
            return deliverToMeshEndpoint(meta) ? TargetSendOutcome::Sent : TargetSendOutcome::Unreachable;
        }
        const auto resolved = ::SCE::SendHelper::classifyTarget(target, ownSessionId);
        std::optional<::SCE::ScheduledRoute> route;
        switch (resolved.kind) {
        case Kind::Unsupported:
        case Kind::Uri:
        case Kind::Http:
            // A generated machine registers no target schemes: a URI of any
            // scheme but its own locations — an http(s) URL included, which
            // only a BasicHTTP send reaches — is one the SCXML Event I/O
            // Processor cannot address here.
            return TargetSendOutcome::Unsupported;
        case Kind::Unreachable:
            return TargetSendOutcome::Unreachable;
        case Kind::SelfExternal:
            // §scxml-5.10: `event` is the member the sender's name resolved to in
            // THIS machine's document, so its own queues are told the name it was
            // sent under when the member is not called that.
            if (delayed) {
                if (arrivalNameOf(event, eventName).empty()) {
                    scheduleEvent(event, delay, sendId, eventData, senderSessionId);
                } else {
                    scheduleRoutedEvent(event, delay, sendId, eventData, senderSessionId,
                                        ::SCE::ScheduledRoute{RouteKind::ExternalQueue, eventName});
                }
            } else {
                EventWithMetadata meta = sentByASession(event, eventData, sendId, senderSessionId);
                meta.name = arrivalNameOf(event, eventName);
                raiseExternal(std::move(meta));
            }
            return TargetSendOutcome::Sent;
        case Kind::Mesh:
            if (delayed) {
                // SCE_MESH.md §mesh-9.5 + §mesh-10.7: the peer and the invokeid
                // of the event being processed now are fixed at the send; the
                // deadline delivers them through deliverRouted.
                route = ::SCE::ScheduledRoute{RouteKind::MeshPeer, "", target, currentEventInvokeId_};
                break;
            }
            // A Mesh peer: the external path hands it to the Mesh transport.
            // (Built in place — `reader_names` reads `Type name(...)` in this
            // header as a member the generated machine carries.)
            raiseExternal(EventWithMetadata(event, eventData, ::SCE::IOProcessorHelper::scxmlOriginOf(senderSessionId),
                                            sendId, "external", "", target));
            return TargetSendOutcome::Sent;
        case Kind::Internal:
            route = ::SCE::ScheduledRoute{RouteKind::InternalQueue, eventName, ""};
            break;
        case Kind::Parent:
            // §scxml-C-1: a session nothing invoked has no parent to address,
            // delayed or not.
            // `hasParentSession` is emitted beside `deliverToParent`, so the
            // trait that finds one finds both.
            if constexpr (SCE::Core::HasParentDeliveryTrait<StatePolicy>::value) {
                if (!policy_.hasParentSession()) {
                    return TargetSendOutcome::Unreachable;
                }
            } else {
                return TargetSendOutcome::Unreachable;
            }
            route = ::SCE::ScheduledRoute{RouteKind::Parent, eventName, ""};
            break;
        case Kind::Invocation:
            // §scxml-6.4 + §scxml-C-1: an invocation that is not running now is
            // not there to address, delayed or not.
            // `isInvocationRunning` is emitted beside `deliverToInvocation`, so
            // the trait that finds one finds both.
            if constexpr (SCE::Core::HasInvocationDeliveryTrait<StatePolicy>::value) {
                if (!policy_.isInvocationRunning(resolved.id)) {
                    return TargetSendOutcome::Unreachable;
                }
            } else {
                return TargetSendOutcome::Unreachable;
            }
            route = ::SCE::ScheduledRoute{RouteKind::Invocation, eventName, resolved.id};
            break;
        case Kind::Session:
            // §scxml-C-1: the location the session that invoked this one
            // publishes is a target, as `#_parent` is — the address a reply to
            // an event the parent sent goes to (docs/adr/0090). `isParentSession`
            // is emitted beside `deliverToParent`, so the trait that finds one
            // finds both.
            if constexpr (SCE::Core::HasParentDeliveryTrait<StatePolicy>::value) {
                if (policy_.isParentSession(resolved.id)) {
                    route = ::SCE::ScheduledRoute{RouteKind::Parent, eventName, ""};
                    break;
                }
            }
            // §scxml-C-1: a session that does not exist is reported when the
            // send is made, delayed or not.
            // `isChildSessionRunning` is emitted beside `deliverToChildSession`,
            // so the trait that finds one finds both.
            if constexpr (SCE::Core::HasChildSessionDeliveryTrait<StatePolicy>::value) {
                if (!policy_.isChildSessionRunning(resolved.id)) {
                    return TargetSendOutcome::Unreachable;
                }
            } else {
                return TargetSendOutcome::Unreachable;
            }
            route = ::SCE::ScheduledRoute{RouteKind::Session, eventName, resolved.id};
            break;
        }
        if (delayed) {
            scheduleRoutedEvent(event, delay, sendId, eventData, senderSessionId, *route);
            return TargetSendOutcome::Sent;
        }
        return deliverRouted(event, eventData, sendId, senderSessionId, *route) ? TargetSendOutcome::Sent
                                                                                : TargetSendOutcome::Unreachable;
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
     *
     * §scxml-6.4: the session's OWN scheduler is not the whole answer. This
     * machine's `tick()` advances every running child, so a child's deadline is
     * a deadline of this machine's clock, and the answer is the nearest of the
     * two. Counting only the parent's own made a host that walked time by it
     * step over a timer its child had armed — or, for a parent that arms
     * nothing itself, be told there was nothing to wait for at all.
     */
    std::optional<std::chrono::milliseconds> timeUntilNextScheduled() const {
        // docs/adr/0045: a restore that re-armed a host-run invocation owes the machine a
        // tick before any clock does. The invocation starts at the top of the first
        // macrostep and the deadline it kept is armed there, so until that tick the
        // scheduler holds nothing to wait for, and a host asleep on this answer would
        // never drive the machine at all. That is now.
        if (hostInvokes_.hasRestored()) {
            return std::chrono::milliseconds::zero();
        }
        std::optional<std::chrono::milliseconds> soonest;
        if (auto next = scheduler_.nextFireTime(); next.has_value()) {
            uint64_t now = schedNowMs();
            soonest = (*next <= now) ? std::chrono::milliseconds::zero()
                                     : std::chrono::milliseconds(static_cast<int64_t>(*next - now));
        }
        if constexpr (SCE::Core::HasNextChildDeadlineTrait<StatePolicy>::value) {
            if (auto child = policy_.nextChildDeadline(); child.has_value() && (!soonest || *child < *soonest)) {
                soonest = child;
            }
        }
        return soonest;
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
        // Before the clock moves: a refused call must leave the engine's time as it was.
        driveOnThisThread("advanceTimeMs");
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
     * @brief Record which §scxml-B-2-8-1 rung the payload just bound got.
     *
     * Called by generated code immediately after it binds `_event`, because
     * that is the only moment the rung is known. Four of the five readings are
     * the ladder working and are recorded by being ignored; the fifth is the
     * one a host is wrong about.
     */
    void notePayloadReading(Event event, ::SCE::PayloadReading reading) {
        if (reading == ::SCE::PayloadReading::Undecodable) {
            ++undecodablePayloads_;
            lastUndecodablePayload_ = event;
            hasUndecodablePayload_ = true;
        }
    }

    /**
     * @brief Events delivered with a payload the datamodel could not read
     *
     * The clause requires the fallback: content the processor cannot interpret
     * becomes a space-normalized string. What it does not require — and what
     * nothing here used to provide — is any way for the host that SENT the
     * payload to learn its fields have stopped existing. The document reads
     * `_event.data.field`, gets nothing, assigns nothing, and the run
     * continues; measured 2026-08-22 on three independent Lua implementations,
     * a payload in Lua's own table syntax silently emptied every variable the
     * receiving transition assigned, including the one that primes the next
     * session.
     *
     * Counts only the reading a host can act on. Prose delivered as text is
     * the ladder working (W3C test 562) and is not counted, because a
     * diagnostic that fires when nothing is wrong is one nobody reads.
     */
    uint32_t undecodablePayloads() const {
        return undecodablePayloads_;
    }

    /**
     * @brief The most recent event `undecodablePayloads()` counted
     *
     * `std::nullopt` while that count is zero, for the same reason as
     * `lastDiscardedEvent()`: the generated `Event` enum's zero value is a
     * real event and cannot stand in for "none".
     */
    std::optional<Event> lastUndecodablePayload() const {
        if (!hasUndecodablePayload_) {
            return std::nullopt;
        }
        return lastUndecodablePayload_;
    }

    /**
     * @brief Record one external event this machine will never look at
     *
     * W3C SCXML Appendix D's main event loop: the loop that would have dequeued it has ended,
     * so the event is not "pending" — it is over.
     */
    void noteUnseenEvent(Event event) {
        ++unseenExternalEvents_;
        lastUnseenEvent_ = event;
        hasUnseenEvent_ = true;
    }

    /**
     * @brief Empty the external queue into the count above
     *
     * Drained rather than left in place so each event is counted exactly once:
     * a host that keeps driving a halted machine would otherwise re-count the
     * same queue on every call, and a count that grows while nothing arrives
     * is a count nobody can use.
     */
    void recordUnseenExternalEvents() {
        while (externalQueue_.hasEvents()) {
            noteUnseenEvent(externalQueue_.pop().event);
        }
    }

    /**
     * @brief External events the host sent that this machine never looked at
     *
     * W3C SCXML Appendix D's main event loop exits when the machine reaches a top-level final
     * state, and §scxml-3.13 is explicit that the interpreter is then done.
     * Refusing the event is therefore correct — and, exactly as with
     * `discardedExternalEvents()` and `undecodablePayloads()`, being unable to
     * SAY it happened is not part of the clause.
     *
     * This is the count that separates the third explanation from the other
     * two. A host that sent an event and saw nothing move has three
     * candidates:
     *
     * | what happened | which count moves |
     * | --- | --- |
     * | dequeued, no transition matched | `discardedExternalEvents()` |
     * | dequeued, a transition matched but its guard was false | neither |
     * | never dequeued — the machine had stopped | this one |
     *
     * Measured 2026-08-22: a consumer reported a guarded transition that
     * "never fires", and four rewrites of the guard later the guard was still
     * the suspect. Driving the same document here fired it on the first try,
     * at that consumer's own pinned revision — so the difference was never the
     * guard, and nothing in this engine could have said so.
     */
    uint32_t unseenExternalEvents() const {
        return unseenExternalEvents_;
    }

    /**
     * @brief Which event the count above last recorded
     *
     * `std::nullopt` while that count is zero — the event enum's zero value is
     * a real event and cannot stand in for "none".
     */
    std::optional<Event> lastUnseenEvent() const {
        if (!hasUnseenEvent_) {
            return std::nullopt;
        }
        return lastUnseenEvent_;
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
     * @brief How many invocations of the main event loop this engine handed
     *        back with an external event still queued, because the invocation
     *        had already taken `maxExternalEventsPerCall()` of them
     *        (ARCHITECTURE.md "External-Event Budget")
     *
     * `truncatedMacrosteps()` counts a macrostep that does not end; this counts
     * the other way a call fails to return: every macrostep ends, and each one
     * queues the event that starts the next (`<send event="again"/>`, no target,
     * answered by a transition that sends it again). The specification bounds
     * neither, and an engine that ran either to the letter would never return,
     * with `getCurrentState()` answering, `isRunning()` true and no sign that
     * anything went wrong. This is that sign.
     *
     * Counted when the loop still had work after the budget. A call that takes
     * exactly the budget and empties the queue counts zero: a long backlog is
     * ordinary, an endless one is not. The events left queued stay queued, so a
     * host that calls again gets another budget and the machine goes on from
     * where it was; what the host learns is that this call did not reach quiet.
     *
     * A cut of the same-instant bound on the scheduler counts here too (see
     * `tick()`).
     */
    uint32_t truncatedEventChains() const {
        return externalEventBudget_.cuts();
    }

    /**
     * @brief The event at the head of the queue when this engine last handed a
     *        call back that way
     *
     * `std::nullopt` while `truncatedEventChains()` is zero — and for a cut of
     * the scheduler whose due entry delivers no event of this machine's. The
     * generated `Event` enum's zero value is a real event and cannot stand in for
     * "none". In a chain that sends itself an event it is that event: the count
     * says a call did not reach quiet and this says what it was still taking.
     */
    std::optional<Event> lastTruncatedEvent() const {
        return externalEventBudget_.lastCutHead();
    }

    /// @brief The most external events one invocation of the main event loop may take
    uint32_t maxExternalEventsPerCall() const {
        return externalEventBudget_.limit();
    }

    /**
     * @brief Let a host choose the budget of one invocation
     *
     * A host that hands the machine a backlog it means the machine to work
     * through in one call knows its size, and this engine does not. Settable at
     * any time. The default is `Core::DEFAULT_MAX_EXTERNAL_EVENTS_PER_CALL`.
     *
     * Refused below one — returns false and changes nothing: a budget that takes
     * no event is a machine that cannot run, not a stricter one, since every call
     * would hand control back with the queue untouched and say it had been cut.
     */
    bool setMaxExternalEventsPerCall(uint32_t limit) {
        return externalEventBudget_.setLimit(limit);
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
     * @brief Appendix D's exitInterpreter (§scxml-D-exitInterpreter)
     *
     * Every state still in the configuration is exited in exitOrder — onexit,
     * then its invocations cancelled, then out of the configuration, the same
     * per-state exit a microstep performs — so the configuration ends EMPTY
     * and the machine stops running. Called once a run has entered a
     * top-level `<final>` (the only state left by then is that final), and by
     * `stop()` for a run the host ends early.
     *
     * The run's terminal state is kept: it is recorded when the final is
     * entered, and it is how a host learns where the run ended once the
     * configuration is empty. The datamodel is kept too, so a host can still
     * read what the run left behind.
     *
     * Idempotent: a second call finds nothing to exit.
     */
    void exitInterpreter() {
        if (interpreterExited_) {
            return;
        }
        const std::vector<State> configuration = getActiveStates();
        std::vector<State> statesToExit = configuration;
        // §scxml-D-exitInterpreter: `configuration.toList().sort(exitOrder)`
        // — descendants before ancestors, reverse document order otherwise,
        // which together are exactly reverse document order.
        std::sort(statesToExit.begin(), statesToExit.end(), [](const State &a, const State &b) {
            return StatePolicy::getDocumentOrder(a) > StatePolicy::getDocumentOrder(b);
        });
        MicrostepHost host{*this};
        for (const State &s : statesToExit) {
            host.exitState(s, configuration);
        }
        interpreterExited_ = true;
        isRunning_ = false;
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
    void runMainEventLoop(uint32_t alreadyTaken = 0) {
        // External events this invocation has taken off the queue, against
        // `externalEventBudget_`. A local, because the budget is one
        // invocation's: a cut leaves the queue as it is and the next invocation
        // gets a budget of its own (ARCHITECTURE.md "External-Event Budget"). The
        // host's own event is the first of its invocation, so a door that handles
        // it directly (`processEvent`) says so and starts with one spent.
        uint32_t taken = alreadyTaken;
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
                // §scxml-D-mainEventLoop ends here, and whatever the host put
                // on the external queue ends with it. That is the clause: a
                // machine that reached a top-level final state has exited the
                // interpreter. Saying nothing about it is not the clause —
                // see `unseenExternalEvents()`. The inbox closes with a run that
                // ended, and what was posted before is applied first, so an
                // event posted to it is counted with the rest. (A stopped run
                // closed it in `stop()`; one never started keeps what was posted
                // for its first boundary.)
                if (isInFinalState()) {
                    closeInbox();
                }
                recordUnseenExternalEvents();
                // §scxml-D-exitInterpreter: the loop is over because the run
                // entered a top-level final, so every state left is exited —
                // before any completion callback tells a parent (§scxml-6.4:
                // done.invoke comes after the child's onexit).
                if (isInFinalState()) {
                    exitInterpreter();
                }
                break;
            }

            // §scxml-6.4: invokes for states entered during this macrostep.
            if constexpr (SCE::Core::HasInvokeSupportTrait<StatePolicy, StaticExecutionEngine>::value) {
                policy_.executePendingInvokes(*this);
            }
            // docs/adr/0045: the host-run invocations a restore re-armed start as the
            // ones the document just entered do, before the external queue is touched.
            startRestoredHostInvokes();

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

            // docs/adr/0084: the macrostep is complete, and this is the point
            // before the external queue is consulted. What other threads posted
            // since the last drain is applied here, in the order the inbox took
            // it. What was posted before an event this macrostep put on the
            // external queue was applied then (`applyHeldBeforeExternalEnqueue`);
            // what is left came after, and joins the queue behind it.
            drainInbox();
            // Applying an input can raise an internal event (an event whose
            // target is refused raises error.execution), which comes before
            // anything external, as one raised in the macrostep would.
            if (!macrostepTruncated_ && internalQueue_.hasEvents()) {
                continue;
            }

            if (!externalQueue_.hasEvents()) {
                break;
            }
            // Every macrostep this loop runs ends, so `MAX_MACROSTEP_MICROSTEPS`
            // bounds none of this: a machine that sends itself an external event
            // on every one never empties the queue. Past the budget the queue is
            // left exactly as it is and the call hands back, counted.
            if (!externalEventBudget_.admitNext(
                    taken, [this] { return std::optional<Event>(externalQueue_.front().event); })) {
                break;
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
                currentEventInvokeId_ = eventWithMeta.invokeId;
                currentEventFromExternalQueue_ = false;
                // The door binds it, or the `error.execution` taken in its place;
                // a delivery refused with nothing to take in its place took no
                // microstep and moved nothing (docs/adr/0086, section 4.2).
                std::optional<EventWithMetadata> inPlace;
                if (bindTypedPayload(eventWithMeta, inPlace) == Admission::Refused) {
                    return true;
                }
                const Event event = inPlace ? inPlace->event : eventWithMeta.event;

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
     *
     * The door comes first (docs/adr/0086, section 4.2): a delivery whose data
     * does not fit its type is refused whole, and what the door made of it is
     * answered. The `error.execution` it takes in the delivery's place, left
     * in `inPlace`, is the engine's own error, so no `<finalize>` or
     * autoforward runs for it.
     */
    Admission applyExternalEventPreamble(const EventWithMetadata &eventWithMeta,
                                         std::optional<EventWithMetadata> &inPlace) {
        currentEventInvokeId_ = eventWithMeta.invokeId;
        currentEventFromExternalQueue_ = true;
        const Admission admission = bindTypedPayload(eventWithMeta, inPlace);
        if (admission != Admission::Admitted) {
            return admission;
        }

        // §scxml-6.5: Execute finalize BEFORE processing child events
        if constexpr (SCE::Core::HasFinalizeTrait<StatePolicy, EventWithMetadata,
                                                  StaticExecutionEngine<StatePolicy>>::value) {
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
        if constexpr (SCE::Core::HasAutoforwardTrait<StatePolicy, StaticExecutionEngine>::value) {
            ::SCE::Common::ForwardedEvent forwarded;
            // §scxml-6.4 + §scxml-5.10: the copy is exact, so it is forwarded under the
            // name the event arrived under, not the member it was matched through.
            forwarded.name = eventWithMeta.name.empty() ? std::string(policy_.getEventName(eventWithMeta.event))
                                                        : eventWithMeta.name;
            forwarded.data = eventWithMeta.data;
            // docs/adr/0090, section 2: autoforward copies the origin and the
            // sender of the event it forwards, unchanged.
            forwarded.origin = eventWithMeta.origin;
            forwarded.sender = eventWithMeta.sender;
            forwarded.sendId = eventWithMeta.sendId;
            forwarded.type = eventWithMeta.type;
            forwarded.invokeId = eventWithMeta.invokeId;
            SCE_LOG_DEBUG("AOT applyExternalEventPreamble: Autoforwarding external event '{}'", forwarded.name);
            policy_.forwardToAutoforwardChildren(forwarded, *this);
        }
        return Admission::Admitted;
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
        // §scxml-6.4: a host-run invocation's `done.invoke` counts only when
        // completeHostInvoke() confirmed the invocation was still running and
        // stamped its token. Without the stamp it may be a cancelled run's late
        // reply, which the processor ignores — so it is refused here, counted,
        // and never reaches transition selection. Returns true because an
        // event was taken.
        if constexpr (SCE::Core::HasHostInvokeIdsTrait<StatePolicy>::value) {
            if (!eventWithMeta.hostInvokeToken.has_value() &&
                ::SCE::isHostInvokeCompletion(policy_.getEventName(eventWithMeta.event), eventWithMeta.invokeId,
                                              StatePolicy::HOST_INVOKE_IDS, std::size(StatePolicy::HOST_INVOKE_IDS))) {
                ++refusedHostInvokeCompletions_;
                return true;
            }
        }
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
            const Event event = eventWithMeta.event;
            std::optional<EventWithMetadata> inPlace;
            switch (applyExternalEventPreamble(eventWithMeta, inPlace)) {
            case Admission::Refused:
                // Counted as a discarded external event (docs/adr/0086,
                // section 4.2): nothing was taken in its place.
                recordEventOutcome(event, EventOutcome{});
                break;
            case Admission::InPlace:
                (void)answerInPlace(inPlace->event);
                break;
            case Admission::Admitted:
                recordEventOutcome(event, executeTransition(event));
                break;
            }
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

        while (true) {
            SCE_LOG_DEBUG("AOT checkEventlessTransitions: Microstep {}, currentState={}", macrostepMicrostepsTaken_,
                          static_cast<int>(currentState_));

            const std::vector<TransitionInfo> enabled = selectTransitions(Event());
            if (enabled.empty()) {
                // §scxml-3.13: nothing enabled by NULL — the configuration is
                // stable. Internal events are the caller's to drain
                // (runMainEventLoop or step).
                break;
            }
            if (macrostepMicrostepsTaken_ == MAX_MACROSTEP_MICROSTEPS) {
                // The chain is still going one microstep past the budget, so
                // this is the case the specification's Principles and
                // Constraints call a macrostep that does not terminate. The
                // microstep is refused rather than taken — selection moves
                // nothing, so refusing it is exact — and the refusal is
                // published: the configuration left behind is not a stable one
                // and only this counter says so. The machine keeps running; the
                // specification allows the document, so declining to run it
                // forever is a fact to report, not grounds to kill a session
                // whose other states still work.
                recordTruncatedMacrostep(currentState_);
                SCE_LOG_ERROR("StaticExecutionEngine: macrostep still going after {} microsteps; stopped taking them",
                              MAX_MACROSTEP_MICROSTEPS);
                break;
            }
            ++macrostepMicrostepsTaken_;
            // A targetless transition takes a microstep too: it runs its
            // content in place, and the chain it opens is walked like any other.
            microstep(enabled);
        }

        // §scxml-3.13: a macrostep that entered a top-level final state halts
        // the machine.
        if (isInFinalState()) {
            SCE_LOG_INFO("AOT checkEventlessTransitions: Reached top-level final state {}, halting processing (W3C "
                         "SCXML 3.13)",
                         static_cast<int>(currentState_));
            stop();
        }
    }

public:
    StaticExecutionEngine() : currentState_(StatePolicy::initialState()) {}

    /**
     * @brief Whether a host running this machine as a ROOT — started by the
     *        host, not by an `<invoke>` — should refuse to start it, and why.
     *
     * §scxml-6.2.4: `#_parent` names the session that invoked this one, and a
     * root has none. `initialize()` still runs such a machine — each such send
     * then raises error.communication, which is the default. This answers for
     * a host that would rather not start it at all, from the policy's
     * `NEEDS_PARENT`, the manifest's `needs_parent`.
     */
    static std::optional<::SCE::Common::RootStartRefusal> rootStartRefusal() {
        if constexpr (::SCE::Core::NeedsParentTrait<StatePolicy>::value) {
            return ::SCE::Common::RootStartRefusal::NeedsParent;
        } else {
            return std::nullopt;
        }
    }

    /**
     * @brief `initialize()`, for a host that refuses to start a machine needing
     *        a parent as a root (`rootStartRefusal()`).
     *
     * A refusal starts nothing: no datamodel is declared and no state is
     * entered, so the engine is as its constructor left it.
     */
    std::optional<::SCE::Common::RootStartRefusal> initializeAsRoot() {
        if (auto refusal = rootStartRefusal()) {
            return refusal;
        }
        initialize();
        return std::nullopt;
    }

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
        // ARCHITECTURE.md "Engine Threading": a run may be entered on any thread,
        // and its owner is the thread that drives it from here (the first
        // `step`, `tick` or `processEvent`). A run that ended or was stopped
        // closed the inbox; this one takes posts again, and what was posted
        // before the first run is applied by it, at its first boundary or
        // before the first event it puts on the external queue. The session enters
        // its tree's directory before its first macrostep, so an ending posted for
        // it from now on reaches its inbox.
        requireOwner("initialize");
        inbox_.state().releaseOwner();
        inbox_.state().open();
        enterTree();

        // §scxml-3.13: entering the initial configuration is one turn, and the
        // `<onentry>` handlers it runs arm their `<send delay>`s against one
        // instant — see `beginTurn()` for what reading the clock per `<send>`
        // did to two of them.
        TurnGuard turn(*this);

        isRunning_ = true;
        terminalState_.reset();
        interpreterExited_ = false;
        lateBoundStates_.clear();

        // §scxml-5.3: Initialize datamodel before any state entry
        // This ensures error.execution events are raised immediately if initialization fails
        if constexpr (SCE::Core::HasDataModelInitTrait<StatePolicy, StaticExecutionEngine>::value) {
            policy_.initializeDataModel(*this);
        }

        // §scxml-D-interpret: enterStates([doc.initialTransition]) — the
        // document's own initial transition, whose source is the <scxml>
        // element and whose domain is therefore the whole document. A target
        // set, a deep initial, an initial naming a <history>: the entry
        // procedures answer all of them, so nothing here resolves a leaf
        // first.
        enterStates({SCE::Core::EntryTransition<State, History>{std::nullopt, StatePolicy::getDocumentInitialTargets(),
                                                                /*isInternal=*/false}});

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
            // The final's onexit has already run: `runMainEventLoop` ran
            // exitInterpreter when the run ended, on this path as on every
            // other.
            completionCallback_();
        }
    }

    /**
     * @brief Enter a configuration this machine was already in, WITHOUT running
     *        `<onentry>`.
     *
     * `initialize()` is the other door and the contrast is the whole point: it
     * enters the document's initial configuration and runs the entry actions of
     * every state on the way in. This one enters a configuration the caller
     * names and runs none of them. A host that has persisted where a machine
     * was, and is bringing it back in a new process, wants the second:
     * §scxml-3.8 entry actions are "executed when the state is entered", and
     * re-executing them is a replay of what the earlier run already did — an
     * `<onentry><send>` would post its event a second time, to a peer that
     * already received it.
     *
     * ## What it takes, and why two arguments
     *
     * Exactly what the two readers on this engine publish: `getActiveStates()`
     * and `getCurrentState()`. Handing back both is not redundancy — for a
     * machine with `<parallel>` states the configuration does not determine the
     * current state. `currentState_` is the leaf the engine descended to, so
     * which region it sits in is a fact about the transition history rather
     * than about the configuration, and a set alone cannot recover it.
     *
     * ## What it refuses
     *
     * Every set that is not a configuration of THIS document — see
     * `SCE::Core::validateConfiguration` for the rules and
     * `SCE::Core::ConfigurationRejection` for what each refusal names.
     * Validation runs before any mutation, so a refused call leaves the engine
     * exactly as it was; entering "near" the requested configuration is the one
     * outcome this door must never produce, because a host has no way to detect
     * it afterwards.
     *
     * ## What it does not do
     *
     * - No `<onentry>`, and no `<onexit>`: no state is entered or left.
     * - No macrostep. `initialize()` settles the machine before returning; this
     *   does not, because the configuration handed in was already a settled
     *   one — running the loop here could take an eventless transition the
     *   earlier run had no reason to take, and fire the `<send>`s on the way.
     *   The host drives the machine on with `step()` or `tick()` as it
     *   otherwise would.
     * - No datamodel restore. §scxml-5.3 declaration still runs, so the
     *   variables exist with their document defaults and a host can then put
     *   its saved values back through `IScriptEngine` — the engine does not
     *   persist datamodel state and does not pretend to.
     *
     * The C++ twin of the Rust runtime's `Engine::enter_at`.
     *
     * @return `ConfigurationRejection::None` on success; the rule that was
     *         broken otherwise, with the engine untouched.
     */
    [[nodiscard]] SCE::Core::ConfigurationRejection enterAt(const std::vector<State> &configuration, State current) {
        // Before anything is touched: a rejection must not half-enter.
        const auto verdict = SCE::Core::validateConfiguration<StatePolicy>(configuration, current);
        if (verdict != SCE::Core::ConfigurationRejection::None) {
            SCE_LOG_ERROR("AOT enterAt: refused — {}", SCE::Core::configurationRejectionText(verdict));
            return verdict;
        }

        // The run resumed here is entered as `initialize` enters one: its owner is
        // the thread that drives it next, its inbox takes posts, and it enters its
        // tree's directory under the id it carries on under (a restore has bound it).
        requireOwner("enterAt");
        inbox_.state().releaseOwner();
        inbox_.state().open();
        enterTree();

        // §scxml-5.3: the datamodel is declared before anything can read it.
        // This is not a state entry action — `<datamodel>` holds `<data>`, not
        // executable content — so it runs here for the same reason it runs in
        // `initialize()`: a `cond` or an `assign` evaluated after this call
        // would otherwise reference variables that were never declared.
        if constexpr (SCE::Core::HasDataModelInitTrait<StatePolicy, StaticExecutionEngine>::value) {
            policy_.initializeDataModel(*this);
        }

        currentState_ = current;
        // A configuration restored AT a top-level <final> is a run that has
        // already ended there; any other is still running.
        if (StatePolicy::isFinalState(current) && !StatePolicy::getParent(current).has_value()) {
            terminalState_ = current;
        } else {
            terminalState_.reset();
        }
        interpreterExited_ = false;
        // The restored states were entered by the run being resumed, so their
        // late-bound `<data>` is that run's — the host restores it, and a later
        // re-entry must not bind it again (§scxml-D-enterStates isFirstEntry).
        lateBoundStates_ = std::set<State>(configuration.begin(), configuration.end());

        // §scxml-3.4: a machine that keeps its own active set is handed it
        // back. The condition is the one the generator emits `setActiveStates`
        // under, so a policy reached here has the method.
        if constexpr (StatePolicy::HAS_PARALLEL_STATES) {
            if constexpr (SCE::Core::HasActiveStatesTrait<StatePolicy>::value) {
                policy_.setActiveStates(configuration);
            }
        }

        isRunning_ = true;
        return SCE::Core::ConfigurationRejection::None;
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
        // host takes to run them — see `beginTurn()`. Asked of the owner first,
        // so a refused call has not touched the turn either.
        driveOnThisThread("step");
        TurnGuard turn(*this);

        // A machine with delayed sends hands `step()` a queue it cannot reach:
        // `runMainEventLoop()` never consults the scheduler, so the event is
        // neither delivered nor refused. Say it once — the host is driving with
        // the wrong call, and every later macrostep would repeat the same word.
        if constexpr (::SCE::Core::NeedsEventSchedulerTrait<StatePolicy>::value) {
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
        driveOnThisThread("processEvent");
        if (!isRunning_) {
            // Refused rather than queued, so the drain in `runMainEventLoop`
            // never sees it — which is why the count is taken here as well as
            // there. See `unseenExternalEvents()`.
            noteUnseenEvent(event);
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
     * External events with metadata carry their sender's session id for invoke
     * finalize. Used when events come from child sessions via invoke.
     *
     * @param event External event to process
     * @param metadata Event metadata (origin, sender, etc.)
     */
    void processEvent(Event event, const SCE::Core::EventMetadata &metadata) {
        driveOnThisThread("processEvent");
        if (!isRunning_) {
            // Same refusal as the overload above, same reason it is counted.
            noteUnseenEvent(event);
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
        carried.origin = metadata.origin;
        carried.sender = metadata.sender;
        carried.sendId = metadata.sendId;
        carried.type = metadata.type;
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
            if constexpr (SCE::Core::HasActiveStatesTrait<StatePolicy>::value) {
                return policy_.getActiveStates();
            }
        }

        // §scxml-D-exitInterpreter: a run that has ended has no configuration.
        // (A machine that keeps its own active set emptied it state by state
        // above; this one derives the set from `currentState_`, which still
        // names the final, so the answer has to be given here.)
        if (interpreterExited_) {
            return {};
        }

        // §scxml-3.11: For non-parallel, use shared HistoryHelper for full active hierarchy (Zero Duplication
        // Principle) Returns [currentState, parent, grandparent, ...] for proper history recording
        return ::SCE::Core::HistoryHelper::getActiveHierarchy(currentState_,
                                                              [](State s) { return StatePolicy::getParent(s); });
    }

    /**
     * @brief Check whether this session has ended (§scxml-3.7 / §scxml-6.4)
     *
     * True only once the run has entered a `<final>` that has no parent,
     * i.e. whose parent is the `<scxml>` element — the same answer as
     * `terminalState().has_value()`. §scxml-D-enterStates sets
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
     * @return true once the run has entered a top-level `<final>`
     */
    bool isInFinalState() const {
        return terminalState_.has_value();
    }

    /**
     * @brief Whether the event being processed came off the external queue
     *        (§scxml-5.10.1 `_event.type`)
     *
     * Decided where the event was dequeued, so an internal event raised while
     * an external one waits — or the reverse — is typed by the queue it was
     * taken from, not by whatever was enqueued last.
     */
    bool isCurrentEventExternal() const {
        return currentEventFromExternalQueue_;
    }

    /**
     * @brief The top-level `<final>` the run ended in (§scxml-D-enterStates),
     *        or `std::nullopt` while it is still running or when it was never
     *        started.
     *
     * The one answer to "where did this run end". It is recorded when the
     * final is entered, so it does not depend on what the configuration holds
     * afterwards — Appendix D's exitInterpreter leaves the configuration
     * empty.
     */
    std::optional<State> terminalState() const {
        return terminalState_;
    }

    /**
     * @brief Appendix D's `s.isFirstEntry`, taken (§scxml-D-enterStates)
     *
     * True exactly once per run for @p s — the entry on which a late-binding
     * document binds that state's `<data>` — and false on every entry after
     * it. The generated entry code asks this rather than keeping its own flag,
     * so the rule is written once for every machine.
     */
    bool claimLateBindingFirstEntry(State s) {
        return lateBoundStates_.insert(s).second;
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
     * @brief Stop state machine execution (§scxml-D-exitInterpreter)
     *
     * A host ending a run early ends it the way the appendix ends one: every
     * active state's onexit runs and its invocations are cancelled, innermost
     * first, and the configuration ends empty. The run has no terminal state —
     * it was stopped, not completed. A machine that already ended, or never
     * started, has nothing to exit.
     *
     * The inbox closes with the run, after what it holds is applied as a
     * direct call would be now, and the engine has no owner until a thread
     * drives the next run (ARCHITECTURE.md "Engine Threading").
     */
    void stop() {
        requireOwner("stop");
        if (isRunning_) {
            exitInterpreter();
        }
        isRunning_ = false;
        closeInbox();
        inbox_.state().releaseOwner();
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
        driveOnThisThread("pumpScheduledEvents");
        TurnGuard turn(*this);
        std::string eventData;
        std::string sendId;
        std::string senderSessionId;
        Event event;
        while (scheduler_.popReadyEvent(schedNowMs(), event, eventData, sendId, senderSessionId)) {
            raiseScheduled(event, eventData, sendId, senderSessionId);
        }
    }

    /**
     * @brief Tick scheduler and process ready internal events (§scxml-6.2)
     *
     * This method polls the event scheduler and processes any ready scheduled events.
     * Should be called periodically in a polling loop to allow delayed sends to fire
     * at the correct time. An owner-thread operation, and the one where what was
     * posted to `inbox()` since the last call is applied, at the macrostep
     * boundaries it reaches (ARCHITECTURE.md "Engine Threading").
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
        driveOnThisThread("tick");
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
            std::string sendId;
            std::string senderSessionId;
            Event event;
            std::shared_ptr<const ::SCE::HostSendRequest> hostSend;
            std::shared_ptr<const ::SCE::HostInvokeDeadline> deadline;
            std::shared_ptr<const ::SCE::ScheduledRoute> route;
            // A scheduler that can deliver an entry due at the instant it is
            // popping is a drain that refills itself: an entry a handler re-arms
            // at that same reading (a `delay` of zero, which reaches this
            // scheduler here, or a `delayexpr` that evaluates to zero) is due now,
            // so this loop pops it in the same call, and each pass takes ONE
            // event, so the budget on the external drain never trips. The pops of
            // entries due AT the reading are bounded by the same budget, reset
            // when the reading advances (ARCHITECTURE.md "External-Event
            // Budget", rule 5).
            //
            // Entries due AT the reading, not every entry popped at it: the
            // reading is latched for the whole tick, so a host that jumps the
            // clock a long way pops every entry that came due on the way at ONE
            // reading, and those are due at earlier instants. A handler arms
            // relative to the latched reading, so what it arms is due at the
            // reading itself or after it, and only the first kind can be popped
            // by this tick. A clock the host moved is bounded by how far it moved
            // and is not cut. The counter is this call's own, so a host that ticks
            // again at the same reading gets a budget of its own, as it does for
            // the external drain.
            uint64_t reading = UINT64_MAX;
            uint32_t poppedDueAtReading = 0;
            while (true) {
                const uint64_t now = schedNowMs();
                if (const auto head = scheduler_.peekReady(now); head && head->fireTime == now) {
                    if (reading != now) {
                        reading = now;
                        poppedDueAtReading = 0;
                    }
                    if (!externalEventBudget_.admitNext(poppedDueAtReading, [&head] {
                            return head->ownEvent ? std::optional<Event>(*head->ownEvent) : std::nullopt;
                        })) {
                        break;
                    }
                }
                if (!scheduler_.popReadyAct(now, event, eventData, sendId, senderSessionId, hostSend, deadline,
                                            route)) {
                    break;
                }
                if (hostSend) {
                    // §scxml-6.2.4: the wait is over, so now the act happens.
                    performDeferredHostSend(*hostSend);
                    hostSend.reset();
                } else if (deadline) {
                    expireHostInvoke(*deadline);
                    deadline.reset();
                } else if (route) {
                    // §scxml-6.2: to the target the send named when it was made.
                    deliverScheduled(event, eventData, sendId, senderSessionId, *route);
                    route.reset();
                } else {
                    raiseScheduled(event, eventData, sendId, senderSessionId);
                }
                // The macrostep this act drives may `<cancel>` a later one,
                // so the queue is re-consulted after it rather than before.
                runMainEventLoop();
                if (!isRunning_ || isInFinalState()) {
                    break;
                }
            }
        }

        // §scxml-6.4: Tick child state machines to process their events
        // Children need to run independently during parent's event loop
        if constexpr (SCE::Core::HasChildTickTrait<StatePolicy, StaticExecutionEngine>::value) {
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
     * @brief Tell @p observer each transition this engine takes from now on,
     *        by the row of its emitted transition table (docs/adr/0089)
     *
     * The observer is told after the selection and before the microstep's
     * first exit, each transition of the optimal enabled set once and in
     * selection order, so a run's set of pairs is the set of rows it took: the
     * table's coverage, measured on the engine that ran. It reads and is asked
     * nothing, so a machine with one set takes the same transitions as one
     * without. It runs on the thread that drives the engine, inside the
     * macrostep, and must not call back into the engine; an exception it
     * throws is not caught. Set it before `initialize()` to be told the
     * transitions the first macrostep takes.
     *
     * @param observer Told each transition taken (an empty function to stop)
     */
    void setTransitionObserver(TransitionObserver observer) {
        transitionObserver_ = std::move(observer);
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
     * @brief Dispatch a BasicHTTP send once its delay has elapsed (§scxml-C-2,
     *        §scxml-6.2.4)
     *
     * A delay is a property of the send and not of the processor it named, so
     * a delayed BasicHTTP send waits in the queue every delayed send does: one
     * deadline order, one `<cancel sendid>` path (§scxml-6.3), one answer from
     * the next-deadline query. Its entry is the host-served shape carrying
     * the BasicHTTP type, and the deadline performs the POST through
     * performHttpSend() — see performDeferredHostSend().
     *
     * @return The sendId used, generated when @p sendId is empty
     */
    std::string scheduleHttpSend(const std::string &target, const std::string &eventName, const std::string &content,
                                 const std::map<std::string, std::vector<std::string>> &params,
                                 std::chrono::milliseconds delay, const std::string &sendId) {
        ::SCE::HostSendRequest request;
        request.processorType = ::SCE::Constants::BASIC_HTTP_EVENT_PROCESSOR_URI;
        request.eventName = eventName;
        request.target = target;
        request.content = content;
        request.params = params;
        request.sendId = sendId;
        return scheduleHostSend(request, delay, sendId);
    }

    /**
     * @brief §scxml-6.2.5: serve `<send type="processorType">` from the host.
     *
     * The build must also have been told about the type
     * (`sce-codegen --host-processor <type>`): codegen decides at compile time
     * whether a site dispatches or refuses, and a registration alone cannot
     * change emitted code. Registering an already-registered type replaces its
     * handler, which is what a host re-wiring one expects.
     *
     * @throws std::invalid_argument when `processorType` is under the `sce:`
     * prefix SCE keeps for its own processors (`SendHelper::isReservedHostType`).
     * C++ Mesh sends reach the generated TransportRouter, not a host processor,
     * so there is no router to register here.
     */
    void registerEventProcessor(const std::string &processorType, ::SCE::HostSendHandler handler) {
        ::SCE::SendHelper::refuseReservedHostType("registerEventProcessor", processorType);
        hostProcessors_[processorType] = std::move(handler);
    }

    /**
     * @brief Whether a handler is registered for `processorType`.
     *
     * Two things can go wrong with a host-served send — no handler, or a
     * handler that answered nothing — and only the first is an error. The
     * generated site reads this to tell them apart, which is why it is public.
     *
     * `performHostSend`'s own answer now separates them too: `std::nullopt` is
     * "no handler" and an empty list is "ran, reported nothing". The generated
     * guard still asks both, and that redundancy is deliberate — the two are
     * one fact from the document's side, so a future shape that blurs them
     * again has to get past a site that checks each.
     */
    bool hasEventProcessor(const std::string &processorType) const {
        return hostProcessors_.find(processorType) != hostProcessors_.end();
    }

    /**
     * @brief §scxml-6.2: dispatch a host-served `<send>` and raise, in order,
     *        every event the handler says the act produced.
     *
     * With no handler registered the send raises `error.execution` at the
     * generated site, the same outcome an undeclared type produces. That is
     * the point: the document asked for an act, and from its side "no
     * processor implements this type" and "the processor was never wired up"
     * are one fact. Reporting them differently would make a wiring mistake
     * look like a document error, or worse, look like success.
     *
     * The two answers are therefore "no handler" (`std::nullopt`) and "the
     * handler ran and produced these" (a list, possibly empty). An empty list
     * is a success: plenty of acts report nothing, and real work answers later
     * through the host's own loop.
     *
     * §scxml-C-1: a reply from outside the machine arrives on the EXTERNAL
     * queue, like any event the host raises — `raiseExternal` by name, so a
     * reply naming an event this machine does not declare is dropped exactly
     * as any such event is rather than crashing the run. Raised in list order,
     * because a handler reporting two observations is reporting a sequence:
     * see HostSendHandler for the case that decided the shape.
     */
    std::optional<std::vector<::SCE::HostSendResponse>> performHostSend(const ::SCE::HostSendRequest &request) {
        const auto it = hostProcessors_.find(request.processorType);
        if (it == hostProcessors_.end()) {
            return std::nullopt;
        }
        auto replies = it->second(request);
        for (const auto &reply : replies) {
            if (!reply.eventName.empty()) {
                // docs/adr/0090, section 2: the reply carries the pair the host
                // named for it, or none, as on the Interpreter.
                raiseExternal(reply.eventName, reply.eventData, reply.origin);
            }
        }
        return replies;
    }

    /**
     * @brief §scxml-6.4.1: register what RUNS every `<invoke type="<t>">` this
     *        machine executes
     *
     * Separate from registerEventProcessor() because they are separate
     * contracts: a host that can deliver an event is not thereby able to run a
     * process with a lifecycle, and one registry would make declaring either
     * silently claim both.
     *
     * The build's half is the `--host-invoker` declaration that made codegen
     * emit a start here instead of a refusal; a registration alone cannot
     * change emitted code. Registering an already-registered type replaces its
     * handler, which is what a host re-wiring one expects.
     *
     * docs/adr/0045: the invoker serves every session of this machine's tree, the
     * children it starts included, since the set of invocable types is the
     * processor's and not a session's (§scxml-6.4.1); each request names the
     * session that made it (`HostInvokeRequest::sessionId`).
     */
    void registerInvoker(const std::string &processorType, ::SCE::HostInvokeHandler handler) {
        ::SCE::SendHelper::refuseReservedHostType("registerInvoker", processorType);
        invokers_->handlers[processorType] = std::move(handler);
    }

    /**
     * @brief Whether an invoker is registered for `processorType`
     */
    bool hasInvoker(const std::string &processorType) const {
        return invokers_->handlers.find(processorType) != invokers_->handlers.end();
    }

    /**
     * @brief Hand this machine's tree to @p child, a session it is starting or
     *        resuming, before the child runs
     *
     * What every session of the tree shares (`Core::SessionTree`): the invokers a host
     * registers through `registerInvoker`, which serve every session of the tree, one
     * registered afterwards included, since it is the tree's own registry the child
     * holds (docs/adr/0045); the directory of the tree's sessions, which the child
     * enters when its run starts; and the waker the host installed on the root, which a
     * post to the child's inbox calls, since the tree has one owner (docs/adr/0084, D2
     * as amended at step 10).
     */
    template <typename ChildEngine> void shareTreeWith(ChildEngine &child) {
        child.joinTree(::SCE::Core::SessionTree{invokers_, treeSessions_, inbox_.state().waker()});
    }

    /**
     * @brief Be a session of @p tree, and hand it on to every child this machine holds
     *        now
     *
     * A child a restore resumed holds children of its own, resumed with it before its
     * parent could hand it anything. A session whose run has started already moves its
     * entry into the tree's directory.
     */
    void joinTree(const ::SCE::Core::SessionTree &tree) {
        invokers_ = tree.invokers;
        treeSessions_ = tree.sessions;
        inbox_.state().wakeThrough(tree.waker);
        if (isRunning_) {
            enterTree();
        }
        if constexpr (SCE::Core::HasHostInvokerChildrenTrait<StatePolicy, ::SCE::HostInvokeEnding>::value) {
            policy_.joinTreeInChildren(tree);
        }
    }

    /**
     * @brief Deliver @p ending to the session @p sessionId names, if that session is
     *        this machine or one in the tree below it
     *
     * @return `reached` when a session of that id runs here or below, and `delivered`
     *         when it was running the start the ending names and took it. Neither when
     *         the id is no session of this tree, so the parent that asked can try
     *         another child.
     */
    ::SCE::HostInvokeRoute routeHostInvokeEnd(const std::string &sessionId, const ::SCE::HostInvokeEnding &ending) {
        requireOwner("routeHostInvokeEnd");
        if (!sessionId.empty() && sessionId == ownSessionId()) {
            return {true, endHostInvoke(ending.processorType, ending.invokeId, ending.token, !ending.done, ending.data,
                                        ending.origin)};
        }
        if constexpr (SCE::Core::HasHostInvokerChildrenTrait<StatePolicy, ::SCE::HostInvokeEnding>::value) {
            return policy_.routeHostInvokeEnd(sessionId, ending);
        }
        return {};
    }

    /**
     * @brief End a host-run invocation of the session @p sessionId, this one or one
     *        below it at any depth, reached by its id as a send to a child session is
     *        (docs/adr/0045)
     *
     * The session's queue takes `done.invoke` or `error.invoke` as `completeHostInvoke`
     * and `failHostInvoke` raise them; a child takes it when its parent next drives it.
     *
     * An owner-thread operation, made between macrosteps: it reaches the session it
     * names through the children each session holds, and that session raises the
     * ending's event through its own external enqueue. A host whose invocation ends on a
     * thread of its own posts an `InboxHostInvokeEnding` naming the session to `inbox()`
     * instead, which hands it to that session's inbox when it is posted (`Inbox::post`;
     * ARCHITECTURE.md "Engine Threading").
     */
    ::SCE::HostInvokeEndOutcome endHostInvokeIn(const std::string &sessionId, const ::SCE::HostInvokeEnding &ending) {
        requireOwner("endHostInvokeIn");
        const ::SCE::HostInvokeRoute route = routeHostInvokeEnd(sessionId, ending);
        if (!route.reached) {
            return ::SCE::HostInvokeEndOutcome::NoSuchSession;
        }
        return route.delivered ? ::SCE::HostInvokeEndOutcome::Delivered : ::SCE::HostInvokeEndOutcome::NotRunning;
    }

    /**
     * @brief `endHostInvokeIn` for a completion: `done.invoke.<invokeId>` with
     *        @p doneData as `_event.data`, in the session @p sessionId names
     *
     * What a host that serves the whole tree calls with the session id and token the
     * request carried.
     */
    ::SCE::HostInvokeEndOutcome completeHostInvokeIn(const std::string &sessionId, const std::string &processorType,
                                                     const std::string &invokeId, uint64_t token,
                                                     const std::string &doneData = "") {
        requireOwner("completeHostInvokeIn");
        return endHostInvokeIn(sessionId, ::SCE::HostInvokeEnding{/*processorType=*/processorType,
                                                                  /*invokeId=*/invokeId,
                                                                  /*token=*/token,
                                                                  /*done=*/true,
                                                                  /*data=*/doneData,
                                                                  /*origin=*/std::nullopt});
    }

    /**
     * @brief `endHostInvokeIn` for a failure: `error.invoke.<invokeId>` with
     *        @p errorData as `_event.data`
     */
    ::SCE::HostInvokeEndOutcome failHostInvokeIn(const std::string &sessionId, const std::string &processorType,
                                                 const std::string &invokeId, uint64_t token,
                                                 const std::string &errorData = "") {
        requireOwner("failHostInvokeIn");
        return endHostInvokeIn(sessionId, ::SCE::HostInvokeEnding{/*processorType=*/processorType,
                                                                  /*invokeId=*/invokeId,
                                                                  /*token=*/token,
                                                                  /*done=*/false,
                                                                  /*data=*/errorData,
                                                                  /*origin=*/std::nullopt});
    }

    /**
     * @brief §scxml-6.4.1: start a host-run invocation, reporting whether
     *        anything ran
     *
     * `false` means no invoker was registered, which the generated site turns
     * into `error.execution` — an invoke nobody ran is the same fact whether
     * the type was undeclared or the handler was never wired up.
     *
     * A started invocation is RECORDED here, in the ledger (`hostInvokes_`, which
     * says why that bookkeeping is the engine's). The token is assigned and
     * recorded BEFORE the handler runs, so a handler that completes
     * synchronously completes an invocation the engine already knows is running.
     *
     * §scxml-6.4: a completion the host reports NOW takes the same door as one
     * it reports later, completeHostInvoke(), so the two cannot disagree about
     * whether the invocation is over. The engine never synthesises a
     * completion the host did not report.
     *
     * A `_sce_deadline_ms` param is the engine's, not the host's: it is taken
     * out of the request and armed with the start. A value that is not a whole
     * number of milliseconds raises `error.execution` and starts nothing — an
     * argument of the element that cannot be evaluated (§scxml-6.4.1) — and
     * the call answers `true` because the report has been made; `false` is
     * kept for "no invoker ran it", asked first because it is the prior fact.
     */
    bool performHostInvoke(const ::SCE::HostInvokeRequest &request) {
        const auto report =
            hostInvokes_.start(*invokers_, request, ownSessionId(), scheduler_, [this] { return schedNowMs(); });
        if (!report.has_value()) {
            return false;
        }
        raiseHostInvokeReport(*report);
        return true;
    }

    /**
     * @brief §scxml-6.4: a host-run invocation finished; raise its
     *        `done.invoke.<invokeId>` with `doneData` as `_event.data`
     *
     * `token` is the one the invocation's start request carried. The
     * completion is accepted only while that start is still running, and
     * accepting it ends the invocation, so it is accepted at most once and the
     * state's exit no longer cancels it. Returns `false` — raising nothing —
     * for a completion of an invocation that was cancelled, already completed,
     * or restarted since (a stale token): §scxml-6.4 has the processor ignore
     * what a cancelled process sends, and a restarted `<invoke>` is a
     * different process under the same id.
     *
     * This is the only way a host-run invocation's `done.invoke` reaches the
     * document. One raised through the ordinary external-event API skips the
     * check above, so the engine refuses it when it is dequeued.
     *
     * `origin` is the completion's `_event.origin` and `_event.origintype`
     * (§scxml-5.10.1) as one pair, for a host whose invocation stands for
     * another party it can name — as a Mesh router's `sce:mesh-rpc` stands for
     * the peer that answered (SCE_MESH.md §mesh-9.5). The host names it in its
     * own terms (docs/adr/0090): its declared type and the address a `<send>`
     * of that type reaches the party at. None reads both fields empty; the
     * engine never fills in the SCXML processor's type for a host.
     *
     * An owner-thread operation (ARCHITECTURE.md "Engine Threading"): a host
     * whose invocation finishes on a thread of its own posts an
     * `InboxHostInvokeEnding` to `inbox()` instead, and the owner applies it
     * with this call when it next drains the inbox (`drainInbox`), where the
     * token is checked.
     */
    bool completeHostInvoke(const std::string &processorType, const std::string &invokeId, uint64_t token,
                            const std::string &doneData,
                            const std::optional<Core::EventOrigin> &origin = std::nullopt) {
        requireOwner("completeHostInvoke");
        return endHostInvoke(processorType, invokeId, token, false, doneData, origin);
    }

    /**
     * @brief §scxml-6.4: a host-run invocation failed; raise its
     *        `error.invoke.<invokeId>` — or the generic `error.invoke` — with
     *        `errorData` as `_event.data`
     *
     * The same door as completeHostInvoke(), and the same rules: accepted only
     * while start `token` is still running, ending the invocation, so a run
     * ends once — by success, failure or deadline, whichever comes first.
     * Returns `false`, raising nothing, for an invocation that was cancelled,
     * already ended, or restarted since. An owner-thread operation, posted
     * from another thread as `completeHostInvoke` is.
     */
    bool failHostInvoke(const std::string &processorType, const std::string &invokeId, uint64_t token,
                        const std::string &errorData, const std::optional<Core::EventOrigin> &origin = std::nullopt) {
        requireOwner("failHostInvoke");
        return endHostInvoke(processorType, invokeId, token, true, errorData, origin);
    }

private:
    /// This session's id as the policy states it, or none for a machine that holds no
    /// session id (docs/adr/0045).
    std::string ownSessionId() const {
        if constexpr (SCE::Core::HasSessionIdTextTrait<StatePolicy>::value) {
            return policy_.sessionIdText();
        } else {
            return std::string();
        }
    }

    /**
     * @brief Start the invocations a restore re-armed, as the macrostep's start would
     *        start ones the document had just entered (§scxml-6.4)
     *
     * A deadline that came due while the machine was away ends the invocation without a
     * start: the host is not asked to begin what it would be told to stop at once, and the
     * document receives the `error.invoke` the deadline raises. An invoker that is not
     * registered by now is the same error a document's own start gets. What each one
     * leaves to raise is raised before the next one starts.
     */
    void startRestoredHostInvokes() {
        if (!hostInvokes_.hasRestored()) {
            return;
        }
        hostInvokes_.startRestored(*invokers_, ownSessionId(), scheduler_,
                                   [this](const Core::HostInvokeReport &report) { raiseHostInvokeReport(report); });
    }

    /**
     * @brief Do what a start or a deadline of the ledger left this engine to do
     *
     * A start that did not happen is an error of the element (see
     * `performHostInvoke`), raised as `error.execution` on the internal queue with
     * what the host said or the engine found. A completion the host returned with the
     * start ends the run through the door a later completion takes. A deadline is an
     * event of the invocation, from outside the machine, and takes the external queue.
     */
    void raiseHostInvokeReport(const Core::HostInvokeReport &report) {
        if (const auto *notStarted = std::get_if<Core::HostInvokeNotStarted>(&report)) {
            if (auto error = policy_.getEventFromName("error.execution")) {
                raise(EventWithMetadata(*error, notStarted->reason));
            }
        } else if (const auto *completed = std::get_if<Core::HostInvokeCompletedAtOnce>(&report)) {
            (void)endHostInvoke(completed->processorType, completed->invokeId, completed->token, false,
                                completed->doneData, std::nullopt);
        } else if (const auto *timedOut = std::get_if<Core::HostInvokeTimedOut>(&report)) {
            raiseHostInvokeFailure(*timedOut);
        }
    }

    /**
     * @brief Tell the document a host-run invocation's deadline passed, as
     *        `error.invoke.<id>` with the reason as its data
     *
     * The reason is the JSON spelling of the string `"deadline"`, as every other backend
     * carries it: §scxml-B-2-8-1 reads data as JSON first, so a bare word would reach
     * `_event.data` only through the fallback rung.
     */
    void raiseHostInvokeFailure(const Core::HostInvokeTimedOut &timedOut) {
        // §scxml-5.10: `_event.name` is the specific name, whichever descriptor of the
        // document (`error.invoke.<id>`, `error.invoke`, `error`) it was matched through.
        const std::string name = timedOut.eventName();
        const auto failure = resolveEventByName(name);
        if (failure) {
            EventWithMetadata raised(*failure, Core::HostInvokeTimedOut::data(), std::nullopt, "", "external",
                                     timedOut.invokeId);
            raised.name = arrivalNameOf(*failure, name);
            raiseExternal(raised);
        }
    }

    /// The one body behind a host-run invocation's completion and failure: the
    /// ledger's token check and deadline drop, and the event.
    bool endHostInvoke(const std::string &processorType, const std::string &invokeId, uint64_t token, bool failed,
                       const std::string &data, const std::optional<Core::EventOrigin> &origin) {
        const auto ending = hostInvokes_.endRun(processorType, invokeId, token, failed, data, origin, scheduler_);
        if (!ending.has_value()) {
            return false;
        }
        // §scxml-5.10.1: the completion is an event of the invocation, so its
        // `_event.invokeid` is the invocation's id — the one the event names and
        // the host was handed.
        // §scxml-3.12.1: the specific `done.invoke.<id>` / `error.invoke.<id>`
        // when the document names it, and otherwise the generic event its
        // descriptor matches — as an SCXML child's completion does.
        const std::string name = ending->eventName();
        // §scxml-5.10: `_event.name` is the specific name, whichever descriptor of the
        // document (`done.invoke.<id>`, `done.invoke`, `done`) it was matched through.
        const auto ended = resolveEventByName(name);
        if (ended) {
            // docs/adr/0090, section 2: the pair the host named, as it named it,
            // or none; never the SCXML processor's type by default.
            EventWithMetadata completion(*ended, ending->data, ending->origin, "", "external", ending->invokeId);
            completion.name = arrivalNameOf(*ended, name);
            completion.hostInvokeToken = ending->token;
            raiseExternal(completion);
        } else {
            // The degradation raiseExternal(name) has: a document that wrote
            // no transition on the completion declares no such event, and
            // there is nothing to deliver it to.
            SCE_LOG_DEBUG("AOT host invoke end: '{}' not in Event enum, ignoring", name);
        }
        return true;
    }

public:
    /**
     * @brief How many host-run invocations' `done.invoke` events the engine
     *        refused because they did not arrive through completeHostInvoke()
     *
     * A refusal is correct — such an event may be a cancelled run's late
     * reply — and saying it happened is not optional: a host that raised its
     * completion the old way and saw the document never move would otherwise
     * have nothing to find.
     */
    uint64_t refusedHostInvokeCompletions() const {
        return refusedHostInvokeCompletions_;
    }

    /**
     * @brief §scxml-6.4: stop a host-run invocation whose state has exited
     *
     * Unconditional from the emitted exit chain; the engine knows whether the
     * invocation is still running and stays silent when it never started or
     * already completed.
     *
     * @return whether a cancel was delivered
     */
    bool cancelHostInvoke(const std::string &processorType, const std::string &invokeId) {
        requireOwner("cancelHostInvoke");
        // docs/adr/0045: one a restore re-armed and has not started again was never
        // started by this engine, so there is nothing to tell the host to stop; the
        // ledger drops it, so it is never started.
        return hostInvokes_.cancel(*invokers_, processorType, invokeId, scheduler_);
    }

private:
    /**
     * @brief The deadline of a host-run invocation start came due
     *
     * If that start is still running, end it: the host is told to stop, and
     * the document receives `error.invoke.<id>` — or the generic
     * `error.invoke` when it names no specific one — with `_event.invokeid`
     * set and `_event.data` the string `"deadline"`. Through the same door a
     * completion takes, so exactly one of the two happens.
     */
    void expireHostInvoke(const ::SCE::HostInvokeDeadline &deadline) {
        raiseHostInvokeReport(hostInvokes_.expire(*invokers_, deadline));
    }

public:
    /**
     * @brief The id for a `<send idlocation>`, which the document keeps and may
     *        name in a `<cancel>` later
     *
     * `_auto_send_` and the number of ids this machine has generated, counted
     * from one. Unique within the machine, as the send's id must be, and the
     * same text on every backend (at most 31 bytes: the largest `uint64_t` is
     * twenty digits).
     */
    std::string nextAutoSendId() {
        return "_auto_send_" + std::to_string(++autoSendSeq_);
    }

    /**
     * @brief §scxml-6.2.4 + §scxml-6.2.5: arm a host-served `<send delay>`,
     *        to be performed when the delay elapses
     *
     * The delayed twin of performHostSend(), called by the generated send site
     * in its place when the document wrote a `delay`. §scxml-6.2.4 makes the
     * wait a property of the send and not of the processor it named, so the two
     * differ in WHEN the act happens and in nothing else — including the
     * §scxml-6.2 report owed when nobody performs it, which tick() makes at the
     * deadline.
     *
     * The act lands in the same queue as scheduleEvent(), so cancelEvent()
     * drops it (§scxml-6.3) and timeUntilNextScheduled() counts it.
     *
     * §scxml-6.2.4: a delay that is zero — written as `0s` or evaluated to it
     * by a `delayexpr` — is no deferral. The act is performed here, before
     * this call returns, exactly as the immediate site performs it: waiting
     * for the next tick would make the same send take effect at one instant
     * when the document wrote the wait as a literal and at another when it
     * computed it. Nothing is queued, so there is nothing for `<cancel>` to
     * reach and the id is the caller's own.
     *
     * @return The sendId used, generated when @p sendId is empty and the send
     *         was queued
     */
    std::string scheduleHostSend(const ::SCE::HostSendRequest &request, std::chrono::milliseconds delay,
                                 const std::string &sendId = "") {
        if (delay.count() <= 0) {
            performDeferredHostSend(request);
            return sendId;
        }
        const uint64_t fireTimeMs = schedNowMs() + static_cast<uint64_t>(delay.count());
        return scheduler_.scheduleHostSendAt(std::make_shared<const ::SCE::HostSendRequest>(request), fireTimeMs,
                                             sendId);
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
     * @brief Register the factory of the routers of this machine's children
     *        (SCE_MESH.md, Child Sessions in Mesh; docs/adr/0043)
     *
     * A child session started by `<invoke type="scxml">` is given none of this
     * machine's registrations: its peer sends would leave under this machine's
     * name. The generated parent asks @p factory instead, each time an
     * invocation starts a child (on entry, and for the candidate a hybrid
     * `<invoke>` chose), with the invoke's id and the stem of the child's
     * document, and installs the answer on the child before the child
     * initializes (startChildMesh()). A factory that answers none, or none
     * registered, leaves the child with no router: its peer send raises
     * `error.execution` in the child, and nothing falls back to this machine's
     * router.
     */
    void registerChildMeshRouters(ChildMeshRouters factory) {
        childMeshRouters_ = std::move(factory);
        // The children a restore resumed before the host had a factory to give: each is
        // asked now, before anything drives it (`oweChildMesh`).
        std::vector<std::function<void()>> owed;
        owed.swap(owedChildMeshes_);
        for (const auto &ask : owed) {
            ask();
        }
    }

    /**
     * @brief Record that @p child, which a restore resumed, is owed the router this
     *        machine's factory gives the invoke @p invokeId of the document whose stem is
     *        @p document (docs/adr/0045, section 3, step 3)
     *
     * Not asked here: this machine is being restored, and its host registers the factory
     * of its children's routers only once the restore has returned it.
     * `registerChildMeshRouters` asks for each, before anything drives the child
     * (SCE_MESH.md, Child Sessions in Mesh). A factory registered already is asked at
     * once.
     */
    template <typename ChildEngine>
    void oweChildMesh(std::shared_ptr<ChildEngine> child, const std::string &invokeId, const std::string &document) {
        if (childMeshRouters_) {
            startChildMesh(*child, invokeId, document);
            return;
        }
        owedChildMeshes_.push_back(
            [this, child = std::move(child), invokeId, document] { startChildMesh(*child, invokeId, document); });
    }

    /**
     * @brief Ask this machine's factory for the router of @p child and install
     *        the answer on it
     *
     * @p child is what the invoke @p invokeId is starting from the document
     * whose stem is @p document (empty for a child written inline). Called by
     * the generated parent before @p child initializes, since an `<onentry>` of
     * its initial state may already send. Leaves @p child as it is when no
     * factory is registered or the factory answers none.
     */
    template <typename ChildEngine>
    void startChildMesh(ChildEngine &child, const std::string &invokeId, const std::string &document) {
        if (!childMeshRouters_) {
            return;
        }
        if (auto router = childMeshRouters_(invokeId, document)) {
            child.installChildMeshRouter(std::move(router));
        }
    }

    /**
     * @brief Install @p router as this machine's Mesh router and the factory of
     *        its children's routers
     *
     * The two a host installs on a machine it starts, served here by the one
     * router the parent was given for this session. A send the router does not
     * take is refused in this machine as a send no router is there for is.
     */
    void installChildMeshRouter(std::shared_ptr<IChildMeshRouter> router) {
        setMeshSendCallback([this, router](const std::string &target, const std::string &eventName,
                                           const std::string &data, const std::string &sendId,
                                           const std::string &invokeId) {
            if (!router->send(target, eventName, data, sendId, invokeId)) {
                refuseMeshSend(target, "the Mesh router of this session did not take the send");
            }
            return true;
        });
        // Through the registration a host makes, so the children this session was restored
        // with, owed the router its parent's factory gives them, are asked now.
        installedMesh_ = router;
        registerChildMeshRouters([router](const std::string &invokeId, const std::string &document) {
            return router->childRouter(invokeId, document);
        });
    }

    /**
     * @brief Apply what arrived for this machine on the router its parent gave
     *        it, in the order the router released it
     *
     * Each is raised on the external queue with the `_event` the endpoint built
     * (`raiseExternal(const ForwardedEvent &)`); a name the machine does not
     * declare is dropped, as the engine drops any such event. The generated
     * parent calls it each time it drives this child, before the child's
     * macrostep. Does nothing for a machine given no router.
     *
     * An owner-thread operation, called by the parent that drives this child on
     * the thread that drives it, so the router is asked for its calls only
     * there. A router that receives on a thread of its own keeps what it
     * received under its own lock until then: `takeCalls` and `send` are called
     * on the owner, and only the router's own receiving side runs elsewhere.
     */
    void applyMeshCalls() {
        requireOwner("applyMeshCalls");
        if (!installedMesh_) {
            return;
        }
        for (const ::SCE::Common::ForwardedEvent &call : installedMesh_->takeCalls()) {
            raiseExternal(call);
        }
    }

    /**
     * @brief End the child session this machine is, because its parent cancelled
     *        it, before the parent drops it
     *
     * Not exited: its `<onexit>` would send to the parent at once, and a parent
     * takes nothing from a session it cancelled (W3C test 252). Its run is over
     * all the same, and ends as any run ends (docs/adr/0084, D2): the sessions it
     * started end first, innermost first; the host-run invocations it started are
     * over, and their invokers are told to stop them; then its inbox closes and
     * what it holds is applied, so an ending finds its invocation over and is
     * refused by the token check and an event joins a queue nothing dequeues and
     * is counted unseen; and it leaves the tree's directory (`closeInbox`). Last,
     * its Mesh router is told (`detachMesh`). The generated parent calls it from
     * the exit of the invoking state, which the parent's own `stop` and final
     * state reach too.
     */
    void endAsCancelled() {
        requireOwner("endAsCancelled");
        if constexpr (SCE::Core::HasCancellableChildrenTrait<StatePolicy>::value) {
            policy_.endChildrenAsCancelled();
        }
        // The ledger's requests change as each is cancelled, so the ones to cancel
        // are named first.
        std::vector<std::pair<std::string, std::string>> started;
        for (const auto &running : hostInvokes_.running(scheduler_, [this] { return schedNowMs(); })) {
            started.emplace_back(running.request->processorType, running.request->invokeId);
        }
        for (const auto &[processorType, invokeId] : started) {
            (void)hostInvokes_.cancel(*invokers_, processorType, invokeId, scheduler_);
        }
        isRunning_ = false;
        closeInbox();
        recordUnseenExternalEvents();
        inbox_.state().releaseOwner();
        detachMesh();
    }

    /**
     * @brief The child session this machine is has ended: tell the routers of
     *        the sessions it still runs, innermost first, then the router its
     *        parent gave it, each once (SCE_MESH.md, Child Sessions in Mesh)
     *
     * Called after the child reached a top-level `<final>`, and by
     * `endAsCancelled` when its parent cancels it. Does nothing for a session
     * given no router, or told already.
     */
    void detachMesh() {
        if constexpr (SCE::Core::HasChildMeshDetachTrait<StatePolicy>::value) {
            policy_.detachChildMeshSessions();
        }
        if (std::shared_ptr<IChildMeshRouter> router = std::exchange(installedMesh_, nullptr)) {
            router->detach();
        }
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

private:
    /**
     * @brief Refuse a `<send target="#<peer>">` no router took: `error.execution`
     *        in this machine, saying why (SCE_MESH.md, Errors)
     *
     * Never a send to this machine's own queue, which would be a peer's event
     * answered by the sender. One body for a machine whose host wired no Mesh
     * transport and for a child whose router did not take the send.
     */
    void refuseMeshSend(const std::string &target, const char *why) {
        if (auto error = policy_.getEventFromName("error.execution")) {
            raise(EventWithMetadata(*error, "<send target='" + target + "'>: " + why));
        }
    }

    /**
     * @brief The door of an event just taken off a queue (docs/adr/0086, section 4.2)
     *
     * Binds `delivery` as `_event` with its typed `_event.data` view (NL→IR Item
     * C1 Path A). The generated policy's hook has two sources for that view. The
     * type-erased carrier is filled by ONE producer, the generated
     * `raise<Event>` inject seam; every other producer — `<send>` with
     * `<param>`, an invoke forwarding either way, autoforward, BasicHTTP, mesh —
     * fills `data`, so the fields are lifted out of that instead when no carrier
     * arrived. That is what lets the same guard answer the same way whichever
     * producer sent the event.
     *
     * A delivery whose data does not read as its schema is refused whole: it is
     * never offered to the configuration, and the `error.execution` that carries
     * the refusal is bound in its place and left in `inPlace`, where a guard
     * that cannot be evaluated is only false (§scxml-3.13). The verdict lives
     * here rather than in each generated policy so every machine answers alike;
     * a document that declares no `error.execution` transition has nothing to
     * take in its place (§scxml-3.12.2), and the delivery is refused all the
     * same.
     */
    Admission bindTypedPayload(const EventWithMetadata &delivery, std::optional<EventWithMetadata> &inPlace) {
        SCE::Common::EventMetadataHelper::populatePolicyFromMetadata<StatePolicy, Event>(policy_, delivery);
        if constexpr (SCE::Common::detail::has_populateTypedPayload<StatePolicy, EventWithMetadata>::value) {
            const std::string refusal = policy_.populateTypedPayload(delivery);
            if (refusal.empty()) {
                return Admission::Admitted;
            }
            const auto errorEvent = policy_.getEventFromName("error.execution");
            if (!errorEvent) {
                return Admission::Refused;
            }
            inPlace.emplace(*errorEvent,
                            std::string("`") + policy_.getEventName(delivery.event) + "` payload: " + refusal);
            // The engine's own error: it came off no queue a host or a child
            // fills, and it carries no typed payload, so binding it clears the
            // view the refused delivery left half read.
            currentEventInvokeId_.clear();
            currentEventFromExternalQueue_ = false;
            SCE::Common::EventMetadataHelper::populatePolicyFromMetadata<StatePolicy, Event>(policy_, *inPlace);
            (void)policy_.populateTypedPayload(*inPlace);
            return Admission::InPlace;
        } else {
            (void)inPlace;
            return Admission::Admitted;
        }
    }

    /**
     * @brief Answer `error`, the `error.execution` a door took in a refused
     *        external delivery's place, as one off the internal queue is
     *
     * An error raised while it is answered is a link in its chain, and one no
     * transition matched is counted as unhandled (docs/adr/0086, section 4.2).
     */
    EventOutcome answerInPlace(Event error) {
        handlingErrorEvent_ = true;
        const EventOutcome outcome = executeTransition(error);
        handlingErrorEvent_ = false;
        recordInternalEventOutcome(error, outcome);
        if (outcome.selected) {
            ++macrostepMicrostepsTaken_;
        }
        return outcome;
    }

    /**
     * @brief Perform a host-served send whose delay has elapsed, and report it
     *        if nobody did (§scxml-6.2 + §scxml-6.2.4)
     *
     * The immediate path raises `error.execution` at the send site, which knows
     * the document's event enum by name. A deferred one cannot: that site
     * returned when the send was armed, so the engine owes the report — and it
     * can make it, because `getEventFromName()` is the same lookup
     * performHostSend() already uses to turn a handler's replies into events. A
     * document that declares no `error.execution` transition resolves nothing
     * and nothing is raised, which is what the generated site's own template
     * guard does.
     *
     * Without this, a wiring mistake on a delayed send is perfect silence: the
     * act never happens, nothing says so, and the document goes on waiting for
     * a reply that has nobody left to come from.
     */
    void performDeferredHostSend(const ::SCE::HostSendRequest &request) {
        // §scxml-C-2 + §scxml-6.2.4: a delayed BasicHTTP send waits in this
        // queue as a host-served one does, and its deadline performs the POST
        // the immediate send would have made — through the same callback, so
        // the reply it draws comes back as it does from an immediate one.
        if (request.processorType == ::SCE::Constants::BASIC_HTTP_EVENT_PROCESSOR_URI) {
            performHttpSend(request.target, request.eventName, request.content, request.params, request.sendId);
            return;
        }
        const auto served = performHostSend(request);
        if (served.has_value() || hasEventProcessor(request.processorType)) {
            return;
        }
        if (auto event = policy_.getEventFromName("error.execution")) {
            // The same sentence the immediate site emits. One wording for one
            // fact: a consumer matching on the message must not have to know
            // whether the send it wrote carried a delay.
            //
            // §scxml-6.2.4 + §scxml-5.10 (test 332): the error event MUST carry the
            // sendid, and a deferred send always has one — the scheduler needed
            // it to be cancellable.
            raise(EventWithMetadata(*event,
                                    "<send type='" + request.processorType +
                                        "'> names a processor the host declared but never registered",
                                    std::nullopt, request.sendId));
        }
    }
};

}  // namespace SCE::Static
