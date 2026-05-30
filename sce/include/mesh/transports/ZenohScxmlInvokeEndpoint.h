// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §mesh-9.6.2 Session 5 — Zenoh cross-device scxml-invoke endpoint.
//
// ─── Why a shared zenoh::Session (Zenoh diverges from SOME/IP here) ────────
// §mesh-9.6 cross-device `<invoke type="scxml" src="#peer">` traffic on Zenoh
// rides the SAME `zenoh_session_` that ordinary `<send>` zenoh targets use,
// not a dedicated one. The Session 4b SOME/IP rationale for a dedicated
// `<machine>_scxml_invoke_app_` does NOT carry over:
//
//   1. No §mesh-13 OEM boundary on the Zenoh side. vsomeip.json `applications[*]`
//      is OEM-owned territory; Zenoh has no equivalent OEM-allocated
//      identifier whose registration must stay outside SCE-named spaces.
//      The SCE-reserved §mesh-9.6 namespace is carved out via key-expression
//      prefix (`SCXML_INVOKE_KEY_PREFIX = "sce/scxml_invoke"`), not
//      via session identity, so an OEM zenoh.json5 cannot collide with
//      §mesh-9.6 traffic regardless of which session carries it.
//   2. No 128-ID counter or service_id collision domain. SOME/IP's
//      RFC F.X-1 hybrid (counter + author-pin) allocator carves out a
//      bounded sub-range [0x8100, 0x817F] whose ceiling is observable
//      on its own routing tuple; Zenoh routes by full key-expression
//      with no analogous bounded namespace, so the parallel allocator
//      is unnecessary.
//      Method ID BCD-style `methodForPattern` mapping is also absent —
//      direction is encoded in the key-expression (`p2c/...` vs
//      `c2p/...`), and pattern dispatch happens off the CBOR
//      `MeshEnvelope::pattern` field on receive.
//   3. Failure isolation. A §mesh-9.6 peer disconnect or handler exception
//      surfaces on the Zenoh runtime callback thread that already
//      handles `<send>` traffic. Sharing the session here matches the
//      existing `zenoh_subscribers_` map, which also dispatches on the
//      same callback thread without isolating per-pattern.
//
// Reliability: §mesh-9.6 wire-14/18 lifecycle is reliable-required (a dropped
// wire-14 means the invoke never starts; a dropped wire-18 leaves the
// parent waiting indefinitely on `done.invoke.<id>`). The Publisher is
// declared with `CongestionControl::Block` (default for
// `declare_publisher` is push-default which may or may not be Block in
// the C ABI; we set it explicitly so a C-side default flip cannot
// silently downgrade §mesh-9.6 reliability) and `Priority::Data`. Zenoh's
// reliable transport for TCP-backed sessions then guarantees in-order
// delivery to a connected subscriber; the runtime DedupRouter still
// runs on receive because the Zenoh router fabric may reorder envelopes
// across multi-hop peers (transport.rs `supplies_dedup: false`).
//
// Wire shape (SCE_MESH.md §mesh-9.6.2 Session 5):
//   * Per-direction key-expression. Each peer has TWO keys:
//       - P2C: parent → child (wire-14 InvokeStart, wire-17 ParentEvent,
//         wire-19 InvokeCancel)
//       - C2P: child → parent (wire-15 InvokeStarted, wire-16 ChildEvent,
//         wire-18 InvokeDone, wire-20 InvokeError)
//     The endpoint declares ONE Publisher on its own-direction key and
//     ONE Subscriber on the peer-direction key. Pattern discrimination
//     happens off `MeshEnvelope::pattern` in the receive callback, not
//     off the key-expression — same shape as `<send>` zenoh path which
//     uses one key per binding and pattern-dispatches on receive.
//   * Payload: canonical CBOR MeshEnvelope (encodeEnvelope/decodeEnvelope).
//
// Threading model: Zenoh dispatches on its runtime callback thread. The
// receive callback installed via `setReceiveHandler` runs on that
// thread and must hand off to the engine via the mechanism codegen
// wires (parent: `dispatchToSession(env, 0)`, worker: per-peer staging
// queue + pump-thread drain). Same strategy-pattern split as
// `SomeipScxmlInvokeEndpoint` (SCE_MESH.md §mesh-14.4 callback-thread
// dispatch convention).
//
// Lifecycle: the helper does NOT own the `zenoh::Session` — codegen
// owns it (the device-shared `zenoh_session_` declared in
// TransportRouter). The helper holds a reference to it and declares
// its Publisher + Subscriber on `start()`. Construction must run AFTER
// `zenoh_session_.emplace(...)` so the reference target exists; codegen
// stores the endpoint as `std::optional<ScxmlInvokeEndpoint>` and
// `emplace`s it inside the same `init()` `try { ... }` block as the
// session opens, mirroring how `<send>`-target zenoh publishers are
// declared (`{{ target.target_snake }}_publisher_.emplace(...)`).
//
// `start()` is the caller's responsibility (codegen's `init()`) so the
// receive handler can be installed before any inbound sample can
// arrive.

#pragma once

#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <zenoh.hxx>

#include <functional>
#include <optional>
#include <string>
#include <string_view>
#include <utility>

namespace SCE::Mesh::Zenoh {

// ── SCE-reserved §mesh-9.6 namespace constants ───────────────────────────────────

/// SCE-reserved §mesh-9.6 scxml-invoke key-expression prefix. All §mesh-9.6
/// cross-device traffic over Zenoh travels under
/// `sce/scxml_invoke/...`; SCE-managed namespaces stay disjoint from
/// any author-supplied `<send>` zenoh `key:` value (which by SCE
/// convention does not begin with `sce/`). A future deploy-time
/// validator can grep author keys for this prefix to reject §mesh-9.6
/// reservation collisions.
inline constexpr std::string_view SCXML_INVOKE_KEY_PREFIX = "sce/scxml_invoke";

/// Build the parent→child key for a (parent, child) machine pair.
/// First argument is the source (parent) machine, second is the
/// destination (child). Pattern: `sce/scxml_invoke/p2c/<parent>/<child>`.
///
/// Runtime concat (not constexpr) — `std::string` concatenation is the
/// straightforward path here, and codegen emits string literals into
/// the ctor initializer list so the runtime cost is the once-per-peer
/// startup cost, not per-envelope. A `std::string_view`-based compile-
/// time concat would require a `fixed_string` template, which is
/// unnecessary today (no consumer demands compile-time keys) and
/// rejected as premature.
inline std::string keyExprP2C(std::string_view parent_machine,
                              std::string_view child_machine) {
    std::string out;
    out.reserve(SCXML_INVOKE_KEY_PREFIX.size() + 5 +
                parent_machine.size() + child_machine.size());
    out.append(SCXML_INVOKE_KEY_PREFIX);
    out.append("/p2c/");
    out.append(parent_machine);
    out.push_back('/');
    out.append(child_machine);
    return out;
}

/// Build the child→parent key for a (child, parent) machine pair.
/// First argument is the source (child) machine, second is the
/// destination (parent). Pattern: `sce/scxml_invoke/c2p/<child>/<parent>`.
///
/// Direction-oriented naming (`<from>/<to>`) mirrors the SHM
/// fallthrough channel naming convention in the codegen template
/// (`/sce_p2c_<parent>_<child>` / `/sce_c2p_<child>_<parent>`) so a
/// reader switching between transports sees the same direction encoded
/// the same way.
inline std::string keyExprC2P(std::string_view child_machine,
                              std::string_view parent_machine) {
    std::string out;
    out.reserve(SCXML_INVOKE_KEY_PREFIX.size() + 5 +
                child_machine.size() + parent_machine.size());
    out.append(SCXML_INVOKE_KEY_PREFIX);
    out.append("/c2p/");
    out.append(child_machine);
    out.push_back('/');
    out.append(parent_machine);
    return out;
}

// ── ScxmlInvokeEndpoint ─────────────────────────────────────────────────────

/// Receive callback signature: invoked on a Zenoh runtime callback
/// thread once per successfully decoded envelope. The callback owns
/// dispatch into the engine; this layer does no policy interpretation.
/// Mirrors `SCE::Mesh::Someip::ScxmlInvokeEndpoint::ReceiveCallback`
/// and `SCE::Mesh::CustomTcp::ReceiveCallback` (SCE_MESH.md §mesh-14.4
/// callback-thread dispatch convention).
using ReceiveCallback = std::function<void(const SCE::Mesh::MeshEnvelope&)>;

/// Decode-error callback signature: invoked on the Zenoh runtime
/// callback thread once per inbound sample whose CBOR decode failed.
/// Codegen wires this to `raiseCommunicationError(ENVELOPE_CORRUPT,
/// transport="zenoh")` so the §mesh-16.7 row 4 catalog row fires at this
/// hand-written endpoint tier the same way codegen `decodeEnvelope`
/// sites do. The callback runs on the same thread the subscriber's
/// on_sample fires on; per §mesh-14.4 callback-thread dispatch, it MUST
/// be cheap and re-entrant.
using DecodeErrorCallback = std::function<void()>;

/// Per-peer §mesh-9.6 Zenoh endpoint. Declares ONE Publisher on the
/// own-direction key (so we can publish wires the local role emits)
/// and ONE Subscriber on the peer-direction key (so we receive wires
/// the peer emits). Pattern discrimination happens off
/// `MeshEnvelope::pattern` in the receive callback — codegen-installed.
///
/// The `zenoh::Session` is owned by codegen (one per device,
/// `zenoh_session_`) and shared across every Zenoh peer + every
/// `<send>` zenoh target. The endpoint holds a reference to it; the
/// caller MUST keep the session alive for the endpoint's lifetime.
///
/// Lifecycle ordering (caller responsibility):
///   1. zenoh_session_.emplace(zenoh::Session::open(...))
///   2. construct ScxmlInvokeEndpoint(session_ref, own_pub_key, peer_sub_key)
///   3. setReceiveHandler(...)
///   4. endpoint.start() — declare_publisher + declare_subscriber
///
/// Steps 2 + 3 + 4 run inside the codegen `init()` `try { ... }
/// catch (zenoh::ZException&)` block so a failed declare propagates
/// to `init()` returning false (same shape as `<send>` zenoh
/// `declare_publisher` error-handling at template L2456+).
class ScxmlInvokeEndpoint {
public:
    /// @param session       Reference to the device-shared `zenoh::Session`.
    ///                      Caller (codegen) owns lifetime.
    /// @param own_pub_key   Key-expression this endpoint PUBLISHES on
    ///                      (assembled by `keyExprP2C` / `keyExprC2P`
    ///                      from the local machine's role for this peer).
    /// @param peer_sub_key  Key-expression this endpoint SUBSCRIBES to
    ///                      (assembled from the peer machine's role for
    ///                      this peer — opposite direction of own_pub_key).
    ///
    /// Argument order (own_pub before peer_sub) follows the publish-
    /// before-subscribe lifecycle: `start()` calls `declare_publisher`
    /// before `declare_subscriber`, so own_pub_key is the primary
    /// identity established by this endpoint.
    ScxmlInvokeEndpoint(zenoh::Session& session,
                        std::string own_pub_key,
                        std::string peer_sub_key) noexcept
        : session_(session),
          own_pub_key_(std::move(own_pub_key)),
          peer_sub_key_(std::move(peer_sub_key)) {}

    ~ScxmlInvokeEndpoint() = default;

    ScxmlInvokeEndpoint(const ScxmlInvokeEndpoint&) = delete;
    ScxmlInvokeEndpoint& operator=(const ScxmlInvokeEndpoint&) = delete;
    ScxmlInvokeEndpoint(ScxmlInvokeEndpoint&&) = delete;
    ScxmlInvokeEndpoint& operator=(ScxmlInvokeEndpoint&&) = delete;

    /// Install the receive handler. Invoked on a Zenoh runtime callback
    /// thread per inbound sample on `peer_sub_key_`. Caller MUST install
    /// this BEFORE `start()` so no inbound envelope is lost to a missing
    /// handler.
    void setReceiveHandler(ReceiveCallback handler) {
        on_receive_ = std::move(handler);
    }

    /// Install the decode-error handler (§mesh-16.7 row 4). Invoked on a
    /// Zenoh runtime callback thread per inbound sample whose CBOR
    /// decode fails. Caller wires this to
    /// `raiseCommunicationError(ENVELOPE_CORRUPT, transport="zenoh")`.
    /// May be left unset — in that case decode failures revert to the
    /// pre-row-4 silent-drop semantics, which is acceptable for
    /// fixtures that explicitly do not need the catalog row.
    void setDecodeErrorHandler(DecodeErrorCallback handler) {
        on_decode_error_ = std::move(handler);
    }

    /// Declare the Publisher on `own_pub_key_` (with reliability
    /// options pinned for §mesh-9.6 lifecycle traffic) and the Subscriber
    /// on `peer_sub_key_`. Idempotent — second call is a no-op
    /// (`started_` guards). Returns false if no receive handler was
    /// installed (caller programming error). Throws `zenoh::ZException`
    /// if the Zenoh runtime rejects either declaration; codegen
    /// surrounds the call with the same `try` block that wraps
    /// `zenoh::Session::open`.
    [[nodiscard]] bool start() {
        if (!on_receive_) return false;
        if (started_) return true;

        // Declare publisher with reliability options pinned for §mesh-9.6
        // lifecycle. `Z_CONGESTION_CONTROL_BLOCK` keeps publish calls
        // back-pressured against a slow subscriber rather than dropping
        // wire-14/18 envelopes — §mesh-9.6 cannot recover from a silent drop
        // because there is no resend protocol on the parent/child
        // staging queues. `Z_PRIORITY_DATA` is the default data-class
        // priority; raising to `_HIGH` would compete with author-side
        // `<send>` traffic on the same session and is rejected as
        // premature. The defaults come from
        // `::z_internal_congestion_control_default_push()` in zenoh-c
        // — we set explicitly so a future C-ABI default flip cannot
        // silently downgrade §mesh-9.6 reliability.
        ::zenoh::Session::PublisherOptions pub_opts =
            ::zenoh::Session::PublisherOptions::create_default();
        pub_opts.congestion_control = Z_CONGESTION_CONTROL_BLOCK;
        pub_opts.priority = Z_PRIORITY_DATA;
        publisher_.emplace(session_.declare_publisher(
            ::zenoh::KeyExpr(own_pub_key_), std::move(pub_opts)));

        // Declare subscriber with on_sample + on_drop closures. The
        // on_sample closure decodes the CBOR payload and forwards the
        // envelope to `on_receive_`; pattern dispatch (parent vs
        // worker) is the caller's responsibility (codegen-injected
        // lambda). The on_drop closure runs once when this endpoint's
        // subscriber handle is destroyed (router shutdown / endpoint
        // dtor) — no per-sample state to unwind, mirroring the
        // `[]() noexcept {}` on_drop the `<send>` EventSubscribe path
        // installs at template L4308.
        subscriber_.emplace(session_.declare_subscriber(
            ::zenoh::KeyExpr(peer_sub_key_),
            [this](const ::zenoh::Sample& sample) {
                if (!on_receive_) return;
                auto bytes = sample.get_payload().as_vector();
                SCE::Mesh::MeshEnvelope env;
                if (!SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), env)) {
                    // §mesh-16.7 row 4: malformed CBOR. Surface via
                    // on_decode_error_ before dropping the sample so
                    // SCXML authors observe the catalog row instead
                    // of a silent drop.
                    if (on_decode_error_) on_decode_error_();
                    return;
                }
                on_receive_(env);
            },
            []() noexcept {}));

        started_ = true;
        return true;
    }

    /// Encode `env` and dispatch on `own_pub_key_` via the declared
    /// Publisher's `put`. The Publisher's pinned CongestionControl +
    /// Priority apply to every envelope; no per-call options are
    /// passed. Returns false if `start()` has not been called yet
    /// (defence-in-depth — codegen always starts before the first
    /// dispatch site fires) or if the envelope encodes to an empty
    /// buffer (malformed envelope, codec contract violation).
    [[nodiscard]] bool send(const SCE::Mesh::MeshEnvelope& env) {
        if (!started_ || !publisher_) return false;
        auto buf = SCE::Mesh::encodeEnvelope(env);
        if (buf.empty()) return false;
        publisher_->put(::zenoh::Bytes(std::move(buf)));
        return true;
    }

private:
    ::zenoh::Session& session_;
    std::string own_pub_key_;
    std::string peer_sub_key_;
    ReceiveCallback on_receive_;
    DecodeErrorCallback on_decode_error_;
    std::optional<::zenoh::Publisher> publisher_;
    std::optional<::zenoh::Subscriber<void>> subscriber_;
    bool started_ = false;
};

}  // namespace SCE::Mesh::Zenoh
