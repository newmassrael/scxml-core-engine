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
//!     | union:<alias>
//!     | list<T>[B]
//! B ::= <positive integer> | <limit name>=<positive integer>
//! ```
//!
//! A record is a product, holding every one of its fields; a union is a sum,
//! holding exactly one of the cases its document declares, told apart by a tag
//! of a closed enum (docs/adr/0092, section 1). Each names its document by an
//! alias, and its shape is read from that document through [`TypeEnv`].
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
//! By the same recursion it has a grammar, in two forms: a JSON Schema
//! ([`json_schema_of`]) and a GBNF grammar ([`gbnf_of`]), docs/adr/0087
//! section 1.

use std::fmt;

use serde::Serialize;

use crate::forge::model::{EntryType, SceType};

/// A bound written in a type: the most elements of a list, or the most bytes of
/// a text or a byte string. `limit` is the name a build may give another number
/// (`[name=N]`, docs/adr/0050); `max` is the number the build holds, the one
/// written until a build resolves the name ([`WrittenType::resolve_limits`]).
///
/// `min` is the fewest elements of a list, its lower count (`[M..N]`,
/// docs/adr/0086 section 15): 0 where none is written, and on every text and
/// byte string, whose bracket takes none. A name moves `max` alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    pub max: u32,
    pub limit: Option<String>,
    pub min: u32,
}

impl Bound {
    /// A bound no name carries.
    pub fn of(max: u32) -> Self {
        Self {
            max,
            limit: None,
            min: 0,
        }
    }

    /// A bound a build may move by `limit`.
    pub fn named(limit: &str, max: u32) -> Self {
        Self {
            max,
            limit: Some(limit.to_string()),
            min: 0,
        }
    }

    /// This bound with the lower count `min`: a list of at least `min`
    /// elements.
    pub fn at_least(self, min: u32) -> Self {
        Self { min, ..self }
    }
}

impl fmt::Display for Bound {
    /// The bracket as the grammar spells it: `[8]` or `[name=8]`, the lower
    /// count before `..` where one is stated (`[3..8]`, `[3..name=8]`). A
    /// lower count of zero is not printed: `[0..8]` and `[8]` are one bound.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        if self.min > 0 {
            write!(f, "{}..", self.min)?;
        }
        match &self.limit {
            Some(name) => write!(f, "{name}={}]", self.max),
            None => write!(f, "{}]", self.max),
        }
    }
}

/// A bound crosses the forge-AST as the number the build holds: the name a
/// build may move it by is the document's, not the IR's, as a field's
/// `sce:limit` is (docs/adr/0050), so a bound is one shape whichever spelling
/// the document wrote it in (docs/adr/0086, section 13).
impl Serialize for Bound {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.max)
    }
}

#[cfg(test)]
impl schemars::JsonSchema for Bound {
    fn schema_name() -> String {
        "Bound".to_string()
    }

    fn is_referenceable() -> bool {
        false
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <u32 as schemars::JsonSchema>::json_schema(generator)
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
///
/// Serialized as the model's other types are: a scalar is the bare string it
/// always was (`"int64"`), and each other constructor an object keyed by its
/// own name, its bound beside it: `{"string": 8}`, `{"bytes": 4}`,
/// `{"optional": "int64"}`, `{"record": "Item"}`, `{"union": "Shape"}`,
/// `{"list": "uint8", "max": 2}`. A text, byte string or list whose bound is
/// not written (an algorithm's parameter, whose outermost bound is its
/// argument's) is the shape it had before a type held bounds: `"string"`,
/// `"bytes"`, `{"list": "uint8"}`.
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
    /// `union:<alias>`: a value of exactly one case of the union the aliased
    /// document declares, its tag naming which (docs/adr/0092, section 1).
    /// Like a record it states no bound of its own: its largest value is its
    /// largest case.
    Union { alias: String },
    /// `list<T>[B]`: an ordered sequence of at most `max` values of `elem`.
    List { elem: Box<TypeOf<B>>, max: B },
}

impl<B: BoundSlot> Serialize for TypeOf<B> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        fn keyed<S: serde::Serializer>(
            serializer: S,
            key: &str,
            bound: &Bound,
        ) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(Some(1))?;
            map.serialize_entry(key, bound)?;
            map.end()
        }
        match self {
            Self::Scalar(t) => t.serialize(serializer),
            Self::Text { max } => match max.stated() {
                Some(bound) => keyed(serializer, "string", bound),
                None => SceType::String.serialize(serializer),
            },
            Self::Bytes { max } => match max.stated() {
                Some(bound) => keyed(serializer, "bytes", bound),
                None => SceType::Bytes.serialize(serializer),
            },
            Self::Optional { elem } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("optional", &**elem)?;
                map.end()
            }
            Self::Record { alias } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("record", alias)?;
                map.end()
            }
            // docs/adr/0092, section 1.1, item 4: a type that holds no union
            // exports as it always did.
            Self::Union { alias } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("union", alias)?;
                map.end()
            }
            Self::List { elem, max } => {
                let stated = max.stated();
                // The lower count beside the bound, where one is stated
                // (docs/adr/0086, section 15.3): a list that states none is
                // the shape it always was.
                let least = stated.map_or(0, |bound| bound.min);
                let entries = 1 + usize::from(stated.is_some()) + usize::from(least > 0);
                let mut map = serializer.serialize_map(Some(entries))?;
                map.serialize_entry("list", &**elem)?;
                if let Some(bound) = stated {
                    map.serialize_entry("max", bound)?;
                }
                if least > 0 {
                    map.serialize_entry("min", &least)?;
                }
                map.end()
            }
        }
    }
}

/// The schema of the shape [`TypeOf`]'s `Serialize` writes: one definition
/// for a [`ValueType`], whose every bound is required, and one for a
/// [`WrittenType`], whose bounds may be left out.
#[cfg(test)]
impl<B: BoundSlot> schemars::JsonSchema for TypeOf<B> {
    fn schema_name() -> String {
        B::SCHEMA_NAME.to_string()
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let scalar = generator.subschema_for::<SceType>();
        let this = generator.subschema_for::<Self>();
        // A bound, and a list's lower count: one of zero is not written, so one
        // that is written is at least 1 too.
        let bound = serde_json::json!({"type": "integer", "format": "uint32", "minimum": 1});
        let list_required = if B::REQUIRED {
            serde_json::json!(["list", "max"])
        } else {
            serde_json::json!(["list"])
        };
        serde_json::from_value(serde_json::json!({
            "description": "A type of the one type grammar (docs/adr/0086, section 1): a \
                            scalar's bare name, or an object keyed by its constructor, its \
                            bound beside it.",
            "anyOf": [
                scalar,
                {"type": "object", "required": ["string"], "properties": {"string": bound}},
                {"type": "object", "required": ["bytes"], "properties": {"bytes": bound}},
                {"type": "object", "required": ["optional"], "properties": {"optional": this}},
                {"type": "object", "required": ["record"], "properties": {"record": {"type": "string"}}},
                {"type": "object", "required": ["union"], "properties": {"union": {"type": "string"}}},
                {
                    "type": "object",
                    "required": list_required,
                    "properties": {"list": this, "max": bound, "min": bound}
                }
            ]
        }))
        .expect("the schema of a type is a JSON schema")
    }
}

/// A type whose every bound is stated: what a position holds.
pub type ValueType = TypeOf<Bound>;

/// A type as its author wrote it: a bound may be left to an attribute.
pub type WrittenType = TypeOf<Option<Bound>>;

/// What stands where a bound is written: a stated bound, or one that may be
/// absent. The one reading the printer needs of either.
pub trait BoundSlot {
    /// The name of the schema of a type of this kind.
    #[cfg(test)]
    const SCHEMA_NAME: &'static str;
    /// Whether a type of this kind states every bound.
    #[cfg(test)]
    const REQUIRED: bool;
    /// The bound written there, if one is.
    fn stated(&self) -> Option<&Bound>;
}

impl BoundSlot for Bound {
    #[cfg(test)]
    const SCHEMA_NAME: &'static str = "ValueType";
    #[cfg(test)]
    const REQUIRED: bool = true;
    fn stated(&self) -> Option<&Bound> {
        Some(self)
    }
}

impl BoundSlot for Option<Bound> {
    #[cfg(test)]
    const SCHEMA_NAME: &'static str = "WrittenType";
    #[cfg(test)]
    const REQUIRED: bool = false;
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
            Self::Union { alias } => write!(f, "union:{alias}"),
            Self::List { elem, max } => {
                write!(f, "list<{elem}>")?;
                bracket(f, max)
            }
        }
    }
}

impl<B> TypeOf<B> {
    /// The name of the constructor at the top of the type, as a refusal names
    /// it: `list`, `optional`, `record`, `union`, `string`, `bytes`, or the
    /// scalar's own spelling.
    pub fn constructor(&self) -> String {
        match self {
            Self::Scalar(t) => t.as_attr(),
            Self::Text { .. } => "string".to_string(),
            Self::Bytes { .. } => "bytes".to_string(),
            Self::Optional { .. } => "optional".to_string(),
            Self::Record { .. } => "record".to_string(),
            Self::Union { .. } => "union".to_string(),
            Self::List { .. } => "list".to_string(),
        }
    }

    /// The aliases of every union the type names, in the order they are
    /// written, at every depth. The records a union's cases hold are its own
    /// document's, and are not among them.
    pub fn union_aliases(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.walk(&mut |ty| {
            if let Self::Union { alias } = ty {
                out.push(alias.as_str());
            }
        });
        out
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
            Self::Scalar(_)
            | Self::Text { .. }
            | Self::Bytes { .. }
            | Self::Record { .. }
            | Self::Union { .. } => {}
        }
    }

    /// How deep the type nests: 1 for a type of no element, one more for each
    /// `optional<…>` and `list<…>` around it.
    pub fn depth(&self) -> usize {
        match self {
            Self::Optional { elem } | Self::List { elem, .. } => 1 + elem.depth(),
            Self::Scalar(_)
            | Self::Text { .. }
            | Self::Bytes { .. }
            | Self::Record { .. }
            | Self::Union { .. } => 1,
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
    /// record, a union or an optional.
    BoundOnUnboundable { place: String },
    /// `optional<optional<T>>`.
    NestedOptional { place: String },
    /// A text, byte string or list with no bound.
    Unbounded { place: String },
    /// The outermost type carries a bracket and an attribute bounds it too.
    BothForms { place: String },
    /// An attribute states a bound beside a type that has none to state.
    NothingToBound { place: String },
    /// A list's lower count past its bound, as written or after a build moved
    /// the bound (docs/adr/0086, section 15.2): the type has no value.
    CountPastBound {
        place: String,
        least: u32,
        most: u32,
    },
    /// A lower count where nothing reads it (docs/adr/0086, sections 15.2,
    /// 15.7 and 15.8): in the bracket of a text or a byte string, or in the
    /// type of a place that is filled an element at a time. `place` is the
    /// part of the type that states it; `why` says where it stands.
    CountMisplaced { place: String, why: CountPlace },
}

/// Where a lower count stands that nothing reads (docs/adr/0086, section 15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountPlace {
    /// In the bracket of a `string` or `bytes`, which counts bytes from above
    /// only.
    Length,
    /// In the type of a variable or an algorithm's slot, which starts empty
    /// and is filled an element at a time.
    Built,
    /// On a list a later revision of its schema adds, which a delivery that
    /// lacks it reads as the empty list.
    Added,
}

impl TypeRefusal {
    /// The rule the type breaks, as a refusal's sentence states it.
    pub fn rule(&self) -> String {
        match self {
            Self::Unknown { written } => format!(
                "`{written}` names no type: a type is a number (uint8 to uint64, int8 to \
                 int64, float32, float64), bool, enum:<alias>, string[N], bytes[N], \
                 optional<T>, record:<alias>, union:<alias> or list<T>[N]"
            ),
            Self::Malformed { place, expected } => {
                format!("the type is not complete after `{place}`: {expected}")
            }
            Self::BadBound { place, written } => format!(
                "the bound `[{written}]` of `{place}` is a whole number of at least one that \
                 fits 32 bits, or `name=N` with such a number; a list's may follow its lower \
                 count, a whole number of zero or more, as `M..N`"
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
            Self::CountPastBound { place, least, most } => format!(
                "the lower count {least} of `{place}` is past its bound {most}: a list holds \
                 at least its lower count and at most its bound, so the lower count is at \
                 most the bound, or the type has no value"
            ),
            Self::CountMisplaced { place, why } => match why {
                CountPlace::Length => format!(
                    "`{place}` takes no lower count: a string's or a bytes' bracket is the most \
                     bytes it holds, and only a list states the fewest elements it holds \
                     (`list<T>[M..N]`)"
                ),
                CountPlace::Built => format!(
                    "`{place}` states a lower count, and a variable or an algorithm's slot \
                     starts empty and is filled an element at a time, so no lower count holds \
                     of it until it is full: its type states none (`list<T>[N]`)"
                ),
                CountPlace::Added => format!(
                    "`{place}` states a lower count, and a list a later revision adds reads \
                     as the empty list from a delivery that lacks it: it states none \
                     (`list<T>[N]`), or is declared optional<…> and reads absent"
                ),
            },
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
            Self::Scalar(_) | Self::Record { .. } | Self::Union { .. } => None,
        }
    }

    /// The outermost text, byte string or list of the type, seen through every
    /// `optional<…>`, or `None` for a type with no length: the type a legacy
    /// attribute bounds, and so which attribute a position reads for it.
    pub fn outermost_bounded(&self) -> Option<&Self> {
        match self {
            Self::Text { .. } | Self::Bytes { .. } | Self::List { .. } => Some(self),
            Self::Optional { elem } => elem.outermost_bounded(),
            Self::Scalar(_) | Self::Record { .. } | Self::Union { .. } => None,
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
            Self::Scalar(_) | Self::Record { .. } | Self::Union { .. } => {
                Err(TypeRefusal::NothingToBound {
                    place: place.to_string(),
                })
            }
        }
    }

    /// The type with every bound stated, or the refusal of the outermost text,
    /// byte string or list that states none, or of the outermost list whose
    /// lower count is past its bound, which a build that moved the bound below
    /// the lower count leaves (docs/adr/0086, section 15.2).
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
            Self::Union { alias } => ValueType::Union { alias },
            Self::Optional { elem } => ValueType::Optional {
                elem: Box::new(elem.bounded()?),
            },
            Self::List { elem, max } => {
                let max = required(max)?;
                if max.min > max.max {
                    return Err(TypeRefusal::CountPastBound {
                        place: place.clone(),
                        least: max.min,
                        most: max.max,
                    });
                }
                ValueType::List {
                    elem: Box::new(elem.bounded()?),
                    max,
                }
            }
        })
    }

    /// The refusal of the outermost list of the type, at any depth, that states
    /// a lower count, or `None`: the check of a place that is filled an element
    /// at a time, a variable or an algorithm's slot, whose own type states none
    /// (docs/adr/0086, section 15.7).
    pub fn lower_count_inside(&self) -> Option<TypeRefusal> {
        let mut found = None;
        self.walk(&mut |ty| {
            if let Self::List {
                max: Some(bound), ..
            } = ty
            {
                if bound.min > 0 && found.is_none() {
                    found = Some(TypeRefusal::CountMisplaced {
                        place: ty.to_string(),
                        why: CountPlace::Built,
                    });
                }
            }
        });
        found
    }

    /// The refusal [`Self::bounded`] would give for a bound left out below the
    /// outermost bounded type, or `None`: the check of a position whose
    /// outermost bound is its argument's (an algorithm's parameter), and so is
    /// not written, while every bound inside it is.
    pub fn unbounded_inside(&self) -> Option<TypeRefusal> {
        match self {
            Self::Scalar(_)
            | Self::Text { .. }
            | Self::Bytes { .. }
            | Self::Record { .. }
            | Self::Union { .. } => None,
            Self::Optional { elem } => elem.unbounded_inside(),
            Self::List { elem, .. } => (**elem).clone().bounded().err(),
        }
    }

    /// The same type with the number of every named bound replaced by the one
    /// `resolve` gives its name: what a build that moves a limit holds
    /// (`crate::limits::resolve`). A bound no name carries is kept.
    pub fn resolve_limits(self, resolve: &mut dyn FnMut(&str, u32) -> u32) -> Self {
        // A name moves the upper bound alone; a list's lower count is a
        // literal of the type (docs/adr/0086, section 15.2).
        let mut bound = |max: Option<Bound>| {
            max.map(|b| {
                let max = match b.limit.as_deref() {
                    Some(name) => resolve(name, b.max),
                    None => b.max,
                };
                Bound { max, ..b }
            })
        };
        match self {
            Self::Scalar(t) => Self::Scalar(t),
            Self::Record { alias } => Self::Record { alias },
            Self::Union { alias } => Self::Union { alias },
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

/// The scalar a leaf of a value type is read as, with the bound of a text or a
/// byte string, or `None` for a type that is not one leaf.
fn value_leaf(ty: &ValueType) -> Option<(SceType, Option<&Bound>)> {
    match ty {
        ValueType::Scalar(t) => Some((t.clone(), None)),
        ValueType::Text { max } => Some((SceType::String, Some(max))),
        ValueType::Bytes { max } => Some((SceType::Bytes, Some(max))),
        ValueType::Optional { .. }
        | ValueType::Record { .. }
        | ValueType::Union { .. }
        | ValueType::List { .. } => None,
    }
}

impl ValueType {
    /// The type seen as one entry: a scalar, a text or a byte string, or an
    /// optional one, with the bound of its text or byte string. The leaf view
    /// a lowering that holds no list, record or nested optional reads an event
    /// schema's field through; any other type is a composition, named so that
    /// such a lowering refuses it by name (docs/adr/0086, section 7.3).
    pub fn entry(&self) -> Result<(EntryType, Option<&Bound>), Composition> {
        if let Some((t, max)) = value_leaf(self) {
            return Ok((EntryType::Scalar(t), max));
        }
        match self {
            Self::Optional { elem } => value_leaf(elem)
                .map(|(t, max)| (EntryType::Optional { elem: t }, max))
                .ok_or_else(|| Composition::of(&**elem)),
            Self::Scalar(_)
            | Self::Text { .. }
            | Self::Bytes { .. }
            | Self::Record { .. }
            | Self::Union { .. }
            | Self::List { .. } => Err(Composition::of(self)),
        }
    }

    /// Whether a value of the type may be absent anywhere inside it: the type
    /// is `optional<…>` or holds one at some depth. What a unit that declares
    /// the backend's optional type asks.
    pub fn holds_optional(&self) -> bool {
        let mut found = false;
        self.walk(&mut |ty| found |= matches!(ty, Self::Optional { .. }));
        found
    }

    /// The enum a value of the type holds at its one leaf, seen through every
    /// `optional<…>` and `list<…>` around it; `None` for a type whose leaf is
    /// no enum, or that names a record (whose enums are its own schema's). A
    /// type has one leaf: a record is named, never written out.
    pub fn enum_leaf(&self) -> Option<&crate::forge::model::EnumRef> {
        match self {
            Self::Scalar(SceType::Enum(reference)) => Some(reference),
            Self::Optional { elem } | Self::List { elem, .. } => elem.enum_leaf(),
            Self::Scalar(_)
            | Self::Text { .. }
            | Self::Bytes { .. }
            | Self::Record { .. }
            | Self::Union { .. } => None,
        }
    }

    /// [`Self::enum_leaf`], to be renamed in place.
    pub fn enum_leaf_mut(&mut self) -> Option<&mut crate::forge::model::EnumRef> {
        match self {
            Self::Scalar(SceType::Enum(reference)) => Some(reference),
            Self::Optional { elem } | Self::List { elem, .. } => elem.enum_leaf_mut(),
            Self::Scalar(_)
            | Self::Text { .. }
            | Self::Bytes { .. }
            | Self::Record { .. }
            | Self::Union { .. } => None,
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
            Self::Scalar(_) | Self::Record { .. } | Self::Union { .. } => None,
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
            Self::Union { alias } => WrittenType::Union {
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
            let max = read_bracket(cursor, start, Counted::Elements)?;
            return Ok(WrittenType::List {
                elem: Box::new(elem),
                max,
            });
        }
        "string" => {
            let max = read_bracket(cursor, start, Counted::Bytes)?;
            return Ok(WrittenType::Text { max });
        }
        "bytes" => {
            let max = read_bracket(cursor, start, Counted::Bytes)?;
            return Ok(WrittenType::Bytes { max });
        }
        "enum" | "record" | "union" => {
            if cursor.peek() != Some(':') {
                return Err(TypeRefusal::Malformed {
                    place: cursor.since(start),
                    expected: "an alias follows `:`, as in `enum:Level`, `record:Item` or \
                               `union:Shape`",
                });
            }
            cursor.pos += 1;
            let alias = cursor.take_while(is_word);
            if !is_alias(alias) {
                return Err(TypeRefusal::Unknown {
                    written: cursor.since(start),
                });
            }
            match word {
                "enum" => WrittenType::Scalar(SceType::Enum(crate::forge::model::EnumRef {
                    alias: alias.to_string(),
                })),
                "record" => WrittenType::Record {
                    alias: alias.to_string(),
                },
                _ => WrittenType::Union {
                    alias: alias.to_string(),
                },
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

/// What the bracket of a type counts: a list's elements, which it bounds from
/// below and above, or a text's or a byte string's bytes, which it bounds from
/// above only (docs/adr/0086, section 15.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Counted {
    Elements,
    Bytes,
}

/// The bracket after a type that takes one, or `None` when none is written.
///
/// A list's bracket may write its lower count before `..` (`[3..8]`,
/// `[3..name=8]`): a whole number of zero or more, at most the bound written.
/// A text before `..` that is no such number is read as the bracket always
/// was, so a limit name that holds `..` keeps its reading and a negative count
/// is refused as a malformed bound.
fn read_bracket(
    cursor: &mut Cursor<'_>,
    start: usize,
    counted: Counted,
) -> Result<Option<Bound>, TypeRefusal> {
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
    let digits = |text: &str| {
        let text = text.trim();
        !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
    };
    let whole = |text: &str| {
        if digits(text) {
            text.trim().parse::<u32>().ok()
        } else {
            None
        }
    };
    let number = |text: &str| whole(text).filter(|n| *n > 0);
    // The lower count, where the text before `..` is written in digits; one
    // past 32 bits is a malformed bound.
    let (least, upper) = match inside.split_once("..") {
        Some((lower, upper)) if digits(lower) => (Some(whole(lower).ok_or_else(refused)?), upper),
        _ => (None, inside),
    };
    let bound = match upper.split_once('=') {
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
            Bound::named(name, max)
        }
        None => Bound::of(number(upper).ok_or_else(refused)?),
    };
    let Some(least) = least else {
        return Ok(Some(bound));
    };
    if counted == Counted::Bytes {
        return Err(TypeRefusal::CountMisplaced {
            place: cursor.since(start),
            why: CountPlace::Length,
        });
    }
    if least > bound.max {
        return Err(TypeRefusal::CountPastBound {
            place: cursor.since(start),
            least,
            most: bound.max,
        });
    }
    Ok(Some(bound.at_least(least)))
}

/// `written`, the text of a type as a document spells it (`list&lt;uint8&gt;[1..3]`
/// in XML), with every lower count removed from its brackets
/// (`list&lt;uint8&gt;[3]`): the one type that keeps everything else the
/// declaration says, which a refusal of a lower count nothing reads offers as
/// its replacement (docs/adr/0086, section 15.13). A bracket whose text before
/// `..` is not written in digits is kept as it is, as the reader keeps it.
pub fn without_lower_counts(written: &str) -> String {
    let mut out = String::with_capacity(written.len());
    let mut rest = written;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..=open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            out.push_str(after);
            return out;
        };
        let inside = &after[..close];
        match inside.split_once("..") {
            Some((lower, upper))
                if !lower.trim().is_empty() && lower.trim().chars().all(|c| c.is_ascii_digit()) =>
            {
                out.push_str(upper.trim_start());
            }
            _ => out.push_str(inside),
        }
        out.push(']');
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
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
    /// The composition `ty` is, named by its whole text and its outermost
    /// constructor.
    pub fn of<B: BoundSlot>(ty: &TypeOf<B>) -> Self {
        Self {
            place: ty.to_string(),
            constructor: ty.constructor(),
        }
    }

    /// The sentence of the refusal of the composition where `element` declares
    /// it, in `position` (`an algorithm`, `a statechart's variable`): one
    /// wording for every place a slot's composition is refused by name.
    pub fn sentence(&self, element: &str, position: &str) -> String {
        format!(
            "{element}: `{}` in {position} is a type of the language, and no backend lowers \
             its {} there yet",
            self.place, self.constructor
        )
    }
}

/// The sentence of the refusal of `union:<alias>` by a walk of a type that
/// does not lower a union yet (docs/adr/0092, section 7): what each such walk
/// answers, one wording for all, as [`Composition::sentence`] is for a slot.
pub fn union_unlowered(alias: &str) -> String {
    format!(
        "`{}{alias}`: {}",
        WrittenType::UNION_PREFIX,
        crate::forge::union_check::NOT_LOWERED_YET
    )
}
// ── A slot: an algorithm's parameter, return and local, a statechart's variable ──

/// The scalar a text reads as through [`WrittenType::scalar`].
static TEXT_SCALAR: SceType = SceType::String;
/// The scalar a byte string reads as through [`WrittenType::scalar`].
static BYTES_SCALAR: SceType = SceType::Bytes;

impl WrittenType {
    /// The prefix of an optional type's spelling, `optional<T>`.
    pub const OPTIONAL_PREFIX: &'static str = "optional<";

    /// The prefix of a record type's spelling, `record:<alias>`.
    pub const RECORD_PREFIX: &'static str = "record:";

    /// The prefix of a union type's spelling, `union:<alias>`.
    pub const UNION_PREFIX: &'static str = "union:";

    /// The prefix of a list type's spelling, `list<T>`.
    pub const LIST_PREFIX: &'static str = "list<";

    /// What stands inside `optional<…>` in `text`, untrimmed, or `None` when
    /// `text` is not written as an optional type. The one reading of the
    /// spelling, so the parser, the statechart's rules and a codec's entry
    /// cannot disagree about what is written as one.
    pub fn optional_inner(text: &str) -> Option<&str> {
        text.trim()
            .strip_prefix(Self::OPTIONAL_PREFIX)
            .and_then(|t| t.strip_suffix('>'))
    }

    /// Whether the lowerings of a slot hold a list of `elem` today: a
    /// fixed-width number or `bool`. A list of records is held too
    /// ([`Self::slot_view`]); a list of anything else is a composition they do
    /// not lower yet.
    pub fn list_elem_admitted(elem: &SceType) -> bool {
        matches!(
            elem,
            SceType::Uint8
                | SceType::Uint16
                | SceType::Uint32
                | SceType::Uint64
                | SceType::Int8
                | SceType::Int16
                | SceType::Int32
                | SceType::Int64
                | SceType::Float32
                | SceType::Float64
                | SceType::Bool
        )
    }

    /// The type of one value of the scalar `t`, as written with no bound: a
    /// text for `string`, a byte string for `bytes`, the scalar itself
    /// otherwise. The one way a scalar becomes a type, so a text is never a
    /// `Scalar`.
    pub fn of_scalar(t: SceType) -> Self {
        match t {
            SceType::String => Self::Text { max: None },
            SceType::Bytes => Self::Bytes { max: None },
            other => Self::Scalar(other),
        }
    }

    /// The type as one scalar: a fixed-width number, `bool` or an enum, or a
    /// text or a byte string read as `string` or `bytes`; `None` for a list,
    /// a record, a union or an optional value. An optional scalar is not a
    /// scalar: a value of it may be absent.
    pub fn scalar(&self) -> Option<&SceType> {
        match self {
            Self::Scalar(t) => Some(t),
            Self::Text { .. } => Some(&TEXT_SCALAR),
            Self::Bytes { .. } => Some(&BYTES_SCALAR),
            Self::List { .. }
            | Self::Record { .. }
            | Self::Union { .. }
            | Self::Optional { .. } => None,
        }
    }

    /// The element type of a list, or `None` for any other type.
    pub fn list_elem(&self) -> Option<&Self> {
        match self {
            Self::List { elem, .. } => Some(elem),
            Self::Scalar(_)
            | Self::Text { .. }
            | Self::Bytes { .. }
            | Self::Record { .. }
            | Self::Union { .. }
            | Self::Optional { .. } => None,
        }
    }

    /// The import alias a record names, or `None` for any other type. An
    /// optional record is not a record: it may be absent.
    pub fn record_alias(&self) -> Option<&str> {
        match self {
            Self::Record { alias } => Some(alias),
            Self::Scalar(_)
            | Self::Text { .. }
            | Self::Bytes { .. }
            | Self::Union { .. }
            | Self::List { .. }
            | Self::Optional { .. } => None,
        }
    }

    /// The element of an optional value, or `None` for any other type.
    pub fn optional_elem(&self) -> Option<&Self> {
        match self {
            Self::Optional { elem } => Some(elem),
            Self::Scalar(_)
            | Self::Text { .. }
            | Self::Bytes { .. }
            | Self::List { .. }
            | Self::Record { .. }
            | Self::Union { .. } => None,
        }
    }

    /// Whether a local of this type is a buffer filled by `<sce:append>`, a
    /// byte string or a list, and so starts empty, takes no `init`, and is the
    /// one shape a buffer-returning algorithm returns.
    pub fn is_append_buffer(&self) -> bool {
        matches!(self, Self::Bytes { .. } | Self::List { .. })
    }

    /// The `type=` attribute spelling: the type as the grammar spells it,
    /// every bound it states in its bracket.
    pub fn as_attr(&self) -> String {
        self.to_string()
    }

    /// Read the `type=` spelling [`Self::as_attr`] writes, for text SCE wrote
    /// itself: a type of the grammar that a slot's lowerings hold
    /// ([`Self::slot_view`]), or `None`. A document's types are read by the
    /// parser, which adds the refusals a document needs.
    pub fn from_attr(s: &str) -> Option<Self> {
        Self::read(s).ok().filter(|ty| ty.slot_view().is_ok())
    }

    /// The type as the lowerings of a slot (an algorithm's parameter, return
    /// and local, a statechart's variable) hold it today: a scalar, a text or a
    /// byte string; a record; a list of fixed-width numbers, `bool` or
    /// records; an optional scalar, text, byte string or record. Any other
    /// type is a composition those lowerings do not hold yet, named by the
    /// part of it they stop at, so that a lowering refuses it by name
    /// (docs/adr/0086, section 7.3), as [`ValueType::entry`] does for an
    /// event schema's field. No slot holds a union yet (docs/adr/0092, landing
    /// order), at any depth.
    pub fn slot_view(&self) -> Result<&Self, Composition> {
        match self {
            Self::Scalar(_) | Self::Text { .. } | Self::Bytes { .. } | Self::Record { .. } => {
                Ok(self)
            }
            Self::Union { .. } => Err(Composition::of(self)),
            Self::List { elem, .. } => match &**elem {
                Self::Scalar(t) if Self::list_elem_admitted(t) => Ok(self),
                Self::Record { .. } => Ok(self),
                other => Err(Composition::of(other)),
            },
            Self::Optional { elem } => match &**elem {
                Self::Scalar(_) | Self::Text { .. } | Self::Bytes { .. } | Self::Record { .. } => {
                    Ok(self)
                }
                other => Err(Composition::of(other)),
            },
        }
    }

    /// The type as a lowering of a statechart's variable holds it: what every
    /// slot holds ([`Self::slot_view`]), and, where the lowering holds unions
    /// (`unions`, [`crate::forge::event_schema_check::ComposedLowering::unions`]),
    /// `union:U` and `optional<union:U>` too (docs/adr/0092, section 6.1 item 5
    /// and its amendment of 2026-11-07). A lowering that holds none refuses a
    /// union by name, as every slot does.
    pub fn variable_view(&self, unions: bool) -> Result<&Self, Composition> {
        match self {
            Self::Union { .. } if unions => Ok(self),
            Self::Optional { elem } if unions && matches!(**elem, Self::Union { .. }) => Ok(self),
            _ => self.slot_view(),
        }
    }

    /// The bound stated on the outermost text, byte string or list of the
    /// type, seen through every `optional<…>`: what a held slot (a return, a
    /// local, a statechart's variable) holds at most, and `None` for a
    /// parameter, whose outermost bound is its argument's, and for a type with
    /// no length.
    pub fn stated_outer_bound(&self) -> Option<&Bound> {
        self.outer_bound().and_then(Option::as_ref)
    }

    /// The same type with its outermost bound left out: the shape two slots
    /// are compared by where each states its own bound (a buffer local and
    /// the return it is).
    pub fn without_outer_bound(&self) -> Self {
        self.clone().split_outer_bound().0
    }

    /// The same type with the bound of its outermost text, byte string or
    /// list, seen through every `optional<…>`, taken out of it, and that
    /// bound.
    pub fn split_outer_bound(self) -> (Self, Option<Bound>) {
        match self {
            Self::Text { max } => (Self::Text { max: None }, max),
            Self::Bytes { max } => (Self::Bytes { max: None }, max),
            Self::List { elem, max } => (Self::List { elem, max: None }, max),
            Self::Optional { elem } => {
                let (elem, max) = elem.split_outer_bound();
                (
                    Self::Optional {
                        elem: Box::new(elem),
                    },
                    max,
                )
            }
            other @ (Self::Scalar(_) | Self::Record { .. } | Self::Union { .. }) => (other, None),
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
    /// The enum `alias` names: its variants, each with its declared name and
    /// number, its underlying type, and whether it is closed; `None` when no
    /// enum goes by that alias.
    fn enum_type(&self, alias: &str) -> Option<&ScopedEnum>;
    /// The environment the fields of the record `alias` names are read in: the
    /// record's own document, whose aliases name its enums and records, and
    /// which need not be the aliases of the document that names the record.
    /// `None` when the record's fields are read here, as a table that holds
    /// one set of aliases answers.
    fn record_scope(&self, _alias: &str) -> Option<&dyn TypeEnv> {
        None
    }
    /// What tells the record `alias` names from every other, across the
    /// documents its fields are read in: what a record that holds itself is
    /// found by. The alias itself where one set of aliases is read.
    fn record_key(&self, alias: &str) -> String {
        alias.to_string()
    }
    /// The name of the document the record `alias` names (its `<scxml
    /// name>`): what a grammar of a type that holds the record names it by
    /// ([`json_schema_of`], [`gbnf_of`]). The alias itself where one set of
    /// aliases is read.
    fn record_name(&self, alias: &str) -> String {
        alias.to_string()
    }
    /// The union `alias` names (docs/adr/0092, section 1.1): the key its tag
    /// is written under, each case with its variant's declared name and
    /// number and the alias of the record it holds, in the tag enum's
    /// declaration order, and the scope its own document names those records
    /// in; `None` when no union goes by that alias.
    fn union_type(&self, alias: &str) -> Option<&ScopedUnion>;
}

/// An enum, as a size reads it: the name of its document, its variants, its
/// underlying type, and whether it is closed. Its JSON value is the name of a
/// variant; its CBOR value is a number (docs/adr/0086, section 9.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedEnum {
    /// The document's name, which its unit is named by on every backend: what
    /// a unit that holds a value of the enum through a record of another
    /// document names the enum's type by.
    pub name: String,
    /// Every variant, in declaration order.
    pub variants: Vec<ScopedVariant>,
    /// The integer its numbers are (`sce:underlying-type`).
    pub underlying: SceType,
    /// Whether a number no variant declares is refused (`sce:strict-variants`,
    /// true unless the document says otherwise). A closed enum holds only its
    /// declared numbers; an open one every number of its underlying type.
    pub strict: bool,
}

impl ScopedEnum {
    /// Whether `other` holds the values this enum holds: the same variants
    /// with the same numbers, the same underlying type, closed alike. Two such
    /// enums are one type whatever documents declare them (docs/adr/0086,
    /// section 5.1), so the name of a document tells none apart.
    pub fn holds_the_values_of(&self, other: &ScopedEnum) -> bool {
        self.variants == other.variants
            && self.underlying == other.underlying
            && self.strict == other.strict
    }
}

/// A variant of an enum, as a size reads it: its declared name and its
/// declared number (`value=`, ADR 0047).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedVariant {
    pub name: String,
    pub value: i128,
}

/// A record, as a size reads it: the document it is (its identity), the name
/// that document gives itself, its fields in the schema's order, and the scope
/// its own document names types in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScopedRecord {
    pub identity: String,
    /// The document's `<scxml name>`, which a grammar of the record names it by.
    pub name: String,
    pub fields: Vec<(String, ValueType)>,
    pub scope: TypeScope,
}

/// A union, as a size reads it (docs/adr/0092, section 1.1): the document it
/// is (its identity), the name that document gives itself, the key its tag is
/// written under, the alias of its tag's enum, each case in the enum's
/// declaration order, and the scope its own document names the enum and the
/// cases' records in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScopedUnion {
    pub identity: String,
    /// The document's `<scxml name>`.
    pub name: String,
    /// The key the tag is written under, beside the case's fields.
    pub tag: String,
    /// The alias, in [`Self::scope`], of the enum the tag is a value of.
    pub tag_enum: String,
    /// One case per variant of the tag's enum, in the enum's declaration
    /// order.
    pub cases: Vec<ScopedCase>,
    pub scope: TypeScope,
}

/// A case of a [`ScopedUnion`]: the variant of the tag's enum it is for, by
/// its declared name and number, and the alias, in the union's scope, of the
/// record it holds, or `None` for an empty case, its tag alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedCase {
    pub variant: ScopedVariant,
    pub record: Option<String>,
}

impl ScopedUnion {
    /// What tells this union from every other record and union, across the
    /// documents its cases are read in: its identity, or `alias` where none
    /// was read (a table of one set of aliases).
    fn key(&self, alias: &str) -> String {
        if self.identity.is_empty() {
            format!("{}{alias}", WrittenType::UNION_PREFIX)
        } else {
            self.identity.clone()
        }
    }
}

/// The enums, records and unions one document's aliases name, each record and
/// union with the scope of its own document: the environment a type of that
/// document is sized in ([`TypeEnv`]). A record's fields are read with its own
/// document's aliases, at every depth, so a record that names its enum `Level`
/// is sized by its enum whatever the document that holds the record calls it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeScope {
    pub enums: std::collections::BTreeMap<String, ScopedEnum>,
    pub records: std::collections::BTreeMap<String, ScopedRecord>,
    pub unions: std::collections::BTreeMap<String, ScopedUnion>,
}

impl TypeEnv for TypeScope {
    fn record_fields(&self, alias: &str) -> Option<Vec<(String, ValueType)>> {
        self.records.get(alias).map(|r| r.fields.clone())
    }
    fn enum_type(&self, alias: &str) -> Option<&ScopedEnum> {
        self.enums.get(alias)
    }
    fn record_scope(&self, alias: &str) -> Option<&dyn TypeEnv> {
        self.records.get(alias).map(|r| &r.scope as &dyn TypeEnv)
    }
    fn record_key(&self, alias: &str) -> String {
        self.records
            .get(alias)
            .map_or_else(|| alias.to_string(), |r| r.identity.clone())
    }
    fn record_name(&self, alias: &str) -> String {
        self.records
            .get(alias)
            .map_or_else(|| alias.to_string(), |r| r.name.clone())
    }
    fn union_type(&self, alias: &str) -> Option<&ScopedUnion> {
        self.unions.get(alias)
    }
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

/// The length of the deterministic CBOR of the integer `n` (RFC 8949 sections
/// 3.1 and 4.2.1): major type 0 with the argument `n` when it is zero or more,
/// major type 1 with the argument `-1 - n` otherwise, each with its shortest
/// head. `None` when the argument is past 2^64 - 1, which no head holds.
pub fn cbor_integer_len(n: i128) -> Option<u64> {
    let argument = if n < 0 { -1 - n } else { n };
    u64::try_from(argument).ok().map(cbor_head_len)
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
        (SceType::Enum(reference), _) => {
            let unresolved = || SizeError::Unresolved {
                alias: reference.alias.clone(),
            };
            let enumeration = env.enum_type(&reference.alias).ok_or_else(unresolved)?;
            match measure {
                Measure::Json => {
                    2 + enumeration
                        .variants
                        .iter()
                        .map(|v| json_escaped_len(&v.name))
                        .max()
                        .unwrap_or(0)
                }
                // A closed enum's value is one of its declared numbers, each
                // written with its shortest head (docs/adr/0086, section 9.2),
                // so the longest of those heads is its size.
                Measure::Cbor if enumeration.strict => {
                    let mut longest: Option<u64> = None;
                    for variant in &enumeration.variants {
                        let len = cbor_integer_len(variant.value).ok_or(SizeError::Overflow)?;
                        longest = longest.max(Some(len));
                    }
                    // A closed enum with no variant holds no value: it was not
                    // read from its document, which declares at least one.
                    longest.ok_or_else(unresolved)?
                }
                // An open enum holds every number of its underlying type.
                Measure::Cbor => {
                    if matches!(enumeration.underlying, SceType::Enum(_)) {
                        return Err(unresolved());
                    }
                    scalar_size(&enumeration.underlying, measure, env)?
                }
            }
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
            let record_key = env.record_key(alias);
            if records.contains(&record_key) {
                let mut cycle = records.clone();
                cycle.push(record_key);
                return Err(SizeError::ContainsItself { cycle });
            }
            let fields = env
                .record_fields(alias)
                .ok_or_else(|| SizeError::Unresolved {
                    alias: alias.clone(),
                })?;
            // The record's fields name types by its own document's aliases.
            let inner = env.record_scope(alias).unwrap_or(env);
            records.push(record_key);
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
                total = add(add(total, key)?, size(field, measure, inner, records)?)?;
            }
            records.pop();
            Ok(total)
        }
        // docs/adr/0092, section 5.1: the largest case, each counted with its
        // own variant's name and number, so the size is tight.
        ValueType::Union { alias } => {
            let union = enter_union(alias, env, records)?;
            let mut largest = 0u64;
            for case in &union.cases {
                largest = largest.max(case_size(union, case, measure, records)?);
            }
            records.pop();
            Ok(largest)
        }
    }
}

/// `J` or `C` of the case `case` of `union` (docs/adr/0092, section 5.1): one
/// object of the tag under its key, whose value is the case's variant (its
/// quoted name on JSON, its declared number on CBOR, as an enum's value is
/// signed, docs/adr/0086 section 9.2), and the fields of the record the case
/// holds, none for an empty case.
///
/// - `J`: 2 + (`J(quoted t)` + 1 + 2 + the escaped length of the variant's
///   name) + Σ (`J(quoted id)` + 1 + `J(Tf)`) + m, a comma between each two of
///   the m + 1 members;
/// - `C`: h(1 + m) + `C(t)` + the head of the variant's number + Σ (`C(id)` +
///   `C(Tf)`).
fn case_size(
    union: &ScopedUnion,
    case: &ScopedCase,
    measure: Measure,
    records: &mut Vec<String>,
) -> Result<u64, SizeError> {
    let (fields, inner) = enter_case(union, case, records)?;
    let m = fields.len() as u64;
    let tag = union.tag.len() as u64;
    let mut total = match measure {
        // The braces, the quoted tag, its colon and the quoted variant name,
        // and a comma between each two members.
        Measure::Json => {
            2 + (2 + json_escaped_len(&union.tag) + 1 + 2 + json_escaped_len(&case.variant.name))
                + m
        }
        Measure::Cbor => {
            let number = cbor_integer_len(case.variant.value).ok_or(SizeError::Overflow)?;
            add(cbor_head_len(1 + m) + cbor_head_len(tag) + tag, number)?
        }
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
        total = add(add(total, key)?, size(field, measure, inner, records)?)?;
    }
    leave_case(case, records);
    Ok(total)
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
        fn enum_type(&self, alias: &str) -> Option<&ScopedEnum> {
            self.env.enum_type(alias)
        }
        fn record_scope(&self, alias: &str) -> Option<&dyn TypeEnv> {
            if alias.is_empty() {
                None
            } else {
                self.env.record_scope(alias)
            }
        }
        fn record_key(&self, alias: &str) -> String {
            self.env.record_key(alias)
        }
        fn record_name(&self, alias: &str) -> String {
            self.env.record_name(alias)
        }
        fn union_type(&self, alias: &str) -> Option<&ScopedUnion> {
            self.env.union_type(alias)
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
    // `path` holds each record on the way by its key ([`TypeEnv::record_key`]),
    // which tells two records apart across the documents their aliases are
    // read in, and by the alias the cycle is reported by.
    fn visit(
        alias: &str,
        env: &dyn TypeEnv,
        path: &mut Vec<(String, String)>,
    ) -> Option<Vec<String>> {
        let key = env.record_key(alias);
        if let Some(at) = path.iter().position(|(k, _)| *k == key) {
            let mut cycle: Vec<String> = path[at..].iter().map(|(_, a)| a.clone()).collect();
            cycle.push(alias.to_string());
            return Some(cycle);
        }
        let fields = env.record_fields(alias)?;
        // The record's fields name records by its own document's aliases.
        let scope = env.record_scope(alias).unwrap_or(env);
        path.push((key, alias.to_string()));
        for (_, ty) in &fields {
            for inner in ty.record_aliases() {
                if let Some(cycle) = visit(inner, scope, path) {
                    return Some(cycle);
                }
            }
            // A union holds the records of its cases, named by its own
            // document's aliases (docs/adr/0092, section 1.1, item 3).
            for union in ty.union_aliases() {
                let Some(held) = scope.union_type(union) else {
                    continue;
                };
                for record in held.cases.iter().filter_map(|c| c.record.as_deref()) {
                    if let Some(cycle) = visit(record, &held.scope, path) {
                        return Some(cycle);
                    }
                }
            }
        }
        path.pop();
        None
    }
    visit(alias, env, &mut Vec::new())
}

// ── What a lift reads of a type (docs/adr/0087, section 2) ──────────────────
//
// Two folds beside the size calculus, by the same recursion and environment:
// the deepest container a value holds, which bounds what a `<sce:lift>` reads
// before it reads a container, and the longest strict RFC 6901 pointer to a
// place a value declares, which bounds the `path` of a refusal it keeps.

/// A record a fold enters: the fields the environment reads it as and the
/// environment those fields are read in, or the refusal of an alias no record
/// goes by and of a record that holds itself. `records` holds each record on
/// the way by its key ([`TypeEnv::record_key`]), which the caller pops.
type EnteredRecord<'e> = (Vec<(String, ValueType)>, &'e dyn TypeEnv);

fn enter_record<'e>(
    alias: &str,
    env: &'e dyn TypeEnv,
    records: &mut Vec<String>,
) -> Result<EnteredRecord<'e>, SizeError> {
    let key = env.record_key(alias);
    if records.contains(&key) {
        let mut cycle = records.clone();
        cycle.push(key);
        return Err(SizeError::ContainsItself { cycle });
    }
    let fields = env
        .record_fields(alias)
        .ok_or_else(|| SizeError::Unresolved {
            alias: alias.to_string(),
        })?;
    records.push(key);
    // The record's fields name types by its own document's aliases.
    Ok((fields, env.record_scope(alias).unwrap_or(env)))
}

/// A union a fold enters (docs/adr/0092, section 1.1): the union the
/// environment reads it as, or the refusal of an alias no union goes by, of a
/// union of no case, and of a union that holds itself through the records of
/// its cases. `records` holds each record and union on the way by its key,
/// which the caller pops.
fn enter_union<'e>(
    alias: &str,
    env: &'e dyn TypeEnv,
    records: &mut Vec<String>,
) -> Result<&'e ScopedUnion, SizeError> {
    let unresolved = || SizeError::Unresolved {
        alias: alias.to_string(),
    };
    let union = env.union_type(alias).ok_or_else(unresolved)?;
    // A union with no case holds no value: it was not read from its
    // document, which declares at least one.
    if union.cases.is_empty() {
        return Err(unresolved());
    }
    let key = union.key(alias);
    if records.contains(&key) {
        let mut cycle = records.clone();
        cycle.push(key);
        return Err(SizeError::ContainsItself { cycle });
    }
    records.push(key);
    Ok(union)
}

/// The fields of the record the case `case` of `union` holds, none for an
/// empty case, and the environment they are read in. `records` gains the
/// record's key where the case holds one, which [`leave_case`] pops.
fn enter_case<'e>(
    union: &'e ScopedUnion,
    case: &ScopedCase,
    records: &mut Vec<String>,
) -> Result<EnteredRecord<'e>, SizeError> {
    // The cases' records are named by the union's own document's aliases.
    let scope: &'e dyn TypeEnv = &union.scope;
    match &case.record {
        Some(record) => enter_record(record, scope, records),
        None => Ok((Vec::new(), scope)),
    }
}

/// Leave what [`enter_case`] entered.
fn leave_case(case: &ScopedCase, records: &mut Vec<String>) {
    if case.record.is_some() {
        records.pop();
    }
}

fn depth_at(
    ty: &ValueType,
    env: &dyn TypeEnv,
    records: &mut Vec<String>,
) -> Result<u32, SizeError> {
    match ty {
        ValueType::Scalar(_) | ValueType::Text { .. } | ValueType::Bytes { .. } => Ok(0),
        ValueType::Optional { elem } => depth_at(elem, env, records),
        ValueType::List { elem, .. } => Ok(depth_at(elem, env, records)?.saturating_add(1)),
        ValueType::Record { alias } => {
            let (fields, inner) = enter_record(alias, env, records)?;
            let mut deepest = 0u32;
            for (_, field) in &fields {
                deepest = deepest.max(depth_at(field, inner, records)?);
            }
            records.pop();
            Ok(deepest.saturating_add(1))
        }
        // docs/adr/0092, section 4.1, item 3: one level, as a record is, and
        // the deepest field of any case; the tag is a value of an enum, no
        // container.
        ValueType::Union { alias } => {
            let union = enter_union(alias, env, records)?;
            let mut deepest = 0u32;
            for case in &union.cases {
                let (fields, inner) = enter_case(union, case, records)?;
                for (_, field) in &fields {
                    deepest = deepest.max(depth_at(field, inner, records)?);
                }
                leave_case(case, records);
            }
            records.pop();
            Ok(deepest.saturating_add(1))
        }
    }
}

/// `depth(T)`: how deep the deepest container a value of `ty` holds is, the
/// levels counted from 1 for the outermost: a record and a list each add a
/// level, an optional value none, and a number, a truth value, an enum, a text
/// and a byte string are no container (docs/adr/0087, section 2.1, item 4). A
/// lift of a record of `R` refuses a container opened deeper than `depth(R)`
/// before it reads it (`tooDeep`), so its work and its stack are bounded by the
/// type whatever the text holds.
pub fn container_depth(ty: &ValueType, env: &dyn TypeEnv) -> Result<u32, SizeError> {
    depth_at(ty, env, &mut Vec::new())
}

fn pointer_at(
    ty: &ValueType,
    env: &dyn TypeEnv,
    records: &mut Vec<String>,
) -> Result<(String, u64), SizeError> {
    match ty {
        ValueType::Scalar(_) | ValueType::Text { .. } | ValueType::Bytes { .. } => {
            Ok((String::new(), 0))
        }
        ValueType::Optional { elem } => pointer_at(elem, env, records),
        ValueType::List { elem, max } => {
            // A list of no element has no index to point at.
            let Some(last) = max.max.checked_sub(1) else {
                return Ok((String::new(), 0));
            };
            let (below, below_len) = pointer_at(elem, env, records)?;
            let token = last.to_string();
            let len = add(add(1, token.len() as u64)?, below_len)?;
            Ok((format!("/{token}{below}"), len))
        }
        ValueType::Record { alias } => {
            let (fields, inner) = enter_record(alias, env, records)?;
            let mut longest = (String::new(), 0u64);
            for (id, field) in &fields {
                let (below, below_len) = pointer_at(field, inner, records)?;
                let token = pointer_token(id);
                let len = add(add(1, token.len() as u64)?, below_len)?;
                if len > longest.1 {
                    longest = (format!("/{token}{below}"), len);
                }
            }
            records.pop();
            Ok(longest)
        }
        // docs/adr/0092, section 4.1, item 3: the union adds no level and no
        // key of its own, so a place it declares is its tag or a field of a
        // case, each one token below the union's pointer. The tag comes first,
        // as it stands first in each case's grammar.
        ValueType::Union { alias } => {
            let union = enter_union(alias, env, records)?;
            let tag = pointer_token(&union.tag);
            let mut longest = (format!("/{tag}"), add(1, tag.len() as u64)?);
            for case in &union.cases {
                let (fields, inner) = enter_case(union, case, records)?;
                for (id, field) in &fields {
                    let (below, below_len) = pointer_at(field, inner, records)?;
                    let token = pointer_token(id);
                    let len = add(add(1, token.len() as u64)?, below_len)?;
                    if len > longest.1 {
                        longest = (format!("/{token}{below}"), len);
                    }
                }
                leave_case(case, records);
            }
            records.pop();
            Ok(longest)
        }
    }
}

/// The longest strict RFC 6901 pointer to a place a value of `ty` declares,
/// with its length in bytes: `/` before each reference token, `~` written `~0`
/// and `/` written `~1`, a list's largest index for its elements, and the empty
/// pointer for a value that holds no place (docs/adr/0087, section 2.4). Every
/// refusal a lift keeps names a place its type declares, so no pointer it
/// writes is longer than this. Of two of one length, the first in the schema's
/// order.
pub fn longest_pointer(ty: &ValueType, env: &dyn TypeEnv) -> Result<(String, u64), SizeError> {
    pointer_at(ty, env, &mut Vec::new())
}

// ── The grammar of a value (docs/adr/0087, section 1) ───────────────────────
//
// Two folds beside the size calculus, by the same structural recursion over
// the type and the same environment: `S(T)`, the JSON Schema (2020-12,
// draft-bhutton-json-schema-01 and draft-bhutton-json-schema-validation-01) of
// a value, and `G(T)`, its GBNF grammar (the grammar format of llama.cpp,
// `grammars/README.md` of ggml-org/llama.cpp and its parser
// `src/llama-grammar.cpp`, read at commit acecd56032dd of 2026-09-12). Each is
// one exhaustive match with an arm per constructor. Both state one spelling of
// each value: a record with every key and no other, its keys in the order the
// record's document declares them (a decoder held to either form writes them
// in the order it states, docs/adr/0087 section 1.1 item 2, where SCE's
// writers order them ascending, ARCHITECTURE.md "JSON Object Key Order"), a
// 64-bit integer as the quoted text of its digits, no whitespace outside a
// text. A reader accepts more, keys in any order among it; a text in the
// language of either is one it accepts.

/// Why a type has no grammar the build can write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrammarError {
    /// An alias no record, enum or union of the environment goes by, or a
    /// union of no case.
    Unresolved { alias: String },
    /// A record or a union that holds itself, directly or through other
    /// records and the records of its cases: the keys on the way, the first
    /// repeated at the end.
    ContainsItself { cycle: Vec<String> },
    /// A `Scalar` built over a text or a byte string, whose bound is not part
    /// of it: the reader of a type never builds one.
    Unbounded { constructor: String },
    /// An enum whose set of variants is open (`sce:strict-variants="false"`):
    /// a value of it may be a number no declared name spells, and a grammar
    /// spells a value of an enum by its name.
    OpenEnum { alias: String },
    /// Two records or unions of different documents under one document name,
    /// which a JSON Schema's `$defs` would name alike.
    SharedName { name: String },
}

impl fmt::Display for GrammarError {
    /// The reason as a refusal states it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The two a size refuses for are stated as the size states them.
            Self::Unresolved { alias } => fmt::Display::fmt(
                &SizeError::Unresolved {
                    alias: alias.clone(),
                },
                f,
            ),
            Self::ContainsItself { cycle } => fmt::Display::fmt(
                &SizeError::ContainsItself {
                    cycle: cycle.clone(),
                },
                f,
            ),
            Self::Unbounded { constructor } => write!(
                f,
                "a `{constructor}` with no bound has no largest value, and no grammar"
            ),
            Self::OpenEnum { alias } => write!(
                f,
                "the enum `{alias}` is open (`sce:strict-variants=\"false\"`): a value of it may \
                 be a number no name spells, and a grammar spells a value of an enum by its name"
            ),
            Self::SharedName { name } => write!(
                f,
                "two records or unions of different documents are named `{name}`, which the \
                 schema's `$defs` would name alike"
            ),
        }
    }
}

/// A record as a fold of a grammar meets it.
struct GrammarRecord<'e> {
    /// What tells it from every other record ([`TypeEnv::record_key`]).
    key: String,
    /// Its document's name ([`TypeEnv::record_name`]).
    name: String,
    fields: Vec<(String, ValueType)>,
    /// The environment its fields are read in: its own document's.
    scope: &'e dyn TypeEnv,
}

/// The record `alias` names, or why a fold cannot descend into it. `on_path`
/// holds the key of each record the fold is inside, so a record met again
/// holds itself.
fn grammar_record<'e>(
    alias: &str,
    env: &'e dyn TypeEnv,
    on_path: &[String],
) -> Result<GrammarRecord<'e>, GrammarError> {
    let key = env.record_key(alias);
    if on_path.contains(&key) {
        let mut cycle = on_path.to_vec();
        cycle.push(key);
        return Err(GrammarError::ContainsItself { cycle });
    }
    let fields = env
        .record_fields(alias)
        .ok_or_else(|| GrammarError::Unresolved {
            alias: alias.to_string(),
        })?;
    Ok(GrammarRecord {
        key,
        name: env.record_name(alias),
        fields,
        scope: env.record_scope(alias).unwrap_or(env),
    })
}

/// The union `alias` names, with a case, or why a fold cannot descend into it
/// (docs/adr/0092, section 3.1): an alias no union goes by, a union of no
/// case, which was not read from its document, and a union met again inside
/// itself. `on_path` holds the key of each record and union the fold is
/// inside.
fn grammar_union<'e>(
    alias: &str,
    env: &'e dyn TypeEnv,
    on_path: &[String],
) -> Result<&'e ScopedUnion, GrammarError> {
    let unresolved = || GrammarError::Unresolved {
        alias: alias.to_string(),
    };
    let union = env.union_type(alias).ok_or_else(unresolved)?;
    if union.cases.is_empty() {
        return Err(unresolved());
    }
    tag_closed(union)?;
    let key = union.key(alias);
    if on_path.contains(&key) {
        let mut cycle = on_path.to_vec();
        cycle.push(key);
        return Err(GrammarError::ContainsItself { cycle });
    }
    Ok(union)
}

/// The keys a fold of the union `union`, as the whole of a schema, starts
/// inside: the union's own, so a case that holds the union again holds
/// itself; or the refusal of a union of no case and of an open tag, as
/// [`grammar_union`] refuses them.
fn whole_union(union: &ScopedUnion) -> Result<Vec<String>, GrammarError> {
    if union.cases.is_empty() {
        return Err(GrammarError::Unresolved {
            alias: union.name.clone(),
        });
    }
    tag_closed(union)?;
    Ok(vec![union.key(&union.name)])
}

/// The refusal of a union whose tag is a value of an open enum, as an enum
/// value's grammar refuses one ([`GrammarError::OpenEnum`]): a number no
/// variant names would be a value with no case, which the import refuses
/// first (`crate::forge::union_check`).
fn tag_closed(union: &ScopedUnion) -> Result<(), GrammarError> {
    match union.scope.enum_type(&union.tag_enum) {
        Some(tag) if !tag.strict => Err(GrammarError::OpenEnum {
            alias: union.tag_enum.clone(),
        }),
        _ => Ok(()),
    }
}

/// What `fold` makes of each case of `union`, in its tag enum's declaration
/// order: `fold` is given the case, the fields of the record it holds (none
/// for an empty case) and the scope they are read in, the union's own
/// document's for the record's alias and the record's own for its fields,
/// with the record's key on `on_path` while it runs.
fn fold_cases<T, F>(
    union: &ScopedUnion,
    on_path: &mut Vec<String>,
    mut fold: F,
) -> Result<Vec<T>, GrammarError>
where
    F: FnMut(
        &ScopedCase,
        &[(String, ValueType)],
        &dyn TypeEnv,
        &mut Vec<String>,
    ) -> Result<T, GrammarError>,
{
    let scope: &dyn TypeEnv = &union.scope;
    let empty: &[(String, ValueType)] = &[];
    let mut folded = Vec::with_capacity(union.cases.len());
    for case in &union.cases {
        folded.push(match &case.record {
            None => fold(case, empty, scope, on_path)?,
            Some(alias) => {
                let record = grammar_record(alias, scope, on_path)?;
                on_path.push(record.key);
                let one = fold(case, &record.fields, record.scope, on_path)?;
                on_path.pop();
                one
            }
        });
    }
    Ok(folded)
}

/// The enum `reference` names, closed and with a variant, or why a grammar
/// cannot spell a value of it.
fn closed_enum<'e>(
    reference: &crate::forge::model::EnumRef,
    env: &'e dyn TypeEnv,
) -> Result<&'e ScopedEnum, GrammarError> {
    let alias = reference.alias.as_str();
    let unresolved = || GrammarError::Unresolved {
        alias: alias.to_string(),
    };
    let enumeration = env.enum_type(alias).ok_or_else(unresolved)?;
    if !enumeration.strict {
        return Err(GrammarError::OpenEnum {
            alias: alias.to_string(),
        });
    }
    // A closed enum with no variant holds no value: it was not read from its
    // document, which declares at least one.
    if enumeration.variants.is_empty() {
        return Err(unresolved());
    }
    Ok(enumeration)
}

/// The least and the greatest value of a fixed-width integer type.
fn integer_bounds(t: &SceType) -> (i128, i128) {
    t.int_value_range()
        .expect("a fixed-width integer type has a range")
}

/// `text` as a JSON string, as every writer of SCE spells it (ARCHITECTURE.md,
/// "JSON Text"): `"` and `\` escaped, the five short escapes, every other
/// control character as `\u00xx` in lowercase hex, everything else as it is.
fn json_quoted(text: &str) -> String {
    serde_json::to_string(text).expect("a text is a JSON string")
}

/// One alternative of the decimal spelling of a range of whole numbers: the
/// digits `prefix`, one digit from `first` to `last`, then from `free.0` to
/// `free.1` digits of any value.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DigitRun {
    prefix: String,
    first: u8,
    last: u8,
    free: (usize, usize),
}

/// The decimal spellings, with no leading zero, of exactly the numbers from 0
/// (from 1 when `zero` is false) to `hi`, by the standard decomposition of a
/// range into digit-prefix alternatives: `0`; every number with fewer digits
/// than `hi`; then, for each digit of `hi`, the numbers that agree with `hi`
/// before it and are below it there, the last digit up to `hi`'s own.
fn digit_runs(zero: bool, hi: u128) -> Vec<DigitRun> {
    let text = hi.to_string();
    let digits: Vec<u8> = text.bytes().map(|b| b - b'0').collect();
    let n = digits.len();
    let mut runs = Vec::new();
    if zero {
        runs.push(DigitRun {
            prefix: String::new(),
            first: 0,
            last: 0,
            free: (0, 0),
        });
    }
    if n >= 2 {
        runs.push(DigitRun {
            prefix: String::new(),
            first: 1,
            last: 9,
            free: (0, n - 2),
        });
    }
    for (at, &digit) in digits.iter().enumerate() {
        let first = u8::from(at == 0);
        let last = if at + 1 == n {
            digit
        } else {
            match digit.checked_sub(1) {
                Some(below) => below,
                None => continue,
            }
        };
        if first > last {
            continue;
        }
        let rest = n - 1 - at;
        runs.push(DigitRun {
            prefix: text[..at].to_string(),
            first,
            last,
            free: (rest, rest),
        });
    }
    runs
}

/// From `from` to `to` digits of any value, as GBNF and a regular expression
/// both spell the repetition; `None` for none.
fn free_digits((from, to): (usize, usize)) -> Option<String> {
    match (from, to) {
        (0, 0) => None,
        (1, 1) => Some("[0-9]".to_string()),
        (from, to) if from == to => Some(format!("[0-9]{{{from}}}")),
        (from, to) => Some(format!("[0-9]{{{from},{to}}}")),
    }
}

/// A digit run as GBNF: a literal of its fixed digits, then its classes.
fn gbnf_run(run: &DigitRun) -> String {
    let mut literal = run.prefix.clone();
    if run.first == run.last {
        literal.push(char::from(b'0' + run.first));
    }
    let mut parts = Vec::new();
    if !literal.is_empty() {
        parts.push(format!("\"{literal}\""));
    }
    if run.first != run.last {
        parts.push(format!("[{}-{}]", run.first, run.last));
    }
    parts.extend(free_digits(run.free));
    parts.join(" ")
}

/// A digit run as a regular expression.
fn regex_run(run: &DigitRun) -> String {
    let mut out = run.prefix.clone();
    if run.first == run.last {
        out.push(char::from(b'0' + run.first));
    } else {
        out.push_str(&format!("[{}-{}]", run.first, run.last));
    }
    out.extend(free_digits(run.free));
    out
}

/// The GBNF alternatives of exactly the integers `lo..=hi`, `lo` zero or
/// below and `hi` zero or above: `-` before the magnitude of a negative one.
fn gbnf_range(lo: i128, hi: i128) -> String {
    let positive: Vec<String> = digit_runs(true, hi.unsigned_abs())
        .iter()
        .map(gbnf_run)
        .collect();
    if lo < 0 {
        let negative: Vec<String> = digit_runs(false, lo.unsigned_abs())
            .iter()
            .map(gbnf_run)
            .collect();
        format!(
            "\"-\" ( {} ) | {}",
            negative.join(" | "),
            positive.join(" | ")
        )
    } else {
        positive.join(" | ")
    }
}

/// The regular expression (ECMA-262, anchored) of exactly the integers
/// `lo..=hi`, by the same decomposition as [`gbnf_range`].
fn regex_range(lo: i128, hi: i128) -> String {
    let mut alternatives = Vec::new();
    if lo < 0 {
        let negative: Vec<String> = digit_runs(false, lo.unsigned_abs())
            .iter()
            .map(regex_run)
            .collect();
        alternatives.push(format!("-({})", negative.join("|")));
    }
    alternatives.extend(digit_runs(true, hi.unsigned_abs()).iter().map(regex_run));
    format!("^({})$", alternatives.join("|"))
}

/// A keyword of the subset of JSON Schema 2020-12 a payload schema is written
/// in (`schemas/sce-payload-schema.v1.schema.json`), declared in the one order
/// every schema object writes its keywords in, whatever order a fold puts them
/// in: what names the schema (`$schema`, `$id`, the source hash), what it says
/// of itself, the form of a value (`$ref`, `type`, `const`, `enum`, `anyOf`,
/// `oneOf`), its bounds, each annotation beside the keyword it qualifies, then
/// a record's keys and the definitions the schema gathered. It is the order of
/// the example of docs/adr/0087, section 1.1, item 4, and it makes the text of
/// a schema one text. `const` (a union case's tag) stands before `enum` and
/// `oneOf` (a union's cases) after `anyOf` (docs/adr/0092, section 3.1, item
/// 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Keyword {
    Schema,
    Id,
    SourceHash,
    Description,
    Ref,
    Type,
    Const,
    Enum,
    AnyOf,
    OneOf,
    Minimum,
    Maximum,
    MaxLength,
    MaxBytes,
    Pattern,
    Items,
    MinItems,
    MaxItems,
    Properties,
    Required,
    AdditionalProperties,
    Defs,
}

impl Keyword {
    /// The keyword as a schema spells it.
    pub fn name(self) -> &'static str {
        match self {
            Keyword::Schema => "$schema",
            Keyword::Id => "$id",
            Keyword::SourceHash => "x-sce-source-hash",
            Keyword::Description => "description",
            Keyword::Ref => "$ref",
            Keyword::Type => "type",
            Keyword::Const => "const",
            Keyword::Enum => "enum",
            Keyword::AnyOf => "anyOf",
            Keyword::OneOf => "oneOf",
            Keyword::Minimum => "minimum",
            Keyword::Maximum => "maximum",
            Keyword::MaxLength => "maxLength",
            Keyword::MaxBytes => "x-sce-max-bytes",
            Keyword::Pattern => "pattern",
            Keyword::Items => "items",
            Keyword::MinItems => "minItems",
            Keyword::MaxItems => "maxItems",
            Keyword::Properties => "properties",
            Keyword::Required => "required",
            Keyword::AdditionalProperties => "additionalProperties",
            Keyword::Defs => "$defs",
        }
    }
}

/// What a payload schema holds under a keyword.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaNode {
    /// A value that holds no schema: a text, a number, a truth value, or a
    /// list of texts (`const`, `enum`, `required`).
    Value(serde_json::Value),
    /// One schema (`items`).
    Schema(SchemaObject),
    /// Schemas in order (`anyOf`, `oneOf`).
    Schemas(Vec<SchemaObject>),
    /// Schemas by name (`properties`, `$defs`), written in the order they
    /// were put in: a record's keys in the order its document declares them,
    /// a union case's tag before them.
    Named(Vec<(String, SchemaObject)>),
}

impl SchemaNode {
    /// `value`, a value that holds no schema.
    pub fn value(value: impl Into<serde_json::Value>) -> Self {
        Self::Value(value.into())
    }

    fn write(&self, out: &mut String) {
        match self {
            Self::Value(value) => out.push_str(&value.to_string()),
            Self::Schema(schema) => schema.write(out),
            Self::Schemas(schemas) => {
                out.push('[');
                for (at, schema) in schemas.iter().enumerate() {
                    if at > 0 {
                        out.push(',');
                    }
                    schema.write(out);
                }
                out.push(']');
            }
            Self::Named(members) => {
                out.push('{');
                for (at, (name, schema)) in members.iter().enumerate() {
                    if at > 0 {
                        out.push(',');
                    }
                    out.push_str(&json_quoted(name));
                    out.push(':');
                    schema.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// A JSON Schema of the subset a payload schema is written in: an object of
/// keywords, held and written in the order of [`Keyword`].
///
/// JSON Schema calls the order of an object's members insignificant, and a
/// validator reads it so; a decoder held to a schema does not. llguidance,
/// XGrammar and Outlines write an object's properties in the order the schema
/// states them, so the order a schema is written in is the order a model held
/// to it answers in, and a record's keys are stated in the order its document
/// declares them (docs/adr/0087, section 1.1, item 2).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SchemaObject {
    keywords: std::collections::BTreeMap<Keyword, SchemaNode>,
}

impl SchemaObject {
    /// The schema whose `type` is `name`.
    pub fn typed(name: &str) -> Self {
        Self::default().with(Keyword::Type, SchemaNode::value(name))
    }

    /// This schema with `node` under `keyword`, in place of what it held there.
    pub fn with(mut self, keyword: Keyword, node: SchemaNode) -> Self {
        self.insert(keyword, node);
        self
    }

    /// Put `node` under `keyword`, in place of what the schema held there.
    pub fn insert(&mut self, keyword: Keyword, node: SchemaNode) {
        self.keywords.insert(keyword, node);
    }

    /// The schema as compact JSON text, ASCII (every other character
    /// escaped): its keywords in the order of [`Keyword`] at every depth, the
    /// members of `properties` and `$defs` in the order they were put in.
    pub fn text(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        ascii_only(&out, EscapeForm::Json)
    }

    fn write(&self, out: &mut String) {
        out.push('{');
        for (at, (keyword, node)) in self.keywords.iter().enumerate() {
            if at > 0 {
                out.push(',');
            }
            out.push_str(&json_quoted(keyword.name()));
            out.push(':');
            node.write(out);
        }
        out.push('}');
    }
}

/// The definitions a JSON Schema of a type gathers as its fold walks it: each
/// record and each union the type holds, once, under its document's name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SchemaDefs {
    /// Each definition by its name, with the key of the record or union it
    /// defines.
    defs: std::collections::BTreeMap<String, (String, SchemaObject)>,
}

impl SchemaDefs {
    /// What the schema holds under `$defs`, each definition by its name in
    /// ascending order, or `None` when the type holds no record.
    pub fn into_node(self) -> Option<SchemaNode> {
        if self.defs.is_empty() {
            return None;
        }
        Some(SchemaNode::Named(
            self.defs
                .into_iter()
                .map(|(name, (_, schema))| (name, schema))
                .collect(),
        ))
    }
}

/// The form a code point is escaped in: JSON's (`\uxxxx`, a UTF-16 pair past
/// U+FFFF; also the escape of an ECMA-262 regular expression) or GBNF's
/// (`\xhh` below U+0080, `\uxxxx`, then `\Uxxxxxxxx`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EscapeForm {
    Json,
    Gbnf,
}

/// `c` as an escape of `form`, its hex in lowercase: the one spelling both
/// forms give a code point that is not ASCII, and the bounds of every class
/// they write, so the texts are ASCII whatever a document names.
fn escaped(c: char, form: EscapeForm) -> String {
    let n = u32::from(c);
    match form {
        EscapeForm::Gbnf if n < 0x80 => format!("\\x{n:02x}"),
        EscapeForm::Json | EscapeForm::Gbnf if n <= 0xffff => format!("\\u{n:04x}"),
        EscapeForm::Gbnf => format!("\\U{n:08x}"),
        EscapeForm::Json => {
            let mut units = [0u16; 2];
            c.encode_utf16(&mut units)
                .iter()
                .map(|unit| format!("\\u{unit:04x}"))
                .collect()
        }
    }
}

/// `text` with every character past ASCII escaped in `form`; `text` is a JSON
/// text or a GBNF grammar, where such a character stands only inside a string
/// or a literal, which reads the escape as the character.
fn ascii_only(text: &str, form: EscapeForm) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii() {
                c.to_string()
            } else {
                escaped(c, form)
            }
        })
        .collect()
}

/// The pattern of a byte string's text: every character one byte, U+0000 to
/// U+00FF (the Latin-1 text the wire spells, docs/adr/0086 section 3.1).
fn latin1_pattern() -> String {
    format!(
        "^[{}-{}]*$",
        escaped('\u{0}', EscapeForm::Json),
        escaped('\u{ff}', EscapeForm::Json)
    )
}

/// `name` as a reference token of a JSON Pointer (RFC 6901, section 3): `~`
/// written `~0`, `/` written `~1`.
fn pointer_token(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

/// `S(T)`: the JSON Schema (2020-12) of a value of `ty`, the records it holds
/// gathered in `defs` (docs/adr/0087, section 1.1).
///
/// - a number of at most 32 bits is an `integer` between its type's ends; a
///   64-bit one a `string` whose `pattern` is exactly its type's range;
/// - a real is a `number`, a `float32` between its largest finite values;
/// - a closed enum is the `enum` of its names; an open one has none;
/// - a text's `maxLength` counts characters and admits every text of its
///   bound in bytes, which `x-sce-max-bytes` states;
/// - a byte string's characters are each one byte;
/// - an optional value is `anyOf` its element and `null`, and its key stays
///   required;
/// - a record is a `$ref` to its definition, an object of every key, in the
///   order its document declares them, and no other;
/// - a union is a `$ref` to its definition, the `oneOf` of one object per
///   case ([`json_schema_union`]);
/// - a list is an `array` of at most its bound.
pub fn json_schema_of(
    ty: &ValueType,
    env: &dyn TypeEnv,
    defs: &mut SchemaDefs,
) -> Result<SchemaObject, GrammarError> {
    json_schema(ty, env, defs, &mut Vec::new())
}

/// `S` of a record of `fields`: an object of every key, each required, and no
/// other, the keys in the order of `fields`. What a payload's schema states at
/// its root and a record's definition states in `$defs`.
pub fn json_schema_object(
    fields: &[(String, ValueType)],
    env: &dyn TypeEnv,
    defs: &mut SchemaDefs,
) -> Result<SchemaObject, GrammarError> {
    json_object(None, fields, env, defs, &mut Vec::new())
}

/// `S` of a value of the union `union` as the whole of a schema: the `oneOf`
/// of one object per case, what a union document's unit states at its root
/// (docs/adr/0092, section 3.1, item 4), and the definition a type that holds
/// the union refers to. The records the cases' fields hold are gathered in
/// `defs`.
///
/// Each case is an object of its tag, a string `const` of its variant's name,
/// then the fields of the record it holds, written inline, every key required
/// and no other. The tag stands in each case's `required` and is a `const`
/// literal, and the literals differ, so a converter can prove the cases
/// disjoint: llguidance refuses a `oneOf` it cannot, and a `$ref` to the
/// record is never proven so (section 3.1, item 2).
pub fn json_schema_union(
    union: &ScopedUnion,
    defs: &mut SchemaDefs,
) -> Result<SchemaObject, GrammarError> {
    json_cases(union, defs, &mut whole_union(union)?)
}

/// An object of `fields`, each key required and no other, the keys in the
/// order of `fields`, after `lead` where it is given: a union case's tag,
/// which stands first in `properties` and in `required` (docs/adr/0092,
/// section 3.1, item 1).
fn json_object(
    lead: Option<(String, SchemaObject)>,
    fields: &[(String, ValueType)],
    env: &dyn TypeEnv,
    defs: &mut SchemaDefs,
    on_path: &mut Vec<String>,
) -> Result<SchemaObject, GrammarError> {
    let mut properties = Vec::with_capacity(fields.len() + 1);
    properties.extend(lead);
    for (id, field) in fields {
        properties.push((id.clone(), json_schema(field, env, defs, on_path)?));
    }
    // Every key, in the order the document declares them, as `properties`
    // states them: a decoder held to the schema writes the keys in this order
    // (docs/adr/0087, section 1.1, item 2).
    let required: Vec<&str> = properties.iter().map(|(id, _)| id.as_str()).collect();
    let required = SchemaNode::value(required);
    Ok(SchemaObject::typed("object")
        .with(Keyword::Properties, SchemaNode::Named(properties))
        .with(Keyword::Required, required)
        .with(Keyword::AdditionalProperties, SchemaNode::value(false)))
}

/// `{"oneOf":[...]}` of the cases of `union` ([`json_schema_union`]).
fn json_cases(
    union: &ScopedUnion,
    defs: &mut SchemaDefs,
    on_path: &mut Vec<String>,
) -> Result<SchemaObject, GrammarError> {
    let cases = fold_cases(union, on_path, |case, fields, scope, on_path| {
        let tag = SchemaObject::default().with(
            Keyword::Const,
            SchemaNode::value(case.variant.name.as_str()),
        );
        json_object(Some((union.tag.clone(), tag)), fields, scope, defs, on_path)
    })?;
    Ok(SchemaObject::default().with(Keyword::OneOf, SchemaNode::Schemas(cases)))
}

/// `{"$ref":"#/$defs/<name>"}`, the record or union `key` names defined in
/// `defs` under `name` by `define`, called with `key` on `on_path`, the first
/// time the fold meets it, and once: a type met twice is defined once. Two
/// documents under one name are refused, whichever is met first and also when
/// the one inside the other takes the name first.
fn defined<D>(
    defs: &mut SchemaDefs,
    name: String,
    key: String,
    on_path: &mut Vec<String>,
    define: D,
) -> Result<SchemaObject, GrammarError>
where
    D: FnOnce(&mut SchemaDefs, &mut Vec<String>) -> Result<SchemaObject, GrammarError>,
{
    match defs.defs.get(&name) {
        Some((known, _)) if *known == key => {}
        Some(_) => return Err(GrammarError::SharedName { name }),
        None => {
            on_path.push(key.clone());
            let object = define(defs, on_path)?;
            on_path.pop();
            if defs.defs.contains_key(&name) {
                return Err(GrammarError::SharedName { name });
            }
            defs.defs.insert(name.clone(), (key, object));
        }
    }
    let reference = format!("#/$defs/{}", pointer_token(&name));
    Ok(SchemaObject::default().with(Keyword::Ref, SchemaNode::value(reference)))
}

fn json_schema(
    ty: &ValueType,
    env: &dyn TypeEnv,
    defs: &mut SchemaDefs,
    on_path: &mut Vec<String>,
) -> Result<SchemaObject, GrammarError> {
    Ok(match ty {
        ValueType::Scalar(t) => json_scalar(t, env)?,
        // `maxLength` counts characters (draft-bhutton-json-schema-validation-01,
        // section 6.3.1): a text of at most B bytes has at most B of them, so it
        // admits every value and some longer, which the reader refuses.
        ValueType::Text { max } => SchemaObject::typed("string")
            .with(Keyword::MaxLength, SchemaNode::value(max.max))
            .with(Keyword::MaxBytes, SchemaNode::value(max.max)),
        ValueType::Bytes { max } => SchemaObject::typed("string")
            .with(Keyword::MaxLength, SchemaNode::value(max.max))
            .with(Keyword::Pattern, SchemaNode::value(latin1_pattern())),
        // A writer writes the key of a value that is absent, as `null`.
        ValueType::Optional { elem } => {
            let element = json_schema(elem, env, defs, on_path)?;
            let null = SchemaObject::typed("null");
            SchemaObject::default().with(Keyword::AnyOf, SchemaNode::Schemas(vec![element, null]))
        }
        // A lower count is `minItems`, written only where one is stated
        // (docs/adr/0086, section 15.9): a list that states none is the schema
        // it always was.
        ValueType::List { elem, max } => {
            let element = json_schema(elem, env, defs, on_path)?;
            let mut array =
                SchemaObject::typed("array").with(Keyword::Items, SchemaNode::Schema(element));
            if max.min > 0 {
                array.insert(Keyword::MinItems, SchemaNode::value(max.min));
            }
            array.with(Keyword::MaxItems, SchemaNode::value(max.max))
        }
        ValueType::Record { alias } => {
            let GrammarRecord {
                key,
                name,
                fields,
                scope,
            } = grammar_record(alias, env, on_path)?;
            defined(defs, name, key, on_path, |defs, on_path| {
                json_object(None, &fields, scope, defs, on_path)
            })?
        }
        // docs/adr/0092, section 3.1: defined once, as a record is, under its
        // document's name.
        ValueType::Union { alias } => {
            let union = grammar_union(alias, env, on_path)?;
            defined(
                defs,
                union.name.clone(),
                union.key(alias),
                on_path,
                |defs, on_path| json_cases(union, defs, on_path),
            )?
        }
    })
}

/// `S` of a scalar.
fn json_scalar(t: &SceType, env: &dyn TypeEnv) -> Result<SchemaObject, GrammarError> {
    Ok(match t {
        SceType::Uint8
        | SceType::Uint16
        | SceType::Uint32
        | SceType::Int8
        | SceType::Int16
        | SceType::Int32 => {
            let (lo, hi) = integer_bounds(t);
            let narrow = |n: i128| i64::try_from(n).expect("a 32-bit end fits 64 bits");
            SchemaObject::typed("integer")
                .with(Keyword::Minimum, SchemaNode::value(narrow(lo)))
                .with(Keyword::Maximum, SchemaNode::value(narrow(hi)))
        }
        // A 64-bit integer crosses as the quoted text of its digits
        // (docs/adr/0086, section 3.1).
        SceType::Uint64 | SceType::Int64 => {
            let (lo, hi) = integer_bounds(t);
            SchemaObject::typed("string")
                .with(Keyword::Pattern, SchemaNode::value(regex_range(lo, hi)))
        }
        SceType::Float32 => {
            let most = f64::from(f32::MAX);
            SchemaObject::typed("number")
                .with(Keyword::Minimum, SchemaNode::value(-most))
                .with(Keyword::Maximum, SchemaNode::value(most))
        }
        SceType::Float64 => SchemaObject::typed("number"),
        SceType::Bool => SchemaObject::typed("boolean"),
        SceType::Enum(reference) => {
            let names: Vec<&str> = closed_enum(reference, env)?
                .variants
                .iter()
                .map(|v| v.name.as_str())
                .collect();
            SchemaObject::default().with(Keyword::Enum, SchemaNode::value(names))
        }
        SceType::String | SceType::Bytes => {
            return Err(GrammarError::Unbounded {
                constructor: t.as_attr(),
            })
        }
    })
}

/// What `"` is in a GBNF literal.
const GBNF_QUOTE: &str = r#""\"""#;

/// The characters of a text, by the bytes each takes (docs/adr/0087, section
/// 1.1, item 3), as the writer spells them: `c1` a character of one byte
/// written raw (U+0001 to U+007F save `"` and `\`) or an escape, whose value is
/// one byte: `\"`, `\\`, the five short escapes, every other control character
/// `\u00xx` in lowercase hex, U+0000 left out; `c2` U+0080 to U+07FF; `c3`
/// U+0800 to U+FFFF without the surrogates; `c4` U+10000 to U+10FFFF.
fn gbnf_text_characters() -> [(&'static str, String); 4] {
    [
        (
            "c1",
            format!(
                "{} | {GBNF_ESCAPES} [1-7bef] | {GBNF_U001}",
                gbnf_class(&[(0x20, 0x21), (0x23, 0x5b), (0x5d, 0x7f)])
            ),
        ),
        ("c2", gbnf_class(&[(0x80, 0x7ff)])),
        ("c3", gbnf_class(&[(0x800, 0xd7ff), (0xe000, 0xffff)])),
        ("c4", gbnf_class(&[(0x1_0000, 0x10_ffff)])),
    ]
}

/// The escapes of the writer whose value is one byte: `\"`, `\\`, the five
/// short escapes, then `\u000` before the last digit of the controls up to
/// U+000F that have no short escape, which a class follows.
const GBNF_ESCAPES: &str = r#""\\" [\"\\bfnrt] | "\\u000""#;

/// The controls from U+0010 to U+001F, each `\u001` and a lowercase digit.
const GBNF_U001: &str = r#""\\u001" [0-9a-f]"#;

/// A GBNF class of the code point ranges `ranges`, each bound escaped.
fn gbnf_class(ranges: &[(u32, u32)]) -> String {
    let bound = |n: u32| escaped(char::from_u32(n).expect("a scalar value"), EscapeForm::Gbnf);
    let inside: String = ranges
        .iter()
        .map(|&(lo, hi)| format!("{}-{}", bound(lo), bound(hi)))
        .collect();
    format!("[{inside}]")
}

/// A character of a byte string's text, each one byte (U+0000 to U+00FF), as
/// the writer spells it: raw, or the escape a text's `c1` writes, U+0000
/// among them.
fn gbnf_byte() -> String {
    format!(
        "{} | {GBNF_ESCAPES} [0-7bef] | {GBNF_U001}",
        gbnf_class(&[(0x20, 0x21), (0x23, 0x5b), (0x5d, 0xff)])
    )
}

/// A number as RFC 8259 section 6 writes one.
const GBNF_NUMBER: &str = r#""-"? ( "0" | [1-9] [0-9]* ) ( "." [0-9]+ )? ( [eE] [-+]? [0-9]+ )?"#;

/// A GBNF literal of `text`, a JSON text: `"` and `\` escaped, a character
/// past ASCII escaped ([`escaped`]), and every other character as it is, a
/// JSON text holding no raw control character.
fn gbnf_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if matches!(c, '"' | '\\') {
            out.push('\\');
        }
        if c.is_ascii() {
            out.push(c);
        } else {
            out.push_str(&escaped(c, EscapeForm::Gbnf));
        }
    }
    out.push('"');
    out
}

/// Whether `expression` is the name of a rule alone.
fn is_rule_name(expression: &str) -> bool {
    !expression.is_empty()
        && expression
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The rules a GBNF grammar of a type gathers as its fold walks it
/// ([`gbnf_of`]): a rule for each record, each union and each list element,
/// the rule of each integer type and of a number, and the chains that count a
/// text's bytes, a byte string's bytes and a list's elements.
///
/// Every bound is a chain of rules, never a repetition `{0,N}`: llama.cpp's
/// parser makes a repetition past 2000 unbounded (`MAX_REPETITION_THRESHOLD`,
/// `src/llama-grammar.cpp:655` at acecd56032dd), so a chain is how a bound
/// holds there at any size, and it is the rules that parser writes for a
/// repetition. A repetition stands only in an integer's digits, at most 19.
///
/// A rule's name is a word of lowercase letters, digits and `-`, the one
/// alphabet the parser reads in a name (`is_word_char`): a record's rule, a
/// union's and a list element's are numbered in the order the walk meets them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GbnfRules {
    /// Each rule the walk named, a record's, a union's or a list element's,
    /// in the order it named them.
    named: Vec<(String, String)>,
    /// The rule of each record, by its key ([`TypeEnv::record_key`]).
    records: std::collections::BTreeMap<String, String>,
    /// The rule of each union, by its key (docs/adr/0092, section 3.1).
    unions: std::collections::BTreeMap<String, String>,
    /// The rule of each list element's expression that is no rule alone.
    items: std::collections::BTreeMap<String, String>,
    /// The most elements after the first a list of each element rule holds.
    tails: std::collections::BTreeMap<String, u32>,
    /// The most elements after the first a list of each element rule must hold:
    /// its lower count less one, the longest (docs/adr/0086, section 15.10).
    leasts: std::collections::BTreeMap<String, u32>,
    /// The rule of each integer type the grammar holds, by its name.
    integers: std::collections::BTreeMap<String, String>,
    /// Whether the grammar holds a real.
    number: bool,
    /// The largest bound of a text the grammar holds.
    text: Option<u32>,
    /// The largest bound of a byte string the grammar holds.
    bytes: Option<u32>,
}

/// `name ::= body` on a line of its own.
fn push_rule(out: &mut String, name: &str, body: &str) {
    out.push_str(name);
    out.push_str(" ::= ");
    out.push_str(body);
    out.push('\n');
}

/// The documents a grammar names a rule of its own for, each numbered apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocumentRule {
    Record,
    Union,
}

impl GbnfRules {
    /// The rule of the record or union `key` names, its body written by
    /// `body`, called with `key` on `on_path`, the first time the walk meets
    /// it: a document met twice has one rule, named before its body is
    /// written so the rules stand in the order the walk meets them.
    fn document_rule<B>(
        &mut self,
        kind: DocumentRule,
        key: String,
        on_path: &mut Vec<String>,
        body: B,
    ) -> Result<String, GrammarError>
    where
        B: FnOnce(&mut Self, &mut Vec<String>) -> Result<String, GrammarError>,
    {
        let (table, prefix) = match kind {
            DocumentRule::Record => (&mut self.records, "record"),
            DocumentRule::Union => (&mut self.unions, "union"),
        };
        if let Some(rule) = table.get(&key) {
            return Ok(rule.clone());
        }
        let rule = format!("{prefix}-{}", table.len() + 1);
        table.insert(key.clone(), rule.clone());
        let slot = self.named.len();
        self.named.push((rule.clone(), String::new()));
        on_path.push(key);
        let written = body(self, on_path)?;
        on_path.pop();
        self.named[slot].1 = written;
        Ok(rule)
    }

    /// The rule of a list element `expression` is: itself when it is a rule's
    /// name, else a rule of its own, one for each distinct expression.
    fn item_rule(&mut self, expression: String) -> String {
        if is_rule_name(&expression) {
            return expression;
        }
        if let Some(rule) = self.items.get(&expression) {
            return rule.clone();
        }
        let rule = format!("item-{}", self.items.len() + 1);
        self.items.insert(expression.clone(), rule.clone());
        self.named.push((rule.clone(), expression));
        rule
    }

    /// The grammar whose `root` rule is `root`, one rule a line: `root`, each
    /// rule the walk named in the order it named it, the chain of each list's
    /// element, each integer's rule, a number's, then the characters and the
    /// chain of the byte strings and of the texts.
    ///
    /// A text of at most `k` bytes is `tk ::= "" | c1 t(k-1) | c2 t(k-2) | c3
    /// t(k-3) | c4 t(k-4)`, the terms of a negative index left out: one chain,
    /// up to the largest bound, serves every text (docs/adr/0087, section 1.1,
    /// item 3). A byte string's is `bk ::= "" | byte b(k-1)`, and the elements
    /// after a list's first `X-tail-k ::= "" | "," X X-tail-(k-1)`; the elements
    /// a list must hold after its first, by its lower count, are
    /// `X-least-k ::= "," X X-least-(k-1)` down to `X-least-1 ::= "," X`.
    pub fn grammar(&self, root: &str) -> String {
        let mut out = String::new();
        push_rule(&mut out, "root", root);
        for (name, body) in &self.named {
            push_rule(&mut out, name, body);
        }
        for (item, most) in &self.leasts {
            push_rule(
                &mut out,
                &format!("{item}-least-1"),
                &format!("\",\" {item}"),
            );
            for k in 2..=*most {
                push_rule(
                    &mut out,
                    &format!("{item}-least-{k}"),
                    &format!("\",\" {item} {item}-least-{}", k - 1),
                );
            }
        }
        for (item, most) in &self.tails {
            push_rule(&mut out, &format!("{item}-tail-0"), "\"\"");
            for k in 1..=*most {
                push_rule(
                    &mut out,
                    &format!("{item}-tail-{k}"),
                    &format!("\"\" | \",\" {item} {item}-tail-{}", k - 1),
                );
            }
        }
        for (name, body) in &self.integers {
            push_rule(&mut out, name, body);
        }
        if self.number {
            push_rule(&mut out, "number", GBNF_NUMBER);
        }
        if let Some(most) = self.bytes {
            push_rule(&mut out, "byte", &gbnf_byte());
            push_rule(&mut out, "b0", "\"\"");
            for k in 1..=most {
                push_rule(
                    &mut out,
                    &format!("b{k}"),
                    &format!("\"\" | byte b{}", k - 1),
                );
            }
        }
        if let Some(most) = self.text {
            for (name, body) in gbnf_text_characters() {
                push_rule(&mut out, name, &body);
            }
            push_rule(&mut out, "t0", "\"\"");
            for k in 1..=most {
                let mut body = "\"\"".to_string();
                for width in 1..=k.min(4) {
                    body.push_str(&format!(" | c{width} t{}", k - width));
                }
                push_rule(&mut out, &format!("t{k}"), &body);
            }
        }
        out
    }
}

/// `G(T)`: the GBNF expression of a value of `ty`, the rules it names gathered
/// in `rules` (docs/adr/0087, section 1.1).
///
/// - a number of at most 32 bits is the decimal spelling of exactly its
///   type's range, by digit-prefix alternatives; a 64-bit one the same
///   between quotes;
/// - a real is RFC 8259's number: a `float32`'s magnitude is the one bound
///   the grammar does not hold, and the reader does;
/// - `bool` is `true` or `false`; a closed enum one quoted name of its own;
/// - a text and a byte string are quoted, their bytes counted exactly;
/// - an optional value is `null` or its element's;
/// - a record is its rule: `{`, each key in the order its document declares
///   them with its value, `,` between two, `}`;
/// - a union is its rule, one alternative per case ([`gbnf_union`]);
/// - a list is `[`, at least its lower count and at most its bound of its
///   element's, `,` between two, `]`.
pub fn gbnf_of(
    ty: &ValueType,
    env: &dyn TypeEnv,
    rules: &mut GbnfRules,
) -> Result<String, GrammarError> {
    gbnf(ty, env, rules, &mut Vec::new())
}

/// `G` of a record of `fields`: what a payload's `root` is and a record's rule
/// holds.
pub fn gbnf_object(
    fields: &[(String, ValueType)],
    env: &dyn TypeEnv,
    rules: &mut GbnfRules,
) -> Result<String, GrammarError> {
    gbnf_record(None, fields, env, rules, &mut Vec::new())
}

/// `G` of a value of the union `union` as the whole of a grammar: what a
/// union document's `root` is and a union's rule holds (docs/adr/0092,
/// section 3.1). One alternative per case, in its tag enum's declaration
/// order: `{`, the tag's key and the variant's quoted name, then each field of
/// the case's record in the order its document declares them with its value,
/// `,` between two, `}`; an empty case is its tag alone. Every alternative
/// begins with its tag, so the variant's name a decoder writes decides the
/// case, and every token after it is held to that case's fields.
pub fn gbnf_union(union: &ScopedUnion, rules: &mut GbnfRules) -> Result<String, GrammarError> {
    gbnf_cases(union, rules, &mut whole_union(union)?)
}

/// The alternation of the cases of `union` ([`gbnf_union`]).
fn gbnf_cases(
    union: &ScopedUnion,
    rules: &mut GbnfRules,
    on_path: &mut Vec<String>,
) -> Result<String, GrammarError> {
    let alternatives = fold_cases(union, on_path, |case, fields, scope, on_path| {
        let tag = format!(
            "{}:{}",
            json_quoted(&union.tag),
            json_quoted(&case.variant.name)
        );
        gbnf_record(Some(tag.as_str()), fields, scope, rules, on_path)
    })?;
    Ok(alternatives.join(" | "))
}

/// `{`, `lead` where it is given (a union case's tag and its value, as JSON
/// text), each key of `fields` with its value, `,` between two, `}`.
fn gbnf_record(
    lead: Option<&str>,
    fields: &[(String, ValueType)],
    env: &dyn TypeEnv,
    rules: &mut GbnfRules,
    on_path: &mut Vec<String>,
) -> Result<String, GrammarError> {
    // The keys in the order the record's document declares them, the order
    // the JSON Schema's `properties` states them in: a decoder held to either
    // form writes them so (docs/adr/0087, section 1.1, item 2). A case's tag
    // stands before them in both (docs/adr/0092, section 3.1, item 1).
    let mut parts = Vec::new();
    let mut literal = String::from("{");
    if let Some(lead) = lead {
        literal.push_str(lead);
    }
    for (at, (id, field)) in fields.iter().enumerate() {
        if at > 0 || lead.is_some() {
            literal.push(',');
        }
        literal.push_str(&json_quoted(id));
        literal.push(':');
        parts.push(gbnf_literal(&literal));
        literal.clear();
        parts.push(gbnf(field, env, rules, on_path)?);
    }
    literal.push('}');
    parts.push(gbnf_literal(&literal));
    Ok(parts.join(" "))
}

fn gbnf(
    ty: &ValueType,
    env: &dyn TypeEnv,
    rules: &mut GbnfRules,
    on_path: &mut Vec<String>,
) -> Result<String, GrammarError> {
    Ok(match ty {
        ValueType::Scalar(t) => gbnf_scalar(t, env, rules)?,
        ValueType::Text { max } => {
            rules.text = rules.text.max(Some(max.max));
            format!("{GBNF_QUOTE} t{} {GBNF_QUOTE}", max.max)
        }
        ValueType::Bytes { max } => {
            rules.bytes = rules.bytes.max(Some(max.max));
            format!("{GBNF_QUOTE} b{} {GBNF_QUOTE}", max.max)
        }
        ValueType::Optional { elem } => {
            format!("( \"null\" | {} )", gbnf(elem, env, rules, on_path)?)
        }
        ValueType::List { elem, max } => match max.max.checked_sub(1) {
            // A list of at most no element is the empty list.
            None => "\"[]\"".to_string(),
            // A list that states no lower count may be empty.
            Some(more) if max.min == 0 => {
                let element = gbnf(elem, env, rules, on_path)?;
                let item = rules.item_rule(element);
                let most = rules.tails.entry(item.clone()).or_insert(0);
                *most = (*most).max(more);
                format!("\"[\" ( {item} {item}-tail-{more} )? \"]\"")
            }
            // A list of at least `min` elements writes its first, the
            // `min - 1` it must hold after it, then at most the rest of its
            // bound (docs/adr/0086, section 15.10): the empty list, and every
            // shorter one, is no sentence of it. A chain of no element is
            // left out.
            Some(more) => {
                let element = gbnf(elem, env, rules, on_path)?;
                let item = rules.item_rule(element);
                let required = max.min - 1;
                let optional = more.saturating_sub(required);
                let mut parts = vec!["\"[\"".to_string(), item.clone()];
                if required > 0 {
                    let most = rules.leasts.entry(item.clone()).or_insert(0);
                    *most = (*most).max(required);
                    parts.push(format!("{item}-least-{required}"));
                }
                if optional > 0 {
                    let most = rules.tails.entry(item.clone()).or_insert(0);
                    *most = (*most).max(optional);
                    parts.push(format!("{item}-tail-{optional}"));
                }
                parts.push("\"]\"".to_string());
                parts.join(" ")
            }
        },
        ValueType::Record { alias } => {
            let GrammarRecord {
                key, fields, scope, ..
            } = grammar_record(alias, env, on_path)?;
            rules.document_rule(DocumentRule::Record, key, on_path, |rules, on_path| {
                gbnf_record(None, &fields, scope, rules, on_path)
            })?
        }
        // docs/adr/0092, section 3.1: one rule per union document, one
        // alternative per case.
        ValueType::Union { alias } => {
            let union = grammar_union(alias, env, on_path)?;
            rules.document_rule(
                DocumentRule::Union,
                union.key(alias),
                on_path,
                |rules, on_path| gbnf_cases(union, rules, on_path),
            )?
        }
    })
}

/// `G` of a scalar.
fn gbnf_scalar(
    t: &SceType,
    env: &dyn TypeEnv,
    rules: &mut GbnfRules,
) -> Result<String, GrammarError> {
    Ok(match t {
        SceType::Uint8
        | SceType::Uint16
        | SceType::Uint32
        | SceType::Int8
        | SceType::Int16
        | SceType::Int32 => {
            let name = t.as_attr();
            let (lo, hi) = integer_bounds(t);
            rules
                .integers
                .entry(name.clone())
                .or_insert_with(|| gbnf_range(lo, hi));
            name
        }
        // The quoted text of its digits (docs/adr/0086, section 3.1).
        SceType::Uint64 | SceType::Int64 => {
            let name = t.as_attr();
            let (lo, hi) = integer_bounds(t);
            rules
                .integers
                .entry(name.clone())
                .or_insert_with(|| format!("{GBNF_QUOTE} ( {} ) {GBNF_QUOTE}", gbnf_range(lo, hi)));
            name
        }
        SceType::Float32 | SceType::Float64 => {
            rules.number = true;
            "number".to_string()
        }
        SceType::Bool => "( \"true\" | \"false\" )".to_string(),
        SceType::Enum(reference) => {
            let names: Vec<String> = closed_enum(reference, env)?
                .variants
                .iter()
                .map(|v| gbnf_literal(&json_quoted(&v.name)))
                .collect();
            format!("( {} )", names.join(" | "))
        }
        SceType::String | SceType::Bytes => {
            return Err(GrammarError::Unbounded {
                constructor: t.as_attr(),
            })
        }
    })
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
    /// Whether the union `from` names is the union `to` names: one document,
    /// or two revisions of one union (docs/adr/0092, section 6.1, item 7).
    fn same_union(&self, from: &str, to: &str) -> bool;
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
    /// The value's list may hold fewer elements than the lower count of the
    /// place expected (docs/adr/0086, section 15.6).
    LowerCount,
    /// The value may be absent and the place expected may not.
    MayBeAbsent,
    /// Two records, two enums or two unions, of different documents.
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
            MisfitReason::LowerCount => format!(
                "'{place}' is a {}, which may hold fewer than the {} expected there",
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
///   ([`TypeIdentity::same_record`]);
/// - `union:U` fits `union:V` when both are one union
///   ([`TypeIdentity::same_union`], docs/adr/0092, section 6.1, item 7).
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
            if a.max > b.max {
                Err(misfit(MisfitReason::Bound))
            } else if a.min < b.min {
                Err(misfit(MisfitReason::LowerCount))
            } else {
                Ok(())
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
        (ValueType::Union { alias: u }, ValueType::Union { alias: v }) => {
            if identity.same_union(u, v) {
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
            | ValueType::Record { .. }
            | ValueType::Union { .. },
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

    fn scoped_enum(underlying: SceType, variants: &[(&str, i128)], strict: bool) -> ScopedEnum {
        ScopedEnum {
            name: "level".to_string(),
            variants: variants
                .iter()
                .map(|(name, value)| ScopedVariant {
                    name: name.to_string(),
                    value: *value,
                })
                .collect(),
            underlying,
            strict,
        }
    }

    /// An enum that holds only the numbers its variants declare.
    fn closed_enum(underlying: SceType, variants: &[(&str, i128)]) -> ScopedEnum {
        scoped_enum(underlying, variants, true)
    }

    /// An enum that holds every number of its underlying type
    /// (`sce:strict-variants="false"`).
    fn open_enum(underlying: SceType, variants: &[(&str, i128)]) -> ScopedEnum {
        scoped_enum(underlying, variants, false)
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

    /// A list states its lower count before `..` in its one bracket
    /// (docs/adr/0086, section 15.2), a named bound after it, at any depth; a
    /// lower count of zero is the list that states none, and is not printed.
    #[test]
    fn a_list_states_its_lower_count_beside_its_bound() {
        let read = |text: &str| WrittenType::read(text).unwrap().bounded().unwrap();
        let ValueType::List { max, .. } = read("list<uint8>[3..8]") else {
            panic!("a list");
        };
        assert_eq!(max, Bound::of(8).at_least(3));
        assert_eq!(read("list<uint8>[3..8]").to_string(), "list<uint8>[3..8]");
        assert_eq!(
            read("list<uint8>[ 3 .. 8 ]").to_string(),
            "list<uint8>[3..8]"
        );
        // `[M..M]`: exactly M.
        assert_eq!(read("list<uint8>[2..2]").to_string(), "list<uint8>[2..2]");
        assert_eq!(read("list<uint8>[0..8]"), read("list<uint8>[8]"));
        assert_eq!(read("list<uint8>[0..8]").to_string(), "list<uint8>[8]");
        assert_ne!(read("list<uint8>[1..8]"), read("list<uint8>[8]"));
        let ValueType::List { max, .. } = read("list<record:Op>[1..shared_parts=64]") else {
            panic!("a list");
        };
        assert_eq!(max, Bound::named("shared_parts", 64).at_least(1));
        assert_eq!(
            read("list<record:Op>[1..shared_parts=64]").to_string(),
            "list<record:Op>[1..shared_parts=64]"
        );
        assert_eq!(
            read("list<list<string[8]>[1..4]>[2..3]").to_string(),
            "list<list<string[8]>[1..4]>[2..3]"
        );
        // A limit name that holds `..` keeps the reading it had.
        assert_eq!(
            read("list<uint8>[a..b=4]"),
            ValueType::List {
                elem: Box::new(ValueType::Scalar(SceType::Uint8)),
                max: Bound::named("a..b", 4),
            }
        );
    }

    /// Each lower count that leaves no type, or that no reader reads in a
    /// bracket, is refused as what it breaks (docs/adr/0086, section 15.2).
    #[test]
    fn an_ill_formed_lower_count_is_refused_by_what_it_breaks() {
        for text in [
            "list<uint8>[4..3]",
            "list<uint8>[9..n=8]",
            "list<list<uint8>[3..2]>[4]",
        ] {
            assert!(
                matches!(
                    WrittenType::read(text),
                    Err(TypeRefusal::CountPastBound { .. })
                ),
                "{text}: {:?}",
                WrittenType::read(text)
            );
        }
        assert_eq!(
            WrittenType::read("list<uint8>[4..3]"),
            Err(TypeRefusal::CountPastBound {
                place: "list<uint8>[4..3]".into(),
                least: 4,
                most: 3,
            })
        );
        for inside in [
            "-1..3",
            "..3",
            "3..",
            "x..3",
            "3..0",
            "4294967296..4294967295",
            "1..2..3",
        ] {
            let text = format!("list<uint8>[{inside}]");
            assert!(
                matches!(WrittenType::read(&text), Err(TypeRefusal::BadBound { .. })),
                "{text}: {:?}",
                WrittenType::read(&text)
            );
        }
        for text in ["string[1..8]", "bytes[0..4]", "list<string[1..8]>[3]"] {
            assert!(
                matches!(
                    WrittenType::read(text),
                    Err(TypeRefusal::CountMisplaced {
                        why: CountPlace::Length,
                        ..
                    })
                ),
                "{text}: {:?}",
                WrittenType::read(text)
            );
        }
        assert!(matches!(
            WrittenType::read("uint8[1..2]"),
            Err(TypeRefusal::BoundOnUnboundable { .. })
        ));
    }

    /// A build that moves a named bound below the lower count leaves a type
    /// with no value, refused where every bound is required; the lower count
    /// itself no build moves (docs/adr/0086, section 15.2).
    #[test]
    fn a_limit_moved_below_the_lower_count_leaves_no_type() {
        let written = WrittenType::read("list<uint8>[3..n=8]").unwrap();
        let moved = written.clone().resolve_limits(&mut |_, _| 5);
        assert_eq!(moved.to_string(), "list<uint8>[3..n=5]");
        assert!(moved.bounded().is_ok());
        let below = written.resolve_limits(&mut |_, _| 2);
        assert_eq!(
            below.bounded(),
            Err(TypeRefusal::CountPastBound {
                place: "list<uint8>[3..n=2]".into(),
                least: 3,
                most: 2,
            })
        );
    }

    /// A place filled an element at a time finds the outermost lower count
    /// its own type states, at any depth (docs/adr/0086, section 15.7).
    #[test]
    fn a_lower_count_in_a_built_place_is_found_at_any_depth() {
        let found = |text: &str| WrittenType::read(text).unwrap().lower_count_inside();
        assert_eq!(found("list<uint8>[8]"), None);
        assert_eq!(found("list<uint8>[0..8]"), None);
        assert_eq!(found("optional<list<uint8>[8]>"), None);
        assert_eq!(
            found("list<uint8>[1..8]"),
            Some(TypeRefusal::CountMisplaced {
                place: "list<uint8>[1..8]".into(),
                why: CountPlace::Built,
            })
        );
        assert_eq!(
            found("optional<list<list<uint8>[2..3]>[4]>"),
            Some(TypeRefusal::CountMisplaced {
                place: "list<uint8>[2..3]".into(),
                why: CountPlace::Built,
            })
        );
        // A list with no bracket, a parameter's, states no lower count.
        assert_eq!(found("list<uint8>"), None);
    }

    /// The replacement a refusal of a misplaced lower count offers is the text
    /// as written with every lower count removed, nothing else moved
    /// (docs/adr/0086, section 15.13).
    #[test]
    fn a_lower_count_is_removed_from_the_spelling_it_stands_in() {
        assert_eq!(
            without_lower_counts("list&lt;uint8&gt;[1..3]"),
            "list&lt;uint8&gt;[3]"
        );
        assert_eq!(
            without_lower_counts("list<list<string[8]>[1..4]>[2..n=3]"),
            "list<list<string[8]>[4]>[n=3]"
        );
        assert_eq!(without_lower_counts("string[1..8]"), "string[8]");
        assert_eq!(without_lower_counts("list<uint8>[3]"), "list<uint8>[3]");
        // A limit name that holds `..` is no lower count.
        assert_eq!(
            without_lower_counts("list<uint8>[a..b=4]"),
            "list<uint8>[a..b=4]"
        );
        // What it gives is read as the type without its lower counts.
        assert_eq!(
            WrittenType::read(&without_lower_counts("list<list<uint8>[1..2]>[1..3]"))
                .unwrap()
                .lower_count_inside(),
            None
        );
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
    fn an_entry_is_a_leaf_or_an_optional_leaf_and_names_any_composition() {
        let read = |t: &str| WrittenType::read(t).unwrap().bounded().unwrap();
        assert_eq!(
            read("string[8]").entry(),
            Ok((EntryType::Scalar(SceType::String), Some(&Bound::of(8))))
        );
        assert_eq!(
            read("optional<bytes[n=4]>").entry(),
            Ok((
                EntryType::Optional {
                    elem: SceType::Bytes
                },
                Some(&Bound::named("n", 4))
            ))
        );
        assert_eq!(
            read("enum:Level").entry(),
            Ok((EntryType::Scalar(enum_ty("Level")), None))
        );
        for (text, place, constructor) in [
            ("list<uint8>[2]", "list<uint8>[2]", "list"),
            ("record:Item", "record:Item", "record"),
            ("optional<list<uint8>[2]>", "list<uint8>[2]", "list"),
            ("optional<record:Item>", "record:Item", "record"),
        ] {
            assert_eq!(
                read(text).entry(),
                Err(Composition {
                    place: place.into(),
                    constructor: constructor.into()
                }),
                "{text}"
            );
        }
    }

    #[test]
    fn a_record_is_sized_by_the_enums_its_own_document_names() {
        let mut item_scope = TypeScope::default();
        item_scope.enums.insert(
            "Level".into(),
            closed_enum(SceType::Uint16, &[("low", 0), ("critical", 300)]),
        );
        // The document that holds the record calls another enum `Level`.
        let mut outer = TypeScope::default();
        outer
            .enums
            .insert("Level".into(), closed_enum(SceType::Uint16, &[("a", 0)]));
        outer.records.insert(
            "Item".into(),
            ScopedRecord {
                identity: "item.scxml".into(),
                name: "item".into(),
                fields: vec![("level".into(), ValueType::Scalar(enum_ty("Level")))],
                scope: item_scope,
            },
        );
        let item = ValueType::Record {
            alias: "Item".into(),
        };
        // `{"level":"critical"}`: the braces, the quoted key and its colon, and
        // the longest name of the record's own enum, quoted.
        assert_eq!(json_max_len(&item, &outer), Ok(2 + (2 + 5 + 1) + (2 + 8)));
        // The map's head, the key, and `critical`'s number 300 with its head.
        assert_eq!(cbor_max_len(&item, &outer), Ok(1 + (1 + 5) + 3));
        assert_eq!(contains_itself("Item", &outer), None);
    }

    #[test]
    fn an_optional_is_found_at_every_depth() {
        let read = |t: &str| WrittenType::read(t).unwrap().bounded().unwrap();
        assert!(read("optional<int64>").holds_optional());
        assert!(read("list<optional<uint16>>[2]").holds_optional());
        assert!(read("list<list<optional<bool>>[2]>[2]").holds_optional());
        assert!(!read("list<list<uint8>[2]>[2]").holds_optional());
        assert!(!read("record:Item").holds_optional());
    }

    #[test]
    fn a_type_crosses_the_ast_in_one_shape_its_bounds_inside_it() {
        let read = |t: &str| WrittenType::read(t).unwrap().bounded().unwrap();
        let json = |t: &str| serde_json::to_value(read(t)).unwrap();
        assert_eq!(json("int64"), serde_json::json!("int64"));
        assert_eq!(json("string[n=8]"), serde_json::json!({"string": 8}));
        assert_eq!(json("bytes[4]"), serde_json::json!({"bytes": 4}));
        assert_eq!(
            json("optional<int64>"),
            serde_json::json!({"optional": "int64"})
        );
        assert_eq!(json("record:Item"), serde_json::json!({"record": "Item"}));
        assert_eq!(
            json("list<list<string[8]>[4]>[3]"),
            serde_json::json!({"list": {"list": {"string": 8}, "max": 4}, "max": 3})
        );
        // A lower count stands beside the bound, and one of zero is not
        // written (docs/adr/0086, section 15.3).
        assert_eq!(
            json("list<list<uint8>[1..4]>[2..n=3]"),
            serde_json::json!({"list": {"list": "uint8", "max": 4, "min": 1}, "max": 3, "min": 2})
        );
        assert_eq!(
            json("list<uint8>[0..3]"),
            serde_json::json!({"list": "uint8", "max": 3})
        );
    }

    #[test]
    fn a_slot_holds_what_its_lowerings_hold_and_names_any_composition() {
        // The oracle, from the text the enumeration built: every leaf is held;
        // a list of a fixed-width number, `bool` or a record is held; an
        // optional over any leaf is held; anything else is a composition named
        // by its element.
        let held_list_elements = ["uint8", "int64", "float64", "bool", "record:Item"];
        let expected = |text: &str| -> Result<(), String> {
            if let Some(inner) = text
                .strip_prefix("optional<")
                .and_then(|t| t.strip_suffix('>'))
            {
                let leaf = LEAVES
                    .iter()
                    .any(|(leaf, _)| inner == *leaf || inner.strip_prefix(leaf) == Some("[8]"));
                return if leaf { Ok(()) } else { Err(inner.to_string()) };
            }
            if let Some(rest) = text.strip_prefix("list<") {
                let inner = rest
                    .trim_end_matches("[3]")
                    .strip_suffix('>')
                    .expect("a list's element is closed");
                return if held_list_elements.contains(&inner) {
                    Ok(())
                } else {
                    Err(inner.to_string())
                };
            }
            Ok(())
        };
        let mut held = 0;
        let mut refused = 0;
        for case in cases(3).into_iter().filter(|c| c.well_formed) {
            let ty = WrittenType::read(&case.text).unwrap();
            let view = ty.slot_view().map(|_| ()).map_err(|c| c.place);
            assert_eq!(view, expected(&case.text), "{}", case.text);
            if view.is_ok() {
                held += 1;
                assert_eq!(WrittenType::from_attr(&case.text), Some(ty.clone()));
            } else {
                refused += 1;
                assert_eq!(WrittenType::from_attr(&case.text), None, "{}", case.text);
            }
        }
        assert!(held > 0 && refused > 0, "{held} held, {refused} refused");
    }

    #[test]
    fn the_outer_bound_of_a_slot_is_split_off_and_folds_back() {
        for case in cases(3).into_iter().filter(|c| c.well_formed) {
            let ty = WrittenType::read(&case.text).unwrap();
            let outer = ty.outer_bound().cloned().flatten();
            let (bare, bound) = ty.clone().split_outer_bound();
            assert_eq!(bound, outer, "{}", case.text);
            assert_eq!(ty.stated_outer_bound(), outer.as_ref(), "{}", case.text);
            assert_eq!(ty.without_outer_bound(), bare, "{}", case.text);
            assert_eq!(bare.outer_bound().cloned().flatten(), None, "{}", case.text);
            match bound {
                Some(bound) => assert_eq!(bare.with_outer_bound(bound), Ok(ty), "{}", case.text),
                None => assert_eq!(bare, ty, "{}", case.text),
            }
        }
    }

    #[test]
    fn a_scalar_is_one_type_and_reads_back_as_that_scalar() {
        let scalars = [
            SceType::Uint8,
            SceType::Uint64,
            SceType::Int32,
            SceType::Float64,
            SceType::Bool,
            SceType::String,
            SceType::Bytes,
            enum_ty("Level"),
        ];
        for t in scalars {
            let ty = WrittenType::of_scalar(t.clone());
            assert_eq!(ty.scalar(), Some(&t), "{t:?}");
            assert_eq!(
                matches!(ty, WrittenType::Scalar(_)),
                !matches!(t, SceType::String | SceType::Bytes),
                "a text or a byte string is never a Scalar: {t:?}"
            );
            assert_eq!(ty.is_append_buffer(), matches!(t, SceType::Bytes), "{t:?}");
            assert_eq!(ty.list_elem(), None);
            assert_eq!(ty.record_alias(), None);
            assert_eq!(ty.optional_elem(), None);
            assert_eq!(WrittenType::from_attr(&ty.as_attr()), Some(ty.clone()));
        }
        let list = WrittenType::read("list<record:Item>").unwrap();
        assert_eq!(
            list.list_elem().and_then(WrittenType::record_alias),
            Some("Item")
        );
        assert!(list.is_append_buffer());
        assert_eq!(list.scalar(), None);
        let optional = WrittenType::read("optional<string>").unwrap();
        assert_eq!(
            optional.optional_elem().and_then(WrittenType::scalar),
            Some(&SceType::String)
        );
        assert_eq!(optional.scalar(), None);
        assert_eq!(optional.record_alias(), None);
    }

    #[test]
    fn a_type_with_no_bound_crosses_the_ast_in_the_shape_it_had_before_bounds() {
        let json = |t: &str| serde_json::to_value(WrittenType::read(t).unwrap()).unwrap();
        assert_eq!(json("string"), serde_json::json!("string"));
        assert_eq!(json("bytes"), serde_json::json!("bytes"));
        assert_eq!(json("list<int64>"), serde_json::json!({"list": "int64"}));
        assert_eq!(
            json("optional<record:Item>"),
            serde_json::json!({"optional": {"record": "Item"}})
        );
        assert_eq!(json("string[8]"), serde_json::json!({"string": 8}));
        assert_eq!(
            json("list<uint8>[2]"),
            serde_json::json!({"list": "uint8", "max": 2})
        );
    }

    // ── The size calculus ─────────────────────────────────────────────────

    /// A table of records, enums and unions.
    #[derive(Default)]
    struct Table {
        records: BTreeMap<String, Vec<(String, ValueType)>>,
        enums: BTreeMap<String, ScopedEnum>,
        unions: BTreeMap<String, ScopedUnion>,
    }

    impl TypeEnv for Table {
        fn record_fields(&self, alias: &str) -> Option<Vec<(String, ValueType)>> {
            self.records.get(alias).cloned()
        }
        fn enum_type(&self, alias: &str) -> Option<&ScopedEnum> {
            self.enums.get(alias)
        }
        fn union_type(&self, alias: &str) -> Option<&ScopedUnion> {
            self.unions.get(alias)
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
            closed_enum(SceType::Uint8, &[("low", 0), ("medium", 1), ("high", 2)]),
        );
        // A closed enum whose longest number is a negative one, and an open
        // enum, which holds numbers no variant declares.
        table.enums.insert(
            "Reading".into(),
            closed_enum(SceType::Int8, &[("none", -25), ("some", 3)]),
        );
        table.enums.insert(
            "Code".into(),
            open_enum(SceType::Uint16, &[("ok", 0), ("busy", 1)]),
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
        // "medium" is the longest name; its numbers, the closed enum's only
        // values on CBOR, are each one byte.
        assert_eq!(both("enum:SignalLevel"), (8, 1));
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

    #[test]
    fn an_integer_is_as_long_on_cbor_as_the_argument_of_its_head() {
        let u64_max = i128::from(u64::MAX);
        for (n, len) in [
            (0, Some(1)),
            (23, Some(1)),
            (24, Some(2)),
            (-1, Some(1)),
            // Below zero the argument is `-1 - n`: -24 is written as 23.
            (-24, Some(1)),
            (-25, Some(2)),
            (-256, Some(2)),
            (-257, Some(3)),
            (u64_max, Some(9)),
            (-u64_max - 1, Some(9)),
            (u64_max + 1, None),
            (-u64_max - 2, None),
        ] {
            assert_eq!(cbor_integer_len(n), len, "{n}");
        }
    }

    /// `C` of `enum:E`, `E` alone in its table.
    fn enum_cbor_len(enumeration: ScopedEnum) -> Result<u64, SizeError> {
        let mut env = Table::default();
        env.enums.insert("E".into(), enumeration);
        cbor_max_len(&ValueType::Scalar(enum_ty("E")), &env)
    }

    #[test]
    fn a_closed_enum_is_as_long_on_cbor_as_its_longest_declared_number() {
        let digits: [(&str, i128); 4] = [("a", 0), ("b", 1), ("c", 2), ("d", 3)];
        assert_eq!(enum_cbor_len(closed_enum(SceType::Uint8, &digits)), Ok(1));
        assert_eq!(
            enum_cbor_len(closed_enum(SceType::Uint8, &[("a", 0), ("b", 24)])),
            Ok(2)
        );
        // Three bytes, not the five of a uint32.
        assert_eq!(
            enum_cbor_len(closed_enum(SceType::Uint32, &[("a", 0), ("b", 256)])),
            Ok(3)
        );
        // Major type 1: -25 is written with the argument 24.
        assert_eq!(
            enum_cbor_len(closed_enum(SceType::Int8, &[("a", -25), ("b", 3)])),
            Ok(2)
        );
        assert_eq!(
            enum_cbor_len(closed_enum(SceType::Int8, &[("a", -24), ("b", 3)])),
            Ok(1)
        );
    }

    #[test]
    fn an_open_enum_is_as_long_on_cbor_as_its_underlying_type() {
        let small: &[(&str, i128)] = &[("a", 0), ("b", 1)];
        let large: &[(&str, i128)] = &[("a", 0), ("b", 200)];
        for variants in [small, large] {
            assert_eq!(
                enum_cbor_len(open_enum(SceType::Uint8, variants)),
                Ok(2),
                "{variants:?}"
            );
        }
        assert_eq!(enum_cbor_len(open_enum(SceType::Int32, &[("a", 0)])), Ok(5));
    }

    #[test]
    fn an_enum_whose_numbers_no_head_holds_has_no_cbor_size() {
        assert_eq!(
            enum_cbor_len(closed_enum(
                SceType::Uint64,
                &[("a", i128::from(u64::MAX) + 1)]
            )),
            Err(SizeError::Overflow)
        );
        let unresolved: Result<u64, SizeError> = Err(SizeError::Unresolved { alias: "E".into() });
        // A closed enum with no variant was not read from its document.
        assert_eq!(enum_cbor_len(closed_enum(SceType::Uint8, &[])), unresolved);
        // An enum whose numbers are an enum is not resolved, not recursed into.
        assert_eq!(
            enum_cbor_len(open_enum(enum_ty("E"), &[("a", 0)])),
            unresolved
        );
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
                    let variants = &env.enums[&r.alias].variants;
                    let longest = variants.iter().max_by_key(|v| v.name.len()).unwrap();
                    format!("\"{}\"", longest.name)
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
            // The longest of the cases, each the tag beside the case's fields
            // (docs/adr/0092, section 2.1). The cases' records here hold no
            // enum, so the table reads their fields as their own scope would.
            ValueType::Union { alias } => {
                let union = &env.unions[alias];
                union
                    .cases
                    .iter()
                    .map(|case| {
                        let mut members =
                            vec![format!("\"{}\":\"{}\"", union.tag, case.variant.name)];
                        if let Some(record) = &case.record {
                            members.extend(
                                union.scope.records[record]
                                    .fields
                                    .iter()
                                    .map(|(id, f)| format!("\"{id}\":{}", maximal_json(f, env))),
                            );
                        }
                        format!("{{{}}}", members.join(","))
                    })
                    .max_by_key(String::len)
                    .unwrap()
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
                    let enumeration = &env.enums[&r.alias];
                    if enumeration.strict {
                        // The declared number whose argument is greatest.
                        let (major, argument) = enumeration
                            .variants
                            .iter()
                            .map(|v| {
                                if v.value < 0 {
                                    (1u8, (-1 - v.value) as u64)
                                } else {
                                    (0u8, v.value as u64)
                                }
                            })
                            .max_by_key(|&(_, argument)| argument)
                            .unwrap();
                        cbor_head(major, argument)
                    } else {
                        maximal_cbor(&ValueType::Scalar(enumeration.underlying.clone()), env)
                    }
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
            // The longest of the cases, each a map of the tag, whose value is
            // the variant's declared number, and the case's fields
            // (docs/adr/0092, section 5.3).
            ValueType::Union { alias } => {
                let union = &env.unions[alias];
                union
                    .cases
                    .iter()
                    .map(|case| {
                        let fields: &[(String, ValueType)] = match &case.record {
                            Some(record) => &union.scope.records[record].fields,
                            None => &[],
                        };
                        let mut v = cbor_head(5, 1 + fields.len() as u64);
                        v.extend(cbor_head(3, union.tag.len() as u64));
                        v.extend(union.tag.as_bytes());
                        let number = case.variant.value;
                        v.extend(if number < 0 {
                            cbor_head(1, (-1 - number) as u64)
                        } else {
                            cbor_head(0, number as u64)
                        });
                        for (id, f) in fields {
                            v.extend(cbor_head(3, id.len() as u64));
                            v.extend(id.as_bytes());
                            v.extend(maximal_cbor(f, env));
                        }
                        v
                    })
                    .max_by_key(Vec::len)
                    .unwrap()
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
            "enum:Reading",
            "enum:Code",
            "list<enum:Code>[2]",
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

    /// `depth(T)` (docs/adr/0087, section 2.1, item 4): a record and a list
    /// each add a level, an optional value none, and a leaf is no container.
    #[test]
    fn a_container_is_as_deep_as_the_records_and_lists_on_the_way() {
        let mut env = table();
        env.records.insert(
            "Flat".into(),
            vec![
                ("id".into(), ty("uint32")),
                ("note".into(), ty("optional<string[8]>")),
            ],
        );
        env.records.insert(
            "Batch".into(),
            vec![
                ("items".into(), ty("list<record:Flat>[3]")),
                ("tags".into(), ty("optional<list<string[8]>[2]>")),
            ],
        );
        for (text, depth) in [
            ("uint8", 0),
            ("string[8]", 0),
            ("bytes[4]", 0),
            ("optional<int64>", 0),
            ("record:Flat", 1),
            ("optional<record:Flat>", 1),
            ("list<uint8>[2]", 1),
            ("list<list<uint8>[2]>[2]", 2),
            // `Item` holds a list of texts.
            ("record:Item", 2),
            ("record:Batch", 3),
        ] {
            assert_eq!(container_depth(&ty(text), &env), Ok(depth), "depth({text})");
        }
        env.records.insert(
            "Node".into(),
            vec![("next".into(), ty("optional<record:Node>"))],
        );
        assert!(matches!(
            container_depth(&ty("record:Node"), &env),
            Err(SizeError::ContainsItself { .. })
        ));
    }

    /// The longest pointer to a place a value declares (docs/adr/0087, section
    /// 2.4): each record's longest id, each list's largest index, `~` and `/`
    /// escaped, the first of two of one length.
    #[test]
    fn the_longest_pointer_is_the_longest_path_a_value_declares() {
        let mut env = table();
        env.records.insert(
            "Batch".into(),
            vec![
                ("items".into(), ty("list<record:Item>[12]")),
                ("a~b/c".into(), ty("uint8")),
            ],
        );
        env.records.insert(
            "Flat".into(),
            vec![("ab".into(), ty("uint8")), ("cd".into(), ty("bool"))],
        );
        let longest = |text: &str| longest_pointer(&ty(text), &env).unwrap();
        assert_eq!(longest("uint8"), (String::new(), 0));
        assert_eq!(longest("list<uint8>[1]"), ("/0".to_string(), 2));
        assert_eq!(longest("list<uint8>[10]"), ("/9".to_string(), 2));
        assert_eq!(longest("record:Flat"), ("/ab".to_string(), 3));
        assert_eq!(longest("record:Item"), ("/amount".to_string(), 7));
        // `/items/11/amount` (16) is longer than `/a~0b~1c` (8).
        assert_eq!(
            longest("record:Batch"),
            ("/items/11/amount".to_string(), 16)
        );
        env.records
            .insert("Odd".into(), vec![("a~b/c".into(), ty("uint8"))]);
        assert_eq!(
            longest_pointer(&ty("record:Odd"), &env).unwrap(),
            ("/a~0b~1c".to_string(), 8)
        );
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
    /// schema and `Q` another schema; `E` and `E2` one enum and `F` another;
    /// `U` and `U2` one union and `V` another.
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
        fn same_union(&self, from: &str, to: &str) -> bool {
            document_of(from) == document_of(to)
        }
    }

    /// Every value of `t`, each spelled once, independent of the relation: a
    /// number or `bool` is one value standing for every value of its type, a
    /// record or an enum one value of its document, a text or a byte string one
    /// value per length, an absent optional `null` beside the values of its
    /// element (the wire spells a present one as its element), and a list every
    /// sequence of values of its element from its lower count up to its bound.
    fn values_of(t: &ValueType) -> BTreeSet<String> {
        match t {
            ValueType::Scalar(SceType::Enum(r)) => {
                [format!("enum:{}", document_of(&r.alias))].into()
            }
            ValueType::Scalar(s) => [s.as_attr()].into(),
            ValueType::Record { alias } => [format!("record:{}", document_of(alias))].into(),
            ValueType::Union { alias } => [format!("union:{}", document_of(alias))].into(),
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
                let mut out: BTreeSet<String> = BTreeSet::new();
                if max.min == 0 {
                    out.insert("[]".to_string());
                }
                for length in 1..=max.max {
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
                    if length >= max.min {
                        out.extend(sequences.iter().map(|seq| format!("[{}]", seq.join(","))));
                    }
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

    /// A list's lower count is part of the relation (docs/adr/0086, section
    /// 15.6): over lists that state one and lists that do not, a type fits
    /// another exactly when every value of it is a value of the other, and a
    /// list that may hold fewer than the place's lower count is refused as
    /// that.
    #[test]
    fn a_lower_count_fits_a_place_that_needs_no_more() {
        let mut universe: Vec<String> = Vec::new();
        for leaf in ["uint8", "bool"] {
            for bracket in ["[1]", "[2]", "[1..1]", "[1..2]", "[2..2]", "[0..2]"] {
                universe.push(format!("list<{leaf}>{bracket}"));
                universe.push(format!("optional<list<{leaf}>{bracket}>"));
            }
        }
        universe.push("list<list<uint8>[1..2]>[2]".into());
        universe.push("list<list<uint8>[2]>[2]".into());
        let mut reasons = BTreeSet::new();
        for from_text in &universe {
            for to_text in &universe {
                let (from, to) = (ty(from_text), ty(to_text));
                let oracle = values_of(&from).is_subset(&values_of(&to));
                match fits(&from, &to, &Documents) {
                    Ok(()) => assert!(oracle, "{from_text} fits {to_text}"),
                    Err(misfit) => {
                        assert!(!oracle, "{from_text} does not fit {to_text}: {misfit:?}");
                        reasons.insert(format!("{:?}", misfit.why));
                    }
                }
            }
        }
        assert!(reasons.contains("LowerCount"), "{reasons:?}");
        let refused = |from: &str, to: &str| fits(&ty(from), &ty(to), &Documents).unwrap_err();
        let at = refused("list<uint8>[3]", "list<uint8>[1..3]");
        assert_eq!((at.path.as_str(), at.why), ("", MisfitReason::LowerCount));
        assert_eq!(
            at.sentence("items"),
            "'items' is a list<uint8>[3], which may hold fewer than the list<uint8>[1..3] \
             expected there"
        );
        let at = refused("list<list<uint8>[2]>[2]", "list<list<uint8>[1..2]>[2]");
        assert_eq!((at.path.as_str(), at.why), ("/*", MisfitReason::LowerCount));
        assert_eq!(
            fits(
                &ty("list<uint8>[2..3]"),
                &ty("list<uint8>[1..4]"),
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

    // ── The grammar of a value (docs/adr/0087, section 1) ────────────────

    /// The environment the grammar's goldens read: a closed enum `Kind`, an
    /// open enum `Code`, a closed enum with no variant, the record `Item`
    /// (document `item`) whose enum `Mood` only its own document names, a
    /// record of another document also named `item`, and a record that holds
    /// itself.
    fn grammar_scope() -> TypeScope {
        let mut item_scope = TypeScope::default();
        item_scope.enums.insert(
            "Mood".into(),
            closed_enum(SceType::Uint8, &[("calm", 0), ("tense", 1)]),
        );
        let mut scope = TypeScope::default();
        scope.enums.insert(
            "Kind".into(),
            closed_enum(SceType::Uint8, &[("meeting", 0), ("task", 1)]),
        );
        scope
            .enums
            .insert("Code".into(), open_enum(SceType::Uint8, &[("ok", 0)]));
        scope
            .enums
            .insert("Empty".into(), closed_enum(SceType::Uint8, &[]));
        scope.records.insert(
            "Item".into(),
            ScopedRecord {
                identity: "here/item.scxml".into(),
                name: "item".into(),
                fields: vec![("mood".into(), ty("enum:Mood")), ("id".into(), ty("uint8"))],
                scope: item_scope,
            },
        );
        scope.records.insert(
            "Other".into(),
            ScopedRecord {
                identity: "there/item.scxml".into(),
                name: "item".into(),
                fields: vec![("x".into(), ty("bool"))],
                scope: TypeScope::default(),
            },
        );
        let mut loop_scope = TypeScope::default();
        let looping = ScopedRecord {
            identity: "loop.scxml".into(),
            name: "loop".into(),
            fields: vec![("next".into(), ty("optional<record:Loop>"))],
            scope: TypeScope::default(),
        };
        loop_scope.records.insert("Loop".into(), looping.clone());
        scope.records.insert(
            "Loop".into(),
            ScopedRecord {
                scope: loop_scope,
                ..looping
            },
        );
        scope
    }

    /// `text`, a schema's text, read back as JSON.
    fn read_json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap_or_else(|e| panic!("not JSON ({e}): {text}"))
    }

    /// The text of what a schema holds under `$defs`.
    fn defs_text(defs: SchemaNode) -> String {
        let text = SchemaObject::default().with(Keyword::Defs, defs).text();
        let inner = text
            .strip_prefix("{\"$defs\":")
            .and_then(|t| t.strip_suffix('}'));
        inner.expect("one keyword").to_string()
    }

    /// `S` of `ty` in [`grammar_scope`], read back from its text, and the
    /// `$defs` it gathered.
    fn schema_of(
        ty: &ValueType,
    ) -> Result<(serde_json::Value, Option<serde_json::Value>), GrammarError> {
        let mut defs = SchemaDefs::default();
        let schema = json_schema_of(ty, &grammar_scope(), &mut defs)?;
        let gathered = defs.into_node().map(|d| read_json(&defs_text(d)));
        Ok((read_json(&schema.text()), gathered))
    }

    /// `S` of `ty` in [`grammar_scope`] as it is written.
    fn schema_text(text: &str) -> String {
        let mut defs = SchemaDefs::default();
        json_schema_of(&ty(text), &grammar_scope(), &mut defs)
            .expect("a grammar")
            .text()
    }

    /// The whole GBNF grammar whose root is `G` of `ty` in [`grammar_scope`].
    fn grammar_of(ty: &ValueType) -> Result<String, GrammarError> {
        let mut rules = GbnfRules::default();
        let root = gbnf_of(ty, &grammar_scope(), &mut rules)?;
        Ok(rules.grammar(&root))
    }

    fn json_of(text: &str) -> serde_json::Value {
        let (schema, defs) = schema_of(&ty(text)).expect("a grammar");
        assert_eq!(defs, None, "{text} holds no record");
        schema
    }

    fn gbnf_text(text: &str) -> String {
        grammar_of(&ty(text)).expect("a grammar")
    }

    /// The rule of `uint8`: 0, 1 to 99, 100 to 199, 200 to 249, 250 to 255.
    const UINT8_RULE: &str =
        r#"uint8 ::= "0" | [1-9] [0-9]{0,1} | "1" [0-9]{2} | "2" [0-4] [0-9] | "25" [0-5]"#;

    /// The characters of a text and the chain of a text of at most five bytes.
    const TEXT_TO_FIVE: &str = concat!(
        "c1 ::= [\\x20-\\x21\\x23-\\x5b\\x5d-\\x7f] | \"\\\\\" [\\\"\\\\bfnrt] | \"\\\\u000\" \
         [1-7bef] | \"\\\\u001\" [0-9a-f]\n",
        "c2 ::= [\\u0080-\\u07ff]\n",
        "c3 ::= [\\u0800-\\ud7ff\\ue000-\\uffff]\n",
        "c4 ::= [\\U00010000-\\U0010ffff]\n",
        "t0 ::= \"\"\n",
        "t1 ::= \"\" | c1 t0\n",
        "t2 ::= \"\" | c1 t1 | c2 t0\n",
        "t3 ::= \"\" | c1 t2 | c2 t1 | c3 t0\n",
        "t4 ::= \"\" | c1 t3 | c2 t2 | c3 t1 | c4 t0\n",
        "t5 ::= \"\" | c1 t4 | c2 t3 | c3 t2 | c4 t1\n",
    );
    #[test]
    fn a_number_of_at_most_32_bits_is_an_integer_between_its_ends() {
        for (text, lo, hi) in [
            ("uint8", 0i64, 255i64),
            ("uint16", 0, 65_535),
            ("uint32", 0, 4_294_967_295),
            ("int8", -128, 127),
            ("int16", -32_768, 32_767),
            ("int32", -2_147_483_648, 2_147_483_647),
        ] {
            assert_eq!(
                json_of(text),
                serde_json::json!({"type": "integer", "minimum": lo, "maximum": hi}),
                "{text}"
            );
        }
        // Its keywords in the one order every schema writes them in.
        assert_eq!(
            schema_text("int8"),
            "{\"type\":\"integer\",\"minimum\":-128,\"maximum\":127}"
        );
        assert_eq!(
            gbnf_text("uint8"),
            format!("root ::= uint8\n{UINT8_RULE}\n")
        );
        assert_eq!(
            gbnf_text("int8"),
            "root ::= int8\nint8 ::= \"-\" ( [1-9] [0-9]{0,1} | \"1\" [0-1] [0-9] | \"12\" [0-8] ) \
             | \"0\" | [1-9] [0-9]{0,1} | \"1\" [0-1] [0-9] | \"12\" [0-7]\n"
        );
        // The decomposition docs/adr/0087 section 1.1 writes out.
        assert_eq!(
            gbnf_text("uint16"),
            "root ::= uint16\nuint16 ::= \"0\" | [1-9] [0-9]{0,3} | [1-5] [0-9]{4} | \"6\" [0-4] \
             [0-9]{3} | \"65\" [0-4] [0-9]{2} | \"655\" [0-2] [0-9] | \"6553\" [0-5]\n"
        );
    }

    #[test]
    fn a_64_bit_integer_is_the_quoted_text_of_its_digits() {
        for text in ["uint64", "int64"] {
            let schema = json_of(text);
            assert_eq!(schema["type"], "string", "{text}");
            let pattern = schema["pattern"].as_str().expect("a pattern");
            assert_eq!(schema.as_object().map(|o| o.len()), Some(2), "{schema}");
            assert!(
                pattern.starts_with("^(") && pattern.ends_with(")$"),
                "{pattern}"
            );
            assert!(
                schema_text(text).starts_with("{\"type\":\"string\",\"pattern\":\"^("),
                "{text}"
            );
            let grammar = gbnf_text(text);
            assert!(
                grammar.starts_with(&format!("root ::= {text}\n{text} ::= \"\\\"\" ( ")),
                "{grammar}"
            );
            assert!(grammar.ends_with(" ) \"\\\"\"\n"), "{grammar}");
        }
        assert!(json_of("uint64")["pattern"]
            .as_str()
            .unwrap()
            .ends_with("|1844674407370955161[0-5])$"));
        assert!(json_of("int64")["pattern"]
            .as_str()
            .unwrap()
            .starts_with("^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|"));
    }

    /// The decomposition of a range spells exactly the integers in it, checked
    /// by a regular-expression engine that is not the generator, on every
    /// integer near each end and each power of ten.
    #[test]
    fn a_range_spells_exactly_the_integers_in_it() {
        for t in [
            SceType::Uint8,
            SceType::Uint16,
            SceType::Uint32,
            SceType::Uint64,
            SceType::Int8,
            SceType::Int16,
            SceType::Int32,
            SceType::Int64,
        ] {
            let (lo, hi) = integer_bounds(&t);
            let pattern = regex::Regex::new(&regex_range(lo, hi)).expect("a regular expression");
            let mut probes: Vec<i128> = vec![lo - 1, lo, lo + 1, -1, 0, 1, hi - 1, hi, hi + 1];
            let mut power: i128 = 1;
            while power <= hi.saturating_mul(10) && power < i128::MAX / 10 {
                for n in [power - 1, power, power + 1] {
                    probes.extend([n, -n]);
                }
                power *= 10;
            }
            for n in probes {
                assert_eq!(
                    pattern.is_match(&n.to_string()),
                    (lo..=hi).contains(&n),
                    "{} and {n}",
                    t.as_attr()
                );
            }
            for unwritten in ["", "-", "-0", "00", "01", "+1", " 1", "1 ", "1.0"] {
                assert!(
                    !pattern.is_match(unwritten),
                    "{} admits {unwritten:?}",
                    t.as_attr()
                );
            }
        }
        // Every value of the narrowest types, and one past each end.
        for t in [SceType::Uint8, SceType::Int8] {
            let (lo, hi) = integer_bounds(&t);
            let pattern = regex::Regex::new(&regex_range(lo, hi)).unwrap();
            for n in lo - 1000..=hi + 1000 {
                assert_eq!(pattern.is_match(&n.to_string()), (lo..=hi).contains(&n));
            }
        }
    }

    #[test]
    fn a_real_a_truth_value_and_an_enum_have_their_spelling() {
        let most = f64::from(f32::MAX);
        assert_eq!(
            json_of("float32"),
            serde_json::json!({"type": "number", "minimum": -most, "maximum": most})
        );
        let single = schema_text("float32");
        assert!(
            single.starts_with("{\"type\":\"number\",\"minimum\":-")
                && single.contains(",\"maximum\":"),
            "{single}"
        );
        assert_eq!(json_of("float64"), serde_json::json!({"type": "number"}));
        let number = "number ::= \"-\"? ( \"0\" | [1-9] [0-9]* ) ( \".\" [0-9]+ )? ( [eE] [-+]? \
                      [0-9]+ )?\n";
        for text in ["float32", "float64"] {
            assert_eq!(gbnf_text(text), format!("root ::= number\n{number}"));
        }
        assert_eq!(json_of("bool"), serde_json::json!({"type": "boolean"}));
        assert_eq!(gbnf_text("bool"), "root ::= ( \"true\" | \"false\" )\n");
        assert_eq!(
            json_of("enum:Kind"),
            serde_json::json!({"enum": ["meeting", "task"]})
        );
        assert_eq!(
            gbnf_text("enum:Kind"),
            "root ::= ( \"\\\"meeting\\\"\" | \"\\\"task\\\"\" )\n"
        );
    }

    #[test]
    fn a_text_counts_its_bound_in_bytes() {
        assert_eq!(
            json_of("string[16]"),
            serde_json::json!({"type": "string", "maxLength": 16, "x-sce-max-bytes": 16})
        );
        // The annotation stands beside the keyword it qualifies.
        assert_eq!(
            schema_text("string[16]"),
            "{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16}"
        );
        assert_eq!(
            gbnf_text("string[5]"),
            format!("root ::= \"\\\"\" t5 \"\\\"\"\n{TEXT_TO_FIVE}")
        );
        // Two texts share one chain, up to the larger bound.
        let mut rules = GbnfRules::default();
        let root = gbnf_object(
            &[("a".into(), ty("string[2]")), ("b".into(), ty("string[5]"))],
            &grammar_scope(),
            &mut rules,
        )
        .unwrap();
        assert_eq!(
            rules.grammar(&root),
            format!(
                "root ::= \"{{\\\"a\\\":\" \"\\\"\" t2 \"\\\"\" \",\\\"b\\\":\" \"\\\"\" t5 \
                 \"\\\"\" \"}}\"\n{TEXT_TO_FIVE}"
            )
        );
    }

    #[test]
    fn a_byte_string_is_a_text_of_one_byte_characters() {
        assert_eq!(
            json_of("bytes[4]"),
            serde_json::json!({"type": "string", "maxLength": 4, "pattern": "^[\\u0000-\\u00ff]*$"})
        );
        assert_eq!(
            schema_text("bytes[4]"),
            "{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"}"
        );
        assert_eq!(
            gbnf_text("bytes[3]"),
            "root ::= \"\\\"\" b3 \"\\\"\"\n\
             byte ::= [\\x20-\\x21\\x23-\\x5b\\x5d-\\u00ff] | \"\\\\\" [\\\"\\\\bfnrt] | \"\\\\u000\" \
             [0-7bef] | \"\\\\u001\" [0-9a-f]\n\
             b0 ::= \"\"\n\
             b1 ::= \"\" | byte b0\n\
             b2 ::= \"\" | byte b1\n\
             b3 ::= \"\" | byte b2\n"
        );
    }

    #[test]
    fn an_optional_value_is_null_or_its_element() {
        assert_eq!(
            json_of("optional<string[8]>"),
            serde_json::json!({"anyOf": [
                {"type": "string", "maxLength": 8, "x-sce-max-bytes": 8},
                {"type": "null"}
            ]})
        );
        assert_eq!(
            schema_text("optional<bool>"),
            "{\"anyOf\":[{\"type\":\"boolean\"},{\"type\":\"null\"}]}"
        );
        assert_eq!(
            gbnf_text("optional<bool>"),
            "root ::= ( \"null\" | ( \"true\" | \"false\" ) )\n"
        );
    }

    #[test]
    fn a_list_holds_at_most_its_bound() {
        assert_eq!(
            json_of("list<uint8>[3]"),
            serde_json::json!({
                "type": "array",
                "items": {"type": "integer", "minimum": 0, "maximum": 255},
                "maxItems": 3
            })
        );
        assert_eq!(
            schema_text("list<uint8>[3]"),
            "{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\
             \"maxItems\":3}"
        );
        assert_eq!(
            gbnf_text("list<uint8>[3]"),
            format!(
                "root ::= \"[\" ( uint8 uint8-tail-2 )? \"]\"\n\
                 uint8-tail-0 ::= \"\"\n\
                 uint8-tail-1 ::= \"\" | \",\" uint8 uint8-tail-0\n\
                 uint8-tail-2 ::= \"\" | \",\" uint8 uint8-tail-1\n\
                 {UINT8_RULE}\n"
            )
        );
        assert_eq!(
            gbnf_text("list<uint8>[1]"),
            format!("root ::= \"[\" ( uint8 uint8-tail-0 )? \"]\"\nuint8-tail-0 ::= \"\"\n{UINT8_RULE}\n")
        );
        // An element that is no rule alone is given one.
        assert_eq!(
            gbnf_text("list<bool>[2]"),
            "root ::= \"[\" ( item-1 item-1-tail-1 )? \"]\"\n\
             item-1 ::= ( \"true\" | \"false\" )\n\
             item-1-tail-0 ::= \"\"\n\
             item-1-tail-1 ::= \"\" | \",\" item-1 item-1-tail-0\n"
        );
        // Two lists of one element share one chain, up to the larger bound,
        // and one rule for the element.
        let mut rules = GbnfRules::default();
        let root = gbnf_object(
            &[
                ("a".into(), ty("list<bool>[2]")),
                ("b".into(), ty("list<bool>[3]")),
            ],
            &grammar_scope(),
            &mut rules,
        )
        .unwrap();
        assert_eq!(
            rules.grammar(&root),
            "root ::= \"{\\\"a\\\":\" \"[\" ( item-1 item-1-tail-1 )? \"]\" \",\\\"b\\\":\" \"[\" \
             ( item-1 item-1-tail-2 )? \"]\" \"}\"\n\
             item-1 ::= ( \"true\" | \"false\" )\n\
             item-1-tail-0 ::= \"\"\n\
             item-1-tail-1 ::= \"\" | \",\" item-1 item-1-tail-0\n\
             item-1-tail-2 ::= \"\" | \",\" item-1 item-1-tail-1\n"
        );
        // A list of at most no element, which no reader of a type builds, is
        // the empty list and names no rule of its element.
        let none = ValueType::List {
            elem: Box::new(ty("uint8")),
            max: Bound::of(0),
        };
        assert_eq!(grammar_of(&none), Ok("root ::= \"[]\"\n".to_string()));
    }

    /// A list's lower count is `minItems` in the JSON Schema and, in the GBNF
    /// grammar, the chain of the elements it must hold after its first, so no
    /// shorter list is a sentence of either (docs/adr/0086, sections 15.9 and
    /// 15.10).
    #[test]
    fn a_list_holds_at_least_its_lower_count() {
        assert_eq!(
            json_of("list<uint8>[2..3]"),
            serde_json::json!({
                "type": "array",
                "items": {"type": "integer", "minimum": 0, "maximum": 255},
                "minItems": 2,
                "maxItems": 3
            })
        );
        // `minItems` stands between `items` and `maxItems`, in the one order.
        assert_eq!(
            schema_text("list<uint8>[2..3]"),
            "{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\
             \"minItems\":2,\"maxItems\":3}"
        );
        // A lower count of zero writes none.
        assert_eq!(
            schema_text("list<uint8>[0..3]"),
            schema_text("list<uint8>[3]")
        );
        // At least two of at most four: the first, one it must hold, at most two
        // more.
        assert_eq!(
            gbnf_text("list<uint8>[2..4]"),
            format!(
                "root ::= \"[\" uint8 uint8-least-1 uint8-tail-2 \"]\"\n\
                 uint8-least-1 ::= \",\" uint8\n\
                 uint8-tail-0 ::= \"\"\n\
                 uint8-tail-1 ::= \"\" | \",\" uint8 uint8-tail-0\n\
                 uint8-tail-2 ::= \"\" | \",\" uint8 uint8-tail-1\n\
                 {UINT8_RULE}\n"
            )
        );
        // At least one: the first, then at most the rest; no chain it must hold.
        assert_eq!(
            gbnf_text("list<uint8>[1..3]"),
            format!(
                "root ::= \"[\" uint8 uint8-tail-2 \"]\"\n\
                 uint8-tail-0 ::= \"\"\n\
                 uint8-tail-1 ::= \"\" | \",\" uint8 uint8-tail-0\n\
                 uint8-tail-2 ::= \"\" | \",\" uint8 uint8-tail-1\n\
                 {UINT8_RULE}\n"
            )
        );
        // Exactly three: no chain it may hold.
        assert_eq!(
            gbnf_text("list<uint8>[3..3]"),
            format!(
                "root ::= \"[\" uint8 uint8-least-2 \"]\"\n\
                 uint8-least-1 ::= \",\" uint8\n\
                 uint8-least-2 ::= \",\" uint8 uint8-least-1\n\
                 {UINT8_RULE}\n"
            )
        );
        assert_eq!(
            gbnf_text("list<uint8>[1..1]"),
            format!("root ::= \"[\" uint8 \"]\"\n{UINT8_RULE}\n")
        );
        // A lower count of zero is the grammar it always was.
        assert_eq!(gbnf_text("list<uint8>[0..3]"), gbnf_text("list<uint8>[3]"));
        // An element that is no rule alone is given one, and the chains are
        // its rule's.
        assert_eq!(
            gbnf_text("list<bool>[2..2]"),
            "root ::= \"[\" item-1 item-1-least-1 \"]\"\n\
             item-1 ::= ( \"true\" | \"false\" )\n\
             item-1-least-1 ::= \",\" item-1\n"
        );
    }

    #[test]
    fn a_record_is_defined_once_with_every_key_and_no_other() {
        let (schema, defs) = schema_of(&ty("record:Item")).unwrap();
        assert_eq!(schema, serde_json::json!({"$ref": "#/$defs/item"}));
        // Its enum is read in the record's own document, and its keys stand in
        // the order that document declares them, `mood` before `id`, in the
        // definition and in the grammar.
        let item = "{\"type\":\"object\",\"properties\":{\"mood\":{\"enum\":[\"calm\",\"tense\"]},\
                    \"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\
                    \"required\":[\"mood\",\"id\"],\"additionalProperties\":false}";
        assert_eq!(defs, Some(read_json(&format!("{{\"item\":{item}}}"))));
        let mut gathered = SchemaDefs::default();
        json_schema_of(&ty("record:Item"), &grammar_scope(), &mut gathered).unwrap();
        assert_eq!(
            gathered.into_node().map(defs_text),
            Some(format!("{{\"item\":{item}}}"))
        );
        assert_eq!(
            gbnf_text("record:Item"),
            format!(
                "root ::= record-1\n\
                 record-1 ::= \"{{\\\"mood\\\":\" ( \"\\\"calm\\\"\" | \"\\\"tense\\\"\" ) \
                 \",\\\"id\\\":\" uint8 \"}}\"\n\
                 {UINT8_RULE}\n"
            )
        );
        // Met twice, it is defined once; the keys of the record that holds it
        // stand in the order of its fields, `b` before `a`.
        let twice = [
            ("b".to_string(), ty("record:Item")),
            ("a".to_string(), ty("list<optional<record:Item>>[2]")),
        ];
        let mut defs = SchemaDefs::default();
        let object = json_schema_object(&twice, &grammar_scope(), &mut defs).unwrap();
        assert_eq!(
            object.text(),
            "{\"type\":\"object\",\"properties\":{\"b\":{\"$ref\":\"#/$defs/item\"},\
             \"a\":{\"type\":\"array\",\"items\":{\"anyOf\":[{\"$ref\":\"#/$defs/item\"},\
             {\"type\":\"null\"}]},\"maxItems\":2}},\"required\":[\"b\",\"a\"],\
             \"additionalProperties\":false}"
        );
        let Some(SchemaNode::Named(definitions)) = defs.into_node() else {
            panic!("the definitions of a type that holds a record");
        };
        assert_eq!(definitions.len(), 1, "one definition");
        let mut rules = GbnfRules::default();
        let root = gbnf_object(&twice, &grammar_scope(), &mut rules).unwrap();
        assert_eq!(
            rules.grammar(&root),
            format!(
                "root ::= \"{{\\\"b\\\":\" record-1 \",\\\"a\\\":\" \"[\" ( item-1 item-1-tail-1 )? \
                 \"]\" \"}}\"\n\
                 record-1 ::= \"{{\\\"mood\\\":\" ( \"\\\"calm\\\"\" | \"\\\"tense\\\"\" ) \
                 \",\\\"id\\\":\" uint8 \"}}\"\n\
                 item-1 ::= ( \"null\" | record-1 )\n\
                 item-1-tail-0 ::= \"\"\n\
                 item-1-tail-1 ::= \"\" | \",\" item-1 item-1-tail-0\n\
                 {UINT8_RULE}\n"
            )
        );
        // A record of no field is the empty object.
        let mut defs = SchemaDefs::default();
        assert_eq!(
            read_json(
                &json_schema_object(&[], &grammar_scope(), &mut defs)
                    .unwrap()
                    .text()
            ),
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": false
            })
        );
        let mut rules = GbnfRules::default();
        assert_eq!(
            gbnf_object(&[], &grammar_scope(), &mut rules),
            Ok("\"{}\"".to_string())
        );
    }

    #[test]
    fn a_type_with_no_grammar_is_refused_by_name() {
        let both = |ty: ValueType| -> (GrammarError, GrammarError) {
            (
                schema_of(&ty).expect_err("no JSON Schema"),
                grammar_of(&ty).expect_err("no GBNF grammar"),
            )
        };
        let open = GrammarError::OpenEnum {
            alias: "Code".into(),
        };
        assert_eq!(both(ty("enum:Code")), (open.clone(), open.clone()));
        assert_eq!(both(ty("list<enum:Code>[2]")), (open.clone(), open));
        for alias in ["Nope", "Empty"] {
            let unresolved = GrammarError::Unresolved {
                alias: alias.into(),
            };
            assert_eq!(
                both(ty(&format!("enum:{alias}"))),
                (unresolved.clone(), unresolved)
            );
        }
        let unresolved = GrammarError::Unresolved {
            alias: "Nope".into(),
        };
        assert_eq!(both(ty("record:Nope")), (unresolved.clone(), unresolved));
        let cycle = GrammarError::ContainsItself {
            cycle: vec!["loop.scxml".into(), "loop.scxml".into()],
        };
        assert_eq!(both(ty("record:Loop")), (cycle.clone(), cycle));
        let unbounded = GrammarError::Unbounded {
            constructor: "string".into(),
        };
        assert_eq!(
            both(ValueType::Scalar(SceType::String)),
            (unbounded.clone(), unbounded)
        );
        // Two records of different documents under one name: the schema's
        // `$defs` would name them alike, whichever is met first.
        let shared = GrammarError::SharedName {
            name: "item".into(),
        };
        for fields in [
            [
                ("a".to_string(), ty("record:Item")),
                ("b".to_string(), ty("record:Other")),
            ],
            [
                ("a".to_string(), ty("record:Other")),
                ("b".to_string(), ty("record:Item")),
            ],
        ] {
            let mut defs = SchemaDefs::default();
            assert_eq!(
                json_schema_object(&fields, &grammar_scope(), &mut defs),
                Err(shared.clone())
            );
        }
        // ... and when the one inside the other takes the name first.
        let mut wrapper_scope = TypeScope::default();
        wrapper_scope.records.insert(
            "Inner".into(),
            ScopedRecord {
                identity: "inner/item.scxml".into(),
                name: "item".into(),
                fields: vec![("x".into(), ty("bool"))],
                scope: TypeScope::default(),
            },
        );
        let mut outer = TypeScope::default();
        outer.records.insert(
            "Wrapper".into(),
            ScopedRecord {
                identity: "outer/item.scxml".into(),
                name: "item".into(),
                fields: vec![("inner".into(), ty("record:Inner"))],
                scope: wrapper_scope,
            },
        );
        let mut defs = SchemaDefs::default();
        assert_eq!(
            json_schema_of(&ty("record:Wrapper"), &outer, &mut defs),
            Err(shared)
        );
    }

    #[test]
    fn a_refusal_of_a_grammar_says_why() {
        for (refusal, sentence) in [
            (
                GrammarError::Unresolved {
                    alias: "Nope".into(),
                },
                "`Nope` names no record or enum the document imports",
            ),
            (
                GrammarError::ContainsItself {
                    cycle: vec!["Loop".into(), "Loop".into()],
                },
                "the record `Loop` holds itself (Loop -> Loop), and has no largest value",
            ),
            (
                GrammarError::Unbounded {
                    constructor: "bytes".into(),
                },
                "a `bytes` with no bound has no largest value, and no grammar",
            ),
            (
                GrammarError::OpenEnum {
                    alias: "Code".into(),
                },
                "the enum `Code` is open (`sce:strict-variants=\"false\"`): a value of it may be a \
                 number no name spells, and a grammar spells a value of an enum by its name",
            ),
            (
                GrammarError::SharedName {
                    name: "item".into(),
                },
                "two records or unions of different documents are named `item`, which the \
                 schema's `$defs` would name alike",
            ),
        ] {
            assert_eq!(refusal.to_string(), sentence);
        }
    }

    #[test]
    fn a_name_is_written_as_each_form_reads_it() {
        assert_eq!(pointer_token("a~b/c"), "a~0b~1c");
        assert_eq!(gbnf_literal(r#"{"a\b":"#), r#""{\"a\\b\":""#);
        assert_eq!(json_quoted("a\"\u{1}\u{7f}é"), "\"a\\\"\\u0001\u{7f}é\"");
        // A code point past ASCII is escaped, its hex in lowercase: GBNF past
        // U+FFFF in eight digits, JSON as a UTF-16 pair.
        assert_eq!(escaped('A', EscapeForm::Gbnf), "\\x41");
        assert_eq!(escaped('\u{0}', EscapeForm::Json), "\\u0000");
        assert_eq!(escaped('\u{e9}', EscapeForm::Gbnf), "\\u00e9");
        assert_eq!(escaped('\u{e9}', EscapeForm::Json), "\\u00e9");
        assert_eq!(escaped('\u{1f600}', EscapeForm::Gbnf), "\\U0001f600");
        assert_eq!(escaped('\u{1f600}', EscapeForm::Json), "\\ud83d\\ude00");
        assert_eq!(gbnf_literal("\"caf\u{e9}\""), "\"\\\"caf\\u00e9\\\"\"");
        assert_eq!(
            SchemaObject::default()
                .with(
                    Keyword::Enum,
                    SchemaNode::value(vec!["caf\u{e9}", "\u{1f600}"])
                )
                .text(),
            "{\"enum\":[\"caf\\u00e9\",\"\\ud83d\\ude00\"]}"
        );
        assert_eq!(latin1_pattern(), "^[\\u0000-\\u00ff]*$");
        assert!(is_rule_name("item-12"));
        assert!(!is_rule_name(""));
        assert!(!is_rule_name("( a )"));
    }

    // ── A union (docs/adr/0092) ───────────────────────────────────────────

    /// A record of `fields`, read in a scope of no import, as a case holds it.
    fn case_record(identity: &str, fields: &[(&str, &str)]) -> ScopedRecord {
        ScopedRecord {
            identity: identity.to_string(),
            name: identity.to_string(),
            fields: fields
                .iter()
                .map(|(id, written)| (id.to_string(), ty(written)))
                .collect(),
            scope: TypeScope::default(),
        }
    }

    /// A shape, in the manner of docs/adr/0092 section 12.1: a dot of no
    /// field, a circle and a disc that hold one record, a rect of two fields,
    /// and a label of a text and a list; its tag is written under `kind`.
    fn shape_union() -> ScopedUnion {
        let variants: &[(&str, i128)] = &[
            ("dot", 0),
            ("circle", 1),
            ("disc", 2),
            ("rect", 3),
            ("label", 4),
        ];
        let mut scope = TypeScope::default();
        scope
            .enums
            .insert("ShapeKind".into(), closed_enum(SceType::Uint8, variants));
        scope.records.insert(
            "Radius".into(),
            case_record("radius", &[("radius", "uint16")]),
        );
        scope.records.insert(
            "Rect".into(),
            case_record("rect", &[("width", "uint16"), ("height", "uint16")]),
        );
        scope.records.insert(
            "Label".into(),
            case_record(
                "label",
                &[("text", "string[4]"), ("points", "list<uint8>[3]")],
            ),
        );
        let record = |variant: &str| match variant {
            "circle" | "disc" => Some("Radius".to_string()),
            "rect" => Some("Rect".to_string()),
            "label" => Some("Label".to_string()),
            _ => None,
        };
        ScopedUnion {
            identity: "shape".into(),
            name: "shape".into(),
            tag: "kind".into(),
            tag_enum: "ShapeKind".into(),
            cases: variants
                .iter()
                .map(|(name, value)| ScopedCase {
                    variant: ScopedVariant {
                        name: name.to_string(),
                        value: *value,
                    },
                    record: record(name),
                })
                .collect(),
            scope,
        }
    }

    /// The batch item's table, and the shape under `Shape`.
    fn union_table() -> Table {
        let mut env = table();
        env.unions.insert("Shape".into(), shape_union());
        env
    }

    #[test]
    fn a_union_is_a_type_of_the_grammar_named_by_its_alias() {
        let written = WrittenType::read("list<optional<union:Shape>>[3]").unwrap();
        assert_eq!(written.to_string(), "list<optional<union:Shape>>[3]");
        assert_eq!(written.union_aliases(), ["Shape"]);
        assert!(written.record_aliases().is_empty());
        let union = WrittenType::read("union:Shape").unwrap();
        assert_eq!(union.constructor(), "union");
        assert_eq!(union.depth(), 1);
        assert_eq!(
            serde_json::to_value(&union).unwrap(),
            serde_json::json!({"union": "Shape"})
        );
        assert_eq!(
            union.clone().bounded().unwrap(),
            ValueType::Union {
                alias: "Shape".into()
            }
        );
        // A union states no bound of its own: a bracket on it and an attribute
        // beside it are refused as on a record, and so is a missing alias.
        assert!(matches!(
            WrittenType::read("union:Shape[3]"),
            Err(TypeRefusal::BoundOnUnboundable { .. })
        ));
        assert!(matches!(
            union.clone().with_outer_bound(Bound::of(3)),
            Err(TypeRefusal::NothingToBound { .. })
        ));
        assert!(matches!(
            WrittenType::read("union"),
            Err(TypeRefusal::Malformed { .. })
        ));
        assert!(matches!(
            WrittenType::read("union:"),
            Err(TypeRefusal::Unknown { .. })
        ));
        // No slot holds one yet, at any depth, and none is one entry: each
        // refusal names the union by its constructor.
        assert_eq!(union.slot_view().unwrap_err().constructor, "union");
        assert_eq!(
            WrittenType::read("list<union:Shape>[2]")
                .unwrap()
                .slot_view()
                .unwrap_err()
                .place,
            "union:Shape"
        );
        assert_eq!(
            ty("optional<union:Shape>").entry().unwrap_err().constructor,
            "union"
        );
        assert!(union_unlowered("Shape").starts_with("`union:Shape`: "));
    }

    /// A statechart's variable holds `union:U` and `optional<union:U>` on a
    /// lowering that holds unions, and nothing more than a slot does on one
    /// that holds none; a list of unions is held by neither yet.
    #[test]
    fn a_variable_holds_a_union_only_where_its_lowering_holds_one() {
        let union = WrittenType::read("union:Shape").unwrap();
        let optional = WrittenType::read("optional<union:Shape>").unwrap();
        let listed = WrittenType::read("list<union:Shape>[2]").unwrap();
        assert!(union.variable_view(true).is_ok());
        assert!(optional.variable_view(true).is_ok());
        assert_eq!(union.variable_view(false).unwrap_err().constructor, "union");
        assert_eq!(
            optional.variable_view(false).unwrap_err().constructor,
            "union"
        );
        assert_eq!(listed.variable_view(true).unwrap_err().place, "union:Shape");
        assert!(WrittenType::read("uint8")
            .unwrap()
            .variable_view(false)
            .is_ok());
    }

    #[test]
    fn a_union_is_as_large_as_its_largest_case_counted_with_its_own_variant() {
        let env = union_table();
        let shape = ty("union:Shape");
        // `{"kind":"label","text":"…","points":[255,255,255]}`, each byte of
        // the text at its worst escape.
        assert_eq!(json_max_len(&shape, &env), Ok(73));
        assert_eq!(cbor_max_len(&shape, &env), Ok(31));
        // Each case by its own variant's name and number (docs/adr/0092,
        // section 5.1): the disc is two bytes shorter than the circle on
        // JSON, and as long on CBOR.
        let union = &env.unions["Shape"];
        let per_case = |measure: Measure| -> Vec<u64> {
            union
                .cases
                .iter()
                .map(|case| case_size(union, case, measure, &mut Vec::new()).unwrap())
                .collect()
        };
        assert_eq!(per_case(Measure::Json), [14, 32, 30, 44, 73]);
        assert_eq!(per_case(Measure::Cbor), [7, 17, 17, 26, 31]);
        // Tight: a maximal value is exactly as long, alone and inside a
        // composition.
        for text in [
            "union:Shape",
            "optional<union:Shape>",
            "list<union:Shape>[2]",
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
        assert_eq!(
            json_max_len(&ty("union:Nope"), &env),
            Err(SizeError::Unresolved {
                alias: "Nope".into()
            })
        );
    }

    #[test]
    fn a_union_is_one_level_and_its_longest_pointer_is_its_tag_or_a_field_of_a_case() {
        let env = union_table();
        // The label's list is a container inside the union's own level.
        assert_eq!(container_depth(&ty("union:Shape"), &env), Ok(2));
        assert_eq!(container_depth(&ty("list<union:Shape>[2]"), &env), Ok(3));
        assert_eq!(
            longest_pointer(&ty("union:Shape"), &env),
            Ok(("/points/2".to_string(), 9))
        );
        assert_eq!(
            longest_pointer(&ty("list<union:Shape>[2]"), &env),
            Ok(("/1/points/2".to_string(), 11))
        );
        // A tag longer than every field of every case is the longest place a
        // value declares, since a refusal of the tag points at it.
        let mut env = union_table();
        let mut flat = shape_union();
        flat.tag = "shape_category".into();
        flat.cases
            .retain(|case| case.record.as_deref() != Some("Label"));
        env.unions.insert("Flat".into(), flat);
        assert_eq!(
            longest_pointer(&ty("union:Flat"), &env),
            Ok(("/shape_category".to_string(), 15))
        );
        assert_eq!(container_depth(&ty("union:Flat"), &env), Ok(1));
    }

    #[test]
    fn a_union_that_holds_itself_through_a_case_has_no_size() {
        // `N` holds `union:U`, whose one case holds `N` again: each read in its
        // own document's scope, they are the same two documents.
        let mut inner = TypeScope::default();
        inner
            .enums
            .insert("K".into(), closed_enum(SceType::Uint8, &[("a", 0)]));
        inner
            .records
            .insert("N".into(), case_record("n", &[("id", "uint8")]));
        let looping = ScopedUnion {
            identity: "u".into(),
            name: "u".into(),
            tag: "k".into(),
            tag_enum: "K".into(),
            cases: vec![ScopedCase {
                variant: ScopedVariant {
                    name: "a".into(),
                    value: 0,
                },
                record: Some("N".into()),
            }],
            scope: inner,
        };
        let mut node_scope = TypeScope::default();
        node_scope.unions.insert("U".into(), looping);
        let mut env = TypeScope::default();
        env.records.insert(
            "N".into(),
            ScopedRecord {
                identity: "n".into(),
                name: "n".into(),
                fields: vec![("next".into(), ty("optional<union:U>"))],
                scope: node_scope,
            },
        );
        assert_eq!(
            contains_itself("N", &env),
            Some(vec!["N".to_string(), "N".into()])
        );
        assert_eq!(
            json_max_len(&ty("record:N"), &env),
            Err(SizeError::ContainsItself {
                cycle: vec!["n".into(), "u".into(), "n".into()]
            })
        );
    }

    #[test]
    fn a_union_fits_only_a_union_of_its_own_document() {
        assert_eq!(fits(&ty("union:U"), &ty("union:U2"), &Documents), Ok(()));
        assert_eq!(
            fits(&ty("union:U"), &ty("optional<union:U>"), &Documents),
            Ok(())
        );
        assert_eq!(
            fits(
                &ty("list<union:U2>[2]"),
                &ty("list<union:U>[3]"),
                &Documents
            ),
            Ok(())
        );
        let refused = |from: &str, to: &str| fits(&ty(from), &ty(to), &Documents).unwrap_err();
        assert_eq!(refused("union:U", "union:V").why, MisfitReason::Identity);
        assert_eq!(
            refused("union:U", "record:U").why,
            MisfitReason::Constructor
        );
        assert_eq!(
            refused("record:R", "union:U").why,
            MisfitReason::Constructor
        );
        assert_eq!(
            refused("optional<union:U>", "union:U").why,
            MisfitReason::MayBeAbsent
        );
        // Held to the values each type holds, as every other constructor is.
        let universe: Vec<ValueType> = [
            "union:U",
            "union:U2",
            "union:V",
            "record:U",
            "uint8",
            "optional<union:U>",
            "optional<union:V>",
            "list<union:U>[1]",
            "list<union:U2>[2]",
        ]
        .into_iter()
        .map(ty)
        .collect();
        for from in &universe {
            for to in &universe {
                let oracle = values_of(from).is_subset(&values_of(to));
                assert_eq!(
                    fits(from, to, &Documents).is_ok(),
                    oracle,
                    "{from} into {to}"
                );
            }
        }
    }

    /// The definition of the shape in `$defs`, as it is written: one object
    /// per case in the tag enum's order, the tag first and a string `const`,
    /// the case's fields inline after it (docs/adr/0092, section 3.1).
    const SHAPE_CASES: &str = concat!(
        r#"{"oneOf":["#,
        r#"{"type":"object","properties":{"kind":{"const":"dot"}},"#,
        r#""required":["kind"],"additionalProperties":false},"#,
        r#"{"type":"object","properties":{"kind":{"const":"circle"},"#,
        r#""radius":{"type":"integer","minimum":0,"maximum":65535}},"#,
        r#""required":["kind","radius"],"additionalProperties":false},"#,
        r#"{"type":"object","properties":{"kind":{"const":"disc"},"#,
        r#""radius":{"type":"integer","minimum":0,"maximum":65535}},"#,
        r#""required":["kind","radius"],"additionalProperties":false},"#,
        r#"{"type":"object","properties":{"kind":{"const":"rect"},"#,
        r#""width":{"type":"integer","minimum":0,"maximum":65535},"#,
        r#""height":{"type":"integer","minimum":0,"maximum":65535}},"#,
        r#""required":["kind","width","height"],"additionalProperties":false},"#,
        r#"{"type":"object","properties":{"kind":{"const":"label"},"#,
        r#""text":{"type":"string","maxLength":4,"x-sce-max-bytes":4},"#,
        r#""points":{"type":"array","items":{"type":"integer","minimum":0,"maximum":255},"#,
        r#""maxItems":3}},"#,
        r#""required":["kind","text","points"],"additionalProperties":false}"#,
        r#"]}"#,
    );

    /// The alternation of the shape's cases, as its rule and a union
    /// document's `root` hold it: each alternative begins with its tag.
    const SHAPE_ALTERNATIVES: &str = concat!(
        r#""{\"kind\":\"dot\"}""#,
        r#" | "{\"kind\":\"circle\",\"radius\":" uint16 "}""#,
        r#" | "{\"kind\":\"disc\",\"radius\":" uint16 "}""#,
        r#" | "{\"kind\":\"rect\",\"width\":" uint16 ",\"height\":" uint16 "}""#,
        r#" | "{\"kind\":\"label\",\"text\":" "\"" t4 "\"" ",\"points\":" "[" ( uint8 uint8-tail-2 )? "]" "}""#,
    );

    /// The rule of `uint16`.
    const UINT16_RULE: &str = concat!(
        r#"uint16 ::= "0" | [1-9] [0-9]{0,3} | [1-5] [0-9]{4} | "6" [0-4] [0-9]{3} | "#,
        r#""65" [0-4] [0-9]{2} | "655" [0-2] [0-9] | "6553" [0-5]"#,
    );

    #[test]
    fn a_union_is_defined_once_as_the_one_of_of_its_cases() {
        let env = union_table();
        let mut defs = SchemaDefs::default();
        let schema = json_schema_of(&ty("union:Shape"), &env, &mut defs).unwrap();
        assert_eq!(schema.text(), r##"{"$ref":"#/$defs/shape"}"##);
        assert_eq!(
            defs.into_node().map(defs_text),
            Some(format!("{{\"shape\":{SHAPE_CASES}}}"))
        );
        // Met twice, inside a list and an optional, it is defined once.
        let mut defs = SchemaDefs::default();
        let fields = [
            ("one".to_string(), ty("union:Shape")),
            ("many".to_string(), ty("list<optional<union:Shape>>[2]")),
        ];
        let object = json_schema_object(&fields, &env, &mut defs).unwrap();
        assert_eq!(
            object.text(),
            concat!(
                r##"{"type":"object","properties":{"one":{"$ref":"#/$defs/shape"},"##,
                r##""many":{"type":"array","items":{"anyOf":[{"$ref":"#/$defs/shape"},"##,
                r##"{"type":"null"}]},"maxItems":2}},"required":["one","many"],"##,
                r##""additionalProperties":false}"##,
            )
        );
        assert_eq!(
            defs.into_node().map(defs_text),
            Some(format!("{{\"shape\":{SHAPE_CASES}}}"))
        );
        // The whole of a union document's schema is the same `oneOf`, with no
        // definition of its own.
        let mut defs = SchemaDefs::default();
        assert_eq!(
            json_schema_union(&shape_union(), &mut defs).map(|s| s.text()),
            Ok(SHAPE_CASES.to_string())
        );
        assert_eq!(defs.into_node(), None);
    }

    /// docs/adr/0092, section 3.1, item 2: llguidance accepts a `oneOf` only
    /// where it proves the cases disjoint, which two objects are when a key
    /// both require holds different string `const` literals; a `$ref` is
    /// never proven so. Each case therefore requires the tag, holds it as a
    /// string `const` of its own, and writes its fields inline.
    #[test]
    fn every_case_requires_its_tag_and_holds_it_as_a_string_const() {
        let union = shape_union();
        let mut defs = SchemaDefs::default();
        let schema = read_json(&json_schema_union(&union, &mut defs).unwrap().text());
        let cases = schema["oneOf"].as_array().expect("a oneOf of the cases");
        assert_eq!(cases.len(), union.cases.len());
        let mut literals = BTreeSet::new();
        for (case, object) in union.cases.iter().zip(cases) {
            let name = &case.variant.name;
            let required = object["required"].as_array().expect("required keys");
            assert_eq!(required.first(), Some(&serde_json::json!("kind")), "{name}");
            let properties = object["properties"].as_object().expect("properties");
            assert_eq!(
                properties.get("kind"),
                Some(&serde_json::json!({ "const": name })),
                "{name}"
            );
            assert!(
                properties["kind"]["const"].is_string(),
                "{name}: the tag is a string literal"
            );
            assert!(literals.insert(name.clone()), "{name}: one literal a case");
            assert_eq!(object["additionalProperties"], false, "{name}");
            assert_eq!(object.get("$ref"), None, "{name}: written inline");
            for (key, value) in properties {
                assert!(
                    required.contains(&serde_json::json!(key)),
                    "{name}: `{key}` is required"
                );
                assert_eq!(value.get("$ref"), None, "{name}: `{key}` is no record");
            }
        }
    }

    #[test]
    fn a_union_is_one_rule_of_one_alternative_per_case_its_tag_first() {
        let env = union_table();
        let text_to_four = &TEXT_TO_FIVE[..TEXT_TO_FIVE.find("t5 ::=").unwrap()];
        let tail = concat!(
            "uint8-tail-0 ::= \"\"\n",
            "uint8-tail-1 ::= \"\" | \",\" uint8 uint8-tail-0\n",
            "uint8-tail-2 ::= \"\" | \",\" uint8 uint8-tail-1\n",
        );
        let mut rules = GbnfRules::default();
        let root = gbnf_of(&ty("union:Shape"), &env, &mut rules).unwrap();
        assert_eq!(
            rules.grammar(&root),
            format!(
                "root ::= union-1\nunion-1 ::= {SHAPE_ALTERNATIVES}\n{tail}{UINT16_RULE}\n\
                 {UINT8_RULE}\n{text_to_four}"
            )
        );
        // Met twice, it has one rule.
        let mut rules = GbnfRules::default();
        let root = gbnf_of(&ty("list<union:Shape>[2]"), &env, &mut rules).unwrap();
        let grammar = rules.grammar(&root);
        assert!(
            grammar.starts_with(&format!(
                "root ::= \"[\" ( union-1 union-1-tail-1 )? \"]\"\n\
                 union-1 ::= {SHAPE_ALTERNATIVES}\n"
            )),
            "{grammar}"
        );
        assert_eq!(grammar.matches("union-1 ::=").count(), 1, "{grammar}");
        // The whole of a union document's grammar is the same alternation, at
        // its root.
        let mut rules = GbnfRules::default();
        let root = gbnf_union(&shape_union(), &mut rules).unwrap();
        assert_eq!(root, SHAPE_ALTERNATIVES);
        assert!(rules
            .grammar(&root)
            .starts_with(&format!("root ::= {SHAPE_ALTERNATIVES}\n{tail}")));
    }

    #[test]
    fn a_union_with_no_grammar_is_refused_by_name() {
        let both = |ty: &ValueType, env: &dyn TypeEnv| -> (GrammarError, GrammarError) {
            (
                json_schema_of(ty, env, &mut SchemaDefs::default()).expect_err("no JSON Schema"),
                gbnf_of(ty, env, &mut GbnfRules::default()).expect_err("no GBNF grammar"),
            )
        };
        let mut env = union_table();
        // A union of no case was not read from its document.
        let hollow = ScopedUnion {
            cases: Vec::new(),
            ..shape_union()
        };
        env.unions.insert("Hollow".into(), hollow.clone());
        for alias in ["Nope", "Hollow"] {
            let unresolved = GrammarError::Unresolved {
                alias: alias.into(),
            };
            assert_eq!(
                both(&ty(&format!("optional<union:{alias}>")), &env),
                (unresolved.clone(), unresolved)
            );
        }
        let unresolved = GrammarError::Unresolved {
            alias: "shape".into(),
        };
        assert_eq!(
            json_schema_union(&hollow, &mut SchemaDefs::default()),
            Err(unresolved.clone())
        );
        assert_eq!(
            gbnf_union(&hollow, &mut GbnfRules::default()),
            Err(unresolved)
        );
        // A tag of an open enum: a number no variant names would be a value
        // of no case, and a grammar spells the tag by its variant's name.
        let mut open = shape_union();
        open.scope.enums.insert(
            "ShapeKind".into(),
            open_enum(
                SceType::Uint8,
                &[
                    ("dot", 0),
                    ("circle", 1),
                    ("disc", 2),
                    ("rect", 3),
                    ("label", 4),
                ],
            ),
        );
        env.unions.insert("Open".into(), open.clone());
        let refusal = GrammarError::OpenEnum {
            alias: "ShapeKind".into(),
        };
        assert_eq!(
            both(&ty("list<union:Open>[2]"), &env),
            (refusal.clone(), refusal.clone())
        );
        assert_eq!(
            json_schema_union(&open, &mut SchemaDefs::default()),
            Err(refusal.clone())
        );
        assert_eq!(gbnf_union(&open, &mut GbnfRules::default()), Err(refusal));
        // A case's record that holds the union again: `N` holds `union:U`,
        // whose one case holds `N`.
        let mut inner = TypeScope::default();
        inner
            .enums
            .insert("K".into(), closed_enum(SceType::Uint8, &[("a", 0)]));
        inner
            .records
            .insert("N".into(), case_record("n", &[("id", "uint8")]));
        let looping = ScopedUnion {
            identity: "u".into(),
            name: "u".into(),
            tag: "k".into(),
            tag_enum: "K".into(),
            cases: vec![ScopedCase {
                variant: ScopedVariant {
                    name: "a".into(),
                    value: 0,
                },
                record: Some("N".into()),
            }],
            scope: inner,
        };
        let mut node_scope = TypeScope::default();
        node_scope.unions.insert("U".into(), looping);
        let mut cyclic = TypeScope::default();
        cyclic.records.insert(
            "N".into(),
            ScopedRecord {
                identity: "n".into(),
                name: "n".into(),
                fields: vec![("next".into(), ty("optional<union:U>"))],
                scope: node_scope,
            },
        );
        let cycle = GrammarError::ContainsItself {
            cycle: vec!["n".into(), "u".into(), "n".into()],
        };
        assert_eq!(both(&ty("record:N"), &cyclic), (cycle.clone(), cycle));
        // A union and a record of another document under one name: the
        // schema's `$defs` would name them alike, whichever is met first.
        let mut shared = TypeScope::default();
        shared.unions.insert("Shape".into(), shape_union());
        shared.records.insert(
            "Other".into(),
            ScopedRecord {
                identity: "elsewhere/shape.scxml".into(),
                name: "shape".into(),
                fields: vec![("x".into(), ty("bool"))],
                scope: TypeScope::default(),
            },
        );
        for fields in [
            [
                ("a".to_string(), ty("union:Shape")),
                ("b".to_string(), ty("record:Other")),
            ],
            [
                ("a".to_string(), ty("record:Other")),
                ("b".to_string(), ty("union:Shape")),
            ],
        ] {
            assert_eq!(
                json_schema_object(&fields, &shared, &mut SchemaDefs::default()),
                Err(GrammarError::SharedName {
                    name: "shape".into()
                })
            );
        }
    }
}
