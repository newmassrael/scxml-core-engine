// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! One type grammar for every position that declares a value (docs/adr/0086,
//! section 1): an event schema's field, a `sce-static` variable, an algorithm's
//! parameter, return and local.
//!
//! ```text
//! T ::= uint8 | uint16 | uint32 | uint64 | int8 | int16 | int32 | int64 | float32 | float64 | bool
//!     | enum:<alias>
//!     | string[B] | bytes[B]
//!     | optional<T>
//!     | record:<alias>
//!     | list<T>[B]
//! B ::= <positive integer> | <limit name>=<positive integer>
//! ```
//!
//! Every constructor takes every type, and a bound is written in the type it
//! bounds, at every depth: `list<list<string[8]>[4]>[3]` is at most 3 lists of
//! at most 4 texts of at most 8 bytes. Two shapes stand on what stays refused:
//! `optional<optional<T>>`, whose two absences no wire tells apart, and a type
//! that contains itself, which has no largest value ([`contains_itself`]).
//!
//! The type is read in two steps. [`WrittenType::read`] reads the text as the
//! author wrote it, where a bound may be left out; [`WrittenType::bounded`]
//! then requires every bound, which yields a [`ValueType`]. Between them a
//! position folds in the bound a legacy attribute states for the outermost type
//! ([`WrittenType::with_outer_bound`]): `sce:max-size`, `sce:max-count`,
//! `sce:capacity`, an algorithm's `capacity=` and `returns-max-size=` are
//! synonyms of the bracket on the outermost type they bound, read with
//! `validation/bound-attribute-superseded` ([`crate::forge::bound_spelling`]).
//!
//! A [`ValueType`] has a largest encoding the build computes by structural
//! recursion ([`json_max_len`], [`cbor_max_len`]), docs/adr/0086 section 2.

use std::fmt;

use crate::forge::model::{AlgorithmValueType, FieldType, ListElemType, OptionalElemType, SceType};

/// A bound written in a type: the most elements of a list, or the most bytes of
/// a text or a byte string. `limit` is the name a build may give another number
/// (`[name=N]`, docs/adr/0050); `max` is the number the build holds, the one
/// written until a build resolves the name ([`WrittenType::resolve_limits`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    pub max: u32,
    pub limit: Option<String>,
}

impl Bound {
    /// A bound no name carries.
    pub fn of(max: u32) -> Self {
        Self { max, limit: None }
    }

    /// A bound a build may move by `limit`.
    pub fn named(limit: &str, max: u32) -> Self {
        Self {
            max,
            limit: Some(limit.to_string()),
        }
    }
}

impl fmt::Display for Bound {
    /// The bracket as the grammar spells it: `[8]` or `[name=8]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.limit {
            Some(name) => write!(f, "[{name}={}]", self.max),
            None => write!(f, "[{}]", self.max),
        }
    }
}

/// A type of the grammar, generic over what stands where a bound is written:
/// [`Bound`] in a [`ValueType`], whose every bound is stated, and
/// `Option<Bound>` in a [`WrittenType`], a type as written before a legacy
/// attribute has supplied its outermost bound.
///
/// One constructor per line of the grammar. `Scalar` holds a fixed-width
/// number, `bool` or a value of an enum, never `string` or `bytes`: a text and a
/// byte string are `Text` and `Bytes`, whose bound is part of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeOf<B> {
    /// A fixed-width number, `bool`, or `enum:<alias>`.
    Scalar(SceType),
    /// `string[B]`: a UTF-8 text of at most `max` bytes.
    Text { max: B },
    /// `bytes[B]`: a byte string of at most `max` bytes.
    Bytes { max: B },
    /// `optional<T>`: absent, or a value of `elem`. `elem` is never itself
    /// optional.
    Optional { elem: Box<TypeOf<B>> },
    /// `record:<alias>`: a value of the struct the aliased event schema declares.
    Record { alias: String },
    /// `list<T>[B]`: an ordered sequence of at most `max` values of `elem`.
    List { elem: Box<TypeOf<B>>, max: B },
}

/// A type whose every bound is stated: what a position holds.
pub type ValueType = TypeOf<Bound>;

/// A type as its author wrote it: a bound may be left to an attribute.
pub type WrittenType = TypeOf<Option<Bound>>;

/// What stands where a bound is written: a stated bound, or one that may be
/// absent. The one reading the printer needs of either.
pub trait BoundSlot {
    /// The bound written there, if one is.
    fn stated(&self) -> Option<&Bound>;
}

impl BoundSlot for Bound {
    fn stated(&self) -> Option<&Bound> {
        Some(self)
    }
}

impl BoundSlot for Option<Bound> {
    fn stated(&self) -> Option<&Bound> {
        self.as_ref()
    }
}

/// The bracket after a type that takes a bound, or nothing where none is written.
fn bracket<B: BoundSlot>(f: &mut fmt::Formatter<'_>, max: &B) -> fmt::Result {
    match max.stated() {
        Some(bound) => write!(f, "{bound}"),
        None => Ok(()),
    }
}

impl<B: BoundSlot> fmt::Display for TypeOf<B> {
    /// The type as the grammar spells it, every bound in its bracket:
    /// `list<string[8]>[4]`, `optional<int64>`, `string[merge_text=96]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(t) => write!(f, "{}", t.as_attr()),
            Self::Text { max } => {
                write!(f, "string")?;
                bracket(f, max)
            }
            Self::Bytes { max } => {
                write!(f, "bytes")?;
                bracket(f, max)
            }
            Self::Optional { elem } => write!(f, "optional<{elem}>"),
            Self::Record { alias } => write!(f, "record:{alias}"),
            Self::List { elem, max } => {
                write!(f, "list<{elem}>")?;
                bracket(f, max)
            }
        }
    }
}

impl<B> TypeOf<B> {
    /// The name of the constructor at the top of the type, as a refusal names
    /// it: `list`, `optional`, `record`, `string`, `bytes`, or the scalar's own
    /// spelling.
    pub fn constructor(&self) -> String {
        match self {
            Self::Scalar(t) => t.as_attr(),
            Self::Text { .. } => "string".to_string(),
            Self::Bytes { .. } => "bytes".to_string(),
            Self::Optional { .. } => "optional".to_string(),
            Self::Record { .. } => "record".to_string(),
            Self::List { .. } => "list".to_string(),
        }
    }

    /// The aliases of every enum the type names, in the order they are
    /// written, at every depth.
    pub fn enum_aliases(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.walk(&mut |ty| {
            if let Self::Scalar(SceType::Enum(reference)) = ty {
                out.push(reference.alias.as_str());
            }
        });
        out
    }

    /// The aliases of every record the type names, in the order they are
    /// written, at every depth.
    pub fn record_aliases(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.walk(&mut |ty| {
            if let Self::Record { alias } = ty {
                out.push(alias.as_str());
            }
        });
        out
    }

    /// Every type the type is made of, itself first, outermost to innermost.
    fn walk<'a>(&'a self, visit: &mut dyn FnMut(&'a Self)) {
        visit(self);
        match self {
            Self::Optional { elem } | Self::List { elem, .. } => elem.walk(visit),
            Self::Scalar(_) | Self::Text { .. } | Self::Bytes { .. } | Self::Record { .. } => {}
        }
    }

    /// How deep the type nests: 1 for a type of no element, one more for each
    /// `optional<…>` and `list<…>` around it.
    pub fn depth(&self) -> usize {
        match self {
            Self::Optional { elem } | Self::List { elem, .. } => 1 + elem.depth(),
            Self::Scalar(_) | Self::Text { .. } | Self::Bytes { .. } | Self::Record { .. } => 1,
        }
    }
}

/// Why a type's text is refused: what it breaks, and where in the type.
///
/// `place` is the part of the type the refusal is about, spelled as the
/// grammar spells it, so a refusal of `list<list<string>[4]>[3]` names
/// `string` and not the whole type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRefusal {
    /// A word that names no type: a misspelled keyword, or an empty one.
    Unknown { written: String },
    /// The text is not a type of the grammar past `place`: a missing `<`,
    /// `>` or `]`, or text after the type.
    Malformed {
        place: String,
        expected: &'static str,
    },
    /// A bracket that is not a positive 32-bit integer or `name=N`.
    BadBound { place: String, written: String },
    /// A bracket on a type that has no bound: a number, `bool`, an enum, a
    /// record or an optional.
    BoundOnUnboundable { place: String },
    /// `optional<optional<T>>`.
    NestedOptional { place: String },
    /// A text, byte string or list with no bound.
    Unbounded { place: String },
    /// The outermost type carries a bracket and an attribute bounds it too.
    BothForms { place: String },
    /// An attribute states a bound beside a type that has none to state.
    NothingToBound { place: String },
}

impl TypeRefusal {
    /// The rule the type breaks, as a refusal's sentence states it.
    pub fn rule(&self) -> String {
        match self {
            Self::Unknown { written } => format!(
                "`{written}` names no type: a type is a number (uint8 to uint64, int8 to \
                 int64, float32, float64), bool, enum:<alias>, string[N], bytes[N], \
                 optional<T>, record:<alias> or list<T>[N]"
            ),
            Self::Malformed { place, expected } => {
                format!("the type is not complete after `{place}`: {expected}")
            }
            Self::BadBound { place, written } => format!(
                "the bound `[{written}]` of `{place}` is a whole number of at least one that \
                 fits 32 bits, or `name=N` with such a number"
            ),
            Self::BoundOnUnboundable { place } => format!(
                "`{place}` takes no bound: a bound is written after a string, a bytes or a \
                 list, the types whose values have a length"
            ),
            Self::NestedOptional { place } => format!(
                "`{place}` is an optional value inside an optional value, and the wire spells \
                 both absences as one null"
            ),
            Self::Unbounded { place } => format!(
                "`{place}` has no bound: every string, bytes and list states the most it \
                 holds in its type (`string[N]`, `bytes[N]`, `list<T>[N]`), so the type has \
                 a largest value the build can size"
            ),
            Self::BothForms { place } => format!(
                "the bound of `{place}` is written in the type and by an attribute: one bound, \
                 one spelling, and the type's is the one that stays"
            ),
            Self::NothingToBound { place } => format!(
                "`{place}` has no length to bound: an attribute bound stands beside a \
                 string, a bytes or a list"
            ),
        }
    }
}

/// A reader over the text of one type.
struct Cursor<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn skip_ws(&mut self) {
        let rest = &self.text[self.pos..];
        self.pos += rest.len() - rest.trim_start().len();
    }

    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    /// Consume `c` after any whitespace, or say it is not there.
    fn eat(&mut self, c: char) -> bool {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }

    /// The run of characters `keep` takes, from here.
    fn take_while(&mut self, keep: impl Fn(char) -> bool) -> &'a str {
        let rest = &self.text[self.pos..];
        let len = rest.find(|c: char| !keep(c)).unwrap_or(rest.len());
        self.pos += len;
        &rest[..len]
    }

    /// The text from `start` to here, trimmed: the part of the type read since.
    fn since(&self, start: usize) -> String {
        self.text[start..self.pos].trim().to_string()
    }
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_limit_name(c: char) -> bool {
    is_word(c) || c == '-' || c == '.'
}

/// An alias as `enum:` and `record:` take it: a letter or `_`, then letters,
/// digits and `_`.
fn is_alias(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(is_word)
}

/// The scalar a keyword names: a fixed-width number or `bool`.
fn fixed_scalar(word: &str) -> Option<SceType> {
    SceType::from_attr(word)
        .filter(|t| !matches!(t, SceType::String | SceType::Bytes | SceType::Enum(_)))
}

impl WrittenType {
    /// Read the text of a type, every bound as written. The text is the
    /// attribute's decoded value (`list<int64>[16]`, not `list&lt;…`).
    pub fn read(text: &str) -> Result<Self, TypeRefusal> {
        let mut cursor = Cursor { text, pos: 0 };
        let ty = read_type(&mut cursor)?;
        cursor.skip_ws();
        if cursor.pos < text.len() {
            return Err(TypeRefusal::Malformed {
                place: text.trim().to_string(),
                expected: "nothing follows a complete type",
            });
        }
        Ok(ty)
    }

    /// The bound written on the outermost type that takes one, seen through
    /// every `optional<…>` around it: `Some(slot)` for a text, a byte string or
    /// a list, `None` for a type with no length (a number, `bool`, an enum, a
    /// record, or an optional over one of them). What a legacy attribute
    /// bounds.
    pub fn outer_bound(&self) -> Option<&Option<Bound>> {
        match self {
            Self::Text { max } | Self::Bytes { max } | Self::List { max, .. } => Some(max),
            Self::Optional { elem } => elem.outer_bound(),
            Self::Scalar(_) | Self::Record { .. } => None,
        }
    }

    /// The outermost text, byte string or list of the type, seen through every
    /// `optional<…>`, or `None` for a type with no length: the type a legacy
    /// attribute bounds, and so which attribute a position reads for it.
    pub fn outermost_bounded(&self) -> Option<&Self> {
        match self {
            Self::Text { .. } | Self::Bytes { .. } | Self::List { .. } => Some(self),
            Self::Optional { elem } => elem.outermost_bounded(),
            Self::Scalar(_) | Self::Record { .. } => None,
        }
    }

    /// The same type with `bound` as the bound of its outermost bounded type:
    /// the reading of a legacy attribute as the bracket it is a synonym of.
    ///
    /// Refused when the type already writes that bracket (one bound, one
    /// spelling, even when the numbers agree) and when the type has no bound to
    /// state.
    pub fn with_outer_bound(self, bound: Bound) -> Result<Self, TypeRefusal> {
        let place = self.to_string();
        self.set_outer(bound, &place)
    }

    fn set_outer(self, bound: Bound, place: &str) -> Result<Self, TypeRefusal> {
        match self {
            Self::Text { max: None } => Ok(Self::Text { max: Some(bound) }),
            Self::Bytes { max: None } => Ok(Self::Bytes { max: Some(bound) }),
            Self::List { elem, max: None } => Ok(Self::List {
                elem,
                max: Some(bound),
            }),
            Self::Text { max: Some(_) }
            | Self::Bytes { max: Some(_) }
            | Self::List { max: Some(_), .. } => Err(TypeRefusal::BothForms {
                place: place.to_string(),
            }),
            Self::Optional { elem } => Ok(Self::Optional {
                elem: Box::new(elem.set_outer(bound, place)?),
            }),
            Self::Scalar(_) | Self::Record { .. } => Err(TypeRefusal::NothingToBound {
                place: place.to_string(),
            }),
        }
    }

    /// The type with every bound stated, or the refusal of the outermost text,
    /// byte string or list that states none.
    pub fn bounded(self) -> Result<ValueType, TypeRefusal> {
        let place = self.to_string();
        let required = |max: Option<Bound>| {
            max.ok_or_else(|| TypeRefusal::Unbounded {
                place: place.clone(),
            })
        };
        Ok(match self {
            Self::Scalar(t) => ValueType::Scalar(t),
            Self::Text { max } => ValueType::Text {
                max: required(max)?,
            },
            Self::Bytes { max } => ValueType::Bytes {
                max: required(max)?,
            },
            Self::Record { alias } => ValueType::Record { alias },
            Self::Optional { elem } => ValueType::Optional {
                elem: Box::new(elem.bounded()?),
            },
            Self::List { elem, max } => {
                let max = required(max)?;
                ValueType::List {
                    elem: Box::new(elem.bounded()?),
                    max,
                }
            }
        })
    }

    /// The refusal [`Self::bounded`] would give for a bound left out below the
    /// outermost bounded type, or `None`: the check of a position whose
    /// outermost bound is its argument's (an algorithm's parameter), and so is
    /// not written, while every bound inside it is.
    pub fn unbounded_inside(&self) -> Option<TypeRefusal> {
        match self {
            Self::Scalar(_) | Self::Text { .. } | Self::Bytes { .. } | Self::Record { .. } => None,
            Self::Optional { elem } => elem.unbounded_inside(),
            Self::List { elem, .. } => (**elem).clone().bounded().err(),
        }
    }

    /// The same type with the number of every named bound replaced by the one
    /// `resolve` gives its name: what a build that moves a limit holds
    /// (`crate::limits::resolve`). A bound no name carries is kept.
    pub fn resolve_limits(self, resolve: &mut dyn FnMut(&str, u32) -> u32) -> Self {
        let mut bound = |max: Option<Bound>| {
            max.map(|b| {
                let max = match b.limit.as_deref() {
                    Some(name) => resolve(name, b.max),
                    None => b.max,
                };
                Bound {
                    max,
                    limit: b.limit,
                }
            })
        };
        match self {
            Self::Scalar(t) => Self::Scalar(t),
            Self::Record { alias } => Self::Record { alias },
            Self::Text { max } => Self::Text { max: bound(max) },
            Self::Bytes { max } => Self::Bytes { max: bound(max) },
            Self::Optional { elem } => Self::Optional {
                elem: Box::new(elem.resolve_limits(resolve)),
            },
            Self::List { elem, max } => {
                let max = bound(max);
                Self::List {
                    elem: Box::new(elem.resolve_limits(resolve)),
                    max,
                }
            }
        }
    }
}

impl ValueType {
    /// The type an event schema's field declares, from the model's reading of
    /// it: its scalar or optional scalar, and the bound and limit name beside
    /// it that the field's text or byte string states. `None` for a text or a
    /// byte string with no bound, which no event schema of the language holds
    /// (docs/adr/0086, section 1.4).
    pub fn of_field(ty: &FieldType, max_size: Option<u32>, limit: Option<&str>) -> Option<Self> {
        let bound = || {
            max_size.map(|max| Bound {
                max,
                limit: limit.map(str::to_string),
            })
        };
        let leaf = |t: &SceType| match t {
            SceType::String => bound().map(|max| Self::Text { max }),
            SceType::Bytes => bound().map(|max| Self::Bytes { max }),
            other => Some(Self::Scalar(other.clone())),
        };
        match ty {
            FieldType::Scalar(t) => leaf(t),
            FieldType::Optional { elem } => leaf(elem).map(|elem| Self::Optional {
                elem: Box::new(elem),
            }),
        }
    }

    /// The bound of the outermost text, byte string or list of the type, seen
    /// through every `optional<…>`, and that part of the type; `None` for a
    /// type with no length (a number, `bool`, an enum, a record, or an
    /// optional over one): what a unit names its field's outer bound by.
    pub fn outermost_bound(&self) -> Option<(&Bound, &Self)> {
        match self {
            Self::Text { max } | Self::Bytes { max } | Self::List { max, .. } => Some((max, self)),
            Self::Optional { elem } => elem.outermost_bound(),
            Self::Scalar(_) | Self::Record { .. } => None,
        }
    }

    /// The same type, as a type that may leave a bound out: what a printer of
    /// the written form reads.
    pub fn written(&self) -> WrittenType {
        match self {
            Self::Scalar(t) => WrittenType::Scalar(t.clone()),
            Self::Text { max } => WrittenType::Text {
                max: Some(max.clone()),
            },
            Self::Bytes { max } => WrittenType::Bytes {
                max: Some(max.clone()),
            },
            Self::Optional { elem } => WrittenType::Optional {
                elem: Box::new(elem.written()),
            },
            Self::Record { alias } => WrittenType::Record {
                alias: alias.clone(),
            },
            Self::List { elem, max } => WrittenType::List {
                elem: Box::new(elem.written()),
                max: Some(max.clone()),
            },
        }
    }
}

/// One type, from the cursor on.
fn read_type(cursor: &mut Cursor<'_>) -> Result<WrittenType, TypeRefusal> {
    cursor.skip_ws();
    let start = cursor.pos;
    let word = cursor.take_while(is_word);
    let ty = match word {
        "optional" => {
            let elem = read_element(cursor, start, "optional")?;
            if matches!(elem, WrittenType::Optional { .. }) {
                return Err(TypeRefusal::NestedOptional {
                    place: cursor.since(start),
                });
            }
            WrittenType::Optional {
                elem: Box::new(elem),
            }
        }
        "list" => {
            let elem = read_element(cursor, start, "list")?;
            let max = read_bracket(cursor, start)?;
            return Ok(WrittenType::List {
                elem: Box::new(elem),
                max,
            });
        }
        "string" => {
            let max = read_bracket(cursor, start)?;
            return Ok(WrittenType::Text { max });
        }
        "bytes" => {
            let max = read_bracket(cursor, start)?;
            return Ok(WrittenType::Bytes { max });
        }
        "enum" | "record" => {
            if cursor.peek() != Some(':') {
                return Err(TypeRefusal::Malformed {
                    place: cursor.since(start),
                    expected: "an alias follows `:`, as in `enum:Level` or `record:Item`",
                });
            }
            cursor.pos += 1;
            let alias = cursor.take_while(is_word);
            if !is_alias(alias) {
                return Err(TypeRefusal::Unknown {
                    written: cursor.since(start),
                });
            }
            if word == "enum" {
                WrittenType::Scalar(SceType::Enum(crate::forge::model::EnumRef {
                    alias: alias.to_string(),
                }))
            } else {
                WrittenType::Record {
                    alias: alias.to_string(),
                }
            }
        }
        other => match fixed_scalar(other) {
            Some(t) => WrittenType::Scalar(t),
            None => {
                let rest = &cursor.text[start..];
                let written = rest
                    .split(|c: char| matches!(c, '<' | '>' | '[' | ']') || c.is_whitespace())
                    .next()
                    .unwrap_or(rest);
                return Err(TypeRefusal::Unknown {
                    written: written.to_string(),
                });
            }
        },
    };
    // A type of no length takes no bound.
    cursor.skip_ws();
    if cursor.peek() == Some('[') {
        let place = cursor.since(start);
        return Err(TypeRefusal::BoundOnUnboundable { place });
    }
    Ok(ty)
}

/// The `<T>` after `optional` or `list`.
fn read_element(
    cursor: &mut Cursor<'_>,
    start: usize,
    constructor: &str,
) -> Result<WrittenType, TypeRefusal> {
    if !cursor.eat('<') {
        return Err(TypeRefusal::Malformed {
            place: cursor.since(start),
            expected: if constructor == "list" {
                "`list` takes its element in angle brackets, as in `list<int64>[16]`"
            } else {
                "`optional` takes its element in angle brackets, as in `optional<int64>`"
            },
        });
    }
    let elem = read_type(cursor)?;
    if !cursor.eat('>') {
        return Err(TypeRefusal::Malformed {
            place: cursor.since(start),
            expected: "a `>` closes the element",
        });
    }
    Ok(elem)
}

/// The bracket after a type that takes one, or `None` when none is written.
fn read_bracket(cursor: &mut Cursor<'_>, start: usize) -> Result<Option<Bound>, TypeRefusal> {
    let before = cursor.pos;
    if !cursor.eat('[') {
        cursor.pos = before;
        return Ok(None);
    }
    let open = cursor.pos;
    let Some(close) = cursor.text[open..].find(']') else {
        return Err(TypeRefusal::Malformed {
            place: cursor.since(start),
            expected: "a `]` closes the bound",
        });
    };
    let inside = &cursor.text[open..open + close];
    cursor.pos = open + close + 1;
    let place = cursor.text[start..before].trim().to_string();
    let refused = || TypeRefusal::BadBound {
        place: place.clone(),
        written: inside.to_string(),
    };
    let number = |text: &str| {
        let text = text.trim();
        if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        text.parse::<u32>().ok().filter(|n| *n > 0)
    };
    match inside.split_once('=') {
        Some((name, value)) => {
            let name = name.trim();
            let starts_well = name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
            if !starts_well || !name.chars().all(is_limit_name) {
                return Err(refused());
            }
            let max = number(value).ok_or_else(refused)?;
            Ok(Some(Bound::named(name, max)))
        }
        None => Ok(Some(Bound::of(number(inside).ok_or_else(refused)?))),
    }
}

// ── The narrowing onto the shapes a position lowers ─────────────────────────

/// A type a position's lowering does not hold yet: the part of the type it
/// stops at, and why. Refused by name where the position is read, with
/// `generate/unsupported-feature` (docs/adr/0086, section 7.3), never as a
/// rule of the grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composition {
    /// The part of the type that is composed, as the grammar spells it.
    pub place: String,
    /// The constructor of that part: `list`, `record`, `optional`.
    pub constructor: String,
}

impl Composition {
    fn of<B: BoundSlot>(ty: &TypeOf<B>) -> Self {
        Self {
            place: ty.to_string(),
            constructor: ty.constructor(),
        }
    }
}

/// The leaf a type is when it is one value of a scalar, a text or a byte
/// string, with the bound of a text or a byte string: what the lowerings of a
/// field, an entry and a slot read as a `SceType` and a bound beside it.
fn leaf(ty: &WrittenType) -> Option<(SceType, Option<Bound>)> {
    match ty {
        WrittenType::Scalar(t) => Some((t.clone(), None)),
        WrittenType::Text { max } => Some((SceType::String, max.clone())),
        WrittenType::Bytes { max } => Some((SceType::Bytes, max.clone())),
        WrittenType::Optional { .. } | WrittenType::Record { .. } | WrittenType::List { .. } => {
            None
        }
    }
}

impl WrittenType {
    /// The type as an event schema's field holds it today: a scalar, a text or
    /// a byte string, or an optional one, with the bound of its text. Any other
    /// type is a composition the field's lowerings do not hold yet.
    pub fn as_field_type(&self) -> Result<(FieldType, Option<Bound>), Composition> {
        if let Some((t, max)) = leaf(self) {
            return Ok((FieldType::Scalar(t), max));
        }
        match self {
            Self::Optional { elem } => leaf(elem)
                .map(|(t, max)| (FieldType::Optional { elem: t }, max))
                .ok_or_else(|| Composition::of(&**elem)),
            _ => Err(Composition::of(self)),
        }
    }

    /// The type as a statechart's variable and an algorithm's slot hold it
    /// today: a scalar, a text or a byte string; a record; a list of fixed-width
    /// numbers, `bool` or records; an optional scalar, text, byte string or
    /// record. With the bound of the outermost bounded type. Any other type is a
    /// composition those lowerings do not hold yet.
    pub fn as_algorithm_type(&self) -> Result<(AlgorithmValueType, Option<Bound>), Composition> {
        if let Some((t, max)) = leaf(self) {
            return Ok((AlgorithmValueType::Scalar(t), max));
        }
        match self {
            Self::Record { alias } => Ok((
                AlgorithmValueType::Record {
                    alias: alias.clone(),
                },
                None,
            )),
            Self::List { elem, max } => {
                let elem = match &**elem {
                    Self::Scalar(t) if AlgorithmValueType::list_elem_admitted(t) => {
                        ListElemType::Scalar(t.clone())
                    }
                    Self::Record { alias } => ListElemType::Record {
                        alias: alias.clone(),
                    },
                    other => return Err(Composition::of(other)),
                };
                Ok((AlgorithmValueType::List { elem }, max.clone()))
            }
            Self::Optional { elem } => {
                if let Self::Record { alias } = &**elem {
                    return Ok((
                        AlgorithmValueType::Optional {
                            elem: OptionalElemType::Record {
                                alias: alias.clone(),
                            },
                        },
                        None,
                    ));
                }
                leaf(elem)
                    .map(|(t, max)| {
                        (
                            AlgorithmValueType::Optional {
                                elem: OptionalElemType::Scalar(t),
                            },
                            max,
                        )
                    })
                    .ok_or_else(|| Composition::of(&**elem))
            }
            Self::Scalar(_) | Self::Text { .. } | Self::Bytes { .. } => Err(Composition::of(self)),
        }
    }
}

// ── The size calculus (docs/adr/0086, section 2) ────────────────────────────

/// What the size of a type needs from outside it: the fields of the records it
/// names and the variants of the enums it names. A build answers from its
/// imports; a test answers from a table.
pub trait TypeEnv {
    /// The fields of the record `alias` names, in the schema's order, or `None`
    /// when no record goes by that alias.
    fn record_fields(&self, alias: &str) -> Option<Vec<(String, ValueType)>>;
    /// The declared names of the variants of the enum `alias` names, or `None`
    /// when no enum goes by that alias.
    fn enum_names(&self, alias: &str) -> Option<Vec<String>>;
    /// The underlying type of the enum `alias` names: what its value is on a
    /// CBOR wire.
    fn enum_underlying(&self, alias: &str) -> Option<SceType>;
}

/// Why a type has no size the build can compute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeError {
    /// An alias no record or enum of the environment goes by.
    Unresolved { alias: String },
    /// A record that holds itself, directly or through other records: the
    /// aliases on the way, the first repeated at the end.
    ContainsItself { cycle: Vec<String> },
    /// A size past 2^64 - 1 bytes.
    Overflow,
}

impl fmt::Display for SizeError {
    /// The reason as a refusal states it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unresolved { alias } => {
                write!(f, "`{alias}` names no record or enum the document imports")
            }
            Self::ContainsItself { cycle } => write!(
                f,
                "the record `{}` holds itself ({}), and has no largest value",
                cycle.first().map_or("", String::as_str),
                cycle.join(" -> ")
            ),
            Self::Overflow => write!(f, "it is past 2^64 - 1 bytes"),
        }
    }
}

/// The length of a CBOR head whose argument is `n` (RFC 8949 section 3): 1 below
/// 24, 2 below 2^8, 3 below 2^16, 5 below 2^32, 9 otherwise.
pub fn cbor_head_len(n: u64) -> u64 {
    match n {
        0..=23 => 1,
        24..=0xff => 2,
        0x100..=0xffff => 3,
        0x1_0000..=0xffff_ffff => 5,
        _ => 9,
    }
}

/// The bytes `text` takes inside a JSON string: two for a quote, a backslash
/// and the five controls with a short escape, six for any other control
/// (`\u00XX`), and its UTF-8 bytes otherwise.
pub fn json_escaped_len(text: &str) -> u64 {
    text.chars()
        .map(|c| match c {
            '"' | '\\' | '\u{8}' | '\u{c}' | '\n' | '\r' | '\t' => 2,
            c if (c as u32) < 0x20 => 6,
            c => c.len_utf8() as u64,
        })
        .sum()
}

/// The two sizes a type has, computed by one recursion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Measure {
    /// `J(T)`: the largest JSON text of a value.
    Json,
    /// `C(T)`: the largest deterministic CBOR encoding of a value.
    Cbor,
}

fn add(a: u64, b: u64) -> Result<u64, SizeError> {
    a.checked_add(b).ok_or(SizeError::Overflow)
}

fn mul(a: u64, b: u64) -> Result<u64, SizeError> {
    a.checked_mul(b).ok_or(SizeError::Overflow)
}

/// `J` or `C` of a scalar.
fn scalar_size(t: &SceType, measure: Measure, env: &dyn TypeEnv) -> Result<u64, SizeError> {
    Ok(match (t, measure) {
        (SceType::Uint8, Measure::Json) => 3,
        (SceType::Uint16, Measure::Json) => 5,
        (SceType::Uint32, Measure::Json) => 10,
        (SceType::Int8, Measure::Json) => 4,
        (SceType::Int16, Measure::Json) => 6,
        (SceType::Int32, Measure::Json) => 11,
        // A 64-bit integer crosses as the quoted text of its digits.
        (SceType::Uint64 | SceType::Int64, Measure::Json) => 22,
        // A single crosses as the double it widens to, and a double is spelled
        // as ECMAScript's Number::toString spells it (ARCHITECTURE.md, "JSON
        // Number Text"): seventeen digits after `-0.00000`, which only the
        // decimal band from 1e-6 to 1e-5 reaches, are its longest spelling; a
        // spelling with an exponent is at most 24.
        (SceType::Float32 | SceType::Float64, Measure::Json) => 25,
        (SceType::Bool, Measure::Json) => 5,
        (SceType::Uint8 | SceType::Int8, Measure::Cbor) => 2,
        (SceType::Uint16 | SceType::Int16, Measure::Cbor) => 3,
        (SceType::Uint32 | SceType::Int32 | SceType::Float32, Measure::Cbor) => 5,
        (SceType::Uint64 | SceType::Int64 | SceType::Float64, Measure::Cbor) => 9,
        (SceType::Bool, Measure::Cbor) => 1,
        (SceType::Enum(reference), Measure::Json) => {
            let names = env
                .enum_names(&reference.alias)
                .ok_or_else(|| SizeError::Unresolved {
                    alias: reference.alias.clone(),
                })?;
            2 + names.iter().map(|n| json_escaped_len(n)).max().unwrap_or(0)
        }
        (SceType::Enum(reference), Measure::Cbor) => {
            let underlying =
                env.enum_underlying(&reference.alias)
                    .ok_or_else(|| SizeError::Unresolved {
                        alias: reference.alias.clone(),
                    })?;
            if matches!(underlying, SceType::Enum(_)) {
                return Err(SizeError::Unresolved {
                    alias: reference.alias.clone(),
                });
            }
            scalar_size(&underlying, measure, env)?
        }
        // A `Scalar` never holds a text or a byte string; were one built so, it
        // is sized as the unbounded text it would be, which has no size.
        (SceType::String | SceType::Bytes, _) => return Err(SizeError::Overflow),
    })
}

fn size(
    ty: &ValueType,
    measure: Measure,
    env: &dyn TypeEnv,
    records: &mut Vec<String>,
) -> Result<u64, SizeError> {
    match ty {
        ValueType::Scalar(t) => scalar_size(t, measure, env),
        ValueType::Text { max } | ValueType::Bytes { max } => {
            let b = u64::from(max.max);
            match measure {
                // Every byte at its worst escape, `\u00XX`, and the quotes.
                Measure::Json => add(2, mul(6, b)?),
                Measure::Cbor => add(cbor_head_len(b), b),
            }
        }
        ValueType::Optional { elem } => {
            let inner = size(elem, measure, env, records)?;
            Ok(match measure {
                // `null`.
                Measure::Json => inner.max(4),
                // `0xf6`.
                Measure::Cbor => inner.max(1),
            })
        }
        ValueType::List { elem, max } => {
            let b = u64::from(max.max);
            let inner = size(elem, measure, env, records)?;
            match measure {
                // The brackets, the elements and a comma between each two.
                Measure::Json => add(add(2, mul(b, inner)?)?, b.saturating_sub(1)),
                Measure::Cbor => add(cbor_head_len(b), mul(b, inner)?),
            }
        }
        ValueType::Record { alias } => {
            if records.iter().any(|r| r == alias) {
                let mut cycle = records.clone();
                cycle.push(alias.clone());
                return Err(SizeError::ContainsItself { cycle });
            }
            let fields = env
                .record_fields(alias)
                .ok_or_else(|| SizeError::Unresolved {
                    alias: alias.clone(),
                })?;
            records.push(alias.clone());
            let n = fields.len() as u64;
            let mut total = match measure {
                // The braces and a comma between each two members.
                Measure::Json => 2 + n.saturating_sub(1),
                Measure::Cbor => cbor_head_len(n),
            };
            for (id, field) in &fields {
                let key = match measure {
                    // The quoted key and its colon.
                    Measure::Json => 2 + json_escaped_len(id) + 1,
                    Measure::Cbor => {
                        let len = id.len() as u64;
                        cbor_head_len(len) + len
                    }
                };
                total = add(add(total, key)?, size(field, measure, env, records)?)?;
            }
            records.pop();
            Ok(total)
        }
    }
}

/// `J(T)`: the length of the largest JSON text of a value of `ty`, as the wire
/// spells it (docs/adr/0086, section 2.1).
pub fn json_max_len(ty: &ValueType, env: &dyn TypeEnv) -> Result<u64, SizeError> {
    size(ty, Measure::Json, env, &mut Vec::new())
}

/// `C(T)`: the length of the largest deterministic CBOR encoding of a value of
/// `ty` (RFC 8949 section 4.2.1; docs/adr/0086, section 2.1).
pub fn cbor_max_len(ty: &ValueType, env: &dyn TypeEnv) -> Result<u64, SizeError> {
    size(ty, Measure::Cbor, env, &mut Vec::new())
}

/// `J` of a payload: the record of `fields`, as a schema's `DATA_MAX_LEN` is.
pub fn payload_json_max_len(
    fields: &[(String, ValueType)],
    env: &dyn TypeEnv,
) -> Result<u64, SizeError> {
    struct Payload<'a> {
        fields: &'a [(String, ValueType)],
        env: &'a dyn TypeEnv,
    }
    impl TypeEnv for Payload<'_> {
        fn record_fields(&self, alias: &str) -> Option<Vec<(String, ValueType)>> {
            if alias.is_empty() {
                Some(self.fields.to_vec())
            } else {
                self.env.record_fields(alias)
            }
        }
        fn enum_names(&self, alias: &str) -> Option<Vec<String>> {
            self.env.enum_names(alias)
        }
        fn enum_underlying(&self, alias: &str) -> Option<SceType> {
            self.env.enum_underlying(alias)
        }
    }
    json_max_len(
        &ValueType::Record {
            alias: String::new(),
        },
        &Payload { fields, env },
    )
}

/// The cycle through which the record `alias` holds itself, directly or
/// through the records its fields name at any depth, or `None` when it does
/// not: the aliases on the way, `alias` first and last. A record whose alias the
/// environment does not resolve is no cycle here; its refusal is the import's.
pub fn contains_itself(alias: &str, env: &dyn TypeEnv) -> Option<Vec<String>> {
    fn visit(alias: &str, env: &dyn TypeEnv, path: &mut Vec<String>) -> Option<Vec<String>> {
        if let Some(at) = path.iter().position(|a| a == alias) {
            let mut cycle = path[at..].to_vec();
            cycle.push(alias.to_string());
            return Some(cycle);
        }
        let fields = env.record_fields(alias)?;
        path.push(alias.to_string());
        for (_, ty) in &fields {
            for inner in ty.record_aliases() {
                if let Some(cycle) = visit(inner, env, path) {
                    return Some(cycle);
                }
            }
        }
        path.pop();
        None
    }
    visit(alias, env, &mut Vec::new())
}

// ── The relation "fits" (docs/adr/0086, section 5.1) ────────────────────────

/// What "fits" needs from outside a type: whether two aliases name one record
/// type and one enum. A record type is its document, and two revisions of one
/// schema are one record (SCE_FORGE.md section 4.12, docs/adr/0044); an enum is
/// its document or a revision of it. A build answers from the documents its
/// aliases resolve to, each alias read in the document that writes it.
pub trait TypeIdentity {
    /// Whether the record `from` names (in the type of the value) is the
    /// record `to` names (in the type expected).
    fn same_record(&self, from: &str, to: &str) -> bool;
    /// Whether the enum `from` names is the enum `to` names.
    fn same_enum(&self, from: &str, to: &str) -> bool;
}

/// Why a value of one type may not stand where another is expected: the
/// first place, outermost first, where the two differ.
///
/// `path` leads from the outermost type to that place, one `/*` for each list
/// element on the way (an optional is passed through, since a present value is
/// the value of its element): `""` for the types themselves, `/*/*` for the
/// elements of the elements. A caller puts the name of the field or variable in
/// front of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Misfit {
    pub path: String,
    /// The part of the value's type at `path`, as the grammar spells it.
    pub from: String,
    /// The part of the expected type at `path`.
    pub to: String,
    /// Why it does not fit.
    pub why: MisfitReason,
}

/// The reason a part of a type does not fit the part it stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MisfitReason {
    /// The two are of different constructors, or of different numbers.
    Constructor,
    /// The value's text, byte string or list may be longer than the place
    /// expected holds.
    Bound,
    /// The value may be absent and the place expected may not.
    MayBeAbsent,
    /// Two records, or two enums, of different documents.
    Identity,
}

impl Misfit {
    /// The sentence a refusal states, the field's or variable's name `name`
    /// in front of the path.
    pub fn sentence(&self, name: &str) -> String {
        let place = format!("{name}{}", self.path);
        match self.why {
            MisfitReason::Constructor => format!(
                "'{place}' is a {} where a {} is expected",
                self.from, self.to
            ),
            MisfitReason::Bound => format!(
                "'{place}' is a {}, which may hold more than the {} expected there",
                self.from, self.to
            ),
            MisfitReason::MayBeAbsent => format!(
                "'{place}' is a {}, which may be absent where a {} is expected",
                self.from, self.to
            ),
            MisfitReason::Identity => format!(
                "'{place}' is a {}, of another document than the {} expected there",
                self.from, self.to
            ),
        }
    }
}

/// Whether a value of type `from` may stand where `to` is expected
/// (docs/adr/0086, section 5.1), or the first place where it may not.
///
/// - a number or `bool` fits only its own type, and an enum only its own
///   enum ([`TypeIdentity::same_enum`]);
/// - `string[a]` fits `string[b]`, and `bytes[a]` fits `bytes[b]`, when
///   `a <= b`;
/// - `list<S>[a]` fits `list<T>[b]` when `S` fits `T` and `a <= b`;
/// - `optional<S>` fits `optional<T>` when `S` fits `T`, and `S` fits
///   `optional<T>` when `S` fits `T`;
/// - `record:R` fits `record:Q` when both are one record
///   ([`TypeIdentity::same_record`]).
///
/// Every bound is the number the build holds, after its limit name and
/// `--limit` ([`WrittenType::resolve_limits`]), so the relation is the one a
/// build that moves a limit keeps. A value that fits is a value of the type
/// expected, whatever it holds, so a copy of it checks nothing at run time.
pub fn fits(from: &ValueType, to: &ValueType, identity: &dyn TypeIdentity) -> Result<(), Misfit> {
    fits_at(from, to, identity, "")
}

fn fits_at(
    from: &ValueType,
    to: &ValueType,
    identity: &dyn TypeIdentity,
    path: &str,
) -> Result<(), Misfit> {
    let misfit = |why: MisfitReason| Misfit {
        path: path.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        why,
    };
    match (from, to) {
        (ValueType::Scalar(SceType::Enum(a)), ValueType::Scalar(SceType::Enum(b))) => {
            if identity.same_enum(&a.alias, &b.alias) {
                Ok(())
            } else {
                Err(misfit(MisfitReason::Identity))
            }
        }
        (ValueType::Scalar(a), ValueType::Scalar(b)) => {
            if a == b {
                Ok(())
            } else {
                Err(misfit(MisfitReason::Constructor))
            }
        }
        (ValueType::Text { max: a }, ValueType::Text { max: b })
        | (ValueType::Bytes { max: a }, ValueType::Bytes { max: b }) => {
            if a.max <= b.max {
                Ok(())
            } else {
                Err(misfit(MisfitReason::Bound))
            }
        }
        (ValueType::List { elem: s, max: a }, ValueType::List { elem: t, max: b }) => {
            fits_at(s, t, identity, &format!("{path}/*"))?;
            if a.max <= b.max {
                Ok(())
            } else {
                Err(misfit(MisfitReason::Bound))
            }
        }
        (ValueType::Optional { elem: s }, ValueType::Optional { elem: t }) => {
            fits_at(s, t, identity, path)
        }
        (ValueType::Optional { .. }, _) => Err(misfit(MisfitReason::MayBeAbsent)),
        (_, ValueType::Optional { elem: t }) => fits_at(from, t, identity, path),
        (ValueType::Record { alias: r }, ValueType::Record { alias: q }) => {
            if identity.same_record(r, q) {
                Ok(())
            } else {
                Err(misfit(MisfitReason::Identity))
            }
        }
        (
            ValueType::Scalar(_)
            | ValueType::Text { .. }
            | ValueType::Bytes { .. }
            | ValueType::List { .. }
            | ValueType::Record { .. },
            _,
        ) => Err(misfit(MisfitReason::Constructor)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::EnumRef;
    use std::collections::{BTreeMap, BTreeSet};

    fn enum_ty(alias: &str) -> SceType {
        SceType::Enum(EnumRef {
            alias: alias.to_string(),
        })
    }

    // ── The oracle: every written type up to depth 3 ──────────────────────

    /// One leaf of the enumeration: its text, and whether it takes a bound.
    const LEAVES: &[(&str, bool)] = &[
        ("uint8", false),
        ("int64", false),
        ("float64", false),
        ("bool", false),
        ("enum:Level", false),
        ("record:Item", false),
        ("string", true),
        ("bytes", true),
    ];

    /// A written type of the enumeration, with what the oracle says of it:
    /// whether it is a type of the grammar at all (`well_formed`: no optional
    /// directly inside an optional) and whether every bound is stated.
    #[derive(Debug, Clone)]
    struct Case {
        text: String,
        well_formed: bool,
        every_bound: bool,
        depth: usize,
        /// The text with every bracket removed from its outermost bounded
        /// type, and that bracket, when the type has one: the legacy spelling.
        outer: Option<(String, String)>,
    }

    /// Every written type up to `depth` constructors deep, each text, byte
    /// string and list once with its bound and once without.
    fn cases(depth: usize) -> Vec<Case> {
        let mut out: Vec<Case> = Vec::new();
        for (leaf, bounded) in LEAVES {
            if *bounded {
                for (stated, bracket) in [(true, "[8]"), (false, "")] {
                    out.push(Case {
                        text: format!("{leaf}{bracket}"),
                        well_formed: true,
                        every_bound: stated,
                        depth: 1,
                        outer: Some((leaf.to_string(), bracket.to_string())),
                    });
                }
            } else {
                out.push(Case {
                    text: leaf.to_string(),
                    well_formed: true,
                    every_bound: true,
                    depth: 1,
                    outer: None,
                });
            }
        }
        let mut previous: Vec<Case> = out.clone();
        for d in 2..=depth {
            let mut next = Vec::new();
            for inner in &previous {
                next.push(Case {
                    text: format!("optional<{}>", inner.text),
                    well_formed: inner.well_formed && !inner.text.starts_with("optional<"),
                    every_bound: inner.every_bound,
                    depth: d,
                    // An optional has no bound: an attribute bounds the type inside.
                    outer: inner.outer.clone(),
                });
                for (stated, bracket) in [(true, "[3]"), (false, "")] {
                    next.push(Case {
                        text: format!("list<{}>{bracket}", inner.text),
                        well_formed: inner.well_formed,
                        every_bound: inner.every_bound && stated,
                        depth: d,
                        outer: Some((format!("list<{}>", inner.text), bracket.to_string())),
                    });
                }
            }
            out.extend(next.iter().cloned());
            previous = next;
        }
        out
    }

    #[test]
    fn the_enumeration_covers_every_constructor_at_every_depth() {
        let all = cases(3);
        for d in 1..=3 {
            let constructors: &[&str] = if d == 1 {
                &["string", "bytes", "record:", "enum:", "uint8", "bool"]
            } else {
                &["optional<", "list<", "string", "bytes", "record:", "enum:"]
            };
            for constructor in constructors {
                assert!(
                    all.iter()
                        .any(|c| c.depth == d && c.text.contains(constructor)),
                    "no case of depth {d} holds {constructor}"
                );
            }
        }
        // 10 written leaves, then 3 per type of the depth below.
        assert_eq!(all.iter().filter(|c| c.depth == 1).count(), 10);
        assert_eq!(all.iter().filter(|c| c.depth == 2).count(), 30);
        assert_eq!(all.iter().filter(|c| c.depth == 3).count(), 90);
    }

    #[test]
    fn a_type_is_read_exactly_when_the_oracle_says_it_is_one() {
        for case in cases(3) {
            let read = WrittenType::read(&case.text);
            if !case.well_formed {
                assert!(
                    matches!(read, Err(TypeRefusal::NestedOptional { .. })),
                    "{}: {read:?}",
                    case.text
                );
                continue;
            }
            let written = read.unwrap_or_else(|e| panic!("{}: {e:?}", case.text));
            assert_eq!(written.to_string(), case.text, "the printer is the inverse");
            assert_eq!(written.depth(), case.depth, "{}", case.text);
            match written.bounded() {
                Ok(ty) => {
                    assert!(case.every_bound, "{} has a bound left out", case.text);
                    assert_eq!(ty.to_string(), case.text);
                    assert_eq!(ty.written().to_string(), case.text);
                }
                Err(refusal) => {
                    assert!(!case.every_bound, "{}: {refusal:?}", case.text);
                    assert!(
                        matches!(refusal, TypeRefusal::Unbounded { .. }),
                        "{refusal:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_legacy_bound_is_the_bracket_on_the_outermost_bounded_type() {
        for case in cases(3).into_iter().filter(|c| c.well_formed) {
            let Some((bare, bracket)) = &case.outer else {
                // A type of no length: an attribute beside it bounds nothing.
                let written = WrittenType::read(&case.text).unwrap();
                assert_eq!(written.outer_bound(), None, "{}", case.text);
                assert!(
                    matches!(
                        written.with_outer_bound(Bound::of(5)),
                        Err(TypeRefusal::NothingToBound { .. })
                    ),
                    "{}",
                    case.text
                );
                continue;
            };
            let written = WrittenType::read(&case.text).unwrap();
            if bracket.is_empty() {
                // The legacy spelling: the attribute supplies the bracket.
                let legacy = written.with_outer_bound(Bound::of(5)).unwrap();
                let expected = case.text.replacen(bare.as_str(), &format!("{bare}[5]"), 1);
                assert_eq!(legacy.to_string(), expected, "{}", case.text);
            } else {
                // Both spellings: refused, even when the numbers agree.
                let agreeing = bracket
                    .trim_matches(|c| c == '[' || c == ']')
                    .parse()
                    .unwrap();
                assert!(
                    matches!(
                        written.with_outer_bound(Bound::of(agreeing)),
                        Err(TypeRefusal::BothForms { .. })
                    ),
                    "{}",
                    case.text
                );
            }
        }
    }

    #[test]
    fn the_legacy_and_the_bracket_spelling_read_as_one_type() {
        let bracket = WrittenType::read("optional<list<string[8]>[4]>").unwrap();
        let legacy = WrittenType::read("optional<list<string[8]>>")
            .unwrap()
            .with_outer_bound(Bound::of(4))
            .unwrap();
        assert_eq!(bracket, legacy);
        assert_eq!(
            legacy.bounded().unwrap().to_string(),
            "optional<list<string[8]>[4]>"
        );
    }

    #[test]
    fn the_outermost_unbounded_type_is_the_place_named() {
        let refusal = WrittenType::read("list<list<string>[4]>")
            .unwrap()
            .bounded();
        assert_eq!(
            refusal,
            Err(TypeRefusal::Unbounded {
                place: "list<list<string>[4]>".into()
            })
        );
        let refusal = WrittenType::read("list<list<string>[4]>[3]")
            .unwrap()
            .bounded();
        assert_eq!(
            refusal,
            Err(TypeRefusal::Unbounded {
                place: "string".into()
            })
        );
    }

    #[test]
    fn a_parameter_is_checked_below_its_outermost_bound_only() {
        let view = WrittenType::read("list<record:Item>").unwrap();
        assert_eq!(view.unbounded_inside(), None);
        let view = WrittenType::read("bytes").unwrap();
        assert_eq!(view.unbounded_inside(), None);
        let inner = WrittenType::read("list<string>").unwrap();
        assert_eq!(
            inner.unbounded_inside(),
            Some(TypeRefusal::Unbounded {
                place: "string".into()
            })
        );
        let through_optional = WrittenType::read("optional<list<bytes>>").unwrap();
        assert_eq!(
            through_optional.unbounded_inside(),
            Some(TypeRefusal::Unbounded {
                place: "bytes".into()
            })
        );
    }

    #[test]
    fn a_named_bound_is_read_and_resolved() {
        let written = WrittenType::read("list<string[merge_text=96]>[shared_parts=64]").unwrap();
        assert_eq!(
            written.to_string(),
            "list<string[merge_text=96]>[shared_parts=64]"
        );
        let resolved =
            written.resolve_limits(&mut |name, n| if name == "merge_text" { 12 } else { n });
        assert_eq!(
            resolved.to_string(),
            "list<string[merge_text=12]>[shared_parts=64]"
        );
        let ty = resolved.bounded().unwrap();
        let ValueType::List { elem, max } = ty else {
            panic!("a list");
        };
        assert_eq!(max, Bound::named("shared_parts", 64));
        assert_eq!(
            *elem,
            ValueType::Text {
                max: Bound::named("merge_text", 12)
            }
        );
    }

    #[test]
    fn whitespace_between_tokens_is_read_and_not_printed() {
        let written = WrittenType::read(" list < string [ 8 ] > [ 4 ] ").unwrap();
        assert_eq!(written.to_string(), "list<string[8]>[4]");
    }

    #[test]
    fn every_malformed_bound_is_refused_as_one() {
        for inside in [
            "0",
            "-1",
            "4294967296",
            "x",
            "",
            "=4",
            "1x=4",
            "n=0",
            "n=",
            "n=4=5",
            "0x10",
        ] {
            let text = format!("string[{inside}]");
            assert!(
                matches!(WrittenType::read(&text), Err(TypeRefusal::BadBound { .. })),
                "{text}: {:?}",
                WrittenType::read(&text)
            );
        }
        assert_eq!(
            WrittenType::read("string[4294967295]").unwrap(),
            WrittenType::Text {
                max: Some(Bound::of(u32::MAX))
            }
        );
    }

    #[test]
    fn a_bracket_on_a_type_of_no_length_is_refused() {
        for text in [
            "uint8[4]",
            "bool[1]",
            "enum:Level[2]",
            "record:Item[3]",
            "optional<string[8]>[2]",
        ] {
            assert!(
                matches!(
                    WrittenType::read(text),
                    Err(TypeRefusal::BoundOnUnboundable { .. })
                ),
                "{text}: {:?}",
                WrittenType::read(text)
            );
        }
    }

    #[test]
    fn a_text_that_is_no_type_is_refused_by_what_it_lacks() {
        let unknown =
            |text: &str| matches!(WrittenType::read(text), Err(TypeRefusal::Unknown { .. }));
        let malformed =
            |text: &str| matches!(WrittenType::read(text), Err(TypeRefusal::Malformed { .. }));
        assert!(unknown("strin[8]"));
        assert!(unknown("list<int65>[3]"));
        assert!(unknown(""));
        assert!(unknown("enum:"));
        assert!(unknown("record:9x"));
        assert!(malformed("list<int64"));
        assert!(malformed("list int64"));
        assert!(malformed("optional int64"));
        assert!(malformed("string[8"));
        assert!(malformed("enum Level"));
        assert!(malformed("int64 int64"));
        assert!(malformed("list<int64>[3]>"));
    }

    #[test]
    fn the_aliases_a_type_names_are_found_at_every_depth() {
        let written = WrittenType::read("list<optional<list<record:Item>[2]>>[3]").unwrap();
        assert_eq!(written.record_aliases(), ["Item"]);
        let written = WrittenType::read("optional<list<enum:Level>[2]>").unwrap();
        assert_eq!(written.enum_aliases(), ["Level"]);
    }

    // ── The narrowing onto today's lowerings ──────────────────────────────

    #[test]
    fn a_field_holds_a_leaf_or_an_optional_leaf_and_names_any_composition() {
        let read = |t: &str| WrittenType::read(t).unwrap();
        assert_eq!(
            read("string[8]").as_field_type(),
            Ok((FieldType::Scalar(SceType::String), Some(Bound::of(8))))
        );
        assert_eq!(
            read("optional<bytes[n=4]>").as_field_type(),
            Ok((
                FieldType::Optional {
                    elem: SceType::Bytes
                },
                Some(Bound::named("n", 4))
            ))
        );
        assert_eq!(
            read("enum:Level").as_field_type(),
            Ok((FieldType::Scalar(enum_ty("Level")), None))
        );
        for (text, place, constructor) in [
            ("list<uint8>[2]", "list<uint8>[2]", "list"),
            ("record:Item", "record:Item", "record"),
            ("optional<list<uint8>[2]>", "list<uint8>[2]", "list"),
            ("optional<record:Item>", "record:Item", "record"),
        ] {
            assert_eq!(
                read(text).as_field_type(),
                Err(Composition {
                    place: place.into(),
                    constructor: constructor.into()
                }),
                "{text}"
            );
        }
    }

    #[test]
    fn a_slot_holds_what_its_lowerings_hold_and_names_any_composition() {
        let read = |t: &str| WrittenType::read(t).unwrap();
        assert_eq!(
            read("list<int64>[16]").as_algorithm_type(),
            Ok((
                AlgorithmValueType::List {
                    elem: ListElemType::Scalar(SceType::Int64)
                },
                Some(Bound::of(16))
            ))
        );
        assert_eq!(
            read("list<record:Item>").as_algorithm_type(),
            Ok((
                AlgorithmValueType::List {
                    elem: ListElemType::Record {
                        alias: "Item".into()
                    }
                },
                None
            ))
        );
        assert_eq!(
            read("optional<record:Item>").as_algorithm_type(),
            Ok((
                AlgorithmValueType::Optional {
                    elem: OptionalElemType::Record {
                        alias: "Item".into()
                    }
                },
                None
            ))
        );
        assert_eq!(
            read("optional<string[8]>").as_algorithm_type(),
            Ok((
                AlgorithmValueType::Optional {
                    elem: OptionalElemType::Scalar(SceType::String)
                },
                Some(Bound::of(8))
            ))
        );
        for (text, place) in [
            ("list<string[8]>[4]", "string[8]"),
            ("list<enum:Level>[2]", "enum:Level"),
            ("list<list<uint8>[2]>[2]", "list<uint8>[2]"),
            ("list<optional<uint16>>[2]", "optional<uint16>"),
            ("optional<list<uint8>[2]>", "list<uint8>[2]"),
        ] {
            assert_eq!(
                read(text).as_algorithm_type().map_err(|c| c.place),
                Err(place.to_string()),
                "{text}"
            );
        }
    }

    // ── The size calculus ─────────────────────────────────────────────────

    /// A table of records and enums.
    #[derive(Default)]
    struct Table {
        records: BTreeMap<String, Vec<(String, ValueType)>>,
        enums: BTreeMap<String, (Vec<String>, SceType)>,
    }

    impl TypeEnv for Table {
        fn record_fields(&self, alias: &str) -> Option<Vec<(String, ValueType)>> {
            self.records.get(alias).cloned()
        }
        fn enum_names(&self, alias: &str) -> Option<Vec<String>> {
            self.enums.get(alias).map(|(n, _)| n.clone())
        }
        fn enum_underlying(&self, alias: &str) -> Option<SceType> {
            self.enums.get(alias).map(|(_, t)| t.clone())
        }
    }

    fn ty(text: &str) -> ValueType {
        WrittenType::read(text).unwrap().bounded().unwrap()
    }

    /// The tree's own batch item (docs/adr/0086, section 14.1).
    fn table() -> Table {
        let mut table = Table::default();
        table.enums.insert(
            "SignalLevel".into(),
            (
                vec!["low".into(), "medium".into(), "high".into()],
                SceType::Uint8,
            ),
        );
        table.records.insert(
            "Item".into(),
            vec![
                ("id".into(), ty("uint32")),
                ("amount".into(), ty("int64")),
                ("label".into(), ty("string[8]")),
                ("level".into(), ty("enum:SignalLevel")),
                ("due".into(), ty("optional<int64>")),
                ("blob".into(), ty("bytes[4]")),
                ("tags".into(), ty("list<string[8]>[4]")),
            ],
        );
        table
    }

    #[test]
    fn each_constructor_has_the_size_the_table_states() {
        let env = table();
        let both = |text: &str| {
            let t = ty(text);
            (
                json_max_len(&t, &env).unwrap(),
                cbor_max_len(&t, &env).unwrap(),
            )
        };
        assert_eq!(both("uint8"), (3, 2));
        assert_eq!(both("uint16"), (5, 3));
        assert_eq!(both("uint32"), (10, 5));
        assert_eq!(both("int8"), (4, 2));
        assert_eq!(both("int16"), (6, 3));
        assert_eq!(both("int32"), (11, 5));
        assert_eq!(both("uint64"), (22, 9));
        assert_eq!(both("int64"), (22, 9));
        assert_eq!(both("float32"), (25, 5));
        assert_eq!(both("float64"), (25, 9));
        assert_eq!(both("bool"), (5, 1));
        // "medium" is the longest name; the enum is a uint8 on CBOR.
        assert_eq!(both("enum:SignalLevel"), (8, 2));
        assert_eq!(both("string[8]"), (2 + 48, 1 + 8));
        assert_eq!(both("bytes[24]"), (2 + 144, 2 + 24));
        assert_eq!(both("optional<bool>"), (5, 1));
        assert_eq!(both("optional<uint8>"), (4, 2));
        assert_eq!(both("list<uint8>[3]"), (2 + 9 + 2, 1 + 6));
        assert_eq!(both("list<list<uint8>[2]>[2]"), (2 + 2 * 9 + 1, 1 + 2 * 5));
    }

    #[test]
    fn a_cbor_head_is_as_long_as_its_argument_needs() {
        for (n, len) in [
            (0, 1),
            (23, 1),
            (24, 2),
            (255, 2),
            (256, 3),
            (65_535, 3),
            (65_536, 5),
            (u64::from(u32::MAX), 5),
            (u64::from(u32::MAX) + 1, 9),
            (u64::MAX, 9),
        ] {
            assert_eq!(cbor_head_len(n), len, "{n}");
        }
    }

    // An independent writer of the largest value of a type: each runtime's
    // writer spells a value so, and a size that is only an upper bound would
    // differ from its length (docs/adr/0086, section 2.3).

    fn maximal_json(t: &ValueType, env: &Table) -> String {
        match t {
            ValueType::Scalar(s) => match s {
                SceType::Uint8 => "255".into(),
                SceType::Uint16 => "65535".into(),
                SceType::Uint32 => "4294967295".into(),
                SceType::Int8 => "-128".into(),
                SceType::Int16 => "-32768".into(),
                SceType::Int32 => "-2147483648".into(),
                SceType::Uint64 => "\"18446744073709551615\"".into(),
                SceType::Int64 => "\"-9223372036854775808\"".into(),
                SceType::Float32 => small_band_text(f64::from(-f32::from_bits(0x35a5_b36d))),
                SceType::Float64 => small_band_text(-f64::from_bits(0x3eb4_b66d_c01e_c6fb)),
                SceType::Bool => "false".into(),
                SceType::Enum(r) => {
                    let names = &env.enums[&r.alias].0;
                    let longest = names.iter().max_by_key(|n| n.len()).unwrap();
                    format!("\"{longest}\"")
                }
                SceType::String | SceType::Bytes => unreachable!("never a scalar"),
            },
            ValueType::Text { max } | ValueType::Bytes { max } => {
                format!("\"{}\"", "\\u0001".repeat(max.max as usize))
            }
            ValueType::Optional { elem } => {
                let present = maximal_json(elem, env);
                if present.len() >= 4 {
                    present
                } else {
                    "null".into()
                }
            }
            ValueType::List { elem, max } => {
                let one = maximal_json(elem, env);
                format!("[{}]", vec![one; max.max as usize].join(","))
            }
            ValueType::Record { alias } => {
                let members: Vec<String> = env.records[alias]
                    .iter()
                    .map(|(id, f)| format!("\"{id}\":{}", maximal_json(f, env)))
                    .collect();
                format!("{{{}}}", members.join(","))
            }
        }
    }

    /// A negative double between -1e-5 and -1e-6 as ECMAScript's
    /// Number::toString spells it: `-0.00000` and then its shortest digits.
    fn small_band_text(x: f64) -> String {
        let scientific = format!("{:e}", -x);
        let (mantissa, exponent) = scientific.split_once('e').unwrap();
        assert_eq!(exponent, "-6", "{x} is in the band from 1e-6 to 1e-5");
        let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
        assert_eq!(digits.len(), 17, "{x} needs seventeen digits");
        format!("-0.00000{digits}")
    }

    #[test]
    fn a_number_is_spelled_at_most_as_long_as_its_size() {
        // The longest spelling of each of ECMAScript's four notations, for a
        // negative value of at most seventeen digits (ARCHITECTURE.md, "JSON
        // Number Text"): a whole value below 1e21, a fraction above 1, a
        // fraction in the band from 1e-6 to 1, and `d.ddde-nnn`.
        let digits = 17;
        let whole = 1 + 21;
        let above_one = 1 + digits + 1;
        let small_band = 1 + "0.00000".len() + digits;
        let exponent_double = 1 + 1 + 1 + (digits - 1) + 2 + 3;
        let exponent_single = 1 + 1 + 1 + (digits - 1) + 2 + 2;
        let env = table();
        for (text, exponent) in [("float64", exponent_double), ("float32", exponent_single)] {
            let longest = [whole, above_one, small_band, exponent]
                .into_iter()
                .max()
                .unwrap();
            assert_eq!(
                json_max_len(&ty(text), &env).unwrap(),
                longest as u64,
                "{text}"
            );
        }
        assert_eq!(small_band, 25);
    }

    fn cbor_head(major: u8, n: u64) -> Vec<u8> {
        let m = major << 5;
        match n {
            0..=23 => vec![m | n as u8],
            24..=0xff => vec![m | 24, n as u8],
            0x100..=0xffff => {
                let mut v = vec![m | 25];
                v.extend((n as u16).to_be_bytes());
                v
            }
            0x1_0000..=0xffff_ffff => {
                let mut v = vec![m | 26];
                v.extend((n as u32).to_be_bytes());
                v
            }
            _ => {
                let mut v = vec![m | 27];
                v.extend(n.to_be_bytes());
                v
            }
        }
    }

    fn maximal_cbor(t: &ValueType, env: &Table) -> Vec<u8> {
        match t {
            ValueType::Scalar(s) => match s {
                SceType::Uint8 => cbor_head(0, 255),
                SceType::Uint16 => cbor_head(0, 65_535),
                SceType::Uint32 => cbor_head(0, u64::from(u32::MAX)),
                SceType::Uint64 => cbor_head(0, u64::MAX),
                SceType::Int8 => cbor_head(1, 127),
                SceType::Int16 => cbor_head(1, 32_767),
                SceType::Int32 => cbor_head(1, 2_147_483_647),
                SceType::Int64 => cbor_head(1, i64::MAX as u64),
                // A single no half holds; a double no single holds.
                SceType::Float32 => {
                    let mut v = vec![0xfa];
                    v.extend(0.1f32.to_be_bytes());
                    v
                }
                SceType::Float64 => {
                    let mut v = vec![0xfb];
                    v.extend(0.1f64.to_be_bytes());
                    v
                }
                SceType::Bool => vec![0xf4],
                SceType::Enum(r) => {
                    maximal_cbor(&ValueType::Scalar(env.enums[&r.alias].1.clone()), env)
                }
                SceType::String | SceType::Bytes => unreachable!("never a scalar"),
            },
            ValueType::Text { max } => {
                let mut v = cbor_head(3, u64::from(max.max));
                v.resize(v.len() + max.max as usize, b'a');
                v
            }
            ValueType::Bytes { max } => {
                let mut v = cbor_head(2, u64::from(max.max));
                v.resize(v.len() + max.max as usize, 0xff);
                v
            }
            ValueType::Optional { elem } => {
                let present = maximal_cbor(elem, env);
                if present.is_empty() {
                    vec![0xf6]
                } else {
                    present
                }
            }
            ValueType::List { elem, max } => {
                let mut v = cbor_head(4, u64::from(max.max));
                for _ in 0..max.max {
                    v.extend(maximal_cbor(elem, env));
                }
                v
            }
            ValueType::Record { alias } => {
                let fields = &env.records[alias];
                let mut v = cbor_head(5, fields.len() as u64);
                for (id, f) in fields {
                    v.extend(cbor_head(3, id.len() as u64));
                    v.extend(id.as_bytes());
                    v.extend(maximal_cbor(f, env));
                }
                v
            }
        }
    }

    #[test]
    fn the_sizes_are_the_lengths_of_a_maximal_value() {
        let env = table();
        for text in [
            "uint8",
            "int16",
            "uint64",
            "float32",
            "float64",
            "bool",
            "enum:SignalLevel",
            "string[8]",
            "bytes[300]",
            "optional<int64>",
            "optional<bool>",
            "record:Item",
            "list<record:Item>[3]",
            "list<list<uint8>[2]>[2]",
            "list<enum:SignalLevel>[2]",
            "list<optional<uint16>>[2]",
            "optional<list<uint8>[2]>",
            "list<list<string[8]>[4]>[3]",
            "list<string[30]>[24]",
        ] {
            let t = ty(text);
            assert_eq!(
                json_max_len(&t, &env).unwrap(),
                maximal_json(&t, &env).len() as u64,
                "J({text})"
            );
            assert_eq!(
                cbor_max_len(&t, &env).unwrap(),
                maximal_cbor(&t, &env).len() as u64,
                "C({text})"
            );
        }
    }

    #[test]
    fn a_payload_is_sized_as_the_record_of_its_fields() {
        let env = table();
        let fields = env.records["Item"].clone();
        assert_eq!(
            payload_json_max_len(&fields, &env).unwrap(),
            json_max_len(&ty("record:Item"), &env).unwrap()
        );
        assert_eq!(payload_json_max_len(&[], &env).unwrap(), 2);
    }

    #[test]
    fn a_record_that_holds_itself_has_no_size() {
        let mut env = table();
        env.records.insert(
            "Node".into(),
            vec![("next".into(), ty("list<record:Link>[2]"))],
        );
        env.records.insert(
            "Link".into(),
            vec![("to".into(), ty("optional<record:Node>"))],
        );
        assert_eq!(
            contains_itself("Node", &env),
            Some(vec!["Node".to_string(), "Link".into(), "Node".into()])
        );
        assert_eq!(
            json_max_len(&ty("record:Node"), &env),
            Err(SizeError::ContainsItself {
                cycle: vec!["Node".into(), "Link".into(), "Node".into()]
            })
        );
        assert_eq!(contains_itself("Item", &env), None);
    }

    #[test]
    fn an_alias_no_table_resolves_has_no_size() {
        let env = table();
        assert_eq!(
            json_max_len(&ty("record:Nope"), &env),
            Err(SizeError::Unresolved {
                alias: "Nope".into()
            })
        );
        assert_eq!(
            cbor_max_len(&ty("enum:Nope"), &env),
            Err(SizeError::Unresolved {
                alias: "Nope".into()
            })
        );
    }

    #[test]
    fn a_size_past_64_bits_is_refused_not_wrapped() {
        let env = table();
        let huge = ty("list<list<list<string[4294967295]>[4294967295]>[4294967295]>[4294967295]");
        assert_eq!(json_max_len(&huge, &env), Err(SizeError::Overflow));
        assert_eq!(cbor_max_len(&huge, &env), Err(SizeError::Overflow));
    }

    // ── "fits", held to the values each type holds ────────────────────────

    /// The documents behind the aliases: `R` and `R2` are two revisions of one
    /// schema and `Q` another schema; `E` and `E2` one enum and `F` another.
    struct Documents;

    fn document_of(alias: &str) -> &str {
        alias.trim_end_matches('2')
    }

    impl TypeIdentity for Documents {
        fn same_record(&self, from: &str, to: &str) -> bool {
            document_of(from) == document_of(to)
        }
        fn same_enum(&self, from: &str, to: &str) -> bool {
            document_of(from) == document_of(to)
        }
    }

    /// Every value of `t`, each spelled once, independent of the relation: a
    /// number or `bool` is one value standing for every value of its type, a
    /// record or an enum one value of its document, a text or a byte string one
    /// value per length, an absent optional `null` beside the values of its
    /// element (the wire spells a present one as its element), and a list every
    /// sequence of values of its element up to its bound.
    fn values_of(t: &ValueType) -> BTreeSet<String> {
        match t {
            ValueType::Scalar(SceType::Enum(r)) => {
                [format!("enum:{}", document_of(&r.alias))].into()
            }
            ValueType::Scalar(s) => [s.as_attr()].into(),
            ValueType::Record { alias } => [format!("record:{}", document_of(alias))].into(),
            ValueType::Text { max } => (0..=max.max).map(|n| format!("s{n}")).collect(),
            ValueType::Bytes { max } => (0..=max.max).map(|n| format!("b{n}")).collect(),
            ValueType::Optional { elem } => {
                let mut out = values_of(elem);
                out.insert("null".into());
                out
            }
            ValueType::List { elem, max } => {
                let elems: Vec<String> = values_of(elem).into_iter().collect();
                let mut sequences: Vec<Vec<String>> = vec![Vec::new()];
                let mut out: BTreeSet<String> = ["[]".to_string()].into();
                for _ in 0..max.max {
                    sequences = sequences
                        .iter()
                        .flat_map(|seq| {
                            elems.iter().map(move |e| {
                                let mut longer = seq.clone();
                                longer.push(e.clone());
                                longer
                            })
                        })
                        .collect();
                    out.extend(sequences.iter().map(|seq| format!("[{}]", seq.join(","))));
                }
                out
            }
        }
    }

    /// Every type up to three constructors deep over leaves that tell each rule
    /// apart: two bounds of each text and byte string and list, two revisions of
    /// a record and an enum beside another of each, and numbers of two widths.
    fn fit_universe() -> Vec<String> {
        let leaves = [
            "uint8",
            "int64",
            "bool",
            "enum:E",
            "enum:E2",
            "enum:F",
            "record:R",
            "record:R2",
            "record:Q",
            "string[1]",
            "string[2]",
            "bytes[1]",
            "bytes[2]",
        ];
        let mut all: Vec<String> = leaves.iter().map(|l| l.to_string()).collect();
        let mut previous = all.clone();
        for _ in 2..=3 {
            let mut next = Vec::new();
            for inner in &previous {
                if !inner.starts_with("optional<") {
                    next.push(format!("optional<{inner}>"));
                }
                next.push(format!("list<{inner}>[1]"));
                next.push(format!("list<{inner}>[2]"));
            }
            all.extend(next.iter().cloned());
            previous = next;
        }
        all
    }

    #[test]
    fn a_type_fits_another_exactly_when_its_values_are_values_of_the_other() {
        let universe: Vec<(String, ValueType)> = fit_universe()
            .into_iter()
            .map(|text| {
                let t = ty(&text);
                (text, t)
            })
            .collect();
        let values: Vec<BTreeSet<String>> = universe.iter().map(|(_, t)| values_of(t)).collect();
        let mut reasons = BTreeSet::new();
        let (mut fitting, mut misfitting) = (0usize, 0usize);
        for (i, (from_text, from)) in universe.iter().enumerate() {
            for (j, (to_text, to)) in universe.iter().enumerate() {
                let oracle = values[i].is_subset(&values[j]);
                match fits(from, to, &Documents) {
                    Ok(()) => {
                        assert!(
                            oracle,
                            "{from_text} fits {to_text}, and a value of it is none of the other's"
                        );
                        fitting += 1;
                    }
                    Err(misfit) => {
                        assert!(!oracle, "{from_text} does not fit {to_text} ({misfit:?}), and every value of it is one of the other's");
                        reasons.insert(format!("{:?}", misfit.why));
                        misfitting += 1;
                    }
                }
            }
        }
        assert_eq!(universe.len(), 13 + 39 + 104);
        assert!(fitting > universe.len(), "{fitting} pairs fit");
        assert!(misfitting > fitting, "{misfitting} pairs do not fit");
        assert_eq!(
            reasons.into_iter().collect::<Vec<_>>(),
            ["Bound", "Constructor", "Identity", "MayBeAbsent"]
        );
    }

    #[test]
    fn a_misfit_names_the_first_place_by_its_path() {
        let refused = |from: &str, to: &str| fits(&ty(from), &ty(to), &Documents).unwrap_err();
        let at = refused("list<list<string[9]>[4]>[3]", "list<list<string[8]>[4]>[3]");
        assert_eq!(
            (at.path.as_str(), at.from.as_str(), at.to.as_str(), at.why),
            ("/*/*", "string[9]", "string[8]", MisfitReason::Bound)
        );
        assert_eq!(
            at.sentence("items"),
            "'items/*/*' is a string[9], which may hold more than the string[8] expected there"
        );
        // The elements are judged before the bound of the list that holds them.
        let at = refused("list<optional<uint16>>[2]", "list<uint16>[1]");
        assert_eq!(
            (at.path.as_str(), at.why),
            ("/*", MisfitReason::MayBeAbsent)
        );
        let at = refused("list<uint8>[3]", "list<uint8>[2]");
        assert_eq!((at.path.as_str(), at.why), ("", MisfitReason::Bound));
        // An optional is passed through: its present value is its element's.
        let at = refused("optional<list<record:Q>[2]>", "optional<list<record:R>[2]>");
        assert_eq!(
            (at.path.as_str(), at.from.as_str(), at.why),
            ("/*", "record:Q", MisfitReason::Identity)
        );
        assert_eq!(
            fits(
                &ty("list<record:R2>[2]"),
                &ty("optional<list<record:R>[3]>"),
                &Documents
            ),
            Ok(())
        );
    }

    #[test]
    fn an_escaped_text_is_as_long_as_json_spells_it() {
        assert_eq!(json_escaped_len("id"), 2);
        assert_eq!(json_escaped_len("a\"b"), 4);
        assert_eq!(json_escaped_len("\n"), 2);
        assert_eq!(json_escaped_len("\u{1}"), 6);
        assert_eq!(json_escaped_len("\u{e9}"), 2);
    }
}
