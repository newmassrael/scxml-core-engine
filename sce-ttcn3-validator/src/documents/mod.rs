// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The profile's documents a suite reaches, read with `roxmltree`, each
//! construct named as the witness names it (docs/SCE_TEST_PROFILE.md §9.4,
//! **Constructs**; §9.5, **Input**).
//!
//! From the `test-suite`, the walk follows each `<sce:import>` of a test
//! case, a configuration, a test component, a port type, a template, an event
//! schema, an enum or a union, its `src` resolved against the directory of
//! the document that imports it, and reads each document once, by its one
//! identity whatever path reaches it. The feature model the suite imports is
//! read and not followed. A document is named by its path relative to the
//! suite's directory, `.` and `..` folded and `/` its separator. A document
//! of the standard library (`sce:std/…`) is not read: this validator carries
//! no copy of the library, and says so by name.
//!
//! A construct is a node of a document: an element by its kind and id
//! (`state 'requesting'`, `final 'failed'`, `invoke 'helper'`,
//! `sce:test-port 'sut'`), a child by its kind and its position from 1 under
//! one (`state 'requesting' / transition 1 / send 3`), and a document a rule
//! translates whole by its root, `scxml`. Its row is the row of the node's
//! start tag, with three exceptions the witness states: a component's root
//! and a feature keep none, and an element of executable content takes the
//! row of the state or the transition whose block holds it.

mod statechart;

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::PathBuf;

use crate::rules::{Facts, Kind};
use crate::witness::Construct;
use crate::InputError;

/// The SCXML namespace.
pub const SCXML_NAMESPACE: &str = "http://www.w3.org/2005/07/scxml";

/// SCE's namespace, of `sce:kind`, `sce:import` and the profile's elements.
pub const SCE_NAMESPACE: &str = "http://sce.dev/ext";

/// The scheme of a document of the standard library.
const STANDARD: &str = "sce:std/";

/// Where the documents are read from, each by its name: its path relative to
/// the suite's directory.
pub trait Files {
    /// The text of the document `name`.
    fn text(&self, name: &str) -> io::Result<String>;
    /// The one identity of the document `name`, the same for every name of
    /// one document.
    fn identity(&self, name: &str) -> String;
}

/// The documents of a directory, the suite's.
#[derive(Debug, Clone)]
pub struct Directory {
    root: PathBuf,
}

impl Directory {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl Files for Directory {
    fn text(&self, name: &str) -> io::Result<String> {
        std::fs::read_to_string(self.root.join(name))
    }

    fn identity(&self, name: &str) -> String {
        let path = self.root.join(name);
        std::fs::canonicalize(&path)
            .unwrap_or(path)
            .display()
            .to_string()
    }
}

/// The kind of a document of the profile, its `sce:kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    TestSuite,
    TestCase,
    TestConfiguration,
    TestComponent,
    TestPortType,
    TestTemplate,
    EventSchema,
    Enum,
    Union,
    FeatureModel,
}

impl DocumentKind {
    /// The kind `sce:kind` names, among the kinds a suite reaches.
    pub fn from_attr(text: &str) -> Option<Self> {
        Some(match text {
            "test-suite" => DocumentKind::TestSuite,
            "test-case" => DocumentKind::TestCase,
            "test-configuration" => DocumentKind::TestConfiguration,
            "test-component" => DocumentKind::TestComponent,
            "test-port-type" => DocumentKind::TestPortType,
            "test-template" => DocumentKind::TestTemplate,
            "event-schema" => DocumentKind::EventSchema,
            "enum" => DocumentKind::Enum,
            "union" => DocumentKind::Union,
            "feature-model" => DocumentKind::FeatureModel,
            _ => return None,
        })
    }

    /// The kind as `sce:kind` writes it.
    pub fn as_attr(self) -> &'static str {
        match self {
            DocumentKind::TestSuite => "test-suite",
            DocumentKind::TestCase => "test-case",
            DocumentKind::TestConfiguration => "test-configuration",
            DocumentKind::TestComponent => "test-component",
            DocumentKind::TestPortType => "test-port-type",
            DocumentKind::TestTemplate => "test-template",
            DocumentKind::EventSchema => "event-schema",
            DocumentKind::Enum => "enum",
            DocumentKind::Union => "union",
            DocumentKind::FeatureModel => "feature-model",
        }
    }

    /// Whether the walk follows an import of this kind: every kind a rule
    /// reads, but the suite and its feature model.
    fn followed(self) -> bool {
        !matches!(self, DocumentKind::TestSuite | DocumentKind::FeatureModel)
    }

    /// The kind of the document's root as a construct.
    fn root(self) -> Kind {
        match self {
            DocumentKind::TestSuite => Kind::Suite,
            DocumentKind::TestCase => Kind::TestCase,
            DocumentKind::TestConfiguration => Kind::Configuration,
            DocumentKind::TestComponent => Kind::Component,
            DocumentKind::TestPortType => Kind::PortType,
            DocumentKind::TestTemplate => Kind::Template,
            DocumentKind::EventSchema | DocumentKind::Enum | DocumentKind::Union => Kind::Types,
            DocumentKind::FeatureModel => Kind::FeatureModel,
        }
    }
}

/// A construct of a document, its kind, and what the documents fix about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub node: String,
    pub line: Option<u32>,
    pub kind: Kind,
    pub facts: Facts,
}

/// An `<sce:import>`, and the document it names, once read.
#[derive(Debug, Clone)]
struct Import {
    kind: String,
    alias: String,
    src: String,
    target: Option<usize>,
}

/// What a document says of the documents it imports, by alias.
#[derive(Debug, Clone, Default)]
struct Links {
    /// A suite: the test case each `<sce:execute>` names.
    executes: Vec<String>,
    /// A test case: its configuration.
    configuration: Option<String>,
    /// A configuration: the component of its MTC and of each PTC.
    components: Vec<String>,
    /// A component: whether it invokes a PTC.
    invokes: bool,
}

/// One document the suite reaches.
#[derive(Debug, Clone)]
pub struct Document {
    /// Its path relative to the suite's directory.
    pub name: String,
    pub kind: DocumentKind,
    pub nodes: Vec<Node>,
    /// A component: the row of each receive branch that matches with a
    /// template, at which the local the message is stored in is named.
    received: BTreeSet<u32>,
    imports: Vec<Import>,
    links: Links,
}

impl Document {
    /// The construct `node` on `line`, of `kind`; its index.
    fn push(&mut self, node: String, line: Option<u32>, kind: Kind) -> usize {
        self.nodes.push(Node {
            node,
            line,
            kind,
            facts: Facts::default(),
        });
        self.nodes.len() - 1
    }

    /// The document this one imports as `alias`, of `kind`, once read.
    fn target(&self, alias: &str, kind: DocumentKind) -> Option<usize> {
        self.imports
            .iter()
            .find(|import| import.alias == alias && import.kind == kind.as_attr())
            .and_then(|import| import.target)
    }
}

/// Every document a suite reaches, the suite first.
#[derive(Debug, Clone)]
pub struct Documents {
    /// The suite's document, by its name.
    pub suite: String,
    pub documents: Vec<Document>,
}

impl Documents {
    /// The documents the suite `suite`, named relative to the directory
    /// `files` reads from, reaches.
    pub fn read(files: &impl Files, suite: &str) -> Result<Self, InputError> {
        let suite = fold(suite);
        let first = read_document(files, &suite)?;
        if first.kind != DocumentKind::TestSuite {
            return Err(InputError::Document {
                document: suite,
                reason: format!(
                    "it is a {}, and an export is of a test-suite",
                    first.kind.as_attr()
                ),
            });
        }
        let mut documents = vec![first];
        let mut identities = BTreeMap::from([(files.identity(&suite), 0)]);
        // The order the exporter walks in, a stack, so a document two paths
        // reach is named by the same one of them.
        let mut pending = vec![0];
        while let Some(at) = pending.pop() {
            let imports = documents[at].imports.clone();
            for (position, import) in imports.iter().enumerate() {
                let Some(kind) = DocumentKind::from_attr(&import.kind) else {
                    continue;
                };
                if !kind.followed() {
                    continue;
                }
                let (index, new) = reach(files, &mut documents, &mut identities, at, import, kind)?;
                documents[at].imports[position].target = Some(index);
                if new {
                    pending.push(index);
                }
            }
        }
        // The feature model, which rule E15 alone reads and the walk does
        // not follow.
        let feature_model = documents[0]
            .imports
            .iter()
            .position(|import| import.kind == DocumentKind::FeatureModel.as_attr());
        if let Some(position) = feature_model {
            let import = documents[0].imports[position].clone();
            let (index, _) = reach(
                files,
                &mut documents,
                &mut identities,
                0,
                &import,
                DocumentKind::FeatureModel,
            )?;
            documents[0].imports[position].target = Some(index);
        }
        let mut read = Self { suite, documents };
        read.settle();
        Ok(read)
    }

    /// The facts one document fixes about another: which test cases the
    /// suite executes and the configurations they run in, and which
    /// components a configuration holds.
    fn settle(&mut self) {
        let suite = &self.documents[0];
        let cases: BTreeSet<usize> = suite
            .links
            .executes
            .iter()
            .filter_map(|alias| suite.target(alias, DocumentKind::TestCase))
            .collect();
        let configurations: BTreeSet<usize> = cases
            .iter()
            .filter_map(|&case| {
                let case = &self.documents[case];
                case.links
                    .configuration
                    .as_deref()
                    .and_then(|alias| case.target(alias, DocumentKind::TestConfiguration))
            })
            .collect();
        let mut held = BTreeSet::new();
        for document in &self.documents {
            if document.kind != DocumentKind::TestConfiguration {
                continue;
            }
            for alias in &document.links.components {
                held.extend(document.target(alias, DocumentKind::TestComponent));
            }
        }
        for (index, document) in self.documents.iter_mut().enumerate() {
            let executed = cases.contains(&index) || configurations.contains(&index);
            let component_held = !document.links.invokes || held.contains(&index);
            for node in &mut document.nodes {
                node.facts.executed = executed;
                node.facts.held = component_held;
            }
        }
    }

    /// The document named `name`.
    pub fn document(&self, name: &str) -> Option<&Document> {
        self.documents.iter().find(|document| document.name == name)
    }

    /// Whether the documents hold `construct`: its document is one the suite
    /// reaches, and the node is one of it on the row the construct names.
    /// The local a receive branch stores its message in is named after the
    /// record, on the row of a receive branch that matches with a template.
    pub fn holds(&self, construct: &Construct) -> bool {
        let Some(document) = self.document(&construct.document) else {
            return false;
        };
        if document
            .nodes
            .iter()
            .any(|node| node.node == construct.node && node.line == construct.line)
        {
            return true;
        }
        document.kind == DocumentKind::TestComponent
            && construct
                .node
                .strip_prefix("received ")
                .is_some_and(is_identifier)
            && construct
                .line
                .is_some_and(|line| document.received.contains(&line))
    }
}

/// Whether `name` is a TTCN-3 identifier.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The document `import`, of `kind`, that the document `from` names: its
/// index, and whether it is read for the first time.
fn reach(
    files: &impl Files,
    documents: &mut Vec<Document>,
    identities: &mut BTreeMap<String, usize>,
    from: usize,
    import: &Import,
    kind: DocumentKind,
) -> Result<(usize, bool), InputError> {
    if import.src.starts_with(STANDARD) {
        return Err(InputError::Document {
            document: documents[from].name.clone(),
            reason: format!(
                "it imports '{}', a document of the standard library, which this validator does \
                 not read",
                import.src
            ),
        });
    }
    let name = fold(&joined(directory(&documents[from].name), &import.src));
    let identity = files.identity(&name);
    if let Some(&index) = identities.get(&identity) {
        return Ok((index, false));
    }
    let document = read_document(files, &name)?;
    if document.kind != kind {
        return Err(InputError::Document {
            document: name,
            reason: format!(
                "it is a {}, and '{}' imports it as a {}",
                document.kind.as_attr(),
                documents[from].name,
                kind.as_attr()
            ),
        });
    }
    documents.push(document);
    identities.insert(identity, documents.len() - 1);
    Ok((documents.len() - 1, true))
}

/// `name` with each `.` dropped and each `..` folded into the segment before
/// it, where there is one to fold it into; `/` its separator.
pub fn fold(name: &str) -> String {
    let rooted = name.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in name.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|last| *last != "..") {
                    parts.pop();
                } else if !rooted {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    if rooted {
        format!("/{joined}")
    } else {
        joined
    }
}

/// The directory of the document `name`: all of it before its last `/`.
fn directory(name: &str) -> &str {
    name.rfind('/').map_or("", |at| &name[..at])
}

/// `src` read from the directory `directory`.
fn joined(directory: &str, src: &str) -> String {
    if directory.is_empty() || src.starts_with('/') {
        src.to_string()
    } else {
        format!("{directory}/{src}")
    }
}

/// The row of `node`'s start tag.
fn row(node: &roxmltree::Node<'_, '_>) -> u32 {
    node.document().text_pos_at(node.range().start).row
}

/// Whether `node` is the element `local` of `namespace`.
fn is_element(node: &roxmltree::Node<'_, '_>, namespace: &str, local: &str) -> bool {
    node.is_element()
        && node.tag_name().namespace() == Some(namespace)
        && node.tag_name().name() == local
}

/// The children of `node` that are the element `local` of `namespace`.
fn children<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    namespace: &str,
    local: &str,
) -> Vec<roxmltree::Node<'a, 'input>> {
    node.children()
        .filter(|child| is_element(child, namespace, local))
        .collect()
}

/// The value of `node`'s attribute `name`, trimmed, where it is written.
fn attribute(node: &roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    node.attribute(name).map(|value| value.trim().to_string())
}

/// The value of `node`'s `sce:` attribute `local`, trimmed, where it is
/// written.
fn sce_attribute(node: &roxmltree::Node<'_, '_>, local: &str) -> Option<String> {
    node.attribute((SCE_NAMESPACE, local))
        .map(|value| value.trim().to_string())
}

/// The document `name`, read and its constructs named.
fn read_document(files: &impl Files, name: &str) -> Result<Document, InputError> {
    let text = files.text(name).map_err(|error| InputError::Read {
        path: name.to_string(),
        reason: error.to_string(),
    })?;
    let xml = roxmltree::Document::parse(&text).map_err(|error| InputError::Xml {
        document: name.to_string(),
        reason: error.to_string(),
    })?;
    let root = xml.root_element();
    let written = sce_attribute(&root, "kind").ok_or_else(|| InputError::Document {
        document: name.to_string(),
        reason: "it declares no sce:kind: a statechart, which no rule of section 9.2 translates"
            .to_string(),
    })?;
    let kind = DocumentKind::from_attr(&written).ok_or_else(|| InputError::Document {
        document: name.to_string(),
        reason: format!("its sce:kind '{written}' is no kind a suite reaches"),
    })?;
    let imports = children(root, SCE_NAMESPACE, "import")
        .iter()
        .map(|import| Import {
            kind: attribute(import, "kind").unwrap_or_default(),
            alias: attribute(import, "as").unwrap_or_default(),
            src: import.attribute("src").unwrap_or_default().to_string(),
            target: None,
        })
        .collect();
    let mut document = Document {
        name: name.to_string(),
        kind,
        nodes: Vec::new(),
        received: BTreeSet::new(),
        imports,
        links: Links::default(),
    };
    if kind == DocumentKind::TestComponent {
        statechart::read(root, &mut document)?;
    } else {
        profile_document(root, &mut document);
    }
    Ok(document)
}

/// The constructs of a document of the profile that is no component: its
/// root, and the module parameters and executes of a suite, the features of
/// a feature model and the system ports of a configuration.
fn profile_document(root: roxmltree::Node<'_, '_>, document: &mut Document) {
    document.push("scxml".to_string(), Some(row(&root)), document.kind.root());
    match document.kind {
        DocumentKind::TestSuite => {
            for parameter in children(root, SCE_NAMESPACE, "module-parameter") {
                let id = attribute(&parameter, "id").unwrap_or_default();
                document.push(
                    format!("sce:module-parameter '{id}'"),
                    Some(row(&parameter)),
                    Kind::ModuleParameter,
                );
            }
            for execute in children(root, SCE_NAMESPACE, "execute") {
                let test_case = attribute(&execute, "test-case").unwrap_or_default();
                document.push(
                    format!("sce:execute '{test_case}'"),
                    Some(row(&execute)),
                    Kind::Execute,
                );
                document.links.executes.push(test_case);
            }
        }
        DocumentKind::FeatureModel => {
            for feature in children(root, SCE_NAMESPACE, "feature") {
                let name = attribute(&feature, "name").unwrap_or_default();
                document.push(format!("sce:feature '{name}'"), None, Kind::Feature);
            }
        }
        DocumentKind::TestConfiguration => {
            for port in children(root, SCE_NAMESPACE, "system-port") {
                let id = attribute(&port, "id").unwrap_or_default();
                document.push(
                    format!("sce:system-port '{id}'"),
                    Some(row(&port)),
                    Kind::SystemPort,
                );
            }
            for local in ["mtc", "ptc"] {
                for component in children(root, SCE_NAMESPACE, local) {
                    document
                        .links
                        .components
                        .extend(attribute(&component, "type"));
                }
            }
        }
        DocumentKind::TestCase => {
            document.links.configuration = sce_attribute(&root, "configuration");
        }
        _ => {}
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::rules::Rule;

    /// Documents held in memory, each by its name, its identity its folded
    /// name.
    pub(crate) struct Memory(pub(crate) BTreeMap<&'static str, &'static str>);

    impl Files for Memory {
        fn text(&self, name: &str) -> io::Result<String> {
            self.0
                .get(name)
                .map(|text| text.to_string())
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, name.to_string()))
        }

        fn identity(&self, name: &str) -> String {
            fold(name)
        }
    }

    #[test]
    fn a_name_is_folded() {
        assert_eq!(fold("a/../b.scxml"), "b.scxml");
        assert_eq!(fold("./parts/./c.scxml"), "parts/c.scxml");
        assert_eq!(fold("../x/../../y.scxml"), "../../y.scxml");
        assert_eq!(fold("/r/../s.scxml"), "/s.scxml");
        assert_eq!(
            fold(&joined(directory("parts/a.scxml"), "../b.scxml")),
            "b.scxml"
        );
        assert_eq!(joined(directory("a.scxml"), "b.scxml"), "b.scxml");
    }

    const SUITE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="s">
  <sce:import kind="feature-model" src="parts/features.scxml" as="Features"/>
  <sce:import kind="test-case" src="parts/case.scxml" as="Case"/>
  <sce:import kind="test-case" src="parts/idle.scxml" as="Idle"/>
  <sce:module-parameter id="limit" type="uint32" value="5"/>
  <sce:execute test-case="Case"/>
</scxml>"#;

    const FEATURES: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="feature-model" name="f">
  <sce:feature name="fast"/>
</scxml>"#;

    const CASE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-case" name="case"
       sce:configuration="Config">
  <sce:import kind="test-configuration" src="../config.scxml" as="Config"/>
</scxml>"#;

    const IDLE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-case" name="idle"
       sce:configuration="Config">
  <sce:import kind="test-configuration" src="./../config.scxml" as="Config"/>
</scxml>"#;

    const CONFIG: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-configuration" name="config">
  <sce:import kind="test-port-type" src="port.scxml" as="Port"/>
  <sce:system-port id="sut" type="Port"/>
</scxml>"#;

    const PORT: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-port-type" name="port"/>"#;

    fn memory() -> Memory {
        Memory(BTreeMap::from([
            ("s.scxml", SUITE),
            ("parts/features.scxml", FEATURES),
            ("parts/case.scxml", CASE),
            ("parts/idle.scxml", IDLE),
            ("config.scxml", CONFIG),
            ("port.scxml", PORT),
        ]))
    }

    fn construct(document: &str, node: &str, line: Option<u32>) -> Construct {
        Construct {
            document: document.to_string(),
            node: node.to_string(),
            line,
        }
    }

    /// The walk reads each document once by its folded name, names its
    /// constructs, and fixes which test case and configuration the suite
    /// executes.
    #[test]
    fn the_documents_a_suite_reaches_are_read_once_each() {
        let documents = Documents::read(&memory(), "s.scxml").expect("documents");
        let names: Vec<&str> = documents
            .documents
            .iter()
            .map(|document| document.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "s.scxml",
                "parts/case.scxml",
                "parts/idle.scxml",
                "config.scxml",
                "port.scxml",
                "parts/features.scxml"
            ]
        );
        assert!(documents.holds(&construct("s.scxml", "scxml", Some(2))));
        assert!(documents.holds(&construct(
            "s.scxml",
            "sce:module-parameter 'limit'",
            Some(8)
        )));
        assert!(documents.holds(&construct("s.scxml", "sce:execute 'Case'", Some(9))));
        assert!(documents.holds(&construct(
            "parts/features.scxml",
            "sce:feature 'fast'",
            None
        )));
        assert!(!documents.holds(&construct("s.scxml", "scxml", Some(3))));
        assert!(!documents.holds(&construct("other.scxml", "scxml", Some(2))));

        let required = |name: &str, node: &str| -> Vec<Rule> {
            let document = documents.document(name).expect("the document");
            let node = document
                .nodes
                .iter()
                .find(|n| n.node == node)
                .expect("the node");
            crate::rules::required(node.kind, &node.facts)
        };
        assert_eq!(required("s.scxml", "scxml"), [Rule::E1, Rule::E15]);
        assert_eq!(required("parts/case.scxml", "scxml"), [Rule::E14]);
        assert!(required("parts/idle.scxml", "scxml").is_empty());
        assert_eq!(
            required("config.scxml", "sce:system-port 'sut'"),
            [Rule::E14]
        );
        assert_eq!(required("port.scxml", "scxml"), [Rule::E3]);
        assert!(required("parts/features.scxml", "scxml").is_empty());
    }

    /// A document of the standard library, and one of another kind than its
    /// import says, are not read, and say so by name.
    #[test]
    fn a_document_the_validator_does_not_read_is_named() {
        let mut files = memory();
        files.0.insert(
            "config.scxml",
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
               sce:kind="test-configuration">
  <sce:import kind="test-port-type" src="sce:std/test/port.scxml" as="Port"/>
</scxml>"#,
        );
        assert!(matches!(
            Documents::read(&files, "s.scxml"),
            Err(InputError::Document { document, .. }) if document == "config.scxml"
        ));
        let mut files = memory();
        files.0.insert(
            "port.scxml",
            r#"<scxml xmlns:sce="http://sce.dev/ext" sce:kind="enum"/>"#,
        );
        assert!(matches!(
            Documents::read(&files, "s.scxml"),
            Err(InputError::Document { document, .. }) if document == "port.scxml"
        ));
    }
}
