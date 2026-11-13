// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The check of a recorded input trace against the suite it is of, without
//! running it (docs/SCE_TTCN3_LANE.md §4, §10.2): `sce-codegen check-trace
//! <suite> <trace>`.
//!
//! The suite is read as the export reads it: parsed, held to the rules `check`
//! holds it to, with every document it reaches read and held to the rules of
//! its kind. A suite the export refuses is refused here the same way, as an
//! input that is not read. The trace is read by the Rust runtime's reader,
//! `sce_rust_runtime::test::Trace::parse`, the authoritative one (§10.4), which
//! refuses a text that is not JSON, not of the shape `sce-test-trace.v1`, out
//! of the order of its instants or not its one canonical text; nothing else of
//! such a text is read. Every other refusal of §10.2 is reported, each one:
//!
//! - the trace's `suite` is not the suite, or its `test_case` names no test
//!   case the suite runs, by the names the runner reads a trace by (the
//!   documents' own);
//! - an input's `port` is no system port of the test case's configuration, or
//!   its `type` is not on the `in` list of that port's type: read from the
//!   table every backend's unit of the configuration writes its `input` from
//!   (`trace_records_of`), and said in the words of the Rust runtime's
//!   refusal (`TraceRefusal::not_an_input`);
//! - an input's `value` does not belong to its record type. The value is read
//!   as the generated Rust reader of the record reads it, by the runtime's own
//!   reader of each form (`sce_rust_runtime::test::from_canonical`), in the
//!   order that reader reads it: a record's members before its fields, its
//!   fields in the order the schema declares them, a list's bounds before its
//!   elements, a union's case before the case's record. The first refusal of
//!   a value is so the one the runner gives, in its words. Which types have a
//!   form is decided by the one walk of the form (`canonical_form`), which
//!   refuses the configuration of a record that has none, as every generator
//!   refuses it.
//!
//! Each refusal is placed in the trace file, on its one line, and names the
//! place it is about by its JSON pointer (`/inputs/0/value/code`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sce_rust_runtime::json::Value;
use sce_rust_runtime::test::{from_canonical, Trace, TraceRefusal};

use super::behaviour::configurations;
use super::execution::{executed_case, run_in};
use super::reach::Reached;
use super::{read_suite, ExportError, ReadSuite};
use crate::forge::error::{ForgeError, Located, TraceRule, ValidationError};
use crate::forge::generator::test_tables::{self, trace_records_of};
use crate::forge::generator::test_template_walk::{
    canonical_form, formless, optional_out_of_place, unimported, CanonicalCase, CanonicalForm,
};
use crate::forge::generator::TestPortTypeImport;
use crate::forge::model::{ForgeDocument, ForgeKind, SceType, TestSuiteModel};
use crate::forge::value_type::{
    Bound, ScopedEnum, ScopedRecord, ScopedUnion, TypeOf, TypeScope, ValueType,
};

/// Check the trace `text`, read from the file the caller named `trace`,
/// against the suite at `suite`, whose `<xi:include>` and `<sce:use>`
/// fragments `include_dirs` resolve (docs/SCE_TTCN3_LANE.md §10.2).
///
/// Every refusal of the trace, each placed in the trace file: none where the
/// trace fits the suite, one where the text is no trace, and otherwise each
/// place that does not fit. Refused, as an input not read, where the suite is
/// refused, as `export-ttcn3` refuses it, or where the configuration of the
/// trace's test case takes a record that has no canonical form.
pub fn check_trace(
    suite: &Path,
    include_dirs: &[PathBuf],
    trace: &str,
    text: &str,
) -> Result<Vec<Located<ForgeError>>, ExportError> {
    let read = read_suite(suite, include_dirs)?;
    let suite = read.suite()?;
    let parsed = match Trace::parse(text) {
        Ok(parsed) => parsed,
        // Nothing of a text that is no trace is read, so the refusal names
        // no place in it.
        Err(refusal) => {
            return Ok(vec![Located::new(
                refusal_of(
                    TraceRule::Text,
                    String::new(),
                    String::new(),
                    refusal.reason().to_string(),
                ),
                trace,
                None,
                None,
            )])
        }
    };
    // A trace is one canonical text, so every place in it is on its first
    // line.
    let placed = |rule, pointer, actual, detail| {
        Located::new(
            refusal_of(rule, pointer, actual, detail),
            trace,
            Some(1),
            None,
        )
    };
    let mut refusals = Vec::new();
    if parsed.suite != suite.name {
        refusals.push(placed(
            TraceRule::TestCase,
            "/suite".to_string(),
            spelled(&parsed.suite),
            format!(
                "the trace is of the test case '{}' of the suite '{}', and the suite it is \
                 checked against is '{}'",
                parsed.test_case, parsed.suite, suite.name
            ),
        ));
    }
    let Some(case) = test_case(&read, suite, &parsed.test_case)? else {
        refusals.push(placed(
            TraceRule::TestCase,
            "/test_case".to_string(),
            spelled(&parsed.test_case),
            format!(
                "the trace is of the test case '{}', which the suite '{}' does not run",
                parsed.test_case, suite.name
            ),
        ));
        return Ok(refusals);
    };

    let configurations = configurations(&read.reach).map_err(|e| read.refused(e))?;
    let configuration = run_in(case, &read.reach, &configurations).map_err(|e| read.refused(e))?;
    let (document, model) = (configuration.document, configuration.model);
    // A record its system ports take that has no canonical form is the
    // configuration's refusal, as every generator gives it.
    let configuration_refused = |why: String| {
        read.refused(Located::in_file(
            test_tables::refused(
                ForgeKind::TestConfiguration,
                &model.name,
                model.source_location.as_ref(),
                why,
            ),
            document.label.clone(),
        ))
    };
    // Each test port type the configuration imports, as the import enrichment
    // reads it for a generator: its `in` list and the scope its own imports
    // name its records in.
    let port_types: BTreeMap<&str, TestPortTypeImport> = document
        .parsed
        .imports
        .iter()
        .filter(|import| import.kind == ForgeKind::TestPortType)
        .filter_map(|import| {
            let port_type = read.reach.imported(&document.dir, import)?;
            match &port_type.parsed.document {
                ForgeDocument::TestPortType(m) => Some((
                    import.alias.as_str(),
                    TestPortTypeImport::of(m, port_type.scope.clone()),
                )),
                _ => None,
            }
        })
        .collect();
    let traced = trace_records_of(model, |alias| port_types.get(alias))
        .map_err(|e| read.refused(Located::in_file(e, document.label.clone())))?;
    for record in &traced.records {
        canonical_form(&Unspelled, &record.row.record_type, &(), record.row.scope)
            .map_err(configuration_refused)?;
    }

    for (index, input) in parsed.inputs.iter().enumerate() {
        // The words the player adds to a reader's refusal of an input
        // (`TraceAdapter::replaying`).
        let of_input = |why: &str| format!("input {index}, at {}: {why}", input.at);
        let Some(arm) = traced
            .arms
            .iter()
            .find(|arm| arm.port == input.port && arm.type_name == input.message_type)
        else {
            let refusal =
                TraceRefusal::not_an_input(&input.port, &input.message_type, &traced.system_ports);
            let (rule, member, written) = if traced.system_ports.contains(&input.port.as_str()) {
                (TraceRule::Type, "type", &input.message_type)
            } else {
                (TraceRule::Port, "port", &input.port)
            };
            refusals.push(placed(
                rule,
                format!("/inputs/{index}/{member}"),
                spelled(written),
                of_input(refusal.reason()),
            ));
            continue;
        };
        // Every type an arm names is one of the records, each once
        // (`trace_records_of`).
        if let Some(record) = traced
            .records
            .iter()
            .find(|record| record.type_name == arm.type_name)
        {
            let at = Place {
                value: &input.value,
                what: record.type_name.clone(),
                pointer: format!("/inputs/{index}/value"),
            };
            match read_value(&record.row.record_type, &at, record.row.scope) {
                Ok(()) => {}
                Err(Unread::Refused(misread)) => refusals.push(placed(
                    TraceRule::Value,
                    misread.pointer,
                    token(misread.value),
                    of_input(misread.refusal.reason()),
                )),
                Err(Unread::NoForm(why)) => return Err(configuration_refused(why)),
            }
        }
    }
    Ok(refusals)
}

/// The refusal of a trace for `rule`, at `pointer`, where it spells `actual`,
/// in the words `detail`.
fn refusal_of(rule: TraceRule, pointer: String, actual: String, detail: String) -> ForgeError {
    ValidationError::TestTrace {
        rule,
        pointer,
        actual,
        detail,
    }
    .into()
}

/// The test case the suite `suite`, read as `read`, runs under the name
/// `name`, which is the name its document gives itself and the name the
/// runner reads its trace by; none where the suite runs no such test case.
fn test_case<'r>(
    read: &'r ReadSuite,
    suite: &TestSuiteModel,
    name: &str,
) -> Result<Option<&'r Reached>, ExportError> {
    for execute in &suite.executes {
        let case = executed_case(
            suite,
            execute,
            &read.parsed.imports,
            &read.dir,
            &read.at,
            &read.reach,
        )
        .map_err(|e| read.refused(e))?;
        if matches!(&case.parsed.document, ForgeDocument::TestCase(m) if m.name == name) {
            return Ok(Some(case));
        }
    }
    Ok(None)
}

/// `text` as the trace spells it, where it is a token of the trace: a text the
/// canonical form writes with no escape. Empty for one it escapes, which the
/// trace does not hold as it reads.
fn spelled(text: &str) -> String {
    if text.chars().any(|c| c == '"' || c == '\\' || c < ' ') {
        String::new()
    } else {
        text.to_string()
    }
}

/// `value` as the token the trace spells it as, where it is one: a text no
/// escape changes, a truth value, a number, `null`. Empty for an array and an
/// object, which are no token.
fn token(value: &Value) -> String {
    match value {
        Value::Text(text) => spelled(text),
        Value::Bool(held) => held.to_string(),
        Value::Number(number) => number.clone(),
        Value::Null => "null".to_string(),
        Value::Array(_) | Value::Object(_) => String::new(),
    }
}

/// `name` as a reference token of a JSON pointer (RFC 6901): `~` as `~0`, `/`
/// as `~1`.
fn pointer_token(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

/// A value of a trace and where it stands: in the words of the Rust reader's
/// refusals, its record's type name, then each field by its id and `[]` for an
/// element of a list (`what`); in the trace, its JSON pointer.
struct Place<'v> {
    value: &'v Value,
    what: String,
    pointer: String,
}

impl<'v> Place<'v> {
    /// `value`, which stands in this value under `name` (a field's id, an
    /// element's index, a case's variant), named `what`.
    fn inner(&self, value: &'v Value, name: &str, what: String) -> Self {
        Place {
            value,
            what,
            pointer: format!("{}/{}", self.pointer, pointer_token(name)),
        }
    }

    /// This value, refused by the reader for `refusal`.
    fn refused(&self, refusal: TraceRefusal) -> Unread<'v> {
        Unread::Refused(Misread {
            pointer: self.pointer.clone(),
            value: self.value,
            refusal,
        })
    }

    /// The reader's answer `read` for this value, its value set aside.
    fn held<T>(&self, read: Result<T, TraceRefusal>) -> Result<(), Unread<'v>> {
        read.map(|_| ()).map_err(|refusal| self.refused(refusal))
    }
}

/// A value the reader refused: where it stands, the value, and why.
struct Misread<'v> {
    pointer: String,
    value: &'v Value,
    refusal: TraceRefusal,
}

/// Why a value is not read: the reader refused it, which is the trace's
/// refusal, or its type has no canonical form, which is the configuration's.
enum Unread<'v> {
    Refused(Misread<'v>),
    NoForm(String),
}

/// Read the value at `at` as a value of `ty`, read in `scope`, as the
/// generated Rust reader of its record reads it (the generator's
/// `RustReading`, over `canonical_form`): the first refusal, in the reader's
/// words, or none.
fn read_value<'v>(ty: &ValueType, at: &Place<'v>, scope: &TypeScope) -> Result<(), Unread<'v>> {
    match ty {
        TypeOf::Scalar(SceType::Bool) => at.held(from_canonical::boolean(at.value, &at.what)),
        TypeOf::Scalar(SceType::Enum(reference)) => {
            let declared = scope
                .enums
                .get(&reference.alias)
                .ok_or_else(|| Unread::NoForm(unimported("enum", &reference.alias)))?;
            read_enum(declared, at)
        }
        TypeOf::Scalar(scalar) => {
            read_integer(scalar, at).unwrap_or_else(|| Err(Unread::NoForm(formless(ty))))
        }
        TypeOf::Text { max } => at.held(from_canonical::text::<String>(
            at.value,
            max.max as usize,
            &at.what,
        )),
        TypeOf::Bytes { max } => at.held(from_canonical::bytes::<Vec<u8>>(
            at.value,
            max.max as usize,
            &at.what,
        )),
        TypeOf::Optional { .. } => Err(Unread::NoForm(optional_out_of_place(ty))),
        TypeOf::List { elem, max } => read_list(elem, max, at, scope),
        TypeOf::Record { alias } => read_record(alias, at, scope),
        TypeOf::Union { alias } => read_union(alias, at, scope),
    }
}

/// An integer of the width `scalar`, read into the Rust integer of that
/// width; `None` where `scalar` is no integer.
fn read_integer<'v>(scalar: &SceType, at: &Place<'v>) -> Option<Result<(), Unread<'v>>> {
    let (value, what) = (at.value, at.what.as_str());
    Some(match scalar {
        SceType::Uint8 => at.held(from_canonical::integer::<u8>(value, what)),
        SceType::Uint16 => at.held(from_canonical::integer::<u16>(value, what)),
        SceType::Uint32 => at.held(from_canonical::integer::<u32>(value, what)),
        SceType::Uint64 => at.held(from_canonical::integer::<u64>(value, what)),
        SceType::Int8 => at.held(from_canonical::integer::<i8>(value, what)),
        SceType::Int16 => at.held(from_canonical::integer::<i16>(value, what)),
        SceType::Int32 => at.held(from_canonical::integer::<i32>(value, what)),
        SceType::Int64 => at.held(from_canonical::integer::<i64>(value, what)),
        _ => return None,
    })
}

/// A value of the enum `declared`: by a declared name, and, for an enum of an
/// open set, by a number of its underlying type no variant declares.
fn read_enum<'v>(declared: &ScopedEnum, at: &Place<'v>) -> Result<(), Unread<'v>> {
    let variants: Vec<(&str, i128)> = declared
        .variants
        .iter()
        .map(|variant| (variant.name.as_str(), variant.value))
        .collect();
    if declared.strict {
        return at.held(from_canonical::enumerated(at.value, &variants, &at.what));
    }
    let (value, what) = (at.value, at.what.as_str());
    let read = match declared.underlying {
        SceType::Uint8 => open::<u8>(value, &variants, what),
        SceType::Uint16 => open::<u16>(value, &variants, what),
        SceType::Uint32 => open::<u32>(value, &variants, what),
        SceType::Uint64 => open::<u64>(value, &variants, what),
        SceType::Int8 => open::<i8>(value, &variants, what),
        SceType::Int16 => open::<i16>(value, &variants, what),
        SceType::Int32 => open::<i32>(value, &variants, what),
        SceType::Int64 => open::<i64>(value, &variants, what),
        _ => {
            return Err(Unread::NoForm(formless(&TypeOf::Scalar(
                declared.underlying.clone(),
            ))))
        }
    };
    at.held(read)
}

/// A value of an enum of an open set whose numbers are `U`s, among the
/// `declared` names and numbers.
fn open<U: TryFrom<i128> + Into<i128>>(
    value: &Value,
    declared: &[(&str, i128)],
    what: &str,
) -> Result<i128, TraceRefusal> {
    from_canonical::open_enumerated(value, declared, U::into, what)
}

/// A list of `elem` within the bounds `max`: its count first, then each
/// element, `null` where an element of a list of `optional<T>` is absent.
fn read_list<'v>(
    elem: &ValueType,
    max: &Bound,
    at: &Place<'v>,
    scope: &TypeScope,
) -> Result<(), Unread<'v>> {
    let items = from_canonical::elements(at.value, max.min as usize, max.max as usize, &at.what)
        .map_err(|refusal| at.refused(refusal))?;
    let what = format!("{}[]", at.what);
    for (index, item) in items.iter().enumerate() {
        let element = at.inner(item, &index.to_string(), what.clone());
        match elem {
            TypeOf::Optional { elem: held } => {
                if from_canonical::nullable(item).is_some() {
                    read_value(held, &element, scope)?;
                }
            }
            other => read_value(other, &element, scope)?,
        }
    }
    Ok(())
}

/// The record `record:<alias>` of `scope`: its members first, each a field it
/// declares once, then each field in the order the schema declares them, read
/// in the scope of the record's own document; a field whose type is optional
/// is read where it is present.
fn read_record<'v>(alias: &str, at: &Place<'v>, scope: &TypeScope) -> Result<(), Unread<'v>> {
    let record = scope
        .records
        .get(alias)
        .ok_or_else(|| Unread::NoForm(unimported("record", alias)))?;
    let ids: Vec<&str> = record.fields.iter().map(|(id, _)| id.as_str()).collect();
    let object =
        from_canonical::record(at.value, &ids, &at.what).map_err(|refusal| at.refused(refusal))?;
    for (id, field_type) in &record.fields {
        let what = format!("{}.{id}", at.what);
        match field_type {
            TypeOf::Optional { elem } => {
                if let Some(value) = from_canonical::optional_field(object, id) {
                    read_value(elem, &at.inner(value, id, what), &record.scope)?;
                }
            }
            held => {
                let value = from_canonical::field(object, id, &what)
                    .map_err(|refusal| at.refused(refusal))?;
                read_value(held, &at.inner(value, id, what), &record.scope)?;
            }
        }
    }
    Ok(())
}

/// The union `union:<alias>` of `scope`: one member named by its case's
/// variant, then the case's record, read in the scope of the union's own
/// document, or the empty object of a case that holds none.
fn read_union<'v>(alias: &str, at: &Place<'v>, scope: &TypeScope) -> Result<(), Unread<'v>> {
    let union = scope
        .unions
        .get(alias)
        .ok_or_else(|| Unread::NoForm(unimported("union", alias)))?;
    let variants: Vec<&str> = union
        .cases
        .iter()
        .map(|case| case.variant.name.as_str())
        .collect();
    let (variant, value) = from_canonical::union_case(at.value, &variants, &at.what)
        .map_err(|refusal| at.refused(refusal))?;
    // The reader has held the variant to the declared ones, so this reads the
    // one case it names.
    for case in union
        .cases
        .iter()
        .filter(|case| case.variant.name == variant)
    {
        let place = at.inner(value, variant, at.what.clone());
        match &case.record {
            Some(record) => read_record(record, &place, &union.scope)?,
            None => place.held(from_canonical::no_record(
                value,
                &format!("{}.{variant}", at.what),
            ))?,
        }
    }
    Ok(())
}

/// The direction of the walk of the canonical form that spells nothing:
/// walked over a type, it answers whether the type has a form, in the words
/// of the walk every generator's reader and writer take (`canonical_form`),
/// which are the generators' refusal of a record that has none.
struct Unspelled;

impl CanonicalForm for Unspelled {
    type At = ();

    fn root(&self) -> Self::At {}
    fn field(&self, _at: &(), _id: &str) -> Self::At {}
    fn bound(&self, _at: &(), _prefix: &str) -> (String, ()) {
        (String::new(), ())
    }
    fn case_record(&self, _bound: (), _record: &str) -> Self::At {}
    fn list_element(&self, _list: &(), _bound: ()) -> Self::At {}
    fn optional_value(&self, _field: &(), _bound: ()) -> Self::At {}
    fn boolean(&self, _at: &()) -> String {
        String::new()
    }
    fn enumerated(&self, _at: &(), _declared: &ScopedEnum) -> String {
        String::new()
    }
    fn integer(&self, _at: &(), _width: &SceType) -> String {
        String::new()
    }
    fn text(&self, _at: &(), _max: &Bound) -> String {
        String::new()
    }
    fn bytes(&self, _at: &(), _max: &Bound) -> String {
        String::new()
    }
    fn list(&self, _at: &(), _max: &Bound, _binding: &str, _element: String) -> String {
        String::new()
    }
    fn held_field(&self, _written: String) -> String {
        String::new()
    }
    fn optional_field(&self, _field: &(), _binding: &str, _written: String) -> String {
        String::new()
    }
    fn optional_element(&self, _element: &(), _binding: &str, _written: String) -> String {
        String::new()
    }
    fn record(&self, _at: &(), _record: &ScopedRecord, _fields: Vec<(String, String)>) -> String {
        String::new()
    }
    fn union(
        &self,
        _at: &(),
        _union: &ScopedUnion,
        _cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        Ok(String::new())
    }
}
