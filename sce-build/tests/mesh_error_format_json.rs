// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// End-to-end contract test for the mesh pipeline under
// `sce-codegen --error-format=json --deploy ...`.
//
// Complements `error_format_json.rs` (forge / CLI boundary) by pinning
// the NDJSON wire format for every mesh stage that can terminate the
// process. The lib-level `mesh_external_config.rs` exercises
// `compile_mesh_transport` directly; these tests spawn the real CLI
// binary so a regression in the JSON-emission path (e.g. a mesh error
// printed via `eprintln!` instead of routed through
// `ErrorFormat::emit_and_exit`) surfaces loudly.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

/// Scoped scratch directory under `target/`. Holds the SCXML + deploy.yaml
/// side-files for a single test and deletes them on drop.
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
    fn write(&self, name: &str, body: &str) -> PathBuf {
        let p = self.path().join(name);
        std::fs::write(&p, body).expect("write fixture");
        p
    }
}
impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Minimal SCXML that sends one event to `#motor`. Used by fixtures
/// that want the mesh pipeline to do some work before failing so the
/// failure routes through the stage under test, not short-circuits at
/// parse time.
const BRAKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="brake" initial="idle">
  <state id="idle">
    <transition event="press" target="idle">
      <send target="#motor" event="service.request.compute_force"/>
    </transition>
  </state>
</scxml>
"##;

/// Spawn `sce-codegen generate` with `--deploy` and the JSON error format.
fn run_with_deploy(
    scxml: &PathBuf,
    deploy: &PathBuf,
    out_dir: &std::path::Path,
) -> std::process::Output {
    Command::new(sce_codegen_bin())
        .args([
            "--error-format",
            "json",
            "generate",
            scxml.to_str().unwrap(),
            "--language",
            "rust",
            "--output-dir",
            out_dir.to_str().unwrap(),
            "--deploy",
            deploy.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen")
}

/// Parse exactly one NDJSON record from `stderr`. The mesh pipeline
/// aborts on the first error (no multi-record expansion), so any test
/// that expects more than one line is indicative of a contract change.
fn sole_ndjson_record(stderr: &str) -> serde_json::Value {
    let trimmed = stderr.trim_end();
    assert!(!trimmed.is_empty(), "stderr was empty");
    let mut lines = trimmed.lines();
    let first = lines.next().expect("at least one line");
    assert!(
        lines.next().is_none(),
        "expected exactly one NDJSON record on stderr, got multiple lines:\n{trimmed}"
    );
    assert!(
        first.starts_with('{') && first.ends_with('}'),
        "line is not a JSON object: {first}"
    );
    serde_json::from_str(first).expect("stderr line is valid JSON")
}

/// Assert the invariants every mesh NDJSON record must satisfy before
/// the per-test code / stage / fix checks. Failure here means a mesh
/// emission path skipped the serde contract.
fn assert_core_shape(line: &serde_json::Value) {
    assert_eq!(
        line["v"].as_u64(),
        Some(1),
        "schema version pinned at 1: {line}"
    );
    assert!(
        line["id"].as_str().unwrap_or("").starts_with("fnv1a:"),
        "id must be content-hashed: {line}"
    );
    for key in ["id", "code", "stage", "message"] {
        assert!(
            line.get(key).is_some(),
            "missing required field '{key}' in: {line}"
        );
    }
}

#[test]
fn deploy_unsupported_version_is_ndjson() {
    // deploy.yaml declares a future version the compiler has never
    // seen → `mesh/deploy-unsupported-version` (fix = replace_one_of).
    let dir = ScratchDir::new("mesh-unsupported-version");
    let scxml = dir.write("brake.scxml", BRAKE_SCXML);
    let deploy = dir.write(
        "deploy.yaml",
        r#"
version: "99"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
"#,
    );
    let out = run_with_deploy(&scxml, &deploy, dir.path());

    assert!(
        !out.status.success(),
        "process must fail on unsupported version"
    );
    assert_eq!(
        out.status.code(),
        Some(10),
        "DeployError maps to MeshError::exit_code() == 10"
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/deploy-unsupported-version",
        "record: {rec}"
    );
    assert_eq!(rec["stage"], "mesh-deploy", "record: {rec}");
    // The supported-versions list rides `fix` (repair candidates) —
    // never duplicated on `expected` (SCE_ERROR_CONTRACT.md §3.2).
    assert_eq!(rec["fix"]["kind"], "replace_one_of");
    assert!(
        rec["fix"]["candidates"]
            .as_array()
            .map_or(false, |a| !a.is_empty()),
        "fix.candidates must be populated: {rec}"
    );
    assert!(
        rec.get("expected").is_none(),
        "expected must not duplicate fix.candidates: {rec}"
    );
    // Spec anchor: deploy.yaml schema lives in SCE_MESH.md §14.
    assert_eq!(rec["spec"], "SCE Mesh §14", "record: {rec}");
}

#[test]
fn deploy_parse_error_is_ndjson() {
    // Malformed YAML (control character in `version:` scalar) routes
    // through serde_yaml → `mesh/deploy-parse`. Stage taxonomy must
    // report `mesh-deploy`, not `cli/*`, so agents triage to the YAML
    // repair path and not the CLI-flag one.
    let dir = ScratchDir::new("mesh-deploy-parse");
    let scxml = dir.write("brake.scxml", BRAKE_SCXML);
    let deploy = dir.write(
        "deploy.yaml",
        // `version:` as a block scalar with invalid indentation under
        // it produces a YAML parse error.
        "version: [unterminated\n",
    );
    let out = run_with_deploy(&scxml, &deploy, dir.path());
    assert!(
        !out.status.success(),
        "process must fail on YAML parse error"
    );
    assert_eq!(out.status.code(), Some(10));
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(rec["code"], "mesh/deploy-parse", "record: {rec}");
    assert_eq!(rec["stage"], "mesh-deploy");
    // Parse failures cannot name a deterministic repair — `fix` is
    // absent so agents do not try to apply a non-existent transform.
    assert!(
        rec.get("fix").is_none(),
        "fix must be absent for unstructured parse failures: {rec}"
    );
}

#[test]
fn topology_machine_not_found_is_ndjson() {
    // SCXML `name="ghost"` but deploy.yaml only declares `brake`. The
    // topology resolver cannot find the sender → `mesh/topology-machine-
    // not-found` with a candidate list (`fix = replace_one_of`).
    let dir = ScratchDir::new("mesh-topology-machine");
    let ghost = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="ghost" initial="idle">
  <state id="idle">
    <transition event="press" target="idle">
      <send target="#motor" event="service.request.compute_force"/>
    </transition>
  </state>
</scxml>
"##;
    let scxml = dir.write("ghost.scxml", ghost);
    let deploy = dir.write(
        "deploy.yaml",
        r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
"#,
    );
    // brake.scxml must exist for deploy.yaml integrity; its contents
    // are irrelevant because resolution fails before it is parsed.
    dir.write("brake.scxml", BRAKE_SCXML);
    let out = run_with_deploy(&scxml, &deploy, dir.path());
    assert!(
        !out.status.success(),
        "process must fail on unknown machine"
    );
    assert_eq!(
        out.status.code(),
        Some(11),
        "TopologyError maps to MeshError::exit_code() == 11"
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/topology-machine-not-found",
        "record: {rec}"
    );
    assert_eq!(rec["stage"], "mesh-topology");
    assert_eq!(rec["fix"]["kind"], "replace_one_of");
    let candidates = rec["fix"]["candidates"]
        .as_array()
        .expect("candidates must be populated");
    assert!(
        candidates.iter().any(|v| v == "brake"),
        "fix.candidates must include the declared machine: {rec}"
    );
    assert!(
        rec.get("expected").is_none(),
        "expected must stay absent when fix carries candidates: {rec}"
    );
    assert_eq!(rec["spec"], "SCE Mesh §14");
}
