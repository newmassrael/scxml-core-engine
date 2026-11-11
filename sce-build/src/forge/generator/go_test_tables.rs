// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Go units of the tables of a test (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.8, §12.2): a test configuration's `Configuration`, `Machine` and
//! `ModuleParameters`, a test case's `TestCase`, and a test suite's `Suite`,
//! each the data the profile's runtime and its runner read (the package
//! `testprofile` of `sce-go-runtime`, `RunSuite`). A configuration's unit also
//! reads the inputs of a recorded trace (`Input`, docs/SCE_TTCN3_LANE.md
//! §10.5), with a reader and a writer of each record its system ports take
//! ([`GoReading`], [`GoCanonical`]), and a suite's declares the entry of its
//! runner program, `Main`, and the reader of its module parameters' values,
//! `ModuleParameter` (§4, `testprofile.RunnerMain`).
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

use super::go_test_template::{
    go_literal as literal, GoCanonical, GoReading, GoSpelling, CANONICAL_PATH,
};
use super::test_tables::{
    configuration_rows, execute_rows, module_parameter_defaults, selection_body,
    test_case_configuration, trace_records, EndpointRow,
};
use super::test_template_walk::{
    canonical_form, canonical_value, hex_octets, integer_literal, CanonicalForm, CanonicalSpelling,
};
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
    let traced = trace_inputs(m, imports, options)?;
    for spec in traced.imports {
        import_once(&mut go_imports, spec);
    }

    let l = LangCtx::new(Language::Go, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert(
        "system_ports".into(),
        format!("[]string{{{}}}", traced.system_ports.join(", ")).into(),
    );
    ctx.insert("inputs".into(), serde_json::Value::Array(traced.inputs));
    ctx.insert("readers".into(), serde_json::Value::Array(traced.readers));
    ctx.insert("canonical_path".into(), CANONICAL_PATH.into());
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

/// What the unit of a configuration writes of the inputs a recorded trace may
/// carry ([`trace_inputs`]).
struct TraceInputs {
    /// Each system port's id, as a Go string literal.
    system_ports: Vec<String>,
    /// An arm of the unit's `Input` per port and record.
    inputs: Vec<serde_json::Value>,
    /// Each record's reader and writer, once.
    readers: Vec<serde_json::Value>,
    /// The import spec of each package the readers and writers name.
    imports: Vec<String>,
}

/// The name of the unit's function that reads, or writes, the record whose
/// type in the test log is `type_name`: `Read<Record>`, `Canonical<Record>`.
fn record_function(verb: &str, type_name: &str) -> String {
    format!("{verb}{}", filters::to_pascal_case(type_name.to_string()))
}

/// The inputs a recorded trace may carry on the system ports of the test
/// configuration `m` (docs/SCE_TTCN3_LANE.md §10.2, §10.5), as every backend
/// reads them ([`trace_records`]): each system port's id; an arm of the unit's
/// `Input` per port and record its type's `in` list names, in document order;
/// and each record's reader and writer of its canonical form, once, both over
/// the one walk of the form ([`canonical_form`]), so the value a trace hands
/// the run and the value the test log writes of it are the same bytes. Each
/// record is named by its type in the test log, which names its functions, and
/// the package of each document its reader and writer name is imported under
/// `options`' module path, the package of the record among them, whose
/// `EventName` its message carries.
fn trace_inputs(
    m: &TestConfigurationModel,
    imports: &[ImportContext],
    options: &crate::ForgeCompileOptions,
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
    let spelling = GoSpelling::default();
    let mut text = String::new();
    let mut readers = Vec::new();
    for record in &traced.records {
        let row = &record.row;
        let payload = spelling.type_of(&row.record_type, row.scope);
        let reading = GoReading {
            spelling: &spelling,
            record_type: &row.record_type,
            scope: row.scope,
            what: record.type_name.clone(),
        };
        let read_expr = canonical_form(&reading, &row.record_type, &reading.root(), row.scope)
            .map_err(refused)?;
        let canonical_expr = canonical_value(
            &GoCanonical,
            &row.record_type,
            &GoCanonical.root(),
            row.scope,
        )
        .map_err(refused)?;
        for spelled in [&payload, &read_expr, &canonical_expr] {
            text.push_str(spelled);
            text.push('\n');
        }
        readers.push(serde_json::json!({
            "type": record.type_name,
            "read": record_function("Read", &record.type_name),
            "canonical": record_function("Canonical", &record.type_name),
            "payload": payload,
            "read_expr": read_expr,
            "canonical_expr": canonical_expr,
        }));
    }
    if let Some(why) = spelling.take_refusal() {
        return Err(refused(why));
    }
    let inputs: Vec<serde_json::Value> = traced
        .arms
        .iter()
        .map(|arm| {
            let event = format!(
                "{}.EventName",
                go_package_alias(&filters::to_snake_case(arm.record.name.clone()))
            );
            text.push_str(&event);
            text.push('\n');
            serde_json::json!({
                "port": literal(arm.port),
                "type": literal(&arm.type_name),
                "event": event,
                "read": record_function("Read", &arm.type_name),
                "canonical": record_function("Canonical", &arm.type_name),
            })
        })
        .collect();
    Ok(TraceInputs {
        system_ports: traced.system_ports.iter().map(|id| literal(id)).collect(),
        inputs,
        readers,
        imports: spelling.document_imports(&text, options)?,
    })
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
    ctx.insert("inputs".into(), format!("{alias}.Input").into());
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
        .zip(&m.module_parameters)
        .map(|((id, value), parameter)| {
            let reader = parameter_reader(&parameter.parameter_type).ok_or_else(|| {
                super::test_tables::refused(
                    ForgeKind::TestSuite,
                    &m.name,
                    m.source_location.as_ref(),
                    format!(
                        "the module parameter '{id}' of type {} has no Go reader of its value",
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

/// The writer of the saved value of the integer type `scalar`
/// (`sce.SavedWrite<Type>`, named for its Go type): the one a machine's saved
/// state writes a value of that type with.
fn saved_writer(scalar: &SceType) -> String {
    format!(
        "{SAVED_VALUES}.SavedWrite{}",
        filters::to_pascal_case(go_type(scalar).to_string())
    )
}

/// The runner's reader of a value of a module parameter of type `ty`, given on
/// its command line as a literal of docs/SCE_TEST_PROFILE.md §11.4
/// (docs/SCE_TTCN3_LANE.md §4): an expression of the saved value of `ty` the
/// literal `literal` is, for the module parameter `id`, and the refusal that
/// names why where it is none, by the readers of the profile's runtime
/// (`testprofile.IntegerParameter` and its siblings). An integer is read at its
/// width, which the writer of its saved value names. `None` for the types
/// [`saved_default`] has no value of, of which no suite declares a module
/// parameter.
fn parameter_reader(ty: &ValueType) -> Option<String> {
    match ty {
        TypeOf::Scalar(SceType::Bool) => Some(format!("{RUNTIME}.BoolParameter(id, literal)")),
        TypeOf::Scalar(SceType::Enum(_)) => Some(format!("{RUNTIME}.EnumParameter(literal)")),
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => Some(format!(
            "{RUNTIME}.IntegerParameter(id, literal, {})",
            saved_writer(scalar)
        )),
        TypeOf::Text { .. } => Some(format!("{RUNTIME}.TextParameter(literal)")),
        TypeOf::Bytes { .. } => Some(format!("{RUNTIME}.BytesParameter(id, literal)")),
        TypeOf::Scalar(_)
        | TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => None,
    }
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
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => Some(format!(
            "{}({})",
            saved_writer(scalar),
            integer_literal(Language::Go, value)?
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
