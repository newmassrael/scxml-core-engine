//! C5 — Cache maintenance intrinsics wired into §synth-5-E codegen.
//!
//! Pins the spec-driven 6-code surface (RFC §synth-5-E lines 1543-1545 +
//! 1548 + 1552-1553) + the auto-inject + 2 author-visible cache call
//! sites + sidecar pinning matrix. Per `feedback_silently_broken_hooks.md`
//! every conditional emission path needs an explicit fixture.

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located, ValidationError};
use sce_build::forge::model::ForgeDocument;
use sce_build::generator::Language;
use sce_build::mesh;
use sce_build::{compile_forge_with_deploy, DocumentLabel};

/// Render a buffer-pool with the given cache-policy + alignment +
/// slot-size against the supplied deploy.yaml fixture and return the
/// compile result.
///
/// `GeneratedOutput` doesn't implement `Debug`, so the negative tests
/// use `expect_compile_err` (below) instead of `Result::expect_err`.
fn compile_pool_with_deploy(
    cache_policy: &str,
    alignment: u32,
    slot_size: u32,
    deploy_yaml: &str,
    machine: &str,
    language: Language,
) -> Result<sce_build::generator::GeneratedOutput, Located<ForgeError>> {
    let scxml = format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>{slot_size}</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>{alignment}</sce:alignment>
  <sce:cache-policy>{cache_policy}</sce:cache-policy>
</scxml>"##,
    );
    let deploy = mesh::deploy::parse_deploy_str(deploy_yaml).expect("deploy.yaml parses");
    let label = DocumentLabel {
        identifier: "rx_pool_sram1",
        diagnostic_label: "rx_pool_sram1.scxml",
    };
    compile_forge_with_deploy(&scxml, label, language, Some(&deploy), Some(machine))
}

/// Variant of `Result::expect_err` for `GeneratedOutput`, which lacks
/// `Debug` so the stock helper does not compile.
fn expect_compile_err(
    result: Result<sce_build::generator::GeneratedOutput, Located<ForgeError>>,
    msg: &str,
) -> Located<ForgeError> {
    match result {
        Ok(_) => panic!("{msg}"),
        Err(e) => e,
    }
}

/// Standard deploy.yaml fixture: speculative-prefetch core (M7+/A-class)
/// with 32-byte cache lines and a 64 KiB sram1 region.
fn deploy_speculative_core() -> &'static str {
    r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: pool_owner.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
          has_speculative_prefetch: true
        memory:
          sram_regions:
            sram1:
              base: 0x08000000
              size: 65536
"#
}

/// Standard deploy.yaml fixture: non-speculative core (M3/M4) with
/// 32-byte cache lines.
fn deploy_non_speculative_core() -> &'static str {
    r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: pool_owner.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
          has_speculative_prefetch: false
        memory:
          sram_regions:
            sram1:
              base: 0x08000000
              size: 65536
"#
}

/// Standard deploy.yaml fixture: no-D-cache core (M0/M0+/M3/M4 without
/// cache).
fn deploy_no_dcache_core() -> &'static str {
    r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: pool_owner.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: false
        memory:
          sram_regions:
            sram1:
              base: 0x08000000
              size: 65536
"#
}

// ────────────────────────────────────────────────────────────────────
// Happy-path matrix — cache-policy × has_speculative_prefetch
// ────────────────────────────────────────────────────────────────────

#[test]
fn cache_maintain_with_speculative_prefetch_emits_both_edges_rust() {
    // Spec §5.E lines 1186-1198: cache-clean before TX hand-off (always
    // when maintain) AND pre-arm cache-invalidate before RX arm
    // (gated on has_speculative_prefetch=true).
    let out = compile_pool_with_deploy(
        "maintain",
        32,
        64,
        deploy_speculative_core(),
        "mcu_node",
        Language::Rust,
    )
    .expect("speculative core + maintain pool compiles");
    let body = &out.files.first().expect("primary rust file emitted").1;
    assert!(
        body.contains("sce_dcache_clean_by_addr("),
        "TX-side cache-clean must emit when cache-policy=maintain:\n{body}"
    );
    assert!(
        body.contains("sce_dcache_invalidate_by_addr("),
        "RX-side pre-arm invalidate must emit when has_speculative_prefetch=true:\n{body}"
    );
}

#[test]
fn cache_maintain_with_speculative_prefetch_emits_both_edges_c11() {
    let out = compile_pool_with_deploy(
        "maintain",
        32,
        64,
        deploy_speculative_core(),
        "mcu_node",
        Language::C11,
    )
    .expect("speculative core + maintain pool compiles");
    let body = &out.files.first().expect("primary c11 header emitted").1;
    assert!(
        body.contains("sce_dcache_clean_by_addr("),
        "C11 TX cache-clean missing:\n{body}"
    );
    assert!(
        body.contains("sce_dcache_invalidate_by_addr("),
        "C11 RX pre-arm invalidate missing:\n{body}"
    );
}

#[test]
fn cache_maintain_without_speculative_prefetch_emits_only_clean_rust() {
    // Spec lines 1199-1212: M3/M4 (no speculative prefetch) doesn't
    // need pre-arm RX invalidate; emit just the TX cache-clean.
    let out = compile_pool_with_deploy(
        "maintain",
        32,
        64,
        deploy_non_speculative_core(),
        "mcu_node",
        Language::Rust,
    )
    .expect("non-speculative core + maintain pool compiles");
    let body = &out.files.first().expect("primary rust file emitted").1;
    assert!(body.contains("sce_dcache_clean_by_addr("));
    assert!(
        !body.contains("sce_dcache_invalidate_by_addr("),
        "RX pre-arm invalidate must NOT emit when has_speculative_prefetch=false:\n{body}"
    );
}

#[test]
fn cache_non_cacheable_emits_no_cache_calls() {
    let out = compile_pool_with_deploy(
        "non-cacheable",
        32,
        64,
        deploy_speculative_core(),
        "mcu_node",
        Language::Rust,
    )
    .expect("non-cacheable pool compiles");
    let body = &out.files.first().expect("primary rust file emitted").1;
    assert!(
        !body.contains("sce_dcache_clean_by_addr("),
        "non-cacheable must emit no cache calls:\n{body}"
    );
    assert!(!body.contains("sce_dcache_invalidate_by_addr("));
}

#[test]
fn cache_none_emits_no_cache_calls() {
    let out = compile_pool_with_deploy(
        "none",
        32,
        64,
        deploy_no_dcache_core(),
        "mcu_node",
        Language::Rust,
    )
    .expect("cache-policy=none on no-dcache core compiles");
    let body = &out.files.first().expect("primary rust file emitted").1;
    assert!(!body.contains("sce_dcache_clean_by_addr("));
    assert!(!body.contains("sce_dcache_invalidate_by_addr("));
}

// ────────────────────────────────────────────────────────────────────
// Sidecar auto-inject pinning ("same sidecar" lock)
// ────────────────────────────────────────────────────────────────────

#[test]
fn cache_maintain_auto_injects_three_externs_in_sidecar_rust() {
    // Spec lines 1736-1740: `sce_dcache_clean_by_addr`,
    // `sce_dcache_invalidate_by_addr`, `sce_dcache_clean_invalidate_by_addr`.
    // All 3 visible in `<snake>_externs.rs` for deploy
    // review transparency (auto-injected by parser hook when
    // cache-policy=maintain).
    let out = compile_pool_with_deploy(
        "maintain",
        32,
        64,
        deploy_speculative_core(),
        "mcu_node",
        Language::Rust,
    )
    .expect("maintain pool compiles");
    let sidecar = out
        .files
        .iter()
        .find(|(n, _)| n.ends_with("_externs.rs"))
        .map(|(_, b)| b.as_str())
        .expect("rust externs sidecar must emit when cache-policy=maintain");
    for sym in [
        "sce_dcache_clean_by_addr",
        "sce_dcache_invalidate_by_addr",
        "sce_dcache_clean_invalidate_by_addr",
    ] {
        assert!(
            sidecar.contains(&format!("pub fn {sym}")),
            "auto-inject of {sym} missing from rust sidecar:\n{sidecar}"
        );
    }
}

#[test]
fn cache_non_cacheable_emits_no_externs_sidecar() {
    // Sidecar emit is gated on non-empty externs; when no
    // author externs and cache-policy != maintain, no sidecar emits.
    let out = compile_pool_with_deploy(
        "non-cacheable",
        32,
        64,
        deploy_speculative_core(),
        "mcu_node",
        Language::Rust,
    )
    .expect("non-cacheable pool compiles");
    assert!(
        !out.files.iter().any(|(n, _)| n.ends_with("_externs.rs")),
        "non-cacheable must not auto-inject cache externs"
    );
}

// ────────────────────────────────────────────────────────────────────
// Negative diagnostics — one fixture per spec-named code
// ────────────────────────────────────────────────────────────────────

#[test]
fn mem_cache_line_alignment_fires_when_alignment_smaller_than_dcache_line_size() {
    // Spec line 1544: alignment=16 < dcache_line_size=32 with maintain
    let err = expect_compile_err(
        compile_pool_with_deploy(
            "maintain",
            16,
            64,
            deploy_speculative_core(),
            "mcu_node",
            Language::Rust,
        ),
        "alignment-smaller violation must reject",
    );
    let diags = err.to_diagnostics();
    assert_eq!(diags.len(), 1);
    assert!(matches!(
        diags[0].code,
        DiagnosticCode::MemCacheLineAlignment
    ));
    assert!(diags[0].message.contains("16"));
    assert!(diags[0].message.contains("32"));
}

#[test]
fn mem_cache_line_alignment_passes_when_alignment_equals_dcache_line_size() {
    // Boundary case — alignment == dcache_line_size is legal.
    let out = compile_pool_with_deploy(
        "maintain",
        32,
        64,
        deploy_speculative_core(),
        "mcu_node",
        Language::Rust,
    );
    assert!(out.is_ok(), "alignment == dcache_line_size must pass");
}

#[test]
fn mem_slot_size_not_cache_line_multiple_fires_when_remainder_nonzero() {
    // Spec line 1545: slot_size=100, dcache_line_size=32, remainder=4.
    let err = expect_compile_err(
        compile_pool_with_deploy(
            "maintain",
            32,
            100,
            deploy_speculative_core(),
            "mcu_node",
            Language::Rust,
        ),
        "slot-size remainder violation must reject",
    );
    let diags = err.to_diagnostics();
    assert_eq!(diags.len(), 1);
    assert!(matches!(
        diags[0].code,
        DiagnosticCode::MemSlotSizeNotCacheLineMultiple
    ));
    assert!(diags[0].message.contains("100"));
    assert!(diags[0].message.contains("32"));
    assert!(diags[0].message.contains("128")); // next_multiple
}

#[test]
fn mem_cache_policy_unsupported_on_no_dcache_core_for_maintain() {
    // Spec line 1543: maintain on has_dcache=false rejects.
    let err = expect_compile_err(
        compile_pool_with_deploy(
            "maintain",
            32,
            64,
            deploy_no_dcache_core(),
            "mcu_node",
            Language::Rust,
        ),
        "maintain on no-dcache core must reject",
    );
    let diags = err.to_diagnostics();
    assert_eq!(diags.len(), 1);
    assert!(matches!(
        diags[0].code,
        DiagnosticCode::MemCachePolicyUnsupportedOnNoDcacheCore
    ));
    assert!(diags[0].message.contains("maintain"));
}

#[test]
fn mem_cache_policy_unsupported_on_no_dcache_core_for_non_cacheable() {
    // Spec line 1543 covers both `maintain` and `non-cacheable` on
    // has_dcache=false — both are meaningless, only `none` is legal.
    let err = expect_compile_err(
        compile_pool_with_deploy(
            "non-cacheable",
            32,
            64,
            deploy_no_dcache_core(),
            "mcu_node",
            Language::Rust,
        ),
        "non-cacheable on no-dcache core must reject",
    );
    let diags = err.to_diagnostics();
    assert!(matches!(
        diags[0].code,
        DiagnosticCode::MemCachePolicyUnsupportedOnNoDcacheCore
    ));
    assert!(diags[0].message.contains("non-cacheable"));
}

#[test]
fn pool_speculative_prefetch_flag_missing_when_maintain_and_has_dcache_true() {
    // Spec line 1553: has_dcache=true + maintain pool, but
    // has_speculative_prefetch unset.
    let deploy_yaml = r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: pool_owner.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
        memory:
          sram_regions:
            sram1:
              base: 0x08000000
              size: 65536
"#;
    let err = expect_compile_err(
        compile_pool_with_deploy("maintain", 32, 64, deploy_yaml, "mcu_node", Language::Rust),
        "missing has_speculative_prefetch must reject",
    );
    let diags = err.to_diagnostics();
    assert!(matches!(
        diags[0].code,
        DiagnosticCode::PoolSpeculativePrefetchFlagMissing
    ));
    assert!(diags[0].message.contains("mcu_node"));
    assert!(diags[0].message.contains("rx_pool_sram1"));
}

#[test]
fn pool_speculative_prefetch_silent_when_pool_is_none() {
    // The diagnostic is per-pool: a `cache-policy: none` pool does not
    // require has_speculative_prefetch even when has_dcache=true.
    let deploy_yaml = r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: pool_owner.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
        memory:
          sram_regions:
            sram1:
              base: 0x08000000
              size: 65536
"#;
    let out = compile_pool_with_deploy("none", 32, 64, deploy_yaml, "mcu_node", Language::Rust);
    assert!(
        out.is_ok(),
        "cache-policy=none must skip speculative-prefetch requirement"
    );
}

#[test]
fn pool_cache_maintenance_misplaced_for_each_trio_symbol() {
    // Spec line 1548 + lines 1222-1227: author authoring of the cache
    // trio via `<sce:extern>` is forbidden. Fires at parse time
    // before the §5.I baseline whitelist validator.
    use sce_build::forge::parser::parse_forge_with_imports;

    for sym in [
        "sce_dcache_clean_by_addr",
        "sce_dcache_invalidate_by_addr",
        "sce_dcache_clean_invalidate_by_addr",
    ] {
        let scxml = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="t" version="1.0">
  <sce:extern name="{sym}" sig="(*const c_void, usize)" abi="c"/>
  <datamodel><data id="x" sce:type="uint32" sce:direction="in"/></datamodel>
</scxml>"##
        );
        let label = DocumentLabel {
            identifier: "t",
            diagnostic_label: "t.scxml",
        };
        let err = parse_forge_with_imports(&scxml, label).expect_err("must reject cache trio");
        let diags = err.to_diagnostics();
        assert_eq!(diags.len(), 1, "single diagnostic for symbol {sym}");
        assert!(
            matches!(diags[0].code, DiagnosticCode::PoolCacheMaintenanceMisplaced),
            "symbol {sym} must trigger PoolCacheMaintenanceMisplaced; got {:?}",
            diags[0].code
        );
        assert!(diags[0].message.contains(sym));
    }
}

// ────────────────────────────────────────────────────────────────────
// Codegen-invariant force-fixture (`pool/cache-pre-arm-invalidate-missing-on-speculative-core`)
// ────────────────────────────────────────────────────────────────────

#[test]
fn pool_cache_pre_arm_invalidate_missing_force_fixture() {
    // Spec line 1552 codegen-invariant. In normal use the template
    // emits the pre-arm invalidate by construction; this fixture
    // drives the ValidationError → Diagnostic → DiagnosticCode pipeline
    // directly so the diagnostic has a live consumer per
    // `feedback_silently_broken_hooks.md` and the wire format /
    // convert path stays byte-stable. Mirrors the β
    // `mem/inter-pool-padding-not-emitted` codegen self-check shape.
    let err: ForgeError = ValidationError::PoolCachePreArmInvalidateMissingOnSpeculativeCore {
        name: "rx_pool_sram1".into(),
        backend: "rust".into(),
    }
    .into();
    let located: Located<ForgeError> = Located::new(err, "rx_pool_sram1.scxml", None, None);
    let diags = located.to_diagnostics();
    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert!(matches!(
        d.code,
        DiagnosticCode::PoolCachePreArmInvalidateMissingOnSpeculativeCore
    ));
    assert!(d.message.contains("rx_pool_sram1"));
    assert!(d.message.contains("rust"));
    assert!(d.message.contains("free → dma-armed-rx"));
    assert!(d.message.contains("§5.E lines 1186-1198"));
}

// ────────────────────────────────────────────────────────────────────
// Deploy-unaware path uses conservative defaults
// ────────────────────────────────────────────────────────────────────

#[test]
fn cache_maintain_deploy_unaware_emits_clean_only() {
    // sce_codegen / compile_forge_with_imports has no platform info →
    // ForgeCompileOptions::cache_platform = None → conservative
    // defaults → cache-clean emits (cache_maintain=true), pre-arm
    // invalidate skipped (has_speculative_prefetch=false default).
    use sce_build::{compile_forge_with_imports, ForgeCompileOptions};
    use std::path::Path;

    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>64</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
</scxml>"##;
    let label = DocumentLabel {
        identifier: "rx_pool_sram1",
        diagnostic_label: "rx_pool_sram1.scxml",
    };
    let out = compile_forge_with_imports(
        scxml,
        label,
        Language::Rust,
        Path::new("."),
        &ForgeCompileOptions::default(),
    )
    .expect("deploy-unaware compile of maintain pool");
    let body = &out.files.first().unwrap().1;
    assert!(
        body.contains("sce_dcache_clean_by_addr("),
        "deploy-unaware maintain still emits cache-clean:\n{body}"
    );
    assert!(
        !body.contains("sce_dcache_invalidate_by_addr("),
        "deploy-unaware path defaults has_speculative_prefetch=false:\n{body}"
    );
}

// ────────────────────────────────────────────────────────────────────
// Document shape sanity
// ────────────────────────────────────────────────────────────────────

#[test]
fn cache_maintain_document_kind_is_buffer_pool() {
    // Sanity: the auto-inject hook predicate in parse_forge_with_imports
    // matches only ForgeDocument::BufferPool. Other document kinds
    // (transform, codec, etc.) never auto-inject even if their authors
    // happened to mention `cache-policy` somewhere. This test doubles
    // as a reminder for future kind additions.
    use sce_build::forge::parser::parse_forge_with_imports;
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="p" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>64</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
</scxml>"##;
    let label = DocumentLabel {
        identifier: "p",
        diagnostic_label: "p.scxml",
    };
    let parsed = parse_forge_with_imports(scxml, label)
        .expect("parses")
        .expect("buffer-pool produces ParsedForge");
    assert!(matches!(parsed.document, ForgeDocument::BufferPool(_)));
    assert_eq!(parsed.externs.len(), 3);
}
