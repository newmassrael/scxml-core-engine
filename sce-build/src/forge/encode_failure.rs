// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The refusals a positional codec's encode answers a value with
// (docs/adr/0065 §2), as the conformance oracle names them, and each
// backend's spelling of them.
//
// The encode-side counterpart of `codec_failure`: a refused case of the
// oracle (`refused`, docs/adr/0081 §5) states the value as the codec's
// interface presents it and the refusal its encode must answer, and the
// harness of a backend that prints the codec's plan compares the encode's
// answer with that refusal by the symbol named here. A value outside the
// type its interface presents (a byte string past its bound, two lists of
// one count that differ) is no value of the codec, and no refusal names it:
// the type refuses it before an encode is called.

use crate::forge::codec_totality::RefusalKind;
use crate::generator::Language;

/// A refusal a positional encode answers a value of its interface with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeFailure {
    /// A length or a count the encoding derives does not fit the field or the
    /// bits that carry it.
    LengthOutOfRange,
    /// A gated field is given where a gate the encode cannot derive says it is
    /// absent, or left out where it says it is there.
    PresentIfMismatch,
    /// Two things the value says of one number disagree.
    InconsistentSource,
}

impl EncodeFailure {
    /// Every refusal, in declaration order: the set `parse` accepts.
    pub const ALL: [EncodeFailure; 3] = [
        EncodeFailure::LengthOutOfRange,
        EncodeFailure::PresentIfMismatch,
        EncodeFailure::InconsistentSource,
    ];

    /// The kebab-case name a refused case of the oracle states as its `error`.
    pub fn wire_name(self) -> &'static str {
        match self {
            EncodeFailure::LengthOutOfRange => "length-out-of-range",
            EncodeFailure::PresentIfMismatch => "present-if-mismatch",
            EncodeFailure::InconsistentSource => "inconsistent-source",
        }
    }

    /// Parse a refused case's `error`. The set is closed: a name it does not
    /// hold is an authoring mistake, and the render that reads it fails.
    pub fn parse(name: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|f| f.wire_name() == name)
            .ok_or_else(|| {
                format!(
                    "unknown encode refusal '{name}': expected one of: {}",
                    Self::ALL.map(|f| f.wire_name()).join(", ")
                )
            })
    }

    /// The kind the totality analysis gives this refusal, which says where a
    /// codec's encode can answer it.
    pub fn refusal_kind(self) -> RefusalKind {
        match self {
            EncodeFailure::LengthOutOfRange => RefusalKind::LengthOutOfRange,
            EncodeFailure::PresentIfMismatch => RefusalKind::PresentIfMismatch,
            EncodeFailure::InconsistentSource => RefusalKind::InconsistentSource,
        }
    }

    /// The expression a harness compares an encode's refusal with on a backend
    /// whose encode of a source answers the runtime's typed error, or `None`
    /// where the backend has no name for this refusal: no harness of it can
    /// state the case, and a fold that meets one says so by name. The C++
    /// runtime names no inconsistent source yet.
    pub fn symbol(self, lang: Language) -> Option<&'static str> {
        Some(match (lang, self) {
            (Language::Rust, EncodeFailure::LengthOutOfRange) => {
                "::sce_forge_runtime::codec::CodecError::LengthOutOfRange"
            }
            (Language::Rust, EncodeFailure::PresentIfMismatch) => {
                "::sce_forge_runtime::codec::CodecError::PresentIfMismatch"
            }
            (Language::Rust, EncodeFailure::InconsistentSource) => {
                "::sce_forge_runtime::codec::CodecError::InconsistentSource"
            }
            (Language::Cpp, EncodeFailure::LengthOutOfRange) => {
                "::SCE::Forge::CodecError::LengthOutOfRange"
            }
            (Language::Cpp, EncodeFailure::PresentIfMismatch) => {
                "::SCE::Forge::CodecError::PresentIfMismatch"
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every refusal round-trips through its oracle name, so an oracle author
    /// and this table cannot disagree about spelling.
    #[test]
    fn wire_names_round_trip() {
        for f in EncodeFailure::ALL {
            assert_eq!(EncodeFailure::parse(f.wire_name()), Ok(f));
        }
    }

    #[test]
    fn an_unknown_refusal_name_is_refused_by_name() {
        let err =
            EncodeFailure::parse("need-more-bytes").expect_err("a decode failure is no refusal");
        assert!(err.contains("present-if-mismatch"), "{err}");
    }

    /// The symbol a harness compares with is the variant the totality analysis
    /// names, which the printed encode returns: the two tables cannot drift.
    #[test]
    fn a_symbol_names_the_variant_the_analysis_names() {
        for f in EncodeFailure::ALL {
            let variant = f
                .refusal_kind()
                .error_variant()
                .expect("an encode refusal is one the encode returns itself");
            for lang in [Language::Rust, Language::Cpp] {
                if let Some(symbol) = f.symbol(lang) {
                    assert!(
                        symbol.ends_with(&format!("CodecError::{variant}")),
                        "{lang:?} {f:?}: `{symbol}` does not name `{variant}`"
                    );
                }
            }
        }
    }

    /// Rust names every refusal; C++ names all but the inconsistent source,
    /// which its runtime does not hold yet, and no other backend's harness
    /// states a refused case.
    #[test]
    fn the_backends_that_name_a_refusal() {
        for f in EncodeFailure::ALL {
            assert!(f.symbol(Language::Rust).is_some(), "{f:?}");
            assert_eq!(
                f.symbol(Language::Cpp).is_some(),
                f != EncodeFailure::InconsistentSource,
                "{f:?}"
            );
            for lang in [
                Language::C11,
                Language::Go,
                Language::Kotlin,
                Language::Python,
            ] {
                assert!(f.symbol(lang).is_none(), "{lang:?} {f:?}");
            }
        }
    }
}
