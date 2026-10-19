// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `machines.<m>.queryables` — the queries a machine serves (SCE
//! Protocol-Synthesis RFC §synth-5-R, Queryables a machine serves).
//!
//! What this file holds the deploy to:
//! - a block of well-formed queryables parses, and the machine's resolved legs
//!   carry the key, the reply and the size bound the deploy implies;
//! - each way a block can fail is refused at parse time under its own
//!   diagnostic code, and the message names the queryable and the field;
//! - the Rust backend emits `{machine}_queryables.rs` from the block, changing
//!   the key or a field or a bound changes the output, and a deploy that
//!   states no block emits nothing, so every existing output stays
//!   byte-identical.

use sce_build::forge::queryables::render_machine_queryables_artifacts;
use sce_build::generator::Language;
use sce_build::mesh::deploy::parse_deploy_str;
use sce_build::mesh::error::{DeployError, QueryableDocumentFault, QueryableKeyFault};
use sce_build::mesh::queryable::resolve_queryables;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

fn template_root() -> PathBuf {
    let mut cursor = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let candidate = cursor.join("tools/codegen/templates");
        if candidate.is_dir() {
            return candidate;
        }
        assert!(
            cursor.pop(),
            "template root not found from CARGO_MANIFEST_DIR ancestry"
        );
    }
}

/// A deploy with one machine, `admin`, whose `queryables:` body is
/// `queryables_yaml`, each line already indented to sit under `queryables:`.
fn deploy_with(queryables_yaml: &str) -> String {
    format!(
        r#"
version: "1.0"
topology:
  host:
    machines:
      admin:
        source: admin.scxml
        queryables:
{queryables_yaml}
"#
    )
}

const GOOD: &str = r#"          - name: status_connect
            key: status/connect
            reply:
              encoding: application/json
              document:
                - { name: endpoints, type: array, max_items: 4, items: { type: string, max_len: 64 } }
                - { name: last_write, type: u64, nullable: true }
          - name: peer
            key: "peer/**"
            reply: { encoding: text/plain, body: peer }"#;

fn rejection(queryables_yaml: &str) -> DeployError {
    parse_deploy_str(&deploy_with(queryables_yaml)).expect_err("deploy must be refused")
}

fn code_of(error: DeployError) -> &'static str {
    use sce_build::forge::diagnostic::SingleDiagnostic;
    sce_build::mesh::error::MeshError::from(error)
        .diagnostic_payload()
        .code
        .as_str()
}

// ── parse and resolve ─────────────────────────────────────────

#[test]
fn well_formed_queryables_resolve_to_legs_with_their_bounds() {
    let cfg = parse_deploy_str(&deploy_with(GOOD)).expect("deploy parses");
    let admin = &cfg.topology["host"].machines["admin"];
    let legs = resolve_queryables("admin", &admin.queryables).expect("resolves");
    assert_eq!(legs.len(), 2);
    assert_eq!(legs[0].key, "status/connect");
    assert!(legs[0].key_is_literal);
    assert_eq!(legs[1].key, "peer/**");
    assert!(!legs[1].key_is_literal);
    assert_eq!(legs[1].max_body_bytes, 4);
    // {"endpoints":[<4 strings of 64>],"last_write":<u64>}
    let string = 2 + 6 * 64;
    let array = 2 + 4 * string + 3;
    assert_eq!(
        legs[0].max_body_bytes as usize,
        2 + ("endpoints".len() + 3) + array + ("last_write".len() + 3) + 20 + 1
    );
}

#[test]
fn a_machine_without_queryables_has_none() {
    let yaml = r#"
version: "1.0"
topology:
  host:
    machines:
      admin:
        source: admin.scxml
"#;
    let cfg = parse_deploy_str(yaml).expect("deploy parses");
    assert!(cfg.topology["host"].machines["admin"].queryables.is_empty());
}

#[test]
fn an_unknown_key_in_a_queryable_is_refused() {
    let err = rejection(
        r#"          - name: a
            key: x
            reply: { encoding: text/plain, body: x }
            keyy: oops"#,
    );
    assert!(
        err.to_string().contains("keyy"),
        "the message names the unknown field: {err}"
    );
}

// ── refusals ──────────────────────────────────────────────────

#[test]
fn a_key_with_an_empty_chunk_is_refused_under_its_code() {
    let err = rejection(
        r#"          - { name: a, key: "a//b", reply: { encoding: text/plain, body: x } }"#,
    );
    let message = err.to_string();
    assert!(
        message.contains("queryable 'a'") && message.contains("'a//b'"),
        "{message}"
    );
    assert!(matches!(
        &err,
        DeployError::QueryableKeyInvalid {
            fault: QueryableKeyFault::EmptyChunk,
            ..
        }
    ));
    assert_eq!(code_of(err), "deploy/queryable-key-invalid");
}

#[test]
fn a_sub_chunk_wildcard_in_a_declared_key_is_refused() {
    let err = rejection(
        r#"          - { name: a, key: "co$*", reply: { encoding: text/plain, body: x } }"#,
    );
    assert!(matches!(
        err,
        DeployError::QueryableKeyInvalid {
            fault: QueryableKeyFault::SubChunkWildcard,
            ..
        }
    ));
}

#[test]
fn a_chunk_after_a_double_star_that_a_canonical_key_would_fold_is_refused() {
    let err = rejection(
        r#"          - { name: a, key: "a/**/*", reply: { encoding: text/plain, body: x } }"#,
    );
    assert!(matches!(
        err,
        DeployError::QueryableKeyInvalid {
            fault: QueryableKeyFault::NotCanonical,
            ..
        }
    ));
}

#[test]
fn the_same_key_twice_is_refused_naming_both() {
    let err = rejection(
        r#"          - { name: a, key: config, reply: { encoding: text/plain, body: x } }
          - { name: b, key: config, reply: { encoding: text/plain, body: y } }"#,
    );
    let message = err.to_string();
    assert!(message.contains("'a'") && message.contains("'b'") && message.contains("'config'"));
    assert_eq!(code_of(err), "deploy/queryable-key-duplicate");
}

#[test]
fn a_name_that_is_not_an_identifier_is_refused() {
    let err = rejection(
        r#"          - { name: "a-b", key: x, reply: { encoding: text/plain, body: x } }"#,
    );
    assert_eq!(code_of(err), "deploy/queryable-name-invalid");
}

#[test]
fn a_reply_with_neither_body_nor_document_is_refused() {
    let err = rejection(r#"          - { name: a, key: x, reply: { encoding: text/plain } }"#);
    assert_eq!(code_of(err), "deploy/queryable-reply-invalid");
}

#[test]
fn an_unbounded_string_in_a_document_is_refused_at_its_path() {
    let err = rejection(
        r#"          - name: a
            key: x
            reply:
              encoding: application/json
              document:
                - name: outer
                  type: object
                  fields:
                    - { name: inner, type: string }"#,
    );
    let DeployError::QueryableDocumentInvalid { path, fault, .. } = &err else {
        panic!("{err:?}");
    };
    assert_eq!(path, "outer.inner");
    assert_eq!(
        *fault,
        QueryableDocumentFault::BoundMissing { attr: "max_len" }
    );
    assert!(err.to_string().contains("outer.inner"));
    assert_eq!(code_of(err), "deploy/queryable-document-invalid");
}

// ── emission ──────────────────────────────────────────────────

fn render(queryables_yaml: &str, language: Language) -> Vec<(String, String)> {
    let cfg = parse_deploy_str(&deploy_with(queryables_yaml)).expect("deploy parses");
    let admin = &cfg.topology["host"].machines["admin"];
    let legs = resolve_queryables("admin", &admin.queryables).expect("resolves");
    render_machine_queryables_artifacts(&template_root(), language, "admin", &legs)
        .expect("render succeeds")
}

#[test]
fn rust_emits_one_module_carrying_the_matching_and_the_writers() {
    let files = render(GOOD, Language::Rust);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "admin_queryables.rs");
    let body = &files[0].1;
    for needle in [
        "pub enum Leg {",
        "StatusConnect,",
        "pub fn answering(query: &str) -> LegSet {",
        "\"status/connect\" => set.insert(Leg::StatusConnect),",
        "keyexpr::intersects(query, \"peer/**\")",
        "pub fn write_status_connect(",
        "pub fn write_peer(out: &mut [u8])",
        "pub const MAX_BODY_BYTES: usize =",
    ] {
        assert!(body.contains(needle), "missing `{needle}`:\n{body}");
    }
}

#[test]
fn changing_a_key_a_field_or_a_bound_changes_the_output() {
    let base = render(GOOD, Language::Rust)[0].1.clone();
    let key = render(
        &GOOD.replace("status/connect", "status/link"),
        Language::Rust,
    )[0]
    .1
    .clone();
    let field = render(&GOOD.replace("last_write", "last_seen"), Language::Rust)[0]
        .1
        .clone();
    let bound = render(
        &GOOD.replace("max_items: 4", "max_items: 5"),
        Language::Rust,
    )[0]
    .1
    .clone();
    assert_ne!(base, key);
    assert_ne!(base, field);
    assert_ne!(base, bound);
}

#[test]
fn a_backend_that_does_not_link_the_rust_module_emits_nothing() {
    assert!(render(GOOD, Language::C11).is_empty());
    assert!(render(GOOD, Language::Cpp).is_empty());
}

#[test]
fn no_queryables_emits_nothing() {
    let legs = Vec::new();
    let files =
        render_machine_queryables_artifacts(&template_root(), Language::Rust, "admin", &legs)
            .expect("render succeeds");
    assert!(files.is_empty());
}

// ── through the orchestrator ──────────────────────────────────

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

struct ScratchDir(PathBuf);
impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const ADMIN_FSM: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       name="admin"
       version="1.0"
       initial="idle"
       datamodel="ecmascript">
  <state id="idle"/>
</scxml>"##;

fn orchestrate(label: &str, deploy_yaml: &str) -> BTreeMap<String, Vec<u8>> {
    let dir = ScratchDir::new(label);
    let scxml = dir.path().join("admin.scxml");
    std::fs::write(&scxml, ADMIN_FSM).expect("write scxml");
    let deploy = dir.path().join("deploy.yaml");
    std::fs::write(&deploy, deploy_yaml).expect("write deploy");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("mkdir out");
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("orchestrate")
        .arg("--scxml")
        .arg(&scxml)
        .args(["--language", "rust", "--output-dir"])
        .arg(&out_dir)
        .arg("--deploy")
        .arg(&deploy)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen orchestrate");
    assert!(
        out.status.success(),
        "orchestrate must succeed (stderr: {})",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut files = BTreeMap::new();
    collect(&out_dir, &out_dir, &mut files);
    files
}

fn collect(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).expect("read output dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            let rel = path
                .strip_prefix(root)
                .expect("under root")
                .to_string_lossy()
                .into_owned();
            files.insert(rel, std::fs::read(&path).expect("read output file"));
        }
    }
}

fn queryables_file(files: &BTreeMap<String, Vec<u8>>) -> Option<&String> {
    files
        .keys()
        .find(|name| name.ends_with("admin_queryables.rs"))
}

/// The `source-hash` header covers the deploy file's text and the `From:`
/// header names the scratch directory of the run; the body is what must stay.
fn without_run_headers(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(name, bytes)| {
            let body = String::from_utf8_lossy(bytes)
                .lines()
                .filter(|line| {
                    !line.starts_with("// source-hash:") && !line.starts_with("// From:")
                })
                .collect::<Vec<_>>()
                .join("\n");
            (name.clone(), body)
        })
        .collect()
}

#[test]
fn the_orchestrator_writes_the_queryables_module() {
    let files = orchestrate("queryables-emits", &deploy_with(GOOD));
    let name = queryables_file(&files).unwrap_or_else(|| {
        panic!(
            "admin_queryables.rs missing; wrote {:?}",
            files.keys().collect::<Vec<_>>()
        )
    });
    let body = String::from_utf8_lossy(&files[name]);
    assert!(body.contains("pub fn answering("), "{body}");
}

#[test]
fn stating_queryables_adds_one_file_and_changes_no_other() {
    let control = r#"
version: "1.0"
topology:
  host:
    machines:
      admin:
        source: admin.scxml
"#;
    let without = orchestrate("queryables-control", control);
    let with = orchestrate("queryables-treated", &deploy_with(GOOD));
    assert!(
        queryables_file(&without).is_none(),
        "a deploy with no queryables must emit no queryables file"
    );
    let name = queryables_file(&with).expect("the treated run writes the module");
    let mut with_rest = with.clone();
    with_rest.remove(name.as_str());
    assert_eq!(
        without_run_headers(&without),
        without_run_headers(&with_rest),
        "every other output must be byte-identical with and without the block"
    );
}
