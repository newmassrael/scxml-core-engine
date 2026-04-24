// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh transport registry — single source of truth for transport metadata.
//
// Each transport declares its codegen shape (field layout in TransportRouter)
// and communication capabilities (pattern validation) in ONE entry.
//
// Adding a new transport requires exactly TWO changes:
//   1. Add one entry to `lookup()` below   (Rust — shape + capabilities)
//   2. Add {% elif %} blocks in mesh_transport.h.jinja2  (C++ codegen)
// The template's `#error` fallback catches (2) drift at C++ compile time.
//
// Per-transport scxml-invoke codegen helpers (SCE_MESH.md §9.6 L1399 (b))
// live in sibling submodules: resolve_connect_endpoint and future
// per-peer resolvers stay off this registry so the descriptor layer
// does not re-grow inline per-transport branches.

pub mod custom_tcp;
pub mod shm;
pub mod someip;

use std::fmt;

// ── Codegen shape ───────────────────────────────────────────

/// Describes how a transport's C++ router fields are laid out in
/// TransportRouter. Separates per-target state (local engine reference,
/// SHM channel, SOME/IP application) from device-shared state
/// (Zenoh session).
///
/// The template consumes these flags (via `TargetContext`) to decide
/// whether to emit a per-target field declaration and matching
/// constructor initializer for each target, without hardcoding transport
/// names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportShape {
    /// Does this transport emit a per-target field in TransportRouter
    /// (and a matching entry in the constructor initializer list)?
    ///
    /// `true` for local/shm/someip (each target has its own channel/app,
    /// constructed via reference or ctor-initializer). `false` for zenoh
    /// (all targets share one Session, constructed in `init()` after the
    /// TransportRouter is already live).
    pub has_per_target_field: bool,
    /// Does this transport use a device-shared session resource?
    /// `true` for zenoh. The template emits the shared field once per
    /// transport (not per target) and initializes it in `init()`.
    pub has_shared_session: bool,
}

// ── Communication capabilities ──────────────────────────────

/// Transport capability categories (SCE_MESH.md Section 8.2).
///
/// Each transport declares which capability categories it supports.
/// Pattern validation checks that the detected pattern's required
/// capability is in the transport's supported set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportCapability {
    /// Request/Response: method call, RPC, service invocation.
    RequestReply,
    /// Fire-and-forget: one-way send. Supported by all transports.
    FireForget,
    /// Pub/Sub: topic subscription and event notification.
    PubSub,
    /// Field access: named data field read/write.
    FieldAccess,
}

impl fmt::Display for TransportCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestReply => write!(f, "request/reply"),
            Self::FireForget => write!(f, "fire-and-forget"),
            Self::PubSub => write!(f, "pub/sub"),
            Self::FieldAccess => write!(f, "field access"),
        }
    }
}

// ── Unified descriptor ──────────────────────────────────────

/// Complete metadata for a known transport. Codegen reads `shape`;
/// pattern validation reads `capabilities`. Both come from the same
/// `lookup()` entry — no possibility of drift.
pub struct TransportDescriptor {
    /// C++ TransportRouter field layout.
    pub shape: TransportShape,
    /// Communication patterns this transport supports.
    pub capabilities: &'static [TransportCapability],
    /// Does the Jinja2 template have `{% elif %}` blocks for this transport?
    ///
    /// `true` for transports with full codegen support (local, shm, someip,
    /// zenoh). `false` for transports whose capabilities are known (enabling
    /// pattern validation) but whose template has not been added yet (dds,
    /// can). Codegen rejects `implemented == false` at the Rust level —
    /// users get a clear build error instead of a deferred C++ `#error`.
    pub implemented: bool,
    /// Per-binding fields that deploy.yaml MUST provide for this transport.
    ///
    /// Validated at the Rust level (topology stage) before codegen. Without
    /// this, missing fields would only surface as C++ `#error` directives
    /// in the generated template — a two-stage failure that gives users a
    /// cryptic compiler error instead of a clear deploy.yaml diagnostic.
    ///
    /// Empty for transports with no required per-binding config (local, shm).
    pub required_binding_fields: &'static [&'static str],
    /// Does this transport inherently suppress envelope duplicates?
    ///
    /// Consumed by the codegen's `dispatchToSender` branching (SCE_MESH.md
    /// §10.5): when every transport on a receiver sets this `true`, the
    /// generated `TransportRouter` omits the runtime `DedupRouter` member
    /// entirely. When at least one is `false`, the receiver emits a
    /// `DedupRouter` and every inbound call site funnels through
    /// `admitEnvelope(env)` before reaching the engine.
    ///
    /// Classification rationale:
    /// - in-process queueing (local, shm) cannot duplicate — no wire
    /// - single-stream TCP (custom_tcp, SOME/IP over TCP default) is
    ///   duplicate-free by design
    /// - Zenoh's reliable mode can still reorder across routers, so
    ///   application-level dedup runs
    /// - multicast bus transports (dds, can) have no single source
    ///   ordering
    pub supplies_dedup: bool,
    /// Does this transport inherently deliver per-(source, target)
    /// envelopes in send order?
    ///
    /// Consumed by the codegen's ordering branching (SCE_MESH.md §10.6):
    /// when a binding declares `ordering: required` AND this flag is
    /// `false`, the generated `TransportRouter` emits an
    /// `OrderingBuffer` member and routes inbound envelopes through
    /// `admitOrdered`. When `true`, the binding's `ordering` value is
    /// a no-op — the transport's native FIFO already holds.
    ///
    /// Classification rationale:
    /// - in-process queueing + single-stream TCP preserve send order by
    ///   construction (local, shm, custom_tcp — same set as supplies_dedup)
    /// - UDP-capable transports (someip default, zenoh, dds) may reorder
    ///   across routers or datagrams
    /// - CAN frame arbitration is priority-based, not sender-FIFO
    pub supplies_ordering: bool,
    /// Can a receiver, given a sender-stamped per-(source, target)
    /// `sequence_no`, reconstruct send order?
    ///
    /// `true` for every transport with a point-to-point delivery model
    /// (sender → receiver pair has a private sequence domain). `false`
    /// for broadcast buses where every frame reaches every participant:
    /// CAN has no per-receiver sequence domain because the sender's
    /// counter stream is observed identically by every ECU on the bus,
    /// so the runtime `OrderingBuffer` cannot distinguish "skipped for
    /// this receiver" from "not destined for this receiver".
    ///
    /// Topology validation (SCE_MESH.md §10.6.2) rejects a binding
    /// declaring `ordering: required` on a transport whose
    /// `ordering_representable` is `false`.
    pub ordering_representable: bool,
    /// Does this transport carry a native routing layer that can
    /// substitute runtime values into a binding's address (SCE_MESH.md
    /// §14.4)?
    ///
    /// `true` for transports whose native stack delivers an envelope to
    /// whatever peer matches a given address without SCE maintaining a
    /// peer table:
    ///   - Zenoh: `session.put` / `session.get` dispatch on KeyExpr; the
    ///     address may be assembled at runtime from a
    ///     placeholder-bearing `key:` value.
    ///   - SOME/IP: vsomeip's routing manager delivers to any instance
    ///     whose availability has been registered. Because
    ///     `request_service(SERVICE, ANY_INSTANCE)` is interpreted as
    ///     specific-instance-0xFFFF (not a wildcard), SOME/IP's pool
    ///     support is *bounded* — the binding must declare an explicit
    ///     `instances:` list so codegen can emit one `request_service`
    ///     per member at `init()`.
    ///
    /// `false` for transports with no routing-layer substitution:
    ///   - local / shm: endpoints are compile-time process addresses.
    ///   - custom_tcp: `connect:` / `listen:` are single static TCP
    ///     endpoints. Adding a pool would require SCE to implement its
    ///     own SD protocol, which the §3.3 design invariant explicitly
    ///     rejects.
    ///   - can: broadcast bus with no peer-level addressing.
    ///
    /// A binding value field containing a `{name}` placeholder token is
    /// rejected at the topology stage when the transport declares
    /// `supports_pool: false`
    /// (`mesh/pool-not-supported-by-transport`).
    pub supports_pool: bool,
    /// Is the deploy.yaml `machines.<name>.subscriptions:` (SCE_MESH.md
    /// §13 machine-lifetime path) realised end-to-end on this transport?
    ///
    /// Distinct from the general `PubSub` capability: a transport may
    /// support pub/sub for SCXML-driven `<send event="event.subscribe.X">`
    /// (which flows through per-event resolved metadata —
    /// `SomeipEventIds::EventGroup`, Zenoh key) while *not* supporting
    /// the machine-lifetime synthesis path, which dispatches from
    /// deploy.yaml-only information with no per-event external
    /// resolution.
    ///
    /// Classification rationale:
    /// - `local` / `shm` / `custom_tcp`: no pub/sub capability at all.
    ///   Already rejected earlier; set `false` for structural
    ///   consistency.
    /// - `someip`: pub/sub works through SCXML `<send>` (template emits
    ///   `request_event` + `subscribe` from per-event event_group_id
    ///   resolved via vsomeip.json), but machine-lifetime synthesis
    ///   never consults vsomeip.json — it has no event_group_id to
    ///   emit, so the subscribe would land in the "unknown event" arm
    ///   and silently drop. Tracked under
    ///   `mesh_someip_sd_gaps_roadmap.md`.
    /// - `zenoh`: binding-wide `key:` is sufficient for subscribe
    ///   dispatch — no per-event resolution needed. `true`.
    /// - `dds` / `can`: unimplemented; set `false`.
    ///
    /// Consumed by the topology-stage contributor
    /// (`topology::contribute_subscription_partials`): a subscription source whose
    /// binding transport has this set to `false` is rejected with
    /// `mesh/topology-machine-lifetime-subscription-unsupported`.
    pub supports_machine_lifetime_subscribe: bool,
    /// Can this transport host an SCE machine as a multi-instance
    /// server? (SCE_MESH.md §14.4 multi-instance server pool, Gap 7.)
    ///
    /// `true` for transports whose native routing layer delivers an
    /// inbound message tagged with a peer-identifying instance
    /// dimension, so the generated TransportRouter can dispatch to
    /// per-instance SCXML sessions:
    ///   - SOME/IP: `msg->get_instance()` returns the `instance_id`
    ///     vsomeip assigned to the inbound request; one
    ///     `offer_service(SERVICE, i)` per declared instance + one
    ///     `register_message_handler` per (instance, method) pair
    ///     realises the pool end-to-end.
    ///
    /// `false` for transports without a peer-level inbound
    /// distinguisher:
    ///   - Zenoh: a KeyExpr identifies a *subject* not a *peer*; there
    ///     is no server-side inbound attribute that distinguishes one
    ///     hosted instance from another. SCE_MESH.md §14.4
    ///     multi-instance scope deliberately excludes Zenoh server
    ///     pools.
    ///   - local / shm / custom_tcp: endpoints are compile-time process
    ///     addresses; a single process cannot semantically back N
    ///     independent peer identities on the same in-process channel.
    ///   - dds / can: unimplemented; flag stays `false` for
    ///     consistency — the runtime has no session-pool scaffolding
    ///     for broadcast-bus shapes regardless.
    ///
    /// Consumed by `deploy::validate_server_pool_rejection`: a machine
    /// declaring `server.instances:` on a transport whose flag is
    /// `false` is rejected at parse time with the transport name in
    /// the diagnostic (`DeployError::ServerPoolNotSupported`).
    pub supports_multi_instance_server: bool,
    /// Can this transport carry inter-partition IPC traffic within a
    /// single machine (SCE_MESH.md §14 L2729-2730)?
    ///
    /// `partitions:` splits a machine across M OS processes; traffic
    /// between those processes flows over a transport chosen via
    /// `transport_binding:`. Only transports whose primary purpose is
    /// same-machine IPC qualify:
    ///
    /// - `shm` — shared-memory ring buffer per channel; canonical
    ///   same-machine IPC.
    /// - `custom_tcp` — TCP loopback with length-prefixed CBOR framing;
    ///   the `tcp` half of spec L2730's "kind tcp/shm" default pair.
    ///
    /// Every other transport is rejected:
    ///
    /// - `local` — in-process direct dispatch; cannot cross the OS
    ///   process boundary that a partition defines.
    /// - `someip` / `zenoh` — designed as inter-machine middleware /
    ///   fabric. Forcing partition IPC through them routes through a
    ///   daemon or routing fabric instead of the intended same-machine
    ///   channel; authors should pick `shm` or `custom_tcp` when the
    ///   traffic never leaves the device.
    /// - `dds` / `can` — unimplemented + non-IPC semantics (broadcast
    ///   bus, multi-participant DCPS).
    ///
    /// Consumed by `deploy::validate_partitions_schema`: a partition
    /// whose `transport_binding:` names a transport with this flag
    /// `false` is rejected at parse time with
    /// `MeshDeployPartitionTransportBindingUnsupported`.
    pub supports_inter_partition_ipc: bool,
}

// ── Single registry ─────────────────────────────────────────

/// Single source of truth for transport metadata.
///
/// Returns `None` for unknown transports. Codegen treats this as a build
/// error (`CodegenError::UnsupportedTransport`); pattern validation treats
/// it conservatively (validation skipped).
///
/// When adding a new transport, add one entry here and one `{% elif %}`
/// block in `mesh_transport.h.jinja2` (the template's `#error` catches
/// drift at C++ compile time).
pub fn lookup(transport: &str) -> Option<&'static TransportDescriptor> {
    use TransportCapability::*;

    static LOCAL: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: true, has_shared_session: false },
        capabilities: &[RequestReply, FireForget, PubSub, FieldAccess],
        implemented: true,
        required_binding_fields: &[],
        // In-process direct dispatch — no wire, no reordering.
        supplies_dedup: true,
        // Direct function call preserves invocation order.
        supplies_ordering: true,
        ordering_representable: true,
        // Same-process target identity is compile-time; no runtime
        // address substitution.
        supports_pool: false,
        // No pub/sub capability at all — machine-lifetime is moot.
        supports_machine_lifetime_subscribe: false,
        // In-process direct dispatch: one process hosts one identity
        // per machine; a second instance would be a second process.
        supports_multi_instance_server: false,
        // In-process direct dispatch cannot cross the OS process
        // boundary that `partitions:` defines (§14 L2729-2730).
        supports_inter_partition_ipc: false,
    };
    static SHM: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: true, has_shared_session: false },
        capabilities: &[FireForget, FieldAccess],
        implemented: true,
        required_binding_fields: &[],
        // Single shared ring per channel with READY_MAGIC gating — a
        // producer cannot publish the same slot twice.
        supplies_dedup: true,
        // FIFO ring buffer per channel preserves producer order.
        supplies_ordering: true,
        ordering_representable: true,
        // SHM channels are pre-declared in deploy.yaml; runtime values
        // cannot address a new channel.
        supports_pool: false,
        // No pub/sub capability; machine-lifetime path does not apply.
        supports_machine_lifetime_subscribe: false,
        // SHM channels are pre-declared and addressed by compile-time
        // names — no peer-identity inbound distinguisher.
        supports_multi_instance_server: false,
        // Shared-memory ring buffer per channel is the canonical
        // same-machine IPC mechanism (§14 L2730 "kind tcp/shm").
        supports_inter_partition_ipc: true,
    };
    static SOMEIP: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: true, has_shared_session: false },
        capabilities: &[RequestReply, FireForget, PubSub, FieldAccess],
        implemented: true,
        // SOME/IP identity (`service_id` + `instance_id`) and per-event IDs
        // are NOT verified via this generic `extra`-key presence check —
        // they live on typed `ResolvedTarget` fields (`someip_service`,
        // `event_bindings`) populated by `finalize_targets`, and
        // topology runs a typed check after that pass.
        required_binding_fields: &[],
        // SOME/IP cannot guarantee at-most-once delivery across its layers:
        // service discovery is UDP multicast (retransmits + bridged
        // segments), eventgroups default to UDP, and current in-tree
        // fixtures pick UDP for method calls too. Even per-binding
        // `protocol: tcp` does not cover the SD layer, so the descriptor
        // declares `false` across the board and receivers run the runtime
        // DedupWindow on every inbound SOME/IP envelope. If an all-TCP
        // SOME/IP profile with deduped SD ever lands, it deserves a
        // separate registry entry (e.g. `someip_tcp`) rather than a
        // per-binding probe here.
        supplies_dedup: false,
        // Default UDP datagrams may reorder; per-binding `protocol: tcp`
        // is a runtime upgrade handled by the codegen's
        // `compute_needs_ordering` helper, not by flipping this flag.
        // Same rationale as `supplies_dedup`.
        supplies_ordering: false,
        // Per-(service_id, method_id) or (service_id, event_group_id)
        // routing is point-to-point; receivers can track a sender-stamped
        // sequence per binding.
        ordering_representable: true,
        // Bounded pool: placeholder on `instance_id:` requires an
        // explicit `instances:` list because `request_service(SERVICE,
        // ANY_INSTANCE)` does not subscribe to every instance (vsomeip
        // routing_manager_impl treats 0xFFFF as a specific instance ID).
        // Codegen emits one `request_service(SERVICE, i)` per declared
        // instance at init().
        supports_pool: true,
        // SCXML-driven subscribes work (template emits request_event +
        // subscribe from vsomeip.json-resolved event_group_id), but the
        // machine-lifetime synthesis path has no external resolution
        // step — the subscribe envelope would hit the "unknown event"
        // arm and drop. Gap tracked under
        // `mesh_someip_sd_gaps_roadmap.md`.
        supports_machine_lifetime_subscribe: false,
        // SOME/IP's routing_manager tracks per-(service, instance) state
        // independently: `offer_service(SERVICE, i)` advertises one
        // instance, `register_message_handler(SERVICE, i, METHOD, ...)`
        // binds inbound to that instance, and `msg->get_instance()`
        // exposes the instance on dispatch. This is the sole registry
        // entry with `true` today (Gap 7).
        supports_multi_instance_server: true,
        // SOME/IP is an inter-machine middleware: traffic runs through
        // the vsomeip routing daemon and relies on service discovery.
        // Same-machine IPC via SOME/IP would route through that daemon
        // instead of the direct channel §14 intends; authors should
        // pick `shm` or `custom_tcp` for inter-partition traffic.
        supports_inter_partition_ipc: false,
    };
    static ZENOH: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: false, has_shared_session: true },
        // Zenoh supports RPC via queryable/query primitives — `session.get()`
        // against a `declare_queryable()` endpoint. Correlation is handled
        // natively by the Zenoh runtime (reply callbacks), so no per-router
        // correlation table is needed.
        capabilities: &[RequestReply, FireForget, PubSub, FieldAccess],
        implemented: true,
        required_binding_fields: &["key"],
        // Zenoh router reordering is visible to applications even in reliable
        // mode — the runtime-level DedupWindow filters re-delivered envelopes.
        supplies_dedup: false,
        // Same router-reorder concern applies to FIFO: reliable delivery
        // does not imply ordered delivery across a multi-hop Zenoh fabric.
        supplies_ordering: false,
        // Per-(key, subscriber) stream carries a private sequence domain.
        ordering_representable: true,
        // Open pool: KeyExpr substitution passes the assembled string to
        // Zenoh's native routing, which delivers to whichever peer has a
        // matching subscriber. No SCE-side enumeration required.
        supports_pool: true,
        // Binding-wide `key:` is sufficient for dispatch — no per-event
        // external resolution needed for subscribe. SCE_MESH.md §13
        // machine-lifetime path is fully wired end-to-end.
        supports_machine_lifetime_subscribe: true,
        // Zenoh KeyExpr identifies a subject, not a peer. There is no
        // server-side inbound distinguisher between two instances of
        // the same hosted machine (a subscriber/queryable on the same
        // key receives every request identically). SCE_MESH.md §14.4
        // multi-instance exclusion clause codifies this.
        supports_multi_instance_server: false,
        // Zenoh is an inter-machine fabric: traffic flows through a
        // routing layer that scouts peers across the network. Same-
        // machine IPC via Zenoh would route through that fabric
        // instead of the direct channel §14 intends; authors should
        // pick `shm` or `custom_tcp` for inter-partition traffic.
        supports_inter_partition_ipc: false,
    };
    // SCE Mesh §16.8.3 reference transport: TCP loopback, length-prefixed
    // CBOR envelope framing, zero external dependencies. Each binding has a
    // per-target client (its `connect:` endpoint) and the device exposes a
    // single `listen:` server (declared in `transports.custom_tcp.listen`),
    // so both shape flags apply: per-target client field + device-shared
    // server. FireForget only in this session — RPC/PubSub/FieldAccess
    // arrive with the dedup layer + duplex correlation in the next session.
    static CUSTOM_TCP: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: true, has_shared_session: true },
        capabilities: &[FireForget],
        implemented: true,
        required_binding_fields: &["connect"],
        // One TCP socket per sender→receiver pair; the stream itself
        // guarantees at-most-once delivery of any framed envelope.
        supplies_dedup: true,
        // TCP preserves byte stream order; the length-prefixed CBOR
        // framing layered on top preserves envelope order.
        supplies_ordering: true,
        ordering_representable: true,
        // No native routing layer. Endpoints are static in deploy.yaml;
        // adding a pool would require SCE to implement its own SD
        // protocol (§3.3 design invariant rejects middleware SD).
        supports_pool: false,
        // No pub/sub capability in this session — machine-lifetime is
        // moot. Dedup + duplex correlation for FireForget-only today.
        supports_machine_lifetime_subscribe: false,
        // Single static client→server TCP endpoint — no peer-identity
        // dimension on the inbound side.
        supports_multi_instance_server: false,
        // TCP loopback with length-prefixed CBOR framing is the `tcp`
        // half of spec L2730's "kind tcp/shm" default pair for
        // inter-partition traffic within the same machine.
        supports_inter_partition_ipc: true,
    };
    static DDS: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: true, has_shared_session: false },
        capabilities: &[FireForget, PubSub, FieldAccess],
        implemented: false,
        required_binding_fields: &[],
        // DDS BEST_EFFORT and multicast paths admit application-visible
        // duplicates; reliable-only deployments still need dedup for
        // cross-participant fan-out.
        supplies_dedup: false,
        // BEST_EFFORT can reorder; reliable multicast participants still
        // see late-join replay. Runtime buffer is required for
        // `ordering: required` bindings.
        supplies_ordering: false,
        // Per-(topic, reader) unicast delivery from each publisher
        // supports a stamped sequence domain.
        ordering_representable: true,
        // DCPS participant discovery is native. Gated on
        // `implemented: false` but the capability already exists.
        supports_pool: true,
        // Unimplemented; gated by `implemented: false` before this
        // flag is ever consulted. Consistent `false` keeps the
        // registry shape uniform.
        supports_machine_lifetime_subscribe: false,
        // DCPS participant discovery exists but the runtime has no
        // server-pool scaffolding; `implemented: false` gates this
        // before the flag is consulted.
        supports_multi_instance_server: false,
        // DDS is an inter-machine multi-participant middleware, not
        // a same-machine IPC channel. Unimplemented + out of §14's
        // IPC scope.
        supports_inter_partition_ipc: false,
    };
    static CAN: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: true, has_shared_session: false },
        capabilities: &[FireForget, FieldAccess],
        implemented: false,
        required_binding_fields: &[],
        // CAN is a broadcast bus — retransmits and bridged segments can
        // redeliver the same frame.
        supplies_dedup: false,
        // CAN frame arbitration is priority-based, not sender-FIFO; a
        // burst of high-priority frames can overtake a queued lower-
        // priority one from the same sender.
        supplies_ordering: false,
        // Broadcast bus — every frame reaches every participant, so a
        // sender-stamped per-receiver sequence has no meaning on the
        // wire. Topology rejects `ordering: required` for CAN bindings.
        ordering_representable: false,
        // CAN frame IDs are compile-time allocations; the bus has no
        // peer-level addressing for placeholder substitution.
        supports_pool: false,
        // Unimplemented + broadcast bus; machine-lifetime subscribe
        // semantic does not apply.
        supports_machine_lifetime_subscribe: false,
        // Broadcast bus — every frame reaches every participant; the
        // concept of a per-peer server instance does not map.
        supports_multi_instance_server: false,
        // CAN is a priority-arbitrated broadcast bus — not a direct
        // same-machine IPC channel.
        supports_inter_partition_ipc: false,
    };

    match transport {
        "local" => Some(&LOCAL),
        "shm" => Some(&SHM),
        "someip" => Some(&SOMEIP),
        "zenoh" => Some(&ZENOH),
        "custom_tcp" => Some(&CUSTOM_TCP),
        "dds" => Some(&DDS),
        "can" => Some(&CAN),
        _ => None,
    }
}

/// Check whether a transport supports a specific capability.
///
/// Unknown transports return `true` (conservative: validation skipped).
pub fn supports(transport: &str, capability: TransportCapability) -> bool {
    match lookup(transport) {
        Some(d) => d.capabilities.contains(&capability),
        None => true,
    }
}

/// Wire-facing list of currently-implemented transport names. Used by
/// diagnostic emission (`MeshCodegenUnsupportedTransport`) so upstream
/// agents receive a structured candidate list instead of having to
/// parse the error prose. Order matches the `lookup()` dispatch so
/// drift between the two is obvious in code review.
pub fn implemented_names() -> &'static [&'static str] {
    &["local", "shm", "someip", "zenoh", "custom_tcp"]
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use TransportCapability::*;

    // ── shape ───────────────────────────────────────────────

    #[test]
    fn local_is_per_target() {
        let d = lookup("local").expect("known");
        assert!(d.shape.has_per_target_field);
        assert!(!d.shape.has_shared_session);
    }

    #[test]
    fn someip_is_per_target() {
        let d = lookup("someip").expect("known");
        assert!(d.shape.has_per_target_field);
        assert!(!d.shape.has_shared_session);
    }

    #[test]
    fn zenoh_is_shared() {
        let d = lookup("zenoh").expect("known");
        assert!(!d.shape.has_per_target_field);
        assert!(d.shape.has_shared_session);
    }

    #[test]
    fn dds_is_per_target() {
        let d = lookup("dds").expect("known");
        assert!(d.shape.has_per_target_field);
        assert!(!d.shape.has_shared_session);
    }

    #[test]
    fn can_is_per_target() {
        let d = lookup("can").expect("known");
        assert!(d.shape.has_per_target_field);
        assert!(!d.shape.has_shared_session);
    }

    #[test]
    fn unknown_returns_none() {
        assert!(lookup("iceoryx2").is_none());
    }

    // ── implemented ─────────────────────────────────────────

    #[test]
    fn implemented_transports_have_template() {
        for name in &["local", "shm", "someip", "zenoh", "custom_tcp"] {
            assert!(
                lookup(name).unwrap().implemented,
                "transport '{name}' should be marked implemented"
            );
        }
    }

    #[test]
    fn custom_tcp_per_target_with_shared_server() {
        // §16.8.3 reference transport: per-binding `connect:` (client field)
        // + device-level `transports.custom_tcp.listen:` (shared server).
        let d = lookup("custom_tcp").expect("known");
        assert!(d.shape.has_per_target_field);
        assert!(d.shape.has_shared_session);
    }

    #[test]
    fn custom_tcp_fire_forget_only_in_session_e2_step1() {
        // Other patterns require dedup + duplex correlation, landing in
        // subsequent E2 sessions. The capability list documents the
        // current contract; widening must update the template + tests.
        let d = lookup("custom_tcp").expect("known");
        assert!(d.capabilities.contains(&FireForget));
        assert!(!d.capabilities.contains(&RequestReply));
        assert!(!d.capabilities.contains(&PubSub));
        assert!(!d.capabilities.contains(&FieldAccess));
    }

    #[test]
    fn custom_tcp_requires_connect_field() {
        // Topology validation rejects a custom_tcp binding lacking `connect:`.
        let d = lookup("custom_tcp").expect("known");
        assert!(d.required_binding_fields.contains(&"connect"));
    }

    #[test]
    fn unimplemented_transports_have_capabilities_but_no_template() {
        for name in &["dds", "can"] {
            let d = lookup(name).unwrap();
            assert!(!d.implemented, "transport '{name}' has no template yet");
            assert!(!d.capabilities.is_empty(), "transport '{name}' should still have capabilities for pattern validation");
        }
    }

    // ── supplies_dedup (SCE_MESH.md §10.5) ──────────────────

    #[test]
    fn in_process_transports_supply_dedup() {
        // local + shm cannot duplicate — no wire, no reordering.
        for name in &["local", "shm"] {
            assert!(
                lookup(name).unwrap().supplies_dedup,
                "in-process transport '{name}' should not need runtime dedup"
            );
        }
    }

    #[test]
    fn custom_tcp_supplies_dedup() {
        // custom_tcp is a single client→server TCP stream per binding
        // with length-prefixed framing — the stream itself guarantees
        // at-most-once delivery. No SD layer, no multicast fallback.
        assert!(lookup("custom_tcp").unwrap().supplies_dedup);
    }

    #[test]
    fn someip_requires_runtime_dedup() {
        // SOME/IP's service discovery is always UDP multicast, and
        // current in-tree fixtures pick UDP for method calls too, so
        // the descriptor is `false` across the board. Codegen must
        // route every inbound SOME/IP envelope through the runtime
        // DedupWindow. If an all-TCP profile with deduped SD lands,
        // it should get its own registry entry (e.g. `someip_tcp`)
        // rather than flipping this flag.
        assert!(!lookup("someip").unwrap().supplies_dedup);
    }

    #[test]
    fn zenoh_requires_runtime_dedup() {
        // Zenoh router reordering is visible to applications even in
        // reliable mode; receivers must run the DedupWindow.
        assert!(!lookup("zenoh").unwrap().supplies_dedup);
    }

    #[test]
    fn broadcast_transports_require_runtime_dedup() {
        // dds + can broadcast semantics admit duplicates; runtime dedup
        // is required as soon as the templates land.
        for name in &["dds", "can"] {
            assert!(
                !lookup(name).unwrap().supplies_dedup,
                "broadcast transport '{name}' must require runtime dedup"
            );
        }
    }

    // ── supplies_ordering / ordering_representable (SCE_MESH.md §10.6) ──

    #[test]
    fn in_process_transports_supply_ordering() {
        // local + shm preserve sender order by construction (direct
        // dispatch / FIFO ring).
        for name in &["local", "shm"] {
            let d = lookup(name).unwrap();
            assert!(
                d.supplies_ordering,
                "in-process transport '{name}' preserves order"
            );
            assert!(d.ordering_representable);
        }
    }

    #[test]
    fn custom_tcp_supplies_ordering() {
        // TCP stream preserves bytes in order; CBOR length-prefix framing
        // preserves envelope order.
        let d = lookup("custom_tcp").unwrap();
        assert!(d.supplies_ordering);
        assert!(d.ordering_representable);
    }

    #[test]
    fn someip_requires_runtime_ordering() {
        // Default UDP path may reorder. Per-binding `protocol: tcp` is a
        // runtime upgrade in `compute_needs_ordering`, not here.
        let d = lookup("someip").unwrap();
        assert!(!d.supplies_ordering);
        assert!(d.ordering_representable);
    }

    #[test]
    fn zenoh_requires_runtime_ordering() {
        // Router fabric may reorder even under reliable QoS.
        let d = lookup("zenoh").unwrap();
        assert!(!d.supplies_ordering);
        assert!(d.ordering_representable);
    }

    #[test]
    fn dds_can_be_ordered_by_runtime_buffer() {
        // BEST_EFFORT multicast reorders; per-(topic, reader) unicast
        // still supports a sender-stamped sequence domain.
        let d = lookup("dds").unwrap();
        assert!(!d.supplies_ordering);
        assert!(d.ordering_representable);
    }

    #[test]
    fn can_cannot_represent_ordering() {
        // Priority-arbitrated broadcast bus has no per-receiver sequence
        // domain. Topology rejects `ordering: required` for CAN.
        let d = lookup("can").unwrap();
        assert!(!d.supplies_ordering);
        assert!(!d.ordering_representable);
    }

    #[test]
    fn exactly_two_implemented_transports_require_runtime_ordering() {
        // Regression guard mirroring `exactly_two_implemented_transports_require_runtime_dedup`:
        // today SOME/IP and Zenoh are the implemented transports that may
        // reorder. local/shm/custom_tcp preserve order by construction.
        // If this count changes, the classification table in
        // supplies_ordering comments MUST be updated in the same commit.
        let unordered_impls: Vec<&&str> = implemented_names()
            .iter()
            .filter(|name| !lookup(name).unwrap().supplies_ordering)
            .collect();
        assert_eq!(
            unordered_impls.len(),
            2,
            "expected exactly two implemented transports to require runtime ordering \
             (someip + zenoh); got {unordered_impls:?}"
        );
    }

    #[test]
    fn only_can_cannot_represent_ordering() {
        // Regression guard: CAN is the sole transport whose broadcast
        // semantics make sender-stamped sequence meaningless at the
        // receiver. Adding another such transport would force this test
        // to update alongside the topology reject path.
        let nonrepresentable: Vec<&str> = ["local", "shm", "someip", "zenoh", "custom_tcp", "dds", "can"]
            .iter()
            .copied()
            .filter(|n| !lookup(n).unwrap().ordering_representable)
            .collect();
        assert_eq!(nonrepresentable, vec!["can"]);
    }

    #[test]
    fn only_shm_and_custom_tcp_support_inter_partition_ipc() {
        // SCE_MESH.md §14 L2729-2730: `partitions.<n>.transport_binding:`
        // chooses the transport that carries inter-partition traffic
        // within a single machine. The spec default line reads "kind
        // tcp/shm" — shm is the canonical same-machine IPC mechanism
        // (ring buffer per channel) and custom_tcp is the TCP loopback
        // reference transport (§16.8.3). Every other transport is
        // designed either for intra-process dispatch (local) or for
        // inter-machine middleware (someip / zenoh / dds / can); forcing
        // partition IPC through them contradicts the spec's direct-
        // channel intent. Adding another same-machine IPC transport
        // (e.g. iceoryx2, a local unix socket) must flip this flag in
        // the same commit that adds the registry entry — this guard
        // fails loudly instead of the addition landing silently.
        let ipc_true: Vec<&str> = ["local", "shm", "someip", "zenoh", "custom_tcp", "dds", "can"]
            .iter()
            .copied()
            .filter(|n| lookup(n).unwrap().supports_inter_partition_ipc)
            .collect();
        assert_eq!(ipc_true, vec!["shm", "custom_tcp"]);
    }

    #[test]
    fn only_someip_supports_multi_instance_server() {
        // SCE_MESH.md §14.4 / Gap 7: SOME/IP is the sole transport
        // whose native routing layer exposes a peer-level inbound
        // distinguisher (`msg->get_instance()`). Every other
        // transport — implemented or not — stays at `false` until a
        // new transport with an equivalent distinguisher arrives.
        // A future addition must update this regression guard in the
        // same commit that flips the flag, so the transport registry
        // and the parse-time reject stay synchronised.
        let multi_instance_true: Vec<&str> = ["local", "shm", "someip", "zenoh", "custom_tcp", "dds", "can"]
            .iter()
            .copied()
            .filter(|n| lookup(n).unwrap().supports_multi_instance_server)
            .collect();
        assert_eq!(multi_instance_true, vec!["someip"]);
    }

    #[test]
    fn exactly_two_implemented_transports_require_runtime_dedup() {
        // Regression guard: today SOME/IP and Zenoh are the two
        // implemented transports that admit duplicates. local/shm/
        // custom_tcp are duplicate-free by construction. If this count
        // changes, the classification table in supplies_dedup comments
        // MUST be updated in the same commit — this test fails loudly
        // instead of the change landing silently.
        let undeduped_impls: Vec<&&str> = implemented_names()
            .iter()
            .filter(|name| !lookup(name).unwrap().supplies_dedup)
            .collect();
        assert_eq!(
            undeduped_impls.len(),
            2,
            "expected exactly two implemented transports to lack inherent dedup (someip + zenoh); got {undeduped_impls:?}"
        );
    }

    // ── capabilities ────────────────────────────────────────

    #[test]
    fn local_supports_all() {
        let d = lookup("local").unwrap();
        assert!(d.capabilities.contains(&RequestReply));
        assert!(d.capabilities.contains(&FireForget));
        assert!(d.capabilities.contains(&PubSub));
        assert!(d.capabilities.contains(&FieldAccess));
    }

    #[test]
    fn shm_fire_forget_and_field() {
        let d = lookup("shm").unwrap();
        assert!(d.capabilities.contains(&FireForget));
        assert!(d.capabilities.contains(&FieldAccess));
        assert!(!d.capabilities.contains(&RequestReply));
        assert!(!d.capabilities.contains(&PubSub));
    }

    #[test]
    fn someip_supports_all() {
        let d = lookup("someip").unwrap();
        assert!(d.capabilities.contains(&RequestReply));
    }

    #[test]
    fn dds_no_request_reply() {
        let d = lookup("dds").unwrap();
        assert!(!d.capabilities.contains(&RequestReply));
        assert!(d.capabilities.contains(&PubSub));
    }

    #[test]
    fn zenoh_supports_all() {
        let d = lookup("zenoh").unwrap();
        // Zenoh realizes all four categories: put/get/pub-sub/queryable.
        assert!(d.capabilities.contains(&RequestReply));
        assert!(d.capabilities.contains(&FireForget));
        assert!(d.capabilities.contains(&PubSub));
        assert!(d.capabilities.contains(&FieldAccess));
    }

    #[test]
    fn can_signal_only() {
        let d = lookup("can").unwrap();
        assert!(d.capabilities.contains(&FireForget));
        assert!(d.capabilities.contains(&FieldAccess));
        assert!(!d.capabilities.contains(&RequestReply));
        assert!(!d.capabilities.contains(&PubSub));
    }

    // ── supports() ──────────────────────────────────────────

    #[test]
    fn known_transport_checked() {
        assert!(supports("shm", FireForget));
        assert!(!supports("shm", RequestReply));
    }

    #[test]
    fn unknown_transport_assumed_supported() {
        assert!(supports("custom_ipc", RequestReply));
    }

    // ── Display ─────────────────────────────────────────────

    #[test]
    fn capability_display() {
        assert_eq!(RequestReply.to_string(), "request/reply");
        assert_eq!(PubSub.to_string(), "pub/sub");
    }
}
