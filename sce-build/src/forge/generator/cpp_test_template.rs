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
//!
//! A template over a record also writes `TYPE` and `canonical`, the name and
//! the canonical form of a message of it in the test log (§12.5), whose walk
//! over the record's types every backend shares
//! ([`super::test_template_walk::canonical_value`]); [`CppCanonical`] spells
//! it with the writers of `test/Canonical.h`. Its inverse, [`CppReading`],
//! reads a value back from that form with the readers of
//! `test/FromCanonical.h`, over the same walk
//! ([`super::test_template_walk::canonical_form`]): the unit of a test
//! configuration reads the values of a recorded input trace with it
//! (docs/SCE_TTCN3_LANE.md §10.5).

use super::test_template_walk::{
    integer_literal, CanonicalCase, CanonicalForm, CanonicalSpelling, Held, Position,
    TemplateFunction, TemplateSpelling, TemplateWalk, Use,
};
use super::*;
use crate::forge::schema_unit_type::{unit_type, C11Declarations};
use crate::forge::value_type::{Bound, ScopedEnum, ScopedRecord, ScopedUnion, TypeOf, ValueType};
use crate::generator::Language;

/// The header of the test profile's runtime that declares the length of a
/// text in characters, which a length restriction on a `string[N]` counts.
const TEXT_LENGTH_HEADER: &str = "test/Length.h";
/// The function [`TEXT_LENGTH_HEADER`] declares.
const TEXT_LENGTH: &str = "SCE::Test::textLength";
/// The header of the forge runtime that declares the bounded types a specific
/// template's value builds a text, a byte string and a list through
/// (docs/adr/0086, section 7.4).
const FORGE_BOUNDED_HEADER: &str = "sce/forge/bounded.h";

/// The header of the test profile's runtime that declares the writers of the
/// canonical form of a value in the test log, which the unit of a template over
/// a record writes its `canonical` with (docs/SCE_TEST_PROFILE.md §12.5).
const CANONICAL_HEADER: &str = "test/Canonical.h";
/// The namespace of the writers [`CANONICAL_HEADER`] declares.
const CANONICAL: &str = "::SCE::Test::Canonical";

/// The unit of the test template `m`, whose imports are `imports`. Refused by
/// name where the template's type or a parameter's reaches a union at any
/// depth: C++ generates a union's unit, and its test profile matches no union
/// yet (`codegen_matrix::TEST_PROFILE_UNION_BACKENDS`,
/// [`crate::forge::value_type::union_unlowered`]).
pub(super) fn render_test_template_cpp(
    env: &minijinja::Environment,
    m: &TestTemplateModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Cpp, imports);
    let scope = import_type_scope(imports);
    let spelling = CppSpelling {
        counts_characters: std::cell::Cell::new(false),
        builds_bounded: std::cell::Cell::new(false),
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
    // Only a template over a record, the type of a message, writes a message's
    // type name and its canonical form (`TemplateWalk::message_form`).
    let (log_type, canonical_expr) = walk.message_form(&CppCanonical, &scope)?.unzip();

    let mut ctx = l.base_context(&m.name);
    ctx.insert("template_name".into(), m.name.clone().into());
    ctx.insert("template_type".into(), m.template_type.to_string().into());
    ctx.insert("value_type".into(), value_type.into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("match_expr".into(), match_expr.into());
    ctx.insert("value_expr".into(), serde_json::json!(value_expr));
    ctx.insert("canonical_expr".into(), serde_json::json!(canonical_expr));
    ctx.insert("log_type".into(), serde_json::json!(log_type));
    ctx.insert("canonical_header".into(), CANONICAL_HEADER.into());
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
    // The forge runtime's bounded types are included where the value builds
    // through one, whether or not the unit of the value's type holds one.
    ctx.insert(
        "builds_bounded".into(),
        spelling.builds_bounded.get().into(),
    );
    ctx.insert("forge_bounded_header".into(), FORGE_BOUNDED_HEADER.into());
    l.render(env, "test_template", ctx)
}

/// The name the generated `match` and `value` take the formal parameter `id`
/// by: `param_<id>`, so that no parameter's name is one the unit declares
/// (`Value`, `value`, `match`) or a keyword of the language.
fn cpp_parameter(id: &str) -> String {
    format!("param_{id}")
}

/// A value the canonical writer reads, in C++.
pub(super) struct CppCanonicalAt {
    /// What names the value: `value`, a lambda's parameter, or a field of
    /// either; each is a `const` reference, so a field is read by `.`.
    base: String,
    /// How many lambdas the value stands inside: the next one's number.
    depth: usize,
}

/// The C++ spelling of the canonical form of a value in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5): an `::SCE::Saved::JsonValue`, by the
/// writers of [`CANONICAL`]. Which form each SCE type takes is the walk's
/// ([`super::test_template_walk::canonical_value`]). A value inside a list or
/// an optional field is bound by a generic lambda the writer calls; an enum's
/// declared name and number are read by the free functions its unit declares
/// beside it (`declared_name`, `to_underlying`), found by argument-dependent
/// lookup through the enum's own namespace, whatever document declares it.
/// C++'s test profile matches no union yet, so the walk's refusal of one
/// stands. The unit of a test configuration writes its records' forms with it too, beside their
/// readers ([`CppReading`]).
pub(super) struct CppCanonical;

impl CanonicalSpelling for CppCanonical {
    type At = CppCanonicalAt;

    /// `canonical`'s parameter `value`, a `const Value &`.
    fn root(&self) -> CppCanonicalAt {
        CppCanonicalAt {
            base: "value".to_string(),
            depth: 0,
        }
    }

    fn field(&self, at: &CppCanonicalAt, id: &str) -> CppCanonicalAt {
        CppCanonicalAt {
            base: format!(
                "{}.{}",
                at.base,
                event_schema_field_ident(id, Language::Cpp)
            ),
            depth: at.depth,
        }
    }

    fn bound(&self, at: &CppCanonicalAt, prefix: &str) -> (String, CppCanonicalAt) {
        let name = format!("{prefix}_{}", at.depth);
        let value = CppCanonicalAt {
            base: name.clone(),
            depth: at.depth + 1,
        };
        (name, value)
    }

    fn boolean(&self, at: &CppCanonicalAt) -> String {
        format!("{CANONICAL}::boolean({})", at.base)
    }

    fn enumerated(&self, at: &CppCanonicalAt, strict: bool) -> String {
        if strict {
            format!("{CANONICAL}::enumerated(declared_name({}))", at.base)
        } else {
            format!(
                "{CANONICAL}::openEnumerated(declared_name({0}), to_underlying({0}))",
                at.base
            )
        }
    }

    fn integer(&self, at: &CppCanonicalAt) -> String {
        format!("{CANONICAL}::integer({})", at.base)
    }

    fn text(&self, at: &CppCanonicalAt) -> String {
        format!("{CANONICAL}::text({})", at.base)
    }

    fn bytes(&self, at: &CppCanonicalAt) -> String {
        format!("{CANONICAL}::bytes({})", at.base)
    }

    fn list(&self, at: &CppCanonicalAt, binding: &str, element: String) -> String {
        format!(
            "{CANONICAL}::list({}, [](const auto &{binding}) {{ return {element}; }})",
            at.base
        )
    }

    /// As it is: a `std::optional` of the writer's value converts from it.
    fn held_field(&self, written: String) -> String {
        written
    }

    fn optional_field(&self, field: &CppCanonicalAt, binding: &str, written: String) -> String {
        format!(
            "{CANONICAL}::optional({}, [](const auto &{binding}) {{ return {written}; }})",
            field.base
        )
    }

    /// The element's value through the same lambda as a field's, or `null`
    /// where it holds none.
    fn optional_element(&self, element: &CppCanonicalAt, binding: &str, written: String) -> String {
        format!(
            "{CANONICAL}::optionalElement({}, [](const auto &{binding}) {{ return {written}; }})",
            element.base
        )
    }

    fn record(&self, fields: Vec<(String, String)>) -> String {
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| format!("{{\"{}\", {value}}}", filters::escape_cpp(id)))
            .collect();
        format!("{CANONICAL}::record({{{}}})", fields.join(", "))
    }
}

/// The readers of the canonical form of a value, in the profile's C++ runtime
/// (`sce/include/test/FromCanonical.h`).
const FROM_CANONICAL: &str = "::SCE::Test::FromCanonical";

/// The type of the JSON value a C++ reader reads, as the generated code names
/// it.
const JSON_VALUE: &str = "::SCE::Saved::JsonValue";

/// `text` as a C++ string literal.
pub(super) fn cpp_literal(text: &str) -> String {
    format!("\"{}\"", filters::escape_cpp(text.to_string()))
}

/// A value the canonical reader reads, in C++: the JSON it is read from and
/// the type it is read into.
#[derive(Clone)]
pub(super) struct CppReadingAt {
    /// An expression of the `const ::SCE::Saved::JsonValue &` the value is
    /// read from, where it is present.
    json: String,
    /// An expression of the `const ::SCE::Saved::JsonValue *` the value is
    /// where it may be absent, null where it is: a field's member of its
    /// record's object, null where it is omitted, or any other value, null
    /// where it is JSON `null`, as an element of a list of `optional<T>` is.
    optional: String,
    /// The C++ type the value is read into, as the schema unit declares the
    /// place it stands in: the record's payload, `decltype` of a field of a
    /// record, and the `value_type` of an optional value or of a list.
    cpp: String,
    /// Where the value stands, as a refusal names it: the record's type name,
    /// then each field by its id, `[]` for an element of a list.
    what: String,
    /// How many bindings the value stands inside: the next one's number.
    depth: usize,
    /// How many records the value stands inside: the number of the next
    /// record's object.
    records: usize,
}

impl CppReadingAt {
    /// The value `json` names, read into `cpp`, where it stands at `what`,
    /// inside `depth` bindings and `records` records: absent where it is
    /// `null`, as an element of a list of `optional<T>` is.
    fn of(json: String, cpp: String, what: String, depth: usize, records: usize) -> Self {
        CppReadingAt {
            optional: format!("{FROM_CANONICAL}::nullable({json})"),
            json,
            cpp,
            what,
            depth,
            records,
        }
    }
}

/// The C++ reader of the canonical form of a value (docs/SCE_TEST_PROFILE.md
/// §12.5; docs/SCE_TTCN3_LANE.md §10.3): the value of the type the event
/// schema's unit declares, read from a `const ::SCE::Saved::JsonValue &` by
/// the readers of [`FROM_CANONICAL`], which throw `SCE::Test::TraceRefusal`,
/// naming where the value stands, where it does not belong to its type. Which
/// form each SCE type takes is the walk its writer's is
/// ([`super::test_template_walk::canonical_form`]), so the reader reads
/// exactly what [`CppCanonical`] writes.
///
/// Each read is an expression of the value: a record a lambda, called where it
/// stands, that binds its object and builds its struct by position, in the
/// order the schema declares its fields, which is the order a braced
/// initializer reads them in; a list a lambda that reads each element into a
/// `std::vector` and makes the bounded list of it; a value that may be absent
/// a lambda that answers `std::nullopt` where it is and its read otherwise. An
/// integer, a text, a byte string and a list are read into the type the place
/// they stand in holds, which the reader names as the schema unit declares it
/// (`decltype` of the field, `value_type` of an optional or a list), since a
/// C++ function infers no type from where its result goes. C++'s test profile
/// matches no union yet, so the walk's refusal of one stands.
///
/// No reference a read binds can outlive a temporary argument of the call
/// that made it (GCC's `-Wdangling-reference`): a record's object is bound as
/// the view `record()` returns by value (`CanonicalRecord`), a field read where
/// it stands is handed straight to the reader of its form, and a value that
/// may be absent is bound through the pointer to it. The one reference bound
/// to what a call returns is the range of the loop over a list, the view
/// `elements()` returns (`CanonicalList`). GCC checks a view as the reference
/// it holds, so `record()` and `elements()` take by reference only the value
/// the view points into, and the names of the fields and where the value
/// stands, which the reader spells as literals, by value.
pub(super) struct CppReading {
    /// The payload type of the record read.
    pub(super) payload: String,
    /// What the value read is named in a refusal: its record's type name.
    pub(super) what: String,
}

/// The read of a value that may be absent, of the type `cpp`: `optional`, an
/// expression of the pointer to the JSON it is, null where it is absent, bound
/// as `binding` where it is not and read as `written`.
fn cpp_read_present(optional: &str, cpp: &str, binding: &str, written: String) -> String {
    format!(
        "[&]() -> {cpp} {{ const {JSON_VALUE} *{binding}_held = {optional}; if ({binding}_held == \
         nullptr) {{ return ::std::nullopt; }} const {JSON_VALUE} &{binding} = *{binding}_held; \
         return {written}; }}()"
    )
}

impl CanonicalForm for CppReading {
    type At = CppReadingAt;

    /// The reader's parameter `value`, a `const ::SCE::Saved::JsonValue &`,
    /// read into the record's payload.
    fn root(&self) -> CppReadingAt {
        CppReadingAt::of(
            "value".to_string(),
            self.payload.clone(),
            self.what.clone(),
            0,
            0,
        )
    }

    /// The member `id` of the object its record bound: present where the
    /// field's type is not optional, and absent where it is omitted otherwise.
    fn field(&self, at: &CppReadingAt, id: &str) -> CppReadingAt {
        let object = format!("record_{}", at.records);
        let what = format!("{}.{id}", at.what);
        CppReadingAt {
            json: format!(
                "{FROM_CANONICAL}::field({object}, {}, {})",
                cpp_literal(id),
                cpp_literal(&what)
            ),
            optional: format!(
                "{FROM_CANONICAL}::optionalField({object}, {})",
                cpp_literal(id)
            ),
            cpp: format!(
                "decltype({}::{})",
                at.cpp,
                event_schema_field_ident(id, Language::Cpp)
            ),
            what,
            depth: at.depth,
            records: at.records + 1,
        }
    }

    fn bound(&self, at: &CppReadingAt, prefix: &str) -> (String, CppReadingAt) {
        let name = format!("{prefix}_{}", at.depth);
        let value = CppReadingAt::of(
            name.clone(),
            at.cpp.clone(),
            at.what.clone(),
            at.depth + 1,
            at.records,
        );
        (name, value)
    }

    fn case_record(&self, bound: CppReadingAt, _record: &str) -> CppReadingAt {
        bound
    }

    /// The element the loop over the list binds, read into the list's
    /// `value_type`.
    fn list_element(&self, list: &CppReadingAt, bound: CppReadingAt) -> CppReadingAt {
        CppReadingAt {
            cpp: format!("{}::value_type", list.cpp),
            what: format!("{}[]", list.what),
            ..bound
        }
    }

    /// The value an optional holds, read into its `value_type`.
    fn optional_value(&self, field: &CppReadingAt, bound: CppReadingAt) -> CppReadingAt {
        CppReadingAt {
            cpp: format!("{}::value_type", field.cpp),
            ..bound
        }
    }

    fn boolean(&self, at: &CppReadingAt) -> String {
        format!(
            "{FROM_CANONICAL}::boolean({}, {})",
            at.json,
            cpp_literal(&at.what)
        )
    }

    /// The variant whose declared name the text is, through the enum's own
    /// unit: a closed enum's variants, or an open enum's and the number no
    /// variant declares, which the unit's `from_underlying` makes a value of.
    fn enumerated(&self, at: &CppReadingAt, declared: &ScopedEnum) -> String {
        let identity = forge_import_identity(
            &declared.name,
            &Language::Cpp,
            false,
            &crate::ForgeCompileOptions::default(),
        );
        let enum_type = format!("::{}::{}", identity.namespace, identity.type_name);
        let variants: Vec<String> = declared
            .variants
            .iter()
            .map(|variant| {
                format!(
                    "{{{}, {enum_type}::{}}}",
                    cpp_literal(&variant.name),
                    crate::forge::enum_naming::variant_ident(
                        Language::Cpp,
                        &declared.name,
                        &variant.name
                    )
                )
            })
            .collect();
        if declared.strict {
            format!(
                "{FROM_CANONICAL}::enumerated<{enum_type}>({}, {{{}}}, {})",
                at.json,
                variants.join(", "),
                cpp_literal(&at.what)
            )
        } else {
            format!(
                "{FROM_CANONICAL}::openEnumerated<{enum_type}>({}, {{{}}}, &::{}::from_underlying, \
                 {})",
                at.json,
                variants.join(", "),
                identity.namespace,
                cpp_literal(&at.what)
            )
        }
    }

    /// Read into the integer type the place holds, which bounds it.
    fn integer(&self, at: &CppReadingAt, _width: &SceType) -> String {
        format!(
            "{FROM_CANONICAL}::integer<{}>({}, {})",
            at.cpp,
            at.json,
            cpp_literal(&at.what)
        )
    }

    fn text(&self, at: &CppReadingAt, max: &Bound) -> String {
        format!(
            "{FROM_CANONICAL}::text<{}>({}, {}u, {})",
            at.cpp,
            at.json,
            max.max,
            cpp_literal(&at.what)
        )
    }

    fn bytes(&self, at: &CppReadingAt, max: &Bound) -> String {
        format!(
            "{FROM_CANONICAL}::bytes<{}>({}, {}u, {})",
            at.cpp,
            at.json,
            max.max,
            cpp_literal(&at.what)
        )
    }

    /// A lambda that reads each element within the list's bounds into a
    /// `std::vector`, which becomes the bounded list the place holds.
    fn list(&self, at: &CppReadingAt, max: &Bound, binding: &str, element: String) -> String {
        let items = format!("items_{}", at.depth);
        let what = cpp_literal(&at.what);
        format!(
            "[&] {{ ::std::vector<{cpp}::value_type> {items}; for (const auto &{binding} : \
             {FROM_CANONICAL}::elements({json}, {min}u, {most}u, {what})) {{ \
             {items}.push_back({element}); }} return {FROM_CANONICAL}::list<{cpp}>(::std::move({items}), \
             {what}); }}()",
            cpp = at.cpp,
            json = at.json,
            min = max.min,
            most = max.max,
        )
    }

    fn held_field(&self, written: String) -> String {
        written
    }

    /// The field read where the record's object names it
    /// ([`cpp_read_present`]).
    fn optional_field(&self, field: &CppReadingAt, binding: &str, written: String) -> String {
        cpp_read_present(&field.optional, &field.cpp, binding, written)
    }

    /// The element read where it is not `null` ([`cpp_read_present`]).
    fn optional_element(&self, element: &CppReadingAt, binding: &str, written: String) -> String {
        cpp_read_present(&element.optional, &element.cpp, binding, written)
    }

    /// A lambda that binds the view of the record's object, by value, refused
    /// where it names a field the record does not declare, and builds the
    /// record's struct of its fields by position.
    fn record(
        &self,
        at: &CppReadingAt,
        record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        let ids: Vec<String> = fields.iter().map(|(id, _)| cpp_literal(id)).collect();
        let object = format!(
            "{FROM_CANONICAL}::record({}, {{{}}}, {})",
            at.json,
            ids.join(", "),
            cpp_literal(&at.what)
        );
        let members: Vec<(String, String)> = fields
            .into_iter()
            .map(|(id, value)| (event_schema_field_ident(&id, Language::Cpp), value))
            .collect();
        let built = format!(
            "::{}{{{}}}",
            event_schema_payload_type(&record.name, Language::Cpp),
            cpp_positional_init(&members)
        );
        if members.is_empty() {
            // No field reads the object, so it is not bound.
            format!("[&] {{ static_cast<void>({object}); return {built}; }}()")
        } else {
            format!(
                "[&] {{ const {FROM_CANONICAL}::CanonicalRecord record_{} = {object}; return {built}; \
                 }}()",
                at.records
            )
        }
    }

    fn union(
        &self,
        _at: &CppReadingAt,
        union: &ScopedUnion,
        _cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        Err(crate::forge::value_type::union_unlowered(&union.name))
    }
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
    /// Whether the value builds a text, a byte string or a list through the
    /// forge runtime's bounded type of it, which [`FORGE_BOUNDED_HEADER`]
    /// declares.
    builds_bounded: std::cell::Cell<bool>,
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

    /// `built`, an expression that builds through the forge runtime's
    /// bounded type, noted so the unit includes [`FORGE_BOUNDED_HEADER`].
    fn bounded(&self, built: String) -> String {
        self.builds_bounded.set(true);
        built
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
    /// text, or a byte string in hex. A text and a byte string compare as the
    /// literal they are, and are built as the runtime's bounded type of their
    /// bound (`OwnedText`, `OwnedBytes`; docs/adr/0086, section 7.4), whose
    /// `literal` the compiler judges against it; a field of the schema unit
    /// that holds a byte string in a `std::vector` takes the bounded one as the
    /// vector it holds.
    fn literal(
        &self,
        text: &str,
        at: &Position<'_, CppAt>,
        purpose: Use,
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
            TypeOf::Text { max } => Ok(match purpose {
                Use::Compare => filters::cpp_text_expression(text),
                Use::Build => self.bounded(format!(
                    "{}::literal({})",
                    crate::forge::composed_lowering::cpp_text_type(max.max),
                    crate::forge::expr::cpp_text_literal(text)
                )),
            }),
            TypeOf::Bytes { max } => {
                let octets: Vec<String> = text
                    .trim()
                    .as_bytes()
                    .chunks(2)
                    .map(|pair| pair.iter().map(|b| char::from(*b)).collect::<String>())
                    .map(|pair| format!("0x{pair}"))
                    .collect();
                let held = crate::forge::composed_lowering::cpp_bytes_type(max.max);
                Ok(match (purpose, octets.is_empty()) {
                    (Use::Compare, _) => {
                        format!("std::vector<std::uint8_t>{{{}}}", octets.join(", "))
                    }
                    (Use::Build, true) => self.bounded(format!("{held}{{}}")),
                    (Use::Build, false) => {
                        self.bounded(format!("{held}::literal({{{}}})", octets.join(", ")))
                    }
                })
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

    /// The list the schema unit holds, `OwnedList<T, N>` (docs/adr/0086, section
    /// 7.4): built by its `literal`, whose count the compiler judges against
    /// the bound, and an empty one by its default.
    fn list_value(&self, at: &Position<'_, CppAt>, elements: Vec<String>) -> String {
        if elements.is_empty() {
            format!("{}{{}}", at.at.cpp)
        } else {
            self.bounded(format!(
                "{}::literal({{{}}})",
                at.at.cpp,
                elements.join(", ")
            ))
        }
    }
}
