// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python units of the tables of a test (docs/SCE_TEST_PROFILE.md §11.6
//! to §11.8, §12.2): a test configuration's `configuration`, `machine` and
//! `module_parameters`, a test case's `test_case`, and a test suite's `suite`,
//! each the data the profile's runtime and its runner read
//! (`sce_runtime.testprofile`, `run_suite`).
//!
//! Which rows each unit holds, and what is refused, is decided by the rows
//! every backend shares ([`super::test_tables`]); this file spells them in
//! Python, and the templates
//! (`forge/python/test_{configuration,case,suite}.py.jinja2`) write the
//! declarations around them.
//!
//! A unit imports each module it reads as a sibling, as every Python unit
//! does: a suite its test cases' units and a test case its configuration's,
//! each under the alias every Python unit imports a sibling by
//! ([`python_module_alias`], through [`forge_import_identity`]). A
//! configuration imports the module of each component's machine, which is the
//! module the component's statechart is generated as: `<machine>_sm`, the
//! name [`TestComponentImport::machine`] holds being the stem of the
//! statechart's file, as the file `<stem>_sm.py` is named
//! (`python/state_machine.py.jinja2`), and the component the runtime drives
//! is the `<Machine>Component` it declares. So a configuration, its
//! components, its test cases and the suite are one package, as the machines
//! and the units they import are.
//!
//! The runtime modules a unit reads are imported under names with a leading
//! underscore: the profile's runtime under the name a component's machine
//! imports it by ([`crate::forge::static_lowering::PYTHON_TEST_RUNTIME`]), and
//! the writers of a saved value under [`SAVED_VALUES`]. No sibling's alias
//! (`sce_<document>`) and no declaration of a unit (`configuration`,
//! `machine`, `module_parameters`, `NAME`, `test_case`, `suite`) begins with
//! one.

use super::test_tables::{
    configuration_rows, execute_rows, module_parameter_defaults, selection_body_in,
    test_case_configuration, EndpointRow,
};
use super::test_template_walk::{hex_octets, integer_literal, Connectives};
use super::*;
use crate::forge::static_lowering::PYTHON_TEST_RUNTIME as RUNTIME;
use crate::forge::value_type::{TypeOf, ValueType};
use crate::generator::Language;

/// The name the runtime's writers of a saved value
/// ([`SAVED_VALUES_PATH`]) are bound to in a suite's unit, which writes each
/// module parameter's default through the writer of its type
/// (`saved_write_<type>`).
const SAVED_VALUES: &str = "_sce_saved";

/// The module of the runtime's writers and readers of a saved value.
const SAVED_VALUES_PATH: &str = "sce_runtime.saved_values";

/// `text` as a Python string literal.
fn literal(text: &str) -> String {
    filters::py_string_literal(text.to_string())
}

/// The import statement and the alias of the sibling module of the document
/// `document`.
fn sibling(document: &str, options: &crate::ForgeCompileOptions) -> (String, String) {
    let identity = forge_import_identity(document, &Language::Python, false, options);
    (identity.include_stmt, identity.namespace)
}

/// `statement` added to `statements` unless it is there already: a module two
/// rows name is imported once.
fn import_once(statements: &mut Vec<String>, statement: String) {
    if !statements.contains(&statement) {
        statements.push(statement);
    }
}

/// The import statement and the alias of the module the machine of
/// `component` is generated as: `<machine>_sm`, beside the unit.
fn component_module(component: &TestComponentImport) -> (String, String) {
    let module = format!("{}_sm", component.machine);
    let alias = python_module_alias(&module);
    (format!("from . import {module} as {alias}"), alias)
}

/// The component the profile's runtime drives for `component`, generated
/// beside its machine (`<Machine>Component`, docs/SCE_TEST_PROFILE.md §12.3),
/// through the module's alias `alias`.
fn component_type(component: &TestComponentImport, alias: &str) -> String {
    format!(
        "{alias}.{}Component",
        filters::to_pascal_case(component.machine.clone())
    )
}

// ── §11.6: the configuration ───────────────────────────────────

/// The unit of the test configuration `m`, whose imports are `imports`: the
/// configuration the runtime reads, the factory of the machines of its
/// component types and the module parameters those components read.
pub(super) fn render_test_configuration_python(
    env: &minijinja::Environment,
    m: &TestConfigurationModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let rows = configuration_rows(m, imports)?;
    let ptcs: Vec<String> = rows
        .ptcs
        .iter()
        .map(|(id, component)| format!("{}: {}", literal(id), literal(&component.machine)))
        .collect();
    // A type that declares no port, or no timer, has no entry in that table,
    // as the runtime reads an absent entry as none.
    let entry = |machine: &str, ids: &[String]| -> Option<String> {
        if ids.is_empty() {
            return None;
        }
        let ids: Vec<String> = ids.iter().map(|id| literal(id)).collect();
        Some(format!("{}: [{}]", literal(machine), ids.join(", ")))
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
    let mut python_imports = Vec::new();
    let mut machines = Vec::new();
    for ty in &rows.component_types {
        let (statement, alias) = component_module(ty);
        import_once(&mut python_imports, statement);
        machines.push(serde_json::json!({
            "name": literal(&ty.machine),
            "component": component_type(ty, &alias),
        }));
    }
    let endpoint = |row: EndpointRow<'_>| {
        format!(
            "{RUNTIME}.Endpoint(component={}, port={})",
            literal(row.component),
            literal(row.port)
        )
    };
    let connections: Vec<String> = rows
        .connections
        .iter()
        .map(|(a, b)| {
            format!(
                "{RUNTIME}.Connection(a={}, b={})",
                endpoint(*a),
                endpoint(*b)
            )
        })
        .collect();
    let mappings: Vec<String> = rows
        .mappings
        .iter()
        .map(|(port, system_port)| {
            format!(
                "{RUNTIME}.Mapping(port={}, system_port={})",
                endpoint(*port),
                literal(system_port)
            )
        })
        .collect();
    let module_parameters: Vec<String> = rows
        .module_parameters
        .iter()
        .map(|id| literal(id))
        .collect();

    let l = LangCtx::new(Language::Python, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("configuration_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), RUNTIME.into());
    ctx.insert(
        "runtime_path".into(),
        crate::std_imports::PYTHON_TEST_RUNTIME_PATH.into(),
    );
    ctx.insert("python_imports".into(), serde_json::json!(python_imports));
    ctx.insert("mtc_type".into(), literal(&rows.mtc.machine).into());
    ctx.insert("ptc_types".into(), serde_json::json!(ptcs));
    ctx.insert("ports_of_type".into(), serde_json::json!(ports_of_type));
    ctx.insert("timers_of_type".into(), serde_json::json!(timers_of_type));
    ctx.insert("connections".into(), serde_json::json!(connections));
    ctx.insert("mappings".into(), serde_json::json!(mappings));
    ctx.insert("machines".into(), serde_json::Value::Array(machines));
    // The whole list, so the template writes no bracket beside a value it
    // renders.
    ctx.insert(
        "module_parameters".into(),
        format!("[{}]", module_parameters.join(", ")).into(),
    );
    l.render(env, "test_configuration", ctx)
}

// ── §11.7: the test case ───────────────────────────────────────

/// The unit of the test case `m`: its name, and its configuration's tables,
/// read through the module of the configuration's unit.
pub(super) fn render_test_case_python(
    env: &minijinja::Environment,
    m: &TestCaseModel,
    imports: &[ImportContext],
    options: &crate::ForgeCompileOptions,
) -> Result<String, ForgeError> {
    let configuration = test_case_configuration(m, imports)?;
    let (statement, alias) = sibling(&configuration.document_name, options);
    let l = LangCtx::new(Language::Python, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("case_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), RUNTIME.into());
    ctx.insert(
        "runtime_path".into(),
        crate::std_imports::PYTHON_TEST_RUNTIME_PATH.into(),
    );
    ctx.insert("python_imports".into(), serde_json::json!([statement]));
    ctx.insert("name_literal".into(), literal(&m.name).into());
    // The three declarations of the configuration's unit the entry reads.
    ctx.insert(
        "configuration".into(),
        format!("{alias}.configuration").into(),
    );
    ctx.insert("machine".into(), format!("{alias}.machine").into());
    ctx.insert(
        "module_parameters".into(),
        format!("{alias}.module_parameters").into(),
    );
    l.render(env, "test_case", ctx)
}

// ── §11.8: the suite ───────────────────────────────────────────

/// The unit of the test suite `m`: its module parameters at their defaults,
/// and its test cases in run order, each with its guard and its selection.
pub(super) fn render_test_suite_python(
    env: &minijinja::Environment,
    m: &TestSuiteModel,
    imports: &[ImportContext],
    options: &crate::ForgeCompileOptions,
) -> Result<String, ForgeError> {
    let parameters: Vec<serde_json::Value> = module_parameter_defaults(m, "Python", saved_default)?
        .into_iter()
        .map(|(id, value)| {
            serde_json::json!({
                "id": literal(id),
                "value": value,
            })
        })
        .collect();
    let mut python_imports = Vec::new();
    let mut executes = Vec::new();
    for execute in execute_rows(m, imports)? {
        let (statement, alias) = sibling(&execute.test_case.document_name, options);
        import_once(&mut python_imports, statement);
        // The runtime's clock counts microseconds in an `int`, and a test case
        // with no guard has `None`.
        let guard = match execute.guard {
            None => "None".to_string(),
            Some(us) => us.to_string(),
        };
        let selection = match &execute.selection {
            None => "None".to_string(),
            Some(expr) => format!(
                "lambda world: {}",
                selection_body_in(
                    expr,
                    &|feature: &str| format!("{RUNTIME}.enabled(world, {})", literal(feature)),
                    Connectives::PYTHON,
                )
            ),
        };
        executes.push(serde_json::json!({
            "test_case": format!("{alias}.test_case()"),
            "guard": guard,
            "selection": selection,
        }));
    }
    let l = LangCtx::new(Language::Python, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("suite_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), RUNTIME.into());
    ctx.insert(
        "runtime_path".into(),
        crate::std_imports::PYTHON_TEST_RUNTIME_PATH.into(),
    );
    ctx.insert("saved_values".into(), SAVED_VALUES.into());
    ctx.insert("saved_values_path".into(), SAVED_VALUES_PATH.into());
    ctx.insert("python_imports".into(), serde_json::json!(python_imports));
    ctx.insert("name_literal".into(), literal(&m.name).into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("executes".into(), serde_json::Value::Array(executes));
    l.render(env, "test_suite", ctx)
}

/// The default `value` of a module parameter of type `ty`, a literal of
/// docs/SCE_TEST_PROFILE.md §11.4, as the saved value the runtime holds it in
/// (`sce_runtime.testprofile.ModuleParameters`): the value a machine's saved
/// state writes a value of that type as, so the component reads it back by the
/// reader of its own type (`saved_read_<type>`). A number, a `bool`, a text and
/// a byte string are written by the writer of their type
/// (`saved_write_<type>`), which writes a 64-bit integer as its text and a
/// byte string as its Latin-1 text; a value of an enum is saved as the name its
/// document declares for the variant, written as a text is. `None` for a type a
/// suite declares no module parameter of, and for a literal that is none of
/// its type.
fn saved_default(ty: &ValueType, value: &str) -> Option<String> {
    let write =
        |writer: &str, written: String| format!("{SAVED_VALUES}.saved_write_{writer}({written})");
    match ty {
        TypeOf::Scalar(SceType::Bool) => match value.trim() {
            "true" => Some(write("bool", "True".to_string())),
            "false" => Some(write("bool", "False".to_string())),
            _ => None,
        },
        TypeOf::Scalar(SceType::Enum(_)) => Some(write("string", literal(value.trim()))),
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => Some(write(
            &scalar.as_attr(),
            integer_literal(Language::Python, value)?,
        )),
        TypeOf::Text { .. } => Some(write("string", literal(value))),
        TypeOf::Bytes { .. } => {
            let octets: String = hex_octets(value)?
                .into_iter()
                .map(|pair| format!("\\x{pair}"))
                .collect();
            Some(write("bytes", format!("b\"{octets}\"")))
        }
        TypeOf::Scalar(_)
        | TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => None,
    }
}
