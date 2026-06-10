// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// `<sce:extern>` target-plugin loader — watching-zenoh RFC §synth-5-I.
// Spec lines 1760-1787 verbatim: architectures may extend the §synth-5-I
// whitelist through a target plugin declared via deploy.yaml
// (`extern_symbols.target_plugin: <path>`). The plugin file is a YAML
// document listing additional symbols with signatures.
//
// Path-pointed YAML file, single plugin per deploy.
// Plugin entries are *additive*; redefining a §synth-5-I
// baseline symbol surfaces as `extern/target-plugin-symbol-conflict`
// (spec line 1852 verbatim — "target plugin redefines a core whitelist
// symbol"). Repair shape: plugin author renames the
// conflicting entry to a non-baseline name; SCE cannot synthesize a
// candidate so the diagnostic carries no `Fix` payload.
//
// File format (spec lines 1772-1787):
//
//   symbols:
//     - name: sce_hw_sem_take
//       sig: "(u32) -> bool"
//       abi: c
//       purpose: cross-core-mutex
//     - name: sce_hw_sem_release
//       sig: "(u32)"
//       abi: c
//       purpose: cross-core-mutex
//
// Plugin-extension axes that have no consumer yet (`linker_flavor`,
// `fuzz_coverage_transport`, `extern/ordering-insufficient-for-cross-core`)
// are accepted as opaque YAML keys here so a later change can lift
// them into typed fields without a schema break — `serde(deny_unknown_fields)` on
// [`TargetPluginFile`] would prevent forward-compat plugin YAML files
// from loading on a current sce-build, so the top level intentionally
// allows additional fields.

use crate::forge::intrinsic_registry::{lookup_symbol, Abi};
use serde::Deserialize;
use std::path::Path;

/// Owned counterpart to [`crate::forge::intrinsic_registry::Symbol`] for
/// plugin-loaded entries. The baseline registry uses `&'static str`
/// because its data is compile-time-known; plugin entries are read from
/// disk at build time so their strings live for the loader's lifetime.
///
/// The loader returns a `Vec<PluginSymbol>` that the validator threads
/// alongside the baseline — composition happens at lookup time
/// ([`crate::forge::extern_validator::validate_extern_with_plugin`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginSymbol {
    /// Vendor-specific symbol name (e.g. `sce_hw_sem_take` for STM32H7
    /// HSEM). Lookup key against `<sce:extern name="...">` after the
    /// baseline lookup misses.
    pub name: String,
    /// Canonical signature (compared exactly against `<sce:extern sig>`
    /// — same byte-equality discipline as baseline entries).
    pub sig: String,
    /// Required ABI (closed two-element set).
    pub abi: Abi,
    /// Free-form purpose tag (optional). Surfaces in
    /// diagnostic repair guidance when this symbol appears in a
    /// `Fix::ReplaceOneOf` candidate list (future atomic).
    pub purpose: Option<String>,
    /// Crate that provides the implementation (optional).
    /// `None` means the plugin author defers to the deploy's
    /// downstream Cargo.toml to resolve the symbol — typical case for
    /// vendor crates already on the dependency tree.
    pub crate_name: Option<String>,
}

/// Top-level YAML structure emitted by a target plugin file.
/// Forward-compat: additional keys (`linker_flavor`,
/// `linker_fragment_path`, `fuzz_coverage_transport`) load as opaque
/// `Value` so a later change can lift them into typed fields without
/// breaking existing plugins on rotated sce-build releases.
#[derive(Debug, Clone, Deserialize)]
struct TargetPluginFile {
    /// Required list of vendor-specific symbol declarations.
    symbols: Vec<PluginSymbolEntry>,
    /// Forward-compat slot — captured but not consumed. None of
    /// these axes has a typed lift yet (not implemented until a
    /// consumer needs them):
    /// - `linker_flavor` (spec lines 1820-1854)
    /// - `linker_fragment_path`
    /// - `fuzz_coverage_transport` (spec lines 1856-1864; the spec
    ///   itself defers the F4 transport — no implementation exists)
    #[serde(flatten, default)]
    _extras: std::collections::BTreeMap<String, serde_yaml_ng::Value>,
}

/// Single `symbols:` list entry as it appears in YAML. Owned strings
/// because the YAML deserializer borrows from a buffer that lives only
/// during parse — converted to [`PluginSymbol`] post-validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginSymbolEntry {
    name: String,
    sig: String,
    abi: String,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default, rename = "crate")]
    crate_name: Option<String>,
}

/// Plugin-loader failure modes. The variants partition into IO errors
/// (read / parse) and semantic errors (unknown ABI / baseline conflict).
/// The semantic [`TargetPluginLoadError::BaselineConflict`] arm carries
/// the `extern/target-plugin-symbol-conflict` (spec line 1852 verbatim)
/// diagnostic when surfaced by [`crate::lib`]'s `compile_forge_with_deploy`.
#[derive(Debug, thiserror::Error)]
pub enum TargetPluginLoadError {
    /// Plugin path could not be opened (typo in deploy.yaml,
    /// permissions, or the file was deleted between deploy.yaml authoring
    /// and `sce-codegen` invocation). Wraps the underlying `io::Error`.
    #[error("read target plugin {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// Plugin YAML failed to parse — malformed structure, unknown
    /// fields on a symbol entry (`deny_unknown_fields` on
    /// [`PluginSymbolEntry`]), or a missing required key
    /// (`name`/`sig`/`abi`).
    #[error("parse target plugin YAML at {path}: {source}")]
    Yaml {
        path: String,
        #[source]
        source: serde_yaml_ng::Error,
    },
    /// Plugin entry's `abi` field is outside the closed set
    /// `[c, rust]`. Distinct from the runtime
    /// `extern/abi-mismatch` diagnostic — that fires on
    /// `<sce:extern>` referencing a registry symbol with a wrong ABI;
    /// this fires at plugin LOAD time when the plugin file itself
    /// declares an unparseable ABI.
    #[error(
        "target plugin {path} symbol `{name}` declares unknown ABI `{abi}`; \
         only `c` and `rust` are accepted (closed set)"
    )]
    UnknownAbi {
        path: String,
        name: String,
        abi: String,
    },
    /// Plugin entry's `name` field already exists in the §synth-5-I baseline
    /// registry. Surfaced as `extern/target-plugin-symbol-conflict`
    /// (spec line 1852 verbatim) — additive-composition
    /// lock: plugins extend, never override.
    #[error(
        "target plugin {path} symbol `{name}` redefines a baseline whitelist \
         entry (additive rule — plugins extend, not override)"
    )]
    BaselineConflict { path: String, name: String },
}

/// Load a target plugin YAML and return the validated symbol list.
///
/// Validation pipeline:
/// 1. Read file from disk → [`TargetPluginLoadError::ReadFile`] on IO failure.
/// 2. Deserialize YAML → [`TargetPluginLoadError::Yaml`] on shape error.
/// 3. Parse `abi` field via [`Abi::from_attr`] → `UnknownAbi` if outside
///    the closed set.
/// 4. Check `name` is not in
///    [`crate::forge::intrinsic_registry::BASELINE_SYMBOLS`] →
///    `BaselineConflict` if so (spec line 1852 trigger).
/// 5. Return `Vec<PluginSymbol>` in source order (plugin authors expect
///    deterministic ordering matching their YAML).
///
/// The conflict check runs against the §synth-5-I 101-entry baseline using
/// [`lookup_symbol`] — exactly the same key the runtime
/// `<sce:extern>` validator uses, so any baseline-name change propagates
/// to plugin-conflict detection without a separate code path.
pub fn parse_target_plugin_yaml(path: &Path) -> Result<Vec<PluginSymbol>, TargetPluginLoadError> {
    let content = std::fs::read_to_string(path).map_err(|e| TargetPluginLoadError::ReadFile {
        path: path.display().to_string(),
        source: e,
    })?;
    parse_target_plugin_str(&content, path)
}

/// Parse a target plugin from an in-memory YAML string. Splits the
/// IO step from semantic validation so unit tests can exercise the
/// validator without writing temp files. The `path` argument carries
/// through to error messages so failure surfacing names the YAML
/// source even when the bytes are inlined in a fixture.
pub fn parse_target_plugin_str(
    content: &str,
    path: &Path,
) -> Result<Vec<PluginSymbol>, TargetPluginLoadError> {
    let path_str = path.display().to_string();

    let parsed: TargetPluginFile =
        serde_yaml_ng::from_str(content).map_err(|e| TargetPluginLoadError::Yaml {
            path: path_str.clone(),
            source: e,
        })?;

    let mut symbols = Vec::with_capacity(parsed.symbols.len());
    for entry in parsed.symbols {
        // Closed-set ABI lookup (delegates to the same
        // registry helper the runtime `<sce:extern>` validator uses,
        // so future ABI extensions land in one place).
        let abi = Abi::from_attr(&entry.abi).ok_or_else(|| TargetPluginLoadError::UnknownAbi {
            path: path_str.clone(),
            name: entry.name.clone(),
            abi: entry.abi.clone(),
        })?;
        // Additive-composition lock: a plugin entry
        // whose name appears in the baseline triggers
        // `extern/target-plugin-symbol-conflict` regardless of
        // whether the sig matches — same name = redefinition per
        // spec line 1852.
        if lookup_symbol(&entry.name).is_some() {
            return Err(TargetPluginLoadError::BaselineConflict {
                path: path_str.clone(),
                name: entry.name,
            });
        }
        symbols.push(PluginSymbol {
            name: entry.name,
            sig: entry.sig,
            abi,
            purpose: entry.purpose,
            crate_name: entry.crate_name,
        });
    }
    Ok(symbols)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> &Path {
        Path::new(s)
    }

    #[test]
    fn happy_path_loads_two_vendor_symbols() {
        let yaml = r#"
symbols:
  - name: sce_hw_sem_take
    sig: "(u32) -> bool"
    abi: c
    purpose: cross-core-mutex
  - name: sce_hw_sem_release
    sig: "(u32)"
    abi: c
"#;
        let result = parse_target_plugin_str(yaml, p("plugin.yaml")).expect("parse ok");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "sce_hw_sem_take");
        assert_eq!(result[0].sig, "(u32) -> bool");
        assert_eq!(result[0].abi, Abi::C);
        assert_eq!(result[0].purpose.as_deref(), Some("cross-core-mutex"));
        assert_eq!(result[1].name, "sce_hw_sem_release");
        assert_eq!(result[1].purpose, None);
    }

    #[test]
    fn unknown_abi_rejected() {
        let yaml = r#"
symbols:
  - name: sce_hw_sem_take
    sig: "()"
    abi: system
"#;
        let err = parse_target_plugin_str(yaml, p("plugin.yaml")).unwrap_err();
        match err {
            TargetPluginLoadError::UnknownAbi { name, abi, .. } => {
                assert_eq!(name, "sce_hw_sem_take");
                assert_eq!(abi, "system");
            }
            other => panic!("expected UnknownAbi, got {other:?}"),
        }
    }

    #[test]
    fn baseline_conflict_rejected_same_sig() {
        // Even with the *same* sig as baseline, redefinition is
        // disallowed per the additive-composition lock.
        let yaml = r#"
symbols:
  - name: sce_atomic_load_acquire_u32
    sig: "(*const u32) -> u32"
    abi: c
"#;
        let err = parse_target_plugin_str(yaml, p("plugin.yaml")).unwrap_err();
        match err {
            TargetPluginLoadError::BaselineConflict { name, .. } => {
                assert_eq!(name, "sce_atomic_load_acquire_u32");
            }
            other => panic!("expected BaselineConflict, got {other:?}"),
        }
    }

    #[test]
    fn baseline_conflict_rejected_different_sig() {
        // Different sig but baseline name → still conflict (spec line
        // 1852 trigger is "redefines", not "redefines incompatibly").
        let yaml = r#"
symbols:
  - name: sce_atomic_load_acquire_u32
    sig: "(*const u8) -> u8"
    abi: c
"#;
        let err = parse_target_plugin_str(yaml, p("plugin.yaml")).unwrap_err();
        assert!(matches!(
            err,
            TargetPluginLoadError::BaselineConflict { .. }
        ));
    }

    #[test]
    fn malformed_yaml_returns_yaml_error() {
        let yaml = "symbols: [not-a-mapping]";
        let err = parse_target_plugin_str(yaml, p("plugin.yaml")).unwrap_err();
        assert!(matches!(err, TargetPluginLoadError::Yaml { .. }));
    }

    #[test]
    fn unknown_field_on_symbol_entry_rejected() {
        // `deny_unknown_fields` on PluginSymbolEntry — typos like
        // `signature:` instead of `sig:` surface as Yaml errors.
        let yaml = r#"
symbols:
  - name: sce_hw_sem_take
    signature: "()"
    abi: c
"#;
        let err = parse_target_plugin_str(yaml, p("plugin.yaml")).unwrap_err();
        assert!(matches!(err, TargetPluginLoadError::Yaml { .. }));
    }

    #[test]
    fn forward_compat_top_level_extras_accepted() {
        // `linker_flavor` etc. are forward-compat axes — their
        // presence in the YAML must not break v1 plugin loading.
        let yaml = r#"
linker_flavor: scatter_arm
linker_fragment_path: linker_fragment.sct
symbols:
  - name: sce_hw_sem_take
    sig: "(u32) -> bool"
    abi: c
"#;
        let result =
            parse_target_plugin_str(yaml, p("plugin.yaml")).expect("forward-compat tolerant");
        assert_eq!(result.len(), 1);
    }
}
