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
}

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

// ── Transport capabilities ──────────────────────────────────

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
///   1. Explicit annotation: `<send sce:pattern="request"/>` → ServiceRequest
///   2. Event name prefix: `"service.request.brake_status"` → ServiceRequest
///   3. No match → None (application-specific event, no validation)
///
/// The explicit annotation overrides convention. This handles cases where
/// an event name incidentally matches a prefix but has different semantics.
pub fn detect_pattern(event: &str, explicit_pattern: &str) -> Option<CommunicationPattern> {
    // Priority 1: explicit sce:pattern attribute
    if !explicit_pattern.is_empty() {
        return parse_explicit_pattern(explicit_pattern);
    }

    // Priority 2: convention-based prefix matching
    detect_pattern_from_event(event)
}

/// Parse an explicit `sce:pattern` attribute value into a CommunicationPattern.
///
/// Accepted values (case-insensitive, matching SCE_MESH.md §8.1):
///   "request" | "service.request"      → ServiceRequest
///   "response" | "service.response"    → ServiceResponse
///   "fire_forget" | "service.fire_forget" → FireForget
///   "subscribe" | "event.subscribe"    → Subscribe
///   "notification" | "event.notification" → Notification
///   "field_get" | "field.get"          → FieldGet
///   "field_set" | "field.set"          → FieldSet
///   "none"                             → None (explicit opt-out)
fn parse_explicit_pattern(value: &str) -> Option<CommunicationPattern> {
    match value.to_ascii_lowercase().as_str() {
        "request" | "service.request" => Some(CommunicationPattern::ServiceRequest),
        "response" | "service.response" => Some(CommunicationPattern::ServiceResponse),
        "fire_forget" | "service.fire_forget" => Some(CommunicationPattern::FireForget),
        "subscribe" | "event.subscribe" => Some(CommunicationPattern::Subscribe),
        "notification" | "event.notification" => Some(CommunicationPattern::Notification),
        "field_get" | "field.get" => Some(CommunicationPattern::FieldGet),
        "field_set" | "field.set" => Some(CommunicationPattern::FieldSet),
        "none" => None, // Explicit opt-out from pattern validation
        _ => None,      // Unrecognized value — treated as no pattern
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

// ── Transport capability declarations ───────────────────────

/// Known transport capabilities (SCE_MESH.md Section 8.2 Transport Capability Matrix).
///
/// Returns `None` for unknown/custom transports (conservative: no validation).
/// Returns `Some(capabilities)` for transports whose pattern support is known
/// at compile time.
pub fn transport_capabilities(transport: &str) -> Option<&'static [TransportCapability]> {
    use TransportCapability::*;
    match transport {
        // Same-process direct call — all patterns are trivially supported.
        "local" => Some(&[RequestReply, FireForget, PubSub, FieldAccess]),
        // Shared memory — signal-like semantics: fire-and-forget + field access.
        "shm" => Some(&[FireForget, FieldAccess]),
        // SOME/IP — full automotive service model.
        "someip" => Some(&[RequestReply, FireForget, PubSub, FieldAccess]),
        // DDS — pub/sub with topic-based field access, no native request/reply.
        "dds" => Some(&[FireForget, PubSub, FieldAccess]),
        // Zenoh — pub/sub with key-expression field access, no native request/reply.
        "zenoh" => Some(&[FireForget, PubSub, FieldAccess]),
        // CAN — signal-based: fire-and-forget + DBC field access.
        "can" => Some(&[FireForget, FieldAccess]),
        // Unknown/custom transports: no validation (conservative).
        _ => None,
    }
}

/// Check whether a transport supports a specific capability.
///
/// Returns `true` if the transport is known and supports the capability,
/// or if the transport is unknown (conservative: assume supported).
pub fn transport_supports(transport: &str, capability: TransportCapability) -> bool {
    match transport_capabilities(transport) {
        Some(caps) => caps.contains(&capability),
        None => true, // Unknown transport — no validation
    }
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

    // ── detect_pattern (convention fallback) ───────────────

    #[test]
    fn detect_exact_service_request() {
        assert_eq!(
            detect_pattern("service.request", ""),
            Some(CommunicationPattern::ServiceRequest)
        );
    }

    #[test]
    fn detect_prefixed_service_request() {
        assert_eq!(
            detect_pattern("service.request.brake_status", ""),
            Some(CommunicationPattern::ServiceRequest)
        );
    }

    #[test]
    fn detect_service_response() {
        assert_eq!(
            detect_pattern("service.response", ""),
            Some(CommunicationPattern::ServiceResponse)
        );
        assert_eq!(
            detect_pattern("service.response.brake_status", ""),
            Some(CommunicationPattern::ServiceResponse)
        );
    }

    #[test]
    fn detect_fire_forget() {
        assert_eq!(
            detect_pattern("service.fire_forget", ""),
            Some(CommunicationPattern::FireForget)
        );
        assert_eq!(
            detect_pattern("service.fire_forget.motor_cmd", ""),
            Some(CommunicationPattern::FireForget)
        );
    }

    #[test]
    fn detect_subscribe() {
        assert_eq!(
            detect_pattern("event.subscribe", ""),
            Some(CommunicationPattern::Subscribe)
        );
        assert_eq!(
            detect_pattern("event.subscribe.speed_updates", ""),
            Some(CommunicationPattern::Subscribe)
        );
    }

    #[test]
    fn detect_notification() {
        assert_eq!(
            detect_pattern("event.notification", ""),
            Some(CommunicationPattern::Notification)
        );
        assert_eq!(
            detect_pattern("event.notification.speed_changed", ""),
            Some(CommunicationPattern::Notification)
        );
    }

    #[test]
    fn detect_field_get() {
        assert_eq!(
            detect_pattern("field.get", ""),
            Some(CommunicationPattern::FieldGet)
        );
        assert_eq!(
            detect_pattern("field.get.vehicle_speed", ""),
            Some(CommunicationPattern::FieldGet)
        );
    }

    #[test]
    fn detect_field_set() {
        assert_eq!(
            detect_pattern("field.set", ""),
            Some(CommunicationPattern::FieldSet)
        );
        assert_eq!(
            detect_pattern("field.set.target_speed", ""),
            Some(CommunicationPattern::FieldSet)
        );
    }

    #[test]
    fn detect_none_for_application_events() {
        assert_eq!(detect_pattern("brake.activate", ""), None);
        assert_eq!(detect_pattern("motor.start", ""), None);
        assert_eq!(detect_pattern("error.communication", ""), None);
    }

    #[test]
    fn detect_none_for_partial_prefix() {
        assert_eq!(detect_pattern("service.requestX", ""), None);
        assert_eq!(detect_pattern("field.getAll", ""), None);
        assert_eq!(detect_pattern("event.subscribeNow", ""), None);
    }

    #[test]
    fn detect_none_for_empty_event() {
        assert_eq!(detect_pattern("", ""), None);
    }

    // ── detect_pattern (explicit annotation) ────────────────

    #[test]
    fn explicit_overrides_convention() {
        // Event name says "service.request" but explicit says "fire_forget".
        assert_eq!(
            detect_pattern("service.request.brake", "fire_forget"),
            Some(CommunicationPattern::FireForget)
        );
    }

    #[test]
    fn explicit_none_opts_out() {
        // Event name matches convention, but explicit "none" disables validation.
        assert_eq!(detect_pattern("service.request.brake", "none"), None);
    }

    #[test]
    fn explicit_short_forms() {
        assert_eq!(
            detect_pattern("any.event", "request"),
            Some(CommunicationPattern::ServiceRequest)
        );
        assert_eq!(
            detect_pattern("any.event", "response"),
            Some(CommunicationPattern::ServiceResponse)
        );
        assert_eq!(
            detect_pattern("any.event", "subscribe"),
            Some(CommunicationPattern::Subscribe)
        );
        assert_eq!(
            detect_pattern("any.event", "notification"),
            Some(CommunicationPattern::Notification)
        );
        assert_eq!(
            detect_pattern("any.event", "field_get"),
            Some(CommunicationPattern::FieldGet)
        );
        assert_eq!(
            detect_pattern("any.event", "field_set"),
            Some(CommunicationPattern::FieldSet)
        );
    }

    #[test]
    fn explicit_long_forms() {
        assert_eq!(
            detect_pattern("any.event", "service.request"),
            Some(CommunicationPattern::ServiceRequest)
        );
        assert_eq!(
            detect_pattern("any.event", "event.subscribe"),
            Some(CommunicationPattern::Subscribe)
        );
        assert_eq!(
            detect_pattern("any.event", "field.get"),
            Some(CommunicationPattern::FieldGet)
        );
    }

    #[test]
    fn explicit_case_insensitive() {
        assert_eq!(
            detect_pattern("any.event", "Request"),
            Some(CommunicationPattern::ServiceRequest)
        );
        assert_eq!(
            detect_pattern("any.event", "FIRE_FORGET"),
            Some(CommunicationPattern::FireForget)
        );
    }

    #[test]
    fn explicit_unknown_value_returns_none() {
        assert_eq!(detect_pattern("any.event", "invalid_pattern"), None);
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

    // ── transport_capabilities ──────────────────────────────

    #[test]
    fn local_supports_all() {
        let caps = transport_capabilities("local").unwrap();
        assert!(caps.contains(&TransportCapability::RequestReply));
        assert!(caps.contains(&TransportCapability::FireForget));
        assert!(caps.contains(&TransportCapability::PubSub));
        assert!(caps.contains(&TransportCapability::FieldAccess));
    }

    #[test]
    fn shm_supports_fire_forget_and_field() {
        let caps = transport_capabilities("shm").unwrap();
        assert!(caps.contains(&TransportCapability::FireForget));
        assert!(caps.contains(&TransportCapability::FieldAccess));
        assert!(!caps.contains(&TransportCapability::RequestReply));
        assert!(!caps.contains(&TransportCapability::PubSub));
    }

    #[test]
    fn someip_supports_all() {
        let caps = transport_capabilities("someip").unwrap();
        assert!(caps.contains(&TransportCapability::RequestReply));
        assert!(caps.contains(&TransportCapability::FireForget));
        assert!(caps.contains(&TransportCapability::PubSub));
        assert!(caps.contains(&TransportCapability::FieldAccess));
    }

    #[test]
    fn dds_no_request_reply() {
        let caps = transport_capabilities("dds").unwrap();
        assert!(!caps.contains(&TransportCapability::RequestReply));
        assert!(caps.contains(&TransportCapability::PubSub));
    }

    #[test]
    fn zenoh_no_request_reply() {
        let caps = transport_capabilities("zenoh").unwrap();
        assert!(!caps.contains(&TransportCapability::RequestReply));
        assert!(caps.contains(&TransportCapability::PubSub));
    }

    #[test]
    fn can_signal_only() {
        let caps = transport_capabilities("can").unwrap();
        assert!(caps.contains(&TransportCapability::FireForget));
        assert!(caps.contains(&TransportCapability::FieldAccess));
        assert!(!caps.contains(&TransportCapability::RequestReply));
        assert!(!caps.contains(&TransportCapability::PubSub));
    }

    #[test]
    fn unknown_transport_returns_none() {
        assert!(transport_capabilities("custom_ipc").is_none());
        assert!(transport_capabilities("iceoryx2").is_none());
    }

    // ── transport_supports ──────────────────────────────────

    #[test]
    fn known_transport_checked() {
        assert!(transport_supports("shm", TransportCapability::FireForget));
        assert!(!transport_supports("shm", TransportCapability::RequestReply));
    }

    #[test]
    fn unknown_transport_assumed_supported() {
        // Conservative: unknown transports pass validation.
        assert!(transport_supports("custom_ipc", TransportCapability::RequestReply));
    }

    // ── Display ─────────────────────────────────────────────

    #[test]
    fn pattern_display() {
        assert_eq!(CommunicationPattern::ServiceRequest.to_string(), "service.request");
        assert_eq!(CommunicationPattern::FieldSet.to_string(), "field.set");
    }

    #[test]
    fn capability_display() {
        assert_eq!(TransportCapability::RequestReply.to_string(), "request/reply");
        assert_eq!(TransportCapability::PubSub.to_string(), "pub/sub");
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
