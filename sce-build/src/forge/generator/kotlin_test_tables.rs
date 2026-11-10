// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Kotlin units of the tables of a test (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.8, §12.2): a test configuration's `configuration`, `machine` and
//! `moduleParameters`, with the reader and the writer of each record a
//! recorded input trace carries on its system ports and `input`, which reads
//! one (docs/SCE_TTCN3_LANE.md §10.5); a test case's `testCase`; and a test
//! suite's `suite`, `moduleParameter` and the runner program's `main`
//! (docs/SCE_TTCN3_LANE.md §4), each the data the profile's runtime and its
//! runner read (`com.sce.runtime.test`, `runSuite`, `runSuiteWithTraces`).
//!
//! Which rows each unit holds, and what is refused, is decided by the rows
//! every backend shares ([`super::test_tables`]); this file spells them in
//! Kotlin, and the templates
//! (`forge/kotlin/test_{configuration,case,suite}.kt.jinja2`) write the
//! declarations around them.
//!
//! A unit imports no package: it names each unit it reads by the package of
//! that unit's document ([`kotlin_qualified`]), a component's machine by the
//! package its statechart is generated into when no package prefix is given
//! ([`crate::generator::kotlin_machine_package`]), and the runtime by its own
//! package, so no name another unit declares (each table's own
//! `configuration`, `machine`, `testCase` and `suite` among them) can hide one
//! of its own.

use super::kotlin_test_template::{
    kotlin_bytes_literal, kotlin_integer_literal, kotlin_literal as literal, kotlin_record_type,
    KotlinCanonical, KotlinReading,
};
use super::test_tables::{
    configuration_rows, execute_rows, module_parameter_defaults, selection_body,
    test_case_configuration, trace_records, EndpointRow,
};
use super::test_template_walk::{
    canonical_form, canonical_value, CanonicalForm, CanonicalSpelling,
};
use super::*;
use crate::forge::value_type::{TypeOf, ValueType};
use crate::generator::Language;

/// The package of the profile's Kotlin runtime and runner.
const RUNTIME: &str = "com.sce.runtime.test";

/// The component the profile's runtime drives for `component`, generated
/// beside its machine (`<Machine>Component`, docs/SCE_TEST_PROFILE.md §12.3),
/// in the package its statechart is generated into when the run names no
/// `--kotlin-package-prefix`: a configuration cannot know the prefix another
/// run chose, as an importer of a forge unit cannot.
fn component_class(component: &TestComponentImport) -> String {
    format!(
        "{}.{}Component",
        crate::generator::kotlin_machine_package(None, &component.machine),
        filters::to_pascal_case(component.machine.clone())
    )
}

// ── §11.6: the configuration ───────────────────────────────────

/// The unit of the test configuration `m`, whose imports are `imports`: the
/// configuration the runtime reads, the factory of the machines of its
/// component types and the module parameters those components read.
pub(super) fn render_test_configuration_kotlin(
    env: &minijinja::Environment,
    m: &TestConfigurationModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let rows = configuration_rows(m, imports)?;
    let ptcs: Vec<String> = rows
        .ptcs
        .iter()
        .map(|(id, component)| format!("{} to {}", literal(id), literal(&component.machine)))
        .collect();
    // A type that declares no port, or no timer, has no entry in that table,
    // as the runtime reads an absent entry as none.
    let entry = |machine: &str, ids: &[String]| -> Option<String> {
        if ids.is_empty() {
            return None;
        }
        let ids: Vec<String> = ids.iter().map(|id| literal(id)).collect();
        Some(format!(
            "{} to listOf({})",
            literal(machine),
            ids.join(", ")
        ))
    };
    let ports_of_type: Vec<String> = rows
        .component_types
        .iter()
        .filter_map(|ty| entry(&ty.machine, &ty.ports))
        .collect();
    let timers_of_type: Vec<String> = rows
        .component_types
        .iter()
        .filter_map(|ty| entry(&ty.machine, &ty.timers))
        .collect();
    let machines: Vec<serde_json::Value> = rows
        .component_types
        .iter()
        .map(|ty| {
            serde_json::json!({
                "name": literal(&ty.machine),
                "component": component_class(ty),
            })
        })
        .collect();
    let endpoint = |row: EndpointRow<'_>| {
        format!(
            "{RUNTIME}.Endpoint({}, {})",
            literal(row.component),
            literal(row.port)
        )
    };
    let connections: Vec<String> = rows
        .connections
        .iter()
        .map(|(a, b)| format!("{} to {}", endpoint(*a), endpoint(*b)))
        .collect();
    let mappings: Vec<String> = rows
        .mappings
        .iter()
        .map(|(port, system_port)| format!("{} to {}", endpoint(*port), literal(system_port)))
        .collect();
    let module_parameters: Vec<String> = rows
        .module_parameters
        .iter()
        .map(|id| literal(id))
        .collect();
    let (system_ports, inputs, readers) = trace_inputs(m, imports)?;

    let l = LangCtx::new(Language::Kotlin, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("system_ports".into(), serde_json::json!(system_ports));
    ctx.insert("inputs".into(), serde_json::Value::Array(inputs));
    ctx.insert("readers".into(), serde_json::Value::Array(readers));
    ctx.insert("configuration_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), RUNTIME.into());
    ctx.insert("mtc_type".into(), literal(&rows.mtc.machine).into());
    ctx.insert("ptc_types".into(), serde_json::json!(ptcs));
    ctx.insert("ports_of_type".into(), serde_json::json!(ports_of_type));
    ctx.insert("timers_of_type".into(), serde_json::json!(timers_of_type));
    ctx.insert("connections".into(), serde_json::json!(connections));
    ctx.insert("mappings".into(), serde_json::json!(mappings));
    ctx.insert("machines".into(), serde_json::Value::Array(machines));
    ctx.insert(
        "module_parameters".into(),
        serde_json::json!(module_parameters),
    );
    l.render(env, "test_configuration", ctx)
}

/// What the unit of a configuration writes of the inputs a recorded trace may
/// carry ([`trace_inputs`]): each system port's id as a literal, a branch of
/// `input` per port and record, and each record's reader and writer once.
type TraceInputs = (Vec<String>, Vec<serde_json::Value>, Vec<serde_json::Value>);

/// The inputs a recorded trace may carry on the system ports of the test
/// configuration `m` (docs/SCE_TTCN3_LANE.md §10.2, §10.5), as every backend
/// reads them ([`trace_records`]): each system port's id; a branch of the
/// unit's `input` per port and record its type's `in` list names, in document
/// order; and each record's reader and writer of its canonical form, once,
/// both over the one walk of the form ([`canonical_form`]), so the value a
/// trace hands the run and the value the test log writes of it are the same
/// bytes.
///
/// Each record is named by its type in the test log, which names its
/// functions, `read_<type>` and `canonical_<type>`, as the Rust unit names
/// them.
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
            let reading = KotlinReading {
                what: record.type_name.clone(),
            };
            let read_expr = canonical_form(&reading, &row.record_type, &reading.root(), row.scope)
                .map_err(refused)?;
            let canonical_expr = canonical_value(
                &KotlinCanonical,
                &row.record_type,
                &KotlinCanonical.root(),
                row.scope,
            )
            .map_err(refused)?;
            Ok(serde_json::json!({
                "type": record.type_name,
                "read": format!("read_{}", record.type_name),
                "canonical": format!("canonical_{}", record.type_name),
                "payload": kotlin_record_type(row.record),
                "read_expr": read_expr,
                "canonical_expr": canonical_expr,
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
                "event": kotlin_qualified(&arm.record.name, "EVENT_NAME"),
                "read": format!("read_{}", arm.type_name),
                "canonical": format!("canonical_{}", arm.type_name),
            })
        })
        .collect();
    let system_ports = traced.system_ports.iter().map(|id| literal(id)).collect();
    Ok((system_ports, inputs, readers))
}

// ── §11.7: the test case ───────────────────────────────────────

/// The unit of the test case `m`: its name, and its configuration's tables.
pub(super) fn render_test_case_kotlin(
    env: &minijinja::Environment,
    m: &TestCaseModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let configuration = test_case_configuration(m, imports)?;
    let l = LangCtx::new(Language::Kotlin, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("case_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), RUNTIME.into());
    ctx.insert("name_literal".into(), literal(&m.name).into());
    // The four declarations of the configuration's unit the entry reads.
    let of_configuration = |name: &str| kotlin_qualified(&configuration.document_name, name);
    ctx.insert(
        "configuration".into(),
        of_configuration("configuration").into(),
    );
    ctx.insert("machine".into(), of_configuration("machine").into());
    ctx.insert(
        "module_parameters".into(),
        of_configuration("moduleParameters").into(),
    );
    ctx.insert("input".into(), of_configuration("input").into());
    l.render(env, "test_case", ctx)
}

// ── §11.8: the suite ───────────────────────────────────────────

/// The unit of the test suite `m`: its module parameters at their defaults,
/// and its test cases in run order, each with its guard and its selection.
pub(super) fn render_test_suite_kotlin(
    env: &minijinja::Environment,
    m: &TestSuiteModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let parameters: Vec<serde_json::Value> = module_parameter_defaults(m, "Kotlin", saved_default)?
        .into_iter()
        .zip(&m.module_parameters)
        .map(|((id, value), parameter)| {
            let reader = parameter_reader(&parameter.parameter_type).ok_or_else(|| {
                super::test_tables::refused(
                    ForgeKind::TestSuite,
                    &m.name,
                    m.source_location.as_ref(),
                    format!(
                        "the module parameter '{id}' of type {} has no Kotlin reader of its value",
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
    let executes: Vec<serde_json::Value> = execute_rows(m, imports)?
        .into_iter()
        .map(|execute| {
            // The runtime's clock counts microseconds in a `Long`.
            let guard = match execute.guard {
                None => "null".to_string(),
                Some(us) => format!("{us}L"),
            };
            let selection = match &execute.selection {
                None => "null".to_string(),
                Some(expr) => format!(
                    "{{ world -> {} }}",
                    selection_body(expr, &|feature: &str| format!(
                        "{RUNTIME}.enabled(world, {})",
                        literal(feature)
                    ))
                ),
            };
            serde_json::json!({
                "test_case": format!(
                    "{}()",
                    kotlin_qualified(&execute.test_case.document_name, "testCase")
                ),
                "guard": guard,
                "selection": selection,
            })
        })
        .collect();
    let l = LangCtx::new(Language::Kotlin, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("suite_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), RUNTIME.into());
    ctx.insert("name_literal".into(), literal(&m.name).into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("executes".into(), serde_json::Value::Array(executes));
    l.render(env, "test_suite", ctx)
}

/// The runner's reader of a value of a module parameter of type `ty`, given
/// on its command line as a literal of docs/SCE_TEST_PROFILE.md §11.4
/// (docs/SCE_TTCN3_LANE.md §4): an expression of the saved value of `ty` the
/// literal `literal` is, for the module parameter `id`, by the readers of the
/// runtime's runner (`Runner.kt`), which throw an `OverrideRefusal` naming why
/// where it is none. An integer is read at its type's width and saved by
/// `SavedValues.of` of that type, as its default is ([`saved_default`]).
/// `None` for the types [`saved_default`] has no value of, of which no suite
/// declares a module parameter.
fn parameter_reader(ty: &ValueType) -> Option<String> {
    match ty {
        TypeOf::Scalar(SceType::Bool) => Some(format!("{RUNTIME}.boolParameter(id, literal)")),
        TypeOf::Scalar(SceType::Enum(_)) => Some(format!("{RUNTIME}.enumParameter(literal)")),
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => Some(format!(
            "com.sce.runtime.SavedValues.of({RUNTIME}.integerParameter(id, literal, \
             {RUNTIME}.FromCanonical.{}))",
            scalar.as_attr().to_uppercase()
        )),
        TypeOf::Text { .. } => Some(format!("{RUNTIME}.textParameter(literal)")),
        TypeOf::Bytes { .. } => Some(format!("{RUNTIME}.bytesParameter(id, literal)")),
        TypeOf::Scalar(_)
        | TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => None,
    }
}

/// The default `value` of a module parameter of type `ty`, a literal of
/// docs/SCE_TEST_PROFILE.md §11.4, as the saved value the runtime holds it in
/// (`com.sce.runtime.test.ModuleParameters`): the value a machine's saved
/// state writes a value of that type as, so the component reads it back by its
/// own type's reader (`SavedValues`). A number, a `bool`, a text and a byte
/// string are written by `SavedValues.of` of a literal of their own type, which
/// writes a 64-bit integer as its text and a byte string as its Latin-1 text;
/// a value of an enum is saved as the name its document declares for the
/// variant. `None` for a type a suite declares no module parameter of, and for
/// a literal that is none of its type.
fn saved_default(ty: &ValueType, value: &str) -> Option<String> {
    let written = match ty {
        TypeOf::Scalar(SceType::Bool) => match value.trim() {
            held @ ("true" | "false") => held.to_string(),
            _ => return None,
        },
        TypeOf::Scalar(SceType::Enum(_)) => return Some(literal(value.trim())),
        TypeOf::Scalar(scalar) => kotlin_integer_literal(scalar, value)?,
        TypeOf::Text { .. } => filters::kotlin_text_expression(value),
        TypeOf::Bytes { .. } => kotlin_bytes_literal(value)?,
        TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => return None,
    };
    Some(format!("com.sce.runtime.SavedValues.of({written})"))
}
