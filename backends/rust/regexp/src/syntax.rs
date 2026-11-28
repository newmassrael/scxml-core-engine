// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The reader of RFC 9485's grammar (section 3), which builds as it reads: one pass over the
//! text, no recursion, a frame for each group open at once.
//!
//! ```text
//! i-regexp = branch *( "|" branch )
//! branch = *piece
//! piece = atom [ quantifier ]
//! quantifier = ( "*" / "+" / "?" ) / range-quantifier
//! range-quantifier = "{" QuantExact [ "," [ QuantExact ] ] "}"
//! atom = NormalChar / charClass / ( "(" i-regexp ")" )
//! charClass = "." / SingleCharEsc / charClassEsc / charClassExpr
//! charClassExpr = "[" [ "^" ] ( "-" / CCE1 ) *CCE1 [ "-" ] "]"
//! CCE1 = ( CCchar [ "-" CCchar ] ) / charClassEsc
//! ```
//!
//! The semantics are XSD-2's (RFC 9485, section 4): `^` and `$` are characters like any other,
//! and a range or a range quantifier whose start is past its end is refused.

use icu_properties::props::GeneralCategoryGroup;

use crate::nfa::{Builder, Frag, Kind};
use crate::{Error, Reason, Shortage};

/// What one group holds while it is open: the alternatives read so far and the branch being
/// read.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    /// The group's first state: where its atom begins, for a quantifier to copy.
    first: u64,
    alternatives: Option<Frag>,
    branch: Frag,
}

impl Frame {
    /// A frame of storage that holds no group yet.
    pub const VACANT: Frame = Frame {
        first: 0,
        alternatives: None,
        branch: Frag::Empty,
    };

    const fn open(first: u64) -> Frame {
        Frame {
            first,
            alternatives: None,
            branch: Frag::Empty,
        }
    }

    /// A `|`: the branch read is one more alternative.
    fn alternative(&mut self, builder: &mut Builder<'_>) {
        let branch = core::mem::replace(&mut self.branch, Frag::Empty);
        self.alternatives = Some(match self.alternatives {
            Some(before) => builder.alternate(before, branch),
            None => branch,
        });
    }

    /// The group's language: its alternatives, the last branch among them.
    fn close(self, builder: &mut Builder<'_>) -> Frag {
        match self.alternatives {
            Some(before) => builder.alternate(before, self.branch),
            None => self.branch,
        }
    }
}

/// An I-Regexp read whole.
pub(crate) struct Read {
    pub(crate) whole: Frag,
    pub(crate) depth: usize,
}

struct Reader<'t> {
    text: &'t str,
    at: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<char> {
        self.text[self.at..].chars().next()
    }

    fn peek_second(&self) -> Option<char> {
        self.text[self.at..].chars().nth(1)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.at += c.len_utf8();
        Some(c)
    }

    /// The grammar stops here: at the end, or at a character it has no place for.
    fn stops(&self, reason: Reason) -> Error {
        let reason = if self.peek().is_none() {
            Reason::UnexpectedEnd
        } else {
            reason
        };
        invalid(self.at, reason)
    }

    fn expect(&mut self, want: char) -> Result<(), Error> {
        if self.peek() == Some(want) {
            self.bump();
            Ok(())
        } else {
            Err(self.stops(Reason::Unexpected))
        }
    }
}

fn invalid(offset: usize, reason: Reason) -> Error {
    Error::Invalid { offset, reason }
}

/// Reads `text` whole and builds its automaton's fragment, every group's frame in `frames`.
pub(crate) fn read(
    text: &str,
    builder: &mut Builder<'_>,
    frames: &mut [Frame],
) -> Result<Read, Error> {
    let mut reader = Reader { text, at: 0 };
    let mut group = Frame::open(0);
    let mut depth = 0;
    let mut deepest = 0;
    while let Some(c) = reader.peek() {
        let at = reader.at;
        let first = builder.len();
        let atom = match c {
            '|' => {
                reader.bump();
                group.alternative(builder);
                continue;
            }
            '(' => {
                reader.bump();
                let Some(frame) = frames.get_mut(depth) else {
                    return Err(Error::Storage(Shortage::Frames));
                };
                *frame = group;
                depth += 1;
                deepest = deepest.max(depth);
                group = Frame::open(first);
                continue;
            }
            ')' => {
                if depth == 0 {
                    return Err(invalid(at, Reason::Unexpected));
                }
                reader.bump();
                let inner = group;
                depth -= 1;
                group = frames[depth];
                let first = inner.first;
                let language = inner.close(builder);
                piece(&mut reader, builder, first, language)?
            }
            '*' | '+' | '?' | '{' => return Err(invalid(at, Reason::NothingToRepeat)),
            ']' | '}' => return Err(invalid(at, Reason::Unexpected)),
            '.' => {
                reader.bump();
                let dot = builder.step(Kind::Dot);
                piece(&mut reader, builder, first, dot)?
            }
            '[' => {
                let set = class(&mut reader, builder)?;
                piece(&mut reader, builder, first, set)?
            }
            '\\' => {
                let escaped = escape(&mut reader, builder)?;
                piece(&mut reader, builder, first, escaped)?
            }
            normal => {
                reader.bump();
                let one = builder.step(Kind::Char(normal as u32));
                piece(&mut reader, builder, first, one)?
            }
        };
        group.branch = builder.concat(group.branch, atom);
    }
    if depth > 0 {
        return Err(invalid(text.len(), Reason::UnexpectedEnd));
    }
    Ok(Read {
        whole: group.close(builder),
        depth: deepest,
    })
}

/// The atom read, with the quantifier that follows it if one does.
fn piece(
    reader: &mut Reader<'_>,
    builder: &mut Builder<'_>,
    first: u64,
    atom: Frag,
) -> Result<Frag, Error> {
    let at = reader.at;
    Ok(match reader.peek() {
        Some('*') => {
            reader.bump();
            builder.star(atom)
        }
        Some('+') => {
            reader.bump();
            builder.plus(atom)
        }
        Some('?') => {
            reader.bump();
            builder.optional(atom)
        }
        Some('{') => {
            reader.bump();
            let low = count(reader)?;
            let high = if reader.peek() == Some(',') {
                reader.bump();
                if reader.peek() == Some('}') {
                    None
                } else {
                    Some(count(reader)?)
                }
            } else {
                Some(low)
            };
            reader.expect('}')?;
            if high.is_some_and(|high| high.less_than(low)) {
                return Err(invalid(at, Reason::QuantifierOutOfOrder));
            }
            builder.repeat(first, atom, low.value, high.map(|high| high.value))
        }
        _ => atom,
    })
}

/// A QuantExact: its digits past any leading zero, compared exactly, and its value, which stays
/// at `u64::MAX` past it.
#[derive(Clone, Copy)]
struct Count<'t> {
    digits: &'t str,
    value: u64,
}

impl Count<'_> {
    fn less_than(self, other: Count<'_>) -> bool {
        (self.digits.len(), self.digits) < (other.digits.len(), other.digits)
    }
}

fn count<'t>(reader: &mut Reader<'t>) -> Result<Count<'t>, Error> {
    let from = reader.at;
    let mut value: u64 = 0;
    while let Some(digit) = reader.peek().and_then(|c| c.to_digit(10)) {
        reader.bump();
        value = value.saturating_mul(10).saturating_add(u64::from(digit));
    }
    if reader.at == from {
        return Err(reader.stops(Reason::Unexpected));
    }
    let text: &'t str = reader.text;
    Ok(Count {
        digits: text[from..reader.at].trim_start_matches('0'),
        value,
    })
}

/// The character a SingleCharEsc names, given the character after its backslash.
fn single(c: char) -> Option<u32> {
    match c {
        '(' | ')' | '*' | '+' | '-' | '.' | '?' | '[' | '\\' | ']' | '^' | '{' | '|' | '}' => {
            Some(c as u32)
        }
        'n' => Some(0x0A),
        'r' => Some(0x0D),
        't' => Some(0x09),
        _ => None,
    }
}

/// A SingleCharEsc, from the character after its backslash.
fn single_escape(reader: &mut Reader<'_>) -> Result<u32, Error> {
    let at = reader.at;
    match reader.bump() {
        None => Err(invalid(at, Reason::UnexpectedEnd)),
        Some(c) => single(c).ok_or_else(|| invalid(at, Reason::Escape)),
    }
}

/// A catEsc or complEsc, from its `p` or `P`: the General Category values it names.
fn category_escape(reader: &mut Reader<'_>) -> Result<GeneralCategoryGroup, Error> {
    let complement = reader.bump() == Some('P');
    reader.expect('{')?;
    let major_at = reader.at;
    let major = reader
        .bump()
        .ok_or_else(|| invalid(major_at, Reason::UnexpectedEnd))?;
    let mut group = named(major, None).ok_or_else(|| invalid(major_at, Reason::Category))?;
    if reader.peek() != Some('}') {
        let minor_at = reader.at;
        let minor = reader
            .bump()
            .ok_or_else(|| invalid(minor_at, Reason::UnexpectedEnd))?;
        group = named(major, Some(minor)).ok_or_else(|| invalid(minor_at, Reason::Category))?;
    }
    reader.expect('}')?;
    Ok(if complement {
        group.complement()
    } else {
        group
    })
}

/// The General Category value or group an IsCategory names: RFC 9485's closed list, the values
/// of the Unicode Character Database and their one-letter groups, `Cs` and `LC` left out.
fn named(major: char, minor: Option<char>) -> Option<GeneralCategoryGroup> {
    use GeneralCategoryGroup as G;
    Some(match (major, minor) {
        ('L', None) => G::Letter,
        ('L', Some('l')) => G::LowercaseLetter,
        ('L', Some('m')) => G::ModifierLetter,
        ('L', Some('o')) => G::OtherLetter,
        ('L', Some('t')) => G::TitlecaseLetter,
        ('L', Some('u')) => G::UppercaseLetter,
        ('M', None) => G::Mark,
        ('M', Some('c')) => G::SpacingMark,
        ('M', Some('e')) => G::EnclosingMark,
        ('M', Some('n')) => G::NonspacingMark,
        ('N', None) => G::Number,
        ('N', Some('d')) => G::DecimalNumber,
        ('N', Some('l')) => G::LetterNumber,
        ('N', Some('o')) => G::OtherNumber,
        ('P', None) => G::Punctuation,
        ('P', Some('c')) => G::ConnectorPunctuation,
        ('P', Some('d')) => G::DashPunctuation,
        ('P', Some('e')) => G::ClosePunctuation,
        ('P', Some('f')) => G::FinalPunctuation,
        ('P', Some('i')) => G::InitialPunctuation,
        ('P', Some('o')) => G::OtherPunctuation,
        ('P', Some('s')) => G::OpenPunctuation,
        ('Z', None) => G::Separator,
        ('Z', Some('l')) => G::LineSeparator,
        ('Z', Some('p')) => G::ParagraphSeparator,
        ('Z', Some('s')) => G::SpaceSeparator,
        ('S', None) => G::Symbol,
        ('S', Some('c')) => G::CurrencySymbol,
        ('S', Some('k')) => G::ModifierSymbol,
        ('S', Some('m')) => G::MathSymbol,
        ('S', Some('o')) => G::OtherSymbol,
        ('C', None) => G::Other,
        ('C', Some('c')) => G::Control,
        ('C', Some('f')) => G::Format,
        ('C', Some('n')) => G::Unassigned,
        ('C', Some('o')) => G::PrivateUse,
        _ => return None,
    })
}

/// An escape outside a class: a SingleCharEsc, or a catEsc or complEsc, a class of its own.
fn escape(reader: &mut Reader<'_>, builder: &mut Builder<'_>) -> Result<Frag, Error> {
    reader.bump();
    if matches!(reader.peek(), Some('p' | 'P')) {
        let categories = category_escape(reader)?;
        let from = builder.ranges_written();
        Ok(builder.class(from, categories, false))
    } else {
        let code_point = single_escape(reader)?;
        Ok(builder.step(Kind::Char(code_point)))
    }
}

/// A CCchar: any character but `[`, `]`, `-` and the backslash, or a SingleCharEsc.
fn class_char(reader: &mut Reader<'_>) -> Result<u32, Error> {
    let at = reader.at;
    match reader.bump() {
        None => Err(invalid(at, Reason::UnexpectedEnd)),
        Some('\\') => single_escape(reader),
        Some('[' | ']' | '-') => Err(invalid(at, Reason::Unexpected)),
        Some(c) => Ok(c as u32),
    }
}

const HYPHEN: u32 = '-' as u32;

/// A charClassExpr, from its `[` to its `]`: its ranges written in order, then sorted and
/// merged into one class.
fn class(reader: &mut Reader<'_>, builder: &mut Builder<'_>) -> Result<Frag, Error> {
    reader.bump();
    let negated = reader.peek() == Some('^');
    if negated {
        reader.bump();
    }
    let from = builder.ranges_written();
    let mut categories = GeneralCategoryGroup::empty();
    let mut items = 0;
    if reader.peek() == Some('-') {
        reader.bump();
        builder.range(HYPHEN, HYPHEN);
        items += 1;
    }
    loop {
        let at = reader.at;
        match reader.peek() {
            Some(']') if items > 0 => {
                reader.bump();
                break;
            }
            Some(']') => return Err(invalid(at, Reason::EmptyClass)),
            Some('-') => {
                // A hyphen that ends no range stands last.
                reader.bump();
                if reader.peek() != Some(']') {
                    return Err(reader.stops(Reason::Unexpected));
                }
                builder.range(HYPHEN, HYPHEN);
            }
            Some('\\') if matches!(reader.peek_second(), Some('p' | 'P')) => {
                reader.bump();
                categories = categories.union(category_escape(reader)?);
                items += 1;
            }
            _ => {
                let first = class_char(reader)?;
                let mut last = first;
                if reader.peek() == Some('-') && reader.peek_second().is_some_and(|c| c != ']') {
                    reader.bump();
                    last = class_char(reader)?;
                    if last < first {
                        return Err(invalid(at, Reason::RangeOutOfOrder));
                    }
                }
                builder.range(first, last);
                items += 1;
            }
        }
    }
    Ok(builder.class(from, categories, negated))
}
