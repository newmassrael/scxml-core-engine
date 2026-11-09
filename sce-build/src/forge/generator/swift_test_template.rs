// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.2): a caseless enum named by the document, as an algorithm's
//! unit is, which declares `Value`, the type the template matches; `matches`,
//! whether a value matches it (ES 201 873-1 Annex B.1); and, for a specific
//! template (§6.2), `value`, the one value it sends. A template over a record,
//! the type of a message, declares `TYPE` and `canonical` besides, the name of
//! its record and the canonical form of its value in the test log (§12.5,
//! [`SwiftCanonical`]). Every unit of a run is a file of one module, so these
//! are members of the enum: two templates never declare one name twice, and a
//! template another references is called through its enum (`<Name>.matches`,
//! `<Name>.value`).
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
//! `count` counts grapheme clusters, which is another number.
//!
//! A union is the enum its document's unit declares, bare, one case per
//! variant of the tag's enum, named as that enum's Swift declaration names the
//! variant ([`swift_case_ident`]), holding the case's record or nothing. A case
//! is tested through the union's `kind`, the value of the tag's enum the case
//! is for, written as every enum literal is (`value.kind == ShapeKind.circle`),
//! and the record of a case that holds one is read through the case's proven
//! read ([`swift_case_read`]) only after that test, bound by the closure of
//! `Optional`'s `map` as a present value is bound. A union's value is the case
//! through its enum (`UnionShapeMinimal.circle(...)`), never a bare case,
//! which an optional position reads as `Optional`'s own case where the two
//! share a name (`.none`). Until Swift lowers a union
//! ([`crate::forge::codegen_matrix::UNION_BACKENDS`]), a template whose type
//! reaches one is refused by the walk before anything is spelled.

use super::test_template_walk::{
    hex_octets, integer_literal, negate, CanonicalCase, CanonicalSpelling, CaseTest, Held,
    JoinSpelling, Op, Position, Stands, TemplateFunction, TemplateSpelling, TemplateWalk, Use,
};
use super::*;
use crate::forge::composed_lowering::{swift_case_ident, swift_case_read, SWIFT_UNION_KIND};
use crate::forge::schema_unit_type::{unit_type, C11Declarations, UnitNames};
use crate::forge::value_type::{
    ScopedEnum, ScopedRecord, ScopedUnion, TypeOf, TypeScope, ValueType,
};
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
    // Only a template over a record, the type of a message, writes a message's
    // type name and its canonical form (`TemplateWalk::message_form`).
    let (log_type, canonical_expr) = walk.message_form(&SwiftCanonical, &scope)?.unzip();
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
        "unit_imports".into(),
        serde_json::json!(unit_imports(counts_text, canonical_expr.is_some())),
    );
    ctx.insert("canonical_expr".into(), serde_json::json!(canonical_expr));
    ctx.insert("log_type".into(), serde_json::json!(log_type));
    ctx.insert("canonical_value".into(), CANONICAL_VALUE.into());
    l.render(env, "test_template", ctx)
}

/// The modules a template's unit imports, in the order it imports them: the
/// profile's runtime where a length restriction counts a text's characters
/// through it, or where the unit writes a message's canonical form with its
/// writers ([`CANONICAL`]), and then the runtime whose `JSONValue` that form is
/// ([`CANONICAL_VALUE`]) too. Every enum, record and referenced template is a
/// file of the unit's own module, so nothing else is imported.
fn unit_imports(counts_text: bool, writes_canonical: bool) -> Vec<&'static str> {
    if writes_canonical {
        vec![SWIFT_RUNTIME_MODULE, SWIFT_TEST_RUNTIME_MODULE]
    } else if counts_text {
        vec![SWIFT_TEST_RUNTIME_MODULE]
    } else {
        Vec::new()
    }
}

/// The module of the Swift runtime whose `JSONValue` a canonical form is.
const SWIFT_RUNTIME_MODULE: &str = "SceRuntime";

/// The type of a value's canonical form in the test log, named by its module
/// so that no declaration of the unit's own module can hide it.
const CANONICAL_VALUE: &str = "SceRuntime.JSONValue";

/// The enum of the profile's Swift runtime whose functions write the canonical
/// form of a value in the test log
/// (`backends/swift/runtime/Sources/SceTestProfile/Canonical.swift`), named by
/// its module as [`text_length_call`] names its function.
const CANONICAL: &str = "SceTestProfile.Canonical";

/// A value the canonical writer reads, in Swift.
struct CanonicalAt {
    /// What names the value: `value`, a closure's parameter, the record a
    /// union's case pattern bound, or a field of any of them.
    expr: String,
    /// How many bindings the value stands inside: the next one's number.
    depth: usize,
}

/// The Swift spelling of a value's canonical form in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), with the writers of
/// `SceTestProfile.Canonical`: a `JSONValue` of texts, truth values, arrays and
/// objects, which the runtime's `writeCanonicalJSON` writes. The walk is the
/// one every backend shares (`test_template_walk::canonical_value`, through
/// `TemplateWalk::message_form`); an optional field is its `map`, whose
/// closure is handed the present value and whose `nil` the record omits, a list
/// its `map`, and a union a `switch` over its enum's cases, each pattern
/// binding the case's record, in a closure called where it stands so that it
/// is one expression wherever a value is. Each closure states its result, as a
/// matcher's does, so the type checker reads no more than the closure to type
/// it. Since the pattern binds the record itself, the default
/// [`CanonicalSpelling::case_record`] names it.
struct SwiftCanonical;

impl CanonicalSpelling for SwiftCanonical {
    type At = CanonicalAt;

    /// `canonical`'s parameter `value`.
    fn root(&self) -> CanonicalAt {
        CanonicalAt {
            expr: "value".to_string(),
            depth: 0,
        }
    }

    fn field(&self, at: &CanonicalAt, id: &str) -> CanonicalAt {
        CanonicalAt {
            expr: format!(
                "{}.{}",
                at.expr,
                event_schema_field_ident(id, Language::Swift)
            ),
            depth: at.depth,
        }
    }

    /// A closure binds the value it is given, and a case pattern the case's
    /// record, numbered by how many it stands inside, so an inner one never
    /// hides an outer one's.
    fn bound(&self, at: &CanonicalAt, prefix: &str) -> (String, CanonicalAt) {
        let name = format!("{prefix}_{}", at.depth);
        let value = CanonicalAt {
            expr: name.clone(),
            depth: at.depth + 1,
        };
        (name, value)
    }

    fn boolean(&self, at: &CanonicalAt) -> String {
        format!("{CANONICAL}.boolean({})", at.expr)
    }

    /// A strict enum's `declaredName`; an open one's, which is nil for a
    /// number no variant declares, beside its number. A record that holds an
    /// enum of an open set is refused on Swift before the walk
    /// ([`refuse_unwritten_types`]), so only a later lowering of one reads
    /// the second form.
    fn enumerated(&self, at: &CanonicalAt, strict: bool) -> String {
        if strict {
            format!("{CANONICAL}.enumerated({}.declaredName)", at.expr)
        } else {
            format!(
                "{CANONICAL}.openEnumerated({0}.declaredName, {0}.rawValue)",
                at.expr
            )
        }
    }

    fn integer(&self, at: &CanonicalAt) -> String {
        format!("{CANONICAL}.integer({})", at.expr)
    }

    fn text(&self, at: &CanonicalAt) -> String {
        format!("{CANONICAL}.text({})", at.expr)
    }

    fn bytes(&self, at: &CanonicalAt) -> String {
        format!("{CANONICAL}.bytes({})", at.expr)
    }

    fn list(&self, at: &CanonicalAt, binding: &str, element: String) -> String {
        format!(
            "{CANONICAL}.list({}.map {{ {binding} -> {CANONICAL_VALUE} in {element} }})",
            at.expr
        )
    }

    /// As it is: a `JSONValue` stands where the record takes an optional one.
    fn held_field(&self, written: String) -> String {
        written
    }

    /// The field's `map`, nil where the field is absent, which the record
    /// omits.
    fn optional_field(&self, field: &CanonicalAt, binding: &str, written: String) -> String {
        format!(
            "{}.map {{ {binding} -> {CANONICAL_VALUE} in {written} }}",
            field.expr
        )
    }

    fn record(&self, fields: Vec<(String, String)>) -> String {
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| format!("(\"{}\", {value})", filters::escape_swift(id)))
            .collect();
        format!("{CANONICAL}.record({})", fields.join(", "))
    }

    /// A `switch` on one line, a case per variant through the union's enum,
    /// the pattern binding the case's record where the case holds one, in a
    /// closure called where it stands. The `switch` names every case, so it is
    /// exhaustive.
    fn union(
        &self,
        at: &CanonicalAt,
        union: &ScopedUnion,
        cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        let arms: Vec<String> = cases
            .into_iter()
            .map(|(variant, held)| {
                let case = format!(
                    "{}.{}",
                    swift_union_type(union),
                    swift_union_case(union, &variant)
                );
                let name = filters::escape_swift(variant);
                match held {
                    Some((binding, written)) => format!(
                        "case {case}(let {binding}): return {CANONICAL}.unionCase(\"{name}\", \
                         {written})"
                    ),
                    None => format!("case {case}: return {CANONICAL}.unionCase(\"{name}\", nil)"),
                }
            })
            .collect();
        Ok(format!(
            "{{ () -> {CANONICAL_VALUE} in switch {} {{ {} }} }}()",
            at.expr,
            arms.join("; ")
        ))
    }
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

/// The enum the unit of the union `union` declares, named bare by its
/// document, as every unit of the one module is ([`forge_import_identity`]).
fn swift_union_type(union: &ScopedUnion) -> String {
    forge_import_identity(
        &union.name,
        &Language::Swift,
        false,
        &crate::ForgeCompileOptions::default(),
    )
    .type_name
}

/// The enum the tag of the union `union` is a value of. A union is read with
/// its tag's enum in its own scope (`import_source::union_in_scope`), so every
/// union a walk is handed holds it.
fn swift_tag_enum(union: &ScopedUnion) -> &ScopedEnum {
    union.scope.enums.get(&union.tag_enum).unwrap_or_else(|| {
        panic!(
            "the union `{}` is read without its tag's enum `{}`",
            union.name, union.tag_enum
        )
    })
}

/// The identifier of the case `variant` of the union `union` in its enum: the
/// variant as the Swift declaration of the tag's enum names it.
fn swift_union_case(union: &ScopedUnion, variant: &str) -> String {
    swift_case_ident(&swift_tag_enum(union).name, variant)
}

/// The value of the tag's enum the case `variant` of the union `union` is for,
/// through the enum's type, as every enum literal is written
/// ([`crate::forge::enum_naming::variant_ref`]).
fn swift_union_kind(union: &ScopedUnion, variant: &str) -> String {
    let tag = swift_tag_enum(union);
    crate::forge::enum_naming::variant_ref(
        Language::Swift,
        &enum_qualified_type(&tag.name, Language::Swift),
        &tag.name,
        variant,
    )
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
    // Asked once `unit_type` spells a union on Swift, which until then refuses
    // one by name before it asks.
    fn union(&self, alias: &str) -> Option<String> {
        self.scope.unions.get(alias).map(swift_union_type)
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

    /// The union's `kind` compared with the value of the tag's enum the case
    /// is for, and, where the case's matcher reads its record, the record the
    /// case's proven read answers, bound by the closure of `Optional`'s `map`
    /// and read only once the test of the kind holds, since `&&` reads its
    /// right operand only then. The `??` that unwraps the `map`'s answer binds
    /// tighter than `&&`, and is put in parentheses so the two reads stand
    /// apart. Negated, the comparison of the kind is the one that holds where
    /// it does not, and the test of the record is negated whole, as `!` binds
    /// it only in parentheses.
    fn case_test(
        &self,
        at: &SwiftAt,
        union: &ScopedUnion,
        variant: &str,
        record: CaseTest<'_, '_, SwiftAt>,
        negated: bool,
    ) -> String {
        let kind = swift_union_kind(union, variant);
        match record {
            CaseTest::Empty | CaseTest::AnyRecord => format!(
                "{}.{SWIFT_UNION_KIND} {} {kind}",
                at.expr,
                Op::Eq.spelled(negated)
            ),
            CaseTest::Holds { inner, then } => {
                let test = format!(
                    "{}.{SWIFT_UNION_KIND} == {kind} && (Optional({}.{}).map {{ {} -> Bool in {} \
                     }} ?? false)",
                    at.expr,
                    at.expr,
                    swift_case_read(variant),
                    inner.binding(),
                    self.join(then, false, Stands::Whole)
                );
                if negated {
                    format!("!({test})")
                } else {
                    test
                }
            }
        }
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

    /// The parameter of the closure the case's proven read is handed to
    /// ([`JoinSpelling::case_test`]), numbered by how many closures it stands
    /// inside, as a present value's is.
    fn case(&self, at: &SwiftAt, _union: &ScopedUnion, _variant: &str) -> Result<SwiftAt, String> {
        let name = format!("case_{}", at.depth);
        Ok(SwiftAt {
            expr: name.clone(),
            bound: Some(name),
            depth: at.depth + 1,
        })
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

    /// The case through the union's enum, holding its record where it holds
    /// one: named by the enum, since a bare case at an optional position is
    /// read as `Optional`'s own case where the two share a name (`.none`).
    fn case_value(
        &self,
        _at: &Position<'_, SwiftAt>,
        union: &ScopedUnion,
        variant: &str,
        record: Option<String>,
    ) -> Result<String, String> {
        let case = format!(
            "{}.{}",
            swift_union_type(union),
            swift_union_case(union, variant)
        );
        Ok(match record {
            Some(record) => format!("{case}({record})"),
            None => case,
        })
    }
}

/// The Swift arm of a template over a union, walked by a spelling that lowers
/// a union ([`super::test_template_walk::LowersUnions`]): no generation
/// reaches it until Swift lowers a union
/// ([`crate::forge::codegen_matrix::UNION_BACKENDS`]), since the walk refuses
/// the template first (`TemplateWalk::refuse_unlowered_unions`), so the arm is
/// held to the spelling of the union unit before that unit is written.
#[cfg(test)]
mod tests {
    use super::super::test_template_walk::LowersUnions;
    use super::*;
    use std::path::{Path, PathBuf};

    /// The directory of the test profile's documents.
    fn profile_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile")
    }

    /// The test template `content` and its imports, read relative to
    /// `base_dir` for Swift as `compile_forge_file` reads them.
    fn read(content: &str, base_dir: &Path) -> (TestTemplateModel, Vec<ImportContext>) {
        let label = crate::DocumentLabel::symmetric("template");
        let parsed = crate::forge::parser::parse_forge_with_imports(content, label)
            .unwrap_or_else(|e| panic!("the template parses: {e:?}"))
            .expect("a forge document");
        let options = crate::ForgeCompileOptions::default();
        let mut imports =
            crate::forge::generator::resolve_imports(&parsed.imports, &Language::Swift, &options)
                .unwrap_or_else(|e| panic!("the imports resolve: {e:?}"));
        crate::validate_and_enrich_imports(
            &mut imports,
            &parsed.imports,
            base_dir,
            &Language::Swift,
            &options,
            label.diagnostic_label,
        )
        .unwrap_or_else(|e| panic!("the imports are read: {e:?}"));
        let crate::forge::model::ForgeDocument::TestTemplate(m) = parsed.document else {
            panic!("a test template");
        };
        (m, imports)
    }

    /// What the test template `content` matches on Swift, and the value it
    /// sends where it is specific.
    fn walked(content: &str, base_dir: &Path) -> (String, Option<String>) {
        let (m, imports) = read(content, base_dir);
        let scope = import_type_scope(&imports);
        let spelling = LowersUnions(SwiftSpelling);
        let walk = TemplateWalk {
            template: &m,
            imports: &imports,
            spelling: &spelling,
        };
        let root = walk.root(&scope);
        let held = walk
            .matches(&m.matcher, &root)
            .unwrap_or_else(|e| panic!("the template is walked: {e:?}"));
        let value = walk.specific().then(|| {
            walk.value(&m.matcher, &root)
                .unwrap_or_else(|e| panic!("the template has a value: {e:?}"))
        });
        (spelling.join(&held, false, Stands::Whole), value)
    }

    /// The fixture `<name>.scxml` of `dir`, walked as [`walked`] walks it.
    fn walked_fixture(dir: &Path, name: &str) -> (String, Option<String>) {
        let path = dir.join(format!("{name}.scxml"));
        let loaded = crate::load_forge_source(&path, &[])
            .unwrap_or_else(|e| panic!("{name} is read: {e:?}"));
        walked(loaded.text(), dir)
    }

    /// A template of the profile's directory over the union of the union
    /// kind's fixtures (`union_shape_minimal`: `dot`, empty, `circle` of a
    /// `radius`, `rect`), holding `matcher`.
    fn over_shape(name: &str, matcher: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="{name}"
       sce:type="union:Shape">
  <sce:import kind="union" src="../union/union_shape_minimal.scxml" as="Shape"/>
  {matcher}
</scxml>
"#
        )
    }

    /// The case `circle` whose record's radius is 3.
    const CIRCLE_OF_THREE: &str = r#"<sce:union-match variant="circle">
    <sce:record-match>
      <sce:specific field="radius" value="3"/>
    </sce:record-match>
  </sce:union-match>"#;

    /// The test of the case `circle` whose radius is 3, as the union's `kind`
    /// and the proven read of the case bound by `map`.
    const CIRCLE_OF_THREE_TEST: &str = "value.kind == ShapeKind.circle && \
         (Optional(value.provenCircle).map { case_0 -> Bool in case_0.radius == 3 } ?? false)";

    /// A case whose matcher reads nothing of its record is the test of the
    /// union's `kind` alone, the value of the tag's enum named through its
    /// type; a template that is not specific has no value.
    #[test]
    fn a_case_is_tested_by_the_kind_of_the_union() {
        let (matched, value) = walked_fixture(&profile_dir(), "shape_is_circle");
        assert_eq!(matched, "value.kind == ShapeKind.circle");
        assert_eq!(value, None);
    }

    /// A case's record is read through its proven read only once the test of
    /// the kind holds, bound by the closure of `map`, and an empty case is the
    /// test of the kind alone: a union field of a record, between the other
    /// cases of a value list.
    #[test]
    fn a_case_record_is_read_through_its_proven_read_after_the_kind() {
        let (matched, _) = walked_fixture(&profile_dir().join("imports"), "order_placed_match");
        assert!(
            matched.contains(
                "(value.payment.kind == PaymentKind.card && \
                 (Optional(value.payment.provenCard).map { case_0 -> Bool in \
                 SceTestProfile.textLength(case_0.last4) >= 4 && case_0.amount >= 1 } ?? false) \
                 || value.payment.kind == PaymentKind.cash)"
            ),
            "{matched}"
        );
    }

    /// A specific template's union value is the case through the union's
    /// enum, holding its record's value, or the case alone for an empty one.
    #[test]
    fn a_specific_union_value_is_its_case_through_the_unions_enum() {
        let (matched, value) = walked(
            &over_shape("circle_of_three", CIRCLE_OF_THREE),
            &profile_dir(),
        );
        assert_eq!(matched, CIRCLE_OF_THREE_TEST);
        assert_eq!(
            value.as_deref(),
            Some("UnionShapeMinimal.circle(ShapeRadiusMinimalPayload(radius: 3))")
        );
        let (matched, value) = walked(
            &over_shape("a_dot", r#"<sce:union-match variant="dot"/>"#),
            &profile_dir(),
        );
        assert_eq!(matched, "value.kind == ShapeKind.dot");
        assert_eq!(value.as_deref(), Some("UnionShapeMinimal.dot"));
    }

    /// A complement of cases is the conjunction of the tests that do not
    /// hold: an empty case's kind compared by `!=`, and the test of a case
    /// whose record is read negated whole.
    #[test]
    fn a_complement_of_cases_negates_each_test() {
        let complement = format!(
            "<sce:complement>\n  <sce:union-match variant=\"dot\"/>\n  {CIRCLE_OF_THREE}\n  \
             </sce:complement>"
        );
        let (matched, value) = walked(&over_shape("no_dot_nor_three", &complement), &profile_dir());
        assert_eq!(
            matched,
            format!("value.kind != ShapeKind.dot && !({CIRCLE_OF_THREE_TEST})")
        );
        assert_eq!(value, None);
    }

    /// The type a template's unit holds a union in is the enum the union's
    /// unit declares, bare; an alias the template reads no union by names
    /// none.
    #[test]
    fn a_union_is_named_by_the_enum_of_its_unit() {
        let (_, imports) = read(
            &over_shape("a_dot", r#"<sce:union-match variant="dot"/>"#),
            &profile_dir(),
        );
        let scope = import_type_scope(&imports);
        let names = SwiftNames { scope: &scope };
        assert_eq!(names.union("Shape").as_deref(), Some("UnionShapeMinimal"));
        assert_eq!(names.union("Other"), None);
    }
}
