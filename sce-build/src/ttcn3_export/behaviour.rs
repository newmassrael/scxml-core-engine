// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The behaviour of each test component a suite reaches, as the definitions
//! of the behaviour module (docs/SCE_TEST_PROFILE.md §9.2 E4 to E13, §9.5),
//! and the module parameters it reads.
//!
//! - **E4.** Each component is a component type: its ports, each of its port
//!   type; its timers, each with its default duration; its variables, each
//!   with its initial value. A variable that is a module parameter is no
//!   variable of the type: it is the suite's `modulepar` of the same name,
//!   which every expression reads in its place.
//! - **E5.** Its statechart is one function that `runs on` the type, named
//!   `f_<component>`. A component that invokes a PTC applies the connects
//!   and maps of the configuration it runs in as it creates one (P9), so its
//!   behaviour depends on the configuration, and it has one function for each
//!   configuration that holds it, `f_<component>_in_<configuration>`.
//! - **E6, E7.** Each atomic state that is no top-level `<final>` is a label.
//!   After the label come the checks of the eventless transitions of the
//!   state and its ancestors, in the order of P3, each a `goto` when taken;
//!   then the start of the invocations of the state and its ancestors that
//!   are pending, outermost first, as the engine starts them; then an `alt`
//!   whose alternatives are the branches of the state and of each ancestor,
//!   in the order of P3.
//! - **E8.** A branch's block is what taking it does, in the order the engine
//!   does it ([`super::macrostep`]): each exited state's onexit and the kill
//!   of its PTCs, the transition's content, each entered state's onentry and
//!   a default entry's initial content; then a `goto` to the atomic state the
//!   machine is left in, or, where that is a top-level `<final>`, the exit of
//!   it and a `return` (E12).
//! - **E9, E10, E13.** A send is `p.send(t)`, a timer's start `T.start(d)` with
//!   `d` the exact decimal of its seconds, its stop `T.stop`, a verdict
//!   `setverdict(v, reason)`, a log `log(…)`.
//! - **E11.** A PTC is held in a reference local to the function, `v_<id>`,
//!   `null` while it is not created. Its creation is pending exactly while
//!   the reference is `null` and the invoking state is active: entering the
//!   state sets nothing, the start at a label creates it alive, applies each
//!   connect and map of the configuration that names it, and starts its
//!   function; leaving the state kills it and sets the reference to `null`.
//!
//! Every view carries the construct it was translated from and the rule that
//! translated it ([`Source`]), so the correspondence witness of §9.4 is a
//! serialization of these views. Every name written is held to the name
//! contract ([`super::names`]); an expression is written by
//! [`super::expression`].

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::expression::{self, octetstring, Reading};
use super::macrostep::{Content, Machine, Step, Taken};
use super::names::{written, Declared, Namespace, Origin, Scope};
use super::reach::{Reach, Reached, ReachedComponent};
use super::types::{type_expr, Site, TypeExpr};
use crate::forge::error::{ForgeError, Located};
use crate::forge::expr::{ExprKind, TypedExpr};
use crate::forge::model::{
    ForgeDocument, ForgeKind, SceType, TestConfigurationModel, TestSuiteModel,
};
use crate::forge::type_ctx::{StaticEnum, StaticEnumSpelling, StaticScope};
use crate::forge::types::InferredType;
use crate::forge::value_type::{TypeOf, TypeScope, ValueType};
use crate::model::{Action, BlockRole, NestedBlock, State, Transition};
use crate::test_component::{TemplateUse, TestInvocation, TestStatement, Verdict};

/// The prefix of the event a done branch names (docs/SCE_TEST_PROFILE.md
/// §4.2).
const DONE_INVOKE_PREFIX: &str = "done.invoke.";

/// Where a definition or a statement of the behaviour module comes from: the
/// rule of §9.2 that wrote it, the document and the node of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Source {
    /// The rule of docs/SCE_TEST_PROFILE.md §9.2, `E4` to `E15`.
    pub rule: &'static str,
    /// The document, by the label its refusals carry: its path.
    pub document: String,
    /// The node: an element by its kind and id, and a child by its position
    /// from 1, as `state 'requesting' / transition 1 / send 2`.
    pub node: String,
    /// The row the node is written on, where the model keeps one.
    pub line: Option<u32>,
}

/// A module parameter the suite declares (§11.8), as
/// `modulepar T name := value;`.
#[derive(Debug, Clone, Serialize)]
pub struct ModuleParameterView {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
    /// The default, as a literal of the type.
    pub value: String,
    pub source: Source,
}

/// A component type (E4).
#[derive(Debug, Clone, Serialize)]
pub struct ComponentTypeView {
    pub name: String,
    pub ports: Vec<PortView>,
    pub timers: Vec<TimerView>,
    pub variables: Vec<VariableView>,
    pub source: Source,
}

/// A port of a component type, of its port type.
#[derive(Debug, Clone, Serialize)]
pub struct PortView {
    pub name: String,
    pub port_type: String,
    pub source: Source,
}

/// A timer of a component type, with its default duration in seconds.
#[derive(Debug, Clone, Serialize)]
pub struct TimerView {
    pub name: String,
    pub duration: Option<String>,
    pub source: Source,
}

/// A variable of a component type, with its initial value.
#[derive(Debug, Clone, Serialize)]
pub struct VariableView {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
    pub init: String,
    pub source: Source,
}

/// A component's behaviour (E5): a function that `runs on` its type.
#[derive(Debug, Clone, Serialize)]
pub struct FunctionView {
    pub name: String,
    /// The component type it runs on.
    pub component: String,
    /// The configuration it is written for, where the behaviour depends on
    /// it.
    pub configuration: Option<String>,
    pub locals: Vec<LocalView>,
    pub body: Vec<Statement>,
    pub source: Source,
}

/// A variable local to a function: a reference to a PTC, `null` until it is
/// created, or the message a receive branch stores, unbound until it does.
#[derive(Debug, Clone, Serialize)]
pub struct LocalView {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
    /// Whether it starts at `null`: a reference to a PTC does.
    pub starts_null: bool,
    pub source: Source,
}

/// An alternative, and the message it stores, where it stores one: the
/// variable, the record, and the row of the branch.
struct Alternative {
    branch: Branch,
    stored: Option<(String, String, Option<u32>)>,
}

/// A template as a send or a receive names it, with the variables bound to
/// its formal parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TemplateCall {
    pub name: String,
    pub args: Vec<String>,
}

/// A `connect` or a `map` of a configuration, applied as a PTC is created:
/// `when`, where it is written, is a reference to a PTC that must be created
/// for the operation to have two ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionView {
    pub operation: &'static str,
    pub left: String,
    pub left_port: String,
    pub right: String,
    pub right_port: String,
    pub when: Option<String>,
}

/// One statement of a function's body.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Statement {
    Label {
        name: String,
        source: Source,
    },
    Goto {
        label: String,
        source: Source,
    },
    Return {
        source: Source,
    },
    Send {
        port: String,
        template: TemplateCall,
        source: Source,
    },
    StartTimer {
        timer: String,
        duration: Option<String>,
        source: Source,
    },
    StopTimer {
        timer: String,
        source: Source,
    },
    Setverdict {
        verdict: &'static str,
        reason: Option<String>,
        source: Source,
    },
    Log {
        items: Vec<String>,
        source: Source,
    },
    Assign {
        location: String,
        value: String,
        source: Source,
    },
    If {
        condition: String,
        then: Vec<Statement>,
        otherwise: Vec<Statement>,
        source: Source,
    },
    /// The start of a pending invocation: created alive while its reference
    /// is `null`, connected and mapped, then started.
    Create {
        reference: String,
        component_type: String,
        ptc: String,
        connections: Vec<ConnectionView>,
        function: String,
        source: Source,
    },
    /// The kill of a PTC as its invoking state is exited, where it was
    /// created.
    Kill {
        reference: String,
        source: Source,
    },
    Alt {
        branches: Vec<Branch>,
        source: Source,
    },
}

/// One alternative of an `alt` (E7, E8).
#[derive(Debug, Clone, Serialize)]
pub struct Branch {
    pub guard: Option<String>,
    pub event: BranchEvent,
    pub block: Vec<Statement>,
    pub source: Source,
}

/// What an alternative waits for (P2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum BranchEvent {
    Receive {
        port: String,
        template: Option<TemplateCall>,
        redirect: Option<String>,
    },
    Timeout {
        timer: String,
    },
    Done {
        reference: String,
    },
}

/// The behaviour of a suite: every component type and function, in the order
/// of their names.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Behaviour {
    pub component_types: Vec<ComponentTypeView>,
    pub functions: Vec<FunctionView>,
}

/// A duration in microseconds as the exact decimal of its seconds, as TTCN-3
/// writes a `float` (`2.0`, `0.5`, `0.000001`): never rounded, since every
/// duration of the profile is a whole number of microseconds (§4.7).
pub(super) fn seconds(us: u64) -> String {
    let mut fraction = format!("{:06}", us % 1_000_000);
    while fraction.len() > 1 && fraction.ends_with('0') {
        fraction.pop();
    }
    format!("{}.{fraction}", us / 1_000_000)
}

/// The value `text`, a literal of §11.4, as TTCN-3 writes a value of `ty`.
fn literal(ty: &ValueType, text: &str) -> Option<String> {
    match ty {
        TypeOf::Scalar(SceType::Bool) => match text.trim() {
            "true" => Some("true".to_string()),
            "false" => Some("false".to_string()),
            _ => None,
        },
        TypeOf::Scalar(SceType::Enum(_)) => Some(text.trim().to_string()),
        TypeOf::Scalar(scalar) if scalar.int_bit_width().is_some() => {
            crate::forge::test_profile::decimal_integer(text).map(|n| n.to_string())
        }
        TypeOf::Text { .. } => Some(crate::filters::ttcn3_charstring(text.to_string())),
        TypeOf::Bytes { .. } => Some(format!("'{}'O", text.trim().to_ascii_uppercase())),
        _ => None,
    }
}

/// Each module parameter `suite` declares, read in `scope`, every name
/// entered in `names`; `at` is the suite.
pub(super) fn module_parameters(
    suite: &TestSuiteModel,
    scope: &TypeScope,
    at: &Origin,
    names: &mut Namespace,
) -> Result<Vec<ModuleParameterView>, Located<ForgeError>> {
    let mut views = Vec::new();
    for parameter in &suite.module_parameters {
        let origin = at.at(parameter.source_line);
        names.definition(
            &parameter.id,
            Declared::read("the module parameter", origin.clone()),
        )?;
        let site = Site {
            origin: origin.clone(),
            subject: format!(
                "test-suite '{}': the module parameter '{}'",
                suite.name, parameter.id
            ),
        };
        let ty = type_expr(&parameter.parameter_type, scope, &site)?;
        let value = literal(&parameter.parameter_type, &parameter.value).ok_or_else(|| {
            site.refuse(format!("'{}' is no literal of its type", parameter.value))
        })?;
        views.push(ModuleParameterView {
            name: parameter.id.clone(),
            ty,
            value,
            source: Source {
                rule: "E15",
                document: at.label.clone(),
                node: format!("sce:module-parameter '{}'", parameter.id),
                line: parameter.source_line,
            },
        });
    }
    Ok(views)
}

/// A configuration the suite reaches, its components resolved.
struct Configuration<'r> {
    name: String,
    model: &'r TestConfigurationModel,
    mtc: &'r ReachedComponent,
    /// Each PTC by its id, with its component.
    ptcs: Vec<(String, &'r ReachedComponent)>,
}

impl<'r> Configuration<'r> {
    /// Whether the component `component` takes part in this configuration.
    fn holds(&self, component: &ReachedComponent) -> bool {
        self.mtc.identity == component.identity
            || self
                .ptcs
                .iter()
                .any(|(_, c)| c.identity == component.identity)
    }

    /// The name `component` is known by here: `mtc`, or its PTC's id. A
    /// component that invokes a PTC is in a configuration once (§11.6: each
    /// PTC is created by exactly one invocation).
    fn name_of(&self, component: &ReachedComponent) -> String {
        if self.mtc.identity == component.identity {
            return TestConfigurationModel::MTC.to_string();
        }
        self.ptcs
            .iter()
            .find(|(_, c)| c.identity == component.identity)
            .map(|(id, _)| id.clone())
            .unwrap_or_default()
    }

    fn ptc(&self, id: &str) -> Option<&'r ReachedComponent> {
        self.ptcs.iter().find(|(p, _)| p == id).map(|(_, c)| *c)
    }
}

/// Every configuration the suite reaches, in the order of their names.
fn configurations(reach: &Reach) -> Result<Vec<Configuration<'_>>, Located<ForgeError>> {
    let mut configurations = Vec::new();
    for document in reach.sorted() {
        let ForgeDocument::TestConfiguration(model) = &document.parsed.document else {
            continue;
        };
        let component = |alias: &str| {
            document
                .parsed
                .imports
                .iter()
                .find(|i| i.alias == alias && i.kind == ForgeKind::TestComponent)
                .and_then(|import| reach.component(&document.dir, import))
                .ok_or_else(|| unread(document, alias))
        };
        let mtc = component(&model.mtc)?;
        let mut ptcs = Vec::new();
        for ptc in &model.ptcs {
            ptcs.push((ptc.id.clone(), component(&ptc.component)?));
        }
        configurations.push(Configuration {
            name: document.name.clone(),
            model,
            mtc,
            ptcs,
        });
    }
    Ok(configurations)
}

fn unread(document: &Reached, alias: &str) -> Located<ForgeError> {
    Site {
        origin: document.origin(),
        subject: format!(
            "{} '{}'",
            document.parsed.document.kind(),
            document.parsed.document.name()
        ),
    }
    .refuse(format!("'{alias}' names no document the suite reaches"))
}

/// The behaviour of every component the suite reaches, every name it writes
/// entered in `names`.
pub(super) fn behaviour(
    reach: &Reach,
    names: &mut Namespace,
) -> Result<Behaviour, Located<ForgeError>> {
    let configurations = configurations(reach)?;
    let mut behaviour = Behaviour::default();
    let mut writers = Vec::new();
    for component in reach.components() {
        let writer = Writer::new(reach, component)?;
        let mut scope = Scope::default();
        behaviour
            .component_types
            .push(writer.component_type(names, &mut scope)?);
        writers.push((writer, scope));
    }
    for (writer, scope) in &writers {
        let holding: Vec<&Configuration<'_>> = configurations
            .iter()
            .filter(|c| c.holds(writer.component))
            .collect();
        if writer.invocations().is_empty() {
            behaviour
                .functions
                .push(writer.function(names, scope, None)?);
            continue;
        }
        for configuration in holding {
            behaviour
                .functions
                .push(writer.function(names, scope, Some(configuration))?);
        }
    }
    behaviour.functions.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(behaviour)
}

/// The name of the function `component` runs in `configuration`.
fn function_name(component: &ReachedComponent, configuration: &Configuration<'_>) -> String {
    let invokes = component
        .model
        .test_component
        .as_ref()
        .is_some_and(|p| !p.invocations.is_empty());
    if invokes {
        format!("f_{}_in_{}", component.name, configuration.name)
    } else {
        format!("f_{}", component.name)
    }
}

/// The message a receive branch stores: the variable, and the schema
/// `_event.data` is typed by.
struct Message {
    variable: String,
    record: String,
    schema: Option<String>,
    read: Cell<bool>,
}

/// What one component's behaviour is written with.
struct Writer<'r> {
    reach: &'r Reach,
    component: &'r ReachedComponent,
    machine: Machine<'r>,
    scope: StaticScope,
    enums: Vec<StaticEnum>,
    /// Every variable and module parameter, by id: what an expression reads.
    readable: BTreeSet<String>,
    /// The variables of the component type, each with its type.
    variables: BTreeMap<String, ValueType>,
    received: BTreeMap<String, String>,
}

impl<'r> Writer<'r> {
    fn new(reach: &'r Reach, component: &'r ReachedComponent) -> Result<Self, Located<ForgeError>> {
        let model = &component.model;
        let site = Site {
            origin: component.origin(None),
            subject: format!("test-component '{}'", model.name),
        };
        let machine = Machine::of(model).map_err(|why| site.refuse(why))?;
        let scope = StaticScope::of(model)
            .ok_or_else(|| site.refuse("it declares no sce-static data model"))?;
        let enums = StaticEnum::from_imports(&model.imported_enums, |alias, _| {
            StaticEnumSpelling::unlowered(alias)
        });
        let parameters: BTreeSet<&str> = model
            .test_component
            .iter()
            .flat_map(|p| p.module_parameters.iter().map(String::as_str))
            .collect();
        let mut readable = BTreeSet::new();
        let mut variables = BTreeMap::new();
        for variable in &scope.variables {
            readable.insert(variable.id.clone());
            if parameters.contains(variable.id.as_str()) {
                continue;
            }
            let site = Site {
                origin: component.origin(variable.source_location.as_ref().and_then(|l| l.line)),
                subject: format!(
                    "test-component '{}': the variable '{}'",
                    model.name, variable.id
                ),
            };
            let ty = variable
                .value_type
                .clone()
                .ok_or_else(|| site.refuse("it declares no type"))?
                .bounded()
                .map_err(|why| site.refuse(why.rule()))?;
            let scalar = match &ty {
                TypeOf::Scalar(scalar) => !scalar.is_float(),
                TypeOf::Text { .. } | TypeOf::Bytes { .. } => true,
                _ => false,
            };
            if !scalar {
                return Err(site.refuse(format!(
                    "it is of {ty}: a component variable the export writes is a number, a bool, \
                     a variant of an enum, a text or a byte string"
                )));
            }
            variables.insert(variable.id.clone(), ty);
        }
        Ok(Self {
            reach,
            component,
            machine,
            scope,
            enums,
            readable,
            variables,
            received: model.received_events(),
        })
    }

    fn invocations(&self) -> &[TestInvocation] {
        self.component
            .model
            .test_component
            .as_ref()
            .map_or(&[], |p| p.invocations.as_slice())
    }

    fn source(&self, rule: &'static str, node: String, line: Option<u32>) -> Source {
        Source {
            rule,
            document: self.component.label.clone(),
            node,
            line,
        }
    }

    fn site(&self, node: &str, line: Option<u32>) -> Site {
        Site {
            origin: self.component.origin(line),
            subject: format!("test-component '{}': {node}", self.component.model.name),
        }
    }

    /// The name of the document the component imports as `alias`, of `kind`.
    fn imported(&self, alias: &str, kind: ForgeKind) -> Option<String> {
        let import = self
            .component
            .model
            .forge_imports
            .iter()
            .find(|i| i.alias == alias && i.kind == kind)?;
        self.reach
            .imported(&self.component.dir, import)
            .map(|document| document.name.clone())
    }

    fn template_call(
        &self,
        used: &TemplateUse,
        site: &Site,
    ) -> Result<TemplateCall, Located<ForgeError>> {
        let alias = used
            .template
            .as_deref()
            .ok_or_else(|| site.refuse("it names no template"))?;
        let name = self
            .imported(alias, ForgeKind::TestTemplate)
            .ok_or_else(|| site.refuse(format!("'{alias}' names no template the suite reaches")))?;
        for arg in &used.args {
            if !self.readable.contains(arg) {
                return Err(site.refuse(format!("'{arg}' is no variable of the component")));
            }
        }
        Ok(TemplateCall {
            name,
            args: used.args.clone(),
        })
    }

    /// The component type (E4), each name of it entered in `scope`.
    fn component_type(
        &self,
        names: &mut Namespace,
        scope: &mut Scope,
    ) -> Result<ComponentTypeView, Located<ForgeError>> {
        let component = self.component;
        let model = &component.model;
        names.definition(
            &component.name,
            Declared::read("the component type", component.origin(None)),
        )?;
        let profile = model.test_component.clone().unwrap_or_default();
        let mut ports = Vec::new();
        for port in &profile.ports {
            let line = port.source_location.as_ref().and_then(|l| l.line);
            names.local(
                &[],
                scope,
                &port.id,
                Declared::read("the port", component.origin(line)),
            )?;
            let node = format!("sce:test-port '{}'", port.id);
            let port_type = self
                .imported(&port.port_type, ForgeKind::TestPortType)
                .ok_or_else(|| {
                    self.site(&node, line).refuse(format!(
                        "'{}' names no port type the suite reaches",
                        port.port_type
                    ))
                })?;
            ports.push(PortView {
                name: port.id.clone(),
                port_type,
                source: self.source("E4", node, line),
            });
        }
        let mut timers = Vec::new();
        for timer in &profile.timers {
            let line = timer.source_location.as_ref().and_then(|l| l.line);
            names.local(
                &[],
                scope,
                &timer.id,
                Declared::read("the timer", component.origin(line)),
            )?;
            let node = format!("sce:test-timer '{}'", timer.id);
            let duration = timer
                .duration
                .as_deref()
                .map(|d| self.duration(d, &self.site(&node, line)))
                .transpose()?;
            timers.push(TimerView {
                name: timer.id.clone(),
                duration,
                source: self.source("E4", node, line),
            });
        }
        let mut variables = Vec::new();
        for variable in &self.scope.variables {
            let Some(ty) = self.variables.get(&variable.id) else {
                continue;
            };
            let line = variable.source_location.as_ref().and_then(|l| l.line);
            names.local(
                &[],
                scope,
                &variable.id,
                Declared::read("the variable", component.origin(line)),
            )?;
            let node = format!("data '{}'", variable.id);
            let site = self.site(&node, line);
            let init = self.value_for(ty, &variable.expr, None, &site)?;
            variables.push(VariableView {
                name: variable.id.clone(),
                ty: type_expr(ty, &model.imported_type_scope, &site)?,
                init,
                source: self.source("E4", node, line),
            });
        }
        Ok(ComponentTypeView {
            name: component.name.clone(),
            ports,
            timers,
            variables,
            source: self.source("E4", "scxml".to_string(), None),
        })
    }

    fn duration(&self, written_duration: &str, site: &Site) -> Result<String, Located<ForgeError>> {
        crate::forge::test_profile::duration_us(written_duration)
            .map(seconds)
            .map_err(|_| {
                site.refuse(format!(
                    "'{written_duration}' is no whole number of microseconds"
                ))
            })
    }

    /// `text` typed against the component's scope, the received message's
    /// fields in it where `message` is one, and written as TTCN-3.
    fn expression(
        &self,
        text: &str,
        message: Option<&Message>,
        site: &Site,
    ) -> Result<(TypedExpr, String), Located<ForgeError>> {
        let paths = self.scope.paths(message.and_then(|m| m.schema.as_deref()));
        let ctx = self.scope.ctx(&paths, &self.enums);
        let typed = crate::forge::expr::resolve(text, &ctx).map_err(|refusal| {
            site.refuse(format!(
                "`{text}` has no TTCN-3 form the export writes: {}",
                refusal.error
            ))
        })?;
        let reading = Reading {
            names: &self.readable,
            enums: &self.component.model.imported_enums,
            message: message.map(|m| m.variable.as_str()),
        };
        let printed = expression::print(&typed, &reading)
            .map_err(|why| site.refuse(format!("`{text}` has no TTCN-3 form: {why}")))?;
        if let (Some(message), true) = (message, printed.reads_message) {
            message.read.set(true);
        }
        Ok((typed, printed.text))
    }

    /// `text` as the value a place of type `place` takes. A text or a byte
    /// string is taken from a literal alone, whose bound the data model has
    /// held already: from anything else, `sce-static` fails an assignment
    /// past the place's bound, and TTCN-3 bounds neither. An integer is taken
    /// from a value of a type the place holds every value of, for the same
    /// reason.
    fn value_for(
        &self,
        place: &ValueType,
        text: &str,
        message: Option<&Message>,
        site: &Site,
    ) -> Result<String, Located<ForgeError>> {
        let (typed, value) = self.expression(text, message, site)?;
        match place {
            TypeOf::Text { .. } if !matches!(typed.kind, ExprKind::StringLit { .. }) => Err(site
                .refuse(format!(
                    "`{text}` is a text whose length the place may not hold, which sce-static \
                     fails at run time and a TTCN-3 universal charstring does not bound"
                ))),
            TypeOf::Bytes { .. } => match &typed.kind {
                ExprKind::BytesLit { bytes } => Ok(octetstring(bytes)),
                ExprKind::StringLit { value } => Ok(octetstring(value.as_bytes())),
                _ => Err(site.refuse(format!(
                    "`{text}` is a byte string whose length the place may not hold, which \
                     sce-static fails at run time and a TTCN-3 octetstring does not bound"
                ))),
            },
            TypeOf::Scalar(scalar) => {
                if let (Some((low, high)), InferredType::Int { signed, bits }) =
                    (scalar.int_value_range(), typed.ty)
                {
                    let (from, to) = if signed {
                        let half = 1i128 << (bits - 1);
                        (-half, half - 1)
                    } else {
                        (0, (1i128 << bits) - 1)
                    };
                    if from < low || to > high {
                        return Err(site.refuse(format!(
                            "`{text}` may be a number {} does not hold, which sce-static fails \
                             at run time and TTCN-3 refuses as an error",
                            scalar.as_attr()
                        )));
                    }
                }
                Ok(value)
            }
            _ => Ok(value),
        }
    }

    /// The statements of `actions`, the executable content at `node`.
    fn actions(
        &self,
        actions: &[Action],
        message: Option<&Message>,
        node: &str,
        line: Option<u32>,
    ) -> Result<Vec<Statement>, Located<ForgeError>> {
        let mut statements = Vec::new();
        for (position, action) in actions.iter().enumerate() {
            let node = format!("{node} / {} {}", action.action_type, position + 1);
            let site = self.site(&node, line);
            if let Some(statement) = &action.test_statement {
                statements.push(self.test_statement(statement, message, &node, line, &site)?);
                continue;
            }
            match action.action_type.as_str() {
                "assign" => {
                    let place = self.variables.get(&action.location).ok_or_else(|| {
                        site.refuse(format!(
                            "'{}' is no variable of the component the export writes",
                            action.location
                        ))
                    })?;
                    statements.push(Statement::Assign {
                        location: action.location.clone(),
                        value: self.value_for(place, &action.expr, message, &site)?,
                        source: self.source("E8", node, line),
                    });
                }
                "log" => {
                    let mut items = Vec::new();
                    let value = if action.expr.trim().is_empty() {
                        None
                    } else {
                        Some(self.expression(&action.expr, message, &site)?.1)
                    };
                    match (&value, action.label.is_empty()) {
                        (Some(_), false) => items.push(crate::filters::ttcn3_charstring(format!(
                            "{}: ",
                            action.label
                        ))),
                        (None, _) => {
                            items.push(crate::filters::ttcn3_charstring(action.label.clone()))
                        }
                        (Some(_), true) => {}
                    }
                    items.extend(value);
                    statements.push(Statement::Log {
                        items,
                        source: self.source("E13", node, line),
                    });
                }
                "send" => {
                    let used = action
                        .test_template
                        .as_ref()
                        .ok_or_else(|| site.refuse("a send names a template of the profile"))?;
                    statements.push(Statement::Send {
                        port: action.target.clone(),
                        template: self.template_call(used, &site)?,
                        source: self.source("E9", node, line),
                    });
                }
                "if" => statements.push(self.conditional(action, message, &node, line)?),
                other => {
                    return Err(
                        site.refuse(format!("<{other}> has no TTCN-3 form the export writes"))
                    )
                }
            }
        }
        Ok(statements)
    }

    /// An `<if>`, its `<elseif>`s as `if`s nested in the `else` before them.
    /// The blocks are read through [`Action::nested_blocks`], the one
    /// definition of what lies inside an action.
    fn conditional(
        &self,
        action: &Action,
        message: Option<&Message>,
        node: &str,
        line: Option<u32>,
    ) -> Result<Statement, Located<ForgeError>> {
        let site = self.site(node, line);
        let blocks = action.nested_blocks();
        let mut then: &[Action] = &[];
        let mut elseifs: Vec<&NestedBlock<'_>> = Vec::new();
        let mut otherwise: &[Action] = &[];
        for block in &blocks {
            match block.role {
                BlockRole::Then => then = block.actions,
                BlockRole::ElseIf => elseifs.push(block),
                BlockRole::Else => otherwise = block.actions,
                BlockRole::Body if block.actions.is_empty() => {}
                BlockRole::Body => return Err(site.refuse("an <if> holds no loop body")),
            }
        }
        let mut otherwise = self.actions(otherwise, message, &format!("{node} / else"), line)?;
        for (position, branch) in elseifs.iter().enumerate().rev() {
            let branch_node = format!("{node} / elseif {}", position + 1);
            let cond = branch
                .cond
                .ok_or_else(|| site.refuse("an <elseif> holds its condition"))?;
            otherwise = vec![Statement::If {
                condition: self.expression(cond, message, &site)?.1,
                then: self.actions(branch.actions, message, &branch_node, line)?,
                otherwise,
                source: self.source("E8", branch_node, line),
            }];
        }
        Ok(Statement::If {
            condition: self.expression(&action.cond, message, &site)?.1,
            then: self.actions(then, message, node, line)?,
            otherwise,
            source: self.source("E8", node.to_string(), line),
        })
    }

    fn test_statement(
        &self,
        statement: &TestStatement,
        message: Option<&Message>,
        node: &str,
        line: Option<u32>,
        site: &Site,
    ) -> Result<Statement, Located<ForgeError>> {
        Ok(match statement {
            TestStatement::Setverdict { verdict, reason } => Statement::Setverdict {
                verdict: match verdict {
                    Verdict::Pass => "pass",
                    Verdict::Inconclusive => "inconc",
                    Verdict::Fail => "fail",
                    Verdict::None => "none",
                },
                reason: reason
                    .as_deref()
                    .map(|reason| self.expression(reason, message, site).map(|(_, text)| text))
                    .transpose()?,
                source: self.source("E13", node.to_string(), line),
            },
            TestStatement::StartTimer { timer, duration } => Statement::StartTimer {
                timer: timer.clone(),
                duration: duration
                    .as_deref()
                    .map(|d| self.duration(d, site))
                    .transpose()?,
                source: self.source("E10", node.to_string(), line),
            },
            TestStatement::StopTimer { timer } => Statement::StopTimer {
                timer: timer.clone(),
                source: self.source("E10", node.to_string(), line),
            },
        })
    }

    fn state(&self, id: &str) -> Option<&'r State> {
        self.component.model.states.get(id)
    }

    /// `leaf` and each of its ancestors, innermost first.
    fn chain(&self, leaf: &str) -> Vec<&'r State> {
        let mut chain: Vec<&'r State> =
            crate::transition_function::configuration(&self.component.model, leaf)
                .iter()
                .filter_map(|id| self.state(id))
                .collect();
        chain.reverse();
        chain
    }

    /// The message a receive branch of `transition` stores, when its content
    /// may read one.
    fn message(&self, transition: &Transition) -> Option<Message> {
        let branch = transition.test_branch.as_ref()?;
        branch.receive.as_ref()?;
        let template = branch.template.template.as_ref()?;
        let record_alias = self.component.model.received_records.get(template)?;
        let record = self.imported(record_alias, ForgeKind::EventSchema)?;
        let schema = self
            .component
            .model
            .imported_event_schemas
            .get(transition.payload_event(&self.received))
            .map(|schema| schema.name.clone());
        Some(Message {
            variable: format!("v_{record}"),
            record,
            schema,
            read: Cell::new(false),
        })
    }

    /// The statements of `steps`, the transition the content of which reads
    /// `message` where one is.
    fn steps(
        &self,
        steps: &[Step],
        message: Option<&Message>,
    ) -> Result<Vec<Statement>, Located<ForgeError>> {
        let mut statements = Vec::new();
        for step in steps {
            match step {
                Step::Exit(id) => {
                    let Some(state) = self.state(id) else {
                        continue;
                    };
                    let line = state.source_location.as_ref().and_then(|l| l.line);
                    for (position, block) in state.on_exit_blocks.iter().enumerate() {
                        let node = format!("state '{id}' / onexit {}", position + 1);
                        statements.extend(self.actions(block, None, &node, line)?);
                    }
                    for invocation in self.invocations().iter().filter(|i| &i.state == id) {
                        let ptc = invocation.id.clone().unwrap_or_default();
                        statements.push(Statement::Kill {
                            reference: format!("v_{ptc}"),
                            source: self.source(
                                "E11",
                                format!("invoke '{ptc}'"),
                                invocation.source_location.as_ref().and_then(|l| l.line),
                            ),
                        });
                    }
                }
                Step::Content(Content::Written { state, index }) => {
                    let Some(transition) =
                        self.state(state).and_then(|s| s.transitions.get(*index))
                    else {
                        continue;
                    };
                    let node = format!("state '{state}' / transition {}", index + 1);
                    let line = transition.source_location.as_ref().and_then(|l| l.line);
                    statements.extend(self.actions(&transition.actions, message, &node, line)?);
                }
                Step::Content(Content::Initial(id)) => {
                    if let Some(state) = self.state(id) {
                        let node = format!("state '{id}' / initial");
                        let line = state.source_location.as_ref().and_then(|l| l.line);
                        statements.extend(self.actions(
                            &state.initial_transition_actions,
                            None,
                            &node,
                            line,
                        )?);
                    }
                }
                Step::Content(Content::Document) => {}
                Step::Enter { state: id, default } => {
                    let Some(state) = self.state(id) else {
                        continue;
                    };
                    let line = state.source_location.as_ref().and_then(|l| l.line);
                    for (position, block) in state.on_entry_blocks.iter().enumerate() {
                        let node = format!("state '{id}' / onentry {}", position + 1);
                        statements.extend(self.actions(block, None, &node, line)?);
                    }
                    if *default {
                        let node = format!("state '{id}' / initial");
                        statements.extend(self.actions(
                            &state.initial_transition_actions,
                            None,
                            &node,
                            line,
                        )?);
                    }
                }
            }
        }
        Ok(statements)
    }

    /// The statements that end a macrostep that left the machine in
    /// `taken.leaf`: a `goto` its label, or, for a top-level `<final>`, its
    /// exit and a `return` (E12).
    fn ending(
        &self,
        taken: &Taken,
        node: &str,
        line: Option<u32>,
    ) -> Result<Vec<Statement>, Located<ForgeError>> {
        if self.machine.is_top_level_final(&taken.leaf) {
            let mut statements = self.steps(&self.machine.halt(&taken.leaf), None)?;
            let line = self
                .state(&taken.leaf)
                .and_then(|s| s.source_location.as_ref())
                .and_then(|l| l.line);
            statements.push(Statement::Return {
                source: self.source("E12", format!("final '{}'", taken.leaf), line),
            });
            return Ok(statements);
        }
        Ok(vec![Statement::Goto {
            label: taken.leaf.clone(),
            source: self.source("E8", node.to_string(), line),
        }])
    }

    /// What taking the `index`-th transition of `state` from `leaf` does.
    fn take(
        &self,
        leaf: &str,
        state: &State,
        index: usize,
        message: Option<&Message>,
    ) -> Result<Vec<Statement>, Located<ForgeError>> {
        let node = format!("state '{}' / transition {}", state.id, index + 1);
        let line = state.transitions[index]
            .source_location
            .as_ref()
            .and_then(|l| l.line);
        let taken = self.machine.take(leaf, &state.id, index).ok_or_else(|| {
            self.site(&node, line)
                .refuse("it is no transition of the tables")
        })?;
        let mut statements = self.steps(&taken.steps, message)?;
        statements.extend(self.ending(&taken, &node, line)?);
        Ok(statements)
    }

    /// The function the component runs in `configuration`, or in every
    /// configuration when it invokes no PTC (E5), every name of it entered in
    /// `names` with the component type's `component_scope` outside it.
    fn function(
        &self,
        names: &mut Namespace,
        component_scope: &Scope,
        configuration: Option<&Configuration<'_>>,
    ) -> Result<FunctionView, Located<ForgeError>> {
        let component = self.component;
        let model = &component.model;
        let name = match configuration {
            Some(configuration) => format!("f_{}_in_{}", component.name, configuration.name),
            None => format!("f_{}", component.name),
        };
        names.definition(
            &name,
            Declared::made("the function", component.origin(None)),
        )?;
        let mut scope = Scope::default();
        let mut locals = Vec::new();
        for invocation in self.invocations() {
            let line = invocation.source_location.as_ref().and_then(|l| l.line);
            let ptc = invocation.id.clone().unwrap_or_default();
            written(&ptc, &Declared::read("the PTC", component.origin(line)))?;
            let reference = format!("v_{ptc}");
            names.local(
                &[component_scope],
                &mut scope,
                &reference,
                Declared::made("the reference to the PTC", component.origin(line)),
            )?;
            let site = self.site(&format!("invoke '{ptc}'"), line);
            let ptc_component = configuration
                .and_then(|c| c.ptc(&ptc))
                .ok_or_else(|| site.refuse("no configuration of the suite declares the PTC"))?;
            locals.push(LocalView {
                name: reference,
                ty: TypeExpr::Ref {
                    name: ptc_component.name.clone(),
                },
                starts_null: true,
                source: self.source("E11", format!("invoke '{ptc}'"), line),
            });
        }

        let mut body = Vec::new();
        let start = self.machine.start();
        body.extend(self.steps(&start.steps, None)?);
        body.extend(self.ending(&start, "scxml", None)?);
        let mut messages: BTreeMap<String, (String, Option<u32>)> = BTreeMap::new();
        let mut labels = Vec::new();
        let mut leaves: Vec<&State> = model
            .states
            .values()
            .filter(|s| s.children.is_empty() && !s.is_parallel)
            .filter(|s| !self.machine.is_top_level_final(&s.id))
            .collect();
        leaves.sort_by_key(|s| s.document_order);
        for leaf in leaves {
            let line = leaf.source_location.as_ref().and_then(|l| l.line);
            labels.push((leaf.id.clone(), line));
            body.push(Statement::Label {
                name: leaf.id.clone(),
                source: self.source("E6", format!("state '{}'", leaf.id), line),
            });
            let chain = self.chain(&leaf.id);
            let mut ended = false;
            'eventless: for state in &chain {
                for (index, transition) in state.transitions.iter().enumerate() {
                    if !transition.event.is_empty() || transition.test_branch.is_some() {
                        continue;
                    }
                    let block = self.take(&leaf.id, state, index, None)?;
                    let node = format!("state '{}' / transition {}", state.id, index + 1);
                    let tline = transition.source_location.as_ref().and_then(|l| l.line);
                    if transition.cond.trim().is_empty() {
                        body.extend(block);
                        ended = true;
                        break 'eventless;
                    }
                    let site = self.site(&node, tline);
                    body.push(Statement::If {
                        condition: self.expression(&transition.cond, None, &site)?.1,
                        then: block,
                        otherwise: Vec::new(),
                        source: self.source("E6", node, tline),
                    });
                }
            }
            if ended {
                continue;
            }
            for state in chain.iter().rev() {
                for invocation in self.invocations().iter().filter(|i| i.state == state.id) {
                    body.push(self.create(invocation, configuration)?);
                }
            }
            let mut branches = Vec::new();
            for state in &chain {
                for (index, transition) in state.transitions.iter().enumerate() {
                    if let Some(alternative) = self.branch(&leaf.id, state, index, transition)? {
                        if let Some((variable, record, line)) = alternative.stored {
                            messages.insert(variable, (record, line));
                        }
                        branches.push(alternative.branch);
                    }
                }
            }
            body.push(Statement::Alt {
                branches,
                source: self.source("E7", format!("state '{}'", leaf.id), line),
            });
        }
        for (variable, (record, line)) in &messages {
            names.local(
                &[component_scope],
                &mut scope,
                variable,
                Declared::made(
                    "the variable a received message is stored in",
                    component.origin(*line),
                ),
            )?;
            locals.push(LocalView {
                name: variable.clone(),
                ty: TypeExpr::Ref {
                    name: record.clone(),
                },
                starts_null: false,
                source: self.source("E8", format!("received {record}"), *line),
            });
        }
        for (label, line) in &labels {
            names.local(
                &[component_scope],
                &mut scope,
                label,
                Declared::read("the state", component.origin(*line)),
            )?;
        }
        Ok(FunctionView {
            name,
            component: component.name.clone(),
            configuration: configuration.map(|c| c.name.clone()),
            locals,
            body,
            source: self.source("E5", "scxml".to_string(), None),
        })
    }

    /// The alternative the `index`-th transition of `state` is from `leaf`,
    /// when it is a branch, with the message it stores, if it stores one.
    fn branch(
        &self,
        leaf: &str,
        state: &State,
        index: usize,
        transition: &Transition,
    ) -> Result<Option<Alternative>, Located<ForgeError>> {
        let node = format!("state '{}' / transition {}", state.id, index + 1);
        let line = transition.source_location.as_ref().and_then(|l| l.line);
        let site = self.site(&node, line);
        let profile_branch = transition.test_branch.as_ref();
        let done = transition.event.strip_prefix(DONE_INVOKE_PREFIX);
        if profile_branch.is_none() && done.is_none() {
            return Ok(None);
        }
        let message = self.message(transition);
        let block = self.take(leaf, state, index, message.as_ref())?;
        let guard = if transition.cond.trim().is_empty() {
            None
        } else {
            Some(self.expression(&transition.cond, None, &site)?.1)
        };
        let event = match (profile_branch, done) {
            (Some(branch), _) if branch.receive.is_some() => BranchEvent::Receive {
                port: branch.receive.clone().unwrap_or_default(),
                template: if branch.template.template.is_some() {
                    Some(self.template_call(&branch.template, &site)?)
                } else {
                    None
                },
                redirect: message
                    .as_ref()
                    .filter(|m| m.read.get())
                    .map(|m| m.variable.clone()),
            },
            (Some(branch), _) => BranchEvent::Timeout {
                timer: branch.timeout.clone().unwrap_or_default(),
            },
            (None, Some(ptc)) => BranchEvent::Done {
                reference: format!("v_{ptc}"),
            },
            (None, None) => return Ok(None),
        };
        let stored = message
            .filter(|m| m.read.get())
            .map(|m| (m.variable, m.record, line));
        Ok(Some(Alternative {
            branch: Branch {
                guard,
                event,
                block,
                source: self.source("E8", node, line),
            },
            stored,
        }))
    }

    /// The start of `invocation` (E11): created alive, joined by every
    /// connect and map of `configuration` that names it, then started.
    fn create(
        &self,
        invocation: &TestInvocation,
        configuration: Option<&Configuration<'_>>,
    ) -> Result<Statement, Located<ForgeError>> {
        let ptc = invocation.id.clone().unwrap_or_default();
        let line = invocation.source_location.as_ref().and_then(|l| l.line);
        let node = format!("invoke '{ptc}'");
        let site = self.site(&node, line);
        let configuration = configuration
            .ok_or_else(|| site.refuse("no configuration of the suite declares the PTC"))?;
        let created = configuration
            .ptc(&ptc)
            .ok_or_else(|| site.refuse("the configuration declares no such PTC"))?;
        let me = configuration.name_of(self.component);
        let siblings: BTreeSet<String> = self
            .invocations()
            .iter()
            .filter_map(|i| i.id.clone())
            .collect();
        let reference = format!("v_{ptc}");
        // The other end of a connect, as the creator names it: the MTC, the
        // creator itself, the PTC, or another PTC the creator creates, which
        // must be created for the connect to have two ends; a PTC the
        // creator holds no reference to is named by none of these
        // (-1 §21.3.1: a reference outside its scope of creation is passed
        // to the component that uses it).
        let end = |component: &str| -> Result<(String, Option<String>), Located<ForgeError>> {
            if component == ptc {
                Ok((reference.clone(), None))
            } else if component == TestConfigurationModel::MTC {
                Ok(("mtc".to_string(), None))
            } else if component == me {
                Ok(("self".to_string(), None))
            } else if siblings.contains(component) {
                let other = format!("v_{component}");
                Ok((other.clone(), Some(other)))
            } else {
                Err(site.refuse(format!(
                    "the configuration '{}' connects it to '{component}', which its creator \
                     holds no reference to: the export applies a PTC's connects as its creator \
                     creates it, naming the other end by `mtc`, `self` or a PTC the creator \
                     creates",
                    configuration.name
                )))
            }
        };
        let mut connections = Vec::new();
        for connect in &configuration.model.connects {
            let [first, second] = connect.endpoints.as_slice() else {
                continue;
            };
            let (named, other) = if first.component == ptc {
                (first, second)
            } else if second.component == ptc {
                (second, first)
            } else {
                continue;
            };
            let (other_ref, when) = end(&other.component)?;
            connections.push(ConnectionView {
                operation: "connect",
                left: other_ref,
                left_port: other.port.clone(),
                right: reference.clone(),
                right_port: named.port.clone(),
                when,
            });
        }
        for map in configuration
            .model
            .maps
            .iter()
            .filter(|m| m.component == ptc)
        {
            connections.push(ConnectionView {
                operation: "map",
                left: reference.clone(),
                left_port: map.port.clone(),
                right: "system".to_string(),
                right_port: map.system_port.clone(),
                when: None,
            });
        }
        Ok(Statement::Create {
            reference,
            component_type: created.name.clone(),
            ptc,
            connections,
            // The PTC's own behaviour, in the same configuration.
            function: function_name(created, configuration),
            source: self.source("E11", node, line),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_duration_is_the_exact_decimal_of_its_seconds() {
        assert_eq!(seconds(2_000_000), "2.0");
        assert_eq!(seconds(500_000), "0.5");
        assert_eq!(seconds(10_000), "0.01");
        assert_eq!(seconds(100), "0.0001");
        assert_eq!(seconds(1), "0.000001");
        assert_eq!(seconds(1_234_567), "1.234567");
        assert_eq!(seconds(0), "0.0");
    }
}
