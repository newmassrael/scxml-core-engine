// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Each `test-template` a suite reaches as a TTCN-3 template
//! (docs/SCE_TEST_PROFILE.md §6.1, §9.2 E2, §9.5).
//!
//! A template is written under its document's name, of its type, with its
//! value parameters in order, and each matching mechanism of §6.1 is written
//! as ES 201 873-1 Annex B.1 writes it: a value, `?`, `*`, `omit`, a value
//! list `(a, b)`, `complement(a)`, a range `(lower .. upper)` with
//! `-infinity` or `infinity` for a bound not written, `length(m .. n)`,
//! `ifpresent`, a record `{ f := m, … }` in the record's field order, a list
//! `{ m, … }`, a union `{ v := m }` and a reference to a template with its
//! arguments. A value parameter of an `optional<T>` type is a template
//! parameter restricted to a value or `omit`, `template (omit) T` (-1 §15.8),
//! which is what a value that may be absent is in TTCN-3.
//!
//! What is written is a model of the template ([`TemplateView`]); the
//! template `ttcn3/template.ttcn.jinja2` spells it.

use serde::Serialize;

use super::names::{Declared, Namespace};
use super::reach::{Reach, Reached};
use super::types::{type_expr, Site, TypeExpr};
use crate::forge::error::{ForgeError, Located};
use crate::forge::model::{
    ForgeDocument, ForgeKind, SceType, TestMatcher, TestMatcherForm, TestTemplateModel, TestValue,
};
use crate::forge::test_profile::{decimal_integer, list_element, present};
use crate::forge::value_type::{ScopedRecord, TypeOf, TypeScope, ValueType};

/// A template as the export writes it.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateView {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
    pub parameters: Vec<ParameterView>,
    pub matcher: MatcherView,
}

/// A formal parameter: its name, its type, and whether it may be `omit`.
#[derive(Debug, Clone, Serialize)]
pub struct ParameterView {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
    pub omit: bool,
}

/// One matcher: what it matches with, and the attributes every matcher may
/// carry.
#[derive(Debug, Clone, Serialize)]
pub struct MatcherView {
    pub body: Body,
    pub length: Option<LengthView>,
    pub ifpresent: bool,
}

/// A length restriction: `max` is `None` where the template writes no upper
/// count, which TTCN-3 writes `infinity`.
#[derive(Debug, Clone, Serialize)]
pub struct LengthView {
    pub min: u32,
    pub max: Option<u32>,
}

/// A matcher of a record's field, by the field's name.
#[derive(Debug, Clone, Serialize)]
pub struct FieldMatcher {
    pub name: String,
    pub matcher: MatcherView,
}

/// What a matcher matches with, one form per mechanism of §6.1 and per kind
/// of value a specific value is.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "form", rename_all = "kebab-case")]
pub enum Body {
    /// An integer, as the text of its digits.
    Integer {
        value: String,
    },
    Boolean {
        value: bool,
    },
    /// A variant of an enumerated type, by its name.
    Enumerated {
        variant: String,
    },
    /// A text, written by the filter `ttcn3_charstring`.
    Charstring {
        text: String,
    },
    /// A byte string, its hex digits in upper case.
    Octetstring {
        hex: String,
    },
    /// A formal parameter of the template.
    Param {
        name: String,
    },
    Record {
        fields: Vec<FieldMatcher>,
    },
    List {
        elements: Vec<MatcherView>,
    },
    Union {
        variant: String,
        case: Option<Box<MatcherView>>,
    },
    ValueList {
        items: Vec<MatcherView>,
    },
    Complement {
        items: Vec<MatcherView>,
    },
    AnyValue,
    AnyValueOrNone,
    /// A range, each bound the text of its digits or a formal parameter;
    /// `None` where no bound is written.
    Range {
        lower: Option<String>,
        upper: Option<String>,
    },
    Omit,
    /// A reference to a template, by its name, with its arguments.
    Reference {
        template: String,
        args: Vec<String>,
    },
}

/// Each template the suite reaches, in the order of their names, every name
/// it writes entered in `names`.
pub(super) fn templates(
    reach: &Reach,
    names: &mut Namespace,
) -> Result<Vec<TemplateView>, Located<ForgeError>> {
    let mut views = Vec::new();
    for document in reach.sorted() {
        if let ForgeDocument::TestTemplate(template) = &document.parsed.document {
            views.push(template_view(reach, document, template, names)?);
        }
    }
    Ok(views)
}

/// Where a matcher stands: the type of the value it matches, read in
/// `scope`.
struct Position<'s> {
    ty: ValueType,
    scope: &'s TypeScope,
}

/// What a template's matchers are read with: the suite, the template's
/// document, and where a refusal of one stands.
struct Reading<'r> {
    reach: &'r Reach,
    document: &'r Reached,
    template: &'r TestTemplateModel,
}

impl Reading<'_> {
    /// The refusal of the matcher on `line` for `why`.
    fn site(&self, line: Option<u32>) -> Site {
        Site {
            origin: self.document.origin().at(line),
            subject: format!("test-template '{}'", self.template.name),
        }
    }
}

fn template_view(
    reach: &Reach,
    document: &Reached,
    template: &TestTemplateModel,
    names: &mut Namespace,
) -> Result<TemplateView, Located<ForgeError>> {
    let origin = document.origin();
    names.definition(
        &document.name,
        Declared::read("the template", origin.clone()),
    )?;
    let reading = Reading {
        reach,
        document,
        template,
    };
    let site = reading.site(None);
    let ty = declared_type(present(&template.template_type), &document.scope, &site)?;
    let mut parameters = Vec::new();
    for parameter in &template.parameters {
        names.nested(
            &parameter.id,
            Declared::read("the formal parameter", origin.at(parameter.source_line)),
        )?;
        let site = reading.site(parameter.source_line);
        let omit = matches!(parameter.parameter_type, TypeOf::Optional { .. });
        parameters.push(ParameterView {
            name: parameter.id.clone(),
            ty: declared_type(present(&parameter.parameter_type), &document.scope, &site)?,
            omit,
        });
    }
    let root = Position {
        ty: template.template_type.clone(),
        scope: &document.scope,
    };
    Ok(TemplateView {
        name: document.name.clone(),
        ty,
        parameters,
        matcher: matcher(&reading, &template.matcher, &root)?,
    })
}

/// The type a template or a formal parameter is declared of. TTCN-3 declares
/// either by a type's name, and a list has none in the documents, so a list
/// is refused by name.
fn declared_type(
    ty: &ValueType,
    scope: &TypeScope,
    site: &Site,
) -> Result<TypeExpr, Located<ForgeError>> {
    if matches!(ty, TypeOf::List { .. }) {
        return Err(site.refuse(format!(
            "{ty} is a list, and TTCN-3 declares a template and a parameter by a type's \
             name, which a list of the documents does not have"
        )));
    }
    type_expr(ty, scope, site)
}

/// The record a value at `at` is of, read in its own document.
fn record_at<'s>(at: &Position<'s>) -> Option<&'s ScopedRecord> {
    let scope: &'s TypeScope = at.scope;
    match present(&at.ty) {
        TypeOf::Record { alias } => scope.records.get(alias),
        _ => None,
    }
}

fn matcher(
    reading: &Reading<'_>,
    m: &TestMatcher,
    at: &Position<'_>,
) -> Result<MatcherView, Located<ForgeError>> {
    let site = reading.site(m.source_line);
    let unread = || {
        site.refuse(format!(
            "<sce:{}> stands where a value of {} is matched, which it cannot match",
            m.form.element(),
            at.ty
        ))
    };
    let body = match &m.form {
        TestMatcherForm::Specific {
            value: TestValue::Param(name),
        } => Body::Param { name: name.clone() },
        TestMatcherForm::Specific {
            value: TestValue::Value(text),
        } => match present(&at.ty) {
            TypeOf::Scalar(SceType::Bool) => Body::Boolean {
                value: text.trim() == "true",
            },
            TypeOf::Scalar(SceType::Enum(_)) => Body::Enumerated {
                variant: text.trim().to_string(),
            },
            TypeOf::Scalar(_) => Body::Integer {
                value: decimal_integer(text).ok_or_else(unread)?.to_string(),
            },
            TypeOf::Text { .. } => Body::Charstring { text: text.clone() },
            TypeOf::Bytes { .. } => Body::Octetstring {
                hex: text.trim().to_ascii_uppercase(),
            },
            _ => return Err(unread()),
        },
        TestMatcherForm::RecordMatch { fields } => {
            let record = record_at(at).ok_or_else(unread)?;
            let mut matched = Vec::new();
            for (name, ty) in &record.fields {
                let field = fields
                    .iter()
                    .find(|f| f.field.as_deref() == Some(name.as_str()))
                    .ok_or_else(unread)?;
                let position = Position {
                    ty: ty.clone(),
                    scope: &record.scope,
                };
                matched.push(FieldMatcher {
                    name: name.clone(),
                    matcher: matcher(reading, field, &position)?,
                });
            }
            Body::Record { fields: matched }
        }
        TestMatcherForm::ListMatch { elements } => {
            let element = Position {
                ty: list_element(&at.ty).ok_or_else(unread)?.clone(),
                scope: at.scope,
            };
            Body::List {
                elements: elements
                    .iter()
                    .map(|e| matcher(reading, e, &element))
                    .collect::<Result<_, _>>()?,
            }
        }
        TestMatcherForm::UnionMatch { variant, case } => {
            let TypeOf::Union { alias } = present(&at.ty) else {
                return Err(unread());
            };
            let union = at.scope.unions.get(alias).ok_or_else(unread)?;
            let held = union
                .cases
                .iter()
                .find(|c| &c.variant.name == variant)
                .ok_or_else(unread)?;
            let case = match (case, &held.record) {
                (Some(case), Some(record)) => {
                    let position = Position {
                        ty: TypeOf::Record {
                            alias: record.clone(),
                        },
                        scope: &union.scope,
                    };
                    Some(Box::new(matcher(reading, case, &position)?))
                }
                (None, _) => None,
                (Some(_), None) => return Err(unread()),
            };
            Body::Union {
                variant: variant.clone(),
                case,
            }
        }
        TestMatcherForm::ValueList { items } => Body::ValueList {
            items: items
                .iter()
                .map(|item| matcher(reading, item, at))
                .collect::<Result<_, _>>()?,
        },
        TestMatcherForm::Complement { items } => Body::Complement {
            items: items
                .iter()
                .map(|item| matcher(reading, item, at))
                .collect::<Result<_, _>>()?,
        },
        TestMatcherForm::AnyValue => Body::AnyValue,
        TestMatcherForm::AnyValueOrNone => Body::AnyValueOrNone,
        // A bound's line places a refusal of the document; the module writes
        // only the bound.
        TestMatcherForm::ValueRange {
            lower,
            upper,
            lower_line: _,
            upper_line: _,
        } => Body::Range {
            lower: lower
                .as_ref()
                .map(|value| bound(value).ok_or_else(unread))
                .transpose()?,
            upper: upper
                .as_ref()
                .map(|value| bound(value).ok_or_else(unread))
                .transpose()?,
        },
        TestMatcherForm::Omit => Body::Omit,
        TestMatcherForm::TemplateRef { template, args } => Body::Reference {
            template: referenced(reading, template).ok_or_else(unread)?,
            args: args.clone(),
        },
    };
    let length = (m.length_min.is_some() || m.length_max.is_some()).then(|| LengthView {
        min: m.length_min.unwrap_or(0),
        max: m.length_max,
    });
    Ok(MatcherView {
        body,
        length,
        ifpresent: m.ifpresent,
    })
}

/// A bound of a range: the digits of a literal, or a formal parameter.
fn bound(value: &TestValue) -> Option<String> {
    match value {
        TestValue::Value(text) => decimal_integer(text).map(|n| n.to_string()),
        TestValue::Param(name) => Some(name.clone()),
    }
}

/// The name of the template the template being read imports as `alias`.
fn referenced(reading: &Reading<'_>, alias: &str) -> Option<String> {
    let import = reading
        .document
        .parsed
        .imports
        .iter()
        .find(|i| i.alias == alias && i.kind == ForgeKind::TestTemplate)?;
    reading
        .reach
        .imported(&reading.document.dir, import)
        .map(|document| document.name.clone())
}
