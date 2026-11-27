// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
        candidates.contains(&"zenoh_keyexpr") && !candidates.contains(&"wss"),
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
          "#server": { transport: zenoh_keyexpr, key: "cal/server" }
  cloud:
    machines:
      server: { source: server.scxml }
"##,
    );
    let out = run_with_deploy_for("rust", &scxml, &deploy, dir.path());
    assert!(
        !out.status.success(),
        "a host core must refuse zenoh_keyexpr"
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/codegen-unsupported-transport",
        "record: {rec}"
    );
    assert_eq!(
        rec["fix"]["candidates"],
        serde_json::json!(["wss", "zenoh_inbox", "https"]),
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
    let table = generated_file_under(dir.path(), "_mesh_peers.rs");
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

/// The machine [`CLIENT_SCXML`] sends to, answering its `ping` with a `pong`: a server that sends
/// to its client, so that its own peer table holds the client.
const REPLYING_SERVER_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="server" initial="idle">
  <state id="idle">
    <transition event="ping" target="idle">
      <send target="#client" event="pong"/>
    </transition>
  </state>
</scxml>
"##;

/// [`CLIENT_SCXML`] that also takes the `pong` [`REPLYING_SERVER_SCXML`] answers with, so that
/// every send between the two has a transition that receives it.
const ANSWERED_CLIENT_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="client" initial="idle">
  <state id="idle">
    <onentry><send target="#server" event="ping"/></onentry>
    <transition event="pong" target="idle"/>
  </state>
</scxml>
"##;

/// A client that posts its exchanges to the server and a server that keeps its mailbox, over the
/// HTTPS binding (SCE_MESH.md section 22).
const HTTPS_DEPLOY: &str = r##"
version: "1.0"
topology:
  phone:
    machines:
      client:
        source: client.scxml
        bindings:
          "#server": { transport: https, url: "https://cal.example/api", authenticator: install, hold_ms: 9000 }
  cloud:
    machines:
      server:
        source: server.scxml
        bindings:
          "#client": { transport: https, listen: "0.0.0.0:8443", authenticator: install, mailbox: 64 }
"##;

#[test]
fn an_https_dialing_binding_reaches_the_rust_peer_table_with_its_url_and_hold() {
    // SCE_MESH.md section 22: the side with `url` posts the exchanges, so its link carries the
    // server's base, the authenticator the host proves its name with, and the poll's hold.
    let dir = ScratchDir::new("mesh-https-dial-rust");
    let scxml = dir.write("client.scxml", ANSWERED_CLIENT_SCXML);
    dir.write("server.scxml", REPLYING_SERVER_SCXML);
    let deploy = dir.write("deploy.yaml", HTTPS_DEPLOY);
    let out = run_with_deploy_for("rust", &scxml, &deploy, dir.path());
    assert!(
        out.status.success(),
        "the Rust host core carries https; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let table = generated_file_under(dir.path(), "_mesh_peers.rs");
    for line in [
        "transport: \"https\",",
        "link: PeerLink::HttpsDial {",
        "url: \"https://cal.example/api\",",
        "authenticator: sce_rust_mesh::peers::HttpsAuthenticator::Install,",
        "hold_ms: 9000,",
    ] {
        assert!(table.contains(line), "missing `{line}`:\n{table}");
    }
    assert!(
        !table.contains("PeerLink::HttpsAccept") && !table.contains("PeerLink::Wss"),
        "the dialing side is read as one link only:\n{table}"
    );
}

#[test]
fn an_https_accepting_binding_reaches_the_rust_peer_table_with_its_bounds() {
    // SCE_MESH.md section 22: the side with `listen` keeps the client's mailbox, so its link
    // carries the address and every bound, the ones the binding leaves out at their defaults.
    let dir = ScratchDir::new("mesh-https-accept-rust");
    dir.write("client.scxml", ANSWERED_CLIENT_SCXML);
    let scxml = dir.write("server.scxml", REPLYING_SERVER_SCXML);
    let deploy = dir.write("deploy.yaml", HTTPS_DEPLOY);
    let out = run_with_deploy_for("rust", &scxml, &deploy, dir.path());
    assert!(
        out.status.success(),
        "the Rust host core carries https; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let table = generated_file_under(dir.path(), "_mesh_peers.rs");
    for line in [
        "link: PeerLink::HttpsAccept {",
        "listen: \"0.0.0.0:8443\",",
        "authenticator: sce_rust_mesh::peers::HttpsAuthenticator::Install,",
        "max_hold_ms: 30000,",
        "max_body_bytes: 1048576,",
        "absence_ms: 120000,",
        "mailbox: 64,",
    ] {
        assert!(table.contains(line), "missing `{line}`:\n{table}");
    }
}

/// [`ANSWERED_CLIENT_SCXML`] under the statically typed data model, whose state a checkpoint holds.
const SAVING_CLIENT_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="client" initial="idle" datamodel="sce-static">
  <state id="idle">
    <onentry><send target="#server" event="ping"/></onentry>
    <transition event="pong" target="idle"/>
  </state>
</scxml>
"##;

/// [`REPLYING_SERVER_SCXML`] under the statically typed data model.
const SAVING_SERVER_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="server" initial="idle" datamodel="sce-static">
  <state id="idle">
    <transition event="ping" target="idle">
      <send target="#client" event="pong"/>
    </transition>
  </state>
</scxml>
"##;

/// The HTTPS binding of [`HTTPS_DEPLOY`] under `receipt: durable` (docs/adr/0109, D10), both
/// machines checkpointed, the client sending under a `receipt:` block of its own and the server
/// keeping each outbox entry a day.
const HTTPS_DURABLE_DEPLOY: &str = r##"
version: "1.0"
topology:
  phone:
    machines:
      client:
        source: client.scxml
        persistence: checkpoint
        receipt: { window: 4, retry_ms: 250 }
        bindings:
          "#server": { transport: https, url: "https://cal.example/api", authenticator: install, receipt: durable }
  cloud:
    machines:
      server:
        source: server.scxml
        persistence: checkpoint
        bindings:
          "#client": { transport: https, listen: "0.0.0.0:9443", authenticator: install, receipt: durable, retain_ms: 86400000 }
"##;

/// docs/adr/0109, D10: each side of a durable `https` binding reaches the Rust peer table with
/// its receipt, the client's with the window and retry of its machine's `receipt:` block and the
/// server's with its outbox's retain. Red if the template drops `receipt` or `retain_ms`.
#[test]
fn a_durable_https_binding_reaches_the_rust_peer_table_with_its_receipt() {
    for (label, own, own_scxml, other, other_scxml, lines) in [
        (
            "mesh-https-durable-client",
            "client.scxml",
            SAVING_CLIENT_SCXML,
            "server.scxml",
            SAVING_SERVER_SCXML,
            [
                "link: PeerLink::HttpsDial {",
                "receipt: sce_rust_mesh::peers::Receipt::Durable { window: 4, retry_ms: 250 },",
            ],
        ),
        (
            "mesh-https-durable-server",
            "server.scxml",
            SAVING_SERVER_SCXML,
            "client.scxml",
            SAVING_CLIENT_SCXML,
            [
                "receipt: sce_rust_mesh::peers::Receipt::Durable { window: 256, retry_ms: 1000 },",
                "retain_ms: 86400000,",
            ],
        ),
    ] {
        let dir = ScratchDir::new(label);
        let scxml = dir.write(own, own_scxml);
        dir.write(other, other_scxml);
        let deploy = dir.write("deploy.yaml", HTTPS_DURABLE_DEPLOY);
        let out = run_with_deploy_for("rust", &scxml, &deploy, dir.path());
        assert!(
            out.status.success(),
            "{label}: the Rust host core keeps a durable receipt; stderr:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let table = generated_file_under(dir.path(), "_mesh_peers.rs");
        for line in lines {
            assert!(table.contains(line), "{label}: missing `{line}`:\n{table}");
        }
    }
}

/// docs/adr/0109, D0 and D10: a machine with a durable binding is refused by name by every core
/// but Rust's, the C++ router's dialing arm of `https` included
/// (`mesh/codegen-durable-receipt-unsupported`); and `persistence: checkpoint` on a document
/// that saves nothing is refused before anything is emitted
/// (`mesh/codegen-persistence-without-saved-state`).
#[test]
fn what_only_the_rust_core_keeps_durable_is_refused_elsewhere_by_its_codes() {
    let dir = ScratchDir::new("mesh-https-durable-cpp");
    let scxml = dir.write("client.scxml", SAVING_CLIENT_SCXML);
    dir.write("server.scxml", SAVING_SERVER_SCXML);
    let deploy = dir.write("deploy.yaml", HTTPS_DURABLE_DEPLOY);
    let out = run_with_deploy_for("cpp", &scxml, &deploy, dir.path());
    assert!(
        !out.status.success(),
        "the C++ router keeps no durable receipt"
    );
    assert_eq!(out.status.code(), Some(12), "a CodegenError");
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/codegen-durable-receipt-unsupported",
        "record: {rec}"
    );
    assert_eq!(rec["spec"], "SCE Mesh §10.11", "record: {rec}");
    assert_eq!(rec["actual"], "client", "record: {rec}");

    let dir = ScratchDir::new("mesh-persistence-without-saved-state");
    let scxml = dir.write("server.scxml", REPLYING_SERVER_SCXML);
    dir.write("client.scxml", SAVING_CLIENT_SCXML);
    let deploy = dir.write("deploy.yaml", HTTPS_DURABLE_DEPLOY);
    let out = run_with_deploy_for("rust", &scxml, &deploy, dir.path());
    assert!(!out.status.success(), "a document that saves nothing");
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/codegen-persistence-without-saved-state",
        "record: {rec}"
    );
    assert_eq!(rec["actual"], "server", "record: {rec}");
}

/// The HTTPS binding of [`HTTPS_DEPLOY`] with message signatures (docs/adr/0109, D9): the client
/// signs its requests with its key and verifies what the server sends under a trust set, and the
/// server signs with a key of its host and holds signed requests to a skew of its own.
const HTTPS_SIGNED_DEPLOY: &str = r##"
version: "1.0"
topology:
  phone:
    machines:
      client:
        source: client.scxml
        bindings:
          "#server": { transport: https, url: "https://cal.example/api", authenticator: key, verify: servers }
  cloud:
    machines:
      server:
        source: server.scxml
        bindings:
          "#client": { transport: https, listen: "0.0.0.0:9443", authenticator: key, sign: "generated:0a1b", max_skew_ms: 60000 }
"##;

/// docs/adr/0109, D9: each side's link carries its half of the signatures: the trust set the
/// client verifies under, and the key the server signs with and the skew it allows; and a binding
/// that names none of them carries `None` and the default skew. Red if the Rust peer table drops a
/// key or spells the `key` authenticator as another.
#[test]
fn an_https_binding_that_signs_reaches_the_rust_peer_table_with_its_keys() {
    for (side, lines) in [
        (
            "client.scxml",
            [
                "authenticator: sce_rust_mesh::peers::HttpsAuthenticator::Key,",
                "verify: Some(\"servers\"),",
            ]
            .as_slice(),
        ),
        (
            "server.scxml",
            [
                "authenticator: sce_rust_mesh::peers::HttpsAuthenticator::Key,",
                "sign: Some(\"generated:0a1b\"),",
                "max_skew_ms: 60000,",
            ]
            .as_slice(),
        ),
    ] {
        // A directory of its own for each side, so that the one table generated in it is that
        // side's.
        let dir = ScratchDir::new("mesh-https-signed-rust");
        dir.write("client.scxml", ANSWERED_CLIENT_SCXML);
        dir.write("server.scxml", REPLYING_SERVER_SCXML);
        let deploy = dir.write("deploy.yaml", HTTPS_SIGNED_DEPLOY);
        let scxml = dir.path().join(side);
        let out = run_with_deploy_for("rust", &scxml, &deploy, dir.path());
        assert!(
            out.status.success(),
            "the Rust host core carries https; stderr:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let table = generated_file_under(dir.path(), "_mesh_peers.rs");
        for line in lines {
            assert!(table.contains(line), "missing `{line}`:\n{table}");
        }
    }

    let plain = ScratchDir::new("mesh-https-unsigned-rust");
    plain.write("client.scxml", ANSWERED_CLIENT_SCXML);
    let scxml = plain.write("server.scxml", REPLYING_SERVER_SCXML);
    let deploy = plain.write("deploy.yaml", HTTPS_DEPLOY);
    let out = run_with_deploy_for("rust", &scxml, &deploy, plain.path());
    assert!(out.status.success());
    let table = generated_file_under(plain.path(), "_mesh_peers.rs");
    for line in ["sign: None,", "max_skew_ms: 300000,"] {
        assert!(table.contains(line), "missing `{line}`:\n{table}");
    }
}

#[test]
fn zenoh_inbox_reaches_the_swift_peer_table_as_a_link_with_no_parameters() {
    // SCE_MESH.md §mesh-20.4: the Swift core carries the Zenoh binding over a
    // client the host supplies, so its peer table names the link the way the
    // Rust one does, with no URL and no key.
    let dir = ScratchDir::new("mesh-zenoh-mesh-swift");
    let scxml = dir.write("client.scxml", CLIENT_SCXML);
    dir.write("server.scxml", SERVER_SCXML);
    let deploy = dir.write("deploy.yaml", &zenoh_inbox_deploy("transport: zenoh_inbox"));
    let out = run_with_deploy_for("swift", &scxml, &deploy, dir.path());
    assert!(
        out.status.success(),
        "the Swift host core carries zenoh_inbox; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let table = generated_file_under(dir.path(), "MeshPeers.swift");
    assert!(
        table.contains("link: .zenohInbox"),
        "the peer is reached through the host's Zenoh session:\n{table}"
    );
    assert!(
        table.contains("transport: \"zenoh_inbox\","),
        "the peer carries the registry's name:\n{table}"
    );
    assert!(
        !table.contains(".wss"),
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
    // SCE_MESH.md §mesh-20: the Rust and the Swift core carry the Zenoh
    // binding, and the Kotlin one does not. A Kotlin peer table naming it would
    // have no `PeerLink` to read, so it is refused by name, and the repair
    // points at every backend that has it, read from the registry.
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
            && message.contains("`--lang rust` or `--lang swift`"),
        "record: {rec}"
    );
}

/// A deploy.yaml whose one binding, `#server`, is the route of a causal group
/// (docs/adr/0085, section 3.2) holding the client and the server.
const CAUSAL_DEPLOY: &str = r##"
version: "1.0"
topology:
  phone:
    machines:
      client:
        source: client.scxml
        bindings:
          "#server":
            transport: wss
            url: "wss://cal.example"
            delivery: causal
            group: { name: people, view: [phone/client, cloud/server] }
  cloud:
    machines:
      server: { source: server.scxml }
"##;

/// What a host core's peer table writes for the causal binding of
/// [`CAUSAL_DEPLOY`]: the file it is in, the lines that carry the group and
/// its view of epoch 0, the duplicate window the causal route replaces, and
/// what the same binding without `delivery: causal` carries instead.
struct HostCausalTable {
    language: &'static str,
    file_suffix: &'static str,
    group: &'static [&'static str],
    dedup_off: &'static str,
    plain: &'static str,
}

/// Every host core the Mesh codegen has an emitter for (`mesh_backend` in
/// `sce-build/src/mesh/codegen.rs`). The C++ router, the fourth emitter, is
/// held to the same by `the_cpp_router_routes_a_causal_binding_over_custom_tcp`,
/// over a transport it serves.
const HOST_CAUSAL_TABLES: &[HostCausalTable] = &[
    HostCausalTable {
        language: "rust",
        file_suffix: "_mesh_peers.rs",
        group: &[
            "causal: Some(sce_rust_mesh::causal::CausalGroup {",
            "name: \"people\",",
            "epoch: 0,",
            "view: &[\"phone/client\", \"cloud/server\"],",
        ],
        dedup_off: "dedup: false,",
        plain: "causal: None,",
    },
    HostCausalTable {
        language: "kotlin",
        file_suffix: "MeshPeers.kt",
        group: &[
            "causal = com.sce.mesh.CausalGroup(",
            "name = \"people\",",
            "epoch = 0uL,",
            "view = listOf(\"phone/client\", \"cloud/server\"),",
        ],
        dedup_off: "Delivery(dedup = false,",
        plain: "causal = null,",
    },
    HostCausalTable {
        language: "swift",
        file_suffix: "MeshPeers.swift",
        group: &[
            "causal: CausalGroup(",
            "name: \"people\",",
            "epoch: 0,",
            "view: [\"phone/client\", \"cloud/server\"]",
        ],
        dedup_off: "Delivery(dedup: false,",
        plain: "causal: nil",
    },
];

#[test]
fn every_host_core_peer_table_carries_a_causal_bindings_group() {
    // docs/adr/0085, section 3.2: each host core's router routes a causal
    // binding to its causal layer, so its peer table carries the group and
    // its view of epoch 0 with the duplicate window off, and a binding that
    // is not causal carries none. Red if a backend's build refuses the
    // binding, if the peer policy ignores `delivery:`, or if a backend's
    // template drops the group.
    for table in HOST_CAUSAL_TABLES {
        let language = table.language;
        let dir = ScratchDir::new(&format!("mesh-causal-{language}"));
        let scxml = dir.write("client.scxml", CLIENT_SCXML);
        dir.write("server.scxml", SERVER_SCXML);
        let deploy = dir.write("deploy.yaml", CAUSAL_DEPLOY);
        let out = run_with_deploy_for(language, &scxml, &deploy, dir.path());
        assert!(
            out.status.success(),
            "the {language} core routes a causal binding; stderr:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let peers = generated_file_under(dir.path(), table.file_suffix);
        for expected in table.group {
            assert!(
                peers.contains(expected),
                "{language}: missing `{expected}`:\n{peers}"
            );
        }
        assert!(
            peers.contains(table.dedup_off),
            "{language}: the causal route replaces the duplicate window:\n{peers}"
        );

        let plain = ScratchDir::new(&format!("mesh-causal-{language}-plain"));
        let scxml = plain.write("client.scxml", CLIENT_SCXML);
        plain.write("server.scxml", SERVER_SCXML);
        let deploy = plain.write("deploy.yaml", &plain_of(CAUSAL_DEPLOY));
        let out = run_with_deploy_for(language, &scxml, &deploy, plain.path());
        assert!(
            out.status.success(),
            "{language} stderr:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let peers = generated_file_under(plain.path(), table.file_suffix);
        assert!(
            peers.contains(table.plain) && !peers.contains(table.group[0]),
            "{language}: a plain binding carries no group:\n{peers}"
        );
    }
}

/// `deploy` with the causal binding's `delivery:` and `group:` lines
/// removed, so the same binding is a plain route.
fn plain_of(deploy: &str) -> String {
    deploy
        .replace("            delivery: causal\n", "")
        .replace(
            "            group: { name: people, view: [phone/client, cloud/server] }\n",
            "",
        )
}

/// [`CAUSAL_DEPLOY`]'s group over `binding`, the client's one binding to
/// `#server`, with a custom_tcp listener on the server's device.
fn causal_deploy_over(binding: &str) -> String {
    format!(
        r##"
version: "1.0"
topology:
  phone:
    machines:
      client:
        source: client.scxml
        bindings:
          "#server":
            {binding}
  cloud:
    transports:
      custom_tcp:
        listen: "127.0.0.1:55999"
    machines:
      server: {{ source: server.scxml }}
"##
    )
}

/// The causal group of [`causal_deploy_over`], as the binding's own keys.
const CAUSAL_GROUP_KEYS: &str = "delivery: causal\n            \
     group: { name: people, view: [phone/client, cloud/server] }";

#[test]
fn the_cpp_router_routes_a_causal_binding_over_custom_tcp() {
    // docs/adr/0085, section 3.2: the generated C++ router holds a causal
    // route for the binding, with its group and view of epoch 0, offers the
    // binding's sends there and takes the peer's envelopes there before
    // dispatch; a binding that is not causal holds none. Red if the C++
    // emitter drops the binding's group, or if the template ignores it.
    let dir = ScratchDir::new("mesh-causal-cpp");
    let scxml = dir.write("client.scxml", CLIENT_SCXML);
    dir.write("server.scxml", SERVER_SCXML);
    let binding =
        format!("transport: custom_tcp\n            connect: \"127.0.0.1:55999\"\n            {CAUSAL_GROUP_KEYS}");
    let deploy = dir.write("deploy.yaml", &causal_deploy_over(&binding));
    let out = run_with_deploy_for("cpp", &scxml, &deploy, dir.path());
    assert!(
        out.status.success(),
        "the C++ core routes a causal binding over custom_tcp; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let router = generated_file_under(dir.path(), "_transport.h");
    for expected in [
        "#include \"mesh/CausalRoute.h\"",
        "SCE::Mesh::CausalRoute causal_server_{",
        // As clang-format 19 writes it: every generated C++ file passes
        // through the pinned formatter.
        "SCE::Mesh::CausalGroup{\"people\", 0u, {\"phone/client\", \"cloud/server\"}}",
        "return offerCausal(\"#server\", causal_server_,",
        "if (env.source == \"server\") {",
        "if (takeCausal(env, session_idx)) {",
        "bool startCausal(",
        "bool causalCommitted(",
        "void causalTick(",
    ] {
        assert!(router.contains(expected), "missing `{expected}`:\n{router}");
    }

    let plain = ScratchDir::new("mesh-causal-cpp-plain");
    let scxml = plain.write("client.scxml", CLIENT_SCXML);
    plain.write("server.scxml", SERVER_SCXML);
    let deploy = plain.write(
        "deploy.yaml",
        &causal_deploy_over("transport: custom_tcp\n            connect: \"127.0.0.1:55999\""),
    );
    let out = run_with_deploy_for("cpp", &scxml, &deploy, plain.path());
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !generated_file_under(plain.path(), "_transport.h").contains("CausalRoute"),
        "a plain binding holds no causal route"
    );
}

/// The file `sce-codegen generate --deploy` wrote under `dir` whose name ends
/// in `suffix`, wherever the generator put it: `_transport.h` for the C++
/// router, `_mesh_peers.rs`, `MeshPeers.kt` and `MeshPeers.swift` for the
/// Rust, Kotlin and Swift peer tables.
fn generated_file_under(dir: &std::path::Path, suffix: &str) -> String {
    fn find(dir: &std::path::Path, suffix: &str) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find(&path, suffix) {
                    return Some(found);
                }
            } else if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(suffix))
            {
                return Some(path);
            }
        }
        None
    }
    let path = find(dir, suffix)
        .unwrap_or_else(|| panic!("no generated file ending in `{suffix}` under {dir:?}"));
    std::fs::read_to_string(path).expect("read the generated file")
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
        server: { transport: zenoh_keyexpr, key: "cal/client" }
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

/// The request the rooms server serves: a room's name and who asks.
const JOIN_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="join" sce:kind="event-schema" sce:event-name="service.request.join">
  <datamodel>
    <data id="room" sce:type="string[16]" sce:direction="in"/>
    <data id="who" sce:type="string[8]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A server that answers each client's `service.request.join`.
const ROOMS_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="rooms" initial="open">
  <sce:import kind="event-schema" src="join.scxml" as="Join"/>
  <state id="open">
    <transition event="service.request.join" target="open">
      <send target="#guest" event="service.response.join"/>
    </transition>
  </state>
</scxml>
"##;

/// The client of [`ROOMS_SCXML`], which takes its answer.
const GUEST_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="guest" initial="idle">
  <state id="idle">
    <transition event="service.response.join" target="idle"/>
  </state>
</scxml>
"##;

/// The rooms server's deployment, its machine served as a pool by
/// `sessions`.
fn rooms_deploy(sessions: &str) -> String {
    format!(
        r##"
version: "1.0"
topology:
  cloud:
    machines:
      rooms:
        source: rooms.scxml
        sessions: {sessions}
        bindings:
          "#guest": {{ transport: wss }}
  phone:
    machines:
      guest:
        source: guest.scxml
        bindings:
          "#rooms": {{ transport: wss, url: "wss://rooms.example" }}
"##
    )
}

/// Generate the rooms server for `language` with `join` as its request's
/// schema and `sessions` as its pool, in a scratch directory that holds every
/// document the deployment names.
fn generate_rooms(
    label: &str,
    language: &str,
    join: &str,
    sessions: &str,
) -> (std::process::Output, ScratchDir) {
    let dir = ScratchDir::new(label);
    dir.write("join.scxml", join);
    dir.write("guest.scxml", GUEST_SCXML);
    let scxml = dir.write("rooms.scxml", ROOMS_SCXML);
    let deploy = dir.write("deploy.yaml", &rooms_deploy(sessions));
    let out = run_with_deploy_for(language, &scxml, &deploy, dir.path());
    (out, dir)
}

#[test]
fn a_keyed_pool_reaches_the_rust_peer_table_and_no_other_core() {
    // SCE_MESH.md §14.4, "Keyed session pool": the Rust host core serves a
    // machine as a pool of sessions, so its peer table carries where the key
    // is read and how long it may be, read from the request's schema. Red if
    // the build drops `sessions:`, or reads the bound from anywhere but the
    // key field's type.
    let (out, dir) = generate_rooms(
        "mesh-keyed-pool-rust",
        "rust",
        JOIN_SCHEMA,
        "{ key_from: data.room, idle_ms: 30000 }",
    );
    assert!(
        out.status.success(),
        "the Rust core serves a keyed pool; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let peers = generated_file_under(dir.path(), "_mesh_peers.rs");
    for expected in [
        "sessions: Some(sce_rust_mesh::peers::Sessions {",
        "key_from: sce_rust_mesh::peers::KeyFrom::Field(\"room\"),",
        "max_key_bytes: 16,",
        "idle_ms: 30000,",
        // docs/adr/0109, D1: the bounds the section leaves out, at their defaults.
        "max_live: 100000,",
        "max_inbox: 64,",
        // docs/adr/0109, D15: a commit waits for nothing unless the section says so.
        "commit_delay_us: 0,",
    ] {
        assert!(peers.contains(expected), "missing `{expected}`:\n{peers}");
    }

    // A pool keyed by the client's name reads no field.
    let (out, dir) = generate_rooms(
        "mesh-keyed-pool-client",
        "rust",
        JOIN_SCHEMA,
        "{ key_from: client, idle_ms: 30000 }",
    );
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let peers = generated_file_under(dir.path(), "_mesh_peers.rs");
    assert!(
        peers.contains("key_from: sce_rust_mesh::peers::KeyFrom::Client,")
            && peers.contains("max_key_bytes: 256,"),
        "{peers}"
    );

    // Every other core refuses it rather than serve every key from one state.
    for language in ["cpp", "kotlin", "swift"] {
        let (out, _dir) = generate_rooms(
            &format!("mesh-keyed-pool-{language}"),
            language,
            JOIN_SCHEMA,
            "{ key_from: data.room, idle_ms: 30000 }",
        );
        assert!(!out.status.success(), "{language} must refuse a keyed pool");
        assert_eq!(out.status.code(), Some(12), "{language}: a CodegenError");
        let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
        let rec = sole_ndjson_record(&stderr);
        assert_core_shape(&rec);
        assert_eq!(
            rec["code"], "mesh/codegen-keyed-pool-unsupported",
            "record: {rec}"
        );
        assert_eq!(rec["stage"], "mesh-codegen", "record: {rec}");
        assert_eq!(rec["spec"], "SCE Mesh §14.4", "record: {rec}");
        assert_eq!(rec["actual"], "rooms", "record: {rec}");
        assert!(
            rec["message"]
                .as_str()
                .is_some_and(|m| m.contains(&format!("the {language} Mesh core"))),
            "record: {rec}"
        );
    }
}

#[test]
fn a_keyed_pool_whose_key_no_request_carries_is_refused() {
    // The key field is judged against the request's schema when the machine
    // is generated: a request that does not carry it names no session.
    let without_room = JOIN_SCHEMA.replace(
        "    <data id=\"room\" sce:type=\"string[16]\" sce:direction=\"in\"/>\n",
        "",
    );
    let (out, _dir) = generate_rooms(
        "mesh-keyed-pool-no-field",
        "rust",
        &without_room,
        "{ key_from: data.room, idle_ms: 30000 }",
    );
    assert!(!out.status.success(), "a key no request carries is refused");
    assert_eq!(out.status.code(), Some(10), "a DeployError");
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/deploy-sessions-key-not-in-request",
        "record: {rec}"
    );
    assert_eq!(rec["stage"], "mesh-deploy", "record: {rec}");
    assert_eq!(rec["actual"], "room", "record: {rec}");
    assert!(
        rec["message"]
            .as_str()
            .is_some_and(|m| m.contains("'service.request.join' declares no such field")),
        "record: {rec}"
    );
}

#[test]
fn a_keyed_pool_section_is_checked_when_the_deployment_is_read() {
    let (out, _dir) = generate_rooms(
        "mesh-keyed-pool-invalid",
        "rust",
        JOIN_SCHEMA,
        "{ key_from: room, idle_ms: 30000 }",
    );
    assert!(!out.status.success(), "an unreadable key source is refused");
    assert_eq!(out.status.code(), Some(10), "a DeployError");
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(rec["code"], "mesh/deploy-invalid-sessions", "record: {rec}");
    assert_eq!(rec["stage"], "mesh-deploy", "record: {rec}");
    assert_eq!(rec["spec"], "SCE Mesh §14.4", "record: {rec}");
    assert_eq!(rec["actual"], "rooms", "record: {rec}");
}

/// A server that answers each `ping` at the origin it carried, as a document answers an instance of
/// an open machine (docs/adr/0113, D2).
const ANSWERING_SERVER_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="server" datamodel="ecmascript" initial="idle">
  <state id="idle">
    <transition event="ping">
      <send targetexpr="_event.origin" event="pong"/>
    </transition>
  </state>
</scxml>
"##;

/// A server that takes its client's requests and sends nothing, so it resolves no target.
const TAKING_SERVER_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="server" initial="idle">
  <state id="idle">
    <transition event="ping" target="idle"/>
  </state>
</scxml>
"##;

/// [`HTTPS_DEPLOY`] whose client runs as an open set of instances (docs/adr/0113, D1).
fn open_https_deploy() -> String {
    HTTPS_DEPLOY.replace(
        "        source: client.scxml\n",
        "        source: client.scxml\n        instances: open\n        members: { lease_ms: 2592000000, provisional_ms: 60000, max_instances: 200 }\n",
    )
}

/// A directory that holds the open deployment, its client and `server` as its server.
fn open_deployment(label: &str, server: &str) -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new(label);
    dir.write("client.scxml", ANSWERED_CLIENT_SCXML);
    dir.write("server.scxml", server);
    let deploy = dir.write("deploy.yaml", &open_https_deploy());
    (dir, deploy)
}

/// docs/adr/0113, D1 and D3: the open machine's own table says it is open and binds its server by
/// name, and its server's table holds it beside its peers and not among them, under one
/// configuration whose responder is the instance itself. Red if the template drops either field,
/// or the open machine is bound as a peer.
#[test]
fn an_open_machine_reaches_the_rust_peer_tables_of_both_sides() {
    let (dir, deploy) = open_deployment("mesh-open-client", ANSWERING_SERVER_SCXML);
    let out = run_with_deploy_for(
        "rust",
        &dir.path().join("client.scxml"),
        &deploy,
        dir.path(),
    );
    assert!(
        out.status.success(),
        "the Rust host core serves an open machine; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let table = generated_file_under(dir.path(), "_mesh_peers.rs");
    for line in [
        "instances: sce_rust_mesh::peers::Instances::Open,",
        "responders: sce_rust_mesh::router::Responders::Named(&[\"server\"]),",
        "link: PeerLink::HttpsDial {",
    ] {
        assert!(table.contains(line), "missing `{line}`:\n{table}");
    }
    assert!(!table.contains("Responders::Instance"), "{table}");

    let (dir, deploy) = open_deployment("mesh-open-server", ANSWERING_SERVER_SCXML);
    let out = run_with_deploy_for(
        "rust",
        &dir.path().join("server.scxml"),
        &deploy,
        dir.path(),
    );
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let table = generated_file_under(dir.path(), "_mesh_peers.rs");
    let (peers, open) = table
        .split_once("open_peers: &[")
        .expect("the open machines are a list of their own");
    assert!(!peers.contains("name: \"client\""), "no peer by its name");
    for line in [
        "name: \"client\",",
        "lease_ms: 2592000000,",
        "provisional_ms: 60000,",
        "max_instances: 200,",
        "responders: sce_rust_mesh::router::Responders::Instance,",
        "link: PeerLink::HttpsAccept {",
        "instances: sce_rust_mesh::peers::Instances::One,",
    ] {
        assert!(open.contains(line), "missing `{line}`:\n{table}");
    }
}

/// docs/adr/0113, D7: each backend but Rust refuses a machine of a deployment that declares an
/// open one, by the open machine's name, whichever machine of it is generated. Red if a backend is
/// added to the ones that serve an open machine, or the refusal is asked of the open machine alone.
#[test]
fn every_backend_but_rust_refuses_a_deployment_with_an_open_machine() {
    use sce_build::generator::Language;
    use sce_build::mesh::error::{CodegenError, MeshError};

    let (_dir, deploy) = open_deployment("mesh-open-refused", TAKING_SERVER_SCXML);
    for (name, document) in [
        ("client", ANSWERED_CLIENT_SCXML),
        ("server", TAKING_SERVER_SCXML),
    ] {
        for language in Language::ALL {
            let mut model = sce_build::parser::SCXMLParser::new()
                .parse_string(document, name)
                .expect("parse");
            let compiled = sce_build::compile_mesh_transport(&mut model, &deploy, *language);
            let serves_open_instances = match language {
                Language::Rust => true,
                Language::Cpp
                | Language::Kotlin
                | Language::Go
                | Language::Python
                | Language::C11
                | Language::Swift => false,
            };
            match (serves_open_instances, compiled) {
                (true, Ok(_)) => {}
                (true, Err(e)) => panic!("{name}: {language:?} serves an open machine: {e}"),
                (
                    false,
                    Err(MeshError::Codegen(CodegenError::OpenInstancesUnsupported {
                        machine,
                        backend,
                    })),
                ) => {
                    assert_eq!(machine, "client", "{name}: the open machine is named");
                    assert_eq!(backend, language.canonical_name());
                }
                (false, Err(e)) => panic!("{name}: {language:?} refused by another: {e}"),
                (false, Ok(_)) => panic!("{name}: {language:?} took an open machine"),
            }
        }
    }
}

#[test]
fn a_backend_that_serves_no_open_machine_refuses_it_by_its_code() {
    let (dir, deploy) = open_deployment("mesh-open-cpp", TAKING_SERVER_SCXML);
    let out = run_with_deploy_for("cpp", &dir.path().join("client.scxml"), &deploy, dir.path());
    assert!(
        !out.status.success(),
        "the C++ router serves no open machine"
    );
    assert_eq!(out.status.code(), Some(12), "a CodegenError");
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = sole_ndjson_record(&stderr);
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/codegen-open-instances-unsupported",
        "record: {rec}"
    );
    assert_eq!(rec["stage"], "mesh-codegen", "record: {rec}");
    assert_eq!(rec["spec"], "ADR 0113 D7", "record: {rec}");
    assert_eq!(rec["actual"], "client", "record: {rec}");
}

/// docs/adr/0113, D2: a literal address of an open machine, in a send at any depth and in a
/// request, is refused when the machine is generated, naming the origin as the repair; a run-time
/// target is not. Red if a nested send is not read, or the machine's own binding to the name lets
/// the literal through.
#[test]
fn a_literal_address_of_an_open_machine_is_refused() {
    for (label, body, written) in [
        (
            "send",
            r##"<send target="#client" event="pong"/>"##,
            r##"<send target="#client">"##,
        ),
        (
            "nested-send",
            r##"<if cond="true"><send target="#client" event="pong"/></if>"##,
            r##"<send target="#client">"##,
        ),
    ] {
        let server = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="server" datamodel="ecmascript" initial="idle">
  <state id="idle">
    <transition event="ping">{body}</transition>
  </state>
</scxml>
"##
        );
        refused_as_named_literally(
            &format!("mesh-open-literal-{label}"),
            &server,
            "idle",
            written,
        );
    }
    let asking = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="server" datamodel="ecmascript" initial="asking">
  <state id="asking">
    <invoke type="sce:mesh-rpc" src="#client">
      <param name="_mesh_event" expr="'service.request.look'"/>
    </invoke>
    <transition event="ping" target="asking"/>
  </state>
</scxml>
"##;
    refused_as_named_literally(
        "mesh-open-literal-request",
        asking,
        "asking",
        r##"<invoke type="sce:mesh-rpc" src="#client">"##,
    );
}

/// The server `document` of the open deployment is refused for the address it `written`, in the
/// state `state`.
fn refused_as_named_literally(label: &str, document: &str, state: &str, written: &str) {
    use sce_build::mesh::error::{MeshError, TopologyError};

    let (dir, deploy) = open_deployment(label, document);
    let mut model = sce_build::parser::SCXMLParser::new()
        .parse_string(document, "server")
        .expect("parse");
    let language = sce_build::generator::Language::Rust;
    match sce_build::compile_mesh_transport(&mut model, &deploy, language) {
        Err(MeshError::Topology(refusal)) => match *refusal {
            TopologyError::OpenMachineNamedLiterally {
                machine,
                state: at,
                target,
                written: address,
            } => {
                assert_eq!(machine, "server");
                assert_eq!(at, state);
                assert_eq!(target.as_str(), "#client");
                assert_eq!(address, written);
            }
            other => panic!("{label}: refused by another: {other}"),
        },
        Err(e) => panic!("{label}: refused by another: {e}"),
        Ok(_) => panic!("{label}: a literal address of an open machine was taken"),
    }

    let out = run_with_deploy(&dir.path().join("server.scxml"), &deploy, dir.path());
    assert!(!out.status.success(), "{label}");
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let rec = stderr
        .lines()
        .rfind(|line| line.starts_with('{'))
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("valid JSON"))
        .unwrap_or_else(|| panic!("{label}: no record on stderr:\n{stderr}"));
    assert_core_shape(&rec);
    assert_eq!(
        rec["code"], "mesh/topology-open-machine-named-literally",
        "record: {rec}"
    );
    assert_eq!(rec["stage"], "mesh-topology", "record: {rec}");
    assert_eq!(rec["spec"], "ADR 0113 D2", "record: {rec}");
    assert_eq!(rec["actual"], "#client", "record: {rec}");
    assert!(
        rec["message"]
            .as_str()
            .is_some_and(|m| m.contains("<send targetexpr=\"_event.origin\">")),
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
