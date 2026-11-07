// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the documents of the test profile share (docs/adr/0091,
//! docs/SCE_TEST_PROFILE.md §11): the refusal a backend gives one until the
//! profile's runtime lands there for its kind, the reading of a duration, and what a literal
//! of a type and a matcher at a position of a type are.
//!
//! The parser asks the last two wherever a document states the type itself:
//! the root of a template and every position its own type reaches, a formal
//! parameter, a module parameter. A position whose type is a field of an
//! imported record or a case of an imported union is asked the same
//! questions where the imports are read ([`crate::forge::test_template_check`]),
//! by the one reading of a matcher at a position both share
//! ([`check_position`], [`check_range_bound`]).

use crate::forge::error::{GenerateError, SourceLocation, TestProfileRule};
use crate::forge::model::{
    ForgeKind, SceType, TestMatcher, TestMatcherForm, TestTemplateParameter, TestValue,
};
use crate::forge::value_type::{TypeOf, ValueType};

/// Why a backend does not generate a document of the test profile yet, as
/// every refusal of one says it.
pub const NOT_GENERATED_YET: &str = "a document of the test profile is read and checked on \
                                     every backend, and a backend generates one of its kind \
                                     only once the profile's runtime lands there for that kind \
                                     (docs/SCE_TEST_PROFILE.md §11.9)";

/// The refusal of a document of the test profile on a backend that does not
/// generate its kind yet (`generate/unsupported-feature`): `kind` and `name`
/// name it, `at` is its root.
pub fn document_not_generated(
    kind: ForgeKind,
    name: &str,
    at: Option<SourceLocation>,
) -> GenerateError {
    GenerateError::unsupported_at(format!("{kind} '{name}': {NOT_GENERATED_YET}"), at)
}

// ── Durations ──────────────────────────────────────────────────

/// Why a text is not a duration of the profile (docs/SCE_TEST_PROFILE.md
/// §4.7: SCXML's delay syntax, whole microseconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationRefusal {
    /// Not a decimal number followed by `s` or `ms`, SCXML's delay syntax.
    NotADelay,
    /// A delay that is not a whole number of microseconds.
    NotWholeMicroseconds,
    /// More microseconds than 64 bits hold.
    TooLong,
}

impl DurationRefusal {
    /// What a legal duration is, phrased to stand after "expected".
    pub fn rule(self) -> &'static str {
        match self {
            Self::NotADelay => {
                "a delay in SCXML's syntax: a decimal number followed by `s` or `ms`, as `2s` \
                 or `500ms`"
            }
            Self::NotWholeMicroseconds => {
                "a whole number of microseconds: no digit past the sixth decimal place of a \
                 second, or past the third of a millisecond"
            }
            Self::TooLong => "at most 2^64 - 1 microseconds",
        }
    }
}

/// The duration `text` writes, in microseconds.
pub fn duration_us(text: &str) -> Result<u64, DurationRefusal> {
    let written = text
        .trim_matches(|c: char| c.is_ascii_whitespace())
        .to_ascii_lowercase();
    let (number, scale) = if let Some(number) = written.strip_suffix("ms") {
        (number, 1_000u64)
    } else if let Some(number) = written.strip_suffix('s') {
        (number, 1_000_000u64)
    } else {
        return Err(DurationRefusal::NotADelay);
    };
    let (whole, fraction) = match number.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (number, None),
    };
    let all_digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(whole)
        || fraction.is_some_and(|f| f.is_empty() || !all_digits(f))
        || (whole.is_empty() && fraction.is_none())
    {
        return Err(DurationRefusal::NotADelay);
    }
    let mut us: u64 = 0;
    for digit in whole.bytes() {
        us = us
            .checked_mul(10)
            .and_then(|v| v.checked_add(u64::from(digit - b'0')))
            .ok_or(DurationRefusal::TooLong)?;
    }
    us = us.checked_mul(scale).ok_or(DurationRefusal::TooLong)?;
    let mut place = scale / 10;
    for digit in fraction.unwrap_or("").bytes() {
        let digit = u64::from(digit - b'0');
        if place == 0 {
            if digit != 0 {
                return Err(DurationRefusal::NotWholeMicroseconds);
            }
            continue;
        }
        us = us
            .checked_add(digit * place)
            .ok_or(DurationRefusal::TooLong)?;
        place /= 10;
    }
    Ok(us)
}

// ── Types, literals and matchers ───────────────────────────────

/// Whether `ty` holds a `float32` or a `float64` anywhere its own spelling
/// reaches (docs/SCE_TEST_PROFILE.md §8.1). A record or a union is named by
/// its alias, and what it holds is asked of its document.
pub fn holds_float(ty: &ValueType) -> bool {
    match ty {
        TypeOf::Scalar(scalar) => scalar.is_float(),
        TypeOf::Optional { elem } | TypeOf::List { elem, .. } => holds_float(elem),
        TypeOf::Text { .. }
        | TypeOf::Bytes { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. } => false,
    }
}

/// Whether `text` is a literal of `ty` (docs/SCE_TEST_PROFILE.md §11.4): an
/// integer in decimal within the type's range, `true` or `false`, the name of
/// a variant, a text within its bound, an even number of hex digits within
/// its bound. The `Err` is the rule a literal of `ty` satisfies, phrased to
/// stand after "expected". A type no literal is written for (a float, a
/// record, a list, a union) is refused with its own rule.
///
/// Whether a name is a variant of its enum needs the enum's document, and is
/// asked where the imports are read; here a variant is any name.
pub fn literal_of(ty: &ValueType, text: &str) -> Result<(), String> {
    match ty {
        TypeOf::Scalar(scalar) => scalar_literal(scalar, text),
        TypeOf::Optional { elem } => literal_of(elem, text),
        TypeOf::Text { max } => {
            if text.len() <= max.max as usize {
                Ok(())
            } else {
                Err(format!(
                    "a text of at most {} bytes, the bound of {ty}",
                    max.max
                ))
            }
        }
        TypeOf::Bytes { max } => {
            let hex = text.trim();
            if !hex.len().is_multiple_of(2) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("an even number of hex digits, two for each octet".to_string());
            }
            if hex.len() / 2 > max.max as usize {
                return Err(format!(
                    "at most {} octets, the bound of {ty}, as hex digits",
                    max.max
                ));
            }
            Ok(())
        }
        TypeOf::Record { .. } | TypeOf::Union { .. } | TypeOf::List { .. } => Err(format!(
            "a matcher for each part of a {}: no literal is written for one",
            ty.constructor()
        )),
    }
}

/// [`literal_of`] for a scalar.
fn scalar_literal(scalar: &SceType, text: &str) -> Result<(), String> {
    if let Some((lo, hi)) = scalar.int_value_range() {
        let rule = || {
            format!(
                "an integer in decimal in {lo}..={hi} ({})",
                scalar.as_attr()
            )
        };
        return match decimal_integer(text) {
            Some(n) if n >= lo && n <= hi => Ok(()),
            _ => Err(rule()),
        };
    }
    match scalar {
        SceType::Bool => match text.trim() {
            "true" | "false" => Ok(()),
            _ => Err("`true` or `false`".to_string()),
        },
        SceType::Enum(reference) => {
            let name = text.trim();
            if !name.is_empty() && !name.contains(char::is_whitespace) {
                Ok(())
            } else {
                Err(format!(
                    "the name of a variant of the enum {}",
                    reference.alias
                ))
            }
        }
        _ => Err(format!(
            "a type a literal is written for: {} is not one (docs/SCE_TEST_PROFILE.md §8.1)",
            scalar.as_attr()
        )),
    }
}

/// `text` read as an integer in decimal: an optional `-` and digits.
pub fn decimal_integer(text: &str) -> Option<i128> {
    let text = text.trim();
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<i128>().ok()
}

/// What stands at a position of type `ty` once an optional's absence is set
/// apart: the type a present value has.
pub fn present(ty: &ValueType) -> &ValueType {
    match ty {
        TypeOf::Optional { elem } => elem.as_ref(),
        other => other,
    }
}

/// Whether a matcher of `form` may stand where a value of `ty` is matched
/// (the table of docs/SCE_TEST_PROFILE.md §11.4). At an optional position a
/// matcher of the present value's type stands too, and `omit` and
/// `any-value-or-none` stand only there. The `Err` is the rule, phrased to
/// stand after "expected".
pub fn matcher_admitted(form: &TestMatcherForm, ty: &ValueType) -> Result<(), String> {
    let optional = matches!(ty, TypeOf::Optional { .. });
    let value = present(ty);
    let admitted = match form {
        TestMatcherForm::Specific { .. } => match value {
            TypeOf::Scalar(scalar) => !scalar.is_float(),
            TypeOf::Text { .. } | TypeOf::Bytes { .. } => true,
            TypeOf::Record { .. }
            | TypeOf::Union { .. }
            | TypeOf::List { .. }
            | TypeOf::Optional { .. } => false,
        },
        TestMatcherForm::RecordMatch { .. } => matches!(value, TypeOf::Record { .. }),
        TestMatcherForm::ListMatch { .. } => matches!(value, TypeOf::List { .. }),
        TestMatcherForm::UnionMatch { .. } => matches!(value, TypeOf::Union { .. }),
        TestMatcherForm::ValueRange { .. } => {
            matches!(value, TypeOf::Scalar(scalar) if scalar.int_value_range().is_some())
        }
        TestMatcherForm::AnyValueOrNone | TestMatcherForm::Omit => optional,
        TestMatcherForm::ValueList { .. }
        | TestMatcherForm::Complement { .. }
        | TestMatcherForm::AnyValue
        | TestMatcherForm::TemplateRef { .. } => true,
    };
    if admitted {
        return Ok(());
    }
    Err(match form {
        TestMatcherForm::Specific { .. } => {
            "an integer, `bool`, `enum:`, `string[N]` or `bytes[N]` position".to_string()
        }
        TestMatcherForm::RecordMatch { .. } => "a `record:` position".to_string(),
        TestMatcherForm::ListMatch { .. } => "a `list<T>[B]` position".to_string(),
        TestMatcherForm::UnionMatch { .. } => "a `union:` position".to_string(),
        TestMatcherForm::ValueRange { .. } => "an integer position".to_string(),
        TestMatcherForm::AnyValueOrNone | TestMatcherForm::Omit => {
            "an `optional<T>` position".to_string()
        }
        TestMatcherForm::ValueList { .. }
        | TestMatcherForm::Complement { .. }
        | TestMatcherForm::AnyValue
        | TestMatcherForm::TemplateRef { .. } => String::new(),
    })
}

/// Whether the attributes `matcher` carries may stand where a value of `ty`
/// is matched: a length restriction on a `string[N]`, `bytes[N]` or
/// `list<T>[B]` (B.1.4.1), `ifpresent` on an `optional<T>` (B.1.4.2). The
/// `Err` names the attribute and its rule.
pub fn attributes_admitted(
    matcher: &TestMatcher,
    ty: &ValueType,
) -> Result<(), (&'static str, &'static str)> {
    let has_length = matches!(
        present(ty),
        TypeOf::Text { .. } | TypeOf::Bytes { .. } | TypeOf::List { .. }
    );
    if !has_length {
        if matcher.length_min.is_some() {
            return Err((
                "length-min",
                "a `string[N]`, `bytes[N]` or `list<T>[B]` position",
            ));
        }
        if matcher.length_max.is_some() {
            return Err((
                "length-max",
                "a `string[N]`, `bytes[N]` or `list<T>[B]` position",
            ));
        }
    }
    if matcher.ifpresent && !matches!(ty, TypeOf::Optional { .. }) {
        return Err(("ifpresent", "an `optional<T>` position"));
    }
    Ok(())
}

/// The type of the elements of a list at a position of type `ty`, when `ty`
/// is one.
pub fn list_element(ty: &ValueType) -> Option<&ValueType> {
    match present(ty) {
        TypeOf::List { elem, .. } => Some(elem.as_ref()),
        _ => None,
    }
}

// ── A matcher at its position ──────────────────────────────────

/// A rule of docs/SCE_TEST_PROFILE.md §11.4 that one matcher breaks where it
/// stands: the rule, which decides the wire code; what the document wrote
/// where the rule is broken; and what is wrong, phrased as the refusal's
/// detail. The reader that holds the matcher places it: the parser on the
/// element, the import-time check on the matcher's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatcherRefusal {
    pub rule: TestProfileRule,
    pub actual: String,
    pub detail: String,
}

/// The formal parameter `name` of a template whose parameters are
/// `parameters`, or the refusal of `element` naming one the template does
/// not declare (`test-profile/template-parameter`).
pub fn formal_parameter<'p>(
    parameters: &'p [TestTemplateParameter],
    element: &str,
    name: &str,
) -> Result<&'p TestTemplateParameter, MatcherRefusal> {
    parameters
        .iter()
        .find(|p| p.id == name)
        .ok_or_else(|| MatcherRefusal {
            rule: TestProfileRule::TemplateParameter,
            actual: name.to_string(),
            detail: format!(
                "{element} names '{name}', which is not a formal parameter of the template \
                 (a <data sce:direction=in> with no expr)"
            ),
        })
}

/// Whether a value of `value` stands where a value of `position` is matched:
/// the two are one type by `same`, or `position` is optional and `value` is
/// the type of its present value. `same` is asked `(position, value)`, in that
/// order, since the two may be read in the scopes of different documents.
pub fn stands_at(
    position: &ValueType,
    value: &ValueType,
    same: &dyn Fn(&ValueType, &ValueType) -> bool,
) -> bool {
    same(position, value)
        || matches!(position, TypeOf::Optional { elem } if same(elem.as_ref(), value))
}

/// The rules of a matcher where a value of `ty` is matched
/// (docs/SCE_TEST_PROFILE.md §11.4): its form and its attributes are admitted
/// there (`test-profile/matcher-type`), and a specific value is a literal of
/// the type (`test-profile/literal`) or a formal parameter of it
/// (`test-profile/template-parameter`).
///
/// `parameters` are the formal parameters of the template that holds the
/// matcher, and `same` tells whether a parameter's type is the position's
/// ([`stands_at`]). Whether a literal of an enum names one of its variants
/// needs the enum's document, and is the caller's to ask.
pub fn check_position(
    matcher: &TestMatcher,
    ty: &ValueType,
    parameters: &[TestTemplateParameter],
    same: &dyn Fn(&ValueType, &ValueType) -> bool,
) -> Result<(), MatcherRefusal> {
    let local = matcher.form.element();
    if let Err(rule) = matcher_admitted(&matcher.form, ty) {
        return Err(MatcherRefusal {
            rule: TestProfileRule::MatcherType,
            actual: local.to_string(),
            detail: format!(
                "<sce:{local}> stands where a value of {ty} is matched: it stands only at {rule}"
            ),
        });
    }
    if let Err((attr, rule)) = attributes_admitted(matcher, ty) {
        return Err(MatcherRefusal {
            rule: TestProfileRule::MatcherType,
            actual: attr.to_string(),
            detail: format!(
                "<sce:{local}> carries {attr} where a value of {ty} is matched: it is written \
                 only at {rule}"
            ),
        });
    }
    match &matcher.form {
        TestMatcherForm::Specific {
            value: TestValue::Value(text),
        } => literal_of(ty, text).map_err(|rule| MatcherRefusal {
            rule: TestProfileRule::Literal,
            actual: text.clone(),
            detail: format!("<sce:{local}> value '{text}' is not a value of {ty}: expected {rule}"),
        }),
        TestMatcherForm::Specific {
            value: TestValue::Param(name),
        } => {
            let parameter = formal_parameter(parameters, &format!("<sce:{local}>"), name)?;
            if stands_at(ty, &parameter.parameter_type, same) {
                return Ok(());
            }
            Err(MatcherRefusal {
                rule: TestProfileRule::TemplateParameter,
                actual: name.clone(),
                detail: format!(
                    "<sce:{local}> names '{name}', of type {}, where a value of {ty} is matched",
                    parameter.parameter_type
                ),
            })
        }
        _ => Ok(()),
    }
}

/// A bound of a `value-range` (B.1.2.5), written by `element`: an integer in
/// decimal, or a formal parameter of an integer type; of the position's type
/// `ty` where the caller knows it (`None` holds the bound to the integer
/// syntax alone). `parameters` and `same` are as [`check_position`] takes them.
pub fn check_range_bound(
    element: &str,
    value: &TestValue,
    ty: Option<&ValueType>,
    parameters: &[TestTemplateParameter],
    same: &dyn Fn(&ValueType, &ValueType) -> bool,
) -> Result<(), MatcherRefusal> {
    match value {
        TestValue::Value(text) => {
            let fits = match ty {
                Some(ty) => literal_of(ty, text),
                None => decimal_integer(text)
                    .map(|_| ())
                    .ok_or_else(|| "an integer in decimal".to_string()),
            };
            fits.map_err(|rule| MatcherRefusal {
                rule: TestProfileRule::Literal,
                actual: text.clone(),
                detail: format!(
                    "{element} value '{text}' is not a bound of the range: expected {rule}"
                ),
            })
        }
        TestValue::Param(name) => {
            let parameter = formal_parameter(parameters, element, name)?;
            let integer = matches!(
                &parameter.parameter_type,
                TypeOf::Scalar(scalar) if scalar.int_value_range().is_some()
            );
            let of_position = ty.is_none_or(|ty| stands_at(ty, &parameter.parameter_type, same));
            if integer && of_position {
                return Ok(());
            }
            Err(MatcherRefusal {
                rule: TestProfileRule::TemplateParameter,
                actual: name.clone(),
                detail: format!(
                    "{element} names '{name}', of type {}, and a bound of this range is a value \
                     of {}",
                    parameter.parameter_type,
                    ty.map_or_else(|| "an integer type".to_string(), |ty| ty.to_string())
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::value_type::Bound;

    #[test]
    fn a_duration_is_a_delay_in_whole_microseconds() {
        assert_eq!(duration_us("2s"), Ok(2_000_000));
        assert_eq!(duration_us("500ms"), Ok(500_000));
        assert_eq!(duration_us("0.5ms"), Ok(500));
        assert_eq!(duration_us("1.000001s"), Ok(1_000_001));
        assert_eq!(duration_us(" 10S "), Ok(10_000_000));
        assert_eq!(
            duration_us("0.0005ms"),
            Err(DurationRefusal::NotWholeMicroseconds)
        );
        assert_eq!(duration_us("0.5000ms"), Ok(500));
        assert_eq!(duration_us("10"), Err(DurationRefusal::NotADelay));
        assert_eq!(duration_us("1.s"), Err(DurationRefusal::NotADelay));
        assert_eq!(duration_us("s"), Err(DurationRefusal::NotADelay));
        assert_eq!(duration_us("-1s"), Err(DurationRefusal::NotADelay));
        assert_eq!(
            duration_us("99999999999999999999s"),
            Err(DurationRefusal::TooLong)
        );
    }

    #[test]
    fn a_literal_is_read_by_its_type() {
        let uint8 = TypeOf::Scalar(SceType::Uint8);
        assert!(literal_of(&uint8, "255").is_ok());
        assert!(literal_of(&uint8, "256").is_err());
        assert!(literal_of(&uint8, "0x10").is_err(), "decimal only");
        assert!(literal_of(&TypeOf::Scalar(SceType::Int8), "-128").is_ok());
        assert!(literal_of(&TypeOf::Scalar(SceType::Bool), "true").is_ok());
        assert!(literal_of(&TypeOf::Scalar(SceType::Bool), "1").is_err());
        let text: ValueType = TypeOf::Text { max: Bound::of(3) };
        assert!(literal_of(&text, "abc").is_ok());
        assert!(literal_of(&text, "abcd").is_err());
        let bytes: ValueType = TypeOf::Bytes { max: Bound::of(2) };
        assert!(literal_of(&bytes, "0aFF").is_ok());
        assert!(literal_of(&bytes, "0aF").is_err());
        assert!(literal_of(&bytes, "0a0b0c").is_err());
        assert!(literal_of(&TypeOf::Scalar(SceType::Float32), "1").is_err());
        let optional: ValueType = TypeOf::Optional {
            elem: Box::new(uint8.clone()),
        };
        assert!(literal_of(&optional, "7").is_ok());
    }

    #[test]
    fn a_matcher_stands_where_its_type_admits_it() {
        let uint8: ValueType = TypeOf::Scalar(SceType::Uint8);
        let optional: ValueType = TypeOf::Optional {
            elem: Box::new(uint8.clone()),
        };
        let omit = TestMatcherForm::Omit;
        assert!(matcher_admitted(&omit, &uint8).is_err());
        assert!(matcher_admitted(&omit, &optional).is_ok());
        let range = TestMatcherForm::ValueRange {
            lower: None,
            upper: None,
        };
        assert!(matcher_admitted(&range, &uint8).is_ok());
        assert!(matcher_admitted(&range, &optional).is_ok());
        assert!(matcher_admitted(&range, &TypeOf::Scalar(SceType::Bool)).is_err());
        let record = TestMatcherForm::RecordMatch { fields: Vec::new() };
        assert!(matcher_admitted(&record, &uint8).is_err());
        assert!(matcher_admitted(
            &record,
            &TypeOf::Record {
                alias: "R".to_string()
            }
        )
        .is_ok());
        assert!(holds_float(&TypeOf::List {
            elem: Box::new(TypeOf::Scalar(SceType::Float64)),
            max: Bound::of(2),
        }));
        assert!(!holds_float(&optional));
    }
}
