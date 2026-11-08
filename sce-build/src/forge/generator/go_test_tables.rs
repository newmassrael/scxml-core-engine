// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Go units of the tables of a test (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.8, §12.2): a test configuration's `Configuration`, `Machine` and
//! `ModuleParameters`, a test case's `TestCase`, and a test suite's `Suite`,
//! each the data the profile's runtime and its runner read (the package
//! `testprofile` of `sce-go-runtime`, `RunSuite`).
//!
//! Which rows each unit holds, and what is refused, is decided by the rows
//! every backend shares ([`super::test_tables`]); this file spells them in Go,
//! and the templates (`forge/go/test_{configuration,case,suite}.go.jinja2`)
//! write the declarations around them.
//!
//! A Go unit names another package only through an import path, which has no
//! bare form, so every package a unit reads is a sibling below the module path
//! the run is given (`--go-module-prefix`), imported under the alias every Go
//! unit imports a sibling by ([`go_package_alias`]): a test case its
//! configuration's unit, a suite its test cases' units, and a configuration
//! the package of each component's machine. A component's machine is a
//! statechart's, generated into the package `<snake>` of its machine's name
//! ([`component_package`]): the directory a forge unit of that name is placed
//! in below the same module ([`forge_unit_dir`]), so the configuration and its
//! components are generated under one module path, as every Go importer and
//! the units it imports are. The runtime is named by its own package name,
//! which no declaration of a unit (`Configuration`, `Machine`,
//! `ModuleParameters`, `Name`, `TestCase`, `Suite`) and no sibling's alias can
//! be.

use super::test_tables::{
    configuration_rows, execute_rows, module_parameter_defaults, selection_body,
    test_case_configuration, EndpointRow,
};
use super::test_template_walk::{hex_octets, integer_literal};
use super::*;
use crate::forge::value_type::{TypeOf, ValueType};
use crate::generator::Language;

/// The name the profile's Go runtime is bound to in a unit: its package's own
/// name, imported from [`crate::std_imports::GO_TEST_RUNTIME_PATH`].
const RUNTIME: &str = "testprofile";

/// The name the SCE Go runtime is bound to in a suite's unit, which writes
/// each module parameter's default through the writer of a saved value of its
/// type (`sce.SavedWrite<Type>`).
const SAVED_VALUES: &str = "sce";

/// `text` as a Go string literal.
fn literal(text: &str) -> String {
    format!("\"{}\"", filters::escape_go(text.to_string()))
}

/// The import spec and the alias of the sibling package of the document
/// `document`, below `options`' `go_module_prefix`, which is refused by name
/// where it is missing ([`go_import_root`]).
fn sibling(
    document: &str,
    options: &crate::ForgeCompileOptions,
) -> Result<(String, String), ForgeError> {
    go_import_root(options.go_module_prefix.as_deref())?;
    let identity = forge_import_identity(document, &Language::Go, false, options);
    Ok((identity.include_stmt.trim().to_string(), identity.namespace))
}

/// `spec` added to `specs` unless it is there already: a package two rows
/// name is imported once, as Go refuses a second import of one name.
fn import_once(specs: &mut Vec<String>, spec: String) {
    if !specs.contains(&spec) {
        specs.push(spec);
    }
}

/// The package the machine of `component` is generated into: the one its
/// statechart declares (`package <snake>`, `go/state_machine.go.jinja2`),
/// imported from the directory of that name below the run's module path,
/// where a unit of the same name is placed ([`forge_unit_dir`]).
fn component_package(
    component: &TestComponentImport,
    options: &crate::ForgeCompileOptions,
) -> Result<(String, String), ForgeError> {
    sibling(&component.machine, options)
}

/// The component the profile's runtime drives for `component`, generated
/// beside its machine (`<Machine>Component`, docs/SCE_TEST_PROFILE.md §12.3),
/// through the package's alias `alias`.
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
pub(super) fn render_test_configuration_go(
    env: &minijinja::Environment,
    m: &TestConfigurationModel,
    imports: &[ImportContext],
    options: &crate::ForgeCompileOptions,
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
        Some(format!("{}: {{{}}}", literal(machine), ids.join(", ")))
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
    let mut go_imports = Vec::new();
    let mut machines = Vec::new();
    for ty in &rows.component_types {
        let (spec, alias) = component_package(ty, options)?;
        import_once(&mut go_imports, spec);
        machines.push(serde_json::json!({
            "name": literal(&ty.machine),
            "component": component_type(ty, &alias),
        }));
    }
    let endpoint = |row: EndpointRow<'_>| {
        format!(
            "{RUNTIME}.Endpoint{{Component: {}, Port: {}}}",
            literal(row.component),
            literal(row.port)
        )
    };
    let connections: Vec<String> = rows
        .connections
        .iter()
        .map(|(a, b)| format!("{{A: {}, B: {}}}", endpoint(*a), endpoint(*b)))
        .collect();
    let mappings: Vec<String> = rows
        .mappings
        .iter()
        .map(|(port, system_port)| {
            format!(
                "{{Port: {}, SystemPort: {}}}",
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

    let l = LangCtx::new(Language::Go, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("configuration_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), RUNTIME.into());
    ctx.insert(
        "runtime_path".into(),
        crate::std_imports::GO_TEST_RUNTIME_PATH.into(),
    );
    ctx.insert("go_imports".into(), serde_json::json!(go_imports));
    ctx.insert("mtc_type".into(), literal(&rows.mtc.machine).into());
    ctx.insert("ptc_types".into(), serde_json::json!(ptcs));
    ctx.insert("ports_of_type".into(), serde_json::json!(ports_of_type));
    ctx.insert("timers_of_type".into(), serde_json::json!(timers_of_type));
    ctx.insert("connections".into(), serde_json::json!(connections));
    ctx.insert("mappings".into(), serde_json::json!(mappings));
    ctx.insert("machines".into(), serde_json::Value::Array(machines));
    // The whole slice, so the template writes no brace beside a value it
    // renders.
    ctx.insert(
        "module_parameters".into(),
        format!("[]string{{{}}}", module_parameters.join(", ")).into(),
    );
    l.render(env, "test_configuration", ctx)
}

// ── §11.7: the test case ───────────────────────────────────────

/// The unit of the test case `m`: its name, and its configuration's tables,
/// read through the package of the configuration's unit.
pub(super) fn render_test_case_go(
    env: &minijinja::Environment,
    m: &TestCaseModel,
    imports: &[ImportContext],
    options: &crate::ForgeCompileOptions,
) -> Result<String, ForgeError> {
    let configuration = test_case_configuration(m, imports)?;
    let (spec, alias) = sibling(&configuration.document_name, options)?;
    let l = LangCtx::new(Language::Go, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("case_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), RUNTIME.into());
    ctx.insert(
        "runtime_path".into(),
        crate::std_imports::GO_TEST_RUNTIME_PATH.into(),
    );
    ctx.insert("go_imports".into(), serde_json::json!([spec]));
    ctx.insert("name_literal".into(), literal(&m.name).into());
    // The three declarations of the configuration's unit the entry reads.
    ctx.insert(
        "configuration".into(),
        format!("{alias}.Configuration").into(),
    );
    ctx.insert("machine".into(), format!("{alias}.Machine").into());
    ctx.insert(
        "module_parameters".into(),
        format!("{alias}.ModuleParameters").into(),
    );
    l.render(env, "test_case", ctx)
}

// ── §11.8: the suite ───────────────────────────────────────────

/// The unit of the test suite `m`: its module parameters at their defaults,
/// and its test cases in run order, each with its guard and its selection.
pub(super) fn render_test_suite_go(
    env: &minijinja::Environment,
    m: &TestSuiteModel,
    imports: &[ImportContext],
    options: &crate::ForgeCompileOptions,
) -> Result<String, ForgeError> {
    let parameters: Vec<serde_json::Value> = module_parameter_defaults(m, "Go", saved_default)?
        .into_iter()
        .map(|(id, value)| {
            serde_json::json!({
                "id": literal(id),
                "value": value,
            })
        })
        .collect();
    let mut go_imports = Vec::new();
    let mut executes = Vec::new();
    for execute in execute_rows(m, imports)? {
        let (spec, alias) = sibling(&execute.test_case.document_name, options)?;
        import_once(&mut go_imports, spec);
        // The runtime's clock counts microseconds in an `int64`.
        let guard = match execute.guard {
            None => format!("{RUNTIME}.NoGuard"),
            Some(us) => format!("{RUNTIME}.GuardOf({us})"),
        };
        let selection = match &execute.selection {
            None => "nil".to_string(),
            Some(expr) => format!(
                "func(world {RUNTIME}.Features) bool {{ return {} }}",
                selection_body(expr, &|feature: &str| format!(
                    "{RUNTIME}.Enabled(world, {})",
                    literal(feature)
                ))
            ),
        };
        executes.push(serde_json::json!({
            "test_case": format!("{alias}.TestCase()"),
            "guard": guard,
            "selection": selection,
        }));
    }
    let l = LangCtx::new(Language::Go, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("suite_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), RUNTIME.into());
    ctx.insert(
        "runtime_path".into(),
        crate::std_imports::GO_TEST_RUNTIME_PATH.into(),
    );
    ctx.insert("saved_values".into(), SAVED_VALUES.into());
    ctx.insert("go_imports".into(), serde_json::json!(go_imports));
    ctx.insert("name_literal".into(), literal(&m.name).into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("executes".into(), serde_json::Value::Array(executes));
    l.render(env, "test_suite", ctx)
}

/// The default `value` of a module parameter of type `ty`, a literal of
/// docs/SCE_TEST_PROFILE.md §11.4, as the saved value the runtime holds it in
/// (`testprofile.ModuleParameters`): the value a machine's saved state writes
/// a value of that type as, so the component reads it back by the reader of
/// its own type (`sce.SavedRead<Type>`). A number, a `bool`, a text and a byte
/// string are written by the writer of their type (`sce.SavedWrite<Type>`),
/// which writes a 64-bit integer as its text and a byte string as its Latin-1
/// text; a value of an enum is saved as the name its document declares for the
/// variant, written as a text is. An integer is an untyped constant, which
/// takes the type of the writer's parameter. `None` for a type a suite
/// declares no module parameter of, and for a literal that is none of its
/// type.
fn saved_default(ty: &ValueType, value: &str) -> Option<String> {
    let write =
        |writer: &str, written: String| format!("{SAVED_VALUES}.SavedWrite{writer}({written})");
    match ty {
        TypeOf::Scalar(SceType::Bool) => match value.trim() {
            held @ ("true" | "false") => Some(write("Bool", held.to_string())),
            _ => None,
        },
        TypeOf::Scalar(SceType::Enum(_)) => Some(write("String", literal(value.trim()))),
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => Some(write(
            &filters::to_pascal_case(go_type(scalar).to_string()),
            integer_literal(Language::Go, value)?,
        )),
        TypeOf::Text { .. } => Some(write("String", literal(value))),
        TypeOf::Bytes { .. } => {
            let octets: Vec<String> = hex_octets(value)?
                .into_iter()
                .map(|pair| format!("0x{pair}"))
                .collect();
            Some(write("Bytes", format!("[]byte{{{}}}", octets.join(", "))))
        }
        TypeOf::Scalar(_)
        | TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => None,
    }
}
