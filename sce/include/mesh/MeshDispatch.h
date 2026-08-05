// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
// envelope carries `invoke_id` and `child_session_id`. Dispatch invokes
// `engine.onInvokeStarted(env)` (SFINAE-gated) which stamps the child
// session URI into `activeInvokes_[invoke_id].sessionId` so subsequent
// wire-16 ChildEvent envelopes match the child's identity for finalize
// and autoforward routing.
//
// SCE_MESH.md §mesh-9.6.2 wire 16 (`ChildEvent`): parent-side receiver. Same
// `raiseExternal` path as FireForget but additionally sets
// `_event.origin = child_session_id`, `_event.invokeid = invoke_id`, and
// `_event.origintype` to the W3C SCXML processor URI (§mesh-9.6.3 L1463-1466).
// The parent engine's `<finalize>` matching (invoke_methods.jinja2)
// compares `activeInvokes_[id].sessionId == meta.origin`, inherited
// unchanged from the local-invoke path.
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

#include "common/SCXMLConstants.h"
#include "common/Uuid.h"
#include "mesh/MeshEnvelope.h"

#include <string>
#include <type_traits>
#include <utility>

namespace SCE::Mesh {

/// URI scheme for `_event.origin` on a distributed event; see
/// [meshOriginUri] for the form and its rationale. Declared once because
/// this header is the single source of truth for envelope-to-engine
/// delivery — a second literal elsewhere would be a wire form co-declared
/// in two places, free to drift.
inline constexpr const char *kMeshOriginScheme = "mesh://";

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

/// SFINAE probe: does `Engine` expose
/// `onInvokeStarted(const MeshEnvelope&)`? Only parent engines that host at
/// least one `<invoke type="scxml">` targeting a mesh peer emit the hook;
/// machines without authored invokes drop the wire-15 envelope.
template <typename Engine, typename = void> struct HasInvokeStartedHook : std::false_type {};

template <typename Engine>
struct HasInvokeStartedHook<
    Engine, std::void_t<decltype(std::declval<Engine &>().onInvokeStarted(std::declval<const MeshEnvelope &>()))>>
    : std::true_type {};

template <typename Engine>
bool tryDeliverInvokeStarted(const MeshEnvelope &env, Engine &engine, std::true_type /*has_hook*/) {
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
/// the wire-15/18 hooks use when absent.
template <typename Engine, typename = void> struct HasActiveChildSessionHook : std::false_type {};

template <typename Engine>
struct HasActiveChildSessionHook<
    Engine, std::void_t<decltype(std::declval<Engine &>().hasActiveChildSession(std::declval<const std::string &>()))>>
    : std::true_type {};

template <typename Engine>
bool childSessionIsActive(const std::string &child_session_id, Engine &engine, std::true_type /*has_hook*/) {
    return engine.hasActiveChildSession(child_session_id);
}

template <typename Engine>
bool childSessionIsActive(const std::string & /*child_session_id*/, Engine & /*engine*/, std::false_type /*has_hook*/) {
    return false;
}

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

}  // namespace detail

/// Dispatch a MeshEnvelope to a state machine engine based on pattern kind.
///
/// Inbound patterns (FireForget, RpcRequest, RpcReply, EventNotify,
/// FieldNotify): resolve event name via Policy::getEventFromName and enqueue
/// via engine.raiseExternal(). Outbound-only patterns return false (echo-back
/// guard). Returns false for an unresolved event name to signal the caller
/// the envelope could not be delivered.
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
        auto ev = Policy::getEventFromName(env.type.c_str());
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
        meta.origin = meshOriginUri(env.source);
        // Every envelope reaching this helper is SCE's own CBOR MeshEnvelope
        // (§mesh-7.5), so the traffic is inter-SCXML by construction and takes
        // the W3C processor URI. The section's "transport-specific URIs for
        // bridged traffic" case does not arrive as a MeshEnvelope at all.
        meta.originType = SCE::Constants::SCXML_EVENT_PROCESSOR_TYPE;
        // "envelope `subject` (or unset if not `<send>`-originated)" — left
        // empty when absent rather than synthesized, so `<cancel sendid=…>`
        // can never match an id no author issued.
        if (env.subject) {
            meta.sendId = *env.subject;
        }
        if (env.invoke_id) {
            meta.invokeId = SCE::uuid::to_string(*env.invoke_id);
        }
        engine.raiseExternal(std::move(meta));
        return true;
    }
    // SCE_MESH.md §mesh-9.6.2 wire 16 (ChildEvent): parent receives an event
    // emitted by a remote child session via `<send target="#_parent">` or
    // via §scxml-5.10 auto-raise. Event name is `env.type`; metadata fields
    // are wired per §mesh-9.6.3 L1463-1466:
    //   _event.type        = "external"
    //   _event.origin      = child's session URI (env.child_session_id)
    //   _event.origintype  = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"
    //   _event.invokeid    = env.invoke_id (same UUID the parent stored)
    //   _event.sendid      = env.subject (child's sendid, transparent)
    //   _event.data        = env.data payload
    // This wiring keeps the parent's existing local-invoke finalize/autoforward
    // code paths (`childSession.sessionId == meta.origin` match) unchanged.
    case PatternKind::ChildEvent: {
        // SCE_MESH.md §mesh-9.6.4 steps 1-2 and its closing rule. This is the
        // arrival point ("ChildEvent arrives at parent's transport layer"),
        // and enqueueing here is what puts the event on the parent's external
        // queue for the next macrostep. Step 4 — identifying the `<invoke>`
        // the event belongs to — is resolved before the enqueue rather than
        // after it: once the parent has left the invoking state the section
        // requires the event to be "discarded silently (finalize not
        // executed, transition not considered)", and an event already sitting
        // in the queue would still be offered to transition selection.
        // Gating on arrival, not on selection, is also what keeps a wire-16
        // that landed *before* the exit eligible — `activeInvokes_` survives
        // until the invoking state's onexit precisely so those still match.
        if (!env.child_session_id ||
            !detail::childSessionIsActive(*env.child_session_id, engine, detail::HasActiveChildSessionHook<Engine>{})) {
            return false;
        }
        auto ev = Policy::getEventFromName(env.type.c_str());
        if (!ev) {
            return false;
        }
        typename Engine::EventWithMetadata meta;
        meta.event = *ev;
        meta.data = std::string(env.data.begin(), env.data.end());
        meta.type = "external";
        meta.originType = SCE::Constants::SCXML_EVENT_PROCESSOR_TYPE;
        // §mesh-9.6.3 L1463-1466 overrides the generic §mesh-10.7 origin row
        // for this arm: the parent matches `<finalize>` and autoforward by
        // comparing `activeInvokes_[id].sessionId == _event.origin`, and that
        // sessionId is the §mesh-9.6.1 child session id wire-15 delivered.
        // Rewriting this to `mesh://<source>` for table uniformity would stop
        // every remote finalize from matching, so the divergence is intended.
        if (env.child_session_id) {
            meta.origin = *env.child_session_id;
        }
        if (env.subject) {
            meta.sendId = *env.subject;
        }
        if (env.invoke_id) {
            meta.invokeId = SCE::uuid::to_string(*env.invoke_id);
        }
        engine.raiseExternal(std::move(meta));
        return true;
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
        engine.raiseExternal(std::move(meta));
        return true;
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
    }
    return false;  // unknown pattern kind — forward compatibility
}

}  // namespace SCE::Mesh
