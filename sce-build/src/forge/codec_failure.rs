// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// RFC synth-5-B — the decode-side failure vocabulary a generated codec can
// signal, and each backend's spelling of it.
//
// This table is the one source of the kinds of rejection a decode answers
// (docs/adr/0099, amendment of 2026-11-16). Every runtime declares the whole
// of it in its own error type, under the name `runtime_name` derives from the
// kind's wire name by that language's rule, and
// `tests::every_runtime_declares_every_failure` reads each runtime's source to
// hold it there: a kind added here and not to a runtime, or a runtime's name
// spelled another way, fails that test rather than drifting.
//
// Three consumers share this table and must not grow private copies:
//   - `forge::generator` emits the statement that raises a failure from
//     inside a generated decode;
//   - `conformance` renders the assertion a reject vector makes about that
//     same failure from outside;
//   - the runtimes, whose declarations the test above holds to it.

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
    /// A text field's bytes are not well-formed UTF-8 (RFC 3629): the frame
    /// is refused and never waited on.
    InvalidUtf8,
    /// A field of a closed enum (`sce:strict-variants`, the default) whose
    /// carrier holds a value no variant declares: not a value of the type.
    UndeclaredEnumValue,
    /// A value inside its field's bound and past what the form that stores
    /// it holds: the capacity a deployment gave an inline buffer or list
    /// (docs/adr/0077 §7). Raised only where a container has a capacity of
    /// its own (Rust, C11); a runtime whose containers grow declares it and
    /// never raises it.
    CapacityExceeded,
    /// The frame's variant tag names an arm the document declares under a
    /// build feature this build leaves out (docs/adr/0072). Raised only where
    /// a build can leave an arm out (Rust, C11, C++); a runtime that keeps
    /// every arm declares it and never raises it.
    VariantArmNotBuilt,
    /// An `sce:encoding="cbor"` codec (SCE_FORGE.md §4.6.1) read an item that
    /// is not well-formed (RFC 8949 Appendix F), or one of indefinite length,
    /// which the codec's profile of definite lengths does not hold
    /// (docs/adr/0101 §3.1).
    CborMalformed,
    /// A CBOR item of a major type, a simple value or a tag the type at that
    /// place does not take (docs/adr/0101 §3.3): a text where a number
    /// stands, `null` where no list's optional element stands, a tagged
    /// item, a key of the wrong type.
    CborWrongType,
    /// A CBOR map holds one of its declared keys twice (RFC 8949 §5.6).
    CborDuplicateKey,
    /// A CBOR union's one key names none of its cases.
    CborUnknownVariant,
    /// A CBOR union's map holds no entry, or more than one.
    CborUnionEntries,
    /// A CBOR map lacks an entry of a bare type, one that is not `optional<T>`.
    CborRequiredKeyMissing,
    /// A CBOR byte string is not its declared `sce:length`.
    CborWrongLength,
    /// The value of a key the codec does not declare nests deeper than the
    /// skip admits (SCE_FORGE.md §4.6.1: 16).
    CborTooDeep,
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
    pub const ALL: [CodecFailure; 23] = [
        CodecFailure::NeedMoreBytes,
        CodecFailure::LengthOutOfRange,
        CodecFailure::TooManyElements,
        CodecFailure::VleWidthOverflow,
        CodecFailure::InvalidUtf8,
        CodecFailure::UndeclaredEnumValue,
        CodecFailure::CapacityExceeded,
        CodecFailure::VariantArmNotBuilt,
        CodecFailure::CborMalformed,
        CodecFailure::CborWrongType,
        CodecFailure::CborDuplicateKey,
        CodecFailure::CborUnknownVariant,
        CodecFailure::CborUnionEntries,
        CodecFailure::CborRequiredKeyMissing,
        CodecFailure::CborWrongLength,
        CodecFailure::CborTooDeep,
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
            CodecFailure::InvalidUtf8 => "invalid-utf8",
            CodecFailure::UndeclaredEnumValue => "undeclared-enum-value",
            CodecFailure::CapacityExceeded => "capacity-exceeded",
            CodecFailure::VariantArmNotBuilt => "variant-arm-not-built",
            CodecFailure::CborMalformed => "cbor-malformed",
            CodecFailure::CborWrongType => "cbor-wrong-type",
            CodecFailure::CborDuplicateKey => "cbor-duplicate-key",
            CodecFailure::CborUnknownVariant => "cbor-unknown-variant",
            CodecFailure::CborUnionEntries => "cbor-union-entries",
            CodecFailure::CborRequiredKeyMissing => "cbor-required-key-missing",
            CodecFailure::CborWrongLength => "cbor-wrong-length",
            CodecFailure::CborTooDeep => "cbor-too-deep",
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
                | CodecFailure::CborWrongType
                | CodecFailure::CborDuplicateKey
                | CodecFailure::CborUnknownVariant
                | CodecFailure::CborUnionEntries
                | CodecFailure::CborRequiredKeyMissing
                | CodecFailure::CborWrongLength
                | CodecFailure::CborTooDeep
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

    /// The name the runtime of `lang` declares this failure under, spelled from
    /// the wire name by that language's own rule for the declaration it is: a
    /// type or a variant in PascalCase (Rust, C++, Kotlin, Python), an enum
    /// case in lowerCamelCase (Swift, The Swift API Design Guidelines), an
    /// enumerator in upper snake case behind the runtime's prefix (C11), and an
    /// error sentinel `Err` + PascalCase with the initialisms in capitals (Go,
    /// Go Code Review Comments, "Initialisms"). No runtime's name is written
    /// here by hand, so a kind has one spelling per language and every lane
    /// reads it from this one place.
    pub fn runtime_name(self, lang: Language) -> String {
        let mut words = self.wire_name().split('-');
        match lang {
            Language::Rust | Language::Cpp | Language::Kotlin | Language::Python => {
                words.map(capitalized).collect()
            }
            Language::Swift => {
                let first = words.next().expect("a wire name has a first word");
                format!("{first}{}", words.map(capitalized).collect::<String>())
            }
            Language::C11 => format!(
                "SCE_FORGE_CODEC_{}",
                self.wire_name().replace('-', "_").to_ascii_uppercase()
            ),
            Language::Go => format!("Err{}", words.map(go_word).collect::<String>()),
        }
    }

    /// The expression a generated decode and a caller name this failure by in
    /// `lang`: the runtime's name, qualified as code outside the runtime
    /// reaches it.
    pub fn symbol(self, lang: Language) -> String {
        let name = self.runtime_name(lang);
        match lang {
            Language::Rust => format!("CodecError::{name}"),
            Language::Cpp => format!("::SCE::Forge::CodecError::{name}"),
            Language::Kotlin | Language::Swift => format!("CodecError.{name}"),
            Language::Go => format!("codec.{name}"),
            Language::Python | Language::C11 => name,
        }
    }

    /// The statement a generated decode refuses its frame with for this
    /// failure, in the channel each language has for it (docs/adr/0099,
    /// amendment of 2026-11-16): Rust's `Err`, a C11 status, a C++ and a Kotlin
    /// `Decoded` holding the error, Go's `error`, a Python exception and a
    /// Swift typed throw. Every kind has one on every backend: a decode is
    /// refused with the kind that refused it, never with a sentinel that
    /// stands for all of them.
    pub fn raise_stmt(self, lang: Language) -> String {
        let symbol = self.symbol(lang);
        match lang {
            Language::Rust => format!("return Err({symbol});"),
            Language::C11 | Language::Cpp => format!("return {symbol};"),
            Language::Kotlin => format!("return Decoded.Refused({symbol})"),
            Language::Go => format!("return nil, {symbol}"),
            Language::Python => format!("raise {symbol}()"),
            Language::Swift => format!("throw {symbol}"),
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
}

/// `word` with its first letter in capitals: one word of a PascalCase name.
fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// The words of a wire name that are initialisms, which a Go name spells in
/// capitals (`ErrVLEWidthOverflow`, `ErrInvalidUTF8`).
const GO_INITIALISMS: [&str; 2] = ["vle", "utf8"];

/// One word of a Go name: an initialism in capitals, any other capitalized.
fn go_word(word: &str) -> String {
    if GO_INITIALISMS.contains(&word) {
        word.to_ascii_uppercase()
    } else {
        capitalized(word)
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
        assert_eq!(CodecFailure::ALL.iter().filter(|f| f.is_cbor()).count(), 9);
    }

    /// The four kinds docs/adr/0101 §3.3 adds, spelled on each runtime as the
    /// record names them, so a runtime declares the name a reader of the
    /// record looks for.
    #[test]
    fn the_cbor_kinds_of_adr_0101_are_spelled_as_the_record_names_them() {
        let kinds = [
            CodecFailure::CborWrongType,
            CodecFailure::CborDuplicateKey,
            CodecFailure::CborUnknownVariant,
            CodecFailure::CborUnionEntries,
        ];
        let spelled = |lang: Language| kinds.map(|f| f.runtime_name(lang));
        assert_eq!(
            spelled(Language::Rust),
            [
                "CborWrongType",
                "CborDuplicateKey",
                "CborUnknownVariant",
                "CborUnionEntries"
            ]
        );
        assert_eq!(
            spelled(Language::Go),
            [
                "ErrCborWrongType",
                "ErrCborDuplicateKey",
                "ErrCborUnknownVariant",
                "ErrCborUnionEntries"
            ]
        );
        assert_eq!(
            spelled(Language::Swift),
            [
                "cborWrongType",
                "cborDuplicateKey",
                "cborUnknownVariant",
                "cborUnionEntries"
            ]
        );
        assert_eq!(
            spelled(Language::C11),
            [
                "SCE_FORGE_CODEC_CBOR_WRONG_TYPE",
                "SCE_FORGE_CODEC_CBOR_DUPLICATE_KEY",
                "SCE_FORGE_CODEC_CBOR_UNKNOWN_VARIANT",
                "SCE_FORGE_CODEC_CBOR_UNION_ENTRIES"
            ]
        );
    }

    #[test]
    fn an_unknown_failure_name_is_refused_by_name() {
        let err = CodecFailure::parse("too-many-elements ").expect_err("trailing space is not it");
        assert!(err.contains("too-many-elements"), "{err}");
    }

    /// Each language's rule, held on the kinds whose words test it: Go's
    /// initialisms, Swift's lower first word, C11's prefix.
    #[test]
    fn each_language_spells_a_kind_by_its_own_rule() {
        let vle = CodecFailure::VleWidthOverflow;
        assert_eq!(vle.runtime_name(Language::Rust), "VleWidthOverflow");
        assert_eq!(vle.runtime_name(Language::Go), "ErrVLEWidthOverflow");
        assert_eq!(vle.runtime_name(Language::Swift), "vleWidthOverflow");
        assert_eq!(
            vle.runtime_name(Language::C11),
            "SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW"
        );
        let utf8 = CodecFailure::InvalidUtf8;
        assert_eq!(utf8.runtime_name(Language::Kotlin), "InvalidUtf8");
        assert_eq!(utf8.runtime_name(Language::Go), "ErrInvalidUTF8");
        assert_eq!(
            CodecFailure::CborRequiredKeyMissing.symbol(Language::Cpp),
            "::SCE::Forge::CodecError::CborRequiredKeyMissing"
        );
        assert_eq!(
            CodecFailure::NeedMoreBytes.symbol(Language::Python),
            "NeedMoreBytes"
        );
    }

    /// A decode refuses with the kind that refused it on every backend: the
    /// statement names the symbol a caller compares the refusal with, so a
    /// reject vector asserts against the name the decode produces.
    #[test]
    fn every_raise_names_its_kind_on_every_backend() {
        for &lang in Language::ALL {
            for f in CodecFailure::ALL {
                let raise = f.raise_stmt(lang);
                let symbol = f.symbol(lang);
                assert!(
                    raise.contains(&symbol),
                    "{lang:?} {f:?}: raise `{raise}` does not name `{symbol}`",
                );
            }
        }
    }

    /// The failures are distinguishable on every backend: one that named two
    /// the same would let a reject vector pass on the wrong refusal.
    #[test]
    fn no_backend_names_two_failures_alike() {
        for &lang in Language::ALL {
            let mut named: Vec<String> = CodecFailure::ALL
                .into_iter()
                .map(|f| f.symbol(lang))
                .collect();
            named.sort_unstable();
            named.dedup();
            assert_eq!(
                named.len(),
                CodecFailure::ALL.len(),
                "{lang:?} names two failures identically"
            );
        }
    }

    /// The source of each runtime's codec error type.
    fn runtime_sources(lang: Language) -> &'static [&'static str] {
        match lang {
            Language::Rust => &[include_str!(
                "../../../backends/rust/forge-runtime/src/codec.rs"
            )],
            Language::Cpp => &[include_str!(
                "../../../backends/cpp/forge-runtime/include/sce/forge/codec.h"
            )],
            Language::Kotlin => &[include_str!(
                "../../../backends/kotlin/forge-runtime/src/commonMain/kotlin/com/sce/forge/runtime/Codec.kt"
            )],
            Language::Go => &[
                include_str!("../../../backends/go/forge-runtime/codec/codec.go"),
                include_str!("../../../backends/go/forge-runtime/codec/cbor.go"),
                include_str!("../../../backends/go/forge-runtime/codec/content_line.go"),
            ],
            Language::Python => &[
                include_str!("../../../backends/python/forge-runtime/sce_forge_runtime/codec.py"),
                include_str!("../../../backends/python/forge-runtime/sce_forge_runtime/cbor.py"),
                include_str!(
                    "../../../backends/python/forge-runtime/sce_forge_runtime/content_line.py"
                ),
            ],
            Language::C11 => &[include_str!(
                "../../../backends/c/forge-runtime/include/sce/forge/codec.h"
            )],
            Language::Swift => &[include_str!(
                "../../../backends/swift/forge-runtime/Sources/SceForgeRuntime/Codec.swift"
            )],
        }
    }

    /// The text from `open` up to the first `close` after it: the body of the
    /// one declaration a runtime holds its error type in.
    fn declaration<'s>(lang: Language, text: &'s str, open: &str, close: &str) -> &'s str {
        let start = text
            .find(open)
            .unwrap_or_else(|| panic!("{lang:?}: the runtime declares no `{open}`"));
        let body = &text[start..];
        let end = body
            .find(close)
            .unwrap_or_else(|| panic!("{lang:?}: `{open}` has no `{close}`"));
        &body[..end]
    }

    /// Whether the runtime of `lang` declares `name` as one kind of its codec
    /// error: a variant of its error enum (Rust, Swift, C++, C11), an object of
    /// its sealed class (Kotlin), a subclass of `CodecError` (Python) or an
    /// error sentinel of the `codec` package (Go).
    fn declares(lang: Language, name: &str) -> bool {
        let sources = runtime_sources(lang);
        // `line` is `before`, then `name`, then text that starts with `after`
        // (that ends there, where `after` is empty).
        let names = |line: &str, before: &str, after: &str| {
            line.strip_prefix(before)
                .and_then(|rest| rest.strip_prefix(name))
                .is_some_and(|rest| {
                    if after.is_empty() {
                        rest.is_empty()
                    } else {
                        rest.starts_with(after)
                    }
                })
        };
        let in_block = |open: &str, close: &str, before: &str, after: &str| {
            sources.iter().any(|text| {
                declaration(lang, text, open, close)
                    .lines()
                    .any(|l| names(l.trim(), before, after))
            })
        };
        let at_top = |before: &str, after: &str| {
            sources
                .iter()
                .any(|text| text.lines().any(|l| names(l, before, after)))
        };
        match lang {
            Language::Rust => in_block("pub enum CodecError {", "\n}", "", ","),
            Language::Swift => in_block("public enum CodecError: Error", "\n}", "case ", ""),
            Language::Cpp => in_block("enum class CodecError", "\n};", "", " = "),
            Language::C11 => in_block("typedef enum {", "} sce_forge_codec_status_t;", "", " = "),
            Language::Kotlin => in_block(
                "sealed class CodecError {",
                "\n}",
                "data object ",
                " : CodecError()",
            ),
            Language::Python => at_top("class ", "(CodecError):"),
            Language::Go => at_top("var ", " = errors.New("),
        }
    }

    /// Every runtime declares every kind of this table under the name derived
    /// for it: the vocabulary is one, and a runtime that lacks a kind, or
    /// spells one another way, is named here with the kind it lacks.
    #[test]
    fn every_runtime_declares_every_failure() {
        let mut missing = Vec::new();
        for &lang in Language::ALL {
            for f in CodecFailure::ALL {
                let name = f.runtime_name(lang);
                if !declares(lang, &name) {
                    missing.push(format!("{lang:?} {name}"));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "runtimes lacking a kind of the decode vocabulary: {missing:?}"
        );
    }
}
