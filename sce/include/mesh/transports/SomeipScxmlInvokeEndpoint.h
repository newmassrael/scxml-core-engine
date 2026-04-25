// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6.2 Session 4b — SOME/IP cross-device scxml-invoke endpoint.
//
// ─── Why a dedicated SCE-namespaced vsomeip application (RFC F.X-2) ────────
// This helper runs §9.6 cross-device <invoke type="scxml" src="#peer">
// traffic on a vsomeip application named `<machine>[_<partition>]_sce` that
// is SEPARATE from the per-`<send>`-target applications generated for
// ordinary SOME/IP method/event/field bindings. The split is the
// SCE-vs-OEM boundary, not a per-subsystem split inside SCE: every
// SCE-reserved subsystem on this binary (today §9.6 invoke; F.X-3
// onwards: region-liveness; possible future SCE subsystems) shares the
// single `<machine>[_<partition>]_sce` app. Three rationales survive
// consolidation under RFC F.X-2:
//
//   1. §13 OEM boundary protection. vsomeip.json `applications[*]` is OEM-
//      owned territory. SCE does not register SCE-reserved services
//      (0x8100..0x81FF, see SCXML_INVOKE_SERVICE_BASE) inside an OEM-
//      declared application. The SCE-named `<machine>[_<partition>]_sce`
//      keeps SCE's service registrations on an SCE-named application that
//      the OEM explicitly declares for that purpose, preserving the
//      bidirectional contract.
//   2. Failure isolation against `<send>` traffic. A §9.6 peer disconnect
//      or handler exception is contained inside the SCE app's callback
//      thread and cannot block the `<send>` SOME/IP path that may carry
//      safety-relevant traffic (e.g. brake control). vsomeip's
//      routing_manager dispatches per (application, service) tuple, so
//      the SCE app and each `<send>` target's app are isolated at the
//      routing layer. Inside the SCE app, sibling SCE-reserved subsystems
//      share one callback thread — defense-in-depth try-catch at the
//      vsomeip → SCE callback boundary (`invokeReceiveSafely` below)
//      prevents one subsystem's handler from starving siblings via an
//      escaped exception.
//   3. Service/instance ID responsibility split. SCE-reserved range
//      (0x8100..0x81FF) collision detection is SCE codegen's
//      responsibility (RFC F.X-1 hybrid allocator). OEM service ID
//      collision detection is OEM vsomeip.json's responsibility. The
//      dedicated SCE-namespaced app makes the boundary observable at the
//      routing layer — `(app, service)` tuple lookup naturally partitions
//      the two responsibility domains.
//
// Per-partition naming closes the latent collision where two partition
// binaries of the same machine would otherwise both call
// `create_application("<machine>_scxml_invoke")` and clash on vsomeip's
// routing-manager application-name uniqueness; with
// `<machine>_<partition>_sce` each partition binary registers a unique
// SCE app.
// ──────────────────────────────────────────────────────────────────────────
//
// Wire shape (SCE_MESH.md §9.6.2 Session 4b L1393):
//   * fireAndForget method calls (MT_REQUEST_NO_RETURN). Each §9.6 wire
//     direction is an independent method dispatch — wire-14 (P→C InvokeStart)
//     and wire-15 (C→P InvokeStarted) carry no request/response coupling at
//     the SOME/IP layer; the round-trip is two unrelated method calls.
//   * Payload: canonical CBOR MeshEnvelope (encodeEnvelope/decodeEnvelope).
//   * Service IDs: per-machine, produced at build time by the RFC F.X-1
//     hybrid (counter + optional author-pin) allocator in `sce-build` and
//     emitted as named constants (`SCE_SOMEIP_SERVICE_SELF`,
//     `SCE_SOMEIP_SERVICE_PEER_<peer_name>`) in the generated
//     `<machine>_transport.h`. 128-slot ceiling within sub-range
//     [0x8100, 0x817F]; deploy-time validator rejects overflow / pin out
//     of range / pin collision before codegen.
//   * Instance: 0x0001 (single-instance MVP — SOME/IP server pool support is
//     scoped to ordinary `<send>` paths via §14.4 Gap 7 and is not extended
//     to §9.6 endpoints in this session).
//   * Method IDs: hex digits encode the §9.6 wire number for vsomeip-trace
//     readability — wire-14 → 0x0014, wire-20 → 0x0020. NOT a numeric
//     identity with `PatternKind` enum values (0x0014 = 20 decimal, not
//     14); the mapping is `PatternKind::InvokeStart..InvokeError` →
//     `SCXML_INVOKE_METHOD_WIRE14..WIRE20` realised by `methodForPattern`,
//     and a static_assert below pins each pair so a future drift in
//     either constant fails at compile time.
//
// Threading model: vsomeip dispatches on its own callback thread. The
// receive callback installed via `setReceiveHandler` runs on that thread
// and must hand off to the engine via the mechanism codegen wires
// (parent: `dispatchToSession(env, 0)`, worker: `worker_session_host_`
// staging path). This helper does not interpret pattern semantics — same
// strategy-pattern split as Custom_tcp::Server (SCE_MESH.md §14.4
// callback-thread dispatch convention).
//
// Lifecycle: the helper does NOT own the vsomeip::application — codegen
// does (the per-machine `<machine>_scxml_invoke_app_` shared_ptr declared
// in TransportRouter). The helper only wires offer_service +
// register_message_handler + request_service for this particular peer.
// `start()` is the caller's responsibility (codegen's init() thread loop)
// so the same application can host multiple peers without spawning
// per-peer threads.

#pragma once

#include "core/LogMacros.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/PatternKind.h"

#include <exception>
#include <vsomeip/vsomeip.hpp>

#include <cstdint>
#include <functional>
#include <memory>
#include <string>
#include <string_view>
#include <utility>

namespace SCE::Mesh::Someip {

// ── SCE-reserved §9.6 namespace constants ───────────────────────────────────

/// Base of the SCE-reserved §9.6 scxml-invoke service ID range.
/// SCE-managed services occupy `[SCXML_INVOKE_SERVICE_BASE,
/// SCXML_INVOKE_SERVICE_BASE + 0x80)` (0x8100..0x817F) under RFC F.X-1
/// subsystem range partitioning; the upper half `[0x8180, 0x81FF]` is
/// reserved for the §16.4 region-liveness landing (RFC F.X-3). OEM
/// services are required to stay outside the full SCE-reserved
/// `[0x8100, 0x81FF]` range — collision is a deploy-time configuration
/// error, not a SCE-side hazard. Per-machine service IDs are produced
/// by the codegen-time hybrid (counter + optional author-pin) allocator
/// in `sce-build` and emitted as named constants
/// (`SCE_SOMEIP_SERVICE_SELF`, `SCE_SOMEIP_SERVICE_PEER_<peer_name>`)
/// in the generated `<machine>_transport.h`.
inline constexpr vsomeip::service_t SCXML_INVOKE_SERVICE_BASE =
    static_cast<vsomeip::service_t>(0x8100);

/// Inclusive ceiling of the §9.6 invoke sub-range under RFC F.X-1.
/// `[SCXML_INVOKE_SERVICE_BASE, SCXML_INVOKE_SERVICE_CEILING]` is the
/// allocator's output domain — 128 slots; the upper half of the
/// SCE-reserved range is reserved for §16.4 region-liveness.
inline constexpr vsomeip::service_t SCXML_INVOKE_SERVICE_CEILING =
    static_cast<vsomeip::service_t>(0x817F);

/// Single-instance MVP for §9.6 endpoints. Extending §9.6 to multi-
/// instance pool requires lifting §14.4 Gap 7 plumbing into the helper
/// here; not in this session (consumer-driven, no fixture demands it).
inline constexpr vsomeip::instance_t SCXML_INVOKE_INSTANCE_ID =
    static_cast<vsomeip::instance_t>(0x0001);

/// Per-wire method IDs. The low byte equals the SCE_MESH.md §9.6.2 wire
/// number (14..20), matching `PatternKind` enum values for InvokeStart..
/// InvokeError. Identity mapping keeps wire dumps human-readable.
inline constexpr vsomeip::method_t SCXML_INVOKE_METHOD_WIRE14_INVOKE_START   = 0x0014;
inline constexpr vsomeip::method_t SCXML_INVOKE_METHOD_WIRE15_INVOKE_STARTED = 0x0015;
inline constexpr vsomeip::method_t SCXML_INVOKE_METHOD_WIRE16_CHILD_EVENT    = 0x0016;
inline constexpr vsomeip::method_t SCXML_INVOKE_METHOD_WIRE17_PARENT_EVENT   = 0x0017;
inline constexpr vsomeip::method_t SCXML_INVOKE_METHOD_WIRE18_INVOKE_DONE    = 0x0018;
inline constexpr vsomeip::method_t SCXML_INVOKE_METHOD_WIRE19_INVOKE_CANCEL  = 0x0019;
inline constexpr vsomeip::method_t SCXML_INVOKE_METHOD_WIRE20_INVOKE_ERROR   = 0x0020;

/// Maps a §9.6 wire pattern to its SOME/IP method ID. Returns 0 for any
/// `PatternKind` outside the wire-14..20 range — codegen filters before
/// this call, so the zero-return path is a defence-in-depth fallback,
/// not an expected runtime branch.
constexpr vsomeip::method_t methodForPattern(PatternKind pattern) noexcept {
    switch (pattern) {
        case PatternKind::InvokeStart:   return SCXML_INVOKE_METHOD_WIRE14_INVOKE_START;
        case PatternKind::InvokeStarted: return SCXML_INVOKE_METHOD_WIRE15_INVOKE_STARTED;
        case PatternKind::ChildEvent:    return SCXML_INVOKE_METHOD_WIRE16_CHILD_EVENT;
        case PatternKind::ParentEvent:   return SCXML_INVOKE_METHOD_WIRE17_PARENT_EVENT;
        case PatternKind::InvokeDone:    return SCXML_INVOKE_METHOD_WIRE18_INVOKE_DONE;
        case PatternKind::InvokeCancel:  return SCXML_INVOKE_METHOD_WIRE19_INVOKE_CANCEL;
        case PatternKind::InvokeError:   return SCXML_INVOKE_METHOD_WIRE20_INVOKE_ERROR;
        default: return static_cast<vsomeip::method_t>(0);
    }
}

// ── Drift guards ────────────────────────────────────────────────────────────
// `PatternKind` enum values (decimal 14..20) and the SOME/IP method IDs
// (hex 0x0014..0x0020) are intentionally NOT numerically equal — the hex
// digits replicate the wire number for vsomeip-trace readability, not for
// arithmetic identity. These static_asserts pin every pair in the
// switch above so a future drift in either constant fails at compile time
// instead of producing a silently mis-routed envelope.
static_assert(methodForPattern(PatternKind::InvokeStart)   == SCXML_INVOKE_METHOD_WIRE14_INVOKE_START);
static_assert(methodForPattern(PatternKind::InvokeStarted) == SCXML_INVOKE_METHOD_WIRE15_INVOKE_STARTED);
static_assert(methodForPattern(PatternKind::ChildEvent)    == SCXML_INVOKE_METHOD_WIRE16_CHILD_EVENT);
static_assert(methodForPattern(PatternKind::ParentEvent)   == SCXML_INVOKE_METHOD_WIRE17_PARENT_EVENT);
static_assert(methodForPattern(PatternKind::InvokeDone)    == SCXML_INVOKE_METHOD_WIRE18_INVOKE_DONE);
static_assert(methodForPattern(PatternKind::InvokeCancel)  == SCXML_INVOKE_METHOD_WIRE19_INVOKE_CANCEL);
static_assert(methodForPattern(PatternKind::InvokeError)   == SCXML_INVOKE_METHOD_WIRE20_INVOKE_ERROR);
// Out-of-range pattern returns 0 — defence-in-depth contract; codegen
// filters before this call.
static_assert(methodForPattern(PatternKind::FireForget)         == 0);
static_assert(methodForPattern(PatternKind::ParallelRegionDone) == 0);

// ── ScxmlInvokeEndpoint ─────────────────────────────────────────────────────

/// Receive callback signature: invoked on a vsomeip callback thread once
/// per successfully decoded envelope. The callback owns dispatch into the
/// engine; this layer does no policy interpretation. Mirrors
/// `SCE::Mesh::CustomTcp::ReceiveCallback` (SCE_MESH.md §14.4
/// callback-thread dispatch convention).
using ReceiveCallback = std::function<void(const SCE::Mesh::MeshEnvelope&)>;

/// RFC F.X-2 D8: defense-in-depth try-catch at the vsomeip → SCE callback
/// boundary. The consolidated `<machine>[_<partition>]_sce` app hosts every
/// SCE-reserved subsystem's callbacks on a single vsomeip callback thread;
/// a callback that lets an exception escape would tear down that thread
/// and starve sibling subsystems. Catch all exceptions here, log via
/// `SCE_LOG_ERROR`, and return silently — vsomeip continues dispatching
/// subsequent envelopes unaffected.
///
/// Marked `noexcept` so the compiler enforces "no escape from this
/// function" at the boundary the caller (vsomeip's runtime thread) sees.
/// The body's catch-all guarantees noexcept by construction.
///
/// Free function (not a member) so the catch boundary is testable in
/// isolation without requiring a fully-constructed `ScxmlInvokeEndpoint`
/// or a vsomeip runtime — see the `invoke_receive_safely_*` unit tests.
inline void invokeReceiveSafely(const ReceiveCallback& on_receive,
                                const SCE::Mesh::MeshEnvelope& env) noexcept {
    if (!on_receive) return;
    try {
        on_receive(env);
    } catch (const std::exception& ex) {
        SCE_LOG_ERROR("§9.6 SOMEIP scxml-invoke handler exception: {}", ex.what());
    } catch (...) {
        SCE_LOG_ERROR("§9.6 SOMEIP scxml-invoke handler exception (unknown type)");
    }
}

/// Per-peer §9.6 SOME/IP endpoint. Offers the local machine's service so
/// the peer can dispatch wire envelopes to us (`offer_service` +
/// `register_message_handler`), and requests the peer's service so we can
/// dispatch wire envelopes to them (`request_service` + `send`).
///
/// The vsomeip::application is owned by codegen (one per machine,
/// `<machine>_scxml_invoke_app_`) and shared across all §9.6 peers on
/// that machine — see Q5 in the Session 4b design notes. The endpoint
/// holds a non-owning reference to the application; the caller MUST
/// keep the application alive for the endpoint's lifetime and call
/// `app->start()` exactly once after constructing all endpoints
/// (codegen does this on a dedicated thread inside `init()`).
///
/// Lifecycle ordering (caller responsibility, mirroring SOME/IP <send>
/// init sequence at template L2070+):
///   1. create_application + app->init()
///   2. construct ScxmlInvokeEndpoint(app, own_svc, peer_svc) for each peer
///   3. setReceiveHandler(...) for each endpoint
///   4. endpoint.start() for each endpoint (offer + request + register)
///   5. spawn thread running app->start()
///
/// Step 4 is split out from the constructor so callers can install the
/// receive handler before any inbound message can arrive.
class ScxmlInvokeEndpoint {
public:
    /// @param app       Shared vsomeip application for this machine (owned
    ///                  by codegen as `<machine>_scxml_invoke_app_`).
    /// @param own_svc   Service ID this endpoint OFFERS — read by codegen
    ///                  from the generated `SCE_SOMEIP_SERVICE_SELF`
    ///                  constant (RFC F.X-1 allocator output). Inbound
    ///                  messages from the peer arrive at
    ///                  `(own_svc, INSTANCE_ID, METHOD_WIRE_*)`.
    /// @param peer_svc  Service ID this endpoint REQUESTS — read by codegen
    ///                  from the generated `SCE_SOMEIP_SERVICE_PEER_<peer>`
    ///                  constant. Outbound messages dispatch to
    ///                  `(peer_svc, INSTANCE_ID, METHOD_WIRE_*)`.
    ///
    /// Argument order (`app`, `own_svc`, `peer_svc`) follows the offer-
    /// before-request lifecycle: `start()` calls `offer_service(own_svc)`
    /// before `request_service(peer_svc)`, so own_svc is the primary
    /// identity established by this endpoint.
    ScxmlInvokeEndpoint(std::shared_ptr<vsomeip::application> app,
                        vsomeip::service_t own_svc,
                        vsomeip::service_t peer_svc) noexcept
        : app_(std::move(app)), own_svc_(own_svc), peer_svc_(peer_svc) {}

    ~ScxmlInvokeEndpoint() = default;

    ScxmlInvokeEndpoint(const ScxmlInvokeEndpoint&) = delete;
    ScxmlInvokeEndpoint& operator=(const ScxmlInvokeEndpoint&) = delete;
    ScxmlInvokeEndpoint(ScxmlInvokeEndpoint&&) = delete;
    ScxmlInvokeEndpoint& operator=(ScxmlInvokeEndpoint&&) = delete;

    /// Install the receive handler. Invoked on a vsomeip callback thread
    /// per inbound (own_svc, INSTANCE_ID, METHOD_WIRE_*) message. Caller
    /// MUST install this BEFORE `start()` so no inbound envelope is lost
    /// to a missing handler.
    void setReceiveHandler(ReceiveCallback handler) {
        on_receive_ = std::move(handler);
    }

    /// Wire offer_service + per-wire register_message_handler for own_svc,
    /// and request_service for peer_svc. Idempotent — second call is a
    /// no-op (started_ guards). Returns false if the application is null
    /// or no receive handler was installed (caller programming error).
    [[nodiscard]] bool start() {
        if (!app_) return false;
        if (!on_receive_) return false;
        if (started_) return true;

        app_->offer_service(own_svc_, SCXML_INVOKE_INSTANCE_ID);

        // Per-wire receive handlers. Each METHOD_WIRE_* is a distinct
        // SOME/IP method, so register one handler per wire on own_svc.
        // The handler decodes the CBOR payload and forwards the envelope
        // to on_receive_ — pattern dispatch (parent vs worker) is the
        // caller's responsibility (codegen-injected lambda).
        registerWire(SCXML_INVOKE_METHOD_WIRE14_INVOKE_START);
        registerWire(SCXML_INVOKE_METHOD_WIRE15_INVOKE_STARTED);
        registerWire(SCXML_INVOKE_METHOD_WIRE16_CHILD_EVENT);
        registerWire(SCXML_INVOKE_METHOD_WIRE17_PARENT_EVENT);
        registerWire(SCXML_INVOKE_METHOD_WIRE18_INVOKE_DONE);
        registerWire(SCXML_INVOKE_METHOD_WIRE19_INVOKE_CANCEL);
        registerWire(SCXML_INVOKE_METHOD_WIRE20_INVOKE_ERROR);

        app_->request_service(peer_svc_, SCXML_INVOKE_INSTANCE_ID);

        started_ = true;
        return true;
    }

    /// Encode `env` and dispatch to peer_svc as a fireAndForget method
    /// call. Method ID is derived from `env.pattern` via
    /// `methodForPattern`; envelopes whose pattern is outside the
    /// wire-14..20 range are rejected (return false) — codegen never
    /// hands such envelopes to this endpoint, so the rejection is a
    /// defence-in-depth fallback.
    [[nodiscard]] bool send(const SCE::Mesh::MeshEnvelope& env) {
        if (!app_ || !started_) return false;
        const auto method = methodForPattern(env.pattern);
        if (method == 0) return false;

        auto request = vsomeip::runtime::get()->create_request();
        request->set_service(peer_svc_);
        request->set_instance(SCXML_INVOKE_INSTANCE_ID);
        request->set_method(method);
        // fireAndForget: response is not coupled to this dispatch. The
        // reverse-direction wire (e.g. wire-15 reply to wire-14) is an
        // independent method call from the peer back to us, not a
        // SOME/IP response. SCE_MESH.md §9.6.2 Session 4b L1393.
        request->set_message_type(vsomeip::message_type_e::MT_REQUEST_NO_RETURN);
        // vsomeip's `message_base_impl` defaults `is_reliable_` to false,
        // which routes outbound traffic to the UDP endpoint of the remote
        // service. §9.6 ScxmlInvokeEndpoint declares its service with a
        // `reliable.port` only (no UDP), so without this set_reliable(true)
        // the routing manager calls `find_or_create_remote_client(svc, inst,
        // false)`, finds no UDP endpoint in `remote_service_info_`, and
        // logs "Routing info for remote service could not be found!". The
        // 1-process Session 4b roundtrip happens to work without this flag
        // because in-process routing dispatches via local IPC regardless of
        // reliability, but the cross-host SD-driven path requires the TCP
        // selection here. Mirror the wire-14/15/18 framing requirement
        // (§9.6.2: "no resend protocol on staging queues") by always
        // demanding the reliable transport variant.
        request->set_reliable(true);

        auto buf = SCE::Mesh::encodeEnvelope(env);
        if (buf.empty()) return false;

        auto payload = vsomeip::runtime::get()->create_payload(
            reinterpret_cast<const vsomeip::byte_t*>(buf.data()),
            static_cast<uint32_t>(buf.size()));
        request->set_payload(payload);

        app_->send(request);
        return true;
    }

private:
    void registerWire(vsomeip::method_t method) {
        app_->register_message_handler(
            own_svc_, SCXML_INVOKE_INSTANCE_ID, method,
            [this](const std::shared_ptr<vsomeip::message>& msg) {
                auto payload = msg->get_payload();
                if (!payload) return;
                SCE::Mesh::MeshEnvelope env;
                if (!SCE::Mesh::decodeEnvelope(
                        payload->get_data(),
                        static_cast<std::size_t>(payload->get_length()),
                        env)) {
                    return;  // malformed envelope — drop, defence-in-depth
                }
                // RFC F.X-2 D8: catch any exception at the vsomeip → SCE
                // callback boundary. The consolidated SCE app hosts every
                // SCE-reserved subsystem's callbacks on this thread; an
                // escaped exception would starve sibling subsystems.
                invokeReceiveSafely(on_receive_, env);
            });
    }

    std::shared_ptr<vsomeip::application> app_;
    vsomeip::service_t own_svc_;
    vsomeip::service_t peer_svc_;
    ReceiveCallback on_receive_;
    bool started_ = false;
};

}  // namespace SCE::Mesh::Someip
