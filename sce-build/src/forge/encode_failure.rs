// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
    /// state the case, and a fold that meets one says so by name. Rust, C++,
    /// Python, Swift, Kotlin, Go and C11 name every refusal.
    pub fn symbol(self, lang: Language) -> Option<&'static str> {
        match lang {
            Language::Rust => Some(match self {
                EncodeFailure::LengthOutOfRange => {
                    "::sce_forge_runtime::codec::CodecError::LengthOutOfRange"
                }
                EncodeFailure::PresentIfMismatch => {
                    "::sce_forge_runtime::codec::CodecError::PresentIfMismatch"
                }
                EncodeFailure::InconsistentSource => {
                    "::sce_forge_runtime::codec::CodecError::InconsistentSource"
                }
            }),
            Language::Cpp => Some(match self {
                EncodeFailure::LengthOutOfRange => "::SCE::Forge::CodecError::LengthOutOfRange",
                EncodeFailure::PresentIfMismatch => "::SCE::Forge::CodecError::PresentIfMismatch",
                EncodeFailure::InconsistentSource => "::SCE::Forge::CodecError::InconsistentSource",
            }),
            // The runtime's exception class, which the harness names through
            // the module it imports whole.
            Language::Python => Some(match self {
                EncodeFailure::LengthOutOfRange => "sce_forge_runtime.codec.LengthOutOfRange",
                EncodeFailure::PresentIfMismatch => "sce_forge_runtime.codec.PresentIfMismatch",
                EncodeFailure::InconsistentSource => "sce_forge_runtime.codec.InconsistentSource",
            }),
            // The case of the runtime's typed `CodecError` a printed encode
            // throws (docs/adr/0099), which `#expect(throws:)` compares with.
            Language::Swift => Some(match self {
                EncodeFailure::LengthOutOfRange => "CodecError.lengthOutOfRange",
                EncodeFailure::PresentIfMismatch => "CodecError.presentIfMismatch",
                EncodeFailure::InconsistentSource => "CodecError.inconsistentSource",
            }),
            // The runtime's error object, which the harness names whole, as
            // it names every declaration of a case.
            Language::Kotlin => Some(match self {
                EncodeFailure::LengthOutOfRange => {
                    "com.sce.forge.runtime.CodecError.LengthOutOfRange"
                }
                EncodeFailure::PresentIfMismatch => {
                    "com.sce.forge.runtime.CodecError.PresentIfMismatch"
                }
                EncodeFailure::InconsistentSource => {
                    "com.sce.forge.runtime.CodecError.InconsistentSource"
                }
            }),
            // The runtime's sentinel of the refusal's name, which a printed
            // encode returns unwrapped (`go_source::refusal`), so the harness
            // compares it as the decode's rejection vectors compare theirs.
            Language::Go => Some(match self {
                EncodeFailure::LengthOutOfRange => "codec.ErrLengthOutOfRange",
                EncodeFailure::PresentIfMismatch => "codec.ErrPresentIfMismatch",
                EncodeFailure::InconsistentSource => "codec.ErrInconsistentSource",
            }),
            // The C runtime's status codes (`sce/forge/codec.h`).
            Language::C11 => Some(match self {
                EncodeFailure::LengthOutOfRange => "SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE",
                EncodeFailure::PresentIfMismatch => "SCE_FORGE_CODEC_PRESENT_IF_MISMATCH",
                EncodeFailure::InconsistentSource => "SCE_FORGE_CODEC_INCONSISTENT_SOURCE",
            }),
        }
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
            // A Python refusal is the runtime's exception class of that name.
            let symbol = f
                .symbol(Language::Python)
                .expect("Python names every refusal");
            assert!(
                symbol.ends_with(&format!("codec.{variant}")),
                "Python {f:?}: `{symbol}` does not name `{variant}`"
            );
            // Swift spells a case in lower camel case.
            let mut chars = variant.chars();
            let case: String = chars
                .next()
                .map(|first| first.to_lowercase().chain(chars).collect())
                .unwrap_or_default();
            assert_eq!(
                f.symbol(Language::Swift),
                Some(format!("CodecError.{case}").as_str()),
                "{f:?}"
            );
            // A Kotlin refusal is the runtime's object of that name.
            let symbol = f
                .symbol(Language::Kotlin)
                .expect("Kotlin names every refusal");
            assert!(
                symbol.ends_with(&format!("CodecError.{variant}")),
                "Kotlin {f:?}: `{symbol}` does not name `{variant}`"
            );
            // A Go refusal is the runtime's sentinel of that name.
            let symbol = f.symbol(Language::Go).expect("Go names every refusal");
            assert_eq!(
                symbol,
                format!("codec.Err{variant}"),
                "Go {f:?}: `{symbol}` does not name `{variant}`"
            );
            // The C runtime names a status after the variant in upper snake
            // case (`LengthOutOfRange`, `SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE`).
            let symbol = f.symbol(Language::C11).expect("C11 names every refusal");
            assert_eq!(
                symbol
                    .strip_prefix("SCE_FORGE_CODEC_")
                    .map(|s| s.replace('_', "")),
                Some(variant.to_ascii_uppercase()),
                "C11 {f:?}: `{symbol}` does not name `{variant}`"
            );
        }
    }

    /// Rust, C++, Python, Swift, Kotlin, Go and C11 name every refusal, the
    /// inconsistent source among them (`CodecError::InconsistentSource` of the
    /// Rust and C++ runtimes, the exception `InconsistentSource` of the Python
    /// one, the case `inconsistentSource` of the Swift one,
    /// `CodecError.InconsistentSource` of the Kotlin one, the sentinel
    /// `codec.ErrInconsistentSource` of the Go one,
    /// `SCE_FORGE_CODEC_INCONSISTENT_SOURCE` of the C one): every backend's
    /// harness states a refused case.
    ///
    /// Turns red where `symbol` drops the C11 arm or a refusal of it.
    #[test]
    fn the_backends_that_name_a_refusal() {
        for f in EncodeFailure::ALL {
            for lang in [
                Language::Rust,
                Language::Cpp,
                Language::Python,
                Language::Swift,
                Language::Kotlin,
                Language::Go,
                Language::C11,
            ] {
                assert!(f.symbol(lang).is_some(), "{lang:?} {f:?}");
            }
        }
    }
}
