// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE-VERIFIES: mesh-18 mesh-19 mesh-20
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
    scxml: &std::path::Path,
    deploy: &std::path::Path,
    out_dir: &std::path::Path,
) -> std::process::Output {
    run_with_deploy_for("rust", scxml, deploy, out_dir)
}

/// [`run_with_deploy`] for the backend named by `language`.
fn run_with_deploy_for(
    language: &str,
    scxml: &std::path::Path,
    deploy: &std::path::Path,
    out_dir: &std::path::Path,
) -> std::process::Output {
    Command::new(sce_codegen_bin())
        .args([
            "--error-format",
            "json",
            "generate",
            scxml.to_str().unwrap(),
            "--language",
            language,
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
            .is_some_and(|a| !a.is_empty()),
        "fix.candidates must be populated: {rec}"
    );
    assert!(
        rec.get("expected").is_none(),
        "expected must not duplicate fix.candidates: {rec}"
    );
    // Spec anchor: deploy.yaml schema lives in SCE_MESH.md §14.
    assert_eq!(rec["spec"], "SCE Mesh §14", "record: {rec}");
}

/// A client that sends one fire-and-forget event to `#server` — the one
/// pattern every implemented transport carries, so a refusal below is
/// about which core serves the transport and never about the pattern.
const CLIENT_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="client" initial="idle">
  <state id="idle">
    <onentry><send target="#server" event="ping"/></onentry>
  </state>
</scxml>
"##;

/// The machine [`CLIENT_SCXML`] sends to, which receives its `ping`.
const SERVER_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="server" initial="idle">
  <state id="idle">
    <transition event="ping" target="idle"/>
  </state>
</scxml>
"##;

#[test]
fn a_transport_the_other_core_serves_is_refused_with_the_asking_cores_set() {
    // SCE_MESH.md §mesh-10.4.2 `served_by`: `wss` is carried by the host
    // cores, so the C++ router has no arm for it. The repair offers what
    // the C++ core does carry — naming `wss` there would send the author
    // back to the transport that was just refused.
    let dir = ScratchDir::new("mesh-served-by");
    let scxml = dir.write("client.scxml", CLIENT_SCXML);
    dir.write("server.scxml", SERVER_SCXML);
    let deploy = dir.write(
        "deploy.yaml",
        r##"
version: "1.0"
topology:
  phone:
    machines:
      client:
        source: client.scxml
        bindings:
          "#server": { transport: wss, url: "wss://cal.example" }
  cloud:
    machines:
      server: { source: server.scxml }
"##,
    );
    let out = run_with_deploy_for("cpp", &scxml, &deploy, dir.path());
    assert!(
        !out.status.success(),
        "C++ must refuse a host-served transport"
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/codegen-unsupported-transport",
        "record: {rec}"
    );
    assert_eq!(rec["actual"], "wss", "record: {rec}");
    let candidates: Vec<&str> = rec["fix"]["candidates"]
        .as_array()
        .expect("fix.candidates")
        .iter()
        .filter_map(|c| c.as_str())
        .collect();
    assert!(
        candidates.contains(&"zenoh") && !candidates.contains(&"wss"),
        "the candidates are the C++ core's own: {candidates:?}"
    );
}

#[test]
fn a_host_core_backend_refuses_a_transport_the_cpp_router_serves() {
    // The same rule from the other side: a Rust machine's peer table is
    // read by a host core, which carries no arm for a C++ template transport
    // (the host cores' own Zenoh binding is `zenoh_inbox`). The repair offers
    // what the host cores do carry.
    let dir = ScratchDir::new("mesh-served-by-host");
    let scxml = dir.write("client.scxml", CLIENT_SCXML);
    dir.write("server.scxml", SERVER_SCXML);
    let deploy = dir.write(
        "deploy.yaml",
        r##"
version: "1.0"
topology:
  phone:
    machines:
      client:
        source: client.scxml
        bindings:
          "#server": { transport: zenoh, key: "cal/server" }
  cloud:
    machines:
      server: { source: server.scxml }
"##,
    );
    let out = run_with_deploy_for("rust", &scxml, &deploy, dir.path());
    assert!(!out.status.success(), "a host core must refuse zenoh");
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/codegen-unsupported-transport",
        "record: {rec}"
    );
    assert_eq!(
        rec["fix"]["candidates"],
        serde_json::json!(["wss", "zenoh_inbox"]),
        "record: {rec}"
    );
}

/// A deploy.yaml whose one binding, `#server`, is on `transport` (written as
/// YAML flow-mapping entries, so a test may add keys after the name).
fn zenoh_inbox_deploy(binding: &str) -> String {
    format!(
        r##"
version: "1.0"
topology:
  phone:
    machines:
      client:
        source: client.scxml
        bindings:
          "#server": {{ {binding} }}
  cloud:
    machines:
      server: {{ source: server.scxml }}
"##
    )
}

#[test]
fn zenoh_inbox_is_refused_by_the_cpp_router_and_is_not_its_own_repair() {
    // SCE_MESH.md §mesh-20: the Zenoh binding of the host cores is
    // `zenoh_inbox`, which the C++ router has no arm for. A C++ machine bound
    // over it is refused with the C++ core's own set, which is not empty and
    // does not offer the refused name back.
    let dir = ScratchDir::new("mesh-zenoh-mesh-cpp");
    let scxml = dir.write("client.scxml", CLIENT_SCXML);
    dir.write("server.scxml", SERVER_SCXML);
    let deploy = dir.write("deploy.yaml", &zenoh_inbox_deploy("transport: zenoh_inbox"));
    let out = run_with_deploy_for("cpp", &scxml, &deploy, dir.path());
    assert!(
        !out.status.success(),
        "C++ must refuse a host-served transport"
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/codegen-unsupported-transport",
        "record: {rec}"
    );
    assert_eq!(rec["actual"], "zenoh_inbox", "record: {rec}");
    let candidates: Vec<&str> = rec["fix"]["candidates"]
        .as_array()
        .expect("fix.candidates")
        .iter()
        .filter_map(|c| c.as_str())
        .collect();
    assert!(
        !candidates.is_empty() && !candidates.contains(&"zenoh_inbox"),
        "the candidates are the C++ core's own: {candidates:?}"
    );
}

#[test]
fn zenoh_inbox_reaches_the_rust_peer_table_as_a_link_with_no_parameters() {
    // SCE_MESH.md §mesh-20: the binding names the peer and nothing else, so
    // the table's link carries no URL and no key; the host builds its Zenoh
    // session itself. The peer's transport label is the registry name.
    let dir = ScratchDir::new("mesh-zenoh-mesh-rust");
    let scxml = dir.write("client.scxml", CLIENT_SCXML);
    dir.write("server.scxml", SERVER_SCXML);
    let deploy = dir.write("deploy.yaml", &zenoh_inbox_deploy("transport: zenoh_inbox"));
    let out = run_with_deploy_for("rust", &scxml, &deploy, dir.path());
    assert!(
        out.status.success(),
        "the Rust host core carries zenoh_inbox; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let table = peer_table_under(dir.path());
    assert!(
        table.contains("link: PeerLink::ZenohInbox,"),
        "the peer is reached through the host's Zenoh session:\n{table}"
    );
    assert!(
        table.contains("transport: \"zenoh_inbox\","),
        "the peer carries the registry's name:\n{table}"
    );
    assert!(
        !table.contains("PeerLink::Wss"),
        "a Zenoh peer must not be read as a WebSocket one:\n{table}"
    );
}

#[test]
fn a_zenoh_inbox_binding_takes_no_key_of_its_own() {
    // The key is derived from the two machine names (§mesh-20.1), so a
    // `key:` here would be a setting that parses and then does nothing. It
    // is refused as an unknown binding key: the registry's closed set for
    // the transport is empty.
    let dir = ScratchDir::new("mesh-zenoh-mesh-key");
    let scxml = dir.write("client.scxml", CLIENT_SCXML);
    dir.write("server.scxml", SERVER_SCXML);
    let deploy = dir.write(
        "deploy.yaml",
        &zenoh_inbox_deploy("transport: zenoh_inbox, key: \"cal/server\""),
    );
    let out = run_with_deploy_for("rust", &scxml, &deploy, dir.path());
    assert!(
        !out.status.success(),
        "a key on zenoh_inbox must be refused"
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/deploy-unknown-binding-field",
        "record: {rec}"
    );
    assert_eq!(rec["stage"], "mesh-deploy", "record: {rec}");
    assert_eq!(rec["actual"], "key", "record: {rec}");
}

#[test]
fn the_kotlin_core_has_no_zenoh_adapter_so_its_peer_table_refuses_zenoh_inbox() {
    // SCE_MESH.md §mesh-20: only the Rust core carries the Zenoh adapter. A
    // Kotlin peer table naming it would have no `PeerLink` to read, so it is
    // refused by name, and the repair points at the backend that has it.
    let dir = ScratchDir::new("mesh-zenoh-mesh-kotlin");
    let scxml = dir.write("client.scxml", CLIENT_SCXML);
    dir.write("server.scxml", SERVER_SCXML);
    let deploy = dir.write("deploy.yaml", &zenoh_inbox_deploy("transport: zenoh_inbox"));
    let out = run_with_deploy_for("kotlin", &scxml, &deploy, dir.path());
    assert!(
        !out.status.success(),
        "the Kotlin core must refuse a transport it has no adapter for"
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/codegen-host-core-unsupported",
        "record: {rec}"
    );
    assert_eq!(rec["spec"], "SCE Mesh §19", "record: {rec}");
    let message = rec["message"].as_str().expect("message");
    assert!(
        message.contains("transport 'zenoh_inbox'")
            && message.contains("kotlin host Mesh core")
            && message.contains("--lang rust"),
        "record: {rec}"
    );
}

/// The Rust peer table `sce-codegen generate --deploy` wrote under `dir`,
/// wherever the generator put it.
fn peer_table_under(dir: &std::path::Path) -> String {
    fn find(dir: &std::path::Path) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find(&path) {
                    return Some(found);
                }
            } else if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with("_mesh_peers.rs"))
            {
                return Some(path);
            }
        }
        None
    }
    let path = find(dir).expect("the Rust backend writes `<document>_mesh_peers.rs`");
    std::fs::read_to_string(path).expect("read the peer table")
}

#[test]
fn a_host_core_refuses_a_feature_only_the_cpp_router_has() {
    // SCE_MESH.md §mesh-19: a `server:` block is a request/reply server the
    // C++ router hosts. A host core's peer table has nowhere to put it, so it
    // is refused rather than left out of a table that would still build.
    //
    // The document has to SERVE something for the block to mean anything:
    // a `server:` section over a machine answering no request builds no
    // server on any backend, so there would be nothing to refuse.
    const SERVING_CLIENT: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="client" initial="idle">
  <state id="idle">
    <onentry><send target="#server" event="ping"/></onentry>
    <transition event="service.request.lookup" target="idle">
      <raise event="service.response.lookup"/>
    </transition>
  </state>
</scxml>
"##;
    let dir = ScratchDir::new("mesh-host-core-gap");
    let scxml = dir.write("client.scxml", SERVING_CLIENT);
    dir.write("server.scxml", SERVER_SCXML);
    let deploy = dir.write(
        "deploy.yaml",
        r##"
version: "1.0"
topology:
  phone:
    machines:
      client:
        source: client.scxml
        server: { transport: zenoh, key: "cal/client" }
        bindings:
          "#server": { transport: wss, url: "wss://cal.example" }
  cloud:
    machines:
      server: { source: server.scxml }
"##,
    );
    let out = run_with_deploy_for("rust", &scxml, &deploy, dir.path());
    assert!(
        !out.status.success(),
        "a host core must refuse a server block"
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/codegen-host-core-unsupported",
        "record: {rec}"
    );
    assert_eq!(rec["spec"], "SCE Mesh §19", "record: {rec}");
    assert!(
        rec["message"]
            .as_str()
            .is_some_and(|m| m.contains("`server:` block")),
        "record: {rec}"
    );
}

#[test]
fn deploy_parse_error_is_ndjson() {
    // Malformed YAML (control character in `version:` scalar) routes
    // through serde_yaml → `mesh/deploy-parse`. Stage taxonomy must
    // report `mesh-deploy`, not `cli/*`, so consumers triage to the YAML
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
    // absent so consumers do not try to apply a non-existent transform.
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
