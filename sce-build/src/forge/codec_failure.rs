// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
// table answers both. Cpp and Kotlin return one sentinel for every failure
// (the "MCU-only codec sub-features" convention); a refusal that is not a
// frame short of bytes is recorded on the cursor beside it (a bound's,
// docs/adr/0077 §4), and Python's generated `decode` funnels every
// `CodecError` through one `except` into `None`. For all three the failure
// is raised by name inside the decode but is *not* observable by name
// outside it, so `observable_symbol` reports `None` and a caller can only
// assert that the decode refused — which is the whole of what it can see.

use crate::generator::Language;

/// What a field's bound counts, which names the failure a value outside it is
/// refused with ([`CodecFailure::over_bound`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundKind {
    /// The bytes of a `bytes` or `string` field.
    Length,
    /// The entries of a `<sce:repeat>` or a `<sce:tlv-chain>`.
    Count,
}

impl BoundKind {
    /// What the bound of `field` counts, or `None` for a field whose size is
    /// not held to a bound (a number, an embedded body).
    pub fn of(field: &crate::forge::model::CodecField) -> Option<Self> {
        use crate::forge::model::BitSize;
        Some(match field.bit_size {
            BitSize::Tail | BitSize::LengthRef => BoundKind::Length,
            BitSize::Repeat { .. } | BitSize::TlvChain { .. } => BoundKind::Count,
            BitSize::Fixed { .. } | BitSize::Vle { .. } | BitSize::Embed => return None,
        })
    }
}

/// A decode-side failure a generated codec can signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecFailure {
    /// The peer's frame ended before the codec's declared shape was
    /// complete. Under `terminate-on="entry-flag"` this includes a chain
    /// whose last entry declared a successor the wire never carried.
    NeedMoreBytes,
    /// A byte string's or a text's length outside its bound, the declared
    /// `sce:max-size` intersected with what its length field can say after
    /// its `sce:length-arith` (docs/adr/0077 §1), judged before the frame's
    /// own length.
    LengthOutOfRange,
    /// A list's count past its declared `max-count`, an entry past it in a
    /// list read to the end of the frame (docs/adr/0077 §2), or a chain's
    /// entry past its declared `max-depth`.
    TooManyElements,
    /// A `sce:bit-size="vle"` field's ZInt holds a value past the field's
    /// width (`codec/vle-width-overflow`). The wire of every width is a
    /// ZInt64, so a longer encoding of a value the type holds is no such
    /// failure: only the value is. Raised by the runtime's VLE reader, not by
    /// a generated statement.
    VleWidthOverflow,
    /// An `sce:encoding="cbor"` map is not one the codec reads
    /// (SCE_FORGE.md §4.6.1): not a definite-length map, a key given twice,
    /// or an entry of the wrong major type.
    CborMalformed,
    /// A CBOR map lacks an entry of a bare type, one that is not `optional<T>`.
    CborRequiredKeyMissing,
    /// A CBOR byte string is not its declared `sce:length`.
    CborWrongLength,
    /// A CBOR value does not fit its declared type or `sce:max-size`.
    CborOutOfRange,
    /// A `sce:encoding="content-line"` component has a line, a parameter or
    /// an `END:` the grammar does not admit (SCE_FORGE.md §4.6.4).
    LineMalformed,
    /// A content-line component lacks a property or parameter of a bare type,
    /// one that is not `optional<T>`; on encode, a parameter was given without
    /// the property it belongs to.
    LineRequiredMissing,
    /// A content-line property that holds one value occurs twice, a
    /// parameter is given twice in one line, or a list passes its
    /// `sce:max-count`.
    LineTooMany,
    /// A content-line value passes its `sce:max-size`.
    LineTooLong,
    /// A content-line TEXT carries an escape other than `\\`, `\;`, `\,`,
    /// `\n` and `\N`.
    LineBadEscape,
    /// A content-line value its entry cannot hold: a control character,
    /// invalid UTF-8, an integer or `bool` out of its type, a parameter of
    /// more values than one, a `"` in a parameter value.
    LineBadValue,
}

impl CodecFailure {
    /// Every failure, in declaration order — the set `parse` accepts.
    pub const ALL: [CodecFailure; 14] = [
        CodecFailure::NeedMoreBytes,
        CodecFailure::LengthOutOfRange,
        CodecFailure::TooManyElements,
        CodecFailure::VleWidthOverflow,
        CodecFailure::CborMalformed,
        CodecFailure::CborRequiredKeyMissing,
        CodecFailure::CborWrongLength,
        CodecFailure::CborOutOfRange,
        CodecFailure::LineMalformed,
        CodecFailure::LineRequiredMissing,
        CodecFailure::LineTooMany,
        CodecFailure::LineTooLong,
        CodecFailure::LineBadEscape,
        CodecFailure::LineBadValue,
    ];

    /// Kebab-case name naming this failure in the conformance oracle's
    /// reject vectors.
    pub fn wire_name(self) -> &'static str {
        match self {
            CodecFailure::NeedMoreBytes => "need-more-bytes",
            CodecFailure::LengthOutOfRange => "length-out-of-range",
            CodecFailure::TooManyElements => "too-many-elements",
            CodecFailure::VleWidthOverflow => "vle-width-overflow",
            CodecFailure::CborMalformed => "cbor-malformed",
            CodecFailure::CborRequiredKeyMissing => "cbor-required-key-missing",
            CodecFailure::CborWrongLength => "cbor-wrong-length",
            CodecFailure::CborOutOfRange => "cbor-out-of-range",
            CodecFailure::LineMalformed => "line-malformed",
            CodecFailure::LineRequiredMissing => "line-required-missing",
            CodecFailure::LineTooMany => "line-too-many",
            CodecFailure::LineTooLong => "line-too-long",
            CodecFailure::LineBadEscape => "line-bad-escape",
            CodecFailure::LineBadValue => "line-bad-value",
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

    /// Whether this is a failure of the content-line reader or writer
    /// (SCE_FORGE.md §4.6.4) rather than of a positional decode.
    pub fn is_content_line(self) -> bool {
        matches!(
            self,
            CodecFailure::LineMalformed
                | CodecFailure::LineRequiredMissing
                | CodecFailure::LineTooMany
                | CodecFailure::LineTooLong
                | CodecFailure::LineBadEscape
                | CodecFailure::LineBadValue
        )
    }

    /// Whether a runtime reader raises this failure and no generated statement
    /// does: a CBOR or content-line failure, which the readers and the codec
    /// templates of those encodings return as a value (SCE_FORGE.md §4.6.1,
    /// §4.6.4), and a VLE's value past its width, which the runtime's VLE
    /// reader returns and the generated decode passes on.
    pub fn is_raised_by_a_reader(self) -> bool {
        self.is_cbor() || self.is_content_line() || self == CodecFailure::VleWidthOverflow
    }

    /// The statement a generated positional decode uses to signal this
    /// failure. `None` for a failure a runtime reader raises
    /// ([`Self::is_raised_by_a_reader`]): the positional codec's statements
    /// do not raise one.
    pub fn raise_stmt(self, lang: Language) -> Option<&'static str> {
        match self {
            CodecFailure::NeedMoreBytes => Some(match lang {
                Language::Rust => "return Err(CodecError::NeedMoreBytes);",
                Language::C11 => "return SCE_FORGE_CODEC_NEED_MORE_BYTES;",
                Language::Cpp => "return std::nullopt;",
                Language::Kotlin => "return null",
                Language::Go => "return nil, codec.ErrNeedMoreBytes",
                Language::Python => "raise NeedMoreBytes()",
                Language::Swift => "throw CodecError.needMoreBytes",
            }),
            // A bound's refusal is not a frame short of bytes, so C++ and
            // Kotlin record it on the cursor before they return their one
            // sentinel (docs/adr/0077 §4; `SceCursor::refuse`, as ADR 0072
            // records a left-out arm's).
            CodecFailure::LengthOutOfRange => Some(match lang {
                Language::Rust => "return Err(CodecError::LengthOutOfRange);",
                Language::C11 => "return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;",
                Language::Cpp => {
                    "cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;"
                }
                Language::Kotlin => "cursor.refuse(CodecError.LengthOutOfRange); return null",
                Language::Go => "return nil, codec.ErrLengthOutOfRange",
                Language::Python => "raise LengthOutOfRange()",
                Language::Swift => "throw CodecError.lengthOutOfRange",
            }),
            CodecFailure::TooManyElements => Some(match lang {
                Language::Rust => "return Err(CodecError::TooManyElements);",
                Language::C11 => "return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;",
                Language::Cpp => {
                    "cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt;"
                }
                Language::Kotlin => "cursor.refuse(CodecError.TooManyElements); return null",
                Language::Go => "return nil, codec.ErrTooManyElements",
                Language::Python => "raise TooManyElements()",
                Language::Swift => "throw CodecError.tooManyElements",
            }),
            CodecFailure::VleWidthOverflow
            | CodecFailure::CborMalformed
            | CodecFailure::CborRequiredKeyMissing
            | CodecFailure::CborWrongLength
            | CodecFailure::CborOutOfRange
            | CodecFailure::LineMalformed
            | CodecFailure::LineRequiredMissing
            | CodecFailure::LineTooMany
            | CodecFailure::LineTooLong
            | CodecFailure::LineBadEscape
            | CodecFailure::LineBadValue => None,
        }
    }

    /// The statement a generated encode uses to refuse a value whose length or
    /// count is outside its field's bound, before its first byte is written
    /// (docs/adr/0077 §3), on the backends whose encode returns or raises the
    /// error itself. `None` for a failure an encode does not refuse with, and
    /// for Rust, whose source cannot present such a value.
    ///
    /// The error is the one a decode of that frame gives (docs/adr/0058 law
    /// 3).
    pub fn encode_refusal_stmt(self, lang: Language) -> Option<&'static str> {
        let (length, count) = match lang {
            Language::C11 => (
                "return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;",
                "return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;",
            ),
            Language::Cpp => (
                "return ::SCE::Forge::CodecError::LengthOutOfRange;",
                "return ::SCE::Forge::CodecError::TooManyElements;",
            ),
            Language::Kotlin => (
                "return CodecError.LengthOutOfRange",
                "return CodecError.TooManyElements",
            ),
            Language::Go => (
                "return codec.ErrLengthOutOfRange",
                "return codec.ErrTooManyElements",
            ),
            Language::Python => ("raise LengthOutOfRange()", "raise TooManyElements()"),
            Language::Swift => (
                "throw CodecError.lengthOutOfRange",
                "throw CodecError.tooManyElements",
            ),
            Language::Rust => return None,
        };
        match self {
            CodecFailure::LengthOutOfRange => Some(length),
            CodecFailure::TooManyElements => Some(count),
            _ => None,
        }
    }

    /// The failure a value outside a field's bound is refused with: a byte or
    /// text length `LengthOutOfRange`, a list's or a chain's count
    /// `TooManyElements` (docs/adr/0077 §4).
    pub fn over_bound(kind: BoundKind) -> Self {
        match kind {
            BoundKind::Length => CodecFailure::LengthOutOfRange,
            BoundKind::Count => CodecFailure::TooManyElements,
        }
    }

    /// The symbol a caller compares the cursor's record of a refusal against
    /// on the backends whose decode returns one sentinel for every failure
    /// and records the reason beside it (C++ `SceCursor::last_refusal`,
    /// Kotlin `SceCursor.lastRefusal`), or `None` where the failure is not
    /// recorded there. A frame short of bytes records nothing, and a caller
    /// can check that too ([`Self::records_nothing`]).
    pub fn cursor_record(self, lang: Language) -> Option<&'static str> {
        let (length, count) = match lang {
            Language::Cpp => (
                "::SCE::Forge::CodecError::LengthOutOfRange",
                "::SCE::Forge::CodecError::TooManyElements",
            ),
            Language::Kotlin => (
                "com.sce.forge.runtime.CodecError.LengthOutOfRange",
                "com.sce.forge.runtime.CodecError.TooManyElements",
            ),
            Language::Rust | Language::C11 | Language::Go | Language::Python | Language::Swift => {
                return None
            }
        };
        match self {
            CodecFailure::LengthOutOfRange => Some(length),
            CodecFailure::TooManyElements => Some(count),
            _ => None,
        }
    }

    /// The flag of the cursor that holds when a decode refused for this reason,
    /// read as `<cursor>.<flag>`, on the backends whose decode returns one
    /// sentinel for every failure and keeps this one's reason on the cursor
    /// apart from its record of refusals: the VLE reader's overflow flag
    /// (C++ `SceCursor::last_vle_overflow`, Kotlin `SceCursor.lastVleOverflow`).
    /// `None` for any other failure or backend.
    pub fn cursor_flag(self, lang: Language) -> Option<&'static str> {
        match (lang, self) {
            (Language::Cpp, CodecFailure::VleWidthOverflow) => Some("last_vle_overflow()"),
            (Language::Kotlin, CodecFailure::VleWidthOverflow) => Some("lastVleOverflow"),
            _ => None,
        }
    }

    /// Whether a refusal of this kind leaves the cursor's record empty on the
    /// backends that keep one: a frame short of bytes is the one a caller may
    /// resume by appending, so it must not read as a refusal for a reason.
    pub fn records_nothing(self, lang: Language) -> bool {
        matches!(lang, Language::Cpp | Language::Kotlin)
            && matches!(self, CodecFailure::NeedMoreBytes)
    }

    /// The symbol a *caller* can compare a refused decode against, or `None`
    /// on backends where the failure is not observable by name.
    pub fn observable_symbol(self, lang: Language) -> Option<&'static str> {
        match (lang, self) {
            (Language::Rust, CodecFailure::NeedMoreBytes) => Some("CodecError::NeedMoreBytes"),
            (Language::C11, CodecFailure::NeedMoreBytes) => Some("SCE_FORGE_CODEC_NEED_MORE_BYTES"),
            (Language::Go, CodecFailure::NeedMoreBytes) => Some("codec.ErrNeedMoreBytes"),
            // A bound's refusal (docs/adr/0077 §4) is named on the same three.
            (Language::Rust, CodecFailure::LengthOutOfRange) => {
                Some("CodecError::LengthOutOfRange")
            }
            (Language::Rust, CodecFailure::TooManyElements) => Some("CodecError::TooManyElements"),
            (Language::C11, CodecFailure::LengthOutOfRange) => {
                Some("SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE")
            }
            (Language::C11, CodecFailure::TooManyElements) => {
                Some("SCE_FORGE_CODEC_TOO_MANY_ELEMENTS")
            }
            (Language::Go, CodecFailure::LengthOutOfRange) => Some("codec.ErrLengthOutOfRange"),
            (Language::Go, CodecFailure::TooManyElements) => Some("codec.ErrTooManyElements"),
            // A VLE's value past its width is the runtime reader's own error,
            // which the generated decode returns as it is on the same three.
            (Language::Rust, CodecFailure::VleWidthOverflow) => {
                Some("CodecError::VleWidthOverflow")
            }
            (Language::C11, CodecFailure::VleWidthOverflow) => {
                Some("SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW")
            }
            (Language::Go, CodecFailure::VleWidthOverflow) => Some("codec.ErrVLEWidthOverflow"),
            // The CBOR reader's refusals are named on Rust, Go and C11, whose
            // decodes return the runtime's typed error or status.
            (Language::Rust, CodecFailure::CborMalformed) => Some("CodecError::CborMalformed"),
            (Language::Rust, CodecFailure::CborRequiredKeyMissing) => {
                Some("CodecError::CborRequiredKeyMissing")
            }
            (Language::Rust, CodecFailure::CborWrongLength) => Some("CodecError::CborWrongLength"),
            (Language::Rust, CodecFailure::CborOutOfRange) => Some("CodecError::CborOutOfRange"),
            (Language::Go, CodecFailure::CborMalformed) => Some("codec.ErrCborMalformed"),
            (Language::Go, CodecFailure::CborRequiredKeyMissing) => {
                Some("codec.ErrCborRequiredKeyMissing")
            }
            (Language::Go, CodecFailure::CborWrongLength) => Some("codec.ErrCborWrongLength"),
            (Language::Go, CodecFailure::CborOutOfRange) => Some("codec.ErrCborOutOfRange"),
            (Language::C11, CodecFailure::CborMalformed) => Some("SCE_FORGE_CODEC_CBOR_MALFORMED"),
            (Language::C11, CodecFailure::CborRequiredKeyMissing) => {
                Some("SCE_FORGE_CODEC_CBOR_REQUIRED_KEY_MISSING")
            }
            (Language::C11, CodecFailure::CborWrongLength) => {
                Some("SCE_FORGE_CODEC_CBOR_WRONG_LENGTH")
            }
            (Language::C11, CodecFailure::CborOutOfRange) => {
                Some("SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE")
            }
            // The content-line reader and writer return the runtime's typed
            // error or status on the same three backends.
            (Language::Rust, CodecFailure::LineMalformed) => Some("CodecError::LineMalformed"),
            (Language::Rust, CodecFailure::LineRequiredMissing) => {
                Some("CodecError::LineRequiredMissing")
            }
            (Language::Rust, CodecFailure::LineTooMany) => Some("CodecError::LineTooMany"),
            (Language::Rust, CodecFailure::LineTooLong) => Some("CodecError::LineTooLong"),
            (Language::Rust, CodecFailure::LineBadEscape) => Some("CodecError::LineBadEscape"),
            (Language::Rust, CodecFailure::LineBadValue) => Some("CodecError::LineBadValue"),
            (Language::Go, CodecFailure::LineMalformed) => Some("codec.ErrLineMalformed"),
            (Language::Go, CodecFailure::LineRequiredMissing) => {
                Some("codec.ErrLineRequiredMissing")
            }
            (Language::Go, CodecFailure::LineTooMany) => Some("codec.ErrLineTooMany"),
            (Language::Go, CodecFailure::LineTooLong) => Some("codec.ErrLineTooLong"),
            (Language::Go, CodecFailure::LineBadEscape) => Some("codec.ErrLineBadEscape"),
            (Language::Go, CodecFailure::LineBadValue) => Some("codec.ErrLineBadValue"),
            (Language::C11, CodecFailure::LineMalformed) => Some("SCE_FORGE_CODEC_LINE_MALFORMED"),
            (Language::C11, CodecFailure::LineRequiredMissing) => {
                Some("SCE_FORGE_CODEC_LINE_REQUIRED_MISSING")
            }
            (Language::C11, CodecFailure::LineTooMany) => Some("SCE_FORGE_CODEC_LINE_TOO_MANY"),
            (Language::C11, CodecFailure::LineTooLong) => Some("SCE_FORGE_CODEC_LINE_TOO_LONG"),
            (Language::C11, CodecFailure::LineBadEscape) => Some("SCE_FORGE_CODEC_LINE_BAD_ESCAPE"),
            (Language::C11, CodecFailure::LineBadValue) => Some("SCE_FORGE_CODEC_LINE_BAD_VALUE"),
            // Cpp / Kotlin / Python: refusal is observable, its name is not.
            // Swift's decode throws the runtime's typed error, which the conformance schedule
            // does not yet name.
            (Language::Cpp | Language::Kotlin | Language::Python | Language::Swift, _) => None,
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

    /// A failure belongs to one family: a reject vector names the family its
    /// codec has, and the harness refuses one from another.
    #[test]
    fn a_failure_is_cbor_or_content_line_or_neither_and_never_both() {
        for f in CodecFailure::ALL {
            assert!(!(f.is_cbor() && f.is_content_line()), "{f:?}");
        }
        assert_eq!(
            CodecFailure::ALL
                .iter()
                .filter(|f| f.is_content_line())
                .count(),
            6
        );
    }

    #[test]
    fn an_unknown_failure_name_is_refused_by_name() {
        let err = CodecFailure::parse("too-many-elements ").expect_err("trailing space is not it");
        assert!(err.contains("too-many-elements"), "{err}");
    }

    /// A backend that can name the failure must raise it by that same name —
    /// otherwise a reject vector would assert against a symbol the decode
    /// never produces.
    #[test]
    fn an_observable_symbol_is_the_one_the_emit_raises() {
        // Only a backend that emits a codec raises anything.
        for lang in crate::forge::codegen_matrix::emitting(crate::forge::model::ForgeKind::Codec) {
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

    /// A failure a generated statement raises has a raise statement on every
    /// backend, and one a runtime reader raises (CBOR, content-line, a VLE's
    /// value past its width) on none — the reader returns it.
    #[test]
    fn only_a_failure_no_reader_raises_has_a_raise_statement() {
        for lang in crate::forge::codegen_matrix::emitting(crate::forge::model::ForgeKind::Codec) {
            for f in CodecFailure::ALL {
                assert_eq!(
                    f.raise_stmt(lang).is_some(),
                    !f.is_raised_by_a_reader(),
                    "{lang:?} {f:?}"
                );
            }
        }
    }

    /// A VLE's value past its width is named where the runtime's reader
    /// returns a typed error (Rust, C11, Go), and read from the cursor's flag
    /// where the decode answers one sentinel (C++, Kotlin): a reject vector of
    /// it asserts the reason on five backends, not only that a decode refused.
    #[test]
    fn a_vle_past_its_width_is_observable_by_name_or_by_the_cursor_flag() {
        let f = CodecFailure::VleWidthOverflow;
        assert_eq!(CodecFailure::parse("vle-width-overflow"), Ok(f));
        assert!(!f.is_cbor() && !f.is_content_line() && f.is_raised_by_a_reader());
        for (lang, symbol) in [
            (Language::Rust, "CodecError::VleWidthOverflow"),
            (Language::C11, "SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW"),
            (Language::Go, "codec.ErrVLEWidthOverflow"),
        ] {
            assert_eq!(f.observable_symbol(lang), Some(symbol), "{lang:?}");
            assert_eq!(f.cursor_flag(lang), None, "{lang:?}");
        }
        assert_eq!(f.cursor_flag(Language::Cpp), Some("last_vle_overflow()"));
        assert_eq!(f.cursor_flag(Language::Kotlin), Some("lastVleOverflow"));
        for lang in [Language::Python, Language::Swift] {
            assert_eq!(f.observable_symbol(lang), None, "{lang:?}");
            assert_eq!(f.cursor_flag(lang), None, "{lang:?}");
        }
        // No other failure is read from a cursor flag.
        for &lang in Language::ALL {
            for other in CodecFailure::ALL.into_iter().filter(|o| *o != f) {
                assert_eq!(other.cursor_flag(lang), None, "{lang:?} {other:?}");
            }
        }
    }

    /// The failures must be distinguishable wherever they are observable at
    /// all — a backend that named two the same would let a reject vector
    /// pass on the wrong refusal.
    #[test]
    fn observable_failures_are_distinguishable() {
        for &lang in Language::ALL {
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
