//! Cross-doc orchestrator integration tests — SCE Protocol-Synthesis RFC §synth-5-D
//! worker/outbox cross-doc surface.
//!
//! The distinguishing value pinned here: production wire-up of
//! `validate_on_sample_link_references` (parser.rs:3187) via the
//! `compile_scxml_with_imports` orchestrator entry point. Before this
//! wire-up landed, the on-sample cross-ref validator existed only as a
//! pub fn callable from tests — every production build path
//! (`compile_scxml`, `compile_scxml_to_string`, `compile_forge_with_imports`,
//! `sce-codegen generate`) processed files one at a time with no
//! cross-doc registry construction, so `<sce:on-sample link="undeclared">`
//! references silently passed (`feedback_silently_broken_hooks.md`
//! instance). These tests pin that the orchestrator surface CLOSES
//! the silent hole — undeclared refs now fire production-side.
//!
//! Test matrix:
//! 1. happy_orchestrator_compiles_multi_doc — 1 statechart + 1 link
//!    with stage-pool resolves cleanly; outputs emitted per-doc.
//! 2. on_sample_link_not_declared_fires_in_production — name not in
//!    cross-doc registry → `scxml/on-sample-link-not-declared` fires.
//! 3. on_sample_sample_take_without_stage_pool_fires — name resolves
//!    to a link without `<sce:stage-pool>` → `pool/sample-take-
//!    without-stage-pool` fires.
//! 4. on_sample_link_wrong_kind_fires — name resolves to a non-link
//!    kind (statechart with colliding name) → `scxml/on-sample-link-
//!    wrong-kind` fires (this arm became reachable in production once
//!    the registry extension landed multi-kind variants).
//! 5. empty_file_lists_yield_empty_outputs — no-op edge with both
//!    slices empty; orchestrator does not crash, returns Vec::new().
//! 6. worker_doc_records_into_cross_doc_registry — registry
//!    foundation: a worker doc's name reaches the registry so
//!    `<sce:outbox ref="worker.inbox">` resolution (covered in
//!    `c2_worker_outbox.rs`) lands cleanly on it.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

fn default_options() -> ForgeCompileOptions {
    ForgeCompileOptions::default()
}

/// Locate the workspace `tools/codegen/templates` directory once per
/// test. Mirrors `sce_build::find_template_dir_for(Language::Rust)`'s
/// semantics without crossing the private boundary — tests can use
/// the same root irrespective of language because the orchestrator's
/// per-language dispatch happens inside `compile_scxml_lang_typed`.
fn template_dir() -> PathBuf {
    sce_build::find_template_dir_for(Language::Rust)
}

/// Minimal statechart SCXML with one `<sce:on-sample>` block. The
/// `name` attribute lets the cross-doc registry classify this doc as
/// a statechart; the `<sce:on-sample>` block triggers cross-ref
/// validation against the orchestrator's link registry.
fn statechart_with_on_sample(name: &str, link: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="{name}"
       version="1.0"
       initial="running"
       datamodel="ecmascript">
  <state id="running">
    <sce:on-sample link="{link}" event="scout.tick"/>
    <transition event="scout.tick" target="running"/>
  </state>
</scxml>"##
    )
}

fn link_with_stage_pool() -> &'static str {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="scout_link" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:stage-pool ref="scout_stage_pool"/>
</scxml>"##
}

fn link_without_stage_pool() -> &'static str {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="scout_link" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
</scxml>"##
}

fn worker_minimal(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="{name}" version="1.0">
  <sce:import as="scout_link" src="scout_link.scxml" kind="link"/>
  <sce:link-rx ref="scout_link"/>
  <sce:inbox depth="16" ordering="acq_rel"/>
</scxml>"##
    )
}

fn write_doc(dir: &Path, basename: &str, content: &str) -> PathBuf {
    let path = dir.join(basename);
    fs::write(&path, content).expect("write doc");
    path
}

// ─── 1. Happy multi-doc compile ───────────────────────────────────────

#[test]
fn happy_orchestrator_compiles_multi_doc() {
    // Statechart with on-sample references link "scout_link"; forge
    // link doc declares "scout_link" with a stage-pool ref. Both
    // cross-ref checks succeed; orchestrator returns one output per
    // input doc.
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_with_on_sample("session_fsm", "scout_link"),
    );
    let forge = write_doc(dir.path(), "scout_link.scxml", link_with_stage_pool());
    // `link_with_stage_pool`'s `<sce:stage-pool ref>` names this
    // document. `Sample::take()` copies into it, so the ref has to
    // resolve to something the build can see — `link/pool-ref-not-
    // declared` refuses a link whose pool ref names nothing.
    let stage_pool = write_doc(
        dir.path(),
        "scout_stage_pool.scxml",
        &buffer_pool_default("scout_stage_pool", 8, 1536),
    );

    let scxml_refs: &[&Path] = &[scxml.as_path()];
    let forge_refs: &[&Path] = &[stage_pool.as_path(), forge.as_path()];

    let outputs = compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
        None,
    )
    .expect("happy multi-doc compile must succeed");

    assert_eq!(outputs.len(), 3, "expected 2 forge + 1 scxml = 3 outputs");
    // Forge emits first (input order), then SCXML.
    assert_eq!(outputs[0].0, "scout_stage_pool.scxml");
    assert_eq!(outputs[1].0, "scout_link.scxml");
    assert_eq!(outputs[2].0, "session_fsm.scxml");
}

// ─── 2. on-sample-link-not-declared fires in production ─────────────

#[test]
fn on_sample_link_not_declared_fires_in_production() {
    // The on-sample reference names "unknown_link"; the only forge
    // link in the build is "scout_link". Before the orchestrator
    // wire-up this passed silently because no production path built
    // the registry. The orchestrator now closes that hole.
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_with_on_sample("session_fsm", "unknown_link"),
    );
    let forge = write_doc(dir.path(), "scout_link.scxml", link_with_stage_pool());
    // `link_with_stage_pool`'s `<sce:stage-pool ref>` names this
    // document. `Sample::take()` copies into it, so the ref has to
    // resolve to something the build can see — `link/pool-ref-not-
    // declared` refuses a link whose pool ref names nothing.
    let stage_pool = write_doc(
        dir.path(),
        "scout_stage_pool.scxml",
        &buffer_pool_default("scout_stage_pool", 8, 1536),
    );

    let scxml_refs: &[&Path] = &[scxml.as_path()];
    let forge_refs: &[&Path] = &[stage_pool.as_path(), forge.as_path()];

    let err = match compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
        None,
    ) {
        Ok(_) => panic!("undeclared on-sample link must fire diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::OnSampleLinkNotDeclared {
                link, candidates, ..
            } => {
                assert_eq!(link, "unknown_link");
                assert_eq!(candidates, vec!["scout_link".to_string()]);
            }
            other => panic!("expected OnSampleLinkNotDeclared, got: {other:?}"),
        },
        other => panic!("expected OnSampleLinkNotDeclared, got: {other:?}"),
    }
}

// ─── 3. sample-take-without-stage-pool fires in production ───────────

#[test]
fn on_sample_sample_take_without_stage_pool_fires() {
    // Link "scout_link" exists in the registry, but lacks
    // `<sce:stage-pool>`. The on-sample callback that takes
    // ownership would route through a runtime panic hook —
    // the orchestrator wire-up surfaces the gap at compile time.
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_with_on_sample("session_fsm", "scout_link"),
    );
    let forge = write_doc(dir.path(), "scout_link.scxml", link_without_stage_pool());

    let scxml_refs: &[&Path] = &[scxml.as_path()];
    let forge_refs: &[&Path] = &[forge.as_path()];

    let err = match compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
        None,
    ) {
        Ok(_) => panic!("missing stage-pool must fire diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::PoolSampleTakeWithoutStagePool { link, .. } => {
                assert_eq!(link, "scout_link");
            }
            other => panic!("expected PoolSampleTakeWithoutStagePool, got: {other:?}"),
        },
        other => panic!("expected PoolSampleTakeWithoutStagePool, got: {other:?}"),
    }
}

// ─── 4. on-sample-link-wrong-kind fires in production ───────────────

#[test]
fn on_sample_link_wrong_kind_fires() {
    // The on-sample reference names "scout_helper"; the cross-doc
    // registry holds "scout_helper" as a STATECHART (sibling doc),
    // not a link kind. The on-sample validator's wrong-kind arm
    // became reachable in production only once the registry
    // extension introduced multi-kind variants; before that
    // the registry could only hold Link kinds so this arm was
    // forward-compat-only.
    let dir = tempdir().expect("tempdir");
    let scxml_main = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_with_on_sample("session_fsm", "scout_helper"),
    );
    // A statechart named "scout_helper" — wrong kind for on-sample.
    let scxml_collider = write_doc(
        dir.path(),
        "scout_helper.scxml",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       name="scout_helper"
       version="1.0"
       initial="idle"
       datamodel="ecmascript">
  <state id="idle"/>
</scxml>"##,
    );

    let scxml_refs: &[&Path] = &[scxml_main.as_path(), scxml_collider.as_path()];
    let forge_refs: &[&Path] = &[];

    let err = match compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
        None,
    ) {
        Ok(_) => panic!("wrong-kind on-sample target must fire diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::OnSampleLinkWrongKind {
                link, actual_kind, ..
            } => {
                assert_eq!(link, "scout_helper");
                assert_eq!(actual_kind, "statechart");
            }
            other => panic!("expected OnSampleLinkWrongKind, got: {other:?}"),
        },
        other => panic!("expected OnSampleLinkWrongKind, got: {other:?}"),
    }
}

// ─── 5. Empty file lists are a legal no-op ──────────────────────────

#[test]
fn empty_file_lists_yield_empty_outputs() {
    // The orchestrator MUST handle the no-doc edge gracefully —
    // callers that gate on a manifest may invoke it with empty
    // slices when the manifest is empty.
    let scxml_refs: &[&Path] = &[];
    let forge_refs: &[&Path] = &[];

    let outputs = compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
        None,
    )
    .expect("empty file lists must not error");

    assert!(outputs.is_empty(), "empty input must yield empty output");
}

// ─── 6. Worker doc lands in cross-doc registry (outbox prereq) ─────

#[test]
fn worker_doc_records_into_cross_doc_registry() {
    // The worker schema includes a `name` attribute and the
    // cross-doc registry records worker docs alongside statecharts
    // + links.
    // This test pins that an SCXML on-sample reference targeting a
    // WORKER name (mispoint — workers aren't link subscribers)
    // fires wrong-kind, proving the worker's name reached the
    // registry. `<sce:outbox ref="rx_loop.inbox">` resolution
    // (covered in `c2_worker_outbox.rs`) walks the same registry.
    let dir = tempdir().expect("tempdir");
    // Worker fixture needs sibling link doc for its `<sce:import>`
    // to resolve at parse time.
    let link_sib = write_doc(dir.path(), "scout_link.scxml", link_with_stage_pool());
    // `link_with_stage_pool`'s `<sce:stage-pool ref>` names this
    // document. `Sample::take()` copies into it, so the ref has to
    // resolve to something the build can see — `link/pool-ref-not-
    // declared` refuses a link whose pool ref names nothing.
    let stage_pool = write_doc(
        dir.path(),
        "scout_stage_pool.scxml",
        &buffer_pool_default("scout_stage_pool", 8, 1536),
    );
    let worker = write_doc(dir.path(), "rx_loop.scxml", &worker_minimal("rx_loop"));
    let main = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_with_on_sample("session_fsm", "rx_loop"),
    );

    let scxml_refs: &[&Path] = &[main.as_path()];
    // Order matters: the link sibling registers first so the worker
    // doc's `<sce:import as="scout_link" kind="link"/>` resolves to a
    // known-registered link when the worker parse runs.
    let forge_refs: &[&Path] = &[stage_pool.as_path(), link_sib.as_path(), worker.as_path()];

    let err = match compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
        None,
    ) {
        Ok(_) => panic!("on-sample targeting worker name must fire wrong-kind"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::OnSampleLinkWrongKind {
                link, actual_kind, ..
            } => {
                assert_eq!(link, "rx_loop");
                assert_eq!(
                    actual_kind, "worker",
                    "registry must classify rx_loop as worker — \
                     record_document's Worker arm must be wired"
                );
            }
            other => panic!("expected OnSampleLinkWrongKind, got: {other:?}"),
        },
        other => panic!("expected OnSampleLinkWrongKind, got: {other:?}"),
    }
}

// ─── 7. Links + reassembly deploy-aware orchestrator wiring ─────────
//
// These tests pin that `compile_scxml_with_imports(..., Some(deploy))`
// fires the deploy-time cross-doc validators (validate_links_cross_doc
// + validate_links_burst_invariants + validate_reassembly_cross_doc).
// Before this orchestrator wiring landed, the 3 validators were
// publicly exposed but had no production caller — closing that
// silent-broken-hook gap [[feedback-silently-broken-hooks]].

/// Forge `<sce:link>` with an `<sce:rx-pool ref>` that the deploy-time
/// burst + reassembly validators follow to a `<sce:kind="buffer-pool">`
/// document in the same build.
fn link_with_rx_pool(name: &str, rx_pool_ref: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="{name}" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:rx-pool ref="{rx_pool_ref}"/>
</scxml>"##
    )
}

/// Forge `<sce:kind="buffer-pool">` Default-variant pool document
/// (no `<sce:variant>` element ⇒ BufferPoolVariant::Default).
/// Slot sizes passed here must be whole multiples of the 32-byte
/// alignment below (`mem/slot-size-not-alignment-multiple`), which is
/// why the MTU-sized fixtures use 1536 rather than 1500: a pool
/// carrying Ethernet frames has to round the payload up to the DMA
/// boundary, and 1536 is what a real MTU pool declares.
fn buffer_pool_default(name: &str, slot_count: u32, slot_size: u32) -> String {
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

/// Deploy fixture YAML — a single machine with platform + scheduler +
/// memory + per-test `links:` body plugged in.
fn deploy_with_links(links_yaml: &str) -> String {
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

/// Tiny statechart with no on-sample / outbox — just enough to satisfy
/// the orchestrator's pass-2 walk without triggering other cross-ref
/// validators. C13 tests focus on the deploy-aware path only.
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

/// Deploy-aware path silently passes when no deploy is supplied —
/// orchestrator must NOT fire C13 validators with `None` deploy
/// (absent-input silent-skip precedent). This is the baseline that
/// every existing call site relies on.
#[test]
fn c13_validators_silent_skip_when_deploy_none() {
    use sce_build::mesh::deploy::parse_deploy_str;
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    // Forge has `udp_data` but if we passed deploy with no matching
    // entry, validate_links_cross_doc WOULD fire. With None deploy,
    // the orchestrator silent-skips all 3 validators.
    let pool = write_doc(
        dir.path(),
        "rx_data_pool.scxml",
        &buffer_pool_default("rx_data_pool", 16, 1536),
    );
    let link = write_doc(
        dir.path(),
        "udp_data.scxml",
        &link_with_rx_pool("udp_data", "rx_data_pool"),
    );

    // Sanity: a deploy WITHOUT `udp_data` would trigger
    // LinkNotDeclaredInDeploy if we passed it; we pass None instead.
    let _orphan_deploy = parse_deploy_str(&deploy_with_links(
        r#"          udp_scout:
            bind: "224.0.0.224:7446"
            driver: lwip_udp
"#,
    ))
    .expect("orphan deploy parses");

    compile_scxml_with_imports(
        &[scxml.as_path()],
        &[pool.as_path(), link.as_path()],
        &template_dir(),
        Language::Rust,
        &default_options(),
        // ← deploy: None ⇒ C13 validators silent-skip.
        None,
    )
    .expect("None deploy ⇒ C13 validators silent-skip ⇒ Ok");
}

/// `validate_links_cross_doc` fires through the orchestrator:
/// forge declares `udp_data` but the supplied deploy.yaml has no
/// matching `machines.<n>.links.udp_data` entry.
#[test]
fn c13_link_not_declared_in_deploy_fires_through_orchestrator() {
    use sce_build::mesh::deploy::parse_deploy_str;
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    let pool = write_doc(
        dir.path(),
        "rx_data_pool.scxml",
        &buffer_pool_default("rx_data_pool", 16, 1536),
    );
    let link = write_doc(
        dir.path(),
        "udp_data.scxml",
        &link_with_rx_pool("udp_data", "rx_data_pool"),
    );

    // Deploy has `udp_scout` but forge has `udp_data` — Pass A
    // (forge → deploy) fires LinkNotDeclaredInDeploy.
    let deploy = parse_deploy_str(&deploy_with_links(
        r#"          udp_scout:
            bind: "224.0.0.224:7446"
            driver: lwip_udp
"#,
    ))
    .expect("deploy parses");

    let err = match compile_scxml_with_imports(
        &[scxml.as_path()],
        &[pool.as_path(), link.as_path()],
        &template_dir(),
        Language::Rust,
        &default_options(),
        Some(&deploy),
    ) {
        Ok(_) => panic!("forge udp_data + deploy udp_scout must fire"),
        Err(e) => e,
    };

    // The orchestrator routes DeployError through ForgeError::Mesh.
    match err.error {
        ForgeError::Mesh(ref boxed) => match boxed.as_ref() {
            sce_build::mesh::error::MeshError::Deploy(deploy_err) => match deploy_err.as_ref() {
                sce_build::mesh::error::DeployError::LinkNotDeclaredInDeploy {
                    link_name, ..
                } => {
                    assert_eq!(link_name, "udp_data");
                }
                other => panic!("expected DeployError::LinkNotDeclaredInDeploy, got {other:?}"),
            },
            other => panic!("expected MeshError::Deploy, got {other:?}"),
        },
        other => panic!("expected ForgeError::Mesh, got: {other:?}"),
    }
}

/// `validate_reassembly_cross_doc` fires through the
/// orchestrator: pool slot_size < link.mtu_bytes triggers
/// `mem/reassembly-slot-size-below-declared-mtu`.
#[test]
fn c13_reassembly_slot_size_fires_through_orchestrator() {
    use sce_build::mesh::deploy::parse_deploy_str;
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    // Pool slot_size=256, link mtu_bytes=1500 → #1 fires (slot < mtu).
    let pool = write_doc(
        dir.path(),
        "rx_data_pool.scxml",
        &buffer_pool_default("rx_data_pool", 16, 256),
    );
    let link = write_doc(
        dir.path(),
        "udp_data.scxml",
        &link_with_rx_pool("udp_data", "rx_data_pool"),
    );

    let deploy = parse_deploy_str(&deploy_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            mtu_bytes: 1500
            domain_attrs:
              trust_class: established_session
"#,
    ))
    .expect("deploy parses");

    let err = match compile_scxml_with_imports(
        &[scxml.as_path()],
        &[pool.as_path(), link.as_path()],
        &template_dir(),
        Language::Rust,
        &default_options(),
        Some(&deploy),
    ) {
        Ok(_) => panic!("slot_size 256 < mtu 1500 must fire"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::MemReassemblySlotSizeBelowDeclaredMtu {
                pool_name,
                slot_size,
                mtu_bytes,
                ..
            } => {
                assert_eq!(pool_name, "rx_data_pool");
                assert_eq!(slot_size, 256);
                assert_eq!(mtu_bytes, 1500);
            }
            other => panic!("expected MemReassemblySlotSizeBelowDeclaredMtu, got: {other:?}"),
        },
        other => panic!("expected MemReassemblySlotSizeBelowDeclaredMtu, got: {other:?}"),
    }
}

/// `validate_links_burst_invariants` fires through the
/// orchestrator: burst_pps overruns the RX pool drain capacity.
#[test]
fn c13_burst_absorption_fires_through_orchestrator() {
    use sce_build::mesh::deploy::parse_deploy_str;
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    // slot_count=16, tick_period_us=1000, burst_pps=50_000 ⇒
    // drain capacity (16 × 1_000_000 = 16M) < burst load × 2.0 safety
    // (50_000 × 1000 × 2 = 100M) ⇒ #A fires.
    let pool = write_doc(
        dir.path(),
        "rx_data_pool.scxml",
        &buffer_pool_default("rx_data_pool", 16, 1536),
    );
    let link = write_doc(
        dir.path(),
        "udp_data.scxml",
        &link_with_rx_pool("udp_data", "rx_data_pool"),
    );

    let deploy = parse_deploy_str(&deploy_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            burst_pps: 50000
            rx_dispatch: isr_to_pool
"#,
    ))
    .expect("deploy parses");

    let err = match compile_scxml_with_imports(
        &[scxml.as_path()],
        &[pool.as_path(), link.as_path()],
        &template_dir(),
        Language::Rust,
        &default_options(),
        Some(&deploy),
    ) {
        Ok(_) => panic!("50k pps vs 16 slots must fire absorption"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Mesh(ref boxed) => match boxed.as_ref() {
            sce_build::mesh::error::MeshError::Deploy(deploy_err) => match deploy_err.as_ref() {
                sce_build::mesh::error::DeployError::LinkBurstAbsorptionInsufficient {
                    link_name,
                    burst_pps,
                    slot_count,
                    ..
                } => {
                    assert_eq!(link_name, "udp_data");
                    assert_eq!(*burst_pps, 50_000);
                    assert_eq!(*slot_count, 16);
                }
                other => panic!("expected LinkBurstAbsorptionInsufficient, got {other:?}"),
            },
            other => panic!("expected MeshError::Deploy, got {other:?}"),
        },
        other => panic!("expected ForgeError::Mesh, got: {other:?}"),
    }
}

/// Happy path: deploy + forge align; orchestrator emits codegen
/// output. Proves the wiring doesn't false-positive when inputs are
/// well-formed.
#[test]
fn c13_orchestrator_happy_path_emits_outputs() {
    use sce_build::mesh::deploy::parse_deploy_str;
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    // Well-formed: slot_size >= mtu, burst absorption satisfied.
    let pool = write_doc(
        dir.path(),
        "rx_data_pool.scxml",
        &buffer_pool_default("rx_data_pool", 2000, 1536),
    );
    let link = write_doc(
        dir.path(),
        "udp_data.scxml",
        &link_with_rx_pool("udp_data", "rx_data_pool"),
    );

    let deploy = parse_deploy_str(&deploy_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            mtu_bytes: 1500
            burst_pps: 50
            rx_dispatch: isr_to_pool
            domain_attrs:
              trust_class: established_session
"#,
    ))
    .expect("deploy parses");

    let outputs = compile_scxml_with_imports(
        &[scxml.as_path()],
        &[pool.as_path(), link.as_path()],
        &template_dir(),
        Language::Rust,
        &default_options(),
        Some(&deploy),
    )
    .expect("well-formed inputs ⇒ orchestrator succeeds");
    assert!(!outputs.is_empty(), "happy path must emit outputs");
}

// ── Link pool-ref cross-doc resolution ───────────────────────────
//
// `<sce:rx-pool ref>` / `<sce:tx-pool ref>` / `<sce:stage-pool ref>`
// name a `sce:kind="buffer-pool"` document. Every downstream consumer
// of those refs resolves them by *join*, and every one of those joins
// is written to skip on a miss: `resolve_link_rx_pool_slot_count`
// returns `None`, `validate_links_burst_invariants` and
// `validate_reassembly_cross_doc` `continue`, and
// `validate_link_pool_framer_resolution` falls through its `None` arm.
// A skip is the right behaviour for a *partial topology* — the pool
// genuinely is not part of this build — but it is the wrong behaviour
// for a *typo*, which is indistinguishable from a partial topology
// unless somebody checks that the name resolves at all.
//
// The tests below pin the difference. The orchestrator owns the closed
// world (it is handed every document in the build), so it is the layer
// that can tell the two apart, and it must refuse the typo rather than
// let it turn the MCU capacity validators off.

/// Forge `<sce:link>` carrying any combination of the three pool refs.
/// Generalises [`link_with_rx_pool`]; the `Option` arms let one test
/// body drive the rx / tx / stage axes independently.
fn link_with_pools(
    name: &str,
    rx_pool: Option<&str>,
    tx_pool: Option<&str>,
    stage_pool: Option<&str>,
) -> String {
    let elem = |tag: &str, r: Option<&str>| match r {
        Some(v) => format!("  <sce:{tag} ref=\"{v}\"/>\n"),
        None => String::new(),
    };
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="{name}" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
{rx}{tx}{stage}</scxml>"##,
        rx = elem("rx-pool", rx_pool),
        tx = elem("tx-pool", tx_pool),
        stage = elem("stage-pool", stage_pool),
    )
}

/// A typo in `<sce:rx-pool ref>` must not silently switch the burst
/// validator off.
///
/// This is the same document set and the same deploy topology as
/// [`c13_burst_absorption_fires_through_orchestrator`] — 16 slots
/// against 50k pps, which that test pins as a refusal. The only edit
/// is one character in the pool ref. If the orchestrator accepts the
/// edited set, then a typo is a way to *pass* a deploy that the spec
/// says must fail, which is strictly worse than the typo being
/// reported.
#[test]
fn rx_pool_ref_typo_does_not_silently_disable_the_burst_validator() {
    use sce_build::mesh::deploy::parse_deploy_str;
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    let pool = write_doc(
        dir.path(),
        "rx_data_pool.scxml",
        &buffer_pool_default("rx_data_pool", 16, 1536),
    );
    // `rx_data_poool` — one keystroke away from the pool above.
    let link = write_doc(
        dir.path(),
        "udp_data.scxml",
        &link_with_rx_pool("udp_data", "rx_data_poool"),
    );

    let deploy = parse_deploy_str(&deploy_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            burst_pps: 50000
            rx_dispatch: isr_to_pool
"#,
    ))
    .expect("deploy parses");

    let err = match compile_scxml_with_imports(
        &[scxml.as_path()],
        &[pool.as_path(), link.as_path()],
        &template_dir(),
        Language::Rust,
        &default_options(),
        Some(&deploy),
    ) {
        Ok(_) => panic!(
            "a typo in <sce:rx-pool ref> made a refused deploy topology pass: \
             the same set with the ref spelled correctly fires \
             deploy/link-burst-absorption-insufficient"
        ),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::LinkPoolRefNotDeclared {
                link_name,
                pool_side,
                pool_ref,
                candidates,
            } => {
                assert_eq!(link_name, "udp_data");
                assert_eq!(pool_side, "rx");
                assert_eq!(pool_ref, "rx_data_poool");
                assert_eq!(
                    candidates,
                    vec!["rx_data_pool".to_string()],
                    "the repair candidates must be the build's declared buffer-pool names"
                );
            }
            other => panic!("expected ValidationError::LinkPoolRefNotDeclared, got: {other:?}"),
        },
        other => panic!("expected ForgeError::Validation, got: {other:?}"),
    }
}

/// One pool-ref site: the side label the diagnostic is expected to
/// report, paired with a builder that puts a ref on that side alone.
type PoolRefSite = (&'static str, fn(&str) -> String);

/// Each of the three pool-ref sites is joined against the build.
///
/// Driven per site rather than through one representative ref: a
/// validator that only walks `rx_pool` passes a single-ref test while
/// leaving two sites unguarded, and "the gate is green" would not
/// distinguish the two.
#[test]
fn every_pool_ref_site_is_joined_against_the_build() {
    let sites: [PoolRefSite; 3] = [
        ("rx", |r| link_with_pools("udp_data", Some(r), None, None)),
        ("tx", |r| link_with_pools("udp_data", None, Some(r), None)),
        ("stage", |r| {
            link_with_pools("udp_data", None, None, Some(r))
        }),
    ];

    for (expected_side, build_link) in sites {
        let dir = tempdir().expect("tempdir");
        let scxml = write_doc(
            dir.path(),
            "session_fsm.scxml",
            &statechart_minimal("session_fsm"),
        );
        let pool = write_doc(
            dir.path(),
            "rx_data_pool.scxml",
            &buffer_pool_default("rx_data_pool", 2000, 1536),
        );
        let link = write_doc(dir.path(), "udp_data.scxml", &build_link("nosuch_pool"));

        let err = match compile_scxml_with_imports(
            &[scxml.as_path()],
            &[pool.as_path(), link.as_path()],
            &template_dir(),
            Language::Rust,
            &default_options(),
            None,
        ) {
            Ok(_) => panic!("dangling <sce:{expected_side}-pool ref> must be refused"),
            Err(e) => e,
        };

        match err.error {
            ForgeError::Validation(boxed) => match *boxed {
                ValidationError::LinkPoolRefNotDeclared {
                    pool_side,
                    pool_ref,
                    ..
                } => {
                    assert_eq!(
                        pool_side, expected_side,
                        "the diagnostic must name the side that actually dangles"
                    );
                    assert_eq!(pool_ref, "nosuch_pool");
                }
                other => {
                    panic!("[{expected_side}] expected LinkPoolRefNotDeclared, got: {other:?}")
                }
            },
            other => panic!("[{expected_side}] expected ForgeError::Validation, got: {other:?}"),
        }
    }
}

/// The control for the two tests above: with every ref spelled
/// correctly the same shapes compile. Without this, a validator that
/// refused *every* link would satisfy both refusal tests.
#[test]
fn resolving_pool_refs_on_all_three_sites_still_compiles() {
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    let rx = write_doc(
        dir.path(),
        "rx_data_pool.scxml",
        &buffer_pool_default("rx_data_pool", 2000, 1536),
    );
    let tx = write_doc(
        dir.path(),
        "tx_data_pool.scxml",
        &buffer_pool_default("tx_data_pool", 2000, 1536),
    );
    let stage = write_doc(
        dir.path(),
        "stage_data_pool.scxml",
        &buffer_pool_default("stage_data_pool", 2000, 1536),
    );
    let link = write_doc(
        dir.path(),
        "udp_data.scxml",
        &link_with_pools(
            "udp_data",
            Some("rx_data_pool"),
            Some("tx_data_pool"),
            Some("stage_data_pool"),
        ),
    );

    let outputs = compile_scxml_with_imports(
        &[scxml.as_path()],
        &[rx.as_path(), tx.as_path(), stage.as_path(), link.as_path()],
        &template_dir(),
        Language::Rust,
        &default_options(),
        None,
    )
    .expect("every pool ref resolves ⇒ orchestrator succeeds");
    assert!(!outputs.is_empty(), "control must emit outputs");
}

/// A pool ref also resolves through the link's own `<sce:import>`,
/// with the pool document absent from the build's input list.
///
/// Both routes are genuine: the import form is how a link document
/// names a pool the caller did not hand to the orchestrator, and
/// `validate_link_pool_framer_resolution` already follows it to
/// compare slot sizes. Pinning it separately because a resolution
/// check written against the input list alone would reject this shape,
/// and the input-list tests above would not notice.
#[test]
fn a_pool_ref_resolves_through_the_links_own_import() {
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_minimal("session_fsm"),
    );
    // Written next to the link so the `src` resolves at parse time,
    // but deliberately NOT passed as a build input below.
    write_doc(
        dir.path(),
        "rx_data_pool.scxml",
        &buffer_pool_default("rx_data_pool", 2000, 1536),
    );
    let link = write_doc(
        dir.path(),
        "udp_data.scxml",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_data" version="1.0">
  <sce:import as="rx_data_pool" src="rx_data_pool.scxml" kind="buffer-pool"/>
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:rx-pool ref="rx_data_pool"/>
</scxml>"##,
    );

    let outputs = compile_scxml_with_imports(
        &[scxml.as_path()],
        // Link only — the pool reaches the build through the import.
        &[link.as_path()],
        &template_dir(),
        Language::Rust,
        &default_options(),
        None,
    )
    .expect("an imported pool is a resolved pool");
    assert!(!outputs.is_empty(), "import route must still emit outputs");
}
