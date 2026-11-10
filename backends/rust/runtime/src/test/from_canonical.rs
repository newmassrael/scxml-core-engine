// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A value read back from the canonical form of the test log
//! (docs/SCE_TEST_PROFILE.md §12.5; docs/SCE_TTCN3_LANE.md §10.3): the inverse
//! of [`super::canonical`], which a recorded input trace's values are read
//! with (`schemas/sce-test-trace.v1.schema.json`).
//!
//! Each reader takes one form and accepts its one spelling, so a value read
//! here and written back by [`super::canonical`] is the same bytes:
//!
//! - an integer is the text of its decimal digits, with no sign but a minus,
//!   no leading zero and no `-0`, within the range of the type it is read as;
//! - a `bool` is a truth value;
//! - a value of an enum is the name its document declares for the variant, or,
//!   for an enum of an open set, the text of a number no variant declares;
//! - a text is a text of at most its bound in UTF-8 bytes;
//! - a byte string is its octets in upper-case hex, two digits each;
//! - a record is an object of its fields by their declared ids, an absent
//!   field omitted, no field it does not declare and none twice;
//! - a list is an array of its elements within its bounds, an absent element
//!   of a list of `optional<T>` `null`;
//! - a value of a union is an object of one member, named by its case's
//!   variant, whose value is the case's record, or `{}` for a case that holds
//!   none.
//!
//! A value that does not belong to its type is refused, naming where it stands
//! (`what`) and why, never converted. Which form a value takes is decided by
//! the types its document declares and not by its Rust type, since a byte
//! string and a list of `uint8` are one Rust type: the generated reader of a
//! record (`read_<record>` in the unit of a test configuration) calls these by
//! the walk its canonical writer is spelled by
//! (`forge::generator::test_template_walk::canonical_form`).

use std::fmt::Display;

use sce_portable_bytes::{BoundedList, SceString};

use super::trace::TraceRefusal;
use crate::json::Value;

/// The refusal of the value at `what`, for `why`.
fn refused(what: &str, why: impl Display) -> TraceRefusal {
    TraceRefusal::new(format!("{what}: {why}"))
}

/// The kind of a JSON value, as a refusal names it.
fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a truth value",
        Value::Number(_) => "a number",
        Value::Text(_) => "a text",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// The refusal of `value`, at `what`, which is not of the form `form`.
fn not_a(value: &Value, what: &str, form: &str) -> TraceRefusal {
    refused(what, format!("{} is not {form}", kind(value)))
}

/// The integer `digits` spells as the canonical form writes one: an optional
/// minus and decimal digits with no leading zero, never `-0`. None for any
/// other text, a number past `i128` among them.
pub(crate) fn decimal(digits: &str) -> Option<i128> {
    let number: i128 = digits.parse().ok()?;
    (number.to_string() == digits).then_some(number)
}

/// An integer of the type `T`: the text of its decimal digits.
pub fn integer<T: TryFrom<i128>>(value: &Value, what: &str) -> Result<T, TraceRefusal> {
    let Value::Text(digits) = value else {
        return Err(not_a(
            value,
            what,
            "an integer, which is the text of its decimal digits",
        ));
    };
    let number = decimal(digits).ok_or_else(|| {
        refused(
            what,
            format!("'{digits}' is not the text of an integer's decimal digits"),
        )
    })?;
    T::try_from(number)
        .map_err(|_| refused(what, format!("{digits} is past the range of its type")))
}

/// A `bool`.
pub fn boolean(value: &Value, what: &str) -> Result<bool, TraceRefusal> {
    match value {
        Value::Bool(held) => Ok(*held),
        _ => Err(not_a(value, what, "a truth value")),
    }
}

/// A Rust type a text of the profile is held in: the engine's owned text of a
/// record that holds each field as one entry, or the bounded text of a
/// composition.
pub trait FromCanonicalText: Sized {
    /// `text` as this type; none where the type cannot hold it.
    fn from_canonical_text(text: &str) -> Option<Self>;
}

impl FromCanonicalText for String {
    fn from_canonical_text(text: &str) -> Option<Self> {
        Some(text.to_string())
    }
}

impl<const N: usize> FromCanonicalText for SceString<N> {
    fn from_canonical_text(text: &str) -> Option<Self> {
        SceString::from_text(text).ok()
    }
}

/// A text of at most `max` bytes of UTF-8, the bound its type declares.
pub fn text<T: FromCanonicalText>(
    value: &Value,
    max: usize,
    what: &str,
) -> Result<T, TraceRefusal> {
    let Value::Text(text) = value else {
        return Err(not_a(value, what, "a text"));
    };
    if text.len() > max {
        return Err(refused(
            what,
            format!(
                "a text of {} bytes is past the {max} its type holds",
                text.len()
            ),
        ));
    }
    T::from_canonical_text(text).ok_or_else(|| {
        refused(
            what,
            format!("a text of {} bytes is past what its type holds", text.len()),
        )
    })
}

/// A Rust type a byte string of the profile is held in.
pub trait FromCanonicalOctets: Sized {
    /// `octets` as this type; none where the type cannot hold them.
    fn from_canonical_octets(octets: Vec<u8>) -> Option<Self>;
}

impl FromCanonicalOctets for Vec<u8> {
    fn from_canonical_octets(octets: Vec<u8>) -> Option<Self> {
        Some(octets)
    }
}

impl<const N: usize> FromCanonicalOctets for BoundedList<u8, N> {
    fn from_canonical_octets(octets: Vec<u8>) -> Option<Self> {
        BoundedList::try_from_vec(octets).ok()
    }
}

/// The value of one upper-case hex digit.
fn hex_digit(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

/// The octets `text` spells in upper-case hex, two digits each.
fn upper_hex(text: &str) -> Option<Vec<u8>> {
    let pairs = text.as_bytes().chunks_exact(2);
    if !pairs.remainder().is_empty() {
        return None;
    }
    pairs
        .map(|pair| Some((hex_digit(pair[0])? << 4) | hex_digit(pair[1])?))
        .collect()
}

/// A byte string of at most `max` octets: the text of its octets in
/// upper-case hex, two digits each.
pub fn bytes<T: FromCanonicalOctets>(
    value: &Value,
    max: usize,
    what: &str,
) -> Result<T, TraceRefusal> {
    let Value::Text(hex) = value else {
        return Err(not_a(value, what, "a byte string, which is a text of hex"));
    };
    let octets = upper_hex(hex).ok_or_else(|| {
        refused(
            what,
            format!("'{hex}' is not octets in upper-case hex, two digits each"),
        )
    })?;
    if octets.len() > max {
        return Err(refused(
            what,
            format!(
                "a byte string of {} octets is past the {max} its type holds",
                octets.len()
            ),
        ));
    }
    let count = octets.len();
    T::from_canonical_octets(octets).ok_or_else(|| {
        refused(
            what,
            format!("a byte string of {count} octets is past what its type holds"),
        )
    })
}

/// A value of an enum of a closed set: the variant whose declared name the
/// text is, from `declared`, each variant's name and its value.
pub fn enumerated<E: Copy>(
    value: &Value,
    declared: &[(&str, E)],
    what: &str,
) -> Result<E, TraceRefusal> {
    let Value::Text(name) = value else {
        return Err(not_a(value, what, "a value of an enum, which is its name"));
    };
    declared
        .iter()
        .find(|(declared, _)| *declared == name.as_str())
        .map(|(_, variant)| *variant)
        .ok_or_else(|| refused(what, format!("'{name}' names no variant of its enum")))
}

/// A value of an enum of an open set (`sce:strict-variants="false"`): the
/// variant whose declared name the text is, or the value `from_underlying`
/// makes of the number the text spells, where no variant declares that number.
/// A declared number is written by its variant's name, so its digits are
/// refused as the second spelling of one value.
pub fn open_enumerated<E: Copy + PartialEq, U: TryFrom<i128>>(
    value: &Value,
    declared: &[(&str, E)],
    from_underlying: fn(U) -> E,
    what: &str,
) -> Result<E, TraceRefusal> {
    let Value::Text(name) = value else {
        return Err(not_a(value, what, "a value of an enum, which is its name"));
    };
    if let Some((_, variant)) = declared
        .iter()
        .find(|(declared, _)| *declared == name.as_str())
    {
        return Ok(*variant);
    }
    let number = decimal(name).ok_or_else(|| {
        refused(
            what,
            format!("'{name}' names no variant of its enum, and is no number"),
        )
    })?;
    let raw = U::try_from(number)
        .map_err(|_| refused(what, format!("{name} is past the range of its enum")))?;
    let held = from_underlying(raw);
    if declared.iter().any(|(_, variant)| *variant == held) {
        return Err(refused(
            what,
            format!("{name} is the number of a declared variant, which is written by its name"),
        ));
    }
    Ok(held)
}

/// A record whose fields are `fields`: an object, each member of which names
/// one of them once. Returns the object, whose fields [`field`] and
/// [`optional_field`] read.
pub fn record<'v>(
    value: &'v Value,
    fields: &[&str],
    what: &str,
) -> Result<&'v Value, TraceRefusal> {
    let Value::Object(members) = value else {
        return Err(not_a(
            value,
            what,
            "a record, which is an object of its fields",
        ));
    };
    for (index, (name, _)) in members.iter().enumerate() {
        if !fields.contains(&name.as_str()) {
            return Err(refused(what, format!("'{name}' is no field of its record")));
        }
        if members[..index].iter().any(|(earlier, _)| earlier == name) {
            return Err(refused(what, format!("the field '{name}' is given twice")));
        }
    }
    Ok(value)
}

/// The field `id` of the record `record`, whose type is not optional, so it is
/// present.
pub fn field<'v>(record: &'v Value, id: &str, what: &str) -> Result<&'v Value, TraceRefusal> {
    record
        .member(id)
        .ok_or_else(|| refused(what, "the field is absent, and its type is not optional"))
}

/// The field `id` of the record `record`, whose type is optional: none where it
/// is absent, which the canonical form writes by omitting it.
pub fn optional_field<'v>(record: &'v Value, id: &str) -> Option<&'v Value> {
    record.member(id)
}

/// The elements of a list of at least `min` and at most `max` elements.
pub fn elements<'v>(
    value: &'v Value,
    min: usize,
    max: usize,
    what: &str,
) -> Result<&'v [Value], TraceRefusal> {
    let Value::Array(items) = value else {
        return Err(not_a(value, what, "a list, which is an array"));
    };
    if items.len() < min || items.len() > max {
        return Err(refused(
            what,
            format!(
                "a list of {} elements is outside its bounds, {min} to {max}",
                items.len()
            ),
        ));
    }
    Ok(items.as_slice())
}

/// The elements `items`, read, as the bounded list of the type they are read
/// into, whose bound [`elements`] has held them to.
pub fn list<T, const N: usize>(
    items: Vec<T>,
    what: &str,
) -> Result<BoundedList<T, N>, TraceRefusal> {
    let count = items.len();
    BoundedList::try_from_vec(items).map_err(|_| {
        refused(
            what,
            format!("a list of {count} elements is past the {N} its type holds"),
        )
    })
}

/// An element of a list of `optional<T>`: none for `null`, which the canonical
/// form writes in the place of an absent element, and the element otherwise.
pub fn nullable(value: &Value) -> Option<&Value> {
    match value {
        Value::Null => None,
        held => Some(held),
    }
}

/// A value of a union whose cases are `variants`: an object of one member,
/// named by one of them, the variant of its case and the case's value. A
/// variant no case declares is refused here, so a reader that matches on the
/// variant read has an arm for each declared one and needs none for another.
pub fn union_case<'v>(
    value: &'v Value,
    variants: &[&str],
    what: &str,
) -> Result<(&'v str, &'v Value), TraceRefusal> {
    let Value::Object(members) = value else {
        return Err(not_a(
            value,
            what,
            "a value of a union, which is an object of one member",
        ));
    };
    let [(variant, case)] = members.as_slice() else {
        return Err(refused(
            what,
            format!(
                "a value of a union is an object of one member, and this one has {}",
                members.len()
            ),
        ));
    };
    if !variants.contains(&variant.as_str()) {
        return Err(refused(
            what,
            format!("'{variant}' names no case of its union"),
        ));
    }
    Ok((variant.as_str(), case))
}

/// The value of a case of a union that holds no record: the empty object.
pub fn no_record(value: &Value, what: &str) -> Result<(), TraceRefusal> {
    match value {
        Value::Object(members) if members.is_empty() => Ok(()),
        _ => Err(refused(
            what,
            "a case that holds no record is the empty object",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{parse, write_canonical};
    use crate::test::canonical;

    fn text_of(text: &str) -> Value {
        Value::Text(text.to_string())
    }

    /// The reason of the refusal `result` holds.
    fn reason<T>(result: Result<T, TraceRefusal>) -> String {
        match result {
            Ok(_) => panic!("the value is read where it should be refused"),
            Err(refusal) => refusal.reason().to_string(),
        }
    }

    /// An integer is read from the one spelling the writer gives it, at every
    /// width, and any other spelling, or a value past its type, is refused,
    /// naming where it stands.
    #[test]
    fn an_integer_is_read_from_the_text_of_its_digits_alone() {
        assert_eq!(integer::<u8>(&canonical::integer(0_u8), "n"), Ok(0));
        assert_eq!(
            integer::<u64>(&canonical::integer(u64::MAX), "n"),
            Ok(u64::MAX)
        );
        assert_eq!(
            integer::<i64>(&canonical::integer(i64::MIN), "n"),
            Ok(i64::MIN)
        );
        for spelling in ["007", "+7", "-0", "7.0", "", " 7"] {
            let refusal = reason(integer::<u32>(&text_of(spelling), "job_reply.code"));
            assert!(refusal.starts_with("job_reply.code: "), "{refusal}");
            assert!(refusal.contains("decimal digits"), "{spelling}: {refusal}");
        }
        assert!(reason(integer::<u8>(&text_of("256"), "n")).contains("past the range"));
        assert!(reason(integer::<u32>(&text_of("-1"), "n")).contains("past the range"));
        assert!(reason(integer::<u32>(&Value::Number("7".to_string()), "n"))
            .contains("a number is not an integer"));
    }

    #[test]
    fn a_bool_is_a_truth_value() {
        assert_eq!(boolean(&canonical::boolean(true), "b"), Ok(true));
        assert!(reason(boolean(&text_of("true"), "b")).contains("a text is not a truth value"));
    }

    /// A text is read within its bound in bytes, into the engine's text and
    /// into a bounded one.
    #[test]
    fn a_text_is_read_within_its_bound() {
        assert_eq!(
            text::<String>(&canonical::text("abc"), 3, "t"),
            Ok("abc".to_string())
        );
        let bounded: SceString<3> = text(&canonical::text("abc"), 3, "t").expect("it fits");
        assert_eq!(bounded.as_str(), "abc");
        assert!(reason(text::<String>(&canonical::text("abcd"), 3, "t")).contains("past the 3"));
        assert!(
            reason(text::<SceString<2>>(&canonical::text("abc"), 3, "t"))
                .contains("past what its type holds")
        );
    }

    /// A byte string is read from upper-case hex alone, the writer's spelling,
    /// and back to the same text.
    #[test]
    fn a_byte_string_is_read_from_upper_case_hex() {
        let written = canonical::bytes(&[0x00, 0xab, 0x0f, 0xff]);
        let read: Vec<u8> = bytes(&written, 4, "b").expect("the writer's spelling");
        assert_eq!(read, [0x00, 0xab, 0x0f, 0xff]);
        let bounded: BoundedList<u8, 4> = bytes(&written, 4, "b").expect("it fits");
        assert_eq!(canonical::bytes(bounded.as_slice()), written);
        for spelling in ["00ab", "0", "0G"] {
            assert!(
                reason(bytes::<Vec<u8>>(&text_of(spelling), 4, "b")).contains("upper-case hex"),
                "{spelling}"
            );
        }
        assert!(reason(bytes::<Vec<u8>>(&written, 3, "b")).contains("past the 3"));
    }

    /// A closed enum is read by its declared names; an open one by its names
    /// and by the numbers no variant declares, a declared number being the
    /// second spelling of its variant.
    #[test]
    fn an_enum_is_read_by_its_declared_name() {
        let declared = [("red", 0_u8), ("green", 1_u8)];
        assert_eq!(enumerated(&text_of("green"), &declared, "c"), Ok(1));
        assert!(reason(enumerated(&text_of("blue"), &declared, "c")).contains("'blue' names no"));
        let open = |raw: u8| raw;
        assert_eq!(
            open_enumerated(&text_of("red"), &declared, open, "c"),
            Ok(0)
        );
        assert_eq!(open_enumerated(&text_of("7"), &declared, open, "c"), Ok(7));
        assert!(reason(open_enumerated(&text_of("1"), &declared, open, "c"))
            .contains("written by its name"));
        assert!(
            reason(open_enumerated(&text_of("300"), &declared, open, "c"))
                .contains("past the range")
        );
    }

    /// A record holds only the fields it declares, each once; a field whose
    /// type is not optional is present.
    #[test]
    fn a_record_holds_its_declared_fields_once() {
        let value = parse("{\"code\":\"0\",\"job\":\"7\"}").expect("JSON");
        let object = record(&value, &["job", "code"], "job_reply").expect("a record");
        assert_eq!(
            integer::<u32>(field(object, "job", "job_reply.job").expect("held"), "job"),
            Ok(7)
        );
        assert!(optional_field(object, "note").is_none());
        let stray = parse("{\"job\":\"7\",\"jobs\":\"1\"}").expect("JSON");
        assert!(
            reason(record(&stray, &["job", "code"], "job_reply")).contains("'jobs' is no field")
        );
        // The reader of a text refuses a member named twice; a value built
        // in place can still hold one.
        let twice = Value::Object(vec![
            ("job".to_string(), text_of("7")),
            ("job".to_string(), text_of("8")),
        ]);
        assert!(reason(record(&twice, &["job"], "job_reply")).contains("given twice"));
        let short = parse("{\"job\":\"7\"}").expect("JSON");
        assert_eq!(
            reason(field(&short, "code", "job_reply.code")),
            "job_reply.code: the field is absent, and its type is not optional"
        );
    }

    /// A list is read within its bounds, an absent element as null, into the
    /// bounded list of its type.
    #[test]
    fn a_list_is_read_within_its_bounds() {
        let value = parse("[\"1\",null]").expect("JSON");
        let items = elements(&value, 1, 2, "l").expect("within its bounds");
        let read: Vec<Option<u8>> = items
            .iter()
            .map(|item| nullable(item).map(|held| integer::<u8>(held, "l[]").expect("an integer")))
            .collect();
        assert_eq!(read, [Some(1), None]);
        let bounded: BoundedList<Option<u8>, 2> = list(read, "l").expect("it fits");
        assert_eq!(bounded.as_slice(), [Some(1), None]);
        assert!(reason(elements(&value, 3, 4, "l")).contains("outside its bounds, 3 to 4"));
        assert!(reason(list::<u8, 1>(vec![1, 2], "l")).contains("past the 1"));
    }

    /// A union is one member named by its case; a case that holds no record is
    /// the empty object.
    #[test]
    fn a_union_is_one_member_named_by_its_case() {
        let card = canonical::union_case(
            "card",
            Some(canonical::record([(
                "amount",
                Some(canonical::integer(1_u32)),
            )])),
        );
        let cases = ["card", "cash"];
        let (variant, value) = union_case(&card, &cases, "p").expect("one member");
        assert_eq!(variant, "card");
        assert_eq!(
            write_canonical(value).expect("canonical"),
            "{\"amount\":\"1\"}"
        );
        let cash = canonical::union_case("cash", None);
        let (_, value) = union_case(&cash, &cases, "p").expect("one member");
        assert_eq!(no_record(value, "p"), Ok(()));
        assert!(reason(no_record(&card, "p")).contains("is the empty object"));
        let two = parse("{\"card\":{},\"cash\":{}}").expect("JSON");
        assert!(reason(union_case(&two, &cases, "p")).contains("this one has 2"));
        let cheque = canonical::union_case("cheque", None);
        assert_eq!(
            reason(union_case(&cheque, &cases, "p")),
            "p: 'cheque' names no case of its union"
        );
    }
}
