// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh Communication Pattern Semantics (SCE_MESH.md Section 8.1).
//
// Transport-agnostic event vocabulary that SCXML authors use. Each transport
// template maps these patterns to native API calls. Pattern detection from
// `<send event="...">` enables build-time validation that the bound transport
// supports the required communication pattern (Section 8.2).

use std::fmt;

use super::transport::TransportCapability;

// ── SOME/IP field kinds ─────────────────────────────────────

/// The family of SOME/IP numeric slot a pattern binds to.
///
/// Every recognized pattern maps to exactly one kind (see
/// [`CommunicationPattern::someip_field`]). The kind is what
/// resolution, validation, and codegen dispatch on — not the raw
/// pattern, and not the probing of which `Option` field happens
/// to be populated.
///
/// `EventGroup` additionally requires a contained `event_id`; the
/// tagged-enum per-event value type (`topology::SomeipEventIds`)
/// carries both as part of the same variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SomeipFieldKind {
    /// RPC/FireForget methods (`services[*].methods[*]`).
    Method,
    /// Subscribe/Notification event groups (`services[*].eventgroups[*]`).
    EventGroup,
    /// Field read (`services[*].methods[*]` treated as getter).
    Getter,
    /// Field write (`services[*].methods[*]` treated as setter).
    Setter,
}

// ── Communication patterns ──────────────────────────────────

/// Communication patterns recognized from SCXML `<send event="...">` names.
///
/// Convention: the event name prefix determines the pattern. For example,
/// `service.request.brake_status` has prefix `service.request` and is
/// recognized as `ServiceRequest`. Events without a known prefix are
/// application-specific and not subject to pattern capability validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommunicationPattern {
    /// `service.request` — Request/Response, expects reply (method call, RPC).
    ServiceRequest,
    /// `service.response` — Reply to a prior request (method return).
    ServiceResponse,
    /// `service.fire_forget` — One-way send, no reply expected.
    FireForget,
    /// `event.subscribe` — Register interest in a topic/event group (Pub/Sub setup).
    Subscribe,
    /// `event.unsubscribe` — Cancel interest in a topic/event group (Pub/Sub teardown).
    /// Lifecycle-emitted by codegen for auto-symmetry (`<onexit>` unsubscribe
    /// paired with `<onentry>` subscribe — SCE_MESH.md §13). Authors may also
    /// write explicit `<send event="event.unsubscribe.*">` when auto-symmetry
    /// does not apply (conditional subscribe, manual lifecycle management).
    Unsubscribe,
    /// `event.notification` — Received event from subscription (Pub/Sub delivery).
    Notification,
    /// `field.get` — Read a named data field (property access).
    FieldGet,
    /// `field.set` — Write a named data field (property access).
    FieldSet,
}

impl CommunicationPattern {
    /// Every variant, in a stable order suitable for iteration.
    ///
    /// Consumers that want to scan every pattern (e.g. `detect_pattern`) do
    /// so through this constant instead of rewriting match arms — there is
    /// one place to add a variant when the vocabulary grows.
    pub const ALL: &'static [Self] = &[
        Self::FireForget,
        Self::ServiceRequest,
        Self::ServiceResponse,
        Self::Subscribe,
        Self::Unsubscribe,
        Self::Notification,
        Self::FieldGet,
        Self::FieldSet,
    ];

    /// The reserved event-name prefix that identifies this pattern.
    ///
    /// Single source of truth for the pattern vocabulary — `Display`,
    /// `match_suffix`, and `infer_reply_event` all derive from this value.
    /// `const fn` so the compiler can fold it at call sites.
    pub const fn prefix_str(self) -> &'static str {
        match self {
            Self::FireForget      => "service.fire_forget",
            Self::ServiceRequest  => "service.request",
            Self::ServiceResponse => "service.response",
            Self::Subscribe       => "event.subscribe",
            Self::Unsubscribe     => "event.unsubscribe",
            Self::Notification    => "event.notification",
            Self::FieldGet        => "field.get",
            Self::FieldSet        => "field.set",
        }
    }

    /// The transport capability category this pattern requires.
    pub fn required_capability(self) -> TransportCapability {
        match self {
            Self::ServiceRequest | Self::ServiceResponse => TransportCapability::RequestReply,
            Self::FireForget => TransportCapability::FireForget,
            Self::Subscribe | Self::Unsubscribe | Self::Notification => TransportCapability::PubSub,
            Self::FieldGet | Self::FieldSet => TransportCapability::FieldAccess,
        }
    }

    /// Which SOME/IP numeric-ID slot this pattern requires at runtime.
    ///
    /// Single source of truth for "pattern → required SOME/IP field" —
    /// resolution fan-out, validation, and codegen dispatch all read this
    /// instead of duplicating a `match` over `CommunicationPattern` (or
    /// worse, over `PatternKind` wire values). Adding a new pattern variant
    /// is a single edit here.
    ///
    /// Every recognized pattern addresses **exactly one** slot family —
    /// `Method`, `EventGroup`, `Getter`, or `Setter`. If a future pattern
    /// needs zero slots or multiple slots, this returns type must change
    /// (e.g. `&'static [SomeipFieldKind]`) and the callers that currently
    /// rely on infallibility need to adapt.
    pub fn someip_field(self) -> SomeipFieldKind {
        use SomeipFieldKind::*;
        match self {
            Self::FireForget | Self::ServiceRequest | Self::ServiceResponse => Method,
            Self::Subscribe | Self::Unsubscribe | Self::Notification => EventGroup,
            Self::FieldGet => Getter,
            Self::FieldSet => Setter,
        }
    }

    /// If `event` belongs to this pattern — either the bare prefix or
    /// `<prefix>.<suffix>` — return the suffix (empty for the bare form).
    /// Otherwise `None`. Boundary check is strict: `service.requestX`
    /// does not match `ServiceRequest` because `X` is not preceded by `.`.
    pub fn match_suffix(self, event: &str) -> Option<&str> {
        let prefix = self.prefix_str();
        if event == prefix {
            return Some("");
        }
        event.strip_prefix(prefix)?.strip_prefix('.')
    }

    /// Wire value corresponding to the C++ `SCE::Mesh::PatternKind` enum.
    ///
    /// Single source of truth for the Rust → C++ wire value mapping.
    /// Values MUST match `sce/include/mesh/PatternKind.h`; any drift is caught
    /// by the C++ static_asserts in that header.
    ///
    /// Note: `CommunicationPattern` covers 8 of the 9 C++ `PatternKind`
    /// variants. `FieldNotify` (9) is inbound-only (arrives via
    /// `register_message_handler`, never sent from SCXML) and has no
    /// Rust counterpart.
    pub fn wire_value(self) -> u16 {
        match self {
            Self::FireForget       => 1,
            Self::ServiceRequest   => 2,
            Self::ServiceResponse  => 3,
            Self::Subscribe        => 4,
            Self::Unsubscribe      => 5,
            Self::Notification     => 6,
            Self::FieldGet         => 7,
            Self::FieldSet         => 8,
        }
    }

    /// Inverse of [`wire_value`]. Returns `None` for wire values that
    /// exist in the C++ `PatternKind` enum but have no Rust counterpart
    /// (currently only `FieldNotify` = 9, which is inbound-only).
    ///
    /// Consumers that need to recover the symbolic pattern from a cached
    /// `pattern_kind_value` (e.g. validators) go through this — no open-coded
    /// wire → variant `match` elsewhere.
    pub fn from_wire(v: u16) -> Option<Self> {
        Self::ALL.iter().copied().find(|p| p.wire_value() == v)
    }

    /// Derive the paired reply event name for an RPC request by convention
    /// (SCE_MESH.md §13 path B). Returns `None` for patterns that have no
    /// reply semantics or for events that do not match this variant's prefix.
    ///
    /// Only `ServiceRequest` has a paired reply under path B — other
    /// patterns either never reply (`FireForget`, `FieldSet`, `Subscribe`,
    /// `Unsubscribe`) or ARE the reply themselves (`ServiceResponse`,
    /// `Notification`, `FieldGet`).  The paired reply is built from
    /// `ServiceResponse.prefix_str()`; no literal strings are duplicated.
    pub fn infer_reply_event(self, event: &str) -> Option<String> {
        if self != Self::ServiceRequest {
            return None;
        }
        let suffix = self.match_suffix(event)?;
        let resp = Self::ServiceResponse.prefix_str();
        Some(if suffix.is_empty() {
            resp.to_string()
        } else {
            format!("{resp}.{suffix}")
        })
    }
}

/// Derive the paired unsubscribe event name from a subscribe event name.
///
/// `event.subscribe.X` → `Some("event.unsubscribe.X")`
/// `event.subscribe`   → `Some("event.unsubscribe")`
/// anything else        → `None`
///
/// SCE_MESH.md §13 auto-symmetry: the topology analyzer uses this to
/// synthesize `<onexit>` unsubscribe sends from `<onentry>` subscribe sends.
pub fn subscribe_to_unsubscribe(event: &str) -> Option<String> {
    let suffix = CommunicationPattern::Subscribe.match_suffix(event)?;
    let unsub = CommunicationPattern::Unsubscribe.prefix_str();
    Some(if suffix.is_empty() {
        unsub.to_string()
    } else {
        format!("{unsub}.{suffix}")
    })
}

// ── Wire value constants (C++ PatternKind.h mirror) ─────────
//
// Wire-stable values used by codegen for pattern category detection. Mirrors
// PatternKind.h exactly. Never reuse once shipped. Use these instead of
// literal magic numbers when comparing against encoded wire values.

/// `PatternKind::FireForget` wire value.
pub const WIRE_FIRE_FORGET:        u16 = 1;
/// `PatternKind::RpcRequest` wire value.
pub const WIRE_RPC_REQUEST:        u16 = 2;
/// `PatternKind::RpcReply` wire value.
pub const WIRE_RPC_REPLY:          u16 = 3;
/// `PatternKind::EventSubscribe` wire value.
pub const WIRE_EVENT_SUBSCRIBE:    u16 = 4;
/// `PatternKind::EventUnsubscribe` wire value.
pub const WIRE_EVENT_UNSUBSCRIBE:  u16 = 5;
/// `PatternKind::EventNotify` wire value.
pub const WIRE_EVENT_NOTIFY:       u16 = 6;
/// `PatternKind::FieldRead` wire value.
pub const WIRE_FIELD_READ:         u16 = 7;
/// `PatternKind::FieldWrite` wire value.
pub const WIRE_FIELD_WRITE:        u16 = 8;
/// `PatternKind::FieldNotify` wire value.
pub const WIRE_FIELD_NOTIFY:       u16 = 9;

impl fmt::Display for CommunicationPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.prefix_str())
    }
}

// ── Pattern detection ───────────────────────────────────────

/// Detect communication pattern from event name prefix convention.
///
/// SCE_MESH.md §13 path B (SCXML purity): pattern annotations are no longer
/// carried as `sce:pattern` attributes. The event name itself is the pattern
/// declaration — the reserved prefixes returned by `prefix_str` form the
/// stable vocabulary that SCXML authors commit to.
///
/// Matching rules (delegated to `CommunicationPattern::match_suffix`):
///   - Exact match: `"service.request"` → Some(ServiceRequest)
///   - Prefix + dot: `"service.request.brake_status"` → Some(ServiceRequest)
///   - No match: `"brake.activate"` → None (application-specific event)
///
/// No two reserved prefixes share a common prefix, so variant iteration
/// order is irrelevant for correctness.
pub fn detect_pattern(event: &str) -> Option<CommunicationPattern> {
    CommunicationPattern::ALL
        .iter()
        .copied()
        .find(|p| p.match_suffix(event).is_some())
}


// ── Pattern violation ───────────────────────────────────────

/// A single pattern/transport capability mismatch detected at build time.
///
/// Produced by `validate_pattern_capability()` in topology.rs. When non-empty,
/// the mesh pipeline emits a build error (not a warning) per SCE_MESH.md §8.2.
#[derive(Debug, Clone)]
pub struct PatternViolation {
    /// The state containing the `<send>`.
    pub state: String,
    /// The target of the `<send>` (e.g. "#motor").
    pub target: super::target::TargetId,
    /// The event name of the `<send>`.
    pub event: String,
    /// The detected communication pattern.
    pub pattern: CommunicationPattern,
    /// The required capability that the transport lacks.
    pub required: TransportCapability,
    /// The transport type from deploy.yaml.
    pub transport: String,
}

impl fmt::Display for PatternViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "send target=\"{}\" event=\"{}\" uses pattern '{}' (requires {} capability), \
             but transport '{}' does not support it",
            self.target, self.event, self.pattern, self.required, self.transport
        )
    }
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── detect_pattern (convention-only) ──────────────────

    #[test]
    fn detect_exact_service_request() {
        assert_eq!(
            detect_pattern("service.request"),
            Some(CommunicationPattern::ServiceRequest)
        );
    }

    #[test]
    fn detect_prefixed_service_request() {
        assert_eq!(
            detect_pattern("service.request.brake_status"),
            Some(CommunicationPattern::ServiceRequest)
        );
    }

    #[test]
    fn detect_service_response() {
        assert_eq!(
            detect_pattern("service.response"),
            Some(CommunicationPattern::ServiceResponse)
        );
        assert_eq!(
            detect_pattern("service.response.brake_status"),
            Some(CommunicationPattern::ServiceResponse)
        );
    }

    #[test]
    fn detect_fire_forget() {
        assert_eq!(
            detect_pattern("service.fire_forget"),
            Some(CommunicationPattern::FireForget)
        );
        assert_eq!(
            detect_pattern("service.fire_forget.motor_cmd"),
            Some(CommunicationPattern::FireForget)
        );
    }

    #[test]
    fn detect_subscribe() {
        assert_eq!(
            detect_pattern("event.subscribe"),
            Some(CommunicationPattern::Subscribe)
        );
        assert_eq!(
            detect_pattern("event.subscribe.speed_updates"),
            Some(CommunicationPattern::Subscribe)
        );
    }

    #[test]
    fn detect_unsubscribe() {
        assert_eq!(
            detect_pattern("event.unsubscribe"),
            Some(CommunicationPattern::Unsubscribe)
        );
        assert_eq!(
            detect_pattern("event.unsubscribe.speed_updates"),
            Some(CommunicationPattern::Unsubscribe)
        );
    }

    #[test]
    fn detect_notification() {
        assert_eq!(
            detect_pattern("event.notification"),
            Some(CommunicationPattern::Notification)
        );
        assert_eq!(
            detect_pattern("event.notification.speed_changed"),
            Some(CommunicationPattern::Notification)
        );
    }

    #[test]
    fn detect_field_get() {
        assert_eq!(
            detect_pattern("field.get"),
            Some(CommunicationPattern::FieldGet)
        );
        assert_eq!(
            detect_pattern("field.get.vehicle_speed"),
            Some(CommunicationPattern::FieldGet)
        );
    }

    #[test]
    fn detect_field_set() {
        assert_eq!(
            detect_pattern("field.set"),
            Some(CommunicationPattern::FieldSet)
        );
        assert_eq!(
            detect_pattern("field.set.target_speed"),
            Some(CommunicationPattern::FieldSet)
        );
    }

    #[test]
    fn detect_none_for_application_events() {
        assert_eq!(detect_pattern("brake.activate"), None);
        assert_eq!(detect_pattern("motor.start"), None);
        assert_eq!(detect_pattern("error.communication"), None);
    }

    #[test]
    fn detect_none_for_partial_prefix() {
        assert_eq!(detect_pattern("service.requestX"), None);
        assert_eq!(detect_pattern("field.getAll"), None);
        assert_eq!(detect_pattern("event.subscribeNow"), None);
    }

    #[test]
    fn detect_none_for_empty_event() {
        assert_eq!(detect_pattern(""), None);
    }

    // ── CommunicationPattern::infer_reply_event ────────────

    #[test]
    fn infer_reply_event_from_request_prefix() {
        assert_eq!(
            CommunicationPattern::ServiceRequest
                .infer_reply_event("service.request.compute_force")
                .as_deref(),
            Some("service.response.compute_force")
        );
    }

    #[test]
    fn infer_reply_event_exact_prefix() {
        assert_eq!(
            CommunicationPattern::ServiceRequest
                .infer_reply_event("service.request")
                .as_deref(),
            Some("service.response")
        );
    }

    #[test]
    fn infer_reply_event_none_for_non_request_pattern() {
        // The method is only meaningful on ServiceRequest; every other
        // variant returns None regardless of the event string.
        for p in [
            CommunicationPattern::ServiceResponse,
            CommunicationPattern::FireForget,
            CommunicationPattern::Subscribe,
            CommunicationPattern::Unsubscribe,
            CommunicationPattern::Notification,
            CommunicationPattern::FieldGet,
            CommunicationPattern::FieldSet,
        ] {
            assert_eq!(p.infer_reply_event("service.request.x"), None,
                       "pattern {p:?} must not produce a reply");
        }
    }

    #[test]
    fn infer_reply_event_none_for_non_request_event_name() {
        let req = CommunicationPattern::ServiceRequest;
        assert_eq!(req.infer_reply_event("service.response.x"), None);
        assert_eq!(req.infer_reply_event("event.subscribe.status"), None);
        assert_eq!(req.infer_reply_event("brake.activate"), None);
    }

    #[test]
    fn infer_reply_event_none_for_partial_prefix_match() {
        // "service.requestX" is not a valid request event (no dot separator).
        let req = CommunicationPattern::ServiceRequest;
        assert_eq!(req.infer_reply_event("service.requestX"), None);
    }

    // ── required_capability ─────────────────────────────────

    #[test]
    fn request_response_capability() {
        assert_eq!(
            CommunicationPattern::ServiceRequest.required_capability(),
            TransportCapability::RequestReply
        );
        assert_eq!(
            CommunicationPattern::ServiceResponse.required_capability(),
            TransportCapability::RequestReply
        );
    }

    #[test]
    fn fire_forget_capability() {
        assert_eq!(
            CommunicationPattern::FireForget.required_capability(),
            TransportCapability::FireForget
        );
    }

    #[test]
    fn pubsub_capability() {
        assert_eq!(
            CommunicationPattern::Subscribe.required_capability(),
            TransportCapability::PubSub
        );
        assert_eq!(
            CommunicationPattern::Unsubscribe.required_capability(),
            TransportCapability::PubSub
        );
        assert_eq!(
            CommunicationPattern::Notification.required_capability(),
            TransportCapability::PubSub
        );
    }

    #[test]
    fn field_access_capability() {
        assert_eq!(
            CommunicationPattern::FieldGet.required_capability(),
            TransportCapability::FieldAccess
        );
        assert_eq!(
            CommunicationPattern::FieldSet.required_capability(),
            TransportCapability::FieldAccess
        );
    }

    // ── someip_field ────────────────────────────────────────

    #[test]
    fn someip_field_method_family() {
        use SomeipFieldKind::Method;
        assert_eq!(CommunicationPattern::FireForget.someip_field(), Method);
        assert_eq!(CommunicationPattern::ServiceRequest.someip_field(), Method);
        assert_eq!(CommunicationPattern::ServiceResponse.someip_field(), Method);
    }

    #[test]
    fn someip_field_event_group_family() {
        use SomeipFieldKind::EventGroup;
        assert_eq!(CommunicationPattern::Subscribe.someip_field(), EventGroup);
        assert_eq!(CommunicationPattern::Unsubscribe.someip_field(), EventGroup);
        assert_eq!(CommunicationPattern::Notification.someip_field(), EventGroup);
    }

    #[test]
    fn someip_field_getter_setter_family() {
        use SomeipFieldKind::{Getter, Setter};
        assert_eq!(CommunicationPattern::FieldGet.someip_field(), Getter);
        assert_eq!(CommunicationPattern::FieldSet.someip_field(), Setter);
    }

    #[test]
    fn someip_field_exhaustive_across_all_variants() {
        // Every recognized pattern must return some SomeipFieldKind. The
        // infallible return type already guarantees this at compile time;
        // this test also exercises the call on every variant in the ALL
        // catalogue so a future variant added to `ALL` without a match arm
        // in `someip_field` fails the test instead of panicking in prod.
        for p in CommunicationPattern::ALL.iter().copied() {
            let _ = p.someip_field();
        }
    }

    // ── Display ─────────────────────────────────────────────

    // ── subscribe_to_unsubscribe ──────────────────────────────

    #[test]
    fn subscribe_to_unsubscribe_with_suffix() {
        assert_eq!(
            subscribe_to_unsubscribe("event.subscribe.brake_status"),
            Some("event.unsubscribe.brake_status".to_string())
        );
    }

    #[test]
    fn subscribe_to_unsubscribe_bare_prefix() {
        assert_eq!(
            subscribe_to_unsubscribe("event.subscribe"),
            Some("event.unsubscribe".to_string())
        );
    }

    #[test]
    fn subscribe_to_unsubscribe_none_for_non_subscribe() {
        assert_eq!(subscribe_to_unsubscribe("event.notification.x"), None);
        assert_eq!(subscribe_to_unsubscribe("service.request.x"), None);
        assert_eq!(subscribe_to_unsubscribe("brake.activate"), None);
    }

    // ── Display ─────────────────────────────────────────────

    #[test]
    fn pattern_display() {
        assert_eq!(CommunicationPattern::ServiceRequest.to_string(), "service.request");
        assert_eq!(CommunicationPattern::Unsubscribe.to_string(), "event.unsubscribe");
        assert_eq!(CommunicationPattern::FieldSet.to_string(), "field.set");
    }

    #[test]
    fn violation_display() {
        let v = PatternViolation {
            state: "braking".to_string(),
            target: super::super::target::TargetId::new("#motor").unwrap(),
            event: "service.request.brake_status".to_string(),
            pattern: CommunicationPattern::ServiceRequest,
            required: TransportCapability::RequestReply,
            transport: "can".to_string(),
        };
        let s = v.to_string();
        assert!(s.contains("service.request"));
        assert!(s.contains("request/reply"));
        assert!(s.contains("can"));
    }
}
