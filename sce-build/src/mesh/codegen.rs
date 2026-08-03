// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! SCE Mesh codegen dispatcher — per-target transport template rendering.
//!
//! Each target routes through its deploy.yaml-bound transport. Mixed
//! transports are supported: e.g. `#motor` via local, `#display` via
//! shm, `#logger` via someip, `#telemetry` via zenoh. The unified
//! template generates a single `TransportRouter` that dispatches
//! per-target to the appropriate transport-specific send function.
//!
//! Adding a new transport — two required changes:
//!   1. Add one entry to `transport::lookup()` (shape + capabilities).
//!   2. Add `{% elif %}` blocks in `mesh_transport.h.jinja2`.
//!
//! If the transport has device-shared session config, also:
//!   3. Add a typed struct field to `deploy::TransportConfigs`.
//!   4. Thread the config through `generate_mesh()` in `lib.rs`.
//!
//! `Option<T>` fields in this module follow the serialization convention
//! documented at [`crate::model`]: `is not none`-guarded template consumers
//! omit `skip_serializing_if` (serialize JSON `null`); truthy / subfield /
//! `| default(y)` consumers keep it (omit from JSON). When adding a new
//! `Option<T>` field, consult that docstring before picking a shape.

use crate::filters;
use crate::forge::error::SourceLocation;
use crate::generator::{GeneratedOutput, Language};
use crate::mesh::deploy::{
    CustomTcpTransportConfig, DdsTransportConfig, LivelinessConfig, OrderingTimings,
    SomeipTransportConfig, ZenohTransportConfig,
};
use crate::mesh::error::CodegenError;
use crate::mesh::topology::ResolvedTarget;
use crate::mesh::transport;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

// ── JSON5 → C++ string literal escaping ──────────────────────

/// Render a `serde_json`-serialized JSON5 fragment as a complete C++
/// string literal (including surrounding quotes).
///
/// The template embeds these literals verbatim into generated code, e.g.
/// `config.insert_json5("mode", "\"peer\"")`.
///
/// By pre-escaping in Rust we avoid manual `R"(...)"` raw-string
/// interpolation in Jinja, which would break on endpoints containing `)"`
/// or control characters. Control bytes outside printable ASCII are
/// hex-escaped (`\xNN`); common whitespace uses short escapes (`\n`, `\r`,
/// `\t`). Output is ASCII-safe for any UTF-8 input.
fn cpp_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Template context for Zenoh session configuration: pre-escaped C++ string
/// literals, ready to drop into `config.insert_json5(...)` calls.
///
/// Each field is `None` when the corresponding deploy.yaml key is absent.
/// When present, the value is a complete C++ quoted string whose runtime
/// contents are a valid JSON5 fragment (produced by `serde_json`).
///
/// `config_file` references the external zenoh.json5 via `Config::from_file`
/// at runtime (SCE_MESH.md §mesh-13, §mesh-14) — deploy.yaml-level overrides
/// (mode/connect/listen) merge over the file.
#[derive(Debug, Clone, Default, serde::Serialize)]
struct ZenohSessionJson5 {
    /// e.g. `"\"peer\""` (a C++ literal whose runtime value is `"peer"`).
    mode: Option<String>,
    /// e.g. `"[\"tcp/host:7447\"]"` as a C++ literal.
    connect: Option<String>,
    listen: Option<String>,
    /// External zenoh.json5 path as a C++ string literal. When set, the
    /// template emits `zenoh::Config::from_file(<this>)` as the base config;
    /// mode/connect/listen are applied as overrides on top.
    config_file: Option<String>,
}

impl ZenohSessionJson5 {
    /// Build pre-escaped literals from a validated `ZenohTransportConfig`.
    ///
    /// Uses `serde_json::to_string` directly on the typed values
    /// (`ZenohMode` and `Vec<String>` both implement `Serialize`), so
    /// no Display → String detour is needed. The result is a JSON
    /// fragment that `cpp_string_literal` then wraps as a C++ literal.
    fn from_config(cfg: &ZenohTransportConfig) -> Self {
        // `serde_json` on infallible types can never fail; `expect` documents
        // the invariant and lets us use `?`-free code.
        let mode_json = cfg
            .mode
            .map(|m| serde_json::to_string(&m).expect("ZenohMode serialize is infallible"));
        let connect_json = cfg.connect.as_ref().map(|endpoints| {
            serde_json::to_string(endpoints).expect("Vec<String> serialize is infallible")
        });
        let listen_json = cfg.listen.as_ref().map(|endpoints| {
            serde_json::to_string(endpoints).expect("Vec<String> serialize is infallible")
        });
        let config_file = cfg
            .config
            .as_ref()
            .and_then(|p| p.to_str())
            .map(cpp_string_literal);
        Self {
            mode: mode_json.as_deref().map(cpp_string_literal),
            connect: connect_json.as_deref().map(cpp_string_literal),
            listen: listen_json.as_deref().map(cpp_string_literal),
            config_file,
        }
    }

    fn is_empty(&self) -> bool {
        self.mode.is_none()
            && self.connect.is_none()
            && self.listen.is_none()
            && self.config_file.is_none()
    }
}

/// Template context for SOME/IP device-shared configuration.
///
/// Currently carries only `application_name` — the vsomeip application
/// identity that binds generated per-target `vsomeip::application`
/// instances to an entry in `applications[*].name` inside vsomeip.json
/// (SCE_MESH.md §mesh-13). The template uses it verbatim as the argument to
/// `vsomeip::runtime::get()->create_application(<name>)`; when `None`
/// the template falls back to the synthetic `<machine>_<target>` name so
/// test fixtures that predate the external-config integration keep
/// compiling without a vsomeip.json on the side.
///
/// Pre-escaped as a complete C++ string literal so the template embeds
/// it without manual escaping logic.
#[derive(Debug, Clone, Default, serde::Serialize)]
struct SomeipTransportContext {
    /// Complete C++ string literal of the application name, e.g. `"\"brake_app\""`.
    /// `None` if deploy.yaml did not declare `application_name:`.
    application_name: Option<String>,
}

impl SomeipTransportContext {
    fn from_config(cfg: &SomeipTransportConfig) -> Self {
        Self {
            application_name: cfg.application_name.as_deref().map(cpp_string_literal),
        }
    }

    fn is_empty(&self) -> bool {
        self.application_name.is_none()
    }
}

/// Template context for custom_tcp device-shared configuration.
///
/// Carries the optional `listen:` endpoint pre-escaped as a complete C++
/// string literal so the template can pass it directly to the server's
/// `bind()` call. `None` when the device is a pure client and runs no
/// server (the template skips server emission in that case).
#[derive(Debug, Clone, Default, serde::Serialize)]
struct CustomTcpTransportContext {
    /// Complete C++ string literal of the listen endpoint, e.g.
    /// `"\"127.0.0.1:9000\""`. `None` if deploy.yaml omitted `listen:`.
    listen: Option<String>,
}

impl CustomTcpTransportContext {
    fn from_config(cfg: &CustomTcpTransportConfig) -> Self {
        Self {
            listen: cfg.listen.as_deref().map(cpp_string_literal),
        }
    }

    fn is_empty(&self) -> bool {
        self.listen.is_none()
    }
}

/// Template context for DDS device-shared configuration.
///
/// Only the domain id: a DDS domain is the isolation unit, and everything
/// else Cyclone DDS exposes is tuning that belongs in its own XML config
/// (reached through `CYCLONEDDS_URI`) rather than in the mesh schema.
///
/// Unlike `CustomTcpTransportContext` this has no "is it empty" notion —
/// a device with dds bindings always joins a domain, defaulting to 0, so
/// the context is always meaningful when dds is in play.
#[derive(Debug, Clone, Default, serde::Serialize)]
struct DdsTransportContext {
    /// Domain the device's single participant joins. Rendered as a plain
    /// integer literal; `0` is the DDS default and the value used when
    /// deploy.yaml omits the block entirely.
    domain_id: u32,
}

impl DdsTransportContext {
    fn from_config(cfg: Option<&DdsTransportConfig>) -> Self {
        Self {
            domain_id: cfg.and_then(|c| c.domain_id).unwrap_or(0),
        }
    }
}

// ── Template context ─────────────────────────────────────────

/// SOME/IP service identity, pre-rendered as `0xNNNN` hex strings so the
/// template emits literals without probing integer formatters. `None` for
/// non-SOME/IP targets.
#[derive(Debug, Clone, serde::Serialize)]
struct SomeipServiceLiterals {
    service_id: String,
    instance_id: String,
}

/// Per-target transport-specific state, formatted for the template.
///
/// Mirrors [`crate::mesh::topology::TransportState`] but with values
/// pre-rendered for direct emission: SOME/IP IDs become `0xNNNN` hex
/// strings, capacity tunables retain their `Option<u32>` so the template's
/// `| default(...)` filter still produces the legacy fallback constants.
///
/// `tag = "kind"` lets the template dispatch on
/// `target.state.kind == "local" | "shm" | "someip" | "zenoh"` instead of
/// the deprecated `target.transport == "..."` string. The `Unimplemented`
/// variant never reaches the template because codegen rejects unsupported
/// transports up front.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TargetStateView {
    Local,
    Shm {
        #[serde(skip_serializing_if = "Option::is_none")]
        arena_bytes: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ring_capacity: Option<u32>,
    },
    Someip {
        service: SomeipServiceLiterals,
        // Always serialize as an object so the template can probe
        // optional members like `target.state.extra.protocol` with
        // `is defined` even when the binding declared no passthrough
        // keys. Skipping an empty map would make the parent undefined
        // and the probe would error instead of falling back to default.
        extra: HashMap<String, serde_yaml_ng::Value>,
    },
    Zenoh {
        key: String,
        extra: HashMap<String, serde_yaml_ng::Value>,
    },
    CustomTcp {
        /// Server endpoint to dial (`host:port`), pre-escaped as a complete
        /// C++ string literal (including outer quotes) so the template emits
        /// it verbatim into the generated client constructor.
        connect: String,
        extra: HashMap<String, serde_yaml_ng::Value>,
    },
    Dds {
        /// Request-leg topic name, pre-escaped as a complete C++ string
        /// literal (including outer quotes) so the template emits it
        /// verbatim. The reply and notification topic names are derived
        /// from this one at emission time rather than carried separately —
        /// a binding cannot pair a request topic with an unrelated reply
        /// topic because there is no field in which to express that.
        topic: String,
        extra: HashMap<String, serde_yaml_ng::Value>,
    },
}

impl TargetStateView {
    /// Translate the topology-level [`TransportState`] into a template-ready
    /// view. The `Unimplemented` variant is unreachable here because
    /// codegen rejects unsupported transports at the registry-lookup stage
    /// before any target reaches this function.
    fn from_topology(state: &crate::mesh::topology::TransportState) -> Self {
        use crate::mesh::topology::TransportState;
        match state {
            TransportState::Local => Self::Local,
            TransportState::Shm {
                arena_bytes,
                ring_capacity,
            } => Self::Shm {
                arena_bytes: *arena_bytes,
                ring_capacity: *ring_capacity,
            },
            TransportState::Someip { service, extra, .. } => Self::Someip {
                service: SomeipServiceLiterals {
                    service_id: fmt_someip_id(service.service_id),
                    instance_id: fmt_someip_id(service.instance_id),
                },
                extra: extra.clone(),
            },
            TransportState::Zenoh { key, extra } => Self::Zenoh {
                key: key.clone(),
                extra: extra.clone(),
            },
            TransportState::CustomTcp { connect, extra } => Self::CustomTcp {
                connect: cpp_string_literal(connect),
                extra: extra.clone(),
            },
            TransportState::Dds { topic, extra } => Self::Dds {
                topic: cpp_string_literal(topic),
                extra: extra.clone(),
            },
            TransportState::Unimplemented { transport_name } => unreachable!(
                "TargetStateView::from_topology: unimplemented transport '{transport_name}' \
                 should have been rejected by the registry-lookup stage in generate_cpp_mesh"
            ),
        }
    }
}

/// Template context for a single resolved send target.
/// `target` uses `TargetId` directly — `#[serde(transparent)]` makes the
/// wire form identical to a bare string, so Jinja2 sees `"#motor"` with no
/// String round-trip at the template boundary.
#[derive(Debug, Clone, serde::Serialize)]
struct TargetContext {
    target: super::target::TargetId,
    target_stem: String,
    target_snake: String,
    target_pascal: String,
    events: Vec<String>,
    /// Tagged transport state — the template branches on
    /// `target.state.kind` and reads typed payload fields off the
    /// matching variant. Replaces the legacy `transport: String` +
    /// `extra` + `someip_service` triple.
    state: TargetStateView,
    /// Emit a per-target field in TransportRouter and a matching ctor
    /// initializer? Data-driven — removes transport-name hardcoding from
    /// the template's field/ctor sections.
    has_per_target_field: bool,
    /// Does THIS binding need the runtime DedupWindow on inbound envelopes?
    ///
    /// Per-binding refinement of the transport-level `supplies_dedup`
    /// default: a SOME/IP binding with `protocol: tcp` runs on a single
    /// TCP stream per client↔server pair, so duplicates are physically
    /// impossible and the inbound path can call `dispatchToSender`
    /// directly. The default SOME/IP path (UDP + multicast SD) still
    /// needs dedup.
    ///
    /// SCE_MESH.md §mesh-10.5: the runtime DedupWindow is keyed on
    /// `(env.source, env.id)` and emitted per-machine when any binding
    /// reports `needs_dedup = true` via the machine-wide
    /// `has_undeduped_transport` flag.
    needs_dedup: bool,
    /// Does THIS binding need the runtime OrderingBuffer on inbound
    /// envelopes (SCE_MESH.md §mesh-10.6)?
    ///
    /// `true` iff the binding declares `ordering: required` AND the
    /// transport cannot supply order natively AND no per-binding
    /// upgrade (e.g. SOME/IP `protocol: tcp`) lifts that default.
    /// Drives both the per-target `seq_counter` emission on the
    /// sender side and the receiver-side `admitOrdered` branch.
    needs_ordering: bool,
    /// SCE_MESH.md §mesh-14.6 responder set — machine names (no leading
    /// `#`) whose RpcReply may retire a correlation entry for a request
    /// sent to this target. Carried verbatim from
    /// [`ResolvedTarget::responders`]; the template emits it as the
    /// `respondersFor` arm for this target and as the per-target
    /// allow-list the SOME/IP response handler checks. Never empty for
    /// a declared target — the deploy validator rejects an empty
    /// `reply_from:` and the default arm yields the target itself.
    responders: Vec<String>,
    /// SCE_MESH.md §mesh-16.7 row 3 retry policy carried verbatim into the
    /// template context. `None` ⇒ the OutboundBuffer's dispatcher
    /// goes straight to the transport-send closure (Stage 1/2
    /// behaviour). `Some(_)` ⇒ codegen emits a per-target
    /// `RetryingDispatcher` member declared BEFORE the OutboundBuffer
    /// in member order so `-Wreorder` stays satisfied; the
    /// OutboundBuffer's dispatcher closure routes through the
    /// retrying wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    retry: Option<RetryPolicyContext>,
    /// SCE_MESH.md §mesh-16.7 row 10 — per-binding auth policy projected into
    /// the template context. `None` ⇒ no row-10 wiring is emitted;
    /// transport rejection signals stay classified as row 1 / row 8.
    /// `Some(_)` ⇒ the generator's transport classify-arm reads
    /// `peer_fingerprint` (zenoh) or `sd_denied_classifies_as_unauthorized`
    /// (someip) and emits the UNAUTHORIZED classification at the
    /// matching rejection site.
    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<AuthPolicyContext>,
    /// Per-event pattern metadata for pattern-aware send logic.
    event_patterns: Vec<EventPatternContext>,
    /// True if any event uses RPC patterns (ServiceRequest/ServiceResponse).
    has_rpc: bool,
    /// True if any event uses PubSub patterns (Subscribe/Notification).
    has_pubsub: bool,
    /// True if any event uses Field patterns (FieldGet/FieldSet).
    has_field: bool,
    /// True if target receives responses (RPC, EventNotify, FieldNotify).
    /// Enables receive handler generation in init().
    has_receive: bool,
    /// `<invoke type="sce:mesh-rpc">` sites targeting this binding.
    /// Populated from `ResolvedTarget::invoke_sites`. The template gates
    /// invokeMeshRpc / cancelMeshRpc emission on `target.invoke_sites`
    /// being non-empty (`has_mesh_rpc.v` flag).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    invoke_sites: Vec<super::topology::MeshRpcInvokeSite>,
    /// SCE_MESH.md §mesh-14.4 — runtime pool substitution plan. `None` when the
    /// binding has no placeholder (all existing fixtures). When present,
    /// the template routes the matched-site block through a pool-specific
    /// dispatch path that decodes `env.data` JSON once per invoke and
    /// builds the runtime address (Zenoh KeyExpr) or selects the runtime
    /// instance id (SOME/IP), bypassing the literal-address `route_send`
    /// path. Non-pool targets render byte-identical output because every
    /// pool branch is gated on `{% if target.pool_plan %}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pool_plan: Option<super::topology::PoolPlan>,
}

/// SCE_MESH.md §mesh-16.7 row 3 retry policy as it surfaces to the
/// template. Mirrors `RetryPolicyConfig` field-for-field, but typed
/// as plain serializable scalars so the jinja template can emit
/// numeric initializers directly. `Option`-wrapping a struct that
/// already has `Option`-wrapped fields would force the template to
/// reach into nested optionals; the flat record keeps the codegen
/// site readable.
#[derive(Debug, Clone, serde::Serialize)]
struct RetryPolicyContext {
    max_retries: u32,
    initial_backoff_ms: u64,
    backoff_multiplier: f64,
    max_backoff_ms: u64,
    jitter_pct: u32,
}

impl From<super::deploy::RetryPolicyConfig> for RetryPolicyContext {
    fn from(c: super::deploy::RetryPolicyConfig) -> Self {
        Self {
            max_retries: c.max_retries,
            initial_backoff_ms: c.initial_backoff_ms,
            backoff_multiplier: c.backoff_multiplier,
            max_backoff_ms: c.max_backoff_ms,
            jitter_pct: c.backoff_jitter_pct,
        }
    }
}

/// SCE Mesh §mesh-16.7 row 10 — codegen-side projection of the per-binding
/// auth policy. Mirrors [`RetryPolicyContext`] in shape and rationale:
/// the deploy.yaml struct is `Option`-rich, but the template only sees
/// either "no auth wiring" (None at the TargetContext level) or a
/// fully-populated record with the transport-required fields collapsed
/// down to flat scalars. The validator (`validate_auth_policy`) has
/// already proven the required fields are present for the transport,
/// so this struct carries the post-validation invariant ("if you see
/// this struct, you have everything you need to emit the auth wiring").
#[derive(Debug, Clone, serde::Serialize)]
struct AuthPolicyContext {
    /// SHA-256 peer-cert fingerprint pinned for this target. Populated
    /// only for zenoh bindings (validator rejected the field on
    /// someip / custom_tcp / shm). When absent, the template emits the
    /// SOMEIP variant of row-10 wiring instead of the zenoh variant.
    peer_fingerprint: Option<String>,

    /// SOMEIP-specific opt-in: classify `register_availability_handler(false)`
    /// as row 10 instead of row 8. Populated only for someip bindings;
    /// validator ensures this is `Some(true)` when the binding declared
    /// `required: true` on a someip transport, and `None` on zenoh.
    sd_denied_classifies_as_unauthorized: Option<bool>,
}

impl AuthPolicyContext {
    /// Convert from the deploy.yaml shape. Returns `None` if the policy
    /// is opt-out (`required: false` or absent) — the validator has
    /// already proven the rest of the fields are blank in that case,
    /// so the codegen path can short-circuit by treating `required:
    /// false` identically to "no auth section".
    fn from_config(c: super::deploy::AuthPolicyConfig) -> Option<Self> {
        if !c.required {
            return None;
        }
        Some(Self {
            peer_fingerprint: c.peer_fingerprint,
            sd_denied_classifies_as_unauthorized: c.sd_denied_classifies_as_unauthorized,
        })
    }
}

/// Per-event pattern context for template rendering.
///
/// Carries both the pattern classification and the per-event SOME/IP
/// numeric IDs so the template can emit per-event constants and dispatch
/// on event name (different SCXML events on the same target can use
/// different methods or event groups, SCE_MESH.md §mesh-14).
///
/// Template dispatch is driven by `field_kind` (`"method"` /
/// `"event_group"` / `"getter"` / `"setter"`), NOT by probing which ID
/// Option is populated. The individual ID strings are only for value
/// rendering — keying on them creates a 4-way dispatch duplicated across
/// validator, attach, and template, and that was the whole point of the
/// `SomeipEventIds` tagged enum.
#[derive(Debug, Clone, serde::Serialize)]
struct EventPatternContext {
    event: String,
    /// C++-identifier-safe upper-snake form of the event name, used for
    /// per-event constant naming (`SOMEIP_METHOD_<TARGET>_<EVENT>`).
    event_const: String,
    /// C++ PatternKind wire value (1-9).
    pattern_kind: u16,
    /// Paired reply event, inferred by convention (RPC request only).
    /// `None` for non-RPC events — the template filters on truthiness so
    /// `{% if ep.reply_event %}` and `{% for ep in ... if ep.reply_event %}`
    /// both continue to work unchanged.
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_event: Option<String>,
    /// Discriminator for SOME/IP per-event dispatch. One of `"method"`,
    /// `"event_group"`, `"getter"`, `"setter"`, or `None` for non-SOME/IP
    /// targets (or SOME/IP events whose resolution produced no mapping —
    /// topology validation rejects those before codegen, so at this point
    /// `None` iff transport != "someip").
    #[serde(skip_serializing_if = "Option::is_none")]
    field_kind: Option<&'static str>,
    /// Per-event SOME/IP numeric IDs, rendered as `0x####` literals. Only
    /// the field(s) matching `field_kind` are populated — the others stay
    /// `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    method_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    event_group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    getter_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    setter_id: Option<String>,
    /// True when this event should NOT register a receive handler in
    /// SOME/IP `init()`. Currently true only for EventUnsubscribe: it is
    /// outbound-only (sends a control message to the bus) and shares the
    /// same event_id as its paired EventSubscribe — registering a second
    /// handler on the same (service, instance, event_id) triple would
    /// silently replace the subscribe handler in vsomeip. The codegen
    /// sets this flag so the template avoids hard-coding wire values.
    skip_receive_handler: bool,
}

/// Convert an SCXML event name (`service.request.compute_force`) into a
/// C++-safe upper-snake constant suffix (`SERVICE_REQUEST_COMPUTE_FORCE`).
/// `.`/`-`/`/` map to `_`; anything else outside `[A-Za-z0-9_]` also
/// becomes `_`. Identical inputs produce identical suffixes (deterministic).
fn event_to_const_suffix(event: &str) -> String {
    event
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Re-export of the canonical u16 → SOME/IP literal renderer. Kept as a
/// local alias so the call sites in this module read naturally; the actual
/// format lives in [`crate::mesh::someip_format`] so the resolution path
/// and codegen path cannot drift.
use crate::mesh::someip_format::hex_id as fmt_someip_id;

/// Fan a [`SomeipEventIds`] variant into the per-event template fields:
/// `(field_kind, method_id, event_group_id, event_id, getter_id,
/// setter_id)` with only the fields matching the variant populated.
/// Centralizes the single "variant → template fields" translation so
/// the template never has to probe which option happened to be set.
type EventIdsTemplateFields = (
    Option<&'static str>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn event_ids_to_template(ids: &crate::mesh::topology::SomeipEventIds) -> EventIdsTemplateFields {
    use crate::mesh::topology::SomeipEventIds;
    match *ids {
        SomeipEventIds::Method { method_id } => (
            Some("method"),
            Some(fmt_someip_id(method_id)),
            None,
            None,
            None,
            None,
        ),
        SomeipEventIds::EventGroup {
            event_group_id,
            event_id,
        } => (
            Some("event_group"),
            None,
            Some(fmt_someip_id(event_group_id)),
            Some(fmt_someip_id(event_id)),
            None,
            None,
        ),
        SomeipEventIds::Getter { getter_id } => (
            Some("getter"),
            None,
            None,
            None,
            Some(fmt_someip_id(getter_id)),
            None,
        ),
        SomeipEventIds::Setter { setter_id } => (
            Some("setter"),
            None,
            None,
            None,
            None,
            Some(fmt_someip_id(setter_id)),
        ),
    }
}

// ── Server-side template context (SCE_MESH.md §mesh-13 Session E) ──

/// Template context for server-side transport registration.
///
/// Gated by `{% if server %}` in the template. Contains the transport
/// kind, service identity (SOME/IP) or key expression (Zenoh), and
/// per-RPC-pair metadata for handler registration.
#[derive(Debug, Clone, serde::Serialize)]
struct ServerContext {
    /// Transport variant: `"someip"` or `"zenoh"`.
    transport_kind: String,
    /// Tagged transport state (same as TargetStateView) for service IDs
    /// or key expression.
    state: TargetStateView,
    /// Per-RPC-pair context for server handler registration.
    rpc_pairs: Vec<ServerRpcPairContext>,
    /// Per-event context for FireForget inbound handler registration
    /// (SCE_MESH.md §mesh-8.3). SOME/IP registers one
    /// `register_message_handler` per event; Zenoh shares a single
    /// `declare_subscriber` on the server key across all FireForget
    /// events and dispatches by `env.type`. The template uses
    /// `has_fire_forget` to toggle the Zenoh subscriber declaration.
    fire_forget_events: Vec<ServerFireForgetContext>,
    /// True when [`Self::fire_forget_events`] is non-empty.
    has_fire_forget: bool,
    /// Per-pair context for FieldAccess inbound handler registration
    /// (SCE_MESH.md §mesh-8.3). SOME/IP registers one
    /// `register_message_handler` per (getter or setter) method; Zenoh
    /// shares the single queryable (FieldRead) and the single server
    /// subscriber (FieldWrite) declared for RPC + FireForget and
    /// dispatches by `env.type`. The paired `field.notify.X` reply is
    /// routed through `handleServerResponse` identically to RPC.
    field_access_pairs: Vec<ServerFieldAccessContext>,
    /// True when the server must accept inbound `session.put` — either
    /// for FireForget (§mesh-8.3) or FieldWrite, which also uses
    /// `session.put` on the Zenoh key. Controls emission of the Zenoh
    /// `zenoh_server_put_sub_` subscriber.
    has_server_put: bool,
    /// Unique `field.notify.X` event names the server may emit — one
    /// entry per distinct response event across `field_access_pairs`
    /// AND `eventgroup_events`. A single suffix can appear as both
    /// getter and setter (e.g. `field.get.position` +
    /// `field.set.position`) and raises the same `field.notify.position`;
    /// the template uses this dedup list for `resolvePattern` so the
    /// generated `if` chain has no duplicates.
    field_notify_events: Vec<String>,
    /// Per-event context for eventgroup notification publish
    /// (SCE_MESH.md §mesh-8.1). SOME/IP: `offer_event` + `notify`. Zenoh:
    /// `session.put`. Empty when no eventgroup events are declared.
    eventgroup_events: Vec<ServerEventgroupContext>,
    /// True when [`Self::eventgroup_events`] is non-empty. Controls
    /// emission of `publishEventgroupNotify` and the fallback path in
    /// the mesh send callback.
    has_eventgroup: bool,
    /// SCE Mesh §mesh-9.5 gap Z2: per-server Zenoh queryable response
    /// deadline in milliseconds. `None` ⇒ no deadline emitted,
    /// matching the pre-Z2 behaviour where `pending_server_queries_`
    /// leaks entries whose engine never responds. Some ⇒ the template
    /// gates `has_server_query_timeout`, instantiates the deadline
    /// scheduler (gate-shared with `has_mesh_rpc.v`), and arms a
    /// scheduler entry at every `pending_server_queries_` insert.
    #[serde(skip_serializing_if = "Option::is_none")]
    query_timeout_ms: Option<u64>,
    /// SCE_MESH.md §mesh-14.4 (Gap 7): multi-instance
    /// server pool member list. Propagated from
    /// [`crate::mesh::topology::ServerBinding::instance_pool`]. When
    /// present, the template renders `SOMEIP_SERVER_INSTANCES` as the
    /// declared `std::array` so init() can offer each instance and
    /// register per-(instance, method) handlers. Absent ⇒ non-pool
    /// server; template degenerates to a 1-element array whose sole
    /// entry is the binding-default instance id.
    ///
    /// Zenoh's registry flag `supports_multi_instance_server` is
    /// `false`, so `ServerBinding::instance_pool` is always `None`
    /// for Zenoh servers; this field is SOME/IP-only in practice.
    #[serde(skip_serializing_if = "Option::is_none")]
    instance_pool: Option<Vec<u16>>,
}

/// Per-RPC-pair context for server-side codegen.
///
/// The inbound request event no longer appears here because the server
/// handler trusts the decoded envelope's `env.type` verbatim (SCE_MESH.md
/// §mesh-13 Session H: wire format is SSOT). Only fields the template actually
/// reads are kept; adding fields back requires a matching template
/// consumer.
#[derive(Debug, Clone, serde::Serialize)]
struct ServerRpcPairContext {
    /// Outbound response event (e.g. `"service.response.compute_force"`).
    /// Used by `resolvePattern` to classify `<send>` of the response as
    /// `RpcReply` so the server interceptor routes it through
    /// `handleServerResponse`.
    response_event: String,
    /// C++-safe upper-snake constant suffix for per-event naming.
    event_const: String,
    /// SOME/IP method ID for `register_message_handler` (`0xNNNN`).
    /// `None` for non-SOME/IP transports.
    #[serde(skip_serializing_if = "Option::is_none")]
    method_id: Option<String>,
}

/// Per-event context for server-side FireForget inbound handler codegen
/// (SCE_MESH.md §mesh-8.3). SOME/IP resolves `method_id` from the deploy.yaml
/// `server.events.<event>.method` binding; Zenoh leaves it absent because
/// all FireForget events land on the shared server key.
#[derive(Debug, Clone, serde::Serialize)]
struct ServerFireForgetContext {
    /// C++-safe upper-snake constant suffix for per-event naming.
    event_const: String,
    /// SOME/IP method ID (`0xNNNN`). `None` for Zenoh.
    #[serde(skip_serializing_if = "Option::is_none")]
    method_id: Option<String>,
}

/// Per-pair context for server-side FieldAccess handler codegen
/// (SCE_MESH.md §mesh-8.3). Mirrors [`ServerRpcPairContext`] — both patterns
/// ride the same transport-level reply path (`create_response` /
/// `Query::reply`).
///
/// Request/response event strings are not stored here: the decoded
/// envelope owns `env.type`, and `resolvePattern` classifies the notify
/// event through the deduped [`ServerContext::field_notify_events`] list
/// (getter + setter on the same suffix share one notify event).
#[derive(Debug, Clone, serde::Serialize)]
struct ServerFieldAccessContext {
    /// C++-safe upper-snake constant suffix, derived from the request
    /// event (same basis as [`ServerRpcPairContext::event_const`]).
    event_const: String,
    /// `"getter"` or `"setter"` — discriminator used by the template to
    /// pick the SOME/IP method constant. Single source of truth for the
    /// getter/setter split so the template does not probe multiple fields.
    kind: &'static str,
    /// SOME/IP getter/setter method ID (`0xNNNN`). `None` for Zenoh.
    #[serde(skip_serializing_if = "Option::is_none")]
    method_id: Option<String>,
}

/// Per-event context for server-side eventgroup notification codegen
/// (SCE_MESH.md §mesh-8.1). SOME/IP: `offer_event` + `notify`. Zenoh:
/// `session.put` on the server key.
///
/// Eventgroup events are published spontaneously (without a preceding
/// request). The mesh send callback routes them through
/// `publishEventgroupNotify` when `handleServerResponse` returns false
/// (no pending request to correlate against).
#[derive(Debug, Clone, serde::Serialize)]
struct ServerEventgroupContext {
    /// Event name to publish (e.g. `"field.notify.vehicle_speed"`).
    event: String,
    /// C++-safe upper-snake constant suffix for per-event naming.
    event_const: String,
    /// SOME/IP eventgroup ID (`0xNNNN`). `None` for Zenoh.
    #[serde(skip_serializing_if = "Option::is_none")]
    event_group_id: Option<String>,
    /// SOME/IP event ID (`0xNNNN`, >= 0x8000). `None` for Zenoh.
    #[serde(skip_serializing_if = "Option::is_none")]
    event_id: Option<String>,
    /// Wire `PatternKind` this event resolves to, so the emitted
    /// `resolvePattern` table can classify a server-published event
    /// without re-deriving the vocabulary from name prefixes in Jinja2.
    /// `field.notify.X` resolves to `FieldNotify` and
    /// `event.notification.X` to `EventNotify`; both are published, but
    /// only the first is also a paired reply.
    pattern_kind: u16,
}

/// Build a [`ServerContext`] from a resolved [`super::topology::ServerBinding`].
fn build_server_context(binding: &super::topology::ServerBinding) -> ServerContext {
    use crate::mesh::pattern::SomeipFieldKind;
    use crate::mesh::topology::{FieldAccessKind, SomeipEventIds, TransportState};

    let transport_kind = binding.state.transport_name().to_string();
    let state = TargetStateView::from_topology(&binding.state);

    // Look up the numeric SOME/IP ID for an event, expecting a specific
    // slot family. The caller names the family via `SomeipFieldKind` so
    // typos are compile errors rather than silent `None` returns that
    // would drop a handler. The resolver guarantees the per-event variant
    // aligns with the pattern family (`Method` for RPC/FireForget,
    // `Getter`/`Setter` for FieldAccess), so a mismatch means upstream
    // drift.
    let someip_id_for = |event: &str, want: SomeipFieldKind| -> Option<String> {
        let TransportState::Someip { event_bindings, .. } = &binding.state else {
            return None;
        };
        let ids = event_bindings.get(event)?;
        match (ids, want) {
            (SomeipEventIds::Method { method_id }, SomeipFieldKind::Method) => {
                Some(fmt_someip_id(*method_id))
            }
            (SomeipEventIds::Getter { getter_id }, SomeipFieldKind::Getter) => {
                Some(fmt_someip_id(*getter_id))
            }
            (SomeipEventIds::Setter { setter_id }, SomeipFieldKind::Setter) => {
                Some(fmt_someip_id(*setter_id))
            }
            _ => None,
        }
    };

    let rpc_pairs: Vec<ServerRpcPairContext> = binding
        .rpc_pairs
        .iter()
        .map(|pair| ServerRpcPairContext {
            response_event: pair.response_event.clone(),
            event_const: event_to_const_suffix(&pair.request_event),
            method_id: someip_id_for(&pair.request_event, SomeipFieldKind::Method),
        })
        .collect();

    let fire_forget_events: Vec<ServerFireForgetContext> = binding
        .fire_forget_events
        .iter()
        .map(|event| ServerFireForgetContext {
            event_const: event_to_const_suffix(event),
            method_id: someip_id_for(event, SomeipFieldKind::Method),
        })
        .collect();
    let has_fire_forget = !fire_forget_events.is_empty();

    let field_access_pairs: Vec<ServerFieldAccessContext> = binding
        .field_access_pairs
        .iter()
        .map(|pair| {
            let (kind_str, slot) = match pair.kind {
                FieldAccessKind::Getter => ("getter", SomeipFieldKind::Getter),
                FieldAccessKind::Setter => ("setter", SomeipFieldKind::Setter),
            };
            ServerFieldAccessContext {
                event_const: event_to_const_suffix(&pair.request_event),
                kind: kind_str,
                method_id: someip_id_for(&pair.request_event, slot),
            }
        })
        .collect();
    // Zenoh server needs a `session.put` subscriber for every inbound
    // pattern that uses `put` on the wire — FireForget (SCE_MESH.md §mesh-8.3)
    // and FieldWrite. The subscriber is a single declaration shared
    // across those patterns; the template toggles it on `has_server_put`.
    let has_setter = binding
        .field_access_pairs
        .iter()
        .any(|p| p.kind == FieldAccessKind::Setter);
    let has_server_put = has_fire_forget || has_setter;

    // Build eventgroup event contexts.
    let eventgroup_events: Vec<ServerEventgroupContext> = binding
        .eventgroup_events
        .iter()
        .map(|eg| {
            let (eg_id, ev_id) = match &binding.state {
                TransportState::Someip { event_bindings, .. } => {
                    match event_bindings.get(&eg.event) {
                        Some(SomeipEventIds::EventGroup {
                            event_group_id,
                            event_id,
                        }) => (
                            Some(fmt_someip_id(*event_group_id)),
                            Some(fmt_someip_id(*event_id)),
                        ),
                        _ => (None, None),
                    }
                }
                _ => (None, None),
            };
            ServerEventgroupContext {
                event: eg.event.clone(),
                event_const: event_to_const_suffix(&eg.event),
                event_group_id: eg_id,
                event_id: ev_id,
                // An unrecognised prefix cannot be published as anything
                // meaningful, so fall back to the notification kind rather
                // than to FireForget — a declared eventgroup event is by
                // definition server-published.
                pattern_kind: crate::mesh::pattern::detect_pattern(&eg.event)
                    .unwrap_or(crate::mesh::pattern::CommunicationPattern::Notification)
                    .wire_value(),
            }
        })
        .collect();
    let has_eventgroup = !eventgroup_events.is_empty();

    // Deduplicate response events: field_access_pairs notifies +
    // eventgroup events with field.notify prefix share the same
    // resolvePattern entry. BTreeSet gives stable ordering.
    let field_notify_events: Vec<String> = binding
        .field_access_pairs
        .iter()
        .map(|p| p.response_event.clone())
        .chain(
            binding
                .eventgroup_events
                .iter()
                .filter(|eg| eg.event.starts_with("field.notify."))
                .map(|eg| eg.event.clone()),
        )
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    ServerContext {
        transport_kind,
        state,
        rpc_pairs,
        fire_forget_events,
        has_fire_forget,
        field_access_pairs,
        has_server_put,
        field_notify_events,
        eventgroup_events,
        has_eventgroup,
        query_timeout_ms: binding.query_timeout_ms,
        instance_pool: binding.instance_pool.clone(),
    }
}

// ── Public entry point ───────────────────────────────────────

/// Generate mesh transport code for a machine's resolved targets.
///
/// `zenoh_session` and `someip_config` come from the owning device's
/// `transports:` block (`DeployConfig::topology[device].transports.*`).
/// Each is `None` when the device has no binding of that transport, or
/// when the corresponding `transports:` block is absent.
///
/// `server` is the resolved server-side binding for machines that act
/// as RPC servers (SCE_MESH.md §mesh-13 Session E). `None` for pure-client
/// machines.
/// Public mesh codegen input bundle — aggregates the 20 deploy /
/// topology / transport / partition / SOMEIP / source-location /
/// template-base dimensions the resolver surfaces. Carries only
/// borrows + Copy-by-value types so the struct itself is `Copy` and
/// can be threaded through the per-language entry points without
/// shape coupling. The `language` selector stays outside the bundle
/// since it picks which entry point runs, not which inputs it sees.
#[derive(Copy, Clone)]
pub struct MeshCodegenInputs<'a> {
    pub machine_name: &'a str,
    pub targets: &'a [ResolvedTarget],
    pub server: Option<&'a super::topology::ServerBinding>,
    pub zenoh_session: Option<&'a ZenohTransportConfig>,
    pub someip_config: Option<&'a SomeipTransportConfig>,
    pub custom_tcp_config: Option<&'a CustomTcpTransportConfig>,
    pub dds_config: Option<&'a DdsTransportConfig>,
    pub subscriptions: &'a [super::deploy::SubscriptionConfig],
    pub machine_ordering: OrderingTimings,
    pub machine_liveliness: Option<LivelinessConfig>,
    pub machine_outbound_buffer: Option<super::deploy::OutboundBufferConfig>,
    pub partition_self_name: Option<&'a str>,
    pub partition_wire21_outbound: &'a BTreeMap<String, String>,
    pub partition_wire21_inbound: &'a [String],
    pub scxml_remote_outbound_peers: &'a [crate::model::ScxmlRemotePeerBinding],
    pub scxml_remote_inbound_peers: &'a [crate::model::ScxmlRemotePeerBinding],
    pub someip_invoke_service_ids: &'a BTreeMap<String, u16>,
    pub someip_liveness_service_ids: &'a BTreeMap<String, u16>,
    pub someip_machine_liveness_service_ids: &'a BTreeMap<String, u16>,
    pub source_location: Option<&'a SourceLocation>,
    pub template_base: &'a Path,
}

pub fn generate_mesh(
    inputs: MeshCodegenInputs<'_>,
    language: Language,
) -> Result<GeneratedOutput, CodegenError> {
    // A custom_tcp `listen:` on the device requires the machine to host a
    // server even when it has no `bindings:`/`server:`/`subscriptions:` of
    // its own — the server field, init() bind, and shutdown() teardown all
    // live on the per-machine TransportRouter. Skipping here would leave a
    // pure-receiver machine with no transport.h and no listening socket.
    // SSoT: `CustomTcpTransportConfig::hosts_server` (see deploy.rs).
    let needs_custom_tcp_server = inputs
        .custom_tcp_config
        .is_some_and(CustomTcpTransportConfig::hosts_server);
    // SCE_MESH.md §mesh-16.5 wire-21: a partition with non-empty wire-21
    // routes still needs `<machine>_transport.h` even when no
    // conventional `<send>`-driven targets, server, subscriptions, or
    // custom_tcp listen exist (rule 12 fixture). Mirrors the same
    // predicate threaded through `compile_mesh_transport`.
    let has_wire21_routing =
        !inputs.partition_wire21_outbound.is_empty() || !inputs.partition_wire21_inbound.is_empty();
    let has_scxml_remote_wire = !inputs.scxml_remote_outbound_peers.is_empty()
        || !inputs.scxml_remote_inbound_peers.is_empty();
    if inputs.targets.is_empty()
        && inputs.subscriptions.is_empty()
        && inputs.server.is_none()
        && !needs_custom_tcp_server
        && !has_wire21_routing
        && !has_scxml_remote_wire
    {
        return Ok(GeneratedOutput::default());
    }

    match language {
        Language::Cpp => generate_cpp_mesh(inputs),
        _ => Err(CodegenError::UnsupportedLanguage(format!("{:?}", language))),
    }
}

/// Per-binding dedup decision (SCE_MESH.md §mesh-10.5).
///
/// The decision composes two facts:
///   1. the transport-level default (`!transport_supplies_dedup`), and
///   2. an optional per-binding upgrade to dedup-safe when the binding
///      pins a reliable substrate the transport itself cannot assume.
///
/// Today the only per-binding upgrade is SOME/IP `protocol: tcp`,
/// which binds the method call to a single TCP stream per
/// client↔server pair and carries the same at-most-once guarantee as
/// custom_tcp. The upgrade is expressed as a conjunction so both
/// inputs flow through every arm: if a future registry flipped
/// SOME/IP's transport-level `supplies_dedup` to `true`, the TCP-pin
/// case would stay correct (no dedup needed) AND the UDP case would
/// defer to the new transport-level claim rather than over-deduping
/// against it.
///
/// If a future transport grows a similar per-binding knob
/// (e.g. Zenoh unicast-only), extend the `if let` below with the
/// same `default && !upgrade` shape.
fn compute_needs_dedup(
    state: &crate::mesh::topology::TransportState,
    transport_supplies_dedup: bool,
) -> bool {
    use crate::mesh::topology::TransportState;
    let default_needs_dedup = !transport_supplies_dedup;
    if let TransportState::Someip { extra, .. } = state {
        let pinned_tcp = extra
            .get("protocol")
            .and_then(|v| v.as_str())
            .is_some_and(|p| p == "tcp");
        return default_needs_dedup && !pinned_tcp;
    }
    default_needs_dedup
}

/// Per-binding ordering decision (SCE_MESH.md §mesh-10.6).
///
/// Composes three facts:
///   1. The per-binding `ordering:` declaration from deploy.yaml
///      ([`OrderingRequirement`]). `None` short-circuits to `false`;
///      no runtime buffer is emitted regardless of transport.
///   2. The transport-level `supplies_ordering` from the registry.
///      When the transport guarantees FIFO natively, the runtime
///      buffer is redundant and is NOT emitted.
///   3. Per-binding SOME/IP `protocol: tcp` upgrade — pins the binding
///      to a single TCP stream per client↔server pair, which also
///      supplies order. Same shape as [`compute_needs_dedup`]'s
///      `pinned_tcp` upgrade.
///
/// Extending this to another transport's per-binding ordering upgrade
/// (e.g. Zenoh `reliability: reliable_ordered` if Zenoh ever exposes
/// that as a per-link knob) follows the same `default && !upgrade`
/// structure.
fn compute_needs_ordering(
    state: &crate::mesh::topology::TransportState,
    transport_supplies_ordering: bool,
    binding_ordering: crate::mesh::deploy::OrderingRequirement,
) -> bool {
    use crate::mesh::deploy::OrderingRequirement;
    use crate::mesh::topology::TransportState;
    if binding_ordering == OrderingRequirement::None {
        return false;
    }
    let default_needs_ordering = !transport_supplies_ordering;
    if let TransportState::Someip { extra, .. } = state {
        let pinned_tcp = extra
            .get("protocol")
            .and_then(|v| v.as_str())
            .is_some_and(|p| p == "tcp");
        return default_needs_ordering && !pinned_tcp;
    }
    default_needs_ordering
}

/// SCE_MESH.md §mesh-10.9 invariant 8: classify the pool + RPC-client
/// rejection surface. The caller has already decided the machine is
/// a pool router (`n_sessions > 1`); this helper inspects the
/// target contexts and returns:
///
/// * `Some(RpcClientKind::MeshRpc)` — any target has
///   `<invoke type="sce:mesh-rpc">` sites, which consume
///   `invoke_correlation_` + `active_invokes_`.
/// * `Some(RpcClientKind::SomeipRpcRequest)` — no mesh-rpc sites,
///   but at least one SOME/IP target with an outbound Request-Reply
///   pattern, which consumes `pending_rpcs_` on a
///   `sessions_[0]`-hard-coded reply dispatch path.
/// * `None` — no router-scoped correlation surface is in use;
///   pool coexistence is safe.
///
/// Mesh-rpc wins reporting priority when both apply so the author
/// lands on the spec-level feature (§mesh-9.5) rather than the
/// by-event-name inference.
fn classify_pool_rpc_client_conflict(
    target_contexts: &[TargetContext],
) -> Option<super::error::RpcClientKind> {
    use super::error::RpcClientKind;
    let has_mesh_rpc_client = target_contexts.iter().any(|t| !t.invoke_sites.is_empty());
    if has_mesh_rpc_client {
        return Some(RpcClientKind::MeshRpc);
    }
    let has_someip_rpc_request_client = target_contexts
        .iter()
        .any(|t| t.has_rpc && matches!(t.state, TargetStateView::Someip { .. }));
    if has_someip_rpc_request_client {
        return Some(RpcClientKind::SomeipRpcRequest);
    }
    None
}

fn generate_cpp_mesh(inputs: MeshCodegenInputs<'_>) -> Result<GeneratedOutput, CodegenError> {
    let MeshCodegenInputs {
        machine_name,
        targets,
        server,
        zenoh_session,
        someip_config,
        custom_tcp_config,
        dds_config,
        subscriptions,
        machine_ordering,
        machine_liveliness,
        machine_outbound_buffer,
        partition_self_name,
        partition_wire21_outbound,
        partition_wire21_inbound,
        scxml_remote_outbound_peers,
        scxml_remote_inbound_peers,
        someip_invoke_service_ids,
        someip_liveness_service_ids,
        someip_machine_liveness_service_ids,
        source_location,
        template_base,
    } = inputs;
    // Validate: every target's transport must be in the registry AND
    // have a template implementation. Two distinct failure modes:
    //   - Unknown transport (not in registry at all)
    //   - Known but not implemented (capabilities known, no template yet)
    // Both fail here at the Rust level — no deferred C++ #error. This is
    // what makes the §mesh-6.4 add-a-transport procedure enforceable: a
    // registry entry missing or half-finished is rejected before codegen.
    for t in targets {
        let name = t.state.transport_name();
        match transport::lookup(name) {
            None => {
                return Err(CodegenError::UnsupportedTransport {
                    transport: name.to_string(),
                    target: t.target.clone(),
                });
            }
            Some(desc) if !desc.implemented => {
                return Err(CodegenError::UnsupportedTransport {
                    transport: name.to_string(),
                    target: t.target.clone(),
                });
            }
            Some(_) => {}
        }
    }

    // Fail fast on event-name collisions: two SCXML events on the same
    // target that collapse to the same C++ constant suffix would emit
    // duplicate `static constexpr` definitions in the generated header,
    // surfacing as a C++ redefinition error far from the actual cause.
    //
    // Scan the union of `event_patterns` (the set the template emits
    // constants for) AND `events` (the raw per-send list from
    // `SendActionSummary.target_events`). Currently the two align since
    // every observed event defaults to FireForget when no prefix matches,
    // but scanning both keeps this check correct if a future template
    // emission keys on the raw event list.
    for t in targets {
        let mut seen: HashMap<String, String> = HashMap::new();
        let names = t
            .event_patterns
            .iter()
            .map(|ep| ep.event.as_str())
            .chain(t.events.iter().map(String::as_str))
            .filter(|e| !e.is_empty());
        for event in names {
            let suffix = event_to_const_suffix(event);
            if let Some(prev) = seen.insert(suffix.clone(), event.to_string()) {
                if prev != event {
                    return Err(CodegenError::EventNameCollision {
                        target: t.target.clone(),
                        suffix,
                        events: vec![prev, event.to_string()],
                    });
                }
            }
        }
    }

    let target_contexts: Vec<TargetContext> = targets
        .iter()
        .map(|t| {
            let stripped = t.target.name();
            let desc = transport::lookup(t.state.transport_name()).expect("transport validated");

            // Per-event SOME/IP IDs live inside `TransportState::Someip`.
            // Non-someip variants have no per-event ID map, so the lookup
            // collapses to `None` and the template's discriminator branch
            // never reads the (still-`None`) ID slots.
            let event_bindings_view: Option<&BTreeMap<String, super::topology::SomeipEventIds>> =
                match &t.state {
                    super::topology::TransportState::Someip { event_bindings, .. } => {
                        Some(event_bindings)
                    }
                    _ => None,
                };

            let event_patterns: Vec<EventPatternContext> = t
                .event_patterns
                .iter()
                .map(|ep| {
                    // Per-event SOME/IP IDs come from `event_bindings` on
                    // the `Someip` variant. Topology validation enforces
                    // one entry per detected event before codegen runs.
                    let ctx_ids = event_bindings_view
                        .and_then(|bs| bs.get(&ep.event))
                        .map(event_ids_to_template);
                    let (field_kind, method_id, event_group_id, event_id, getter_id, setter_id) =
                        ctx_ids.unwrap_or_default();
                    // EventUnsubscribe shares the same event_id as its
                    // paired EventSubscribe. A second register_message_handler
                    // on the same triple silently replaces the first in vsomeip.
                    let skip_receive_handler =
                        CommunicationPattern::from_wire(ep.pattern_kind_value)
                            == Some(CommunicationPattern::Unsubscribe);
                    EventPatternContext {
                        event: ep.event.clone(),
                        event_const: event_to_const_suffix(&ep.event),
                        pattern_kind: ep.pattern_kind_value,
                        reply_event: ep.reply_event.clone(),
                        field_kind,
                        method_id,
                        event_group_id,
                        event_id,
                        getter_id,
                        setter_id,
                        skip_receive_handler,
                    }
                })
                .collect();

            // Detect pattern categories by recovering the symbolic pattern
            // from the cached wire value and consulting its capability.
            // `CommunicationPattern::required_capability()` is the SSOT
            // for "which category does this pattern belong to"; the old
            // per-wire-constant `match` is gone so wire values live solely
            // in pattern.rs.
            use crate::mesh::pattern::CommunicationPattern;
            use crate::mesh::transport::TransportCapability;
            let category_of = |wire: u16| -> Option<TransportCapability> {
                CommunicationPattern::from_wire(wire).map(|p| p.required_capability())
            };
            let has_rpc = event_patterns
                .iter()
                .any(|ep| category_of(ep.pattern_kind) == Some(TransportCapability::RequestReply));
            // SCE_MESH.md §mesh-13: `has_pubsub` fires for either (a) an
            // outbound pub/sub send in the SCXML model, or (b) a
            // machine-lifetime subscription declared in deploy.yaml.
            // The two signals are kept in separate fields so
            // `resolvePattern()`'s outbound-event classification is
            // not polluted by inbound notification names.
            let has_pubsub = event_patterns
                .iter()
                .any(|ep| category_of(ep.pattern_kind) == Some(TransportCapability::PubSub))
                || !t.subscription_events.is_empty();
            let has_field = event_patterns
                .iter()
                .any(|ep| category_of(ep.pattern_kind) == Some(TransportCapability::FieldAccess));
            // Target receives if it has RPC (responses come back), PubSub
            // (notifications), or Field (notifications).
            let has_receive = has_rpc || has_pubsub || has_field;

            let needs_dedup = compute_needs_dedup(&t.state, desc.supplies_dedup);
            let needs_ordering =
                compute_needs_ordering(&t.state, desc.supplies_ordering, t.ordering);

            TargetContext {
                target: t.target.clone(),
                target_stem: stripped.to_string(),
                target_snake: filters::to_snake_case(stripped.to_string()),
                target_pascal: filters::to_pascal_case(stripped.to_string()),
                events: t.events.clone(),
                state: TargetStateView::from_topology(&t.state),
                has_per_target_field: desc.shape.has_per_target_field,
                needs_dedup,
                needs_ordering,
                responders: t.responders.clone(),
                event_patterns,
                has_rpc,
                has_pubsub,
                has_field,
                has_receive,
                invoke_sites: t.invoke_sites.clone(),
                pool_plan: t.pool_plan.clone(),
                retry: t.retry.map(RetryPolicyContext::from),
                auth: t.auth.clone().and_then(AuthPolicyContext::from_config),
            }
        })
        .collect();

    // Discriminator names directly off the typed state — same SSOT the
    // template uses for `target.state.kind`. Sourcing this from
    // `TransportState::transport_name` (instead of the deprecated
    // `ResolvedTarget::transport: String`) guarantees the per-transport
    // include / shared-resource blocks in the template see exactly the
    // set of variants the per-target dispatch will emit code for.
    let mut transport_types: BTreeSet<&str> =
        targets.iter().map(|t| t.state.transport_name()).collect();

    // Server-side transport context (SCE_MESH.md §mesh-13 Session E).
    let server_context = server.map(build_server_context);

    // Include server transport in transport_types so the correct
    // #include directives and shared-resource fields are emitted
    // even for pure-server machines with no client targets.
    if let Some(ref sc) = server_context {
        transport_types.insert(match sc.transport_kind.as_str() {
            "someip" => "someip",
            "zenoh" => "zenoh",
            other => other,
        });
    }

    // SCE_MESH.md §mesh-14.4: session count hosted by this router.
    // A SOME/IP server pool with N declared instances backs N
    // independent SCXML document sessions (one per offered
    // vsomeip instance). Every other router shape runs exactly
    // one session, so the template's sessions_ array collapses
    // to a 1-element brace-init for non-pool deployments.
    let n_sessions: usize = server_context
        .as_ref()
        .and_then(|sc| sc.instance_pool.as_ref())
        .map(|p| p.len())
        .filter(|&len| len > 0)
        .unwrap_or(1);

    // SCE_MESH.md §mesh-14.4 + §mesh-10.9 invariant 8: pool + any outbound
    // RPC client whose correlation state lives in a router-scoped
    // table cannot share a router. Delegation to
    // `classify_pool_rpc_client_conflict` keeps the decision table
    // unit-testable in isolation from the codegen boundary.
    let has_pool = n_sessions > 1;
    if has_pool {
        if let Some(kind) = classify_pool_rpc_client_conflict(&target_contexts) {
            return Err(CodegenError::PoolWithRpcClientUnsupported {
                machine: machine_name.to_string(),
                kind,
            });
        }
    }

    // A device-level custom_tcp listen endpoint emits a server even when no
    // client binding on this machine selected `transport: custom_tcp`. Add
    // the variant to `transport_types` so the template's `{% if "custom_tcp"
    // in transport_types %}` gates fire for the include and server field.
    // SSoT: `CustomTcpTransportConfig::hosts_server` (see deploy.rs).
    if custom_tcp_config.is_some_and(CustomTcpTransportConfig::hosts_server) {
        transport_types.insert("custom_tcp");
    }

    // SCE_MESH.md §mesh-9.6 Session 2: scxml-remote invoke peers that opted
    // into custom_tcp demand the same CustomTcpTransport.h include + any
    // `{% if "custom_tcp" in transport_types %}` scaffolding, even when
    // no `<send>` target on this machine selected custom_tcp. The send
    // target set and scxml-remote peer set are independent surfaces —
    // both feed the include gate.
    if scxml_remote_outbound_peers
        .iter()
        .chain(scxml_remote_inbound_peers.iter())
        .any(|p| p.transport.as_deref() == Some("custom_tcp"))
    {
        transport_types.insert("custom_tcp");
    }

    // SCE_MESH.md §mesh-9.6 Session 4b: scxml-remote invoke peers that opted
    // into someip demand the ScxmlInvokeEndpoint helper + the shared
    // SCE app `<machine>[_<partition>]_sce_app_` (RFC F.X-2), which is
    // orthogonal to any per-`<send>`-target someip application emitted
    // from `targets`. Add the variant so the template's `{% if "someip"
    // in transport_types %}` gate fires for the include block even when
    // no `<send>` target selected someip on this machine.
    if scxml_remote_outbound_peers
        .iter()
        .chain(scxml_remote_inbound_peers.iter())
        .any(|p| p.transport.as_deref() == Some("someip"))
    {
        transport_types.insert("someip");
    }

    // SCE_MESH.md §mesh-9.6 Session 5: scxml-remote invoke peers that opted
    // into zenoh demand `zenoh_session_` to exist (the per-peer
    // ScxmlInvokeEndpoint takes a reference to it) and the
    // `<zenoh.hxx>` include block to fire. Both are gated by the
    // template's `{% if "zenoh" in transport_types %}`, so add the
    // variant when no `<send>` target selected zenoh but a §mesh-9.6 peer
    // did. Mirrors the someip insertion above. The same gate also
    // pulls the existing `zenoh::Session::open(...)` init() block
    // — the §mesh-9.6 endpoint emplace then chains off the session that
    // block produces.
    if scxml_remote_outbound_peers
        .iter()
        .chain(scxml_remote_inbound_peers.iter())
        .any(|p| p.transport.as_deref() == Some("zenoh"))
    {
        transport_types.insert("zenoh");
    }

    // SCE_MESH.md §mesh-10.5: per-binding dedup decision drives machine-level
    // emission. A receiver emits the runtime DedupRouter iff at least one
    // inbound path on the machine actually needs runtime dedup — that's
    // any target with `needs_dedup = true` (typically Zenoh, or a SOME/IP
    // binding without `protocol: tcp`) plus any server-side path whose
    // transport does not supply inherent dedup.
    //
    // Call-site branching lives in the template: client-side SOME/IP
    // receive handlers read `target.needs_dedup`, server-side handlers
    // read `server_needs_dedup`. Mixed-binding machines (e.g. SOME/IP-TCP
    // to one target + Zenoh to another) still emit the DedupRouter once
    // but only the undeduped paths pay the admit cost.
    let any_target_needs_dedup = target_contexts.iter().any(|t| t.needs_dedup);
    let server_needs_dedup = server_context.as_ref().is_some_and(|sc| {
        // Server transport kind is "someip" or "zenoh" today. SOME/IP
        // server sockets default to UDP multicast (per vsomeip.json), so
        // conservatively treat both as needing dedup. If a future deploy
        // schema adds a server-side `protocol: tcp` pin, extend this
        // match to recognise it — same pattern as the target-side
        // `needs_dedup` computation above.
        matches!(sc.transport_kind.as_str(), "someip" | "zenoh" | "dds")
    });
    let has_undeduped_transport = any_target_needs_dedup || server_needs_dedup;

    // SCE_MESH.md §mesh-10.6: machine-level ordering flag drives
    // OrderingBuffer include/member/seq_counter emission. The server
    // role does not yet carry an `ordering:` declaration, so the
    // server side never contributes here — extend the expression
    // alongside any future server-side ordering schema, mirroring how
    // dedup is composed above.
    let has_ordered_binding = target_contexts.iter().any(|t| t.needs_ordering);

    // Pre-escape Zenoh session config into C++ string literals so the template
    // never constructs literals by string concatenation.
    let zenoh_session_json5 = zenoh_session.map(ZenohSessionJson5::from_config);
    let zenoh_session_json5_present = zenoh_session_json5.as_ref().is_some_and(|z| !z.is_empty());

    // Device-shared Zenoh endpoints (listen/connect) surfaced as generated
    // namespace constants. Tests that need the endpoint for raw-peer
    // harnesses read these instead of duplicating the deploy.yaml literal,
    // so a port change in deploy.yaml regenerates the header and the drift
    // surfaces at compile/link time rather than as a runtime flake.
    let zenoh_listen_endpoints: Vec<String> = zenoh_session
        .and_then(|cfg| cfg.listen.clone())
        .unwrap_or_default();
    let zenoh_connect_endpoints: Vec<String> = zenoh_session
        .and_then(|cfg| cfg.connect.clone())
        .unwrap_or_default();

    // SOME/IP device-shared context (application_name). None collapses to
    // empty so the template treats "no someip config" and "someip config
    // without application_name" identically — both fall back to the
    // synthetic `<machine>_<target>` name.
    let someip_transport = someip_config
        .map(SomeipTransportContext::from_config)
        .filter(|s| !s.is_empty());

    // custom_tcp device-shared listen endpoint. Pure-client devices have
    // no `listen:` key and the template renders only client-side code.
    let custom_tcp_transport = custom_tcp_config
        .map(CustomTcpTransportContext::from_config)
        .filter(|s| !s.is_empty());

    // Always present when dds is in play, unlike the transports above: a
    // device with dds bindings joins a domain whether or not deploy.yaml
    // declared the block, so the absent-block case is "domain 0", not
    // "no context".
    let dds_transport = DdsTransportContext::from_config(dds_config);

    let machine_pascal = filters::to_pascal_case(machine_name.to_string());

    let template_name = "mesh/cpp/mesh_transport.h.jinja2";
    let template_path = template_base.join(template_name);
    let template_content =
        std::fs::read_to_string(&template_path).map_err(|e| CodegenError::TemplateRead {
            path: template_path.display().to_string(),
            source: e,
        })?;

    // SCE Protocol-Synthesis RFC §synth-5-O: the transport template imports
    // the shared SCE-MAP marker macro. The mesh codegen builds its own
    // minijinja env (the public `load_templates` helper isn't reachable
    // from inside crate::mesh), so the macro file is loaded explicitly
    // here under the same `_macros/sce_map_marker.jinja2` path callers
    // use in their `{% import %}` statements.
    let macro_template_name = "_macros/sce_map_marker.jinja2";
    let macro_template_path = template_base.join(macro_template_name);
    let macro_template_content =
        std::fs::read_to_string(&macro_template_path).map_err(|e| CodegenError::TemplateRead {
            path: macro_template_path.display().to_string(),
            source: e,
        })?;

    let mut env = minijinja::Environment::new();
    env.add_template_owned(macro_template_name.to_string(), macro_template_content)
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;
    env.add_template("mesh_transport.h.jinja2", &template_content)
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    let tmpl = env
        .get_template("mesh_transport.h.jinja2")
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    // SCE_MESH.md §mesh-10.6.1: per-machine ordering buffer timings.
    // Always emitted, whether or not the machine has an ordered
    // binding — the template only reads inside `{% if
    // has_ordered_binding %}`, but the absent-section default
    // (filled by `MachineConfig::resolved_ordering_timings`) is
    // serialized so a future template branch needing the values
    // cannot crash on undefined.
    let machine_ordering_ctx = serde_json::json!({
        "gap_timeout_ms": machine_ordering.gap_timeout_ms,
        "tick_period_ms": machine_ordering.tick_period_ms,
    });

    // SCE Mesh §mesh-16.7 row 8 (PEER_PARTITIONED): per-machine Zenoh
    // liveliness opt-in. `null` when the deploy.yaml author declared no
    // `liveliness:` section, which the template reads as a falsy gate —
    // zero lines of liveliness code emitted. Present value carries the
    // deploy-validated `lease_ms` (floor enforced by
    // `LivelinessConfig::validation_error`), which the test uses to size
    // the PEER_PARTITIONED observation window.
    let machine_liveliness_ctx = match machine_liveliness {
        Some(l) => serde_json::json!({ "lease_ms": l.lease_ms }),
        None => serde_json::Value::Null,
    };

    // SCE Mesh §mesh-10.10 (OutboundBuffer): per-machine readiness-gated
    // buffering opt-in. `null` when the deploy.yaml author declared no
    // `outbound_buffer:` section — template reads as a falsy gate and
    // emits zero buffer code. Present value carries the deploy-
    // validated `max_pending_per_target` (floor enforced by
    // `OutboundBufferConfig::validation_error`).
    let machine_outbound_buffer_ctx = match machine_outbound_buffer {
        Some(b) => serde_json::json!({
            "max_pending_per_target": b.max_pending_per_target,
        }),
        None => serde_json::Value::Null,
    };

    // SCE_MESH.md §mesh-16.5 wire-21: materialize partition routing context
    // for the template. Outbound entries are keyed by parallel_id with
    // the root partition as value; dedup across parallels sharing the
    // same root so the template emits exactly one shm channel per
    // unique (src_partition, dst_partition) pair. Inbound sources are
    // already sorted + deduplicated by `inject_partition_context_for`.
    // Channel naming mirrors the per-machine shm convention
    // (`/sce_p21_<src>_<dst>`) — one name string per direction, same
    // name on both sides of the wire.
    let wire21_self = partition_self_name.unwrap_or("").to_string();
    let wire21_outbound_routes: Vec<serde_json::Value> = partition_wire21_outbound
        .iter()
        .map(|(parallel_id, dst_partition)| {
            serde_json::json!({
                "parallel_id": parallel_id,
                "dst_partition": dst_partition,
                "channel_name": format!("/sce_p21_{}_{}", wire21_self, dst_partition),
                "dst_partition_snake": filters::to_snake_case(dst_partition.clone()),
            })
        })
        .collect();
    let mut wire21_outbound_unique_dests_map: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for dst_partition in partition_wire21_outbound.values() {
        wire21_outbound_unique_dests_map
            .entry(dst_partition.clone())
            .or_insert_with(|| {
                serde_json::json!({
                    "dst_partition": dst_partition,
                    "dst_partition_snake": filters::to_snake_case(dst_partition.clone()),
                    "channel_name": format!("/sce_p21_{}_{}", wire21_self, dst_partition),
                })
            });
    }
    let wire21_outbound_unique_dests: Vec<serde_json::Value> =
        wire21_outbound_unique_dests_map.into_values().collect();
    let wire21_inbound_sources: Vec<serde_json::Value> = partition_wire21_inbound
        .iter()
        .map(|src_partition| {
            serde_json::json!({
                "src_partition": src_partition,
                "src_partition_snake": filters::to_snake_case(src_partition.clone()),
                "channel_name": format!("/sce_p21_{}_{}", src_partition, wire21_self),
            })
        })
        .collect();

    // SCE Mesh RFC F.X-2: per-binary SCE-namespaced vsomeip Application
    // identity. Replaces the legacy `<machine>_scxml_invoke_app_` per-
    // subsystem app with a single `<machine>[_<partition>]_sce_app_` per
    // partition binary. The optional partition infix closes the latent
    // collision where two partition binaries of the same machine would
    // otherwise both call `create_application("<machine>_scxml_invoke")`
    // and clash on vsomeip's routing-manager application-name uniqueness.
    // F.X-3 onwards, additional SCE-reserved subsystems (region-liveness,
    // future) register on the same app — see SomeipScxmlInvokeEndpoint.h's
    // §mesh-13 docstring for the SCE-vs-OEM rationale that survives
    // consolidation.
    let someip_sce_app_vsomeip_name = sce_app_vsomeip_name(machine_name, partition_self_name);
    let someip_sce_app_field = format!("{someip_sce_app_vsomeip_name}_app_");
    let someip_sce_app_thread = format!("{someip_sce_app_vsomeip_name}_thread_");

    // SCE Mesh RFC F.X-1: per-target SOMEIP service ID constants. Self is
    // present iff this machine is a §mesh-9.6 SOMEIP scxml-invoke participant
    // (the deploy-wide assigner already filtered on participation). Peers
    // are listed for every §mesh-9.6 SOMEIP outbound + inbound peer the codegen
    // sees on this machine; the template emits one named constant per
    // entry (`SCE_SOMEIP_SERVICE_PEER_<peer_name_upper>`) and a single
    // `SCE_SOMEIP_SERVICE_SELF`. Replaces the legacy
    // `serviceIdForMachine(...)` constexpr calls in the template.
    let someip_invoke_service_id_self: Option<u16> =
        someip_invoke_service_ids.get(machine_name).copied();
    let someip_invoke_service_ids_peers: Vec<serde_json::Value> = {
        let mut peer_names: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for peer in scxml_remote_outbound_peers
            .iter()
            .chain(scxml_remote_inbound_peers.iter())
        {
            if peer.transport.as_deref() == Some("someip") {
                peer_names.insert(peer.name.as_str());
            }
        }
        peer_names
            .into_iter()
            .filter_map(|name| {
                someip_invoke_service_ids.get(name).map(|sid| {
                    serde_json::json!({
                        "name": name,
                        "name_upper": name.to_ascii_uppercase(),
                        "service_id_hex": format!("{sid:#06x}"),
                    })
                })
            })
            .collect()
    };
    let someip_invoke_service_id_self_hex: Option<String> =
        someip_invoke_service_id_self.map(|sid| format!("{sid:#06x}"));

    // SCE Mesh RFC F.X-3: per-target SOMEIP region-liveness service ID
    // constants. Self is present iff this binary is a partition (i.e.
    // `partition_self_name.is_some()`) AND the deploy assigner included
    // its `<machine>__P__<partition>` participant key. Sibling partitions
    // are listed for every other partition of the same machine that is
    // also a liveness participant; the template emits one named constant
    // per entry (`SCE_LIVENESS_SERVICE_PEER_<sibling_partition_upper>`)
    // and a single `SCE_LIVENESS_SERVICE_SELF`. Disjoint from F.X-1
    // invoke IDs by sub-range partitioning ([0x8180, 0x81FF]).
    let someip_liveness_service_id_self_hex: Option<String> = partition_self_name.and_then(|p| {
        let key = format!("{machine_name}__P__{p}");
        someip_liveness_service_ids
            .get(&key)
            .map(|sid| format!("{sid:#06x}"))
    });
    let someip_liveness_service_ids_peers: Vec<serde_json::Value> = {
        let self_key = partition_self_name
            .map(|p| format!("{machine_name}__P__{p}"))
            .unwrap_or_default();
        let machine_prefix = format!("{machine_name}__P__");
        someip_liveness_service_ids
            .iter()
            .filter(|(k, _)| k.starts_with(&machine_prefix) && k.as_str() != self_key)
            .map(|(k, sid)| {
                let partition = k
                    .strip_prefix(&machine_prefix)
                    .expect("prefix-filtered above");
                serde_json::json!({
                    "partition": partition,
                    "partition_upper": partition.to_ascii_uppercase(),
                    "service_id_hex": format!("{sid:#06x}"),
                })
            })
            .collect()
    };

    // SCE Mesh RFC F.X-4: per-target SOMEIP machine-level liveness
    // service ID constants ([0x8280, 0x82FF] disjoint sub-range from
    // F.X-1 invoke + F.X-3 region-liveness). Self is present iff this
    // machine opts into SOMEIP machine-level liveness emission (every
    // partition binary of a SOMEIP `liveliness:`-opt-in machine offers
    // the same machine-level service per RFC F.X-4 D5
    // emit-from-every-partition). Peers are every other machine in the
    // deploy that also opts in; codegen emits one named constant per
    // entry (`SCE_MACHINE_LIVENESS_SERVICE_PEER_<peer_machine_upper>`)
    // and a single `SCE_MACHINE_LIVENESS_SERVICE_SELF`.
    let someip_machine_liveness_service_id_self_hex: Option<String> =
        someip_machine_liveness_service_ids
            .get(machine_name)
            .map(|sid| format!("{sid:#06x}"));
    let someip_machine_liveness_service_ids_peers: Vec<serde_json::Value> = {
        someip_machine_liveness_service_ids
            .iter()
            .filter(|(peer_machine, _)| peer_machine.as_str() != machine_name)
            .map(|(peer_machine, sid)| {
                serde_json::json!({
                    "machine": peer_machine,
                    "machine_upper": peer_machine.to_ascii_uppercase(),
                    "service_id_hex": format!("{sid:#06x}"),
                })
            })
            .collect()
    };

    let ctx = minijinja::context! {
        machine_name => machine_name,
        machine_pascal => machine_pascal,
        targets => target_contexts,
        transport_types => transport_types,
        has_undeduped_transport => has_undeduped_transport,
        server_needs_dedup => server_needs_dedup,
        has_ordered_binding => has_ordered_binding,
        machine_ordering => machine_ordering_ctx,
        machine_liveliness => machine_liveliness_ctx,
        machine_outbound_buffer => machine_outbound_buffer_ctx,
        zenoh_session_json5 => zenoh_session_json5,
        zenoh_session_json5_present => zenoh_session_json5_present,
        zenoh_listen_endpoints => zenoh_listen_endpoints,
        zenoh_connect_endpoints => zenoh_connect_endpoints,
        someip_transport => someip_transport,
        custom_tcp_transport => custom_tcp_transport,
        dds_transport => dds_transport,
        machine_subscriptions => subscriptions,
        server => server_context,
        n_sessions => n_sessions,
        partition_wire21_self => wire21_self,
        partition_wire21_outbound_routes => wire21_outbound_routes,
        partition_wire21_outbound_unique_dests => wire21_outbound_unique_dests,
        partition_wire21_inbound_sources => wire21_inbound_sources,
        scxml_remote_outbound_peers => scxml_remote_outbound_peers,
        scxml_remote_inbound_peers => scxml_remote_inbound_peers,
        someip_invoke_service_id_self_hex => someip_invoke_service_id_self_hex,
        someip_invoke_service_ids_peers => someip_invoke_service_ids_peers,
        someip_liveness_service_id_self_hex => someip_liveness_service_id_self_hex,
        someip_liveness_service_ids_peers => someip_liveness_service_ids_peers,
        someip_machine_liveness_service_id_self_hex => someip_machine_liveness_service_id_self_hex,
        someip_machine_liveness_service_ids_peers => someip_machine_liveness_service_ids_peers,
        someip_sce_app_field => someip_sce_app_field,
        someip_sce_app_thread => someip_sce_app_thread,
        someip_sce_app_vsomeip_name => someip_sce_app_vsomeip_name,
        source_location => source_location,
    };

    let code = tmpl
        .render(ctx)
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    Ok(GeneratedOutput {
        files: vec![(format!("{machine_name}_transport.h"), code)],
        ..Default::default()
    })
}

/// SCE Mesh RFC F.X-2: compute the per-binary SCE-namespaced vsomeip
/// Application name. Non-partitioned binaries get `<machine>_sce`;
/// partitioned binaries get `<machine>_<partition>_sce`. The partition
/// infix closes the latent collision where two partition binaries of the
/// same machine would otherwise both call
/// `create_application("<machine>_scxml_invoke")` and clash on vsomeip's
/// routing-manager application-name uniqueness.
///
/// Free function (rather than inlined into the codegen body) so the
/// naming rule is unit-testable in isolation — see the
/// `sce_app_vsomeip_name_*` tests in this module.
fn sce_app_vsomeip_name(machine_name: &str, partition_self_name: Option<&str>) -> String {
    match partition_self_name {
        Some(part) => format!("{machine_name}_{part}_sce"),
        None => format!("{machine_name}_sce"),
    }
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::deploy::ZenohMode;

    // ── RFC F.X-2 sce_app_vsomeip_name ──────────────────────

    #[test]
    fn sce_app_vsomeip_name_non_partitioned() {
        // Without `--partition`, the SCE app is named `<machine>_sce`
        // — single binary per machine, no infix needed. The `_sce`
        // suffix is RFC F.X-2's generic SCE-namespace marker (the
        // app hosts every SCE-reserved subsystem, not just §mesh-9.6 invoke).
        assert_eq!(sce_app_vsomeip_name("brake", None), "brake_sce");
        assert_eq!(
            sce_app_vsomeip_name("scxml_invoke_someip_parent", None),
            "scxml_invoke_someip_parent_sce"
        );
    }

    #[test]
    fn sce_app_vsomeip_name_partitioned_includes_partition_infix() {
        // RFC F.X-2 D2: the partition infix is the latent-bug fix —
        // without it, two partition binaries of the same machine would
        // both call `create_application("<machine>_scxml_invoke")` and
        // clash on vsomeip routing-manager application-name uniqueness.
        assert_eq!(
            sce_app_vsomeip_name("brake", Some("brake_left_part")),
            "brake_brake_left_part_sce"
        );
        assert_eq!(
            sce_app_vsomeip_name("engine_controller", Some("upper_half")),
            "engine_controller_upper_half_sce"
        );
    }

    #[test]
    fn sce_app_vsomeip_name_partitioned_distinct_per_partition() {
        // Two distinct partition names of the same machine produce two
        // distinct vsomeip app names — the property the latent-bug fix
        // hinges on. Without distinct names, the routing manager rejects
        // the second `create_application` call at runtime.
        let left = sce_app_vsomeip_name("brake", Some("left"));
        let right = sce_app_vsomeip_name("brake", Some("right"));
        assert_ne!(left, right);
    }

    // ── cpp_string_literal ───────────────────────────────────

    #[test]
    fn cpp_string_literal_plain_ascii() {
        assert_eq!(cpp_string_literal("peer"), r#""peer""#);
    }

    #[test]
    fn cpp_string_literal_escapes_quote() {
        // Input contains "  → output has \"
        assert_eq!(cpp_string_literal(r#"say "hi""#), r#""say \"hi\"""#);
    }

    #[test]
    fn cpp_string_literal_escapes_backslash() {
        assert_eq!(cpp_string_literal(r"path\file"), r#""path\\file""#);
    }

    #[test]
    fn cpp_string_literal_escapes_newline_and_tab() {
        assert_eq!(cpp_string_literal("a\nb\tc"), r#""a\nb\tc""#);
    }

    #[test]
    fn cpp_string_literal_escapes_control_bytes() {
        // \x01 → \\x01
        let input = "\x01";
        let out = cpp_string_literal(input);
        assert_eq!(out, r#""\x01""#);
    }

    #[test]
    fn cpp_string_literal_nested_json_safe() {
        // serde_json output: "[\"tcp/a:1\"]"
        let json = serde_json::to_string(&vec!["tcp/a:1".to_string()]).unwrap();
        assert_eq!(json, r#"["tcp/a:1"]"#);
        // Embedded as C++ literal: every " escaped
        let cpp = cpp_string_literal(&json);
        assert_eq!(cpp, r#""[\"tcp/a:1\"]""#);
    }

    // ── ZenohSessionJson5 ────────────────────────────────────

    #[test]
    fn zenoh_session_json5_mode_is_complete_literal() {
        let cfg = ZenohTransportConfig {
            mode: Some(ZenohMode::Peer),
            connect: None,
            listen: None,
            config: None,
        };
        let j = ZenohSessionJson5::from_config(&cfg);
        // Literal, ready to drop into insert_json5("mode", <HERE>).
        assert_eq!(j.mode.as_deref(), Some(r#""\"peer\"""#));
    }

    #[test]
    fn zenoh_session_json5_connect_is_complete_literal() {
        let cfg = ZenohTransportConfig {
            mode: None,
            connect: Some(vec!["tcp/192.168.1.1:7447".into()]),
            listen: None,
            config: None,
        };
        let j = ZenohSessionJson5::from_config(&cfg);
        assert_eq!(
            j.connect.as_deref(),
            Some(r#""[\"tcp/192.168.1.1:7447\"]""#)
        );
    }

    #[test]
    fn zenoh_session_json5_endpoint_with_special_chars_is_safe() {
        // Adversarial endpoint string containing ", \, and newline.
        let cfg = ZenohTransportConfig {
            mode: None,
            connect: Some(vec!["a\"b\\c\nd".into()]),
            listen: None,
            config: None,
        };
        let j = ZenohSessionJson5::from_config(&cfg);
        // Unquoting the C++ literal yields valid JSON whose parsed string
        // equals the original input.
        let literal = j.connect.unwrap();
        assert!(literal.starts_with('"') && literal.ends_with('"'));
        // The literal must NOT have unescaped interior quotes.
        let interior = &literal[1..literal.len() - 1];
        // Walk the interior, verifying every unescaped " is actually \".
        let mut prev_backslash = false;
        for (i, c) in interior.char_indices() {
            if c == '"' && !prev_backslash {
                panic!("unescaped \" at position {i} in literal: {literal:?}");
            }
            prev_backslash = c == '\\' && !prev_backslash;
        }
    }

    #[test]
    fn zenoh_session_json5_is_empty_when_all_absent() {
        let cfg = ZenohTransportConfig {
            mode: None,
            connect: None,
            listen: None,
            config: None,
        };
        let j = ZenohSessionJson5::from_config(&cfg);
        assert!(j.is_empty());
    }

    // ── compute_needs_dedup (SCE_MESH.md §mesh-10.5) ─────────────────
    //
    // These tests exercise the per-binding dedup decision in isolation.
    // The helper synthesises a minimal TransportState rather than driving
    // the whole deploy.yaml → topology pipeline — only the fields
    // `compute_needs_dedup` inspects need to be populated.

    use crate::mesh::topology::{SomeipEventIds, SomeipServiceIds, TransportState};
    use std::collections::{BTreeMap, HashMap};

    fn someip_state(extra: HashMap<String, serde_yaml_ng::Value>) -> TransportState {
        TransportState::Someip {
            service: SomeipServiceIds {
                service_id: 0x0001,
                instance_id: 0x0001,
            },
            event_bindings: BTreeMap::<String, SomeipEventIds>::new(),
            extra,
        }
    }

    fn yaml_str(s: &str) -> serde_yaml_ng::Value {
        serde_yaml_ng::Value::String(s.to_string())
    }

    #[test]
    fn someip_without_protocol_needs_dedup() {
        // Default in-tree SOME/IP fixture: no `protocol:` key → UDP by
        // vsomeip convention → runtime dedup required.
        let state = someip_state(HashMap::new());
        assert!(compute_needs_dedup(&state, /* supplies_dedup */ false));
    }

    #[test]
    fn someip_with_protocol_udp_needs_dedup() {
        // Explicit `protocol: udp` — UDP confirmed, dedup required.
        let mut extra = HashMap::new();
        extra.insert("protocol".to_string(), yaml_str("udp"));
        let state = someip_state(extra);
        assert!(compute_needs_dedup(&state, false));
    }

    #[test]
    fn someip_with_protocol_tcp_skips_dedup() {
        // `protocol: tcp` pins the binding to a single TCP stream, so
        // runtime dedup is not needed for this binding. The
        // DedupRouter member may still be emitted at machine level if a
        // sibling binding is undeduped; only this call site is cheap.
        let mut extra = HashMap::new();
        extra.insert("protocol".to_string(), yaml_str("tcp"));
        let state = someip_state(extra);
        assert!(!compute_needs_dedup(&state, false));
    }

    #[test]
    fn someip_with_unknown_protocol_falls_back_to_dedup() {
        // Any string that is not "tcp" falls back to the UDP-default
        // treatment. Keeps the classifier conservative against typos.
        let mut extra = HashMap::new();
        extra.insert("protocol".to_string(), yaml_str("sctp"));
        let state = someip_state(extra);
        assert!(compute_needs_dedup(&state, false));
    }

    #[test]
    fn zenoh_always_needs_dedup_regardless_of_extra() {
        // Zenoh lacks a per-binding TCP pin — the extra map cannot flip
        // the decision. The transport-level `supplies_dedup: false`
        // propagates straight through.
        let state = TransportState::Zenoh {
            key: "brake/cmd".to_string(),
            extra: HashMap::new(),
        };
        assert!(compute_needs_dedup(&state, false));
    }

    #[test]
    fn local_never_needs_dedup() {
        // local dispatch is in-process — duplicates are physically
        // impossible. Transport-level supplies_dedup=true → false.
        assert!(!compute_needs_dedup(&TransportState::Local, true));
    }

    #[test]
    fn custom_tcp_never_needs_dedup() {
        // Single TCP stream per binding.
        let state = TransportState::CustomTcp {
            connect: "127.0.0.1:55821".to_string(),
            extra: HashMap::new(),
        };
        assert!(!compute_needs_dedup(&state, true));
    }

    #[test]
    fn shm_never_needs_dedup() {
        // Single shared ring with READY_MAGIC gating.
        let state = TransportState::Shm {
            arena_bytes: None,
            ring_capacity: None,
        };
        assert!(!compute_needs_dedup(&state, true));
    }

    // ── compute_needs_ordering (SCE_MESH.md §mesh-10.6) ─────────────
    //
    // Exercise the per-binding ordering decision across the four
    // relevant axes: transport supplies_ordering, per-binding
    // ordering declaration, and SOME/IP protocol: tcp upgrade.

    use crate::mesh::deploy::OrderingRequirement;

    #[test]
    fn ordering_none_never_emits_buffer() {
        // The binding declared OrderingRequirement::None — no buffer
        // regardless of transport. Covers zenoh + someip UDP.
        let zenoh = TransportState::Zenoh {
            key: "k".into(),
            extra: HashMap::new(),
        };
        assert!(!compute_needs_ordering(
            &zenoh,
            false,
            OrderingRequirement::None
        ));
        let someip = someip_state(HashMap::new());
        assert!(!compute_needs_ordering(
            &someip,
            false,
            OrderingRequirement::None
        ));
    }

    #[test]
    fn zenoh_with_ordering_required_emits_buffer() {
        // Zenoh has supplies_ordering=false; binding demands order →
        // runtime OrderingBuffer is required.
        let state = TransportState::Zenoh {
            key: "k".into(),
            extra: HashMap::new(),
        };
        assert!(compute_needs_ordering(
            &state,
            false,
            OrderingRequirement::Required
        ));
    }

    #[test]
    fn someip_udp_with_ordering_required_emits_buffer() {
        // Default SOME/IP (no protocol key or explicit udp) → UDP →
        // needs runtime buffer.
        let state = someip_state(HashMap::new());
        assert!(compute_needs_ordering(
            &state,
            false,
            OrderingRequirement::Required
        ));

        let mut udp_extra = HashMap::new();
        udp_extra.insert("protocol".to_string(), yaml_str("udp"));
        let state_udp = someip_state(udp_extra);
        assert!(compute_needs_ordering(
            &state_udp,
            false,
            OrderingRequirement::Required
        ));
    }

    #[test]
    fn someip_tcp_with_ordering_required_skips_buffer() {
        // Per-binding TCP pin supplies order per stream — no runtime
        // buffer even though the transport-level flag is false.
        let mut extra = HashMap::new();
        extra.insert("protocol".to_string(), yaml_str("tcp"));
        let state = someip_state(extra);
        assert!(!compute_needs_ordering(
            &state,
            false,
            OrderingRequirement::Required
        ));
    }

    #[test]
    fn someip_unknown_protocol_falls_back_to_buffer() {
        // Conservative against typos: only the exact literal "tcp"
        // lifts the requirement. Matches compute_needs_dedup's
        // policy so the two helpers treat edge cases identically.
        let mut extra = HashMap::new();
        extra.insert("protocol".to_string(), yaml_str("sctp"));
        let state = someip_state(extra);
        assert!(compute_needs_ordering(
            &state,
            false,
            OrderingRequirement::Required
        ));
    }

    #[test]
    fn local_with_ordering_required_skips_buffer() {
        // In-process direct dispatch — transport supplies_ordering=true
        // short-circuits the decision to `false`.
        assert!(!compute_needs_ordering(
            &TransportState::Local,
            true,
            OrderingRequirement::Required
        ));
    }

    #[test]
    fn custom_tcp_with_ordering_required_skips_buffer() {
        // TCP stream preserves order — buffer is redundant.
        let state = TransportState::CustomTcp {
            connect: "127.0.0.1:9000".into(),
            extra: HashMap::new(),
        };
        assert!(!compute_needs_ordering(
            &state,
            true,
            OrderingRequirement::Required
        ));
    }

    #[test]
    fn shm_with_ordering_required_skips_buffer() {
        // FIFO ring preserves order.
        let state = TransportState::Shm {
            arena_bytes: None,
            ring_capacity: None,
        };
        assert!(!compute_needs_ordering(
            &state,
            true,
            OrderingRequirement::Required
        ));
    }

    // ── classify_pool_rpc_client_conflict (SCE_MESH.md §mesh-10.9 invariant 8) ─────
    //
    // The generated TransportRouter's `invoke_correlation_`,
    // `active_invokes_`, and `pending_rpcs_` are all router-scoped
    // containers. A §mesh-14.4 server pool hosts N sessions under one router,
    // so any router-scoped RPC-client correlation would alias across
    // sessions. `classify_pool_rpc_client_conflict` names the surface
    // that drove the rejection so the author can act on the repair
    // suggestion. Tests cover each branch of the decision table in
    // isolation from the full `generate_cpp_mesh` pipeline.
    //
    // Mutation guide (drop into the helper body to verify these tests
    // are load-bearing):
    //   * Invert priority → mesh-rpc case asserts MeshRpc, SomeipRpcRequest.
    //   * Drop the `matches!(TargetStateView::Someip { .. })` filter →
    //     zenoh_with_rpc_is_safe fails with a spurious SomeipRpcRequest.
    //   * Drop the mesh-rpc branch entirely → mesh_rpc_client_rejects_as_mesh_rpc
    //     fails because the helper falls through to SomeipRpcRequest or None.

    fn mk_target_context(
        state: TargetStateView,
        has_rpc: bool,
        invoke_sites: Vec<crate::mesh::topology::MeshRpcInvokeSite>,
    ) -> TargetContext {
        TargetContext {
            target: crate::mesh::target::TargetId::new("#probe").unwrap(),
            target_stem: "probe".into(),
            target_snake: "probe".into(),
            target_pascal: "Probe".into(),
            events: Vec::new(),
            state,
            has_per_target_field: false,
            needs_dedup: false,
            needs_ordering: false,
            // §14.6 same-target default: the probe target answers for
            // itself, which is what an absent `reply_from:` yields.
            responders: vec!["probe".into()],
            event_patterns: Vec::new(),
            has_rpc,
            has_pubsub: false,
            has_field: false,
            has_receive: false,
            invoke_sites,
            pool_plan: None,
            retry: None,
            auth: None,
        }
    }

    fn sample_invoke_site() -> crate::mesh::topology::MeshRpcInvokeSite {
        crate::mesh::topology::MeshRpcInvokeSite {
            state_name: "compute".into(),
            invoke_id: "inv-0".into(),
            field_suffix: "inv_0".into(),
            mesh_event: "service.request.compute".into(),
            deadline_ms: None,
            params: Vec::new(),
        }
    }

    fn someip_state_no_extra() -> TargetStateView {
        TargetStateView::Someip {
            service: SomeipServiceLiterals {
                service_id: "0x0001".into(),
                instance_id: "0x0001".into(),
            },
            extra: HashMap::new(),
        }
    }

    #[test]
    fn classify_returns_none_without_any_rpc_client() {
        // Pure server pool machine (no outbound RPC client sites at all).
        // No router-scoped correlation table is in use, so pool coexistence
        // is safe and the helper must not flag a rejection.
        let tc = mk_target_context(someip_state_no_extra(), false, Vec::new());
        assert_eq!(classify_pool_rpc_client_conflict(&[tc]), None);
    }

    #[test]
    fn classify_returns_none_on_empty_targets() {
        // Server-only machine with no client targets at all — the common
        // pool shape. Helper must return None, not panic, on an empty slice.
        assert_eq!(classify_pool_rpc_client_conflict(&[]), None);
    }

    #[test]
    fn mesh_rpc_client_rejects_as_mesh_rpc() {
        // Target has `<invoke type="sce:mesh-rpc">` sites. Regardless of
        // transport, `invoke_correlation_` + `active_invokes_` are
        // router-scoped, so pool coexistence would alias invoke_id entries.
        let tc = mk_target_context(someip_state_no_extra(), false, vec![sample_invoke_site()]);
        assert_eq!(
            classify_pool_rpc_client_conflict(&[tc]),
            Some(super::super::error::RpcClientKind::MeshRpc)
        );
    }

    #[test]
    fn someip_rpc_request_rejects_as_someip_rpc_request() {
        // SOME/IP target with `has_rpc` but no mesh-rpc invoke sites —
        // classic `<send event="service.request.X">` shape. The generated
        // `pending_rpcs_` table is router-scoped and the client-side
        // receive handler hard-codes `sessions_[0]` dispatch.
        let tc = mk_target_context(someip_state_no_extra(), true, Vec::new());
        assert_eq!(
            classify_pool_rpc_client_conflict(&[tc]),
            Some(super::super::error::RpcClientKind::SomeipRpcRequest)
        );
    }

    #[test]
    fn zenoh_with_rpc_is_safe() {
        // Zenoh's `session.get()` on_reply closure correlates natively per
        // query handle — no router-scoped `pending_rpcs_` entry is
        // emitted. A pool router (§mesh-14.4 excludes Zenoh server pools anyway)
        // that had a Zenoh RPC client target would not trigger this
        // rejection surface. Guards against a future Zenoh server-pool
        // extension accidentally treating Zenoh clients as unsafe.
        let tc = mk_target_context(
            TargetStateView::Zenoh {
                key: "vehicle/probe".into(),
                extra: HashMap::new(),
            },
            true,
            Vec::new(),
        );
        assert_eq!(classify_pool_rpc_client_conflict(&[tc]), None);
    }

    #[test]
    fn mesh_rpc_priority_wins_over_someip_rpc_request() {
        // Machine has BOTH `<invoke>` sites (on one target) AND a SOME/IP
        // `<send>` RpcRequest client (on another target). Report mesh-rpc
        // first — its surface is spec-level (§mesh-9.5) so the repair suggestion
        // is more tractable than the by-event-name inference. Also keeps
        // the single-diagnostic shape of `CodegenError` stable.
        let mesh_rpc_target =
            mk_target_context(someip_state_no_extra(), false, vec![sample_invoke_site()]);
        let send_rpc_target = mk_target_context(someip_state_no_extra(), true, Vec::new());
        assert_eq!(
            classify_pool_rpc_client_conflict(&[send_rpc_target, mesh_rpc_target]),
            Some(super::super::error::RpcClientKind::MeshRpc)
        );
    }
}
