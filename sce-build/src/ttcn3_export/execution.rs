// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a suite runs, as the definitions of the behaviour module that
//! complete it (docs/SCE_TEST_PROFILE.md §9.2 E14, E15, §9.5).
//!
//! - **E14.** Each test case the suite executes is
//!   `testcase t() runs on M system C { … }`: it runs on the component type
//!   of its configuration's MTC, its `system` is the configuration's system
//!   component type, and its body maps each MTC port the configuration maps,
//!   in document order, then calls the MTC's function (-1 §26.1). Each
//!   configuration a test case runs in is that system component type,
//!   `type component C { port P p; … }`, its system ports in document order
//!   (-1 §6.2.10), named by the configuration's unit name and written once
//!   however many test cases run in it. A map or a connect that names a PTC
//!   is applied where the PTC is created (E11), not here.
//! - **E15.** Each feature of the suite's feature model is a module
//!   parameter `modulepar boolean PICS_<NAME>;` with no default (-1 §8.2.1):
//!   which features a run enables is its caller's to say, as the world is a
//!   native runner's input. The control part executes each test case in the
//!   suite's order, `execute(t(), g)` with its guard `g` in seconds, inside
//!   `if (…) { … }` where the suite selects it by features, the selection
//!   written over those module parameters (-1 §26.1, §26.2; ADR 0091 §2).
//!
//! Every view carries the construct it was translated from and its rule
//! ([`Source`]), as the views of E4 to E13 do.

use std::path::Path;

use serde::Serialize;

use super::behaviour::{
    configurations, function_name, seconds, ComponentTypeView, Configuration, ConnectionView,
    ModuleParameterView, PortView, Source,
};
use super::names::{Declared, Namespace, Origin, Scope};
use super::reach::{Reach, Reached};
use super::types::{Site, TypeExpr};
use crate::feature_model::{upper_spelling, FeatureModel};
use crate::forge::error::{ForgeError, Located};
use crate::forge::model::{
    ForgeDocument, ForgeImport, ForgeKind, TestConfigurationModel, TestExecute, TestSuiteModel,
};
use crate::presence_condition::PresenceCondition;

/// The name of the module parameter of the feature `feature` (E15):
/// `PICS_<NAME>`, `<NAME>` the feature's name upper-cased with each hyphen an
/// underscore, the spelling ADR 0057 §2 and §6 give a feature on every backend
/// that writes it in upper case. A feature's name has no upper-case letter and
/// no underscore, so two features never share one.
pub fn feature_parameter(feature: &str) -> String {
    format!("PICS_{}", upper_spelling(feature))
}

/// A test case (E14).
#[derive(Debug, Clone, Serialize)]
pub struct TestCaseView {
    /// The test case's unit name.
    pub name: String,
    /// The component type of the configuration's MTC, which it runs on.
    pub component: String,
    /// The configuration's system component type, its `system`.
    pub system: String,
    /// Each `map` of the configuration whose component is the MTC, in
    /// document order.
    pub maps: Vec<ConnectionView>,
    /// The MTC's function in the configuration, which it calls.
    pub function: String,
    pub source: Source,
}

/// One `execute` of the control part (E15).
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteView {
    /// The test case it runs, by its unit name.
    pub test_case: String,
    /// The guard, the exact decimal of its seconds, where it has one.
    pub guard: Option<String>,
    /// The selection, as the condition over the features' module parameters
    /// that holds in the worlds it selects, where it has one.
    pub condition: Option<String>,
    pub source: Source,
}

/// The control part (E15): each test case the suite executes, in its order.
#[derive(Debug, Clone, Serialize)]
pub struct ControlView {
    pub executes: Vec<ExecuteView>,
    pub source: Source,
}

/// What the suite runs: the module parameters of its features, the system
/// component types of its configurations in the order of their names, its
/// test cases and its control part, both in its order.
#[derive(Debug, Clone, Serialize)]
pub struct Execution {
    pub features: Vec<ModuleParameterView>,
    pub system_types: Vec<ComponentTypeView>,
    pub test_cases: Vec<TestCaseView>,
    pub control: ControlView,
}

/// One `<sce:execute>`, the test case it runs and that test case's
/// configuration.
struct Executed<'a> {
    test_case: &'a str,
    guard_us: Option<u64>,
    requires: Option<&'a str>,
    line: Option<u32>,
    case: &'a Reached,
    configuration: &'a Configuration<'a>,
}

/// What `suite`, whose imports are `imports`, read in `dir` and at `at`,
/// runs, `features` its feature model where it imports one, every name
/// written entered in `names`.
pub(super) fn execution(
    suite: &TestSuiteModel,
    imports: &[ForgeImport],
    dir: &Path,
    at: &Origin,
    features: Option<&Reached>,
    reach: &Reach,
    names: &mut Namespace,
) -> Result<Execution, Located<ForgeError>> {
    let site = |node: &str, line: Option<u32>| Site {
        origin: at.at(line),
        subject: format!("test-suite '{}': {node}", suite.name),
    };
    let model = features.and_then(|document| match &document.parsed.document {
        ForgeDocument::FeatureModel(model) => Some((document, model)),
        _ => None,
    });

    let mut parameters = Vec::new();
    if let Some((document, model)) = model {
        let mut declared: Vec<&str> = model.features.iter().map(|f| f.name.as_str()).collect();
        declared.sort_unstable();
        for feature in declared {
            let name = feature_parameter(feature);
            names.definition(
                &name,
                Declared::made("the feature's module parameter", document.origin()),
            )?;
            parameters.push(ModuleParameterView {
                name,
                ty: TypeExpr::Boolean,
                value: None,
                source: Source {
                    rule: "E15",
                    document: document.label.clone(),
                    node: format!("sce:feature '{feature}'"),
                    line: None,
                },
            });
        }
    }

    let configurations = configurations(reach)?;
    let mut executed = Vec::new();
    for execute in &suite.executes {
        let case = executed_case(suite, execute, imports, dir, at, reach)?;
        let configuration = run_in(case, reach, &configurations)?;
        executed.push(Executed {
            test_case: &execute.test_case,
            guard_us: execute.guard_us,
            requires: execute.requires.as_deref(),
            line: execute.source_line,
            case,
            configuration,
        });
    }

    let mut system_types = Vec::new();
    for configuration in configurations.iter().filter(|c| {
        executed
            .iter()
            .any(|e| e.configuration.document.identity == c.document.identity)
    }) {
        system_types.push(system_type(configuration, reach, names)?);
    }

    let mut test_cases = Vec::new();
    for e in &executed {
        names.definition(
            &e.case.name,
            Declared::read("the test case", e.case.origin()),
        )?;
        let configuration = e.configuration;
        let maps = configuration
            .model
            .maps
            .iter()
            .filter(|m| m.component == TestConfigurationModel::MTC)
            .map(|m| ConnectionView {
                operation: "map",
                left: "mtc".to_string(),
                left_port: m.port.clone(),
                right: "system".to_string(),
                right_port: m.system_port.clone(),
                when: None,
            })
            .collect();
        test_cases.push(TestCaseView {
            name: e.case.name.clone(),
            component: configuration.mtc.name.clone(),
            system: configuration.name.clone(),
            maps,
            function: function_name(configuration.mtc, configuration),
            source: e.case.source("E14"),
        });
    }

    let resolved = model.map(|(_, model)| (model.name.as_str(), model.feature_model()));
    let mut executes = Vec::new();
    for e in &executed {
        let node = format!("sce:execute '{}'", e.test_case);
        let condition = e
            .requires
            .map(|text| {
                selection(
                    text,
                    resolved.as_ref().map(|(name, model)| (*name, model)),
                    &site(&node, e.line),
                )
            })
            .transpose()?;
        executes.push(ExecuteView {
            test_case: e.case.name.clone(),
            guard: e.guard_us.map(seconds),
            condition,
            source: Source {
                rule: "E15",
                document: at.label.clone(),
                node,
                line: e.line,
            },
        });
    }
    Ok(Execution {
        features: parameters,
        system_types,
        test_cases,
        control: ControlView {
            executes,
            source: Source {
                rule: "E15",
                document: at.label.clone(),
                node: "scxml".to_string(),
                line: at.line,
            },
        },
    })
}

/// The test case the `<sce:execute>` `execute` of `suite` runs: the document
/// the suite, whose imports are `imports` read in `dir` and which is refused
/// at `at`, imports as a test case under the alias it names, among the
/// documents `reach` holds.
pub(super) fn executed_case<'r>(
    suite: &TestSuiteModel,
    execute: &TestExecute,
    imports: &[ForgeImport],
    dir: &Path,
    at: &Origin,
    reach: &'r Reach,
) -> Result<&'r Reached, Located<ForgeError>> {
    imports
        .iter()
        .find(|i| i.alias == execute.test_case && i.kind == ForgeKind::TestCase)
        .and_then(|import| reach.imported(dir, import))
        .ok_or_else(|| {
            Site {
                origin: at.at(execute.source_line),
                subject: format!(
                    "test-suite '{}': sce:execute '{}'",
                    suite.name, execute.test_case
                ),
            }
            .refuse(format!(
                "'{}' names no test case the suite reaches",
                execute.test_case
            ))
        })
}

/// The configuration the test case `case` runs in, among `configurations`.
pub(super) fn run_in<'c, 'r>(
    case: &Reached,
    reach: &Reach,
    configurations: &'c [Configuration<'r>],
) -> Result<&'c Configuration<'r>, Located<ForgeError>> {
    let ForgeDocument::TestCase(model) = &case.parsed.document else {
        return Err(Site {
            origin: case.origin(),
            subject: format!(
                "{} '{}'",
                case.parsed.document.kind(),
                case.parsed.document.name()
            ),
        }
        .refuse("it is no test case"));
    };
    case.parsed
        .imports
        .iter()
        .find(|i| i.alias == model.configuration && i.kind == ForgeKind::TestConfiguration)
        .and_then(|import| reach.imported(&case.dir, import))
        .and_then(|document| {
            configurations
                .iter()
                .find(|c| c.document.identity == document.identity)
        })
        .ok_or_else(|| {
            Site {
                origin: case.origin(),
                subject: format!("test-case '{}'", model.name),
            }
            .refuse(format!(
                "'{}' names no configuration the suite reaches",
                model.configuration
            ))
        })
}

/// The system component type of `configuration` (E14): a port of each of
/// its system ports, in document order, every name entered in `names`.
fn system_type(
    configuration: &Configuration<'_>,
    reach: &Reach,
    names: &mut Namespace,
) -> Result<ComponentTypeView, Located<ForgeError>> {
    let document = configuration.document;
    names.definition(
        &configuration.name,
        Declared::read("the system component type", document.origin()),
    )?;
    let mut scope = Scope::default();
    let mut ports = Vec::new();
    for port in &configuration.model.system_ports {
        let origin = document.origin().at(port.source_line);
        names.local(
            &[],
            &mut scope,
            &port.id,
            Declared::read("the system port", origin.clone()),
        )?;
        let node = format!("sce:system-port '{}'", port.id);
        let port_type = document
            .parsed
            .imports
            .iter()
            .find(|i| i.alias == port.port_type && i.kind == ForgeKind::TestPortType)
            .and_then(|import| reach.imported(&document.dir, import))
            .ok_or_else(|| {
                Site {
                    origin,
                    subject: format!("test-configuration '{}': {node}", configuration.model.name),
                }
                .refuse(format!(
                    "'{}' names no port type the suite reaches",
                    port.port_type
                ))
            })?;
        ports.push(PortView {
            name: port.id.clone(),
            port_type: port_type.name.clone(),
            source: Source {
                rule: "E14",
                document: document.label.clone(),
                node,
                line: port.source_line,
            },
        });
    }
    Ok(ComponentTypeView {
        name: configuration.name.clone(),
        ports,
        timers: Vec::new(),
        variables: Vec::new(),
        source: document.source("E14"),
    })
}

/// The selection `text` of an `<sce:execute>` (ADR 0057's grammar) as the
/// condition of its `if`: each feature its module parameter, written by the
/// spelling every backend writes a selection by. `features` is the suite's
/// feature model, by its name, where it imports one; `site` the execute.
///
/// Refused by name where the selection does not read or names a feature the
/// model does not declare, which `check` refuses first (§11.8).
fn selection(
    text: &str,
    features: Option<(&str, &FeatureModel)>,
    site: &Site,
) -> Result<String, Located<ForgeError>> {
    let expr = crate::presence_condition::parse(text)
        .map_err(|e| site.refuse(format!("the selection '{text}' does not read: {e}")))?;
    let (name, model) = features.ok_or_else(|| {
        site.refuse(format!(
            "the selection '{text}' names features, and the suite imports no feature model"
        ))
    })?;
    PresenceCondition::resolve(&expr, model).map_err(|undeclared| {
        site.refuse(format!(
            "the selection '{text}' names '{}', which the feature model '{name}' does not \
             declare",
            undeclared.name
        ))
    })?;
    Ok(crate::forge::generator::ttcn3_selection(
        &expr,
        &feature_parameter,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A feature's module parameter is its name upper-cased, each hyphen an
    /// underscore, after `PICS_`: the spelling of the feature's C macro after
    /// its own prefix.
    #[test]
    fn a_features_module_parameter_is_its_name_in_upper_case() {
        assert_eq!(feature_parameter("job-timeout"), "PICS_JOB_TIMEOUT");
        assert_eq!(feature_parameter("ptcs"), "PICS_PTCS");
        assert_eq!(feature_parameter("v2-tls13"), "PICS_V2_TLS13");
        assert_eq!(
            feature_parameter("session-unicast")["PICS_".len()..],
            crate::feature_model::macro_spelling("session-unicast")["SCE_FEATURE_".len()..]
        );
    }

    /// A selection is written over the features' module parameters: a
    /// conjunction `and`, a disjunction `or`, a join inside another in
    /// parentheses and the outermost bare, since the `if` holds it.
    #[test]
    fn a_selection_is_written_over_the_features_module_parameters() {
        let model = FeatureModel::new(&[
            crate::feature_model::FeatureDeclaration {
                name: "ptcs".to_string(),
                requires: Vec::new(),
            },
            crate::feature_model::FeatureDeclaration {
                name: "removal".to_string(),
                requires: vec!["ptcs".to_string()],
            },
            crate::feature_model::FeatureDeclaration {
                name: "job-guard".to_string(),
                requires: Vec::new(),
            },
        ])
        .expect("a model");
        let site = Site {
            origin: Origin {
                kind: ForgeKind::TestSuite,
                document: "probe_suite".to_string(),
                label: "probe_suite.scxml".to_string(),
                line: Some(1),
            },
            subject: "test-suite 'probe_suite'".to_string(),
        };
        let written = |text: &str| {
            selection(text, Some(("probe_features", &model)), &site).expect("a selection")
        };
        assert_eq!(written("ptcs"), "PICS_PTCS");
        assert_eq!(written("all(ptcs, removal)"), "PICS_PTCS and PICS_REMOVAL");
        assert_eq!(
            written("any(job-guard, removal)"),
            "PICS_JOB_GUARD or PICS_REMOVAL"
        );
        assert_eq!(
            written("all(job-guard, any(ptcs, removal))"),
            "PICS_JOB_GUARD and (PICS_PTCS or PICS_REMOVAL)"
        );
        assert_eq!(
            written("any(all(ptcs, removal), job-guard)"),
            "(PICS_PTCS and PICS_REMOVAL) or PICS_JOB_GUARD"
        );
        assert!(selection("other", Some(("probe_features", &model)), &site).is_err());
        assert!(selection("ptcs", None, &site).is_err());
    }
}
