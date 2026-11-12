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
//!
//! A template over a record also writes `TYPE` and `canonical`, the name and
//! the canonical form of a message of it in the test log (§12.5), whose walk
//! over the record's types every backend shares
//! ([`super::test_template_walk::canonical_value`]); [`RustCanonical`] spells
//! it with the writers of `sce_rust_runtime::test::canonical`. Its inverse,
//! [`RustReading`], reads a value back from that form with the readers of
//! `sce_rust_runtime::test::from_canonical`, over the same walk
//! ([`super::test_template_walk::canonical_form`]): the unit of a test
//! configuration reads the values of a recorded input trace with it
//! (docs/SCE_TTCN3_LANE.md §10.5).

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
    walk.refuse_unread_enums(&scope)?;
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
    // Only a template over a record, the type of a message, writes a message's
    // type name and its canonical form (`TemplateWalk::message_form`).
    let (log_type, canonical_expr) = walk.message_form(&RustCanonical, &scope)?.unzip();

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

/// The writers of the canonical form of a value in the test log, in the
/// profile's Rust runtime (`backends/rust/runtime/src/test/canonical.rs`).
const CANONICAL: &str = "::sce_rust_runtime::test::canonical";

/// A value the canonical writer reads, in Rust.
pub(super) struct Canonical {
    /// What names the value: `value`, a binding, or a field of either.
    base: String,
    /// Whether `base` is a reference to the value rather than the value.
    by_ref: bool,
    /// How many bindings the value stands inside: the next one's number.
    depth: usize,
}

impl Canonical {
    /// The value itself, for a type that is `Copy`.
    fn copied(&self) -> String {
        if self.by_ref {
            format!("*{}", self.base)
        } else {
            self.base.clone()
        }
    }

    /// A reference to the value.
    fn borrowed(&self) -> String {
        if self.by_ref {
            self.base.clone()
        } else {
            format!("&{}", self.base)
        }
    }
}

/// The Rust spelling of the canonical form of a value in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5): a `::sce_rust_runtime::json::Value`, by
/// the writers of [`CANONICAL`]. Which form each SCE type takes is the walk's
/// ([`super::test_template_walk::canonical_value`]). The unit of a test
/// configuration writes its records' forms with it too, beside their readers
/// ([`RustReading`]).
pub(super) struct RustCanonical;

impl CanonicalSpelling for RustCanonical {
    type At = Canonical;

    /// `canonical`'s parameter `value`, a `&Value`.
    fn root(&self) -> Canonical {
        Canonical {
            base: "value".to_string(),
            by_ref: true,
            depth: 0,
        }
    }

    fn field(&self, at: &Canonical, id: &str) -> Canonical {
        Canonical {
            base: format!(
                "{}.{}",
                at.base,
                event_schema_field_ident(id, Language::Rust)
            ),
            by_ref: false,
            depth: at.depth,
        }
    }

    /// A closure or a pattern binds the value by reference.
    fn bound(&self, at: &Canonical, prefix: &str) -> (String, Canonical) {
        let name = format!("{prefix}_{}", at.depth);
        let value = Canonical {
            base: name.clone(),
            by_ref: true,
            depth: at.depth + 1,
        };
        (name, value)
    }

    fn boolean(&self, at: &Canonical) -> String {
        format!("{CANONICAL}::boolean({})", at.copied())
    }

    fn enumerated(&self, at: &Canonical, strict: bool) -> String {
        if strict {
            format!("{CANONICAL}::enumerated({}.declared_name())", at.base)
        } else {
            format!(
                "{CANONICAL}::open_enumerated({0}.declared_name(), {0}.to_underlying())",
                at.base
            )
        }
    }

    fn integer(&self, at: &Canonical) -> String {
        format!("{CANONICAL}::integer({})", at.copied())
    }

    fn text(&self, at: &Canonical) -> String {
        format!("{CANONICAL}::text({}.as_str())", at.base)
    }

    fn bytes(&self, at: &Canonical) -> String {
        format!("{CANONICAL}::bytes({}.as_slice())", at.base)
    }

    fn list(&self, at: &Canonical, binding: &str, element: String) -> String {
        format!(
            "{CANONICAL}::list({}.as_slice().iter().map(|{binding}| {element}))",
            at.base
        )
    }

    fn held_field(&self, written: String) -> String {
        format!("Some({written})")
    }

    fn optional_field(&self, field: &Canonical, binding: &str, written: String) -> String {
        format!("{}.as_ref().map(|{binding}| {written})", field.base)
    }

    /// The element's value read as a field's is, which the runtime's
    /// `optional_element` writes, or `null` where it holds none. The element
    /// is bound by reference, so `as_ref` reads it through the binding.
    fn optional_element(&self, element: &Canonical, binding: &str, written: String) -> String {
        format!(
            "{CANONICAL}::optional_element({}.as_ref().map(|{binding}| {written}))",
            element.base
        )
    }

    fn record(&self, fields: Vec<(String, String)>) -> String {
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| format!("(\"{}\", {value})", filters::escape_rust(id)))
            .collect();
        format!("{CANONICAL}::record([{}])", fields.join(", "))
    }

    /// A `match` on the value, an arm per case, the case's record bound by
    /// reference.
    fn union(
        &self,
        at: &Canonical,
        union: &ScopedUnion,
        cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        let union_type = rust_union_type(union);
        let arms: Vec<String> = cases
            .into_iter()
            .map(|(variant, held)| {
                let pattern = format!(
                    "{union_type}::{}",
                    crate::forge::composed_lowering::rust_case_ident(&variant)
                );
                let name = filters::escape_rust(variant);
                match held {
                    Some((binding, written)) => format!(
                        "{pattern}({binding}) => {CANONICAL}::union_case(\"{name}\", \
                         Some({written}))"
                    ),
                    None => format!("{pattern} => {CANONICAL}::union_case(\"{name}\", None)"),
                }
            })
            .collect();
        Ok(format!("match {} {{ {} }}", at.borrowed(), arms.join(", ")))
    }
}

/// The readers of the canonical form of a value, in the profile's Rust runtime
/// (`backends/rust/runtime/src/test/from_canonical.rs`).
const FROM_CANONICAL: &str = "::sce_rust_runtime::test::from_canonical";

/// `text` as a Rust string literal.
pub(super) fn rust_literal(text: &str) -> String {
    format!("\"{}\"", filters::escape_rust(text.to_string()))
}

/// A value the canonical reader reads, in Rust: the JSON it is read from.
#[derive(Clone)]
pub(super) struct Reading {
    /// An expression of the `&::sce_rust_runtime::json::Value` the value is
    /// read from, where it is present.
    json: String,
    /// An expression of the `Option<&::sce_rust_runtime::json::Value>` the
    /// value is where it may be absent: a field's member of its record's
    /// object, which is absent where it is omitted, or any other value, which
    /// is absent where it is `null`.
    optional: String,
    /// Where the value stands, as a refusal names it: the record's type name,
    /// then each field by its id, `[]` for an element of a list.
    what: String,
    /// How many bindings the value stands inside: the next one's number.
    depth: usize,
    /// How many records the value stands inside: the number of the next
    /// record's object.
    records: usize,
}

impl Reading {
    /// The value `json` names, where it stands at `what`, inside `depth`
    /// bindings and `records` records: absent where it is `null`, as an element
    /// of a list of `optional<T>` is.
    fn of(json: String, what: String, depth: usize, records: usize) -> Self {
        Reading {
            optional: format!("{FROM_CANONICAL}::nullable({json})"),
            json,
            what,
            depth,
            records,
        }
    }
}

/// The Rust reader of the canonical form of a value (docs/SCE_TEST_PROFILE.md
/// §12.5; docs/SCE_TTCN3_LANE.md §10.3): the value of the type the event
/// schema's unit declares, read from a `&::sce_rust_runtime::json::Value` by
/// the readers of [`FROM_CANONICAL`] and refused, naming where it stands, where
/// it does not belong to its type. Which form each SCE type takes is the walk
/// its writer's is ([`super::test_template_walk::canonical_form`]), so the
/// reader reads exactly what [`RustCanonical`] writes.
///
/// Each read is an expression of the value, whose refusal returns from the
/// reader with `?`: a record is a block that binds its object and builds its
/// struct, a list a `for` loop over the elements that reads each into a `Vec`
/// and makes the bounded list of it, a value that may be absent an `Option`
/// mapped to its read and transposed ([`read_present`]), a union a `match` on
/// its case, whose variant the runtime has held to the declared ones. An
/// integer, a text, a byte string and a list are read into the type the place
/// they stand in holds, which Rust infers.
///
/// The generated code compiles under clippy's `-D warnings` by its shape, not
/// by an `allow`: no `match` maps `Some` to `Some` and `None` to `None`
/// (`manual_map`), no `return` is written (`needless_return`), no iterator is
/// stepped by hand (`while_let_on_iterator`), and no `Ok` wraps a `?`
/// (`needless_question_mark`).
pub(super) struct RustReading {
    /// What the value read is named in a refusal: its record's type name.
    pub(super) what: String,
}

impl CanonicalForm for RustReading {
    type At = Reading;

    /// The reader's parameter `value`, a `&::sce_rust_runtime::json::Value`.
    fn root(&self) -> Reading {
        Reading::of("value".to_string(), self.what.clone(), 0, 0)
    }

    /// The member `id` of the object its record bound: present where the
    /// field's type is not optional, and absent where it is omitted otherwise.
    fn field(&self, at: &Reading, id: &str) -> Reading {
        let object = format!("record_{}", at.records);
        let what = format!("{}.{id}", at.what);
        Reading {
            json: format!(
                "{FROM_CANONICAL}::field({object}, {}, {})?",
                rust_literal(id),
                rust_literal(&what)
            ),
            optional: format!(
                "{FROM_CANONICAL}::optional_field({object}, {})",
                rust_literal(id)
            ),
            what,
            depth: at.depth,
            records: at.records + 1,
        }
    }

    fn bound(&self, at: &Reading, prefix: &str) -> (String, Reading) {
        let name = format!("{prefix}_{}", at.depth);
        let value = Reading::of(name.clone(), at.what.clone(), at.depth + 1, at.records);
        (name, value)
    }

    fn case_record(&self, bound: Reading, _record: &str) -> Reading {
        bound
    }

    fn list_element(&self, list: &Reading, bound: Reading) -> Reading {
        Reading {
            what: format!("{}[]", list.what),
            ..bound
        }
    }

    fn optional_value(&self, _field: &Reading, bound: Reading) -> Reading {
        bound
    }

    fn boolean(&self, at: &Reading) -> String {
        format!(
            "{FROM_CANONICAL}::boolean({}, {})?",
            at.json,
            rust_literal(&at.what)
        )
    }

    /// The variant whose declared name the text is, through the enum's
    /// sibling module: a closed enum's variants, or an open enum's constants
    /// and the number no constant declares.
    fn enumerated(&self, at: &Reading, declared: &ScopedEnum) -> String {
        let enum_type = format!(
            "super::{}",
            enum_qualified_type(&declared.name, Language::Rust)
        );
        let variants: Vec<String> = declared
            .variants
            .iter()
            .map(|variant| {
                format!(
                    "({}, {enum_type}::{})",
                    rust_literal(&variant.name),
                    crate::forge::enum_naming::variant_ident(
                        Language::Rust,
                        &declared.name,
                        &variant.name
                    )
                )
            })
            .collect();
        if declared.strict {
            format!(
                "{FROM_CANONICAL}::enumerated({}, &[{}], {})?",
                at.json,
                variants.join(", "),
                rust_literal(&at.what)
            )
        } else {
            format!(
                "{FROM_CANONICAL}::open_enumerated({}, &[{}], {enum_type}::from_underlying, {})?",
                at.json,
                variants.join(", "),
                rust_literal(&at.what)
            )
        }
    }

    /// Read into the integer type the place holds, which bounds it.
    fn integer(&self, at: &Reading, _width: &SceType) -> String {
        format!(
            "{FROM_CANONICAL}::integer({}, {})?",
            at.json,
            rust_literal(&at.what)
        )
    }

    fn text(&self, at: &Reading, max: &Bound) -> String {
        format!(
            "{FROM_CANONICAL}::text({}, {}, {})?",
            at.json,
            max.max,
            rust_literal(&at.what)
        )
    }

    fn bytes(&self, at: &Reading, max: &Bound) -> String {
        format!(
            "{FROM_CANONICAL}::bytes({}, {}, {})?",
            at.json,
            max.max,
            rust_literal(&at.what)
        )
    }

    /// A loop over the elements within the list's bounds, each read into a
    /// `Vec`, which becomes the bounded list the place holds.
    fn list(&self, at: &Reading, max: &Bound, binding: &str, element: String) -> String {
        let items = format!("items_{}", at.depth);
        let what = rust_literal(&at.what);
        format!(
            "{{ let mut {items} = ::std::vec::Vec::new(); \
             for {binding} in {FROM_CANONICAL}::elements({}, {}, {}, {what})? {{ \
             {items}.push({element}); }} \
             {FROM_CANONICAL}::list({items}, {what})? }}",
            at.json, max.min, max.max
        )
    }

    fn held_field(&self, written: String) -> String {
        written
    }

    /// The field read where the record's object names it ([`read_present`]).
    fn optional_field(&self, field: &Reading, binding: &str, written: String) -> String {
        read_present(&field.optional, binding, written)
    }

    /// The element read where it is not `null` ([`read_present`]).
    fn optional_element(&self, element: &Reading, binding: &str, written: String) -> String {
        read_present(&element.optional, binding, written)
    }

    /// A block that binds the record's object, refused where it names a field
    /// the record does not declare, and builds the record's struct of its
    /// fields.
    fn record(&self, at: &Reading, record: &ScopedRecord, fields: Vec<(String, String)>) -> String {
        let ids: Vec<String> = fields.iter().map(|(id, _)| rust_literal(id)).collect();
        let members: Vec<String> = fields
            .iter()
            .map(|(id, value)| format!("{}: {value}", event_schema_field_ident(id, Language::Rust)))
            .collect();
        let object = format!(
            "{FROM_CANONICAL}::record({}, &[{}], {})?",
            at.json,
            ids.join(", "),
            rust_literal(&at.what)
        );
        let built = format!("{} {{ {} }}", rust_record_type(record), members.join(", "));
        if fields.is_empty() {
            // No field reads the object, so it is not bound.
            format!("{{ {object}; {built} }}")
        } else {
            format!("{{ let record_{} = {object}; {built} }}", at.records)
        }
    }

    /// A `match` on the variant of the case, which the runtime has held to the
    /// declared ones, so each case has an arm and the last is the wildcard;
    /// each arm reads the case's record or, for a case that holds none, the
    /// empty object it is. A union of one case is that case's read alone.
    fn union(
        &self,
        at: &Reading,
        union: &ScopedUnion,
        cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        let union_type = rust_union_type(union);
        let variant = format!("variant_{}", at.depth);
        let case = format!("case_{}", at.depth);
        let names: Vec<String> = cases.iter().map(|(name, _)| rust_literal(name)).collect();
        let reads: Vec<(String, String)> = cases
            .into_iter()
            .map(|(name, held)| {
                let built = format!(
                    "{union_type}::{}",
                    crate::forge::composed_lowering::rust_case_ident(&name)
                );
                let read = match held {
                    Some((_, written)) => format!("{built}({written})"),
                    None => format!(
                        "{{ {FROM_CANONICAL}::no_record({case}, {})?; {built} }}",
                        rust_literal(&format!("{}.{name}", at.what))
                    ),
                };
                (rust_literal(&name), read)
            })
            .collect();
        let read_case = format!(
            "{FROM_CANONICAL}::union_case({}, &[{}], {})?",
            at.json,
            names.join(", "),
            rust_literal(&at.what)
        );
        let Some(((_, last), arms)) = reads.split_last() else {
            return Err(format!("the union '{}' declares no case", union.name));
        };
        if arms.is_empty() {
            return Ok(format!("{{ let (_, {case}) = {read_case}; {last} }}"));
        }
        let arms: Vec<String> = arms
            .iter()
            .map(|(name, read)| format!("{name} => {read}"))
            .collect();
        Ok(format!(
            "{{ let ({variant}, {case}) = {read_case}; match {variant} {{ {}, _ => {last} }} }}",
            arms.join(", ")
        ))
    }
}

/// The type of a refusal of the readers, as the generated code names it.
const REFUSAL: &str = "::sce_rust_runtime::test::TraceRefusal";

/// The read of a value that may be absent: `optional`, an expression of the
/// `Option<&Value>` it is, mapped where it is present, bound as `binding`, to
/// `written`, its read. The read is the body of a closure that returns its
/// value whole, so a `?` inside it returns the refusal from the closure, which
/// `transpose` and the `?` after it return from the reader; the body names the
/// value it read before it is wrapped, so no `Ok` holds a `?`.
fn read_present(optional: &str, binding: &str, written: String) -> String {
    format!(
        "{optional}.map(|{binding}| -> ::std::result::Result<_, {REFUSAL}> {{ \
         let read = {written}; Ok(read) }}).transpose()?"
    )
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
pub(super) fn rust_record_type(record: &ScopedRecord) -> String {
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
    /// the case's matcher reads it; `!` before it where `negated`.
    fn case_test(
        &self,
        at: &RustAt,
        union: &ScopedUnion,
        variant: &str,
        record: CaseTest<'_, '_, RustAt>,
        negated: bool,
    ) -> String {
        let case = format!(
            "{}::{}",
            rust_union_type(union),
            crate::forge::composed_lowering::rust_case_ident(variant)
        );
        let scrutinee = at.borrowed();
        let test = match record {
            CaseTest::Empty => format!("matches!({scrutinee}, {case})"),
            CaseTest::AnyRecord => format!("matches!({scrutinee}, {case}(_))"),
            CaseTest::Holds { inner, then } => format!(
                "matches!({scrutinee}, {case}({}) if {})",
                inner.base,
                self.join(then, false, Stands::Whole)
            ),
        };
        negate(test, negated)
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
                let octets: String = hex_octets(text)
                    .ok_or_else(refused)?
                    .into_iter()
                    .map(|pair| format!("\\x{pair}"))
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

    fn absent(&self, _at: &Position<'_, RustAt>) -> String {
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

#[cfg(test)]
mod tests {
    use super::super::test_template_walk::canonical_form;
    use super::*;
    use crate::forge::value_type::{ScopedCase, ScopedVariant};

    /// The readers of the runtime, as the generated code names them.
    fn read(function: &str) -> String {
        format!("{FROM_CANONICAL}::{function}")
    }

    fn variant(name: &str, value: i128) -> ScopedVariant {
        ScopedVariant {
            name: name.to_string(),
            value,
        }
    }

    /// The record `order` of every form a field takes, and its scope: a text,
    /// a byte string, an optional `bool`, a list of optional integers, a value
    /// of a closed and of an open enum, and a union of a case that holds a
    /// record and one that holds none.
    fn order() -> (ValueType, TypeScope) {
        let paint = |strict: bool| ScopedEnum {
            name: if strict { "paint_color" } else { "paint_code" }.to_string(),
            variants: vec![variant("red", 0)],
            underlying: SceType::Uint8,
            strict,
        };
        let card = ScopedRecord {
            identity: "card.scxml".to_string(),
            name: "card".to_string(),
            fields: vec![("amount".to_string(), TypeOf::Scalar(SceType::Uint32))],
            scope: TypeScope::default(),
        };
        let mut union_scope = TypeScope::default();
        union_scope.records.insert("Card".to_string(), card);
        let payment = ScopedUnion {
            identity: "payment.scxml".to_string(),
            name: "payment".to_string(),
            tag: "kind".to_string(),
            tag_enum: "Kind".to_string(),
            cases: vec![
                ScopedCase {
                    variant: variant("card", 0),
                    record: Some("Card".to_string()),
                },
                ScopedCase {
                    variant: variant("cash", 1),
                    record: None,
                },
            ],
            scope: union_scope,
        };
        let mut scope = TypeScope::default();
        scope.enums.insert("Paint".to_string(), paint(true));
        scope.enums.insert("Code".to_string(), paint(false));
        scope.unions.insert("Payment".to_string(), payment);
        let enum_of = |alias: &str| {
            TypeOf::Scalar(SceType::Enum(EnumRef {
                alias: alias.to_string(),
            }))
        };
        let order = ScopedRecord {
            identity: "order.scxml".to_string(),
            name: "order".to_string(),
            fields: vec![
                ("note".to_string(), TypeOf::Text { max: Bound::of(8) }),
                ("key".to_string(), TypeOf::Bytes { max: Bound::of(4) }),
                (
                    "rush".to_string(),
                    TypeOf::Optional {
                        elem: Box::new(TypeOf::Scalar(SceType::Bool)),
                    },
                ),
                (
                    "tags".to_string(),
                    TypeOf::List {
                        elem: Box::new(TypeOf::Optional {
                            elem: Box::new(TypeOf::Scalar(SceType::Uint8)),
                        }),
                        max: Bound::of(2),
                    },
                ),
                ("color".to_string(), enum_of("Paint")),
                ("code".to_string(), enum_of("Code")),
                (
                    "payment".to_string(),
                    TypeOf::Union {
                        alias: "Payment".to_string(),
                    },
                ),
            ],
            scope,
        };
        let mut outer = TypeScope::default();
        outer.records.insert("Order".to_string(), order);
        (
            TypeOf::Record {
                alias: "Order".to_string(),
            },
            outer,
        )
    }

    /// The reader of a record reads each form through the runtime's reader of
    /// it, every place named in a refusal, every `?` returning from the
    /// reader itself: the record's object bound once, an optional field and an
    /// optional element by a `match` on their presence, a list by a loop into
    /// the bounded list its place holds, a union by a `match` on its case.
    #[test]
    fn a_record_is_read_through_the_reader_of_each_form() {
        let (ty, scope) = order();
        let reading = RustReading {
            what: "order".to_string(),
        };
        let written = canonical_form(&reading, &ty, &reading.root(), &scope)
            .expect("every form has a reader");
        let field = |id: &str| format!("{}(record_0, \"{id}\", \"order.{id}\")?", read("field"));
        let paint = format!(
            "super::{}",
            enum_qualified_type("paint_color", Language::Rust)
        );
        let code = format!(
            "super::{}",
            enum_qualified_type("paint_code", Language::Rust)
        );
        let red = crate::forge::enum_naming::variant_ident(Language::Rust, "paint_color", "red");
        let payment = rust_union_type(&scope.records["Order"].scope.unions["Payment"]);
        let case = |name: &str| crate::forge::composed_lowering::rust_case_ident(name);
        for needle in [
            format!(
                "{{ let record_0 = {}(value, &[\"note\", \"key\", \"rush\", \"tags\", \"color\", \
                 \"code\", \"payment\"], \"order\")?; super::order::OrderPayload {{ ",
                read("record")
            ),
            format!(
                "note: {}({}, 8, \"order.note\")?",
                read("text"),
                field("note")
            ),
            format!(
                "key: {}({}, 4, \"order.key\")?",
                read("bytes"),
                field("key")
            ),
            format!(
                "rush: {}(record_0, \"rush\").map(|present_0| -> ::std::result::Result<_, \
                 {REFUSAL}> {{ let read = {}(present_0, \"order.rush\")?; Ok(read) }})\
                 .transpose()?",
                read("optional_field"),
                read("boolean")
            ),
            format!(
                "tags: {{ let mut items_0 = ::std::vec::Vec::new(); for item_0 in {}({}, 0, 2, \
                 \"order.tags\")? {{ items_0.push({}(item_0).map(|present_1| -> \
                 ::std::result::Result<_, {REFUSAL}> {{ let read = {}(present_1, \
                 \"order.tags[]\")?; Ok(read) }}).transpose()?); }} {}(items_0, \
                 \"order.tags\")? }}",
                read("elements"),
                field("tags"),
                read("nullable"),
                read("integer"),
                read("list")
            ),
            format!(
                "color: {}({}, &[(\"red\", {paint}::{red})], \"order.color\")?",
                read("enumerated"),
                field("color")
            ),
            format!(
                "code: {}({}, &[(\"red\", {code}::{red})], {code}::from_underlying, \
                 \"order.code\")?",
                read("open_enumerated"),
                field("code")
            ),
            format!(
                "payment: {{ let (variant_0, case_0) = {}({}, &[\"card\", \"cash\"], \
                 \"order.payment\")?; match variant_0 {{ \"card\" => {payment}::{}({{ let \
                 record_1 = {}(case_0, &[\"amount\"], \"order.payment\")?; \
                 super::card::CardPayload {{ amount: {}({}(record_1, \"amount\", \
                 \"order.payment.amount\")?, \"order.payment.amount\")? }} }}), _ => {{ \
                 {}(case_0, \"order.payment.cash\")?; {payment}::{} }} }} }}",
                read("union_case"),
                field("payment"),
                case("card"),
                read("record"),
                read("integer"),
                read("field"),
                read("no_record"),
                case("cash")
            ),
        ] {
            assert!(written.contains(&needle), "`{needle}` in\n{written}");
        }
        // The shapes clippy's `-D warnings` refuses are not written.
        for refused in [
            "return",
            "None => None",
            "while let",
            "Ok(read?)",
            "Some(Some",
        ] {
            assert!(!written.contains(refused), "`{refused}` in\n{written}");
        }
    }

    /// A union of one case is that case's read alone: a `match` of one arm is
    /// what clippy refuses as `match_single_binding`.
    #[test]
    fn a_union_of_one_case_is_read_without_a_match() {
        let (ty, mut scope) = order();
        let order = scope.records.get_mut("Order").expect("the order");
        let payment = order.scope.unions.get_mut("Payment").expect("the payment");
        payment.cases.truncate(1);
        let reading = RustReading {
            what: "order".to_string(),
        };
        let written = canonical_form(&reading, &ty, &reading.root(), &scope)
            .expect("every form has a reader");
        let union = format!(
            "payment: {{ let (_, case_0) = {}({}(record_0, \"payment\", \"order.payment\")?, \
             &[\"card\"], \"order.payment\")?; {}::{}({{ let record_1",
            read("union_case"),
            read("field"),
            rust_union_type(&scope.records["Order"].scope.unions["Payment"]),
            crate::forge::composed_lowering::rust_case_ident("card")
        );
        assert!(written.contains(&union), "`{union}` in\n{written}");
        assert!(!written.contains("match variant_0"), "{written}");
    }
}
