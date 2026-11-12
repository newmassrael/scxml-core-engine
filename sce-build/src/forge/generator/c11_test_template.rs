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
//! text in the buffer of its bound, a byte string in the buffer of its bound
//! beside its length, an optional one in the runtime's
//! `sce_optional_<elem>_t`), and a composition laid out statically where it is
//! not ([`unit_type`]), so a template's value, and the value a reference passes
//! the template it names, is the field's own type. A position inside a list is
//! the list's element, which a composition holds in its own types: there a byte
//! string is the composition's, which holds its length too, and an optional
//! value is the composition's.
//!
//! A text is compared by its bytes up to its terminator (`strcmp`) and its
//! length restriction counts characters through the profile's runtime
//! ([`TEXT_LENGTH`]); a byte string is compared by its length and its octets
//! (`memcmp`), the other side read through the runtime's view of bytes, and
//! counted by its length; a value that may be absent is tested by `present` and read by
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
//! as the statements that write it through the runtime's `sce_jcs_t`. Its
//! inverse, [`C11Reading`], reads a value back from that form with the readers
//! of `sce/test/from_canonical.h`, over the same walk
//! ([`super::test_template_walk::canonical_form`]): the unit of a test
//! configuration reads the values of a recorded input trace with it
//! (docs/SCE_TTCN3_LANE.md §10.5).
//!
//! What C11 refuses by name: a union, which its test profile matches nowhere
//! yet ([`crate::forge::codegen_matrix::TEST_PROFILE_UNION_BACKENDS`]), by the walk before
//! anything is spelled; and a reference, inside a list, to a template of an
//! optional type, whose value the list holds in its own optional type rather
//! than the one the referenced template takes.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

use super::test_template_walk::{
    hex_octets, integer_literal, negate, CanonicalCase, CanonicalForm, CanonicalSpelling, CaseTest,
    Held, JoinSpelling, Op, Position, Stands, TemplateFunction, TemplateSpelling, TemplateWalk,
    Use,
};
use super::*;
use crate::forge::schema_unit_type::{unit_type, C11Declarations, UnitNames};
use crate::forge::static_lowering::{
    c_bytes_storage_def, c_bytes_storage_type, c_string_storage_def, c_string_storage_of,
    c_string_storage_type,
};
use crate::forge::value_type::{Bound, ScopedEnum, ScopedRecord, ScopedUnion, TypeOf, ValueType};
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
    let (log_type, canonical_body) = walk.message_form(&C11Canonical, &scope)?.unzip();

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
    // The buffer of each byte string of one entry the template's own type and
    // its parameters hold, beside its length, under the guard every header of
    // that capacity shares; the runtime's view of bytes, which its fill
    // function takes and a comparison of byte strings reads, is included with
    // it.
    let c_bytes_defs: Vec<String> = held
        .bytes
        .iter()
        .map(|&capacity| c_bytes_storage_def(capacity))
        .collect();
    ctx.insert(
        "views_bytes".into(),
        (!c_bytes_defs.is_empty() || spelling.compares_bytes.get()).into(),
    );
    ctx.insert("c_bytes_defs".into(), serde_json::json!(c_bytes_defs));
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

/// The initializer of a byte string of the octets `data`, `len` their count:
/// inside a list, of the composition's type, its length first as it declares
/// it (`{ uint32_t len; uint8_t data[N]; }`); of one entry, of the buffer the
/// event schema's unit holds one in, its octets first
/// (`sce_static_bytes_<N>_t`). Each designator stands in the order of its
/// member, which a C++ reader of the header requires.
fn bytes_initializer(composed: bool, len: &str, data: &[String]) -> String {
    let data = data.join(", ");
    if composed {
        format!("{{ .len = {len}, .data = {{ {data} }} }}")
    } else {
        format!("{{ .data = {{ {data} }}, .len = {len} }}")
    }
}

/// The types the template's own value and its parameters are held in, and
/// what the unit declares for them.
#[derive(Default)]
struct HeldTypes {
    /// The declarations of the compositions they hold.
    compositions: C11Declarations,
    /// The bound of each text they hold as one entry.
    texts: BTreeSet<u32>,
    /// The capacity of each byte string they hold as one entry, in the buffer
    /// beside its length the event schema's unit holds one in.
    bytes: BTreeSet<u32>,
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
            Ok((EntryType::Scalar(SceType::Bytes), bound)) => {
                let capacity =
                    crate::forge::limits::resolve_bytes_max(bound.map(|bound| bound.max));
                self.bytes.insert(capacity);
                Ok(c_bytes_storage_type(capacity))
            }
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
    /// Whether a matcher compares two byte strings, each read through the
    /// runtime's view of bytes (`sce/forge/bytes.h`).
    compares_bytes: Cell<bool>,
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
            // `rhs` is the runtime's view of the other byte string, a literal's
            // octets or a parameter's buffer (`literal`, `parameter`), so one
            // spelling reads both, and a position of one entry and one inside a
            // list alike: each holds its octets in `data` and their count in
            // `len`.
            (TypeOf::Bytes { .. }, Op::Eq) => {
                self.compares_bytes.set(true);
                negate(
                    format!(
                        "({place}.len == ({rhs}).len && \
                         memcmp({place}.data, ({rhs}).data, {place}.len) == 0)"
                    ),
                    negated,
                )
            }
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

    /// Never asked: C11's test profile matches no union yet, and the walk
    /// refuses a `union-match` on such a backend before it builds the test of
    /// a case (`TemplateWalk::union_case`).
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
    /// and builds the buffer of the position's bound; for a byte string, which
    /// is compared through the runtime's view of its buffer and, inside a list,
    /// whose byte string type is the list's own, built octet by octet from its
    /// buffer of the same capacity; and for a value that may be absent built
    /// inside a list, whose optional type is the list's own, so it is built
    /// from the parameter's two parts.
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
            (Use::Compare, TypeOf::Bytes { .. }, _) => {
                format!("((sce_forge_bytes_view_t){{ {param}.data, {param}.len }})")
            }
            (Use::Build, TypeOf::Bytes { .. }, TypeOf::Bytes { max }) if at.at.composed => {
                let data: Vec<String> =
                    (0..max.max).map(|i| format!("{param}.data[{i}]")).collect();
                bytes_initializer(true, &format!("(uint32_t){param}.len"), &data)
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
    /// bound, and a byte string, which compares as the runtime's view of the
    /// string literal of its octets and builds as the initializer of its octets
    /// and their count ([`bytes_initializer`]).
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
                        format!(
                            "((sce_forge_bytes_view_t){{ (const uint8_t *)\"{escaped}\", {}u }})",
                            octets.len()
                        )
                    }
                    Use::Build if octets.is_empty() => "{ .len = 0u }".to_string(),
                    Use::Build => {
                        let data: Vec<String> =
                            octets.iter().map(|pair| format!("0x{pair}")).collect();
                        bytes_initializer(at.at.composed, &format!("{}u", octets.len()), &data)
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
pub(super) fn indented(text: &str) -> String {
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
pub(super) struct C11CanonicalAt {
    /// The C expression of the value: `(*value)`, which the function is lent,
    /// an element of a list by its index, or a field of either.
    expr: String,
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
/// its index ([`CanonicalSpelling::list_element`]), and a field or an element
/// that may be absent is read by `value` behind its `present`
/// ([`CanonicalSpelling::optional_value`]). Each field is named before its
/// value is written; an absent one writes no value after its name, which the
/// writer then leaves out (`sce_jcs_key`), and an absent element writes `null`
/// in its place (`sce_test_canonical_absent_element`). A C enumerator names
/// neither its variant nor its number, so an enum's value is read through the
/// functions of the enum's own unit ([`CanonicalSpelling::enumerated_of`]);
/// and an integer of any width through the runtime's macro, since C has no
/// overload by the width and signedness of a field. A byte string, of one entry or inside a
/// list, holds its octets in `data` and their count in `len`, which its form
/// reads alike. C11's test profile matches no union yet, so the walk's refusal
/// of one stands. The unit of a test configuration writes its records' forms
/// with it too, beside their readers ([`C11Reading`]).
pub(super) struct C11Canonical;

impl CanonicalSpelling for C11Canonical {
    type At = C11CanonicalAt;

    /// `<name>_canonical`'s parameter `value`, a pointer to the value.
    fn root(&self) -> C11CanonicalAt {
        C11CanonicalAt {
            expr: "(*value)".to_string(),
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
            depth: at.depth,
        }
    }

    /// The index of a loop over a list, numbered by the loops it stands in.
    fn bound(&self, at: &C11CanonicalAt, prefix: &str) -> (String, C11CanonicalAt) {
        let name = format!("sce_{prefix}_{}", at.depth);
        let bound = C11CanonicalAt {
            expr: name.clone(),
            depth: at.depth + 1,
        };
        (name, bound)
    }

    /// The element the loop's index names.
    fn list_element(&self, list: &C11CanonicalAt, bound: C11CanonicalAt) -> C11CanonicalAt {
        C11CanonicalAt {
            expr: format!("{}.items[{}]", list.expr, bound.expr),
            depth: bound.depth,
        }
    }

    /// The value the field holds, read behind its `present`: nothing is bound.
    fn optional_value(&self, field: &C11CanonicalAt, _bound: C11CanonicalAt) -> C11CanonicalAt {
        C11CanonicalAt {
            expr: format!("{}.value", field.expr),
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

    /// A byte string by its octets and their count, of one entry or inside a
    /// list alike.
    fn bytes(&self, at: &C11CanonicalAt) -> String {
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

    /// The value written where the element holds one, read behind its
    /// `present` as a field's is, and `null` where it holds none, which the
    /// array keeps in its place.
    fn optional_element(
        &self,
        element: &C11CanonicalAt,
        _binding: &str,
        written: String,
    ) -> String {
        format!(
            "if ({}.present) {{\n{}\n}} else {{\n    sce_test_canonical_absent_element({C_JCS});\n}}",
            element.expr,
            indented(&written)
        )
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

/// `text` as a C string literal.
pub(super) fn c_literal(text: &str) -> String {
    format!("\"{}\"", filters::escape_c(text.to_string()))
}

/// The statement that calls `call`, a reader of `sce/test/from_canonical.h` that
/// answers whether it read its value, and returns false from the generated
/// reader where it did not: the reader's refusal is the generated reader's.
fn c_read(call: &str) -> String {
    format!("if (!{call}) {{\n    return false;\n}}")
}

/// A value the canonical reader reads, in C11: the JSON it is read from and the
/// place it is read into.
pub(super) struct C11ReadingAt {
    /// The `sce_jv_t` variable that holds the value's JSON where it is present:
    /// `value`, the reader's parameter, or a variable a statement of the reader
    /// binds.
    json: String,
    /// The C place the value is read into: `(*out)`, a field of it, an element
    /// of a list by the list's `len`, or an optional's `value`.
    place: String,
    /// Where the value stands, as a refusal names it: the record's type name,
    /// then each field by its id, `[]` for an element of a list.
    what: String,
    /// How many bindings the value stands inside: the number every variable the
    /// reader binds inside it carries, so no variable shadows another
    /// (`-Wshadow`).
    depth: usize,
    /// Whether the value stands inside a list, whose elements a composition
    /// holds in its own types: there a byte string's count is a `uint32_t`, and
    /// a byte string of one entry's a `size_t`.
    composed: bool,
    /// For a field, the variable of the record it stands in: what an optional
    /// field is looked up in. Empty for any other value.
    record: String,
    /// For a field, its id. Empty for any other value.
    id: String,
}

/// The C11 reader of the canonical form of a value (docs/SCE_TEST_PROFILE.md
/// §12.5; docs/SCE_TTCN3_LANE.md §10.3): the statements that read the record a
/// configuration's `<configuration>_read_<record>` reads from its parameter
/// `value`, a span of a judged text (`sce_jv_t`), into `*out`, with the readers
/// of `sce/test/from_canonical.h`, each of which refuses, naming where the value
/// stands, a value that does not belong to its type. Which form each SCE type
/// takes is the walk its writer's is
/// ([`super::test_template_walk::canonical_form`]), so the reader reads exactly
/// what [`C11Canonical`] writes.
///
/// C has no closure and no exception, so each read is a block of statements that
/// returns false from the reader where a value is refused; the record is zeroed
/// first, so a field, an element or a `present` no statement writes holds what
/// an absent one holds. A record checks its object holds no field it does not
/// declare and binds each field it reads, a list walks its array within its
/// bounds and counts each element into its `len`, and a value that may be absent
/// is read behind the test of its presence, which sets its `present`. An integer
/// is read through the runtime's widest integer of its signedness, held to its
/// type's range and narrowed to it; a value of an enum by the names its document
/// declares, beside the constants or the numbers they stand for. C11 lowers no
/// union, so the walk's refusal of one stands.
pub(super) struct C11Reading {
    /// What the value read is named in a refusal: its record's type name.
    pub(super) what: String,
}

impl CanonicalForm for C11Reading {
    type At = C11ReadingAt;

    /// The reader's parameter `value`, read into `*out`.
    fn root(&self) -> C11ReadingAt {
        C11ReadingAt {
            json: "value".to_string(),
            place: "(*out)".to_string(),
            what: self.what.clone(),
            depth: 0,
            composed: false,
            record: String::new(),
            id: String::new(),
        }
    }

    /// The field `id` of the record at `at`, which its record's unit holds as
    /// it holds a field of its type, wherever the record stands: bound by
    /// [`Self::record`] where its type is not optional, and by
    /// [`Self::optional_field`] where it is.
    fn field(&self, at: &C11ReadingAt, id: &str) -> C11ReadingAt {
        let ident = event_schema_field_ident(id, Language::C11);
        C11ReadingAt {
            json: format!("sce_field_{}_{ident}", at.depth),
            place: format!("{}.{ident}", at.place),
            what: format!("{}.{id}", at.what),
            depth: at.depth + 1,
            composed: false,
            record: at.json.clone(),
            id: id.to_string(),
        }
    }

    /// The variable an element of a list, or a present value, is bound to,
    /// numbered by the bindings it stands inside.
    fn bound(&self, at: &C11ReadingAt, prefix: &str) -> (String, C11ReadingAt) {
        let name = format!("sce_{prefix}_{}", at.depth);
        let bound = C11ReadingAt {
            json: name.clone(),
            place: at.place.clone(),
            what: at.what.clone(),
            depth: at.depth + 1,
            composed: at.composed,
            record: String::new(),
            id: String::new(),
        };
        (name, bound)
    }

    fn case_record(&self, bound: C11ReadingAt, _record: &str) -> C11ReadingAt {
        bound
    }

    /// The element the loop over the list binds, read into the place its
    /// `len` names, which counts the elements read before it.
    fn list_element(&self, list: &C11ReadingAt, bound: C11ReadingAt) -> C11ReadingAt {
        C11ReadingAt {
            place: format!("{0}.items[{0}.len]", list.place),
            what: format!("{}[]", list.what),
            composed: true,
            ..bound
        }
    }

    /// The value an optional holds, read into its `value`.
    fn optional_value(&self, field: &C11ReadingAt, bound: C11ReadingAt) -> C11ReadingAt {
        C11ReadingAt {
            place: format!("{}.value", field.place),
            what: field.what.clone(),
            composed: field.composed,
            ..bound
        }
    }

    fn boolean(&self, at: &C11ReadingAt) -> String {
        c_read(&format!(
            "sce_test_from_canonical_boolean({}, {}, &{}, err)",
            at.json,
            c_literal(&at.what),
            at.place
        ))
    }

    /// The variant whose declared name the text is: a closed enum's constant by
    /// the index of its name, and an open enum's number, declared or not, made a
    /// value of the enum's type as its unit's `from_underlying` makes one.
    fn enumerated(&self, at: &C11ReadingAt, declared: &ScopedEnum) -> String {
        let names: Vec<String> = declared
            .variants
            .iter()
            .map(|variant| c_literal(&variant.name))
            .collect();
        let count = names.len();
        let what = c_literal(&at.what);
        let ty = enum_qualified_type(&declared.name, Language::C11);
        if declared.strict {
            let values: Vec<String> = declared
                .variants
                .iter()
                .map(|variant| {
                    crate::forge::enum_naming::variant_ident(
                        Language::C11,
                        &declared.name,
                        &variant.name,
                    )
                })
                .collect();
            let read = c_read(&format!(
                "sce_test_from_canonical_enumerated({}, sce_names, {count}u, {what}, &sce_index, \
                 err)",
                at.json
            ));
            return format!(
                "{{\n    static const char *const sce_names[] = {{{}}};\n    static const {ty} \
                 sce_values[] = {{{}}};\n    size_t sce_index = 0u;\n{}\n    {} = \
                 sce_values[sce_index];\n}}",
                names.join(", "),
                values.join(", "),
                indented(&read),
                at.place
            );
        }
        let limit = c_type(&declared.underlying)
            .trim_end_matches("_t")
            .to_uppercase();
        let (numbers, held, read) = if declared.underlying.is_signed() {
            let numbers: Vec<String> = declared
                .variants
                .iter()
                .map(|variant| {
                    if variant.value == i128::from(i64::MIN) {
                        "INT64_MIN".to_string()
                    } else {
                        format!("INT64_C({})", variant.value)
                    }
                })
                .collect();
            (
                format!(
                    "static const int64_t sce_numbers[] = {{{}}};",
                    numbers.join(", ")
                ),
                "int64_t sce_held = 0;",
                format!(
                    "sce_test_from_canonical_open_signed({}, sce_names, sce_numbers, {count}u, \
                     {limit}_MIN, {limit}_MAX, {what}, &sce_held, err)",
                    at.json
                ),
            )
        } else {
            let numbers: Vec<String> = declared
                .variants
                .iter()
                .map(|variant| format!("UINT64_C({})", variant.value))
                .collect();
            (
                format!(
                    "static const uint64_t sce_numbers[] = {{{}}};",
                    numbers.join(", ")
                ),
                "uint64_t sce_held = 0u;",
                format!(
                    "sce_test_from_canonical_open_unsigned({}, sce_names, sce_numbers, {count}u, \
                     {limit}_MAX, {what}, &sce_held, err)",
                    at.json
                ),
            )
        };
        format!(
            "{{\n    static const char *const sce_names[] = {{{}}};\n    {numbers}\n    {held}\n{}\n    \
             {} = ({ty})sce_held;\n}}",
            names.join(", "),
            indented(&c_read(&read)),
            at.place
        )
    }

    /// Read through the widest integer of its signedness, held to the range of
    /// its type `width`, and narrowed to it.
    fn integer(&self, at: &C11ReadingAt, width: &SceType) -> String {
        let ty = c_type(width);
        let limit = ty.trim_end_matches("_t").to_uppercase();
        let what = c_literal(&at.what);
        let (held, read) = if width.is_signed() {
            (
                "int64_t sce_held = 0;",
                format!(
                    "sce_test_from_canonical_signed({}, {limit}_MIN, {limit}_MAX, {what}, \
                     &sce_held, err)",
                    at.json
                ),
            )
        } else {
            (
                "uint64_t sce_held = 0u;",
                format!(
                    "sce_test_from_canonical_unsigned({}, {limit}_MAX, {what}, &sce_held, err)",
                    at.json
                ),
            )
        };
        format!(
            "{{\n    {held}\n{}\n    {} = ({ty})sce_held;\n}}",
            indented(&c_read(&read)),
            at.place
        )
    }

    /// Into the buffer of its bound, terminated.
    fn text(&self, at: &C11ReadingAt, max: &Bound) -> String {
        c_read(&format!(
            "sce_test_from_canonical_text({}, {}u, {}, {place}.data, sizeof({place}.data), err)",
            at.json,
            max.max,
            c_literal(&at.what),
            place = at.place
        ))
    }

    /// Into the buffer of its bound, its count into its `len`, which a
    /// composition holds as a `uint32_t` and a byte string of one entry as a
    /// `size_t`.
    fn bytes(&self, at: &C11ReadingAt, max: &Bound) -> String {
        let cast = if at.composed { "(uint32_t)" } else { "" };
        let read = c_read(&format!(
            "sce_test_from_canonical_bytes({}, {}u, {}, {place}.data, sizeof({place}.data), \
             &sce_length, err)",
            at.json,
            max.max,
            c_literal(&at.what),
            place = at.place
        ));
        format!(
            "{{\n    size_t sce_length = 0u;\n{}\n    {}.len = {cast}sce_length;\n}}",
            indented(&read),
            at.place
        )
    }

    /// A loop over the elements of its array, held to the list's bounds, each
    /// read into the place the list's `len` names and counted.
    fn list(&self, at: &C11ReadingAt, max: &Bound, binding: &str, element: String) -> String {
        let items = format!("sce_items_{}", at.depth);
        let read = c_read(&format!(
            "sce_test_from_canonical_elements({}, {}u, {}u, {}, &{items}, err)",
            at.json,
            max.min,
            max.max,
            c_literal(&at.what)
        ));
        format!(
            "{{\n    sce_jiter_t {items};\n    sce_jv_t {binding};\n{}\n    while \
             (sce_jiter_element(&{items}, &{binding})) {{\n{}\n        {}.len++;\n    }}\n}}",
            indented(&read),
            indented(&indented(&element)),
            at.place
        )
    }

    /// As it is: the record binds the field before it ([`Self::record`]).
    fn held_field(&self, written: String) -> String {
        written
    }

    /// The field read where the record's object names it, which sets its
    /// `present`, and nothing where it does not, which leaves it absent.
    fn optional_field(&self, field: &C11ReadingAt, binding: &str, written: String) -> String {
        format!(
            "{{\n    sce_jv_t {binding};\n    if (sce_test_from_canonical_optional_field({}, {}, \
             &{binding})) {{\n        {}.present = true;\n{}\n    }}\n}}",
            field.record,
            c_literal(&field.id),
            field.place,
            indented(&indented(&written))
        )
    }

    /// The element read where it is not `null`, which sets its `present`, and
    /// left absent where it is.
    fn optional_element(&self, element: &C11ReadingAt, binding: &str, written: String) -> String {
        format!(
            "if (sce_test_from_canonical_present({json})) {{\n    const sce_jv_t {binding} = \
             {json};\n    {}.present = true;\n{}\n}}",
            element.place,
            indented(&written),
            json = element.json
        )
    }

    /// The record's object held to the fields it declares, then each field: one
    /// whose type is not optional bound to its variable, refused where it is
    /// absent, and read; one whose type is optional as
    /// [`Self::optional_field`] spelled it.
    fn record(
        &self,
        at: &C11ReadingAt,
        record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        let what = c_literal(&at.what);
        let mut read = if fields.is_empty() {
            c_read(&format!(
                "sce_test_from_canonical_record({}, NULL, 0u, {what}, err)",
                at.json
            ))
        } else {
            let ids: Vec<String> = fields.iter().map(|(id, _)| c_literal(id)).collect();
            let names = format!("sce_fields_{}", at.depth);
            format!(
                "{{\n    static const char *const {names}[] = {{{}}};\n{}\n}}",
                ids.join(", "),
                indented(&c_read(&format!(
                    "sce_test_from_canonical_record({}, {names}, {}u, {what}, err)",
                    at.json,
                    ids.len()
                )))
            )
        };
        for ((id, written), (_, ty)) in fields.into_iter().zip(&record.fields) {
            read.push('\n');
            if matches!(ty, TypeOf::Optional { .. }) {
                read.push_str(&written);
                continue;
            }
            let field = self.field(at, &id);
            let bind = c_read(&format!(
                "sce_test_from_canonical_field({}, {}, {}, &{}, err)",
                at.json,
                c_literal(&id),
                c_literal(&field.what),
                field.json
            ));
            read.push_str(&format!(
                "{{\n    sce_jv_t {};\n{}\n{}\n}}",
                field.json,
                indented(&bind),
                indented(&written)
            ));
        }
        read
    }

    fn union(
        &self,
        _at: &C11ReadingAt,
        union: &ScopedUnion,
        _cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        Err(crate::forge::value_type::union_unlowered(&union.name))
    }
}
