// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.2): `Value`, the type the template matches; `matches`, whether a
//! value matches it (ES 201 873-1 Annex B.1); and, for a specific template
//! (§6.2), `value`, the one value it sends. Python keeps a module's names case
//! apart, so the function is `value` beside the alias `Value`, as on Rust and
//! Kotlin.
//!
//! What each matcher holds is decided by the walk every backend shares
//! ([`super::test_template_walk`]), and its tests are joined by the join every
//! backend whose connectives bind as C's do shares ([`JoinSpelling`]), in
//! Python's words ([`Connectives::PYTHON`]: `and`, `or`, `not`, `True`,
//! `False`); this file spells the leaves in Python, and the template
//! (`forge/python/test_template.py.jinja2`) writes the declarations around the
//! two expressions.
//!
//! Every type is the one the schema unit spells ([`unit_type`]), and every
//! enum, record and referenced template is named through the module of the
//! document that declares it, under the alias every Python unit imports a
//! sibling module by ([`python_module_alias`]): an enum a record of another
//! document holds is named by the one declaration of it, whatever document
//! imports it and under whatever alias. The unit imports the module of each
//! document its text names and no other, as the Go unit does, so it reaches
//! no module a run need not have generated.
//!
//! A value that may be absent is held as the value itself or `None`
//! (`Optional[T]`), so it is tested by `is None` and read as it is, and built
//! as its value or `None`. A text, a byte string and a list compare by their
//! contents under `==`, and a byte string is immutable, so a value built from
//! one a parameter gives holds it as given. A length restriction on a text
//! counts its characters through the profile's runtime ([`TEXT_LENGTH`]),
//! which a Python text, a sequence of code points, counts by its `len`. Python
//! lowers no union ([`crate::forge::codegen_matrix::UNION_BACKENDS`]), so a
//! template whose type reaches one is refused by the walk before anything is
//! spelled.

use std::cell::RefCell;
use std::collections::BTreeSet;

use super::test_template_walk::{
    hex_octets, integer_literal, CaseTest, Connectives, Held, JoinSpelling, Op, Position, Stands,
    TemplateFunction, TemplateSpelling, TemplateWalk, Use,
};
use super::*;
use crate::forge::schema_unit_type::{unit_type, C11Declarations, UnitNames};
use crate::forge::value_type::{ScopedRecord, ScopedUnion, TypeOf, TypeScope, ValueType};
use crate::generator::Language;

/// The function of the test profile's Python runtime that counts the
/// characters of a text, which a length restriction on a `string[N]` counts:
/// its code points, as C++'s `SCE::Test::textLength` counts them.
const TEXT_LENGTH: &str = "text_length";

/// The import that brings [`TEXT_LENGTH`] into the unit, from the profile's
/// runtime (`sce_runtime.testprofile`).
const TEXT_LENGTH_IMPORT: &str = "from sce_runtime.testprofile import text_length";

/// The unit of the test template `m`, whose imports are `imports`.
pub(super) fn render_test_template_python(
    env: &minijinja::Environment,
    m: &TestTemplateModel,
    imports: &[ImportContext],
    options: &crate::ForgeCompileOptions,
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Python, imports);
    let scope = import_type_scope(imports);
    let spelling = PythonSpelling::default();
    let walk = TemplateWalk {
        template: m,
        imports,
        spelling: &spelling,
    };
    walk.refuse_unlowered_unions(&scope)?;
    walk.refuse_unread_enums(&scope)?;
    let names = PythonNames {
        scope: &scope,
        spelling: &spelling,
    };
    let spell = |ty: &ValueType| {
        unit_type(
            Language::Python,
            ty,
            &names,
            &mut C11Declarations::default(),
        )
        .map_err(|why| walk.refused(why))
    };
    let value_type = spell(&m.template_type)?;
    let mut types = vec![value_type.clone()];
    let mut parameters = Vec::new();
    for p in &m.parameters {
        let ty = spell(&p.parameter_type)?;
        types.push(ty.clone());
        parameters.push(serde_json::json!({
            "id": p.id,
            "name": python_parameter(&p.id),
            "ty": ty,
            "sce_type": p.parameter_type.to_string(),
        }));
    }
    let root = walk.root(&scope);
    let match_expr = spelling.join(&walk.matches(&m.matcher, &root)?, false, Stands::Whole);
    let value_expr = if walk.specific() {
        Some(walk.value(&m.matcher, &root)?)
    } else {
        None
    };
    let refusal = spelling.refusal.borrow_mut().take();
    if let Some(why) = refusal {
        return Err(walk.refused(why));
    }
    let mut expressions = vec![match_expr.clone()];
    expressions.extend(value_expr.iter().cloned());
    let expressions = expressions.join("\n");
    let text = format!("{}\n{expressions}", types.join("\n"));
    let python_imports = spelling.imports(&text, options);
    // `Optional` is named by a type alone, and a type holds no literal, so the
    // types are what is read for it.
    let needs_optional = types.iter().any(|ty| ty.contains("Optional["));
    let uses_text_length = names_in(&expressions, TEXT_LENGTH, '(');

    let mut ctx = l.base_context(&m.name);
    ctx.insert("template_name".into(), m.name.clone().into());
    ctx.insert("template_type".into(), m.template_type.to_string().into());
    ctx.insert("value_type".into(), value_type.into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("match_expr".into(), match_expr.into());
    ctx.insert("value_expr".into(), serde_json::json!(value_expr));
    ctx.insert("python_imports".into(), serde_json::json!(python_imports));
    ctx.insert("needs_optional".into(), needs_optional.into());
    ctx.insert(
        "text_length_import".into(),
        serde_json::json!(uses_text_length.then_some(TEXT_LENGTH_IMPORT)),
    );
    l.render(env, "test_template", ctx)
}

/// The name the generated `matches` and `value` take the formal parameter `id`
/// by: `param_<id>`, so that no parameter's name is one the unit declares
/// (`Value`, `matches`, `value`), the name `matches` gives the value it is
/// handed, a module the unit imports (each document's under the `sce_` prefix),
/// the runtime's function it calls ([`TEXT_LENGTH`]), a built-in it calls
/// (`len`) or a keyword of the language.
fn python_parameter(id: &str) -> String {
    format!("param_{id}")
}

/// Whether `text` names `name` as Python reads a name: at an occurrence that is
/// not the tail of a longer name or a member after a `.`, followed by `then`
/// (`.` for a module whose member an expression reads, `(` for a function it
/// calls). A text literal that holds the same characters reads as a name too,
/// which costs the unit an import of a module the run generated, and nothing
/// more.
fn names_in(text: &str, name: &str, then: char) -> bool {
    text.match_indices(name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + name.len()..].chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') && after == Some(then)
    })
}

/// What a Python template's unit names from outside it, read in one scope
/// ([`UnitNames`]): every enum and record by the module of its document, which
/// the spelling notes as one the unit may import.
struct PythonNames<'s> {
    scope: &'s TypeScope,
    spelling: &'s PythonSpelling,
}

impl UnitNames for PythonNames<'_> {
    // An enum the scope does not read is refused before a type is spelled
    // (`TemplateWalk::refuse_unread_enums`); its alias stands in its place
    // here.
    fn scalar(&self, ty: &SceType) -> String {
        match ty {
            SceType::Enum(reference) => match self.scope.enums.get(&reference.alias) {
                Some(declared) => {
                    self.spelling.names_document(&declared.name);
                    enum_qualified_type(&declared.name, Language::Python)
                }
                None => reference.alias.clone(),
            },
            other => python_type(other).to_string(),
        }
    }
    fn record(&self, alias: &str) -> Option<String> {
        self.scope.records.get(alias).map(|record| {
            self.spelling.names_document(&record.name);
            event_schema_payload_type(&record.name, Language::Python)
        })
    }
    // Python lowers no union: `unit_type` refuses one by name before it asks.
    fn union(&self, _alias: &str) -> Option<String> {
        None
    }
    // No Python type is named by a mangled composition.
    fn enum_mangle(&self, alias: &str) -> String {
        alias.to_string()
    }
}

/// One position of the value a matcher stands at, in Python: the expression a
/// test reads it by.
#[derive(Clone)]
struct PythonAt {
    expr: String,
}

/// The Python spelling of a template's unit.
#[derive(Default)]
struct PythonSpelling {
    /// Every document whose declaration a spelling named: the modules the unit
    /// may import, of which it imports those its text names.
    named: RefCell<BTreeSet<String>>,
    /// The first refusal a spelling met where the walk asks for a text and not
    /// for a result (an absent value where the position may hold none), which
    /// the unit is refused by once the walk is done.
    refusal: RefCell<Option<String>>,
}

impl PythonSpelling {
    /// The unit names a declaration of the document `document`.
    fn names_document(&self, document: &str) {
        self.named.borrow_mut().insert(document.to_string());
    }

    /// Keeps `why` as the unit's refusal, unless one was kept before it.
    fn refuse(&self, why: String) {
        let mut kept = self.refusal.borrow_mut();
        if kept.is_none() {
            *kept = Some(why);
        }
    }

    /// The import statements of the unit whose text is `text`: the module of
    /// each document a spelling named, under the alias every Python unit
    /// imports it by ([`forge_import_identity`]), kept only where the text
    /// names that alias.
    fn imports(&self, text: &str, options: &crate::ForgeCompileOptions) -> Vec<String> {
        self.named
            .borrow()
            .iter()
            .filter(|document| {
                let alias = python_module_alias(&filters::to_snake_case(document.to_string()));
                names_in(text, &alias, '.')
            })
            .map(|document| {
                forge_import_identity(document, &Language::Python, false, options).include_stmt
            })
            .collect()
    }
}

impl JoinSpelling for PythonSpelling {
    fn connectives(&self) -> Connectives {
        Connectives::PYTHON
    }

    /// A `bool` is compared with a literal by reading it; every other value,
    /// one that may be absent and a byte string among them, by `==`, which
    /// compares Python's values by their contents and holds two absent values
    /// equal.
    fn compare(&self, at: &Position<'_, PythonAt>, op: Op, negated: bool, rhs: &str) -> String {
        let place = &at.at.expr;
        match (&at.ty, op) {
            (TypeOf::Scalar(SceType::Bool), Op::Eq) if rhs == "True" || rhs == "False" => self
                .connectives()
                .negate(place.clone(), (rhs == "True") == negated),
            _ => format!("{place} {} {rhs}", op.spelled(negated)),
        }
    }

    /// A text counts its characters, through the profile's runtime
    /// ([`TEXT_LENGTH`]); a byte string and a list their `len`.
    fn count(&self, at: &Position<'_, PythonAt>, op: Op, negated: bool, n: u32) -> String {
        let operator = op.spelled(negated);
        let place = &at.at.expr;
        match &at.ty {
            TypeOf::Text { .. } => format!("{TEXT_LENGTH}({place}) {operator} {n}"),
            _ => format!("len({place}) {operator} {n}"),
        }
    }

    fn absence(&self, at: &PythonAt, negated: bool) -> String {
        if negated {
            format!("{} is not None", at.expr)
        } else {
            format!("{} is None", at.expr)
        }
    }

    /// The optional's test of presence, joined with the test of the value it
    /// holds, which reads it as it is: the test before it in the same
    /// conjunction or disjunction proved it present. In parentheses where it is
    /// an operand of the other connective.
    fn bound_optional(
        &self,
        at: &PythonAt,
        _inner: &PythonAt,
        then: &Held<'_, PythonAt>,
        absent_holds: bool,
        negated: bool,
        stands: Stands,
    ) -> String {
        let (test, nested) = if absent_holds {
            (
                format!(
                    "{} is None or {}",
                    at.expr,
                    self.join(then, negated, Stands::Disjunct)
                ),
                matches!(stands, Stands::Conjunct),
            )
        } else {
            (
                format!(
                    "{} is not None and {}",
                    at.expr,
                    self.join(then, negated, Stands::Conjunct)
                ),
                matches!(stands, Stands::Disjunct),
            )
        };
        if nested {
            format!("({test})")
        } else {
            test
        }
    }

    /// Never asked: Python lowers no union, and the walk refuses a
    /// `union-match` on a backend that lowers none before it builds the test of
    /// a case (`TemplateWalk::union_case`).
    fn case_test(
        &self,
        _at: &PythonAt,
        union: &ScopedUnion,
        _variant: &str,
        _record: CaseTest<'_, '_, PythonAt>,
        _negated: bool,
    ) -> String {
        unreachable!(
            "the walk refuses the union `{}` on Python before it asks for the test of a case",
            union.name
        )
    }
}

impl TemplateSpelling for PythonSpelling {
    type At = PythonAt;

    fn language(&self) -> Language {
        Language::Python
    }

    /// `matches`'s parameter `value`, of type `Value`.
    fn root(&self) -> PythonAt {
        PythonAt {
            expr: "value".to_string(),
        }
    }

    /// The optional's value, which is the optional itself where a test before
    /// it proved it is not `None`.
    fn present(&self, at: &PythonAt) -> PythonAt {
        at.clone()
    }

    fn element(&self, at: &PythonAt, index: usize) -> PythonAt {
        PythonAt {
            expr: format!("{}[{index}]", at.expr),
        }
    }

    fn field(&self, at: &PythonAt, _record: &ScopedRecord, id: &str, _ty: &ValueType) -> PythonAt {
        PythonAt {
            expr: format!(
                "{}.{}",
                at.expr,
                event_schema_field_ident(id, Language::Python)
            ),
        }
    }

    /// The parameter as it is: a byte string is immutable, so a value built
    /// from one holds no array its caller can write into.
    fn parameter(
        &self,
        name: &str,
        _ty: &ValueType,
        _at: &Position<'_, PythonAt>,
        _purpose: Use,
    ) -> String {
        python_parameter(name)
    }

    /// An integer in decimal, `True` or `False`, a member of an enum through the
    /// module of its document, a text as a string literal, or a byte string as
    /// the bytes literal of its octets: one spelling compares and builds.
    fn literal(
        &self,
        text: &str,
        at: &Position<'_, PythonAt>,
        _purpose: Use,
    ) -> Result<String, String> {
        let refused = || format!("'{text}' is no literal of {}", at.ty);
        match &at.ty {
            TypeOf::Scalar(SceType::Bool) => match text.trim() {
                "true" => Ok("True".to_string()),
                "false" => Ok("False".to_string()),
                _ => Err(refused()),
            },
            TypeOf::Scalar(SceType::Enum(reference)) => {
                let declared = at
                    .scope
                    .enums
                    .get(&reference.alias)
                    .ok_or_else(|| format!("`enum:{}` names no enum read", reference.alias))?;
                self.names_document(&declared.name);
                Ok(crate::forge::enum_naming::variant_ref(
                    Language::Python,
                    &enum_qualified_type(&declared.name, Language::Python),
                    &declared.name,
                    text.trim(),
                ))
            }
            TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
                integer_literal(Language::Python, text).ok_or_else(refused)
            }
            TypeOf::Text { .. } => Ok(filters::py_string_literal(text.to_string())),
            TypeOf::Bytes { .. } => {
                let octets: String = hex_octets(text)
                    .ok_or_else(refused)?
                    .into_iter()
                    .map(|pair| format!("\\x{pair}"))
                    .collect();
                Ok(format!("b\"{octets}\""))
            }
            _ => Err(refused()),
        }
    }

    fn call(
        &self,
        import: &ImportContext,
        function: TemplateFunction,
        first: Option<&PythonAt>,
        args: &[String],
    ) -> String {
        let function = match function {
            TemplateFunction::Match => "matches",
            TemplateFunction::Value => "value",
        };
        let arguments: Vec<String> = first
            .map(|at| at.expr.clone())
            .into_iter()
            .chain(args.iter().map(|arg| python_parameter(arg)))
            .collect();
        self.names_document(&import.document_name);
        format!(
            "{}.{function}({})",
            python_module_alias(&filters::to_snake_case(import.document_name.clone())),
            arguments.join(", ")
        )
    }

    /// `None`, the absent value of every optional position.
    fn absent(&self, at: &Position<'_, PythonAt>) -> String {
        if !at.optional() {
            self.refuse(format!(
                "an <sce:omit> stands where a value of {} is sent",
                at.ty
            ));
        }
        "None".to_string()
    }

    /// The value as it is: an optional position holds its present value
    /// itself.
    fn present_value(&self, _at: &Position<'_, PythonAt>, value: String) -> String {
        value
    }

    /// The record's payload class, each field given by its keyword.
    fn record_value(
        &self,
        _at: &Position<'_, PythonAt>,
        record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        self.names_document(&record.name);
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| {
                format!(
                    "{}={value}",
                    event_schema_field_ident(&id, Language::Python)
                )
            })
            .collect();
        format!(
            "{}({})",
            event_schema_payload_type(&record.name, Language::Python),
            fields.join(", ")
        )
    }

    /// The list of the elements.
    fn list_value(&self, _at: &Position<'_, PythonAt>, elements: Vec<String>) -> String {
        format!("[{}]", elements.join(", "))
    }
}
