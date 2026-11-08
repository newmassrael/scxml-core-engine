// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift units of the tables of a test (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.8, §12.2): a test configuration's `configuration`, `machine` and
//! `moduleParameters`, a test case's `name` and `testCase`, and a test suite's
//! `suite`, each the data the profile's runtime and its runner read
//! (`SceTestProfile`, `runSuite`).
//!
//! Which rows each unit holds, and what is refused, is decided by the rows
//! every backend shares ([`super::test_tables`]); this file spells them in
//! Swift, and the templates
//! (`forge/swift/test_{configuration,case,suite}.swift.jinja2`) write the
//! declarations around them.
//!
//! What is Swift's own: every unit is a file of one module, as the units of the
//! other Swift kinds are, so each declares its members in a caseless enum
//! named by its document, and names what it reads bare, as it is declared in
//! that module: a configuration each component's class
//! (`<Machine>Component`, beside the component's machine) and a test case and
//! a suite the enum of each unit they read. Every type and function of the
//! profile's runtime is named by its module, so no unit's enum (a document
//! named `test_case` declares `TestCase`) hides one of the runtime's.

use super::swift_test_template::{swift_bytes_literal, swift_text_literal};
use super::test_tables::{
    configuration_rows, execute_rows, module_parameter_defaults, selection_body,
    test_case_configuration, EndpointRow,
};
use super::test_template_walk::integer_literal;
use super::*;
use crate::forge::value_type::{TypeOf, ValueType};
use crate::generator::Language;
use crate::std_imports::SWIFT_TEST_RUNTIME_MODULE;

/// The module of the Swift runtime, whose `SavedValues` writes a module
/// parameter's default as the saved value of its type.
const SWIFT_RUNTIME_MODULE: &str = "SceRuntime";

/// The enum the unit of the document named `document_name` declares its
/// members in, as every Swift unit is named (`generate_swift_with_imports`).
fn unit_enum(document_name: &str) -> String {
    filters::to_pascal_case(document_name.to_string())
}

/// The component the profile's runtime drives for `component`, generated
/// beside its machine (`<Machine>Component`, docs/SCE_TEST_PROFILE.md §12.3),
/// whose machine is named by the statechart's name as every Swift machine is
/// (`render_swift`).
fn component_class(component: &TestComponentImport) -> String {
    format!(
        "{}Component",
        filters::to_pascal_case(component.machine.clone())
    )
}

// ── §11.6: the configuration ───────────────────────────────────

/// The unit of the test configuration `m`, whose imports are `imports`: the
/// configuration the runtime reads, the factory of the machines of its
/// component types and the module parameters those components read.
pub(super) fn render_test_configuration_swift(
    env: &minijinja::Environment,
    m: &TestConfigurationModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let rows = configuration_rows(m, imports)?;
    let literal = swift_text_literal;
    // A PTC's id is one key of the table, as each component type is one key
    // of the ports and of the timers (`configuration_rows`): a dictionary
    // literal with a key written twice stops the program where it is built.
    let ptc_types: Vec<String> = rows
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
            "{SWIFT_TEST_RUNTIME_MODULE}.Endpoint(component: {}, port: {})",
            literal(row.component),
            literal(row.port)
        )
    };
    let connections: Vec<String> = rows
        .connections
        .iter()
        .map(|(a, b)| {
            format!(
                "{SWIFT_TEST_RUNTIME_MODULE}.Connection({}, {})",
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
                "{SWIFT_TEST_RUNTIME_MODULE}.Mapping({}, systemPort: {})",
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

    let l = LangCtx::new(Language::Swift, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("configuration_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), SWIFT_TEST_RUNTIME_MODULE.into());
    ctx.insert("mtc_type".into(), literal(&rows.mtc.machine).into());
    ctx.insert("ptc_types".into(), serde_json::json!(ptc_types));
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

// ── §11.7: the test case ───────────────────────────────────────

/// The unit of the test case `m`: its name, and its configuration's tables,
/// reached through the enum of the configuration's unit.
pub(super) fn render_test_case_swift(
    env: &minijinja::Environment,
    m: &TestCaseModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let configuration = test_case_configuration(m, imports)?;
    let l = LangCtx::new(Language::Swift, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("case_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), SWIFT_TEST_RUNTIME_MODULE.into());
    ctx.insert("name_literal".into(), swift_text_literal(&m.name).into());
    ctx.insert(
        "configuration".into(),
        unit_enum(&configuration.document_name).into(),
    );
    l.render(env, "test_case", ctx)
}

// ── §11.8: the suite ───────────────────────────────────────────

/// The unit of the test suite `m`: its module parameters at their defaults,
/// and its test cases in run order, each with its guard and its selection.
pub(super) fn render_test_suite_swift(
    env: &minijinja::Environment,
    m: &TestSuiteModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let parameters: Vec<serde_json::Value> = module_parameter_defaults(m, "Swift", saved_default)?
        .into_iter()
        .map(|(id, value)| {
            serde_json::json!({
                "id": swift_text_literal(id),
                "value": value,
            })
        })
        .collect();
    let executes: Vec<serde_json::Value> = execute_rows(m, imports)?
        .into_iter()
        .map(|execute| {
            // The runtime's clock counts microseconds in an `Int64`
            // (`TestDuration`), which an integer literal is read as.
            let guard = match execute.guard {
                None => "nil".to_string(),
                Some(us) => us.to_string(),
            };
            let selection = match &execute.selection {
                None => "nil".to_string(),
                Some(expr) => format!(
                    "{{ world in {} }}",
                    selection_body(expr, &|feature: &str| format!(
                        "{SWIFT_TEST_RUNTIME_MODULE}.enabled(world, {})",
                        swift_text_literal(feature)
                    ))
                ),
            };
            let test_case = format!("{}.testCase()", unit_enum(&execute.test_case.document_name));
            serde_json::json!({
                "test_case": test_case,
                "guard": guard,
                "selection": selection,
            })
        })
        .collect();
    let l = LangCtx::new(Language::Swift, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("suite_name".into(), m.name.clone().into());
    ctx.insert("runtime".into(), SWIFT_TEST_RUNTIME_MODULE.into());
    ctx.insert("saved_values_module".into(), SWIFT_RUNTIME_MODULE.into());
    ctx.insert("name_literal".into(), swift_text_literal(&m.name).into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("executes".into(), serde_json::Value::Array(executes));
    l.render(env, "test_suite", ctx)
}

/// The default `value` of a module parameter of type `ty`, a literal of
/// docs/SCE_TEST_PROFILE.md §11.4, as the saved value the runtime holds it in
/// (`SceTestProfile.ModuleParameters`): the value a machine's saved state
/// writes a value of that type as, so the component reads it back by its own
/// type's reader (`SavedValues`). A number, a `bool`, a text and a byte string
/// are written by `SavedValues.of` of a literal of their own type, the literal
/// of a number and of a byte string coerced to it so the overload of that type
/// is the one called, which writes a 64-bit integer as its text and a byte
/// string as its Latin-1 text; a value of an enum is saved as the name its
/// document declares for the variant, a text. `None` for a type a suite
/// declares no module parameter of, and for a literal that is none of its
/// type, a number past its type's range among them, which Swift would refuse
/// to compile.
fn saved_default(ty: &ValueType, value: &str) -> Option<String> {
    let written = match ty {
        TypeOf::Scalar(SceType::Bool) => match value.trim() {
            held @ ("true" | "false") => held.to_string(),
            _ => return None,
        },
        TypeOf::Scalar(SceType::Enum(_)) => swift_text_literal(value.trim()),
        TypeOf::Scalar(scalar) => {
            let (least, most) = scalar.int_value_range()?;
            let n = crate::forge::test_profile::decimal_integer(value)?;
            if n < least || n > most {
                return None;
            }
            format!(
                "{} as {}",
                integer_literal(Language::Swift, value)?,
                swift_type(scalar)
            )
        }
        TypeOf::Text { .. } => swift_text_literal(value),
        TypeOf::Bytes { .. } => format!("{} as [UInt8]", swift_bytes_literal(value)?),
        TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => return None,
    };
    Some(format!("{SWIFT_RUNTIME_MODULE}.SavedValues.of({written})"))
}
