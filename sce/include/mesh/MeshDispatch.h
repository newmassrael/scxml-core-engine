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
//                                FieldNotify, FieldRead, FieldWrite, InvokeError
//   Outbound-only (reject):      EventSubscribe, EventUnsubscribe, InvokeStart
//
// SCE_MESH.md §9.6.2 wire 14 (`InvokeStart`) is handled by TransportRouter's
// inbound path before reaching this helper — the worker-side transport answers
// with a wire-20 InvokeError inline without going through engine dispatch, so
// a wire-14 envelope arriving here means the upstream branch did not catch it
// and we drop fail-closed (same shape as the EventSubscribe echo guard).
//
// SCE_MESH.md §9.6.2 wire 20 (`InvokeError`): parent-side receiver. The
// envelope carries `invoke_id`, `rpc_status`, and `rpc_error_message`; this
// dispatch raises `error.execution` on the parent engine with the
// `rpc_error_message` carried through `EventWithMetadata::data` so authors'
// `<transition event="error.execution">` observes the same raise shape as the
// transport-absent local fallback (SCE_MESH.md §9.6 line 1396).
//
// FieldRead/FieldWrite are inbound on the server role (SCE_MESH.md §8.3):
// the server's queryable / `register_message_handler` receives the
// getter/setter request and dispatches it to the engine, which fires the
// matching `<transition event="field.get.X">` / `<transition event="field.set.X">`.
//
// RpcRequest is inbound on the receiver side: the sender's
// `<invoke type="sce:mesh-rpc">` (SCE_MESH.md §9.5) emits an envelope whose
// `type` is the request event name (e.g. `service.request.compute_force`),
// which is enqueued on the receiver engine just like any FireForget event.
// The correlation UUID in `env.invoke_id` is surfaced to the receiver SCXML
// as `_event.invokeid` (W3C SCXML 5.10.1) through the engine's metadata
// pipeline — dispatchEnvelope always constructs `Engine::EventWithMetadata`
// and calls the metadata overload so test doubles observe the same path
// production engines take (no SFINAE fallback that could silently diverge).
//
// Returns false for outbound-only patterns (misconfigured echo-back) and
// unknown pattern kinds (forward compatibility).

#pragma once

#include "common/Uuid.h"
#include "mesh/MeshEnvelope.h"

#include <string>
#include <type_traits>
#include <utility>

namespace SCE::Mesh {

namespace detail {

/// SFINAE probe: does `Engine` expose
/// `onParallelRegionDone(const MeshEnvelope&)`? Only generated SMs with
/// distributed `<parallel>` trackers emit the method; monolithic machines
/// don't, so the wire-21 arm falls through to "return false" (drop) for
/// them — the envelope would have arrived in error anyway (no claimant
/// partition configured).
template <typename Engine, typename = void>
struct HasParallelRegionDoneHook : std::false_type {};

template <typename Engine>
struct HasParallelRegionDoneHook<Engine, std::void_t<decltype(
    std::declval<Engine&>().onParallelRegionDone(std::declval<const MeshEnvelope&>()))>>
    : std::true_type {};

template <typename Engine>
bool tryDeliverParallelRegionDone(const MeshEnvelope& env, Engine& engine,
                                  std::true_type /*has_hook*/) {
    engine.onParallelRegionDone(env);
    return true;
}

template <typename Engine>
bool tryDeliverParallelRegionDone(const MeshEnvelope& /*env*/, Engine& /*engine*/,
                                  std::false_type /*has_hook*/) {
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
/// SCE_MESH.md §16.5 `ParallelRegionDone` (wire 21): bypasses `raiseExternal`
/// and routes to `engine.onParallelRegionDone(env)` when present. Machines
/// without the hook drop the envelope — they could not have authored a
/// distributed `<parallel>` root that expected one.
///
/// @tparam Policy  Receiver's generated StatePolicy (provides getEventFromName)
/// @tparam Engine  StaticExecutionEngine<Policy>
/// @return true if the event was dispatched to the engine
template <typename Policy, typename Engine>
bool dispatchEnvelope(const MeshEnvelope& env, Engine& engine) {
    if (env.pattern == PatternKind::ParallelRegionDone) {
        return detail::tryDeliverParallelRegionDone(
            env, engine, detail::HasParallelRegionDoneHook<Engine>{});
    }
    switch (env.pattern) {
    case PatternKind::FireForget:
    // Inbound notifications are unidirectional — same delivery as FireForget.
    // The pattern discriminator enables transport-level routing (e.g. SOME/IP
    // event group subscription) but the engine dispatch is identical.
    case PatternKind::EventNotify:
    case PatternKind::FieldNotify:
    // SCE_MESH.md §9.5 request: sender's <invoke type="sce:mesh-rpc"> emits an
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
    // on the server-side engine (SCE_MESH.md §8.3).
    case PatternKind::FieldRead:
    case PatternKind::FieldWrite: {
        auto ev = Policy::getEventFromName(env.type.c_str());
        if (!ev) return false;
        typename Engine::EventWithMetadata meta;
        meta.event = *ev;
        meta.data = std::string(env.data.begin(), env.data.end());
        if (env.invoke_id) {
            meta.invokeId = SCE::uuid::to_string(*env.invoke_id);
        }
        engine.raiseExternal(std::move(meta));
        return true;
    }
    // SCE_MESH.md §9.6.2 wire 20 (InvokeError): parent receives the child's
    // "session F not implemented" (or any future invoke-lifecycle error) and
    // translates it into a local `error.execution` raise. The reason text
    // lives in `rpc_error_message`; we surface it via
    // `EventWithMetadata::data` so authors read the same payload shape the
    // transport-absent local raise produces (both paths currently embed the
    // reason in text; the structured `_event.data.reason` JSON is a separate
    // landing at §10.7.1 once an SCXML consumer exists).
    case PatternKind::InvokeError: {
        auto ev = Policy::getEventFromName("error.execution");
        if (!ev) return false;
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
    // ParallelRegionDone is handled by the pre-switch early-return above;
    // listing it here keeps `-Wswitch-enum` exhaustive. Reaching this arm
    // would be a compiler / codegen bug and we fail-closed with drop.
    case PatternKind::ParallelRegionDone:
        return false;
    // SCE_MESH.md §9.6.2 wire 14 (InvokeStart): caught upstream by
    // TransportRouter's inbound branch (it emits the wire-20 InvokeError
    // response inline). If a wire-14 envelope reaches dispatchEnvelope it
    // means the upstream branch missed it — fail-closed drop, same shape as
    // the EventSubscribe echo guard above.
    case PatternKind::InvokeStart:
        return false;
    }
    return false;  // unknown pattern kind — forward compatibility
}

}  // namespace SCE::Mesh
