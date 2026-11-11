// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C11 units of the tables of a test (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.8, §12.2): a test configuration's `<name>_configuration`, the storage
//! and factory of its components' machines and the module parameters they
//! read; a test case's `<name>_test_case`; and a test suite's
//! `<name>_suite`, each the data the profile's runtime and its runner read
//! (`sce/test/test_case_run.h`, `sce/test/suite.h`). A configuration's unit also
//! reads the inputs of a recorded trace (`<name>_input`, docs/SCE_TTCN3_LANE.md
//! §10.5), and a suite's declares the `main` of its runner program and the reader
//! of its module parameters' values (§4, `sce/test/runner.h`).
//!
//! Which rows each unit holds, and what is refused, is decided by the rows
//! every backend shares ([`super::test_tables`]); this file spells them in
//! C11, and the templates (`forge/c/test_{configuration,case,suite}.h.jinja2`)
//! write the declarations around them.
//!
//! What is C's own: nothing is allocated. A configuration's tables are
//! constant storage its unit owns; the machines of its components are made
//! from storage the caller places (`<name>_machines_t`), one component of each
//! type for each component of the configuration of that type, which is the
//! most a run of it holds at once. A suite's storage (`<name>_storage_t`)
//! holds the suite, its entries and the machines of each of its test cases,
//! so a caller places one struct, whose size is known at compile time, for
//! every run of the suite. Each unit is header-only (`static inline`) and C
//! has no namespace, so each declaration carries the document's name, as the
//! units of the other C11 kinds do, and a unit reaches what it reads through
//! the header of the unit that declares it.

use super::c11_test_template::{c_literal as literal, indented, C11Canonical, C11Reading};
use super::test_tables::{
    configuration_rows, execute_rows, module_parameter_defaults, selection_body,
    test_case_configuration, trace_records, EndpointRow,
};
use super::test_template_walk::{
    canonical_form, canonical_value, hex_octets, CanonicalForm, CanonicalSpelling,
};
use super::*;
use crate::forge::value_type::{TypeOf, ValueType};
use crate::generator::Language;

/// The prefix every declaration of the unit of the document named `name`
/// carries, the one an importer reaches it by
/// ([`forge_import_identity`], `ImportContext::namespace`).
fn prefix(name: &str) -> String {
    filters::to_snake_case(name.to_string())
}

/// A component port a configuration names, as the initializer of an
/// `sce_test_endpoint_t`.
fn endpoint(row: EndpointRow<'_>) -> String {
    format!("{{{}, {}}}", literal(row.component), literal(row.port))
}

// ── §11.6: the configuration ───────────────────────────────────

/// The ports a component type declares.
fn ports_of(ty: &TestComponentImport) -> &[String] {
    &ty.ports
}

/// The timers a component type declares.
fn timers_of(ty: &TestComponentImport) -> &[String] {
    &ty.timers
}

/// The ids `ids_of` answers for each of `types`, as the initializers of the
/// `sce_test_declared_t` pairs of one table: the ports and the timers of
/// every component type are one table each, a pair per id, each type's in the
/// order it declares them.
fn declared(
    types: &[&TestComponentImport],
    ids_of: fn(&TestComponentImport) -> &[String],
) -> Vec<String> {
    types
        .iter()
        .flat_map(|&ty| {
            ids_of(ty)
                .iter()
                .map(move |id| format!("{{{}, {}}}", literal(&ty.machine), literal(id)))
        })
        .collect()
}

/// The unit of the test configuration `m`, whose imports are `imports`: the
/// configuration the runtime reads, the storage and the factory of the
/// machines of its component types, and the module parameters those
/// components read.
pub(super) fn render_test_configuration_c11(
    env: &minijinja::Environment,
    m: &TestConfigurationModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let rows = configuration_rows(m, imports)?;
    let ptcs: Vec<String> = rows
        .ptcs
        .iter()
        .map(|(id, component)| format!("{{{}, {}}}", literal(id), literal(&component.machine)))
        .collect();
    let ports = declared(&rows.component_types, ports_of);
    let timers = declared(&rows.component_types, timers_of);
    let connections: Vec<String> = rows
        .connections
        .iter()
        .map(|(a, b)| format!("{{{}, {}}}", endpoint(*a), endpoint(*b)))
        .collect();
    let mappings: Vec<String> = rows
        .mappings
        .iter()
        .map(|(port, system_port)| format!("{{{}, {}}}", endpoint(*port), literal(system_port)))
        .collect();
    // A run holds at once at most the components of the configuration, each
    // created once while it is present: the MTC and each PTC of a type.
    let machines: Vec<serde_json::Value> = rows
        .component_types
        .iter()
        .map(|ty| {
            let count = std::iter::once(rows.mtc)
                .chain(rows.ptcs.iter().map(|(_, component)| *component))
                .filter(|component| component.machine == ty.machine)
                .count();
            serde_json::json!({
                "name": literal(&ty.machine),
                "member": format!("{}_components", ty.machine),
                "component_type": format!("{}_component_t", ty.machine),
                "make": format!("{}_component_make", ty.machine),
                "count": count,
            })
        })
        .collect();
    let mut headers: Vec<String> = rows
        .component_types
        .iter()
        .map(|ty| format!("{}_sm.h", ty.machine))
        .collect();
    headers.sort();
    headers.dedup();
    let (system_ports, inputs, readers) = trace_inputs(m, imports)?;

    let l = LangCtx::new(Language::C11, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("prefix".into(), prefix(&m.name).into());
    ctx.insert("system_ports".into(), serde_json::json!(system_ports));
    ctx.insert("inputs".into(), serde_json::Value::Array(inputs));
    ctx.insert("readers".into(), serde_json::Value::Array(readers));
    ctx.insert("configuration_name".into(), m.name.clone().into());
    ctx.insert("mtc_type".into(), literal(&rows.mtc.machine).into());
    ctx.insert("ptcs".into(), serde_json::json!(ptcs));
    ctx.insert("ports".into(), serde_json::json!(ports));
    ctx.insert("timers".into(), serde_json::json!(timers));
    ctx.insert("connections".into(), serde_json::json!(connections));
    ctx.insert("mappings".into(), serde_json::json!(mappings));
    ctx.insert("machines".into(), serde_json::Value::Array(machines));
    ctx.insert(
        "module_parameters".into(),
        serde_json::json!(rows
            .module_parameters
            .iter()
            .map(|id| literal(id))
            .collect::<Vec<_>>()),
    );
    ctx.insert("machine_headers".into(), serde_json::json!(headers));
    l.render(env, "test_configuration", ctx)
}

/// What the unit of a configuration writes of the inputs a recorded trace may
/// carry ([`trace_inputs`]): each system port's id as a literal, an arm of
/// `<name>_input` per port and record, and each record's reader and writer once.
type TraceInputs = (Vec<String>, Vec<serde_json::Value>, Vec<serde_json::Value>);

/// The name of the unit's function that does `verb` with the record whose type
/// in the test log is `type_name`: `<configuration>_read_<record>`,
/// `<configuration>_canonical_<record>`.
fn record_function(m: &TestConfigurationModel, verb: &str, type_name: &str) -> String {
    format!(
        "{}_{verb}_{}",
        prefix(&m.name),
        filters::to_snake_case(type_name.to_string())
    )
}

/// The inputs a recorded trace may carry on the system ports of the test
/// configuration `m` (docs/SCE_TTCN3_LANE.md §10.2, §10.5), as every backend
/// reads them ([`trace_records`]): each system port's id; an arm of the unit's
/// `<name>_input` per port and record its type's `in` list names, in document
/// order; and each record's reader and writer of its canonical form and its
/// record type in the test log, once, the reader and the writer over the one
/// walk of the form ([`canonical_form`]), so the value a trace hands the run and
/// the value the test log writes of it are the same bytes. Each record is named
/// by its type in the test log, which names its functions, and its unit is
/// included, through which the enums and records it holds are.
fn trace_inputs(
    m: &TestConfigurationModel,
    imports: &[ImportContext],
) -> Result<TraceInputs, ForgeError> {
    let refused = |why: String| {
        super::test_tables::refused(
            ForgeKind::TestConfiguration,
            &m.name,
            m.source_location.as_ref(),
            why,
        )
    };
    let traced = trace_records(m, imports)?;
    let readers = traced
        .records
        .iter()
        .map(|record| {
            let row = &record.row;
            let reading = C11Reading {
                what: record.type_name.clone(),
            };
            let read_body = canonical_form(&reading, &row.record_type, &reading.root(), row.scope)
                .map_err(refused)?;
            let canonical_body = canonical_value(
                &C11Canonical,
                &row.record_type,
                &C11Canonical.root(),
                row.scope,
            )
            .map_err(refused)?;
            let include = forge_import_identity(
                &row.record.name,
                &Language::C11,
                false,
                &crate::ForgeCompileOptions::default(),
            )
            .include_stmt;
            Ok(serde_json::json!({
                "type": record.type_name,
                "type_literal": literal(&record.type_name),
                "read": record_function(m, "read", &record.type_name),
                "canonical": record_function(m, "canonical", &record.type_name),
                "record_type": record_function(m, "record_type_of", &record.type_name),
                "payload": event_schema_payload_type(&row.record.name, Language::C11),
                "read_body": indented(&read_body),
                "canonical_body": indented(&canonical_body),
                "include": include,
            }))
        })
        .collect::<Result<Vec<_>, ForgeError>>()?;
    let inputs = traced
        .arms
        .iter()
        .map(|arm| {
            serde_json::json!({
                "port": literal(arm.port),
                "type": literal(&arm.type_name),
                "event": format!("{}_EVENT_NAME", to_upper_snake(&arm.record.name)),
                "payload": event_schema_payload_type(&arm.record.name, Language::C11),
                "read": record_function(m, "read", &arm.type_name),
                "record_type": record_function(m, "record_type_of", &arm.type_name),
            })
        })
        .collect();
    let system_ports = traced.system_ports.iter().map(|id| literal(id)).collect();
    Ok((system_ports, inputs, readers))
}

// ── §11.7: the test case ───────────────────────────────────────

/// The unit of the test case `m`: its name, and its configuration's tables
/// over the storage of its configuration's machines.
pub(super) fn render_test_case_c11(
    env: &minijinja::Environment,
    m: &TestCaseModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let configuration = test_case_configuration(m, imports)?;
    let l = LangCtx::new(Language::C11, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("prefix".into(), prefix(&m.name).into());
    ctx.insert("case_name".into(), m.name.clone().into());
    ctx.insert(
        "name_macro".into(),
        format!("{}_NAME", to_upper_snake(&m.name)).into(),
    );
    ctx.insert("name_literal".into(), literal(&m.name).into());
    ctx.insert(
        "configuration_include".into(),
        configuration.include_stmt.clone().into(),
    );
    ctx.insert(
        "configuration".into(),
        configuration.namespace.clone().into(),
    );
    l.render(env, "test_case", ctx)
}

// ── §11.8: the suite ───────────────────────────────────────────

/// The unit of the test suite `m`: its module parameters at their defaults,
/// and its test cases in run order, each with its guard and its selection,
/// over storage that holds the suite and the machines of each test case.
pub(super) fn render_test_suite_c11(
    env: &minijinja::Environment,
    m: &TestSuiteModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let parameters: Vec<serde_json::Value> = module_parameter_defaults(m, "C11", saved_default)?
        .into_iter()
        .zip(&m.module_parameters)
        .map(|((id, value), parameter)| {
            let reader = parameter_reader(&parameter.parameter_type).ok_or_else(|| {
                super::test_tables::refused(
                    ForgeKind::TestSuite,
                    &m.name,
                    m.source_location.as_ref(),
                    format!(
                        "the module parameter '{id}' of type {} has no C11 reader of its value",
                        parameter.parameter_type
                    ),
                )
            })?;
            Ok(serde_json::json!({
                "id": literal(id),
                "value": value,
                "reader": reader,
            }))
        })
        .collect::<Result<_, ForgeError>>()?;
    let suite = prefix(&m.name);
    let mut includes: Vec<String> = Vec::new();
    // The storage of the machines of each test case, once however many
    // entries name it.
    let mut cases: Vec<String> = Vec::new();
    let mut selections: Vec<serde_json::Value> = Vec::new();
    let mut executes: Vec<serde_json::Value> = Vec::new();
    for (index, execute) in execute_rows(m, imports)?.into_iter().enumerate() {
        let case = execute.test_case;
        if !includes.contains(&case.include_stmt) {
            includes.push(case.include_stmt.clone());
        }
        if !cases.contains(&case.namespace) {
            cases.push(case.namespace.clone());
        }
        let selection = match &execute.selection {
            None => "NULL".to_string(),
            Some(expr) => {
                let function = format!("{suite}_selects_{}", index + 1);
                selections.push(serde_json::json!({
                    "function": function.as_str(),
                    "test_case": case.alias.as_str(),
                    "body": selection_body(expr, &|feature: &str| format!(
                        "sce_test_enabled(world, {})",
                        literal(feature)
                    )),
                }));
                function
            }
        };
        executes.push(serde_json::json!({
            "test_case": format!("{}_test_case", case.namespace),
            "machines": format!("{}_machines", case.namespace),
            "guarded": execute.guard.is_some(),
            "guard": execute.guard.map_or_else(|| "0".to_string(), |us| format!("INT64_C({us})")),
            "selection": selection,
        }));
    }
    let l = LangCtx::new(Language::C11, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("prefix".into(), suite.into());
    ctx.insert("suite_name".into(), m.name.clone().into());
    ctx.insert("name_literal".into(), literal(&m.name).into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("selections".into(), serde_json::Value::Array(selections));
    ctx.insert("executes".into(), serde_json::Value::Array(executes));
    ctx.insert("cases".into(), serde_json::json!(cases));
    ctx.insert("case_includes".into(), serde_json::json!(includes));
    l.render(env, "test_suite", ctx)
}

/// The runner's reader of a value of a module parameter of type `ty`, given on
/// its command line as a literal of docs/SCE_TEST_PROFILE.md §11.4
/// (docs/SCE_TTCN3_LANE.md §4): a call of a reader of `sce/test/runner.h` that
/// writes the saved value of `ty` the literal `literal` is, for the module
/// parameter `id`, into the `cap` bytes at `out`, as [`saved_default`] writes a
/// default, and refuses, naming why in `*err`, a literal that is none. An
/// integer is held to the range of its type, and one of 64 bits saved as the
/// text of its digits. `None` for the types [`saved_default`] has no value of, of
/// which no suite declares a module parameter.
fn parameter_reader(ty: &ValueType) -> Option<String> {
    Some(match ty {
        TypeOf::Scalar(SceType::Bool) => {
            "sce_test_runner_bool_parameter(id, literal, out, cap, err)".to_string()
        }
        TypeOf::Scalar(SceType::Enum(_)) => {
            "sce_test_runner_enum_parameter(id, literal, out, cap, err)".to_string()
        }
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
            let limit = c_type(scalar).trim_end_matches("_t").to_uppercase();
            let wide = scalar.int_bit_width() == Some(64);
            if scalar.is_signed() {
                format!(
                    "sce_test_runner_signed_parameter(id, literal, {limit}_MIN, {limit}_MAX, \
                     {wide}, out, cap, err)"
                )
            } else {
                format!(
                    "sce_test_runner_unsigned_parameter(id, literal, {limit}_MAX, {wide}, out, \
                     cap, err)"
                )
            }
        }
        TypeOf::Text { .. } => {
            "sce_test_runner_text_parameter(id, literal, out, cap, err)".to_string()
        }
        TypeOf::Bytes { .. } => {
            "sce_test_runner_bytes_parameter(id, literal, out, cap, err)".to_string()
        }
        TypeOf::Scalar(_)
        | TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => return None,
    })
}

/// The default `value` of a module parameter of type `ty`, a literal of
/// docs/SCE_TEST_PROFILE.md §11.4, as the C string literal of the compact JSON
/// text the runtime holds it as (`sce/test/module_parameters.h`): the saved
/// form a machine's saved state writes a value of that type as
/// (`sce/saved_values.h`), so the component reads it back by the reader of its
/// own type. A `bool` and an integer narrower than 64 bits are the JSON value
/// they are, a 64-bit integer is its decimal text, as a saved state writes one
/// JSON's numbers cannot all hold, a text is the JSON text of itself, a value
/// of an enum the name its document declares for the variant, and a byte
/// string its Latin-1 text, each octet the character of the same number.
/// `None` for a type a suite declares no module parameter of, and for a
/// literal that is none of its type.
fn saved_default(ty: &ValueType, value: &str) -> Option<String> {
    let text = |text: String| serde_json::Value::String(text).to_string();
    let json = match ty {
        TypeOf::Scalar(SceType::Bool) => match value.trim() {
            held @ ("true" | "false") => held.to_string(),
            _ => return None,
        },
        TypeOf::Scalar(SceType::Enum(_)) => text(value.trim().to_string()),
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
            let n = crate::forge::test_profile::decimal_integer(value)?;
            let (least, most) = scalar.int_value_range()?;
            if n < least || n > most {
                return None;
            }
            if scalar.int_bit_width() == Some(64) {
                text(n.to_string())
            } else {
                n.to_string()
            }
        }
        TypeOf::Text { .. } => text(value.to_string()),
        TypeOf::Bytes { .. } => {
            let mut latin1 = String::new();
            for pair in hex_octets(value)? {
                latin1.push(char::from(u8::from_str_radix(&pair, 16).ok()?));
            }
            text(latin1)
        }
        TypeOf::Scalar(_)
        | TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => return None,
    };
    Some(literal(&json))
}
