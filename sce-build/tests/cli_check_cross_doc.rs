// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// End-to-end contract test for `sce-codegen check` over a document set.
//
// `cli_check.rs` pins the single-document half of the subcommand:
// `check -l X` agrees with `generate -l X` and writes nothing. This file
// pins the other half. A build whose documents reference each other
// (`<sce:on-sample link>`, `<sce:outbox ref>`) or that is placed onto
// machines by a `deploy.yaml` is validated by `orchestrate`, and the
// cross-doc and deploy validators fire only there — the single-document
// route has no registry to resolve against and no deploy topology to
// check placement against.
//
// Two claims, mirroring the single-document pair:
//
//   1. **Same verdict.** For every deploy variant and every backend,
//      `check` agrees with `orchestrate` on the exit code and on the
//      diagnostic code. The reference producer is the one the invocation
//      shape names: `generate` for a lone document, `orchestrate` for a
//      set.
//
//   2. **Writes nothing.** `orchestrate` must be given an
//      `--output-dir` and materialises the whole build into it on the
//      success path, so asking "is this system valid?" costs a tree of
//      artifacts. `check` answers the same question and creates no file
//      anywhere.
//
// The deploy-refusal case is the load-bearing control: without a run
// that actually reaches `deploy/link-not-declared-in-deploy`, agreement
// between the two commands would only prove that both silent-skip the
// validators this test exists to exercise.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

/// Scoped scratch directory under `target/`; removed on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let dir = root.join(format!("{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// Every entry name currently in the directory tree, relative to its
    /// root and sorted. Recursive because the failure this guards
    /// against — a build materialised under `-o` — nests.
    fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        let mut stack = vec![self.0.clone()];
        while let Some(dir) = stack.pop() {
            let Ok(read) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in read.flatten() {
                let path = entry.path();
                names.push(
                    path.strip_prefix(&self.0)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .into_owned(),
                );
                if path.is_dir() {
                    stack.push(path);
                }
            }
        }
        names.sort();
        names
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Backends, in the order `Language::ALL` declares them.
const BACKENDS: &[&str] = &["rust", "cpp", "kotlin", "go", "python", "c11"];

/// Backends that lower the MCU-class forge kinds (`buffer-pool`,
/// `link`) the fixture set is built from. The others refuse with
/// `codegen/mcu-class-kind-on-non-mcu-language` on the backend axis
/// before any deploy topology is consulted, which is why the parity
/// matrix expects a refusal from them under every deploy variant.
const MCU_CLASS_BACKENDS: &[&str] = &["rust", "c11"];

fn lowers_mcu_class_kinds(backend: &str) -> bool {
    MCU_CLASS_BACKENDS.contains(&backend)
}

/// What one invocation reported: process exit status and, when it
/// failed, the `code` of the first NDJSON diagnostic on stderr.
#[derive(Debug, PartialEq, Eq)]
struct Verdict {
    exit: Option<i32>,
    code: Option<String>,
}

fn first_diagnostic_code(stderr: &str) -> Option<String> {
    stderr.lines().find_map(|line| {
        let line = line.trim();
        if !line.starts_with('{') {
            return None;
        }
        let v: serde_json::Value = serde_json::from_str(line).ok()?;
        v.get("code")?.as_str().map(|s| s.to_string())
    })
}

fn run(args: &[&str], cwd: &Path) -> (Verdict, String) {
    let out = Command::new(sce_codegen_bin())
        .args(args)
        .arg("--error-format=json")
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (
        Verdict {
            exit: out.status.code(),
            code: first_diagnostic_code(&stderr),
        },
        stdout,
    )
}

/// Minimal statechart — carries no cross-doc reference of its own, so
/// the only cross-doc surface a fixture set exercises is the forge and
/// deploy join.
fn statechart_minimal(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       name="{name}"
       version="1.0"
       initial="idle"
       datamodel="ecmascript">
  <state id="idle"/>
</scxml>
"##
    )
}

/// Forge `<sce:link>` referencing a `<sce:rx-pool>`, so the deploy
/// validators have a link name to look for in the topology.
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
</scxml>
"##
    )
}

/// The codec [`link_with_rx_pool`]'s `<sce:framer ref>` names. Staged
/// beside the link so the `<sce:import src>` resolves, and reached
/// through that import rather than the document set so the set both
/// commands are handed stays the one under test.
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
</scxml>
"##
    )
}

/// Slot size is a whole multiple of the declared 32-byte alignment: a
/// pool carrying Ethernet frames rounds the 1500-byte payload up to the
/// DMA boundary, which is why 1536 rather than 1500.
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
</scxml>
"##
    )
}

/// Deploy fixture with a single MCU machine; `link_name` is the one
/// entry under `links:`, which is what the cross-doc validator joins
/// against the forge `<sce:link>` name.
fn deploy_yaml(link_name: &str) -> String {
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
          {link_name}:
            bind: "224.0.0.224:7446"
            driver: lwip_udp
"#
    )
}

/// The document set both commands are handed, staged into `dir`.
struct DocSet {
    scxml: PathBuf,
    pool: PathBuf,
    link: PathBuf,
    /// `deploy.yaml` naming the link the forge doc declares — the set
    /// every validator accepts.
    deploy_ok: PathBuf,
    /// `deploy.yaml` naming a different link, which fires
    /// `deploy/link-not-declared-in-deploy`.
    deploy_missing_link: PathBuf,
    /// Not YAML at all — the parse-failure axis.
    deploy_malformed: PathBuf,
}

fn stage_doc_set(dir: &Path) -> DocSet {
    let write = |name: &str, body: &str| {
        let p = dir.join(name);
        std::fs::write(&p, body).expect("stage fixture");
        p
    };
    // The codec the link's `<sce:framer ref>` names. Staged beside the
    // link so the import resolves, and not carried in `DocSet` because
    // it is never handed to a command — the document set under test is
    // exactly the statechart, the pool and the link.
    write(
        "scout_frame_codec.scxml",
        &framer_codec("scout_frame_codec"),
    );
    DocSet {
        scxml: write("session_fsm.scxml", &statechart_minimal("session_fsm")),
        pool: write("rx_pool.scxml", &buffer_pool("rx_pool", 16, 1536)),
        link: write("udp_data.scxml", &link_with_rx_pool("udp_data", "rx_pool")),
        deploy_ok: write("deploy_ok.yaml", &deploy_yaml("udp_data")),
        deploy_missing_link: write("deploy_missing.yaml", &deploy_yaml("udp_scout")),
        deploy_malformed: write(
            "deploy_bad.yaml",
            "this is not valid yaml :: : ::\n  - garbage\n",
        ),
    }
}

/// The deploy variants the parity sweep runs, paired with whether the
/// variant is expected to be refused. The flag is asserted rather than
/// merely recorded: a sweep in which nothing is refused proves only that
/// both commands accept everything.
fn deploy_variants(docs: &DocSet) -> Vec<(&'static str, Option<&Path>, bool)> {
    vec![
        ("no-deploy", None, false),
        ("deploy-ok", Some(docs.deploy_ok.as_path()), false),
        (
            "deploy-missing-link",
            Some(docs.deploy_missing_link.as_path()),
            true,
        ),
        (
            "deploy-malformed",
            Some(docs.deploy_malformed.as_path()),
            true,
        ),
    ]
}

/// `check` over a document set reaches the same verdict `orchestrate`
/// would, for every deploy variant and every backend.
///
/// The one intended difference is the filesystem, which
/// [`check_over_a_document_set_writes_no_file_anywhere`] pins
/// separately.
#[test]
fn check_and_orchestrate_agree_on_every_document_set_and_backend() {
    let staged = ScratchDir::new("check-xdoc-parity-in");
    let docs = stage_doc_set(staged.path());
    let out_dir = ScratchDir::new("check-xdoc-parity-out");
    let cwd = repo_root();

    let mut compared = 0usize;
    // Refusals the deploy topology is responsible for: the backend
    // lowers every kind in the set, so the only thing left to refuse it
    // is the topology. Counted separately from the backend-axis
    // refusals, which would otherwise let the sweep claim coverage of an
    // axis it never reached.
    let mut deploy_axis_refusals = 0usize;
    let mut disagreements: Vec<String> = Vec::new();

    for (label, deploy, variant_refuses) in deploy_variants(&docs) {
        for backend in BACKENDS {
            let expect_refusal = variant_refuses || !lowers_mcu_class_kinds(backend);
            let mut orch: Vec<&str> = vec![
                "orchestrate",
                "--scxml",
                docs.scxml.to_str().unwrap(),
                "--forge",
                docs.pool.to_str().unwrap(),
                "--forge",
                docs.link.to_str().unwrap(),
                "-l",
                backend,
                "--output-dir",
                out_dir.path().to_str().unwrap(),
            ];
            let mut chk: Vec<&str> = vec![
                "check",
                docs.scxml.to_str().unwrap(),
                "--forge",
                docs.pool.to_str().unwrap(),
                "--forge",
                docs.link.to_str().unwrap(),
                "-l",
                backend,
            ];
            if let Some(d) = deploy {
                let d = d.to_str().unwrap();
                orch.extend_from_slice(&["--deploy", d]);
                chk.extend_from_slice(&["--deploy", d]);
            }

            let (orch_verdict, _) = run(&orch, &cwd);
            let (check_verdict, _) = run(&chk, &cwd);
            compared += 1;
            if variant_refuses && lowers_mcu_class_kinds(backend) {
                deploy_axis_refusals += 1;
            }
            assert_eq!(
                orch_verdict.exit != Some(0),
                expect_refusal,
                "{label} [{backend}]: fixture no longer exercises the axis it was built for \
                 (orchestrate reported {orch_verdict:?})",
            );
            if orch_verdict != check_verdict {
                disagreements.push(format!(
                    "{label} [{backend}]: orchestrate={orch_verdict:?} check={check_verdict:?}",
                ));
            }
        }
    }

    assert!(
        disagreements.is_empty(),
        "check and orchestrate disagree on {} of {compared} document-set/backend pairs:\n  {}",
        disagreements.len(),
        disagreements.join("\n  "),
    );
    assert_eq!(
        compared,
        deploy_variants(&docs).len() * BACKENDS.len(),
        "the sweep must compare every deploy variant against every backend",
    );
    let refusing_variants = deploy_variants(&docs).iter().filter(|v| v.2).count();
    assert_eq!(
        deploy_axis_refusals,
        refusing_variants * MCU_CLASS_BACKENDS.len(),
        "every refusing deploy variant must refuse on every backend that lowers the set; \
         a sweep whose refusals all come from the backend axis proves only that the two \
         commands reject the same kinds, not that either consulted the topology",
    );
    assert!(
        deploy_axis_refusals > 0,
        "no compared pair was refused for a deploy reason; agreement would then prove only \
         that both commands silent-skip the validators this sweep exists to exercise",
    );
}

/// The deploy cross-doc validator is reached by `check`, not skipped.
///
/// Without this, the parity sweep above would be satisfied by a `check`
/// that accepts its `--deploy` argument and ignores it, since
/// `orchestrate` without a deploy also exits 0.
#[test]
fn check_reaches_the_deploy_cross_doc_validator() {
    let staged = ScratchDir::new("check-xdoc-deploy-axis");
    let docs = stage_doc_set(staged.path());
    let cwd = repo_root();

    let (verdict, stdout) = run(
        &[
            "check",
            docs.scxml.to_str().unwrap(),
            "--forge",
            docs.pool.to_str().unwrap(),
            "--forge",
            docs.link.to_str().unwrap(),
            "--deploy",
            docs.deploy_missing_link.to_str().unwrap(),
            "-l",
            "rust",
        ],
        &cwd,
    );
    assert_ne!(
        verdict.exit,
        Some(0),
        "a link absent from deploy.yaml must be fatal",
    );
    assert_eq!(
        verdict.code.as_deref(),
        Some("deploy/link-not-declared-in-deploy"),
        "check must surface the deploy cross-doc diagnostic itself",
    );
    assert!(
        stdout.trim().is_empty(),
        "stdout must stay empty on failure: {stdout}",
    );

    // The same set with a deploy that declares the link is accepted, so
    // the refusal above is attributable to the topology and not to the
    // documents.
    let (ok, _) = run(
        &[
            "check",
            docs.scxml.to_str().unwrap(),
            "--forge",
            docs.pool.to_str().unwrap(),
            "--forge",
            docs.link.to_str().unwrap(),
            "--deploy",
            docs.deploy_ok.to_str().unwrap(),
            "-l",
            "rust",
        ],
        &cwd,
    );
    assert_eq!(
        ok.exit,
        Some(0),
        "a deploy that declares the link must check clean",
    );
}

/// Omitting `--deploy` keeps the deploy validators silent-skipped, the
/// same absent-input discipline `orchestrate` follows. A `check` that
/// invented a default topology would refuse documents `orchestrate`
/// accepts.
#[test]
fn check_without_deploy_silent_skips_the_deploy_validators() {
    let staged = ScratchDir::new("check-xdoc-no-deploy");
    let docs = stage_doc_set(staged.path());
    let cwd = repo_root();

    let (verdict, stdout) = run(
        &[
            "check",
            docs.scxml.to_str().unwrap(),
            "--forge",
            docs.pool.to_str().unwrap(),
            "--forge",
            docs.link.to_str().unwrap(),
            "-l",
            "rust",
        ],
        &cwd,
    );
    assert_eq!(
        verdict.exit,
        Some(0),
        "a document set with no deploy must check clean: {verdict:?}",
    );
    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("check emits one JSON manifest");
    assert_eq!(manifest["kind"], "check");
}

/// `check` creates nothing for a document set either — while
/// `orchestrate`, handed the same set, materialises the build.
///
/// The `orchestrate` half is asserted positively: "check wrote nothing"
/// is only meaningful next to a producer that wrote something.
#[test]
fn check_over_a_document_set_writes_no_file_anywhere() {
    let cwd = ScratchDir::new("check-xdoc-cwd");
    let staged = ScratchDir::new("check-xdoc-input");
    let docs = stage_doc_set(staged.path());

    let before_cwd = cwd.entries();
    let before_staged = staged.entries();

    let (verdict, stdout) = run(
        &[
            "check",
            docs.scxml.to_str().unwrap(),
            "--forge",
            docs.pool.to_str().unwrap(),
            "--forge",
            docs.link.to_str().unwrap(),
            "--deploy",
            docs.deploy_ok.to_str().unwrap(),
            "-l",
            "rust",
        ],
        cwd.path(),
    );
    assert_eq!(verdict.exit, Some(0), "clean document set must check clean");

    assert_eq!(
        cwd.entries(),
        before_cwd,
        "check must not create anything in the working directory",
    );
    assert_eq!(
        staged.entries(),
        before_staged,
        "check must not create anything beside the input documents",
    );

    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("check emits one JSON manifest");
    assert_eq!(
        manifest["artifacts"].as_array().map(|a| a.len()),
        Some(0),
        "check manifest must carry an empty artifacts array: {stdout}",
    );

    // The control: the producer this verdict mirrors does write, so the
    // assertions above are about `check` and not about a document set
    // that happens to generate nothing.
    let orch_out = ScratchDir::new("check-xdoc-orch-out");
    let (orch_verdict, _) = run(
        &[
            "orchestrate",
            "--scxml",
            docs.scxml.to_str().unwrap(),
            "--forge",
            docs.pool.to_str().unwrap(),
            "--forge",
            docs.link.to_str().unwrap(),
            "--deploy",
            docs.deploy_ok.to_str().unwrap(),
            "-l",
            "rust",
            "--output-dir",
            orch_out.path().to_str().unwrap(),
        ],
        cwd.path(),
    );
    assert_eq!(orch_verdict.exit, Some(0), "control run must succeed");
    assert!(
        !orch_out.entries().is_empty(),
        "orchestrate must write the build that check declines to write; \
         an empty output directory makes the no-write claim vacuous",
    );
}

/// A document set swept with no `--language` reports one verdict per
/// backend, the same shape the single-document sweep emits.
#[test]
fn a_document_set_sweep_reports_every_backend() {
    let staged = ScratchDir::new("check-xdoc-sweep");
    let docs = stage_doc_set(staged.path());
    let cwd = repo_root();

    let (verdict, stdout) = run(
        &[
            "check",
            docs.scxml.to_str().unwrap(),
            "--forge",
            docs.pool.to_str().unwrap(),
            "--forge",
            docs.link.to_str().unwrap(),
            "--deploy",
            docs.deploy_ok.to_str().unwrap(),
        ],
        &cwd,
    );
    assert_eq!(
        verdict.exit,
        Some(0),
        "an unnamed sweep reports backend coverage rather than failing: {verdict:?}",
    );

    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("sweep emits a manifest");
    let languages = manifest["languages"]
        .as_array()
        .expect("sweep manifest carries languages");
    assert_eq!(
        languages.len(),
        BACKENDS.len(),
        "sweep must report every backend: {stdout}",
    );

    // The record is only a contract if it is checked against the schema
    // the consumers compile.
    let schema_value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("schemas/sce-manifest.v1.schema.json"))
            .expect("read manifest schema"),
    )
    .expect("manifest schema is JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_value)
        .expect("manifest schema compiles as draft-07");
    let msgs: Vec<String> = match validator.validate(&manifest) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    };
    assert!(
        msgs.is_empty(),
        "document-set check manifest violates the wire schema: {msgs:?}\n{stdout}",
    );
}

/// A refusal that belongs to no single backend is fatal in a sweep too.
///
/// Without `--language` the per-backend verdict rides the manifest and
/// the exit is 0, because "only Rust lowers this" is an answer rather
/// than a failure. A broken deploy topology is not that kind of answer:
/// it is wrong under every backend, so reporting it as six per-backend
/// rejections with exit 0 would tell a caller the system checks out.
/// The document-set route reads the axis off the diagnostic's stage,
/// since its compile call fuses cross-doc validation with rendering.
#[test]
fn a_sweep_still_fails_on_a_refusal_no_backend_could_avoid() {
    let staged = ScratchDir::new("check-xdoc-sweep-axis");
    let docs = stage_doc_set(staged.path());
    let cwd = repo_root();

    let (verdict, stdout) = run(
        &[
            "check",
            docs.scxml.to_str().unwrap(),
            "--forge",
            docs.pool.to_str().unwrap(),
            "--forge",
            docs.link.to_str().unwrap(),
            "--deploy",
            docs.deploy_missing_link.to_str().unwrap(),
        ],
        &cwd,
    );
    assert_ne!(
        verdict.exit,
        Some(0),
        "a deploy refusal must stay fatal without --language: {stdout}",
    );
    assert_eq!(
        verdict.code.as_deref(),
        Some("deploy/link-not-declared-in-deploy"),
    );
    assert!(
        stdout.trim().is_empty(),
        "stdout must stay empty on failure: {stdout}",
    );

    // The control that keeps the rule from collapsing into "every
    // refusal is fatal": a backend that cannot lower an MCU-class kind
    // is exactly the per-backend answer the sweep exists to report, and
    // it must still ride the manifest with exit 0.
    let (swept, swept_stdout) = run(
        &[
            "check",
            docs.scxml.to_str().unwrap(),
            "--forge",
            docs.pool.to_str().unwrap(),
            "--forge",
            docs.link.to_str().unwrap(),
        ],
        &cwd,
    );
    assert_eq!(
        swept.exit,
        Some(0),
        "a backend-only refusal must stay reportable: {swept_stdout}",
    );
    let manifest: serde_json::Value =
        serde_json::from_str(swept_stdout.trim()).expect("sweep emits a manifest");
    let rejected: Vec<&str> = manifest["languages"]
        .as_array()
        .expect("sweep manifest carries languages")
        .iter()
        .filter(|v| v["status"] == "rejected")
        .filter_map(|v| v["language"].as_str())
        .collect();
    assert_eq!(
        rejected.len(),
        BACKENDS.len() - MCU_CLASS_BACKENDS.len(),
        "every backend that cannot lower an MCU-class kind must be reported, not fatal: \
         {swept_stdout}",
    );
}

/// `needs_script_engine` describes the set, not one of its documents.
///
/// A build system reads this key to decide whether to link a script
/// engine, and that decision is made once for the whole build. Reporting
/// the first document's answer, or a fixed `false`, would silently drop
/// the engine for a set in which any one document needs it.
#[test]
fn needs_script_engine_is_the_union_over_the_document_set() {
    let staged = ScratchDir::new("check-xdoc-script-engine");
    let cwd = repo_root();
    let write = |name: &str, body: &str| {
        let p = staged.path().join(name);
        std::fs::write(&p, body).expect("stage fixture");
        p
    };

    let pure = write("pure.scxml", &statechart_minimal("pure"));
    // `<script>` is what puts a document on the engine-backed route; the
    // rest of the document is the same minimal shape as `pure`.
    let scripted = write(
        "scripted.scxml",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       name="scripted"
       version="1.0"
       initial="idle"
       datamodel="ecmascript">
  <datamodel><data id="n" expr="0"/></datamodel>
  <state id="idle">
    <onentry><script>n = n + 1;</script></onentry>
  </state>
</scxml>
"##,
    );

    let engine_of = |paths: &[&PathBuf]| -> bool {
        let mut args = vec!["check".to_string()];
        for (i, p) in paths.iter().enumerate() {
            if i > 0 {
                args.push("--scxml".to_string());
            }
            args.push(p.to_str().unwrap().to_string());
        }
        args.extend(["-l".to_string(), "rust".to_string()]);
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let (verdict, stdout) = run(&borrowed, &cwd);
        assert_eq!(verdict.exit, Some(0), "{args:?} must check clean");
        let manifest: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("check emits one JSON manifest");
        manifest["needs_script_engine"]
            .as_bool()
            .expect("needs_script_engine is a bool")
    };

    // The control: without it, a `false` union would be indistinguishable
    // from a route that never consults the documents at all.
    assert!(
        !engine_of(&[&pure, &pure]),
        "a set of engine-free documents needs no engine",
    );
    assert!(
        engine_of(&[&scripted, &pure]),
        "a set whose first document needs an engine reports true",
    );
    assert!(
        engine_of(&[&pure, &scripted]),
        "a set whose *later* document needs an engine reports true — reading only the \
         first document would answer false here",
    );
}

/// Flags the document-set route cannot honour are refused, not ignored.
///
/// `-I`, `--strict-unresolved` and `--no-std` have no counterpart in the
/// multi-doc compile entry point: includes resolve relative to each
/// document with no search path, and there is no `no_std` variant to
/// render. Accepting them here and quietly dropping them would let a
/// caller believe a constraint was checked when nothing consulted it —
/// the failure mode a silent no-op flag always has.
#[test]
fn document_set_flags_the_route_cannot_honour_are_refused() {
    let staged = ScratchDir::new("check-xdoc-flag-conflicts");
    let docs = stage_doc_set(staged.path());
    let cwd = repo_root();
    let scxml = docs.scxml.to_str().unwrap();
    let forge = docs.link.to_str().unwrap();
    let deploy = docs.deploy_ok.to_str().unwrap();

    // Each flag is legal on its own — the refusal must be attributable
    // to the combination and not to the flag being unknown.
    for single_doc in [
        vec!["check", scxml, "-I", "."],
        vec!["check", scxml, "--strict-unresolved"],
        vec!["check", scxml, "--no-std", "-l", "rust"],
    ] {
        let (verdict, _) = run(&single_doc, &cwd);
        assert_eq!(
            verdict.exit,
            Some(0),
            "{single_doc:?} must stay valid for a single document",
        );
    }

    // Each of the three arguments that select the document-set route
    // must refuse each of the three flags, so a route reached by an
    // untested spelling cannot slip past.
    for route in [
        vec!["--scxml", scxml],
        vec!["--forge", forge],
        vec!["--deploy", deploy],
    ] {
        for flag in [
            vec!["-I", "."],
            vec!["--strict-unresolved"],
            vec!["--no-std"],
        ] {
            let mut args = vec!["check", scxml];
            args.extend_from_slice(&route);
            args.extend_from_slice(&flag);
            let (verdict, stdout) = run(&args, &cwd);
            assert_eq!(
                verdict.exit,
                Some(20),
                "{flag:?} with {route:?} must be refused as `cli/usage` (exit 20), \
                 not accepted — SCE_ERROR_CONTRACT.md §6 reserves exit 2 for `xml/*`",
            );
            assert!(
                stdout.trim().is_empty(),
                "a refused invocation must emit no manifest: {stdout}",
            );
        }
    }
}

/// The cross-doc link join is bidirectional, and `check` reaches both
/// directions.
///
/// The parity sweep above always supplies the forge documents, so it
/// only ever exercises pass A (a forge `<sce:link>` with no deploy entry
/// to place it on). Dropping `--forge` while keeping `--deploy` inverts
/// the question — a deployed link with no forge document to describe it
/// — and that is a different validator. Without this case, `check` could
/// reach one direction and silently skip the other.
#[test]
fn check_reaches_both_directions_of_the_deploy_link_join() {
    let staged = ScratchDir::new("check-xdoc-deploy-only");
    let docs = stage_doc_set(staged.path());
    let out_dir = ScratchDir::new("check-xdoc-deploy-only-out");
    let cwd = repo_root();

    let (verdict, stdout) = run(
        &[
            "check",
            docs.scxml.to_str().unwrap(),
            "--deploy",
            docs.deploy_ok.to_str().unwrap(),
            "-l",
            "rust",
        ],
        &cwd,
    );
    assert_eq!(
        verdict.code.as_deref(),
        Some("deploy/link-not-declared-in-forge"),
        "a deployed link with no forge document must fire the reverse validator",
    );
    assert_ne!(verdict.exit, Some(0), "the reverse refusal must be fatal");
    assert!(
        stdout.trim().is_empty(),
        "stdout must stay empty on failure: {stdout}",
    );

    // The producer reaches the same verdict, which is what makes the
    // refusal a property of the topology rather than of `check`.
    let (orch, _) = run(
        &[
            "orchestrate",
            "--scxml",
            docs.scxml.to_str().unwrap(),
            "--deploy",
            docs.deploy_ok.to_str().unwrap(),
            "-l",
            "rust",
            "--output-dir",
            out_dir.path().to_str().unwrap(),
        ],
        &cwd,
    );
    assert_eq!(
        orch, verdict,
        "check and orchestrate must agree on the reverse direction too",
    );

    // A malformed deploy is fatal before either direction is consulted,
    // because the topology itself failed to parse.
    let (bad, _) = run(
        &[
            "check",
            docs.scxml.to_str().unwrap(),
            "--deploy",
            docs.deploy_malformed.to_str().unwrap(),
            "-l",
            "rust",
        ],
        &cwd,
    );
    assert_ne!(bad.exit, Some(0), "a malformed deploy.yaml must be fatal");
    assert_ne!(
        bad.code, verdict.code,
        "a parse failure must not be reported as a join failure",
    );
}
