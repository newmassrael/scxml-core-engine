// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C11 unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.2): `<name>_value_t`, the type the template matches;
//! `<name>_value_type`, the name a message's value of that type is known by
//! (`sce_host_value_t::type`), the type's own and not the alias, so every
//! template of one type names it alike;
//! `<name>_matches`, whether a value matches it (ES 201 873-1 Annex B.1); and,
//! for a specific template (§6.2), `<name>_value`, the one value it sends. C has
//! no namespace, so each declaration carries the document's name, as the units
//! of the other C11 kinds do, and the value matched is lent by pointer, since a
//! record of the C11 event schema unit is laid out whole.
//!
//! What each matcher holds is decided by the walk every backend shares
//! ([`super::test_template_walk`]), and its tests are joined by the join every
//! backend whose connectives are C's shares ([`JoinSpelling`]); this file
//! spells the leaves in C11, and the template (`forge/c/test_template.h.jinja2`)
//! writes the declarations around the two expressions.
//!
//! Every type is the one the event schema's unit spells a field of that type
//! in: one entry where the type is one (a number, `bool`, a value of an enum, a
//! text in the buffer of its bound, an optional one in the runtime's
//! `sce_optional_<elem>_t`), and a composition laid out statically where it is
//! not ([`unit_type`]), so a template's value, and the value a reference passes
//! the template it names, is the field's own type. A position inside a list is
//! the list's element, which a composition holds in its own types: there a byte
//! string holds its length, and an optional value is the composition's.
//!
//! A text is compared by its bytes up to its terminator (`strcmp`) and its
//! length restriction counts characters through the profile's runtime
//! ([`TEXT_LENGTH`]); a byte string is compared by its length and its octets
//! (`memcmp`); a value that may be absent is tested by `present` and read by
//! `value` behind that test; and an optional value a parameter gives is
//! compared by its two parts, since C compares no two structs (a specific
//! value stands for no list, record or union, `test_profile::matcher_admitted`).
//! A value is built as the initializer of the variable `<name>_value`
//! returns, each record by its fields' designators, so no type is named inside
//! it; a text through the fill function of its buffer.
//!
//! A template over a record also writes `<name>_canonical` and
//! `<name>_record_type`, the canonical form of a message of it in the test log
//! and the record type a send carries it by (§12.5), whose walk is shared
//! ([`super::test_template_walk::canonical_value`]); [`C11Canonical`] spells it
//! as the statements that write it through the runtime's `sce_jcs_t`.
//!
//! What C11 refuses by name: a union, which it lowers nowhere
//! ([`crate::forge::codegen_matrix::UNION_BACKENDS`]), by the walk before
//! anything is spelled; a byte string of one entry, which the C11 event schema
//! unit holds as a pointer with no length, so no value of it can be compared,
//! built or written in the test log, and so no template over a record that
//! holds one; and a reference, inside a list, to a template of an optional
//! type, whose value the list holds in its own optional type rather than the
//! one the referenced template takes.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

use super::test_template_walk::{
    hex_octets, integer_literal, negate, CanonicalSpelling, CaseTest, Held, JoinSpelling, Op,
    Position, Stands, TemplateFunction, TemplateSpelling, TemplateWalk, Use,
};
use super::*;
use crate::forge::schema_unit_type::{unit_type, C11Declarations, UnitNames};
use crate::forge::static_lowering::{
    c_string_storage_def, c_string_storage_of, c_string_storage_type,
};
use crate::forge::value_type::{ScopedEnum, ScopedRecord, ScopedUnion, TypeOf, ValueType};
use crate::generator::Language;

/// The header of the test profile's C11 runtime that declares the length of a
/// text in characters, which a length restriction on a `string[N]` counts.
const TEXT_LENGTH_HEADER: &str = "sce/test/text_length.h";

/// The function [`TEXT_LENGTH_HEADER`] declares: a text's code points, as C++'s
/// `SCE::Test::textLength` counts them.
const TEXT_LENGTH: &str = "sce_test_text_length";

/// The header of the test profile's C11 runtime that declares the writers of
/// the canonical form of a value in the test log and the record type a message
/// carries, which the unit of a template over a record writes its
/// `<name>_canonical` and its `<name>_record_type` with
/// (docs/SCE_TEST_PROFILE.md §12.5).
const CANONICAL_HEADER: &str = "sce/test/canonical.h";

/// The writer `<name>_canonical` is handed, which each of its statements
/// writes through: a `sce_jcs_t *`.
const C_JCS: &str = "jcs";

/// The unit of the test template `m`, whose imports are `imports`.
pub(super) fn render_test_template_c11(
    env: &minijinja::Environment,
    m: &TestTemplateModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::C11, imports);
    let scope = import_type_scope(imports);
    let spelling = CSpelling::default();
    let walk = TemplateWalk {
        template: m,
        imports,
        spelling: &spelling,
    };
    walk.refuse_unlowered_unions(&scope)?;
    walk.refuse_unread_enums(&scope)?;
    let names = SchemaUnitNames { l: &l, imports };
    let mut held = HeldTypes::default();
    let value_type = held.of(&walk, &l, imports, &names, &m.template_type)?;
    let mut parameters = Vec::new();
    for p in &m.parameters {
        let ty = held.of(&walk, &l, imports, &names, &p.parameter_type)?;
        parameters.push(serde_json::json!({
            "id": p.id,
            "name": c_parameter(&p.id),
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
    // record type and its value's canonical form (`TemplateWalk::message_form`).
    let canonical = C11Canonical::default();
    let (log_type, canonical_body) = walk.message_form(&canonical, &scope)?.unzip();
    let refusal = canonical.refusal.borrow_mut().take();
    if let Some(why) = refusal {
        return Err(walk.refused(why));
    }

    let mut ctx = l.base_context(&m.name);
    ctx.insert(
        "prefix".into(),
        filters::to_snake_case(m.name.clone()).into(),
    );
    ctx.insert("template_name".into(), m.name.clone().into());
    ctx.insert("template_type".into(), m.template_type.to_string().into());
    // The name a message's value of the type is known by (`sce_host_value_t::type`),
    // which the runtime compares by its text: the type the unit of the
    // template's type declares, and not this unit's own alias of it, so that a
    // value one template's sender writes is one another template of the same
    // type receives.
    ctx.insert(
        "value_type_name".into(),
        format!("\"{}\"", filters::escape_c(value_type.clone())).into(),
    );
    ctx.insert("value_type".into(), value_type.into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("match_expr".into(), match_expr.into());
    ctx.insert("value_expr".into(), serde_json::json!(value_expr));
    // The statements of `<name>_canonical`, one level inside its body, and the
    // record type's name as a C string literal.
    ctx.insert(
        "canonical_body".into(),
        serde_json::json!(canonical_body.map(|body| indented(&body))),
    );
    ctx.insert(
        "log_type".into(),
        serde_json::json!(log_type.map(|name| format!("\"{}\"", filters::escape_c(name)))),
    );
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
    ctx.insert("needs_optional".into(), held.optional.into());
    // The buffer of each text bound the template's own type and its
    // parameters hold, a composition's among them, under the guard every
    // header of that bound shares; then the optional value of each element the
    // runtime does not declare; then each composition, inner before outer.
    let mut text_bounds = std::mem::take(&mut held.texts);
    text_bounds.extend(held.compositions.texts.iter().copied());
    let c_text_defs: Vec<String> = text_bounds
        .iter()
        .map(|&bound| c_string_storage_def(bound))
        .collect();
    ctx.insert("c_text_defs".into(), serde_json::json!(c_text_defs));
    ctx.insert(
        "optional_defs".into(),
        serde_json::json!(held.optional_defs.iter().collect::<Vec<_>>()),
    );
    ctx.insert(
        "c_composite_defs".into(),
        serde_json::json!(held.compositions.defs),
    );
    l.render(env, "test_template", ctx)
}

/// The name the generated `<name>_matches` and `<name>_value` take the formal
/// parameter `id` by: `param_<id>`, so that no parameter's name is `value`,
/// the name `<name>_matches` gives the value it is handed, a declaration of a
/// unit it includes, or a keyword of the language.
fn c_parameter(id: &str) -> String {
    format!("param_{id}")
}

/// The refusal of a byte string of one entry, of type `ty`.
fn bytes_unheld(ty: &ValueType) -> String {
    format!(
        "`{ty}` is a byte string the C11 unit of an event schema holds as a pointer with no \
         length, so no value of it is compared or built"
    )
}

/// The types the template's own value and its parameters are held in, and
/// what the unit declares for them.
#[derive(Default)]
struct HeldTypes {
    /// The declarations of the compositions they hold.
    compositions: C11Declarations,
    /// The bound of each text they hold as one entry.
    texts: BTreeSet<u32>,
    /// The declaration of each optional value of an element the runtime does
    /// not declare (a text over its buffer, a value of an enum).
    optional_defs: BTreeSet<String>,
    /// Whether one is an optional value of one entry, of the runtime's
    /// `sce/forge/optional.h`.
    optional: bool,
}

impl HeldTypes {
    /// The C11 type a value of `ty` is held in, as the event schema's unit holds
    /// a field of `ty` ([`c11_entry_optional`], [`unit_type`]).
    fn of(
        &mut self,
        walk: &TemplateWalk<'_, CSpelling>,
        l: &LangCtx,
        imports: &[ImportContext],
        names: &dyn UnitNames,
        ty: &ValueType,
    ) -> Result<String, ForgeError> {
        match ty.entry() {
            Ok((EntryType::Scalar(SceType::Bytes), _)) => Err(walk.refused(bytes_unheld(ty))),
            Ok((EntryType::Scalar(SceType::String), Some(bound))) => {
                self.texts.insert(bound.max);
                Ok(c_string_storage_type(bound.max))
            }
            Ok((EntryType::Scalar(scalar), _)) => Ok(names.scalar(&scalar)),
            Ok((EntryType::Optional { elem }, bound)) => {
                let bound = bound.map(|bound| bound.max);
                let optional = c11_entry_optional(l, imports, &elem, bound)?;
                self.optional = true;
                if let (SceType::String, Some(bound)) = (&elem, bound) {
                    self.texts.insert(bound);
                }
                if let Some(declaration) = optional.declaration {
                    self.optional_defs.insert(declaration);
                }
                Ok(optional.ty)
            }
            Err(_) => unit_type(Language::C11, ty, names, &mut self.compositions)
                .map_err(|why| walk.refused(why)),
        }
    }
}

/// One position of the value a matcher stands at, in C11.
#[derive(Clone)]
struct CAt {
    /// The C expression of the value at the position, in `<name>_matches`.
    expr: String,
    /// Whether the position is inside a list, whose elements a composition
    /// holds in its own types rather than as the fields of a record are held.
    composed: bool,
}

/// The C11 spelling of a template's unit.
#[derive(Default)]
struct CSpelling {
    /// Whether a matcher counts the characters of a text, which the runtime's
    /// [`TEXT_LENGTH_HEADER`] declares.
    counts_characters: Cell<bool>,
    /// The first refusal a spelling met where the walk asks for a text and not
    /// for a result, which the unit is refused by once the walk is done.
    refusal: RefCell<Option<String>>,
}

impl CSpelling {
    /// Keeps `why` as the unit's refusal, unless one was kept before it.
    fn refuse(&self, why: String) {
        let mut kept = self.refusal.borrow_mut();
        if kept.is_none() {
            *kept = Some(why);
        }
    }
}

/// Whether the present values at the places `left` and `right`, both of `ty`,
/// are one value: the one entry an optional value a parameter gives holds, a
/// number, a `bool`, a value of an enum or a text. No other stands there: the
/// profile admits a specific value of one entry alone
/// (`test_profile::matcher_admitted`), and no parameter is an optional byte
/// string, whose type [`HeldTypes::of`] refuses as the schema unit refuses it.
fn equal_present(left: &str, right: &str, ty: &ValueType) -> String {
    match ty {
        TypeOf::Text { .. } => format!("strcmp({left}.data, {right}.data) == 0"),
        _ => format!("{left} == {right}"),
    }
}

impl JoinSpelling for CSpelling {
    /// A text by its bytes up to its terminator, a byte string by its length
    /// and its octets, a value that may be absent a parameter gives by its two
    /// parts, and anything else by the operator.
    fn compare(&self, at: &Position<'_, CAt>, op: Op, negated: bool, rhs: &str) -> String {
        let place = &at.at.expr;
        match (&at.ty, op) {
            (TypeOf::Text { .. }, Op::Eq) => {
                format!("strcmp({place}.data, {rhs}) {} 0", Op::Eq.spelled(negated))
            }
            // `rhs` is the string literal of the octets, whose `sizeof` counts
            // them and the terminator after them: only a literal stands here,
            // since no parameter is a byte string of one entry, and the literal
            // of one is refused where it is spelled (`literal`).
            (TypeOf::Bytes { .. }, Op::Eq) => negate(
                format!(
                    "({place}.len == sizeof({rhs}) - 1u && \
                         memcmp({place}.data, {rhs}, sizeof({rhs}) - 1u) == 0)"
                ),
                negated,
            ),
            // A parameter of the optional type itself, the one composed value a
            // specific value stands for (`test_profile::matcher_admitted` admits
            // none of a list, a record or a union): equal where both are absent,
            // or both present and their values equal, since C compares no two
            // structs.
            (TypeOf::Optional { elem }, Op::Eq) => {
                let equal = format!(
                    "({place}.present == {rhs}.present && (!{place}.present || {}))",
                    equal_present(&format!("{place}.value"), &format!("{rhs}.value"), elem)
                );
                if negated {
                    format!("!{equal}")
                } else {
                    equal
                }
            }
            _ => format!("{place} {} {rhs}", op.spelled(negated)),
        }
    }

    /// A text counts its characters, through the profile's runtime
    /// ([`TEXT_LENGTH`]); a byte string and a list their `len`.
    fn count(&self, at: &Position<'_, CAt>, op: Op, negated: bool, n: u32) -> String {
        let operator = op.spelled(negated);
        let place = &at.at.expr;
        match &at.ty {
            TypeOf::Text { .. } => {
                self.counts_characters.set(true);
                format!("{TEXT_LENGTH}({place}.data) {operator} {n}u")
            }
            TypeOf::Bytes { .. } if !at.at.composed => {
                self.refuse(bytes_unheld(&at.ty));
                String::new()
            }
            _ => format!("{place}.len {operator} {n}u"),
        }
    }

    fn absence(&self, at: &CAt, negated: bool) -> String {
        if negated {
            format!("{}.present", at.expr)
        } else {
            format!("!{}.present", at.expr)
        }
    }

    /// The optional's test of presence, joined with the test of the value it
    /// holds, which reads it by `value` ([`TemplateSpelling::present`]): the
    /// test before it in the same conjunction or disjunction proved it present.
    /// In parentheses where it is an operand of the other connective.
    fn bound_optional(
        &self,
        at: &CAt,
        _inner: &CAt,
        then: &Held<'_, CAt>,
        absent_holds: bool,
        negated: bool,
        stands: Stands,
    ) -> String {
        let (test, nested) = if absent_holds {
            (
                format!(
                    "!{}.present || {}",
                    at.expr,
                    self.join(then, negated, Stands::Disjunct)
                ),
                matches!(stands, Stands::Conjunct),
            )
        } else {
            (
                format!(
                    "{}.present && {}",
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

    /// Never asked: C11 lowers no union, and the walk refuses a `union-match`
    /// on a backend that lowers none before it builds the test of a case
    /// (`TemplateWalk::union_case`).
    fn case_test(
        &self,
        _at: &CAt,
        union: &ScopedUnion,
        _variant: &str,
        _record: CaseTest<'_, '_, CAt>,
        _negated: bool,
    ) -> String {
        unreachable!(
            "the walk refuses the union `{}` on C11 before it asks for the test of a case",
            union.name
        )
    }
}

impl TemplateSpelling for CSpelling {
    type At = CAt;

    fn language(&self) -> Language {
        Language::C11
    }

    /// The value `<name>_matches`'s parameter `value` points to.
    fn root(&self) -> CAt {
        CAt {
            expr: "(*value)".to_string(),
            composed: false,
        }
    }

    /// The optional's value, read where a test before it proved it present.
    fn present(&self, at: &CAt) -> CAt {
        CAt {
            expr: format!("{}.value", at.expr),
            composed: at.composed,
        }
    }

    fn element(&self, at: &CAt, index: usize) -> CAt {
        CAt {
            expr: format!("{}.items[{index}]", at.expr),
            composed: true,
        }
    }

    /// The field, which its record's unit holds as it holds a field of its type,
    /// wherever the record stands.
    fn field(&self, at: &CAt, _record: &ScopedRecord, id: &str, _ty: &ValueType) -> CAt {
        CAt {
            expr: format!(
                "{}.{}",
                at.expr,
                event_schema_field_ident(id, Language::C11)
            ),
            composed: false,
        }
    }

    /// The parameter as it is, but for a text, which is compared by its bytes
    /// and builds the buffer of the position's bound, and for a value that may
    /// be absent built inside a list, whose optional type is the list's own, so
    /// it is built from the parameter's two parts.
    fn parameter(
        &self,
        name: &str,
        ty: &ValueType,
        at: &Position<'_, CAt>,
        purpose: Use,
    ) -> String {
        let param = c_parameter(name);
        match (purpose, ty, &at.ty) {
            (Use::Compare, TypeOf::Text { .. }, _) => format!("{param}.data"),
            (Use::Build, TypeOf::Text { .. }, TypeOf::Text { max }) => {
                format!("{}({param}.data)", c_string_storage_of(max.max))
            }
            (Use::Build, TypeOf::Optional { .. }, _) if at.at.composed => {
                format!("{{ .present = {param}.present, .value = {param}.value }}")
            }
            _ => param,
        }
    }

    /// An integer in decimal, `true` or `false`, a variant of an enum by the
    /// constant its document declares, a text, or a byte string: one spelling
    /// compares and builds but for a text, which builds the buffer of its
    /// bound, and a byte string, which compares as the string literal of its
    /// octets and builds as the initializer of its length and its octets.
    fn literal(&self, text: &str, at: &Position<'_, CAt>, purpose: Use) -> Result<String, String> {
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
                    Language::C11,
                    &enum_qualified_type(&declared.name, Language::C11),
                    &declared.name,
                    text.trim(),
                ))
            }
            TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
                integer_literal(Language::C11, text).ok_or_else(refused)
            }
            // A document's text holds no U+0000, which XML admits nowhere, so
            // the literal is a C string of every character it holds.
            TypeOf::Text { max } => {
                let literal = filters::c_text_expression(text);
                Ok(match purpose {
                    Use::Compare => literal,
                    Use::Build => format!("{}({literal})", c_string_storage_of(max.max)),
                })
            }
            TypeOf::Bytes { .. } => {
                if !at.at.composed {
                    return Err(bytes_unheld(&at.ty));
                }
                let octets = hex_octets(text).ok_or_else(refused)?;
                Ok(match purpose {
                    Use::Compare => {
                        let escaped: String = octets
                            .iter()
                            .map(|pair| {
                                u8::from_str_radix(pair, 16)
                                    .map(|byte| format!("\\{byte:03o}"))
                                    .unwrap_or_default()
                            })
                            .collect();
                        format!("\"{escaped}\"")
                    }
                    Use::Build if octets.is_empty() => "{ .len = 0u }".to_string(),
                    Use::Build => {
                        let data: Vec<String> =
                            octets.iter().map(|pair| format!("0x{pair}")).collect();
                        format!(
                            "{{ .len = {}u, .data = {{ {} }} }}",
                            octets.len(),
                            data.join(", ")
                        )
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
        first: Option<&CAt>,
        args: &[String],
    ) -> String {
        let referenced = import.test_template.as_ref().map(|t| &t.template_type);
        if let (Some(at), Some(ty)) = (first, referenced) {
            if at.composed && matches!(ty.entry(), Ok((EntryType::Optional { .. }, _))) {
                self.refuse(format!(
                    "<sce:template-ref template=\"{}\"> of `{ty}` stands inside a list, which \
                     C11 holds in the list's own optional type and not in the one the referenced \
                     template takes",
                    import.alias
                ));
            }
        }
        let function = match function {
            TemplateFunction::Match => "matches",
            TemplateFunction::Value => "value",
        };
        let arguments: Vec<String> = first
            .map(|at| format!("&{}", at.expr))
            .into_iter()
            .chain(args.iter().map(|arg| c_parameter(arg)))
            .collect();
        format!("{}_{function}({})", import.namespace, arguments.join(", "))
    }

    fn absent(&self, _at: &Position<'_, CAt>) -> String {
        "{ .present = false }".to_string()
    }

    fn present_value(&self, at: &Position<'_, CAt>, value: String) -> String {
        if at.optional() {
            format!("{{ .present = true, .value = {value} }}")
        } else {
            value
        }
    }

    /// The record's initializer, each field by its designator, in the order
    /// its schema declares them.
    fn record_value(
        &self,
        _at: &Position<'_, CAt>,
        _record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        let fields: Vec<String> = fields
            .into_iter()
            .map(|(id, value)| {
                format!(
                    ".{} = {value}",
                    event_schema_field_ident(&id, Language::C11)
                )
            })
            .collect();
        format!("{{ {} }}", fields.join(", "))
    }

    /// The list's initializer: its length and its elements.
    fn list_value(&self, _at: &Position<'_, CAt>, elements: Vec<String>) -> String {
        if elements.is_empty() {
            return "{ .len = 0u }".to_string();
        }
        format!(
            "{{ .len = {}u, .items = {{ {} }} }}",
            elements.len(),
            elements.join(", ")
        )
    }
}

/// `text`, each line of it one level further in, as the statements of a block.
fn indented(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("    {line}")
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// A value `<name>_canonical` writes, in C11.
struct C11CanonicalAt {
    /// The C expression of the value: `(*value)`, which the function is lent,
    /// an element of a list by its index, or a field of either.
    expr: String,
    /// Whether the value is an element of a list, which a composition holds in
    /// its own types: there a byte string holds its length.
    composed: bool,
    /// How many loops over a list the value stands inside: the number of the
    /// next loop's index, so no index shadows another (`-Wshadow`).
    depth: usize,
}

/// The C11 spelling of the canonical form of a value in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5): the statements that write it through the
/// runtime's `sce_jcs_t` ([`C_JCS`]), with the writers of [`CANONICAL_HEADER`].
/// Which form each SCE type takes is the walk's
/// ([`super::test_template_walk::canonical_value`]).
///
/// C has no closure, so a list is a loop over its `len` elements, each read by
/// its index ([`CanonicalSpelling::list_element`]), and a field that may be
/// absent is read by `value` behind its `present`
/// ([`CanonicalSpelling::optional_value`]). Each field is named before its
/// value is written; an absent one writes no value after its name, which the
/// writer then leaves out (`sce_jcs_key`). A C enumerator names neither its
/// variant nor its number, so an enum's value is read through the functions of
/// the enum's own unit ([`CanonicalSpelling::enumerated_of`]); and an integer
/// of any width through the runtime's macro, since C has no overload by the
/// width and signedness of a field. A byte string of one entry, which the C11
/// event schema unit holds as a pointer with no length, has no form here and is
/// refused by name; C11 lowers no union, so the walk's refusal of one stands.
#[derive(Default)]
struct C11Canonical {
    /// The first refusal a spelling met, which the unit is refused by once the
    /// walk is done: the walk asks for a text and not for a result.
    refusal: RefCell<Option<String>>,
}

impl C11Canonical {
    /// Keeps `why` as the unit's refusal, unless one was kept before it.
    fn refuse(&self, why: String) {
        let mut kept = self.refusal.borrow_mut();
        if kept.is_none() {
            *kept = Some(why);
        }
    }
}

impl CanonicalSpelling for C11Canonical {
    type At = C11CanonicalAt;

    /// `<name>_canonical`'s parameter `value`, a pointer to the value.
    fn root(&self) -> C11CanonicalAt {
        C11CanonicalAt {
            expr: "(*value)".to_string(),
            composed: false,
            depth: 0,
        }
    }

    /// The field, which its record's unit holds as it holds a field of its
    /// type, wherever the record stands.
    fn field(&self, at: &C11CanonicalAt, id: &str) -> C11CanonicalAt {
        C11CanonicalAt {
            expr: format!(
                "{}.{}",
                at.expr,
                event_schema_field_ident(id, Language::C11)
            ),
            composed: false,
            depth: at.depth,
        }
    }

    /// The index of a loop over a list, numbered by the loops it stands in.
    fn bound(&self, at: &C11CanonicalAt, prefix: &str) -> (String, C11CanonicalAt) {
        let name = format!("sce_{prefix}_{}", at.depth);
        let bound = C11CanonicalAt {
            expr: name.clone(),
            composed: at.composed,
            depth: at.depth + 1,
        };
        (name, bound)
    }

    /// The element the loop's index names.
    fn list_element(&self, list: &C11CanonicalAt, bound: C11CanonicalAt) -> C11CanonicalAt {
        C11CanonicalAt {
            expr: format!("{}.items[{}]", list.expr, bound.expr),
            composed: true,
            depth: bound.depth,
        }
    }

    /// The value the field holds, read behind its `present`: nothing is bound.
    fn optional_value(&self, field: &C11CanonicalAt, _bound: C11CanonicalAt) -> C11CanonicalAt {
        C11CanonicalAt {
            expr: format!("{}.value", field.expr),
            composed: field.composed,
            depth: field.depth,
        }
    }

    fn boolean(&self, at: &C11CanonicalAt) -> String {
        format!("sce_test_canonical_boolean({C_JCS}, {});", at.expr)
    }

    /// Never asked: [`Self::enumerated_of`] writes every value of an enum, by
    /// the functions of the enum's unit, which this does not name.
    fn enumerated(&self, _at: &C11CanonicalAt, _strict: bool) -> String {
        unreachable!("a C11 enum value is written by `enumerated_of`, which names its enum")
    }

    /// The name the enum's unit declares for the value (`<enum>_declared_name`);
    /// for an enum of an open set, its number where no variant declares it.
    fn enumerated_of(&self, at: &C11CanonicalAt, declared: &ScopedEnum) -> String {
        let prefix = crate::forge::enum_naming::c11_function_prefix(&declared.name);
        if declared.strict {
            format!(
                "sce_test_canonical_enumerated({C_JCS}, {prefix}_declared_name({}));",
                at.expr
            )
        } else {
            format!(
                "SCE_TEST_CANONICAL_OPEN_ENUMERATED({C_JCS}, {prefix}_declared_name({0}), {0});",
                at.expr
            )
        }
    }

    fn integer(&self, at: &C11CanonicalAt) -> String {
        format!("SCE_TEST_CANONICAL_INTEGER({C_JCS}, {});", at.expr)
    }

    /// The text up to its terminator, in the buffer of its bound.
    fn text(&self, at: &C11CanonicalAt) -> String {
        format!("sce_test_canonical_text({C_JCS}, {}.data);", at.expr)
    }

    /// A byte string a list holds, by its length and its octets. One of one
    /// entry is held with no length, and is refused.
    fn bytes(&self, at: &C11CanonicalAt) -> String {
        if !at.composed {
            self.refuse(
                "a byte string of one entry, which the C11 unit of an event schema holds as a \
                 pointer with no length, has no form in the test log"
                    .to_string(),
            );
            return String::new();
        }
        format!(
            "sce_test_canonical_bytes({C_JCS}, {0}.data, {0}.len);",
            at.expr
        )
    }

    fn list(&self, at: &C11CanonicalAt, binding: &str, element: String) -> String {
        format!(
            "sce_jcs_begin_array({C_JCS});\n\
             for (uint32_t {binding} = 0u; {binding} < {}.len; ++{binding}) {{\n{}\n}}\n\
             sce_jcs_end_array({C_JCS});",
            at.expr,
            indented(&element)
        )
    }

    /// As it is: the record names the field before it.
    fn held_field(&self, written: String) -> String {
        written
    }

    /// The value written where the field holds one, and nothing where it holds
    /// none, which leaves the name the record wrote before it with no value.
    fn optional_field(&self, field: &C11CanonicalAt, _binding: &str, written: String) -> String {
        format!("if ({}.present) {{\n{}\n}}", field.expr, indented(&written))
    }

    /// The object, each field named by its declared id before its value.
    fn record(&self, fields: Vec<(String, String)>) -> String {
        let mut written = format!("sce_jcs_begin_object({C_JCS});\n");
        for (id, value) in fields {
            written.push_str(&format!(
                "sce_jcs_key({C_JCS}, \"{}\");\n{value}\n",
                filters::escape_c(id)
            ));
        }
        written.push_str(&format!("sce_jcs_end_object({C_JCS});"));
        written
    }
}
