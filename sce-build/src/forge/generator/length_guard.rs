// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A decode's check of a length or a count against its field's bound, and the
//! narrowing of the checked value to the host's length type (docs/adr/0077 §2).
//!
//! The value a frame says is compared in the type it was read in, before it is
//! narrowed to a host length, before a slice is cut by it and before anything
//! is allocated for it. A length outside the bound is not a frame of the
//! document, so it is refused with `LengthOutOfRange` (a list's count with
//! `TooManyElements`), never with `NeedMoreBytes`, which keeps the one meaning
//! "the frame is shorter than its own lengths say".
//!
//! Every decode path of a `tail` field, a `length-ref` field (bytes or text,
//! gated or not, positional or streaming) and a `<sce:repeat>` renders its check
//! through [`Guard`], so no path is without it.

use crate::forge::codec_failure::CodecFailure;
use crate::forge::model::{BitSize, CodecField, CountRef, LengthBound, SceType};
use crate::generator::Language;

/// What a value outside the bound is: a byte or text length, or a count of
/// entries. Each names the error every backend refuses it with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Excess {
    Length,
    Count,
}

/// The type a carried value is held in by the decode that compares it, as far
/// as the comparison's spelling goes. Kotlin compares an unsigned type with an
/// unsigned literal and an `Int` with a plain one; the other languages compare
/// either with a plain literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Held {
    /// An unsigned integer of these bits (`u8` … `u64`, `UByte` … `ULong`).
    Unsigned(u32),
    /// The host's signed length type: Kotlin's `Int`, Go's `int`, a
    /// remaining-bytes count, a dotted flag extracted through `Int`.
    HostInt,
}

/// The check one field's decode makes: the carried values it accepts,
/// `lo..=hi`, each `None` where no value the carrier can say is outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Guard {
    lo: Option<u64>,
    hi: Option<u64>,
    excess: Excess,
    bound: LengthBound,
}

impl Guard {
    /// The check of `field`'s length or count. `None` for a field whose size is
    /// not a length.
    pub(super) fn of(field: &CodecField, fields: &[CodecField]) -> Option<Self> {
        let bound = field.length_bound(fields)?;
        let excess = match field.bit_size {
            BitSize::Repeat { .. } | BitSize::TlvChain { .. } => Excess::Count,
            _ => Excess::Length,
        };
        let (lo, hi) = bound.carried_range();
        Some(Self {
            lo,
            hi,
            excess,
            bound,
        })
    }

    /// Whether every length the guard accepts is at most `i32::MAX`, so the
    /// host's narrowest length type holds it without a check of its own.
    pub(super) fn fits_host(&self) -> bool {
        self.bound.fits_i32()
    }

    /// `sce:length-arith`: the length is the carried value plus this.
    pub(super) fn arith(&self) -> i32 {
        self.bound.arith
    }

    /// The condition, in `lang`, under which `value` (held as `held`) is
    /// outside the accepted range; `None` where no value can be.
    fn condition(&self, lang: Language, value: &str, held: Held) -> Option<String> {
        let literal = |n: u64| -> String {
            match lang {
                Language::Kotlin => match held {
                    Held::Unsigned(64) => format!("{n}uL"),
                    Held::Unsigned(_) if n > u64::from(u32::MAX) => format!("{n}uL"),
                    Held::Unsigned(_) => format!("{n}u"),
                    Held::HostInt => n.to_string(),
                },
                Language::C11 | Language::Cpp if n > i32::MAX as u64 => format!("{n}ULL"),
                Language::C11
                | Language::Cpp
                | Language::Rust
                | Language::Go
                | Language::Python => n.to_string(),
                Language::Swift => no_swift(),
            }
        };
        // Rust spells a check with both ends as the range it accepts (its
        // `manual_range_contains` lint refuses the two comparisons).
        if let (Language::Rust, Some(lo), Some(hi)) = (lang, self.lo, self.hi) {
            return Some(format!(
                "!({}..={}).contains(&{value})",
                literal(lo),
                literal(hi)
            ));
        }
        let below = self.lo.map(|lo| format!("{value} < {}", literal(lo)));
        let above = self.hi.map(|hi| format!("{value} > {}", literal(hi)));
        match (below, above) {
            (None, None) => None,
            (Some(c), None) | (None, Some(c)) => Some(c),
            (Some(b), Some(a)) => {
                let or = match lang {
                    Language::Python => " or ",
                    Language::Rust
                    | Language::Cpp
                    | Language::C11
                    | Language::Kotlin
                    | Language::Go => " || ",
                    Language::Swift => {
                        crate::forge::codegen_matrix::no_swift_emitter("Guard::condition")
                    }
                };
                Some(format!("{b}{or}{a}"))
            }
        }
    }

    /// The statement, in `lang`, that refuses the frame: the one the failure
    /// table raises it with ([`CodecFailure::raise_stmt`]).
    fn refusal(&self, lang: Language) -> &'static str {
        let failure = match self.excess {
            Excess::Length => CodecFailure::LengthOutOfRange,
            Excess::Count => CodecFailure::TooManyElements,
        };
        failure
            .raise_stmt(lang)
            .expect("a bound's refusal has a raise statement on every backend")
    }

    /// The check of `value` (held as `held`) as a statement of `lang` at the
    /// indentation `indent`, followed by a newline and `indent` so the next
    /// statement continues where it stood; the empty string where nothing
    /// can be outside the bound.
    pub(super) fn stmt(&self, lang: Language, value: &str, held: Held, indent: &str) -> String {
        let Some(cond) = self.condition(lang, value, held) else {
            return String::new();
        };
        let step = if matches!(lang, Language::Go) {
            "\t"
        } else {
            "    "
        };
        let inner = format!("{indent}{step}");
        let refusal = self.refusal(lang);
        match lang {
            Language::Python => format!("if {cond}:\n{inner}{refusal}\n{indent}"),
            Language::Rust | Language::Go => {
                format!("if {cond} {{\n{inner}{refusal}\n{indent}}}\n{indent}")
            }
            Language::Cpp | Language::C11 | Language::Kotlin => {
                format!("if ({cond}) {{\n{inner}{refusal}\n{indent}}}\n{indent}")
            }
            Language::Swift => no_swift(),
        }
    }
}

impl Guard {
    /// The check, inside the loop of a list read to the end of the frame, that
    /// the entry about to be decoded is within the bound: a list that already
    /// holds `hi` entries with bytes left is past it, refused before the next
    /// entry is read, so a frame cut short there is refused for its count and
    /// not for its missing bytes. `len` is the number of entries read so far,
    /// held in the host's length type. Followed by a newline and `indent`;
    /// empty where the list has no bound.
    pub(super) fn full_stmt(&self, lang: Language, len: &str, indent: &str) -> String {
        match self.hi {
            Some(hi) => Guard {
                lo: None,
                hi: hi.checked_sub(1),
                ..*self
            }
            .stmt_or_always(lang, len, indent),
            None => String::new(),
        }
    }

    /// [`Self::stmt`] over a host length, or, for a bound of zero (no entry
    /// admitted), the refusal of any entry at all.
    fn stmt_or_always(&self, lang: Language, len: &str, indent: &str) -> String {
        match self.hi {
            Some(_) => self.stmt(lang, len, Held::HostInt, indent),
            None => {
                let zero = Guard {
                    hi: Some(0),
                    ..*self
                };
                // `len > 0` never holds before the first entry, so the first
                // entry itself is the one past a bound of zero: compare
                // `len + 1` instead.
                zero.stmt(lang, &format!("({len} + 1)"), Held::HostInt, indent)
            }
        }
    }
}

fn no_swift() -> String {
    crate::forge::codegen_matrix::no_swift_emitter("Guard::stmt")
}

/// How a host holds a codec integer field's value of the type `ty`.
pub(super) fn held_of(ty: &SceType) -> Held {
    match ty {
        SceType::Uint8 => Held::Unsigned(8),
        SceType::Uint16 => Held::Unsigned(16),
        SceType::Uint32 => Held::Unsigned(32),
        SceType::Uint64 => Held::Unsigned(64),
        _ => Held::HostInt,
    }
}

/// How a host holds the value a length or count field `name` carries, where
/// `name` names a sibling field or a dotted `<carrier>.<flag>`. A dotted flag
/// is extracted in the carrier's type in Rust, C11 and C++ (an unsigned type),
/// and through the host integer in Kotlin and Go; Python's integers are one
/// type.
pub(super) fn carried_held(name: &str, fields: &[CodecField], lang: Language) -> Held {
    let sibling = |id: &str| fields.iter().find(|f| f.id == id);
    match name.split_once('.') {
        Some((carrier, _)) => match lang {
            Language::Kotlin | Language::Go => Held::HostInt,
            Language::Rust | Language::C11 | Language::Cpp | Language::Python => {
                sibling(carrier.trim()).map_or(Held::HostInt, |f| held_of(&f.sce_type))
            }
            Language::Swift => crate::forge::codegen_matrix::no_swift_emitter("carried_held"),
        },
        None => sibling(name).map_or(Held::HostInt, |f| held_of(&f.sce_type)),
    }
}

/// Which errors a codec's decode can refuse a frame with on account of a
/// bound: a byte or text length (`LengthOutOfRange`) and a list's or a chain's
/// count (`TooManyElements`). A host that names each error it raises (Python's
/// imports) asks this.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct DecodeRefusals {
    pub length: bool,
    pub count: bool,
}

/// [`DecodeRefusals`] of the fields `fields` of one codec.
pub(super) fn decode_refusals(fields: &[CodecField]) -> DecodeRefusals {
    let mut out = DecodeRefusals::default();
    for field in fields {
        let Some(guard) = Guard::of(field, fields) else {
            continue;
        };
        let refuses = guard.lo.is_some() || guard.hi.is_some();
        match (guard.excess, &field.bit_size) {
            (Excess::Length, _) => out.length |= refuses,
            // A chain's frame says no count: the guard after its walk refuses.
            (Excess::Count, BitSize::TlvChain { .. }) => out.count = true,
            (Excess::Count, _) => out.count |= refuses,
        }
    }
    out
}

/// The field a `<sce:repeat count=>` reads its count from.
pub(super) fn repeat_count_field(field: &CodecField) -> Option<&str> {
    match &field.bit_size {
        BitSize::Repeat {
            count_ref: CountRef::LengthField(name),
        } => Some(name.as_str()),
        _ => None,
    }
}
