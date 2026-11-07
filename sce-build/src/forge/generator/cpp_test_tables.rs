// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ units of the tables of a test (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.8, §12.2): a test configuration's `configuration()` and `machine`, a
//! test case's `testCase()`, and a test suite's `suite()`, each the data the
//! profile's runtime and its runner read (`sce/include/test/TestCaseRun.h`,
//! `sce/include/test/Suite.h`).
//!
//! Which rows each unit holds, and what is refused, is decided by the rows
//! every backend shares ([`super::test_tables`]); this file spells them in
//! C++, and the templates (`forge/cpp/test_{configuration,case,suite}.h.jinja2`)
//! write the declarations around them.

use super::test_tables::{
    configuration_rows, execute_rows, module_parameter_defaults, selection_expr,
    test_case_configuration, EndpointRow,
};
use super::*;
use crate::forge::value_type::{TypeOf, ValueType};
use crate::generator::Language;

/// `text` as a C++ string literal.
fn literal(text: &str) -> String {
    format!("\"{}\"", filters::escape_cpp(text.to_string()))
}

// ── §11.6: the configuration ───────────────────────────────────

/// The unit of the test configuration `m`, whose imports are `imports`: the
/// configuration the runtime reads, the factory of the machines of its
/// component types and the module parameters those components read.
pub(super) fn render_test_configuration_cpp(
    env: &minijinja::Environment,
    m: &TestConfigurationModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let rows = configuration_rows(m, imports)?;
    let ptcs: Vec<serde_json::Value> = rows
        .ptcs
        .iter()
        .map(|(id, component)| {
            serde_json::json!({
                "id": literal(id),
                "component_type": literal(&component.machine),
            })
        })
        .collect();
    let endpoint = |row: EndpointRow<'_>| {
        format!(
            "::SCE::Test::Endpoint{{{}, {}}}",
            literal(row.component),
            literal(row.port)
        )
    };
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
    let strings = |ids: &[String]| -> String {
        ids.iter()
            .map(|id| literal(id))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let types_json: Vec<serde_json::Value> = rows
        .component_types
        .iter()
        .map(|ty| {
            serde_json::json!({
                "name": literal(&ty.machine),
                "component": format!("::SCE::Generated::{0}::{0}Component", ty.machine),
                "ports": (!ty.ports.is_empty()).then(|| strings(&ty.ports)),
                "timers": (!ty.timers.is_empty()).then(|| strings(&ty.timers)),
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

    let l = LangCtx::new(Language::Cpp, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("configuration_name".into(), m.name.clone().into());
    ctx.insert("mtc_type".into(), literal(&rows.mtc.machine).into());
    ctx.insert("ptcs".into(), serde_json::Value::Array(ptcs));
    ctx.insert(
        "component_types".into(),
        serde_json::Value::Array(types_json),
    );
    ctx.insert("connections".into(), serde_json::json!(connections));
    ctx.insert("mappings".into(), serde_json::json!(mappings));
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

// ── §11.7: the test case ───────────────────────────────────────

/// The unit of the test case `m`: its name, and its configuration's tables.
pub(super) fn render_test_case_cpp(
    env: &minijinja::Environment,
    m: &TestCaseModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let configuration = test_case_configuration(m, imports)?;
    let l = LangCtx::new(Language::Cpp, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("case_name".into(), m.name.clone().into());
    ctx.insert("name_literal".into(), literal(&m.name).into());
    ctx.insert(
        "configuration_include".into(),
        configuration.include_stmt.clone().into(),
    );
    ctx.insert(
        "configuration".into(),
        format!("::{}", configuration.namespace).into(),
    );
    l.render(env, "test_case", ctx)
}

// ── §11.8: the suite ───────────────────────────────────────────

/// The unit of the test suite `m`: its module parameters at their defaults,
/// and its test cases in run order, each with its guard and its selection.
pub(super) fn render_test_suite_cpp(
    env: &minijinja::Environment,
    m: &TestSuiteModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let parameters: Vec<serde_json::Value> = module_parameter_defaults(m, "C++", saved_default)?
        .into_iter()
        .map(|(id, value)| {
            serde_json::json!({
                "id": literal(id),
                "value": value,
            })
        })
        .collect();
    let mut includes: Vec<String> = Vec::new();
    let mut executes: Vec<serde_json::Value> = Vec::new();
    for execute in execute_rows(m, imports)? {
        let case = execute.test_case;
        if !includes.contains(&case.include_stmt) {
            includes.push(case.include_stmt.clone());
        }
        let guard = match execute.guard {
            None => "::std::nullopt".to_string(),
            Some(us) => format!("::SCE::Test::Duration{{{us}}}"),
        };
        let selection = match &execute.selection {
            None => "nullptr".to_string(),
            Some(expr) => format!(
                "[](const ::SCE::Test::Features &world) {{ return {}; }}",
                selection_expr(expr, &|feature: &str| format!(
                    "::SCE::Test::enabled(world, {})",
                    literal(feature)
                ))
            ),
        };
        executes.push(serde_json::json!({
            "test_case": format!("::{}::testCase()", case.namespace),
            "guard": guard,
            "selection": selection,
        }));
    }
    let l = LangCtx::new(Language::Cpp, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("suite_name".into(), m.name.clone().into());
    ctx.insert("name_literal".into(), literal(&m.name).into());
    ctx.insert("parameters".into(), serde_json::Value::Array(parameters));
    ctx.insert("executes".into(), serde_json::Value::Array(executes));
    ctx.insert("case_includes".into(), serde_json::json!(includes));
    l.render(env, "test_suite", ctx)
}

/// The default `value` of a module parameter of type `ty`, a literal of
/// docs/SCE_TEST_PROFILE.md §11.4, as the saved value the runtime holds it in
/// (`sce/include/test/ModuleParameters.h`): written by the writer of its type
/// (`sce/include/saved/SavedValues.h`), the one a machine's saved state writes a
/// value of that type with, so the component reads it back by its own type. A
/// value of an enum is saved as the name its document declares for the
/// variant. `None` for a type a suite declares no module parameter of.
fn saved_default(ty: &ValueType, value: &str) -> Option<String> {
    match ty {
        TypeOf::Scalar(SceType::Bool) => match value.trim() {
            held @ ("true" | "false") => Some(format!("::SCE::Saved::savedWriteBool({held})")),
            _ => None,
        },
        TypeOf::Scalar(SceType::Enum(_)) => Some(format!(
            "::SCE::Saved::JsonValue({})",
            literal(value.trim())
        )),
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
            // The writer is named for the `sce:type` (`savedWriteUint32`).
            let attr = scalar.as_attr();
            let (first, rest) = attr.split_at(1);
            Some(format!(
                "::SCE::Saved::savedWrite{}{rest}({}{{{}}})",
                first.to_uppercase(),
                cpp_type(scalar),
                super::cpp_test_template::cpp_integer_literal(value)?
            ))
        }
        TypeOf::Text { .. } => Some(format!(
            "::SCE::Saved::savedWriteString({})",
            filters::cpp_text_expression(value)
        )),
        TypeOf::Bytes { .. } => {
            let octets: Vec<String> = value
                .trim()
                .as_bytes()
                .chunks(2)
                .map(|pair| format!("0x{}", String::from_utf8_lossy(pair)))
                .collect();
            Some(format!(
                "::SCE::Saved::savedWriteBytes(::std::vector<::std::uint8_t>{{{}}})",
                octets.join(", ")
            ))
        }
        TypeOf::Scalar(_)
        | TypeOf::Optional { .. }
        | TypeOf::Record { .. }
        | TypeOf::Union { .. }
        | TypeOf::List { .. } => None,
    }
}
