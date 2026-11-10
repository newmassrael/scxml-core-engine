// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust units of the tables of a test (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.8, §12.2): a test configuration's `configuration`, `machine` and
//! `module_parameters`, a test case's `test_case`, and a test suite's `suite`,
//! each the data the profile's runtime and its runner read
//! (`sce_rust_runtime::test`, `run_suite`).
//!
//! Which rows each unit holds, and what is refused, is decided by the rows
//! every backend shares ([`super::test_tables`]); this file spells them in
//! Rust, and the templates (`forge/rust/test_{configuration,case,suite}.rs.jinja2`)
//! write the declarations around them.
//!
//! A unit names each unit it reads through its document's sibling module
//! (`super::<snake>`), and a component's machine through the module its
//! statechart is generated as (`super::<name>_sm`), as every generated Rust
//! file reaches its siblings, so no unit has an import line to leave unread.

use super::rust_test_template::{
    rust_literal as literal, rust_record_type, RustCanonical, RustReading,
};
use super::test_tables::{
    configuration_rows, execute_rows, module_parameter_defaults, selection_body, system_port_rows,
    test_case_configuration, trace_record_rows, EndpointRow,
};
use super::test_template_walk::{
    canonical_form, canonical_value, hex_octets, integer_literal, log_type_name, CanonicalForm,
    CanonicalSpelling,
};
use super::*;
use crate::forge::value_type::{TypeOf, ValueType};
use crate::generator::Language;

/// `text` as an owned Rust `String`.
fn owned(text: &str) -> String {
    format!("{}.to_string()", literal(text))
}

/// The component the profile's runtime drives for `component`, generated
/// beside its machine (`<Machine>Component`, docs/SCE_TEST_PROFILE.md §12.3).
fn component_path(component: &TestComponentImport) -> String {
    format!(
        "super::{}_sm::{}Component",
        component.machine,
        filters::to_pascal_case(component.machine.clone())
    )
}

// ── §11.6: the configuration ───────────────────────────────────

/// The unit of the test configuration `m`, whose imports are `imports`: the
/// configuration the runtime reads, the factory of the machines of its
/// component types and the module parameters those components read.
pub(super) fn render_test_configuration_rust(
    env: &minijinja::Environment,
    m: &TestConfigurationModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let rows = configuration_rows(m, imports)?;
    let ptcs: Vec<String> = rows
        .ptcs
        .iter()
        .map(|(id, component)| format!("({}, {})", owned(id), owned(&component.machine)))
        .collect();
    // A type that declares no port, or no timer, has no entry in that table,
    // as the runtime reads an absent entry as none.
    let entry = |machine: &str, ids: &[String]| -> Option<String> {
        if ids.is_empty() {
            return None;
        }
        let ids: Vec<String> = ids.iter().map(|id| owned(id)).collect();
        Some(format!("({}, vec![{}])", owned(machine), ids.join(", ")))
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
                "component": component_path(ty),
            })
        })
        .collect();
    let endpoint = |row: EndpointRow<'_>| {
        format!(
            "::sce_rust_runtime::test::Endpoint::new({}, {})",
            literal(row.component),
            literal(row.port)
        )
    };
    let connections: Vec<String> = rows
        .connections
        .iter()
        .map(|(a, b)| format!("({}, {})", endpoint(*a), endpoint(*b)))
        .collect();
    let mappings: Vec<String> = rows
        .mappings
        .iter()
        .map(|(port, system_port)| format!("({}, {})", endpoint(*port), owned(system_port)))
        .collect();
    let module_parameters: Vec<String> =
        rows.module_parameters.iter().map(|id| owned(id)).collect();
    let (system_ports, inputs, readers) = trace_inputs(m, imports)?;

    let l = LangCtx::new(Language::Rust, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("system_ports".into(), serde_json::json!(system_ports));
    ctx.insert("inputs".into(), serde_json::Value::Array(inputs));
    ctx.insert("readers".into(), serde_json::Value::Array(readers));
    ctx.insert("configuration_name".into(), m.name.clone().into());
    ctx.insert("mtc_type".into(), owned(&rows.mtc.machine).into());
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
/// carry ([`trace_inputs`]): each system port's id as a literal, an arm of
/// `input` per port and record, and each record's reader and writer once.
type TraceInputs = (Vec<String>, Vec<serde_json::Value>, Vec<serde_json::Value>);

/// The inputs a recorded trace may carry on the system ports of the test
/// configuration `m` (docs/SCE_TTCN3_LANE.md §10.2, §10.5): each system port's
/// id; an arm of the unit's `input` per port and record its type's `in` list
/// names, in document order; and each record's reader and writer of its
/// canonical form, once ([`trace_record_rows`]), both over the one walk of the
/// form ([`canonical_form`]), so the value a trace hands the run and the value
/// the test log writes of it are the same bytes.
///
/// Each record is named by its type in the test log ([`log_type_name`]), which
/// names its functions; two records of one name are refused by name, as the
/// types module of the export would hold them under one name.
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
    let ports = system_port_rows(m, imports)?;
    let readers = trace_record_rows(m, &ports)?
        .into_iter()
        .map(|row| {
            let reading = RustReading {
                what: row.type_name.clone(),
            };
            let read_expr = canonical_form(
                &reading,
                &row.input.record_type,
                &reading.root(),
                row.input.scope,
            )
            .map_err(refused)?;
            let canonical_expr = canonical_value(
                &RustCanonical,
                &row.input.record_type,
                &RustCanonical.root(),
                row.input.scope,
            )
            .map_err(refused)?;
            Ok(serde_json::json!({
                "type": row.type_name,
                "read": format!("read_{}", row.type_name),
                "canonical": format!("canonical_{}", row.type_name),
                "payload": rust_record_type(row.input.record),
                "read_expr": read_expr,
                "canonical_expr": canonical_expr,
            }))
        })
        .collect::<Result<Vec<_>, ForgeError>>()?;
    let inputs = ports
        .iter()
        .flat_map(|port| port.inputs.iter().map(move |input| (port.id, input)))
        .map(|(port, input)| {
            let type_name = log_type_name(input.record);
            serde_json::json!({
                "port": literal(port),
                "type": literal(&type_name),
                "event": format!(
                    "super::{}::EVENT_NAME",
                    filters::to_snake_case(input.record.name.clone())
                ),
                "read": format!("read_{type_name}"),
                "canonical": format!("canonical_{type_name}"),
            })
        })
        .collect();
    let system_ports = ports.iter().map(|port| literal(port.id)).collect();
    Ok((system_ports, inputs, readers))
}

// ── §11.7: the test case ───────────────────────────────────────

/// The unit of the test case `m`: its name, and its configuration's tables.
pub(super) fn render_test_case_rust(
    env: &minijinja::Environment,
    m: &TestCaseModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let configuration = test_case_configuration(m, imports)?;
    let l = LangCtx::new(Language::Rust, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("case_name".into(), m.name.clone().into());
    ctx.insert("name_literal".into(), literal(&m.name).into());
    ctx.insert(
        "configuration".into(),
        format!("super::{}", configuration.namespace).into(),
    );
    l.render(env, "test_case", ctx)
}

// ── §11.8: the suite ───────────────────────────────────────────

/// The unit of the test suite `m`: its module parameters at their defaults,
/// and its test cases in run order, each with its guard and its selection.
pub(super) fn render_test_suite_rust(
    env: &minijinja::Environment,
    m: &TestSuiteModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let parameters: Vec<serde_json::Value> = module_parameter_defaults(m, "Rust", saved_default)?
        .into_iter()
        .zip(&m.module_parameters)
        .map(|((id, value), parameter)| {
            let reader = parameter_reader(&parameter.parameter_type).ok_or_else(|| {
                super::test_tables::refused(
                    ForgeKind::TestSuite,
                    &m.name,
                    m.source_location.as_ref(),
                    format!(
                        "the module parameter '{id}' of type {} has no Rust reader of its value",
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
            let guard = match execute.guard {
                None => "None".to_string(),
                Some(us) => format!("Some({us})"),
            };
            let selection = match &execute.selection {
                None => "None".to_string(),
                Some(expr) => format!(
                    "Some(|world: &::sce_rust_runtime::test::Features| {})",
                    selection_body(expr, &|feature: &str| format!(
                        "::sce_rust_runtime::test::enabled(world, {})",
                        literal(feature)
                    ))
                ),
            };
            serde_json::json!({
                "test_case": format!("super::{}::test_case()", execute.test_case.namespace),
                "guard": guard,
                "selection": selection,
            })
        })
        .collect();
    let l = LangCtx::new(Language::Rust, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("suite_name".into(), m.name.clone().into());
    ctx.insert("name_literal".into(), literal(&m.name).into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("executes".into(), serde_json::Value::Array(executes));
    l.render(env, "test_suite", ctx)
}

/// The runner's reader of a value of a module parameter of type `ty`, given
/// on its command line as a literal of docs/SCE_TEST_PROFILE.md §11.4
/// (docs/SCE_TTCN3_LANE.md §4): an expression of the saved value of `ty` the
/// literal `literal` is, or of why it is none, for the module parameter `id`,
/// by the readers of `sce_rust_runtime::test::runner`. `None` for the types
/// [`saved_default`] has no value of, of which no suite declares a module
/// parameter.
fn parameter_reader(ty: &ValueType) -> Option<String> {
    const RUNNER: &str = "::sce_rust_runtime::test::runner";
    match ty {
        TypeOf::Scalar(SceType::Bool) => Some(format!("{RUNNER}::bool_parameter(id, literal)")),
        TypeOf::Scalar(SceType::Enum(_)) => Some(format!("Ok({RUNNER}::enum_parameter(literal))")),
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => Some(format!(
            "{RUNNER}::integer_parameter::<{}>(id, literal)",
            rust_type(scalar)
        )),
        TypeOf::Text { .. } => Some(format!("Ok({RUNNER}::text_parameter(literal))")),
        TypeOf::Bytes { .. } => Some(format!("{RUNNER}::bytes_parameter(id, literal)")),
        TypeOf::Scalar(_)
        | TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => None,
    }
}

/// The default `value` of a module parameter of type `ty`, a literal of
/// docs/SCE_TEST_PROFILE.md §11.4, as the saved value the runtime holds it in
/// (`sce_rust_runtime::test::ModuleParameters`): the value a machine's saved
/// state writes a value of that type as, so the component reads it back by its
/// own type (`SavedValue::from_saved`). An integer is written by the writer of
/// its type, which writes a 64-bit one as its text; a value of an enum is saved
/// as the name its document declares for the variant, and a byte string as its
/// Latin-1 text (`saved_state::bytes_to_saved`). `None` for a type a suite
/// declares no module parameter of, and for a byte string whose literal is no
/// hex.
fn saved_default(ty: &ValueType, value: &str) -> Option<String> {
    match ty {
        TypeOf::Scalar(SceType::Bool) => match value.trim() {
            held @ ("true" | "false") => {
                Some(format!("::sce_rust_runtime::json::Value::Bool({held})"))
            }
            _ => None,
        },
        TypeOf::Scalar(SceType::Enum(_)) => Some(format!(
            "::sce_rust_runtime::json::Value::Text({})",
            owned(value.trim())
        )),
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => Some(format!(
            "<{} as ::sce_rust_runtime::saved_state::SavedValue>::to_saved(&{})",
            rust_type(scalar),
            integer_literal(Language::Rust, value)?
        )),
        TypeOf::Text { .. } => Some(format!(
            "::sce_rust_runtime::json::Value::Text({})",
            owned(value)
        )),
        TypeOf::Bytes { .. } => {
            let octets: String = hex_octets(value)?
                .into_iter()
                .map(|pair| format!("\\x{pair}"))
                .collect();
            Some(format!(
                "::sce_rust_runtime::saved_state::bytes_to_saved(b\"{octets}\")"
            ))
        }
        TypeOf::Scalar(_)
        | TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => None,
    }
}
