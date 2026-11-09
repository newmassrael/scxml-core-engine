// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Go unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.2): `Value`, the type the template matches; `Matches`, whether a
//! value matches it (ES 201 873-1 Annex B.1); and, for a specific template
//! (§6.2), `Specific`, the one value it sends. Go keeps a package's types and
//! functions in one namespace, so the function a specific template's value is
//! built by cannot be `Value` beside the type, as it is `value` on the other
//! backends. A template over a record, the type of a message, declares too the
//! name of its record type in the test log, `TYPE`, and `Canonical`, its
//! value's canonical form there (§12.5, [`GoCanonical`]).
//!
//! What each matcher holds is decided by the walk every backend shares
//! ([`super::test_template_walk`]), and its tests are joined as every backend
//! whose connectives are C's joins them ([`JoinSpelling`]); this file spells
//! the leaves in Go, and the template (`forge/go/test_template.go.jinja2`)
//! writes the declarations around the two expressions.
//!
//! Every type is the one the schema unit spells ([`unit_type`]), and every
//! enum, record and referenced template is named through the package of the
//! document that declares it, under the alias every Go unit imports a sibling
//! package by ([`go_package_alias`]): an enum a record of another document
//! holds is named by the one declaration of it, whatever document imports it
//! and under whatever alias. A Go file must import each package it names and
//! no other, so the unit's imports are the packages its text names, read off
//! the text ([`crate::std_imports::go_imports_named_in`]) among the documents
//! a spelling named.
//!
//! A value that may be absent is the forge runtime's `optional.Of[T]`, tested
//! by `IsPresent` and read by `Proven` where the test before it proved it
//! present; built, it names its type (`optional.Some[T]`, `optional.None[T]`),
//! since Go infers no type of an untyped constant's optional. A byte string is
//! compared by its octets (`bytes.Equal`), since Go's `==` does not compare
//! slices, and a value built from a byte string a parameter gives holds a copy
//! of it. A length restriction on a text counts its characters through the
//! profile's runtime ([`TEXT_LENGTH`]). Go lowers no union
//! ([`crate::forge::codegen_matrix::UNION_BACKENDS`]), so a template whose type
//! reaches one is refused by the walk before anything is spelled.

use std::cell::RefCell;
use std::collections::BTreeSet;

use super::test_template_walk::{
    hex_octets, integer_literal, negate, CanonicalSpelling, CaseTest, Held, JoinSpelling, Op,
    Position, Stands, TemplateFunction, TemplateSpelling, TemplateWalk, Use,
};
use super::*;
use crate::forge::schema_unit_type::{unit_type, C11Declarations, UnitNames};
use crate::forge::value_type::{ScopedRecord, ScopedUnion, TypeOf, TypeScope, ValueType};
use crate::generator::Language;

/// The package of the test profile's Go runtime, which declares the length of
/// a text in characters.
const TEST_RUNTIME_PATH: &str = crate::std_imports::GO_TEST_RUNTIME_PATH;

/// The function of the test profile's Go runtime that counts the characters of
/// a text, which a length restriction on a `string[N]` counts: its code points,
/// as C++'s `SCE::Test::textLength` counts them.
const TEXT_LENGTH: &str = "testprofile.TextLength";

/// The standard package a byte string is compared and copied through.
const BYTES_PACKAGE: &str = "bytes";

/// The unit of the test template `m`, whose imports are `imports`. The
/// packages of the documents it names are imported under `options`'
/// `go_module_prefix`, which a template that names one must be given.
pub(super) fn render_test_template_go(
    env: &minijinja::Environment,
    m: &TestTemplateModel,
    imports: &[ImportContext],
    options: &crate::ForgeCompileOptions,
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Go, imports);
    let scope = import_type_scope(imports);
    let spelling = GoSpelling::default();
    let walk = TemplateWalk {
        template: m,
        imports,
        spelling: &spelling,
    };
    walk.refuse_unlowered_unions(&scope)?;
    walk.refuse_unread_enums(&scope)?;
    let names = GoNames {
        scope: &scope,
        spelling: &spelling,
    };
    let spell = |ty: &ValueType| {
        unit_type(Language::Go, ty, &names, &mut C11Declarations::default())
            .map_err(|why| walk.refused(why))
    };
    let value_type = spell(&m.template_type)?;
    let mut spelled = vec![value_type.clone()];
    let mut parameters = Vec::new();
    for p in &m.parameters {
        let ty = spell(&p.parameter_type)?;
        spelled.push(ty.clone());
        parameters.push(serde_json::json!({
            "id": p.id,
            "name": go_parameter(&p.id),
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
    // Only a template over a record, the type of a message, writes a message's
    // type name and its canonical form (`TemplateWalk::message_form`).
    let (log_type, canonical_expr) = walk.message_form(&GoCanonical, &scope)?.unzip();
    spelled.push(match_expr.clone());
    spelled.extend(value_expr.iter().cloned());
    spelled.extend(canonical_expr.iter().cloned());
    let go_imports = spelling.imports(&spelled.join("\n"), options)?;

    let mut ctx = l.base_context(&m.name);
    ctx.insert("template_name".into(), m.name.clone().into());
    ctx.insert("template_type".into(), m.template_type.to_string().into());
    ctx.insert("value_type".into(), value_type.into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("match_expr".into(), match_expr.into());
    ctx.insert("value_expr".into(), serde_json::json!(value_expr));
    ctx.insert("canonical_expr".into(), serde_json::json!(canonical_expr));
    ctx.insert("log_type".into(), serde_json::json!(log_type));
    ctx.insert("go_imports".into(), serde_json::json!(go_imports));
    l.render(env, "test_template", ctx)
}

/// The package of the test profile's Go runtime whose functions write the
/// canonical form of a value in the test log
/// (`backends/go/runtime/testprofile/canonical/canonical.go`), imported under
/// its own name, [`CANONICAL`].
const CANONICAL_PATH: &str = "github.com/newmassrael/sce-go-runtime/testprofile/canonical";

/// The name a unit names the writers of [`CANONICAL_PATH`] by.
const CANONICAL: &str = "canonical";

/// A value the canonical writer reads, in Go.
struct GoCanonicalAt {
    /// What names the value: `value`, an element of a list by the index a
    /// closure binds, the value of an optional field, or a field of any of
    /// them.
    expr: String,
    /// How many bindings the value stands inside: the next one's number.
    depth: usize,
}

/// The Go spelling of a value's canonical form in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), with the writers of the profile runtime's
/// package `canonical`: a `string`, a `bool`, a `[]any` or a
/// `*sce.JSONObject` of them, which the runtime's `sce.WriteJCS` writes. The
/// walk is the one every backend shares (`test_template_walk::canonical_value`,
/// through `TemplateWalk::message_form`).
///
/// A Go function literal names the type of each parameter, which the walk does
/// not hand a spelling, so a closure binds no value of the record: a list is
/// written through a closure over the index of each element
/// (`CanonicalSpelling::list_element`), and an optional field through a
/// closure of no parameter that reads the field's value where it is present
/// (`CanonicalSpelling::optional_value`), which the runtime calls only there,
/// since what an absent field holds is no value of its type.
struct GoCanonical;

impl CanonicalSpelling for GoCanonical {
    type At = GoCanonicalAt;

    /// `Canonical`'s parameter `value`.
    fn root(&self) -> GoCanonicalAt {
        GoCanonicalAt {
            expr: "value".to_string(),
            depth: 0,
        }
    }

    fn field(&self, at: &GoCanonicalAt, id: &str) -> GoCanonicalAt {
        GoCanonicalAt {
            expr: format!("{}.{}", at.expr, event_schema_field_ident(id, Language::Go)),
            depth: at.depth,
        }
    }

    /// The name a closure binds, numbered by depth: the index of a list's
    /// element, or, for an optional field, a name nothing binds, which only
    /// numbers the bindings inside it.
    fn bound(&self, at: &GoCanonicalAt, prefix: &str) -> (String, GoCanonicalAt) {
        let name = format!("{prefix}_{}", at.depth);
        let value = GoCanonicalAt {
            expr: name.clone(),
            depth: at.depth + 1,
        };
        (name, value)
    }

    /// The list indexed by the closure's parameter.
    fn list_element(&self, list: &GoCanonicalAt, bound: GoCanonicalAt) -> GoCanonicalAt {
        GoCanonicalAt {
            expr: format!("{}[{}]", list.expr, bound.expr),
            depth: bound.depth,
        }
    }

    /// The field's value, read by `Proven` where the closure runs: only for a
    /// field that holds one.
    fn optional_value(&self, field: &GoCanonicalAt, bound: GoCanonicalAt) -> GoCanonicalAt {
        GoCanonicalAt {
            expr: format!("{}.Proven()", field.expr),
            depth: bound.depth,
        }
    }

    fn boolean(&self, at: &GoCanonicalAt) -> String {
        format!("{CANONICAL}.Boolean({})", at.expr)
    }

    /// A Go value of an enum's type answers its declared name, or none, by
    /// `DeclaredName`; a number no variant of an open enum declares is the
    /// text of its carrier's digits.
    fn enumerated(&self, at: &GoCanonicalAt, strict: bool) -> String {
        if strict {
            format!("{CANONICAL}.Enumerated({}.DeclaredName())", at.expr)
        } else {
            format!(
                "{CANONICAL}.OpenEnumerated({0}.DeclaredName(), {0}.ToUnderlying())",
                at.expr
            )
        }
    }

    fn integer(&self, at: &GoCanonicalAt) -> String {
        format!("{CANONICAL}.Integer({})", at.expr)
    }

    fn text(&self, at: &GoCanonicalAt) -> String {
        format!("{CANONICAL}.Text({})", at.expr)
    }

    fn bytes(&self, at: &GoCanonicalAt) -> String {
        format!("{CANONICAL}.Bytes({})", at.expr)
    }

    fn list(&self, at: &GoCanonicalAt, binding: &str, element: String) -> String {
        format!(
            "{CANONICAL}.List(len({}), func({binding} int) any {{ return {element} }})",
            at.expr
        )
    }

    /// As it is: a field a record always holds is never nil.
    fn held_field(&self, written: String) -> String {
        written
    }

    /// Nil where the field is absent, which the record omits.
    fn optional_field(&self, field: &GoCanonicalAt, _binding: &str, written: String) -> String {
        format!(
            "{CANONICAL}.Optional({}.IsPresent(), func() any {{ return {written} }})",
            field.expr
        )
    }

    fn record(&self, fields: Vec<(String, String)>) -> String {
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| {
                format!("{CANONICAL}.Field(\"{}\", {value})", filters::escape_go(id))
            })
            .collect();
        format!("{CANONICAL}.Record({})", fields.join(", "))
    }
}

/// The name the generated `Matches` and `Specific` take the formal parameter
/// `id` by: `param_<id>`, so that no parameter's name is one the unit declares
/// (`Value`, `Matches`, `Specific`), the name `Matches` gives the value it is
/// handed, a package the unit imports (each document's under the `sce_`
/// prefix, `bytes`, `optional`, `testprofile`) or a keyword of the language.
fn go_parameter(id: &str) -> String {
    format!("param_{id}")
}

/// What a Go template's unit names from outside it, read in one scope
/// ([`UnitNames`]): every enum and record by the package of its document,
/// which the spelling notes as one the unit may import.
struct GoNames<'s> {
    scope: &'s TypeScope,
    spelling: &'s GoSpelling,
}

impl UnitNames for GoNames<'_> {
    // An enum the scope does not read is refused before a type is spelled
    // (`TemplateWalk::refuse_unread_enums`); its alias stands in its place
    // here.
    fn scalar(&self, ty: &SceType) -> String {
        match ty {
            SceType::Enum(reference) => match self.scope.enums.get(&reference.alias) {
                Some(declared) => {
                    self.spelling.names_document(&declared.name);
                    enum_qualified_type(&declared.name, Language::Go)
                }
                None => reference.alias.clone(),
            },
            other => go_type(other).to_string(),
        }
    }
    fn record(&self, alias: &str) -> Option<String> {
        self.scope.records.get(alias).map(|record| {
            self.spelling.names_document(&record.name);
            event_schema_payload_type(&record.name, Language::Go)
        })
    }
    // Go lowers no union: `unit_type` refuses one by name before it asks.
    fn union(&self, _alias: &str) -> Option<String> {
        None
    }
    // No Go type is named by a mangled composition.
    fn enum_mangle(&self, alias: &str) -> String {
        alias.to_string()
    }
}

/// One position of the value a matcher stands at, in Go: the expression a
/// test reads it by.
#[derive(Clone)]
struct GoAt {
    expr: String,
}

/// The Go spelling of a template's unit.
#[derive(Default)]
struct GoSpelling {
    /// Every document whose declaration a spelling named: the packages the
    /// unit may import, of which it imports those its text names.
    named: RefCell<BTreeSet<String>>,
    /// The first refusal a spelling met where the walk asks for a text and
    /// not for a result (a type an absent value, a present one or a list
    /// names), which the unit is refused by once the walk is done.
    refusal: RefCell<Option<String>>,
}

impl GoSpelling {
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

    /// The Go type of a value of `ty`, read in `scope`; empty, with the
    /// refusal kept, where the type names what the scope does not read.
    fn type_of(&self, ty: &ValueType, scope: &TypeScope) -> String {
        let names = GoNames {
            scope,
            spelling: self,
        };
        unit_type(Language::Go, ty, &names, &mut C11Declarations::default()).unwrap_or_else(|why| {
            self.refuse(why);
            String::new()
        })
    }

    /// The import specs of the unit whose text is `text`: the standard
    /// `bytes`, the profile's runtime, its writers of the canonical form
    /// ([`CANONICAL_PATH`]) and the package of each document a spelling
    /// named, each kept only where the text names it, since Go
    /// refuses a file that imports a package it never names. A document's
    /// package is the one every Go unit imports it by
    /// ([`forge_import_identity`]), below `options`' `go_module_prefix`, which
    /// is refused by name where it is missing and a document is named.
    fn imports(
        &self,
        text: &str,
        options: &crate::ForgeCompileOptions,
    ) -> Result<Vec<String>, ForgeError> {
        let mut candidates = vec![
            format!("\"{BYTES_PACKAGE}\""),
            format!("\"{TEST_RUNTIME_PATH}\""),
            format!("\"{CANONICAL_PATH}\""),
        ];
        let named = self.named.borrow();
        if !named.is_empty() {
            go_import_root(options.go_module_prefix.as_deref())?;
        }
        candidates.extend(named.iter().map(|document| {
            forge_import_identity(document, &Language::Go, false, options)
                .include_stmt
                .trim()
                .to_string()
        }));
        Ok(crate::std_imports::go_imports_named_in(text, &candidates))
    }
}

impl JoinSpelling for GoSpelling {
    /// A `bool` is compared with a literal by reading it, a value that may be
    /// absent by `Equal`, which holds two absent values equal, and a byte
    /// string by its octets.
    fn compare(&self, at: &Position<'_, GoAt>, op: Op, negated: bool, rhs: &str) -> String {
        let place = &at.at.expr;
        match (&at.ty, op) {
            (TypeOf::Scalar(SceType::Bool), Op::Eq) if rhs == "true" || rhs == "false" => {
                negate(place.clone(), (rhs == "true") == negated)
            }
            (TypeOf::Optional { .. }, Op::Eq) => negate(format!("{place}.Equal({rhs})"), negated),
            (TypeOf::Bytes { .. }, Op::Eq) => {
                negate(format!("{BYTES_PACKAGE}.Equal({place}, {rhs})"), negated)
            }
            _ => format!("{place} {} {rhs}", op.spelled(negated)),
        }
    }

    /// A text counts its characters, through the profile's runtime
    /// ([`TEXT_LENGTH`]); a byte string and a list their `len`.
    fn count(&self, at: &Position<'_, GoAt>, op: Op, negated: bool, n: u32) -> String {
        let operator = op.spelled(negated);
        let place = &at.at.expr;
        match &at.ty {
            TypeOf::Text { .. } => format!("{TEXT_LENGTH}({place}) {operator} {n}"),
            _ => format!("len({place}) {operator} {n}"),
        }
    }

    fn absence(&self, at: &GoAt, negated: bool) -> String {
        if negated {
            format!("{}.IsPresent()", at.expr)
        } else {
            format!("!{}.IsPresent()", at.expr)
        }
    }

    /// The optional's test of presence, joined with the test of the value it
    /// holds, which reads it by `Proven` ([`TemplateSpelling::present`]): the
    /// test before it in the same conjunction or disjunction proved it present.
    /// In parentheses where it is an operand of the other connective.
    fn bound_optional(
        &self,
        at: &GoAt,
        _inner: &GoAt,
        then: &Held<'_, GoAt>,
        absent_holds: bool,
        negated: bool,
        stands: Stands,
    ) -> String {
        let (test, nested) = if absent_holds {
            (
                format!(
                    "!{}.IsPresent() || {}",
                    at.expr,
                    self.join(then, negated, Stands::Disjunct)
                ),
                matches!(stands, Stands::Conjunct),
            )
        } else {
            (
                format!(
                    "{}.IsPresent() && {}",
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

    /// Never asked: Go lowers no union, and the walk refuses a `union-match`
    /// on a backend that lowers none before it builds the test of a case
    /// (`TemplateWalk::union_case`).
    fn case_test(
        &self,
        _at: &GoAt,
        union: &ScopedUnion,
        _variant: &str,
        _record: CaseTest<'_, '_, GoAt>,
        _negated: bool,
    ) -> String {
        unreachable!(
            "the walk refuses the union `{}` on Go before it asks for the test of a case",
            union.name
        )
    }
}

impl TemplateSpelling for GoSpelling {
    type At = GoAt;

    fn language(&self) -> Language {
        Language::Go
    }

    /// `Matches`'s parameter `value`, of type `Value`.
    fn root(&self) -> GoAt {
        GoAt {
            expr: "value".to_string(),
        }
    }

    /// The optional's value, read where a test before it proved it present.
    fn present(&self, at: &GoAt) -> GoAt {
        GoAt {
            expr: format!("{}.Proven()", at.expr),
        }
    }

    fn element(&self, at: &GoAt, index: usize) -> GoAt {
        GoAt {
            expr: format!("{}[{index}]", at.expr),
        }
    }

    fn field(&self, at: &GoAt, _record: &ScopedRecord, id: &str, _ty: &ValueType) -> GoAt {
        GoAt {
            expr: format!("{}.{}", at.expr, event_schema_field_ident(id, Language::Go)),
        }
    }

    /// The parameter as it is, but for a byte string a value is built with,
    /// which is copied, so the value holds no array its caller can write into.
    fn parameter(
        &self,
        name: &str,
        ty: &ValueType,
        _at: &Position<'_, GoAt>,
        purpose: Use,
    ) -> String {
        let param = go_parameter(name);
        match (purpose, ty) {
            (Use::Build, TypeOf::Bytes { .. }) => format!("{BYTES_PACKAGE}.Clone({param})"),
            (Use::Build, TypeOf::Optional { elem }) if matches!(**elem, TypeOf::Bytes { .. }) => {
                format!("optional.Map({param}, {BYTES_PACKAGE}.Clone)")
            }
            _ => param,
        }
    }

    /// An integer in decimal, `true` or `false`, a variant of an enum through
    /// the package of its document, a text, or a byte string as the slice of
    /// its octets: one spelling compares and builds. An integer is an untyped
    /// constant, which takes the type of the position it stands at.
    fn literal(
        &self,
        text: &str,
        at: &Position<'_, GoAt>,
        _purpose: Use,
    ) -> Result<String, String> {
        let refused = || format!("'{text}' is no literal of {}", at.ty);
        match &at.ty {
            TypeOf::Scalar(SceType::Bool) => match text.trim() {
                held @ ("true" | "false") => Ok(held.to_string()),
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
                    Language::Go,
                    &enum_qualified_type(&declared.name, Language::Go),
                    &declared.name,
                    text.trim(),
                ))
            }
            TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
                integer_literal(Language::Go, text).ok_or_else(refused)
            }
            TypeOf::Text { .. } => Ok(format!("\"{}\"", filters::escape_go(text.to_string()))),
            TypeOf::Bytes { .. } => {
                let octets: Vec<String> = hex_octets(text)
                    .ok_or_else(refused)?
                    .into_iter()
                    .map(|pair| format!("0x{pair}"))
                    .collect();
                Ok(format!("[]byte{{{}}}", octets.join(", ")))
            }
            _ => Err(refused()),
        }
    }

    fn call(
        &self,
        import: &ImportContext,
        function: TemplateFunction,
        first: Option<&GoAt>,
        args: &[String],
    ) -> String {
        let function = match function {
            TemplateFunction::Match => "Matches",
            TemplateFunction::Value => "Specific",
        };
        let arguments: Vec<String> = first
            .map(|at| at.expr.clone())
            .into_iter()
            .chain(args.iter().map(|arg| go_parameter(arg)))
            .collect();
        self.names_document(&import.document_name);
        format!(
            "{}.{function}({})",
            go_package_alias(&filters::to_snake_case(import.document_name.clone())),
            arguments.join(", ")
        )
    }

    /// The absent value of the optional's element type.
    fn absent(&self, at: &Position<'_, GoAt>) -> String {
        match &at.ty {
            TypeOf::Optional { elem } => {
                format!("optional.None[{}]()", self.type_of(elem, at.scope))
            }
            other => {
                self.refuse(format!(
                    "an <sce:omit> stands where a value of {other} is sent"
                ));
                String::new()
            }
        }
    }

    /// The value, present, of the optional's element type, which an untyped
    /// constant would not tell `optional.Some` by itself.
    fn present_value(&self, at: &Position<'_, GoAt>, value: String) -> String {
        match &at.ty {
            TypeOf::Optional { elem } => {
                format!("optional.Some[{}]({value})", self.type_of(elem, at.scope))
            }
            _ => value,
        }
    }

    /// The record's payload struct, each field given by its name.
    fn record_value(
        &self,
        _at: &Position<'_, GoAt>,
        record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        self.names_document(&record.name);
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| format!("{}: {value}", event_schema_field_ident(&id, Language::Go)))
            .collect();
        format!(
            "{}{{{}}}",
            event_schema_payload_type(&record.name, Language::Go),
            fields.join(", ")
        )
    }

    /// The slice of the elements, of the list's type.
    fn list_value(&self, at: &Position<'_, GoAt>, elements: Vec<String>) -> String {
        format!(
            "{}{{{}}}",
            self.type_of(&at.ty, at.scope),
            elements.join(", ")
        )
    }
}
