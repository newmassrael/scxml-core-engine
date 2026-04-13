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
// Adding a new transport:
//   1. Update `transport_shape()` with the transport's router-field shape
//      (per-target field? shared session?).
//   2. Update `pattern::transport_capabilities()` with the transport's
//      supported communication patterns.
//   3. If the transport has device-shared session config, add a typed
//      struct field to `deploy::TransportConfigs` (mirror of
//      `ZenohTransportConfig`). `serde` + `deny_unknown_fields` then
//      reject invalid values at parse time — no post-parse extraction
//      pass is required.
//   4. In `lib.rs`, read the new session config via
//      `DeployConfig::device_for_machine(name)` and pass it to
//      `generate_mesh`. Pre-escape for C++ via `cpp_string_literal`
//      before inserting into the template context.
//   5. Add {% elif %} blocks in mesh_transport.h.jinja2 at the "NEW
//      TRANSPORT" extension points.
// The template is the single source of truth for emitted C++ code.

use crate::filters;
use crate::generator::{GeneratedOutput, Language};
use crate::mesh::deploy::ZenohTransportConfig;
use crate::mesh::error::CodegenError;
use crate::mesh::topology::ResolvedTarget;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;

// ── Transport shape metadata ─────────────────────────────────

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
    ///
    /// A single flag suffices today: no current transport declares a
    /// per-target field without a matching ctor initializer. Split into
    /// two flags only when a concrete transport requires it.
    pub has_per_target_field: bool,
    /// Does this transport use a device-shared session resource?
    /// `true` for zenoh. The template emits the shared field once per
    /// transport (not per target) and initializes it in `init()`.
    pub has_shared_session: bool,
}

/// Return the router-field shape for a given transport name.
///
/// Unknown transports default to "per-target" shape (conservative — matches
/// the local/shm/someip pattern). The template's `#error` fallback still
/// catches unsupported transports at C++ compile time.
pub fn transport_shape(transport: &str) -> TransportShape {
    match transport {
        "local" | "shm" | "someip" => TransportShape {
            has_per_target_field: true,
            has_shared_session: false,
        },
        "zenoh" => TransportShape {
            has_per_target_field: false,
            has_shared_session: true,
        },
        _ => TransportShape {
            has_per_target_field: true,
            has_shared_session: false,
        },
    }
}

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
#[derive(Debug, Clone, Default, serde::Serialize)]
struct ZenohSessionJson5 {
    /// e.g. `"\"peer\""` (a C++ literal whose runtime value is `"peer"`).
    mode: Option<String>,
    /// e.g. `"[\"tcp/host:7447\"]"` as a C++ literal.
    connect: Option<String>,
    listen: Option<String>,
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
        Self {
            mode: mode_json.as_deref().map(cpp_string_literal),
            connect: connect_json.as_deref().map(cpp_string_literal),
            listen: listen_json.as_deref().map(cpp_string_literal),
        }
    }

    fn is_empty(&self) -> bool {
        self.mode.is_none() && self.connect.is_none() && self.listen.is_none()
    }
}

// ── Template context ─────────────────────────────────────────

/// Template context for a single resolved send target.
#[derive(Debug, Clone, serde::Serialize)]
struct TargetContext {
    target: String,
    target_stem: String,
    target_snake: String,
    target_pascal: String,
    events: Vec<String>,
    transport: String,
    extra: HashMap<String, serde_yaml_ng::Value>,
    /// Emit a per-target field in TransportRouter and a matching ctor
    /// initializer? Data-driven — removes transport-name hardcoding from
    /// the template's field/ctor sections.
    has_per_target_field: bool,
}

// ── Public entry point ───────────────────────────────────────

/// Generate mesh transport code for a machine's resolved targets.
///
/// `zenoh_session` is the validated device-shared Zenoh session config from
/// `DeployConfig::topology[device].transports.zenoh`. Pass `None` when the
/// device has no zenoh bindings or no zenoh `transports:` block.
pub fn generate_mesh(
    machine_name: &str,
    targets: &[ResolvedTarget],
    zenoh_session: Option<&ZenohTransportConfig>,
    language: Language,
    template_base: &Path,
) -> Result<GeneratedOutput, CodegenError> {
    if targets.is_empty() {
        return Ok(GeneratedOutput { files: vec![] });
    }

    match language {
        Language::Cpp => generate_cpp_mesh(machine_name, targets, zenoh_session, template_base),
        _ => Err(CodegenError::UnsupportedLanguage(format!("{:?}", language))),
    }
}

fn generate_cpp_mesh(
    machine_name: &str,
    targets: &[ResolvedTarget],
    zenoh_session: Option<&ZenohTransportConfig>,
    template_base: &Path,
) -> Result<GeneratedOutput, CodegenError> {
    let target_contexts: Vec<TargetContext> = targets
        .iter()
        .map(|t| {
            let stripped = t.target.trim_start_matches('#');
            let shape = transport_shape(&t.transport);
            TargetContext {
                target: t.target.clone(),
                target_stem: stripped.to_string(),
                target_snake: filters::to_snake_case(stripped.to_string()),
                target_pascal: filters::to_pascal_case(stripped.to_string()),
                events: t.events.clone(),
                transport: t.transport.clone(),
                extra: t.extra.clone(),
                has_per_target_field: shape.has_per_target_field,
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

    // ── transport_shape ──────────────────────────────────────

    #[test]
    fn shape_local_is_per_target() {
        let s = transport_shape("local");
        assert!(s.has_per_target_field);
        assert!(!s.has_shared_session);
    }

    #[test]
    fn shape_someip_is_per_target() {
        let s = transport_shape("someip");
        assert!(s.has_per_target_field);
        assert!(!s.has_shared_session);
    }

    #[test]
    fn shape_zenoh_is_shared() {
        let s = transport_shape("zenoh");
        assert!(!s.has_per_target_field);
        assert!(s.has_shared_session);
    }

    #[test]
    fn shape_unknown_defaults_to_per_target() {
        let s = transport_shape("iceoryx2");
        assert!(s.has_per_target_field);
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
        };
        let j = ZenohSessionJson5::from_config(&cfg);
        assert!(j.is_empty());
    }
}
