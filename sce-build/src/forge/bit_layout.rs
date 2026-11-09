//! Where the bits of a codec's fixed fields lie in its frame (docs/adr/0095).
//!
//! A fixed field of `N` bits at byte `B` and bit offset `O`, in byte order `E`,
//! spans the `W = ⌈(O + N) / 8⌉` bytes from `B`. Those bytes, read as one
//! unsigned integer in order `E`, are its word, and its value is bits
//! `O .. O + N` of the word, counted from the least significant bit.
//!
//! This module is the one place that turns `sce:byte`, `sce:bit-offset`,
//! `sce:bit-size` and `sce:endian` into bits. The parser refuses by it, the
//! encode plan and every backend write and read by it, and the frame's size and
//! drawing measure by it.

use crate::forge::model::{BitSize, CodecField, Endian, SceType};

/// The bits a fixed field holds: `bits` bits from bit `shift` of the word that
/// the `bytes` bytes from `byte` make in byte order `endian`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitSpan {
    /// The first byte of the word, the field's `sce:byte`.
    pub byte: u32,
    /// How many bytes the word spans.
    pub bytes: u32,
    /// The bit of the word the value starts at, the field's `sce:bit-offset`.
    pub shift: u32,
    /// How many bits the value has, the field's `sce:bit-size`.
    pub bits: u32,
    /// The order the word's bytes are in; `native` is read as big, as every
    /// backend reads it.
    pub endian: Endian,
}

/// The bits of a field's value that one byte of the frame holds: the value
/// moved down by `from` and masked by `mask`, then moved up by `to` within the
/// byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    /// The value's first bit in this byte.
    pub from: u32,
    /// The mask of the bits in this byte, at the least significant end.
    pub mask: u8,
    /// The byte's bit the first of them lands on.
    pub to: u32,
}

impl Piece {
    /// The bits of the byte the piece holds.
    pub fn byte_mask(self) -> u8 {
        self.mask << self.to
    }
}

impl BitSpan {
    /// The span of `field`, a fixed field, whose byte order defaults to
    /// `default`; `None` for a field of no fixed size.
    pub fn of(field: &CodecField, default: Endian) -> Option<Self> {
        let BitSize::Fixed { bits } = field.bit_size else {
            return None;
        };
        let shift = field.bit_offset.unwrap_or(0);
        Some(Self {
            byte: field.byte_offset,
            bytes: (shift + bits).div_ceil(8),
            shift,
            bits,
            endian: field.effective_endian(default),
        })
    }

    /// Whether the value is its whole word: whole bytes from bit 0.
    pub fn whole(&self) -> bool {
        self.shift == 0 && self.bits == 8 * self.bytes
    }

    /// The byte after the word.
    pub fn end(&self) -> u32 {
        self.byte + self.bytes
    }

    /// The mask of the value's bits, at the least significant end.
    pub fn mask(&self) -> u64 {
        if self.bits >= 64 {
            u64::MAX
        } else {
            (1u64 << self.bits) - 1
        }
    }

    /// Whether frame byte `at` is one of the word's bytes.
    pub fn covers(&self, at: u32) -> bool {
        (self.byte..self.end()).contains(&at)
    }

    /// How significant frame byte `at` is in the word: 0 for its least
    /// significant byte. `None` for a byte outside the word.
    pub fn rank(&self, at: u32) -> Option<u32> {
        if !self.covers(at) {
            return None;
        }
        let i = at - self.byte;
        Some(match self.endian {
            Endian::Little => i,
            Endian::Big | Endian::Native => self.bytes - 1 - i,
        })
    }

    /// The piece of the value frame byte `at` holds; `None` where the byte
    /// holds none of its bits.
    pub fn piece(&self, at: u32) -> Option<Piece> {
        let low = self.rank(at)? * 8;
        let first = low.max(self.shift);
        let past = (low + 8).min(self.shift + self.bits);
        (first < past).then(|| Piece {
            from: first - self.shift,
            mask: ((1u32 << (past - first)) - 1) as u8,
            to: first - low,
        })
    }

    /// Each byte of the frame the value has bits in, with the mask of them.
    pub fn byte_masks(&self) -> impl Iterator<Item = (u32, u8)> + '_ {
        (self.byte..self.end()).filter_map(|at| self.piece(at).map(|p| (at, p.byte_mask())))
    }
}

/// One step of the encode of a positional frame's fixed bytes, in wire order.
#[derive(Clone, Debug)]
pub enum Slot<'m> {
    /// A whole field that shares none of its bytes, written as one value of its
    /// bytes in its byte order.
    Whole(&'m CodecField),
    /// Byte `at`: the or of the piece each field holds in it, in declaration
    /// order; zero where no field holds one.
    Byte {
        /// The byte of the frame.
        at: u32,
        /// Each field with bits in the byte, and its piece.
        pieces: Vec<(&'m CodecField, Piece)>,
    },
}

/// The fixed fields of `fields` with their spans, in declaration order.
pub fn spans(fields: &[CodecField], default: Endian) -> Vec<(&CodecField, BitSpan)> {
    fields
        .iter()
        .filter_map(|f| BitSpan::of(f, default).map(|s| (f, s)))
        .collect()
}

/// The end of the last fixed field: the fewest bytes a positional frame has.
pub fn fixed_end(fields: &[CodecField], default: Endian) -> u32 {
    spans(fields, default)
        .iter()
        .map(|(_, s)| s.end())
        .max()
        .unwrap_or(0)
}

/// The steps that write the fixed bytes `0 .. fixed_end` of a positional
/// frame (docs/adr/0095 §2).
pub fn slots(fields: &[CodecField], default: Endian) -> Vec<Slot<'_>> {
    let spans = spans(fields, default);
    let end = spans.iter().map(|(_, s)| s.end()).max().unwrap_or(0);
    let alone = |span: &BitSpan, field: &CodecField| {
        spans
            .iter()
            .filter(|(other, _)| !std::ptr::eq(*other, field))
            .all(|(_, other)| other.end() <= span.byte || span.end() <= other.byte)
    };
    let mut out = Vec::new();
    let mut at = 0;
    while at < end {
        if let Some((field, span)) = spans
            .iter()
            .find(|(f, s)| s.byte == at && s.whole() && alone(s, f))
        {
            out.push(Slot::Whole(field));
            at = span.end();
            continue;
        }
        let pieces = spans
            .iter()
            .filter_map(|(f, s)| s.piece(at).map(|p| (*f, p)))
            .collect();
        out.push(Slot::Byte { at, pieces });
        at += 1;
    }
    out
}

/// Why a field's bits are refused (docs/adr/0095 §3): the field, the attribute
/// the rule reads, its text, and the rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// The field's id.
    pub field: String,
    /// The attribute the refusal is reported on.
    pub attr: &'static str,
    /// That attribute's value.
    pub value: String,
    /// The rule the field breaks.
    pub rule: String,
}

/// The width of the integer a field of type `ty` holds, `None` for an enum,
/// whose carrier is resolved after the parse.
fn type_bits(ty: &SceType) -> Option<u32> {
    ty.int_bit_width()
}

/// The first field of `fields` whose bits break a rule of docs/adr/0095 §3,
/// where `positional` says whether the codec lays its fields out by byte
/// (rather than writing them in turn).
pub fn refusal(fields: &[CodecField], default: Endian, positional: bool) -> Option<Refusal> {
    let spans = spans(fields, default);
    for (field, span) in &spans {
        let refuse = |attr: &'static str, value: u32, rule: String| Refusal {
            field: field.id.clone(),
            attr,
            value: value.to_string(),
            rule,
        };
        if span.shift + span.bits > 64 {
            return Some(refuse(
                "sce:bit-size",
                span.bits,
                format!(
                    "{} bits from bit {} make a word of more than 64 bits, which no backend has \
                     an integer to read",
                    span.bits, span.shift
                ),
            ));
        }
        if let Some(width) = type_bits(&field.sce_type).filter(|w| span.bits > *w) {
            return Some(refuse(
                "sce:bit-size",
                span.bits,
                format!(
                    "a `{}` holds {width} bits, fewer than the field's {}",
                    field.sce_type.as_attr(),
                    span.bits
                ),
            ));
        }
        if span.whole() {
            continue;
        }
        let unsigned = field.sce_type.is_unsigned() || matches!(field.sce_type, SceType::Enum(_));
        if !unsigned {
            return Some(refuse(
                "sce:bit-size",
                span.bits,
                format!(
                    "a field of part of its bytes holds an unsigned number of its bits, so its type \
                     is an unsigned integer or an enum, not `{}`",
                    field.sce_type.as_attr()
                ),
            ));
        }
        if !positional {
            return Some(refuse(
                "sce:bit-size",
                span.bits,
                format!(
                    "a frame written field by field gives each field bytes of its own, so a field \
                     of {} bits from bit {} leaves bits no other field can use; give it whole \
                     bytes, or lay the codec out by byte",
                    span.bits, span.shift
                ),
            ));
        }
    }
    if positional {
        for (i, (field, span)) in spans.iter().enumerate() {
            for (earlier, before) in &spans[..i] {
                let shared = span.byte_masks().find(|(at, mask)| {
                    before
                        .byte_masks()
                        .any(|(other, bits)| other == *at && bits & mask != 0)
                });
                if let Some((at, _)) = shared {
                    return Some(Refusal {
                        field: field.id.clone(),
                        attr: "sce:byte",
                        value: field.byte_offset.to_string(),
                        rule: format!(
                            "its bits in byte {at} are bits field '{}' holds; no two fields of a \
                             frame share a bit",
                            earlier.id
                        ),
                    });
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(byte: u32, shift: u32, bits: u32, endian: Endian) -> BitSpan {
        BitSpan {
            byte,
            bytes: (shift + bits).div_ceil(8),
            shift,
            bits,
            endian,
        }
    }

    /// The field of the ADR: twelve bits four bits into byte 1, big-endian,
    /// are the high nibble of byte 2 and all of byte 1.
    #[test]
    fn twelve_bits_four_in_span_two_bytes_big_endian() {
        let s = span(1, 4, 12, Endian::Big);
        assert_eq!(s.bytes, 2);
        assert_eq!(
            s.piece(1),
            Some(Piece {
                from: 4,
                mask: 0xFF,
                to: 0
            })
        );
        assert_eq!(
            s.piece(2),
            Some(Piece {
                from: 0,
                mask: 0x0F,
                to: 4
            })
        );
        assert_eq!(s.piece(0), None);
        assert_eq!(s.piece(3), None);
    }

    /// The same field little-endian: byte 1 is the low byte of the word.
    #[test]
    fn little_endian_puts_the_low_bits_first() {
        let s = span(1, 4, 12, Endian::Little);
        assert_eq!(
            s.piece(1),
            Some(Piece {
                from: 0,
                mask: 0x0F,
                to: 4
            })
        );
        assert_eq!(
            s.piece(2),
            Some(Piece {
                from: 4,
                mask: 0xFF,
                to: 0
            })
        );
    }

    /// A field inside one byte keeps the rule every backend followed: the
    /// value masked and moved up by its offset.
    #[test]
    fn a_field_inside_one_byte_is_one_piece_at_its_offset() {
        let s = span(0, 5, 3, Endian::Big);
        assert_eq!(s.bytes, 1);
        assert_eq!(
            s.piece(0),
            Some(Piece {
                from: 0,
                mask: 0x07,
                to: 5
            })
        );
        assert!(!s.whole());
        assert!(span(0, 0, 16, Endian::Big).whole());
    }
}
