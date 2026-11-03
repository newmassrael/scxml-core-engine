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
//! [`PayloadValue::record`](crate::event_payload::PayloadValue::record). The
//! links name the whole path because these lines are merged with the module's
//! outer docs in `lib.rs`, and rustdoc resolves the merged text from the crate
//! root. A value outside its type is refused before
//! anything of it is kept, and the refusal names the path through every depth
//! as a JSON Pointer (RFC 6901): `'items/2/tags/0' is 9 bytes, past the 8 its
//! type declares`. A key a record's schema does not declare is stepped over.
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

use sce_portable_bytes::{BoundedList, BoundedText};

/// Why an event's data cannot be read as its schema's fields.
///
/// A sentence, not a code: it rides in the `error.execution` event's data,
/// where the document's own handler can read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadRefusal(String);

impl PayloadRefusal {
    /// Refuse a payload, saying why in the words the document's handler reads.
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }

    /// Why the payload was refused.
    pub fn reason(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PayloadRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
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
        let trimmed = data.trim();
        if trimmed.is_empty() {
            return Err(PayloadRefusal::new("the event carries no data"));
        }
        if !trimmed.starts_with('{') {
            return Err(PayloadRefusal::new(
                "the event's data is a bare value, and this event's schema declares named fields",
            ));
        }
        let value = crate::json::parse(trimmed).map_err(|e| match e {
            JsonError::Trailing => {
                PayloadRefusal::new("the event's data carries more than one JSON value")
            }
            JsonError::Malformed(what) => {
                PayloadRefusal::new(format!("the event's data is not JSON ({what})"))
            }
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
        let mut decoded = if data.trim().is_empty() {
            Self::default()
        } else {
            Self::decode(data)?
        };
        for (name, value) in defaults_of(defaults)? {
            if !decoded.fields.iter().any(|(key, _)| *key == name) {
                decoded.fields.push((name, value));
            }
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
        }
    }

    /// The value of the field `name`, at the path its name is; a delivery that
    /// lacks it is refused by name.
    pub fn field(&self, name: &str) -> Result<PayloadValue<'_>, PayloadRefusal> {
        self.record().field(name)
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

    /// A text field, as the text it carries; [`PayloadValue::text`] holds a
    /// text to the bound its type declares.
    pub fn text(&self, name: &str) -> Result<crate::SceString, PayloadRefusal> {
        self.field(name)?.name().map(crate::sce_string_from_str)
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
}

impl<'a> PayloadRecord<'a> {
    /// The value of the field `name`: the delivered one, else the default a
    /// later revision of the record's schema gave it, else a refusal naming the
    /// path. A member the schema does not declare is never asked for, so it is
    /// stepped over.
    pub fn field(&self, name: &str) -> Result<PayloadValue<'a>, PayloadRefusal> {
        let path = join(self.prefix, name);
        let found = self
            .members
            .iter()
            .chain(self.defaults.unwrap_or_default())
            .find(|(key, _)| key == name)
            .map(|(_, value)| value);
        match found {
            Some(value) => Ok(PayloadValue { value, path }),
            None => Err(PayloadRefusal::new(format!(
                "the event's data has no '{path}'"
            ))),
        }
    }
}

/// One value of an event's data, at the path through every depth that names
/// it: a field's name, then each index and field name below it, joined by `/`.
#[derive(Clone, Debug)]
pub struct PayloadValue<'a> {
    value: &'a Value,
    path: String,
}

impl<'a> PayloadValue<'a> {
    /// The path that names this value.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Whether the value is `null`, the one spelling of an absent value.
    pub fn is_null(&self) -> bool {
        matches!(self.value, Value::Null)
    }

    /// A refusal of this value, saying `what` of it after its path:
    /// `'items/1/level' (x) is not a variant of Level`.
    pub fn refuse(&self, what: &str) -> PayloadRefusal {
        PayloadRefusal::new(format!("'{}' {what}", self.path))
    }

    /// The refusal of `null` where the type is not optional, or the refusal
    /// `otherwise` of any other value of the wrong kind.
    fn wrong_kind(&self, otherwise: &str) -> PayloadRefusal {
        if self.is_null() {
            self.refuse("is null, and its type is not optional")
        } else {
            self.refuse(otherwise)
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
        let whole: i64 = text
            .parse()
            .map_err(|_| self.refuse(&format!("is not a whole number ({text})")))?;
        T::try_from(whole).map_err(|_| {
            self.refuse(&format!(
                "does not fit the width its schema declares ({text})"
            ))
        })
    }

    /// An unsigned whole number at the width its type declares.
    pub fn unsigned<T>(&self) -> Result<T, PayloadRefusal>
    where
        T: TryFrom<u64>,
    {
        let text = self.number()?;
        let whole: u64 = text.parse().map_err(|_| {
            self.refuse(&format!("is not a whole number at or above zero ({text})"))
        })?;
        T::try_from(whole).map_err(|_| {
            self.refuse(&format!(
                "does not fit the width its schema declares ({text})"
            ))
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
            Value::Text(text) => {
                Err(self.refuse(&format!("is not a whole number written as digits ({text})")))
            }
            Value::Number(text) => {
                let exact = text
                    .parse::<i128>()
                    .map(|n| n.unsigned_abs() <= EXACT_IN_A_DOUBLE)
                    .map_err(|_| self.refuse(&format!("is not a whole number ({text})")))?;
                if exact {
                    Ok(text)
                } else {
                    Err(self.refuse(&format!(
                        "is a number past 2^53 - 1, which is not exact everywhere ({text}); \
                         a 64-bit integer that large is written as a text of its digits"
                    )))
                }
            }
            _ => Err(self.wrong_kind("is not a number")),
        }
    }

    /// A 64-bit signed value: the canonical text of its digits, or a JSON
    /// number no larger than 2^53 - 1 in magnitude.
    pub fn signed64(&self) -> Result<i64, PayloadRefusal> {
        let text = self.wide(true)?;
        text.parse().map_err(|_| {
            self.refuse(&format!(
                "does not fit the width its schema declares ({text})"
            ))
        })
    }

    /// A 64-bit unsigned value: the canonical text of its digits, or a JSON
    /// number no larger than 2^53 - 1.
    pub fn unsigned64(&self) -> Result<u64, PayloadRefusal> {
        let text = self.wide(false)?;
        text.parse().map_err(|_| {
            self.refuse(&format!(
                "does not fit the width its schema declares ({text})"
            ))
        })
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
        let wide: f64 = text
            .parse()
            .map_err(|_| self.refuse(&format!("is not a number ({text})")))?;
        if wide.abs() > f64::from(f32::MAX) {
            return Err(self.refuse(&format!(
                "does not fit the width its schema declares ({text})"
            )));
        }
        Ok(wide as f32)
    }

    /// A 64-bit fractional value.
    pub fn float64(&self) -> Result<f64, PayloadRefusal> {
        let text = self.number()?;
        text.parse()
            .map_err(|_| self.refuse(&format!("is not a number ({text})")))
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
    pub fn name(&self) -> Result<&'a str, PayloadRefusal> {
        match self.value {
            Value::Text(text) => Ok(text),
            _ => Err(self.wrong_kind("is not a text")),
        }
    }

    /// A text of at most `N` UTF-8 bytes (docs/adr/0086, section 4.4): one past
    /// the bound is refused, at whatever depth it stands, and never cut.
    pub fn text<const N: usize>(&self) -> Result<BoundedText<N>, PayloadRefusal> {
        let text = self.name()?;
        BoundedText::<N>::from_text(text).map_err(|_| self.past_bytes(text.len(), N))
    }

    /// A byte string of at most `N` bytes.
    ///
    /// JSON has no byte string, so the wire carries the byte-exact Latin-1 text
    /// the inject seam writes, and this reads it back the same way: every one
    /// of the 256 values is one character and back. Printable ASCII — what a
    /// bytes guard compares — is the same bytes under either reading. The bound
    /// is held here on every profile: [`crate::SceBytes`] holds whatever it is
    /// given where an allocator is.
    pub fn bytes<const N: usize>(&self) -> Result<crate::SceBytes<N>, PayloadRefusal> {
        let text = match self.value {
            Value::Text(s) => s,
            _ => return Err(self.wrong_kind("is not a byte string")),
        };
        let mut out: Vec<u8> = Vec::with_capacity(text.len());
        for c in text.chars() {
            if (c as u32) > 0xFF {
                return Err(
                    self.refuse("carries a character above U+00FF, which no single byte spells")
                );
            }
            out.push(c as u8);
        }
        if out.len() > N {
            return Err(self.past_bytes(out.len(), N));
        }
        crate::SceBytes::<N>::from_slice(&out).map_err(|_| self.past_bytes(out.len(), N))
    }

    fn past_bytes(&self, held: usize, bound: usize) -> PayloadRefusal {
        self.refuse(&format!(
            "is {held} bytes, past the {bound} its type declares"
        ))
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
        mut read: impl FnMut(&PayloadValue<'a>) -> Result<T, PayloadRefusal>,
    ) -> Result<BoundedList<T, N>, PayloadRefusal> {
        let Value::Array(items) = self.value else {
            return Err(self.wrong_kind("is not a list"));
        };
        if items.len() > N {
            return Err(self.refuse(&format!(
                "holds {} elements, past the {N} its type declares",
                items.len()
            )));
        }
        let mut list = BoundedList::<T, N>::new();
        for (index, value) in items.iter().enumerate() {
            let element = PayloadValue {
                value,
                path: join(&self.path, &index.to_string()),
            };
            // The length was held above, so the push cannot refuse; the arm is
            // the same sentence, for a bound the two could disagree about.
            list.push(read(&element)?).map_err(|_| {
                self.refuse(&format!(
                    "holds {} elements, past the {N} its type declares",
                    items.len()
                ))
            })?;
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
        };
        read(&record)
    }
}

/// The members of a schema's defaults, which the generator spells as one JSON
/// object.
fn defaults_of(defaults: &str) -> Result<Vec<(String, Value)>, PayloadRefusal> {
    match crate::json::parse(defaults.trim()) {
        Ok(Value::Object(members)) => Ok(members),
        _ => Err(PayloadRefusal::new(
            "the schema's defaults are not a JSON object, which the generator never writes",
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::{list_json, optional_json, quote, record_json, PayloadFields};

    const DEFAULTS: &str = r#"{"layout":"week","count":"7","urgent":true}"#;

    #[test]
    fn a_field_the_delivery_lacks_is_read_out_of_the_defaults() {
        let fields = PayloadFields::decode_with_defaults(r#"{"level": 4}"#, DEFAULTS)
            .expect("a delivery of the first revision reads");
        assert_eq!(fields.unsigned::<u8>("level"), Ok(4));
        assert_eq!(
            fields.text("layout").map(|s| s.to_string()),
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
            fields.text("layout").map(|s| s.to_string()),
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
                .optional("title", |f| f.text("title"))
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
        assert_eq!(
            fields.field("note").and_then(|v| v.text::<4>()),
            Err(super::PayloadRefusal::new(
                "'note' is 5 bytes, past the 4 its type declares"
            ))
        );
        assert_eq!(
            fields.field("blob").and_then(|v| v.bytes::<4>()),
            Err(super::PayloadRefusal::new(
                "'blob' is 5 bytes, past the 4 its type declares"
            ))
        );
        assert_eq!(
            fields.bytes::<4>("blob"),
            Err(super::PayloadRefusal::new(
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

    #[test]
    fn a_record_value_names_its_missing_field_and_reads_its_later_fields_from_defaults() {
        let fields =
            PayloadFields::decode(r#"{"r": {"a": 1, "zz": [null]}, "s": 5}"#).expect("reads");
        let r = fields.field("r").expect("present");
        assert_eq!(
            r.record(None, |rec| rec.field("b").map(|_| ())),
            Err(super::PayloadRefusal::new("the event's data has no 'r/b'"))
        );
        assert_eq!(
            r.record(Some(r#"{"b": 9}"#), |rec| rec.field("b")?.unsigned::<u8>()),
            Ok(9)
        );
        assert_eq!(
            fields.field("s").and_then(|s| s.record(None, |_| Ok(()))),
            Err(super::PayloadRefusal::new("'s' is not a record"))
        );
        assert_eq!(
            r.record(Some("[]"), |_| Ok(())),
            Err(super::PayloadRefusal::new(
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
                    return Err(v.refuse(&format!("({name}) is not a variant of Level")));
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
