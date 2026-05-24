// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Forge AST export drift guard.
//
// The wire contract for `apis/forge-ast.v1.schema.json` is single-sourced
// in three places: (1) the Rust `ParsedForge` type tree in
// `sce-build/src/forge/model.rs`, (2) the `ForgeAstEnvelope` wrapper in
// `sce-build/src/forge/ast_export.rs`, and (3) the JSON Schema file at
// `apis/forge-ast.v1.schema.json`. This test enforces the invariants that
// keep the three in sync:
//
//   - kind enum sync:        `ForgeKind::ALL_ATTR_NAMES`
//                            == schema's `definitions.ForgeDocument.properties.kind.enum`
//                            == schema's `definitions.ForgeDocument.oneOf[*].properties.kind.const`
//   - envelope shape frozen: schema's top-level `required` is exactly
//                            {v, schema_status, ast} and `additionalProperties: false`
//   - emitted envelope:      Every fixture under `tests/forge/resources/`
//                            emits an envelope whose top-level keys are a
//                            subset of the schema's, and whose `ast.document.kind`
//                            sits in the closed enum.
//
// The point is to fail loudly the next time a developer renames a
// `ForgeKind` variant, drops the `schema_status` field, or adds a new
// kind without updating the schema — not to perform exhaustive JSON
// Schema validation (which would pull in `jsonschema` for marginal
// coverage given how few constraints the schema actually carries
// beyond `additionalProperties: true` at inner-shape level).

use sce_build::forge::ast_export::{
    write_envelope, ForgeAstEnvelope, FORGE_AST_SCHEMA_STATUS, FORGE_AST_WIRE_VERSION,
    SCE_PRODUCER_VERSION,
};
// FORGE_AST_SCHEMA_STATUS is intentionally *not* a wire field — it is
// the in-process mirror of the schema file's `x-sce-schema-status`
// header. The `envelope_constants_match_schema_header` test asserts
// the two stay in lockstep when the const flips pre-release → stable.
use sce_build::forge::model::ForgeKind;
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::DocumentLabel;

use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

fn load_schema() -> Value {
    let path = repo_root().join("apis/forge-ast.v1.schema.json");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn fixture(name: &str) -> PathBuf {
    repo_root().join("tests/forge/resources").join(name)
}

/// Parse a fixture into the AST-export `ParsedForge` envelope shape.
/// Routes statechart fixtures through the SCXML parser+analyzer (the
/// production AST-export path used by `cmd_generate` / `emit_orchestrate_
/// asts`) and every other fixture through the forge `parse_forge_with_
/// imports` path. The two arms converge on `ParsedForge` via
/// `ast_export::statechart_parsed_forge` so downstream test code (the
/// schema-validation loop, the round-trip checks) does not branch
/// on kind.
fn parse_fixture(path: &PathBuf) -> sce_build::forge::model::ParsedForge {
    let content =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("fixture has a UTF-8 stem");
    let label = DocumentLabel {
        identifier: stem,
        diagnostic_label: stem,
    };
    match parse_forge_with_imports(&content, label) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => {
            // Statechart: run the SCXML pipeline's parse + analyze
            // and wrap the analyzed model in the v1 envelope.
            let mut parser = sce_build::parser::SCXMLParser::new();
            let path_str = path.to_str().expect("UTF-8 fixture path");
            let mut model = parser
                .parse_file(path_str)
                .unwrap_or_else(|e| panic!("scxml parse {}: {e:?}", path.display()));
            sce_build::analyzer::analyze(&mut model, path_str);
            sce_build::forge::ast_export::statechart_parsed_forge(model)
        }
        Err(e) => panic!("parse {}: {e:?}", path.display()),
    }
}

#[test]
fn envelope_constants_match_schema_header() {
    let schema = load_schema();
    assert_eq!(
        schema["x-sce-schema-status"].as_str(),
        Some(FORGE_AST_SCHEMA_STATUS),
        "schema header `x-sce-schema-status` must match \
         `ast_export::FORGE_AST_SCHEMA_STATUS` — pre-release/stable \
         flips must update both in one commit"
    );
    assert_eq!(
        schema["properties"]["v"]["const"].as_u64(),
        Some(FORGE_AST_WIRE_VERSION as u64),
        "schema's `v` const must match `FORGE_AST_WIRE_VERSION`"
    );
}

#[test]
fn envelope_top_level_required_fields_frozen() {
    let schema = load_schema();
    let required: BTreeSet<&str> = schema["required"]
        .as_array()
        .expect("top-level required is an array")
        .iter()
        .map(|v| v.as_str().expect("required entries are strings"))
        .collect();
    let expected: BTreeSet<&str> = ["v", "ast"].into_iter().collect();
    assert_eq!(
        required, expected,
        "envelope's top-level required keys must remain exactly {{v, ast}} — \
         schema lifecycle status (pre-release / stable) is carried by the \
         schema file's `x-sce-schema-status` header, NOT a wire field, \
         matching the diagnostic schema precedent. Adding a required \
         field is a breaking change and requires v2."
    );
    assert_eq!(
        schema["additionalProperties"].as_bool(),
        Some(false),
        "envelope's `additionalProperties` must remain false so consumers \
         can confidently reject unknown top-level keys"
    );
}

/// Schema's `ast.document.kind` enum lists every kind that appears in
/// `enum ForgeDocument`. Every variant of `ForgeKind::ALL_ATTR_NAMES`
/// carries a matching `ForgeDocument` arm — including `Statechart`,
/// which the AST-export v1 second atomic added so the envelope covers
/// SCE's full IR surface (15 forge kinds + 1 statechart). No filter is
/// applied: the test fails loudly if a future kind addition lands in
/// `ForgeKind` without a matching `ForgeDocument` variant.
fn emittable_kind_attr_names() -> Vec<&'static str> {
    ForgeKind::ALL_ATTR_NAMES.to_vec()
}

#[test]
fn schema_kind_discriminator_matches_forge_kind() {
    // Schemars renders the `#[serde(tag = "kind")]` enum as a `oneOf`
    // where each arm pins its `kind` field to a single-element
    // `enum: ["<name>"]` (not `const`, per schemars 0.8 convention).
    // The drift guard collects those values and asserts the *set*
    // matches `ForgeKind::ALL_ATTR_NAMES` minus the statechart
    // sentinel.
    //
    // Set comparison (not ordered): `ForgeKind` and `ForgeDocument`
    // are two distinct enums declared in two separate sites
    // (`ALL_ATTR_NAMES` impl block vs `enum ForgeDocument`). Their
    // declaration orders happen to differ today (e.g.
    // `ForgeKind` lists `procedure` before `validator`; `ForgeDocument`
    // lists `validator` before `procedure`). The order isn't part of
    // the wire contract — only kind *membership* is — so the test
    // asserts membership equality and leaves ordering to the producer.
    //
    // This is the *externally observable* drift check — a consumer's
    // JSON Schema validator reads the discriminator the same way.
    // The schemars-level field-by-field drift (richer but more
    // schemars-internal) is asserted by the lib's own
    // `forge::ast_export::schema_drift::checked_in_schema_matches_generated`
    // unit test.
    let schema = load_schema();
    let one_of = schema["definitions"]["ForgeDocument"]["oneOf"]
        .as_array()
        .expect("ForgeDocument has a `oneOf` array");
    let schema_kinds: BTreeSet<String> = one_of
        .iter()
        .map(|arm| {
            arm["properties"]["kind"]["enum"][0]
                .as_str()
                .expect("each oneOf arm pins `kind` to a single-element enum")
                .to_string()
        })
        .collect();
    let expected: BTreeSet<String> = emittable_kind_attr_names()
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert_eq!(
        schema_kinds, expected,
        "schema's per-arm `kind` discriminator set must equal \
         `ForgeKind::ALL_ATTR_NAMES` minus the `statechart` sentinel. \
         Adding a new kind:\n  \
         1. add the variant to `enum ForgeKind` (model.rs)\n  \
         2. append to `ALL_ATTR_NAMES`\n  \
         3. add a `ForgeDocument` variant — the schema regenerates from there\n  \
         4. run `UPDATE_EXPECT=1 cargo test -p sce-build schema_drift` \
         and commit the regenerated schema"
    );
}

#[test]
fn round_trip_transform_fixture() {
    let parsed = parse_fixture(&fixture("transform_multi_output.scxml"));
    let envelope = ForgeAstEnvelope::new(&parsed);
    let mut buf = Vec::new();
    write_envelope(&mut buf, &parsed).expect("write_envelope");
    let json: Value = serde_json::from_slice(&buf).expect("emitted JSON parses");

    assert_eq!(json["v"], Value::from(FORGE_AST_WIRE_VERSION));
    assert!(
        json.get("schema_status").is_none(),
        "schema_status MUST NOT appear on the wire — it lives in the \
         schema file header. Diagnostic schema precedent: header only."
    );
    assert_eq!(json["ast"]["document"]["kind"], Value::from("transform"));
    assert_eq!(
        json["ast"]["document"]["name"],
        Value::from("transform_multi_output")
    );
    // `inputs[0].id == "celsius"` — proves nested fields are reachable.
    assert_eq!(
        json["ast"]["document"]["inputs"][0]["id"],
        Value::from("celsius")
    );

    // Borrowed envelope must serialize identically to the writer form.
    let direct = serde_json::to_value(&envelope).expect("serialize envelope value");
    assert_eq!(direct, json, "envelope value form must match writer form");
}

#[test]
fn round_trip_lookup_fixture() {
    let parsed = parse_fixture(&fixture("lookup_gear_position.scxml"));
    let mut buf = Vec::new();
    write_envelope(&mut buf, &parsed).expect("write_envelope");
    let json: Value = serde_json::from_slice(&buf).expect("emitted JSON parses");

    assert_eq!(json["ast"]["document"]["kind"], Value::from("lookup"));
    assert_eq!(
        json["ast"]["document"]["name"],
        Value::from("lookup_gear_position")
    );
    // `miss_policy.kind` is an internally-tagged enum — proves the
    // tag=value form survives roundtrip.
    assert_eq!(
        json["ast"]["document"]["miss_policy"]["kind"],
        Value::from("default")
    );
    assert_eq!(
        json["ast"]["document"]["miss_policy"]["value"],
        Value::from("NEUTRAL")
    );
}

#[test]
fn emitted_envelope_kind_is_in_schema_discriminator_set() {
    // Cross-cut: parse every kind-bearing fixture available, emit the
    // envelope, and assert `ast.document.kind` is a member of the
    // schema's per-arm `kind` discriminator set. Catches a future
    // kind variant whose parser routes correctly but whose serialised
    // discriminator drifts (e.g. someone renames
    // `"bounded-collection"` to `"bounded_collection"` in serde
    // without touching the schema).
    let schema = load_schema();
    let schema_kinds: BTreeSet<String> = schema["definitions"]["ForgeDocument"]["oneOf"]
        .as_array()
        .expect("ForgeDocument has a `oneOf` array")
        .iter()
        .map(|arm| {
            arm["properties"]["kind"]["enum"][0]
                .as_str()
                .expect("each oneOf arm pins `kind` to a single-element enum")
                .to_string()
        })
        .collect();

    for (name, expected_kind) in [
        ("transform_multi_output.scxml", "transform"),
        ("lookup_gear_position.scxml", "lookup"),
    ] {
        let parsed = parse_fixture(&fixture(name));
        let mut buf = Vec::new();
        write_envelope(&mut buf, &parsed).expect("write_envelope");
        let json: Value = serde_json::from_slice(&buf).expect("parse emitted");
        let emitted_kind = json["ast"]["document"]["kind"]
            .as_str()
            .expect("kind is a string")
            .to_string();
        assert!(
            schema_kinds.contains(&emitted_kind),
            "{name}: emitted kind `{emitted_kind}` not in schema set {schema_kinds:?}"
        );
        assert_eq!(emitted_kind, expected_kind, "{name}: kind mismatch");
    }
}

/// Per-kind round-trip table. One fixture per emittable kind — the
/// drift guard fails loudly if any kind in `ForgeKind::ALL_ATTR_NAMES`
/// (minus the statechart sentinel) lacks coverage here.
///
/// Adding a new kind requires extending this table; the
/// `every_emittable_kind_has_a_fixture` test below asserts the
/// invariant.
///
/// All fixtures are *standalone* — no `<sce:import>` declarations, so
/// the AST emit path runs without needing sibling files on disk.
/// Cross-doc resolution semantics are exercised by other tests; this
/// table is solely for round-tripping the parsed IR through the wire
/// envelope.
const FIXTURE_PER_KIND: &[(&str, &str)] = &[
    ("statechart", "statechart_ast_export_min.scxml"),
    ("transform", "transform_multi_output.scxml"),
    ("lookup", "lookup_gear_position.scxml"),
    ("condition", "condition_range.scxml"),
    ("codec", "codec_simple_frame.scxml"),
    ("procedure", "procedure_linear.scxml"),
    ("validator", "validator_plausibility_only.scxml"),
    ("filter", "filter_debounce.scxml"),
    ("interpolation", "interpolation_1d_linear.scxml"),
    ("timer", "timer_diag_scheduler.scxml"),
    ("observer", "observer_coolant.scxml"),
    ("algorithm", "algorithm_crc16.scxml"),
    ("link", "link_ast_export_min.scxml"),
    ("buffer-pool", "buffer_pool_ast_export_min.scxml"),
    ("worker", "worker_ast_export_min.scxml"),
    ("bounded-collection", "local_sub_table.scxml"),
    // NL→IR Item C1 Path A Atomic 2: Enum kind round-trip coverage.
    // Minimal-shape fixture (2 variants, uint8 underlying) chosen to
    // mirror `*_ast_export_min` siblings — the round-trip envelope is
    // what's under test, not enum-feature richness.
    ("enum", "enum_ast_export_min.scxml"),
    // NL→IR Item C1 Path A Atomic 3: EventSchema kind round-trip
    // coverage. Minimal-shape fixture (one uint8 field, no enum
    // import) chosen to mirror `*_ast_export_min` siblings — the
    // round-trip envelope is under test, not EventSchema-feature
    // richness. The `ast.export.min` event name is generic and
    // non-reserved so the parse-time built-in-event guard accepts.
    ("event-schema", "event_schema_ast_export_min.scxml"),
];

#[test]
fn every_emittable_kind_has_a_fixture() {
    let covered: BTreeSet<&str> = FIXTURE_PER_KIND.iter().map(|(k, _)| *k).collect();
    let expected: BTreeSet<&str> = emittable_kind_attr_names().into_iter().collect();
    assert_eq!(
        covered, expected,
        "FIXTURE_PER_KIND must cover every emittable kind exactly once. \
         Missing kinds break the round-trip drift guard; extra kinds \
         signal that a kind was removed from `ForgeKind` without \
         updating this table."
    );
}

#[test]
fn round_trip_every_kind() {
    // Compile the schema once outside the loop — `jsonschema` builds
    // a validation tree from the source `Value` and we share it
    // across every fixture iteration.
    let schema_value = load_schema();
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_value)
        .expect("checked-in schema must be valid JSON Schema");

    for (expected_kind, fixture_name) in FIXTURE_PER_KIND {
        let path = fixture(fixture_name);
        assert!(
            path.exists(),
            "fixture {fixture_name} missing for kind {expected_kind} at {}",
            path.display()
        );
        let parsed = parse_fixture(&path);

        // Round-trip: parse → serialize → re-parse JSON → check kind.
        let mut buf = Vec::new();
        write_envelope(&mut buf, &parsed).expect("write_envelope succeeds");
        let json: Value = serde_json::from_slice(&buf).expect("emitted envelope re-parses as JSON");

        // Real JSON Schema validation — the strongest sanity check.
        // A consumer running their own validator against our output
        // gets the same verdict, so any failure here is an immediate
        // wire-format regression rather than a latent consumer break.
        if let Err(errors) = validator.validate(&json) {
            let collected: Vec<String> = errors
                .map(|e| format!("  - {} at {}", e, e.instance_path))
                .collect();
            panic!(
                "fixture {fixture_name}: emitted envelope fails JSON Schema validation:\n{}",
                collected.join("\n")
            );
        }

        assert_eq!(
            json["v"],
            Value::from(FORGE_AST_WIRE_VERSION),
            "fixture {fixture_name}: envelope `v` field"
        );
        let actual_kind = json["ast"]["document"]["kind"]
            .as_str()
            .unwrap_or_else(|| panic!("fixture {fixture_name}: missing kind discriminator"));
        assert_eq!(
            actual_kind, *expected_kind,
            "fixture {fixture_name}: kind discriminator mismatch"
        );

        // The `name` field is derived from the file stem by
        // `parse_fixture` via `DocumentLabel.identifier`; assert it
        // round-trips so a regression in `DocumentLabel` plumbing
        // (or a `name` field rename on a kind model) surfaces here.
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("UTF-8 stem");
        assert_eq!(
            json["ast"]["document"]["name"],
            Value::from(stem),
            "fixture {fixture_name}: `name` field must round-trip from DocumentLabel.identifier"
        );
    }
}

#[test]
fn orchestrate_emit_ast_dir_writes_one_envelope_per_forge_doc() {
    // CLI contract: `orchestrate --emit-ast-dir <dir>` emits one
    // `<stem>.ast.json` per `--forge` input that classifies as a
    // forge document. Statechart inputs / `--scxml` paths silent
    // skip. The emitted files validate against the same v1 schema
    // single-doc `generate --emit-ast` uses.
    let bin = std::path::PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"));
    let scratch_root = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("orch_emit_ast-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch_root);
    let codegen_out = scratch_root.join("codegen");
    let ast_dir = scratch_root.join("ast");
    std::fs::create_dir_all(&codegen_out).expect("create codegen out");

    let transform = fixture("transform_multi_output.scxml");
    let lookup = fixture("lookup_gear_position.scxml");

    let status = std::process::Command::new(&bin)
        .arg("orchestrate")
        .arg("--forge")
        .arg(&transform)
        .arg("--forge")
        .arg(&lookup)
        .arg("--language")
        .arg("rust")
        .arg("--output-dir")
        .arg(&codegen_out)
        .arg("--emit-ast-dir")
        .arg(&ast_dir)
        .status()
        .expect("spawn sce-codegen orchestrate");
    assert!(
        status.success(),
        "orchestrate --emit-ast-dir failed with status {status:?}"
    );

    // Both forge docs emit one AST file each.
    let transform_ast = ast_dir.join("transform_multi_output.ast.json");
    let lookup_ast = ast_dir.join("lookup_gear_position.ast.json");
    assert!(
        transform_ast.exists(),
        "transform AST missing at {}",
        transform_ast.display()
    );
    assert!(
        lookup_ast.exists(),
        "lookup AST missing at {}",
        lookup_ast.display()
    );

    // Validate the emitted envelopes against the schema, matching
    // the round_trip_every_kind test's contract for the single-doc
    // path. Reuses the same validator builder.
    let schema_value = load_schema();
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_value)
        .expect("schema must compile");

    for ast_file in [&transform_ast, &lookup_ast] {
        let content = std::fs::read_to_string(ast_file).expect("read AST file");
        let json: Value = serde_json::from_str(&content).expect("parse emitted AST");
        let validation = validator.validate(&json);
        let collected: Vec<String> = match validation {
            Ok(()) => Vec::new(),
            Err(errors) => errors
                .map(|e| format!("  - {} at {}", e, e.instance_path))
                .collect(),
        };
        assert!(
            collected.is_empty(),
            "orchestrate AST {} failed schema validation:\n{}",
            ast_file.display(),
            collected.join("\n")
        );
    }

    // Cleanup so repeated runs do not accumulate.
    let _ = std::fs::remove_dir_all(&scratch_root);
}

#[test]
fn schema_self_validates() {
    // Sanity: the checked-in schema is itself a valid Draft-07
    // JSON Schema. Catches a malformed schemars output before any
    // fixture validation runs.
    let schema_value = load_schema();
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_value)
        .expect("checked-in schema must compile as a valid JSON Schema");
}

#[test]
fn producer_stamps_sce_version_on_default_path() {
    // Production `--emit-ast` path stamps the current SCE release so
    // downstream issue reports can pin the exact producer. The
    // version string is non-empty and matches the Cargo package
    // version. `new_unversioned()` (test-only) suppresses the stamp.
    let parsed = parse_fixture(&fixture("transform_multi_output.scxml"));
    let env_default = ForgeAstEnvelope::new(&parsed);
    assert_eq!(
        env_default.sce_producer_version,
        Some(SCE_PRODUCER_VERSION),
        "production constructor must stamp the producer version"
    );
    assert!(
        !SCE_PRODUCER_VERSION.is_empty(),
        "producer version must be non-empty"
    );

    let env_unversioned = ForgeAstEnvelope::new_unversioned(&parsed);
    assert_eq!(
        env_unversioned.sce_producer_version, None,
        "test-only constructor must suppress the producer version"
    );

    // Serialised form mirrors the in-process struct: present-when-some,
    // absent-when-none. Drops the field key entirely on `None`, not
    // emitting `"sce_producer_version": null` — consumers MAY check
    // either form per the consumer compatibility checklist.
    let json_default = serde_json::to_value(&env_default).expect("serialise");
    assert!(
        json_default.get("sce_producer_version").is_some(),
        "production wire form must carry the producer version"
    );
    let json_unversioned = serde_json::to_value(&env_unversioned).expect("serialise");
    assert!(
        json_unversioned.get("sce_producer_version").is_none(),
        "unversioned wire form must omit the producer version key entirely"
    );
}

#[test]
fn envelope_trailing_newline_for_git_diff() {
    let parsed = parse_fixture(&fixture("transform_multi_output.scxml"));
    let mut buf = Vec::new();
    write_envelope(&mut buf, &parsed).expect("write_envelope");
    assert_eq!(
        buf.last().copied(),
        Some(b'\n'),
        "envelope must end with a trailing newline so `git diff` does \
         not flag 'no newline at end of file' on every emit"
    );
}
