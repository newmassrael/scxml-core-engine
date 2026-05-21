// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge AST export — wire-stable JSON envelope for external
// consumers (sce-db-gen, sce-eventstore-adapter, sce-ui-gen, etc.).
//
// The schema is `apis/forge-ast.v1.schema.json`; the version-bump
// policy lives in `docs/SCE_FORGE_AST.md`. This module only owns the
// in-process wrapper struct and the serialise-to-writer helper —
// every consumer (the CLI, future build-script callers) goes through
// `emit_to_writer` so the envelope shape stays single-sourced.

use crate::forge::model::ParsedForge;
use serde::Serialize;

/// Wire-format version. Bumped only when a non-additive change to the
/// envelope or inner shape lands; new optional fields stay on v1 per
/// `apis/forge-ast.v1.schema.json` §"additionalProperties" semantics.
pub const FORGE_AST_WIRE_VERSION: u32 = 1;

/// Schema lifecycle marker — *not* a wire field. The diagnostic schema
/// (`schemas/sce-diagnostic.v1.schema.json`) carries `x-sce-schema-status`
/// in the file header alone, and we follow that precedent: the
/// schema-about-the-schema lives next to the schema, not in the
/// payload. Flipping `pre-release` → `stable` requires updating this
/// constant AND the schema file's `x-sce-schema-status` header in one
/// commit; the drift test
/// (`envelope_constants_match_schema_header`) guards the two-way sync.
pub const FORGE_AST_SCHEMA_STATUS: &str = "pre-release";

/// SCE producer version string. Stamped onto the envelope when the
/// `--emit-ast` flag fires so downstream consumers reporting issues
/// can pin the exact SCE release that produced a payload. Pulled
/// from `Cargo.toml` at compile time — moves automatically with each
/// release cut.
pub const SCE_PRODUCER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Envelope serialised as the outer JSON object.
///
/// Borrowed reference to `ParsedForge` so the CLI's existing parsed
/// value is reused without a clone. The lifetime carries from the
/// caller's owned `ParsedForge` value.
///
/// Wire shape is `{v, sce_producer_version?, ast}` — schema lifecycle
/// status lives in the schema file header (see
/// `FORGE_AST_SCHEMA_STATUS` doc). The producer version is optional
/// (`None` skipped from serialisation) so the field is purely
/// additive within v1.
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
pub struct ForgeAstEnvelope<'a> {
    /// Always `FORGE_AST_WIRE_VERSION` (1 for this schema revision).
    pub v: u32,
    /// SCE release that produced this payload. Optional for back-compat
    /// with consumers reading older envelopes — `None` omits the
    /// field. The constructor [`ForgeAstEnvelope::new`] populates it
    /// from [`SCE_PRODUCER_VERSION`]; tests that need byte-equal
    /// goldens across SCE versions can use [`ForgeAstEnvelope::new_unversioned`]
    /// to suppress the stamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sce_producer_version: Option<&'a str>,
    /// Parsed forge document body — mirrors `serde_json::to_string(&ParsedForge)`.
    pub ast: &'a ParsedForge,
}

/// Owned variant for schema generation — schemars 0.8 cannot synthesise
/// a top-level schema from a struct that borrows (`'a` lifetime),
/// because the generated `schema_for!` invocation has no place to
/// thread the lifetime. The owned variant carries identical fields so
/// the generated schema is byte-equal to what the borrowing wire
/// envelope produces; the drift test instantiates this type and
/// asserts the resulting JSON Schema matches the checked-in
/// `apis/forge-ast.v1.schema.json`.
///
/// Only used by tests. Not exposed on the wire; not part of the public
/// API. Adding a field here without mirroring it on `ForgeAstEnvelope`
/// (or vice versa) breaks the drift guard.
#[cfg(test)]
#[derive(Debug, schemars::JsonSchema)]
#[schemars(
    deny_unknown_fields,
    title = "SCE Forge AST (wire schema v1)",
    description = "Compiler-IR-faithful serialization of a parsed SCE Forge document, emitted by `sce-codegen generate --emit-ast=<path>`. Authoritative definitions live in sce-build/src/forge/model.rs (`ForgeDocument`, `ParsedForge`). Consumers MUST ignore unknown fields inside `ast.document` (which mirrors the Rust IR type tree); the top-level envelope is closed. See docs/SCE_FORGE_AST.md for the consumer contract."
)]
#[allow(dead_code)] // Fields are read by the JsonSchema derive macro.
struct ForgeAstEnvelopeOwned {
    /// Always `FORGE_AST_WIRE_VERSION` (1 for this schema revision).
    v: u32,
    /// SCE release that produced this payload. Optional in the schema
    /// so older consumers and `new_unversioned()` golden fixtures
    /// remain compatible.
    sce_producer_version: Option<String>,
    /// Parsed forge document body — mirrors `serde_json::to_string(&ParsedForge)`.
    ast: ParsedForge,
}

impl<'a> ForgeAstEnvelope<'a> {
    /// Production constructor — stamps the current SCE release into
    /// `sce_producer_version`. Used by every `--emit-ast` call site.
    pub fn new(ast: &'a ParsedForge) -> Self {
        Self {
            v: FORGE_AST_WIRE_VERSION,
            sce_producer_version: Some(SCE_PRODUCER_VERSION),
            ast,
        }
    }

    /// Test-only / golden-fixture constructor — omits the producer
    /// version so byte-equal goldens remain stable across SCE
    /// releases. Production code MUST use [`new`] instead so issue
    /// reports can name the exact SCE revision.
    pub fn new_unversioned(ast: &'a ParsedForge) -> Self {
        Self {
            v: FORGE_AST_WIRE_VERSION,
            sce_producer_version: None,
            ast,
        }
    }
}

/// Serialise an envelope to any `std::io::Write` sink as
/// pretty-printed JSON with a trailing newline.
///
/// Pretty-printed because the primary consumer is human inspection +
/// `jq`-style tooling; the diff guard test in
/// `sce-build/tests/forge_ast_export.rs` round-trips the same form.
/// Trailing newline mirrors the conformance fixtures golden — keeps
/// `git diff` from flagging "no newline at end of file" on every
/// regeneration.
pub fn write_envelope<W: std::io::Write>(
    sink: &mut W,
    parsed: &ParsedForge,
) -> std::io::Result<()> {
    let env = ForgeAstEnvelope::new(parsed);
    serde_json::to_writer_pretty(&mut *sink, &env)
        .map_err(std::io::Error::other)?;
    sink.write_all(b"\n")?;
    Ok(())
}

/// Serialise an envelope to a file path. Wraps `write_envelope` with
/// `std::fs::File::create`; intended for direct CLI use. Atomically
/// truncates the destination per `File::create` semantics.
pub fn write_envelope_to_path(path: &std::path::Path, parsed: &ParsedForge) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    write_envelope(&mut f, parsed)
}

/// Wrap a parsed-and-analyzed `SCXMLModel` in the `ParsedForge`
/// envelope shape so the v1 AST export pipeline (`write_envelope` /
/// `write_envelope_to_path`) handles it identically to a forge
/// document. W3C SCXML carries no `<sce:import>` cross-doc references
/// and no `<sce:extern>` declarations of its own (those live on
/// forge kinds), so `imports` and `externs` are always empty for
/// statechart documents — the wire schema's `oneOf` discriminator on
/// `ast.document.kind` is the single signal that distinguishes the
/// statechart arm from the forge arms.
///
/// Takes the model by value (consuming) because `ForgeDocument::Statechart`
/// owns a `Box<SCXMLModel>`. Callers retaining the model for downstream
/// codegen must clone first — typically a one-off cost on the
/// `--emit-ast` path that is not in the hot loop.
pub fn statechart_parsed_forge(model: crate::model::SCXMLModel) -> ParsedForge {
    ParsedForge {
        document: crate::forge::model::ForgeDocument::Statechart(Box::new(model)),
        imports: Vec::new(),
        externs: Vec::new(),
    }
}

#[cfg(test)]
mod schema_drift {
    //! Drift guard between the Rust type tree under `sce-build/src/forge/`
    //! and the checked-in `apis/forge-ast.v1.schema.json`. Co-located with
    //! the producer (not under `tests/`) so it can read the test-gated
    //! `ForgeAstEnvelopeOwned` type without needing a feature flag.
    //!
    //! Bootstrap / refresh: run
    //!
    //!     UPDATE_EXPECT=1 cargo test -p sce-build schema_drift
    //!
    //! and inspect the regenerated file. Commit alongside whatever
    //! type change forced the update — the contract is *not* that the
    //! schema is frozen, it is that the schema matches the Rust IR at
    //! every commit. Long-term external consumers depend on this
    //! invariant for v1 stability; without it the schema would drift
    //! silently whenever a Forge IR field is added or renamed.

    use super::ForgeAstEnvelopeOwned;
    use std::path::PathBuf;

    fn schema_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("sce-build's parent is the repo root")
            .join("apis/forge-ast.v1.schema.json")
    }

    /// Stable `$id` for the v1 schema. Consumers MAY use this in
    /// `$ref` URLs when composing other schemas; mirrors the
    /// diagnostic schema precedent.
    const SCHEMA_ID: &str =
        "https://github.com/newmassrael/scxml-core-engine/blob/main/apis/forge-ast.v1.schema.json";

    fn render_generated() -> String {
        let schema = schemars::schema_for!(ForgeAstEnvelopeOwned);
        let mut json =
            serde_json::to_value(&schema).expect("schemars output serialises as serde_json::Value");

        // Tighten the `v` property: schemars renders a u32 field as
        // `{type: integer, format: uint32, minimum: 0}` — a wide
        // bound. The wire contract is *exactly* the const
        // `FORGE_AST_WIRE_VERSION`; a consumer's validator must
        // reject any other integer outright. Inject the `const`
        // constraint here so the schema actually enforces it.
        if let Some(v_prop) = json
            .get_mut("properties")
            .and_then(|p| p.get_mut("v"))
            .and_then(|v| v.as_object_mut())
        {
            v_prop.insert(
                "const".into(),
                serde_json::json!(super::FORGE_AST_WIRE_VERSION),
            );
            // Drop the redundant `minimum` / `format` now that the
            // const supersedes them — leaving them in would clutter
            // the contract without adding constraint.
            v_prop.remove("minimum");
            v_prop.remove("format");
        }

        // Inject SCE-specific headers schemars doesn't emit on its
        // own. `$id` lets external composers reference this schema
        // from elsewhere; `x-sce-schema-status` is the schema-level
        // lifecycle marker (NOT a wire field — see
        // `FORGE_AST_SCHEMA_STATUS` doc on the producer side).
        if let Some(obj) = json.as_object_mut() {
            // Inserting into a serde_json::Map preserves the existing
            // ordering of subsequent keys. Place these near the top so
            // human reviewers see them first.
            let mut reordered = serde_json::Map::with_capacity(obj.len() + 2);
            reordered.insert(
                "$schema".into(),
                obj.remove("$schema").unwrap_or_else(|| {
                    serde_json::json!("http://json-schema.org/draft-07/schema#")
                }),
            );
            reordered.insert("$id".into(), serde_json::json!(SCHEMA_ID));
            reordered.insert(
                "x-sce-schema-status".into(),
                serde_json::json!(super::FORGE_AST_SCHEMA_STATUS),
            );
            // Drain remaining keys preserving schemars's order.
            for (k, v) in obj.iter() {
                reordered.insert(k.clone(), v.clone());
            }
            *obj = reordered;
        }

        // Pretty-print so the checked-in file stays diff-able when a
        // consumer reviews a wire-format change.
        let mut out = serde_json::to_string_pretty(&json).expect("schema value serialises as JSON");
        // Trailing newline so POSIX-aware editors don't flag "no
        // newline at end of file" on every regeneration.
        out.push('\n');
        out
    }

    #[test]
    fn checked_in_schema_matches_generated() {
        let generated = render_generated();
        let path = schema_path();

        if std::env::var_os("UPDATE_EXPECT").is_some() {
            std::fs::write(&path, &generated)
                .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
            return;
        }

        let checked_in = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        if checked_in != generated {
            // Best-effort hint: surface the first divergent line range
            // so reviewers can see at-a-glance what changed without
            // having to diff externally.
            let max_show = 40usize;
            let mut first_diff: Option<usize> = None;
            for (i, (a, b)) in checked_in.lines().zip(generated.lines()).enumerate() {
                if a != b {
                    first_diff = Some(i);
                    break;
                }
            }
            let context = first_diff.map_or_else(|| {
                    format!(
                        "\nLength differs: checked-in {} bytes, generated {} bytes.",
                        checked_in.len(),
                        generated.len()
                    )
                }, |i| {
                    let lo = i.saturating_sub(2);
                    let hi = (i + 3).min(generated.lines().count());
                    let lines: Vec<&str> = generated.lines().collect();
                    let snippet: Vec<String> = (lo..hi)
                        .map(|j| {
                            let marker = if j == i { ">>" } else { "  " };
                            format!("  {} L{:4}: {}", marker, j + 1, lines[j])
                        })
                        .collect();
                    format!(
                        "\nFirst divergence around line {}:\n{}",
                        i + 1,
                        snippet.join("\n")
                    )
                });
            panic!(
                "\napis/forge-ast.v1.schema.json is out of sync with the Rust \
                 IR types in sce-build/src/forge/. Run:\n  \
                 UPDATE_EXPECT=1 cargo test -p sce-build schema_drift\n\
                 and commit the regenerated file alongside the type change \
                 that triggered the drift.\n\
                 {}\n\
                 (Up to {} lines of context shown.)",
                context
                    .lines()
                    .take(max_show)
                    .collect::<Vec<_>>()
                    .join("\n"),
                max_show
            );
        }
    }
}
