// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// `<sce:extern>` parse-time validator — watching-zenoh RFC §5.I, Atomic
// A. Closed-set lookup over [`crate::forge::intrinsic_registry::BASELINE_SYMBOLS`]
// per Q-Call-1 (a) lock; rejection is parse-time per Q-Call-4 (a) lock
// (matches the `LinkLinkClassUnknown` (B6-γ) closed-enum precedent).
//
// Returns four distinct failure shapes — one per spec diagnostic
// (§5.I lines 1846-1850):
//
//   - [`ExternFailure::NotInWhitelist`]      → `extern/symbol-not-in-whitelist`
//   - [`ExternFailure::AbiMismatch`]          → `extern/abi-mismatch`
//   - [`ExternFailure::SignatureMismatch`]    → `extern/signature-mismatch`
//   - [`ExternFailure::OrderingUnspecified`]  → `extern/ordering-unspecified`
//
// The four failure classes name distinct repair shapes (Q-Call-5
// rationale): NotInWhitelist + OrderingUnspecified offer name-list
// candidates (`Fix::ReplaceOneOf`); SignatureMismatch carries the
// canonical sig (`Fix::Replace`); AbiMismatch picks from a closed
// two-element set (`Fix::ReplaceOneOf {[c, rust]}`).

use crate::forge::intrinsic_registry::{
    lookup_symbol, ordering_suffix_completions, Abi, Symbol, BASELINE_SYMBOLS,
};

/// Validation outcome for one `<sce:extern name sig abi/>` triple.
/// The caller (parser hook in `parse_forge_with_imports`) maps each
/// arm onto a distinct [`crate::forge::error::ValidationError`]
/// variant so the wire-format `code` field carries the spec-verbatim
/// `extern/<axis>` slug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternFailure {
    /// `<sce:extern name="...">` not present in the registry. The
    /// `candidates` list rides `Fix::ReplaceOneOf` so authors get a
    /// closest-match top-N hint without the validator iterating the
    /// entire 101-symbol baseline at parse time.
    NotInWhitelist {
        /// Top-N closest baseline symbols to the authored name
        /// (Levenshtein-prefix sorted; clamped to 8 to keep wire
        /// payload bounded). Empty when no name in the baseline
        /// shares a substring with the input.
        candidates: Vec<&'static str>,
    },
    /// `<sce:extern abi="...">` does not match the registry entry's
    /// canonical ABI. `Fix::ReplaceOneOf` picks from the closed
    /// two-element set [`c`, `rust`] (Q-Call-5 rationale).
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
    /// shape (matches the `feedback_spec_mirror_parity.md` pattern).
    SignatureMismatch {
        /// Registry entry's canonical signature.
        expected: &'static str,
        /// What the author wrote.
        actual: String,
    },
    /// Atomic-family base name written without the ordering + width
    /// suffix (spec line 1850: "atomic intrinsic invoked without
    /// explicit ordering suffix"). Carries the legal completions so
    /// the diagnostic's `Fix::ReplaceOneOf` can list them. Distinct
    /// from `NotInWhitelist` because the repair shape is "pick a
    /// suffix" rather than "pick a different symbol entirely".
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
            let candidates = closest_baseline_names(name);
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
            expected: symbol.sig,
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

fn shared_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_atomic_load_acquire_u32() {
        let s = validate_extern(
            "sce_atomic_load_acquire_u32",
            "(*const u32) -> u32",
            "c",
        )
        .expect("happy path");
        assert_eq!(s.name, "sce_atomic_load_acquire_u32");
    }

    #[test]
    fn happy_path_cache_clean() {
        validate_extern(
            "sce_dcache_clean_by_addr",
            "(*const c_void, usize)",
            "c",
        )
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
        let err = validate_extern(
            "sce_atomic_load_acquire_u32",
            "(*const u32) -> u32",
            "rust",
        )
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
        assert!(hits.iter().any(|n| n.starts_with("sce_atomic_load_acquire_")));
    }
}
