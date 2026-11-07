// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The rules of the test profile for a `test-component` (docs/adr/0091 §3;
//! docs/SCE_TEST_PROFILE.md §8.2, §11.5), asked of the document as it is
//! written and of the documents it names.
//!
//! Three questions, each where its answer can first be read:
//!
//! - [`check_document`] holds what the component writes itself, before any
//!   rule of the data model or of the statechart is asked, so that a
//!   construct the profile has no place for is refused as the profile's:
//!   the elements a component does not hold, native code, `_event` read
//!   outside a receive branch or in a receive any branch, the forms of a
//!   send, of a PTC's invocation and of a branch, a done branch that keeps
//!   its state, a module parameter given a value, the durations and the
//!   names its declarations and references write.
//! - [`read_imports`] holds what needs the documents the component names:
//!   the arguments a template is given, the port a template's message
//!   travels on, a send's template being specific, a PTC being a test
//!   component, a message read through the record the component imports. It
//!   returns the record each received template is of, through which the data
//!   model types the message a receive branch reads. A document read with no
//!   directory has none of them to read.
//! - [`refuse_script_engine`] holds the analysis of the parsed model to the
//!   manifest a test component must show: `needs_script_engine = false`.
//!
//! Each refusal is a `test-profile/<rule>` code, unless a code every kind uses
//! already says it: a name declared twice (`validation/duplicate-id`), a name
//! that names nothing the component declares or imports
//! (`validation/invalid-attribute`), an attribute the form needs and the
//! document left out (`validation/missing-attribute`), a document an import or
//! an invocation names that cannot be read or is not of the kind it is
//! imported as (`import/file-not-found`, `import/read-error`,
//! `import/not-forge`, `import/kind-mismatch`), a verdict's reason
//! that is not a string (the expression codes, asked where every expression
//! of the data model is typed: `crate::forge::static_datamodel`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{
    is_sce, is_scxml, required, TestComponentProfile, TestTemplateUnit, TEST_COMPONENT_INVOKE_TYPE,
    TEST_PORT_PROCESSOR,
};
use crate::forge::error::{ForgeError, ImportError, Located, TestProfileRule, ValidationError};
use crate::forge::import_source::ImportSource;
use crate::forge::model::{
    ForgeDocument, ForgeImport, ForgeKind, SceType, TestPortTypeModel, TestTemplateModel,
    SCE_NAMESPACE,
};
use crate::forge::value_type::{TypeOf, ValueType};
use crate::model::SCXMLModel;

// ── Placing a refusal ──────────────────────────────────────────

/// `err` placed where `node` starts: an element refused as a whole.
fn at_element(
    node: &roxmltree::Node,
    diag_label: &str,
    err: impl Into<ForgeError>,
) -> Located<ForgeError> {
    let pos = node.document().text_pos_at(node.range().start);
    Located::new(err.into(), diag_label, Some(pos.row), Some(pos.col))
}

/// Every element of the document below `root`, in document order, and none
/// of a nested `<scxml>`: an invoked child written in line is a document of
/// its own.
fn own_elements<'a, 'input>(
    root: roxmltree::Node<'a, 'input>,
) -> impl Iterator<Item = roxmltree::Node<'a, 'input>> {
    let top = root.id();
    root.descendants().filter(move |node| {
        node.is_element()
            && node.id() != top
            && node
                .ancestors()
                .take_while(|ancestor| ancestor.id() != top)
                .all(|ancestor| !is_scxml(&ancestor, "scxml"))
    })
}

/// The `sce:` attribute `local` of `node`, trimmed.
///
/// Read past the read ledger: the parser reads each of these where it reads
/// the element, and a read here, which only judges, must not stand in for
/// that one.
fn sce_attribute<'a>(node: &roxmltree::Node<'a, '_>, local: &str) -> Option<&'a str> {
    node.attribute((SCE_NAMESPACE, local)).map(str::trim)
}

/// The variable a location writes: its name up to the first member or index.
fn written_variable(location: &str) -> &str {
    let location = location.trim();
    let end = location
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(location.len());
    &location[..end]
}

// ── What the component writes itself ───────────────────────────

/// The elements of the SCXML namespace a test component does not hold, each
/// with why (docs/SCE_TEST_PROFILE.md §8.2).
const CONSTRUCTS_REFUSED: &[(&str, &str)] = &[
    (
        "parallel",
        "a test component has one control flow, as a TTCN-3 component has one, \
         and expresses concurrency with PTCs",
    ),
    (
        "history",
        "a state is entered the one way its transitions enter it, which a TTCN-3 \
         behaviour has a form for; a history has none",
    ),
    (
        "script",
        "a script is code of one runtime, which neither the other runtimes nor \
         the TTCN-3 export read",
    ),
    (
        "donedata",
        "a PTC's verdict is its local verdict, kept when the PTC is removed, and \
         a done event carries nothing",
    ),
    (
        "raise",
        "an internal event has no TTCN-3 form; a test component writes states \
         and eventless transitions for it",
    ),
    (
        "cancel",
        "a send has no delay, so no send waits to be cancelled; a delay is a \
         timer",
    ),
];

/// The attributes of an expression a test component holds: what may read
/// `_event`.
const EXPRESSION_ATTRIBUTES: &[&str] = &["cond", "expr", "location", "array", "reason"];

/// The attributes a `<send>` of a test component does not take, each with
/// why (docs/SCE_TEST_PROFILE.md §8.2, §11.5).
const SEND_ATTRIBUTES_REFUSED: &[(&str, &str)] = &[
    (
        "event",
        "the event name of the message is its template's record's",
    ),
    (
        "eventexpr",
        "the event name of the message is its template's record's",
    ),
    ("targetexpr", "the target is a port the component declares"),
    ("typeexpr", "the type is the test-port processor"),
    ("delay", "a send has no delay: a delay is a timer (P7)"),
    ("delayexpr", "a send has no delay: a delay is a timer (P7)"),
    (
        "namelist",
        "the message is its template's value, bound by sce:template-args",
    ),
    ("id", "a send is never cancelled, so it names no id"),
    ("idlocation", "a send is never cancelled, so it names no id"),
];

/// The attributes an `<invoke>` of a test component does not take
/// (docs/SCE_TEST_PROFILE.md §11.5): a PTC is created with a literal `id` and
/// a `src`, and is handed nothing.
const INVOKE_ATTRIBUTES_REFUSED: &[&str] = &[
    "typeexpr",
    "srcexpr",
    "idlocation",
    "namelist",
    "autoforward",
];

/// What a test component declares and imports, by name: what a reference in
/// it may name.
struct Component<'a> {
    document: &'a str,
    diag_label: &'a str,
    profile: &'a TestComponentProfile,
    /// The aliases of the `test-port-type` imports.
    port_types: Vec<String>,
    /// The aliases of the `test-template` imports.
    templates: Vec<String>,
    /// The `id` of every `<data>` of the component.
    variables: Vec<String>,
}

impl Component<'_> {
    fn ports(&self) -> Vec<String> {
        self.profile.ports.iter().map(|p| p.id.clone()).collect()
    }

    fn timers(&self) -> Vec<String> {
        self.profile.timers.iter().map(|t| t.id.clone()).collect()
    }

    fn rule(&self, rule: TestProfileRule, actual: &str, detail: String) -> ValidationError {
        ValidationError::TestProfile {
            rule,
            kind: ForgeKind::TestComponent,
            document: self.document.to_string(),
            actual: actual.to_string(),
            detail,
        }
    }

    /// The refusal of `rule`, placed on the attribute of `node` that holds
    /// `actual`, else on `node`.
    fn refused(
        &self,
        node: &roxmltree::Node,
        rule: TestProfileRule,
        actual: &str,
        detail: String,
    ) -> Located<ForgeError> {
        crate::forge::parser::located(node, self.diag_label, self.rule(rule, actual, detail))
    }

    /// The refusal of `rule`, placed where `node` starts.
    fn refused_element(
        &self,
        node: &roxmltree::Node,
        rule: TestProfileRule,
        actual: &str,
        detail: String,
    ) -> Located<ForgeError> {
        at_element(node, self.diag_label, self.rule(rule, actual, detail))
    }

    /// `value` of `element`'s `attr` names none of `allowed`.
    fn unknown(
        &self,
        node: &roxmltree::Node,
        element: &str,
        attr: &str,
        value: &str,
        allowed: Vec<String>,
    ) -> Located<ForgeError> {
        crate::forge::parser::located(
            node,
            self.diag_label,
            ValidationError::InvalidAttribute {
                element: element.to_string(),
                attr: attr.to_string(),
                value: value.to_string(),
                allowed,
            },
        )
    }

    /// `element` leaves out `attr`, which its form needs.
    fn missing(&self, node: &roxmltree::Node, element: &str, attr: &str) -> Located<ForgeError> {
        at_element(
            node,
            self.diag_label,
            ValidationError::MissingAttribute {
                element: element.to_string(),
                attr: attr.to_string(),
            },
        )
    }

    /// The duration `written` of `element`, held to whole microseconds
    /// (docs/SCE_TEST_PROFILE.md §4.7, §11.5).
    fn duration(
        &self,
        node: &roxmltree::Node,
        element: &str,
        written: &str,
    ) -> Result<(), Located<ForgeError>> {
        crate::forge::test_profile::duration_us(written)
            .map(|_| ())
            .map_err(|refusal| {
                self.refused(
                    node,
                    TestProfileRule::Duration,
                    written,
                    format!(
                        "{element} duration '{written}': expected {}",
                        refusal.rule()
                    ),
                )
            })
    }
}

/// What a test component writes itself, held to the profile
/// (docs/adr/0091 §3; docs/SCE_TEST_PROFILE.md §8.2, §11.5): `root` is the
/// component's `<scxml>`, `profile` what [`super::read`] read of it, `imports`
/// its `<sce:import>`s, `document` its name. The first refusal in document
/// order, after those of the declarations.
pub(crate) fn check_document(
    root: &roxmltree::Node,
    profile: &TestComponentProfile,
    imports: &[ForgeImport],
    document: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    let aliases = |kind: ForgeKind| -> Vec<String> {
        imports
            .iter()
            .filter(|i| i.kind == kind)
            .map(|i| i.alias.clone())
            .collect()
    };
    let component = Component {
        document,
        diag_label,
        profile,
        port_types: aliases(ForgeKind::TestPortType),
        templates: aliases(ForgeKind::TestTemplate),
        variables: own_elements(*root)
            .filter(|n| is_scxml(n, "data"))
            .filter_map(|n| n.attribute("id").map(|id| id.trim().to_string()))
            .collect(),
    };
    no_feature_model(root, &component)?;
    declarations(root, &component)?;
    for node in own_elements(*root) {
        element(&node, &component)?;
    }
    Ok(())
}

/// docs/SCE_TEST_PROFILE.md §11.8: feature conditions appear only on a
/// suite's `<sce:execute>`, which selects the test cases a world of the
/// features runs, as a TTCN-3 control part does (ES 201 873-1 §26.1); a
/// component imports no `feature-model`, so no element of it is gated and
/// derivation leaves it as written. A `requires` with no model to read it
/// against is refused by ADR 0057's own rule.
fn no_feature_model(root: &roxmltree::Node, c: &Component) -> Result<(), Located<ForgeError>> {
    let import = root.children().find(|node| {
        is_sce(node, "import") && node.attribute("kind").map(str::trim) == Some("feature-model")
    });
    match import {
        Some(node) => Err(c.refused_element(
            &node,
            TestProfileRule::ComponentConstruct,
            "feature-model",
            "<sce:import kind=\"feature-model\">: a test component takes no feature condition; \
             a world of the features selects the test cases a suite runs, on its \
             <sce:execute>, and leaves each component as written (docs/SCE_TEST_PROFILE.md \
             §11.8)"
                .to_string(),
        )),
        None => Ok(()),
    }
}

/// The ports and the timers (docs/SCE_TEST_PROFILE.md §11.5): each declared
/// once, a port of an imported port type, a timer's default a duration.
fn declarations(root: &roxmltree::Node, c: &Component) -> Result<(), Located<ForgeError>> {
    let mut ports = BTreeSet::new();
    for node in root.children().filter(|n| is_sce(n, "test-port")) {
        let id = node.attribute("id").map(str::trim).unwrap_or_default();
        if !ports.insert(id) {
            return Err(crate::forge::parser::located(
                &node,
                c.diag_label,
                ValidationError::DuplicateId {
                    kind: ForgeKind::TestComponent,
                    what: "<sce:test-port>".into(),
                    id: id.to_string(),
                },
            ));
        }
        let port_type = node.attribute("type").map(str::trim).unwrap_or_default();
        if !c.port_types.iter().any(|alias| alias == port_type) {
            return Err(c.unknown(
                &node,
                "<sce:test-port>",
                "type",
                port_type,
                c.port_types.clone(),
            ));
        }
    }
    let mut timers = BTreeSet::new();
    for node in root.children().filter(|n| is_sce(n, "test-timer")) {
        let id = node.attribute("id").map(str::trim).unwrap_or_default();
        if !timers.insert(id) {
            return Err(crate::forge::parser::located(
                &node,
                c.diag_label,
                ValidationError::DuplicateId {
                    kind: ForgeKind::TestComponent,
                    what: "<sce:test-timer>".into(),
                    id: id.to_string(),
                },
            ));
        }
        if let Some(duration) = node.attribute("duration") {
            c.duration(&node, &format!("<sce:test-timer id=\"{id}\">"), duration)?;
        }
    }
    Ok(())
}

/// One element of the component, held to every rule that names it.
fn element(node: &roxmltree::Node, c: &Component) -> Result<(), Located<ForgeError>> {
    if let Some((name, why)) = CONSTRUCTS_REFUSED
        .iter()
        .find(|(name, _)| is_scxml(node, name))
    {
        return Err(c.refused_element(
            node,
            TestProfileRule::ComponentConstruct,
            name,
            format!("<{name}>: {why} (docs/SCE_TEST_PROFILE.md §8.2)"),
        ));
    }
    native_condition(node, c)?;
    message_read(node, c)?;
    module_parameter_written(node, c)?;
    if is_scxml(node, "send") {
        send(node, c)
    } else if is_scxml(node, "invoke") {
        invoke(node, c)
    } else if is_scxml(node, "transition") {
        transition(node, c)
    } else if is_sce(node, "start-timer") || is_sce(node, "stop-timer") {
        timer_statement(node, c)
    } else {
        Ok(())
    }
}

/// docs/adr/0091 §3: a `cond` of native code (`cpp:`, `kt:`), which one
/// backend reads and the others and the TTCN-3 export do not.
fn native_condition(node: &roxmltree::Node, c: &Component) -> Result<(), Located<ForgeError>> {
    match node.attribute("cond") {
        Some(cond) if crate::parser::is_native_condition(cond) => Err(c.refused(
            node,
            TestProfileRule::NativeCode,
            cond,
            format!(
                "cond \"{cond}\" is native code of one backend: a test component holds \
                 sce-static expressions alone, so that every runtime and the TTCN-3 export \
                 read one test (docs/adr/0091 §3)"
            ),
        )),
        _ => Ok(()),
    }
}

/// docs/SCE_TEST_PROFILE.md §4.4 (P4) and docs/adr/0091 §3: `_event`, the
/// message a receive branch took, is read in the executable content of that
/// branch and nowhere else; its `cond` among the places it is not read,
/// since whether a message matches is its template's question alone. A
/// receive any branch matches no template, and a message it takes is not
/// stored (ES 201 873-1 §22.2.2, restriction d), so its content reads none.
fn message_read(node: &roxmltree::Node, c: &Component) -> Result<(), Located<ForgeError>> {
    let receive = node
        .ancestors()
        .skip(1)
        .find(|ancestor| is_scxml(ancestor, "transition"))
        .filter(|branch| sce_attribute(branch, "receive").is_some());
    let why = match receive {
        Some(branch) if sce_attribute(&branch, "test-template").is_some() => return Ok(()),
        Some(_) => {
            "a receive any branch matches no template, and the message it takes is not \
             stored (ES 201 873-1 §22.2.2, restriction d), so its content reads no _event \
             (docs/SCE_TEST_PROFILE.md §4.4)"
        }
        None => {
            "the message a receive branch takes is matched by its template alone, and is \
             read only in the executable content of that branch (docs/SCE_TEST_PROFILE.md \
             §4.4)"
        }
    };
    for attr in EXPRESSION_ATTRIBUTES {
        let Some(written) = node.attribute(*attr) else {
            continue;
        };
        if crate::forge::expr::references_event_lexically(written) {
            return Err(c.refused(
                node,
                TestProfileRule::MessageRead,
                written,
                format!("{attr}=\"{written}\" reads _event: {why}"),
            ));
        }
    }
    Ok(())
}

/// Whether the executable content of `branch` reads `_event`.
fn content_reads_message(branch: &roxmltree::Node) -> bool {
    branch.descendants().skip(1).any(|node| {
        node.is_element()
            && EXPRESSION_ATTRIBUTES.iter().any(|attr| {
                node.attribute(*attr)
                    .is_some_and(crate::forge::expr::references_event_lexically)
            })
    })
}

/// docs/SCE_TEST_PROFILE.md §11.5: a module parameter's value is the suite's,
/// so the component neither gives it one in its `<data>` nor writes it.
fn module_parameter_written(
    node: &roxmltree::Node,
    c: &Component,
) -> Result<(), Located<ForgeError>> {
    let parameters = &c.profile.module_parameters;
    if is_scxml(node, "data") {
        let id = node.attribute("id").map(str::trim).unwrap_or_default();
        if let Some(expr) = node
            .attribute("expr")
            .filter(|_| parameters.iter().any(|p| p == id))
        {
            return Err(c.refused(
                node,
                TestProfileRule::ModuleParameterWritten,
                expr,
                format!(
                    "<data id=\"{id}\" sce:module-parameter=\"true\"> has expr=\"{expr}\": a \
                     module parameter's value is the one the suite declares, so the component \
                     gives it none (docs/SCE_TEST_PROFILE.md §11.5)"
                ),
            ));
        }
        return Ok(());
    }
    // The attributes of each element that write a variable.
    let writes: &[&str] = if is_scxml(node, "assign") {
        &["location"]
    } else if is_scxml(node, "foreach") {
        &["item", "index"]
    } else if is_sce(node, "append") || is_sce(node, "clear") {
        &["target"]
    } else if is_sce(node, "lift") {
        &["location", "refusal"]
    } else if is_sce(node, "render") {
        &["location"]
    } else {
        &[]
    };
    for attr in writes {
        let Some(location) = node.attribute(*attr) else {
            continue;
        };
        let variable = written_variable(location);
        if parameters.iter().any(|p| p == variable) {
            return Err(c.refused(
                node,
                TestProfileRule::ModuleParameterWritten,
                location,
                format!(
                    "<{}> writes the module parameter '{variable}' ({attr}=\"{location}\"): a \
                     module parameter's value is the one the suite declares, and the component \
                     does not assign it (docs/SCE_TEST_PROFILE.md §11.5)",
                    node.tag_name().name()
                ),
            ));
        }
    }
    Ok(())
}

/// docs/SCE_TEST_PROFILE.md §4.6 (P6), §8.2, §11.5: a send has exactly
/// `type` (the test-port processor), `target` (a port of the component),
/// `sce:test-template` and `sce:template-args`, and no child.
fn send(node: &roxmltree::Node, c: &Component) -> Result<(), Located<ForgeError>> {
    let Some(send_type) = node.attribute("type").map(str::trim) else {
        return Err(c.missing(node, "<send>", "type"));
    };
    if send_type != TEST_PORT_PROCESSOR {
        return Err(c.refused(
            node,
            TestProfileRule::Send,
            send_type,
            format!(
                "<send> type '{send_type}': a send of a test component leaves through one of \
                 its ports, type=\"{TEST_PORT_PROCESSOR}\" (docs/SCE_TEST_PROFILE.md §11.5)"
            ),
        ));
    }
    if let Some((attr, why)) = SEND_ATTRIBUTES_REFUSED
        .iter()
        .find(|(attr, _)| node.attribute(*attr).is_some())
    {
        return Err(c.refused(
            node,
            TestProfileRule::Send,
            attr,
            format!(
                "<send> writes {attr}: a send of a test component has exactly type, target, \
                 sce:test-template and sce:template-args, and {why} \
                 (docs/SCE_TEST_PROFILE.md §11.5)"
            ),
        ));
    }
    if let Some(child) = node
        .children()
        .find(|child| is_scxml(child, "param") || is_scxml(child, "content"))
    {
        let name = child.tag_name().name();
        return Err(c.refused_element(
            &child,
            TestProfileRule::Send,
            name,
            format!(
                "<send> holds <{name}>: the message a send carries is its template's value, \
                 the template's parameters bound by sce:template-args \
                 (docs/SCE_TEST_PROFILE.md §11.5)"
            ),
        ));
    }
    let Some(target) = node.attribute("target").map(str::trim) else {
        return Err(c.missing(node, "<send>", "target"));
    };
    if target == "#_internal" {
        return Err(c.refused(
            node,
            TestProfileRule::Send,
            target,
            "<send> target '#_internal' raises an internal event, which has no TTCN-3 form: a \
             test component writes states and eventless transitions for it \
             (docs/SCE_TEST_PROFILE.md §8.2)"
                .to_string(),
        ));
    }
    if !c.profile.ports.iter().any(|p| p.id == target) {
        return Err(c.unknown(node, "<send>", "target", target, c.ports()));
    }
    if sce_attribute(node, "test-template").is_none() {
        return Err(c.missing(node, "<send>", "sce:test-template"));
    }
    template_use(node, "<send>", c)
}

/// The template `node` names and the variables it binds to the template's
/// parameters: an imported template, and variables the component declares.
fn template_use(
    node: &roxmltree::Node,
    element: &str,
    c: &Component,
) -> Result<(), Located<ForgeError>> {
    if let Some(alias) = sce_attribute(node, "test-template") {
        if !c.templates.iter().any(|t| t == alias) {
            return Err(c.unknown(
                node,
                element,
                "sce:test-template",
                alias,
                c.templates.clone(),
            ));
        }
    }
    if let Some(args) = sce_attribute(node, "template-args") {
        if let Some(unknown) = args
            .split_whitespace()
            .find(|arg| !c.variables.iter().any(|v| v == arg))
        {
            return Err(c.unknown(
                node,
                element,
                "sce:template-args",
                unknown,
                c.variables.clone(),
            ));
        }
    }
    Ok(())
}

/// docs/SCE_TEST_PROFILE.md §4.9 (P9), §8.2, §11.5: a PTC is
/// `<invoke type="<test-component URI>" src id>`, with a literal `id` and
/// nothing else.
fn invoke(node: &roxmltree::Node, c: &Component) -> Result<(), Located<ForgeError>> {
    let Some(invoke_type) = node.attribute("type").map(str::trim) else {
        return Err(c.missing(node, "<invoke>", "type"));
    };
    if invoke_type != TEST_COMPONENT_INVOKE_TYPE {
        return Err(c.refused(
            node,
            TestProfileRule::Invoke,
            invoke_type,
            format!(
                "<invoke> type '{invoke_type}': a test component invokes PTCs alone, \
                 type=\"{TEST_COMPONENT_INVOKE_TYPE}\" (docs/SCE_TEST_PROFILE.md §8.2)"
            ),
        ));
    }
    let ptc_form = "a PTC is created with a literal id and a src, and is handed nothing \
                    (docs/SCE_TEST_PROFILE.md §11.5)";
    if let Some(attr) = INVOKE_ATTRIBUTES_REFUSED
        .iter()
        .find(|attr| node.attribute(**attr).is_some())
    {
        return Err(c.refused(
            node,
            TestProfileRule::Invoke,
            attr,
            format!("<invoke> writes {attr}: {ptc_form}"),
        ));
    }
    if let Some(child) = node.children().find(|child| {
        is_scxml(child, "param") || is_scxml(child, "content") || is_scxml(child, "finalize")
    }) {
        let name = child.tag_name().name();
        return Err(c.refused_element(
            &child,
            TestProfileRule::Invoke,
            name,
            format!("<invoke> holds <{name}>: {ptc_form}"),
        ));
    }
    required(node, "<invoke>", "id", c.diag_label)?;
    required(node, "<invoke>", "src", c.diag_label)?;
    Ok(())
}

/// docs/SCE_TEST_PROFILE.md §4.2 (P2), §8.2: a transition is one branch kind
/// (a receive, a timeout, a done branch, or eventless), and names only what
/// the component declares.
fn transition(node: &roxmltree::Node, c: &Component) -> Result<(), Located<ForgeError>> {
    let event = node
        .attribute("event")
        .map(str::trim)
        .filter(|e| !e.is_empty());
    let receive = sce_attribute(node, "receive");
    let timeout = sce_attribute(node, "timeout");
    let kinds: Vec<&str> = [
        ("event", event.is_some()),
        ("sce:receive", receive.is_some()),
        ("sce:timeout", timeout.is_some()),
    ]
    .into_iter()
    .filter_map(|(name, written)| written.then_some(name))
    .collect();
    if kinds.len() > 1 {
        return Err(c.refused(
            node,
            TestProfileRule::Branch,
            kinds[1],
            format!(
                "<transition> writes {}: a transition is one branch kind, and writes at most \
                 one of event, sce:receive and sce:timeout (docs/SCE_TEST_PROFILE.md §8.2)",
                kinds.join(" and ")
            ),
        ));
    }
    if receive.is_none() {
        if let Some(attr) = ["sce:test-template", "sce:template-args"]
            .into_iter()
            .find(|attr| sce_attribute(node, &attr["sce:".len()..]).is_some())
        {
            return Err(c.refused(
                node,
                TestProfileRule::Branch,
                attr,
                format!(
                    "<transition> writes {attr} and receives nothing: a template matches the \
                     message a receive branch takes, and a transition without sce:receive \
                     takes none (docs/SCE_TEST_PROFILE.md §4.2)"
                ),
            ));
        }
    }
    if let Some(port) = receive {
        if !c.profile.ports.iter().any(|p| p.id == port) {
            return Err(c.unknown(node, "<transition>", "sce:receive", port, c.ports()));
        }
        if sce_attribute(node, "test-template").is_none()
            && sce_attribute(node, "template-args").is_some()
        {
            return Err(c.refused(
                node,
                TestProfileRule::Branch,
                "sce:template-args",
                "<transition> binds sce:template-args and names no sce:test-template: the \
                 arguments are a template's, and a receive with no template takes any message \
                 of its port (docs/SCE_TEST_PROFILE.md §4.2)"
                    .to_string(),
            ));
        }
        template_use(node, "<transition>", c)?;
    }
    if let Some(timer) = timeout {
        if !c.profile.timers.iter().any(|t| t.id == timer) {
            return Err(c.unknown(node, "<transition>", "sce:timeout", timer, c.timers()));
        }
    }
    if let Some(event) = event {
        done_branch(node, event, c)?;
    }
    Ok(())
}

/// docs/SCE_TEST_PROFILE.md §4.2 (P2), §4.4 (P4), §8.2: the one `event` a
/// test component writes is `done.invoke.<id>` of an `<invoke>` of the
/// transition's own state, and the branch exits that state: the PTC stays
/// done (ES 201 873-4 §9.16), so a branch that kept the state active would
/// match again on every snapshot. It keeps the state active when it has no
/// target, or targets the state or a state inside it, as an internal
/// transition within it does.
fn done_branch(
    node: &roxmltree::Node,
    event: &str,
    c: &Component,
) -> Result<(), Located<ForgeError>> {
    let Some(state) = node.parent_element() else {
        return Ok(());
    };
    let invoked: Vec<&str> = state
        .children()
        .filter(|child| is_scxml(child, "invoke"))
        .filter_map(|invoke| invoke.attribute("id").map(str::trim))
        .collect();
    let descriptors: Vec<&str> = event.split_whitespace().collect();
    let done_of = match descriptors.as_slice() {
        [one] => one
            .strip_prefix("done.invoke.")
            .filter(|id| invoked.contains(id)),
        _ => None,
    };
    if done_of.is_none() {
        let invokes = if invoked.is_empty() {
            "invokes nothing".to_string()
        } else {
            format!("invokes '{}'", invoked.join("', '"))
        };
        return Err(c.refused(
            node,
            TestProfileRule::Branch,
            event,
            format!(
                "<transition event=\"{event}\">: a transition of a test component is a receive, \
                 timeout or done branch, or eventless, and the event of a done branch is \
                 done.invoke.<id> of an <invoke> of its own state, which {invokes} \
                 (docs/SCE_TEST_PROFILE.md §8.2)"
            ),
        ));
    }
    let state_id = state.attribute("id").unwrap_or_default();
    let stays = |what: String| -> String {
        format!(
            "<transition event=\"{event}\"> {what}, so '{state_id}' stays active: the PTC stays \
             done, and a done branch that does not exit the state that invoked it would match \
             again on every snapshot (docs/SCE_TEST_PROFILE.md §8.2)"
        )
    };
    let targets: Vec<&str> = node
        .attribute("target")
        .map(|t| t.split_whitespace().collect())
        .unwrap_or_default();
    if targets.is_empty() {
        return Err(c.refused_element(
            node,
            TestProfileRule::DoneBranch,
            "",
            stays("has no target".to_string()),
        ));
    }
    if let Some(target) = targets.into_iter().find(|target| {
        state.descendants().any(|inner| {
            crate::parser::is_scxml_state_element(&inner) && inner.attribute("id") == Some(*target)
        })
    }) {
        return Err(c.refused(
            node,
            TestProfileRule::DoneBranch,
            target,
            stays(format!(
                "targets '{target}', which is '{state_id}' or a state inside it"
            )),
        ));
    }
    Ok(())
}

/// docs/SCE_TEST_PROFILE.md §4.7 (P7), §11.5: a statement names a declared
/// timer, and a start gives a duration in whole microseconds, or starts a
/// timer that declares a default.
fn timer_statement(node: &roxmltree::Node, c: &Component) -> Result<(), Located<ForgeError>> {
    let element = format!("<sce:{}>", node.tag_name().name());
    let timer = node.attribute("timer").map(str::trim).unwrap_or_default();
    let Some(declared) = c.profile.timers.iter().find(|t| t.id == timer) else {
        return Err(c.unknown(node, &element, "timer", timer, c.timers()));
    };
    if !is_sce(node, "start-timer") {
        return Ok(());
    }
    match node.attribute("duration") {
        Some(duration) => c.duration(
            node,
            &format!("<sce:start-timer timer=\"{timer}\">"),
            duration,
        ),
        // The timer declares no default, so the start states the duration.
        None if declared.duration.is_none() => Err(c.missing(
            node,
            &format!("<sce:start-timer timer=\"{timer}\">"),
            "duration",
        )),
        None => Ok(()),
    }
}

// ── What the documents the component names say ─────────────────

/// The imports of one document and the folder they are written relative to:
/// what an alias that document writes names.
struct ImportScope<'a> {
    dir: &'a Path,
    imports: &'a [ForgeImport],
}

impl ImportScope<'_> {
    /// The document `alias` names, as one identity however it is reached
    /// ([`crate::forge::stdlib::identity`]): two documents that import the
    /// same file under two aliases name one document.
    fn document(&self, alias: &str) -> Option<String> {
        self.imports
            .iter()
            .find(|import| import.alias == alias)
            .map(|import| {
                crate::forge::stdlib::identity(&crate::forge::stdlib::resolve(
                    self.dir,
                    &import.src,
                ))
            })
    }
}

/// An imported document of the profile, where it was read from, and its own
/// imports.
struct Imported<T> {
    model: T,
    path: PathBuf,
    imports: Vec<ForgeImport>,
}

impl<T> Imported<T> {
    fn scope(&self) -> ImportScope<'_> {
        ImportScope {
            dir: self.path.parent().unwrap_or(Path::new("")),
            imports: &self.imports,
        }
    }
}

/// How a refusal of a document an import names is placed.
type Refuse<'a> = &'a dyn Fn(ForgeError) -> Located<ForgeError>;

/// The document `import` names, read from `dir` and parsed, held to the kind
/// the import declares; a document that cannot be read is refused by
/// `refuse`, and a defect of the document itself where it is, in its own
/// words.
fn read_imported(
    import: &ForgeImport,
    dir: &Path,
    refuse: Refuse<'_>,
) -> Result<(ForgeDocument, Vec<ForgeImport>, PathBuf), Located<ForgeError>> {
    let source = ImportSource::read(dir, import).map_err(|e| refuse(e.into()))?;
    let parsed = source.parse()?.ok_or_else(|| {
        refuse(
            ImportError::NotForge {
                src: import.src.clone(),
            }
            .into(),
        )
    })?;
    let actual = parsed.document.kind();
    if actual != import.kind {
        return Err(refuse(
            ImportError::KindMismatch {
                src: import.src.clone(),
                declared: import.kind.to_string(),
                actual: actual.to_string(),
            }
            .into(),
        ));
    }
    Ok((parsed.document, parsed.imports, source.path))
}

/// Whether a value of `a`, a type `a_scope`'s document writes, is a value of
/// `b`, a type `b_scope`'s document writes: one shape with the same bounds,
/// and an alias on each side that names one document.
fn same_type(a: &ValueType, a_scope: &ImportScope, b: &ValueType, b_scope: &ImportScope) -> bool {
    let one_document = |x: &str, y: &str| match (a_scope.document(x), b_scope.document(y)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    };
    match (a, b) {
        (TypeOf::Scalar(SceType::Enum(x)), TypeOf::Scalar(SceType::Enum(y))) => {
            one_document(&x.alias, &y.alias)
        }
        (TypeOf::Scalar(x), TypeOf::Scalar(y)) => x == y,
        (TypeOf::Text { max: x }, TypeOf::Text { max: y })
        | (TypeOf::Bytes { max: x }, TypeOf::Bytes { max: y }) => x.max == y.max && x.min == y.min,
        (TypeOf::Optional { elem: x }, TypeOf::Optional { elem: y }) => {
            same_type(x, a_scope, y, b_scope)
        }
        (
            TypeOf::List {
                elem: x,
                max: x_bound,
            },
            TypeOf::List {
                elem: y,
                max: y_bound,
            },
        ) => {
            x_bound.max == y_bound.max
                && x_bound.min == y_bound.min
                && same_type(x, a_scope, y, b_scope)
        }
        (TypeOf::Record { alias: x }, TypeOf::Record { alias: y })
        | (TypeOf::Union { alias: x }, TypeOf::Union { alias: y }) => one_document(x, y),
        _ => false,
    }
}

/// Which way a template's message travels on its port.
#[derive(Clone, Copy)]
enum Direction {
    /// A send: the port type's `out` list.
    Out,
    /// A receive: the port type's `in` list.
    In,
}

/// What a test component asks of the documents it names
/// (docs/SCE_TEST_PROFILE.md §6.2, §11.5), read from `dir`: every
/// `test-port-type` and `test-template` it imports is a document of that
/// kind; `sce:template-args` binds as many variables as the template has
/// formal parameters, each of its parameter's type; a template's record is on
/// its port's `out` list for a send and `in` list for a receive; a send's
/// template is specific; an `<invoke>`'s `src` is a test component; a
/// receive branch whose content reads the message imports the event-schema
/// of its template's record.
///
/// Returns, for each template a receive branch matches with, the alias the
/// component imports its record under ([`SCXMLModel::received_records`]), and
/// for each template the component imports, what the machine it is generated
/// into names of it ([`SCXMLModel::test_templates`]).
///
/// Nothing for a statechart that is no test component. A name
/// [`check_document`] refused never reaches here.
pub(crate) fn read_imports(
    root: &roxmltree::Node,
    model: &SCXMLModel,
    dir: &Path,
    diag_label: &str,
) -> Result<ImportedProfile, Located<ForgeError>> {
    let mut received = BTreeMap::new();
    let Some(profile) = &model.test_component else {
        return Ok(ImportedProfile::default());
    };
    let c = Component {
        document: &model.name,
        diag_label,
        profile,
        port_types: Vec::new(),
        templates: Vec::new(),
        variables: Vec::new(),
    };
    let component = ImportScope {
        dir,
        imports: &model.forge_imports,
    };

    let mut port_types: Vec<(&str, Imported<TestPortTypeModel>)> = Vec::new();
    let mut templates: Vec<(&str, Imported<TestTemplateModel>)> = Vec::new();
    for import in &model.forge_imports {
        if !matches!(
            import.kind,
            ForgeKind::TestPortType | ForgeKind::TestTemplate
        ) {
            continue;
        }
        let at_the_import = |error: ForgeError| Located::new(error, diag_label, import.line, None);
        let (document, imports, path) = read_imported(import, dir, &at_the_import)?;
        match document {
            ForgeDocument::TestPortType(port_type) => port_types.push((
                import.alias.as_str(),
                Imported {
                    model: port_type,
                    path,
                    imports,
                },
            )),
            ForgeDocument::TestTemplate(template) => templates.push((
                import.alias.as_str(),
                Imported {
                    model: template,
                    path,
                    imports,
                },
            )),
            _ => {}
        }
    }
    // What a machine names of each template: its unit, named by the file of
    // its document as every forge unit is (`DocumentLabel::for_input_path`),
    // and the event its record's messages travel under.
    let units: BTreeMap<String, TestTemplateUnit> = templates
        .iter()
        .filter_map(|(alias, template)| {
            let import = model.forge_imports.iter().find(|i| i.alias == *alias)?;
            let unit = Path::new(&import.src)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(&import.src)
                .to_string();
            let scope = template.scope();
            let types = crate::forge::import_source::type_scope(scope.imports, scope.dir);
            Some((
                alias.to_string(),
                TestTemplateUnit {
                    unit,
                    event: message_event(template),
                    union: crate::forge::test_profile::union_reached(
                        &template.model.template_type,
                        &types,
                    ),
                },
            ))
        })
        .collect();

    // Every variable of the component with the type it declares, bounded as
    // the data model holds it.
    let variables: Vec<(&str, Option<ValueType>)> = model
        .variables
        .iter()
        .chain(
            model
                .states
                .values()
                .flat_map(|state| state.datamodel.iter()),
        )
        .map(|var| {
            (
                var.id.as_str(),
                var.value_type.clone().and_then(|ty| ty.bounded().ok()),
            )
        })
        .collect();

    for node in own_elements(*root) {
        if is_scxml(&node, "invoke") {
            invoked_component(&node, dir, &c)?;
            continue;
        }
        let (port, direction) = if is_scxml(&node, "send") {
            (node.attribute("target").map(str::trim), Direction::Out)
        } else if is_scxml(&node, "transition") {
            (sce_attribute(&node, "receive"), Direction::In)
        } else {
            continue;
        };
        let (Some(port), Some(alias)) = (port, sce_attribute(&node, "test-template")) else {
            continue;
        };
        let Some((_, template)) = templates.iter().find(|(a, _)| *a == alias) else {
            continue;
        };
        template_arguments(&node, alias, template, &variables, &component, &c)?;
        let port_type = profile
            .ports
            .iter()
            .find(|p| p.id == port)
            .and_then(|p| port_types.iter().find(|(a, _)| *a == p.port_type));
        if let Some((port_type_alias, port_type)) = port_type {
            port_direction(
                &node,
                (alias, template),
                (port, *port_type_alias, port_type),
                direction,
                &c,
            )?;
        }
        if matches!(direction, Direction::In) {
            match received_record(template, &component) {
                Some(record) => {
                    received.insert(alias.to_string(), record);
                }
                None if content_reads_message(&node) => {
                    return Err(c.refused(
                        &node,
                        TestProfileRule::MessageRead,
                        alias,
                        format!(
                            "the receive branch reads the message it takes with the template \
                             '{alias}', of {}, whose event-schema the component does not \
                             import: a component reads a message through the record it \
                             imports, as a value redirect stores into a variable of a type \
                             its module imports (ES 201 873-1 §8.2.3, §22.2.2; \
                             docs/SCE_TEST_PROFILE.md §11.5)",
                            template.model.template_type
                        ),
                    ));
                }
                None => {}
            }
        }
        if matches!(direction, Direction::Out) {
            let scope = template.scope();
            if !crate::forge::test_template_check::is_specific(
                &template.model,
                scope.imports,
                scope.dir,
            ) {
                return Err(c.refused(
                    &node,
                    TestProfileRule::SpecificTemplate,
                    alias,
                    format!(
                        "<send> sends the template '{alias}', which holds a matching mechanism, \
                         itself or in a template it references: a send's template is specific, \
                         values alone, every parameter bound (docs/SCE_TEST_PROFILE.md §6.2)"
                    ),
                ));
            }
        }
    }
    Ok(ImportedProfile {
        received_records: received,
        templates: units,
    })
}

/// What [`read_imports`] reads of the documents a test component names, for
/// the model ([`SCXMLModel::received_records`], [`SCXMLModel::test_templates`]).
#[derive(Debug, Default)]
pub(crate) struct ImportedProfile {
    /// For each template a receive branch matches with, the alias the
    /// component imports its record under.
    pub received_records: BTreeMap<String, String>,
    /// For each template the component imports, by its alias, its unit and
    /// the event of its record.
    pub templates: BTreeMap<String, TestTemplateUnit>,
}

/// The event name a message of `template`'s record travels under: the
/// `sce:event-name` of the event schema the template's own document imports
/// as its record. `None` for a template of no record, and for a record whose
/// document cannot be read as an event schema, which generating the template
/// itself refuses where it is read.
fn message_event(template: &Imported<TestTemplateModel>) -> Option<String> {
    let TypeOf::Record { alias } = &template.model.template_type else {
        return None;
    };
    let scope = template.scope();
    let import = scope.imports.iter().find(|i| i.alias == *alias)?;
    let source = ImportSource::read(scope.dir, import).ok()?;
    match source.parse().ok()??.document {
        ForgeDocument::EventSchema(schema) => Some(schema.event_name),
        _ => None,
    }
}

/// The alias under which `component` imports the event-schema that is the
/// record of `template`, or `None` when the template is of no record or the
/// component imports its record under no alias. The record is the one
/// document both name, whatever alias each gives it.
fn received_record(
    template: &Imported<TestTemplateModel>,
    component: &ImportScope,
) -> Option<String> {
    let TypeOf::Record { alias: record } = &template.model.template_type else {
        return None;
    };
    let record = template.scope().document(record)?;
    let found = component
        .imports
        .iter()
        .filter(|import| matches!(import.kind, ForgeKind::EventSchema))
        .find(|import| component.document(&import.alias).as_deref() == Some(record.as_str()));
    found.map(|import| import.alias.clone())
}

/// docs/SCE_TEST_PROFILE.md §11.5: `sce:template-args` lists one variable of
/// the component per formal parameter of the template, in order, each of its
/// parameter's type.
fn template_arguments(
    node: &roxmltree::Node,
    alias: &str,
    template: &Imported<TestTemplateModel>,
    variables: &[(&str, Option<ValueType>)],
    component: &ImportScope,
    c: &Component,
) -> Result<(), Located<ForgeError>> {
    let written = sce_attribute(node, "template-args").unwrap_or_default();
    let args: Vec<&str> = written.split_whitespace().collect();
    let parameters = &template.model.parameters;
    let site = format!("<{}>", node.tag_name().name());
    if args.len() != parameters.len() {
        return Err(c.refused(
            node,
            TestProfileRule::TemplateParameter,
            if written.is_empty() { alias } else { written },
            format!(
                "{site} binds {} variable(s) to the template '{alias}', which takes {} formal \
                 parameter(s): sce:template-args lists one variable of the component per \
                 formal parameter, in order (docs/SCE_TEST_PROFILE.md §11.5)",
                args.len(),
                parameters.len()
            ),
        ));
    }
    let scope = template.scope();
    for (arg, parameter) in args.into_iter().zip(parameters) {
        let declared = variables
            .iter()
            .find(|(id, _)| *id == arg)
            .and_then(|(_, ty)| ty.as_ref());
        if declared.is_some_and(|ty| same_type(ty, component, &parameter.parameter_type, &scope)) {
            continue;
        }
        let held = declared.map_or_else(|| "no type".to_string(), ToString::to_string);
        return Err(c.refused(
            node,
            TestProfileRule::TemplateParameter,
            arg,
            format!(
                "{site} binds the variable '{arg}', of {held}, to the formal parameter '{}' of \
                 the template '{alias}', of {}: each variable is of its parameter's type \
                 (docs/SCE_TEST_PROFILE.md §11.5)",
                parameter.id, parameter.parameter_type
            ),
        ));
    }
    Ok(())
}

/// docs/SCE_TEST_PROFILE.md §11.3, §11.5: the record a template is of is on
/// its port type's `out` list when it is sent, and on its `in` list when it
/// is received.
fn port_direction(
    node: &roxmltree::Node,
    (alias, template): (&str, &Imported<TestTemplateModel>),
    (port, port_type_alias, port_type): (&str, &str, &Imported<TestPortTypeModel>),
    direction: Direction,
    c: &Component,
) -> Result<(), Located<ForgeError>> {
    let (listed, list, verb): (Vec<&str>, &str, &str) = match direction {
        Direction::Out => (port_type.model.out_list().collect(), "out", "sends"),
        Direction::In => (port_type.model.in_list().collect(), "in", "receives"),
    };
    let record = match &template.model.template_type {
        TypeOf::Record { alias } => template.scope().document(alias),
        _ => None,
    };
    let ports_scope = port_type.scope();
    let on_the_list = record.as_deref().is_some_and(|record| {
        listed
            .iter()
            .any(|message| ports_scope.document(message).as_deref() == Some(record))
    });
    if on_the_list {
        return Ok(());
    }
    let names = if listed.is_empty() {
        "no message".to_string()
    } else {
        listed.join(", ")
    };
    Err(c.refused(
        node,
        TestProfileRule::PortDirection,
        alias,
        format!(
            "<{}> {verb} the template '{alias}', of {}, on the port '{port}', whose type \
             '{port_type_alias}' has {names} on its {list} list: a message a component {verb} \
             is on its port's {list} list (docs/SCE_TEST_PROFILE.md §11.5)",
            node.tag_name().name(),
            template.model.template_type
        ),
    ))
}

/// docs/SCE_TEST_PROFILE.md §4.9 (P9), §11.5: the `src` of a PTC's
/// invocation is a test component, read from `dir`.
fn invoked_component(
    node: &roxmltree::Node,
    dir: &Path,
    c: &Component,
) -> Result<(), Located<ForgeError>> {
    let Some(src) = node.attribute("src").map(str::trim) else {
        return Ok(());
    };
    let source = ImportSource::read_src(dir, src)
        .map_err(|error| crate::forge::parser::located(node, c.diag_label, error))?;
    let what = match crate::forge::parser::detect_kind(&source.content) {
        Ok(Some(ForgeKind::TestComponent)) => return Ok(()),
        Ok(Some(kind)) => format!("a document of kind {kind}"),
        Ok(None) => "a statechart".to_string(),
        Err(_) => "no document SCE reads".to_string(),
    };
    Err(c.refused(
        node,
        TestProfileRule::Invoke,
        src,
        format!(
            "<invoke> src '{src}' is {what}: a PTC is a test component \
             (docs/SCE_TEST_PROFILE.md §4.9, §11.5)"
        ),
    ))
}

/// docs/adr/0091 §3: a test component needs no script engine, so the
/// manifest of one shows `needs_script_engine = false`. The analysis of the
/// parsed model ([`crate::script_engine_analyzer`]) names each cause; the
/// first is refused where it was written.
///
/// Every cause the analysis names under `sce-static` is a construct
/// [`check_document`] refuses first today; this holds the manifest to the
/// rule whatever cause the analysis learns to name.
pub(crate) fn refuse_script_engine(
    model: &SCXMLModel,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if model.test_component.is_none() {
        return Ok(());
    }
    let Some(cause) = model.script_engine_causes.first() else {
        return Ok(());
    };
    let kind = cause.to_wire().kind;
    let at = cause.location.as_ref();
    Err(Located::new(
        ValidationError::TestProfile {
            rule: TestProfileRule::ScriptEngine,
            kind: ForgeKind::TestComponent,
            document: model.name.clone(),
            actual: kind.to_string(),
            detail: format!(
                "needs a script engine ({kind}): a test component runs on none, so that every \
                 runtime and the TTCN-3 export read one test (docs/adr/0091 §3)"
            ),
        }
        .into(),
        diag_label,
        at.and_then(|l| l.line),
        at.and_then(|l| l.col),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::diagnostic::ToDiagnostics;
    use crate::forge::value_type::Bound;
    use crate::script_engine_analyzer::{NeedsScriptEngineCause, ScriptEngineCauseKind};

    fn import(alias: &str, src: &str) -> ForgeImport {
        crate::forge::parser::parse_imports_only(
            &format!(
                "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
                 xmlns:sce=\"http://sce.dev/ext\" version=\"1.0\">\
                 <sce:import kind=\"enum\" src=\"{src}\" as=\"{alias}\"/></scxml>"
            ),
            "test.scxml",
        )
        .expect("one import")
        .remove(0)
    }

    /// docs/adr/0091 §3: a test component with any cause the analysis names
    /// is refused with the cause's wire kind; one with none, and a statechart
    /// that is no test component, are not.
    #[test]
    fn a_test_component_that_needs_a_script_engine_is_refused_by_its_cause() {
        let cause = NeedsScriptEngineCause {
            kind: ScriptEngineCauseKind::GlobalScript,
            location: None,
        };
        let mut model = SCXMLModel {
            name: "job_tester".to_string(),
            test_component: Some(TestComponentProfile::default()),
            ..Default::default()
        };
        assert!(refuse_script_engine(&model, "job_tester.scxml").is_ok());
        model.script_engine_causes = vec![cause];
        let err = refuse_script_engine(&model, "job_tester.scxml").expect_err("refused");
        let record = &err.error.to_diagnostics()[0];
        assert_eq!(record.code.as_str(), "test-profile/script-engine");
        assert_eq!(record.actual.as_deref(), Some("global-script"));
        model.test_component = None;
        assert!(refuse_script_engine(&model, "job_tester.scxml").is_ok());
    }

    /// `_event` is read when the token names it, not when a string or a
    /// member of another value spells it.
    #[test]
    fn an_expression_reads_the_event_only_by_its_own_name() {
        use crate::forge::expr::references_event_lexically;
        assert!(references_event_lexically("_event.data.count > 0"));
        assert!(references_event_lexically("_event"));
        assert!(!references_event_lexically("'_event.data'"));
        assert!(!references_event_lexically("limit._event"));
        assert!(!references_event_lexically("limit > 0"));
    }

    /// Two types agree by shape and bounds, and an alias on each side agrees
    /// when the two name one document, whatever each alias is.
    #[test]
    fn a_type_agrees_by_shape_and_by_the_document_an_alias_names() {
        let dir = Path::new("/fixtures");
        let ours = [import("Mode", "mode.scxml")];
        let theirs = [
            import("JobMode", "mode.scxml"),
            import("Other", "other.scxml"),
        ];
        let a = ImportScope {
            dir,
            imports: &ours,
        };
        let b = ImportScope {
            dir,
            imports: &theirs,
        };
        let enum_of = |alias: &str| -> ValueType {
            TypeOf::Scalar(SceType::Enum(crate::forge::model::EnumRef {
                alias: alias.to_string(),
            }))
        };
        assert!(same_type(&enum_of("Mode"), &a, &enum_of("JobMode"), &b));
        assert!(!same_type(&enum_of("Mode"), &a, &enum_of("Other"), &b));
        assert!(!same_type(&enum_of("Unknown"), &a, &enum_of("JobMode"), &b));
        let uint32: ValueType = TypeOf::Scalar(SceType::Uint32);
        assert!(same_type(&uint32, &a, &uint32, &b));
        assert!(!same_type(
            &uint32,
            &a,
            &TypeOf::Scalar(SceType::Uint16),
            &b
        ));
        let text = |max: u32| -> ValueType {
            TypeOf::Text {
                max: Bound::of(max),
            }
        };
        assert!(same_type(&text(8), &a, &text(8), &b));
        assert!(!same_type(&text(8), &a, &text(16), &b));
        assert!(!same_type(
            &text(8),
            &a,
            &TypeOf::Optional {
                elem: Box::new(text(8))
            },
            &b
        ));
    }

    #[test]
    fn a_location_writes_the_variable_it_starts_with() {
        assert_eq!(written_variable("limit"), "limit");
        assert_eq!(written_variable(" limit.field "), "limit");
        assert_eq!(written_variable("items[0]"), "items");
    }
}
