// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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
    };
    static SHM: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: true, has_shared_session: false },
        capabilities: &[FireForget, FieldAccess],
        implemented: true,
        required_binding_fields: &[],
    };
    static SOMEIP: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: true, has_shared_session: false },
        capabilities: &[RequestReply, FireForget, PubSub, FieldAccess],
        implemented: true,
        required_binding_fields: &["service_id", "instance_id", "method_id"],
    };
    static ZENOH: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: false, has_shared_session: true },
        capabilities: &[FireForget, PubSub, FieldAccess],
        implemented: true,
        required_binding_fields: &["key"],
    };
    static DDS: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: true, has_shared_session: false },
        capabilities: &[FireForget, PubSub, FieldAccess],
        implemented: false,
        required_binding_fields: &[],
    };
    static CAN: TransportDescriptor = TransportDescriptor {
        shape: TransportShape { has_per_target_field: true, has_shared_session: false },
        capabilities: &[FireForget, FieldAccess],
        implemented: false,
        required_binding_fields: &[],
    };

    match transport {
        "local" => Some(&LOCAL),
        "shm" => Some(&SHM),
        "someip" => Some(&SOMEIP),
        "zenoh" => Some(&ZENOH),
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
        for name in &["local", "shm", "someip", "zenoh"] {
            assert!(
                lookup(name).unwrap().implemented,
                "transport '{name}' should be marked implemented"
            );
        }
    }

    #[test]
    fn unimplemented_transports_have_capabilities_but_no_template() {
        for name in &["dds", "can"] {
            let d = lookup(name).unwrap();
            assert!(!d.implemented, "transport '{name}' has no template yet");
            assert!(!d.capabilities.is_empty(), "transport '{name}' should still have capabilities for pattern validation");
        }
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
    fn zenoh_no_request_reply() {
        let d = lookup("zenoh").unwrap();
        assert!(!d.capabilities.contains(&RequestReply));
        assert!(d.capabilities.contains(&PubSub));
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
