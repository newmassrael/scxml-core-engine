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
    /// `event.notification` — Received event from subscription (Pub/Sub delivery).
    Notification,
    /// `field.get` — Read a named data field (property access).
    FieldGet,
    /// `field.set` — Write a named data field (property access).
    FieldSet,
}

impl CommunicationPattern {
    /// The transport capability category this pattern requires.
    pub fn required_capability(self) -> TransportCapability {
        match self {
            Self::ServiceRequest | Self::ServiceResponse => TransportCapability::RequestReply,
            Self::FireForget => TransportCapability::FireForget,
            Self::Subscribe | Self::Notification => TransportCapability::PubSub,
            Self::FieldGet | Self::FieldSet => TransportCapability::FieldAccess,
        }
    }

    /// Wire value corresponding to the C++ `SCE::Mesh::PatternKind` enum.
    ///
    /// Single source of truth for the Rust → C++ wire value mapping.
    /// Values MUST match `sce/include/mesh/PatternKind.h`; any drift is caught
    /// by the C++ static_asserts in that header.
    ///
    /// Note: `CommunicationPattern` is the SCXML-detectable subset (7 variants).
    /// C++ `PatternKind` has 9 variants — `EventUnsubscribe` (5) is lifecycle-
    /// emitted by codegen (not detected from `<send>`), and `FieldNotify` (9)
    /// is inbound-only (arrives via `register_message_handler`, never sent).
    /// Hence neither appears here; they exist only on the C++ side.
    pub fn wire_value(self) -> u16 {
        match self {
            Self::FireForget       => 1,
            Self::ServiceRequest   => 2,
            Self::ServiceResponse  => 3,
            Self::Subscribe        => 4,
            Self::Notification     => 6,
            Self::FieldGet         => 7,
            Self::FieldSet         => 8,
        }
    }
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
        match self {
            Self::ServiceRequest => write!(f, "service.request"),
            Self::ServiceResponse => write!(f, "service.response"),
            Self::FireForget => write!(f, "service.fire_forget"),
            Self::Subscribe => write!(f, "event.subscribe"),
            Self::Notification => write!(f, "event.notification"),
            Self::FieldGet => write!(f, "field.get"),
            Self::FieldSet => write!(f, "field.set"),
        }
    }
}

// ── Pattern detection ───────────────────────────────────────

/// Ordered from longest to shortest prefix to avoid false matches
/// (e.g., `service.request` must not match `service.response`).
const PATTERN_PREFIXES: &[(&str, CommunicationPattern)] = &[
    ("service.fire_forget", CommunicationPattern::FireForget),
    ("service.request", CommunicationPattern::ServiceRequest),
    ("service.response", CommunicationPattern::ServiceResponse),
    ("event.notification", CommunicationPattern::Notification),
    ("event.subscribe", CommunicationPattern::Subscribe),
    ("field.get", CommunicationPattern::FieldGet),
    ("field.set", CommunicationPattern::FieldSet),
];

/// Detect communication pattern from explicit `sce:pattern` attribute or
/// event name prefix convention.
///
/// Resolution order:
///   1. Explicit annotation: `<send sce:pattern="request"/>` → Ok(Some(ServiceRequest))
///   2. Event name prefix: `"service.request.brake_status"` → Ok(Some(ServiceRequest))
///   3. Explicit "none": → Ok(None) (opt-out)
///   4. No match → Ok(None) (application-specific event, no validation)
///   5. Unrecognized explicit value → Err (likely typo, build error)
pub fn detect_pattern(
    event: &str,
    explicit_pattern: Option<&str>,
) -> Result<Option<CommunicationPattern>, String> {
    // Priority 1: explicit sce:pattern attribute
    if let Some(value) = explicit_pattern {
        return parse_explicit_pattern(value);
    }

    // Priority 2: convention-based prefix matching
    Ok(detect_pattern_from_event(event))
}

/// Parse an explicit `sce:pattern` attribute value.
///
/// Returns:
///   Ok(Some(pattern)) — recognized pattern
///   Ok(None) — explicit "none" (opt-out from validation)
///   Err(value) — unrecognized value (likely typo, caller should emit build error)
fn parse_explicit_pattern(value: &str) -> Result<Option<CommunicationPattern>, String> {
    match value.to_ascii_lowercase().as_str() {
        "request" | "service.request" => Ok(Some(CommunicationPattern::ServiceRequest)),
        "response" | "service.response" => Ok(Some(CommunicationPattern::ServiceResponse)),
        "fire_forget" | "service.fire_forget" => Ok(Some(CommunicationPattern::FireForget)),
        "subscribe" | "event.subscribe" => Ok(Some(CommunicationPattern::Subscribe)),
        "notification" | "event.notification" => Ok(Some(CommunicationPattern::Notification)),
        "field_get" | "field.get" => Ok(Some(CommunicationPattern::FieldGet)),
        "field_set" | "field.set" => Ok(Some(CommunicationPattern::FieldSet)),
        "none" => Ok(None),
        _ => Err(value.to_string()),
    }
}

/// Detect communication pattern from event name prefix convention only.
///
/// Matching rules:
///   - Exact match: `"service.request"` → ServiceRequest
///   - Prefix + dot: `"service.request.brake_status"` → ServiceRequest
///   - No match: `"brake.activate"` → None
fn detect_pattern_from_event(event: &str) -> Option<CommunicationPattern> {
    for &(prefix, pattern) in PATTERN_PREFIXES {
        if event == prefix {
            return Some(pattern);
        }
        if event.len() > prefix.len()
            && event.starts_with(prefix)
            && event.as_bytes()[prefix.len()] == b'.'
        {
            return Some(pattern);
        }
    }
    None
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
    pub target: String,
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

    // ── detect_pattern (convention fallback, no explicit) ──

    #[test]
    fn detect_exact_service_request() {
        assert_eq!(
            detect_pattern("service.request", None).unwrap(),
            Some(CommunicationPattern::ServiceRequest)
        );
    }

    #[test]
    fn detect_prefixed_service_request() {
        assert_eq!(
            detect_pattern("service.request.brake_status", None).unwrap(),
            Some(CommunicationPattern::ServiceRequest)
        );
    }

    #[test]
    fn detect_service_response() {
        assert_eq!(
            detect_pattern("service.response", None).unwrap(),
            Some(CommunicationPattern::ServiceResponse)
        );
        assert_eq!(
            detect_pattern("service.response.brake_status", None).unwrap(),
            Some(CommunicationPattern::ServiceResponse)
        );
    }

    #[test]
    fn detect_fire_forget() {
        assert_eq!(
            detect_pattern("service.fire_forget", None).unwrap(),
            Some(CommunicationPattern::FireForget)
        );
        assert_eq!(
            detect_pattern("service.fire_forget.motor_cmd", None).unwrap(),
            Some(CommunicationPattern::FireForget)
        );
    }

    #[test]
    fn detect_subscribe() {
        assert_eq!(
            detect_pattern("event.subscribe", None).unwrap(),
            Some(CommunicationPattern::Subscribe)
        );
        assert_eq!(
            detect_pattern("event.subscribe.speed_updates", None).unwrap(),
            Some(CommunicationPattern::Subscribe)
        );
    }

    #[test]
    fn detect_notification() {
        assert_eq!(
            detect_pattern("event.notification", None).unwrap(),
            Some(CommunicationPattern::Notification)
        );
        assert_eq!(
            detect_pattern("event.notification.speed_changed", None).unwrap(),
            Some(CommunicationPattern::Notification)
        );
    }

    #[test]
    fn detect_field_get() {
        assert_eq!(
            detect_pattern("field.get", None).unwrap(),
            Some(CommunicationPattern::FieldGet)
        );
        assert_eq!(
            detect_pattern("field.get.vehicle_speed", None).unwrap(),
            Some(CommunicationPattern::FieldGet)
        );
    }

    #[test]
    fn detect_field_set() {
        assert_eq!(
            detect_pattern("field.set", None).unwrap(),
            Some(CommunicationPattern::FieldSet)
        );
        assert_eq!(
            detect_pattern("field.set.target_speed", None).unwrap(),
            Some(CommunicationPattern::FieldSet)
        );
    }

    #[test]
    fn detect_none_for_application_events() {
        assert_eq!(detect_pattern("brake.activate", None).unwrap(), None);
        assert_eq!(detect_pattern("motor.start", None).unwrap(), None);
        assert_eq!(detect_pattern("error.communication", None).unwrap(), None);
    }

    #[test]
    fn detect_none_for_partial_prefix() {
        assert_eq!(detect_pattern("service.requestX", None).unwrap(), None);
        assert_eq!(detect_pattern("field.getAll", None).unwrap(), None);
        assert_eq!(detect_pattern("event.subscribeNow", None).unwrap(), None);
    }

    #[test]
    fn detect_none_for_empty_event() {
        assert_eq!(detect_pattern("", None).unwrap(), None);
    }

    // ── detect_pattern (explicit annotation) ────────────────

    #[test]
    fn explicit_overrides_convention() {
        assert_eq!(
            detect_pattern("service.request.brake", Some("fire_forget")).unwrap(),
            Some(CommunicationPattern::FireForget)
        );
    }

    #[test]
    fn explicit_none_opts_out() {
        assert_eq!(
            detect_pattern("service.request.brake", Some("none")).unwrap(),
            None
        );
    }

    #[test]
    fn explicit_short_forms() {
        assert_eq!(
            detect_pattern("any.event", Some("request")).unwrap(),
            Some(CommunicationPattern::ServiceRequest)
        );
        assert_eq!(
            detect_pattern("any.event", Some("response")).unwrap(),
            Some(CommunicationPattern::ServiceResponse)
        );
        assert_eq!(
            detect_pattern("any.event", Some("subscribe")).unwrap(),
            Some(CommunicationPattern::Subscribe)
        );
        assert_eq!(
            detect_pattern("any.event", Some("notification")).unwrap(),
            Some(CommunicationPattern::Notification)
        );
        assert_eq!(
            detect_pattern("any.event", Some("field_get")).unwrap(),
            Some(CommunicationPattern::FieldGet)
        );
        assert_eq!(
            detect_pattern("any.event", Some("field_set")).unwrap(),
            Some(CommunicationPattern::FieldSet)
        );
    }

    #[test]
    fn explicit_long_forms() {
        assert_eq!(
            detect_pattern("any.event", Some("service.request")).unwrap(),
            Some(CommunicationPattern::ServiceRequest)
        );
        assert_eq!(
            detect_pattern("any.event", Some("event.subscribe")).unwrap(),
            Some(CommunicationPattern::Subscribe)
        );
        assert_eq!(
            detect_pattern("any.event", Some("field.get")).unwrap(),
            Some(CommunicationPattern::FieldGet)
        );
    }

    #[test]
    fn explicit_case_insensitive() {
        assert_eq!(
            detect_pattern("any.event", Some("Request")).unwrap(),
            Some(CommunicationPattern::ServiceRequest)
        );
        assert_eq!(
            detect_pattern("any.event", Some("FIRE_FORGET")).unwrap(),
            Some(CommunicationPattern::FireForget)
        );
    }

    #[test]
    fn explicit_unrecognized_value_is_error() {
        let err = detect_pattern("any.event", Some("requst")).unwrap_err();
        assert_eq!(err, "requst");
    }

    #[test]
    fn explicit_typo_does_not_silently_disable() {
        // "subscribee" is a typo — must be Err, not silent Ok(None)
        assert!(detect_pattern("any.event", Some("subscribee")).is_err());
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

    // ── Display ─────────────────────────────────────────────

    #[test]
    fn pattern_display() {
        assert_eq!(CommunicationPattern::ServiceRequest.to_string(), "service.request");
        assert_eq!(CommunicationPattern::FieldSet.to_string(), "field.set");
    }

    #[test]
    fn violation_display() {
        let v = PatternViolation {
            state: "braking".to_string(),
            target: "#motor".to_string(),
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
