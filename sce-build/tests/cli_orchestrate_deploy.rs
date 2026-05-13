//! End-to-end CLI contract test for `sce-codegen orchestrate --deploy`.
//!
//! Closes the textbook end-user surface for the C13 chain: the
//! orchestrator wiring `b501b18c` exposed `Option<&DeployConfig>` on
//! the library entry point, but the CLI multi-doc subcommand
//! (`Commands::Orchestrate`) had no flag to pass it. This atomic adds
//! `--deploy=PATH` so end-users invoking the binary can fire
//! watching-zenoh RFC §5.K + §5.M cross-doc validators without
//! library-level wrappers.
//!
//! Test matrix (mirrors the library-level `c13_*` tests in
//! `scxml_cross_doc_orchestrator.rs` but at the CLI process boundary):
//!   1. Without `--deploy`, behavior unchanged — orchestrator
//!      silent-skips C13 validators (Q-η5 (a) precedent).
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

/// Forge `<sce:link>` referencing a `<sce:rx-pool>` so C13-α-2's
/// reassembly/burst validators have a 3-way join to follow when deploy
/// is supplied.
fn link_with_rx_pool(name: &str, rx_pool: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="{name}" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:rx-pool ref="{rx_pool}"/>
</scxml>"##
    )
}

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
    cmd.arg("--error-format").arg(error_format).arg("orchestrate");
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
    // --deploy entirely must silent-skip per Q-η5 (a) precedent;
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
        &buffer_pool("rx_pool", 16, 1500),
    );
    let link = write_doc(
        dir.path(),
        "udp_data.scxml",
        &link_with_rx_pool("udp_data", "rx_pool"),
    );
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
    // through the C13-α-1 validator. CLI exits non-zero with NDJSON
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
        &buffer_pool("rx_pool", 16, 1500),
    );
    let link = write_doc(
        dir.path(),
        "udp_data.scxml",
        &link_with_rx_pool("udp_data", "rx_pool"),
    );
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
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("not JSON ({e}): {line}"));
        let code = parsed
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if code == "deploy/link-not-declared-in-deploy" {
            found_target_code = true;
            assert_eq!(
                parsed
                    .get("stage")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
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
    assert!(
        !trimmed.is_empty(),
        "malformed deploy must emit diagnostic",
    );
    // The deploy parser produces `mesh/deploy-yaml-parse` (or similar
    // stable code). Pin the stage so a regression to "io" or generic
    // failure surfaces.
    let mut saw_mesh_deploy_stage = false;
    for line in trimmed.lines() {
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("not JSON ({e}): {line}"));
        if parsed.get("stage").and_then(|v| v.as_str()) == Some("mesh-deploy") {
            saw_mesh_deploy_stage = true;
        }
    }
    assert!(
        saw_mesh_deploy_stage,
        "malformed deploy must carry stage=mesh-deploy; stderr: {stderr}",
    );
}
