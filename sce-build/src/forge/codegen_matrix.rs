// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! SCE Forge — codegen kind × language matrix (RFC §5.J.4 / §5.J.5).
//!
//! Single source of truth for which `(ForgeKind, Language)` pairs are
//! emit-eligible per the watching-zenoh RFC §5.J.4 matrix. Two
//! invariants live here:
//!
//! 1. Every `ForgeKind` declares a `KindClass` (`Generic` or
//!    `McuClass`). Compile-time exhaustive over the enum — adding a
//!    kind variant without classifying it fails `cargo check`.
//! 2. Every emit-eligible `(kind, lang)` pair has a shipped Jinja2
//!    template in `tools/codegen/templates/forge/<lang>/`. Generic
//!    kinds list every backend explicitly so a missing template is a
//!    matrix edit, never a silent skip.
//!
//! The walker turns matrix violations into the two A1 diagnostics:
//! - `codegen/mcu-class-kind-on-non-mcu-language` — MCU-class kind
//!   targeting `{cpp, kotlin, go, python}`.
//! - `codegen/generic-kind-backend-emit-missing` — generic-class kind
//!   whose backend template has not yet shipped.
//!
//! Trust-boundary rationale (see memory `next_watching_zenoh_rfc_phase_a`):
//! SCE owns the contract, so this matrix is declared in SCE code (X
//! form). It extends the existing `SIX_BACKENDS` const at
//! `sce-build/tests/forge_conformance.rs` from a backend-only list to
//! a `(kind, backend)` pair set.

use crate::forge::error::GenerateError;
use crate::forge::model::ForgeKind;
use crate::generator::Language;

/// Class of a kind per RFC §5.J.4. Drives which `(kind, language)`
/// pairs are reachable; combined with `template_ships` it determines
/// which A1 diagnostic fires when an unreachable pair is requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindClass {
    /// Emits on all six backends. Missing template is an SCE bug
    /// (`codegen/generic-kind-backend-emit-missing`).
    Generic,
    /// Protocol-synthesis primitive whose emitter shape is only
    /// meaningful on `(rust, c11)`. Authoring against any other
    /// language is `codegen/mcu-class-kind-on-non-mcu-language`.
    McuClass,
}

/// Outcome of `lookup` for a `(kind, language)` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitOutcome {
    /// Template ships on this backend and the pair is on-policy.
    Emit,
    /// Generic-class kind whose backend template is absent.
    TemplateMissing,
    /// MCU-class kind targeting a non-MCU language.
    McuClassOnNonMcuLanguage,
}

/// Classify a kind per the §5.J.4 matrix.
///
/// Exhaustive over `ForgeKind` so a new variant fails `cargo check`
/// until classified — same lock-in pattern as `ForgeKind::ALL_ATTR_NAMES`.
pub const fn kind_class(kind: ForgeKind) -> KindClass {
    match kind {
        ForgeKind::Statechart
        | ForgeKind::Transform
        | ForgeKind::Lookup
        | ForgeKind::Condition
        | ForgeKind::Codec
        | ForgeKind::Procedure
        | ForgeKind::Validator
        | ForgeKind::Filter
        | ForgeKind::Interpolation
        | ForgeKind::Timer
        | ForgeKind::Observer
        | ForgeKind::Algorithm => KindClass::Generic,
    }
}

/// Whether the per-`(kind, language)` Jinja2 template has shipped.
///
/// Exhaustive over both axes. The 11 baseline kinds shipped on all 6
/// backends at `758aea3f` (commit "close C11 byte-golden parity with
/// 5 backends"). When a new kind variant lands in `ForgeKind`, a new
/// outer `match` arm must list every `Language` arm — silent
/// fall-through to a default branch is forbidden, otherwise a kind
/// can ship a template on backend A while the matrix records it as
/// missing on A.
pub const fn template_ships(kind: ForgeKind, lang: Language) -> bool {
    match kind {
        ForgeKind::Statechart
        | ForgeKind::Transform
        | ForgeKind::Lookup
        | ForgeKind::Condition
        | ForgeKind::Codec
        | ForgeKind::Procedure
        | ForgeKind::Validator
        | ForgeKind::Filter
        | ForgeKind::Interpolation
        | ForgeKind::Timer
        | ForgeKind::Observer => match lang {
            Language::Rust
            | Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::C11 => true,
        },
        // RFC §5.A Algorithm: closed across all six backends.
        // A3 lands Rust + Cpp; A5 closes C11 (RFC §7 line 3382);
        // post-A6 follow-up trio adds Go + Kotlin + Python in three
        // independent commits.
        ForgeKind::Algorithm => match lang {
            Language::Rust | Language::Cpp | Language::C11
            | Language::Go | Language::Kotlin | Language::Python => true,
        },
    }
}

/// Walk the matrix for `(kind, language)`. Returns `EmitOutcome::Emit`
/// when the template ships and the pair is on-policy; otherwise the
/// outcome names the §5.J.4 violation.
pub const fn lookup(kind: ForgeKind, lang: Language) -> EmitOutcome {
    match kind_class(kind) {
        KindClass::McuClass => match lang {
            Language::Rust | Language::C11 => {
                if template_ships(kind, lang) {
                    EmitOutcome::Emit
                } else {
                    EmitOutcome::TemplateMissing
                }
            }
            Language::Cpp | Language::Kotlin | Language::Go | Language::Python => {
                EmitOutcome::McuClassOnNonMcuLanguage
            }
        },
        KindClass::Generic => {
            if template_ships(kind, lang) {
                EmitOutcome::Emit
            } else {
                EmitOutcome::TemplateMissing
            }
        }
    }
}

/// Reject `(kind, lang)` with the matching `GenerateError` if the
/// matrix walker says the pair is unreachable; return `Ok(())` when
/// emit-eligible. Dispatch sites in `forge/generator.rs` call this
/// before routing to the per-kind `render_*` helper.
pub fn check(kind: ForgeKind, lang: Language) -> Result<(), GenerateError> {
    match lookup(kind, lang) {
        EmitOutcome::Emit => Ok(()),
        EmitOutcome::TemplateMissing => {
            Err(GenerateError::CodegenGenericKindBackendEmitMissing {
                kind: kind.to_string(),
                language: language_wire_name(lang).to_string(),
            })
        }
        EmitOutcome::McuClassOnNonMcuLanguage => {
            Err(GenerateError::CodegenMcuClassKindOnNonMcuLanguage {
                kind: kind.to_string(),
                language: language_wire_name(lang).to_string(),
            })
        }
    }
}

/// Stable wire-name for a `Language`, matching `Language::from_str`
/// accept-set so diagnostic `actual` values round-trip cleanly.
const fn language_wire_name(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "rust",
        Language::Cpp => "cpp",
        Language::Kotlin => "kotlin",
        Language::Go => "go",
        Language::Python => "python",
        Language::C11 => "c11",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All 11 baseline kinds × all 6 backends emit at `758aea3f`.
    #[test]
    fn baseline_kinds_emit_on_all_backends() {
        const ALL_LANGS: &[Language] = &[
            Language::Rust,
            Language::Cpp,
            Language::Kotlin,
            Language::Go,
            Language::Python,
            Language::C11,
        ];
        const BASELINE_KINDS: &[ForgeKind] = &[
            ForgeKind::Statechart,
            ForgeKind::Transform,
            ForgeKind::Lookup,
            ForgeKind::Condition,
            ForgeKind::Codec,
            ForgeKind::Procedure,
            ForgeKind::Validator,
            ForgeKind::Filter,
            ForgeKind::Interpolation,
            ForgeKind::Timer,
            ForgeKind::Observer,
        ];
        for &k in BASELINE_KINDS {
            assert_eq!(kind_class(k), KindClass::Generic, "kind {k:?} class");
            for &l in ALL_LANGS {
                assert_eq!(lookup(k, l), EmitOutcome::Emit, "({k:?}, {l:?})");
                assert!(check(k, l).is_ok(), "check({k:?}, {l:?})");
            }
        }
    }

    /// RFC §5.A §5.J.4 matrix closure: Algorithm ships on all six
    /// backends after the post-A6 Go / Kotlin / Python follow-up trio.
    /// No backend remains gated by `codegen/generic-kind-backend-emit-missing`.
    #[test]
    fn algorithm_kind_ships_on_all_backends() {
        assert_eq!(kind_class(ForgeKind::Algorithm), KindClass::Generic);
        for lang in [
            Language::Rust,
            Language::Cpp,
            Language::C11,
            Language::Go,
            Language::Kotlin,
            Language::Python,
        ] {
            assert_eq!(
                lookup(ForgeKind::Algorithm, lang),
                EmitOutcome::Emit,
                "({:?}, {:?}) should emit",
                ForgeKind::Algorithm,
                lang,
            );
            assert!(
                check(ForgeKind::Algorithm, lang).is_ok(),
                "check({:?}, {:?}) should succeed",
                ForgeKind::Algorithm,
                lang,
            );
        }
    }

    #[test]
    fn language_wire_names_match_from_str_accept_set() {
        use std::str::FromStr;
        let cases = [
            (Language::Rust, "rust"),
            (Language::Cpp, "cpp"),
            (Language::Kotlin, "kotlin"),
            (Language::Go, "go"),
            (Language::Python, "python"),
            (Language::C11, "c11"),
        ];
        for (lang, name) in cases {
            assert_eq!(language_wire_name(lang), name);
            assert_eq!(Language::from_str(name).unwrap(), lang);
        }
    }
}
