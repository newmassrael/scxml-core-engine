// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// RFC synth-5-B — the decode-side failure vocabulary a generated codec can
// signal, and each backend's spelling of it.
//
// Two consumers share this table and must not grow private copies:
//   - `forge::generator` emits the statement that raises a failure from
//     inside a generated decode;
//   - `conformance` renders the assertion a reject vector makes about that
//     same failure from outside.
//
// Raising a failure and observing one are not the same question, so the
// table answers both. Cpp and Kotlin construct no typed runtime error at all
// (the "MCU-only codec sub-features" convention — they collapse onto their
// truncation sentinel), and Python's generated `decode` funnels every
// `CodecError` through one `except` into `None`. For all three the failure
// is raised by name inside the decode but is *not* observable by name
// outside it, so `observable_symbol` reports `None` and a caller can only
// assert that the decode refused — which is the whole of what it can see.

use crate::generator::Language;

/// A decode-side failure a generated codec can signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecFailure {
    /// The peer's frame ended before the codec's declared shape was
    /// complete. Under `terminate-on="entry-flag"` this includes a chain
    /// whose last entry declared a successor the wire never carried.
    NeedMoreBytes,
    /// A `<sce:tlv-chain>` carried more entries than `max-depth` admits,
    /// under `on-overflow="reject"`.
    TlvChainOverflow,
    /// An `sce:encoding="cbor"` map is not one the codec reads
    /// (SCE_FORGE.md §4.6.1): not a definite-length map, a key given twice,
    /// or an entry of the wrong major type.
    CborMalformed,
    /// A CBOR map lacks an entry declared `sce:required="true"`.
    CborRequiredKeyMissing,
    /// A CBOR byte string is not its declared `sce:length`.
    CborWrongLength,
    /// A CBOR value does not fit its declared type or `sce:max-size`.
    CborOutOfRange,
}

impl CodecFailure {
    /// Every failure, in declaration order — the set `parse` accepts.
    pub const ALL: [CodecFailure; 6] = [
        CodecFailure::NeedMoreBytes,
        CodecFailure::TlvChainOverflow,
        CodecFailure::CborMalformed,
        CodecFailure::CborRequiredKeyMissing,
        CodecFailure::CborWrongLength,
        CodecFailure::CborOutOfRange,
    ];

    /// Kebab-case name naming this failure in the conformance oracle's
    /// reject vectors.
    pub fn wire_name(self) -> &'static str {
        match self {
            CodecFailure::NeedMoreBytes => "need-more-bytes",
            CodecFailure::TlvChainOverflow => "tlv-chain-overflow",
            CodecFailure::CborMalformed => "cbor-malformed",
            CodecFailure::CborRequiredKeyMissing => "cbor-required-key-missing",
            CodecFailure::CborWrongLength => "cbor-wrong-length",
            CodecFailure::CborOutOfRange => "cbor-out-of-range",
        }
    }

    /// Parse an oracle `error` value. The set is closed: an unrecognised
    /// name is an authoring mistake, and failing the render is how the
    /// author hears about it.
    pub fn parse(name: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|f| f.wire_name() == name)
            .ok_or_else(|| {
                format!(
                    "unknown codec failure '{name}' — expected one of: {}",
                    Self::ALL.map(|f| f.wire_name()).join(", ")
                )
            })
    }

    /// Whether this is a failure of the CBOR map reader rather than of a
    /// positional decode.
    pub fn is_cbor(self) -> bool {
        matches!(
            self,
            CodecFailure::CborMalformed
                | CodecFailure::CborRequiredKeyMissing
                | CodecFailure::CborWrongLength
                | CodecFailure::CborOutOfRange
        )
    }

    /// The statement a generated positional decode uses to signal this
    /// failure. `None` for a CBOR failure: no generated statement raises
    /// one — the runtime's CBOR reader and the CBOR codec templates return
    /// it as a value (SCE_FORGE.md §4.6.1).
    pub fn raise_stmt(self, lang: Language) -> Option<&'static str> {
        let truncated = matches!(self, CodecFailure::NeedMoreBytes);
        match self {
            CodecFailure::NeedMoreBytes | CodecFailure::TlvChainOverflow => Some(match lang {
                Language::Rust if truncated => "return Err(CodecError::NeedMoreBytes);",
                Language::Rust => "return Err(CodecError::TlvChainOverflow);",
                Language::C11 if truncated => "return SCE_FORGE_CODEC_NEED_MORE_BYTES;",
                Language::C11 => "return SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW;",
                Language::Cpp => "return std::nullopt;",
                Language::Kotlin => "return null",
                Language::Go if truncated => "return nil, codec.ErrNeedMoreBytes",
                Language::Go => "return nil, codec.ErrTlvChainOverflow",
                Language::Python if truncated => "raise NeedMoreBytes()",
                Language::Python => "raise TlvChainOverflow()",
            }),
            CodecFailure::CborMalformed
            | CodecFailure::CborRequiredKeyMissing
            | CodecFailure::CborWrongLength
            | CodecFailure::CborOutOfRange => None,
        }
    }

    /// The symbol a *caller* can compare a refused decode against, or `None`
    /// on backends where the failure is not observable by name.
    pub fn observable_symbol(self, lang: Language) -> Option<&'static str> {
        match (lang, self) {
            (Language::Rust, CodecFailure::NeedMoreBytes) => Some("CodecError::NeedMoreBytes"),
            (Language::Rust, CodecFailure::TlvChainOverflow) => {
                Some("CodecError::TlvChainOverflow")
            }
            (Language::C11, CodecFailure::NeedMoreBytes) => Some("SCE_FORGE_CODEC_NEED_MORE_BYTES"),
            (Language::C11, CodecFailure::TlvChainOverflow) => {
                Some("SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW")
            }
            (Language::Go, CodecFailure::NeedMoreBytes) => Some("codec.ErrNeedMoreBytes"),
            (Language::Go, CodecFailure::TlvChainOverflow) => Some("codec.ErrTlvChainOverflow"),
            // The CBOR reader's refusals are named on Rust, whose decode
            // returns the runtime's `CodecError`. C11 and Go do not lower a
            // CBOR codec yet (`forge::cbor_codec::lowers`), so nothing there
            // can be observed; the arm names that instead of a symbol.
            (Language::Rust, CodecFailure::CborMalformed) => Some("CodecError::CborMalformed"),
            (Language::Rust, CodecFailure::CborRequiredKeyMissing) => {
                Some("CodecError::CborRequiredKeyMissing")
            }
            (Language::Rust, CodecFailure::CborWrongLength) => Some("CodecError::CborWrongLength"),
            (Language::Rust, CodecFailure::CborOutOfRange) => Some("CodecError::CborOutOfRange"),
            (
                Language::C11 | Language::Go,
                CodecFailure::CborMalformed
                | CodecFailure::CborRequiredKeyMissing
                | CodecFailure::CborWrongLength
                | CodecFailure::CborOutOfRange,
            ) => None,
            // Cpp / Kotlin / Python: refusal is observable, its name is not.
            (Language::Cpp | Language::Kotlin | Language::Python, _) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every failure round-trips through its oracle name, so an oracle
    /// author and this table cannot disagree about spelling.
    #[test]
    fn wire_names_round_trip() {
        for f in CodecFailure::ALL {
            assert_eq!(CodecFailure::parse(f.wire_name()), Ok(f));
        }
    }

    #[test]
    fn an_unknown_failure_name_is_refused_by_name() {
        let err = CodecFailure::parse("tlv-chain-overflow ").expect_err("trailing space is not it");
        assert!(err.contains("tlv-chain-overflow"), "{err}");
    }

    /// A backend that can name the failure must raise it by that same name —
    /// otherwise a reject vector would assert against a symbol the decode
    /// never produces.
    #[test]
    fn an_observable_symbol_is_the_one_the_emit_raises() {
        for lang in [
            Language::Rust,
            Language::C11,
            Language::Cpp,
            Language::Kotlin,
            Language::Go,
            Language::Python,
        ] {
            for f in CodecFailure::ALL {
                if let (Some(symbol), Some(raise)) = (f.observable_symbol(lang), f.raise_stmt(lang))
                {
                    assert!(
                        raise.contains(symbol),
                        "{lang:?} {f:?}: raise `{raise}` does not mention observable `{symbol}`",
                    );
                }
            }
        }
    }

    /// A positional failure is raised by a generated statement on every
    /// backend, and a CBOR one by none — the reader returns it.
    #[test]
    fn only_a_positional_failure_has_a_raise_statement() {
        for lang in [
            Language::Rust,
            Language::C11,
            Language::Cpp,
            Language::Kotlin,
            Language::Go,
            Language::Python,
        ] {
            for f in CodecFailure::ALL {
                assert_eq!(f.raise_stmt(lang).is_some(), !f.is_cbor(), "{lang:?} {f:?}");
            }
        }
    }

    /// The failures must be distinguishable wherever they are observable at
    /// all — a backend that named two the same would let a reject vector
    /// pass on the wrong refusal.
    #[test]
    fn observable_failures_are_distinguishable() {
        for lang in [
            Language::Rust,
            Language::C11,
            Language::Cpp,
            Language::Kotlin,
            Language::Go,
            Language::Python,
        ] {
            let named: Vec<&str> = CodecFailure::ALL
                .into_iter()
                .filter_map(|f| f.observable_symbol(lang))
                .collect();
            let mut distinct = named.clone();
            distinct.sort_unstable();
            distinct.dedup();
            assert_eq!(
                distinct.len(),
                named.len(),
                "{lang:?} names two failures identically"
            );
        }
    }
}
