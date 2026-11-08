// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.2): a caseless enum named by the document, as an algorithm's
//! unit is, which declares `Value`, the type the template matches; `matches`,
//! whether a value matches it (ES 201 873-1 Annex B.1); and, for a specific
//! template (§6.2), `value`, the one value it sends. Every unit of a run is a
//! file of one module, so the three are members of the enum: two templates
//! never declare one name twice, and a template another references is called
//! through its enum (`<Name>.matches`, `<Name>.value`).
//!
//! What each matcher holds is decided by the walk every backend shares
//! ([`super::test_template_walk`]), and its tests are joined as every backend
//! whose connectives are C's joins them ([`JoinSpelling`]); this file spells
//! the leaves in Swift, and the template
//! (`forge/swift/test_template.swift.jinja2`) writes the declarations around
//! the two expressions.
//!
//! Every type is the one the schema unit spells ([`unit_type`]), and every
//! enum and record is named as its document's unit declares it, bare, since
//! the units are files of one module ([`enum_qualified_type`],
//! [`event_schema_payload_type`]). A record or an enum the Swift unit of its
//! document does not write is refused by name before anything is spelled, by
//! the answer that unit's own generation gives
//! ([`swift_record_fields_unlowered`], [`swift_scoped_enum_unlowered`]), so no
//! unit names a type nothing declares.
//!
//! A value that may be absent is a `T?`. Swift has no expression that binds
//! one, so it is read through `map`, whose closure is handed the present
//! value under a name of its own, and an absent one is decided by `??`:
//! `x.map { present_0 -> Bool in … } ?? false`, which binds tighter than every
//! comparison and connective it stands among, so it is never put in
//! parentheses. A text, a byte string, a list and a value that may be absent
//! compare by their contents under `==`, since each is a value type, and a
//! byte string a parameter gives is held as given for the same reason. A
//! length restriction on a text counts its characters, Unicode code points,
//! through the profile's runtime ([`text_length_call`]): a Swift `String`'s
//! `count` counts grapheme clusters, which is another number. Swift lowers no
//! union ([`crate::forge::codegen_matrix::UNION_BACKENDS`]), so a template
//! whose type reaches one is refused by the walk before anything is spelled.

use super::test_template_walk::{
    hex_octets, integer_literal, negate, CaseTest, Held, JoinSpelling, Op, Position, Stands,
    TemplateFunction, TemplateSpelling, TemplateWalk, Use,
};
use super::*;
use crate::forge::schema_unit_type::{unit_type, C11Declarations, UnitNames};
use crate::forge::value_type::{ScopedRecord, ScopedUnion, TypeOf, TypeScope, ValueType};
use crate::generator::Language;
use crate::std_imports::SWIFT_TEST_RUNTIME_MODULE;

/// The function of the test profile's Swift runtime that counts the
/// characters of a text, which a length restriction on a `string[N]` counts:
/// its code points, as C++'s `SCE::Test::textLength` counts them.
const TEXT_LENGTH: &str = "textLength";

/// The count of the characters of the text `place`, through the profile's
/// runtime, named by its module so that no declaration of the unit's own
/// module can hide it.
fn text_length_call(place: &str) -> String {
    format!("{SWIFT_TEST_RUNTIME_MODULE}.{TEXT_LENGTH}({place})")
}

/// The unit of the test template `m`, whose imports are `imports`.
pub(super) fn render_test_template_swift(
    env: &minijinja::Environment,
    m: &TestTemplateModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Swift, imports);
    let scope = import_type_scope(imports);
    let spelling = SwiftSpelling;
    let walk = TemplateWalk {
        template: m,
        imports,
        spelling: &spelling,
    };
    walk.refuse_unlowered_unions(&scope)?;
    walk.refuse_unread_enums(&scope)?;
    refuse_unwritten_types(&walk, &scope)?;
    let names = SwiftNames { scope: &scope };
    let spell = |ty: &ValueType| {
        unit_type(Language::Swift, ty, &names, &mut C11Declarations::default())
            .map_err(|why| walk.refused(why))
    };
    let value_type = spell(&m.template_type)?;
    let parameters: Vec<serde_json::Value> = m
        .parameters
        .iter()
        .map(|p| {
            Ok(serde_json::json!({
                "id": p.id,
                "name": swift_parameter(&p.id),
                "ty": spell(&p.parameter_type)?,
                "sce_type": p.parameter_type.to_string(),
            }))
        })
        .collect::<Result<_, ForgeError>>()?;
    let root = walk.root(&scope);
    let match_expr = spelling.join(&walk.matches(&m.matcher, &root)?, false, Stands::Whole);
    let value_expr = if walk.specific() {
        Some(walk.value(&m.matcher, &root)?)
    } else {
        None
    };
    // A value counts no text: only a test does, so the match is what is read. A
    // text literal that holds the same characters reads as a call too, which
    // costs the unit an import of a module its target links, and nothing more.
    let counts_text = match_expr.contains(&format!("{SWIFT_TEST_RUNTIME_MODULE}.{TEXT_LENGTH}("));

    let mut ctx = l.base_context(&m.name);
    ctx.insert("template_name".into(), m.name.clone().into());
    ctx.insert("template_type".into(), m.template_type.to_string().into());
    ctx.insert("value_type".into(), value_type.into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("match_expr".into(), match_expr.into());
    ctx.insert("value_expr".into(), serde_json::json!(value_expr));
    ctx.insert(
        "test_runtime_import".into(),
        serde_json::json!(counts_text.then_some(SWIFT_TEST_RUNTIME_MODULE)),
    );
    l.render(env, "test_template", ctx)
}

/// The text `text` as a Swift string literal. The default a suite gives a
/// module parameter of `string[N]` is written by it too (`swift_test_tables`).
pub(super) fn swift_text_literal(text: &str) -> String {
    format!("\"{}\"", filters::escape_swift(text.to_string()))
}

/// The byte string the hex literal `text` writes, two hex digits an octet, as
/// the array literal of its octets; `None` for a literal that is no hex. The
/// default a suite gives a module parameter of `bytes[N]` is written by it too
/// (`swift_test_tables`).
pub(super) fn swift_bytes_literal(text: &str) -> Option<String> {
    let octets: Vec<String> = hex_octets(text)?
        .into_iter()
        .map(|pair| format!("0x{pair}"))
        .collect();
    Some(format!("[{}]", octets.join(", ")))
}

/// The name the generated `matches` and `value` take the formal parameter
/// `id` by: `param_<id>`, so that no parameter's name is one the unit declares
/// (`Value`, `value`, `matches`), the name `matches` gives the value it is
/// handed, the name a closure binds a present value by (`present_<n>`), or a
/// word Swift reserves.
fn swift_parameter(id: &str) -> String {
    format!("param_{id}")
}

/// Refuse the template where its type or a parameter's holds, at any depth, a
/// record or an enum whose document's Swift unit is not written: the unit
/// would name a type nothing declares. Each is refused by what that unit's own
/// generation refuses it for.
fn refuse_unwritten_types(
    walk: &TemplateWalk<'_, SwiftSpelling>,
    scope: &TypeScope,
) -> Result<(), ForgeError> {
    let m = walk.template;
    for ty in
        std::iter::once(&m.template_type).chain(m.parameters.iter().map(|p| &p.parameter_type))
    {
        if let Some(why) = unwritten(ty, scope) {
            return Err(walk.refused(why));
        }
    }
    Ok(())
}

/// What of the type `ty`, read in `scope`, names a record or an enum the Swift
/// backend writes no unit of, described for a refusal; `None` where every one
/// it names is written. A record is read through its own fields, each in its
/// own document's scope, as its schema's unit is.
fn unwritten(ty: &ValueType, scope: &TypeScope) -> Option<String> {
    match ty {
        TypeOf::Scalar(SceType::Enum(reference)) => scope
            .enums
            .get(&reference.alias)
            .and_then(swift_scoped_enum_unlowered)
            .map(|why| {
                format!(
                    "`enum:{}`, an enum with {why}, has no Swift lowering yet",
                    reference.alias
                )
            }),
        TypeOf::Optional { elem } | TypeOf::List { elem, .. } => unwritten(elem, scope),
        TypeOf::Record { alias } => {
            let record = scope.records.get(alias)?;
            let fields: Vec<(&str, &ValueType)> = record
                .fields
                .iter()
                .map(|(id, field)| (id.as_str(), field))
                .collect();
            let enum_unlowered = |enum_alias: &str| {
                record.scope.enums.get(enum_alias).map_or_else(
                    || Some("an enum it does not import".to_string()),
                    swift_scoped_enum_unlowered,
                )
            };
            swift_record_fields_unlowered(&fields, &enum_unlowered)
                .map(|why| {
                    format!("`record:{alias}`, a record with {why}, has no Swift lowering yet")
                })
                .or_else(|| {
                    record
                        .fields
                        .iter()
                        .find_map(|(_, field)| unwritten(field, &record.scope))
                })
        }
        TypeOf::Scalar(_) | TypeOf::Text { .. } | TypeOf::Bytes { .. } | TypeOf::Union { .. } => {
            None
        }
    }
}

/// What a Swift template's unit names from outside it, read in the template's
/// own scope ([`UnitNames`]): every enum and record bare, as its document's
/// unit declares it in the one module.
struct SwiftNames<'s> {
    scope: &'s TypeScope,
}

impl UnitNames for SwiftNames<'_> {
    // An enum the scope does not read is refused before a type is spelled
    // (`TemplateWalk::refuse_unread_enums`); its alias stands in its place
    // here.
    fn scalar(&self, ty: &SceType) -> String {
        match ty {
            SceType::Enum(reference) => self.scope.enums.get(&reference.alias).map_or_else(
                || reference.alias.clone(),
                |declared| enum_qualified_type(&declared.name, Language::Swift),
            ),
            other => swift_type(other).to_string(),
        }
    }
    fn record(&self, alias: &str) -> Option<String> {
        self.scope
            .records
            .get(alias)
            .map(|record| event_schema_payload_type(&record.name, Language::Swift))
    }
    // Swift lowers no union: `unit_type` refuses one by name before it asks.
    fn union(&self, _alias: &str) -> Option<String> {
        None
    }
    // No Swift type is named by a mangled composition.
    fn enum_mangle(&self, alias: &str) -> String {
        alias.to_string()
    }
}

/// One position of the value a matcher stands at, in Swift.
#[derive(Clone)]
struct SwiftAt {
    /// The value at the position, as an expression a test reads.
    expr: String,
    /// The name of the closure parameter a test binds the position's value
    /// through, where it binds one: an optional's present value is that name.
    bound: Option<String>,
    /// How many bindings the position stands inside: the next one's number.
    depth: usize,
}

impl SwiftAt {
    /// The name the position's value is bound by, where a test binds it, and
    /// its expression otherwise.
    fn binding(&self) -> &str {
        self.bound.as_deref().unwrap_or(&self.expr)
    }
}

/// The Swift spelling of a template's unit.
struct SwiftSpelling;

impl JoinSpelling for SwiftSpelling {
    /// A `bool` is compared with a literal by reading it; every other value,
    /// a text, a byte string and one that may be absent among them, by the
    /// operator, which compares a Swift value by its contents.
    fn compare(&self, at: &Position<'_, SwiftAt>, op: Op, negated: bool, rhs: &str) -> String {
        let place = &at.at.expr;
        match (&at.ty, op) {
            (TypeOf::Scalar(SceType::Bool), Op::Eq) if rhs == "true" || rhs == "false" => {
                negate(place.clone(), (rhs == "true") == negated)
            }
            _ => format!("{place} {} {rhs}", op.spelled(negated)),
        }
    }

    /// A text counts its characters, through the profile's runtime
    /// ([`text_length_call`]); a byte string and a list their `count`.
    fn count(&self, at: &Position<'_, SwiftAt>, op: Op, negated: bool, n: u32) -> String {
        let operator = op.spelled(negated);
        let place = &at.at.expr;
        match &at.ty {
            TypeOf::Text { .. } => format!("{} {operator} {n}", text_length_call(place)),
            _ => format!("{place}.count {operator} {n}"),
        }
    }

    fn absence(&self, at: &SwiftAt, negated: bool) -> String {
        let operator = if negated { "!=" } else { "==" };
        format!("{} {operator} nil", at.expr)
    }

    /// The optional's `map`, whose closure binds its present value as `inner`
    /// names it and answers the test of it, the answer for an absent value
    /// given by `??`. The closure states its result, so the type checker reads
    /// no more of the expression than the closure to type it.
    fn bound_optional(
        &self,
        at: &SwiftAt,
        inner: &SwiftAt,
        then: &Held<'_, SwiftAt>,
        absent_holds: bool,
        negated: bool,
        _stands: Stands,
    ) -> String {
        format!(
            "{}.map {{ {} -> Bool in {} }} ?? {}",
            at.expr,
            inner.binding(),
            self.join(then, negated, Stands::Whole),
            self.connectives().truth(absent_holds)
        )
    }

    /// Never asked: Swift lowers no union, and the walk refuses a
    /// `union-match` on a backend that lowers none before it builds the test of
    /// a case (`TemplateWalk::union_case`).
    fn case_test(
        &self,
        _at: &SwiftAt,
        union: &ScopedUnion,
        _variant: &str,
        _record: CaseTest<'_, '_, SwiftAt>,
        _negated: bool,
    ) -> String {
        unreachable!(
            "the walk refuses the union `{}` on Swift before it asks for the test of a case",
            union.name
        )
    }
}

impl TemplateSpelling for SwiftSpelling {
    type At = SwiftAt;

    fn language(&self) -> Language {
        Language::Swift
    }

    /// `matches`'s parameter `value`, of type `Value`.
    fn root(&self) -> SwiftAt {
        SwiftAt {
            expr: "value".to_string(),
            bound: None,
            depth: 0,
        }
    }

    /// The parameter of the closure an optional's `map` hands its present
    /// value to, numbered by how many closures it stands inside, so a closure
    /// inside another never hides the outer one's.
    fn present(&self, at: &SwiftAt) -> SwiftAt {
        let name = format!("present_{}", at.depth);
        SwiftAt {
            expr: name.clone(),
            bound: Some(name),
            depth: at.depth + 1,
        }
    }

    fn element(&self, at: &SwiftAt, index: usize) -> SwiftAt {
        SwiftAt {
            expr: format!("{}[{index}]", at.expr),
            bound: None,
            depth: at.depth,
        }
    }

    fn field(&self, at: &SwiftAt, _record: &ScopedRecord, id: &str, _ty: &ValueType) -> SwiftAt {
        SwiftAt {
            expr: format!(
                "{}.{}",
                at.expr,
                event_schema_field_ident(id, Language::Swift)
            ),
            bound: None,
            depth: at.depth,
        }
    }

    /// The parameter as it is: every value Swift holds a template's type in,
    /// a byte string and a list among them, is a value type, so a value built
    /// from one holds no storage its caller can write into.
    fn parameter(
        &self,
        name: &str,
        _ty: &ValueType,
        _at: &Position<'_, SwiftAt>,
        _purpose: Use,
    ) -> String {
        swift_parameter(name)
    }

    /// An integer in decimal, which the type it stands beside reads at its
    /// width, `true` or `false`, a case of an enum through its type, a text as
    /// a string literal, or a byte string as the array of its octets: one
    /// spelling compares and builds.
    fn literal(
        &self,
        text: &str,
        at: &Position<'_, SwiftAt>,
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
                Ok(crate::forge::enum_naming::variant_ref(
                    Language::Swift,
                    &enum_qualified_type(&declared.name, Language::Swift),
                    &declared.name,
                    text.trim(),
                ))
            }
            TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
                integer_literal(Language::Swift, text).ok_or_else(refused)
            }
            TypeOf::Text { .. } => Ok(swift_text_literal(text)),
            TypeOf::Bytes { .. } => swift_bytes_literal(text).ok_or_else(refused),
            _ => Err(refused()),
        }
    }

    /// The function of the referenced template's enum, named by the
    /// referenced document as every unit is.
    fn call(
        &self,
        import: &ImportContext,
        function: TemplateFunction,
        first: Option<&SwiftAt>,
        args: &[String],
    ) -> String {
        let function = match function {
            TemplateFunction::Match => "matches",
            TemplateFunction::Value => "value",
        };
        let arguments: Vec<String> = first
            .map(|at| at.expr.clone())
            .into_iter()
            .chain(args.iter().map(|arg| swift_parameter(arg)))
            .collect();
        format!(
            "{}.{function}({})",
            filters::to_pascal_case(import.document_name.clone()),
            arguments.join(", ")
        )
    }

    /// `nil`, the absent value of every optional position, whose type the
    /// position it stands at gives it.
    fn absent(&self, _at: &Position<'_, SwiftAt>) -> String {
        "nil".to_string()
    }

    /// As it is: a `T?` takes a value of `T`.
    fn present_value(&self, _at: &Position<'_, SwiftAt>, value: String) -> String {
        value
    }

    /// The record's payload struct, built by its initializer of every field,
    /// each labelled by its name in the order its schema declares it.
    fn record_value(
        &self,
        _at: &Position<'_, SwiftAt>,
        record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| {
                format!(
                    "{}: {value}",
                    event_schema_field_ident(&id, Language::Swift)
                )
            })
            .collect();
        format!(
            "{}({})",
            event_schema_payload_type(&record.name, Language::Swift),
            fields.join(", ")
        )
    }

    /// The array of the elements, its element type read from where it stands.
    fn list_value(&self, _at: &Position<'_, SwiftAt>, elements: Vec<String>) -> String {
        format!("[{}]", elements.join(", "))
    }
}
