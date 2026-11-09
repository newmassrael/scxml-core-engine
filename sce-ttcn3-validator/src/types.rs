// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A type as a document writes it, in the type grammar of an `event-schema`'s
//! fields, read into the TTCN-3 type the export writes for it
//! (docs/SCE_TEST_PROFILE.md §6.3, §9.5 **Types**): an integer type by its
//! own name, `bool` as `boolean`, `string[N]` as `universal charstring`,
//! `bytes[N]` as `octetstring`, `list<T>[M..B]` as `record length (M .. B) of
//! T` with `M` 0 where none is written, and an enum, a record and a union by
//! the unit name of the document imported under its alias. `optional<T>` is
//! T where a value may be absent, which a field writes as `optional` and a
//! template's parameter as `template (omit)`. A float has no type in the
//! profile, and a list of values that may be absent has no TTCN-3 form.

use crate::documents::DocumentKind;
use crate::ttcn3::ast::TypeExpr;

/// The integer types of the type grammar, each with its range: the eight
/// the types module of every suite states (§6.3).
pub const INTEGERS: [(&str, &str, &str); 8] = [
    ("uint8", "0", "255"),
    ("uint16", "0", "65535"),
    ("uint32", "0", "4294967295"),
    ("uint64", "0", "18446744073709551615"),
    ("int8", "-128", "127"),
    ("int16", "-32768", "32767"),
    ("int32", "-2147483648", "2147483647"),
    ("int64", "-9223372036854775808", "9223372036854775807"),
];

/// A written type: the TTCN-3 type of its value, and whether it may be
/// absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub ty: TypeExpr,
    pub optional: bool,
}

/// The unit name of the document of `kind` its document imports under an
/// alias, where it imports one.
pub type Named<'a> = &'a dyn Fn(DocumentKind, &str) -> Option<String>;

/// `text`, a written type, with each alias resolved by `named`.
pub fn read(text: &str, named: Named<'_>) -> Result<Written, String> {
    let text = text.trim();
    if let Some(inner) = text
        .strip_prefix("optional<")
        .and_then(|t| t.strip_suffix('>'))
    {
        return Ok(Written {
            ty: value(inner.trim(), named)?,
            optional: true,
        });
    }
    Ok(Written {
        ty: value(text, named)?,
        optional: false,
    })
}

/// The TTCN-3 type of a value of the written type `text`.
fn value(text: &str, named: Named<'_>) -> Result<TypeExpr, String> {
    if let Some(rest) = text.strip_prefix("list<") {
        return list(text, rest, named);
    }
    if INTEGERS.iter().any(|(name, _, _)| *name == text) {
        return Ok(TypeExpr::Named(text.to_string()));
    }
    let bounded = |base: &str| text == base || text.starts_with(&format!("{base}["));
    if text == "bool" {
        return Ok(TypeExpr::Boolean);
    }
    if bounded("string") {
        return Ok(TypeExpr::UniversalCharstring);
    }
    if bounded("bytes") {
        return Ok(TypeExpr::Octetstring);
    }
    if text == "float32" || text == "float64" {
        return Err(format!(
            "the document writes {text}, which has no type in the profile"
        ));
    }
    for (prefix, kind, what) in [
        ("enum:", DocumentKind::Enum, "enum"),
        ("record:", DocumentKind::EventSchema, "event schema"),
        ("union:", DocumentKind::Union, "union"),
    ] {
        if let Some(alias) = text.strip_prefix(prefix) {
            let alias = alias.trim();
            return named(kind, alias).map(TypeExpr::Named).ok_or_else(|| {
                format!("the type {text} names '{alias}', which the document imports as no {what}")
            });
        }
    }
    Err(format!("'{text}' is no type of the profile"))
}

/// The list type `text`, `rest` what follows its `list<`.
fn list(text: &str, rest: &str, named: Named<'_>) -> Result<TypeExpr, String> {
    let malformed = || format!("'{text}' is no list type of the profile");
    let close = closing(rest).ok_or_else(malformed)?;
    let element = rest[..close].trim();
    let bound = rest[close + 1..]
        .trim()
        .strip_prefix('[')
        .and_then(|bound| bound.strip_suffix(']'))
        .ok_or_else(malformed)?;
    let (min, max) = match bound.split_once("..") {
        Some((min, max)) => (min.trim(), max.trim()),
        None => ("0", bound.trim()),
    };
    // A named bound, `name=N`, is its value where no build moves it.
    let max = max.rsplit_once('=').map_or(max, |(_, value)| value.trim());
    let digits = |count: &str| !count.is_empty() && count.bytes().all(|b| b.is_ascii_digit());
    if !digits(min) || !digits(max) {
        return Err(malformed());
    }
    if element.starts_with("optional<") {
        return Err(format!(
            "'{text}' is a list of values that may be absent, which the elements of a TTCN-3 \
             `record of` never are"
        ));
    }
    Ok(TypeExpr::RecordOf {
        min: canonical(min),
        max: canonical(max),
        element: Box::new(value(element, named)?),
    })
}

/// The decimal digits `count` writes, without leading zeros.
fn canonical(count: &str) -> String {
    let trimmed = count.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The position in `rest` of the `>` that closes the `<` before it.
fn closing(rest: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (position, c) in rest.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return Some(position);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(kind: DocumentKind, alias: &str) -> Option<String> {
        match (kind, alias) {
            (DocumentKind::Enum, "Color") => Some("paint_color".to_string()),
            (DocumentKind::EventSchema, "Line") => Some("order_line".to_string()),
            (DocumentKind::Union, "Payment") => Some("order_payment".to_string()),
            _ => None,
        }
    }

    fn ty(text: &str) -> Result<Written, String> {
        read(text, &named)
    }

    fn plain(ty: TypeExpr) -> Result<Written, String> {
        Ok(Written {
            ty,
            optional: false,
        })
    }

    #[test]
    fn each_type_of_the_grammar_is_read_as_the_export_writes_it() {
        assert_eq!(ty("uint16"), plain(TypeExpr::Named("uint16".to_string())));
        assert_eq!(ty("bool"), plain(TypeExpr::Boolean));
        assert_eq!(ty("string[32]"), plain(TypeExpr::UniversalCharstring));
        assert_eq!(ty("bytes[4]"), plain(TypeExpr::Octetstring));
        assert_eq!(
            ty("enum:Color"),
            plain(TypeExpr::Named("paint_color".to_string()))
        );
        assert_eq!(
            ty("union:Payment"),
            plain(TypeExpr::Named("order_payment".to_string()))
        );
        assert_eq!(
            ty("optional<string[32]>"),
            Ok(Written {
                ty: TypeExpr::UniversalCharstring,
                optional: true
            })
        );
    }

    /// A list states its lower count, 0 where none is written, and a named
    /// bound its value; a list of lists closes each at its own `>`.
    #[test]
    fn a_list_is_a_record_of_with_its_counts() {
        assert_eq!(
            ty("list<record:Line>[4]"),
            plain(TypeExpr::RecordOf {
                min: "0".to_string(),
                max: "4".to_string(),
                element: Box::new(TypeExpr::Named("order_line".to_string())),
            })
        );
        assert_eq!(
            ty("list<list<uint8>[1..4]>[2..n=3]"),
            plain(TypeExpr::RecordOf {
                min: "2".to_string(),
                max: "3".to_string(),
                element: Box::new(TypeExpr::RecordOf {
                    min: "1".to_string(),
                    max: "4".to_string(),
                    element: Box::new(TypeExpr::Named("uint8".to_string())),
                }),
            })
        );
    }

    /// What has no TTCN-3 form, and an alias the document does not import,
    /// is said so.
    #[test]
    fn a_type_with_no_form_is_named() {
        assert!(ty("float64").is_err());
        assert!(ty("list<optional<uint8>>[3]").is_err());
        assert!(ty("record:Other").is_err());
        assert!(ty("list<uint8>").is_err());
        assert!(ty("map<uint8>").is_err());
    }
}
