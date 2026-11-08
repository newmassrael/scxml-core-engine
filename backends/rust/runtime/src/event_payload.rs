// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Lifting an event's typed `_event.data` view out of the data it carries.
//!
//! NL→IR Item C1 Path A gives a schema'd event a typed payload that the
//! natively lowered guards read. One producer fills it: the generated
//! `raise_<event>` inject seam. Every other producer — `<send>` with `<param>`,
//! namelist or `<content>`, an invoke forwarding an event either way,
//! autoforward, BasicHTTP, mesh — fills [`EventMetadata::data`](crate::EventMetadata),
//! the wire §scxml-5.10 describes and §scxml-B-2-8-1 reads.
//!
//! Until this module the two never met, so the same guard answered differently
//! depending on where its event came from, and a typed payload could not cross
//! an invoke boundary at all. What the lift refuses is what the SCRIPT ENGINE
//! already refuses for the same guard, measured on the same document: no data,
//! a missing field, or a value of another type each give `error.execution` and
//! a guard that does not fire. A native lowering that answered differently
//! would make the optimisation observable, which is the one thing it may not
//! be.
//!
//! # One value at one path
//!
//! A field may be of any type of the language (docs/adr/0086): a list of any
//! type, an optional value, a record, nested to any depth. Each is read by
//! [`PayloadValue`](crate::event_payload::PayloadValue), one value at the path
//! that names it, and three combinators walk the constructors that hold other
//! values, each calling the reader of its part:
//! [`PayloadValue::list`](crate::event_payload::PayloadValue::list),
//! [`PayloadValue::optional`](crate::event_payload::PayloadValue::optional) and
//! [`PayloadValue::record`](crate::event_payload::PayloadValue::record); a
//! fourth, [`PayloadValue::union`](crate::event_payload::PayloadValue::union),
//! reads a union's tag and then its case's record from the same object
//! (docs/adr/0092, section 4). The
//! links name the whole path because these lines are merged with the module's
//! outer docs in `lib.rs`, and rustdoc resolves the merged text from the crate
//! root. A value outside its type is refused before
//! anything of it is kept, and the refusal names the path through every depth:
//! `'items/2/tags/0' is 9 bytes, past the 8 its type declares`. A key a
//! record's schema does not declare is stepped over.
//!
//! # A refusal is a sentence and a record
//!
//! The sentence is what the delivery's `error.execution` carries. Beside it a
//! refusal carries what a statechart acts on (docs/adr/0087, section 2.4): a
//! [`RefusalCode`](crate::event_payload::RefusalCode), the place as a strict RFC 6901 pointer (`/items/2/tags/0`,
//! `~` written `~0` and `/` written `~1`, empty for the whole text), and the
//! bound the value broke. The sentence's path keeps the spelling it always had,
//! the reference tokens joined by `/` with nothing escaped. A repeated name
//! anywhere in the text, and a text holding U+0000, are refused, so every
//! runtime's reader holds one set of texts (tests/json_text/payload_refusal.json).
//!
//! ⚠ `std` only, because the wire is: a `no_std` build has no
//! `EventMetadata::data` and no script engine to disagree with, so the typed
//! carrier is the whole channel there. That is why this module is `cfg`'d out
//! rather than given a no-alloc twin — there is nothing for one to read.
//!
//! Cross-language siblings: `sce_runtime.event_payload` (Python),
//! `sce.LiftPayload` (Go), `SCE::Common::EventPayloadFields` (C++).

#![cfg(feature = "std")]

use core::fmt;

use sce_portable_bytes::BoundedList;

/// The class of a refusal: the enum `sce:std/lift/refusal_code.scxml`
/// declares, each variant at the number that document gives it
/// (docs/adr/0087, section 2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RefusalCode {
    /// The text is empty, is not JSON, carries more than one JSON value, or
    /// holds an object that repeats a name.
    NotJson = 1,
    /// The text is one JSON value that is not an object.
    NotObject = 2,
    /// A container is opened deeper than any a value of the schema holds.
    TooDeep = 3,
    /// A field of a record is absent and no revision default fills it.
    Missing = 4,
    /// `null` stands where the type is not optional.
    NullNotOptional = 5,
    /// A value of another JSON kind.
    WrongKind = 6,
    /// A number not in its type's spelling.
    BadNumber = 7,
    /// A whole number past its type's width, or a real past a `float32`'s
    /// range.
    OutOfRange = 8,
    /// A text or byte string past its bound.
    TooLong = 9,
    /// A list past its bound.
    TooMany = 10,
    /// A name the enum does not declare.
    UnknownVariant = 11,
    /// A byte string's text holding a character above U+00FF.
    NotByteText = 12,
    /// A text holding U+0000.
    NulInText = 13,
    /// A list short of the lower count its type states (docs/adr/0086,
    /// section 15).
    TooFew = 14,
}

impl RefusalCode {
    /// Every code, in the order of their numbers.
    pub const ALL: [RefusalCode; 14] = [
        RefusalCode::NotJson,
        RefusalCode::NotObject,
        RefusalCode::TooDeep,
        RefusalCode::Missing,
        RefusalCode::NullNotOptional,
        RefusalCode::WrongKind,
        RefusalCode::BadNumber,
        RefusalCode::OutOfRange,
        RefusalCode::TooLong,
        RefusalCode::TooMany,
        RefusalCode::UnknownVariant,
        RefusalCode::NotByteText,
        RefusalCode::NulInText,
        RefusalCode::TooFew,
    ];

    /// The number the enum declares the code at.
    pub fn number(self) -> u8 {
        self as u8
    }

    /// The name the enum declares the code by.
    pub fn name(self) -> &'static str {
        match self {
            RefusalCode::NotJson => "notJson",
            RefusalCode::NotObject => "notObject",
            RefusalCode::TooDeep => "tooDeep",
            RefusalCode::Missing => "missing",
            RefusalCode::NullNotOptional => "nullNotOptional",
            RefusalCode::WrongKind => "wrongKind",
            RefusalCode::BadNumber => "badNumber",
            RefusalCode::OutOfRange => "outOfRange",
            RefusalCode::TooLong => "tooLong",
            RefusalCode::TooMany => "tooMany",
            RefusalCode::UnknownVariant => "unknownVariant",
            RefusalCode::NotByteText => "notByteText",
            RefusalCode::NulInText => "nulInText",
            RefusalCode::TooFew => "tooFew",
        }
    }

    /// The code the enum declares by `name`, if it declares one.
    pub fn from_name(name: &str) -> Option<RefusalCode> {
        RefusalCode::ALL
            .into_iter()
            .find(|code| code.name() == name)
    }
}

/// Why an event's data cannot be read as its schema's fields.
///
/// The sentence rides in the `error.execution` event's data, where the
/// document's own handler can read it. Beside it the refusal carries what a
/// statechart reads as data (docs/adr/0087, section 2.4): its code, the place
/// as a strict RFC 6901 pointer, and the bound the value broke, for
/// [`RefusalCode::TooLong`], [`RefusalCode::TooMany`] and
/// [`RefusalCode::TooFew`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadRefusal {
    reason: String,
    code: RefusalCode,
    pointer: String,
    bound: Option<u32>,
}

impl PayloadRefusal {
    /// Refuse a payload at `pointer` (empty for the whole text, else a strict
    /// RFC 6901 pointer, [`pointer_below`]), saying why in the words the
    /// document's handler reads.
    pub fn new(code: RefusalCode, pointer: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            code,
            pointer: pointer.into(),
            bound: None,
        }
    }

    /// This refusal, naming the bound the value broke. A bound no `uint32`
    /// holds is no bound a type declares, and is held at the largest one.
    pub fn with_bound(mut self, bound: usize) -> Self {
        self.bound = Some(u32::try_from(bound).unwrap_or(u32::MAX));
        self
    }

    /// Why the payload was refused.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The class of the refusal.
    pub fn code(&self) -> RefusalCode {
        self.code
    }

    /// The place refused, as a strict RFC 6901 pointer: empty for the whole
    /// text.
    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    /// The bound the value broke, for a text, a byte string or a list past
    /// it; `None` for every other refusal.
    pub fn bound(&self) -> Option<u32> {
        self.bound
    }
}

impl fmt::Display for PayloadRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.reason)
    }
}

/// `token` below the strict RFC 6901 pointer `prefix` (empty for the whole
/// text): `/` and the token with `~` written `~0` and `/` written `~1`
/// (RFC 6901, section 3).
pub fn pointer_below(prefix: &str, token: &str) -> String {
    let mut out = String::with_capacity(prefix.len() + token.len() + 1);
    out.push_str(prefix);
    out.push('/');
    for c in token.chars() {
        match c {
            '~' => out.push_str("~0"),
            '/' => out.push_str("~1"),
            c => out.push(c),
        }
    }
    out
}

/// What a `<sce:lift>` reads (docs/adr/0087, section 2.1): `text` as a record
/// of the schema named `schema`, whose fields `read` reads out of the record it
/// is handed, as a delivery of the schema is read; or the refusal of the first
/// place that is not one, its code, pointer and bound the statechart keeps as
/// a record. [`PayloadFields::lift`] says what is refused before the fields are
/// read. Each outcome is logged with the schema's name and, for a refusal, its
/// code, pointer and bound; the text itself is not, since nothing bounds what a
/// sender writes in it.
pub fn lift_record<T>(
    text: &str,
    schema: &str,
    depth: u32,
    path_bound: u32,
    defaults: Option<&str>,
    read: impl FnOnce(&PayloadRecord<'_>) -> Result<T, PayloadRefusal>,
) -> Result<T, PayloadRefusal> {
    let lifted = PayloadFields::lift(text, depth, path_bound, defaults)
        .and_then(|fields| read(&fields.record()));
    log_lift("record", schema, &lifted);
    lifted
}

/// What a `<sce:lift>` into a variable of a union reads (docs/adr/0092,
/// section 6.1 item 5): `text` as one value of the union named `schema`, which
/// `read` reads out of the whole object it is handed, by the reader a
/// delivery's field of the union is read by ([`PayloadValue::union`]: its tag
/// first, then the record of the case the tag names); or the refusal of the
/// first place that is not one. What is refused before the value is read is
/// what [`PayloadFields::lift`] refuses, a text that is no object among it,
/// and a union has no defaults: its revision is its tag enum's, so no field is
/// added to a case by one. Logged as [`lift_record`] logs.
pub fn lift_value<T>(
    text: &str,
    schema: &str,
    depth: u32,
    path_bound: u32,
    read: impl FnOnce(&PayloadValue<'_>) -> Result<T, PayloadRefusal>,
) -> Result<T, PayloadRefusal> {
    let lifted = PayloadFields::lift(text, depth, path_bound, None).and_then(|fields| {
        let object = Value::Object(fields.fields);
        read(&PayloadValue {
            value: &object,
            path: String::new(),
            pointer: String::new(),
        })
    });
    log_lift("value", schema, &lifted);
    lifted
}

/// The line a lift logs of its outcome: the schema's name and, for a refusal,
/// its code, pointer and bound, never the text, since nothing bounds what a
/// sender writes in it. `what` is what the text is read as.
fn log_lift<T>(what: &str, schema: &str, lifted: &Result<T, PayloadRefusal>) {
    match lifted {
        Ok(_) => crate::sce_log_info!("<sce:lift>: the text is a {what} of {schema}"),
        Err(refusal) => crate::sce_log_info!(
            "<sce:lift>: the text is no {what} of {schema}: {} at '{}', bound {:?} ({refusal})",
            refusal.code().name(),
            refusal.pointer(),
            refusal.bound()
        ),
    }
}

/// The refusal of a container `text` opens deeper than `depth`, the deepest a
/// value of a lift's schema holds (docs/adr/0087, section 2.1, item 4), or
/// `None` when it opens none.
///
/// Decided over the brackets alone, before the text is read as JSON, so that
/// every runtime decides it alike and no reader descends past the type: a text
/// is skipped from its opening quote to the next quote no backslash escapes,
/// each `{` and `[` outside one opens a level and each `}` and `]` closes one,
/// and the first opened past `depth` is refused, whatever the text holds after
/// it. A text that never closes ends the walk with nothing refused, and the
/// reading of the text as JSON refuses it.
///
/// The pointer names the container refused (docs/adr/0087, section 2.4): for
/// each container it stands in, outermost first, the element's index in an
/// array (the `,` the scan met at that level since the array opened) or the
/// member's name in an object (the last text closed at that level before a
/// `:`, since the object opened or its last `,`), each an RFC 6901 token. It
/// stops before the first place it cannot write: a member with no name before
/// its colon, a name that is no text (no JSON string, or one holding U+0000),
/// or a token that would take the pointer past `path_bound` UTF-8 bytes, the
/// room of the refusal record's `path`. The bound is `depth`. The scan keeps
/// one entry per open container, at most `depth` of them.
pub fn lift_too_deep(text: &str, depth: u32, path_bound: u32) -> Option<PayloadRefusal> {
    let bytes = text.as_bytes();
    let mut open: Vec<ScanLevel<'_>> = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'"' => {
                let start = at;
                at += 1;
                while at < bytes.len() && bytes[at] != b'"' {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
                if at >= bytes.len() {
                    return None;
                }
                if let Some(level) = open.last_mut() {
                    level.last_text = Some(&text[start..=at]);
                }
            }
            b':' => {
                if let Some(level) = open.last_mut() {
                    level.member = level.last_text.take();
                }
            }
            b',' => {
                if let Some(level) = open.last_mut() {
                    level.member = None;
                    level.last_text = None;
                    level.index += 1;
                }
            }
            b'{' | b'[' => {
                if open.len() >= depth as usize {
                    return Some(
                        PayloadRefusal::new(
                            RefusalCode::TooDeep,
                            scan_pointer(&open, path_bound),
                            format!(
                                "the text opens a container deeper than the {depth} levels a \
                                 value of its schema holds"
                            ),
                        )
                        .with_bound(depth as usize),
                    );
                }
                open.push(ScanLevel {
                    array: bytes[at] == b'[',
                    index: 0,
                    last_text: None,
                    member: None,
                });
            }
            b'}' | b']' => {
                open.pop();
            }
            _ => {}
        }
        at += 1;
    }
    None
}

/// One container [`lift_too_deep`] stands in: whether it is an array, the
/// elements it has passed, the last text closed in it, and the member a `:`
/// after that text named.
struct ScanLevel<'t> {
    array: bool,
    index: u64,
    last_text: Option<&'t str>,
    member: Option<&'t str>,
}

/// The pointer to the place `open` stands at, as [`lift_too_deep`] writes it.
fn scan_pointer(open: &[ScanLevel<'_>], path_bound: u32) -> String {
    let mut pointer = String::new();
    for level in open {
        let token = if level.array {
            level.index.to_string()
        } else {
            match level.member.map(crate::json::parse) {
                Some(Ok(Value::Text(name))) if !name.contains('\0') => name,
                _ => break,
            }
        };
        let below = pointer_below(&pointer, &token);
        if below.len() > path_bound as usize {
            break;
        }
        pointer = below;
    }
    pointer
}

use crate::json::{JsonError, Value};

/// The fields an event's data names, each in the spelling it was written in
/// ([`crate::json`]), so a whole number stays exact until it is read at the
/// width its schema declares. A nested value this document's schema does not
/// name is kept, so a payload carrying more than the schema does still reads.
#[derive(Clone, Debug, Default)]
pub struct PayloadFields {
    fields: Vec<(String, Value)>,
}

impl PayloadFields {
    /// Read an event's data as the fields it names.
    ///
    /// The JSON read is §scxml-B-2-8-1's second rung, the same one the
    /// script engine takes for the same string — through the runtime's one
    /// JSON reader.
    pub fn decode(data: &str) -> Result<Self, PayloadRefusal> {
        // Only the whitespace RFC 8259 has: any other is a text that is not JSON.
        let trimmed = data.trim_matches(crate::json::is_json_whitespace);
        if trimmed.is_empty() {
            return Err(PayloadRefusal::new(
                RefusalCode::NotJson,
                "",
                "the event carries no data",
            ));
        }
        let parsed = crate::json::parse(trimmed);
        if !trimmed.starts_with('{') {
            // One sentence for a text that is no object, as it has always
            // been; the code tells one JSON value that is not an object from a
            // text that is not JSON at all (docs/adr/0087, section 2.3).
            let code = if parsed.is_ok() {
                RefusalCode::NotObject
            } else {
                RefusalCode::NotJson
            };
            return Err(PayloadRefusal::new(
                code,
                "",
                "the event's data is a bare value, and this event's schema declares named fields",
            ));
        }
        let value = parsed.map_err(|e| match e {
            JsonError::Trailing => PayloadRefusal::new(
                RefusalCode::NotJson,
                "",
                "the event's data carries more than one JSON value",
            ),
            JsonError::Malformed(what) => PayloadRefusal::new(
                RefusalCode::NotJson,
                "",
                format!("the event's data is not JSON ({what})"),
            ),
        })?;
        let Value::Object(fields) = value else {
            unreachable!("a text that starts with '{{' and parses is an object")
        };
        Ok(Self { fields })
    }

    /// [`Self::decode`] for a schema a later revision of which added fields:
    /// `defaults` is the JSON object of those fields' defaults, which the
    /// generator spells from the schema, and a field the delivery lacks is
    /// read out of it instead (docs/SCE_ACCEPTED_SUBSET.md, "An event schema
    /// that grows").
    ///
    /// A delivery with no data at all lacks every field, so it is read as one
    /// that names none: a field of the first revision is then refused by name,
    /// and a schema all of whose fields came later reads its defaults. A field
    /// the delivery does carry is read as it came, and held to its type as any
    /// other value is.
    pub fn decode_with_defaults(data: &str, defaults: &str) -> Result<Self, PayloadRefusal> {
        let mut decoded = if data
            .trim_matches(crate::json::is_json_whitespace)
            .is_empty()
        {
            Self::default()
        } else {
            Self::decode(data)?
        };
        decoded.take_defaults(defaults)?;
        Ok(decoded)
    }

    /// The fields of `defaults`, a schema's defaults, that these fields lack,
    /// added to them.
    fn take_defaults(&mut self, defaults: &str) -> Result<(), PayloadRefusal> {
        for (name, value) in defaults_of(defaults)? {
            if !self.fields.iter().any(|(key, _)| *key == name) {
                self.fields.push((name, value));
            }
        }
        Ok(())
    }

    /// Read the text a `<sce:lift>` reads as the fields of a record of a schema
    /// (docs/adr/0087, section 2.1): `depth` is the deepest container a value of
    /// the schema holds, `path_bound` the room of the refusal record's `path`
    /// a `tooDeep` pointer is held to ([`lift_too_deep`]), and `defaults`, for
    /// a schema a later revision of which added fields, the JSON object of their
    /// defaults, which a text that lacks one reads in its place.
    ///
    /// Three refusals come before the fields are read, in this order: a text
    /// that is empty or only whitespace (`notJson`); a container opened deeper
    /// than `depth`, decided over the text's brackets alone before it is read as
    /// JSON, so neither the work nor the stack of a lift grows with what a text
    /// nests ([`lift_too_deep`], `tooDeep`); and a text that is not one JSON
    /// object, as a delivery is refused ([`Self::decode`]). What is refused of a
    /// field after that is what a delivery of the schema is refused for, by the
    /// same readers.
    pub fn lift(
        text: &str,
        depth: u32,
        path_bound: u32,
        defaults: Option<&str>,
    ) -> Result<Self, PayloadRefusal> {
        if text
            .trim_matches(crate::json::is_json_whitespace)
            .is_empty()
        {
            return Err(PayloadRefusal::new(
                RefusalCode::NotJson,
                "",
                "the text is empty",
            ));
        }
        if let Some(refusal) = lift_too_deep(text, depth, path_bound) {
            return Err(refusal);
        }
        let mut decoded = Self::decode(text)?;
        if let Some(defaults) = defaults {
            decoded.take_defaults(defaults)?;
        }
        Ok(decoded)
    }

    /// The payload as the record of its fields, at the empty path: what a
    /// field's reader starts from.
    pub fn record(&self) -> PayloadRecord<'_> {
        PayloadRecord {
            members: &self.fields,
            defaults: None,
            prefix: "",
            pointer: "",
        }
    }

    /// The value of the field `name`, at the path its name is; a delivery that
    /// lacks it is refused by name.
    pub fn field(&self, name: &str) -> Result<PayloadValue<'_>, PayloadRefusal> {
        self.record().field(name)
    }

    /// The refusal of the text `name` the field `field` carries, which the
    /// enum `alias` does not declare a variant by: what a generated reader of
    /// an enum field answers once it has matched every name the enum declares.
    pub fn unknown_variant(&self, field: &str, name: &str, alias: &str) -> PayloadRefusal {
        PayloadRefusal::new(
            RefusalCode::UnknownVariant,
            pointer_below("", field),
            format!("{field} ({name}) is not a variant of {alias}"),
        )
    }

    /// A field of `optional<T>` (docs/adr/0055, section 4): absent where the
    /// delivery carries `null`, and otherwise the value `read` reads of it, held
    /// to `T` as a field of `T` is. A delivery that lacks the key is refused as
    /// it is for any field: `null` is the one spelling of absence, and a missing
    /// key means a sender whose revision does not have the field, which
    /// [`Self::decode_with_defaults`] has already filled where a later revision
    /// added it.
    ///
    /// `read` is the reader of a present value, handed these fields:
    /// `fields.optional("due", |fields| fields.signed64("due"))`.
    pub fn optional<T>(
        &self,
        name: &str,
        read: impl FnOnce(&Self) -> Result<T, PayloadRefusal>,
    ) -> Result<Option<T>, PayloadRefusal> {
        if self.field(name)?.is_null() {
            Ok(None)
        } else {
            read(self).map(Some)
        }
    }

    /// A signed whole number at the width its schema declares
    /// ([`PayloadValue::signed`]).
    pub fn signed<T>(&self, name: &str) -> Result<T, PayloadRefusal>
    where
        T: TryFrom<i64>,
    {
        self.field(name)?.signed()
    }

    /// An unsigned whole number at the width its schema declares
    /// ([`PayloadValue::unsigned`]).
    pub fn unsigned<T>(&self, name: &str) -> Result<T, PayloadRefusal>
    where
        T: TryFrom<u64>,
    {
        self.field(name)?.unsigned()
    }

    /// A 64-bit signed field ([`PayloadValue::signed64`]).
    pub fn signed64(&self, name: &str) -> Result<i64, PayloadRefusal> {
        self.field(name)?.signed64()
    }

    /// A 64-bit unsigned field ([`PayloadValue::unsigned64`]).
    pub fn unsigned64(&self, name: &str) -> Result<u64, PayloadRefusal> {
        self.field(name)?.unsigned64()
    }

    /// A 32-bit fractional field ([`PayloadValue::float32`]).
    pub fn float32(&self, name: &str) -> Result<f32, PayloadRefusal> {
        self.field(name)?.float32()
    }

    /// A 64-bit fractional field.
    pub fn float64(&self, name: &str) -> Result<f64, PayloadRefusal> {
        self.field(name)?.float64()
    }

    /// A truth-value field.
    pub fn truth(&self, name: &str) -> Result<bool, PayloadRefusal> {
        self.field(name)?.truth()
    }

    /// A text field of at most `bound` UTF-8 bytes (docs/adr/0086, section
    /// 4.4), held to its bound by [`PayloadValue::text_within`], the reader a
    /// text at any depth is held by: one past it is refused, and never cut.
    pub fn text(&self, name: &str, bound: usize) -> Result<crate::SceString, PayloadRefusal> {
        self.field(name)?
            .text_within(bound)
            .map(crate::sce_string_from_str)
    }

    /// A byte-string field at the capacity its schema declares
    /// ([`PayloadValue::bytes`]).
    pub fn bytes<const CAP: usize>(
        &self,
        name: &str,
    ) -> Result<crate::SceBytes<CAP>, PayloadRefusal> {
        self.field(name)?.bytes::<CAP>()
    }
}

/// The members of an object a record type is read from, at the path that names
/// the record, and the defaults of the fields a later revision of its schema
/// added.
#[derive(Clone, Copy, Debug)]
pub struct PayloadRecord<'a> {
    members: &'a [(String, Value)],
    defaults: Option<&'a [(String, Value)]>,
    prefix: &'a str,
    pointer: &'a str,
}

impl<'a> PayloadRecord<'a> {
    /// The value of the field `name`: the delivered one, else the default a
    /// later revision of the record's schema gave it, else a refusal naming the
    /// path. A member the schema does not declare is never asked for, so it is
    /// stepped over.
    pub fn field(&self, name: &str) -> Result<PayloadValue<'a>, PayloadRefusal> {
        let path = join(self.prefix, name);
        let pointer = pointer_below(self.pointer, name);
        let found = self
            .members
            .iter()
            .chain(self.defaults.unwrap_or_default())
            .find(|(key, _)| key == name)
            .map(|(_, value)| value);
        match found {
            Some(value) => Ok(PayloadValue {
                value,
                path,
                pointer,
            }),
            None => Err(PayloadRefusal::new(
                RefusalCode::Missing,
                pointer,
                format!("the event's data has no '{path}'"),
            )),
        }
    }
}

/// One value of an event's data, at the path through every depth that names
/// it: a field's name, then each index and field name below it, joined by `/`;
/// and at the strict RFC 6901 pointer to the same place, which a refusal
/// carries beside its sentence.
#[derive(Clone, Debug)]
pub struct PayloadValue<'a> {
    value: &'a Value,
    path: String,
    pointer: String,
}

impl<'a> PayloadValue<'a> {
    /// The path that names this value.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The strict RFC 6901 pointer to this value (`/items/2`).
    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    /// Whether the value is `null`, the one spelling of an absent value.
    pub fn is_null(&self) -> bool {
        matches!(self.value, Value::Null)
    }

    /// A refusal of this value, of class `code`, saying `what` of it after its
    /// path: `'items/1/level' (x) is not a variant of Level`.
    pub fn refuse(&self, code: RefusalCode, what: &str) -> PayloadRefusal {
        PayloadRefusal::new(
            code,
            self.pointer.clone(),
            format!("'{}' {what}", self.path),
        )
    }

    /// The refusal of the text `name` this value carries, which the enum
    /// `alias` does not declare a variant by.
    pub fn unknown_variant(&self, name: &str, alias: &str) -> PayloadRefusal {
        self.refuse(
            RefusalCode::UnknownVariant,
            &format!("({name}) is not a variant of {alias}"),
        )
    }

    /// The refusal of `null` where the type is not optional, or the refusal
    /// `otherwise` of any other value of the wrong kind.
    fn wrong_kind(&self, otherwise: &str) -> PayloadRefusal {
        if self.is_null() {
            self.refuse(
                RefusalCode::NullNotOptional,
                "is null, and its type is not optional",
            )
        } else {
            self.refuse(RefusalCode::WrongKind, otherwise)
        }
    }

    /// A whole-number value's written spelling.
    ///
    /// ⚠ A truth value is not a number here. JSON spells both, and a schema
    /// that declared `uint32` and received `true` has been handed something its
    /// own type says cannot occur.
    fn number(&self) -> Result<&'a str, PayloadRefusal> {
        match self.value {
            Value::Number(text) => Ok(text),
            _ => Err(self.wrong_kind("is not a number")),
        }
    }

    /// A signed whole number at the width its type declares.
    ///
    /// A value the width cannot hold is refused rather than wrapped — silently
    /// truncating is how a guard on a boundary answers for a value the document
    /// never received.
    pub fn signed<T>(&self) -> Result<T, PayloadRefusal>
    where
        T: TryFrom<i64>,
    {
        let text = self.number()?;
        // `-0` is a JSON number and no spelling of a whole number: every runtime
        // refuses it, as it refuses `"-0"` for a 64-bit integer.
        if text == "-0" {
            return Err(self.refuse(
                RefusalCode::BadNumber,
                &format!("is not a whole number ({text})"),
            ));
        }
        let whole: i64 = text.parse().map_err(|_| {
            // A whole number no `i64` holds is past every signed width; any
            // other spelling is no whole number at all.
            let code = if spells_whole(text) {
                RefusalCode::OutOfRange
            } else {
                RefusalCode::BadNumber
            };
            self.refuse(code, &format!("is not a whole number ({text})"))
        })?;
        T::try_from(whole).map_err(|_| {
            self.refuse(
                RefusalCode::OutOfRange,
                &format!("does not fit the width its schema declares ({text})"),
            )
        })
    }

    /// An unsigned whole number at the width its type declares.
    pub fn unsigned<T>(&self) -> Result<T, PayloadRefusal>
    where
        T: TryFrom<u64>,
    {
        let text = self.number()?;
        let whole: u64 = text.parse().map_err(|_| {
            // Below zero is no spelling of an unsigned number; a whole number
            // no `u64` holds is past every unsigned width.
            let code = if spells_whole(text) && !text.starts_with('-') {
                RefusalCode::OutOfRange
            } else {
                RefusalCode::BadNumber
            };
            self.refuse(
                code,
                &format!("is not a whole number at or above zero ({text})"),
            )
        })?;
        T::try_from(whole).map_err(|_| {
            self.refuse(
                RefusalCode::OutOfRange,
                &format!("does not fit the width its schema declares ({text})"),
            )
        })
    }

    /// A 64-bit whole number's written spelling: its decimal digits as a text,
    /// or a JSON number when it is exact in a double (`|n| <= 2^53 - 1`).
    ///
    /// A 64-bit integer crosses as a text because a reader that takes a JSON
    /// number through a double carries one past 2^53 with its low bits wrong and
    /// no error; a number past that is refused here on every engine, so the
    /// same payload gives the same field wherever it is read.
    fn wide(&self, signed: bool) -> Result<&'a str, PayloadRefusal> {
        match self.value {
            Value::Text(text) if is_decimal(text, signed) => Ok(text),
            Value::Text(text) => Err(self.refuse(
                RefusalCode::BadNumber,
                &format!("is not a whole number written as digits ({text})"),
            )),
            // `-0` is no whole number, and a number below zero no spelling of an
            // unsigned one, as for every narrower width.
            Value::Number(text) if text == "-0" || (!signed && text.starts_with('-')) => Err(self
                .refuse(
                    RefusalCode::BadNumber,
                    &format!("is not a whole number of its type ({text})"),
                )),
            Value::Number(text) => {
                let exact = text
                    .parse::<i128>()
                    .map(|n| n.unsigned_abs() <= EXACT_IN_A_DOUBLE)
                    .map_err(|_| {
                        self.refuse(
                            RefusalCode::BadNumber,
                            &format!("is not a whole number ({text})"),
                        )
                    })?;
                if exact {
                    Ok(text)
                } else {
                    Err(self.refuse(
                        RefusalCode::BadNumber,
                        &format!(
                            "is a number past 2^53 - 1, which is not exact everywhere ({text}); \
                             a 64-bit integer that large is written as a text of its digits"
                        ),
                    ))
                }
            }
            _ => Err(self.wrong_kind("is not a number")),
        }
    }

    /// A 64-bit signed value: the canonical text of its digits, or a JSON
    /// number no larger than 2^53 - 1 in magnitude.
    pub fn signed64(&self) -> Result<i64, PayloadRefusal> {
        let text = self.wide(true)?;
        text.parse().map_err(|_| self.past_width(text))
    }

    /// A 64-bit unsigned value: the canonical text of its digits, or a JSON
    /// number no larger than 2^53 - 1.
    pub fn unsigned64(&self) -> Result<u64, PayloadRefusal> {
        let text = self.wide(false)?;
        text.parse().map_err(|_| self.past_width(text))
    }

    /// The refusal of a number past the width its type declares.
    fn past_width(&self, text: &str) -> PayloadRefusal {
        self.refuse(
            RefusalCode::OutOfRange,
            &format!("does not fit the width its schema declares ({text})"),
        )
    }

    /// A 32-bit fractional value: the binary32 nearest the number the payload
    /// carries.
    ///
    /// A JSON number is a double wherever it is read, so the single is the one
    /// nearest that double — every engine rounds once, from the same double —
    /// and a number past the largest single does not fit the field, as a whole
    /// number past its width does not. Parsing the text to an `f32` directly
    /// would round from the text instead, and give infinity for `1e39`.
    pub fn float32(&self) -> Result<f32, PayloadRefusal> {
        let text = self.number()?;
        let wide: f64 = text.parse().map_err(|_| self.not_a_real(text))?;
        if wide.abs() > f64::from(f32::MAX) {
            return Err(self.past_width(text));
        }
        Ok(wide as f32)
    }

    /// A 64-bit fractional value. A number past the largest double (`1e999`)
    /// does not fit it, as one past the largest single does not fit a
    /// `float32`: no runtime holds the infinity a reader would round it to.
    pub fn float64(&self) -> Result<f64, PayloadRefusal> {
        let text = self.number()?;
        let real: f64 = text.parse().map_err(|_| self.not_a_real(text))?;
        if !real.is_finite() {
            return Err(self.past_width(text));
        }
        Ok(real)
    }

    /// The refusal of a number written in no spelling a real is read from.
    fn not_a_real(&self, text: &str) -> PayloadRefusal {
        self.refuse(RefusalCode::BadNumber, &format!("is not a number ({text})"))
    }

    /// A truth value.
    pub fn truth(&self) -> Result<bool, PayloadRefusal> {
        match self.value {
            Value::Bool(b) => Ok(*b),
            _ => Err(self.wrong_kind("is not a truth value")),
        }
    }

    /// The text the value carries, held to no bound: what an enum's variant is
    /// named by, which its reader matches against the names its enum declares.
    ///
    /// A text holding U+0000 is refused: a runtime that holds a text
    /// NUL-terminated would end it there, so no runtime holds one, and every
    /// reader refuses the same texts (docs/adr/0087, section 2.3). A byte
    /// string may hold the byte zero ([`Self::bytes`]).
    pub fn name(&self) -> Result<&'a str, PayloadRefusal> {
        match self.value {
            Value::Text(text) if text.contains('\0') => Err(self.refuse(
                RefusalCode::NulInText,
                "carries U+0000, which no text holds",
            )),
            Value::Text(text) => Ok(text),
            _ => Err(self.wrong_kind("is not a text")),
        }
    }

    /// A text of at most `N` UTF-8 bytes (docs/adr/0086, section 4.4): one past
    /// the bound is refused, at whatever depth it stands, and never cut.
    pub fn text<const N: usize>(&self) -> Result<sce_portable_bytes::SceString<N>, PayloadRefusal> {
        let text = self.text_within(N)?;
        sce_portable_bytes::SceString::<N>::from_text(text)
            .map_err(|_| self.past_bytes(text.len(), N))
    }

    /// The text this value carries, of at most `bound` UTF-8 bytes: the one
    /// check of a text's bound, which [`Self::text`] holds a text in a bounded
    /// type by and [`PayloadFields::text`] a payload's own text field by.
    pub fn text_within(&self, bound: usize) -> Result<&'a str, PayloadRefusal> {
        let text = self.name()?;
        if text.len() > bound {
            return Err(self.past_bytes(text.len(), bound));
        }
        Ok(text)
    }

    /// A byte string of at most `N` bytes.
    ///
    /// JSON has no byte string, so the wire carries the byte-exact Latin-1 text
    /// the inject seam writes, and this reads it back the same way: every one
    /// of the 256 values is one character and back. Printable ASCII — what a
    /// bytes guard compares — is the same bytes under either reading. The bound
    /// is held before the copy, so a refusal names the length the delivery
    /// carried, and [`crate::SceBytes`] holds it again on every profile.
    pub fn bytes<const N: usize>(&self) -> Result<crate::SceBytes<N>, PayloadRefusal> {
        let text = match self.value {
            Value::Text(s) => s,
            _ => return Err(self.wrong_kind("is not a byte string")),
        };
        let mut out: Vec<u8> = Vec::with_capacity(text.len());
        for c in text.chars() {
            if (c as u32) > 0xFF {
                return Err(self.refuse(
                    RefusalCode::NotByteText,
                    "carries a character above U+00FF, which no single byte spells",
                ));
            }
            out.push(c as u8);
        }
        if out.len() > N {
            return Err(self.past_bytes(out.len(), N));
        }
        crate::SceBytes::<N>::from_slice(&out).map_err(|_| self.past_bytes(out.len(), N))
    }

    fn past_bytes(&self, held: usize, bound: usize) -> PayloadRefusal {
        self.refuse(
            RefusalCode::TooLong,
            &format!("is {held} bytes, past the {bound} its type declares"),
        )
        .with_bound(bound)
    }

    fn past_count(&self, held: usize, bound: usize) -> PayloadRefusal {
        self.refuse(
            RefusalCode::TooMany,
            &format!("holds {held} elements, past the {bound} its type declares"),
        )
        .with_bound(bound)
    }

    fn short_count(&self, held: usize, least: usize) -> PayloadRefusal {
        self.refuse(RefusalCode::TooFew, &count_short(held, least))
            .with_bound(least)
    }

    /// A value of `optional<T>` (docs/adr/0055): absent where the value is
    /// `null`, and otherwise what `read` reads of it, held to `T`.
    pub fn optional<T>(
        &self,
        read: impl FnOnce(&Self) -> Result<T, PayloadRefusal>,
    ) -> Result<Option<T>, PayloadRefusal> {
        if self.is_null() {
            Ok(None)
        } else {
            read(self).map(Some)
        }
    }

    /// A value of `list<T>[N]`: an array of at most `N` elements, each read by
    /// `read` at the path of its index, in order.
    ///
    /// The bound is held before any element is read: an array longer than `N`
    /// is refused whole, and no element of it is decoded (ADR 0077, section 2).
    pub fn list<T, const N: usize>(
        &self,
        read: impl FnMut(&PayloadValue<'a>) -> Result<T, PayloadRefusal>,
    ) -> Result<BoundedList<T, N>, PayloadRefusal> {
        self.list_within::<T, N>(0, read)
    }

    /// A value of `list<T>[least..N]`: an array of at least `least` and at
    /// most `N` elements (docs/adr/0086, section 15), each read by `read` at
    /// the path of its index, in order.
    ///
    /// Both counts are held before any element is read: an array longer than
    /// `N` is refused as [`RefusalCode::TooMany`], one shorter than `least` as
    /// [`RefusalCode::TooFew`] naming `least`, and no element of either is
    /// decoded.
    pub fn list_within<T, const N: usize>(
        &self,
        least: usize,
        mut read: impl FnMut(&PayloadValue<'a>) -> Result<T, PayloadRefusal>,
    ) -> Result<BoundedList<T, N>, PayloadRefusal> {
        let Value::Array(items) = self.value else {
            return Err(self.wrong_kind("is not a list"));
        };
        if items.len() > N {
            return Err(self.past_count(items.len(), N));
        }
        if items.len() < least {
            return Err(self.short_count(items.len(), least));
        }
        let mut list = BoundedList::<T, N>::new();
        for (index, value) in items.iter().enumerate() {
            let token = index.to_string();
            let element = PayloadValue {
                value,
                path: join(&self.path, &token),
                pointer: pointer_below(&self.pointer, &token),
            };
            // The length was held above, so the push cannot refuse; the arm is
            // the same refusal, for a bound the two could disagree about.
            list.push(read(&element)?)
                .map_err(|_| self.past_count(items.len(), N))?;
        }
        Ok(list)
    }

    /// A value of a record type: an object whose fields `read` reads through
    /// the record it is handed. `defaults`, for a record whose schema a later
    /// revision added fields to, is the JSON object of their defaults, which
    /// the generator spells from that schema, and a field the value lacks is
    /// read out of it (docs/adr/0086, section 10.3).
    pub fn record<T>(
        &self,
        defaults: Option<&str>,
        read: impl FnOnce(&PayloadRecord<'_>) -> Result<T, PayloadRefusal>,
    ) -> Result<T, PayloadRefusal> {
        let Value::Object(members) = self.value else {
            return Err(self.wrong_kind("is not a record"));
        };
        let defaults = defaults.map(defaults_of).transpose()?;
        let record = PayloadRecord {
            members,
            defaults: defaults.as_deref(),
            prefix: &self.path,
            pointer: &self.pointer,
        };
        read(&record)
    }

    /// A value of a union type (docs/adr/0092, section 4): one object holding
    /// its tag under the key `tag` and the fields of the case the tag names
    /// beside it.
    ///
    /// The tag is read first, by `read_tag`, the reader of the tag's enum,
    /// wherever it stands in the object; `read_case` is then handed the case
    /// the tag named and this same value, and reads the case's record from it
    /// with [`Self::record`] (an empty case reads nothing). The refusals come
    /// in one order on every runtime: a value that is not an object; the tag
    /// absent ([`RefusalCode::Missing`] at the tag's pointer), then whatever
    /// the enum's reader refuses of it (`null`, not a text, a name the enum
    /// does not declare); then the case's record. A key the case does not
    /// declare is never asked for, so it is stepped over, as a record steps
    /// over one, and a text that carries another case's keys, or `null` under
    /// them, reads as the case its tag names.
    pub fn union<E, T>(
        &self,
        tag: &str,
        read_tag: impl FnOnce(&PayloadValue<'_>) -> Result<E, PayloadRefusal>,
        read_case: impl FnOnce(E, &PayloadValue<'a>) -> Result<T, PayloadRefusal>,
    ) -> Result<T, PayloadRefusal> {
        let Value::Object(members) = self.value else {
            return Err(self.wrong_kind("is not a union"));
        };
        let object = PayloadRecord {
            members,
            defaults: None,
            prefix: &self.path,
            pointer: &self.pointer,
        };
        let case = read_tag(&object.field(tag)?)?;
        read_case(case, self)
    }
}

/// The members of a schema's defaults, which the generator spells as one JSON
/// object. A text that is none is the generator's, not a sender's, and is
/// refused as no JSON object of the whole.
fn defaults_of(defaults: &str) -> Result<Vec<(String, Value)>, PayloadRefusal> {
    match crate::json::parse(defaults.trim()) {
        Ok(Value::Object(members)) => Ok(members),
        _ => Err(PayloadRefusal::new(
            RefusalCode::NotJson,
            "",
            "the schema's defaults are not a JSON object, which the generator never writes",
        )),
    }
}

/// Whether `text` spells a whole number: digits, after a `-` or not.
fn spells_whole(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// `segment` below `prefix`, as a JSON Pointer's reference tokens are joined.
fn join(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{prefix}/{segment}")
    }
}

/// The largest whole number every engine holds exactly as a number (`2^53 - 1`,
/// ECMAScript's `Number.MAX_SAFE_INTEGER`), and so the largest a JSON number may
/// carry for a 64-bit field.
const EXACT_IN_A_DOUBLE: u128 = (1 << 53) - 1;

/// Whether `text` is a whole number written as digits and nothing else: no
/// sign but `-` (and only where `signed`), no leading zero, no `-0`.
fn is_decimal(text: &str, signed: bool) -> bool {
    let digits = match text.strip_prefix('-') {
        Some(rest) if signed => rest,
        Some(_) => return false,
        None => text,
    };
    !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && !(digits == "0" && text.starts_with('-'))
}

/// The wire spelling of one byte string, for the inject seam's `data`.
///
/// The writing half of [`PayloadValue::bytes`] — see there for why Latin-1.
pub fn bytes_as_payload_text(bytes: &[u8]) -> String {
    bytes.iter().map(|b| *b as char).collect()
}

/// The JSON spelling of one text, for the inject seam's `data` — the
/// runtime's one writer ([`crate::json::quote`]).
pub fn quote(text: &str) -> String {
    crate::json::quote(text)
}

/// The JSON spelling of one optional value, for the inject seam's `data`
/// (docs/adr/0055, section 4): an absent value is `null` and a present one is
/// what `write` spells it as. The key is always written, so a reader never
/// takes a sender that knows the field for one that does not
/// ([`PayloadFields::optional`]).
pub fn optional_json<T>(value: Option<T>, write: impl FnOnce(T) -> String) -> String {
    value.map_or_else(|| "null".to_string(), write)
}

/// Why a list of `held` elements is no value of a type whose lower count is
/// `least`: one sentence for the reader and the writer's check, the path in
/// front of it where the caller has one.
fn count_short(held: usize, least: usize) -> String {
    format!("holds {held} elements, fewer than the {least} its type declares")
}

/// Hold a list a machine or a host built, `held` elements long, to the lower
/// count `least` of the place it is written into (docs/adr/0086, section
/// 15.6), before the event that carries it is encoded: the one bound a
/// `BoundedList<T, N>` does not hold by its type, since a list is built from
/// empty an element at a time. `Err` is the refusal's sentence, naming `path`,
/// in the words the reader uses; the event is refused whole.
pub fn check_count(path: &str, held: usize, least: usize) -> Result<(), String> {
    if held < least {
        Err(format!("'{path}' {}", count_short(held, least)))
    } else {
        Ok(())
    }
}

/// The JSON spelling of one list, for the inject seam's `data`
/// (docs/adr/0086, section 3.1): an array of what `write` spells each element
/// as, in the list's order. The empty list is `[]`.
pub fn list_json<T>(items: impl IntoIterator<Item = T>, write: impl FnMut(T) -> String) -> String {
    let mut out = String::from("[");
    for (index, item) in items.into_iter().map(write).enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&item);
    }
    out.push(']');
    out
}

/// The JSON spelling of one record, for the inject seam's `data`: each member
/// is a field's id and the JSON its value is spelled as, and the object writes
/// them ascending by the id's UTF-8 bytes, whatever order they are handed in
/// (ARCHITECTURE.md, "JSON Object Key Order", at every depth).
pub fn record_json(mut members: Vec<(&str, String)>) -> String {
    members.sort_unstable_by_key(|(id, _)| *id);
    let mut out = String::from("{");
    for (index, (id, value)) in members.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&quote(id));
        out.push(':');
        out.push_str(value);
    }
    out.push('}');
    out
}

/// The JSON spelling of one union value (docs/adr/0092, section 2.1): the tag
/// under the key `tag`, its value `variant`, the case's name as its enum spells
/// it, beside `members`, the fields of the case's record, each an id and the
/// JSON its value is spelled as (none for an empty case). The object writes the
/// tag among the fields, every key ascending by its UTF-8 bytes, as
/// [`record_json`] writes a record.
pub fn union_json<'k>(tag: &'k str, variant: &str, mut members: Vec<(&'k str, String)>) -> String {
    members.push((tag, quote(variant)));
    record_json(members)
}

#[cfg(test)]
mod tests {
    use super::{
        list_json, optional_json, quote, record_json, union_json, PayloadFields, PayloadRefusal,
        PayloadValue, RefusalCode,
    };

    const DEFAULTS: &str = r#"{"layout":"week","count":"7","urgent":true}"#;

    /// `value` spelled through the typed payload writers, one call per
    /// constructor, as a generated inject seam spells a value of its type: an
    /// object as a record ([`record_json`]), an array as a list ([`list_json`]),
    /// a text by [`quote`], a number and a truth value as written.
    fn typed_json(value: &crate::json::Value) -> String {
        use crate::json::Value;
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.clone(),
            Value::Text(t) => quote(t),
            Value::Array(items) => list_json(items, typed_json),
            Value::Object(members) => record_json(
                members
                    .iter()
                    .map(|(id, member)| (id.as_str(), typed_json(member)))
                    .collect(),
            ),
        }
    }

    /// tests/json_text/object_key_order.json, written by the typed writers a
    /// generated inject seam spells a record and a list with (docs/adr/0086,
    /// section 5.3), at every depth: the same order as every engine's
    /// `_event.data` (ARCHITECTURE.md, "JSON Object Key Order"). A case whose
    /// name repeats is a `<send>`'s list of params, which only the parameter
    /// writer collects: a record holds each field once.
    #[test]
    fn a_record_and_a_list_are_written_in_the_one_order_every_engine_writes() {
        use crate::json::{parse, Value};
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tests/json_text/object_key_order.json"
        );
        let table = std::fs::read_to_string(path).expect("the shared key-order table");
        let table = parse(&table).expect("the table is JSON");
        let Some(Value::Array(cases)) = table.member("cases") else {
            panic!("the table has cases");
        };
        let mut written = 0;
        for case in cases {
            let (Some(Value::Text(name)), Some(Value::Text(data)), Some(Value::Array(params))) = (
                case.member("name"),
                case.member("data"),
                case.member("params"),
            ) else {
                panic!("a case has a name, params and data");
            };
            let members: Vec<(&str, &Value)> = params
                .iter()
                .map(|pair| match pair {
                    Value::Array(pair) => match pair.as_slice() {
                        [Value::Text(id), value] => (id.as_str(), value),
                        _ => panic!("{name}: a param is a [name, value] pair"),
                    },
                    _ => panic!("{name}: a param is a [name, value] pair"),
                })
                .collect();
            let mut ids: Vec<&str> = members.iter().map(|(id, _)| *id).collect();
            ids.sort_unstable();
            ids.dedup();
            if ids.len() < members.len() {
                continue;
            }
            let text = record_json(
                members
                    .iter()
                    .map(|(id, value)| (*id, typed_json(value)))
                    .collect(),
            );
            assert_eq!(&text, data, "{name}");
            written += 1;
        }
        assert!(
            written >= 10,
            "the table lost the cases a record is written for: {written}"
        );
    }

    /// The value `v` read as the type `ty` the shared refusal table describes,
    /// one call per constructor, as a generated reader reads it. The table's
    /// bounds are 2, 3 and 8, each read through the instance of its bound.
    fn read_described(
        ty: &crate::json::Value,
        v: &super::PayloadValue<'_>,
    ) -> Result<(), PayloadRefusal> {
        use crate::json::Value;
        let bound = |key: &str| match ty.member(key) {
            Some(Value::Number(n)) => n.clone(),
            other => panic!("{key} is a number: {other:?}"),
        };
        match ty {
            Value::Text(scalar) => match scalar.as_str() {
                "uint8" => v.unsigned::<u8>().map(|_| ()),
                "uint16" => v.unsigned::<u16>().map(|_| ()),
                "uint32" => v.unsigned::<u32>().map(|_| ()),
                "uint64" => v.unsigned64().map(|_| ()),
                "int8" => v.signed::<i8>().map(|_| ()),
                "int16" => v.signed::<i16>().map(|_| ()),
                "int32" => v.signed::<i32>().map(|_| ()),
                "int64" => v.signed64().map(|_| ()),
                "float32" => v.float32().map(|_| ()),
                "float64" => v.float64().map(|_| ()),
                "bool" => v.truth().map(|_| ()),
                "string" => v.name().map(|_| ()),
                other => panic!("the table names no type {other}"),
            },
            Value::Array(names) => {
                let name = v.name()?;
                if names
                    .iter()
                    .any(|declared| matches!(declared, Value::Text(t) if t == name))
                {
                    Ok(())
                } else {
                    Err(v.unknown_variant(name, "Described"))
                }
            }
            Value::Object(_) => {
                if let Some(inner) = ty.member("optional") {
                    return v
                        .optional(|present| read_described(inner, present))
                        .map(|_| ());
                }
                if ty.member("string").is_some() {
                    return match bound("string").as_str() {
                        "2" => v.text::<2>().map(|_| ()),
                        "3" => v.text::<3>().map(|_| ()),
                        "8" => v.text::<8>().map(|_| ()),
                        other => panic!("the table's bounds are 2, 3 and 8, not {other}"),
                    };
                }
                if ty.member("bytes").is_some() {
                    return match bound("bytes").as_str() {
                        "2" => v.bytes::<2>().map(|_| ()),
                        "3" => v.bytes::<3>().map(|_| ()),
                        "8" => v.bytes::<8>().map(|_| ()),
                        other => panic!("the table's bounds are 2, 3 and 8, not {other}"),
                    };
                }
                if let Some(inner) = ty.member("list") {
                    let each = |element: &super::PayloadValue<'_>| read_described(inner, element);
                    // The lower count, where the case states one (docs/adr/0086,
                    // section 15).
                    let least = match ty.member("min") {
                        Some(Value::Number(n)) => n.parse::<usize>().expect("a lower count"),
                        _ => 0,
                    };
                    return match bound("max").as_str() {
                        "2" => v.list_within::<(), 2>(least, each).map(|_| ()),
                        "3" => v.list_within::<(), 3>(least, each).map(|_| ()),
                        "8" => v.list_within::<(), 8>(least, each).map(|_| ()),
                        other => panic!("the table's bounds are 2, 3 and 8, not {other}"),
                    };
                }
                if let Some(Value::Object(fields)) = ty.member("record") {
                    return v.record(None, |record| {
                        for (id, field) in fields {
                            read_described(field, &record.field(id)?)?;
                        }
                        Ok(())
                    });
                }
                if let Some(described) = ty.member("union") {
                    // The tag's enum is the cases' names; an empty case is `null`,
                    // any other the record it holds (docs/adr/0092, section 4).
                    let (Some(Value::Text(tag)), Some(Value::Object(cases))) =
                        (described.member("tag"), described.member("cases"))
                    else {
                        panic!("a union names its tag and its cases: {described:?}");
                    };
                    return v.union(
                        tag,
                        |named| {
                            let name = named.name()?;
                            cases
                                .iter()
                                .find(|(declared, _)| declared.as_str() == name)
                                .map(|(_, case)| case)
                                .ok_or_else(|| named.unknown_variant(name, "Described"))
                        },
                        |case, object| match case {
                            Value::Null => Ok(()),
                            record => read_described(record, object),
                        },
                    );
                }
                panic!("a type of the table names no constructor: {ty:?}")
            }
            other => panic!("a type of the table is a name, a list or an object: {other:?}"),
        }
    }

    /// tests/json_text/payload_refusal.json: every case read through this
    /// runtime's reader, the code, pointer and bound of each refusal held to
    /// what the table says, and each case it accepts read (docs/adr/0087,
    /// section 9.1, item 3). The table is the one every runtime's reader is
    /// held to.
    #[test]
    fn every_case_of_the_shared_table_is_refused_with_its_code_pointer_and_bound() {
        use crate::json::{parse, Value};
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tests/json_text/payload_refusal.json"
        );
        let table = std::fs::read_to_string(path).expect("the shared refusal table");
        let table = parse(&table).expect("the table is JSON");
        let Some(Value::Array(cases)) = table.member("cases") else {
            panic!("the table has cases");
        };
        assert!(cases.len() >= 40, "the table lost cases: {}", cases.len());
        let mut failures = Vec::new();
        for case in cases {
            let (Some(Value::Text(name)), Some(Value::Text(data)), Some(ty)) = (
                case.member("name"),
                case.member("data"),
                case.member("type"),
            ) else {
                panic!("a case has a name, data and a type: {case:?}");
            };
            // A case that gives a depth is read as a `<sce:lift>` of the record
            // of the one field `v` reads it (docs/adr/0087, section 2.1).
            // Its pointer is held to `pathBound` bytes, the standard refusal's 256
            // where the case gives none.
            let path_bound = match case.member("pathBound") {
                Some(Value::Number(bound)) => bound.parse::<u32>().expect("a uint32"),
                _ => 256,
            };
            let fields = match case.member("depth") {
                Some(Value::Number(depth)) => PayloadFields::lift(
                    data,
                    depth.parse::<u32>().expect("a depth is a uint32"),
                    path_bound,
                    None,
                ),
                _ => PayloadFields::decode(data),
            };
            let read = fields.and_then(|fields| read_described(ty, &fields.field("v")?));
            match (case.member("refusal"), read) {
                (None, Ok(())) => {}
                (Some(want), Err(got)) => {
                    let code = match want.member("code") {
                        Some(Value::Text(code)) => RefusalCode::from_name(code)
                            .unwrap_or_else(|| panic!("{name}: the enum declares no {code}")),
                        other => panic!("{name}: a refusal has a code: {other:?}"),
                    };
                    let pointer = match want.member("pointer") {
                        Some(Value::Text(pointer)) => pointer.as_str(),
                        other => panic!("{name}: a refusal has a pointer: {other:?}"),
                    };
                    let bound = match want.member("bound") {
                        Some(Value::Number(n)) => Some(n.parse::<u32>().expect("a uint32")),
                        Some(Value::Null) => None,
                        other => panic!("{name}: a refusal has a bound or null: {other:?}"),
                    };
                    if (got.code(), got.pointer(), got.bound()) != (code, pointer, bound) {
                        failures.push(format!(
                            "{name}: refused {:?} at {:?} bound {:?} ({got}), the table says \
                             {code:?} at {pointer:?} bound {bound:?}",
                            got.code(),
                            got.pointer(),
                            got.bound()
                        ));
                    }
                }
                (want, got) => {
                    failures.push(format!("{name}: the table says {want:?}, read {got:?}"))
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// A lift refuses a container past its depth over the brackets alone,
    /// before the text is read as JSON, and names the container it refused
    /// (docs/adr/0087, sections 2.1 and 2.4).
    #[test]
    fn a_lift_refuses_a_container_past_its_depth_before_it_reads_the_text() {
        let refused = |text: &str, depth: u32, path_bound: u32| {
            super::lift_too_deep(text, depth, path_bound)
                .map(|r| (r.code(), r.pointer().to_string(), r.bound()))
        };
        let too_deep = |pointer: &str, bound: u32| {
            Some((RefusalCode::TooDeep, pointer.to_string(), Some(bound)))
        };
        assert_eq!(refused(r#"{"title":"a","count":1}"#, 1, 256), None);
        assert_eq!(refused(r#"{"title":["a"]}"#, 1, 256), too_deep("/title", 1));
        assert_eq!(
            refused(r#"{"extra":{"a":1}}"#, 1, 256),
            too_deep("/extra", 1)
        );
        assert_eq!(refused("[[1]]", 1, 256), too_deep("/0", 1));
        assert_eq!(refused("[1,2,[3]]", 1, 256), too_deep("/2", 1));
        // Past the depth and never closed: refused before the grammar is asked.
        assert_eq!(
            refused(r#"{"extra":[[[[[[["#, 1, 256),
            too_deep("/extra", 1)
        );
        // A bracket in a text is no container, an escaped quote no end of one.
        assert_eq!(refused(r#"{"title":"[[{\"[","count":1}"#, 1, 256), None);
        // A container one level deeper than a record is within a depth of 2.
        assert_eq!(refused(r#"{"title":{"a":1}}"#, 2, 256), None);
        // A text that never closes is the grammar's to refuse.
        assert_eq!(refused(r#"{"title":"[["#, 1, 256), None);
        // A member after a comma is named by its own text, not the one before.
        assert_eq!(
            refused(r#"{"title":"x","extra":[1]}"#, 1, 256),
            too_deep("/extra", 1)
        );
        // The container under a nested key, and inside an array.
        assert_eq!(refused(r#"{"a":{"b":[1]}}"#, 2, 256), too_deep("/a/b", 2));
        assert_eq!(
            refused(r#"{"a":[0,{"b":[1]}]}"#, 3, 256),
            too_deep("/a/1/b", 3)
        );
        // RFC 6901: `~` is written `~0` and `/` is written `~1`.
        assert_eq!(refused(r#"{"a/b~c":[1]}"#, 1, 256), too_deep("/a~1b~0c", 1));
        // A member with no name before its colon ends the pointer where it stands.
        assert_eq!(refused("{[1]}", 1, 256), too_deep("", 1));
        // A token past the room of the refusal's path is not written, nor cut.
        assert_eq!(refused(r#"{"a":{"abcdef":[1]}}"#, 2, 4), too_deep("/a", 2));
        assert_eq!(refused(r#"{"a":{"bc":[1]}}"#, 2, 5), too_deep("/a/bc", 2));
    }

    /// What a lift reads: an empty text is not JSON, a container past the depth
    /// is refused first, a field the text lacks is read from the defaults, and
    /// the rest is a delivery's reading.
    #[test]
    fn a_lift_reads_a_text_as_a_delivery_after_its_own_refusals() {
        let code = |r: Result<PayloadFields, PayloadRefusal>| r.map(|_| ()).map_err(|r| r.code());
        assert_eq!(
            code(PayloadFields::lift(" \t", 1, 256, None)),
            Err(RefusalCode::NotJson)
        );
        assert_eq!(
            code(PayloadFields::lift("[[", 1, 256, None)),
            Err(RefusalCode::TooDeep)
        );
        assert_eq!(
            code(PayloadFields::lift("[1]", 1, 256, None)),
            Err(RefusalCode::NotObject)
        );
        assert_eq!(
            code(PayloadFields::lift(r#"{"v":1,"v":2}"#, 1, 256, None)),
            Err(RefusalCode::NotJson)
        );
        let read = PayloadFields::lift(r#"{"v":1}"#, 1, 256, Some(r#"{"w":7}"#))
            .expect("a text with a field from the defaults");
        assert_eq!(read.unsigned::<u8>("w"), Ok(7));
        assert_eq!(read.unsigned::<u8>("v"), Ok(1));
    }

    /// The codes are the variants `sce:std/lift/refusal_code.scxml` declares,
    /// by the same names and at the same numbers, so a refusal is one value of
    /// that enum on every runtime.
    #[test]
    fn every_code_is_the_variant_the_standard_enum_declares() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../stdlib/lift/refusal_code.scxml"
        );
        let document = std::fs::read_to_string(path).expect("the standard refusal code");
        let declared = document.matches("<sce:variant ").count();
        assert_eq!(declared, RefusalCode::ALL.len(), "one code per variant");
        for code in RefusalCode::ALL {
            let variant = format!(
                "<sce:variant name=\"{}\" value=\"{}\"/>",
                code.name(),
                code.number()
            );
            assert!(document.contains(&variant), "{code:?}: {variant}");
            assert_eq!(RefusalCode::from_name(code.name()), Some(code));
        }
    }

    /// A pointer is RFC 6901's: `/` before each reference token, `~` written
    /// `~0` and `/` written `~1`, and `~` escaped first so that `~1` in a name
    /// is not read back as `/`.
    #[test]
    fn a_pointer_escapes_tilde_and_solidus_in_each_token() {
        assert_eq!(super::pointer_below("", "items"), "/items");
        assert_eq!(super::pointer_below("/items", "2"), "/items/2");
        assert_eq!(super::pointer_below("", "a~b/c"), "/a~0b~1c");
        assert_eq!(super::pointer_below("", "~1"), "/~01");
        assert_eq!(super::pointer_below("", ""), "/");
    }

    /// What a generated reader of an enum field answers for a name its enum
    /// does not declare: the sentence it always wrote, and the code and pointer
    /// beside it.
    #[test]
    fn a_name_no_variant_declares_is_refused_as_unknown_variant() {
        let fields = PayloadFields::decode(r#"{"status": "x"}"#).expect("reads");
        let refusal = fields.unknown_variant("status", "x", "Status");
        assert_eq!(refusal.reason(), "status (x) is not a variant of Status");
        assert_eq!(refusal.code(), RefusalCode::UnknownVariant);
        assert_eq!(refusal.pointer(), "/status");
        assert_eq!(refusal.bound(), None);
    }

    #[test]
    fn a_field_the_delivery_lacks_is_read_out_of_the_defaults() {
        let fields = PayloadFields::decode_with_defaults(r#"{"level": 4}"#, DEFAULTS)
            .expect("a delivery of the first revision reads");
        assert_eq!(fields.unsigned::<u8>("level"), Ok(4));
        assert_eq!(
            fields.text("layout", 16).map(|s| s.to_string()),
            Ok("week".into())
        );
        assert_eq!(fields.unsigned64("count"), Ok(7));
        assert_eq!(fields.truth("urgent"), Ok(true));
    }

    #[test]
    fn a_field_the_delivery_carries_is_read_as_it_came() {
        let fields = PayloadFields::decode_with_defaults(
            r#"{"level": 4, "layout": "day", "urgent": false, "source": {"a": [1]}}"#,
            DEFAULTS,
        )
        .expect("a delivery of a later revision reads, an unknown field and all");
        assert_eq!(
            fields.text("layout", 16).map(|s| s.to_string()),
            Ok("day".into())
        );
        assert_eq!(fields.truth("urgent"), Ok(false));
        // A value of another type is refused, default or not.
        let fields = PayloadFields::decode_with_defaults(r#"{"level": 4, "urgent": 1}"#, DEFAULTS)
            .expect("the object reads");
        assert!(fields.truth("urgent").is_err());
    }

    #[test]
    fn a_field_of_the_first_revision_is_still_required() {
        let fields = PayloadFields::decode_with_defaults(r#"{"layout": "day"}"#, DEFAULTS)
            .expect("the object reads");
        let refusal = fields
            .unsigned::<u8>("level")
            .expect_err("no default stands in");
        assert!(refusal.reason().contains("'level'"), "{refusal}");
        // No data at all is a delivery that names no field.
        let fields = PayloadFields::decode_with_defaults("", DEFAULTS).expect("reads as no field");
        assert!(fields.unsigned::<u8>("level").is_err());
        assert_eq!(fields.truth("urgent"), Ok(true));
        // A bare value is still no object of named fields.
        assert!(PayloadFields::decode_with_defaults("5", DEFAULTS).is_err());
    }

    #[test]
    fn an_optional_field_reads_null_as_absent_and_a_value_as_present() {
        let fields = PayloadFields::decode(r#"{"id": 1, "due": null, "title": "soon"}"#)
            .expect("the object reads");
        assert_eq!(fields.optional("due", |f| f.signed64("due")), Ok(None));
        assert_eq!(
            fields
                .optional("title", |f| f.text("title", 16))
                .map(|t| t.map(|s| s.to_string())),
            Ok(Some("soon".to_string()))
        );
        // A present value is held to the field's type, as a field of `T` is.
        assert!(fields.optional("title", |f| f.signed64("title")).is_err());
    }

    #[test]
    fn an_optional_field_whose_key_the_delivery_lacks_is_refused() {
        let fields =
            PayloadFields::decode(r#"{"id": 3, "title": "gone"}"#).expect("the object reads");
        let refusal = fields
            .optional("due", |f| f.signed64("due"))
            .expect_err("a missing key is no absent value");
        assert!(refusal.reason().contains("'due'"), "{refusal}");
        // A later revision's default of `null` is an absent value.
        let fields = PayloadFields::decode_with_defaults(r#"{"id": 3}"#, r#"{"due":null}"#)
            .expect("the object reads");
        assert_eq!(fields.optional("due", |f| f.signed64("due")), Ok(None));
    }

    #[test]
    fn a_field_of_t_refuses_null_as_an_older_receiver_of_a_widened_field_does() {
        // A receiver built before a revision widened `due` to `optional<int64>`
        // reads it as `int64`, and a newer sender's `null` is no value of it: the
        // delivery is refused (docs/adr/0055, section 6), which is why the
        // receivers of a widened field are upgraded before its senders.
        let fields = PayloadFields::decode(r#"{"id": 1, "due": null}"#).expect("the object reads");
        let refusal = fields.signed64("due").expect_err("null is no int64");
        assert_eq!(
            refusal.reason(),
            "'due' is null, and its type is not optional"
        );
        // The widened receiver reads the same delivery, the day absent.
        assert_eq!(fields.optional("due", |f| f.signed64("due")), Ok(None));
    }

    #[test]
    fn an_absent_value_is_written_as_null_and_a_present_one_as_its_writer_spells_it() {
        let digits = |v: i64| format!("\"{v}\"");
        assert_eq!(optional_json(None, digits), "null");
        assert_eq!(optional_json(Some(12_i64), digits), "\"12\"");
        assert_eq!(optional_json(Some("soon"), quote), "\"soon\"");
    }

    #[test]
    fn a_top_level_text_and_byte_string_are_held_to_their_bound_when_read() {
        let fields = PayloadFields::decode(r#"{"note": "abcde", "blob": "abcde", "ok": "abcd"}"#)
            .expect("the object reads");
        let past = |at: &str, what: &str| {
            PayloadRefusal::new(RefusalCode::TooLong, at, what).with_bound(4)
        };
        assert_eq!(
            fields.field("note").and_then(|v| v.text::<4>()),
            Err(past(
                "/note",
                "'note' is 5 bytes, past the 4 its type declares"
            ))
        );
        assert_eq!(
            fields.field("blob").and_then(|v| v.bytes::<4>()),
            Err(past(
                "/blob",
                "'blob' is 5 bytes, past the 4 its type declares"
            ))
        );
        assert_eq!(
            fields.bytes::<4>("blob"),
            Err(past(
                "/blob",
                "'blob' is 5 bytes, past the 4 its type declares"
            ))
        );
        let ok = fields
            .field("ok")
            .and_then(|v| v.text::<4>())
            .expect("fits");
        assert_eq!(ok, "abcd");
        // A text's bound counts UTF-8 bytes: two characters, four bytes.
        let wide = PayloadFields::decode(r#"{"t": "éé"}"#).expect("reads");
        assert!(wide.field("t").and_then(|v| v.text::<3>()).is_err());
        assert!(wide.field("t").and_then(|v| v.text::<4>()).is_ok());
    }

    /// A payload's own text field, which a lift reads into the engine's text, is
    /// held to its bound by the same check as a text at any depth (docs/adr/0086,
    /// section 4.4): one past it is refused with its code, pointer and bound.
    #[test]
    fn a_text_field_of_a_payload_is_held_to_its_bound() {
        let fields = PayloadFields::decode(r#"{"note": "abcde", "wide": "éé"}"#).expect("reads");
        assert_eq!(
            fields.text("note", 4),
            Err(PayloadRefusal::new(
                RefusalCode::TooLong,
                "/note",
                "'note' is 5 bytes, past the 4 its type declares"
            )
            .with_bound(4))
        );
        assert_eq!(
            fields.text("note", 5).map(|s| s.to_string()),
            Ok("abcde".to_string())
        );
        // The bound counts UTF-8 bytes, as a text's bound does at any depth.
        assert_eq!(
            fields.text("wide", 3).map_err(|r| r.code()),
            Err(RefusalCode::TooLong)
        );
        assert_eq!(
            fields.text("wide", 4).map(|s| s.to_string()),
            Ok("éé".to_string())
        );
        // A text holding U+0000 is still refused, inside its bound.
        let nul = PayloadFields::decode("{\"t\": \"a\\u0000\"}").expect("reads");
        assert_eq!(
            nul.text("t", 8).map_err(|r| r.code()),
            Err(RefusalCode::NulInText)
        );
    }

    #[test]
    fn a_list_past_its_bound_is_refused_before_an_element_is_read() {
        let fields = PayloadFields::decode(r#"{"v": [1, 2, "x"]}"#).expect("reads");
        let mut asked = 0;
        let refusal = fields
            .field("v")
            .and_then(|v| {
                v.list::<u8, 2>(|e| {
                    asked += 1;
                    e.unsigned()
                })
            })
            .expect_err("three elements, past two");
        assert_eq!(
            refusal.reason(),
            "'v' holds 3 elements, past the 2 its type declares"
        );
        assert_eq!(asked, 0, "no element was read before the bound was held");
    }

    /// A list short of its lower count is refused as `tooFew`, naming the
    /// lower count, before any element is read; one that holds it is read
    /// (docs/adr/0086, section 15.5).
    #[test]
    fn a_list_short_of_its_lower_count_is_refused_before_an_element_is_read() {
        let fields = PayloadFields::decode(r#"{"v": ["x"], "w": [1, 2], "e": []}"#).expect("reads");
        let mut asked = 0;
        let refusal = fields
            .field("v")
            .and_then(|v| {
                v.list_within::<u8, 3>(2, |e| {
                    asked += 1;
                    e.unsigned()
                })
            })
            .expect_err("one element, short of two");
        assert_eq!(
            refusal.reason(),
            "'v' holds 1 elements, fewer than the 2 its type declares"
        );
        assert_eq!(
            (refusal.code(), refusal.pointer(), refusal.bound()),
            (RefusalCode::TooFew, "/v", Some(2))
        );
        assert_eq!(asked, 0, "no element was read before the count was held");
        let held = fields
            .field("w")
            .and_then(|w| w.list_within::<u8, 3>(2, |e| e.unsigned()))
            .expect("two elements, at least two");
        assert_eq!(held.as_slice(), &[1u8, 2]);
        assert_eq!(
            fields
                .field("e")
                .and_then(|e| e.list_within::<u8, 3>(1, |e| e.unsigned()))
                .map(|_| ())
                .map_err(|refusal| (refusal.code(), refusal.bound())),
            Err((RefusalCode::TooFew, Some(1)))
        );
        // A list that states no lower count reads the empty list.
        assert!(fields
            .field("e")
            .and_then(|e| e.list::<u8, 3>(|e| e.unsigned()))
            .is_ok());
    }

    /// The writer's count check refuses a list short of the place's lower count
    /// in the reader's words, the path in front (docs/adr/0086, section 15.6).
    #[test]
    fn a_list_written_short_of_its_lower_count_is_refused() {
        assert_eq!(super::check_count("items", 3, 3), Ok(()));
        assert_eq!(super::check_count("items", 4, 3), Ok(()));
        assert_eq!(
            super::check_count("items/1/tags", 0, 1),
            Err("'items/1/tags' holds 0 elements, fewer than the 1 its type declares".to_string())
        );
    }

    #[test]
    fn a_record_value_names_its_missing_field_and_reads_its_later_fields_from_defaults() {
        let fields =
            PayloadFields::decode(r#"{"r": {"a": 1, "zz": [null]}, "s": 5}"#).expect("reads");
        let r = fields.field("r").expect("present");
        assert_eq!(
            r.record(None, |rec| rec.field("b").map(|_| ())),
            Err(PayloadRefusal::new(
                RefusalCode::Missing,
                "/r/b",
                "the event's data has no 'r/b'"
            ))
        );
        assert_eq!(
            r.record(Some(r#"{"b": 9}"#), |rec| rec.field("b")?.unsigned::<u8>()),
            Ok(9)
        );
        assert_eq!(
            fields.field("s").and_then(|s| s.record(None, |_| Ok(()))),
            Err(PayloadRefusal::new(
                RefusalCode::WrongKind,
                "/s",
                "'s' is not a record"
            ))
        );
        assert_eq!(
            r.record(Some("[]"), |_| Ok(())),
            Err(PayloadRefusal::new(
                RefusalCode::NotJson,
                "",
                "the schema's defaults are not a JSON object, which the generator never writes"
            ))
        );
    }

    #[test]
    fn a_list_and_a_record_are_written_in_their_order_and_by_their_key_bytes() {
        assert_eq!(list_json(Vec::<u8>::new(), |v| v.to_string()), "[]");
        assert_eq!(list_json([2u8, 1], |v| v.to_string()), "[2,1]");
        assert_eq!(
            record_json(vec![
                ("b", "2".into()),
                ("aa", "1".into()),
                ("B", "3".into())
            ]),
            r#"{"B":3,"aa":1,"b":2}"#
        );
    }

    /// The tag of [`ShapeKind`], read as a generated reader of an enum reads it.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum ShapeKind {
        Dot,
        Circle,
        Disc,
    }

    /// A union over [`ShapeKind`] whose `circle` and `disc` both hold one
    /// record, as a generated sum holds a case's record in its constructor.
    #[derive(Debug, PartialEq)]
    enum Shape {
        Dot,
        Circle(u16),
        Disc(u16),
    }

    fn read_shape(data: &str) -> Result<Shape, PayloadRefusal> {
        let fields = PayloadFields::decode(data).expect("the text is an object");
        shape_at(&fields.field("s")?)
    }

    /// The [`Shape`] `value` holds, read as a generated reader of a union reads
    /// it.
    fn shape_at(value: &PayloadValue<'_>) -> Result<Shape, PayloadRefusal> {
        value.union(
            "kind",
            |tag| {
                tag.name().and_then(|name| match name {
                    "dot" => Ok(ShapeKind::Dot),
                    "circle" => Ok(ShapeKind::Circle),
                    "disc" => Ok(ShapeKind::Disc),
                    other => Err(tag.unknown_variant(other, "ShapeKind")),
                })
            },
            |kind, object| {
                let radius = || object.record(None, |r| r.field("radius")?.unsigned::<u16>());
                match kind {
                    ShapeKind::Dot => Ok(Shape::Dot),
                    ShapeKind::Circle => radius().map(Shape::Circle),
                    ShapeKind::Disc => radius().map(Shape::Disc),
                }
            },
        )
    }

    /// A union's tag is read first, wherever it stands, and names the case
    /// whose record is read from the same object (docs/adr/0092, section 4):
    /// two cases that hold one record stay apart by their tag, a key or a
    /// `null` the case does not declare is stepped over, and each refusal
    /// points at the tag or at the case's field.
    #[test]
    fn a_union_reads_its_tag_first_and_then_the_record_of_its_case() {
        assert_eq!(read_shape(r#"{"s":{"kind":"dot"}}"#), Ok(Shape::Dot));
        assert_eq!(
            read_shape(r#"{"s":{"radius":2,"kind":"disc"}}"#),
            Ok(Shape::Disc(2))
        );
        assert_eq!(
            read_shape(r#"{"s":{"kind":"circle","radius":2,"width":null,"text":"x"}}"#),
            Ok(Shape::Circle(2))
        );
        assert_eq!(
            read_shape(r#"{"s":{"kind":"dot","radius":null}}"#),
            Ok(Shape::Dot)
        );
        let refused = |data: &str| {
            read_shape(data)
                .map(|_| ())
                .map_err(|r| (r.code(), r.pointer().to_string()))
        };
        assert_eq!(
            refused(r#"{"s":[]}"#),
            Err((RefusalCode::WrongKind, "/s".into()))
        );
        assert_eq!(
            refused(r#"{"s":null}"#),
            Err((RefusalCode::NullNotOptional, "/s".into()))
        );
        assert_eq!(
            refused(r#"{"s":{"radius":2}}"#),
            Err((RefusalCode::Missing, "/s/kind".into()))
        );
        assert_eq!(
            refused(r#"{"s":{"kind":null,"radius":null}}"#),
            Err((RefusalCode::NullNotOptional, "/s/kind".into()))
        );
        assert_eq!(
            refused(r#"{"s":{"kind":3,"radius":"x"}}"#),
            Err((RefusalCode::WrongKind, "/s/kind".into()))
        );
        assert_eq!(
            refused(r#"{"s":{"kind":"oval","radius":2}}"#),
            Err((RefusalCode::UnknownVariant, "/s/kind".into()))
        );
        assert_eq!(
            refused(r#"{"s":{"kind":"circle","width":2}}"#),
            Err((RefusalCode::Missing, "/s/radius".into()))
        );
        assert_eq!(
            refused(r#"{"s":{"kind":"disc","radius":null}}"#),
            Err((RefusalCode::NullNotOptional, "/s/radius".into()))
        );
    }

    /// A lift into a union reads the whole text as one value of it, by the
    /// reader a delivery's field of the union is read by, after a lift's own
    /// refusals: its pointers start at the text's root, not under a field.
    #[test]
    fn a_lift_into_a_union_reads_the_whole_text_by_its_tag() {
        let lifted = |text: &str| {
            super::lift_value(text, "Shape", 1, 256, shape_at)
                .map_err(|r| (r.code(), r.pointer().to_string()))
        };
        assert_eq!(lifted(r#"{"radius":3,"kind":"disc"}"#), Ok(Shape::Disc(3)));
        assert_eq!(lifted(r#"{"kind":"dot","radius":null}"#), Ok(Shape::Dot));
        assert_eq!(lifted(" "), Err((RefusalCode::NotJson, String::new())));
        assert_eq!(
            lifted(r#"{"kind":"circle","radius":[1]}"#),
            Err((RefusalCode::TooDeep, "/radius".into()))
        );
        assert_eq!(
            lifted(r#""dot""#),
            Err((RefusalCode::NotObject, String::new()))
        );
        assert_eq!(
            lifted(r#"{"radius":2}"#),
            Err((RefusalCode::Missing, "/kind".into()))
        );
        assert_eq!(
            lifted(r#"{"kind":"circle"}"#),
            Err((RefusalCode::Missing, "/radius".into()))
        );
    }

    /// A union is written as one object, its tag among the fields of its case,
    /// every key ascending by its bytes (docs/adr/0092, section 2.1).
    #[test]
    fn a_union_is_written_with_its_tag_among_the_fields_of_its_case() {
        assert_eq!(union_json("kind", "none", Vec::new()), r#"{"kind":"none"}"#);
        assert_eq!(
            union_json("kind", "relativeWeeks", vec![("count", "2".into())]),
            r#"{"count":2,"kind":"relativeWeeks"}"#
        );
        assert_eq!(
            union_json(
                "kind",
                "absolute",
                vec![
                    ("year", "2027".into()),
                    ("month", "3".into()),
                    ("day", "3".into())
                ]
            ),
            r#"{"day":3,"kind":"absolute","month":3,"year":2027}"#
        );
    }

    /// The cross product (docs/adr/0086, section 14.3): every element kind, at
    /// list depths 1 to 3, each bound 2, with an empty value, a maximal one, one
    /// past the bound at the innermost and at the outermost list, a bad element,
    /// `null` where the element is not optional, an unknown key, and an older
    /// revision. The oracle is the construction: each case is built with the
    /// value or the path it must come to, and the reader is held to that.
    #[test]
    fn every_element_kind_at_every_depth_is_read_or_refused_at_its_path() {
        use super::tests_cross::{run_cross_product, Report};
        let Report { cases, failures } = run_cross_product();
        assert!(
            cases >= 17 * 3 * 8,
            "the cross product shrank to {cases} cases"
        );
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// A maximal value of each kind and depth is written at exactly the size
    /// the calculus gives it (docs/adr/0086, section 2.3), and reads back as
    /// itself.
    #[test]
    fn a_maximal_value_is_written_at_exactly_its_largest_size() {
        use super::tests_cross::run_maximal_sizes;
        let failures = run_maximal_sizes();
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}

#[cfg(test)]
mod tests_cross {
    //! The model the cross-product tests read against: a type, the value a
    //! reader of it must come to, and the size its largest JSON text is.

    use super::{bytes_as_payload_text, PayloadRefusal};
    use super::{list_json, optional_json, quote, record_json, PayloadFields, PayloadValue};

    /// Every bound of the cross product.
    const B: usize = 2;
    /// The bound of a text or byte-string element.
    const T: usize = 3;

    /// A type of the cross product. `Rec` is the record `{a: uint8, b:
    /// string[3]}` whose schema's later revision added `c: uint8` with the
    /// default 9; `Enum` declares `low`, `medium`, `high`.
    #[derive(Clone, Debug)]
    pub(super) enum Ty {
        U8,
        U16,
        U32,
        U64,
        I8,
        I16,
        I32,
        I64,
        F32,
        F64,
        Bool,
        Enum,
        Text,
        Bytes,
        Opt,
        Rec,
        RecList,
        List(Box<Ty>),
    }

    /// What a value of a type holds, as the test model writes it.
    #[derive(Clone, Debug, PartialEq)]
    pub(super) enum Held {
        U(u64),
        I(i64),
        F(f64),
        B(bool),
        S(String),
        Bytes(Vec<u8>),
        Absent,
        Rec(Vec<(String, Held)>),
        List(Vec<Held>),
    }

    const ENUM_NAMES: [&str; 3] = ["low", "medium", "high"];

    pub(super) fn kinds() -> Vec<Ty> {
        vec![
            Ty::U8,
            Ty::U16,
            Ty::U32,
            Ty::U64,
            Ty::I8,
            Ty::I16,
            Ty::I32,
            Ty::I64,
            Ty::F32,
            Ty::F64,
            Ty::Bool,
            Ty::Enum,
            Ty::Text,
            Ty::Bytes,
            Ty::Opt,
            Ty::Rec,
            Ty::RecList,
        ]
    }

    fn nest(kind: &Ty, depth: usize) -> Ty {
        (0..depth).fold(kind.clone(), |inner, _| Ty::List(Box::new(inner)))
    }

    /// The reader of `ty`, one call per constructor, as the generator writes it.
    fn read(ty: &Ty, v: &PayloadValue<'_>) -> Result<Held, PayloadRefusal> {
        Ok(match ty {
            Ty::U8 => Held::U(v.unsigned::<u8>()?.into()),
            Ty::U16 => Held::U(v.unsigned::<u16>()?.into()),
            Ty::U32 => Held::U(v.unsigned::<u32>()?.into()),
            Ty::U64 => Held::U(v.unsigned64()?),
            Ty::I8 => Held::I(v.signed::<i8>()?.into()),
            Ty::I16 => Held::I(v.signed::<i16>()?.into()),
            Ty::I32 => Held::I(v.signed::<i32>()?.into()),
            Ty::I64 => Held::I(v.signed64()?),
            Ty::F32 => Held::F(v.float32()?.into()),
            Ty::F64 => Held::F(v.float64()?),
            Ty::Bool => Held::B(v.truth()?),
            Ty::Enum => {
                let name = v.name()?;
                if !ENUM_NAMES.contains(&name) {
                    return Err(v.unknown_variant(name, "Level"));
                }
                Held::S(name.to_string())
            }
            Ty::Text => Held::S(v.text::<T>()?.as_str().to_string()),
            Ty::Bytes => Held::Bytes(v.bytes::<T>()?.to_vec()),
            Ty::Opt => v
                .optional(|p| p.unsigned::<u8>())?
                .map_or(Held::Absent, |n| Held::U(n.into())),
            Ty::Rec => read_rec(v)?,
            Ty::RecList => v.record(None, |rec| {
                let tags = rec.field("tags")?.list::<Held, B>(|e| read(&Ty::Text, e))?;
                Ok(Held::Rec(vec![(
                    "tags".into(),
                    Held::List(tags.into_vec()),
                )]))
            })?,
            Ty::List(inner) => Held::List(v.list::<Held, B>(|e| read(inner, e))?.into_vec()),
        })
    }

    fn read_rec(v: &PayloadValue<'_>) -> Result<Held, PayloadRefusal> {
        v.record(Some(r#"{"c":9}"#), |rec| {
            let a = rec.field("a")?.unsigned::<u8>()?;
            let b = rec.field("b")?.text::<T>()?;
            let c = rec.field("c")?.unsigned::<u8>()?;
            Ok(Held::Rec(vec![
                ("a".into(), Held::U(a.into())),
                ("b".into(), Held::S(b.as_str().to_string())),
                ("c".into(), Held::U(c.into())),
            ]))
        })
    }

    /// The text of `n` control characters, each of which JSON spells at its
    /// longest (`\u0001`): a text at its bound and at its longest spelling.
    fn controls(n: usize) -> String {
        "\u{1}".repeat(n)
    }

    /// The maximal value of `ty`: every list full, every text at its bound of
    /// control characters, every number at its longest spelling.
    pub(super) fn maximal(ty: &Ty) -> Held {
        match ty {
            Ty::U8 => Held::U(u8::MAX.into()),
            Ty::U16 => Held::U(u16::MAX.into()),
            Ty::U32 => Held::U(u32::MAX.into()),
            Ty::U64 => Held::U(u64::MAX),
            Ty::I8 => Held::I(i8::MIN.into()),
            Ty::I16 => Held::I(i16::MIN.into()),
            Ty::I32 => Held::I(i32::MIN.into()),
            Ty::I64 => Held::I(i64::MIN),
            // Seventeen digits after `0.00000`: the longest spelling of a
            // number, which only the decimal band from 1e-6 to 1e-5 reaches.
            Ty::F32 => Held::F(f64::from(-f32::from_bits(0x35a5_b36d))),
            Ty::F64 => Held::F(-f64::from_bits(0x3eb4_b66d_c01e_c6fb)),
            Ty::Bool => Held::B(false),
            Ty::Enum => Held::S("medium".into()),
            Ty::Text => Held::S(controls(T)),
            Ty::Bytes => Held::Bytes(vec![1; T]),
            // `null` (4 bytes) is longer than any `uint8`'s digits.
            Ty::Opt => Held::Absent,
            Ty::Rec => Held::Rec(vec![
                ("a".into(), Held::U(u8::MAX.into())),
                ("b".into(), Held::S(controls(T))),
                ("c".into(), Held::U(u8::MAX.into())),
            ]),
            Ty::RecList => Held::Rec(vec![(
                "tags".into(),
                Held::List(vec![Held::S(controls(T)); B]),
            )]),
            Ty::List(inner) => Held::List(vec![maximal(inner); B]),
        }
    }

    /// `held` as JSON, written by the runtime's writers, one call per
    /// constructor; a record's members are handed in reverse to show that the
    /// writer orders them.
    pub(super) fn write(ty: &Ty, held: &Held) -> String {
        match (ty, held) {
            (Ty::U64, Held::U(n)) => format!("\"{n}\""),
            (Ty::I64, Held::I(n)) => format!("\"{n}\""),
            (_, Held::U(n)) => n.to_string(),
            (_, Held::I(n)) => n.to_string(),
            (_, Held::F(x)) => crate::json::number_text(*x),
            (_, Held::B(b)) => b.to_string(),
            (_, Held::S(s)) => quote(s),
            (_, Held::Bytes(b)) => quote(&bytes_as_payload_text(b)),
            (Ty::Opt, Held::Absent) => optional_json(None::<u8>, |n| n.to_string()),
            (Ty::Rec, Held::Rec(members)) | (Ty::RecList, Held::Rec(members)) => {
                let tags = Ty::List(Box::new(Ty::Text));
                record_json(
                    members
                        .iter()
                        .rev()
                        .map(|(id, v)| {
                            let field = match id.as_str() {
                                "tags" => tags.clone(),
                                "b" => Ty::Text,
                                _ => Ty::U8,
                            };
                            (id.as_str(), write(&field, v))
                        })
                        .collect(),
                )
            }
            (Ty::List(inner), Held::List(items)) => list_json(items, |e| write(inner, e)),
            (ty, held) => panic!("{held:?} is no value of {ty:?}"),
        }
    }

    /// The largest JSON text of `ty` (docs/adr/0086, section 2.1).
    pub(super) fn largest(ty: &Ty) -> usize {
        let quoted = |id: &str| id.len() + 2;
        let record = |fields: &[(&str, usize)]| {
            2 + fields
                .iter()
                .map(|(id, size)| quoted(id) + 1 + size)
                .sum::<usize>()
                + (fields.len() - 1)
        };
        let list = |elem: usize| 2 + B * elem + (B - 1);
        match ty {
            Ty::U8 => 3,
            Ty::U16 => 5,
            Ty::U32 => 10,
            Ty::U64 | Ty::I64 => 22,
            Ty::I8 => 4,
            Ty::I16 => 6,
            Ty::I32 => 11,
            Ty::F32 | Ty::F64 => 25,
            Ty::Bool => 5,
            Ty::Enum => 2 + "medium".len(),
            Ty::Text | Ty::Bytes => 2 + 6 * T,
            Ty::Opt => largest(&Ty::U8).max(4),
            Ty::Rec => record(&[("a", 3), ("b", 2 + 6 * T), ("c", 3)]),
            Ty::RecList => record(&[("tags", list(2 + 6 * T))]),
            Ty::List(inner) => list(largest(inner)),
        }
    }

    fn data(list: &str) -> String {
        format!(r#"{{"v":{list},"zz":{{"unknown":[1,null]}}}}"#)
    }

    /// The path of the innermost list along the last index at each depth.
    fn innermost(depth: usize) -> String {
        let mut path = String::from("v");
        for _ in 1..depth {
            path.push_str(&format!("/{}", B - 1));
        }
        path
    }

    /// The JSON of `held` with the element at the end of the last index at
    /// every depth replaced by `leaf`.
    fn with_last_leaf(ty: &Ty, held: &Held, leaf: &str) -> String {
        match (ty, held) {
            (Ty::List(inner), Held::List(items)) => {
                let mut parts: Vec<String> = items.iter().map(|e| write(inner, e)).collect();
                let last = parts.len() - 1;
                parts[last] = with_last_leaf(inner, &items[last], leaf);
                format!("[{}]", parts.join(","))
            }
            _ => leaf.to_string(),
        }
    }

    /// The JSON of the maximal value with one more element in the list at
    /// `level` (1 the outermost) along the last index.
    fn one_past(ty: &Ty, held: &Held, level: usize) -> String {
        match (ty, held) {
            (Ty::List(inner), Held::List(items)) => {
                let mut parts: Vec<String> = items.iter().map(|e| write(inner, e)).collect();
                if level == 1 {
                    parts.push(write(inner, &items[0]));
                } else {
                    let last = parts.len() - 1;
                    parts[last] = one_past(inner, &items[last], level - 1);
                }
                format!("[{}]", parts.join(","))
            }
            _ => unreachable!("a level is a list"),
        }
    }

    /// A bad element of `kind`, the path below the element it is refused at,
    /// and the sentence it is refused with.
    fn bad(kind: &Ty) -> (&'static str, &'static str, &'static str) {
        match kind {
            Ty::U8 | Ty::U16 | Ty::U32 | Ty::Opt => ("256000000000", "", "does not fit the width"),
            Ty::U64 => ("\"-1\"", "", "is not a whole number written as digits"),
            Ty::I8 | Ty::I16 | Ty::I32 => ("1.5", "", "is not a whole number"),
            Ty::I64 => ("9007199254740992", "", "past 2^53 - 1"),
            Ty::F32 => ("1e39", "", "does not fit the width"),
            Ty::F64 => ("true", "", "is not a number"),
            Ty::Bool => ("0", "", "is not a truth value"),
            Ty::Enum => ("\"lowest\"", "", "is not a variant of Level"),
            Ty::Text => ("\"abcd\"", "", "is 4 bytes, past the 3 its type declares"),
            Ty::Bytes => ("\"\u{100}\"", "", "carries a character above U+00FF"),
            Ty::Rec => (r#"{"a":"x","b":""}"#, "/a", "is not a number"),
            Ty::RecList => (
                r#"{"tags":["","","x"]}"#,
                "/tags",
                "holds 3 elements, past the 2",
            ),
            Ty::List(_) => unreachable!("a kind is not a list"),
        }
    }

    pub(super) struct Report {
        pub(super) cases: usize,
        pub(super) failures: Vec<String>,
    }

    fn lift(ty: &Ty, text: &str) -> Result<Held, PayloadRefusal> {
        let fields = PayloadFields::decode(text)?;
        let v = fields.field("v")?;
        read(ty, &v)
    }

    pub(super) fn run_cross_product() -> Report {
        let mut report = Report {
            cases: 0,
            failures: Vec::new(),
        };
        let mut expect = |label: String,
                          got: Result<Held, PayloadRefusal>,
                          want: Result<Held, (String, String)>| {
            report.cases += 1;
            let ok = match (&got, &want) {
                (Ok(g), Ok(w)) => g == w,
                (Err(g), Err((path, sentence))) => {
                    g.reason().contains(&format!("'{path}'"))
                        && g.reason().contains(sentence.as_str())
                }
                _ => false,
            };
            if !ok {
                report
                    .failures
                    .push(format!("{label}: read {got:?}, the oracle says {want:?}"));
            }
        };
        for kind in kinds() {
            for depth in 1..=3 {
                let ty = nest(&kind, depth);
                let label = |case: &str| format!("{kind:?} at depth {depth}, {case}");
                let max = maximal(&ty);
                expect(
                    label("empty"),
                    lift(&ty, &data("[]")),
                    Ok(Held::List(vec![])),
                );
                expect(
                    label("maximal"),
                    lift(&ty, &data(&write(&ty, &max))),
                    Ok(max.clone()),
                );
                expect(
                    label("one past the innermost bound"),
                    lift(&ty, &data(&one_past(&ty, &max, depth))),
                    Err((
                        innermost(depth),
                        "holds 3 elements, past the 2 its type declares".into(),
                    )),
                );
                expect(
                    label("one past the outermost bound"),
                    lift(&ty, &data(&one_past(&ty, &max, 1))),
                    Err((
                        "v".into(),
                        "holds 3 elements, past the 2 its type declares".into(),
                    )),
                );
                let leaf = format!("{}/{}", innermost(depth), B - 1);
                if matches!(kind, Ty::Opt) {
                    // The maximal `list<optional<uint8>>` is every element
                    // absent; a present one among them is read as present.
                    let mut present = max.clone();
                    set_last_leaf(&mut present, Held::U(7));
                    expect(
                        label("a present element among absent ones"),
                        lift(&ty, &data(&with_last_leaf(&ty, &max, "7"))),
                        Ok(present),
                    );
                } else {
                    expect(
                        label("null where the element is not optional"),
                        lift(&ty, &data(&with_last_leaf(&ty, &max, "null"))),
                        Err((leaf.clone(), "is null, and its type is not optional".into())),
                    );
                }
                let (text, below, sentence) = bad(&kind);
                expect(
                    label("a bad element"),
                    lift(&ty, &data(&with_last_leaf(&ty, &max, text))),
                    Err((format!("{leaf}{below}"), sentence.into())),
                );
                // A key no schema declares, inside a record element and beside
                // the field, is stepped over.
                let unknown = match kind {
                    Ty::Rec => with_last_leaf(
                        &ty,
                        &max,
                        "{\"zz\":[{\"q\":1}],\"c\":255,\"b\":\"\\u0001\\u0001\\u0001\",\"a\":255}",
                    ),
                    _ => write(&ty, &max),
                };
                expect(
                    label("an unknown key"),
                    lift(&ty, &data(&unknown)),
                    Ok(max.clone()),
                );
                // An older revision: a record element without the field its
                // schema's later revision added reads that field's default, and a
                // payload without a list a later revision added reads it empty.
                match kind {
                    Ty::Rec => {
                        let mut older = max.clone();
                        set_last_leaf(
                            &mut older,
                            Held::Rec(vec![
                                ("a".into(), Held::U(1)),
                                ("b".into(), Held::S(String::new())),
                                ("c".into(), Held::U(9)),
                            ]),
                        );
                        expect(
                            label("an element of an older revision"),
                            lift(&ty, &data(&with_last_leaf(&ty, &max, r#"{"a":1,"b":""}"#))),
                            Ok(older),
                        );
                    }
                    _ => {
                        let fields =
                            PayloadFields::decode_with_defaults(r#"{"zz":1}"#, r#"{"v":[]}"#)
                                .and_then(|f| read(&ty, &f.field("v")?));
                        expect(
                            label("a delivery of the revision before the list"),
                            fields,
                            Ok(Held::List(vec![])),
                        );
                    }
                }
            }
        }
        report
    }

    fn set_last_leaf(held: &mut Held, leaf: Held) {
        match held {
            Held::List(items) => {
                let last = items.len() - 1;
                set_last_leaf(&mut items[last], leaf);
            }
            other => *other = leaf,
        }
    }

    pub(super) fn run_maximal_sizes() -> Vec<String> {
        let mut failures = Vec::new();
        for kind in kinds() {
            for depth in 0..=3 {
                let ty = nest(&kind, depth);
                let max = maximal(&ty);
                let text = write(&ty, &max);
                let size = largest(&ty);
                if text.len() != size {
                    failures.push(format!(
                        "{kind:?} at depth {depth}: {} bytes, the calculus says {size}: {text}",
                        text.len()
                    ));
                }
                match lift(&ty, &data(&text)) {
                    Ok(back) if back == max => {}
                    other => failures.push(format!(
                        "{kind:?} at depth {depth}: written and read back as {other:?}"
                    )),
                }
            }
        }
        failures
    }
}
