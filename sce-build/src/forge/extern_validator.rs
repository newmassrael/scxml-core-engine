// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// `<sce:extern>` parse-time validator — watching-zenoh RFC §5.I.
// Closed-set lookup over [`crate::forge::intrinsic_registry::BASELINE_SYMBOLS`];
// rejection is parse-time (matches the `LinkLinkClassUnknown`
// closed-enum precedent).
//
// Returns four distinct failure shapes — one per spec diagnostic
// (§5.I lines 1846-1850):
//
//   - [`ExternFailure::NotInWhitelist`]      → `extern/symbol-not-in-whitelist`
//   - [`ExternFailure::AbiMismatch`]          → `extern/abi-mismatch`
//   - [`ExternFailure::SignatureMismatch`]    → `extern/signature-mismatch`
//   - [`ExternFailure::OrderingUnspecified`]  → `extern/ordering-unspecified`
//
// The four failure classes name distinct repair shapes:
// NotInWhitelist + OrderingUnspecified offer name-list
// candidates (`Fix::ReplaceOneOf`); SignatureMismatch carries the
// canonical sig (`Fix::Replace`); AbiMismatch picks from a closed
// two-element set (`Fix::ReplaceOneOf {[c, rust]}`).

use crate::forge::intrinsic_registry::{
    lookup_symbol, ordering_suffix_completions, Abi, Symbol, BASELINE_SYMBOLS,
};
use crate::forge::target_plugin::PluginSymbol;

/// Validation outcome for one `<sce:extern name sig abi/>` triple.
/// The caller (parser hook in `parse_forge_with_imports`) maps each
/// arm onto a distinct [`crate::forge::error::ValidationError`]
/// variant so the wire-format `code` field carries the spec-verbatim
/// `extern/<axis>` slug.
///
/// String fields are owned so plugin-loaded entries
/// ([`crate::forge::target_plugin::PluginSymbol`]) can
/// surface through the same failure axes as baseline entries —
/// the registry source is hidden from the wire format, only the
/// failure axis matters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternFailure {
    /// `<sce:extern name="...">` not present in the registry
    /// (baseline + optional plugin). The `candidates` list rides
    /// `Fix::ReplaceOneOf` so authors get a closest-match top-N hint
    /// without the validator iterating the entire 101-symbol baseline
    /// at parse time.
    NotInWhitelist {
        /// Top-N closest registry symbols to the authored name
        /// (shared-prefix sorted; clamped to 8 to keep wire payload
        /// bounded). Spans baseline + plugin entries when the plugin-
        /// aware variant is invoked. Empty when no registry name
        /// shares a substring with the input.
        candidates: Vec<String>,
    },
    /// `<sce:extern abi="...">` does not match the registry entry's
    /// canonical ABI. `Fix::ReplaceOneOf` picks from the closed
    /// two-element set [`c`, `rust`].
    AbiMismatch {
        /// Registry entry's canonical ABI (the one the author should
        /// have spelled).
        expected: Abi,
        /// What the author wrote.
        actual: String,
    },
    /// `<sce:extern sig="...">` does not match the registry entry's
    /// canonical signature. `Fix::Replace` carries the canonical sig
    /// verbatim — the registry is the source of truth for signature
    /// shape (spec-mirror parity).
    SignatureMismatch {
        /// Registry entry's canonical signature.
        expected: String,
        /// What the author wrote.
        actual: String,
    },
    /// Atomic-family base name written without the ordering + width
    /// suffix (spec line 1850: "atomic intrinsic invoked without
    /// explicit ordering suffix"). Carries the legal completions so
    /// the diagnostic's `Fix::ReplaceOneOf` can list them. Distinct
    /// from `NotInWhitelist` because the repair shape is "pick a
    /// suffix" rather than "pick a different symbol entirely".
    /// Baseline-only (plugin entries do not participate in atomic-
    /// family suffix expansion).
    OrderingUnspecified {
        /// Name as written (e.g. `sce_atomic_load`).
        base: String,
        /// Legal suffix-bearing completions (e.g.
        /// `sce_atomic_load_acquire_u32`, ..., `sce_atomic_load_relaxed_usize`).
        /// 10 entries for load/store/fetch_*; 15 for cas_*.
        candidates: Vec<&'static str>,
    },
}

/// Validate a `<sce:extern name sig abi/>` triple against the
/// registry. Caller passes attribute strings verbatim; this layer
/// does no string normalization (case, whitespace) — the registry
/// names are exact.
///
/// Returns `Ok(symbol)` for a valid triple; the caller is free to
/// drop the symbol (wire-format storage doesn't need it today) or
/// thread it into a parsed declaration list for downstream codegen
/// consumption.
pub fn validate_extern(
    name: &str,
    sig: &str,
    abi_attr: &str,
) -> Result<&'static Symbol, ExternFailure> {
    let symbol = match lookup_symbol(name) {
        Some(s) => s,
        None => {
            // Distinguish atomic-family base-name-without-suffix from
            // entirely-unknown name. `ordering_suffix_completions`
            // returns `Some(_)` only for known atomic-family bases —
            // any other miss falls through to `NotInWhitelist`.
            if let Some(candidates) = ordering_suffix_completions(name) {
                return Err(ExternFailure::OrderingUnspecified {
                    base: name.to_string(),
                    candidates,
                });
            }
            let candidates = closest_baseline_names(name)
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            return Err(ExternFailure::NotInWhitelist { candidates });
        }
    };
    // ABI check — closed two-element set ([c, rust]). Anything else
    // raises `AbiMismatch` so the message carries the actual string
    // (an unknown ABI like "system" surfaces here, not as a separate
    // code; the closed-set repair list applies uniformly).
    let parsed_abi = Abi::from_attr(abi_attr);
    if parsed_abi != Some(symbol.abi) {
        return Err(ExternFailure::AbiMismatch {
            expected: symbol.abi,
            actual: abi_attr.to_string(),
        });
    }
    if sig != symbol.sig {
        return Err(ExternFailure::SignatureMismatch {
            expected: symbol.sig.to_string(),
            actual: sig.to_string(),
        });
    }
    Ok(symbol)
}

/// Top-N closest baseline names to `target`, by shared character
/// count from the start. Cheap heuristic — full Levenshtein would
/// inflate parse time on a 101-element baseline for marginal repair-
/// guidance gain. Result is bounded at 8 entries to keep
/// `Fix::ReplaceOneOf` wire payload small.
fn closest_baseline_names(target: &str) -> Vec<&'static str> {
    let mut scored: Vec<(usize, &'static str)> = BASELINE_SYMBOLS
        .iter()
        .map(|s| (shared_prefix_len(target, s.name), s.name))
        .filter(|(score, _)| *score > 0)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    scored.into_iter().take(8).map(|(_, name)| name).collect()
}

/// Closest-name candidates spanning baseline + plugin. Used by
/// [`validate_extern_with_plugin`] when the authored name misses
/// both registries. Plugin entries surface alongside baseline names
/// so an author who typed `sce_hw_smm_take` but loaded a plugin
/// declaring `sce_hw_sem_take` sees the plugin's name in the
/// repair list.
fn closest_baseline_or_plugin_names(target: &str, plugin: &[PluginSymbol]) -> Vec<String> {
    let mut scored: Vec<(usize, String)> = BASELINE_SYMBOLS
        .iter()
        .map(|s| (shared_prefix_len(target, s.name), s.name.to_string()))
        .filter(|(score, _)| *score > 0)
        .collect();
    for p in plugin {
        let score = shared_prefix_len(target, &p.name);
        if score > 0 {
            scored.push((score, p.name.clone()));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored.into_iter().take(8).map(|(_, name)| name).collect()
}

/// Plugin-aware counterpart to [`validate_extern`].
/// Lookup order: baseline → plugin (additive
/// composition; baseline shadowing already ruled out at plugin LOAD
/// time per [`crate::forge::target_plugin::parse_target_plugin_yaml`]'s
/// `BaselineConflict` check, so a name match here is unambiguous).
///
/// Returns `Ok(())` for a valid triple regardless of source. Error
/// arms reuse [`ExternFailure`] verbatim — plugin-source failures
/// still surface as `extern/abi-mismatch` / `extern/signature-mismatch`
/// because the diagnostic axes are identical (the registry source is
/// hidden from the wire format; the `actual` field carries author
/// input either way).
///
/// Unknown-name failure wraps [`ExternFailure::NotInWhitelist`]
/// candidates with both baseline and plugin names so authors typing a
/// vendor-symbol typo see the correct top-N list. Atomic-family base
/// detection ([`ExternFailure::OrderingUnspecified`]) stays baseline-
/// only — plugin entries are vendor-specific and do not participate
/// in atomic-suffix expansion.
pub fn validate_extern_with_plugin(
    name: &str,
    sig: &str,
    abi_attr: &str,
    plugin: &[PluginSymbol],
) -> Result<(), ExternFailure> {
    if let Some(symbol) = lookup_symbol(name) {
        // Baseline hit — same checks as `validate_extern`.
        let parsed_abi = Abi::from_attr(abi_attr);
        if parsed_abi != Some(symbol.abi) {
            return Err(ExternFailure::AbiMismatch {
                expected: symbol.abi,
                actual: abi_attr.to_string(),
            });
        }
        if sig != symbol.sig {
            return Err(ExternFailure::SignatureMismatch {
                expected: symbol.sig.to_string(),
                actual: sig.to_string(),
            });
        }
        return Ok(());
    }
    if let Some(p) = plugin.iter().find(|p| p.name == name) {
        // Plugin hit — same axis checks, owned-string sources.
        let parsed_abi = Abi::from_attr(abi_attr);
        if parsed_abi != Some(p.abi) {
            return Err(ExternFailure::AbiMismatch {
                expected: p.abi,
                actual: abi_attr.to_string(),
            });
        }
        if sig != p.sig {
            return Err(ExternFailure::SignatureMismatch {
                expected: p.sig.clone(),
                actual: sig.to_string(),
            });
        }
        return Ok(());
    }
    // Miss in both → atomic-family base detection runs against the
    // baseline (plugin entries do not participate in atomic-suffix
    // expansion); otherwise emit NotInWhitelist with the merged
    // candidate list.
    if let Some(candidates) = ordering_suffix_completions(name) {
        return Err(ExternFailure::OrderingUnspecified {
            base: name.to_string(),
            candidates,
        });
    }
    Err(ExternFailure::NotInWhitelist {
        candidates: closest_baseline_or_plugin_names(name, plugin),
    })
}

fn shared_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_atomic_load_acquire_u32() {
        let s = validate_extern("sce_atomic_load_acquire_u32", "(*const u32) -> u32", "c")
            .expect("happy path");
        assert_eq!(s.name, "sce_atomic_load_acquire_u32");
    }

    #[test]
    fn happy_path_cache_clean() {
        validate_extern("sce_dcache_clean_by_addr", "(*const c_void, usize)", "c")
            .expect("happy path");
    }

    #[test]
    fn unknown_symbol_yields_not_in_whitelist() {
        let err = validate_extern("sce_does_not_exist", "()", "c").unwrap_err();
        match err {
            ExternFailure::NotInWhitelist { .. } => {}
            other => panic!("expected NotInWhitelist, got {other:?}"),
        }
    }

    #[test]
    fn atomic_base_without_suffix_yields_ordering_unspecified() {
        let err = validate_extern("sce_atomic_load", "(*const u32) -> u32", "c").unwrap_err();
        match err {
            ExternFailure::OrderingUnspecified { base, candidates } => {
                assert_eq!(base, "sce_atomic_load");
                assert_eq!(candidates.len(), 10);
                assert!(candidates.contains(&"sce_atomic_load_acquire_u32"));
            }
            other => panic!("expected OrderingUnspecified, got {other:?}"),
        }
    }

    #[test]
    fn fence_base_without_suffix_yields_ordering_unspecified() {
        let err = validate_extern("sce_atomic_fence", "()", "c").unwrap_err();
        match err {
            ExternFailure::OrderingUnspecified { candidates, .. } => {
                // 4 fence orderings: acquire, release, acq_rel, seq_cst.
                assert_eq!(candidates.len(), 4);
            }
            other => panic!("expected OrderingUnspecified, got {other:?}"),
        }
    }

    #[test]
    fn wrong_abi_yields_abi_mismatch() {
        let err = validate_extern("sce_atomic_load_acquire_u32", "(*const u32) -> u32", "rust")
            .unwrap_err();
        match err {
            ExternFailure::AbiMismatch { expected, actual } => {
                assert_eq!(expected, Abi::C);
                assert_eq!(actual, "rust");
            }
            other => panic!("expected AbiMismatch, got {other:?}"),
        }
    }

    #[test]
    fn unknown_abi_yields_abi_mismatch() {
        let err = validate_extern(
            "sce_atomic_load_acquire_u32",
            "(*const u32) -> u32",
            "system",
        )
        .unwrap_err();
        match err {
            ExternFailure::AbiMismatch { actual, .. } => {
                assert_eq!(actual, "system");
            }
            other => panic!("expected AbiMismatch, got {other:?}"),
        }
    }

    #[test]
    fn wrong_sig_yields_signature_mismatch() {
        let err = validate_extern(
            "sce_atomic_load_acquire_u32",
            "(*const u32) -> u64", // wrong return width
            "c",
        )
        .unwrap_err();
        match err {
            ExternFailure::SignatureMismatch { expected, actual } => {
                assert_eq!(expected, "(*const u32) -> u32");
                assert_eq!(actual, "(*const u32) -> u64");
            }
            other => panic!("expected SignatureMismatch, got {other:?}"),
        }
    }

    #[test]
    fn closest_baseline_names_returns_relevant_candidates() {
        let hits = closest_baseline_names("sce_atomic_load_acquir_u32"); // typo
        assert!(!hits.is_empty());
        assert!(hits
            .iter()
            .any(|n| n.starts_with("sce_atomic_load_acquire_")));
    }

    // ── Plugin-aware tests ─────────────────────────

    fn vendor_plugin() -> Vec<PluginSymbol> {
        vec![PluginSymbol {
            name: "sce_hw_sem_take".to_string(),
            sig: "(u32) -> bool".to_string(),
            abi: Abi::C,
            purpose: Some("cross-core-mutex".to_string()),
            crate_name: None,
        }]
    }

    #[test]
    fn plugin_aware_baseline_happy_path() {
        // Baseline-listed symbol still validates with empty plugin.
        validate_extern_with_plugin(
            "sce_atomic_load_acquire_u32",
            "(*const u32) -> u32",
            "c",
            &[],
        )
        .expect("baseline lookup fires before plugin");
    }

    #[test]
    fn plugin_aware_plugin_happy_path() {
        // Vendor symbol resolves through plugin slice.
        let plugin = vendor_plugin();
        validate_extern_with_plugin("sce_hw_sem_take", "(u32) -> bool", "c", &plugin)
            .expect("plugin lookup happy path");
    }

    #[test]
    fn plugin_aware_plugin_sig_mismatch() {
        let plugin = vendor_plugin();
        let err = validate_extern_with_plugin(
            "sce_hw_sem_take",
            "(u32, *mut bool)", // wrong shape
            "c",
            &plugin,
        )
        .unwrap_err();
        match err {
            ExternFailure::SignatureMismatch { expected, actual } => {
                assert_eq!(expected, "(u32) -> bool");
                assert_eq!(actual, "(u32, *mut bool)");
            }
            other => panic!("expected SignatureMismatch, got {other:?}"),
        }
    }

    #[test]
    fn plugin_aware_plugin_abi_mismatch() {
        let plugin = vendor_plugin();
        let err = validate_extern_with_plugin("sce_hw_sem_take", "(u32) -> bool", "rust", &plugin)
            .unwrap_err();
        match err {
            ExternFailure::AbiMismatch { expected, actual } => {
                assert_eq!(expected, Abi::C);
                assert_eq!(actual, "rust");
            }
            other => panic!("expected AbiMismatch, got {other:?}"),
        }
    }

    #[test]
    fn plugin_aware_unknown_symbol_includes_plugin_in_candidates() {
        // Author types a typo of the plugin's symbol — closest-name
        // candidates must include the plugin entry alongside any
        // nearby baseline entries.
        let plugin = vendor_plugin();
        let err = validate_extern_with_plugin("sce_hw_sm_take", "()", "c", &plugin).unwrap_err();
        match err {
            ExternFailure::NotInWhitelist { candidates } => {
                assert!(
                    candidates.iter().any(|n| n == "sce_hw_sem_take"),
                    "expected vendor symbol in candidates: {candidates:?}",
                );
            }
            other => panic!("expected NotInWhitelist, got {other:?}"),
        }
    }

    #[test]
    fn plugin_aware_atomic_base_still_yields_ordering_unspecified() {
        // Plugin slice does not alter atomic-suffix detection — base
        // names like `sce_atomic_load` still surface as
        // OrderingUnspecified so authors get suffix completions
        // rather than a generic NotInWhitelist.
        let plugin = vendor_plugin();
        let err =
            validate_extern_with_plugin("sce_atomic_load", "(*const u32) -> u32", "c", &plugin)
                .unwrap_err();
        assert!(matches!(err, ExternFailure::OrderingUnspecified { .. }));
    }
}
