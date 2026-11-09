// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Kotlin unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.2): `Value`, the type the template matches; `matches`, whether a
//! value matches it (ES 201 873-1 Annex B.1); and, for a specific template
//! (§6.2), `value`, the one value it sends.
//!
//! What each matcher holds is decided by the walk every backend shares
//! ([`super::test_template_walk`]), and its tests are joined as every backend
//! whose connectives are C's joins them ([`JoinSpelling`]); this file spells
//! the leaves in Kotlin, and the template (`forge/kotlin/test_template.kt.jinja2`)
//! writes the declarations around the two expressions.
//!
//! Every type is the one the schema unit spells ([`unit_type`]), and every
//! enum, record and union is named by the package of the document that
//! declares it ([`kotlin_qualified`]): an enum a record of another document
//! holds is named by the one declaration of it, whatever document imports it
//! and under whatever alias, and the unit imports no package, so no
//! declaration of another unit (each template's own `Value`, `matches` and
//! `value` among them) can hide one of its own.
//!
//! A value that may be absent is read by a `let` on it, its present value
//! bound for the tests that read it and smart-cast by its test of `null`; a
//! union by an `is` test of its case, the value bound the same way where the
//! case's record is read (docs/adr/0092). A byte string, absent or present, is
//! compared by its octets (`contentEquals`), since Kotlin's `==` compares a
//! `ByteArray` by its identity; a `specific` matcher stands only where a
//! number, a `bool`, an enum, a text or a byte string is matched
//! (`test_profile::matcher_admitted`), so no value that holds a byte string
//! inside a record, a list or a union is compared whole.

use super::test_template_walk::{
    hex_octets, negate, CanonicalCase, CanonicalSpelling, CaseTest, Held, JoinSpelling, Op,
    Position, Stands, TemplateFunction, TemplateSpelling, TemplateWalk, Use,
};
use super::*;
use crate::forge::schema_unit_type::{unit_type, C11Declarations, UnitNames};
use crate::forge::value_type::{ScopedRecord, ScopedUnion, TypeOf, TypeScope, ValueType};
use crate::generator::Language;

/// The function of the test profile's Kotlin runtime that counts the
/// characters of a text, which a length restriction on a `string[N]` counts:
/// its code points, as C++'s `SCE::Test::textLength` counts them.
const TEXT_LENGTH: &str = "com.sce.runtime.test.textLength";

/// The unit of the test template `m`, whose imports are `imports`.
pub(super) fn render_test_template_kotlin(
    env: &minijinja::Environment,
    m: &TestTemplateModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Kotlin, imports);
    let scope = import_type_scope(imports);
    let spelling = KotlinSpelling;
    let walk = TemplateWalk {
        template: m,
        imports,
        spelling: &spelling,
    };
    walk.refuse_unlowered_unions(&scope)?;
    walk.refuse_unread_enums(&scope)?;
    let names = KotlinNames { scope: &scope };
    let spell = |ty: &ValueType| {
        unit_type(
            Language::Kotlin,
            ty,
            &names,
            &mut C11Declarations::default(),
        )
        .map_err(|why| walk.refused(why))
    };
    let value_type = spell(&m.template_type)?;
    let parameters: Vec<serde_json::Value> = m
        .parameters
        .iter()
        .map(|p| {
            Ok(serde_json::json!({
                "id": p.id,
                "name": kotlin_parameter(&p.id),
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
    let (log_type, canonical_expr) = walk.message_form(&KotlinCanonical, &scope)?.unzip();

    let mut ctx = l.base_context(&m.name);
    ctx.insert("template_name".into(), m.name.clone().into());
    ctx.insert("template_type".into(), m.template_type.to_string().into());
    ctx.insert("value_type".into(), value_type.into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("match_expr".into(), match_expr.into());
    ctx.insert("value_expr".into(), serde_json::json!(value_expr));
    ctx.insert("canonical_expr".into(), serde_json::json!(canonical_expr));
    ctx.insert("log_type".into(), serde_json::json!(log_type));
    l.render(env, "test_template", ctx)
}

/// The object of the profile's Kotlin runtime whose functions write the
/// canonical form of a value in the test log
/// (`backends/kotlin/runtime/src/commonMain/kotlin/com/sce/runtime/test/Canonical.kt`).
const CANONICAL: &str = "com.sce.runtime.test.Canonical";

/// A value the canonical writer reads, in Kotlin.
struct CanonicalAt {
    /// What names the value: `value`, a lambda's parameter, the record of a
    /// case the `when` bound, or a field of any of them.
    expr: String,
    /// How many bindings the value stands inside: the next one's number.
    depth: usize,
}

/// The Kotlin spelling of a value's canonical form in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), with the writers of
/// `com.sce.runtime.test.Canonical`: a `String`, a `Boolean`, a `List` or a
/// `Map` of them, which the runtime's `Json.writeJcs` writes. The walk is the
/// one every backend shares (`test_template_walk::canonical_value`, through
/// `TemplateWalk::message_form`); an optional field is a
/// `?.let` that leaves an absent one null, which the record omits, a list a
/// `map`, and a union a `when` over the classes of its cases.
struct KotlinCanonical;

impl CanonicalSpelling for KotlinCanonical {
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
                event_schema_field_ident(id, Language::Kotlin)
            ),
            depth: at.depth,
        }
    }

    /// A lambda binds the value it is given, and a union's `when` the union.
    fn bound(&self, at: &CanonicalAt, prefix: &str) -> (String, CanonicalAt) {
        let name = format!("{prefix}_{}", at.depth);
        let value = CanonicalAt {
            expr: name.clone(),
            depth: at.depth + 1,
        };
        (name, value)
    }

    /// The `when` binds the union itself, which each arm reads smart-cast to
    /// its case's class, whose record is its `value`.
    fn case_record(&self, bound: CanonicalAt) -> CanonicalAt {
        CanonicalAt {
            expr: format!("{}.value", bound.expr),
            depth: bound.depth,
        }
    }

    fn boolean(&self, at: &CanonicalAt) -> String {
        format!("{CANONICAL}.boolean({})", at.expr)
    }

    /// A number no variant of an open enum declares is the text of its
    /// digits, written by the same writer as any integer.
    fn enumerated(&self, at: &CanonicalAt, strict: bool) -> String {
        if strict {
            format!("{CANONICAL}.enumerated({}.declaredName())", at.expr)
        } else {
            format!(
                "{CANONICAL}.openEnumerated({0}.declaredName(), \
                 {CANONICAL}.integer({0}.toUnderlying()))",
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
            "{CANONICAL}.list({}.map {{ {binding} -> {element} }})",
            at.expr
        )
    }

    /// As it is: a field a record always holds is never null.
    fn held_field(&self, written: String) -> String {
        written
    }

    /// Null where the field is absent, which the record omits.
    fn optional_field(&self, field: &CanonicalAt, binding: &str, written: String) -> String {
        format!("{}?.let {{ {binding} -> {written} }}", field.expr)
    }

    fn record(&self, fields: Vec<(String, String)>) -> String {
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| format!("\"{}\" to {value}", filters::escape_kotlin(id)))
            .collect();
        format!("{CANONICAL}.record({})", fields.join(", "))
    }

    /// A `when` on one line, an `is` arm per case's class. The union is bound
    /// as the subject where a case's record is read, so the arm reads it
    /// smart-cast; a union no case of which holds a record binds nothing.
    fn union(
        &self,
        at: &CanonicalAt,
        union: &ScopedUnion,
        cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        let binding = cases
            .iter()
            .find_map(|(_, held)| held.as_ref().map(|(binding, _)| binding.clone()));
        let arms: Vec<String> = cases
            .into_iter()
            .map(|(variant, held)| {
                let class = kotlin_case_type(union, &variant);
                let name = filters::escape_kotlin(variant);
                let record = held.map_or_else(|| "null".to_string(), |(_, written)| written);
                format!("is {class} -> {CANONICAL}.unionCase(\"{name}\", {record})")
            })
            .collect();
        let subject = match binding {
            Some(binding) => format!("val {binding} = {}", at.expr),
            None => at.expr.clone(),
        };
        Ok(format!("when ({subject}) {{ {} }}", arms.join("; ")))
    }
}

/// The name the generated `matches` and `value` take the formal parameter
/// `id` by: `param_<id>`, so that no parameter's name is one the unit declares
/// (`Value`, `value`, `matches`), the name `matches` gives the value it is
/// handed, or a keyword of the language.
fn kotlin_parameter(id: &str) -> String {
    format!("param_{id}")
}

/// The enum class (or, for an open enum, the value class) of the enum `alias`
/// names in `scope`, by its document's package.
fn kotlin_enum_type(scope: &TypeScope, alias: &str) -> Option<String> {
    scope.enums.get(alias).map(|declared| {
        kotlin_qualified(
            &declared.name,
            &enum_qualified_type(&declared.name, Language::Kotlin),
        )
    })
}

/// The payload data class of the record `record`, by its document's package.
fn kotlin_record_type(record: &ScopedRecord) -> String {
    kotlin_qualified(
        &record.name,
        &event_schema_payload_type(&record.name, Language::Kotlin),
    )
}

/// The sealed interface the unit of the union `union` declares, by its
/// document's package, named as the import line of its unit names it
/// ([`forge_import_identity`]).
fn kotlin_union_type(union: &ScopedUnion) -> String {
    let identity = forge_import_identity(
        &union.name,
        &Language::Kotlin,
        false,
        &crate::ForgeCompileOptions::default(),
    );
    kotlin_qualified(&union.name, &identity.type_name)
}

/// The class of the case `variant` of the union `union`, nested in its
/// interface.
fn kotlin_case_type(union: &ScopedUnion, variant: &str) -> String {
    format!(
        "{}.{}",
        kotlin_union_type(union),
        kotlin_case_ident(variant)
    )
}

/// The literal of the integer `text` as a value of `scalar`: each width by
/// the conversion Kotlin has for it, since an integer literal is an `Int` or
/// a `Long` and an unsigned one a `UInt` or a `ULong`, and `==` converts
/// neither. The least `Int` and `Long` are their constants, since Kotlin
/// reads `-2147483648` as the negation of a `Long`. The default a suite gives
/// a module parameter of `scalar` is written by it too (`kotlin_test_tables`).
pub(super) fn kotlin_integer_literal(scalar: &SceType, text: &str) -> Option<String> {
    let n = crate::forge::test_profile::decimal_integer(text)?;
    Some(match scalar {
        SceType::Int8 => format!("({n}).toByte()"),
        SceType::Int16 => format!("({n}).toShort()"),
        SceType::Int32 if n == i128::from(i32::MIN) => "Int.MIN_VALUE".to_string(),
        SceType::Int32 => n.to_string(),
        SceType::Int64 if n == i128::from(i64::MIN) => "Long.MIN_VALUE".to_string(),
        SceType::Int64 => format!("{n}L"),
        SceType::Uint8 => format!("{n}u.toUByte()"),
        SceType::Uint16 => format!("{n}u.toUShort()"),
        SceType::Uint32 => format!("{n}u"),
        SceType::Uint64 => format!("{n}uL"),
        SceType::Float32
        | SceType::Float64
        | SceType::Bool
        | SceType::String
        | SceType::Bytes
        | SceType::Enum(_) => return None,
    })
}

/// The byte string the hex literal `text` writes, two hex digits an octet, as
/// a `ByteArray` of its octets; `None` for a literal that is no hex. The
/// default a suite gives a module parameter of `bytes[N]` is written by it too
/// (`kotlin_test_tables`).
pub(super) fn kotlin_bytes_literal(text: &str) -> Option<String> {
    let octets: Vec<String> = hex_octets(text)?
        .into_iter()
        .map(|pair| format!("0x{pair}.toByte()"))
        .collect();
    Some(format!("byteArrayOf({})", octets.join(", ")))
}

/// What a Kotlin template's unit names from outside it, read in the template's
/// own scope ([`UnitNames`]).
struct KotlinNames<'s> {
    scope: &'s TypeScope,
}

impl UnitNames for KotlinNames<'_> {
    // An enum the scope does not read is refused before a type is spelled
    // (`TemplateWalk::refuse_unread_enums`); its alias stands in its place
    // here.
    fn scalar(&self, ty: &SceType) -> String {
        match ty {
            SceType::Enum(reference) => kotlin_enum_type(self.scope, &reference.alias)
                .unwrap_or_else(|| reference.alias.clone()),
            other => kotlin_type(other).to_string(),
        }
    }
    fn record(&self, alias: &str) -> Option<String> {
        self.scope.records.get(alias).map(kotlin_record_type)
    }
    fn union(&self, alias: &str) -> Option<String> {
        self.scope.unions.get(alias).map(kotlin_union_type)
    }
    // No Kotlin type is named by a mangled composition.
    fn enum_mangle(&self, alias: &str) -> String {
        alias.to_string()
    }
}

/// One position of the value a matcher stands at, in Kotlin.
#[derive(Clone)]
struct KotlinAt {
    /// The value at the position, as an expression a test reads.
    expr: String,
    /// The name of the lambda parameter a test binds the position's value
    /// through, where it binds one: an optional's present value is that name,
    /// a union case's record its `value`.
    bound: Option<String>,
    /// How many bindings the position stands inside: the next one's number.
    depth: usize,
}

impl KotlinAt {
    /// The name the position's value is bound by, where a test binds it, and
    /// its expression otherwise.
    fn binding(&self) -> &str {
        self.bound.as_deref().unwrap_or(&self.expr)
    }
}

/// The Kotlin spelling of a template's unit.
struct KotlinSpelling;

impl JoinSpelling for KotlinSpelling {
    /// A `bool` is compared with a literal by reading it, a byte string, absent
    /// or present, by its octets.
    fn compare(&self, at: &Position<'_, KotlinAt>, op: Op, negated: bool, rhs: &str) -> String {
        let operator = op.spelled(negated);
        let place = &at.at.expr;
        let bytes = match &at.ty {
            TypeOf::Bytes { .. } => true,
            TypeOf::Optional { elem } => matches!(**elem, TypeOf::Bytes { .. }),
            _ => false,
        };
        match (&at.ty, op) {
            (TypeOf::Scalar(SceType::Bool), Op::Eq) if rhs == "true" || rhs == "false" => {
                negate(place.clone(), (rhs == "true") == negated)
            }
            (_, Op::Eq) if bytes => negate(format!("{place}.contentEquals({rhs})"), negated),
            _ => format!("{place} {operator} {rhs}"),
        }
    }

    /// A text counts its characters, through the profile's runtime
    /// ([`TEXT_LENGTH`]); a byte string and a list their `size`.
    fn count(&self, at: &Position<'_, KotlinAt>, op: Op, negated: bool, n: u32) -> String {
        let operator = op.spelled(negated);
        let place = &at.at.expr;
        match &at.ty {
            TypeOf::Text { .. } => format!("{TEXT_LENGTH}({place}) {operator} {n}"),
            _ => format!("{place}.size {operator} {n}"),
        }
    }

    fn absence(&self, at: &KotlinAt, negated: bool) -> String {
        let operator = if negated { "!=" } else { "==" };
        format!("{} {operator} null", at.expr)
    }

    /// A `let` on the optional, which binds its value as `inner` names it: a
    /// call, which stands as an operand without parentheses.
    fn bound_optional(
        &self,
        at: &KotlinAt,
        inner: &KotlinAt,
        then: &Held<'_, KotlinAt>,
        absent_holds: bool,
        negated: bool,
        _stands: Stands,
    ) -> String {
        let name = inner.binding();
        let test = if absent_holds {
            format!(
                "{name} == null || {}",
                self.join(then, negated, Stands::Disjunct)
            )
        } else {
            format!(
                "{name} != null && {}",
                self.join(then, negated, Stands::Conjunct)
            )
        };
        format!("{}.let {{ {name} -> {test} }}", at.expr)
    }

    /// An `is` test of the case's class (`!is` where `negated`), and, where
    /// the case's matcher reads its record, a `let` that binds the union and
    /// reads the record of the case the test proved.
    fn case_test(
        &self,
        at: &KotlinAt,
        union: &ScopedUnion,
        variant: &str,
        record: CaseTest<'_, '_, KotlinAt>,
        negated: bool,
    ) -> String {
        let case = kotlin_case_type(union, variant);
        match record {
            CaseTest::Empty | CaseTest::AnyRecord => {
                let test = if negated { "!is" } else { "is" };
                format!("{} {test} {case}", at.expr)
            }
            CaseTest::Holds { inner, then } => {
                let name = inner.binding();
                negate(
                    format!(
                        "{}.let {{ {name} -> {name} is {case} && {} }}",
                        at.expr,
                        self.join(then, false, Stands::Conjunct)
                    ),
                    negated,
                )
            }
        }
    }
}

impl TemplateSpelling for KotlinSpelling {
    type At = KotlinAt;

    fn language(&self) -> Language {
        Language::Kotlin
    }

    /// `matches`'s parameter `value`, of type `Value`.
    fn root(&self) -> KotlinAt {
        KotlinAt {
            expr: "value".to_string(),
            bound: None,
            depth: 0,
        }
    }

    fn present(&self, at: &KotlinAt) -> KotlinAt {
        let name = format!("present_{}", at.depth);
        KotlinAt {
            expr: name.clone(),
            bound: Some(name),
            depth: at.depth + 1,
        }
    }

    fn element(&self, at: &KotlinAt, index: usize) -> KotlinAt {
        KotlinAt {
            expr: format!("{}[{index}]", at.expr),
            bound: None,
            depth: at.depth,
        }
    }

    fn field(&self, at: &KotlinAt, _record: &ScopedRecord, id: &str, _ty: &ValueType) -> KotlinAt {
        KotlinAt {
            expr: format!(
                "{}.{}",
                at.expr,
                event_schema_field_ident(id, Language::Kotlin)
            ),
            bound: None,
            depth: at.depth,
        }
    }

    /// The union bound by the lambda of its `let`, smart-cast by its `is`
    /// test to the case's class, whose record is its `value`.
    fn case(
        &self,
        at: &KotlinAt,
        _union: &ScopedUnion,
        _variant: &str,
    ) -> Result<KotlinAt, String> {
        let name = format!("case_{}", at.depth);
        Ok(KotlinAt {
            expr: format!("{name}.value"),
            bound: Some(name),
            depth: at.depth + 1,
        })
    }

    /// The parameter as it is, but for a byte string a value is built with,
    /// which is copied, so the value holds no array its caller can write into.
    fn parameter(
        &self,
        name: &str,
        ty: &ValueType,
        _at: &Position<'_, KotlinAt>,
        purpose: Use,
    ) -> String {
        let param = kotlin_parameter(name);
        match (purpose, ty) {
            (Use::Build, TypeOf::Bytes { .. }) => format!("{param}.copyOf()"),
            (Use::Build, TypeOf::Optional { elem }) if matches!(**elem, TypeOf::Bytes { .. }) => {
                format!("{param}?.copyOf()")
            }
            _ => param,
        }
    }

    /// An integer in decimal at its type's width, `true` or `false`, a variant
    /// of an enum through its type, a text, or a byte string in hex, each
    /// octet converted to the `Byte` it is: one spelling compares and builds.
    fn literal(
        &self,
        text: &str,
        at: &Position<'_, KotlinAt>,
        _purpose: Use,
    ) -> Result<String, String> {
        let refused = || format!("'{text}' is no literal of {}", at.ty);
        match &at.ty {
            TypeOf::Scalar(SceType::Bool) => match text.trim() {
                held @ ("true" | "false") => Ok(held.to_string()),
                _ => Err(refused()),
            },
            TypeOf::Scalar(SceType::Enum(reference)) => {
                let declared = kotlin_enum_type(at.scope, &reference.alias)
                    .ok_or_else(|| format!("`enum:{}` names no enum read", reference.alias))?;
                Ok(format!(
                    "{declared}.{}",
                    crate::forge::enum_naming::variant_ident(
                        Language::Kotlin,
                        &reference.alias,
                        text.trim()
                    )
                ))
            }
            TypeOf::Scalar(scalar) => kotlin_integer_literal(scalar, text).ok_or_else(refused),
            TypeOf::Text { .. } => Ok(filters::kotlin_text_expression(text)),
            TypeOf::Bytes { .. } => kotlin_bytes_literal(text).ok_or_else(refused),
            _ => Err(refused()),
        }
    }

    fn call(
        &self,
        import: &ImportContext,
        function: TemplateFunction,
        first: Option<&KotlinAt>,
        args: &[String],
    ) -> String {
        let function = match function {
            TemplateFunction::Match => "matches",
            TemplateFunction::Value => "value",
        };
        let arguments: Vec<String> = first
            .map(|at| at.expr.clone())
            .into_iter()
            .chain(args.iter().map(|arg| kotlin_parameter(arg)))
            .collect();
        format!(
            "{}({})",
            kotlin_qualified(&import.document_name, function),
            arguments.join(", ")
        )
    }

    fn absent(&self, _at: &Position<'_, KotlinAt>) -> String {
        "null".to_string()
    }

    /// As it is: a `T?` takes a value of `T`.
    fn present_value(&self, _at: &Position<'_, KotlinAt>, value: String) -> String {
        value
    }

    /// The record's data class, each field given by its name.
    fn record_value(
        &self,
        _at: &Position<'_, KotlinAt>,
        record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| {
                format!(
                    "{} = {value}",
                    event_schema_field_ident(&id, Language::Kotlin)
                )
            })
            .collect();
        format!("{}({})", kotlin_record_type(record), fields.join(", "))
    }

    /// The list of the elements, its element type read from where it stands.
    fn list_value(&self, _at: &Position<'_, KotlinAt>, elements: Vec<String>) -> String {
        format!("listOf({})", elements.join(", "))
    }

    /// The case's class holding its record, or the case's object where it
    /// holds none.
    fn case_value(
        &self,
        _at: &Position<'_, KotlinAt>,
        union: &ScopedUnion,
        variant: &str,
        record: Option<String>,
    ) -> Result<String, String> {
        let case = kotlin_case_type(union, variant);
        Ok(match record {
            Some(record) => format!("{case}({record})"),
            None => case,
        })
    }
}
