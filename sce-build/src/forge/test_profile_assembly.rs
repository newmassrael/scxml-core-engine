// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How the documents of a test fit together: the rules of a port type, a
//! configuration, a test case and a suite that need the documents they import
//! (docs/adr/0091; docs/SCE_TEST_PROFILE.md §11.3, §11.6, §11.7, §11.8).
//!
//! The parser holds each of these documents to every rule it states alone
//! ([`crate::forge::parser`]). What is left is asked here, where the imports
//! are read:
//!
//! * the message types of a port type carry distinct event names (§11.3);
//! * a configuration's components are test components and its system ports
//!   are of port types; each `connect` and `map` joins ports the components
//!   declare, whose message lists agree (-1 §21.1.1 b, c); every port takes
//!   part in exactly one; and its PTCs are created by its components'
//!   invocations, each by one, as a document of its type, in a tree rooted at
//!   the MTC (§11.6);
//! * a test case runs in a test configuration (§11.7);
//! * a suite selects its test cases by features of its feature model, and
//!   declares, with its type, every module parameter a component it reaches
//!   reads (§11.8).
//!
//! A document a test reaches is held to these rules where it is reached: a
//! suite's test cases, their configurations and the port types those read
//! are checked with the suite, and each refusal is placed in the document
//! that breaks the rule. Unlike the readers that run after the import
//! enrichment has spoken ([`crate::forge::import_source::parse_quietly`]),
//! this one is the only reader of a document an import of an import names,
//! so a document it cannot read, or of another kind than its import
//! declares, is refused here, at the import that names it.
//!
//! A test component is a statechart (§11.5). It is read by the statechart
//! parser ([`ImportSource::parse_test_component`]), and its ports, the PTCs it
//! invokes and the module parameters it reads are the record that parser
//! keeps of it ([`crate::test_component`]).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::forge::error::{
    ForgeError, ImportError, Located, SourceLocation, TestProfileRule, ValidationError,
};
use crate::forge::import_source::ImportSource;
use crate::forge::model::{
    FeaturesModel, ForgeDocument, ForgeImport, ForgeKind, ParsedForge, SceType, TestCaseModel,
    TestConfigurationModel, TestExecute, TestPortTypeModel, TestSuiteModel,
};
use crate::forge::stdlib;
use crate::forge::value_type::{TypeOf, ValueType};
use crate::model::{SCXMLModel, Variable};
use crate::presence_condition::PresenceCondition;
use crate::test_component::{TestComponentProfile, TestInvocation};

/// Refuse a port type, a configuration, a test case or a suite that breaks a
/// rule of docs/SCE_TEST_PROFILE.md §11.3, §11.6, §11.7 or §11.8 that its
/// imports decide. `base_dir` is the directory its imports resolve against,
/// and `document` the label its refusals carry. A document of any other kind
/// has none of these rules.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    let site = Site {
        imports: &parsed.imports,
        dir: base_dir,
        label: document,
    };
    match &parsed.document {
        ForgeDocument::TestPortType(port_type) => check_port_type(port_type, &site).map(|_| ()),
        ForgeDocument::TestConfiguration(configuration) => {
            check_configuration(configuration, &site).map(|_| ())
        }
        ForgeDocument::TestCase(case) => check_test_case(case, &site).map(|_| ()),
        ForgeDocument::TestSuite(suite) => check_suite(suite, &site),
        _ => Ok(()),
    }
}

// ── Reading what a test imports ────────────────────────────────

/// Where a document of a test is read: the imports that name its documents,
/// the directory they resolve against, and the label its refusals carry.
#[derive(Clone, Copy)]
struct Site<'a> {
    imports: &'a [ForgeImport],
    dir: &'a Path,
    label: &'a str,
}

impl<'a> Site<'a> {
    /// `error`, placed in this document on `line`.
    fn refuse(&self, error: impl Into<ForgeError>, line: Option<u32>) -> Located<ForgeError> {
        Located::new(error.into(), self.label, line, None)
    }

    /// The refusal of a rule of the test profile that the document `document`
    /// of `kind` breaks on `line`: `actual` is what the document wrote there,
    /// a token of that line (SCE_ERROR_CONTRACT.md §3.1.1). It is empty where
    /// the rule is about an element as a whole, and where what breaks it is
    /// not written in this document: a value read from another document, a
    /// count, a declaration that is missing. The message names those.
    fn profile_refusal(
        &self,
        line: Option<u32>,
        rule: TestProfileRule,
        kind: ForgeKind,
        document: &str,
        actual: &str,
        detail: String,
    ) -> Located<ForgeError> {
        self.refuse(
            ValidationError::TestProfile {
                rule,
                kind,
                document: document.to_string(),
                actual: actual.to_string(),
                detail,
            },
            line,
        )
    }

    /// The import of `kind` this document names `alias` by, which the `attr`
    /// of `element` on `line` writes; refused with the aliases this document
    /// imports of that kind as the candidates when there is none, as the
    /// parser refuses an alias.
    fn import(
        &self,
        alias: &str,
        kind: ForgeKind,
        element: &str,
        attr: &str,
        line: Option<u32>,
    ) -> Result<&'a ForgeImport, Located<ForgeError>> {
        let imports = self.imports;
        imports
            .iter()
            .find(|import| import.alias == alias && import.kind == kind)
            .ok_or_else(|| {
                self.refuse(
                    ValidationError::InvalidAttribute {
                        element: element.to_string(),
                        attr: attr.to_string(),
                        value: alias.to_string(),
                        allowed: imports
                            .iter()
                            .filter(|import| import.kind == kind)
                            .map(|import| import.alias.clone())
                            .collect(),
                    },
                    line,
                )
            })
    }

    /// The one spelling of the document `import` names, whatever import
    /// reaches it ([`stdlib::identity`]).
    fn identity_of(&self, import: &ForgeImport) -> String {
        stdlib::identity(&stdlib::resolve(self.dir, &import.src))
    }

    /// The document `import` names, read; refused at the import when it
    /// cannot be.
    fn source(&self, import: &ForgeImport) -> Result<ImportSource, Located<ForgeError>> {
        ImportSource::read(self.dir, import).map_err(|error| self.refuse(error, import.line))
    }

    /// The forge document `import` names, parsed, and taken by `take` when it
    /// is of the kind `take` takes. A defect of the document is refused in
    /// the document, in its own words; a document of another kind than the
    /// import declares is refused at the import.
    fn open<D>(
        &self,
        import: &ForgeImport,
        take: impl FnOnce(ForgeDocument) -> Option<D>,
    ) -> Result<Opened<D>, Located<ForgeError>> {
        let source = self.source(import)?;
        let Some(parsed) = source.parse()? else {
            return Err(self.refuse(other_kind(import, &source), import.line));
        };
        let actual = parsed.document.kind();
        let Some(document) = take(parsed.document) else {
            return Err(self.refuse(
                ImportError::KindMismatch {
                    src: import.src.clone(),
                    declared: import.kind.to_string(),
                    actual: actual.to_string(),
                },
                import.line,
            ));
        };
        Ok(Opened {
            document,
            imports: parsed.imports,
            at: Place::of(&source),
        })
    }

    /// The test component `import` names, read by the statechart parser. A
    /// defect of the component is refused in the component; a document that
    /// is no test component is refused at the import.
    fn open_component(&self, import: &ForgeImport) -> Result<Component, Located<ForgeError>> {
        let source = self.source(import)?;
        let Some(mut model) = source.parse_test_component()? else {
            return Err(self.refuse(other_kind(import, &source), import.line));
        };
        // The parser records a profile on every test component and on no
        // other statechart, so a model without one was read as a statechart.
        let Some(profile) = model.test_component.take() else {
            return Err(self.refuse(
                ImportError::KindMismatch {
                    src: import.src.clone(),
                    declared: import.kind.to_string(),
                    actual: ForgeKind::Statechart.to_string(),
                },
                import.line,
            ));
        };
        Ok(Component {
            model,
            profile,
            at: Place::of(&source),
        })
    }
}

/// Why the document `source` is not the document `import` declares: it is of
/// another kind, or it is no forge document at all.
fn other_kind(import: &ForgeImport, source: &ImportSource) -> ImportError {
    let actual = match crate::forge::parser::detect_kind(&source.content) {
        Ok(Some(kind)) => Some(kind),
        // A document with no `sce:kind` is a statechart, and so is a test
        // component: an import of one names another kind of statechart.
        Ok(None) if import.kind.reads_as_statechart() => Some(ForgeKind::Statechart),
        Ok(None) | Err(_) => None,
    };
    match actual {
        Some(actual) => ImportError::KindMismatch {
            src: import.src.clone(),
            declared: import.kind.to_string(),
            actual: actual.to_string(),
        },
        None => ImportError::NotForge {
            src: import.src.clone(),
        },
    }
}

/// Where an imported document is: the directory its own imports resolve
/// against, the label its refusals carry, and its one spelling
/// ([`stdlib::identity`]).
struct Place {
    dir: PathBuf,
    label: String,
    identity: String,
}

impl Place {
    fn of(source: &ImportSource) -> Self {
        Self {
            dir: source
                .path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
            label: source.label().diagnostic_label.to_string(),
            identity: stdlib::identity(&source.path),
        }
    }

    /// Where `at`, a position in this document, is, as a refusal of another
    /// document names it.
    fn spell(&self, at: Option<&SourceLocation>) -> String {
        match line(at) {
            Some(line) => format!("{}:{line}", self.label),
            None => self.label.clone(),
        }
    }
}

/// A forge document a test imports, read.
struct Opened<D> {
    document: D,
    imports: Vec<ForgeImport>,
    at: Place,
}

impl<D> Opened<D> {
    fn site(&self) -> Site<'_> {
        Site {
            imports: &self.imports,
            dir: &self.at.dir,
            label: &self.at.label,
        }
    }
}

/// A test component a configuration names, read by the statechart parser.
struct Component {
    model: SCXMLModel,
    /// What the component declares of the test profile.
    profile: TestComponentProfile,
    at: Place,
}

impl Component {
    fn site(&self) -> Site<'_> {
        Site {
            imports: &self.model.forge_imports,
            dir: &self.at.dir,
            label: &self.at.label,
        }
    }

    /// The `<data>` the component declares as `id`, at any depth.
    fn variable(&self, id: &str) -> Option<&Variable> {
        self.model
            .variables
            .iter()
            .chain(
                self.model
                    .states
                    .values()
                    .flat_map(|state| state.datamodel.iter()),
            )
            .find(|variable| variable.id == id)
    }
}

/// The line a position is on.
fn line(at: Option<&SourceLocation>) -> Option<u32> {
    at.and_then(|at| at.line)
}

/// `names`, each quoted, separated by commas.
fn quoted(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("'{name}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

// ── §11.3: the port type ───────────────────────────────────────

/// The message types on a port type's `in` and `out` lists (§11.3): each keyed
/// by the event schema document it is ([`stdlib::identity`]), so two port
/// types that import one schema under two aliases list one type, and spelled
/// as the port type's import names it.
#[derive(Debug, Clone, Default)]
struct PortLists {
    ins: BTreeMap<String, String>,
    outs: BTreeMap<String, String>,
}

/// The message types of `from` that `into` does not hold, as `from` spells
/// them.
fn missing_from<'l>(
    from: &'l BTreeMap<String, String>,
    into: &BTreeMap<String, String>,
) -> Vec<&'l str> {
    from.iter()
        .filter(|(identity, _)| !into.contains_key(*identity))
        .map(|(_, alias)| alias.as_str())
        .collect()
}

/// §11.3: refuse a port type two of whose message types carry one event name,
/// since a message delivered to a port is told apart from the others by its
/// event name (`test-profile/message-event-name`); and give its lists.
fn check_port_type(
    port_type: &TestPortTypeModel,
    site: &Site<'_>,
) -> Result<PortLists, Located<ForgeError>> {
    let mut lists = PortLists::default();
    let mut by_event: BTreeMap<String, &str> = BTreeMap::new();
    for message in &port_type.messages {
        let import = site.import(
            &message.schema,
            ForgeKind::EventSchema,
            "<sce:message>",
            "type",
            message.source_line,
        )?;
        let schema = site.open(import, |document| match document {
            ForgeDocument::EventSchema(schema) => Some(schema),
            _ => None,
        })?;
        let event = schema.document.event_name.as_str();
        if let Some(first) = by_event.insert(event.to_string(), message.schema.as_str()) {
            // The event name is the schema's, written in its document; what
            // this line writes is the message type that carries it.
            return Err(site.profile_refusal(
                message.source_line,
                TestProfileRule::MessageEventName,
                ForgeKind::TestPortType,
                &port_type.name,
                &message.schema,
                format!(
                    "the message types '{first}' and '{}' both carry the event name '{event}': \
                     a message delivered to a port is told apart from the others by its event \
                     name, so the message types of a port type carry distinct ones",
                    message.schema
                ),
            ));
        }
        let identity = schema.at.identity;
        if message.direction.is_in() {
            lists.ins.insert(identity.clone(), message.schema.clone());
        }
        if message.direction.is_out() {
            lists.outs.insert(identity, message.schema.clone());
        }
    }
    Ok(lists)
}

/// The lists of the port type `site` imports as `alias`, which the `type` of
/// `element` on `line` names, the port type held to §11.3.
fn port_type_lists(
    site: &Site<'_>,
    alias: &str,
    element: &str,
    line: Option<u32>,
) -> Result<PortLists, Located<ForgeError>> {
    let import = site.import(alias, ForgeKind::TestPortType, element, "type", line)?;
    let port_type = site.open(import, |document| match document {
        ForgeDocument::TestPortType(port_type) => Some(port_type),
        _ => None,
    })?;
    check_port_type(&port_type.document, &port_type.site())
}

// ── §11.6: the configuration ───────────────────────────────────

/// One component of a configuration: the MTC or a PTC, by the name its
/// `connect`s, `map`s and invocations use, and the alias of its type.
struct Instance<'m> {
    name: &'m str,
    alias: &'m str,
    /// The line of its declaration: the `<sce:mtc>` or the `<sce:ptc>`.
    line: Option<u32>,
    element: &'static str,
}

/// A component type of a configuration, read, with the lists of each port it
/// declares, by the port's id.
struct ComponentType {
    component: Component,
    ports: BTreeMap<String, PortLists>,
}

/// The components of a configuration and their types, each type read once.
struct Components<'m> {
    instances: Vec<Instance<'m>>,
    types: BTreeMap<&'m str, ComponentType>,
}

impl Components<'_> {
    /// The type of the component the configuration names `component`.
    fn type_of(&self, component: &str) -> Option<&ComponentType> {
        let instance = self.instances.iter().find(|i| i.name == component)?;
        self.types.get(instance.alias)
    }

    /// The lists of the port `port` of the component `component`, which the
    /// `<sce:{element}>` on `line` joins; refused with the ports the
    /// component's type declares as the candidates when it declares no such
    /// port.
    fn port(
        &self,
        site: &Site<'_>,
        component: &str,
        port: &str,
        element: &str,
        line: Option<u32>,
    ) -> Result<&PortLists, Located<ForgeError>> {
        let declared = self.type_of(component);
        declared.and_then(|ty| ty.ports.get(port)).ok_or_else(|| {
            site.refuse(
                ValidationError::InvalidAttribute {
                    element: format!("<sce:{element} component=\"{component}\">"),
                    attr: "port".into(),
                    value: port.to_string(),
                    allowed: declared
                        .map(|ty| {
                            ty.component
                                .profile
                                .ports
                                .iter()
                                .map(|p| p.id.clone())
                                .collect()
                        })
                        .unwrap_or_default(),
                },
                line,
            )
        })
    }
}

/// The lists of each port `component` declares, by the port's id, each port's
/// type held to §11.3.
fn component_ports(
    component: &Component,
) -> Result<BTreeMap<String, PortLists>, Located<ForgeError>> {
    let site = component.site();
    let mut ports = BTreeMap::new();
    for port in &component.profile.ports {
        let lists = port_type_lists(
            &site,
            &port.port_type,
            "<sce:test-port>",
            line(port.source_location.as_ref()),
        )?;
        ports.insert(port.id.clone(), lists);
    }
    Ok(ports)
}

/// §11.6: refuse a configuration whose components are no test components or
/// whose system ports are of no port types (`import/kind-mismatch`); a
/// `connect` or `map` naming a port the component's type does not declare
/// (`validation/invalid-attribute`); a `connect` or `map` whose ports' lists
/// do not agree (`test-profile/port-lists`); a component port no `connect` or
/// `map` joins (`test-profile/port-connection`); and PTCs its components'
/// invocations do not create as the configuration declares them
/// (`test-profile/ptc-invocation`). Give the components' types, read.
fn check_configuration(
    configuration: &TestConfigurationModel,
    site: &Site<'_>,
) -> Result<Vec<Component>, Located<ForgeError>> {
    let kind = ForgeKind::TestConfiguration;
    let name = configuration.name.as_str();
    let instances: Vec<Instance<'_>> = std::iter::once(Instance {
        name: TestConfigurationModel::MTC,
        alias: &configuration.mtc,
        line: configuration.mtc_source_line,
        element: "<sce:mtc>",
    })
    .chain(configuration.ptcs.iter().map(|ptc| Instance {
        name: &ptc.id,
        alias: &ptc.component,
        line: ptc.source_line,
        element: "<sce:ptc>",
    }))
    .collect();
    let mut types: BTreeMap<&str, ComponentType> = BTreeMap::new();
    for instance in &instances {
        if let std::collections::btree_map::Entry::Vacant(slot) = types.entry(instance.alias) {
            let import = site.import(
                instance.alias,
                ForgeKind::TestComponent,
                instance.element,
                "type",
                instance.line,
            )?;
            let component = site.open_component(import)?;
            let ports = component_ports(&component)?;
            slot.insert(ComponentType { component, ports });
        }
    }
    let components = Components { instances, types };

    let mut system: BTreeMap<&str, PortLists> = BTreeMap::new();
    for port in &configuration.system_ports {
        let lists = port_type_lists(site, &port.port_type, "<sce:system-port>", port.source_line)?;
        system.insert(&port.id, lists);
    }

    // -1 §21.1.1 c: what the component port sends the system port sends on,
    // what the system port receives the component port receives, and
    // something crosses.
    let mut joined: BTreeSet<(&str, &str)> = BTreeSet::new();
    for map in &configuration.maps {
        let ours = components.port(site, &map.component, &map.port, "map", map.source_line)?;
        let theirs = system.get(map.system_port.as_str()).ok_or_else(|| {
            site.refuse(
                ValidationError::InvalidAttribute {
                    element: "<sce:map>".into(),
                    attr: "system-port".into(),
                    value: map.system_port.clone(),
                    allowed: configuration
                        .system_ports
                        .iter()
                        .map(|p| p.id.clone())
                        .collect(),
                },
                map.source_line,
            )
        })?;
        let joins = format!(
            "the <sce:map> joins the port '{}' of the component '{}' to the port '{}' of the \
             test system interface",
            map.port, map.component, map.system_port
        );
        let sent = missing_from(&ours.outs, &theirs.outs);
        if !sent.is_empty() {
            return Err(site.profile_refusal(
                map.source_line,
                TestProfileRule::PortLists,
                kind,
                name,
                &map.port,
                format!(
                    "{joins}, and the system port's `out` list does not hold {}, which the \
                     component port sends: the component port's `out` list is contained in the \
                     system port's (-1 §21.1.1 c)",
                    quoted(&sent)
                ),
            ));
        }
        let received = missing_from(&theirs.ins, &ours.ins);
        if !received.is_empty() {
            return Err(site.profile_refusal(
                map.source_line,
                TestProfileRule::PortLists,
                kind,
                name,
                &map.system_port,
                format!(
                    "{joins}, and the component port's `in` list does not hold {}, which the \
                     system port receives: the system port's `in` list is contained in the \
                     component port's (-1 §21.1.1 c)",
                    quoted(&received)
                ),
            ));
        }
        if ours.outs.is_empty() && theirs.ins.is_empty() {
            return Err(site.profile_refusal(
                map.source_line,
                TestProfileRule::PortLists,
                kind,
                name,
                "",
                format!(
                    "{joins}, and no message crosses it: the component port sends none and the \
                     system port receives none, and at least one of the two lists is not empty \
                     (-1 §21.1.1 c)"
                ),
            ));
        }
        joined.insert((map.component.as_str(), map.port.as_str()));
    }

    // -1 §21.1.1 b: what each port sends the other receives, and something
    // is sent.
    for connect in &configuration.connects {
        // The parser holds a connect to exactly two endpoints.
        let [a, b] = connect.endpoints.as_slice() else {
            continue;
        };
        // Each port is named on its own `<sce:endpoint>`, so a refusal of a
        // port stands on the line that names it.
        let ends = [
            (
                a,
                components.port(site, &a.component, &a.port, "endpoint", a.source_line)?,
            ),
            (
                b,
                components.port(site, &b.component, &b.port, "endpoint", b.source_line)?,
            ),
        ];
        for (from, to) in [(&ends[0], &ends[1]), (&ends[1], &ends[0])] {
            let stray = missing_from(&from.1.outs, &to.1.ins);
            if !stray.is_empty() {
                return Err(site.profile_refusal(
                    from.0.source_line,
                    TestProfileRule::PortLists,
                    kind,
                    name,
                    &from.0.port,
                    format!(
                        "the <sce:connect> joins the port '{}' of the component '{}' to the port \
                         '{}' of the component '{}', and the second's `in` list does not hold \
                         {}, which the first sends: each port's `out` list is contained in the \
                         other's `in` list (-1 §21.1.1 b)",
                        from.0.port,
                        from.0.component,
                        to.0.port,
                        to.0.component,
                        quoted(&stray)
                    ),
                ));
            }
        }
        if ends[0].1.outs.is_empty() && ends[1].1.outs.is_empty() {
            return Err(site.profile_refusal(
                connect.source_line,
                TestProfileRule::PortLists,
                kind,
                name,
                "",
                format!(
                    "the <sce:connect> joins the port '{}' of the component '{}' to the port \
                     '{}' of the component '{}', and neither sends a message: at least one of \
                     the two `out` lists is not empty (-1 §21.1.1 b)",
                    a.port, a.component, b.port, b.component
                ),
            ));
        }
        joined.insert((a.component.as_str(), a.port.as_str()));
        joined.insert((b.component.as_str(), b.port.as_str()));
    }

    // §2.2: every component port takes part in exactly one connect or map.
    // The parser refused a port joined twice; one joined by none is refused
    // here, where the components' ports are known. It stands on the
    // component's declaration and reports no `actual`: the port is declared
    // in the component's document, and the join this one lacks is written
    // nowhere.
    for instance in &components.instances {
        let Some(ty) = components.types.get(instance.alias) else {
            continue;
        };
        for port in &ty.component.profile.ports {
            if !joined.contains(&(instance.name, port.id.as_str())) {
                return Err(site.profile_refusal(
                    instance.line,
                    TestProfileRule::PortConnection,
                    kind,
                    name,
                    "",
                    format!(
                        "the port '{}' of the component '{}' is joined by no connect or map: \
                         every port of the configuration takes part in exactly one connect or map",
                        port.id, instance.name
                    ),
                ));
            }
        }
    }

    check_ptc_tree(configuration, &components, site)?;
    Ok(components
        .types
        .into_values()
        .map(|ty| ty.component)
        .collect())
}

/// One `<invoke>` of a component of a configuration that creates a PTC: the
/// component that invokes it, the invocation, and the document it is in.
struct Creation<'c> {
    parent: &'c str,
    invocation: &'c TestInvocation,
    component: &'c Component,
}

/// §11.6, P9: refuse an `<invoke>` of a component that names no PTC the
/// configuration declares; a PTC that is the `id` of no `<invoke>` of the
/// configuration's components, or of more than one; a PTC whose `<invoke>`
/// starts a document other than the PTC's type; and a PTC no chain of
/// invocations from the MTC creates (`test-profile/ptc-invocation`).
fn check_ptc_tree(
    configuration: &TestConfigurationModel,
    components: &Components<'_>,
    site: &Site<'_>,
) -> Result<(), Located<ForgeError>> {
    let kind = ForgeKind::TestConfiguration;
    let name = configuration.name.as_str();
    let refuse = |line: Option<u32>, actual: &str, detail: String| {
        site.profile_refusal(
            line,
            TestProfileRule::PtcInvocation,
            kind,
            name,
            actual,
            detail,
        )
    };

    // Every invocation of every component, by the PTC it names. A component
    // type two components share invokes once for each of them.
    let mut created: BTreeMap<&str, Vec<Creation<'_>>> = BTreeMap::new();
    for instance in &components.instances {
        let Some(ty) = components.types.get(instance.alias) else {
            continue;
        };
        for invocation in &ty.component.profile.invocations {
            let Some(id) = invocation.id.as_deref() else {
                continue;
            };
            if !configuration.ptcs.iter().any(|ptc| ptc.id == id) {
                // The PTC is named in the component's document, and the
                // configuration lacks its declaration: the refusal stands on
                // the component that invokes it, with no `actual`.
                return Err(refuse(
                    instance.line,
                    "",
                    format!(
                        "the component '{}' invokes the PTC '{id}' ({}), which no <sce:ptc> of \
                         the configuration declares: every <invoke> of a test component creates \
                         a PTC its configuration declares",
                        instance.name,
                        ty.component.at.spell(invocation.source_location.as_ref())
                    ),
                ));
            }
            created.entry(id).or_default().push(Creation {
                parent: instance.name,
                invocation,
                component: &ty.component,
            });
        }
    }

    let mut parent_of: BTreeMap<&str, &str> = BTreeMap::new();
    for ptc in &configuration.ptcs {
        let creations = created
            .get(ptc.id.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();
        let [creation] = creations else {
            let parents: Vec<&str> = creations.iter().map(|c| c.parent).collect();
            let by = if parents.is_empty() {
                String::new()
            } else {
                format!(", by {}", quoted(&parents))
            };
            return Err(refuse(
                ptc.source_line,
                &ptc.id,
                format!(
                    "the PTC '{}' is the id of {} <invoke> of the configuration's components{by}: \
                     every <sce:ptc> is created by exactly one",
                    ptc.id,
                    creations.len()
                ),
            ));
        };
        let Some(ty) = components.types.get(ptc.component.as_str()) else {
            continue;
        };
        let started = creation.invocation.src.as_deref().map(|src| {
            (
                src,
                stdlib::identity(&stdlib::resolve(&creation.component.at.dir, src)),
            )
        });
        match started {
            Some((_, identity)) if identity == ty.component.at.identity => {}
            Some((src, _)) => {
                // `src` is written in the invoking component's document; what
                // the `<sce:ptc>` writes is the type it declares.
                return Err(refuse(
                    ptc.source_line,
                    &ptc.component,
                    format!(
                        "the <invoke> of the component '{}' that creates the PTC '{}' ({}) \
                         starts '{src}', which is not the test component '{}' the <sce:ptc> \
                         names: a PTC is created as a document of its type",
                        creation.parent,
                        ptc.id,
                        creation
                            .component
                            .at
                            .spell(creation.invocation.source_location.as_ref()),
                        ptc.component
                    ),
                ));
            }
            None => {
                return Err(refuse(
                    ptc.source_line,
                    "",
                    format!(
                        "the <invoke> of the component '{}' that creates the PTC '{}' ({}) names \
                         no src: a PTC is created as a document of its type, '{}'",
                        creation.parent,
                        ptc.id,
                        creation
                            .component
                            .at
                            .spell(creation.invocation.source_location.as_ref()),
                        ptc.component
                    ),
                ));
            }
        }
        parent_of.insert(&ptc.id, creation.parent);
    }

    // The invoking relation is a tree rooted at the MTC: every PTC is reached
    // from the MTC by its chain of creators. Each PTC has one creator, so one
    // that is not reached is on a cycle of components that create each other.
    let mut reached: BTreeSet<&str> = BTreeSet::from([TestConfigurationModel::MTC]);
    loop {
        let before = reached.len();
        for (ptc, parent) in &parent_of {
            if reached.contains(parent) {
                reached.insert(*ptc);
            }
        }
        if reached.len() == before {
            break;
        }
    }
    for ptc in &configuration.ptcs {
        if !reached.contains(ptc.id.as_str()) {
            let parent = parent_of.get(ptc.id.as_str()).copied().unwrap_or_default();
            return Err(refuse(
                ptc.source_line,
                &ptc.id,
                format!(
                    "the PTC '{}' is created by '{parent}', which no chain of invocations from \
                     the MTC creates: the components of a configuration create each other in a \
                     tree rooted at the MTC, so no component creates itself",
                    ptc.id
                ),
            ));
        }
    }
    Ok(())
}

// ── §11.7: the test case ───────────────────────────────────────

/// §11.7: refuse a test case whose configuration is no test configuration
/// (`import/kind-mismatch`), or breaks a rule of §11.6. Give the
/// configuration's component types, read.
fn check_test_case(
    case: &TestCaseModel,
    site: &Site<'_>,
) -> Result<Vec<Component>, Located<ForgeError>> {
    let import = site.import(
        &case.configuration,
        ForgeKind::TestConfiguration,
        "<scxml sce:kind=\"test-case\">",
        "sce:configuration",
        line(case.source_location.as_ref()),
    )?;
    let configuration = site.open(import, |document| match document {
        ForgeDocument::TestConfiguration(configuration) => Some(configuration),
        _ => None,
    })?;
    check_configuration(&configuration.document, &configuration.site())
}

// ── §11.8: the suite ───────────────────────────────────────────

/// §11.8: refuse a suite whose `<sce:execute>` names no test case
/// (`import/kind-mismatch`), or one that breaks a rule of §11.6 or §11.7;
/// whose selection names a feature its feature model does not declare
/// (`validation/feature-undeclared`, ADR 0057 §3); or that does not declare,
/// with its type, a module parameter a component of a test case it runs reads
/// (`test-profile/module-parameter`).
fn check_suite(suite: &TestSuiteModel, site: &Site<'_>) -> Result<(), Located<ForgeError>> {
    let features = match &suite.features {
        None => None,
        Some(alias) => {
            let import = site.import(
                alias,
                ForgeKind::FeatureModel,
                "<sce:import kind=\"feature-model\">",
                "as",
                None,
            )?;
            let model = site.open(import, |document| match document {
                ForgeDocument::FeatureModel(model) => Some(model),
                _ => None,
            })?;
            Some(model.document)
        }
    };
    for execute in &suite.executes {
        if let (Some(condition), Some(features)) = (&execute.requires, &features) {
            check_selection(condition, features, execute, site)?;
        }
        let import = site.import(
            &execute.test_case,
            ForgeKind::TestCase,
            "<sce:execute>",
            "test-case",
            execute.source_line,
        )?;
        let case = site.open(import, |document| match document {
            ForgeDocument::TestCase(case) => Some(case),
            _ => None,
        })?;
        for component in check_test_case(&case.document, &case.site())? {
            check_module_parameters(suite, site, execute, &case.document.name, &component)?;
        }
    }
    Ok(())
}

/// ADR 0057 §3, docs/SCE_TEST_PROFILE.md §11.8: the selection `condition` of
/// `execute` names only features of the suite's feature model.
fn check_selection(
    condition: &str,
    features: &FeaturesModel,
    execute: &TestExecute,
    site: &Site<'_>,
) -> Result<(), Located<ForgeError>> {
    let expr = crate::presence_condition::parse(condition).map_err(|error| {
        site.refuse(
            ValidationError::FeatureConditionMalformed {
                condition: condition.to_string(),
                reason: error.message,
            },
            execute.source_line,
        )
    })?;
    PresenceCondition::resolve(&expr, &features.feature_model()).map_err(|undeclared| {
        site.refuse(
            ValidationError::FeatureUndeclared {
                feature: undeclared.name,
                model: Some(features.name.clone()),
            },
            execute.source_line,
        )
    })?;
    Ok(())
}

/// §11.5, §11.8: every `<data sce:module-parameter="true">` of `component`,
/// which the test case `case` that `execute` runs reaches, has a
/// `<sce:module-parameter>` of the suite with its `id` and its type.
fn check_module_parameters(
    suite: &TestSuiteModel,
    site: &Site<'_>,
    execute: &TestExecute,
    case: &str,
    component: &Component,
) -> Result<(), Located<ForgeError>> {
    let kind = ForgeKind::TestSuite;
    for id in &component.profile.module_parameters {
        let Some(declared) = suite.module_parameters.iter().find(|p| p.id == *id) else {
            // The parameter is read in the component's document and declared
            // nowhere in the suite: the refusal stands on the execute that
            // reaches the component, with no `actual`.
            return Err(site.profile_refusal(
                execute.source_line,
                TestProfileRule::ModuleParameter,
                kind,
                &suite.name,
                "",
                format!(
                    "the test case '{case}' runs the component '{}', which reads the module \
                     parameter '{id}', and the suite declares no <sce:module-parameter> of that \
                     id: every module parameter a component of the suite reads is declared in \
                     the suite with its type",
                    component.model.name
                ),
            ));
        };
        // A module parameter is a `<data>` of a `sce-static` component, which
        // types every `<data>` (§11.5), so the record and its type are there.
        let Some(read) = component
            .variable(id)
            .and_then(|variable| variable.value_type.as_ref())
        else {
            continue;
        };
        let read_key = match read.clone().bounded() {
            Ok(ty) => type_key(&ty, &component.site()),
            Err(_) => read.to_string(),
        };
        if type_key(&declared.parameter_type, site) != read_key {
            let written = declared.parameter_type.to_string();
            return Err(site.profile_refusal(
                declared.source_line,
                TestProfileRule::ModuleParameter,
                kind,
                &suite.name,
                &written,
                format!(
                    "the module parameter '{id}' is declared of type {written}, and the \
                     component '{}', which the test case '{case}' runs, reads it as {read}: a \
                     module parameter is declared with the type its components read it as",
                    component.model.name
                ),
            ));
        }
    }
    Ok(())
}

/// The spelling of `ty` that two documents agree on when they name one type:
/// as written, but an enum named by the document it is rather than by the
/// alias `site` imports it under.
fn type_key(ty: &ValueType, site: &Site<'_>) -> String {
    if let TypeOf::Scalar(SceType::Enum(reference)) = ty {
        if let Some(import) = site
            .imports
            .iter()
            .find(|import| import.alias == reference.alias && import.kind == ForgeKind::Enum)
        {
            return format!("enum:{}", site.identity_of(import));
        }
    }
    ty.to_string()
}
