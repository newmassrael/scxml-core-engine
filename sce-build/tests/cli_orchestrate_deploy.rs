//! End-to-end CLI contract test for `sce-codegen orchestrate --deploy`.
//!
//! Closes the textbook end-user surface for deploy-aware
//! orchestration: the
//! orchestrator wiring `b501b18c` exposed `Option<&DeployConfig>` on
//! the library entry point, but the CLI multi-doc subcommand
//! (`Commands::Orchestrate`) had no flag to pass it. The
//! `--deploy=PATH` flag lets end-users invoking the binary fire
//! SCE Protocol-Synthesis RFC §synth-5-K + §synth-5-M cross-doc validators without
//! library-level wrappers.
//!
//! Test matrix (mirrors the library-level `c13_*` tests in
//! `scxml_cross_doc_orchestrator.rs` but at the CLI process boundary):
//!   1. Without `--deploy`, behavior unchanged — orchestrator
//!      silent-skips the deploy cross-doc validators (shared
//!      silent-skip discipline).
//!   2. With `--deploy` that triggers `deploy/link-not-declared-in-deploy`,
//!      the CLI exits non-zero and emits the diagnostic to stderr.
//!   3. Deploy YAML parse failure routes through the same NDJSON wire
//!      contract — `mesh/deploy-*` codes carry stage=mesh-deploy.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

struct ScratchDir(PathBuf);
impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let dir = root.join(format!("{label}-{}-{id}", std::process::id()));
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

fn write_doc(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).expect("write fixture");
    p
}

/// Minimal SCXML statechart for orchestrator codegen — no on-sample /
/// outbox refs so the only cross-doc surface the test exercises is
/// the C13 (deploy-vs-forge) path.
fn statechart_minimal(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       name="{name}"
       version="1.0"
       initial="idle"
       datamodel="ecmascript">
  <state id="idle"/>
</scxml>"##
    )
}

/// Forge `<sce:link>` referencing a `<sce:rx-pool>` so the cross-doc
/// reassembly/burst validators have a 3-way join to follow when deploy
/// is supplied.
fn link_with_rx_pool(name: &str, rx_pool: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="{name}" version="1.0">
  <sce:import as="scout_frame_codec" src="scout_frame_codec.scxml" kind="codec"/>
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:rx-pool ref="{rx_pool}"/>
</scxml>"##
    )
}

/// The codec [`link_with_rx_pool`]'s `<sce:framer ref>` names.
fn framer_codec(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="little" name="{name}">
  <datamodel>
    <data id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <data id="payload_len" sce:type="uint16" sce:byte="1" sce:bit-size="16"/>
  </datamodel>
</scxml>"##
    )
}

/// Stage a link together with the codec its `<sce:framer ref>` names,
/// returning the link path — the only one a caller passes on the
/// command line.
///
/// The codec reaches the build through the link's own `<sce:import>`
/// rather than a second `--forge` argument, so the CLI invocations
/// these tests pin stay the invocations under test.
fn write_link_with_framer(dir: &Path, name: &str, rx_pool: &str) -> PathBuf {
    write_doc(
        dir,
        "scout_frame_codec.scxml",
        &framer_codec("scout_frame_codec"),
    );
    write_doc(
        dir,
        &format!("{name}.scxml"),
        &link_with_rx_pool(name, rx_pool),
    )
}

/// Slot sizes passed here must be whole multiples of the 32-byte
/// alignment below (`mem/slot-size-not-alignment-multiple`), which is
/// why the MTU-sized fixtures use 1536 rather than 1500: a pool
/// carrying Ethernet frames has to round the payload up to the DMA
/// boundary, and 1536 is what a real MTU pool declares.
fn buffer_pool(name: &str, slot_count: u32, slot_size: u32) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="{name}" version="1.0">
  <sce:slot-count>{slot_count}</sce:slot-count>
  <sce:slot-size>{slot_size}</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##
    )
}

/// Deploy fixture with a single MCU machine; per-test `links_yaml`
/// plugs into the `links:` body.
fn deploy_yaml(links_yaml: &str) -> String {
    format!(
        r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: session_fsm.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
          has_speculative_prefetch: false
          core_count: 1
          clock_freq_mhz: 400
          memcpy_cycles_per_byte: 1.0
        scheduler:
          kind: cooperative
          tick_period_us: 1000
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
        memory:
          sram_regions:
            sram1: {{ base: 0x08000000, size: 65536, attr: [dma_coherent, cacheable] }}
          dma_channels: [DW0_CH0]
        links:
{links_yaml}
"#,
    )
}

fn run_orchestrate(
    bin: &Path,
    scxml: &Path,
    forges: &[&Path],
    output_dir: &Path,
    deploy: Option<&Path>,
    error_format: &str,
) -> std::process::Output {
    let mut cmd = Command::new(bin);
    cmd.arg("--error-format")
        .arg(error_format)
        .arg("orchestrate");
    cmd.arg("--scxml").arg(scxml);
    for f in forges {
        cmd.arg("--forge").arg(f);
    }
    cmd.arg("--language").arg("rust");
    cmd.arg("--output-dir").arg(output_dir);
    if let Some(d) = deploy {
        cmd.arg("--deploy").arg(d);
    }
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen orchestrate")
}

#[test]
fn orchestrate_without_deploy_silent_skips_c13_validators() {
    // Forge declares `udp_data`; a deploy.yaml WITHOUT `udp_data`
    // would fire `deploy/link-not-declared-in-deploy`. Omitting
    // --deploy entirely must silent-skip per the absent-input rule;
    // the CLI exits 0 and emits codegen output.
    let dir = ScratchDir::new("cli-orch-no-deploy");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    let pool = write_doc(
        dir.path(),
        "rx_pool.scxml",
        &buffer_pool("rx_pool", 16, 1536),
    );
    let link = write_link_with_framer(dir.path(), "udp_data", "rx_pool");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("mkdir out");

    let out = run_orchestrate(
        &sce_codegen_bin(),
        &scxml,
        &[&pool, &link],
        &out_dir,
        None,
        "json",
    );

    assert!(
        out.status.success(),
        "orchestrate without --deploy must succeed (stderr: {})",
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn orchestrate_with_deploy_fires_link_not_declared_in_deploy() {
    // Forge has `udp_data`; deploy.yaml has `udp_scout` only. Pass A
    // (forge → deploy) fires `deploy/link-not-declared-in-deploy`
    // through `validate_links_cross_doc`. CLI exits non-zero with NDJSON
    // diagnostic on stderr.
    let dir = ScratchDir::new("cli-orch-not-declared");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    let pool = write_doc(
        dir.path(),
        "rx_pool.scxml",
        &buffer_pool("rx_pool", 16, 1536),
    );
    let link = write_link_with_framer(dir.path(), "udp_data", "rx_pool");
    let deploy = write_doc(
        dir.path(),
        "deploy.yaml",
        &deploy_yaml(
            r#"          udp_scout:
            bind: "224.0.0.224:7446"
            driver: lwip_udp
"#,
        ),
    );
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("mkdir out");

    let out = run_orchestrate(
        &sce_codegen_bin(),
        &scxml,
        &[&pool, &link],
        &out_dir,
        Some(&deploy),
        "json",
    );

    assert!(
        !out.status.success(),
        "deploy missing udp_data must fail (stdout: {})",
        String::from_utf8_lossy(&out.stdout),
    );

    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let mut found_target_code = false;
    for line in stderr.trim_end().lines() {
        assert!(
            line.starts_with('{') && line.ends_with('}'),
            "NDJSON wire shape required: {line}",
        );
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("not JSON ({e}): {line}"));
        let code = parsed.get("code").and_then(|v| v.as_str()).unwrap_or("");
        if code == "deploy/link-not-declared-in-deploy" {
            found_target_code = true;
            assert_eq!(
                parsed.get("stage").and_then(|v| v.as_str()).unwrap_or(""),
                "mesh-deploy",
                "deploy-side diagnostic must carry stage=mesh-deploy",
            );
        }
    }
    assert!(
        found_target_code,
        "CLI must emit deploy/link-not-declared-in-deploy diagnostic; stderr was: {stderr}",
    );
}

#[test]
fn orchestrate_with_malformed_deploy_yaml_fails_cleanly() {
    // A deploy.yaml that fails to parse must route through the same
    // NDJSON wire format the C13 validators use — `mesh/deploy-*`
    // codes from DeployError. The exit code matches `MeshError`'s
    // categorical mapping (delegated via `ForgeError::Mesh`).
    let dir = ScratchDir::new("cli-orch-bad-deploy");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    let deploy = write_doc(
        dir.path(),
        "deploy.yaml",
        "this is not valid yaml :: : ::\n  - garbage\n",
    );
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("mkdir out");

    let out = run_orchestrate(
        &sce_codegen_bin(),
        &scxml,
        &[],
        &out_dir,
        Some(&deploy),
        "json",
    );

    assert!(
        !out.status.success(),
        "malformed deploy.yaml must fail (stdout: {})",
        String::from_utf8_lossy(&out.stdout),
    );

    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let trimmed = stderr.trim_end();
    assert!(!trimmed.is_empty(), "malformed deploy must emit diagnostic",);
    // The deploy parser produces `mesh/deploy-yaml-parse` (or similar
    // stable code). Pin the stage so a regression to "io" or generic
    // failure surfaces.
    let mut saw_mesh_deploy_stage = false;
    for line in trimmed.lines() {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("not JSON ({e}): {line}"));
        if parsed.get("stage").and_then(|v| v.as_str()) == Some("mesh-deploy") {
            saw_mesh_deploy_stage = true;
        }
    }
    assert!(
        saw_mesh_deploy_stage,
        "malformed deploy must carry stage=mesh-deploy; stderr: {stderr}",
    );
}

// ── §10 stdout manifest ──────────────────────────────────────────
//
// `orchestrate` materialises a whole build and used to say nothing on
// stdout. `generate` and `check` each emit one manifest line, and
// `check`'s document-set route mirrors this subcommand's verdict — but
// its `artifacts` is `[]` by contract, so no record on the wire named
// the files a multi-doc build had just written. A consumer driving the
// multi-doc entry point had to guess the output layout or re-walk the
// directory, which cannot distinguish this run's artifacts from what
// was already there.

/// Repo root, for reading the schema the consumers compile against.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Compile the manifest wire schema once per call site.
fn manifest_validator() -> jsonschema::JSONSchema {
    let schema_value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("schemas/sce-manifest.v1.schema.json"))
            .expect("read manifest schema"),
    )
    .expect("manifest schema is JSON");
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_value)
        .expect("manifest schema compiles as draft-07")
}

/// The manifest names every file the run wrote, and nothing else.
///
/// Asserted as set equality against a walk of the output directory
/// rather than as a count: a manifest listing four paths while the run
/// wrote four different ones would satisfy any weaker check. The
/// directory starts empty, so everything in it came from this run.
#[test]
fn orchestrate_manifest_names_exactly_the_files_it_wrote() {
    let bin = sce_codegen_bin();
    let staged = ScratchDir::new("orch-manifest-in");
    let out = ScratchDir::new("orch-manifest-out");

    let scxml = write_doc(
        staged.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    let pool = write_doc(
        staged.path(),
        "rx_data_pool.scxml",
        &buffer_pool("rx_data_pool", 2000, 1536),
    );
    let link = write_link_with_framer(staged.path(), "udp_data", "rx_data_pool");
    let deploy = write_doc(
        staged.path(),
        "deploy.yaml",
        &deploy_yaml(
            r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
"#,
        ),
    );

    let output = run_orchestrate(
        &bin,
        &scxml,
        &[pool.as_path(), link.as_path()],
        out.path(),
        Some(deploy.as_path()),
        "json",
    );
    assert!(
        output.status.success(),
        "well-formed set must orchestrate cleanly; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert_eq!(
        stdout.lines().count(),
        1,
        "the manifest is exactly one line (§10.2): {stdout}",
    );
    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("orchestrate emits one JSON manifest");
    assert_eq!(
        manifest["kind"], "orchestrate",
        "the kind names the producing subcommand: {stdout}",
    );

    let msgs: Vec<String> = match manifest_validator().validate(&manifest) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    };
    assert!(
        msgs.is_empty(),
        "orchestrate manifest violates the wire schema: {msgs:?}\n{stdout}",
    );

    let declared: std::collections::BTreeSet<String> = manifest["artifacts"]
        .as_array()
        .expect("artifacts is an array")
        .iter()
        .map(|a| a["path"].as_str().expect("path is a string").to_string())
        .collect();
    let on_disk: std::collections::BTreeSet<String> = std::fs::read_dir(out.path())
        .expect("read output dir")
        .map(|e| e.expect("dir entry").path().display().to_string())
        .collect();

    assert!(
        !on_disk.is_empty(),
        "the control is vacuous unless the run wrote something",
    );
    assert_eq!(
        declared, on_disk,
        "§10.1: artifacts must be every file written, and only those",
    );
}

/// A refused run writes no manifest.
///
/// §10.2 makes stdout meaningful only on exit 0, so a consumer reading
/// stdout before checking the status must find nothing to parse.
#[test]
fn orchestrate_emits_no_manifest_when_it_refuses() {
    let bin = sce_codegen_bin();
    let staged = ScratchDir::new("orch-manifest-refuse-in");
    let out = ScratchDir::new("orch-manifest-refuse-out");

    let scxml = write_doc(
        staged.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    let pool = write_doc(
        staged.path(),
        "rx_data_pool.scxml",
        &buffer_pool("rx_data_pool", 2000, 1536),
    );
    // Deploy names a link the forge set does not declare.
    let link = write_link_with_framer(staged.path(), "udp_data", "rx_data_pool");
    let deploy = write_doc(
        staged.path(),
        "deploy.yaml",
        &deploy_yaml(
            r#"          udp_scout:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
"#,
        ),
    );

    let output = run_orchestrate(
        &bin,
        &scxml,
        &[pool.as_path(), link.as_path()],
        out.path(),
        Some(deploy.as_path()),
        "json",
    );
    assert!(
        !output.status.success(),
        "a link declared in forge but absent from deploy must be refused",
    );
    assert!(
        output.stdout.is_empty(),
        "§10.2: a failing run leaves stdout empty, got {:?}",
        String::from_utf8_lossy(&output.stdout),
    );
}

/// `needs_script_engine` answers for the **set**, not for one document.
///
/// A build links the engine once, so the question a multi-doc consumer
/// asks is whether *any* input needs it. Driven with the engine-needing
/// document second so a producer that reported only the first input
/// would answer `false` here.
#[test]
fn orchestrate_manifest_reports_the_script_engine_union() {
    let bin = sce_codegen_bin();
    let staged = ScratchDir::new("orch-manifest-union-in");

    let plain = write_doc(
        staged.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    // A `cond` the static lowering cannot fold needs the engine.
    let scripted = write_doc(
        staged.path(),
        "scripted_fsm.scxml",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       name="scripted_fsm"
       version="1.0"
       initial="idle"
       datamodel="ecmascript">
  <datamodel><data id="retry" expr="0"/></datamodel>
  <state id="idle">
    <transition event="go" cond="retry &lt; 3" target="idle"/>
  </state>
</scxml>"##,
    );

    let read_flag = |first: &Path, second: &Path| -> bool {
        let out = ScratchDir::new("orch-manifest-union-out");
        let mut cmd = Command::new(&bin);
        cmd.arg("orchestrate")
            .arg("--scxml")
            .arg(first)
            .arg("--scxml")
            .arg(second)
            .arg("--language")
            .arg("rust")
            .arg("--output-dir")
            .arg(out.path());
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn orchestrate");
        assert!(
            output.status.success(),
            "both documents must orchestrate; stderr: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let manifest: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("manifest parses");
        manifest["needs_script_engine"]
            .as_bool()
            .expect("needs_script_engine is a bool")
    };

    // The control: this document alone is what forces the engine in, so
    // the union claim is about the set rather than about a flag that is
    // always true.
    assert!(
        !read_flag(&plain, &plain),
        "a set of pure-static documents must not claim to need an engine",
    );
    assert!(
        read_flag(&plain, &scripted),
        "the flag is the union over the set, so a later input still sets it",
    );
}
