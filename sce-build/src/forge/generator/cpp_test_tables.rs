// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ units of the tables of a test (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.8, §12.2): a test configuration's `configuration()` and `machine`, a
//! test case's `testCase()`, and a test suite's `suite()`, each the data the
//! profile's runtime and its runner read (`sce/include/test/TestCaseRun.h`,
//! `sce/include/test/Suite.h`).
//!
//! Each unit names only what its own imports are: a configuration the machines
//! of its components, a test case its configuration's unit, a suite its test
//! cases' units. What a unit needs of a document further down is reached
//! through the unit that imports it, so no unit reads past its own imports.
//! The checks of how the documents fit together have run before
//! (`crate::forge::test_profile_assembly`); what is refused here is only what
//! a well-formed set of documents cannot reach, by name.

use super::*;
use crate::forge::value_type::TypeOf;
use crate::generator::Language;
use crate::presence_condition::PresenceExpr;

/// `text` as a C++ string literal.
fn literal(text: &str) -> String {
    format!("\"{}\"", filters::escape_cpp(text.to_string()))
}

/// The refusal of the document `kind` `name`, at `at`, for `why`.
fn refused(
    kind: ForgeKind,
    name: &str,
    at: Option<&crate::forge::error::SourceLocation>,
    why: impl std::fmt::Display,
) -> ForgeError {
    GenerateError::unsupported_at(format!("{kind} '{name}': {why}"), at.cloned()).into()
}

/// The import of kind `kind` (its attribute spelling) the document names
/// `alias`.
fn import_of<'i>(
    imports: &'i [ImportContext],
    alias: &str,
    kind: &str,
) -> Option<&'i ImportContext> {
    imports.iter().find(|i| i.alias == alias && i.kind == kind)
}

// ── §11.6: the configuration ───────────────────────────────────

/// One component type of a configuration, as its unit names it.
struct ComponentType<'i> {
    /// The type's name in the runtime's tables: its machine's name.
    name: &'i str,
    /// The header of its machine.
    header: String,
    /// Its component, from the global namespace.
    component: String,
    ports: &'i [String],
    timers: &'i [String],
}

/// The unit of the test configuration `m`, whose imports are `imports`: the
/// configuration the runtime reads, the factory of the machines of its
/// component types and the module parameters those components read.
pub(super) fn render_test_configuration_cpp(
    env: &minijinja::Environment,
    m: &TestConfigurationModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let kind = ForgeKind::TestConfiguration;
    let at = m.source_location.as_ref();
    let component_of = |alias: &str| {
        import_of(imports, alias, ForgeKind::TestComponent.as_attr())
            .and_then(|i| i.test_component.as_ref())
            .ok_or_else(|| {
                refused(
                    kind,
                    &m.name,
                    at,
                    format!("'{alias}' names no test component the configuration imports"),
                )
            })
    };
    // The MTC's type and each PTC's, in document order; a type two components
    // share is one type of the tables.
    let mtc = component_of(&m.mtc)?;
    let mut components: Vec<&TestComponentImport> = vec![mtc];
    let mut ptcs: Vec<serde_json::Value> = Vec::new();
    for ptc in &m.ptcs {
        let component = component_of(&ptc.component)?;
        components.push(component);
        ptcs.push(serde_json::json!({
            "id": literal(&ptc.id),
            "component_type": literal(&component.machine),
        }));
    }
    let mut types: Vec<ComponentType<'_>> = Vec::new();
    let mut module_parameters: std::collections::BTreeSet<&str> = Default::default();
    for component in components {
        if types.iter().any(|ty| ty.name == component.machine) {
            continue;
        }
        module_parameters.extend(component.module_parameters.iter().map(String::as_str));
        types.push(ComponentType {
            name: &component.machine,
            header: format!("{}_sm.h", component.machine),
            component: format!("::SCE::Generated::{0}::{0}Component", component.machine),
            ports: &component.ports,
            timers: &component.timers,
        });
    }
    let endpoint = |component: &str, port: &str| {
        format!(
            "::SCE::Test::Endpoint{{{}, {}}}",
            literal(component),
            literal(port)
        )
    };
    let mut connections = Vec::new();
    for connect in &m.connects {
        let [a, b] = connect.endpoints.as_slice() else {
            return Err(refused(
                kind,
                &m.name,
                at,
                "a <sce:connect> joins exactly two endpoints",
            ));
        };
        connections.push(format!(
            "{{{}, {}}}",
            endpoint(&a.component, &a.port),
            endpoint(&b.component, &b.port)
        ));
    }
    let mappings: Vec<String> = m
        .maps
        .iter()
        .map(|map| {
            format!(
                "{{{}, {}}}",
                endpoint(&map.component, &map.port),
                literal(&map.system_port)
            )
        })
        .collect();
    let strings = |ids: &[String]| -> String {
        ids.iter()
            .map(|id| literal(id))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let types_json: Vec<serde_json::Value> = types
        .iter()
        .map(|ty| {
            serde_json::json!({
                "name": literal(ty.name),
                "component": ty.component,
                "ports": (!ty.ports.is_empty()).then(|| strings(ty.ports)),
                "timers": (!ty.timers.is_empty()).then(|| strings(ty.timers)),
            })
        })
        .collect();
    let mut headers: Vec<String> = types.iter().map(|ty| ty.header.clone()).collect();
    headers.sort();
    headers.dedup();

    let l = LangCtx::new(Language::Cpp, imports);
    let mut ctx = l.base_context(&m.name);
    ctx.insert("configuration_name".into(), m.name.clone().into());
    ctx.insert("mtc_type".into(), literal(&mtc.machine).into());
    ctx.insert("ptcs".into(), serde_json::Value::Array(ptcs));
    ctx.insert(
        "component_types".into(),
        serde_json::Value::Array(types_json),
    );
    ctx.insert("connections".into(), serde_json::json!(connections));
    ctx.insert("mappings".into(), serde_json::json!(mappings));
    ctx.insert(
        "module_parameters".into(),
        serde_json::json!(module_parameters
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
    let configuration = import_of(
        imports,
        &m.configuration,
        ForgeKind::TestConfiguration.as_attr(),
    )
    .ok_or_else(|| {
        refused(
            ForgeKind::TestCase,
            &m.name,
            m.source_location.as_ref(),
            format!(
                "'{}' names no test configuration the test case imports",
                m.configuration
            ),
        )
    })?;
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
    let kind = ForgeKind::TestSuite;
    let at = m.source_location.as_ref();
    let parameters: Vec<serde_json::Value> = m
        .module_parameters
        .iter()
        .map(|parameter| {
            let value =
                saved_default(&parameter.parameter_type, &parameter.value).ok_or_else(|| {
                    refused(
                        kind,
                        &m.name,
                        at,
                        format!(
                            "the module parameter '{}' of type {} has no C++ value for its \
                             default '{}'",
                            parameter.id, parameter.parameter_type, parameter.value
                        ),
                    )
                })?;
            Ok(serde_json::json!({
                "id": literal(&parameter.id),
                "value": value,
            }))
        })
        .collect::<Result<_, ForgeError>>()?;
    let mut includes: Vec<String> = Vec::new();
    let mut executes: Vec<serde_json::Value> = Vec::new();
    for execute in &m.executes {
        let case = import_of(imports, &execute.test_case, ForgeKind::TestCase.as_attr())
            .ok_or_else(|| {
                refused(
                    kind,
                    &m.name,
                    at,
                    format!(
                        "'{}' names no test case the suite imports",
                        execute.test_case
                    ),
                )
            })?;
        if !includes.contains(&case.include_stmt) {
            includes.push(case.include_stmt.clone());
        }
        let guard = match execute.guard_us {
            None => "::std::nullopt".to_string(),
            Some(us) => {
                let us = i64::try_from(us).map_err(|_| {
                    refused(
                        kind,
                        &m.name,
                        at,
                        format!(
                            "the guard of '{}' is {us} microseconds, past the {} the \
                             runtime's clock counts to",
                            execute.test_case,
                            i64::MAX
                        ),
                    )
                })?;
                format!("::SCE::Test::Duration{{{us}}}")
            }
        };
        let selection = match &execute.requires {
            None => "nullptr".to_string(),
            Some(condition) => {
                let expr = crate::presence_condition::parse(condition).map_err(|error| {
                    refused(
                        kind,
                        &m.name,
                        at,
                        format!("the selection '{condition}' does not read: {error}"),
                    )
                })?;
                format!(
                    "[](const ::SCE::Test::Features &world) {{ return {}; }}",
                    selection_expr(&expr)
                )
            }
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

/// A selection (ADR 0057's grammar) as the C++ expression that holds in the
/// worlds it selects: a feature is enabled in `world`, `all` is a conjunction
/// and `any` a disjunction.
fn selection_expr(expr: &PresenceExpr) -> String {
    let joined = |parts: &[PresenceExpr], op: &str| {
        let parts: Vec<String> = parts.iter().map(selection_expr).collect();
        match parts.len() {
            1 => parts.into_iter().next().unwrap_or_default(),
            _ => format!("({})", parts.join(op)),
        }
    };
    match expr {
        PresenceExpr::Feature { name, .. } => {
            format!("::SCE::Test::enabled(world, {})", literal(name))
        }
        PresenceExpr::All(parts) => joined(parts, " && "),
        PresenceExpr::Any(parts) => joined(parts, " || "),
    }
}

/// The default `value` of a module parameter of type `ty`, a literal of
/// docs/SCE_TEST_PROFILE.md §11.4, as the saved value the runtime holds it in
/// (`sce/include/test/ModuleParameters.h`): written by the writer of its type
/// (`sce/include/saved/SavedValues.h`), the one a machine's saved state writes a
/// value of that type with, so the component reads it back by its own type. A
/// value of an enum is saved as the name its document declares for the
/// variant. `None` for a type a suite declares no module parameter of.
fn saved_default(ty: &crate::forge::value_type::ValueType, value: &str) -> Option<String> {
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
                super::cpp_test_template::integer_literal(value)?
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
