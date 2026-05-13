// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// End-to-end contract test for `sce-codegen --error-format=json`.
//
// These tests launch the real CLI binary (the one cargo builds for
// integration tests and exposes via `CARGO_BIN_EXE_*`) on a forge
// document crafted to fail validation. The assertions pin the wire
// contract consumed by upstream agents:
//
//   * stderr is NDJSON (one JSON object per line)
//   * each line carries a stable `code`, `stage`, and `id`
//   * the process exit code matches `ForgeError::exit_code()`
//   * stdout is not polluted by the diagnostic
//
// Human mode is covered by a negative assertion: its stderr must NOT
// start with `{`, so anything grepping for JSON in human output breaks
// loudly instead of silently mis-parsing.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// Resolve the integration binary cargo built for this crate.
///
/// `CARGO_BIN_EXE_<bin>` is populated automatically for integration
/// tests so we can launch the CLI without hardcoding a path. This
/// requires the `cli` feature to be enabled when tests run — enforced
/// by the `required-features = ["cli"]` manifest entry on this test.
fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// Per-process counter so parallel tests never share a scratch dir.
static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

/// Scoped scratch directory under `target/`. Dropping removes the
/// directory and any fixtures it held.
struct ScratchDir(PathBuf);
impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let dir = root.join(format!("{label}-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Write a lookup-kind forge document with no `<datamodel>` child.
/// This reliably enters the forge pipeline (`sce:kind="lookup"` is an
/// XSD-accepted value) and fails semantic validation with
/// `ValidationError::MissingElement`, whose mapping (`code =
/// "validation/missing-element"`, `stage = "validation"`) is the
/// invariant these tests pin.
fn write_missing_datamodel_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("error-format");
    let path = dir.path().join("bad.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="bad_lookup">
</scxml>
"#;
    std::fs::write(&path, body).expect("write fixture");
    (dir, path)
}

fn run_generate(bin: &PathBuf, scxml: &PathBuf, error_format: &str) -> std::process::Output {
    Command::new(bin)
        .args([
            "--error-format",
            error_format,
            "generate",
            scxml.to_str().unwrap(),
            "--language",
            "rust",
            "--output-dir",
            scxml.parent().unwrap().to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen")
}

#[test]
fn json_mode_emits_single_ndjson_record_on_stderr() {
    let (_dir, scxml) = write_missing_datamodel_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");

    assert!(!out.status.success(), "process must fail on validation error");
    // ValidationError maps to stage=3 per ForgeError::exit_code().
    assert_eq!(out.status.code(), Some(3), "exit code must equal ForgeError::exit_code()");

    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let trimmed = stderr.trim_end();
    // NDJSON: exactly one line here (single error), each line a JSON object.
    assert!(
        !trimmed.is_empty(),
        "json mode must emit at least one diagnostic line"
    );
    for line in trimmed.lines() {
        assert!(line.starts_with('{'), "line must start with '{{': {line}");
        assert!(line.ends_with('}'), "line must end with '}}': {line}");
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line is not valid JSON ({e}): {line}"));
        let obj = parsed.as_object().expect("root must be an object");
        // Required fields per the diagnostic contract.
        for key in ["v", "id", "code", "stage", "message"] {
            assert!(obj.contains_key(key), "missing required field '{key}' in: {line}");
        }
        assert_eq!(
            obj["v"].as_u64(),
            Some(1),
            "schema version pinned at 1 for this release: {line}"
        );
        assert_eq!(
            obj["code"], "validation/missing-element",
            "unexpected code: {line}"
        );
        assert_eq!(obj["stage"], "validation", "unexpected stage: {line}");
        assert!(
            obj["id"].as_str().unwrap_or("").starts_with("fnv1a:"),
            "id must be content-hashed: {line}"
        );
    }
}

#[test]
fn json_mode_does_not_pollute_stdout_with_diagnostics() {
    let (_dir, scxml) = write_missing_datamodel_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    let stdout = String::from_utf8(out.stdout).expect("stdout utf8");
    // stdout is for artifact manifests / progress text; error payloads
    // must ride on stderr so agents can split streams by fd without
    // parsing.
    assert!(
        !stdout.contains("validation/missing-element"),
        "diagnostic code leaked to stdout: {stdout}"
    );
    assert!(
        !stdout.contains("\"code\""),
        "JSON diagnostic shape leaked to stdout: {stdout}"
    );
}

/// CLI-boundary failure (unknown --language) must ride the same wire
/// contract as forge / mesh errors. Pins the "flag is universal"
/// guarantee — if a new subcommand path forgets to route through
/// `cli_exit`, this test fails.
#[test]
fn json_mode_covers_cli_boundary_errors() {
    // Any path works — the language check fires before I/O. Using a
    // bogus path explicitly avoids accidentally invoking the forge
    // pipeline on a real fixture.
    let out = Command::new(sce_codegen_bin())
        .args([
            "--error-format",
            "json",
            "generate",
            "/does-not-exist.scxml",
            "--language",
            "wasm", // deliberately unsupported
            "--output-dir",
            "/tmp",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let line = stderr.trim_end();
    assert!(line.starts_with('{') && line.ends_with('}'), "not NDJSON: {line}");
    let parsed: serde_json::Value =
        serde_json::from_str(line).expect("CLI error line must be JSON");
    assert_eq!(parsed["code"], "cli/unknown-language");
    assert_eq!(parsed["stage"], "cli");
    assert_eq!(parsed["v"].as_u64(), Some(1));
    // Candidate list rides `fix` — repair signals live in one place
    // under the non-overlap rule (contract §3.2). `expected` must
    // stay absent, or upstream agents would face two sources of
    // truth for the same data.
    assert!(
        parsed.get("expected").is_none(),
        "expected must not duplicate fix.candidates: {line}"
    );
    assert_eq!(parsed["fix"]["kind"], "replace_one_of");
    let candidates = parsed["fix"]["candidates"]
        .as_array()
        .expect("fix.candidates must be an array");
    assert!(
        candidates.iter().any(|v| v == "rust"),
        "fix.candidates must include 'rust': {line}"
    );
}

/// `<scxml initial="X">` where `X` is not declared. Pre-W5 this
/// reject was mis-classified as `validation/dynamic-features`
/// (the analyzer treated all three precondition failures as
/// "dynamic features"). RFC §W5 D3 splits the precondition channel:
/// "initial names undeclared state" is a hard semantic violation
/// (the Interpreter would also reject), and now correctly emits
/// `validation/invalid-reference` via
/// `ScxmlSemanticError::InitialStateUnknown`. The fold reuses an
/// existing forge wire code (W4 D4 fold precedent) — concept
/// identity: "name X did not resolve to declared symbol Y".
#[test]
fn json_mode_undeclared_initial_routes_through_invalid_reference() {
    let dir = ScratchDir::new("undeclared-initial");
    let path = dir.path().join("dyn.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0"
       initial="nope"
       name="bad_dyn">
    <state id="s1"/>
</scxml>
"#;
    std::fs::write(&path, body).expect("write fixture");
    let out = run_generate(&sce_codegen_bin(), &path, "json");
    assert!(!out.status.success(), "undeclared initial must fail");
    assert_eq!(
        out.status.code(),
        Some(3),
        "validation stage must exit 3 (W5 keeps validation exit code for SCXML semantic), not cli/20",
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let line = stderr.trim_end().lines().next().expect("at least one ndjson line");
    let parsed: serde_json::Value =
        serde_json::from_str(line).expect("invalid-reference line must be JSON");
    assert_eq!(parsed["code"], "validation/invalid-reference");
    assert_eq!(parsed["stage"], "validation");
    assert_eq!(parsed["actual"], "nope");
    // The `fix.candidates` list should include the available state ids
    // so a repair tool can propose `nope → s1`.
    let fix = &parsed["fix"];
    assert_eq!(fix["kind"], "replace_one_of");
    let candidates = fix["candidates"]
        .as_array()
        .expect("fix.candidates must be an array");
    assert!(
        candidates.iter().any(|v| v == "s1"),
        "fix.candidates must include 's1' (the only declared state): {line}"
    );
}

// "No initial attribute" classification (genuine DynamicFeatures —
// runtime default resolution required) cannot be reached at the
// integration boundary because `parser.rs` auto-defaults
// `model.initial` to the first child state's id (W3C SCXML §3.3
// default applied at parse time). The classification is pinned at
// the analyzer-unit level instead — see
// `analyzer::tests::no_initial_attribute_keeps_dynamic_features`.
// W5 D3 invariant: the genuine DynamicFeatures path stays the
// fallback for codegen limitations, not for semantic violations.

/// Write a document with an `sce:kind` value outside the XSD enum.
///
/// Pre-fix, `is_forge_document()` returned false for unknown kinds
/// (the detector collapsed `Err(UnsupportedKind)` and "absent" into the
/// same boolean), so the document was misrouted through the SCXML
/// parser and reported with `stage="cli"` / `code="cli/scxml-parse"`.
/// The contract promises `stage` is the repair-routing key, so the bug
/// caused agents to branch on the wrong arm.
///
/// The fixture locks in the post-fix routing: any `sce:kind` attribute
/// — known or unknown — dispatches through the forge pipeline, so the
/// bundled XSD emits `xml/schema-validation` with the enum of legal
/// values included in the message.
fn write_unknown_kind_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("error-format");
    let path = dir.path().join("bogus_kind.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bogus" name="bad_kind">
</scxml>
"#;
    std::fs::write(&path, body).expect("write fixture");
    (dir, path)
}

#[test]
fn json_mode_routes_unknown_sce_kind_through_forge_pipeline() {
    let (_dir, scxml) = write_unknown_kind_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");

    assert!(!out.status.success(), "process must fail on unknown sce:kind");
    // `XmlError::SchemaValidation` → `ForgeError::Xml` → exit code 2.
    // If this becomes 1 (clap / ScxmlParse) again, the routing bug has
    // regressed: the document is re-flowing through the SCXML parser.
    assert_eq!(
        out.status.code(),
        Some(2),
        "unknown kind must produce ForgeError::Xml (exit 2), not cli/scxml-parse (exit 1)"
    );

    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let line = stderr.trim_end();
    assert!(
        line.starts_with('{') && line.ends_with('}'),
        "stderr must be a single NDJSON record: {line}"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(line).expect("stderr line must parse as JSON");

    assert_eq!(
        parsed["code"], "xml/schema-validation",
        "unknown kind must surface the XSD violation, not cli/scxml-parse: {line}"
    );
    assert_eq!(
        parsed["stage"], "xml",
        "stage must reflect where the error was caught (XSD), not the CLI boundary: {line}"
    );
    assert_eq!(parsed["v"].as_u64(), Some(1));

    // Location pinning: XSD diagnostics MUST carry file + line from
    // libxml2. Agents route repairs by `stage + location`, so a
    // missing line here reduces them to prose-parsing the message.
    // CLI passes the full basename (with extension) so downstream
    // tooling can open the file without guessing the suffix.
    let location = &parsed["location"];
    assert!(
        location.is_object(),
        "XSD diagnostic must carry location object: {line}"
    );
    assert_eq!(
        location["file"].as_str(),
        scxml.file_name().and_then(|s| s.to_str()),
        "location.file must equal the fixture basename: {line}"
    );
    assert!(
        location["line"].as_u64().is_some_and(|l| l > 0),
        "location.line must be populated and > 0: {line}"
    );

    // The `message` field quotes the XSD enumeration so an agent can
    // repair without consulting SCE_ERROR_CONTRACT.md — pin this guarantee.
    let message = parsed["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("'bogus'"),
        "message must identify the offending value: {line}"
    );
    for legal_kind in ["statechart", "transform", "lookup"] {
        assert!(
            message.contains(&format!("'{legal_kind}'")),
            "message must enumerate legal kinds so agents can repair: missing '{legal_kind}' in {line}"
        );
    }
}

/// Write a codec fixture that violates the XSD in three places so
/// `validate()` returns three diagnostics in one call. Used to pin
/// the multi-record emission path.
///
///   * `sce:default-endian="sideways"` — not in enum [little|big]
///   * two fields with `sce:bit-size="bad*"` — not xs:unsignedInt
fn write_multi_violation_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("xsd-multi");
    let path = dir.path().join("multi_violation.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="sideways" name="x">
  <datamodel>
    <data id="a" sce:type="uint8" sce:byte="0" sce:bit-size="bad1"/>
    <data id="b" sce:type="uint8" sce:byte="1" sce:bit-size="bad2"/>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write multi-violation fixture");
    (dir, path)
}

#[test]
fn json_mode_emits_one_ndjson_record_per_xsd_violation() {
    let (_dir, scxml) = write_multi_violation_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");

    assert!(!out.status.success(), "process must fail on XSD violations");
    assert_eq!(out.status.code(), Some(2), "XmlError exit code");

    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let lines: Vec<&str> = stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    // Each violation gets its own NDJSON line — merging them would hide
    // the per-line data libxml2 already carries. We expect exactly 3
    // for this fixture (endian enum + two bit-size values).
    assert_eq!(
        lines.len(),
        3,
        "expected one NDJSON record per XSD violation, got {}: {stderr}",
        lines.len()
    );

    let mut seen_lines = Vec::new();
    for line in &lines {
        let parsed: serde_json::Value =
            serde_json::from_str(line).expect("each line must parse as JSON");
        assert_eq!(parsed["code"], "xml/schema-validation");
        assert_eq!(parsed["stage"], "xml");
        let location = &parsed["location"];
        assert!(location.is_object(), "location object on every record: {line}");
        assert_eq!(
            location["file"].as_str(),
            Some("multi_violation.scxml"),
            "file must be fixture basename: {line}"
        );
        let lineno = location["line"]
            .as_u64()
            .expect(&format!("line must be present: {line}"));
        assert!(lineno > 0, "line must be > 0: {line}");
        seen_lines.push(lineno);
    }

    // The three violations live on different source lines (scxml
    // element at top, two <data> children below). Identity per record
    // must include line, so `id`s must be distinct — prove it by
    // checking at least two different line numbers surface.
    let distinct: std::collections::HashSet<_> = seen_lines.iter().copied().collect();
    assert!(
        distinct.len() >= 2,
        "multi-violation diagnostics must span different lines, got {seen_lines:?}"
    );
}

/// Write a condition fixture that passes XSD but fails semantic
/// validation on a specific child element: the `<data direction="out">`
/// is missing the `expr` attribute the condition kind requires.
///
/// The root `<scxml>` is deliberately on line 2 and the offending
/// `<data id="y">` on line 6 so the assertion below can distinguish
/// root-level precision (every error would report line 2) from
/// leaf-level precision (the raise-site node's actual line).
fn write_condition_missing_expr_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("cond_missing_expr.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="condition" name="bad_cond">
  <datamodel>
    <data id="x" sce:type="int32" sce:direction="in"/>
    <data id="y" sce:type="bool" sce:direction="out"/>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write condition fixture");
    (dir, path)
}

/// Leaf-precision acceptance test for `parse_condition`.
///
/// Before per-leaf wiring this fixture reported `location.line` equal
/// to the `<scxml>` root line (2), so upstream agents could not tell
/// which child element violated the contract. The expected behaviour
/// is that the diagnostic points at the offending `<data>` element's
/// own line — proving `located(&data, ...)` fires at the raise-site
/// rather than the wrapper collapsing everything to root.
#[test]
fn json_mode_condition_missing_expr_reports_leaf_line() {
    let (_dir, scxml) = write_condition_missing_expr_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");

    assert!(!out.status.success(), "must fail on missing expr");
    assert_eq!(out.status.code(), Some(3), "ValidationError exit code");

    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let line = stderr.trim_end();
    let parsed: serde_json::Value =
        serde_json::from_str(line).expect("stderr must be NDJSON");

    assert_eq!(parsed["code"], "validation/missing-attribute");
    assert_eq!(parsed["stage"], "validation");

    let location = &parsed["location"];
    assert!(location.is_object(), "location object required: {line}");
    assert_eq!(
        location["file"].as_str(),
        scxml.file_name().and_then(|s| s.to_str())
    );
    // The offending <data id="y"> sits on fixture line 7 (root is 2).
    // If leaf precision regressed to wrapper behaviour, this becomes 2.
    assert_eq!(
        location["line"].as_u64(),
        Some(7),
        "must point at the specific <data> element, not the <scxml> root: {line}"
    );
}

/// Write a transform fixture that passes XSD but fails the post-loop
/// `outputs.is_empty()` check inside `parse_transform`. XSD accepts a
/// lone input `<data>` with no output fields, so validation must run
/// against the `<datamodel>` container — the most specific node still
/// in scope at that raise-site (forge/parser.rs:530-535).
///
/// **Why transform, not codec**: this fixture used to ride parse_codec's
/// `fields.is_empty()` check, but RFC §5.B B5-α deliberately accepts
/// zero-field codecs (Zenoh KeepAlive empty-body messages keyed by the
/// surrounding header byte) — parse_codec no longer raises
/// EmptyCollection on a fields-empty body. parse_transform still
/// container-anchors EmptyCollection on missing inputs/outputs (one of
/// the two raise paths still required by transform's two-direction
/// semantic), preserving the leaf-precision contract this test pins.
fn write_transform_no_outputs_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("transform_no_outputs.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="bad_transform">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write transform fixture");
    (dir, path)
}

/// Leaf-precision acceptance test for `parse_transform`.
///
/// The `<scxml>` root is on line 2 and `<datamodel>` on line 5.
/// Post-loop validation reports at `<datamodel>` — the most specific
/// node still in scope — so `location.line == 5`, not `2`. This
/// pins the "most specific node still in scope" rule for the
/// template across container-level raises (not just per-`<data>`
/// raises exercised by the condition test above).
#[test]
fn json_mode_transform_no_outputs_reports_datamodel_line() {
    let (_dir, scxml) = write_transform_no_outputs_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");

    assert!(!out.status.success(), "must fail when no output fields");
    assert_eq!(out.status.code(), Some(3), "ValidationError exit code");

    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let line = stderr.trim_end();
    let parsed: serde_json::Value =
        serde_json::from_str(line).expect("stderr must be NDJSON");

    assert_eq!(parsed["code"], "validation/empty-collection");
    let location = &parsed["location"];
    assert_eq!(
        location["line"].as_u64(),
        Some(5),
        "must point at <datamodel>, not the <scxml> root: {line}"
    );
}

// ── Leaf-precision regression coverage for remaining kinds ──────
//
// These tests pin the per-`<data>` raise sites that prior leaf-
// precision work introduced (parse_filter / parse_observer /
// parse_timer / parse_validator / parse_interpolation) plus the
// container-anchor case for parse_lookup. A future refactor that
// swaps `located(&data, ...)` → `located(&root, ...)` (or, for
// lookup, `&datamodel` → `&root`) regresses the contract; at
// minimum one of these assertions must trip.
//
// Lookup's per-entry leaf precision is added separately by the
// duplicate-key fixture (see Gap A): the post-loop raise sites
// in parse_lookup currently anchor at `<datamodel>`, and the
// `<sce:entry>` contract is XSD-enforced (`key` and `value` are
// `use="required"`), so the parser path tested here is the
// `entries.is_empty()` raise that legitimately points at the
// container.

/// Filter output `<data sce:direction="out">` with no `sce:filter`
/// must report at the output `<data>` line (not `<scxml>` root or
/// `<datamodel>`). Pins parse_filter L1248-1257.
fn write_filter_missing_filter_attr_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("filter_missing_filter.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="filter" name="bad_filter">
  <datamodel>
    <data id="i" sce:type="float64" sce:direction="in"/>
    <data id="o" sce:type="float64" sce:direction="out"/>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write filter fixture");
    (dir, path)
}

#[test]
fn json_mode_filter_missing_filter_attr_reports_output_data_line() {
    let (_dir, scxml) = write_filter_missing_filter_attr_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/missing-attribute");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(7),
        "must point at the output <data>, not <scxml> root: {parsed}"
    );
}

/// Observer monitor `<data sce:monitor="threshold">` missing
/// `sce:enter` must report at the monitor `<data>` line. Pins
/// parse_observer L1850-1859.
fn write_observer_missing_enter_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("observer_missing_enter.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="observer" name="bad_observer">
  <datamodel>
    <data id="x" sce:type="float64" sce:direction="in"/>
    <data id="warn" sce:monitor="threshold" sce:on-enter="emit"/>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write observer fixture");
    (dir, path)
}

#[test]
fn json_mode_observer_missing_enter_reports_monitor_data_line() {
    let (_dir, scxml) = write_observer_missing_enter_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/missing-attribute");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(7),
        "must point at the monitor <data>, not <scxml> root: {parsed}"
    );
}

/// Timer doc missing `<sce:period>` must report at the document
/// root (`<scxml>`) line — the validator anchors at the parent
/// node because the missing child has no source location of its
/// own. Pins parse_timer (watching-zenoh RFC §5.D shape).
fn write_timer_missing_period_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("timer_missing_period.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="timer" name="bad_timer">
  <sce:fire-event>tick</sce:fire-event>
</scxml>
"#;
    std::fs::write(&path, body).expect("write timer fixture");
    (dir, path)
}

#[test]
fn json_mode_timer_missing_period_reports_root_line() {
    let (_dir, scxml) = write_timer_missing_period_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/missing-element");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(2),
        "must point at the <scxml> root (the missing <sce:period> has no own location): {parsed}"
    );
}

/// Validator input `<data>` whose `sce:sample-interval` passes the
/// XSD pattern (`\d+(\.\d+)?(ms|s|m|h)?`) but fails the parser's
/// stricter `ms`-or-`s` requirement must report at the input
/// `<data>` line. Pins parse_validator L622-623 wrapping
/// parse_time_interval.
fn write_validator_bad_sample_interval_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("validator_bad_sample.scxml");
    // `5h` passes the XSD `(ms|s|m|h)?` enum but parse_time_interval
    // only accepts `ms`/`s`, so the diagnostic comes from the parser
    // (validation/numeric-parse) rather than the schema validator.
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="validator" name="bad_validator">
  <datamodel>
    <data id="x" sce:type="int32" sce:direction="in"
          sce:max-delta="50" sce:sample-interval="5h"/>
    <data id="ok" sce:type="bool" sce:direction="out"/>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write validator fixture");
    (dir, path)
}

#[test]
fn json_mode_validator_bad_sample_interval_reports_input_data_line() {
    let (_dir, scxml) = write_validator_bad_sample_interval_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/numeric-parse");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(6),
        "must point at the input <data> (start tag line), not <scxml> root: {parsed}"
    );
}

/// Interpolation output `<data sce:direction="out">` missing
/// `sce:interpolation` must report at the output `<data>` line.
/// Pins parse_interpolation L1402-1410.
fn write_interpolation_missing_method_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("interp_missing_method.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="interpolation" name="bad_interp">
  <datamodel>
    <data id="r" sce:type="uint16" sce:direction="in"/>
    <data id="o" sce:type="float64" sce:direction="out"/>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write interpolation fixture");
    (dir, path)
}

#[test]
fn json_mode_interpolation_missing_method_reports_output_data_line() {
    let (_dir, scxml) = write_interpolation_missing_method_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/missing-attribute");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(7),
        "must point at the output <data>, not <scxml> root: {parsed}"
    );
}
fn write_procedure_bad_transition_target_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("proc_bad_target.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" name="bad_proc"
       initial="A">
  <state id="A">
    <transition target="NONEXISTENT"/>
  </state>
  <final id="end"/>
</scxml>
"#;
    std::fs::write(&path, body).expect("write procedure fixture");
    (dir, path)
}

#[test]
fn json_mode_procedure_bad_transition_target_reports_transition_line() {
    let (_dir, scxml) = write_procedure_bad_transition_target_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/invalid-reference");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(7),
        "must point at the offending <transition>, not the parent <state>: {parsed}"
    );
}

/// Procedure non-final `<state>` containing zero `<transition>`
/// children must report at the `<state>` line, locking the
/// post-loop "non-final state with no transitions" check against
/// regression to root-level anchoring. Complements the transition-
/// target test above: together they prove ProcedureState.line and
/// ProcedureTransition.line both flow through to diagnostics.
fn write_procedure_state_without_transition_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("proc_no_transition.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" name="bad_proc2"
       initial="A">
  <state id="A">
  </state>
  <final id="end"/>
</scxml>
"#;
    std::fs::write(&path, body).expect("write procedure fixture");
    (dir, path)
}

#[test]
fn json_mode_procedure_state_without_transition_reports_state_line() {
    let (_dir, scxml) = write_procedure_state_without_transition_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/empty-collection");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(6),
        "must point at the offending <state>, not the <scxml> root: {parsed}"
    );
}

/// Lookup with two `<sce:entry>` sharing the same `key` must report
/// at the duplicate `<sce:entry>` line, not at the surrounding
/// `<datamodel>`. Pins parse_sce_entries' inline duplicate
/// detection (which replaced the post-loop check that collapsed to
/// the parent container).
fn write_lookup_duplicate_key_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("lookup_dup_key.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="bad_lookup">
  <datamodel>
    <data id="i" sce:type="uint8" sce:direction="in"/>
    <data id="o" sce:type="string" sce:direction="out"/>
    <data id="m">
      <sce:entry key="1" value="A"/>
      <sce:entry key="1" value="B"/>
    </data>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write lookup duplicate-key fixture");
    (dir, path)
}

#[test]
fn json_mode_lookup_duplicate_key_reports_duplicate_entry_line() {
    let (_dir, scxml) = write_lookup_duplicate_key_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/duplicate-id");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(10),
        "must point at the duplicating <sce:entry>, not <datamodel>: {parsed}"
    );
}

/// Lookup `<data sce:default="X" sce:on-miss="error">` declares both
/// an explicit default and an `error` miss policy — the two are
/// mutually exclusive. The diagnostic must anchor at the
/// declaring `<data>` element, not the surrounding `<datamodel>`,
/// so an agent can edit the offending element directly.
fn write_lookup_default_with_error_policy_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("lookup_incompat.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="bad_lookup">
  <datamodel>
    <data id="i" sce:type="uint8" sce:direction="in"/>
    <data id="o" sce:type="string" sce:direction="out"/>
    <data id="m" sce:default="X" sce:on-miss="error">
      <sce:entry key="1" value="A"/>
    </data>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write lookup incompatible-attrs fixture");
    (dir, path)
}

#[test]
fn json_mode_lookup_default_with_error_policy_reports_data_line() {
    let (_dir, scxml) = write_lookup_default_with_error_policy_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/incompatible-attributes");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(8),
        "must point at the <data> declaring both attrs, not <datamodel>: {parsed}"
    );
}

/// Lookup `<data sce:on-miss="bogus">` must report at the declaring
/// `<data>` element — the agent needs to edit *that* element to
/// repair, not the parent `<datamodel>`.
fn write_lookup_invalid_on_miss_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("lookup_bad_onmiss.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="bad_lookup">
  <datamodel>
    <data id="i" sce:type="uint8" sce:direction="in"/>
    <data id="o" sce:type="string" sce:direction="out"/>
    <data id="m" sce:on-miss="bogus">
      <sce:entry key="1" value="A"/>
    </data>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write lookup invalid-on-miss fixture");
    (dir, path)
}

#[test]
fn json_mode_lookup_invalid_on_miss_reports_data_line() {
    let (_dir, scxml) = write_lookup_invalid_on_miss_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/invalid-attribute");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(8),
        "must point at the <data> declaring sce:on-miss, not <datamodel>: {parsed}"
    );
}

/// Build a filter fixture where the output declares `sce:filter=ftype`
/// but omits the parameter required by that filter type. Used by the
/// three branch-coverage tests below.
fn write_filter_missing_param_fixture(
    label: &str,
    file_stem: &str,
    name: &str,
    filter_attr: &str,
) -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new(label);
    let path = dir.path().join(format!("{file_stem}.scxml"));
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="filter" name="{name}">
  <datamodel>
    <data id="i" sce:type="float64" sce:direction="in"/>
    <data id="o" sce:type="float64" sce:direction="out" sce:filter="{filter_attr}"/>
  </datamodel>
</scxml>
"#,
    );
    std::fs::write(&path, body).expect("write filter param fixture");
    (dir, path)
}

/// Assert that running sce-codegen on `scxml` yields a single NDJSON
/// `validation/missing-attribute` record whose `location.line` equals
/// the output `<data>` (line 7 in every fixture above). Used by the
/// three filter param tests so each branch — moving-average, debounce,
/// low-pass — gets its own regression assertion without copy-paste
/// drift.
fn assert_filter_param_anchored_at_output_data(scxml: &PathBuf) {
    let out = run_generate(&sce_codegen_bin(), scxml, "json");
    assert!(!out.status.success(), "must fail when filter param is missing");
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/missing-attribute");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(7),
        "must point at the output <data>, not <datamodel>: {parsed}"
    );
}

/// Moving-average filter without `sce:window`. Pins the
/// `FilterType::MovingAverage` arm of parse_filter's post-loop
/// param validation.
#[test]
fn json_mode_filter_moving_average_missing_window_reports_output_data_line() {
    let (_dir, scxml) = write_filter_missing_param_fixture(
        "leaf-precision",
        "filter_ma_no_window",
        "bad_ma",
        "moving-average",
    );
    assert_filter_param_anchored_at_output_data(&scxml);
}

/// Debounce filter without `sce:window`. Pins the
/// `FilterType::Debounce` arm — separate from moving-average so a
/// future refactor that splits the match arms cannot regress one
/// branch without tripping its own assertion.
#[test]
fn json_mode_filter_debounce_missing_window_reports_output_data_line() {
    let (_dir, scxml) = write_filter_missing_param_fixture(
        "leaf-precision",
        "filter_debounce_no_window",
        "bad_debounce",
        "debounce",
    );
    assert_filter_param_anchored_at_output_data(&scxml);
}

/// Low-pass filter without `sce:alpha`. Pins the
/// `FilterType::LowPass` arm.
#[test]
fn json_mode_filter_low_pass_missing_alpha_reports_output_data_line() {
    let (_dir, scxml) = write_filter_missing_param_fixture(
        "leaf-precision",
        "filter_lowpass_no_alpha",
        "bad_lowpass",
        "low-pass",
    );
    assert_filter_param_anchored_at_output_data(&scxml);
}

/// Lookup with no `<sce:entry>` children (empty entries collection)
/// must report at the `<datamodel>` container line, not the `<scxml>`
/// root. The `<sce:entry>` element's own attributes (`key`, `value`)
/// are XSD-required, so the per-entry leaf path is exercised by
/// parse_sce_entries' direct raises rather than this fixture; the
/// post-loop `entries.is_empty()` raise legitimately anchors at the
/// most specific node still in scope (`<datamodel>`). Pins
/// parse_lookup L272-281.
fn write_lookup_no_entries_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("leaf-precision");
    let path = dir.path().join("lookup_no_entries.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="bad_lookup">
  <datamodel>
    <data id="i" sce:type="uint8" sce:direction="in"/>
    <data id="o" sce:type="string" sce:direction="out"/>
  </datamodel>
</scxml>
"#;
    std::fs::write(&path, body).expect("write lookup fixture");
    (dir, path)
}

#[test]
fn json_mode_lookup_no_entries_reports_datamodel_line() {
    let (_dir, scxml) = write_lookup_no_entries_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8(out.stderr).unwrap().trim_end()).unwrap();
    assert_eq!(parsed["code"], "validation/empty-collection");
    assert_eq!(
        parsed["location"]["line"].as_u64(),
        Some(5),
        "must point at <datamodel>, not <scxml> root: {parsed}"
    );
}

#[test]
fn human_mode_remains_plain_text() {
    let (_dir, scxml) = write_missing_datamodel_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "human");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    // Human mode preserves the pre-existing banner exactly — any
    // change here is a user-visible regression.
    assert!(
        stderr.contains("Forge codegen error:"),
        "human banner missing: {stderr}"
    );
    // And it MUST NOT be JSON — anything grepping for `{` in human
    // output would otherwise silently mis-parse.
    assert!(
        !stderr.trim_start().starts_with('{'),
        "human mode must not emit JSON: {stderr}"
    );
}

/// Minimal but valid statechart the generator accepts without
/// requiring a script engine. Pins the shape of the success-path
/// stdout manifest.
fn write_trivial_statechart_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("stdout-manifest");
    let path = dir.path().join("trivial.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       initial="a" version="1.0" datamodel="ecmascript" name="trivial">
  <state id="a"><transition event="go" target="b"/></state>
  <state id="b"/>
</scxml>
"#;
    std::fs::write(&path, body).expect("write fixture");
    (dir, path)
}

#[test]
fn stdout_emits_single_json_manifest_on_success() {
    let (dir, scxml) = write_trivial_statechart_fixture();
    let out = Command::new(sce_codegen_bin())
        .args([
            "generate",
            scxml.to_str().unwrap(),
            "--language",
            "rust",
            "--output-dir",
            dir.path().to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen");

    assert!(out.status.success(), "generate must succeed on valid SCXML");

    // stderr holds no diagnostics for a clean run. Pinning this keeps
    // the streams orthogonal — warnings/deprecations in future must
    // either be structured on stderr or accepted as a contract change.
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    assert!(
        stderr.trim().is_empty(),
        "stderr must be empty on clean generate: {stderr}"
    );

    // stdout is exactly one JSON line — the manifest.
    let stdout = String::from_utf8(out.stdout).expect("stdout utf8");
    let line = stdout.trim_end();
    assert!(
        !line.contains('\n'),
        "stdout must be a single line, got: {line}"
    );
    assert!(
        line.starts_with('{') && line.ends_with('}'),
        "stdout must be a JSON object: {line}"
    );

    let parsed: serde_json::Value =
        serde_json::from_str(line).expect("stdout manifest must parse as JSON");
    let obj = parsed.as_object().expect("root must be an object");

    // Required keys per SCE_ERROR_CONTRACT.md §10.
    for key in ["v", "kind", "artifacts", "needs_script_engine"] {
        assert!(obj.contains_key(key), "manifest missing '{key}': {line}");
    }
    assert_eq!(obj["v"].as_u64(), Some(1), "schema v pinned at 1: {line}");
    assert_eq!(obj["kind"], "generate", "kind pinned to subcommand: {line}");
    assert_eq!(
        obj["needs_script_engine"].as_bool(),
        Some(false),
        "trivial machine has no scripts: {line}"
    );

    // Artifacts carry {path} objects, not bare strings — future fields
    // (size, hash, kind-of-artifact) must extend the object additively.
    let artifacts = obj["artifacts"]
        .as_array()
        .expect("artifacts must be an array");
    assert!(
        !artifacts.is_empty(),
        "rust backend must write at least one file: {line}"
    );
    for entry in artifacts {
        let e = entry
            .as_object()
            .expect("each artifact must be a JSON object, not a string");
        let p = e["path"]
            .as_str()
            .expect("artifact.path must be a string")
            .to_string();
        assert!(
            std::path::Path::new(&p).exists(),
            "manifest path must refer to a real file on disk: {p}"
        );
    }

    // `rejected` is absent on clean runs — agents branch on presence.
    assert!(
        !obj.contains_key("rejected"),
        "rejected field must be omitted on clean generate: {line}"
    );
}

#[test]
fn stdout_does_not_emit_human_prose() {
    // Pins the removal of the legacy `Generated: X` / `Needs
    // ScriptEngine: Y` prose. Anything grep'ing those strings in
    // stdout must either migrate to reading the JSON manifest or
    // fail loudly here.
    let (dir, scxml) = write_trivial_statechart_fixture();
    let out = Command::new(sce_codegen_bin())
        .args([
            "generate",
            scxml.to_str().unwrap(),
            "--language",
            "rust",
            "--output-dir",
            dir.path().to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen");

    let stdout = String::from_utf8(out.stdout).expect("stdout utf8");
    assert!(
        !stdout.contains("Generated:"),
        "human-mode 'Generated:' line must not appear: {stdout}"
    );
    assert!(
        !stdout.contains("Needs ScriptEngine:"),
        "human-mode 'Needs ScriptEngine:' line must not appear: {stdout}"
    );
    assert!(
        !stdout.contains("Reason:"),
        "stale 'Reason:' prose must not appear: {stdout}"
    );
    assert!(
        !stdout.contains("Document rejected"),
        "human-mode 'Document rejected' line must not appear on clean generate: {stdout}"
    );
}

// ── sce:template preprocessing: end-to-end wire-contract coverage ──
//
// Exercises the full CLI path (read file → xinclude → template
// expansion → parse → validate → emit diagnostic) for `<sce:use>`
// failure modes. Each negative test confirms the `code`, `stage`,
// and `exit_code` an upstream agent keys on. A positive test pins
// that a successfully-expanded template document produces clean
// codegen output (no diagnostic line on stderr).

/// Build a scratch directory populated with the given named
/// files. Returns the directory handle plus the primary file path
/// (the first entry, by convention the SCXML that sce-codegen
/// consumes).
fn write_template_fixture(
    label: &str,
    files: &[(&str, &str)],
) -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new(label);
    let mut main_path: Option<PathBuf> = None;
    for (name, body) in files {
        let path = dir.path().join(name);
        std::fs::write(&path, body).expect("write fixture");
        if main_path.is_none() {
            main_path = Some(path);
        }
    }
    (dir, main_path.expect("fixtures must include at least one file"))
}

/// Read the single NDJSON diagnostic line from stderr. Panics if
/// stderr doesn't contain exactly one JSON record.
fn single_diagnostic(stderr: &str) -> serde_json::Value {
    let trimmed = stderr.trim_end();
    let lines: Vec<&str> = trimmed.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "expected exactly one diagnostic line, got {}:\n{trimmed}",
        lines.len()
    );
    serde_json::from_str(lines[0])
        .unwrap_or_else(|e| panic!("stderr line is not valid JSON ({e}): {}", lines[0]))
}

#[test]
fn template_expansion_succeeds_on_well_formed_document() {
    let main_scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s1" name="with_template">
  <state id="s1">
    <sce:use template="guard.sce-template.xml" port="80"/>
  </state>
</scxml>
"#;
    let tpl = r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns:sce="http://sce.dev/ext" name="guard">
  <sce:param name="port" required="true"/>
  <transition cond="_event.data == {$port}" target="s1"/>
</sce:template>
"#;
    let (_dir, scxml) = write_template_fixture(
        "template-positive",
        &[("main.scxml", main_scxml), ("guard.sce-template.xml", tpl)],
    );
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(
        out.status.success(),
        "codegen must succeed on expanded document; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    assert!(
        stderr.trim().is_empty(),
        "json mode emits no diagnostic line on success: {stderr}"
    );
}

#[test]
fn template_not_found_emits_xml_template_not_found() {
    let main_scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s1" name="missing_template">
  <state id="s1">
    <sce:use template="missing.sce-template.xml" port="80"/>
  </state>
</scxml>
"#;
    let (_dir, scxml) = write_template_fixture(
        "template-not-found",
        &[("main.scxml", main_scxml)],
    );
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success(), "missing template file must fail");
    assert_eq!(out.status.code(), Some(2), "XML stage exit code is 2");
    let diag = single_diagnostic(&String::from_utf8(out.stderr).unwrap());
    assert_eq!(diag["code"], "xml/template-not-found");
    assert_eq!(diag["stage"], "xml");
    assert_eq!(diag["actual"], "missing.sce-template.xml");
}

#[test]
fn template_malformed_root_emits_xml_template_malformed() {
    let main_scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s1" name="bad_template">
  <state id="s1">
    <sce:use template="bad.sce-template.xml"/>
  </state>
</scxml>
"#;
    // Root is not <sce:template> — expander rejects as Malformed.
    let bad_tpl = r#"<not-a-template><x/></not-a-template>"#;
    let (_dir, scxml) = write_template_fixture(
        "template-malformed",
        &[("main.scxml", main_scxml), ("bad.sce-template.xml", bad_tpl)],
    );
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    let diag = single_diagnostic(&String::from_utf8(out.stderr).unwrap());
    assert_eq!(diag["code"], "xml/template-malformed");
    assert_eq!(diag["stage"], "xml");
}

#[test]
fn template_missing_required_param_emits_add_attribute_fix() {
    let main_scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s1" name="missing_param">
  <state id="s1">
    <sce:use template="guard.sce-template.xml"/>
  </state>
</scxml>
"#;
    let tpl = r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns:sce="http://sce.dev/ext" name="guard">
  <sce:param name="port" required="true"/>
  <transition cond="_event.data == {$port}" target="s1"/>
</sce:template>
"#;
    let (_dir, scxml) = write_template_fixture(
        "template-missing-param",
        &[("main.scxml", main_scxml), ("guard.sce-template.xml", tpl)],
    );
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    let diag = single_diagnostic(&String::from_utf8(out.stderr).unwrap());
    assert_eq!(diag["code"], "xml/template-missing-param");
    assert_eq!(diag["actual"], "port");
    // Structured fix must name the missing attribute so repair bots
    // can patch without re-parsing the message.
    assert_eq!(diag["fix"]["kind"], "add_attribute");
    assert_eq!(diag["fix"]["element"], "sce:use");
    assert_eq!(diag["fix"]["attr"], "port");
}

#[test]
fn template_unknown_param_emits_xml_template_unknown_param() {
    let main_scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s1" name="unknown_param">
  <state id="s1">
    <sce:use template="guard.sce-template.xml" port="80" typo="x"/>
  </state>
</scxml>
"#;
    let tpl = r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns:sce="http://sce.dev/ext" name="guard">
  <sce:param name="port" required="true"/>
  <transition cond="_event.data == {$port}" target="s1"/>
</sce:template>
"#;
    let (_dir, scxml) = write_template_fixture(
        "template-unknown-param",
        &[("main.scxml", main_scxml), ("guard.sce-template.xml", tpl)],
    );
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    let diag = single_diagnostic(&String::from_utf8(out.stderr).unwrap());
    assert_eq!(diag["code"], "xml/template-unknown-param");
    assert_eq!(diag["actual"], "typo");
}

#[test]
fn template_missing_attribute_emits_add_attribute_fix() {
    let main_scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s1" name="missing_attr">
  <state id="s1">
    <sce:use port="80"/>
  </state>
</scxml>
"#;
    let (_dir, scxml) = write_template_fixture(
        "template-missing-attr",
        &[("main.scxml", main_scxml)],
    );
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success(), "missing template attribute must fail");
    let diag = single_diagnostic(&String::from_utf8(out.stderr).unwrap());
    // Deterministic fix points at the exact attribute to insert.
    assert_eq!(diag["code"], "xml/template-missing-attribute");
    assert_eq!(diag["stage"], "xml");
    assert_eq!(diag["fix"]["kind"], "add_attribute");
    assert_eq!(diag["fix"]["element"], "sce:use");
    assert_eq!(diag["fix"]["attr"], "template");
}

#[test]
fn template_cycle_emits_xml_template_cycle() {
    let main_scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s1" name="cycle">
  <state id="s1">
    <sce:use template="a.sce-template.xml"/>
  </state>
</scxml>
"#;
    // a uses b which uses a — chain detected when `a` reappears on
    // the expansion stack.
    let a_tpl = r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns:sce="http://sce.dev/ext" name="a">
  <sce:use template="b.sce-template.xml"/>
</sce:template>
"#;
    let b_tpl = r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns:sce="http://sce.dev/ext" name="b">
  <sce:use template="a.sce-template.xml"/>
</sce:template>
"#;
    let (_dir, scxml) = write_template_fixture(
        "template-cycle",
        &[
            ("main.scxml", main_scxml),
            ("a.sce-template.xml", a_tpl),
            ("b.sce-template.xml", b_tpl),
        ],
    );
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    assert!(!out.status.success());
    let diag = single_diagnostic(&String::from_utf8(out.stderr).unwrap());
    assert_eq!(diag["code"], "xml/template-cycle");
    assert_eq!(diag["stage"], "xml");
}
