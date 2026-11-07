// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §12.2): `Value`, the type the template matches; `match`, whether a value
//! matches it (ES 201 873-1 Annex B.1); and, for a specific template (§6.2),
//! `value`, the one value it sends.
//!
//! The matcher is a tree, so it is lowered here, one expression per matcher
//! and the expression of a matcher made of its children's, and the template
//! (`forge/cpp/test_template.h.jinja2`) writes the declarations around the two
//! expressions. Matching is on the values the event schema's unit declares, not
//! on an encoding (ADR 0091 C2).
//!
//! Every type is the one the schema unit spells. The template's own type and
//! its parameters' are spelled by [`unit_type`], as a field of that type is.
//! A position below a record's field is read through the type the record's
//! unit declares for the field (`decltype(<Record>Payload::<field>)`), and
//! through `value_type` below an optional or a list, so an enum a nested record
//! holds is named by the one declaration of it, whatever document imports it
//! and under whatever alias.

use super::*;
use crate::forge::expr::{wide_literal_spelling, WideLiteral};
use crate::forge::schema_unit_type::{unit_type, C11Declarations};
use crate::forge::value_type::{ScopedRecord, TypeOf, TypeScope, ValueType};
use crate::generator::Language;

/// The header of the test profile's runtime that declares the length of a
/// text in characters, which a length restriction on a `string[N]` counts.
const TEXT_LENGTH_HEADER: &str = "test/Length.h";
/// The function [`TEXT_LENGTH_HEADER`] declares.
const TEXT_LENGTH: &str = "SCE::Test::textLength";

/// The unit of the test template `m`, whose imports are `imports`. Refused by
/// name where the template's type or a parameter's reaches a union at any
/// depth, as a schema unit's union field is on C++
/// ([`crate::forge::value_type::union_unlowered`]).
pub(super) fn render_test_template_cpp(
    env: &minijinja::Environment,
    m: &TestTemplateModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Cpp, imports);
    let scope = import_type_scope(imports);
    let unit = TemplateUnit {
        template: m,
        imports,
        counts_characters: std::cell::Cell::new(false),
    };
    for ty in
        std::iter::once(&m.template_type).chain(m.parameters.iter().map(|p| &p.parameter_type))
    {
        if let Some(alias) = crate::forge::test_profile::union_reached(ty, &scope) {
            return Err(unit.refused(crate::forge::value_type::union_unlowered(&alias)));
        }
    }
    let names = SchemaUnitNames { l: &l, imports };
    let spell = |ty: &ValueType| {
        unit_type(Language::Cpp, ty, &names, &mut C11Declarations::default())
            .map_err(|why| unit.refused(why))
    };
    let value_type = spell(&m.template_type)?;
    let parameters: Vec<serde_json::Value> = m
        .parameters
        .iter()
        .map(|p| {
            Ok(serde_json::json!({
                "id": p.id,
                "name": cpp_parameter(&p.id),
                "ty": spell(&p.parameter_type)?,
                "sce_type": p.parameter_type.to_string(),
            }))
        })
        .collect::<Result<_, ForgeError>>()?;
    let root = Position::root(&m.template_type, &scope);
    let match_expr = unit.matches(&m.matcher, &root)?;
    let value_expr = if unit.specific() {
        Some(unit.value(&m.matcher, &root)?)
    } else {
        None
    };

    let mut ctx = l.base_context(&m.name);
    ctx.insert("template_name".into(), m.name.clone().into());
    ctx.insert("template_type".into(), m.template_type.to_string().into());
    ctx.insert("value_type".into(), value_type.into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("match_expr".into(), match_expr.into());
    ctx.insert("value_expr".into(), serde_json::json!(value_expr));
    // The record, enum and referenced templates' units, each as the import
    // names it ([`forge_import_identity`]); a union was refused above.
    let type_includes: Vec<String> = imports.iter().map(|i| i.include_stmt.clone()).collect();
    ctx.insert("type_includes".into(), serde_json::json!(type_includes));
    // The runtime's length of a text is included where a matcher counts one,
    // and nowhere else.
    ctx.insert(
        "counts_characters".into(),
        unit.counts_characters.get().into(),
    );
    ctx.insert("text_length_header".into(), TEXT_LENGTH_HEADER.into());
    l.render(env, "test_template", ctx)
}

/// The name the generated `match` and `value` take the formal parameter `id`
/// by: `param_<id>`, so that no parameter's name is one the unit declares
/// (`Value`, `value`, `match`) or a keyword of the language.
fn cpp_parameter(id: &str) -> String {
    format!("param_{id}")
}

/// One position of the value a matcher stands at.
#[derive(Clone)]
struct Position<'s> {
    /// The C++ expression of the value at the position, in `match`.
    expr: String,
    /// The C++ type the value at the position is held in, as the unit of its
    /// schema declares it.
    cpp: String,
    /// The type of the position, read in `scope`.
    ty: ValueType,
    scope: &'s TypeScope,
}

impl<'s> Position<'s> {
    /// The template's own value: `match`'s parameter `value`, of type `Value`.
    fn root(ty: &ValueType, scope: &'s TypeScope) -> Self {
        Position {
            expr: "value".to_string(),
            cpp: "Value".to_string(),
            ty: ty.clone(),
            scope,
        }
    }

    /// Whether the position is an `optional<T>`, which may hold no value.
    fn optional(&self) -> bool {
        matches!(self.ty, TypeOf::Optional { .. })
    }

    /// The value an optional position holds where it holds one, and any other
    /// position as it is.
    fn present(&self) -> Self {
        match &self.ty {
            TypeOf::Optional { elem } => Position {
                expr: format!("(*{})", self.expr),
                cpp: format!("{}::value_type", self.cpp),
                ty: (**elem).clone(),
                scope: self.scope,
            },
            _ => self.clone(),
        }
    }

    /// The element at `index` of the list this position holds.
    fn element(&self, index: usize, elem: &ValueType) -> Self {
        Position {
            expr: format!("{}[{index}]", self.expr),
            cpp: format!("{}::value_type", self.cpp),
            ty: elem.clone(),
            scope: self.scope,
        }
    }

    /// The field `id`, of type `ty`, of the record whose unit's payload type
    /// is `payload` and whose document reads its types in `scope`.
    fn field(&self, payload: &str, id: &str, ty: &ValueType, scope: &'s TypeScope) -> Self {
        let member = event_schema_field_ident(id, Language::Cpp);
        Position {
            expr: format!("{}.{member}", self.expr),
            cpp: format!("decltype({payload}::{member})"),
            ty: ty.clone(),
            scope,
        }
    }
}

/// The conjunction of `parts`, each already one operand: `true` for none.
fn all(parts: Vec<String>) -> String {
    let parts: Vec<String> = parts.into_iter().filter(|p| p != "true").collect();
    match parts.len() {
        0 => "true".to_string(),
        1 => parts.into_iter().next().unwrap_or_default(),
        _ => format!("({})", parts.join(" && ")),
    }
}

/// The disjunction of `parts`, each already one operand: `false` for none.
fn any(parts: Vec<String>) -> String {
    match parts.len() {
        0 => "false".to_string(),
        1 => parts.into_iter().next().unwrap_or_default(),
        _ => format!("({})", parts.join(" || ")),
    }
}

/// An integer literal of the profile (`test_profile::decimal_integer`) as C++
/// writes it: the two values no decimal constant of C++ holds as written are
/// spelled as every C++ expression of them is ([`wide_literal_spelling`]). A
/// suite's module parameter default is written by the same spelling.
pub(super) fn integer_literal(text: &str) -> Option<String> {
    let n = crate::forge::test_profile::decimal_integer(text)?;
    let raw = n.to_string();
    let wide = u64::try_from(n)
        .ok()
        .and_then(WideLiteral::of_unsigned)
        .or_else(|| i64::try_from(n).ok().and_then(WideLiteral::of_signed));
    Some(
        wide.and_then(|wide| wide_literal_spelling(Language::Cpp, wide, &raw))
            .unwrap_or(raw),
    )
}

/// One template, lowered: what each matcher of it matches and the value of a
/// specific one.
struct TemplateUnit<'a> {
    template: &'a TestTemplateModel,
    imports: &'a [ImportContext],
    /// Whether a matcher counts the characters of a text, which the runtime's
    /// [`TEXT_LENGTH_HEADER`] declares.
    counts_characters: std::cell::Cell<bool>,
}

impl TemplateUnit<'_> {
    /// The refusal of the template, by name, for `why`.
    fn refused(&self, why: impl std::fmt::Display) -> ForgeError {
        GenerateError::unsupported_at(
            format!("test-template '{}': {why}", self.template.name),
            self.template.source_location.clone(),
        )
        .into()
    }

    /// The template the import `alias` names: the namespace of its unit and
    /// what the import read of it.
    fn reference(&self, alias: &str) -> Result<(&str, &TestTemplateImport), ForgeError> {
        self.imports
            .iter()
            .find(|i| i.alias == alias && i.kind == "test-template")
            .and_then(|i| {
                i.test_template
                    .as_ref()
                    .map(|read| (i.namespace.as_str(), read))
            })
            .ok_or_else(|| {
                self.refused(format!(
                    "<sce:template-ref template=\"{alias}\"> names no test template the document \
                     imports"
                ))
            })
    }

    /// Whether the template is specific (docs/SCE_TEST_PROFILE.md §6.2): its
    /// own matchers are values alone, and every template it references is
    /// specific in turn, as its import read it.
    fn specific(&self) -> bool {
        if !self.template.is_specific() {
            return false;
        }
        let mut pending = vec![&self.template.matcher];
        while let Some(matcher) = pending.pop() {
            if let TestMatcherForm::TemplateRef { template, .. } = &matcher.form {
                match self.reference(template) {
                    Ok((_, referenced)) if referenced.specific => {}
                    _ => return false,
                }
            }
            pending.extend(matcher.children());
        }
        true
    }

    /// The call of the function `function` of the referenced template's unit
    /// in `namespace`, given `first` (the value `match` is asked of, or none
    /// for `value`) and the arguments `args` binds, each one of the unit's own
    /// parameters (-1 §15.1).
    fn call(namespace: &str, function: &str, first: Option<&str>, args: &[String]) -> String {
        let arguments: Vec<String> = first
            .map(str::to_string)
            .into_iter()
            .chain(args.iter().map(|arg| cpp_parameter(arg)))
            .collect();
        format!("{namespace}::{function}({})", arguments.join(", "))
    }

    /// The record a record position names, with the payload type its unit
    /// declares.
    fn record<'s>(&self, at: &Position<'s>) -> Result<(&'s ScopedRecord, String), ForgeError> {
        let TypeOf::Record { alias } = &at.ty else {
            return Err(self.refused(format!(
                "a <sce:record-match> stands where a value of {} is matched",
                at.ty
            )));
        };
        let record =
            at.scope.records.get(alias).ok_or_else(|| {
                self.refused(format!("`record:{alias}` names no event schema read"))
            })?;
        let payload = event_schema_payload_type(&record.name, Language::Cpp);
        Ok((record, payload))
    }

    /// Whether the value at `at` matches `m`: its form, then its length
    /// restriction, then `ifpresent` (B.1.4.1, B.1.4.2).
    fn matches(&self, m: &TestMatcher, at: &Position<'_>) -> Result<String, ForgeError> {
        let mut held = self.form(m, at)?;
        if m.length_min.is_some() || m.length_max.is_some() {
            held = all(vec![held, self.length(m, at)?]);
        }
        if m.ifpresent && at.optional() {
            held = format!("(!{}.has_value() || {held})", at.expr);
        }
        Ok(held)
    }

    /// Whether `m`, standing at an optional position, reads whether the
    /// position holds a value, rather than the value it holds: `omit` and
    /// `any-value-or-none` (B.1.2.4, B.1.2.8), a list or a complement of
    /// matchers at the same position, each of which may be one, a parameter
    /// and a reference whose type is the optional's own.
    fn reads_absence(&self, m: &TestMatcher) -> bool {
        let optional = |ty: &ValueType| matches!(ty, TypeOf::Optional { .. });
        match &m.form {
            TestMatcherForm::Omit
            | TestMatcherForm::AnyValueOrNone
            | TestMatcherForm::ValueList { .. }
            | TestMatcherForm::Complement { .. } => true,
            TestMatcherForm::Specific {
                value: TestValue::Param(name),
            } => self
                .template
                .parameter(name)
                .is_some_and(|p| optional(&p.parameter_type)),
            TestMatcherForm::TemplateRef { template, .. } => self
                .reference(template)
                .is_ok_and(|(_, referenced)| optional(&referenced.template_type)),
            TestMatcherForm::Specific { .. }
            | TestMatcherForm::RecordMatch { .. }
            | TestMatcherForm::ListMatch { .. }
            | TestMatcherForm::UnionMatch { .. }
            | TestMatcherForm::AnyValue
            | TestMatcherForm::ValueRange { .. } => false,
        }
    }

    /// Whether the value at `at` matches the form of `m`. At an optional
    /// position a matcher of the value's type requires the value present.
    fn form(&self, m: &TestMatcher, at: &Position<'_>) -> Result<String, ForgeError> {
        if at.optional() && !self.reads_absence(m) {
            let held = self.form(m, &at.present())?;
            return Ok(all(vec![format!("{}.has_value()", at.expr), held]));
        }
        Ok(match &m.form {
            TestMatcherForm::AnyValue | TestMatcherForm::AnyValueOrNone => "true".to_string(),
            TestMatcherForm::Omit => format!("!{}.has_value()", at.expr),
            TestMatcherForm::Specific { value } => {
                format!("({} == {})", at.expr, self.specific_value(value, at)?)
            }
            TestMatcherForm::RecordMatch { fields } => {
                let (record, payload) = self.record(at)?;
                let mut parts = Vec::new();
                for (id, ty) in &record.fields {
                    let matcher = fields
                        .iter()
                        .find(|f| f.field.as_deref() == Some(id.as_str()))
                        .ok_or_else(|| self.refused(format!("no matcher for the field `{id}`")))?;
                    let field = at.field(&payload, id, ty, &record.scope);
                    parts.push(self.matches(matcher, &field)?);
                }
                all(parts)
            }
            TestMatcherForm::ListMatch { elements } => {
                let elem = self.list_element(at)?;
                let mut parts = vec![format!("({}.size() == {}u)", at.expr, elements.len())];
                for (index, element) in elements.iter().enumerate() {
                    parts.push(self.matches(element, &at.element(index, &elem))?);
                }
                all(parts)
            }
            TestMatcherForm::ValueList { items } => any(items
                .iter()
                .map(|item| self.matches(item, at))
                .collect::<Result<_, _>>()?),
            TestMatcherForm::Complement { items } => format!(
                "!{}",
                any(items
                    .iter()
                    .map(|item| self.matches(item, at))
                    .collect::<Result<_, _>>()?)
            ),
            TestMatcherForm::ValueRange { lower, upper } => {
                let TypeOf::Scalar(scalar) = &at.ty else {
                    return Err(self.refused(format!("a value range over {}", at.ty)));
                };
                let (least, most) = scalar
                    .int_value_range()
                    .ok_or_else(|| self.refused(format!("a value range over {}", at.ty)))?;
                let mut parts = Vec::new();
                for (bound, op, limit) in [(lower, ">=", least), (upper, "<=", most)] {
                    let Some(bound) = bound else {
                        continue;
                    };
                    // A literal bound at the type's own limit holds every
                    // value, and its comparison is one a compiler warns is
                    // always true: it is left out, as a missing bound is.
                    if let TestValue::Value(text) = bound {
                        if crate::forge::test_profile::decimal_integer(text) == Some(limit) {
                            continue;
                        }
                    }
                    parts.push(format!(
                        "({} {op} {})",
                        at.expr,
                        self.specific_value(bound, at)?
                    ));
                }
                all(parts)
            }
            TestMatcherForm::TemplateRef { template, args } => {
                let (namespace, _) = self.reference(template)?;
                Self::call(namespace, "match", Some(at.expr.as_str()), args)
            }
            TestMatcherForm::UnionMatch { .. } => {
                return Err(self.refused(format!(
                    "a <sce:union-match> stands where a value of {} is matched",
                    at.ty
                )))
            }
        })
    }

    /// The type of the elements of the list at `at`.
    fn list_element(&self, at: &Position<'_>) -> Result<ValueType, ForgeError> {
        match &at.ty {
            TypeOf::List { elem, .. } => Ok((**elem).clone()),
            other => Err(self.refused(format!(
                "a <sce:list-match> stands where a value of {other} is matched"
            ))),
        }
    }

    /// The length restriction of `m` on the value at `at`
    /// (B.1.4.1): characters of a text, octets of a byte string, elements of a
    /// list. An absent optional value is held to none: whether it may be
    /// absent is the matcher's own question.
    fn length(&self, m: &TestMatcher, at: &Position<'_>) -> Result<String, ForgeError> {
        let present = at.present();
        let count = match &present.ty {
            TypeOf::Text { .. } => {
                self.counts_characters.set(true);
                format!("{TEXT_LENGTH}({})", present.expr)
            }
            TypeOf::Bytes { .. } | TypeOf::List { .. } => format!("{}.size()", present.expr),
            other => return Err(self.refused(format!("a length restriction on {other}"))),
        };
        let mut parts = Vec::new();
        if let Some(least) = m.length_min.filter(|least| *least > 0) {
            parts.push(format!("({count} >= {least}u)"));
        }
        if let Some(most) = m.length_max {
            parts.push(format!("({count} <= {most}u)"));
        }
        let held = all(parts);
        Ok(if at.optional() && held != "true" {
            format!("(!{}.has_value() || {held})", at.expr)
        } else {
            held
        })
    }

    /// A value a matcher names, at `at`: a formal parameter, or a literal of
    /// the type of the value the position holds.
    fn specific_value(&self, value: &TestValue, at: &Position<'_>) -> Result<String, ForgeError> {
        match value {
            TestValue::Param(name) => Ok(cpp_parameter(name)),
            TestValue::Value(text) => self.literal(text, &at.present()),
        }
    }

    /// The literal `text` of the type of `at` (docs/SCE_TEST_PROFILE.md §11.4):
    /// an integer in decimal, `true` or `false`, a variant of an enum, a text,
    /// or a byte string in hex.
    fn literal(&self, text: &str, at: &Position<'_>) -> Result<String, ForgeError> {
        let refused = || self.refused(format!("'{text}' is no literal of {}", at.ty));
        match &at.ty {
            TypeOf::Scalar(SceType::Bool) => match text.trim() {
                held @ ("true" | "false") => Ok(held.to_string()),
                _ => Err(refused()),
            },
            // C++ scopes a variant under its type, so the variant is named
            // through the type the position is held in, whatever document
            // declares the enum.
            TypeOf::Scalar(SceType::Enum(reference)) => Ok(format!(
                "{}::{}",
                at.cpp,
                crate::forge::enum_naming::variant_ident(
                    Language::Cpp,
                    &reference.alias,
                    text.trim()
                )
            )),
            TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
                integer_literal(text).ok_or_else(refused)
            }
            TypeOf::Text { .. } => Ok(filters::cpp_text_expression(text)),
            TypeOf::Bytes { .. } => {
                let octets: Vec<String> = text
                    .trim()
                    .as_bytes()
                    .chunks(2)
                    .map(|pair| pair.iter().map(|b| char::from(*b)).collect::<String>())
                    .map(|pair| format!("0x{pair}"))
                    .collect();
                Ok(format!("{}{{{}}}", at.cpp, octets.join(", ")))
            }
            _ => Err(refused()),
        }
    }

    /// The value the specific matcher `m` sends at `at`
    /// (docs/SCE_TEST_PROFILE.md §6.2), as an expression of a type the
    /// position's converts from: a record's fields in the order its schema
    /// declares them, a list's elements in order, an absent optional value,
    /// and a referenced template's own `value`.
    fn value(&self, m: &TestMatcher, at: &Position<'_>) -> Result<String, ForgeError> {
        Ok(match &m.form {
            TestMatcherForm::Omit => "std::nullopt".to_string(),
            TestMatcherForm::Specific { value } => self.specific_value(value, at)?,
            TestMatcherForm::RecordMatch { fields } => {
                let present = at.present();
                let (record, payload) = self.record(&present)?;
                let mut members = Vec::new();
                for (id, ty) in &record.fields {
                    let matcher = fields
                        .iter()
                        .find(|f| f.field.as_deref() == Some(id.as_str()))
                        .ok_or_else(|| self.refused(format!("no matcher for the field `{id}`")))?;
                    let field = present.field(&payload, id, ty, &record.scope);
                    members.push((
                        event_schema_field_ident(id, Language::Cpp),
                        self.value(matcher, &field)?,
                    ));
                }
                format!("{payload}{{{}}}", cpp_positional_init(&members))
            }
            TestMatcherForm::ListMatch { elements } => {
                let present = at.present();
                let elem = self.list_element(&present)?;
                let values: Vec<String> = elements
                    .iter()
                    .enumerate()
                    .map(|(index, element)| self.value(element, &present.element(index, &elem)))
                    .collect::<Result<_, _>>()?;
                format!("{}{{{}}}", present.cpp, values.join(", "))
            }
            TestMatcherForm::TemplateRef { template, args } => {
                let (namespace, _) = self.reference(template)?;
                Self::call(namespace, "value", None, args)
            }
            TestMatcherForm::UnionMatch { .. }
            | TestMatcherForm::ValueList { .. }
            | TestMatcherForm::Complement { .. }
            | TestMatcherForm::AnyValue
            | TestMatcherForm::AnyValueOrNone
            | TestMatcherForm::ValueRange { .. } => {
                return Err(self.refused(format!(
                    "<sce:{}> has no value to send: a specific template holds values alone",
                    m.form.element()
                )))
            }
        })
    }
}
