// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A codec as the byte layout of one frame, in the manner of a packet
//! diagram: rows of eight bytes, each field a cell as wide as its bits.
//!
//! # What is placed where
//!
//! A field of fixed width before the first field of variable width is at
//! the offset the document states, so it is drawn there: a gap between two
//! such fields is drawn too, as a cell that says the document does not
//! state what is in it. A sub-byte field is drawn by the bits it occupies,
//! counted from the least significant bit of its byte as the document
//! counts them, so the diagram reads most significant bit first.
//!
//! A field of variable width — one whose length is read from another field,
//! one that takes the rest of the message, a variable-length integer, a
//! repeated body, a chain of tag-length-value entries — is drawn from where
//! it starts to the end of its row, with its right edge dashed: the document
//! does not say where it ends. Whatever follows it is NOT at an offset the
//! document states (the offset a later field carries is the one it would
//! have had were the variable field empty), so it is drawn on rows of its
//! own, headed by the field it follows.
//!
//! # What is not drawn
//!
//! A codec that writes a map (`cbor`) has no byte layout to draw. Fields
//! that overlap, a sub-byte field that crosses its byte, or a layout of more
//! than [`MAX_ROWS`] rows is not drawn rather than drawn wrong; the field
//! table still holds it, and the legend under the picture says in words
//! what the cells cannot: sizes, byte order, the bits of each flag, which
//! fields are present only under a condition.

use super::{caption, facts, say, Picture};
use crate::diagram::boxes::BoxError;
use crate::diagram::canvas::Canvas;
use crate::diagram::fit::{Page, Refusal};
use crate::diagram::metrics::Face;
use crate::diagram::sheet::{Ink, Outline};
use crate::diagram::words::{self, Phrase};
use crate::forge::model::{BitSize, CodecEncoding, CodecField, CodecModel, CountRef, Endian};
use crate::forge::page::Lexicon;

/// Bits in a row: eight bytes.
const ROW_BITS: u64 = 64;

/// The most rows a layout is drawn in; a longer one has no picture.
const MAX_ROWS: u64 = 24;

/// What a placed piece is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Label {
    Field(String),
    /// Bits the document does not account for, between two fields.
    Gap,
    /// What follows the fields when the codec dispatches on a variant.
    Arms,
}

/// How far a piece reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Extent {
    Fixed(u64),
    /// To an end the document does not state.
    Open,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Piece {
    label: Label,
    /// Bits from the start of its frame, most significant bit first.
    start: u64,
    extent: Extent,
}

impl Piece {
    fn end(&self) -> u64 {
        match self.extent {
            Extent::Fixed(bits) => self.start + bits,
            Extent::Open => self.start,
        }
    }
}

/// Pieces placed from one origin: the start of the message, or the end of
/// the variable field the frame follows (`after`, empty for the arms).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Frame {
    after: Option<String>,
    pieces: Vec<Piece>,
}

/// One thing the layout places, in declaration order.
enum Item<'a> {
    Field(&'a CodecField),
    Arms,
}

/// A fixed field's start, in bits from the start of its frame, most
/// significant bit first; `None` for a sub-byte field that does not fit its
/// byte.
fn fixed_start(f: &CodecField, bits: u64) -> Option<u64> {
    let byte = u64::from(f.byte_offset) * 8;
    match f.bit_offset {
        None => Some(byte),
        Some(offset) => {
            let top = u64::from(offset) + bits;
            (top <= 8).then(|| byte + 8 - top)
        }
    }
}

/// The frames of `m`, or `None` when it has no layout to draw.
fn frames(m: &CodecModel) -> Option<Vec<Frame>> {
    if m.encoding != CodecEncoding::Positional || m.fields.is_empty() {
        return None;
    }
    let items: Vec<Item> = m
        .fields
        .iter()
        .map(Item::Field)
        .chain(m.variant.iter().map(|_| Item::Arms))
        .collect();

    // The fields before the first of variable width sit at their stated
    // offsets, in whatever order the document wrote them.
    let mut known: Vec<Piece> = Vec::new();
    let mut follows: Option<String> = None;
    let mut rest = items.iter();
    for item in rest.by_ref() {
        match item {
            Item::Field(f) => match f.bit_size {
                BitSize::Fixed { bits } => {
                    let bits = u64::from(bits);
                    known.push(Piece {
                        label: Label::Field(f.id.clone()),
                        start: fixed_start(f, bits)?,
                        extent: Extent::Fixed(bits),
                    });
                }
                _ => {
                    known.push(Piece {
                        label: Label::Field(f.id.clone()),
                        start: u64::from(f.byte_offset) * 8,
                        extent: Extent::Open,
                    });
                    follows = Some(f.id.clone());
                    break;
                }
            },
            Item::Arms => {
                let start = known.iter().map(Piece::end).max().unwrap_or(0);
                known.push(Piece {
                    label: Label::Arms,
                    start,
                    extent: Extent::Open,
                });
                follows = Some(String::new());
                break;
            }
        }
    }
    known.sort_by_key(|p| p.start);
    let mut placed: Vec<Piece> = Vec::new();
    let mut cursor = 0u64;
    for piece in known {
        // Nothing may overlap what came before, or follow a piece with no end.
        if piece.start < cursor || placed.last().is_some_and(|p| p.extent == Extent::Open) {
            return None;
        }
        if piece.start > cursor {
            placed.push(Piece {
                label: Label::Gap,
                start: cursor,
                extent: Extent::Fixed(piece.start - cursor),
            });
        }
        cursor = piece.end();
        placed.push(piece);
    }
    let mut frames = vec![Frame {
        after: None,
        pieces: placed,
    }];

    // Everything after a variable field is placed one after another, on
    // frames of its own: each variable field ends a frame.
    let mut current = follows.map(|after| Frame {
        after: Some(after),
        pieces: Vec::new(),
    });
    let mut cursor = 0u64;
    for item in rest {
        let frame = current.as_mut()?;
        let (label, extent) = match item {
            Item::Field(f) => match f.bit_size {
                BitSize::Fixed { bits } => {
                    if f.bit_offset.is_some() {
                        return None;
                    }
                    (Label::Field(f.id.clone()), Extent::Fixed(u64::from(bits)))
                }
                _ => (Label::Field(f.id.clone()), Extent::Open),
            },
            Item::Arms => (Label::Arms, Extent::Open),
        };
        let piece = Piece {
            label,
            start: cursor,
            extent,
        };
        cursor = piece.end();
        let id = match &piece.label {
            Label::Field(id) => id.clone(),
            Label::Gap | Label::Arms => String::new(),
        };
        let open = piece.extent == Extent::Open;
        frame.pieces.push(piece);
        if open {
            frames.extend(current.take());
            current = Some(Frame {
                after: Some(id),
                pieces: Vec::new(),
            });
            cursor = 0;
        }
    }
    frames.extend(current);
    frames.retain(|f| !f.pieces.is_empty());
    Some(frames)
}

/// One row's share of a piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Segment {
    row: u64,
    from: u64,
    to: u64,
    starts: bool,
    ends: bool,
}

fn segments(p: &Piece) -> Vec<Segment> {
    match p.extent {
        Extent::Open => vec![Segment {
            row: p.start / ROW_BITS,
            from: p.start % ROW_BITS,
            to: ROW_BITS,
            starts: true,
            ends: false,
        }],
        Extent::Fixed(0) => Vec::new(),
        Extent::Fixed(bits) => {
            let (first, last) = (p.start / ROW_BITS, (p.start + bits - 1) / ROW_BITS);
            (first..=last)
                .map(|row| Segment {
                    row,
                    from: p.start.max(row * ROW_BITS) - row * ROW_BITS,
                    to: (p.start + bits).min((row + 1) * ROW_BITS) - row * ROW_BITS,
                    starts: row == first,
                    ends: row == last,
                })
                .collect()
        }
    }
}

/// The rows a frame takes.
fn rows_of(frame: &Frame) -> u64 {
    frame
        .pieces
        .iter()
        .flat_map(segments)
        .map(|s| s.row + 1)
        .max()
        .unwrap_or(0)
}

/// `bit 7`, or `bits 7..5`, in the page's words: the bits a field takes,
/// counted from the least significant bit of its byte.
fn bit_range(lexicon: &Lexicon, bit: u32, width: u32) -> Result<String, Refusal> {
    Ok(if width <= 1 {
        format!("{} {bit}", say(lexicon, Phrase::Bit)?)
    } else {
        format!("{} {}..{bit}", say(lexicon, Phrase::Bits)?, bit + width - 1)
    })
}

/// The legend: each field's size and byte order in words, each flag's bits,
/// each condition a field is present under.
fn legend(m: &CodecModel, lexicon: &Lexicon) -> Result<Vec<(String, String)>, Refusal> {
    let mut lines = Vec::new();
    for f in &m.fields {
        let mut parts: Vec<String> = Vec::new();
        match &f.bit_size {
            BitSize::Fixed { bits } => {
                parts.push(match f.bit_offset {
                    Some(offset) => bit_range(lexicon, offset, *bits)?,
                    None => format!("{bits} {}", say(lexicon, Phrase::Bits)?),
                });
                if *bits > 8 {
                    let order = match f.endian.unwrap_or(m.default_endian) {
                        Endian::Big => Phrase::BigEndian,
                        Endian::Little => Phrase::LittleEndian,
                        Endian::Native => Phrase::NativeEndian,
                    };
                    parts.push(say(lexicon, order)?.to_string());
                }
            }
            BitSize::LengthRef => {
                if let Some(field) = &f.length_field {
                    parts.push(format!("{} {field}", say(lexicon, Phrase::LengthIn)?));
                }
                if let Some(max) = f.max_size {
                    parts.push(format!(
                        "{} {max} {}",
                        say(lexicon, Phrase::UpTo)?,
                        say(lexicon, Phrase::Bytes)?
                    ));
                }
            }
            BitSize::Tail => parts.push(say(lexicon, Phrase::RestOfMessage)?.to_string()),
            BitSize::Vle { width_bits } => parts.push(format!("vle, width_bits {width_bits}")),
            BitSize::Repeat { count_ref } => {
                parts.push(say(lexicon, Phrase::Repeated)?.to_string());
                parts.push(match count_ref {
                    CountRef::LengthField(field) => {
                        format!("{} {field}", say(lexicon, Phrase::CountIn)?)
                    }
                    CountRef::UntilEof => say(lexicon, Phrase::UntilEnd)?.to_string(),
                });
            }
            BitSize::TlvChain { max_depth, .. } => {
                parts.push(format!("tlv_chain, max_depth {max_depth}"));
            }
            // Another document's layout, read inline: the document's own
            // spelling and the codec it names, since where it ends is that
            // codec's to say.
            BitSize::Embed => {
                parts.push(f.bit_size.as_attr());
                parts.extend(f.embed_body_alias.clone());
            }
        }
        if let Some(p) = &f.present_if {
            parts.push(format!(
                "{} {}.{}",
                say(lexicon, Phrase::PresentIf)?,
                p.field_id,
                p.flag_name
            ));
        }
        lines.push((f.id.clone(), parts.join(", ")));
        for flag in &f.flags {
            lines.push((
                format!("{}.{}", f.id, flag.name),
                bit_range(lexicon, flag.bit, flag.width)?,
            ));
        }
    }
    if let Some(variant) = &m.variant {
        let tag = variant
            .tag_field
            .as_deref()
            .or(variant.tag_flag.as_deref())
            .unwrap_or_default();
        lines.push((
            tag.to_string(),
            format!("{} {}", variant.arms.len(), say(lexicon, Phrase::Arms)?),
        ));
    }
    Ok(lines)
}

/// What a piece's label says.
fn text_of(label: &Label, lexicon: &Lexicon) -> Result<String, Refusal> {
    Ok(match label {
        Label::Field(id) => id.clone(),
        Label::Gap => say(lexicon, Phrase::Unstated)?.to_string(),
        Label::Arms => say(lexicon, Phrase::Arms)?.to_string(),
    })
}

/// A label that did not fit its cell: where it starts, its width, its text
/// and the x of the cell it names.
struct Pending {
    x: f64,
    width: f64,
    text: String,
    cell_x: f64,
}

/// The picture of `m`: its byte layout.
pub fn codec(m: &CodecModel, lexicon: &Lexicon, page: Page) -> Result<Vec<Picture>, Refusal> {
    let Some(frames) = frames(m) else {
        return Ok(Vec::new());
    };
    if frames.iter().map(rows_of).sum::<u64>() > MAX_ROWS {
        return Ok(Vec::new());
    }

    let mut c = Canvas::new(page);
    let body = c.style().body_pt;
    let line = c.line_height();
    let gap = body * 0.6;
    let unit = body * 0.8;
    let height = line + 1.2 * body;
    let title = format!("{}: {}", m.name, say(lexicon, Phrase::Layout)?);
    c.title(0.0, 0.0, &title)?;

    // Row labels are the byte a row starts at, in a gutter left of the cells.
    let widest_row = frames.iter().map(rows_of).max().unwrap_or(0);
    let gutter = c.width_of(Face::Mono, &(widest_row * ROW_BITS / 8).to_string(), body)?;
    let x0 = gutter + 2.0 * gap;

    let mut y = c.style().title_pt * c.style().leading + line;
    // The byte ruler: which byte of its row each eighth is.
    for k in 0..ROW_BITS / 8 {
        let x = x0 + (k * 8) as f64 * unit;
        c.text(x + gap / 2.0, y, &k.to_string(), Face::Mono)?;
    }
    y += line + gap / 2.0;

    for frame in &frames {
        if let Some(after) = &frame.after {
            let text = words::after_field(lexicon, after)
                .ok_or(Refusal::Box(BoxError::NoPhrases(lexicon.name)))?;
            c.text(0.0, y, &text, Face::Proportional)?;
            y += line + gap / 2.0;
        }
        for row in 0..rows_of(frame) {
            c.text_right(
                x0 - gap,
                y + (height - line) / 2.0,
                &(row * ROW_BITS / 8).to_string(),
                Face::Mono,
            )?;
            // The row's whole width as a faint track, a tick at each byte,
            // so the part a field does not take is seen as the room it is.
            c.rect(
                (x0, y, ROW_BITS as f64 * unit, height),
                None,
                Some(Outline {
                    ink: Ink::Hairline,
                    width_pt: body * 0.05,
                }),
            );
            for k in 1..ROW_BITS / 8 {
                let x = x0 + (k * 8) as f64 * unit;
                c.stroke((x, y), (x, y + body * 0.4), Ink::Hairline, false);
                c.stroke(
                    (x, y + height - body * 0.4),
                    (x, y + height),
                    Ink::Hairline,
                    false,
                );
            }
            let mut pending: Vec<Pending> = Vec::new();
            for piece in &frame.pieces {
                let text = text_of(&piece.label, lexicon)?;
                for (n, s) in segments(piece).into_iter().enumerate() {
                    if s.row != row {
                        continue;
                    }
                    let (xa, xb) = (x0 + s.from as f64 * unit, x0 + s.to as f64 * unit);
                    if matches!(piece.label, Label::Gap | Label::Arms) {
                        c.rect((xa, y, xb - xa, height), Some(Ink::Shade), None);
                    }
                    c.stroke((xa, y), (xb, y), Ink::Black, false);
                    c.stroke((xa, y + height), (xb, y + height), Ink::Black, false);
                    if s.starts {
                        c.stroke((xa, y), (xa, y + height), Ink::Black, false);
                    }
                    if s.ends {
                        c.stroke((xb, y), (xb, y + height), Ink::Black, false);
                    } else if piece.extent == Extent::Open {
                        c.stroke((xb, y), (xb, y + height), Ink::Muted, true);
                    }
                    if n == 0 {
                        let width = c.width_of(Face::Mono, &text, body)?;
                        let mid = (xa + xb) / 2.0;
                        if width + 2.0 * gap <= xb - xa {
                            c.text_centered(mid, y + (height - line) / 2.0, &text, Face::Mono)?;
                        } else {
                            pending.push(Pending {
                                x: mid - width / 2.0,
                                width,
                                text: text.clone(),
                                cell_x: mid,
                            });
                        }
                    }
                }
            }
            // A label that does not fit its cell goes on a lane under the
            // row, tied to the cell by a stroke.
            let mut lane_ends: Vec<f64> = Vec::new();
            let mut lanes: Vec<(usize, Pending)> = Vec::new();
            for p in pending {
                let lane = match lane_ends.iter().position(|end| p.x >= *end + gap) {
                    Some(lane) => lane,
                    None => {
                        lane_ends.push(f64::NEG_INFINITY);
                        lane_ends.len() - 1
                    }
                };
                lane_ends[lane] = p.x + p.width;
                lanes.push((lane, p));
            }
            for (lane, p) in &lanes {
                let top = y + height + gap / 2.0 + *lane as f64 * line;
                c.stroke((p.cell_x, y + height), (p.cell_x, top), Ink::Muted, false);
                c.text(p.x, top, &p.text, Face::Mono)?;
            }
            let below = if lane_ends.is_empty() {
                0.0
            } else {
                gap / 2.0 + lane_ends.len() as f64 * line
            };
            y += height + below + gap / 2.0;
        }
        y += gap;
    }

    // The model's own scalars, then the legend of fields.
    let mut top = y + gap;
    top += caption(&mut c, 0.0, top, &facts(m, &["name"])?)?;
    caption(&mut c, 0.0, top + gap, &legend(m, lexicon)?)?;
    Ok(vec![Picture {
        stem: "layout",
        sheet: c.finish(page, &title)?,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::sheet::{Mark, Sheet};
    use crate::forge::model::{FlagDef, ForgeDocument};
    use crate::forge::page::{EN, KO};
    use crate::forge::parser::parse_forge_with_imports;
    use crate::DocumentLabel;

    fn parse(text: &str, label: &str) -> Option<CodecModel> {
        match parse_forge_with_imports(text, DocumentLabel::for_input_path(label)) {
            Ok(Some(parsed)) => match parsed.document {
                ForgeDocument::Codec(m) => Some(m),
                _ => None,
            },
            _ => None,
        }
    }

    fn example() -> CodecModel {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("kind-examples")
            .join("codec.scxml");
        let text = std::fs::read_to_string(&path).expect("reads");
        parse(&text, path.to_str().unwrap()).expect("a codec")
    }

    fn page() -> Page {
        Page::a4_portrait(7.0)
    }

    fn sheet(m: &CodecModel) -> Sheet {
        codec(m, &EN, page()).expect("draws").remove(0).sheet
    }

    /// The length-prefixed frame: two single-byte fields side by side,
    /// then the payload open to the end of the row, with each field's size
    /// in the legend.
    #[test]
    fn the_example_is_two_bytes_then_an_open_payload() {
        let m = example();
        let f = frames(&m).expect("a layout");
        assert_eq!(f.len(), 1);
        let pieces: Vec<(&Label, u64, Extent)> = f[0]
            .pieces
            .iter()
            .map(|p| (&p.label, p.start, p.extent))
            .collect();
        assert_eq!(
            pieces,
            vec![
                (&Label::Field("msgId".into()), 0, Extent::Fixed(8)),
                (&Label::Field("len".into()), 8, Extent::Fixed(8)),
                (&Label::Field("payload".into()), 16, Extent::Open),
            ]
        );
        let words = sheet(&m)
            .words()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        for expected in [
            "codec: layout",
            "msgId",
            "len",
            "payload",
            "8 bits",
            "length in len, up to 32 bytes",
            "default_endian",
            "big",
        ] {
            assert!(words.iter().any(|w| w == expected), "{expected}: {words:?}");
        }
        let korean = codec(&m, &KO, page()).expect("draws").remove(0).sheet;
        assert_ne!(korean.words(), sheet(&m).words(), "in the page's language");
    }

    fn field(id: &str, byte: u32, bit_offset: Option<u32>, bits: u32) -> CodecField {
        let mut f = example().fields[0].clone();
        f.id = id.into();
        f.byte_offset = byte;
        f.bit_offset = bit_offset;
        f.bit_size = BitSize::Fixed { bits };
        f.length_field = None;
        f.max_size = None;
        f
    }

    fn with(fields: Vec<CodecField>) -> CodecModel {
        let mut m = example();
        m.fields = fields;
        m
    }

    /// Sub-byte fields are drawn by the bits they occupy, counted from the
    /// least significant bit as the document counts them: the top three
    /// bits are on the left, whatever order the document wrote them in.
    #[test]
    fn sub_byte_fields_read_most_significant_bit_first() {
        let m = with(vec![
            field("direction", 0, Some(0), 2),
            field("priority", 0, Some(5), 3),
            field("channel", 0, Some(2), 3),
        ]);
        let f = frames(&m).expect("a layout");
        let starts: Vec<(String, u64)> = f[0]
            .pieces
            .iter()
            .map(|p| match &p.label {
                Label::Field(id) => (id.clone(), p.start),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            starts,
            vec![
                ("priority".into(), 0),
                ("channel".into(), 3),
                ("direction".into(), 6)
            ]
        );
        let words = sheet(&m)
            .words()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        for expected in ["bits 7..5", "bits 4..2", "bits 1..0"] {
            assert!(words.iter().any(|w| w == expected), "{expected}: {words:?}");
        }
    }

    /// A gap between two fields is a cell that says what is in it is not
    /// stated; fields that overlap, or a sub-byte field crossing its byte,
    /// have no layout.
    #[test]
    fn gaps_are_drawn_and_overlaps_are_not_laid_out() {
        let m = with(vec![field("a", 0, None, 8), field("b", 4, None, 8)]);
        let f = frames(&m).expect("a layout");
        assert_eq!(f[0].pieces[1].label, Label::Gap);
        assert_eq!(f[0].pieces[1].extent, Extent::Fixed(24));
        let words = sheet(&m)
            .words()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert!(words.iter().any(|w| w == "not stated"), "{words:?}");

        assert!(frames(&with(vec![field("a", 0, None, 16), field("b", 1, None, 8)])).is_none());
        assert!(frames(&with(vec![field("a", 0, Some(6), 4)])).is_none());
    }

    /// What follows a variable field is not at the offset it carries: it is
    /// on rows of its own under the field it follows.
    #[test]
    fn what_follows_a_variable_field_is_headed_by_it() {
        let mut m = example();
        let mut crc = field("crc32", 4, None, 32);
        crc.endian = None;
        m.fields.push(crc);
        let f = frames(&m).expect("a layout");
        assert_eq!(f.len(), 2);
        assert_eq!(f[1].after.as_deref(), Some("payload"));
        assert_eq!(f[1].pieces[0].start, 0, "from the end of the payload");
        let words = sheet(&m)
            .words()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert!(words.iter().any(|w| w == "after payload"), "{words:?}");
        assert!(
            words.iter().any(|w| w == "32 bits, big-endian"),
            "{words:?}"
        );
    }

    /// A label too wide for its cell goes below the row on a lane tied to
    /// the cell by a stroke, and still reads whole.
    #[test]
    fn a_label_that_does_not_fit_its_cell_is_tied_to_it_below() {
        let mut m = with(vec![
            field("reliable", 0, Some(7), 1),
            field("more", 0, Some(6), 1),
            field("drop", 0, Some(5), 1),
        ]);
        m.fields[0].flags = vec![FlagDef {
            name: "first".into(),
            bit: 7,
            width: 1,
            value: None,
            value_text: String::new(),
            line: None,
        }];
        let s = sheet(&m);
        let words = s.words();
        for expected in ["reliable", "more", "drop", "reliable.first", "bit 7"] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        let leaders = s
            .marks
            .iter()
            .filter(|m| {
                matches!(
                    m,
                    Mark::Stroke {
                        ink: Ink::Muted,
                        dashed: false,
                        ..
                    }
                )
            })
            .count();
        assert!(
            leaders >= 3,
            "a leader for each label below its cell: {leaders}"
        );
    }

    /// A map encoding, an empty codec and a layout too long to draw have no
    /// picture.
    #[test]
    fn a_codec_with_no_layout_to_draw_has_no_picture() {
        let mut m = example();
        m.encoding = CodecEncoding::Cbor;
        assert!(codec(&m, &EN, page()).expect("draws").is_empty());
        m.encoding = CodecEncoding::Positional;
        m.fields.clear();
        assert!(codec(&m, &EN, page()).expect("draws").is_empty());
        let long = with(vec![field("blob", 0, None, 8 * 8 * 30)]);
        assert!(codec(&long, &EN, page()).expect("draws").is_empty());
    }

    /// Every codec fixture of the tree is either drawn or left to its
    /// table; none is refused or panics, and every fixed field a drawn
    /// layout holds is named on the sheet.
    #[test]
    fn every_codec_fixture_is_drawn_or_left_to_its_table() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests/forge/resources");
        let (mut parsed, mut drawn) = (0, 0);
        for entry in std::fs::read_dir(&dir).expect("fixtures") {
            let path = entry.expect("entry").path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if !name.starts_with("codec_") || !name.ends_with(".scxml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("reads");
            let Some(m) = parse(&text, path.to_str().unwrap()) else {
                continue;
            };
            parsed += 1;
            let pictures = codec(&m, &EN, page()).unwrap_or_else(|e| panic!("{name}: {e}"));
            let Some(p) = pictures.first() else { continue };
            drawn += 1;
            let words = p.sheet.words();
            for f in &m.fields {
                assert!(
                    words.contains(&f.id.as_str()),
                    "{name}: {} in {words:?}",
                    f.id
                );
            }
        }
        assert!(parsed >= 60, "the codec fixtures parse: {parsed}");
        assert!(drawn >= parsed / 2, "most are drawn: {drawn} of {parsed}");
    }
}
