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
}

impl CodecFailure {
    /// Kebab-case name naming this failure in the conformance oracle's
    /// reject vectors.
    pub fn wire_name(self) -> &'static str {
        match self {
            CodecFailure::NeedMoreBytes => "need-more-bytes",
            CodecFailure::TlvChainOverflow => "tlv-chain-overflow",
        }
    }

    /// Parse an oracle `error` value. The set is closed: an unrecognised
    /// name is an authoring mistake, and failing the render is how the
    /// author hears about it.
    pub fn parse(name: &str) -> Result<Self, String> {
        [CodecFailure::NeedMoreBytes, CodecFailure::TlvChainOverflow]
            .into_iter()
            .find(|f| f.wire_name() == name)
            .ok_or_else(|| {
                format!(
                    "unknown codec failure '{name}' — expected one of: {}",
                    [CodecFailure::NeedMoreBytes, CodecFailure::TlvChainOverflow]
                        .map(|f| f.wire_name())
                        .join(", ")
                )
            })
    }

    /// The statement a generated decode uses to signal this failure.
    pub fn raise_stmt(self, lang: Language) -> &'static str {
        match (lang, self) {
            (Language::Rust, CodecFailure::NeedMoreBytes) => {
                "return Err(CodecError::NeedMoreBytes);"
            }
            (Language::Rust, CodecFailure::TlvChainOverflow) => {
                "return Err(CodecError::TlvChainOverflow);"
            }
            (Language::C11, CodecFailure::NeedMoreBytes) => {
                "return SCE_FORGE_CODEC_NEED_MORE_BYTES;"
            }
            (Language::C11, CodecFailure::TlvChainOverflow) => {
                "return SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW;"
            }
            (Language::Cpp, _) => "return std::nullopt;",
            (Language::Kotlin, _) => "return null",
            (Language::Go, CodecFailure::NeedMoreBytes) => "return nil, codec.ErrNeedMoreBytes",
            (Language::Go, CodecFailure::TlvChainOverflow) => {
                "return nil, codec.ErrTlvChainOverflow"
            }
            (Language::Python, CodecFailure::NeedMoreBytes) => "raise NeedMoreBytes()",
            (Language::Python, CodecFailure::TlvChainOverflow) => "raise TlvChainOverflow()",
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
        for f in [CodecFailure::NeedMoreBytes, CodecFailure::TlvChainOverflow] {
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
            for f in [CodecFailure::NeedMoreBytes, CodecFailure::TlvChainOverflow] {
                if let Some(symbol) = f.observable_symbol(lang) {
                    assert!(
                        f.raise_stmt(lang).contains(symbol),
                        "{lang:?} {f:?}: raise `{}` does not mention observable `{symbol}`",
                        f.raise_stmt(lang),
                    );
                }
            }
        }
    }

    /// The two failures must be distinguishable wherever they are
    /// observable at all — a backend that named both the same would let a
    /// reject vector pass on the wrong refusal.
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
            let short = CodecFailure::NeedMoreBytes.observable_symbol(lang);
            let over = CodecFailure::TlvChainOverflow.observable_symbol(lang);
            if short.is_some() || over.is_some() {
                assert_ne!(short, over, "{lang:?} names both failures identically");
            }
        }
    }
}
