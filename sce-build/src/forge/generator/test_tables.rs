// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the units of the tables of a test hold on every backend
//! (docs/SCE_TEST_PROFILE.md §11.6 to §11.8, §12.2): a test configuration's
//! component types, its PTCs, its connections and mappings and the module
//! parameters its components read; a test case's configuration; and a test
//! suite's module parameters at their defaults and its test cases in run
//! order, each with its guard and its selection.
//!
//! Which rows there are, in which order, and what is refused is decided once,
//! here; a backend spells the rows in its language (`cpp_test_tables`,
//! `rust_test_tables`), so only the spelling differs between backends.
//!
//! Each unit names only what its own imports are: a configuration the machines
//! of its components, a test case its configuration's unit, a suite its test
//! cases' units. What a unit needs of a document further down is reached
//! through the unit that imports it, so no unit reads past its own imports.
//! The checks of how the documents fit together have run before
//! (`crate::forge::test_profile_assembly`); what is refused here is only what
//! a well-formed set of documents cannot reach, by name.

use super::*;
use crate::forge::value_type::ValueType;
use crate::presence_condition::PresenceExpr;

/// The refusal of the document `kind` `name`, at `at`, for `why`.
pub(super) fn refused(
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

/// A component port a configuration names: the component's name (`mtc` or a
/// PTC's id) and the port's id.
#[derive(Debug, Clone, Copy)]
pub(super) struct EndpointRow<'a> {
    pub(super) component: &'a str,
    pub(super) port: &'a str,
}

/// The rows of the test configuration's tables, in the order its unit writes
/// them.
pub(super) struct ConfigurationRows<'a> {
    /// The MTC's component.
    pub(super) mtc: &'a TestComponentImport,
    /// Each PTC's id and its component, in document order.
    pub(super) ptcs: Vec<(&'a str, &'a TestComponentImport)>,
    /// Each component type once, the MTC's first and then each PTC's in
    /// document order: a type two components share is one type of the
    /// tables.
    pub(super) component_types: Vec<&'a TestComponentImport>,
    /// The ids of the module parameters the component types read, each once,
    /// in the order of their ids.
    pub(super) module_parameters: Vec<&'a str>,
    /// Each `<sce:connect>`, its two endpoints in document order.
    pub(super) connections: Vec<(EndpointRow<'a>, EndpointRow<'a>)>,
    /// Each `<sce:map>`: a component port and the system port it is mapped to.
    pub(super) mappings: Vec<(EndpointRow<'a>, &'a str)>,
}

/// The rows of the test configuration `m`, whose imports are `imports`.
///
/// Refused by name where the MTC or a PTC names no test component the
/// configuration imports, and where a `<sce:connect>` does not join exactly
/// two endpoints.
pub(super) fn configuration_rows<'a>(
    m: &'a TestConfigurationModel,
    imports: &'a [ImportContext],
) -> Result<ConfigurationRows<'a>, ForgeError> {
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
    let mtc = component_of(&m.mtc)?;
    let mut ptcs = Vec::new();
    for ptc in &m.ptcs {
        ptcs.push((ptc.id.as_str(), component_of(&ptc.component)?));
    }
    let mut component_types: Vec<&TestComponentImport> = Vec::new();
    let mut module_parameters: std::collections::BTreeSet<&str> = Default::default();
    for component in std::iter::once(mtc).chain(ptcs.iter().map(|(_, component)| *component)) {
        if component_types
            .iter()
            .any(|ty| ty.machine == component.machine)
        {
            continue;
        }
        module_parameters.extend(component.module_parameters.iter().map(String::as_str));
        component_types.push(component);
    }
    let endpoint = |component: &'a str, port: &'a str| EndpointRow { component, port };
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
        connections.push((
            endpoint(&a.component, &a.port),
            endpoint(&b.component, &b.port),
        ));
    }
    let mappings = m
        .maps
        .iter()
        .map(|map| {
            (
                endpoint(&map.component, &map.port),
                map.system_port.as_str(),
            )
        })
        .collect();
    Ok(ConfigurationRows {
        mtc,
        ptcs,
        component_types,
        module_parameters: module_parameters.into_iter().collect(),
        connections,
        mappings,
    })
}

// ── §11.7: the test case ───────────────────────────────────────

/// The import of the configuration the test case `m` runs in; refused by name
/// where its `sce:configuration` names no test configuration it imports.
pub(super) fn test_case_configuration<'i>(
    m: &TestCaseModel,
    imports: &'i [ImportContext],
) -> Result<&'i ImportContext, ForgeError> {
    import_of(
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
    })
}

// ── §11.8: the suite ───────────────────────────────────────────

/// Each module parameter of the suite `m`, its id and its default as the
/// language `language` writes it, `saved_default` of its type and its literal,
/// in document order.
///
/// Refused by name where `saved_default` has no value for one: a type a suite
/// declares no module parameter of, or a literal that is none of its type.
pub(super) fn module_parameter_defaults<'m>(
    m: &'m TestSuiteModel,
    language: &str,
    saved_default: impl Fn(&ValueType, &str) -> Option<String>,
) -> Result<Vec<(&'m str, String)>, ForgeError> {
    m.module_parameters
        .iter()
        .map(|parameter| -> Result<(&'m str, String), ForgeError> {
            let value =
                saved_default(&parameter.parameter_type, &parameter.value).ok_or_else(|| {
                    refused(
                        ForgeKind::TestSuite,
                        &m.name,
                        m.source_location.as_ref(),
                        format!(
                            "the module parameter '{}' of type {} has no {language} value for its \
                             default '{}'",
                            parameter.id, parameter.parameter_type, parameter.value
                        ),
                    )
                })?;
            Ok((parameter.id.as_str(), value))
        })
        .collect()
}

/// One `<sce:execute>` of a suite, as its unit writes it.
pub(super) struct ExecuteRow<'i> {
    /// The import of the test case it runs.
    pub(super) test_case: &'i ImportContext,
    /// Its guard, in microseconds of the runtime's clock, if it has one.
    pub(super) guard: Option<i64>,
    /// The selection that runs it in a world of the suite's features, read,
    /// or none for a test case every world runs.
    pub(super) selection: Option<PresenceExpr>,
}

/// Each `<sce:execute>` of the suite `m`, whose imports are `imports`, in run
/// order.
///
/// Refused by name where it names no test case the suite imports, where its
/// guard is past the microseconds the runtime's clock counts to, and where its
/// selection does not read.
pub(super) fn execute_rows<'i>(
    m: &TestSuiteModel,
    imports: &'i [ImportContext],
) -> Result<Vec<ExecuteRow<'i>>, ForgeError> {
    let kind = ForgeKind::TestSuite;
    let at = m.source_location.as_ref();
    m.executes
        .iter()
        .map(|execute| -> Result<ExecuteRow<'i>, ForgeError> {
            let test_case = import_of(imports, &execute.test_case, ForgeKind::TestCase.as_attr())
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
            let guard = execute
                .guard_us
                .map(|us| {
                    i64::try_from(us).map_err(|_| {
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
                    })
                })
                .transpose()?;
            let selection = execute
                .requires
                .as_ref()
                .map(|condition| {
                    crate::presence_condition::parse(condition).map_err(|error| {
                        refused(
                            kind,
                            &m.name,
                            at,
                            format!("the selection '{condition}' does not read: {error}"),
                        )
                    })
                })
                .transpose()?;
            Ok(ExecuteRow {
                test_case,
                guard,
                selection,
            })
        })
        .collect()
}

/// A selection (ADR 0057's grammar) as the expression that holds in the
/// worlds it selects, in a language whose conjunction is `&&` and whose
/// disjunction is `||`: `enabled` spells the test that a world enables the
/// feature it is given, `all` is a conjunction and `any` a disjunction, each
/// join of two or more parts in parentheses.
pub(super) fn selection_expr(expr: &PresenceExpr, enabled: &dyn Fn(&str) -> String) -> String {
    spelled(expr, enabled, true)
}

/// [`selection_expr`] with the outermost join left bare: what a language that
/// refuses parentheses around a whole closure body (Rust's `unused_parens`)
/// writes as one. Every join inside it keeps its parentheses.
pub(super) fn selection_body(expr: &PresenceExpr, enabled: &dyn Fn(&str) -> String) -> String {
    spelled(expr, enabled, false)
}

/// `expr` spelled, its join in parentheses where `grouped` holds; every part
/// of a join of two or more is grouped, and a join of one part is that part,
/// spelled as the join would be.
fn spelled(expr: &PresenceExpr, enabled: &dyn Fn(&str) -> String, grouped: bool) -> String {
    let joined = |parts: &[PresenceExpr], op: &str| {
        if let [only] = parts {
            return spelled(only, enabled, grouped);
        }
        let parts: Vec<String> = parts
            .iter()
            .map(|part| spelled(part, enabled, true))
            .collect();
        if grouped {
            format!("({})", parts.join(op))
        } else {
            parts.join(op)
        }
    };
    match expr {
        PresenceExpr::Feature { name, .. } => enabled(name),
        PresenceExpr::All(parts) => joined(parts, " && "),
        PresenceExpr::Any(parts) => joined(parts, " || "),
    }
}
