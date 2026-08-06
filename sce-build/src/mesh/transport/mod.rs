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
// Per-transport scxml-invoke codegen helpers (SCE_MESH.md §mesh-9.6 L1399 (b))
// live in sibling submodules: resolve_connect_endpoint and future
// per-peer resolvers stay off this registry so the descriptor layer
// does not re-grow inline per-transport branches.

pub mod custom_tcp;
pub mod shm;
pub mod someip;
pub mod zenoh;

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
    // §mesh-3.2: dispatch is decided here, at build time, and never at
    // runtime — there is no ITransport to virtual-call through. These two
    // flags are the whole decision surface the one shared template needs to
    // emit transport-native code per target.
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

/// How a transport's pool (SCE_MESH.md §mesh-14.4) resolves the member a
/// substituted address names.
///
/// Deliberately **not** a `supports_pool: bool`. Whether an address can
/// carry a `{name}` placeholder and whether the member set has to be
/// declared up front are different questions, and the second one is
/// author-visible: it decides whether `instances:` is required alongside
/// `instance_from:`. Collapsing them into one flag is what forced the
/// validator to ask `transport == "someip"` — a policy about a
/// transport's discovery model, written at the validation site instead
/// of at the registry that knows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolShape {
    /// No runtime substitution. Endpoints are fixed at build time
    /// (local / shm: compile-time process addresses; custom_tcp: a
    /// single static TCP endpoint, and widening it would mean SCE
    /// implementing its own service discovery, which the mesh design
    /// invariant on middleware-owned discovery rejects; can: a broadcast
    /// bus with no peer-level addressing).
    None,
    /// Any runtime value is a valid member. The substituted address is
    /// dispatched directly with no pre-registration, because the
    /// transport's routing layer resolves it per message — Zenoh's
    /// `session.put` / `session.get` on an assembled KeyExpr.
    Open,
    /// The member set must be declared in `instances:` so codegen can
    /// register each member at `init()`. Two transports, one reason
    /// each, both about discovery rather than about addressing:
    ///   - SOME/IP: `request_service(SERVICE, ANY_INSTANCE)` is
    ///     interpreted as specific-instance-0xFFFF rather than as a
    ///     wildcard, so each member needs its own `request_service`.
    ///   - DDS: a writer created at invoke time has not finished
    ///     discovery when the first sample is written, and a VOLATILE
    ///     writer drops that sample with no error. Creating the member's
    ///     endpoints at `init()` is what makes the first request
    ///     deliverable.
    Bounded,
}

impl PoolShape {
    /// Whether a binding on this transport may carry a `{name}`
    /// placeholder at all.
    pub fn supports_pool(self) -> bool {
        !matches!(self, PoolShape::None)
    }

    /// Whether a placeholder-bearing binding must also enumerate its
    /// members up front. *Which* deploy.yaml key carries that
    /// enumeration is the carrier's question, not the shape's — see
    /// [`PoolMemberCarrier::member_list_field`].
    pub fn requires_member_list(self) -> bool {
        matches!(self, PoolShape::Bounded)
    }
}

/// SCE_MESH.md §mesh-14.4 — the *type* of a pool member on this
/// transport, which decides both how the author selects one at runtime
/// and which deploy.yaml key enumerates the set.
///
/// Orthogonal to [`PoolShape`]. The shape answers "must the member set
/// be declared up front"; the carrier answers "what is a member". Both
/// combinations of the two realised axes exist today, which is why they
/// cannot be folded into one enum:
///
/// | transport | shape | carrier |
/// |---|---|---|
/// | `zenoh` | `Open` | `StringSegment` |
/// | `dds` | `Bounded` | `StringSegment` |
/// | `someip` | `Bounded` | `TypedInstanceId` |
///
/// This is the dimension the pool validator used to ask as
/// `binding.transport != "someip"` with a comment promising the registry
/// would grow it "when a second typed-instance transport actually
/// exists". DDS made the promise due the other way round — a second
/// *bounded* transport whose members are strings — so the axis lands
/// here rather than at the validation site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolMemberCarrier {
    /// No pool; the binding address is fixed at build time.
    None,
    /// A member is a typed `uint16_t` service instance id. The address
    /// has no string slot to substitute into, so the selecting
    /// `<param>` is named by an explicit `instance_from:` key and the
    /// set is enumerated as `instances: [<int>, ...]`.
    TypedInstanceId,
    /// A member is a string segment of the binding's own address
    /// (Zenoh `key:`, DDS `topic:`). The selecting `<param>` is named
    /// syntactically by a `{name}` placeholder embedded in that
    /// address; a bounded set is enumerated as `members: [<string>, ...]`.
    StringSegment,
}

impl PoolMemberCarrier {
    /// The deploy.yaml key that enumerates the member set for this
    /// carrier, or `None` when the transport has no pool at all.
    ///
    /// A [`PoolShape::Open`] carrier still answers here: the key is
    /// what the author *would* write if the shape required it, and
    /// naming it is how the "you declared a member list on a transport
    /// that does not read one" rejection stays specific.
    pub fn member_list_field(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::TypedInstanceId => Some("instances"),
            Self::StringSegment => Some("members"),
        }
    }

    /// Whether the selecting `<param>` is named by an explicit
    /// `instance_from:` key rather than by a `{name}` embed.
    pub fn selects_via_instance_from(self) -> bool {
        matches!(self, Self::TypedInstanceId)
    }

    /// One sentence describing how an author selects a member on this
    /// carrier, for the diagnostic that rejects the *other* carrier's
    /// syntax. `None` when there is no pool to describe.
    ///
    /// Rendered from the registry rather than written into each
    /// rejection site: a carrier that lands moves every message that
    /// offers it, which is the same contract `pool_alternatives` and
    /// `machine_lifetime_subscribe_alternatives` hold.
    pub fn selection_mechanism(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::TypedInstanceId => Some(
                "Its pool members are typed service instance ids: enumerate them in \
                 `instances:` and name the selecting <param> with `instance_from:`.",
            ),
            Self::StringSegment => Some(
                "Its pool members are string segments of the binding address: embed a \
                 `{name}` placeholder naming the selecting <param>, and — on a bounded \
                 pool — enumerate the values in `members:`.",
            ),
        }
    }

    /// Whether a `{name}` placeholder embedded in the binding address
    /// has a substitution target on this carrier.
    pub fn accepts_placeholder(self) -> bool {
        matches!(self, Self::StringSegment)
    }
}

/// Transports that accept a `{name}` placeholder, each paired with the
/// shape its pool resolves in.
///
/// Same contract as [`server_deadline_transports`] and
/// [`machine_lifetime_subscribe_transports`]: the rejection diagnostic
/// names the alternatives by reading the registry, so an arm that lands
/// or regresses moves the message with no second edit.
pub fn pool_transports() -> Vec<(&'static str, PoolShape, PoolMemberCarrier)> {
    known_names()
        .iter()
        .filter_map(|name| {
            let d = lookup(name)?;
            match d.pool_shape {
                PoolShape::None => None,
                shape => Some((*name, shape, d.pool_member_carrier)),
            }
        })
        .collect()
}

/// The alternatives clause the pool rejection diagnostic carries,
/// rendered from [`pool_transports`].
///
/// The "requires" hint names the carrier's own enumeration key rather
/// than a fixed `instances:`, so an author who is told to move a
/// placeholder binding onto DDS is told to write `members:` — the key
/// DDS actually reads.
pub fn pool_alternatives() -> String {
    pool_transports()
        .into_iter()
        .map(
            |(name, shape, carrier)| match (shape, carrier.member_list_field()) {
                (PoolShape::Bounded, Some(field)) => format!("'{name}' (requires {field}:)"),
                _ => format!("'{name}'"),
            },
        )
        .collect::<Vec<_>>()
        .join(", ")
}

/// What the requesting peer observes when a server-side response
/// deadline (`server.response_deadline_ms`, SCE_MESH.md server
/// response deadline) elapses on this transport.
///
/// This is deliberately **not** a `supports_server_response_deadline:
/// bool`. Whether the deadline can be armed and what the peer learns
/// when it fires are different questions, and the second one is
/// author-visible: it decides which `RpcStatus` reaches the requester
/// and therefore what the client SCXML document can branch on.
/// Collapsing the two into one flag would hide a semantic difference
/// behind a capability that reads as uniform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerDeadlineNotice {
    /// The transport cannot arm a server-side response deadline at
    /// all. `server.response_deadline_ms` is rejected at parse time
    /// (`DeployError::InvalidServerResponseDeadline`) rather than
    /// accepted into a generated router that would ignore it.
    Unsupported,
    /// Expiry releases the stored request handle and nothing is sent.
    /// The peer learns only that the exchange ended — it cannot tell
    /// "the server gave up" from "the server vanished".
    ///
    /// Zenoh: destructing the stored `zenoh::Query` signals query-done
    /// to the client, whose `session.get` `on_drop` closure delivers
    /// `RpcStatus::Unavailable`. The query model carries no
    /// server-authored failure channel, so this is the strongest
    /// notice the transport admits, not a design choice SCE made.
    DropSilently,
    /// Expiry sends a protocol-native error reply naming the timeout,
    /// so the peer distinguishes a server that gave up from one that
    /// disappeared.
    ///
    /// What the notice *is* differs by how much of the wire format the
    /// transport owns:
    ///
    /// - SOME/IP: `create_response(request)` with
    ///   `message_type_e::MT_ERROR` + `return_code_e::E_TIMEOUT`
    ///   (AUTOSAR SOME/IP Protocol Specification message-type 0x81 /
    ///   return-code 0x06). The protocol reserves the slot, so the
    ///   timeout is legible to a non-SCE peer as well.
    /// - custom_tcp: SCE's own framed envelope written back on the
    ///   stream the request arrived on (SCE_MESH.md section 10.4.4).
    ///   SCE defines the framing, so there is no foreign slot to fill —
    ///   the envelope *is* the protocol.
    /// - DDS: the envelope published on the reply topic paired with the
    ///   request topic (SCE_MESH.md section 8.2), i.e. the leg a normal
    ///   reply already travels.
    ///
    /// What every arm carries is identical, and that is the part the
    /// author sees: a `MeshEnvelope` whose `rpc_status` is
    /// `DeadlineExceeded`. A requester that used
    /// `<invoke type="sce:mesh-rpc">` raises `error.invoke.<id>` with
    /// that status rather than the `Unavailable` a vanished peer
    /// produces; a plain `<send>` requester sees the server-authored
    /// event name (`error.rpc.deadline`) instead of its declared reply
    /// event, because renaming a failure to the success event would
    /// report a false success on an empty payload.
    ActiveError,
}

impl fmt::Display for ServerDeadlineNotice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(f, "unsupported"),
            Self::DropSilently => write!(f, "silent drop"),
            Self::ActiveError => write!(f, "active error reply"),
        }
    }
}

impl ServerDeadlineNotice {
    /// What the *requester* ends up observing, phrased for a build-time
    /// diagnostic. [`fmt::Display`] names the mechanism ("silent drop");
    /// this names the consequence, which is the half an author acts on
    /// when choosing a transport for a server that needs a bounded
    /// response.
    pub fn requester_outcome(self) -> &'static str {
        match self {
            Self::Unsupported => "nothing — no deadline is armed",
            Self::DropSilently => {
                "the stored request handle is released and the requester \
                 infers RpcStatus::Unavailable from the drop"
            }
            Self::ActiveError => {
                "an error reply carrying RpcStatus::DeadlineExceeded, so the \
                 requester can tell a server that gave up from one that vanished"
            }
        }
    }
}

// ── Unified descriptor ──────────────────────────────────────

/// Complete metadata for a known transport. Codegen reads `shape`;
/// pattern validation reads `capabilities`. Both come from the same
/// `lookup()` entry — no possibility of drift.
pub struct TransportDescriptor {
    // §mesh-10.4.2: this field set IS the descriptor interface the spec
    // tabulates. Adding a capability dimension means a field here plus the
    // row that documents it — the registry stays the single place a
    // transport's build-time properties are declared.
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
    /// Per-binding keys this transport *may* carry beyond the required
    /// ones — the transport-native tunables that reach codegen through
    /// `BindingConfig`'s flattened `extra` map rather than a typed field.
    ///
    /// Together with [`Self::required_binding_fields`] this is the
    /// closed set of extra keys the transport reads. A binding key
    /// outside the union is rejected at parse time
    /// (`DeployError::UnknownBindingField`) instead of sinking into
    /// `extra` unread — the silent failure the typed device-level
    /// `transports:` structs already avoid via `deny_unknown_fields`,
    /// which the per-binding surface cannot use because `extra` is
    /// what carries transport-native keys in the first place.
    ///
    /// Adding a tunable here is half of the work: the other half is the
    /// template arm that reads it. A key listed here with no reader is
    /// worse than an unlisted one, because it advertises a setting that
    /// parses and then does nothing.
    pub optional_binding_fields: &'static [&'static str],
    /// Does this transport inherently suppress envelope duplicates?
    ///
    /// Consumed by the codegen's `dispatchToSender` branching (SCE_MESH.md
    /// §mesh-10.5): when every transport on a receiver sets this `true`, the
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
    /// Consumed by the codegen's ordering branching (SCE_MESH.md §mesh-10.6):
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
    /// Topology validation (SCE_MESH.md §mesh-10.6.2) rejects a binding
    /// declaring `ordering: required` on a transport whose
    /// `ordering_representable` is `false`.
    pub ordering_representable: bool,
    /// Does this transport carry a native routing layer that can
    /// substitute runtime values into a binding's address (SCE_MESH.md
    /// §mesh-14.4)?
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
    ///     own SD protocol, which the §mesh-3.3 design invariant explicitly
    ///     rejects.
    ///   - can: broadcast bus with no peer-level addressing.
    ///
    /// A binding value field containing a `{name}` placeholder token is
    /// rejected at the topology stage when the transport declares
    /// [`PoolShape::None`] (`mesh/pool-not-supported-by-transport`).
    ///
    /// The open/bounded distinction lives here rather than at the
    /// validation site because it is a property of the transport's
    /// discovery model, not of any one deploy.yaml: a validator that
    /// asked `transport == "someip"` would be right until the second
    /// bounded transport landed, and then silently wrong.
    pub pool_shape: PoolShape,
    /// SCE_MESH.md §mesh-14.4 — what a pool member *is* on this
    /// transport, which decides the selecting syntax (`{name}` embed vs
    /// `instance_from:`) and the enumerating key (`members:` vs
    /// `instances:`).
    ///
    /// Independent of [`Self::pool_shape`]: DDS and SOME/IP are both
    /// `Bounded` but enumerate different things, while DDS and Zenoh
    /// share a carrier and differ only in shape. `PoolMemberCarrier::None`
    /// iff `pool_shape` is `PoolShape::None` — a transport that admits
    /// no substitution has nothing to carry, and
    /// `pool_axes_agree_on_absence` locks the biconditional.
    pub pool_member_carrier: PoolMemberCarrier,
    /// Is the deploy.yaml `machines.<name>.subscriptions:` (SCE_MESH.md
    /// §mesh-13 machine-lifetime path) realised end-to-end on this transport?
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
    /// - `someip`: `true`. The subscribe needs an
    ///   `(event_group_id, event_id)` pair, and `resolve_someip_ids`
    ///   projects the binding's eventgroup declaration onto the
    ///   subscription event exactly as it does for an SCXML-driven
    ///   `event.subscribe.X` — the machine-lifetime path reads the same
    ///   vsomeip.json through the same resolver. A subscription whose
    ///   binding declares no eventgroup is rejected by name rather than
    ///   dispatched into the "unknown event" arm.
    /// - `zenoh`: binding-wide `key:` is sufficient for subscribe
    ///   dispatch — no per-event resolution needed. `true`.
    /// - `dds`: implemented, but no fixture drives deploy.yaml
    ///   `subscriptions:` over it, so this stays `false` on the same
    ///   grounds custom_tcp does — the flag's contract is "realised
    ///   end-to-end", not "the mechanism looks generic".
    /// - `can`: unimplemented; set `false`.
    ///
    /// Consumed by the topology-stage contributor
    /// (`topology::contribute_subscription_partials`): a subscription source whose
    /// binding transport has this set to `false` is rejected with
    /// `mesh/topology-machine-lifetime-subscription-unsupported`.
    pub supports_machine_lifetime_subscribe: bool,
    /// Can this transport host an SCE machine as a multi-instance
    /// server? (SCE_MESH.md §mesh-14.4 multi-instance server pool, Gap 7.)
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
    ///     hosted instance from another. SCE_MESH.md §mesh-14.4
    ///     multi-instance scope deliberately excludes Zenoh server
    ///     pools.
    ///   - local / shm / custom_tcp: endpoints are compile-time process
    ///     addresses; a single process cannot semantically back N
    ///     independent peer identities on the same in-process channel.
    ///   - dds: implemented; SCE_MESH.md section 14.4 scopes the
    ///     multi-instance server pool to SOME/IP, and the dds arm emits
    ///     one server endpoint per router.
    ///   - can: unimplemented; a broadcast bus has no per-peer server
    ///     instance for a pool to scale.
    ///
    /// Consumed by `deploy::validate_server_pool_rejection`: a machine
    /// declaring `server.instances:` on a transport whose flag is
    /// `false` is rejected at parse time with the transport name in
    /// the diagnostic (`DeployError::ServerPoolNotSupported`).
    pub supports_multi_instance_server: bool,
    /// Can this transport carry inter-partition IPC traffic within a
    /// single machine (SCE_MESH.md §mesh-14 L2729-2730)?
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
    /// - `dds` — implemented; an inter-machine multi-participant DCPS
    ///   middleware rather than a same-machine channel.
    /// - `can` — unimplemented, and a priority-arbitrated broadcast bus
    ///   is not a direct same-machine IPC channel either.
    ///
    /// Consumed by `deploy::validate_partitions_schema`: a partition
    /// whose `transport_binding:` names a transport with this flag
    /// `false` is rejected at parse time with
    /// `MeshDeployPartitionTransportBindingUnsupported`.
    pub supports_inter_partition_ipc: bool,
    /// Can an RpcReply for a request sent to target A be received from a
    /// different target B on this transport (SCE_MESH.md §mesh-14.6
    /// responder set)?
    ///
    /// `true` for transports whose reply arrives through a correlation
    /// lookup keyed on something the sender minted, so a reply landing on
    /// a different inbound path can still be matched:
    ///   - `someip`: the reply carries `correlation_id` in the envelope
    ///     and the router-scoped `pending_rpcs_` table resolves it; each
    ///     target's `register_message_handler` consults the same table.
    ///   - `local`: in-process dispatch reaches `dispatchToSession`
    ///     directly, where the `invoke_correlation_` table resolves
    ///     `invoke_id` without consulting the arrival path.
    ///
    /// `false` for transports whose reply path is bound to the request
    /// at the protocol layer, leaving no place for another target's
    /// reply to enter:
    ///   - `zenoh`: `session.get(key, …)` installs a per-query reply
    ///     closure on ONE target's KeyExpr. A different target's
    ///     queryable is a different key and therefore a different query;
    ///     there is no correlation table to address. This is a property
    ///     of the query model, not a missing feature — Zenoh is
    ///     same-target by construction and carries no cross-target
    ///     mis-correlation risk either.
    ///   - `custom_tcp`: the reply is written on the stream the request
    ///     arrived on, so another target — by definition another
    ///     connection — has no entry its reply could land in. Same
    ///     conclusion as `zenoh`, reached from the connection model
    ///     rather than the query model.
    ///   - `shm` / `can` / `dds`: no `RequestReply` capability at all, so
    ///     the question does not arise.
    ///
    /// Consumed by `deploy::validate_reply_from`: a binding event
    /// declaring a `reply_from:` set wider than its own target on a
    /// transport whose flag is `false` is rejected at parse time
    /// (`DeployError::CrossTargetReplyNotSupported`). The default
    /// responder set — the binding's own target — is always legal.
    pub supports_cross_target_reply: bool,
    /// What a requesting peer observes when this transport's server
    /// arm lets `server.response_deadline_ms` elapse (SCE_MESH.md
    /// server response deadline).
    ///
    /// The knob is transport-neutral by name because the thing it
    /// bounds — how long a server may hold an inbound request handle
    /// before the router releases it — is transport-neutral. What
    /// differs is the notice, which is exactly what this dimension
    /// records:
    ///
    /// - `someip`: [`ServerDeadlineNotice::ActiveError`]. The protocol
    ///   defines `MT_ERROR` (0x81) with `E_TIMEOUT` (0x06) and vsomeip
    ///   exposes both through `message_base::set_message_type` /
    ///   `set_return_code`, so expiry is reported rather than inferred.
    /// - `custom_tcp`: [`ServerDeadlineNotice::ActiveError`]. The
    ///   stashed `pending_server_links_` entry IS the return leg, so the
    ///   notice goes back on the stream that carried the request —
    ///   the same one-shot resource a normal reply consumes.
    /// - `dds`: [`ServerDeadlineNotice::ActiveError`]. The notice is
    ///   published on the reply topic derived from the request topic,
    ///   after the admitted correlation is erased so a late engine
    ///   response cannot publish a second answer.
    /// - `zenoh`: [`ServerDeadlineNotice::DropSilently`]. A
    ///   `zenoh::Query` has `reply` but no server-authored failure
    ///   channel; dropping it is the only signal available.
    /// - `local`: [`ServerDeadlineNotice::Unsupported`]. In-process
    ///   dispatch has no stored request handle to strand — the reply
    ///   leg is a direct call on the same stack.
    /// - `shm` / `can`: [`ServerDeadlineNotice::Unsupported`]. Neither
    ///   carries `RequestReply`, so there is no server arm at all.
    ///
    /// Per the `supports_machine_lifetime_subscribe` precedent this
    /// flag's contract is "realised end-to-end", not "the mechanism
    /// looks generic": a value above `Unsupported` means a generated arm
    /// arms the deadline and a fixture drives it.
    ///
    /// Consumed by `deploy::validate_server_response_deadline`: a
    /// machine declaring the knob on an `Unsupported` transport is
    /// rejected at parse time with the transport name and its notice
    /// semantics in the diagnostic, rather than shipping a knob the
    /// generated router silently ignores.
    pub server_deadline_notice: ServerDeadlineNotice,
}

impl TransportDescriptor {
    /// The closed set of per-binding keys this transport reads through
    /// the flattened `extra` map: required first, then optional, each in
    /// declaration order.
    ///
    /// Callers use this both to reject unknown keys and to tell the
    /// author what *was* legal, so the ordering is the one a reader
    /// benefits from — mandatory keys ahead of tunables — rather than
    /// alphabetical.
    pub fn known_binding_fields(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.required_binding_fields
            .iter()
            .chain(self.optional_binding_fields.iter())
            .copied()
    }
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
        shape: TransportShape {
            has_per_target_field: true,
            has_shared_session: false,
        },
        capabilities: &[RequestReply, FireForget, PubSub, FieldAccess],
        implemented: true,
        required_binding_fields: &[],
        // In-process dispatch reads no per-binding key: the target is a
        // compile-time reference, so there is nothing to address.
        optional_binding_fields: &[],
        // In-process direct dispatch — no wire, no reordering.
        supplies_dedup: true,
        // Direct function call preserves invocation order.
        supplies_ordering: true,
        ordering_representable: true,
        // Same-process target identity is compile-time; no runtime
        // address substitution.
        pool_shape: PoolShape::None,
        pool_member_carrier: PoolMemberCarrier::None,
        // Not "no pub/sub" — the descriptor advertises `PubSub` above and
        // means it: a notification event is carried to the peer engine
        // like any other. What in-process dispatch has no room for is a
        // subscription *lifetime*. There is no reader to create, no
        // registration to key on, and nothing an unsubscribe could
        // retire, so a `subscriptions:` entry pointed here would have
        // nothing to take effect on. Exercised as the fail-closed case by
        // `topology::tests::subscription_on_transport_without_a_subscribe_arm_rejected`.
        supports_machine_lifetime_subscribe: false,
        // In-process direct dispatch: one process hosts one identity
        // per machine; a second instance would be a second process.
        supports_multi_instance_server: false,
        // In-process direct dispatch cannot cross the OS process
        // boundary that `partitions:` defines (§mesh-14 L2729-2730).
        supports_inter_partition_ipc: false,
        // Replies resolve through `invoke_correlation_` at
        // `dispatchToSession`, which keys on `invoke_id` alone and
        // never consults the arrival path.
        supports_cross_target_reply: true,
        // In-process dispatch never parks a request handle: the reply
        // leg runs on the caller's own stack, so there is no stranded
        // server-side state for a deadline to release.
        server_deadline_notice: ServerDeadlineNotice::Unsupported,
    };
    static SHM: TransportDescriptor = TransportDescriptor {
        shape: TransportShape {
            has_per_target_field: true,
            has_shared_session: false,
        },
        // FireForget only. A `ShmChannel` is a per-target one-way ring
        // (sender SM → receiver SM, ShmChannel.h) with no reverse path,
        // so every pattern requiring a reply leg is unrealisable here:
        // RequestReply needs `service.response`, and FieldAccess needs
        // the SCE_MESH.md §mesh-8.3 `field.notify` reply to a
        // `field.get`/`field.set` request. The field reply machinery
        // (`handleServerResponse`, getter/setter receive handlers) is
        // emitted only for the someip/zenoh transport arms; shm's send
        // arm is a bare one-way `channel.send(env)`. Advertising
        // FieldAccess would let `field.get` over shm pass
        // `validate_pattern_capability` at build time and then silently
        // never receive its reply at runtime. Same exclusion rationale
        // as RequestReply (covered by the `mesh_pattern_capability_rejection`
        // negative test). Inter-machine field access uses someip/zenoh.
        capabilities: &[FireForget],
        implemented: true,
        required_binding_fields: &[],
        // Ring geometry: both are validated by
        // `topology::validate_shm_extras_partial` (u32 range,
        // power-of-two capacity) and reach codegen as
        // `TransportState::Shm`.
        optional_binding_fields: &["shm_arena_bytes", "shm_ring_capacity"],
        // Single shared ring per channel with READY_MAGIC gating — a
        // producer cannot publish the same slot twice.
        supplies_dedup: true,
        // FIFO ring buffer per channel preserves producer order.
        supplies_ordering: true,
        ordering_representable: true,
        // SHM channels are pre-declared in deploy.yaml; runtime values
        // cannot address a new channel.
        pool_shape: PoolShape::None,
        pool_member_carrier: PoolMemberCarrier::None,
        // No pub/sub capability; machine-lifetime path does not apply.
        supports_machine_lifetime_subscribe: false,
        // SHM channels are pre-declared and addressed by compile-time
        // names — no peer-identity inbound distinguisher.
        supports_multi_instance_server: false,
        // Shared-memory ring buffer per channel is the canonical
        // same-machine IPC mechanism (§mesh-14 L2730 "kind tcp/shm").
        supports_inter_partition_ipc: true,
        // No RequestReply capability; the question does not arise.
        supports_cross_target_reply: false,
        // One-way ring with no reverse path — no server arm exists to
        // bound.
        server_deadline_notice: ServerDeadlineNotice::Unsupported,
    };
    static SOMEIP: TransportDescriptor = TransportDescriptor {
        shape: TransportShape {
            has_per_target_field: true,
            has_shared_session: false,
        },
        capabilities: &[RequestReply, FireForget, PubSub, FieldAccess],
        implemented: true,
        // SOME/IP identity (`service_id` + `instance_id`) and per-event IDs
        // are NOT verified via this generic `extra`-key presence check —
        // they live on typed `ResolvedTarget` fields (`someip_service`,
        // `event_bindings`) populated by `finalize_targets`, and
        // topology runs a typed check after that pass.
        required_binding_fields: &[],
        // `protocol:` selects the vsomeip reliability flag per binding
        // (`udp` default, `tcp` reliable) and is read by both the send
        // and the scxml-invoke arms of the template. The numeric SOME/IP
        // ID keys are not listed here on purpose — they are reserved,
        // and `external::reject_reserved_id_keys` refuses them with a
        // diagnostic that names the name-based alternative.
        optional_binding_fields: &["protocol"],
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
        pool_shape: PoolShape::Bounded,
        // A member is the typed `uint16_t` passed to
        // `message->set_instance(...)`. The SOME/IP address has no
        // string slot a `{name}` embed could substitute into, so the
        // selecting `<param>` is named by `instance_from:` instead.
        pool_member_carrier: PoolMemberCarrier::TypedInstanceId,
        // Both subscribe paths share one resolver: a deploy.yaml
        // `subscriptions:` entry projects the binding's eventgroup
        // declaration onto its event the same way an SCXML-driven
        // `event.subscribe.X` does, so init() emits the same
        // request_event + subscribe pair against vsomeip.json-resolved
        // ids (SCE_MESH.md §mesh-13).
        supports_machine_lifetime_subscribe: true,
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
        // instead of the direct channel §mesh-14 intends; authors should
        // pick `shm` or `custom_tcp` for inter-partition traffic.
        supports_inter_partition_ipc: false,
        // The reply carries `correlation_id` and every target's
        // `register_message_handler` resolves it against the same
        // router-scoped `pending_rpcs_` table.
        supports_cross_target_reply: true,
        // AUTOSAR SOME/IP defines the timeout notice the request
        // originator needs: message type `MT_ERROR` (0x81) with return
        // code `E_TIMEOUT` (0x06). `create_response` copies the
        // request's client/session/service/method header, so the error
        // reply correlates on the wire exactly as a normal response
        // would, and the envelope it carries stamps
        // `RpcStatus::DeadlineExceeded`.
        server_deadline_notice: ServerDeadlineNotice::ActiveError,
    };
    static ZENOH: TransportDescriptor = TransportDescriptor {
        shape: TransportShape {
            has_per_target_field: false,
            has_shared_session: true,
        },
        // Zenoh supports RPC via queryable/query primitives — `session.get()`
        // against a `declare_queryable()` endpoint. Correlation is handled
        // natively by the Zenoh runtime (reply callbacks), so no per-router
        // correlation table is needed.
        capabilities: &[RequestReply, FireForget, PubSub, FieldAccess],
        implemented: true,
        required_binding_fields: &["key"],
        // Everything else a Zenoh deployment tunes lives in the device-
        // level `transports.zenoh.config:` json5 file, which Zenoh's own
        // schema validates — a per-binding tunable would have to be
        // merged into that session config, which the session does not
        // support per-publisher.
        optional_binding_fields: &[],
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
        pool_shape: PoolShape::Open,
        // A member is a string segment of the `key:` KeyExpr, selected
        // by a `{name}` embed. Same carrier as DDS; the two differ on
        // the shape axis only.
        pool_member_carrier: PoolMemberCarrier::StringSegment,
        // Binding-wide `key:` is sufficient for dispatch — no per-event
        // external resolution needed for subscribe. SCE_MESH.md §mesh-13
        // machine-lifetime path is fully wired end-to-end.
        supports_machine_lifetime_subscribe: true,
        // Zenoh KeyExpr identifies a subject, not a peer. There is no
        // server-side inbound distinguisher between two instances of
        // the same hosted machine (a subscriber/queryable on the same
        // key receives every request identically). SCE_MESH.md §mesh-14.4
        // multi-instance exclusion clause codifies this.
        supports_multi_instance_server: false,
        // Zenoh is an inter-machine fabric: traffic flows through a
        // routing layer that scouts peers across the network. Same-
        // machine IPC via Zenoh would route through that fabric
        // instead of the direct channel §mesh-14 intends; authors should
        // pick `shm` or `custom_tcp` for inter-partition traffic.
        supports_inter_partition_ipc: false,
        // `session.get` binds the reply closure to ONE target's
        // KeyExpr, so another target's queryable is a different
        // query with no shared correlation table. Same-target by
        // construction.
        supports_cross_target_reply: false,
        // `zenoh::Query` exposes `reply` and destruction, nothing in
        // between: there is no server-authored failure channel to put
        // a timeout on. Dropping the stored query is the whole notice,
        // and the client reads it as `RpcStatus::Unavailable` through
        // the `session.get` `on_drop` closure.
        server_deadline_notice: ServerDeadlineNotice::DropSilently,
    };
    // SCE Mesh §mesh-16.8.3 reference transport: TCP loopback, length-prefixed
    // CBOR envelope framing, zero external dependencies. Each binding has a
    // per-target client (its `connect:` endpoint) and the device exposes a
    // single `listen:` server (declared in `transports.custom_tcp.listen`),
    // so both shape flags apply: per-target client field + device-shared
    // server.
    //
    // All four capabilities are realised (§mesh-10.4.4). The enabling
    // property is that one TCP connection is duplex: a reply and a
    // notification both travel back on the stream that carried the request
    // or the subscribe, so no pattern needs a reversed connection to a peer
    // that may have dialed from an ephemeral port, and none needs a broker.
    static CUSTOM_TCP: TransportDescriptor = TransportDescriptor {
        shape: TransportShape {
            has_per_target_field: true,
            has_shared_session: true,
        },
        capabilities: &[RequestReply, FireForget, PubSub, FieldAccess],
        implemented: true,
        required_binding_fields: &["connect"],
        optional_binding_fields: &[],
        // One TCP socket per sender→receiver pair; the stream itself
        // guarantees at-most-once delivery of any framed envelope.
        supplies_dedup: true,
        // TCP preserves byte stream order; the length-prefixed CBOR
        // framing layered on top preserves envelope order.
        supplies_ordering: true,
        ordering_representable: true,
        // No native routing layer. Endpoints are static in deploy.yaml;
        // adding a pool would require SCE to implement its own SD
        // protocol (§mesh-3.3 design invariant rejects middleware SD).
        pool_shape: PoolShape::None,
        pool_member_carrier: PoolMemberCarrier::None,
        // Realised end to end. `init()` builds an `EventSubscribe`
        // envelope and hands it to the same `route_send` the SCXML-driven
        // path uses; the receiving half (registry registration keyed on
        // the event's axis) is the one
        // `mesh_tcp_multipattern_verification` already covered. What was
        // missing was the deploy.yaml-originated half, which
        // `mesh_tcp_machine_lifetime_subscribe_runtime` now drives on a
        // brake carrying zero SCXML `<send>`s: every framed envelope that
        // reaches the wire traces back to
        // `machines.<name>.subscriptions:`.
        supports_machine_lifetime_subscribe: true,
        // Single static client→server TCP endpoint — no peer-identity
        // dimension on the inbound side.
        supports_multi_instance_server: false,
        // TCP loopback with length-prefixed CBOR framing is the `tcp`
        // half of spec L2730's "kind tcp/shm" default pair for
        // inter-partition traffic within the same machine.
        supports_inter_partition_ipc: true,
        // Same-target by construction, like Zenoh but for a different
        // reason. A pending request is pinned to the stream it left on
        // (`PendingRpc::stream_id`), and another target is by definition
        // another connection — so there is no place for its reply to
        // enter. That pinning is what makes forging `source` useless
        // here, and it is also what forecloses the wider responder set:
        // the property that removes the vulnerability removes the
        // feature. A `reply_from:` wider than the binding's own target
        // is therefore rejected at parse time.
        supports_cross_target_reply: false,
        // `pending_server_links_` holds the reply stream the deadline
        // bounds, and the notice is SCE's own envelope framed on that
        // same stream — the request's return leg (SCE_MESH.md section
        // 10.4.4), so it reaches the requester over a connection that is
        // by construction the one that asked. No protocol error slot is
        // involved because custom_tcp defines the framing itself.
        server_deadline_notice: ServerDeadlineNotice::ActiveError,
    };
    static DDS: TransportDescriptor = TransportDescriptor {
        shape: TransportShape {
            has_per_target_field: true,
            has_shared_session: false,
        },
        // All four are realised (SCE_MESH.md section 8.2). `RequestReply`
        // joins the set that was already advertised: the reply leg is a
        // topic derived from the request topic, and `FieldAccess` was
        // never separable from it — a `field.get` needs the same paired
        // reply a `service.request` does, which is why advertising one
        // without the other was incoherent.
        capabilities: &[FireForget, RequestReply, PubSub, FieldAccess],
        implemented: true,
        // A binding names its request-leg topic; the reply and
        // notification topics are derived from it at emission time, so
        // there is no field in which an author could pair a request topic
        // with an unrelated reply topic.
        required_binding_fields: &["topic"],
        optional_binding_fields: &[],
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
        // Bounded, for the same class of reason SOME/IP is, but arrived
        // at from the other side: what rules out an `Open` pool is
        // discovery. A writer created at invoke time has not matched
        // when the first sample is written, and a VOLATILE writer drops
        // it with no error (`Client::waitForServer` exists precisely
        // because of this). Declaring the members lets codegen build
        // every member's `Dds::Client` at `init()` and settle discovery
        // before any request is dispatched, which is what makes the
        // first request to each member deliverable rather than silently
        // lost.
        pool_shape: PoolShape::Bounded,
        // A member is a string segment substituted into `topic:` by a
        // `{name}` embed — the same carrier Zenoh uses, which is why
        // `members:` and not `instances:` enumerates the set: a DDS
        // topic segment is a name (`front_left`), not a service
        // instance id.
        pool_member_carrier: PoolMemberCarrier::StringSegment,
        // Realised end to end. A DDS subscription is its notification
        // reader, which the router creates and destroys, and
        // `mesh_dds_machine_lifetime_subscribe_runtime` drives that from
        // deploy.yaml on a brake carrying zero SCXML `<send>`s. The DDS
        // arm is where the teardown half is observable at all: the
        // participant stays up when the reader goes, so a publish issued
        // after `shutdown()` is still a real publish and a control
        // subscriber proves it landed — over custom_tcp, hanging up is
        // itself the unsubscribe and that distinction cannot be drawn.
        supports_machine_lifetime_subscribe: true,
        // SCE_MESH.md section 14.4 scopes the multi-instance server pool to SOME/IP.
        // The dds arm emits one server endpoint per router and reads
        // `session_idx` 0 throughout.
        supports_multi_instance_server: false,
        // DDS is an inter-machine multi-participant middleware, not a
        // same-machine IPC channel — out of §mesh-14's IPC scope.
        supports_inter_partition_ipc: false,
        // A reply rides the topic paired with the request topic of the
        // binding it answers, so another target — a different topic
        // pair — has no entry its reply could land in. Same conclusion
        // as zenoh and custom_tcp, reached from the topic model.
        supports_cross_target_reply: false,
        // The notice is published on the reply topic paired with the
        // request topic (SCE_MESH.md section 8.2) — the same leg a
        // normal reply takes, so a requester needs no second reader to
        // observe it. Expiry
        // erases the admitted correlation first, which is what keeps the
        // notice one-shot against a late engine response.
        server_deadline_notice: ServerDeadlineNotice::ActiveError,
    };
    static CAN: TransportDescriptor = TransportDescriptor {
        shape: TransportShape {
            has_per_target_field: true,
            has_shared_session: false,
        },
        capabilities: &[FireForget, FieldAccess],
        implemented: false,
        required_binding_fields: &[],
        optional_binding_fields: &[],
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
        pool_shape: PoolShape::None,
        pool_member_carrier: PoolMemberCarrier::None,
        // Unimplemented + broadcast bus; machine-lifetime subscribe
        // semantic does not apply.
        supports_machine_lifetime_subscribe: false,
        // Broadcast bus — every frame reaches every participant; the
        // concept of a per-peer server instance does not map.
        supports_multi_instance_server: false,
        // CAN is a priority-arbitrated broadcast bus — not a direct
        // same-machine IPC channel.
        supports_inter_partition_ipc: false,
        // Broadcast bus, no RequestReply capability.
        supports_cross_target_reply: false,
        // Unimplemented, and a broadcast bus carries no server arm.
        server_deadline_notice: ServerDeadlineNotice::Unsupported,
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
    &["local", "shm", "someip", "zenoh", "custom_tcp", "dds"]
}

/// Every transport name the registry resolves, in `lookup()` dispatch
/// order. Unlike [`implemented_names`] this includes transports whose
/// `implemented` flag is still `false` — it answers "what does the
/// registry know about", not "what can codegen emit".
///
/// Exists so a diagnostic can enumerate a *dimension* of the registry
/// (see [`server_deadline_transports`]) instead of restating it in prose
/// that goes stale the moment an arm lands.
pub fn known_names() -> &'static [&'static str] {
    &[
        "local",
        "shm",
        "someip",
        "zenoh",
        "custom_tcp",
        "dds",
        "can",
    ]
}

/// Transports whose server arm realises `server.response_deadline_ms`,
/// each paired with the notice its expiry emits.
///
/// Consumed by `deploy`'s rejection diagnostic: when the knob is
/// declared on a transport that cannot arm it, the message names the
/// alternatives by reading the registry rather than by carrying a
/// hand-maintained list. A new arm therefore appears in the diagnostic
/// the moment its descriptor changes, and an arm that regresses to
/// `Unsupported` disappears from it — neither needs a second edit.
pub fn server_deadline_transports() -> Vec<(&'static str, ServerDeadlineNotice)> {
    known_names()
        .iter()
        .filter_map(|name| {
            let d = lookup(name)?;
            match d.server_deadline_notice {
                ServerDeadlineNotice::Unsupported => None,
                notice => Some((*name, notice)),
            }
        })
        .collect()
}

/// Transports whose send path realises the deploy.yaml-originated
/// machine-lifetime subscribe (`machines.<name>.subscriptions:`).
///
/// Same contract as [`server_deadline_transports`] and consumed the same
/// way: the topology stage's rejection diagnostic names the alternatives
/// by reading this dimension of the registry, so an arm that lands or
/// regresses moves the message without a second edit. The custom_tcp
/// landing is exactly the case a prose list would have gone stale on —
/// the message said "e.g. 'zenoh'" while three transports realised it.
pub fn machine_lifetime_subscribe_transports() -> Vec<&'static str> {
    known_names()
        .iter()
        .filter(|name| lookup(name).is_some_and(|d| d.supports_machine_lifetime_subscribe))
        .copied()
        .collect()
}

/// The alternatives clause the machine-lifetime rejection diagnostic
/// carries, rendered from [`machine_lifetime_subscribe_transports`].
///
/// Lives beside the registry rather than at the raise site so the one
/// place that knows the dimension also owns how it reads.
pub fn machine_lifetime_subscribe_alternatives() -> String {
    machine_lifetime_subscribe_transports()
        .into_iter()
        .map(|name| format!("'{name}'"))
        .collect::<Vec<_>>()
        .join(", ")
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
        // §mesh-16.8.3 reference transport: per-binding `connect:` (client field)
        // + device-level `transports.custom_tcp.listen:` (shared server).
        let d = lookup("custom_tcp").expect("known");
        assert!(d.shape.has_per_target_field);
        assert!(d.shape.has_shared_session);
    }

    #[test]
    fn custom_tcp_realizes_every_pattern() {
        // §mesh-10.4.4: one duplex TCP connection carries every pattern in
        // both directions, so nothing here is advertised ahead of its
        // template arm. Narrowing this list again would mean a pattern lost
        // its realisation — which must break a test, not degrade silently.
        let d = lookup("custom_tcp").expect("known");
        for cap in &[RequestReply, FireForget, PubSub, FieldAccess] {
            assert!(
                d.capabilities.contains(cap),
                "custom_tcp should advertise {cap}"
            );
        }
    }

    #[test]
    fn custom_tcp_reply_is_same_target_by_construction() {
        // The stream a request left on IS the responder identity
        // (`PendingRpc::stream_id`), so another target's reply has no
        // correlation entry to reach — the same shape as Zenoh's
        // per-query reply closure, reached by a different mechanism.
        // Advertising cross-target reply here would promise a routing
        // path that the pinning deliberately forecloses.
        let d = lookup("custom_tcp").expect("known");
        assert!(d.capabilities.contains(&RequestReply));
        assert!(!d.supports_cross_target_reply);
    }

    #[test]
    fn every_known_binding_field_has_a_reader() {
        // A key in this set parses; if no arm reads it, the author gets a
        // setting that takes effect nowhere — the exact failure the
        // unknown-key gate exists to prevent, re-introduced from the
        // other side. So the set is pinned here, and growing it means
        // pointing at the reader in the same commit.
        //
        //   shm    → `topology::build_transport_state` (TransportState::Shm)
        //   someip → `mesh_transport.h.jinja2` reliability flag
        //   zenoh/custom_tcp/dds → the address each arm bakes in
        let with_fields: Vec<(&str, Vec<&str>)> = [
            "local",
            "shm",
            "someip",
            "zenoh",
            "custom_tcp",
            "dds",
            "can",
        ]
        .iter()
        .copied()
        .map(|n| (n, lookup(n).unwrap().known_binding_fields().collect()))
        .filter(|(_, f): &(_, Vec<&str>)| !f.is_empty())
        .collect();

        assert_eq!(
            with_fields,
            vec![
                ("shm", vec!["shm_arena_bytes", "shm_ring_capacity"]),
                ("someip", vec!["protocol"]),
                ("zenoh", vec!["key"]),
                ("custom_tcp", vec!["connect"]),
                ("dds", vec!["topic"]),
            ]
        );
    }

    #[test]
    fn required_fields_lead_the_known_set() {
        // `known_binding_fields` is what the unknown-key diagnostic
        // renders as "legal keys", and a reader scanning that list
        // should meet the mandatory ones first.
        let d = lookup("zenoh").expect("known");
        assert_eq!(d.known_binding_fields().next(), Some("key"));
    }

    #[test]
    fn custom_tcp_requires_connect_field() {
        // Topology validation rejects a custom_tcp binding lacking `connect:`.
        let d = lookup("custom_tcp").expect("known");
        assert!(d.required_binding_fields.contains(&"connect"));
    }

    #[test]
    fn unimplemented_transports_have_capabilities_but_no_template() {
        // `can` is the last unimplemented entry: dds joined the implemented
        // set when its template arm landed. Kept as a single case rather than
        // a loop so the shrinking set is visible at the call site.
        let d = lookup("can").expect("known");
        assert!(!d.implemented, "transport 'can' has no template yet");
        assert!(
            !d.capabilities.is_empty(),
            "transport 'can' should still have capabilities for pattern validation"
        );
    }

    #[test]
    fn no_doc_comment_calls_an_implemented_transport_unimplemented() {
        // Field docs summarise why each transport carries a given value,
        // naming transports directly. When one is implemented the summary
        // has to move with it, and three of them did not: dds landed a
        // template in the multi-pattern arm while
        // `supports_machine_lifetime_subscribe`,
        // `supports_multi_instance_server` and
        // `supports_inter_partition_ipc` still documented it as
        // unimplemented. Each flag is still `false`, so no behaviour was
        // wrong — the recorded *reason* was, which is worse than a stale
        // value because it reads as a live justification for a decision
        // nobody would revisit.
        //
        // Scoped to `///` field docs: those are where a transport name and
        // a claim about it sit together. Ordinary `//` comments in this
        // module discuss the same names narratively ("dds joined the
        // implemented set") and are not claims about a field's value.
        //
        // Any implemented transport named on a doc line that also says
        // "unimplemented" is a mismatch, whatever separator joins the
        // names — the stale form was "`dds` / `can`: unimplemented", so
        // keying on the separator would have missed the very case this
        // exists for.
        let source = include_str!("mod.rs");
        let named: Vec<(&str, regex::Regex)> = implemented_names()
            .iter()
            .map(|name| {
                let word = regex::Regex::new(&format!(r"\b{}\b", regex::escape(name)))
                    .expect("transport-name regex compiles");
                (*name, word)
            })
            .collect();

        let mut stale = Vec::new();
        for (idx, line) in source.lines().enumerate() {
            if !line.trim_start().starts_with("///") || !line.contains("unimplemented") {
                continue;
            }
            for (name, word) in &named {
                if word.is_match(line) {
                    stale.push(format!(
                        "  mod.rs:{}: '{}' — {}",
                        idx + 1,
                        name,
                        line.trim()
                    ));
                }
            }
        }

        assert!(
            stale.is_empty(),
            "doc comment(s) describe an implemented transport as unimplemented:\n{}\n\
             Replace the claim with the reason the field actually holds its \
             value — the descriptor body already states it.",
            stale.join("\n")
        );
    }

    // ── supplies_dedup (SCE_MESH.md §mesh-10.5) ──────────────────

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

    // ── supplies_ordering / ordering_representable (SCE_MESH.md §mesh-10.6) ──

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
    fn exactly_three_implemented_transports_require_runtime_ordering() {
        // Regression guard mirroring the dedup counterpart below: today
        // SOME/IP, Zenoh and DDS are the implemented transports that may
        // reorder. local/shm/custom_tcp preserve order by construction.
        // If this count changes, the classification table in
        // supplies_ordering comments MUST be updated in the same commit.
        let unordered_impls: Vec<&&str> = implemented_names()
            .iter()
            .filter(|name| !lookup(name).unwrap().supplies_ordering)
            .collect();
        assert_eq!(
            unordered_impls.len(),
            3,
            "expected exactly three implemented transports to require runtime ordering \
             (someip + zenoh + dds); got {unordered_impls:?}"
        );
    }

    #[test]
    fn only_can_cannot_represent_ordering() {
        // Regression guard: CAN is the sole transport whose broadcast
        // semantics make sender-stamped sequence meaningless at the
        // receiver. Adding another such transport would force this test
        // to update alongside the topology reject path.
        let nonrepresentable: Vec<&str> = [
            "local",
            "shm",
            "someip",
            "zenoh",
            "custom_tcp",
            "dds",
            "can",
        ]
        .iter()
        .copied()
        .filter(|n| !lookup(n).unwrap().ordering_representable)
        .collect();
        assert_eq!(nonrepresentable, vec!["can"]);
    }

    #[test]
    fn only_shm_and_custom_tcp_support_inter_partition_ipc() {
        // SCE_MESH.md §mesh-14 L2729-2730: `partitions.<n>.transport_binding:`
        // chooses the transport that carries inter-partition traffic
        // within a single machine. The spec default line reads "kind
        // tcp/shm" — shm is the canonical same-machine IPC mechanism
        // (ring buffer per channel) and custom_tcp is the TCP loopback
        // reference transport (§mesh-16.8.3). Every other transport is
        // designed either for intra-process dispatch (local) or for
        // inter-machine middleware (someip / zenoh / dds / can); forcing
        // partition IPC through them contradicts the spec's direct-
        // channel intent. Adding another same-machine IPC transport
        // (e.g. iceoryx2, a local unix socket) must flip this flag in
        // the same commit that adds the registry entry — this guard
        // fails loudly instead of the addition landing silently.
        let ipc_true: Vec<&str> = [
            "local",
            "shm",
            "someip",
            "zenoh",
            "custom_tcp",
            "dds",
            "can",
        ]
        .iter()
        .copied()
        .filter(|n| lookup(n).unwrap().supports_inter_partition_ipc)
        .collect();
        assert_eq!(ipc_true, vec!["shm", "custom_tcp"]);
    }

    #[test]
    fn only_someip_supports_multi_instance_server() {
        // SCE_MESH.md §mesh-14.4 / Gap 7: SOME/IP is the sole transport
        // whose native routing layer exposes a peer-level inbound
        // distinguisher (`msg->get_instance()`). Every other
        // transport — implemented or not — stays at `false` until a
        // new transport with an equivalent distinguisher arrives.
        // A future addition must update this regression guard in the
        // same commit that flips the flag, so the transport registry
        // and the parse-time reject stay synchronised.
        let multi_instance_true: Vec<&str> = [
            "local",
            "shm",
            "someip",
            "zenoh",
            "custom_tcp",
            "dds",
            "can",
        ]
        .iter()
        .copied()
        .filter(|n| lookup(n).unwrap().supports_multi_instance_server)
        .collect();
        assert_eq!(multi_instance_true, vec!["someip"]);
    }

    #[test]
    fn exactly_three_implemented_transports_require_runtime_dedup() {
        // Regression guard: today SOME/IP, Zenoh and DDS are the
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
            3,
            "expected exactly three implemented transports to lack inherent dedup (someip + zenoh + dds); got {undeduped_impls:?}"
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
    fn shm_fire_forget_only() {
        // shm is a one-way per-target ring with no reverse path: every
        // reply-bearing pattern (RequestReply via service.response,
        // FieldAccess via field.notify) is unrealisable, so the descriptor
        // advertises FireForget alone. See the capability comment in lookup().
        let d = lookup("shm").unwrap();
        assert!(d.capabilities.contains(&FireForget));
        assert!(!d.capabilities.contains(&FieldAccess));
        assert!(!d.capabilities.contains(&RequestReply));
        assert!(!d.capabilities.contains(&PubSub));
    }

    #[test]
    fn someip_supports_all() {
        let d = lookup("someip").unwrap();
        assert!(d.capabilities.contains(&RequestReply));
    }

    #[test]
    fn dds_realises_all_four() {
        // Request/reply rides a topic derived from the request topic;
        // FieldAccess reuses that same paired leg, which is why the two
        // cannot be advertised apart (SCE_MESH.md section 8.2).
        let d = lookup("dds").unwrap();
        for cap in &[RequestReply, FireForget, PubSub, FieldAccess] {
            assert!(d.capabilities.contains(cap), "dds should advertise {cap}");
        }
    }

    #[test]
    fn custom_tcp_realises_machine_lifetime_subscribe() {
        // Flipped together with `mesh_tcp_machine_lifetime_subscribe_runtime`,
        // which drives `machines.<name>.subscriptions:` on a subscriber
        // document carrying zero SCXML `<send>`s — so the subscribe frame it
        // reads off the wire has exactly one possible producer. Lowering this
        // back to `false` without removing that fixture would under-advertise
        // a path the tree proves; raising a sibling flag without one would be
        // the false advertising this axis exists to prevent.
        let d = lookup("custom_tcp").unwrap();
        assert!(d.supports_machine_lifetime_subscribe);
    }

    #[test]
    fn machine_lifetime_alternatives_are_read_from_the_registry() {
        // The rejection diagnostic used to name "e.g. 'zenoh'" in prose while
        // three transports realised the path. This pins the replacement to the
        // descriptor set rather than to any list: a flag flip in either
        // direction has to move the message, and no second edit can be
        // forgotten because there is no second place to edit.
        let derived = machine_lifetime_subscribe_transports();
        let expected: Vec<&str> = known_names()
            .iter()
            .filter(|n| lookup(n).unwrap().supports_machine_lifetime_subscribe)
            .copied()
            .collect();
        assert_eq!(derived, expected);

        // A transport the registry knows nothing about cannot appear, and the
        // rendered clause must name each entry the way an author would type it
        // into `transport:`.
        let rendered = machine_lifetime_subscribe_alternatives();
        for name in &derived {
            assert!(
                rendered.contains(&format!("'{name}'")),
                "{name} realises the path but is missing from the diagnostic clause"
            );
        }
        for name in known_names() {
            if !derived.contains(name) {
                assert!(
                    !rendered.contains(&format!("'{name}'")),
                    "{name} does not realise the path but the diagnostic offers it"
                );
            }
        }
    }

    /// §8.2's capability matrix is rendered from this registry, and must
    /// match it byte for byte.
    ///
    /// The table used to have patterns as rows and *categories* of
    /// transport as columns ("Req/Reply transports", "Pub/Sub
    /// transports"), which cannot answer the question an author actually
    /// has at a binding site — "what does zenoh give me" — because no row
    /// names a transport. Transposing it makes every cell a specific
    /// claim about a specific descriptor field, and a specific claim in
    /// prose drifts the first time a flag flips unless something compares
    /// the two. This is that something.
    ///
    /// The rendering lives here rather than in a doc generator so the
    /// failure lands on whoever changes the registry, in the same test
    /// run, with the corrected table in the message.
    #[test]
    fn spec_matrix_matches_the_registry() {
        const SPEC: &str = include_str!("../../../../SCE_MESH.md");
        const BEGIN: &str =
            "<!-- BEGIN transport-capability-matrix (generated from the registry) -->";
        const END: &str = "<!-- END transport-capability-matrix -->";

        fn yn(b: bool) -> &'static str {
            if b {
                "yes"
            } else {
                "no"
            }
        }
        fn patterns(d: &TransportDescriptor) -> String {
            let mut s = String::new();
            for (cap, letter) in [
                (RequestReply, 'R'),
                (FireForget, 'F'),
                (PubSub, 'P'),
                (FieldAccess, 'A'),
            ] {
                if d.capabilities.contains(&cap) {
                    s.push(letter);
                }
            }
            s
        }
        fn fields(f: &[&str]) -> String {
            if f.is_empty() {
                "—".to_string()
            } else {
                f.iter()
                    .map(|x| format!("`{x}:`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }

        // Author-facing order: the three that need no middleware, then the
        // three that route through one, then the unimplemented bus.
        const ORDER: &[&str] = &[
            "local",
            "shm",
            "custom_tcp",
            "someip",
            "zenoh",
            "dds",
            "can",
        ];
        assert_eq!(
            ORDER.len(),
            known_names().len(),
            "the matrix row order lists {} transport(s) but the registry has {} — \
             a transport that lands must appear in the table",
            ORDER.len(),
            known_names().len()
        );

        let mut rendered = String::new();
        rendered.push_str(
            "| Transport | Impl | Patterns | Required fields | Dedup | Ordering | Order-repr \
             | Pool shape | Pool member | §13 subscribe | Multi-inst server | Inter-partition IPC \
             | Cross-target reply | Server deadline |\n",
        );
        rendered.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
        for name in ORDER {
            let d = lookup(name).unwrap_or_else(|| panic!("{name} is not in the registry"));
            rendered.push_str(&format!(
                "| `{name}` | {} | {} | {} | {} | {} | {} | {:?} | {:?} | {} | {} | {} | {} | {:?} |\n",
                yn(d.implemented),
                patterns(d),
                fields(d.required_binding_fields),
                yn(d.supplies_dedup),
                yn(d.supplies_ordering),
                yn(d.ordering_representable),
                d.pool_shape,
                d.pool_member_carrier,
                yn(d.supports_machine_lifetime_subscribe),
                yn(d.supports_multi_instance_server),
                yn(d.supports_inter_partition_ipc),
                yn(d.supports_cross_target_reply),
                d.server_deadline_notice,
            ));
        }

        let start = SPEC
            .find(BEGIN)
            .expect("SCE_MESH.md §8.2 lost its matrix BEGIN marker")
            + BEGIN.len();
        let end = SPEC
            .find(END)
            .expect("SCE_MESH.md §8.2 lost its matrix END marker");
        let in_spec = SPEC[start..end].trim();

        assert_eq!(
            in_spec,
            rendered.trim(),
            "\n\nSCE_MESH.md §8.2 disagrees with the transport registry. Replace the \
             table between the BEGIN/END markers with:\n\n{}\n",
            rendered.trim()
        );
    }

    /// SCE_MESH.md may only name `TransportDescriptor::` fields that the
    /// registry actually declares.
    ///
    /// The spec is where an author learns which knobs exist, so a field
    /// named there is a promise the registry has it. Two had gone stale
    /// by the time this landed: `supports_pool`, renamed to `pool_shape`
    /// when the open/bounded distinction became author-visible, and
    /// `degraded_aspects`, which describes a schema for degraded
    /// transports that has no field because every in-tree transport is
    /// conformance-complete. Neither was caught by anything — the
    /// diagnostic-slug gate reads codes, not symbols.
    #[test]
    fn spec_names_only_registry_fields_that_exist() {
        const SPEC: &str = include_str!("../../../../SCE_MESH.md");
        let src = include_str!("mod.rs");

        // Fields of `TransportDescriptor` ONLY — read from this file so a
        // rename moves both sides, and scoped to the struct body because
        // the spec's `TransportDescriptor::x` names a field of that
        // struct. Scanning every `pub` item in the module instead would
        // accept `TransportDescriptor::supports_pool` on the strength of
        // the unrelated `PoolShape::supports_pool()` method — which is
        // exactly the stale name this gate exists to reject.
        let mut declared: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let body = src
            .split_once("pub struct TransportDescriptor {")
            .expect("TransportDescriptor struct declaration")
            .1;
        let body = body
            .split_once("\n}")
            .expect("TransportDescriptor struct terminator")
            .0;
        for line in body.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("pub ") {
                if let Some((n, _)) = rest.split_once(':') {
                    declared.insert(n);
                }
            }
        }
        assert!(
            declared.len() >= 10,
            "extracted only {} descriptor field(s) — the struct-body scan broke, and a gate \
             that reads no fields would pass on anything",
            declared.len()
        );

        let mut stale: Vec<&str> = Vec::new();
        for (_, after) in SPEC
            .match_indices("TransportDescriptor::")
            .map(|(i, m)| (i, &SPEC[i + m.len()..]))
        {
            let name: String = after
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || *c == '_')
                .collect();
            if name.is_empty() {
                continue;
            }
            if !declared.contains(name.as_str()) {
                // Leak-free lookup into the spec's own text.
                if let Some(pos) = SPEC.find(&format!("TransportDescriptor::{name}")) {
                    let _ = pos;
                }
                stale.push(Box::leak(name.into_boxed_str()));
            }
        }
        stale.sort_unstable();
        stale.dedup();
        assert!(
            stale.is_empty(),
            "SCE_MESH.md names TransportDescriptor fields the registry does not declare: {stale:?}\n\
             Either the field was renamed (update the spec) or it was never built (say so — a \
             `TransportDescriptor::x` in the spec is a claim that an author can rely on x)."
        );
    }

    #[test]
    fn pool_axes_agree_on_absence() {
        // The two pool axes are independent in what they say but not in
        // whether they say anything: a transport that admits no
        // substitution has no member type, and a transport that has a
        // member type must admit substitution. Either half alone would
        // let a descriptor advertise a carrier nothing can reach, or a
        // pool whose members have no declared type — both of which the
        // validator would then read and act on.
        for name in known_names() {
            let d = lookup(name).unwrap();
            assert_eq!(
                d.pool_shape == PoolShape::None,
                d.pool_member_carrier == PoolMemberCarrier::None,
                "{name}: pool_shape and pool_member_carrier disagree on whether \
                 this transport has a pool at all"
            );
            // A bounded shape has to name the key that enumerates its
            // members, or the validator has nothing to demand.
            if d.pool_shape.requires_member_list() {
                assert!(
                    d.pool_member_carrier.member_list_field().is_some(),
                    "{name}: bounded pool with no enumerating deploy.yaml key"
                );
            }
        }
    }

    #[test]
    fn dds_does_not_advertise_unrealised_dimensions() {
        // Guards the direction this landing had to resist: a transport
        // gaining `implemented: true` must not carry flags whose "gated
        // by implemented: false" justification just evaporated.
        let d = lookup("dds").unwrap();
        assert_eq!(
            d.pool_shape,
            PoolShape::Bounded,
            "a writer built at invoke time drops its first sample, so members \
             are declared and their endpoints built at init()"
        );
        assert_eq!(
            d.pool_member_carrier,
            PoolMemberCarrier::StringSegment,
            "a DDS topic segment is a name, not a service instance id"
        );
        assert!(
            d.supports_machine_lifetime_subscribe,
            "mesh_dds_machine_lifetime_subscribe_runtime drives deploy.yaml \
             `subscriptions:` over dds and observes the reader's teardown"
        );
        assert!(
            !d.supports_multi_instance_server,
            "pool shape is SOME/IP-only"
        );
        assert!(
            !d.supports_cross_target_reply,
            "a reply rides its own topic pair"
        );
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

    // ── server response deadline ────────────────────────────

    #[test]
    fn someip_answers_a_server_deadline_with_a_protocol_error() {
        // AUTOSAR SOME/IP reserves MT_ERROR (0x81) / E_TIMEOUT (0x06)
        // for this condition, so expiry is reported rather than
        // inferred. The distinction is author-visible — it decides
        // whether the requester sees `DeadlineExceeded` or the
        // `Unavailable` a vanished peer produces — which is why this
        // dimension is an enum and not a `supports_…: bool`.
        let d = lookup("someip").expect("known");
        assert_eq!(d.server_deadline_notice, ServerDeadlineNotice::ActiveError);
    }

    #[test]
    fn zenoh_can_only_drop_on_a_server_deadline() {
        // A `zenoh::Query` exposes `reply` and destruction, nothing in
        // between. This is a property of the query model, not a choice
        // SCE made, so a future arm that started emitting a notice here
        // would have to move this entry first.
        let d = lookup("zenoh").expect("known");
        assert_eq!(d.server_deadline_notice, ServerDeadlineNotice::DropSilently);
    }

    #[test]
    fn custom_tcp_answers_a_server_deadline_on_the_request_stream() {
        // custom_tcp defines its own framing, so there is no foreign
        // error slot to fill — the notice is SCE's own envelope written
        // back on the stashed `pending_server_links_` stream, which is
        // by construction the connection that asked.
        let d = lookup("custom_tcp").expect("known");
        assert_eq!(d.server_deadline_notice, ServerDeadlineNotice::ActiveError);
    }

    #[test]
    fn dds_answers_a_server_deadline_on_the_reply_topic() {
        // The reply topic is derived from the request topic, so the
        // notice travels the leg a normal reply already travels and the
        // requester needs no second reader to observe it.
        let d = lookup("dds").expect("known");
        assert_eq!(d.server_deadline_notice, ServerDeadlineNotice::ActiveError);
    }

    #[test]
    fn transports_with_no_realised_deadline_arm_say_so() {
        // `local` has no stored request handle to strand — its reply leg
        // is a direct call on the same stack. `shm` and `can` carry no
        // RequestReply capability at all, so there is no server arm to
        // bound. Declaring the knob on them is rejected at parse time
        // instead of being accepted into a router that ignores it.
        for name in ["local", "shm", "can"] {
            let d = lookup(name).expect("known");
            assert_eq!(
                d.server_deadline_notice,
                ServerDeadlineNotice::Unsupported,
                "{name} advertises a server deadline notice it does not emit"
            );
        }
    }

    #[test]
    fn every_transport_that_arms_a_deadline_can_serve() {
        // A deadline bounds the gap between receiving a request and
        // answering it, so a transport with no RequestReply capability
        // has nothing to bound. An entry that claimed otherwise would
        // let parse-time validation admit a knob that codegen has no
        // handler to attach to.
        for name in [
            "local",
            "shm",
            "someip",
            "zenoh",
            "custom_tcp",
            "dds",
            "can",
        ] {
            let d = lookup(name).expect("known");
            if d.server_deadline_notice != ServerDeadlineNotice::Unsupported {
                assert!(
                    d.capabilities.contains(&RequestReply),
                    "{name} arms a server response deadline without a request/reply leg"
                );
            }
        }
    }

    // ── Display ─────────────────────────────────────────────

    #[test]
    fn capability_display() {
        assert_eq!(RequestReply.to_string(), "request/reply");
        assert_eq!(PubSub.to_string(), "pub/sub");
    }

    #[test]
    fn server_deadline_notice_display() {
        assert_eq!(ServerDeadlineNotice::Unsupported.to_string(), "unsupported");
        assert_eq!(
            ServerDeadlineNotice::DropSilently.to_string(),
            "silent drop"
        );
        assert_eq!(
            ServerDeadlineNotice::ActiveError.to_string(),
            "active error reply"
        );
    }
}
