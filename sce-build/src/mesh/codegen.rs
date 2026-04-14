// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh codegen dispatcher — per-target transport template rendering.
//
// Each target routes through its deploy.yaml-bound transport. Mixed transports
// are supported: e.g. #motor via local, #display via shm, #logger via someip,
// #telemetry via zenoh. The unified template generates a single TransportRouter
// that dispatches per-target to the appropriate transport-specific send function.
//
// Adding a new transport — two required changes:
//   1. Add one entry to `transport::lookup()` (shape + capabilities)
//   2. Add {% elif %} blocks in mesh_transport.h.jinja2
// If the transport has device-shared session config, also:
//   3. Add a typed struct field to `deploy::TransportConfigs`
//   4. Thread the config through `generate_mesh()` in `lib.rs`

use crate::filters;
use crate::generator::{GeneratedOutput, Language};
use crate::mesh::deploy::{SomeipTransportConfig, ZenohTransportConfig};
use crate::mesh::error::CodegenError;
use crate::mesh::topology::ResolvedTarget;
use crate::mesh::transport;
use std::collections::BTreeSet;
use std::collections::HashMap;
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
/// at runtime (SCE_MESH.md §13, §14) — deploy.yaml-level overrides
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
/// (SCE_MESH.md §13). The template uses it verbatim as the argument to
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

// ── Template context ─────────────────────────────────────────

/// SOME/IP service identity, pre-rendered as `0xNNNN` hex strings so the
/// template emits literals without probing integer formatters. `None` for
/// non-SOME/IP targets.
#[derive(Debug, Clone, serde::Serialize)]
struct SomeipServiceLiterals {
    service_id: String,
    instance_id: String,
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
    transport: String,
    extra: HashMap<String, serde_yaml_ng::Value>,
    /// Typed SOME/IP service identity. Present only when `transport ==
    /// "someip"` — template dispatches on `{% if target.someip_service %}`
    /// rather than probing `extra.service_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    someip_service: Option<SomeipServiceLiterals>,
    /// Emit a per-target field in TransportRouter and a matching ctor
    /// initializer? Data-driven — removes transport-name hardcoding from
    /// the template's field/ctor sections.
    has_per_target_field: bool,
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
}

/// Per-event pattern context for template rendering.
///
/// Carries both the pattern classification and the per-event SOME/IP
/// numeric IDs so the template can emit per-event constants and dispatch
/// on event name (different SCXML events on the same target can use
/// different methods or event groups, SCE_MESH.md §14).
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

/// Fan a [`SomeipEventIds`] variant into the per-event template fields.
/// Returns `(field_kind, method_id, event_group_id, event_id, getter_id,
/// setter_id)` with only the fields matching the variant populated.
/// Centralizes the single "variant → template fields" translation so
/// the template never has to probe which option happened to be set.
fn event_ids_to_template(
    ids: &crate::mesh::topology::SomeipEventIds,
) -> (
    Option<&'static str>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
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
        SomeipEventIds::EventGroup { event_group_id, event_id } => (
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

// ── Public entry point ───────────────────────────────────────

/// Generate mesh transport code for a machine's resolved targets.
///
/// `zenoh_session` and `someip_config` come from the owning device's
/// `transports:` block (`DeployConfig::topology[device].transports.*`).
/// Each is `None` when the device has no binding of that transport, or
/// when the corresponding `transports:` block is absent.
pub fn generate_mesh(
    machine_name: &str,
    targets: &[ResolvedTarget],
    zenoh_session: Option<&ZenohTransportConfig>,
    someip_config: Option<&SomeipTransportConfig>,
    language: Language,
    template_base: &Path,
) -> Result<GeneratedOutput, CodegenError> {
    if targets.is_empty() {
        return Ok(GeneratedOutput { files: vec![] });
    }

    match language {
        Language::Cpp => generate_cpp_mesh(
            machine_name,
            targets,
            zenoh_session,
            someip_config,
            template_base,
        ),
        _ => Err(CodegenError::UnsupportedLanguage(format!("{:?}", language))),
    }
}

fn generate_cpp_mesh(
    machine_name: &str,
    targets: &[ResolvedTarget],
    zenoh_session: Option<&ZenohTransportConfig>,
    someip_config: Option<&SomeipTransportConfig>,
    template_base: &Path,
) -> Result<GeneratedOutput, CodegenError> {
    // Validate: every target's transport must be in the registry AND
    // have a template implementation. Two distinct failure modes:
    //   - Unknown transport (not in registry at all)
    //   - Known but not implemented (capabilities known, no template yet)
    // Both fail here at the Rust level — no deferred C++ #error.
    for t in targets {
        match transport::lookup(&t.transport) {
            None => {
                return Err(CodegenError::UnsupportedTransport {
                    transport: t.transport.clone(),
                    target: t.target.clone(),
                });
            }
            Some(desc) if !desc.implemented => {
                return Err(CodegenError::UnsupportedTransport {
                    transport: t.transport.clone(),
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
            let desc = transport::lookup(&t.transport).expect("transport validated");

            let event_patterns: Vec<EventPatternContext> = t
                .event_patterns
                .iter()
                .map(|ep| {
                    // Per-event SOME/IP IDs come from `event_bindings`,
                    // populated by `topology::finalize_targets`. For
                    // non-someip targets `event_bindings` is empty; for
                    // someip targets every detected event has an entry
                    // (topology validation enforces this before codegen).
                    let ctx_ids = t.event_bindings.get(&ep.event).map(event_ids_to_template);
                    let (field_kind, method_id, event_group_id, event_id, getter_id, setter_id) =
                        ctx_ids.unwrap_or_default();
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
            let has_pubsub = event_patterns
                .iter()
                .any(|ep| category_of(ep.pattern_kind) == Some(TransportCapability::PubSub));
            let has_field = event_patterns
                .iter()
                .any(|ep| category_of(ep.pattern_kind) == Some(TransportCapability::FieldAccess));
            // Target receives if it has RPC (responses come back), PubSub
            // (notifications), or Field (notifications).
            let has_receive = has_rpc || has_pubsub || has_field;

            TargetContext {
                target: t.target.clone(),
                target_stem: stripped.to_string(),
                target_snake: filters::to_snake_case(stripped.to_string()),
                target_pascal: filters::to_pascal_case(stripped.to_string()),
                events: t.events.clone(),
                transport: t.transport.clone(),
                extra: t.extra.clone(),
                someip_service: t.someip_service.map(|s| SomeipServiceLiterals {
                    service_id: fmt_someip_id(s.service_id),
                    instance_id: fmt_someip_id(s.instance_id),
                }),
                has_per_target_field: desc.shape.has_per_target_field,
                event_patterns,
                has_rpc,
                has_pubsub,
                has_field,
                has_receive,
            }
        })
        .collect();

    let transport_types: BTreeSet<&str> =
        target_contexts.iter().map(|t| t.transport.as_str()).collect();

    // Pre-escape Zenoh session config into C++ string literals so the template
    // never constructs literals by string concatenation.
    let zenoh_session_json5 = zenoh_session.map(ZenohSessionJson5::from_config);
    let zenoh_session_json5_present = zenoh_session_json5
        .as_ref()
        .map(|z| !z.is_empty())
        .unwrap_or(false);

    // SOME/IP device-shared context (application_name). None collapses to
    // empty so the template treats "no someip config" and "someip config
    // without application_name" identically — both fall back to the
    // synthetic `<machine>_<target>` name.
    let someip_transport =
        someip_config.map(SomeipTransportContext::from_config).filter(|s| !s.is_empty());

    let machine_pascal = filters::to_pascal_case(machine_name.to_string());

    let template_name = "mesh/cpp/mesh_transport.h.jinja2";
    let template_path = template_base.join(template_name);
    let template_content =
        std::fs::read_to_string(&template_path).map_err(|e| CodegenError::TemplateRead {
            path: template_path.display().to_string(),
            source: e,
        })?;

    let mut env = minijinja::Environment::new();
    env.add_template("mesh_transport.h.jinja2", &template_content)
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    let tmpl = env
        .get_template("mesh_transport.h.jinja2")
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    let ctx = minijinja::context! {
        machine_name => machine_name,
        machine_pascal => machine_pascal,
        targets => target_contexts,
        transport_types => transport_types,
        zenoh_session_json5 => zenoh_session_json5,
        zenoh_session_json5_present => zenoh_session_json5_present,
        someip_transport => someip_transport,
    };

    let code = tmpl
        .render(ctx)
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    Ok(GeneratedOutput {
        files: vec![(format!("{machine_name}_transport.h"), code)],
    })
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::deploy::ZenohMode;

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
        assert_eq!(j.connect.as_deref(), Some(r#""[\"tcp/192.168.1.1:7447\"]""#));
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
                panic!(
                    "unescaped \" at position {i} in literal: {literal:?}"
                );
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
}
