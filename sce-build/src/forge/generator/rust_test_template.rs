// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.2): `Value`, the type the template matches; `matches`, whether a
//! value matches it (ES 201 873-1 Annex B.1); and, for a specific template
//! (§6.2), `value`, the one value it sends.
//!
//! What each matcher holds is decided by the walk every backend shares
//! ([`super::test_template_walk`]); this file spells it in Rust, and the
//! template (`forge/rust/test_template.rs.jinja2`) writes the declarations
//! around the two expressions.
//!
//! Every type is the one the schema unit spells ([`unit_type`]), and every
//! enum, record and union is named through the sibling module of the document
//! that declares it (`super::<document>`), as every generated Rust file
//! reaches its siblings: an enum a record of another document holds is named
//! by the one declaration of it, whatever document imports it and under
//! whatever alias, and the unit brings no module into scope it might not name.
//!
//! A value that may be absent is read by a `match` on it, its present value
//! bound for the tests that read it; a union by `matches!` on its case, the
//! case's record bound for the tests of its matcher (docs/adr/0092). The
//! tests are joined as every backend whose connectives are C's joins them
//! ([`JoinSpelling`]): a negation carried down to the tests, so a complement
//! is written as the comparisons that do not hold rather than as `!` before a
//! join.

use super::test_template_walk::{
    integer_literal, CaseTest, Held, JoinSpelling, Op, Position, Stands, TemplateFunction,
    TemplateSpelling, TemplateWalk, Use,
};
use super::*;
use crate::forge::schema_unit_type::{unit_type, C11Declarations, UnitNames};
use crate::forge::value_type::{ScopedRecord, ScopedUnion, TypeOf, TypeScope, ValueType};
use crate::generator::Language;

/// The crate whose bounded text, byte string and list a Rust unit holds a
/// composition in, as [`unit_type`] names it.
const PORTABLE: &str = "::sce_portable_bytes";

/// The unit of the test template `m`, whose imports are `imports`.
pub(super) fn render_test_template_rust(
    env: &minijinja::Environment,
    m: &TestTemplateModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Rust, imports);
    let scope = import_type_scope(imports);
    let spelling = RustSpelling;
    let walk = TemplateWalk {
        template: m,
        imports,
        spelling: &spelling,
    };
    walk.refuse_unlowered_unions(&scope)?;
    let typed =
        std::iter::once(&m.template_type).chain(m.parameters.iter().map(|p| &p.parameter_type));
    for ty in typed {
        if let Some(reference) = ty.enum_leaf() {
            if !scope.enums.contains_key(&reference.alias) {
                return Err(walk.refused(format!(
                    "`enum:{}` names no enum the document imports",
                    reference.alias
                )));
            }
        }
    }
    let names = RustNames { scope: &scope };
    let spell = |ty: &ValueType| {
        unit_type(Language::Rust, ty, &names, &mut C11Declarations::default())
            .map_err(|why| walk.refused(why))
    };
    let value_type = spell(&m.template_type)?;
    let parameters: Vec<serde_json::Value> = m
        .parameters
        .iter()
        .map(|p| {
            let spelled = spell(&p.parameter_type)?;
            let ty = if passed_by_value(&p.parameter_type) {
                spelled
            } else {
                format!("&{spelled}")
            };
            Ok(serde_json::json!({
                "id": p.id,
                "name": rust_parameter(&p.id),
                "ty": ty,
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

    let mut ctx = l.base_context(&m.name);
    ctx.insert("template_name".into(), m.name.clone().into());
    ctx.insert("template_type".into(), m.template_type.to_string().into());
    ctx.insert("value_type".into(), value_type.into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("match_expr".into(), match_expr.into());
    ctx.insert("value_expr".into(), serde_json::json!(value_expr));
    l.render(env, "test_template", ctx)
}

/// The name the generated `matches` and `value` take the formal parameter
/// `id` by: `param_<id>` in snake case, so that no parameter's name is one the
/// unit declares (`Value`, `value`, `matches`) or a keyword of the language,
/// and every name is one the language's own style accepts.
fn rust_parameter(id: &str) -> String {
    format!("param_{}", filters::to_snake_case(id.to_string()))
}

/// Whether a formal parameter of `ty` is passed by value: a number, a `bool`
/// and a value of an enum, and an optional one, each `Copy`. Every other type
/// is lent (`&T`).
fn passed_by_value(ty: &ValueType) -> bool {
    match ty {
        TypeOf::Scalar(_) => true,
        TypeOf::Optional { elem } => matches!(**elem, TypeOf::Scalar(_)),
        TypeOf::Text { .. }
        | TypeOf::Bytes { .. }
        | TypeOf::List { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. } => false,
    }
}

/// The type of the enum `alias` names in `scope`, through its document's
/// sibling module.
fn rust_enum_type(scope: &TypeScope, alias: &str) -> Option<String> {
    scope.enums.get(alias).map(|declared| {
        format!(
            "super::{}",
            enum_qualified_type(&declared.name, Language::Rust)
        )
    })
}

/// The payload struct of the record `record`, through its document's sibling
/// module.
fn rust_record_type(record: &ScopedRecord) -> String {
    format!(
        "super::{}",
        event_schema_payload_type(&record.name, Language::Rust)
    )
}

/// The enum the unit of the union `union` declares, through its document's
/// sibling module, as the import line of its unit names it
/// ([`forge_import_identity`]).
fn rust_union_type(union: &ScopedUnion) -> String {
    let identity = forge_import_identity(
        &union.name,
        &Language::Rust,
        false,
        &crate::ForgeCompileOptions::default(),
    );
    format!("super::{}::{}", identity.namespace, identity.type_name)
}

/// What a Rust template's unit names from outside it, read in the template's
/// own scope ([`UnitNames`]).
struct RustNames<'s> {
    scope: &'s TypeScope,
}

impl UnitNames for RustNames<'_> {
    // An enum the scope does not read is refused before a type is spelled
    // (`render_test_template_rust`); its alias stands in its place here.
    fn scalar(&self, ty: &SceType) -> String {
        match ty {
            SceType::Enum(reference) => rust_enum_type(self.scope, &reference.alias)
                .unwrap_or_else(|| reference.alias.clone()),
            other => rust_type(other).to_string(),
        }
    }
    fn record(&self, alias: &str) -> Option<String> {
        self.scope.records.get(alias).map(rust_record_type)
    }
    fn union(&self, alias: &str) -> Option<String> {
        self.scope.unions.get(alias).map(rust_union_type)
    }
    // No Rust type is named by a mangled composition.
    fn enum_mangle(&self, alias: &str) -> String {
        alias.to_string()
    }
}

/// One position of the value a matcher stands at, in Rust.
#[derive(Clone)]
struct RustAt {
    /// The value at the position as a place expression: what a comparison
    /// reads.
    place: String,
    /// What a field, a method and an index of the value are read through: the
    /// place, or the reference that holds the value.
    base: String,
    /// Whether `base` is a reference to the value rather than the value.
    by_ref: bool,
    /// How many bindings the position stands inside: the next one's number.
    depth: usize,
    /// Whether the position is a field its record's unit holds as one entry
    /// (a number, a `bool`, an enum, a text or a byte string, or an optional
    /// one), where a byte string is the language's `Vec<u8>` rather than the
    /// bounded byte string of a composition.
    entry: bool,
}

impl RustAt {
    /// A reference to the value at the position.
    fn borrowed(&self) -> String {
        if self.by_ref {
            self.base.clone()
        } else {
            format!("&{}", self.place)
        }
    }

    /// The value a pattern binds as `name`, by reference, inside one more
    /// binding than `depth`.
    fn bound(name: String, depth: usize) -> Self {
        RustAt {
            place: format!("*{name}"),
            base: name,
            by_ref: true,
            depth: depth + 1,
            entry: false,
        }
    }
}

/// The Rust spelling of a template's unit.
struct RustSpelling;

impl JoinSpelling for RustSpelling {
    /// A `bool` is compared with a literal by reading it, a byte string as the
    /// octets it holds.
    fn compare(&self, at: &Position<'_, RustAt>, op: Op, negated: bool, rhs: &str) -> String {
        let operator = op.spelled(negated);
        match &at.ty {
            TypeOf::Scalar(SceType::Bool) if rhs == "true" || rhs == "false" => {
                if (rhs == "true") == (operator == "==") {
                    at.at.place.clone()
                } else {
                    format!("!{}", at.at.place)
                }
            }
            TypeOf::Bytes { .. } => format!("{}[..] {operator} {rhs}", at.at.base),
            _ => format!("{} {operator} {rhs}", at.at.place),
        }
    }

    /// A text counts its characters; a count compared with none or one is
    /// whether the value is empty.
    fn count(&self, at: &Position<'_, RustAt>, op: Op, negated: bool, n: u32) -> String {
        let operator = op.spelled(negated);
        let base = &at.at.base;
        match (operator, n) {
            ("==" | "<=", 0) | ("<", 1) => return format!("{base}.is_empty()"),
            ("!=" | ">", 0) | (">=", 1) => return format!("!{base}.is_empty()"),
            _ => {}
        }
        let counted = match &at.ty {
            TypeOf::Text { .. } => format!("{base}.chars().count()"),
            _ => format!("{base}.len()"),
        };
        format!("{counted} {operator} {n}")
    }

    fn absence(&self, at: &RustAt, negated: bool) -> String {
        let test = if negated { "is_some" } else { "is_none" };
        format!("{}.{test}()", at.base)
    }

    /// A `match` on the optional, in parentheses as an operand.
    fn bound_optional(
        &self,
        at: &RustAt,
        inner: &RustAt,
        then: &Held<'_, RustAt>,
        absent_holds: bool,
        negated: bool,
        stands: Stands,
    ) -> String {
        let test = format!(
            "match {} {{ Some({}) => {}, None => {absent_holds} }}",
            at.borrowed(),
            inner.base,
            self.join(then, negated, Stands::Whole)
        );
        if stands == Stands::Whole {
            test
        } else {
            format!("({test})")
        }
    }

    /// `matches!` on the case, its record bound and held by the guard where
    /// the case's matcher reads it.
    fn case_test(
        &self,
        at: &RustAt,
        union: &ScopedUnion,
        variant: &str,
        record: CaseTest<'_, '_, RustAt>,
    ) -> String {
        let case = format!(
            "{}::{}",
            rust_union_type(union),
            crate::forge::composed_lowering::rust_case_ident(variant)
        );
        let scrutinee = at.borrowed();
        match record {
            CaseTest::Empty => format!("matches!({scrutinee}, {case})"),
            CaseTest::AnyRecord => format!("matches!({scrutinee}, {case}(_))"),
            CaseTest::Holds { inner, then } => format!(
                "matches!({scrutinee}, {case}({}) if {})",
                inner.base,
                self.join(then, false, Stands::Whole)
            ),
        }
    }
}

impl TemplateSpelling for RustSpelling {
    type At = RustAt;

    fn language(&self) -> Language {
        Language::Rust
    }

    /// `matches`'s parameter `value`, a `&Value`.
    fn root(&self) -> RustAt {
        RustAt {
            place: "*value".to_string(),
            base: "value".to_string(),
            by_ref: true,
            depth: 0,
            entry: false,
        }
    }

    fn present(&self, at: &RustAt) -> RustAt {
        RustAt::bound(format!("present_{}", at.depth), at.depth)
    }

    fn element(&self, at: &RustAt, index: usize) -> RustAt {
        let place = format!("{}[{index}]", at.base);
        RustAt {
            base: place.clone(),
            place,
            by_ref: false,
            depth: at.depth,
            entry: false,
        }
    }

    fn field(&self, at: &RustAt, _record: &ScopedRecord, id: &str, ty: &ValueType) -> RustAt {
        let place = format!(
            "{}.{}",
            at.base,
            event_schema_field_ident(id, Language::Rust)
        );
        RustAt {
            base: place.clone(),
            place,
            by_ref: false,
            depth: at.depth,
            entry: ty.entry().is_ok(),
        }
    }

    fn case(&self, at: &RustAt, _union: &ScopedUnion, _variant: &str) -> Result<RustAt, String> {
        Ok(RustAt::bound(format!("case_{}", at.depth), at.depth))
    }

    fn parameter(
        &self,
        name: &str,
        ty: &ValueType,
        at: &Position<'_, RustAt>,
        purpose: Use,
    ) -> String {
        let param = rust_parameter(name);
        let bytes = matches!(ty, TypeOf::Bytes { .. });
        match purpose {
            Use::Compare if bytes => format!("{param}[..]"),
            Use::Compare if passed_by_value(ty) => param,
            Use::Compare => format!("*{param}"),
            Use::Build if passed_by_value(ty) => param,
            Use::Build if bytes && at.at.entry => format!("{param}.to_vec()"),
            Use::Build => format!("{param}.clone()"),
        }
    }

    /// An integer in decimal, `true` or `false`, a variant of an enum through
    /// its type, a text, or a byte string in hex: compared as the `&str` or
    /// the octets a text and a byte string are read as, and built as the
    /// bounded text or byte string the position holds, whose bound the
    /// literal is judged against where it is written (`sce_text!`,
    /// `sce_bytes!`), or as the `Vec<u8>` of a byte string a record holds as
    /// one entry.
    fn literal(
        &self,
        text: &str,
        at: &Position<'_, RustAt>,
        purpose: Use,
    ) -> Result<String, String> {
        let refused = || format!("'{text}' is no literal of {}", at.ty);
        match &at.ty {
            TypeOf::Scalar(SceType::Bool) => match text.trim() {
                held @ ("true" | "false") => Ok(held.to_string()),
                _ => Err(refused()),
            },
            TypeOf::Scalar(SceType::Enum(reference)) => {
                let declared = rust_enum_type(at.scope, &reference.alias)
                    .ok_or_else(|| format!("`enum:{}` names no enum read", reference.alias))?;
                Ok(format!(
                    "{declared}::{}",
                    crate::forge::enum_naming::variant_ident(
                        Language::Rust,
                        &reference.alias,
                        text.trim()
                    )
                ))
            }
            TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
                integer_literal(Language::Rust, text).ok_or_else(refused)
            }
            TypeOf::Text { max } => {
                let literal = format!("\"{}\"", filters::escape_rust(text.to_string()));
                Ok(match purpose {
                    Use::Compare => literal,
                    Use::Build => format!("{PORTABLE}::sce_text!({}, {literal})", max.max),
                })
            }
            TypeOf::Bytes { max } => {
                let hex = text.trim();
                if !hex.len().is_multiple_of(2) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(refused());
                }
                let octets: String = hex
                    .as_bytes()
                    .chunks(2)
                    .map(|pair| {
                        let pair: String = pair.iter().map(|b| char::from(*b)).collect();
                        format!("\\x{}", pair.to_ascii_lowercase())
                    })
                    .collect();
                let literal = format!("b\"{octets}\"");
                Ok(match purpose {
                    Use::Compare => format!("{literal}[..]"),
                    Use::Build if at.at.entry => format!("{literal}.to_vec()"),
                    Use::Build => format!("{PORTABLE}::sce_bytes!({}, {literal})", max.max),
                })
            }
            _ => Err(refused()),
        }
    }

    fn call(
        &self,
        import: &ImportContext,
        function: TemplateFunction,
        first: Option<&RustAt>,
        args: &[String],
    ) -> String {
        let function = match function {
            TemplateFunction::Match => "matches",
            TemplateFunction::Value => "value",
        };
        let arguments: Vec<String> = first
            .map(RustAt::borrowed)
            .into_iter()
            .chain(args.iter().map(|arg| rust_parameter(arg)))
            .collect();
        format!(
            "super::{}::{function}({})",
            import.namespace,
            arguments.join(", ")
        )
    }

    fn absent(&self) -> String {
        "None".to_string()
    }

    fn present_value(&self, _at: &Position<'_, RustAt>, value: String) -> String {
        format!("Some({value})")
    }

    fn record_value(
        &self,
        _at: &Position<'_, RustAt>,
        record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| {
                format!("{}: {value}", event_schema_field_ident(&id, Language::Rust))
            })
            .collect();
        format!("{} {{ {} }}", rust_record_type(record), fields.join(", "))
    }

    /// The list of the elements, whose count is judged against the list's
    /// bound where it is written (`BoundedList::proven`).
    fn list_value(&self, _at: &Position<'_, RustAt>, elements: Vec<String>) -> String {
        format!(
            "{PORTABLE}::BoundedList::proven(&[{}])",
            elements.join(", ")
        )
    }

    fn case_value(
        &self,
        _at: &Position<'_, RustAt>,
        union: &ScopedUnion,
        variant: &str,
        record: Option<String>,
    ) -> Result<String, String> {
        let case = format!(
            "{}::{}",
            rust_union_type(union),
            crate::forge::composed_lowering::rust_case_ident(variant)
        );
        Ok(match record {
            Some(record) => format!("{case}({record})"),
            None => case,
        })
    }
}
