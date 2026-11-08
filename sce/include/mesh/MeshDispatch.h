// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh MeshDispatch — shared envelope-to-engine dispatch helper.
//
// Single source of truth for pattern-based envelope delivery. Used by:
//   - ShmChannel::drain()                     (receiver side)
//   - TransportRouter::onIncoming()           (receiver side)
//   - TransportRouter::route_send() local     (same-process shortcut)
//
// Pattern dispatch:
//   Inbound (enqueue to engine): FireForget, RpcRequest, RpcReply, EventNotify,
//                                FieldNotify, FieldRead, FieldWrite,
//                                ChildEvent, InvokeError
//   Lifecycle hook (engine-side): InvokeStarted (→onInvokeStarted),
//                                 InvokeDone (→onInvokeDone)
//   Outbound-only (reject):      EventSubscribe, EventUnsubscribe, InvokeStart,
//                                ParentEvent, InvokeCancel
//
// SCE_MESH.md §mesh-9.6.2 wire 14 (`InvokeStart`), wire 17 (`ParentEvent`), and
// wire 19 (`InvokeCancel`) flow parent→child. They are consumed by the
// worker's transport router's inbound path (via `WorkerSessionHost`) before
// reaching this helper. If one arrives here it means the upstream branch
// did not catch it and we drop fail-closed (same shape as the
// EventSubscribe echo guard).
//
// SCE_MESH.md §mesh-9.6.2 wire 15 (`InvokeStarted`): parent-side receiver. The
// envelope carries `invoke_id`, `child_session_id` and, in `reply_to`, the
// child session's endpoint. Dispatch invokes `engine.onInvokeStarted(env)`
// (SFINAE-gated) which stamps the child session id into
// `activeInvokes_[invoke_id].sessionId` so subsequent wire-16 ChildEvent
// envelopes match the child's identity for finalize and autoforward
// routing, and the endpoint beside it so a reply sent to the origin of
// the child's events finds the child.
//
// SCE_MESH.md §mesh-9.6.2 wire 16 (`ChildEvent`): parent-side receiver. Same
// inbox post as FireForget. The origin pair is the W3C SCXML processor URI at
// the child session's endpoint (`mesh://<device>/<machine>/<session>`, see
// common/MeshEndpoint.h), which the worker writes in the envelope's
// `reply_to`; the child session id is the event's sender, and
// `_event.invokeid = invoke_id`. The parent engine's `<finalize>` matching
// (invoke_methods.jinja2) compares `activeInvokes_[id].sessionId ==
// meta.sender`, as on the local-invoke path.
//
// Wires 15 and 16 take `child_session_id` only when it names a session: a
// session id is an XML Nmtoken (saved/SessionId.h), and the parent writes
// this one as the sender of the child's events, which a saved state refuses
// when it is no Nmtoken. An envelope whose `child_session_id` is absent or
// no Nmtoken names no session and is dropped, the one rule
// `detail::namesChildSession` holds for both wires. Wires 18 and 20 are
// matched on `subject` and `invoke_id` and do not read the field.
//
// SCE_MESH.md §mesh-9.6.2 wire 18 (`InvokeDone`): parent-side receiver. The
// envelope carries `invoke_id` and donedata in `data`. Dispatch invokes
// `engine.onInvokeDone(env)` (SFINAE-gated) which raises
// `done.invoke.<id>` and releases `activeInvokes_[invoke_id]`. Machines
// without authored invokes do not generate the hook; envelope is dropped.
//
// SCE_MESH.md §mesh-9.6.2 wire 20 (`InvokeError`): parent-side receiver. The
// envelope carries `invoke_id`, `rpc_status`, and `rpc_error_message`; this
// dispatch raises `error.execution` on the parent engine with the
// `rpc_error_message` carried through `EventWithMetadata::data` so authors'
// `<transition event="error.execution">` observes the same raise shape as the
// transport-absent local fallback (SCE_MESH.md §mesh-9.6 line 1396).
//
// FieldRead/FieldWrite are inbound on the server role (SCE_MESH.md §mesh-8.1
// `field.get` / `field.set` pattern semantics): the server's queryable /
// `register_message_handler` receives the
// getter/setter request and dispatches it to the engine, which fires the
// matching `<transition event="field.get.X">` / `<transition event="field.set.X">`.
//
// RpcRequest is inbound on the receiver side: the sender's
// `<invoke type="sce:mesh-rpc">` (SCE_MESH.md §mesh-9.5) emits an envelope whose
// `type` is the request event name (e.g. `service.request.compute_force`),
// which is enqueued on the receiver engine just like any FireForget event.
// The correlation UUID in `env.invoke_id` is surfaced to the receiver SCXML
// as `_event.invokeid` (§scxml-5.10.1) through the engine's metadata
// pipeline — dispatchEnvelope always constructs `Engine::EventWithMetadata`
// and calls the metadata overload so test doubles observe the same path
// production engines take (no SFINAE fallback that could silently diverge).
//
// Returns false for outbound-only patterns (misconfigured echo-back) and
// unknown pattern kinds (forward compatibility).

#pragma once

#include "common/MeshEndpoint.h"
#include "common/SCXMLConstants.h"
#include "common/Uuid.h"
#include "core/EventNameResolver.h"
#include "core/EventOrigin.h"
#include "core/HostProcessor.h"
#include "core/StatePolicyConcepts.h"
#include "mesh/MeshEnvelope.h"
#include "saved/SessionId.h"

#include <string>
#include <type_traits>
#include <utility>

namespace SCE::Mesh {

/// URI scheme for `_event.origin` on a distributed event; see
/// [meshOriginUri] for the form and its rationale. The one scheme a session
/// endpoint (common/MeshEndpoint.h) spells too, so a peer's address and a
/// session's cannot drift apart into two wire forms.
inline constexpr const char *kMeshOriginScheme = ::SCE::Common::MESH_ENDPOINT_SCHEME;

/// Build the `_event.origin` value for an inbound envelope.
inline std::string meshOriginUri(const std::string &source) {
    // SCE_MESH.md §mesh-10.7 fixes the author-facing form as
    // `mesh://<envelope.source>`, so a guard reads the same whether the
    // event came off the local queue or any transport.
    //
    // `source` is the sender's stable machine name (`MeshEnvelope::source`),
    // not its per-session `routing_id`: the section's example guard
    // (`_event.origin == 'mesh://chassis'`) compares against a document
    // identity an author can write down, which a per-session UUID is not.
    return std::string(kMeshOriginScheme) + source;
}

/// The pair the Mesh transport writes for an event a peer's envelope delivers
/// (docs/adr/0090, section 2): the SCXML Event I/O Processor, since every
/// envelope is SCE's own inter-SCXML traffic (SCE_MESH.md, `_event` field
/// wiring for distributed events), at the address [meshOriginUri] gives the peer.
inline ::SCE::Core::EventOrigin meshPeerOrigin(const std::string &source) {
    return ::SCE::Core::EventOrigin(SCE::Constants::SCXML_EVENT_PROCESSOR_TYPE, meshOriginUri(source));
}

/// The pair the Mesh transport writes for the answer to an `<invoke type="sce:mesh-rpc">`
/// (SCE_MESH.md, the runtime mapping of a request's reply to `_event`): the request's own
/// type at the address [meshOriginUri] gives the peer that answered.
inline ::SCE::Core::EventOrigin meshRpcReplyOrigin(const std::string &replier) {
    return ::SCE::Core::EventOrigin(std::string(::SCE::MESH_RPC_INVOKE_TYPE), meshOriginUri(replier));
}

/// The pair the Mesh transport writes for an event a remote child session sent
/// its parent (SCE_MESH.md, `_event` field wiring of a full remote invoke): the
/// SCXML Event I/O Processor at the child session's endpoint, which the worker
/// that runs the child names in the envelope's `reply_to`. A `<send>` of that
/// type to it reaches the child (docs/adr/0090). An envelope whose `reply_to`
/// names no session endpoint comes from a worker that wrote none, and its
/// event takes the address the transport reaches the worker's machine at.
inline ::SCE::Core::EventOrigin meshChildSessionOrigin(const MeshEnvelope &env) {
    if (env.reply_to && ::SCE::Common::isMeshSessionEndpoint(*env.reply_to)) {
        return ::SCE::Core::EventOrigin(SCE::Constants::SCXML_EVENT_PROCESSOR_TYPE, *env.reply_to);
    }
    return meshPeerOrigin(env.source);
}

/// The endpoint (common/MeshEndpoint.h) of the session @p engine runs as
/// machine @p machine on device @p device, the address a remote child names
/// as the origin of the events that session sends it; empty for an engine
/// that holds no session id. Called on the engine's owner thread, since the
/// id is drawn there the first time it is read.
template <typename Engine>
std::string meshEndpointOfSession(const std::string &device, const std::string &machine, Engine &engine) {
    if constexpr (::SCE::Core::HasMachineSessionIdTrait<Engine>::value) {
        return ::SCE::Common::meshSessionEndpoint(device, machine, engine.getPolicy().sessionIdText());
    } else {
        (void)device;
        (void)machine;
        (void)engine;
        return std::string();
    }
}

namespace detail {

/// SFINAE probe: does `Engine` expose
/// `onParallelRegionDone(const MeshEnvelope&)`? Only generated SMs with
/// distributed `<parallel>` trackers emit the method; monolithic machines
/// don't, so the wire-21 arm falls through to "return false" (drop) for
/// them — the envelope would have arrived in error anyway (no claimant
/// partition configured).
template <typename Engine, typename = void> struct HasParallelRegionDoneHook : std::false_type {};

template <typename Engine>
struct HasParallelRegionDoneHook<
    Engine, std::void_t<decltype(std::declval<Engine &>().onParallelRegionDone(std::declval<const MeshEnvelope &>()))>>
    : std::true_type {};

template <typename Engine>
bool tryDeliverParallelRegionDone(const MeshEnvelope &env, Engine &engine, std::true_type /*has_hook*/) {
    engine.onParallelRegionDone(env);
    return true;
}

template <typename Engine>
bool tryDeliverParallelRegionDone(const MeshEnvelope & /*env*/, Engine & /*engine*/, std::false_type /*has_hook*/) {
    return false;
}

/// Whether @p env names a child session in `child_session_id` (SCE_MESH.md,
/// session establishment of a remote invoke): the field is present and holds
/// an XML Nmtoken, which every session id is (saved/SessionId.h). The parent
/// stamps the id into `activeInvokes_` from wire 15 and writes it as the
/// sender of each wire-16 event (docs/adr/0090), and every saved-state reader
/// refuses a sender that is no Nmtoken, so taking one in would let the parent
/// save a state it cannot restore. A worker this build generates never sends
/// one (`WorkerSessionHost::onWire14` brings no session up under it); an
/// envelope from any other carries one only by fault, and it names no
/// session, as one without the field names none.
inline bool namesChildSession(const MeshEnvelope &env) {
    return env.child_session_id.has_value() && ::SCE::Saved::SessionId::isNmtoken(*env.child_session_id);
}

/// SFINAE probe: does `Engine` expose
/// `onInvokeStarted(const MeshEnvelope&)`? Only parent engines that host at
/// least one `<invoke type="scxml">` targeting a mesh peer emit the hook;
/// machines without authored invokes drop the wire-15 envelope.
template <typename Engine, typename = void> struct HasInvokeStartedHook : std::false_type {};

template <typename Engine>
struct HasInvokeStartedHook<
    Engine, std::void_t<decltype(std::declval<Engine &>().onInvokeStarted(std::declval<const MeshEnvelope &>()))>>
    : std::true_type {};

/// A wire-15 envelope that names no child session ([namesChildSession]) is
/// dropped here rather than handed to the hook, so the parent's invoke table
/// never holds an id that is no session id.
template <typename Engine>
bool tryDeliverInvokeStarted(const MeshEnvelope &env, Engine &engine, std::true_type /*has_hook*/) {
    if (!namesChildSession(env)) {
        return false;
    }
    engine.onInvokeStarted(env);
    return true;
}

template <typename Engine>
bool tryDeliverInvokeStarted(const MeshEnvelope & /*env*/, Engine & /*engine*/, std::false_type /*has_hook*/) {
    return false;
}

/// SFINAE probe: does `Engine` expose
/// `hasActiveChildSession(const std::string&)`? Emitted by every SM with at
/// least one `<invoke type="scxml">` targeting a mesh peer, alongside the
/// wire-15/18 hooks. An engine that cannot answer it holds no invoke table
/// and therefore cannot be the parent of any child session, so a wire-16
/// envelope reaching it is misrouted and drops fail-closed — the same shape
/// the wire-15/18 hooks use when absent. Probed, never called here: the
/// question reads the engine's invoke table, which is its owner's, so the
/// owner asks it when it applies the posted event (docs/adr/0084).
template <typename Engine, typename = void> struct HasActiveChildSessionHook : std::false_type {};

template <typename Engine>
struct HasActiveChildSessionHook<
    Engine, std::void_t<decltype(std::declval<Engine &>().hasActiveChildSession(std::declval<const std::string &>()))>>
    : std::true_type {};

/// SFINAE probe: does `Engine` expose
/// `onInvokeDone(const MeshEnvelope&)`? Symmetric to `HasInvokeStartedHook`.
template <typename Engine, typename = void> struct HasInvokeDoneHook : std::false_type {};

template <typename Engine>
struct HasInvokeDoneHook<
    Engine, std::void_t<decltype(std::declval<Engine &>().onInvokeDone(std::declval<const MeshEnvelope &>()))>>
    : std::true_type {};

template <typename Engine>
bool tryDeliverInvokeDone(const MeshEnvelope &env, Engine &engine, std::true_type /*has_hook*/) {
    engine.onInvokeDone(env);
    return true;
}

template <typename Engine>
bool tryDeliverInvokeDone(const MeshEnvelope & /*env*/, Engine & /*engine*/, std::false_type /*has_hook*/) {
    return false;
}

/// §scxml-5.10: record on `meta` the name `type` arrived under, when `event` is the
/// member of a shorter name (§scxml-3.12.1) and not of `type` itself. A receiver that
/// keeps no arrival name — a test double, an engine of a consumer's own — has nowhere
/// to record it, and is left as it was. The two probes are SFINAE rather than a
/// `requires` expression: this header is included by consumers built as C++17.
template <typename Meta, typename = void> struct HasArrivalName : std::false_type {};

template <typename Meta>
struct HasArrivalName<Meta, std::void_t<decltype(std::declval<Meta &>().name)>> : std::true_type {};

template <typename Policy, typename Event, typename = void> struct HasEventNameOf : std::false_type {};

template <typename Policy, typename Event>
struct HasEventNameOf<Policy, Event, std::void_t<decltype(Policy::getEventName(std::declval<const Event &>()))>>
    : std::true_type {};

template <typename Policy, typename Meta, typename Event>
void recordArrivalName(Meta &meta, const Event &event, const std::string &type) {
    if constexpr (HasArrivalName<Meta>::value && HasEventNameOf<Policy, Event>::value) {
        if (type != Policy::getEventName(event)) {
            meta.name = type;
        }
    }
}

/// Hand @p meta to @p engine as an external event through the engine's inbox
/// (docs/adr/0084, ARCHITECTURE.md "Engine Threading"); whether the inbox took
/// it. This helper runs on transport threads (a vsomeip application thread, a
/// zenoh runtime thread, a TCP read thread) as well as on the engine's owner,
/// and the inbox is the one part of an engine any thread may use. The owner
/// applies the event with `raiseExternal` at its next macrostep boundary, before
/// it consults its external queue, so it is the same event a direct raise would
/// have queued.
template <typename Engine> bool postExternal(Engine &engine, typename Engine::EventWithMetadata meta) {
    return engine.inbox().post(typename Engine::InboxExternalEvent{std::move(meta)});
}

}  // namespace detail

/// Dispatch a MeshEnvelope to a state machine engine based on pattern kind.
///
/// Inbound patterns (FireForget, RpcRequest, RpcReply, EventNotify,
/// FieldNotify): resolve event name via Policy::getEventFromName and post the
/// event to the engine's inbox (`detail::postExternal`). Outbound-only patterns
/// return false (echo-back guard). Returns false for an unresolved event name,
/// and for an event the inbox refused, to signal the caller the envelope could
/// not be delivered.
///
/// SCE_MESH.md §mesh-16.5 `ParallelRegionDone` (wire 21): bypasses `raiseExternal`
/// and routes to `engine.onParallelRegionDone(env)` when present. Machines
/// without the hook drop the envelope — they could not have authored a
/// distributed `<parallel>` root that expected one.
///
/// @tparam Policy  Receiver's generated StatePolicy (provides getEventFromName)
/// @tparam Engine  StaticExecutionEngine<Policy>
/// @return true if the event was dispatched to the engine
template <typename Policy, typename Engine> bool dispatchEnvelope(const MeshEnvelope &env, Engine &engine) {
    if (env.pattern == PatternKind::ParallelRegionDone) {
        return detail::tryDeliverParallelRegionDone(env, engine, detail::HasParallelRegionDoneHook<Engine>{});
    }
    if (env.pattern == PatternKind::InvokeStarted) {
        return detail::tryDeliverInvokeStarted(env, engine, detail::HasInvokeStartedHook<Engine>{});
    }
    if (env.pattern == PatternKind::InvokeDone) {
        return detail::tryDeliverInvokeDone(env, engine, detail::HasInvokeDoneHook<Engine>{});
    }
    switch (env.pattern) {
    case PatternKind::FireForget:
    // Inbound notifications are unidirectional — same delivery as FireForget.
    // The pattern discriminator enables transport-level routing (e.g. SOME/IP
    // event group subscription) but the engine dispatch is identical.
    case PatternKind::EventNotify:
    case PatternKind::FieldNotify:
    // SCE_MESH.md §mesh-9.5 request: sender's <invoke type="sce:mesh-rpc"> emits an
    // envelope whose `type` carries the request event name. Receiver engine
    // enqueues it identically to FireForget; the correlation UUID in
    // env.invoke_id stays with the transport layer for reply matching and is
    // not interpreted here.
    case PatternKind::RpcRequest:
    // RPC reply: correlation matching is done by TransportRouter before calling
    // dispatchEnvelope. By the time we get here, env.type is already set to the
    // reply-event name from the correlation table. Delivery is same as FireForget.
    case PatternKind::RpcReply:
    // FieldRead/FieldWrite on the server role: inbound request that fires the
    // matching `<transition event="field.get.X">` / `<transition event="field.set.X">`
    // on the server-side engine (SCE_MESH.md §mesh-8.1 `field.get` / `field.set`).
    case PatternKind::FieldRead:
    case PatternKind::FieldWrite: {
        // A peer names the event, so the name is open (§scxml-3.12.1): it is delivered
        // as the document's event it extends, when it is not one the document writes.
        auto ev = ::SCE::Core::resolveArrivingEventName(
            env.type, [](const std::string &exact) { return Policy::getEventFromName(exact.c_str()); });
        if (!ev) {
            return false;
        }
        // SCE_MESH.md §mesh-10.7 `_event` field wiring for distributed events.
        // The section's table maps envelope fields onto the §scxml-5.10.1
        // `_event` surface, and its stated purpose is that a guard such as
        // `<transition cond="_event.origin == 'mesh://chassis'">` selects
        // identically whether the event arrived locally or over a transport.
        // Wiring it here rather than per-transport is what makes that true
        // for every byte-stream substrate at once: this helper is the sole
        // envelope-to-engine path (ShmChannel::drain, TransportRouter::
        // onIncoming, and route_send's local branch all funnel through it).
        //
        // `_event.type` is deliberately not set: `raiseExternal` puts the
        // event on the external queue, and the engine classifies queue
        // membership as "external" per §scxml-5.10.1. Stamping the string
        // here would duplicate that classification in a second place.
        typename Engine::EventWithMetadata meta;
        meta.event = *ev;
        // §scxml-5.10: `_event.name` is the name the peer sent, not the member it
        // was matched through.
        detail::recordArrivalName<Policy>(meta, *ev, env.type);
        // SCE_MESH.md §mesh-9.5: a reply whose `rpc_status` is present and
        // non-Ok carries no result — the payload is empty by construction
        // and the thing the author needs is the reason. Surfacing
        // `rpc_error_message` through `EventWithMetadata::data` matches
        // the InvokeError arm below, so both failure shapes read the same
        // way regardless of whether the request went out through
        // `<invoke type="sce:mesh-rpc">` or a plain `<send>`.
        //
        // Absent `rpc_status` means "Ok" per the field's contract, so the
        // ordinary success path is unchanged.
        const bool reply_failed = env.rpc_status.has_value() && *env.rpc_status != RpcStatus::Ok;
        if (reply_failed && env.rpc_error_message) {
            meta.data = *env.rpc_error_message;
        } else {
            meta.data = std::string(env.data.begin(), env.data.end());
        }
        // Every envelope reaching this helper is SCE's own CBOR MeshEnvelope
        // (§mesh-7.5), so the traffic is inter-SCXML by construction and takes
        // the W3C processor URI. The section's "transport-specific URIs for
        // bridged traffic" case does not arrive as a MeshEnvelope at all.
        // The transport writes the pair whole (docs/adr/0090, section 2).
        meta.origin = meshPeerOrigin(env.source);
        // "envelope `subject` (or unset if not `<send>`-originated)" — left
        // empty when absent rather than synthesized, so `<cancel sendid=…>`
        // can never match an id no author issued.
        if (env.subject) {
            meta.sendId = *env.subject;
        }
        if (env.invoke_id) {
            meta.invokeId = SCE::uuid::to_string(*env.invoke_id);
        }
        return detail::postExternal(engine, std::move(meta));
    }
    // SCE_MESH.md §mesh-9.6.2 wire 16 (ChildEvent): parent receives an event
    // emitted by a remote child session via `<send target="#_parent">` or
    // via §scxml-5.10 auto-raise. Event name is `env.type`; metadata fields
    // are wired per §mesh-9.6.3 L1463-1466:
    //   _event.type        = "external"
    //   _event.origin      = the child session's endpoint, env.reply_to
    //   _event.origintype  = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"
    //   _event.invokeid    = env.invoke_id (same UUID the parent stored)
    //   _event.sendid      = env.subject (child's sendid, transparent)
    //   _event.data        = env.data payload
    // The child's session id (env.child_session_id) travels as the event's
    // sender, which the parent's local-invoke finalize code path matches on
    // (`childSession.sessionId == meta.sender`), so it is never an address.
    case PatternKind::ChildEvent: {
        // SCE_MESH.md §mesh-9.6.4 steps 1-2 and its closing rule. Step 4 —
        // identifying the `<invoke>` the event belongs to — is resolved before
        // the event joins the parent's external queue rather than after it:
        // once the parent has left the invoking state the section requires
        // the event to be "discarded silently (finalize not executed,
        // transition not considered)", and an event already sitting in the
        // queue would still be offered to transition selection.
        //
        // The event arrives where the parent's owner takes it: at its
        // macrostep boundary (docs/adr/0084), since no input is observed in
        // the middle of a macrostep. So the envelope is posted with the
        // child's session id, and the owner asks `hasActiveChildSession` when
        // it applies it — the parent's invoke table is its owner's alone, and
        // this runs on transport threads. An event that arrived before the
        // macrostep in which the parent exits the invoking state is applied
        // before that exit, while `activeInvokes_` still holds the entry.
        if constexpr (detail::HasActiveChildSessionHook<Engine>::value) {
            // An envelope that names no child session has no sender to give
            // its event and no session to admit it for.
            if (!detail::namesChildSession(env)) {
                return false;
            }
            auto ev = ::SCE::Core::resolveArrivingEventName(
                env.type, [](const std::string &exact) { return Policy::getEventFromName(exact.c_str()); });
            if (!ev) {
                return false;
            }
            typename Engine::EventWithMetadata meta;
            meta.event = *ev;
            detail::recordArrivalName<Policy>(meta, *ev, env.type);
            meta.data = std::string(env.data.begin(), env.data.end());
            meta.type = "external";
            // docs/adr/0090, section 2: the transport writes the §mesh-9.6.3 row,
            // the child session's endpoint the worker named, so a reply sent to
            // the origin reaches that session. The parent matches `<finalize>`
            // by comparing `activeInvokes_[id].sessionId` with the event's
            // sender, and that sessionId is the §mesh-9.6.1 child session id
            // wire-15 delivered, so the id is the sender, never the address.
            meta.origin = meshChildSessionOrigin(env);
            meta.sender = *env.child_session_id;
            if (env.subject) {
                meta.sendId = *env.subject;
            }
            if (env.invoke_id) {
                meta.invokeId = SCE::uuid::to_string(*env.invoke_id);
            }
            return engine.inbox().post(typename Engine::InboxChildSessionEvent{*env.child_session_id, std::move(meta)});
        } else {
            return false;
        }
    }
    // SCE_MESH.md §mesh-9.6.2 wire 20 (InvokeError): parent receives a child-
    // instantiation or transport-unavailable failure and translates it into
    // a local `error.execution` raise. The reason text lives in
    // `rpc_error_message`; we surface it via `EventWithMetadata::data` so
    // authors read the same payload shape the transport-absent local raise
    // produces. The structured `_event.data.reason` JSON is a separate
    // landing at §mesh-10.7.1 once an SCXML consumer exists.
    case PatternKind::InvokeError: {
        auto ev = Policy::getEventFromName("error.execution");
        if (!ev) {
            return false;
        }
        typename Engine::EventWithMetadata meta;
        meta.event = *ev;
        if (env.rpc_error_message) {
            meta.data = *env.rpc_error_message;
        }
        if (env.invoke_id) {
            meta.invokeId = SCE::uuid::to_string(*env.invoke_id);
        }
        return detail::postExternal(engine, std::move(meta));
    }
    // Outbound-only patterns: these are sent by the engine, not received.
    // If they arrive here it means a misconfigured transport is echoing
    // back our own sends — return false so caller can log or drop.
    case PatternKind::EventSubscribe:
    case PatternKind::EventUnsubscribe:
        return false;
    // ParallelRegionDone / InvokeStarted / InvokeDone are handled by the
    // pre-switch early-return above; listing them here keeps `-Wswitch-enum`
    // exhaustive. Reaching these arms would be a compiler / codegen bug and
    // we fail-closed with drop.
    case PatternKind::ParallelRegionDone:
    case PatternKind::InvokeStarted:
    case PatternKind::InvokeDone:
        return false;
    // Parent→child wires: caught upstream by the worker's transport router
    // inbound branch (WorkerSessionHost::onWireN). If one reaches
    // dispatchEnvelope it means the upstream branch missed it — fail-closed
    // drop, same shape as the EventSubscribe echo guard above.
    case PatternKind::InvokeStart:
    case PatternKind::ParentEvent:
    case PatternKind::InvokeCancel:
        return false;
    // A causal route's control message (docs/adr/0085, section 3.2) never
    // reaches an engine. This core carries no causal route (the build refuses
    // `delivery: causal` for C++), so one that arrives is dropped.
    case PatternKind::CausalVector:
        return false;
    }
    return false;  // unknown pattern kind — forward compatibility
}

}  // namespace SCE::Mesh
