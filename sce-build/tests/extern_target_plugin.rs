// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// watching-zenoh RFC §synth-5-I `<sce:extern>` target-plugin extension —
// end-to-end fixtures. Each test exercises plugin loading
// through a constructed `DeployConfig` + `compile_forge_with_deploy`,
// asserting on the surfaced diagnostic axis:
//
//   - Reject fixture: plugin YAML redefines a baseline symbol →
//     `extern/target-plugin-symbol-conflict` (spec line 1852
//     verbatim — the only spec-verbatim code on this surface).
//   - Reject fixture: plugin YAML missing on disk →
//     `io/filesystem` (existing code; plugin file IO failures ride
//     the generic forge `Io` axis — there is no plugin-specific
//     IO diagnostic code).
//   - Reject fixture: plugin YAML malformed →
//     `io/filesystem` (same axis; the plugin file is treated as
//     pipeline input outside the SCXML parser).
//   - Happy fixture: plugin YAML adds vendor symbols + forge SCXML
//     uses `<sce:extern name="sce_hw_sem_take" .../>` → compiles
//     successfully with the plugin entries reaching parser
//     validation through the `validate_extern_with_plugin` path.
//
// Fixtures use `tempfile::NamedTempFile` so the plugin path on disk
// has a real handle for `parse_target_plugin_yaml`'s I/O step;
// in-process YAML strings flow into [`forge::target_plugin::parse_target_plugin_str`]
// directly through the loader unit tests, not here.

use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::Language;
use sce_build::mesh::deploy::{DeployConfig, ExternSymbolsConfig};
use sce_build::DocumentLabel;
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

/// Construct a minimal `DeployConfig` whose `extern_symbols.target_plugin`
/// field points at the supplied path. Topology / partition / etc.
/// fields are absent — `compile_forge_with_deploy` does not require
/// them when no per-machine validation runs.
fn deploy_with_plugin(path: std::path::PathBuf) -> DeployConfig {
    DeployConfig {
        version: None,
        topology: HashMap::new(),
        discovery: None,
        partitions: None,
        distributability: None,
        extern_symbols: Some(ExternSymbolsConfig {
            target_plugin: Some(path),
        }),
        variant_defaults: std::collections::BTreeMap::new(),
    }
}

/// Wrap one or more `<sce:extern>` declarations in a minimal
/// `transform` kind SCXML — same shape used by the
/// `extern_intrinsic_registry.rs` fixtures.
fn fixture_transform_with_externs(extern_decls: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="extern_b_test">
  {extern_decls}
  <datamodel>
    <data id="x" sce:type="uint32" sce:direction="in"/>
    <data id="y" sce:type="uint32" sce:direction="out" expr="x"/>
  </datamodel>
</scxml>
"##
    )
}

/// Write `content` to a temp file with a `.yaml` suffix and return
/// the handle. Caller must keep the handle alive for the duration of
/// the test (drop closes + deletes the file).
fn temp_plugin_file(content: &str) -> NamedTempFile {
    let mut tmp = tempfile::Builder::new()
        .prefix("sce-target-plugin-")
        .suffix(".yaml")
        .tempfile()
        .expect("create temp file");
    tmp.as_file_mut()
        .write_all(content.as_bytes())
        .expect("write plugin yaml");
    tmp
}

#[test]
fn happy_path_vendor_symbol_resolves_through_plugin() {
    // Plugin declares a vendor symbol; SCXML uses it.
    let plugin_yaml = r#"
symbols:
  - name: sce_hw_sem_take
    sig: "(u32) -> bool"
    abi: c
    purpose: cross-core-mutex
"#;
    let plugin = temp_plugin_file(plugin_yaml);
    let cfg = deploy_with_plugin(plugin.path().to_path_buf());

    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_hw_sem_take" sig="(u32) -> bool" abi="c"/>"##,
    );
    let result = sce_build::compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("extern_b_happy"),
        Language::Rust,
        Some(&cfg),
        None,
    );
    if let Err(e) = &result {
        panic!("plugin-extension symbol must compile: {e:?}");
    }
}

#[test]
fn reject_baseline_shadow_fires_spec_line_1852_code() {
    // Plugin tries to redefine `sce_atomic_load_acquire_u32` (a
    // baseline symbol). Spec line 1852: "target plugin redefines a
    // core whitelist symbol" → `extern/target-plugin-symbol-conflict`.
    let plugin_yaml = r#"
symbols:
  - name: sce_atomic_load_acquire_u32
    sig: "(*const u32) -> u32"
    abi: c
"#;
    let plugin = temp_plugin_file(plugin_yaml);
    let plugin_path = plugin.path().to_path_buf();
    let cfg = deploy_with_plugin(plugin_path.clone());

    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>"##,
    );
    let err = match sce_build::compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("extern_b_conflict"),
        Language::Rust,
        Some(&cfg),
        None,
    ) {
        Ok(_) => panic!("baseline shadow must reject"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::ExternTargetPluginSymbolConflict {
                name,
                plugin_path: emitted_path,
            } => {
                assert_eq!(name, "sce_atomic_load_acquire_u32");
                // The diagnostic carries the plugin path so consumers can
                // open the offending YAML file.
                assert_eq!(
                    emitted_path,
                    plugin_path.display().to_string(),
                    "plugin path must surface in diagnostic",
                );
            }
            other => panic!("expected ExternTargetPluginSymbolConflict, got {other:?}"),
        },
        other => panic!("expected ExternTargetPluginSymbolConflict, got {other:?}"),
    }
}

#[test]
fn reject_baseline_shadow_irrespective_of_signature_match() {
    // Locked semantics: even when the plugin's sig matches baseline
    // exactly, redefinition is disallowed (spec line 1852 trigger is
    // "redefines", not "redefines incompatibly"). C5 (spec §synth-5-E line
    // 1548) makes the cache-maintenance trio FSM-driven and rejects
    // its author authoring at parse time before this check could
    // fire. Substitute `sce_atomic_load_acquire_u32` so the plugin
    // shadow-rejection semantic remains exercised on a baseline
    // symbol that authors are actually allowed to write.
    let plugin_yaml = r#"
symbols:
  - name: sce_atomic_load_acquire_u32
    sig: "(*const u32) -> u32"
    abi: c
"#;
    let plugin = temp_plugin_file(plugin_yaml);
    let cfg = deploy_with_plugin(plugin.path().to_path_buf());

    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>"##,
    );
    let err = match sce_build::compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("extern_b_same_sig_conflict"),
        Language::Rust,
        Some(&cfg),
        None,
    ) {
        Ok(_) => panic!("redefinition must be rejected even when sig matches"),
        Err(e) => e,
    };

    assert!(matches!(
        err.error,
        ForgeError::Validation(ref boxed)
            if matches!(**boxed, ValidationError::ExternTargetPluginSymbolConflict { .. }),
    ));
}

#[test]
fn reject_missing_plugin_file_surfaces_through_io_axis() {
    // Plugin-file IO failures route through the existing
    // `io/filesystem` axis — no plugin-specific IO diagnostic
    // code exists.
    let cfg = deploy_with_plugin(std::path::PathBuf::from(
        "/path/that/does/not/exist/sce-target-plugin.yaml",
    ));

    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>"##,
    );
    let err = match sce_build::compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("extern_b_missing_plugin"),
        Language::Rust,
        Some(&cfg),
        None,
    ) {
        Ok(_) => panic!("missing plugin file must reject"),
        Err(e) => e,
    };

    assert!(
        matches!(err.error, ForgeError::Io { .. }),
        "expected ForgeError::Io, got {:?}",
        err.error,
    );
}

#[test]
fn reject_malformed_plugin_yaml_surfaces_through_io_axis() {
    let plugin_yaml = "symbols: [not-a-mapping]";
    let plugin = temp_plugin_file(plugin_yaml);
    let cfg = deploy_with_plugin(plugin.path().to_path_buf());

    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>"##,
    );
    let err = match sce_build::compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("extern_b_malformed_plugin"),
        Language::Rust,
        Some(&cfg),
        None,
    ) {
        Ok(_) => panic!("malformed plugin YAML must reject"),
        Err(e) => e,
    };

    assert!(
        matches!(err.error, ForgeError::Io { .. }),
        "expected ForgeError::Io with InvalidData, got {:?}",
        err.error,
    );
}

#[test]
fn happy_path_baseline_and_plugin_externs_coexist() {
    let plugin_yaml = r#"
symbols:
  - name: sce_hw_mbox_send
    sig: "(u32, *const u8, usize) -> bool"
    abi: c
    purpose: cross-core-notify
"#;
    let plugin = temp_plugin_file(plugin_yaml);
    let cfg = deploy_with_plugin(plugin.path().to_path_buf());

    // SCXML carries one baseline extern + one plugin extern.
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>
  <sce:extern name="sce_hw_mbox_send" sig="(u32, *const u8, usize) -> bool" abi="c"/>"##,
    );
    let result = sce_build::compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("extern_b_mixed"),
        Language::Rust,
        Some(&cfg),
        None,
    );
    if let Err(e) = &result {
        panic!("baseline + plugin externs must coexist: {e:?}");
    }
}

#[test]
fn deploy_without_plugin_preserves_baseline_only_semantics() {
    // No `extern_symbols` field on deploy → baseline-whitelist
    // semantics unchanged. Vendor symbol →
    // `extern/symbol-not-in-whitelist`.
    let cfg = DeployConfig {
        version: None,
        topology: HashMap::new(),
        discovery: None,
        partitions: None,
        distributability: None,
        extern_symbols: None,
        variant_defaults: std::collections::BTreeMap::new(),
    };

    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_hw_sem_take" sig="(u32) -> bool" abi="c"/>"##,
    );
    let err = match sce_build::compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("extern_b_no_plugin"),
        Language::Rust,
        Some(&cfg),
        None,
    ) {
        Ok(_) => panic!("no plugin loaded ⇒ vendor symbol unknown"),
        Err(e) => e,
    };

    assert!(matches!(
        err.error,
        ForgeError::Validation(ref boxed)
            if matches!(**boxed, ValidationError::ExternSymbolNotInWhitelist { .. }),
    ));
}
