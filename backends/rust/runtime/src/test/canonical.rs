// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The canonical form of a message's value in the test log
//! (docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091, C3 and answer 8): one JSON
//! value per value of the profile's types, built from strings, truth values,
//! arrays and objects alone, so that the JSON Canonicalization Scheme
//! ([`crate::json::write_canonical`]) writes two equal values as the same
//! bytes on every runtime.
//!
//! - an integer of any width is the text of its decimal digits, since an
//!   `int64` or a `uint64` is past what a JSON reader that holds numbers as
//!   doubles reads exactly;
//! - a `bool` is a truth value;
//! - a value of an enum is the name its document declares for its variant;
//! - a text is a text;
//! - a byte string is the text of its octets in upper-case hex, two digits each;
//! - a record is an object of its fields by their declared ids, a field that is
//!   absent omitted, as TTCN-3's `omit` leaves it out;
//! - a list is an array of its elements in order;
//! - a value of a union is an object of one member, named by its case's
//!   variant, whose value is the case's record, or `{}` for a case that holds
//!   none.
//!
//! The generated unit of a test template over a record spells its `canonical`
//! with these, by the types its document declares (`forge/rust/test_template.rs.jinja2`):
//! a Rust type alone does not say which form a value takes, since a byte
//! string and a list of `uint8` are one Rust type.

use crate::json::Value;

/// An integer of any width the profile has: the text of its decimal digits.
pub fn integer(value: impl Into<i128>) -> Value {
    Value::Text(value.into().to_string())
}

/// A `bool`.
pub fn boolean(value: bool) -> Value {
    Value::Bool(value)
}

/// A text.
pub fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

/// A byte string: its octets in upper-case hex, two digits each, as TTCN-3
/// writes an `octetstring` (`'00FF'O`) without its quotes and its `O`.
pub fn bytes(value: &[u8]) -> Value {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(value.len() * 2);
    for &octet in value {
        out.push(char::from(HEX[usize::from(octet >> 4)]));
        out.push(char::from(HEX[usize::from(octet & 0x0F)]));
    }
    Value::Text(out)
}

/// A value of a closed enum: the name its document declares for the variant.
pub fn enumerated(declared: &str) -> Value {
    text(declared)
}

/// A value of an enum of an open set (`sce:strict-variants="false"`): the name
/// its document declares for it, or, for a number no variant declares, the
/// text of that number's decimal digits. No variant's name is such a text,
/// since a name is an identifier. The TTCN-3 export refuses an enum of an open
/// set (docs/SCE_TEST_PROFILE.md §9.5), so only a native log holds this form.
pub fn open_enumerated(declared: Option<&str>, number: impl Into<i128>) -> Value {
    match declared {
        Some(declared) => text(declared),
        None => integer(number),
    }
}

/// A record: each field by its declared id, in any order, since the scheme
/// sorts an object's members; a field given as `None` is absent and omitted.
pub fn record<'f>(fields: impl IntoIterator<Item = (&'f str, Option<Value>)>) -> Value {
    Value::Object(
        fields
            .into_iter()
            .filter_map(|(id, value)| value.map(|value| (id.to_string(), value)))
            .collect(),
    )
}

/// A list: its elements in order.
pub fn list(elements: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(elements.into_iter().collect())
}

/// A value of a union: one member, named by the variant of its case, holding
/// the case's record, or the empty object for a case that holds none.
pub fn union_case(variant: &str, record: Option<Value>) -> Value {
    Value::Object(vec![(
        variant.to_string(),
        record.unwrap_or_else(|| Value::Object(Vec::new())),
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::write_canonical;

    fn written(value: &Value) -> String {
        write_canonical(value).expect("a canonical value is written")
    }

    /// An integer is the text of its digits at every width, the ends of the
    /// 64-bit ones among them, which a double does not hold exactly.
    #[test]
    fn an_integer_is_the_text_of_its_decimal_digits() {
        assert_eq!(written(&integer(0_u8)), "\"0\"");
        assert_eq!(written(&integer(-128_i8)), "\"-128\"");
        assert_eq!(written(&integer(u64::MAX)), "\"18446744073709551615\"");
        assert_eq!(written(&integer(i64::MIN)), "\"-9223372036854775808\"");
    }

    #[test]
    fn a_bool_is_a_truth_value_and_a_text_a_text() {
        assert_eq!(written(&boolean(true)), "true");
        assert_eq!(written(&boolean(false)), "false");
        assert_eq!(written(&text("a\"b")), "\"a\\\"b\"");
    }

    /// Two hex digits an octet, upper case, the leading zero kept.
    #[test]
    fn a_byte_string_is_its_octets_in_upper_case_hex() {
        assert_eq!(written(&bytes(&[0x00, 0xab, 0x0f, 0xff])), "\"00AB0FFF\"");
        assert_eq!(written(&bytes(&[])), "\"\"");
    }

    /// A closed enum by its declared name; an open one by its name, or by its
    /// number where no variant declares it.
    #[test]
    fn an_enum_is_its_declared_name() {
        assert_eq!(written(&enumerated("red")), "\"red\"");
        assert_eq!(written(&open_enumerated(Some("red"), 0_u8)), "\"red\"");
        assert_eq!(written(&open_enumerated(None, 7_u8)), "\"7\"");
    }

    /// An absent field is omitted, and the fields are sorted when written,
    /// whatever order they were given in.
    #[test]
    fn a_record_omits_an_absent_field() {
        let value = record([
            ("quantity", Some(integer(2_u16))),
            ("note", None),
            ("color", Some(enumerated("red"))),
        ]);
        assert_eq!(written(&value), "{\"color\":\"red\",\"quantity\":\"2\"}");
        assert_eq!(written(&record([("note", None)])), "{}");
    }

    #[test]
    fn a_list_is_its_elements_in_order() {
        let value = list([3_u8, 1, 2].into_iter().map(integer));
        assert_eq!(written(&value), "[\"3\",\"1\",\"2\"]");
        assert_eq!(written(&list(Vec::new())), "[]");
    }

    /// A union is one member named by its case's variant: the case's record, or
    /// the empty object for a case that holds none.
    #[test]
    fn a_union_is_one_member_named_by_its_variant() {
        let card = union_case("card", Some(record([("amount", Some(integer(1_u32)))])));
        assert_eq!(written(&card), "{\"card\":{\"amount\":\"1\"}}");
        assert_eq!(written(&union_case("cash", None)), "{\"cash\":{}}");
    }
}
