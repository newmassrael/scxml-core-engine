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
//! profile's runtime ([`TEXT_LENGTH`]). A union (docs/adr/0092, section 7, the
//! Go row) is the value struct its unit declares, matched by comparing its
//! `Kind()` with the variant of its tag's enum, the record of the case read
//! through the unit's `Proven<Case>()` after that comparison in the same
//! conjunction, which Go evaluates first, and built by the constructor of its
//! case.
//!
//! The inverse of [`GoCanonical`], [`GoReading`], reads a value back from
//! that form with the readers of the same package `canonical`, over the same
//! walk ([`super::test_template_walk::canonical_form`]): the unit of a test
//! configuration reads the values of a recorded input trace with it
//! (docs/SCE_TTCN3_LANE.md §10.5).

use std::cell::RefCell;
use std::collections::BTreeSet;

use super::test_template_walk::{
    hex_octets, integer_literal, negate, CanonicalCase, CanonicalForm, CanonicalSpelling, CaseTest,
    Held, JoinSpelling, Op, Position, Stands, TemplateFunction, TemplateSpelling, TemplateWalk,
    Use,
};
use super::*;
use crate::forge::schema_unit_type::{unit_type, C11Declarations, UnitNames};
use crate::forge::value_type::{
    Bound, ScopedEnum, ScopedRecord, ScopedUnion, TypeOf, TypeScope, ValueType,
};
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
/// its own name, [`CANONICAL`]. The unit of a test configuration reads and
/// writes its records' forms through it too.
pub(super) const CANONICAL_PATH: &str =
    "github.com/newmassrael/sce-go-runtime/testprofile/canonical";

/// The name a unit names the writers of [`CANONICAL_PATH`] by.
const CANONICAL: &str = "canonical";

/// A value the canonical writer reads, in Go.
pub(super) struct GoCanonicalAt {
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
/// since what an absent field holds is no value of its type. A union is
/// written by a closure of no parameter whose `switch` binds the record of
/// each case where its case is the value's (`CanonicalSpelling::union`). The
/// unit of a test configuration writes its records' forms with it too, beside
/// their readers ([`GoReading`]).
pub(super) struct GoCanonical;

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

    /// JSON `null` where the element is absent, which the list keeps in its
    /// place; its value read as a field's is otherwise.
    fn optional_element(&self, element: &GoCanonicalAt, _binding: &str, written: String) -> String {
        format!(
            "{CANONICAL}.OptionalElement({}.IsPresent(), func() any {{ return {written} }})",
            element.expr
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

    /// A function literal of no parameter, called where it is written, whose
    /// `switch` tells the cases apart by the declared name of the union's tag,
    /// which the unit's `Kind()` answers through its enum's `DeclaredName`, so
    /// the unit names no package of the tag's enum; an arm of a case that holds
    /// a record binds it through the unit's `Proven<Case>()`. A value is built
    /// only by the constructor of a case, so the end after the `switch` is not
    /// reached, and says so.
    fn union(
        &self,
        at: &GoCanonicalAt,
        union: &ScopedUnion,
        cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        use crate::forge::composed_lowering::{go_case_read, GO_UNION_KIND};
        let arms: String = cases
            .into_iter()
            .map(|(variant, held)| {
                let name = filters::escape_go(variant.clone());
                match held {
                    Some((binding, written)) => format!(
                        "case \"{name}\": {binding} := {}.{}(); return \
                         {CANONICAL}.UnionCase(\"{name}\", {written}); ",
                        at.expr,
                        go_case_read(&variant)
                    ),
                    None => {
                        format!("case \"{name}\": return {CANONICAL}.UnionCase(\"{name}\", nil); ")
                    }
                }
            })
            .collect();
        Ok(format!(
            "func() any {{ switch {}.{GO_UNION_KIND}().DeclaredName().Proven() {{ {arms}}}; \
             panic(\"a value of {} is in no case of its tag\") }}()",
            at.expr,
            filters::escape_go(union.name.clone())
        ))
    }
}

/// The name a unit names the forge runtime's optional value by
/// (`optional.Of[T]`, docs/adr/0055), which the render door imports where the
/// text names it.
const OPTIONAL: &str = crate::std_imports::GO_FORGE_OPTIONAL_NAME;

/// `text` as a Go string literal.
pub(super) fn go_literal(text: &str) -> String {
    format!("\"{}\"", filters::escape_go(text.to_string()))
}

/// A value the canonical reader reads, in Go: the JSON it is read from, its
/// type, and where it stands.
pub(super) struct GoReadingAt<'s> {
    /// An expression of the `any` the value is read from: the reader's
    /// parameter `value`, a field of its record's object
    /// (`canonical.Member`), or a name a function binds.
    json: String,
    /// The object of the record the value is a field of, and the field's id,
    /// where it is one: what a field that may be absent is looked up by, with
    /// the object's `Get`.
    member: Option<(String, String)>,
    /// The value's type, read in [`Self::scope`]: what a function that reads
    /// it answers, which Go names in the function's signature.
    ty: ValueType,
    /// The scope the value's type is read in: its record's own document's.
    scope: &'s TypeScope,
    /// Where the value stands, as a refusal names it: the record's type name,
    /// then each field by its id, `[]` for an element of a list.
    what: String,
    /// How many bindings the value stands inside: the next one's number.
    depth: usize,
    /// How many records the value stands inside: the number of the next
    /// record's object.
    records: usize,
}

/// The Go reader of the canonical form of a value (docs/SCE_TEST_PROFILE.md
/// §12.5; docs/SCE_TTCN3_LANE.md §10.3): the value of the type the event
/// schema's unit declares, read from an `any` by the readers of the profile
/// runtime's package `canonical`
/// (`backends/go/runtime/testprofile/canonical/from_canonical.go`), each of
/// which answers the value and an error that names where the value stands
/// where it does not belong to its type. Which form each SCE type takes is the
/// walk its writer's is ([`super::test_template_walk::canonical_form`]), so the
/// reader reads exactly what [`GoCanonical`] writes.
///
/// Each read is an expression of two values, the value read and the error, so
/// a Go statement takes both: a leaf a call of the runtime's reader; a record,
/// a list and a value that may be absent a function literal, called where it
/// stands, that reads its parts one statement at a time and answers the first
/// refusal. A function literal names the types of what it answers, so the
/// position tracks the SCE type it stands at, read in its record's scope, and
/// names it as the schema unit declares it ([`GoSpelling::type_of`]), which
/// notes the documents whose packages the unit imports. A field whose type is
/// not optional is read from `canonical.Member`, which every reader refuses as
/// absent where the object does not name it; one that may be absent is looked
/// up by the object's `Get`, read where it is there and absent otherwise, as a
/// `null` element of a list is. Go lowers no union, so the walk's refusal of
/// one stands.
pub(super) struct GoReading<'s> {
    /// The spelling that names the types the reader answers.
    pub(super) spelling: &'s GoSpelling,
    /// The type of the record read, `record:<alias>` in [`Self::scope`].
    pub(super) record_type: &'s ValueType,
    /// The scope the record type is read in: the port type's that takes it.
    pub(super) scope: &'s TypeScope,
    /// What the value read is named in a refusal: its record's type name.
    pub(super) what: String,
}

impl<'s> GoReading<'s> {
    /// The Go type of the value at `at`.
    fn type_of(&self, at: &GoReadingAt<'s>) -> String {
        self.spelling.type_of(&at.ty, at.scope)
    }

    /// The type `ty` holds inside it, which `inner` reads off it: the element
    /// type of a list, the value type of an optional value. Where it is of
    /// another kind, which the walk never hands here, the refusal is kept and
    /// `ty` stands in its place.
    fn inner_of(
        &self,
        ty: &ValueType,
        kind: &str,
        inner: impl Fn(&ValueType) -> Option<&ValueType>,
    ) -> ValueType {
        inner(ty).cloned().unwrap_or_else(|| {
            self.spelling
                .refuse(format!("a value of {ty} is read where {kind} stands"));
            ty.clone()
        })
    }

    /// A function literal of no parameter that answers `answers` and an
    /// error, its body `statements`, called where it stands.
    fn called(answers: &str, statements: &[String]) -> String {
        format!(
            "func() ({answers}, error) {{ {} }}()",
            statements.join("; ")
        )
    }

    /// The read of a value that may be absent, at `at`, whose type is
    /// `optional<T>`: a function literal whose parameters are `parameters`,
    /// called with `arguments`, that answers the absent value where `absent`
    /// holds and otherwise the value `written` reads, from the parameter it
    /// binds, made present.
    fn optional_read(
        &self,
        at: &GoReadingAt<'s>,
        parameters: &str,
        absent: &str,
        written: String,
        arguments: &str,
    ) -> String {
        let optional = self.type_of(at);
        let held = self.inner_of(&at.ty, "an optional value", |ty| match ty {
            TypeOf::Optional { elem } => Some(elem.as_ref()),
            _ => None,
        });
        let none = format!(
            "{OPTIONAL}.None[{}]()",
            self.spelling.type_of(&held, at.scope)
        );
        format!(
            "func({parameters}) ({optional}, error) {{ if {absent} {{ return {none}, nil }}; read, \
             err := {written}; if err != nil {{ return {none}, err }}; return \
             {OPTIONAL}.Some(read), nil }}({arguments})"
        )
    }
}

impl<'s> CanonicalForm for GoReading<'s> {
    type At = GoReadingAt<'s>;

    /// The reader's parameter `value`, read as the record.
    fn root(&self) -> GoReadingAt<'s> {
        GoReadingAt {
            json: "value".to_string(),
            member: None,
            ty: self.record_type.clone(),
            scope: self.scope,
            what: self.what.clone(),
            depth: 0,
            records: 0,
        }
    }

    /// The field `id` of the object its record bound, of the type its record
    /// declares, read in the record's own scope.
    fn field(&self, at: &GoReadingAt<'s>, id: &str) -> GoReadingAt<'s> {
        let scope: &'s TypeScope = at.scope;
        let declared: Option<(ValueType, &'s TypeScope)> = match &at.ty {
            TypeOf::Record { alias } => scope.records.get(alias).and_then(|record| {
                record
                    .fields
                    .iter()
                    .find(|(field, _)| field.as_str() == id)
                    .map(|(_, ty)| (ty.clone(), &record.scope))
            }),
            _ => None,
        };
        let (ty, field_scope) = declared.unwrap_or_else(|| {
            self.spelling.refuse(format!(
                "a value of {} is read where a record that declares '{id}' stands",
                at.ty
            ));
            (at.ty.clone(), scope)
        });
        let object = format!("object_{}", at.records);
        GoReadingAt {
            json: format!("{CANONICAL}.Member({object}, {})", go_literal(id)),
            member: Some((object, id.to_string())),
            ty,
            scope: field_scope,
            what: format!("{}.{id}", at.what),
            depth: at.depth,
            records: at.records + 1,
        }
    }

    fn bound(&self, at: &GoReadingAt<'s>, prefix: &str) -> (String, GoReadingAt<'s>) {
        let name = format!("{prefix}_{}", at.depth);
        let value = GoReadingAt {
            json: name.clone(),
            member: None,
            ty: at.ty.clone(),
            scope: at.scope,
            what: at.what.clone(),
            depth: at.depth + 1,
            records: at.records,
        };
        (name, value)
    }

    fn case_record(&self, bound: GoReadingAt<'s>) -> GoReadingAt<'s> {
        bound
    }

    /// The element the loop over the list binds, of the list's element type.
    fn list_element(&self, list: &GoReadingAt<'s>, bound: GoReadingAt<'s>) -> GoReadingAt<'s> {
        let ty = self.inner_of(&list.ty, "a list", |ty| match ty {
            TypeOf::List { elem, .. } => Some(elem.as_ref()),
            _ => None,
        });
        GoReadingAt {
            ty,
            what: format!("{}[]", list.what),
            ..bound
        }
    }

    /// The value an optional field or element holds, of its value type, bound
    /// where it is there.
    fn optional_value(&self, field: &GoReadingAt<'s>, bound: GoReadingAt<'s>) -> GoReadingAt<'s> {
        let ty = self.inner_of(&field.ty, "an optional value", |ty| match ty {
            TypeOf::Optional { elem } => Some(elem.as_ref()),
            _ => None,
        });
        GoReadingAt { ty, ..bound }
    }

    fn boolean(&self, at: &GoReadingAt<'s>) -> String {
        format!(
            "{CANONICAL}.ReadBoolean({}, {})",
            at.json,
            go_literal(&at.what)
        )
    }

    /// The variant whose declared name the text is, from the variants of the
    /// enum's own package: a closed enum's, or an open enum's and the number
    /// no variant declares, which the package's `<Enum>FromUnderlying` makes
    /// a value of.
    fn enumerated(&self, at: &GoReadingAt<'s>, declared: &ScopedEnum) -> String {
        self.spelling.names_document(&declared.name);
        let enum_type = enum_qualified_type(&declared.name, Language::Go);
        let variants: Vec<String> = declared
            .variants
            .iter()
            .map(|variant| {
                format!(
                    "{{Name: {}, Value: {}}}",
                    go_literal(&variant.name),
                    crate::forge::enum_naming::variant_ref(
                        Language::Go,
                        &enum_type,
                        &declared.name,
                        &variant.name
                    )
                )
            })
            .collect();
        let variants = format!(
            "[]{CANONICAL}.Variant[{enum_type}]{{{}}}",
            variants.join(", ")
        );
        if declared.strict {
            format!(
                "{CANONICAL}.ReadEnumerated({}, {variants}, {})",
                at.json,
                go_literal(&at.what)
            )
        } else {
            let (_, from_underlying) = enum_number_symbols(&declared.name, true, Language::Go);
            format!(
                "{CANONICAL}.ReadOpenEnumerated({}, {variants}, {from_underlying}, {})",
                at.json,
                go_literal(&at.what)
            )
        }
    }

    /// Read as the integer type of its width, which bounds it.
    fn integer(&self, at: &GoReadingAt<'s>, width: &SceType) -> String {
        format!(
            "{CANONICAL}.ReadInteger[{}]({}, {})",
            go_type(width),
            at.json,
            go_literal(&at.what)
        )
    }

    fn text(&self, at: &GoReadingAt<'s>, max: &Bound) -> String {
        format!(
            "{CANONICAL}.ReadText({}, {}, {})",
            at.json,
            max.max,
            go_literal(&at.what)
        )
    }

    fn bytes(&self, at: &GoReadingAt<'s>, max: &Bound) -> String {
        format!(
            "{CANONICAL}.ReadBytes({}, {}, {})",
            at.json,
            max.max,
            go_literal(&at.what)
        )
    }

    /// A function literal that reads the elements within the list's bounds
    /// and each element in turn, as `binding`, into a slice of the list's
    /// type.
    fn list(&self, at: &GoReadingAt<'s>, max: &Bound, binding: &str, element: String) -> String {
        let list = self.type_of(at);
        let items = format!("items_{}", at.depth);
        Self::called(
            &list,
            &[
                format!(
                    "{items}, err := {CANONICAL}.ReadElements({}, {}, {}, {})",
                    at.json,
                    max.min,
                    max.max,
                    go_literal(&at.what)
                ),
                "if err != nil { return nil, err }".to_string(),
                format!("read := make({list}, 0, len({items}))"),
                format!(
                    "for _, {binding} := range {items} {{ element, err := {element}; if err != \
                     nil {{ return nil, err }}; read = append(read, element) }}"
                ),
                "return read, nil".to_string(),
            ],
        )
    }

    fn held_field(&self, written: String) -> String {
        written
    }

    /// The field read, as `binding`, where its record's object names it, and
    /// absent where it does not, which the canonical form writes by omitting
    /// it. A member that is there and `null` is read, and refused by the
    /// reader of its type, since an absent field is never written as `null`.
    fn optional_field(&self, field: &GoReadingAt<'s>, binding: &str, written: String) -> String {
        let (object, id) = field.member.clone().unwrap_or_else(|| {
            self.spelling.refuse(format!(
                "the value at {} is read as a field where it is none",
                field.what
            ));
            (String::new(), String::new())
        });
        self.optional_read(
            field,
            &format!("{binding} any, held bool"),
            "!held",
            written,
            &format!("{object}.Get({})", go_literal(&id)),
        )
    }

    /// The element read, as `binding`, where it is not `null`.
    fn optional_element(
        &self,
        element: &GoReadingAt<'s>,
        binding: &str,
        written: String,
    ) -> String {
        self.optional_read(
            element,
            &format!("{binding} any"),
            &format!("{binding} == nil"),
            written,
            &element.json,
        )
    }

    /// A function literal that binds the record's object, refused where it
    /// names a field the record does not declare, and reads each field into
    /// the record's payload struct by its name, in the order the schema
    /// declares them, answering the first refusal.
    fn record(
        &self,
        at: &GoReadingAt<'s>,
        _record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        let payload = self.type_of(at);
        let ids: Vec<String> = fields.iter().map(|(id, _)| go_literal(id)).collect();
        let object = format!(
            "{CANONICAL}.ReadRecord({}, []string{{{}}}, {})",
            at.json,
            ids.join(", "),
            go_literal(&at.what)
        );
        let refused = format!("if err != nil {{ return {payload}{{}}, err }}");
        if fields.is_empty() {
            // No field reads the object, so it is not bound.
            return Self::called(
                &payload,
                &[
                    format!("if _, err := {object}; err != nil {{ return {payload}{{}}, err }}"),
                    format!("return {payload}{{}}, nil"),
                ],
            );
        }
        let mut statements = vec![
            format!("object_{}, err := {object}", at.records),
            refused,
            format!("var read {payload}"),
        ];
        for (id, value) in fields {
            statements.push(format!(
                "if read.{}, err = {value}; err != nil {{ return {payload}{{}}, err }}",
                event_schema_field_ident(&id, Language::Go)
            ));
        }
        statements.push("return read, nil".to_string());
        Self::called(&payload, &statements)
    }

    fn union(
        &self,
        _at: &GoReadingAt<'s>,
        union: &ScopedUnion,
        _cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        Err(crate::forge::value_type::union_unlowered(&union.name))
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
    // The value struct the union's unit declares, through its document's
    // package.
    fn union(&self, alias: &str) -> Option<String> {
        self.scope.unions.get(alias).map(|union| {
            self.spelling.names_document(&union.name);
            union_unit_type(&union.name, Language::Go)
        })
    }
    // No Go type is named by a mangled composition.
    fn enum_mangle(&self, alias: &str) -> String {
        alias.to_string()
    }
}

/// One position of the value a matcher stands at, in Go: the expression a
/// test reads it by.
#[derive(Clone)]
pub(super) struct GoAt {
    expr: String,
}

/// The Go spelling of a template's unit, and the names a configuration's
/// readers of its records name ([`GoReading`]).
#[derive(Default)]
pub(super) struct GoSpelling {
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

    /// The variant of the tag's enum of `union` that its case `variant` is
    /// for, through the package of the enum's document; empty, with the
    /// refusal kept, where the union's scope does not read its tag's enum.
    fn case_kind(&self, union: &ScopedUnion, variant: &str) -> String {
        let Some(declared) = union.scope.enums.get(&union.tag_enum) else {
            self.refuse(format!(
                "the tag's enum `{}` of `union:{}` was not read",
                union.tag_enum, union.name
            ));
            return String::new();
        };
        self.names_document(&declared.name);
        crate::forge::enum_naming::variant_ref(
            Language::Go,
            &enum_qualified_type(&declared.name, Language::Go),
            &declared.name,
            variant,
        )
    }

    /// The value struct the unit of `union` declares, through its document's
    /// package.
    fn union_type(&self, union: &ScopedUnion) -> String {
        self.names_document(&union.name);
        union_unit_type(&union.name, Language::Go)
    }

    /// The Go type of a value of `ty`, read in `scope`; empty, with the
    /// refusal kept, where the type names what the scope does not read.
    pub(super) fn type_of(&self, ty: &ValueType, scope: &TypeScope) -> String {
        let names = GoNames {
            scope,
            spelling: self,
        };
        unit_type(Language::Go, ty, &names, &mut C11Declarations::default()).unwrap_or_else(|why| {
            self.refuse(why);
            String::new()
        })
    }

    /// The refusal a spelling kept, if one did, taken: the unit is refused by
    /// it once the walk is done.
    pub(super) fn take_refusal(&self) -> Option<String> {
        self.refusal.borrow_mut().take()
    }

    /// The import specs of the template's unit whose text is `text`: the
    /// standard `bytes`, the profile's runtime, its writers of the canonical
    /// form ([`CANONICAL_PATH`]) and the package of each document a spelling
    /// named ([`Self::document_imports`]), each kept only where the text names
    /// it, since Go refuses a file that imports a package it never names.
    fn imports(
        &self,
        text: &str,
        options: &crate::ForgeCompileOptions,
    ) -> Result<Vec<String>, ForgeError> {
        let candidates = vec![
            format!("\"{BYTES_PACKAGE}\""),
            format!("\"{TEST_RUNTIME_PATH}\""),
            format!("\"{CANONICAL_PATH}\""),
        ];
        let mut imports = crate::std_imports::go_imports_named_in(text, &candidates);
        imports.extend(self.document_imports(text, options)?);
        Ok(imports)
    }

    /// The import spec of the package of each document a spelling named,
    /// kept only where `text` names it: the package every Go unit imports it
    /// by ([`forge_import_identity`]), below `options`' `go_module_prefix`,
    /// which is refused by name where it is missing and a document is named.
    pub(super) fn document_imports(
        &self,
        text: &str,
        options: &crate::ForgeCompileOptions,
    ) -> Result<Vec<String>, ForgeError> {
        let named = self.named.borrow();
        if !named.is_empty() {
            go_import_root(options.go_module_prefix.as_deref())?;
        }
        let candidates: Vec<String> = named
            .iter()
            .map(|document| {
                forge_import_identity(document, &Language::Go, false, options)
                    .include_stmt
                    .trim()
                    .to_string()
            })
            .collect();
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

    /// The union's `Kind()` compared with the variant of its case (`!=` where
    /// `negated`), and, where the case's matcher reads its record, joined in
    /// one conjunction with the test of the record, which reads it through the
    /// unit's `Proven<Case>()` ([`TemplateSpelling::case`]): Go evaluates the
    /// comparison first and reads the record only where it holds. The
    /// conjunction is in parentheses, so `!` before it negates it whole.
    fn case_test(
        &self,
        at: &GoAt,
        union: &ScopedUnion,
        variant: &str,
        record: CaseTest<'_, '_, GoAt>,
        negated: bool,
    ) -> String {
        use crate::forge::composed_lowering::GO_UNION_KIND;
        let kind = self.case_kind(union, variant);
        let tag = format!("{}.{GO_UNION_KIND}()", at.expr);
        match record {
            CaseTest::Empty | CaseTest::AnyRecord => {
                let operator = if negated { "!=" } else { "==" };
                format!("{tag} {operator} {kind}")
            }
            CaseTest::Holds { then, .. } => negate(
                format!(
                    "({tag} == {kind} && {})",
                    self.join(then, false, Stands::Conjunct)
                ),
                negated,
            ),
        }
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

    /// The record of the case, read through the unit's `Proven<Case>()`,
    /// which [`JoinSpelling::case_test`] calls only after the comparison of
    /// the union's `Kind()` that proves the case.
    fn case(&self, at: &GoAt, _union: &ScopedUnion, variant: &str) -> Result<GoAt, String> {
        Ok(GoAt {
            expr: format!(
                "{}.{}()",
                at.expr,
                crate::forge::composed_lowering::go_case_read(variant)
            ),
        })
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

    /// The constructor of the case, given its record, or nothing for a case
    /// that holds none: the only way the unit builds a value.
    fn case_value(
        &self,
        _at: &Position<'_, GoAt>,
        union: &ScopedUnion,
        variant: &str,
        record: Option<String>,
    ) -> Result<String, String> {
        Ok(format!(
            "{}{}({})",
            self.union_type(union),
            crate::forge::composed_lowering::go_case_ident(variant),
            record.unwrap_or_default()
        ))
    }
}
