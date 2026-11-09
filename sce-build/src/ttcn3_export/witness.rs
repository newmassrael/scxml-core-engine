// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The correspondence witness of a suite's export (ADR 0091 answer 12;
//! docs/SCE_TEST_PROFILE.md §9.4), `<suite>.witness.json`, a wire surface of
//! its own (`schemas/sce-ttcn3-witness.v1.schema.json`).
//!
//! It relates each construct of the suite's documents to the TTCN-3 it
//! became: one entry per construct and rule of §9.2 that translated it, and
//! in each entry every place of the two modules that rule wrote for it. It is
//! the model the modules are written from, serialized: the [`Source`] every
//! definition and statement carries, read where the model holds it, never
//! read again from the text the modules are.
//!
//! A place is a module and a path from the module's top, one segment a step:
//!
//! - a definition of the module, by its kind and its name: `type` and
//!   `external-function` in the types module; `port-type`, `template`,
//!   `modulepar`, `component-type`, `function`, `testcase` and `control`,
//!   which the module's name names, in the behaviour module;
//! - a member of a component type: `port`, `timer`, `var`;
//! - in a function: `local`, a local variable; `label`, the label of an
//!   atomic state, which opens the statements after it up to the next label;
//!   and `statement`, the place of a statement from 0 in the statements the
//!   segment before it opens: a function's own, which run before its first
//!   label, a label's, or an alternative's block;
//! - in a statement: `then` and `else`, a statement of an `if`'s two blocks,
//!   and `alternative`, an alternative of an `alt` from 0 in textual order.
//!
//! The control part opens its `execute`s as `statement`s. So every
//! definition and statement the model holds has exactly one place, and every
//! place is in exactly one entry.
//!
//! An `alt` (E7) and each of its alternatives (E8) also carry what the
//! validator of §9.4 holds against the documents, computing it again on its
//! own: the alt, the atomic state it is in and the transitions its
//! alternatives are, in textual order; an alternative, its atomic state, its
//! transition, and the states its block exits and enters, each in the order
//! the block does so.
//!
//! A document is named by its path relative to the directory of the suite's
//! document, `.` and `..` folded, and a document of the standard library by
//! its `sce:std/` name, so the witness is the same bytes wherever the suite
//! is read from.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use serde::Serialize;

use super::behaviour::{
    Behaviour, ComponentTypeView, Construct, FunctionView, ModuleParameterView, Source, Statement,
};
use super::execution::Execution;
use super::port_types::PortTypeView;
use super::templates::TemplateView;
use super::types::TypesModule;

/// The stability status of the witness's wire schema, pinned to the schema
/// file's own `x-sce-schema-status` by `schema_file_declares_status`.
/// `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
pub const WITNESS_SCHEMA_STATUS: &str = "pre-release";

/// The name of the witness's shape, its `schema`.
pub const WITNESS_SCHEMA: &str = "sce-ttcn3-witness.v1";

/// The correspondence witness of one export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Witness {
    /// [`WITNESS_SCHEMA`].
    pub schema: &'static str,
    /// The suite's unit name, which names its modules.
    pub suite: String,
    /// The two modules, the types module first.
    pub modules: Vec<String>,
    /// One entry per construct and rule, in the order of the construct's
    /// document, its row, its node and the rule.
    pub entries: Vec<Entry>,
}

/// One construct, translated by one rule, and every place that rule wrote
/// for it, in the order of the modules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub construct: Construct,
    /// The rule of docs/SCE_TEST_PROFILE.md §9.2, `E1` to `E15`.
    pub rule: &'static str,
    pub produced: Vec<Location>,
}

/// A place of a module: a definition, a member or a statement of one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    pub module: String,
    pub path: Vec<Segment>,
    /// What an `alt` or an alternative claims about the documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim: Option<Claim>,
}

/// One step of a path, written as an object of one member, its kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Segment {
    /// A type of the types module.
    Type(String),
    /// A codec function of the types module.
    ExternalFunction(String),
    PortType(String),
    Template(String),
    Modulepar(String),
    ComponentType(String),
    Function(String),
    Testcase(String),
    /// The control part, named by its module.
    Control(String),
    /// A port of a component type.
    Port(String),
    /// A timer of a component type.
    Timer(String),
    /// A variable of a component type.
    Var(String),
    /// A local variable of a function.
    Local(String),
    /// The label of an atomic state, which opens the statements after it.
    Label(String),
    /// A statement, from 0, of those the segment before it opens.
    Statement(usize),
    /// A statement, from 0, of an `if`'s first block.
    Then(usize),
    /// A statement, from 0, of an `if`'s `else` block.
    Else(usize),
    /// An alternative of an `alt`, from 0 in textual order, which opens its
    /// block.
    Alternative(usize),
}

/// What a place claims about the documents beyond the rule that wrote it,
/// for the validator to hold against its own computation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Claim {
    /// An `alt` (E7): the atomic state it is in, and the transition each
    /// alternative is, in textual order: the order of P3.
    Alt {
        state: Construct,
        alternatives: Vec<Construct>,
    },
    /// An alternative (E8): the atomic state its `alt` is in, the transition
    /// it is, and the states its block exits and enters, each in the order
    /// the block does so.
    Branch {
        state: Construct,
        transition: Construct,
        exited: Vec<Construct>,
        entered: Vec<Construct>,
    },
}

/// The model of an export, as its modules are written from it.
pub struct Parts<'a> {
    /// The suite's unit name.
    pub suite: &'a str,
    /// The directory of the suite's document, as the label of every
    /// document begins with it.
    pub base: &'a Path,
    pub types: &'a TypesModule,
    pub port_types: &'a [PortTypeView],
    pub templates: &'a [TemplateView],
    /// The module parameters the suite declares.
    pub module_parameters: &'a [ModuleParameterView],
    pub behaviour: &'a Behaviour,
    pub execution: &'a Execution,
}

/// The witness of the export `parts` is the model of.
pub fn witness(parts: &Parts<'_>) -> Witness {
    let types_module = format!("{}_Types", parts.suite);
    let module = parts.suite.to_string();
    let mut entries = Entries::new(parts.base);

    let mut types = entries.module(&types_module);
    for ty in &parts.types.types {
        types.definition(ty.source(), Segment::Type(ty.name().to_string()));
    }
    for codec in &parts.types.codec {
        types.definition(
            &codec.source,
            Segment::ExternalFunction(codec.encode.clone()),
        );
        types.definition(
            &codec.source,
            Segment::ExternalFunction(codec.decode.clone()),
        );
    }

    let mut behaviour = entries.module(&module);
    for port_type in parts.port_types {
        behaviour.definition(&port_type.source, Segment::PortType(port_type.name.clone()));
    }
    for template in parts.templates {
        behaviour.definition(&template.source, Segment::Template(template.name.clone()));
    }
    for parameter in parts
        .module_parameters
        .iter()
        .chain(&parts.execution.features)
    {
        behaviour.definition(
            &parameter.source,
            Segment::Modulepar(parameter.name.clone()),
        );
    }
    for component in parts
        .behaviour
        .component_types
        .iter()
        .chain(&parts.execution.system_types)
    {
        behaviour.component_type(component);
    }
    for function in &parts.behaviour.functions {
        behaviour.function(function);
    }
    for case in &parts.execution.test_cases {
        behaviour.definition(&case.source, Segment::Testcase(case.name.clone()));
    }
    let control = vec![Segment::Control(module.clone())];
    behaviour.place(&parts.execution.control.source, control.clone(), None);
    for (index, execute) in parts.execution.control.executes.iter().enumerate() {
        behaviour.place(
            &execute.source,
            extended(&control, Segment::Statement(index)),
            None,
        );
    }

    Witness {
        schema: WITNESS_SCHEMA,
        suite: parts.suite.to_string(),
        modules: vec![types_module, module],
        entries: entries.finish(),
    }
}

/// The witness's text: `witness` as JSON, two spaces an indentation level,
/// every object's members in the order the model declares them.
pub fn witness_text(witness: &Witness) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(witness)
}

/// The name the witness gives the document labelled `label`, the suite's
/// document being in `base`: its `sce:std/` name where it is a document of
/// the standard library, and otherwise its path relative to `base`, `.` and
/// `..` folded, each separator a `/`.
pub fn document_name(label: &str, base: &Path) -> String {
    let path = Path::new(label);
    if crate::forge::stdlib::names_standard(path) {
        return crate::forge::stdlib::identity(path);
    }
    folded(path.strip_prefix(base).unwrap_or(path))
}

/// `path` with each `.` dropped and each `..` folded into the segment before
/// it, where there is one to fold it into.
fn folded(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut rooted = false;
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.last().is_some_and(|last| last != "..") {
                    parts.pop();
                } else if !rooted {
                    parts.push("..".to_string());
                }
            }
            Component::RootDir => rooted = true,
            Component::Prefix(prefix) => {
                parts.push(prefix.as_os_str().to_string_lossy().into_owned())
            }
        }
    }
    let joined = parts.join("/");
    if rooted {
        format!("/{joined}")
    } else {
        joined
    }
}

/// `path` with `segment` after it.
fn extended(path: &[Segment], segment: Segment) -> Vec<Segment> {
    let mut path = path.to_vec();
    path.push(segment);
    path
}

/// An entry's key: the construct's document, its row and its node, and the
/// rule, in the order the entries are written.
type Key = (String, Option<u32>, String, &'static str);

/// The entries as they are collected, each its places in the order they are
/// met, which is the order of the modules.
struct Entries<'b> {
    base: &'b Path,
    entries: BTreeMap<Key, Vec<Location>>,
}

impl<'b> Entries<'b> {
    fn new(base: &'b Path) -> Self {
        Self {
            base,
            entries: BTreeMap::new(),
        }
    }

    /// The places of `module`, as they are collected.
    fn module(&mut self, module: &str) -> ModuleEntries<'_, 'b> {
        ModuleEntries {
            module: module.to_string(),
            entries: self,
        }
    }

    /// `construct` as the witness names it.
    fn construct(&self, construct: &Construct) -> Construct {
        Construct {
            document: document_name(&construct.document, self.base),
            node: construct.node.clone(),
            line: construct.line,
        }
    }

    fn add(&mut self, source: &Source, location: Location) {
        let key = (
            document_name(&source.document, self.base),
            source.line,
            source.node.clone(),
            source.rule,
        );
        self.entries.entry(key).or_default().push(location);
    }

    fn finish(self) -> Vec<Entry> {
        self.entries
            .into_iter()
            .map(|((document, line, node, rule), produced)| Entry {
                construct: Construct {
                    document,
                    node,
                    line,
                },
                rule,
                produced,
            })
            .collect()
    }
}

/// The entries, collecting the places of one module.
struct ModuleEntries<'e, 'b> {
    module: String,
    entries: &'e mut Entries<'b>,
}

impl ModuleEntries<'_, '_> {
    /// The place `path`, written by `source`, with what it claims.
    fn place(&mut self, source: &Source, path: Vec<Segment>, claim: Option<Claim>) {
        let location = Location {
            module: self.module.clone(),
            path,
            claim,
        };
        self.entries.add(source, location);
    }

    fn definition(&mut self, source: &Source, segment: Segment) {
        self.place(source, vec![segment], None);
    }

    fn component_type(&mut self, component: &ComponentTypeView) {
        let at = vec![Segment::ComponentType(component.name.clone())];
        self.place(&component.source, at.clone(), None);
        for port in &component.ports {
            self.place(
                &port.source,
                extended(&at, Segment::Port(port.name.clone())),
                None,
            );
        }
        for timer in &component.timers {
            self.place(
                &timer.source,
                extended(&at, Segment::Timer(timer.name.clone())),
                None,
            );
        }
        for variable in &component.variables {
            self.place(
                &variable.source,
                extended(&at, Segment::Var(variable.name.clone())),
                None,
            );
        }
    }

    /// A function, its locals, and its body: the statements before its
    /// first label as its own, and each label opening those after it.
    fn function(&mut self, function: &FunctionView) {
        let at = vec![Segment::Function(function.name.clone())];
        self.place(&function.source, at.clone(), None);
        for local in &function.locals {
            self.place(
                &local.source,
                extended(&at, Segment::Local(local.name.clone())),
                None,
            );
        }
        let mut opened = at.clone();
        let mut index = 0;
        for statement in &function.body {
            if let Statement::Label { name, source } = statement {
                opened = extended(&at, Segment::Label(name.clone()));
                self.place(source, opened.clone(), None);
                index = 0;
                continue;
            }
            self.statement(extended(&opened, Segment::Statement(index)), statement);
            index += 1;
        }
    }

    /// The statement at `path`, and every statement it holds.
    fn statement(&mut self, path: Vec<Segment>, statement: &Statement) {
        match statement {
            Statement::If {
                then,
                otherwise,
                source,
                ..
            } => {
                self.place(source, path.clone(), None);
                for (index, inner) in then.iter().enumerate() {
                    self.statement(extended(&path, Segment::Then(index)), inner);
                }
                for (index, inner) in otherwise.iter().enumerate() {
                    self.statement(extended(&path, Segment::Else(index)), inner);
                }
            }
            Statement::Alt { branches, source } => {
                let state = self.entries.construct(&source.construct());
                let alternatives = branches
                    .iter()
                    .map(|branch| self.entries.construct(&branch.source.construct()))
                    .collect();
                self.place(
                    source,
                    path.clone(),
                    Some(Claim::Alt {
                        state: state.clone(),
                        alternatives,
                    }),
                );
                for (position, branch) in branches.iter().enumerate() {
                    let alternative = extended(&path, Segment::Alternative(position));
                    let claim = Claim::Branch {
                        state: state.clone(),
                        transition: self.entries.construct(&branch.source.construct()),
                        exited: branch
                            .exited
                            .iter()
                            .map(|c| self.entries.construct(c))
                            .collect(),
                        entered: branch
                            .entered
                            .iter()
                            .map(|c| self.entries.construct(c))
                            .collect(),
                    };
                    self.place(&branch.source, alternative.clone(), Some(claim));
                    for (index, inner) in branch.block.iter().enumerate() {
                        self.statement(extended(&alternative, Segment::Statement(index)), inner);
                    }
                }
            }
            other => self.place(other.source(), path, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ttcn3_export::behaviour::{Branch, BranchEvent, LocalView, PortView, TimerView};
    use crate::ttcn3_export::execution::{ControlView, ExecuteView, TestCaseView};
    use crate::ttcn3_export::types::{Codec, TypeDef, TypeExpr, MANIFEST_VERSION};

    const SCHEMA: &str = include_str!("../../../schemas/sce-ttcn3-witness.v1.schema.json");

    #[test]
    fn schema_file_declares_status() {
        let schema: serde_json::Value =
            serde_json::from_str(SCHEMA).expect("the witness's schema is JSON");
        assert_eq!(
            schema["x-sce-schema-status"].as_str(),
            Some(WITNESS_SCHEMA_STATUS),
            "schemas/sce-ttcn3-witness.v1.schema.json x-sce-schema-status disagrees with \
             WITNESS_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
        assert_eq!(
            schema["properties"]["schema"]["const"].as_str(),
            Some(WITNESS_SCHEMA),
            "the schema pins the name of the shape the producer writes",
        );
    }

    /// A document is named relative to the suite's directory, `.` and `..`
    /// folded, whether the suite was read by an absolute path or a relative
    /// one; a standard document by its `sce:std/` name.
    #[test]
    fn a_document_is_named_relative_to_the_suites_directory() {
        let base = Path::new("/work/suites");
        assert_eq!(
            document_name("/work/suites/tester.scxml", base),
            "tester.scxml"
        );
        assert_eq!(
            document_name("/work/suites/parts/../tester.scxml", base),
            "tester.scxml"
        );
        assert_eq!(
            document_name("/work/suites/../common/./status.scxml", base),
            "../common/status.scxml"
        );
        assert_eq!(
            document_name("suites/parts/port.scxml", Path::new("suites")),
            "parts/port.scxml"
        );
        assert_eq!(document_name("./port.scxml", Path::new(".")), "port.scxml");
        assert_eq!(document_name("port.scxml", Path::new("")), "port.scxml");
        assert_eq!(
            document_name("sce:std/test/./verdict.scxml", base),
            "sce:std/test/verdict.scxml"
        );
    }

    fn source(rule: &'static str, document: &str, node: &str, line: Option<u32>) -> Source {
        Source {
            rule,
            document: document.to_string(),
            node: node.to_string(),
            line,
        }
    }

    fn construct(node: &str, line: u32) -> Construct {
        Construct {
            document: "c.scxml".to_string(),
            node: node.to_string(),
            line: Some(line),
        }
    }

    fn goto(label: &str, transition: &str, line: Option<u32>) -> Statement {
        Statement::Goto {
            label: label.to_string(),
            source: source("E8", "/s/c.scxml", transition, line),
        }
    }

    fn timeout(timer: &str, transition: &str, line: u32, target: (&str, u32)) -> Branch {
        Branch {
            guard: None,
            event: BranchEvent::Timeout {
                timer: timer.to_string(),
            },
            block: vec![goto(target.0, transition, Some(line))],
            exited: vec![construct("state 'a'", 3)],
            entered: vec![construct(&format!("state '{}'", target.0), target.1)],
            source: source("E8", "/s/c.scxml", transition, Some(line)),
        }
    }

    /// A model of one suite written by hand: a types module of one integer
    /// type and one record with its codec; a component with a port, a timer
    /// and a local; a function whose body enters `a`, and whose atomic state
    /// `a` waits in an alt of two alternatives, one of its own and one of
    /// its parent `p`; a test case and a control part of one execute.
    fn model() -> (TypesModule, Behaviour, Execution) {
        let suite = |rule| source(rule, "/s/suite.scxml", "scxml", Some(1));
        let types = TypesModule {
            v: MANIFEST_VERSION,
            suite: "suite".to_string(),
            module: "suite_Types".to_string(),
            types: vec![
                TypeDef::Integer {
                    name: "uint8".to_string(),
                    min: "0".to_string(),
                    max: "255".to_string(),
                    source: suite("E1"),
                },
                TypeDef::Record {
                    name: "status".to_string(),
                    event: "status".to_string(),
                    fields: Vec::new(),
                    source: source("E1", "/s/status.scxml", "scxml", Some(2)),
                },
            ],
            codec: vec![Codec {
                ty: "status".to_string(),
                encode: "enc_status".to_string(),
                decode: "dec_status".to_string(),
                source: source("E1", "/s/status.scxml", "scxml", Some(2)),
            }],
        };
        let component = ComponentTypeView {
            name: "c".to_string(),
            ports: vec![PortView {
                name: "p".to_string(),
                port_type: "pt".to_string(),
                source: source("E4", "/s/c.scxml", "sce:test-port 'p'", Some(4)),
            }],
            timers: vec![TimerView {
                name: "t".to_string(),
                duration: None,
                source: source("E4", "/s/c.scxml", "sce:test-timer 't'", Some(5)),
            }],
            variables: Vec::new(),
            source: source("E4", "/s/c.scxml", "scxml", None),
        };
        let function = FunctionView {
            name: "f_c".to_string(),
            component: "c".to_string(),
            configuration: None,
            locals: vec![LocalView {
                name: "v_job".to_string(),
                ty: TypeExpr::Ref {
                    name: "job".to_string(),
                },
                starts_null: false,
                source: source("E8", "/s/c.scxml", "received job", Some(7)),
            }],
            // The machine's start, the document's own transition, then the
            // atomic state `a`.
            body: vec![
                goto("a", "scxml", None),
                Statement::Label {
                    name: "a".to_string(),
                    source: source("E6", "/s/c.scxml", "state 'a'", Some(3)),
                },
                Statement::If {
                    condition: "true".to_string(),
                    then: vec![goto("b", "state 'a' / transition 1", Some(8))],
                    otherwise: Vec::new(),
                    source: source("E6", "/s/c.scxml", "state 'a' / transition 1", Some(8)),
                },
                Statement::Alt {
                    branches: vec![
                        timeout("t", "state 'a' / transition 2", 9, ("b", 11)),
                        timeout("u", "state 'p' / transition 1", 10, ("b", 11)),
                    ],
                    source: source("E7", "/s/c.scxml", "state 'a'", Some(3)),
                },
            ],
            source: source("E5", "/s/c.scxml", "scxml", None),
        };
        let behaviour = Behaviour {
            component_types: vec![component],
            functions: vec![function],
        };
        let execution = Execution {
            features: Vec::new(),
            system_types: Vec::new(),
            test_cases: vec![TestCaseView {
                name: "t1".to_string(),
                component: "c".to_string(),
                system: "sys".to_string(),
                maps: Vec::new(),
                function: "f_c".to_string(),
                source: source("E14", "/s/t1.scxml", "scxml", Some(1)),
            }],
            control: ControlView {
                executes: vec![ExecuteView {
                    test_case: "t1".to_string(),
                    guard: None,
                    condition: None,
                    source: source("E15", "/s/suite.scxml", "sce:execute 'T1'", Some(6)),
                }],
                source: suite("E15"),
            },
        };
        (types, behaviour, execution)
    }

    fn witness_of(types: &TypesModule, behaviour: &Behaviour, execution: &Execution) -> Witness {
        witness(&Parts {
            suite: "suite",
            base: Path::new("/s"),
            types,
            port_types: &[],
            templates: &[],
            module_parameters: &[],
            behaviour,
            execution,
        })
    }

    fn path(location: &Location) -> String {
        serde_json::to_string(&location.path).expect("a path is JSON")
    }

    /// Every definition, member and statement of the model has one place,
    /// no place is named twice, and every construct and rule has one entry,
    /// in the order of the document, the row, the node and the rule.
    #[test]
    fn every_place_of_the_model_is_named_once() {
        let (types, behaviour, execution) = model();
        let witness = witness_of(&types, &behaviour, &execution);
        assert_eq!(witness.schema, WITNESS_SCHEMA);
        assert_eq!(witness.modules, ["suite_Types", "suite"]);

        let places: Vec<String> = witness
            .entries
            .iter()
            .flat_map(|entry| {
                entry
                    .produced
                    .iter()
                    .map(|l| format!("{} {}", l.module, path(l)))
            })
            .collect();
        // The types module: two types and two codec functions. The
        // behaviour module: the component type, its port and timer; the
        // function, its local, the entry's goto, the label, the if and its
        // goto, the alt, its two alternatives and their gotos; the test
        // case; the control part and its execute.
        assert_eq!(places.len(), 4 + 3 + 11 + 1 + 2, "{places:#?}");
        let mut unique = places.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            unique.len(),
            places.len(),
            "a place named twice: {places:#?}"
        );

        let keys: Vec<(&str, Option<u32>, &str, &str)> = witness
            .entries
            .iter()
            .map(|e| {
                (
                    e.construct.document.as_str(),
                    e.construct.line,
                    e.construct.node.as_str(),
                    e.rule,
                )
            })
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(keys, sorted, "entries out of order or one twice");
        assert!(keys.iter().all(|(document, ..)| !document.starts_with('/')));
    }

    /// The record and its codec functions are one entry, the record's E1,
    /// its places in the order of the types module.
    #[test]
    fn a_codec_is_written_by_its_types_rule() {
        let (types, behaviour, execution) = model();
        let witness = witness_of(&types, &behaviour, &execution);
        let status = witness
            .entries
            .iter()
            .find(|e| e.construct.document == "status.scxml")
            .expect("the record's entry");
        assert_eq!(status.rule, "E1");
        let places: Vec<String> = status.produced.iter().map(path).collect();
        assert_eq!(
            places,
            [
                r#"[{"type":"status"}]"#,
                r#"[{"external-function":"enc_status"}]"#,
                r#"[{"external-function":"dec_status"}]"#,
            ]
        );
    }

    /// The statements before the first label are the function's own, a
    /// label opens those after it, an `if` its blocks and an alternative
    /// its block; the alt claims its state and the transitions of its
    /// alternatives in order, each alternative its state, its transition and
    /// the states its block exits and enters.
    #[test]
    fn a_functions_places_follow_its_labels_and_blocks() {
        let (types, behaviour, execution) = model();
        let witness = witness_of(&types, &behaviour, &execution);
        let entry = |node: &str, rule: &str| {
            witness
                .entries
                .iter()
                .find(|e| e.construct.node == node && e.rule == rule)
                .unwrap_or_else(|| panic!("no entry ({node}, {rule})"))
        };
        let places = |node: &str, rule: &str| -> Vec<String> {
            entry(node, rule).produced.iter().map(path).collect()
        };
        assert_eq!(
            places("scxml", "E8"),
            [r#"[{"function":"f_c"},{"statement":0}]"#]
        );
        assert_eq!(
            places("state 'a'", "E6"),
            [r#"[{"function":"f_c"},{"label":"a"}]"#]
        );
        assert_eq!(
            places("state 'a' / transition 1", "E6"),
            [r#"[{"function":"f_c"},{"label":"a"},{"statement":0}]"#]
        );
        assert_eq!(
            places("state 'a' / transition 1", "E8"),
            [r#"[{"function":"f_c"},{"label":"a"},{"statement":0},{"then":0}]"#]
        );
        assert_eq!(
            places("state 'p' / transition 1", "E8"),
            [
                r#"[{"function":"f_c"},{"label":"a"},{"statement":1},{"alternative":1}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":1},{"alternative":1},{"statement":0}]"#,
            ]
        );
        assert_eq!(
            places("received job", "E8"),
            [r#"[{"function":"f_c"},{"local":"v_job"}]"#]
        );
        assert_eq!(
            places("sce:execute 'T1'", "E15"),
            [r#"[{"control":"suite"},{"statement":0}]"#]
        );

        let alt = &entry("state 'a'", "E7").produced[0];
        assert_eq!(
            alt.claim,
            Some(Claim::Alt {
                state: construct("state 'a'", 3),
                alternatives: vec![
                    construct("state 'a' / transition 2", 9),
                    construct("state 'p' / transition 1", 10),
                ],
            })
        );
        let alternative = &entry("state 'p' / transition 1", "E8").produced;
        assert_eq!(
            alternative[0].claim,
            Some(Claim::Branch {
                state: construct("state 'a'", 3),
                transition: construct("state 'p' / transition 1", 10),
                exited: vec![construct("state 'a'", 3)],
                entered: vec![construct("state 'b'", 11)],
            })
        );
        assert_eq!(alternative[1].claim, None, "a goto claims nothing");
    }

    /// A segment is an object of one member, its kind, and a claim is
    /// tagged by its kind; a place that claims nothing writes no claim.
    #[test]
    fn a_witness_is_written_in_its_wire_shape() {
        let (types, behaviour, execution) = model();
        let text = witness_text(&witness_of(&types, &behaviour, &execution)).expect("JSON");
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(value["schema"], WITNESS_SCHEMA);
        let entry = value["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|e| e["rule"] == "E7")
            .expect("the alt's entry");
        assert_eq!(
            entry["construct"],
            serde_json::json!({"document": "c.scxml", "node": "state 'a'", "line": 3})
        );
        assert_eq!(entry["produced"][0]["claim"]["kind"], "alt");
        assert_eq!(
            entry["produced"][0]["path"],
            serde_json::json!([{"function": "f_c"}, {"label": "a"}, {"statement": 1}])
        );
        let function = value["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|e| e["rule"] == "E5")
            .expect("the function's entry");
        assert!(function["produced"][0].get("claim").is_none());
        assert_eq!(function["construct"]["line"], serde_json::Value::Null);
    }
}
