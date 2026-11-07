// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §12.2): `Value`, the type the template matches; `match`, whether a value
//! matches it (ES 201 873-1 Annex B.1); and, for a specific template (§6.2),
//! `value`, the one value it sends.
//!
//! What each matcher holds is decided by the walk every backend shares
//! ([`super::test_template_walk`]); this file spells it in C++, one expression
//! per matcher and the expression of a matcher made of its children's, and the
//! template (`forge/cpp/test_template.h.jinja2`) writes the declarations around
//! the two expressions.
//!
//! Every type is the one the schema unit spells. The template's own type and
//! its parameters' are spelled by [`unit_type`], as a field of that type is.
//! A position below a record's field is read through the type the record's
//! unit declares for the field (`decltype(<Record>Payload::<field>)`), and
//! through `value_type` below an optional or a list, so an enum a nested record
//! holds is named by the one declaration of it, whatever document imports it
//! and under whatever alias.

use super::test_template_walk::{
    integer_literal, Held, Position, TemplateFunction, TemplateSpelling, TemplateWalk, Use,
};
use super::*;
use crate::forge::schema_unit_type::{unit_type, C11Declarations};
use crate::forge::value_type::{ScopedRecord, TypeOf, ValueType};
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
    let spelling = CppSpelling {
        counts_characters: std::cell::Cell::new(false),
    };
    let walk = TemplateWalk {
        template: m,
        imports,
        spelling: &spelling,
    };
    walk.refuse_unlowered_unions(&scope)?;
    let names = SchemaUnitNames { l: &l, imports };
    let spell = |ty: &ValueType| {
        unit_type(Language::Cpp, ty, &names, &mut C11Declarations::default())
            .map_err(|why| walk.refused(why))
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
    let root = walk.root(&scope);
    let match_expr = spelling.join(&walk, &walk.matches(&m.matcher, &root)?)?;
    let value_expr = if walk.specific() {
        Some(walk.value(&m.matcher, &root)?)
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
        spelling.counts_characters.get().into(),
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

/// An integer literal of the profile as C++ writes it
/// ([`integer_literal`]): what a suite's module parameter default is written
/// by too.
pub(super) fn cpp_integer_literal(text: &str) -> Option<String> {
    integer_literal(Language::Cpp, text)
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

/// One position of the value a matcher stands at, in C++.
#[derive(Clone)]
struct CppAt {
    /// The C++ expression of the value at the position, in `match`.
    expr: String,
    /// The C++ type the value at the position is held in, as the unit of its
    /// schema declares it.
    cpp: String,
}

/// The C++ spelling of a template's unit.
struct CppSpelling {
    /// Whether a matcher counts the characters of a text, which the runtime's
    /// [`TEXT_LENGTH_HEADER`] declares.
    counts_characters: std::cell::Cell<bool>,
}

impl CppSpelling {
    /// `held` as one C++ expression, every operand of a join in parentheses.
    fn join(
        &self,
        walk: &TemplateWalk<'_, Self>,
        held: &Held<'_, CppAt>,
    ) -> Result<String, ForgeError> {
        let joined = |parts: &[Held<'_, CppAt>]| {
            parts
                .iter()
                .map(|part| self.join(walk, part))
                .collect::<Result<Vec<_>, _>>()
        };
        Ok(match held {
            Held::True => "true".to_string(),
            Held::Compare { at, op, rhs } => format!("({} {} {rhs})", at.at.expr, op.symbol()),
            Held::Count { at, op, n } => format!("({} {} {n}u)", self.count(at), op.symbol()),
            Held::Call(call) => call.clone(),
            Held::Absent(at) => format!("!{}.has_value()", at.at.expr),
            Held::Present { at, then, .. } => all(vec![
                format!("{}.has_value()", at.at.expr),
                self.join(walk, then)?,
            ]),
            Held::AbsentOr { at, then, .. } => format!(
                "(!{}.has_value() || {})",
                at.at.expr,
                self.join(walk, then)?
            ),
            Held::All(parts) => all(joined(parts.as_slice())?),
            Held::Any(parts) => any(joined(parts.as_slice())?),
            Held::Not(part) => format!("!{}", self.join(walk, part)?),
            Held::Case { at, .. } => {
                return Err(walk.refused(format!(
                    "a <sce:union-match> stands where a value of {} is matched",
                    at.ty
                )))
            }
        })
    }

    /// What a length restriction counts at `at`: the characters of a text,
    /// through the runtime's [`TEXT_LENGTH`], and the octets or the elements
    /// of anything else the walk counts.
    fn count(&self, at: &Position<'_, CppAt>) -> String {
        match &at.ty {
            TypeOf::Text { .. } => {
                self.counts_characters.set(true);
                format!("{TEXT_LENGTH}({})", at.at.expr)
            }
            _ => format!("{}.size()", at.at.expr),
        }
    }
}

impl TemplateSpelling for CppSpelling {
    type At = CppAt;

    fn language(&self) -> Language {
        Language::Cpp
    }

    /// `match`'s parameter `value`, of type `Value`.
    fn root(&self) -> CppAt {
        CppAt {
            expr: "value".to_string(),
            cpp: "Value".to_string(),
        }
    }

    fn present(&self, at: &CppAt) -> CppAt {
        CppAt {
            expr: format!("(*{})", at.expr),
            cpp: format!("{}::value_type", at.cpp),
        }
    }

    fn element(&self, at: &CppAt, index: usize) -> CppAt {
        CppAt {
            expr: format!("{}[{index}]", at.expr),
            cpp: format!("{}::value_type", at.cpp),
        }
    }

    fn field(&self, at: &CppAt, record: &ScopedRecord, id: &str, _ty: &ValueType) -> CppAt {
        let member = event_schema_field_ident(id, Language::Cpp);
        let payload = event_schema_payload_type(&record.name, Language::Cpp);
        CppAt {
            expr: format!("{}.{member}", at.expr),
            cpp: format!("decltype({payload}::{member})"),
        }
    }

    fn parameter(
        &self,
        name: &str,
        _ty: &ValueType,
        _at: &Position<'_, CppAt>,
        _purpose: Use,
    ) -> String {
        cpp_parameter(name)
    }

    /// An integer in decimal, `true` or `false`, a variant of an enum, a
    /// text, or a byte string in hex: one spelling compares and builds.
    fn literal(
        &self,
        text: &str,
        at: &Position<'_, CppAt>,
        _purpose: Use,
    ) -> Result<String, String> {
        let refused = || format!("'{text}' is no literal of {}", at.ty);
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
                at.at.cpp,
                crate::forge::enum_naming::variant_ident(
                    Language::Cpp,
                    &reference.alias,
                    text.trim()
                )
            )),
            TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
                cpp_integer_literal(text).ok_or_else(refused)
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
                Ok(format!("{}{{{}}}", at.at.cpp, octets.join(", ")))
            }
            _ => Err(refused()),
        }
    }

    fn call(
        &self,
        import: &ImportContext,
        function: TemplateFunction,
        first: Option<&CppAt>,
        args: &[String],
    ) -> String {
        let function = match function {
            TemplateFunction::Match => "match",
            TemplateFunction::Value => "value",
        };
        let arguments: Vec<String> = first
            .map(|at| at.expr.clone())
            .into_iter()
            .chain(args.iter().map(|arg| cpp_parameter(arg)))
            .collect();
        format!("{}::{function}({})", import.namespace, arguments.join(", "))
    }

    fn absent(&self, _at: &Position<'_, CppAt>) -> String {
        "std::nullopt".to_string()
    }

    /// As it is: a `std::optional<T>` converts from a value of `T`.
    fn present_value(&self, _at: &Position<'_, CppAt>, value: String) -> String {
        value
    }

    fn record_value(
        &self,
        _at: &Position<'_, CppAt>,
        record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        let payload = event_schema_payload_type(&record.name, Language::Cpp);
        let members: Vec<(String, String)> = fields
            .into_iter()
            .map(|(id, value)| (event_schema_field_ident(&id, Language::Cpp), value))
            .collect();
        format!("{payload}{{{}}}", cpp_positional_init(&members))
    }

    fn list_value(&self, at: &Position<'_, CppAt>, elements: Vec<String>) -> String {
        format!("{}{{{}}}", at.at.cpp, elements.join(", "))
    }
}
