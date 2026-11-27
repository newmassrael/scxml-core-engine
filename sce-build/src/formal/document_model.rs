// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A document's model over Appendix D (docs/adr/0115, D7): the module
//! `sce-codegen model` writes, which extends `docs/formal/AppendixD.tla` and
//! gives each of its constants by a definition of the document's own.
//!
//! The tables are the ones the generated code is printed from
//! ([`DocumentTables::of`]), in the code's numbering: row `r` of
//! `DocTransitions` is the transition the code numbers `r - 1`, so a run that
//! tells its transitions by number is read against the model with no table
//! between them (D8). Each condition and each block of executable content is
//! written as the operator Appendix D takes it as (`Guard`, `Execute`), over
//! a finite domain: the integers and the booleans a document's `<data>` start
//! as.
//!
//! What has no translation into that domain is refused by name, every such
//! construct of the document at once, and the document then has no model:
//! never a model of part of it (D7). The constructs translated, and the ones
//! refused, are the closed list [`Construct`] names.

use std::collections::BTreeMap;
use std::fmt;

use crate::document_tables::{DocumentTables, StateKind, TargetRef, TransitionOrigin};
use crate::ecmascript::parser::parse_expression;
use crate::ecmascript::{BinOp, Expr, ExprKind, LogicalOp, UnaryOp};
use crate::event_descriptor::EventDescriptor;
use crate::forge::error::SourceLocation;
use crate::model::{Action, Datamodel, SCXMLModel};

use super::FormalError;

/// The module every document's model extends: Appendix D, once.
pub const SEMANTICS: &str = "AppendixD";

/// The text of [`SEMANTICS`], compiled in: the semantics the models this
/// module writes extend is the generator's, so a judge that runs one stages
/// the module it was written against, wherever it runs.
pub const SEMANTICS_TEXT: &str = include_str!("../../../docs/formal/AppendixD.tla");

/// The prefix of the definitions a document's own module gives the constants
/// of [`SEMANTICS`] by (`DocStates` for `States`).
pub const DOCUMENT_PREFIX: &str = "Doc";

/// The modules a document's model may not be named as: the semantics, and the
/// standard modules TLC finds before any module beside it.
const TAKEN_MODULE_NAMES: [&str; 6] = [
    SEMANTICS,
    "Naturals",
    "Integers",
    "Sequences",
    "FiniteSets",
    "TLC",
];

/// The words TLA+ keeps, which no name of a module or a field may be.
const TLA_WORDS: [&str; 52] = [
    "ACTION",
    "ASSUME",
    "ASSUMPTION",
    "AXIOM",
    "BOOLEAN",
    "BY",
    "CASE",
    "CHOOSE",
    "CONSTANT",
    "CONSTANTS",
    "COROLLARY",
    "DEF",
    "DEFINE",
    "DEFS",
    "DOMAIN",
    "ELSE",
    "ENABLED",
    "EXCEPT",
    "EXTENDS",
    "FALSE",
    "HAVE",
    "HIDE",
    "IF",
    "IN",
    "INSTANCE",
    "LAMBDA",
    "LEMMA",
    "LET",
    "LOCAL",
    "MODULE",
    "NEW",
    "OBVIOUS",
    "OMITTED",
    "ONLY",
    "OTHER",
    "PICK",
    "PROOF",
    "PROPOSITION",
    "PROVE",
    "QED",
    "RECURSIVE",
    "STATE",
    "STRING",
    "SUBSET",
    "SUFFICES",
    "TAKE",
    "TEMPORAL",
    "THEN",
    "THEOREM",
    "TRUE",
    "UNCHANGED",
    "UNION",
];

/// The `<history>` children of each state, by the state's number in the tables:
/// where each is written in the document (row and column), and its id.
type HistoriesByState = BTreeMap<u16, Vec<((u32, u32), String)>>;

/// What [`write`] is asked for beside the document.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options<'a> {
    /// The module's name; the machine's name when none is given.
    pub module: Option<&'a str>,
    /// Write the verdict the W3C corpus judges a test by as two properties of
    /// the module: `NeverFails`, the session never ends in the top-level
    /// final state `fail`, and `Passes`, it ends in `pass`.
    pub w3c_verdict: bool,
}

/// A document's model: the module's name and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentModel {
    pub module: String,
    pub text: String,
}

/// A construct with no translation into the model, by kind. The list is
/// closed: what is not refused here is translated, and a construct the
/// generator meets that it neither translates nor names is refused as an
/// action or an expression it has no translation for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Construct {
    /// A data model whose expressions have no translation here (`sce-static`).
    Datamodel(&'static str),
    /// `binding="late"`: the model binds every `<data>` when it starts.
    LateBinding,
    /// Feature conditions (ADR 0057): a model is of one variant.
    FeatureConditions,
    /// A test component (docs/SCE_TEST_PROFILE.md), whose profile the model
    /// does not run.
    TestComponent,
    /// An `<invoke>`: the model runs one session.
    Invoke,
    /// A `<donedata>`: the model's events carry names and no data.
    DoneData,
    /// An `<sce:on-sample>` block.
    OnSample,
    /// A `<datamodel>` inside a state, whose place in the order the data are
    /// bound in the model does not know.
    StateDatamodel,
    /// A state id holding a `.`: Appendix D names a done event by the id as
    /// one token, and a descriptor of it would be split into several.
    DottedStateId(String),
    /// A text a TLA+ string cannot hold: a character outside printable ASCII.
    Unspellable(String),
    /// A name that is no TLA+ identifier, or is a word TLA+ keeps, or names a
    /// module the model cannot be.
    Name(String),
    /// The tables of the document could not be built.
    Tables(String),
    /// A `<data>` the model cannot start: `why` says what it lacks.
    Data(&'static str),
    /// An element of executable content with no translation, by its type.
    Action(String),
    /// A `<send>` with a part the model has no translation for.
    Send(&'static str),
    /// An `<assign>` whose location names no `<data>` of the document.
    AssignLocation(String),
    /// An expression outside the integers and booleans, or one that reads
    /// what the model does not hold.
    Expression { text: String, why: String },
    /// An event name or descriptor with an empty token.
    EventName(String),
    /// The W3C verdict asked of a document with no top-level final state of
    /// this id.
    W3cVerdict(&'static str),
    /// A `<send>` of an observer, which is read by nothing but the properties
    /// and sends nothing (docs/adr/0115, D15).
    ObserverSend,
}

impl fmt::Display for Construct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Construct::Datamodel(name) => {
                write!(
                    f,
                    "the data model `{name}`, whose expressions have no translation"
                )
            }
            Construct::LateBinding => {
                f.write_str("late binding, where the model binds every <data> when it starts")
            }
            Construct::FeatureConditions => f.write_str(
                "feature conditions (ADR 0057): a model is of one variant, written as a plain \
                 document by `sce-codegen project`",
            ),
            Construct::TestComponent => {
                f.write_str("a test component, whose profile the model does not run")
            }
            Construct::Invoke => f.write_str("an <invoke>: the model runs one session"),
            Construct::DoneData => f.write_str("a <donedata>: the model's events carry no data"),
            Construct::OnSample => f.write_str("an <sce:on-sample> block"),
            Construct::StateDatamodel => f.write_str("a <datamodel> inside a state"),
            Construct::DottedStateId(id) => write!(
                f,
                "the state id `{id}` holds a `.`, and Appendix D names a done event by the id \
                 as one token"
            ),
            Construct::Unspellable(text) => {
                write!(f, "`{text}` holds a character a TLA+ string cannot")
            }
            Construct::Name(name) => write!(
                f,
                "`{name}` is no TLA+ identifier, is a word TLA+ keeps, or names a module the \
                 model cannot be"
            ),
            Construct::Tables(why) => write!(f, "the tables: {why}"),
            Construct::Data(why) => write!(f, "a <data> with {why}"),
            Construct::Action(element) => write!(f, "<{element}>, which has no translation"),
            Construct::Send(part) => write!(f, "a <send> with {part}"),
            Construct::AssignLocation(location) => write!(
                f,
                "<assign location=\"{location}\">, which names no <data> of the document"
            ),
            Construct::Expression { text, why } => write!(f, "`{text}`: {why}"),
            Construct::EventName(name) => write!(f, "the event `{name}` has an empty token"),
            Construct::W3cVerdict(id) => write!(
                f,
                "the W3C verdict, and the document has no top-level <final id=\"{id}\">"
            ),
            Construct::ObserverSend => f.write_str(
                "a <send>, and an observer sends nothing: it is read by the properties alone",
            ),
        }
    }
}

/// One construct refused, and where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub place: String,
    pub construct: Construct,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.place, self.construct)
    }
}

/// The model of `model`, read from `document` (the path the module names as
/// its source), or every construct of it that has no translation.
pub fn write(
    model: &SCXMLModel,
    document: &str,
    options: Options<'_>,
) -> Result<DocumentModel, FormalError> {
    write_with_facts(model, document, options).map(|(written, _)| written)
}

/// What a module beside a document's own reads of the document by name
/// (docs/adr/0115, D15): its states and its data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Facts {
    /// The id of every state, in the order the tables number them; a
    /// `<history>` is no state a configuration holds, and is not among them.
    pub(super) states: Vec<String>,
    /// Each `<data>`'s id and type, in document order.
    pub(super) data: Vec<(String, Ty)>,
}

impl Facts {
    fn of(writer: &Writer<'_>) -> Self {
        Self {
            states: writer
                .tables
                .states
                .iter()
                .map(|row| row.id.clone())
                .collect(),
            data: writer.variables.clone(),
        }
    }
}

/// [`write`], and the [`Facts`] of the document a module beside the model
/// reads it by.
pub(super) fn write_with_facts(
    model: &SCXMLModel,
    document: &str,
    options: Options<'_>,
) -> Result<(DocumentModel, Facts), FormalError> {
    let refused = |refusals: Vec<Refusal>| FormalError::NoTranslation {
        document: document.to_string(),
        refusals: refusals.iter().map(Refusal::to_string).collect(),
    };
    let tables = DocumentTables::of(model).map_err(|why| {
        refused(vec![Refusal {
            place: "the document".to_string(),
            construct: Construct::Tables(why.to_string()),
        }])
    })?;
    let mut writer = Writer::new(model, &tables, Role::Machine);
    let module = options.module.unwrap_or(&model.name);
    writer.module_name(module);
    let text = writer.module(module, document, options.w3c_verdict);
    if writer.refusals.is_empty() {
        let facts = Facts::of(&writer);
        Ok((
            DocumentModel {
                module: module.to_string(),
                text,
            },
            facts,
        ))
    } else {
        Err(refused(writer.refusals))
    }
}

/// An observer's tables (docs/adr/0115, D15), as a module that carries it
/// beside a machine writes them.
#[derive(Debug, Clone)]
pub(super) struct ObserverTables {
    /// The definitions, each named `prefix` and the constant of
    /// [`SEMANTICS`] it gives.
    pub(super) text: String,
    pub(super) facts: Facts,
    /// Whether a condition or a block of it needs the integers below zero.
    pub(super) integers: bool,
}

/// The tables of the observer `model` under `prefix`, or every construct of
/// it with no translation. An observer is a statechart as a machine is and is
/// translated by the same rules, but for one: it is read by nothing but the
/// properties, so it sends nothing, and a `<send>` of it is refused.
pub(super) fn observer_tables(
    model: &SCXMLModel,
    prefix: &str,
) -> Result<ObserverTables, Vec<Refusal>> {
    let tables = DocumentTables::of(model).map_err(|why| {
        vec![Refusal {
            place: "the document".to_string(),
            construct: Construct::Tables(why.to_string()),
        }]
    })?;
    let mut writer = Writer::new(model, &tables, Role::Observer);
    writer.whole_document();
    writer.state_ids();
    let data0 = writer.data();
    let text = writer.tables(prefix, &data0);
    if writer.refusals.is_empty() {
        Ok(ObserverTables {
            text,
            facts: Facts::of(&writer),
            integers: writer.integers,
        })
    } else {
        Err(writer.refusals)
    }
}

/// What a document is translated as: a machine, or an observer of one, which
/// sends nothing (docs/adr/0115, D15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Machine,
    Observer,
}

/// The type of a value of the model's data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ty {
    Int,
    Bool,
    /// An event's name, the sequence of its tokens: what a property compares
    /// `_event.name` with (docs/adr/0115, D15). No `<data>` holds one.
    Name,
}

impl Ty {
    pub(super) fn word(self) -> &'static str {
        match self {
            Ty::Int => "an integer",
            Ty::Bool => "a boolean",
            Ty::Name => "an event name",
        }
    }
}

/// How a translated expression binds: an atom stands as an operand, a
/// compound one is parenthesised there, and a junction is parenthesised also
/// where it stands alone as the value of a `CASE` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Binding {
    Atom,
    Compound,
    Junction,
}

/// A translated expression.
#[derive(Debug, Clone)]
pub(super) struct Tla {
    pub(super) text: String,
    pub(super) ty: Ty,
    pub(super) binding: Binding,
}

impl Tla {
    pub(super) fn atom(text: String, ty: Ty) -> Self {
        Self {
            text,
            ty,
            binding: Binding::Atom,
        }
    }

    /// As the operand of an operator.
    pub(super) fn operand(&self) -> String {
        match self.binding {
            Binding::Atom => self.text.clone(),
            Binding::Compound | Binding::Junction => format!("({})", self.text),
        }
    }

    /// As the value of a `CASE` arm or of a definition.
    pub(super) fn alone(&self) -> String {
        match self.binding {
            Binding::Atom | Binding::Compound => self.text.clone(),
            Binding::Junction => format!("({})", self.text),
        }
    }
}

/// The handlers of a state, `<onentry>` or `<onexit>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Handler {
    Entry,
    Exit,
}

impl Handler {
    fn element(self) -> &'static str {
        match self {
            Handler::Entry => "onentry",
            Handler::Exit => "onexit",
        }
    }
}

/// The largest integer TLC holds, which a literal of the model may not pass.
const LARGEST_INTEGER: u64 = i32::MAX as u64;

/// Collects the operators of the module and every refusal met on the way.
struct Writer<'m> {
    model: &'m SCXMLModel,
    tables: &'m DocumentTables,
    refusals: Vec<Refusal>,
    /// Each `<data>`'s id and type, in document order.
    variables: Vec<(String, Ty)>,
    /// Each condition's key and its translation, in the order the rows name
    /// them.
    guards: Vec<(String, String)>,
    /// Each block's name and its `Effect`, in the order the module names them.
    blocks: Vec<(String, String)>,
    /// Whether an expression needs the integers below zero (`Integers`).
    integers: bool,
    role: Role,
}

impl<'m> Writer<'m> {
    fn new(model: &'m SCXMLModel, tables: &'m DocumentTables, role: Role) -> Self {
        Self {
            model,
            tables,
            refusals: Vec::new(),
            variables: Vec::new(),
            guards: Vec::new(),
            blocks: Vec::new(),
            integers: false,
            role,
        }
    }

    fn refuse(&mut self, place: impl Into<String>, construct: Construct) {
        self.refusals.push(Refusal {
            place: place.into(),
            construct,
        });
    }

    fn module_name(&mut self, module: &str) {
        if !is_tla_identifier(module) || TAKEN_MODULE_NAMES.contains(&module) {
            self.refuse("the module", Construct::Name(module.to_string()));
        }
    }

    /// `text` as a TLA+ string, refused where a character of it has no place
    /// in one.
    fn string(&mut self, place: &str, text: &str) -> String {
        if !text.chars().all(|c| (' '..='~').contains(&c)) {
            self.refuse(place, Construct::Unspellable(text.to_string()));
        }
        tla_string(text)
    }

    /// An event's name as the sequence of its tokens.
    fn event_name(&mut self, place: &str, name: &str) -> String {
        if name.split('.').any(str::is_empty) {
            self.refuse(place, Construct::EventName(name.to_string()));
        }
        let tokens: Vec<String> = name
            .split('.')
            .map(|token| self.string(place, token))
            .collect();
        format!("<<{}>>", tokens.join(", "))
    }

    /// The descriptors of a transition's `event` attribute, as Appendix D's
    /// `NameMatch` reads them: `<<"*">>` for one that matches every event,
    /// else the tokens of its prefix, as [`EventDescriptor`] reduces it.
    fn descriptors(&mut self, place: &str, attribute: &str) -> String {
        let written: Vec<String> = attribute
            .split_whitespace()
            .map(|token| match EventDescriptor::parse(token) {
                EventDescriptor::Any => "<<\"*\">>".to_string(),
                EventDescriptor::Prefix(prefix) => self.event_name(place, prefix),
            })
            .collect();
        format!("{{{}}}", written.join(", "))
    }

    /// The ids of the states and the `<history>` elements, each of which the
    /// module writes as a TLA+ string.
    fn state_ids(&mut self) {
        let model = self.model;
        for id in model.states.keys() {
            if let Some(construct) = state_id_refusal(id) {
                self.refuse(format!("state `{id}`"), construct);
            }
        }
        for id in model.history_states.keys() {
            self.string(&format!("<history> `{id}`"), id);
        }
    }

    /// What the whole document or one of its states carries that the model
    /// does not run.
    fn whole_document(&mut self) {
        let model = self.model;
        if model.datamodel == Datamodel::SceStatic {
            self.refuse(
                "the document",
                Construct::Datamodel(Datamodel::SceStatic.as_str()),
            );
        }
        if model.binding == "late" {
            self.refuse("the document", Construct::LateBinding);
        }
        if model.product_line.is_some() {
            self.refuse("the document", Construct::FeatureConditions);
        }
        if model.test_component.is_some() {
            self.refuse("the document", Construct::TestComponent);
        }
        for script in &model.global_scripts {
            self.refuse(
                "the document",
                Construct::Action(script.action_type.clone()),
            );
        }
        for (id, state) in &model.states {
            let place = format!("state `{id}`");
            if !state.invokes.is_empty() {
                self.refuse(place.clone(), Construct::Invoke);
            }
            if state.donedata.is_some() {
                self.refuse(place.clone(), Construct::DoneData);
            }
            if !state.on_sample_blocks.is_empty() {
                self.refuse(place.clone(), Construct::OnSample);
            }
            if !state.datamodel.is_empty() {
                self.refuse(place, Construct::StateDatamodel);
            }
        }
    }

    /// The document's `<data>`, each with the type of the literal it starts
    /// as, and `DocData0`.
    fn data(&mut self) -> String {
        let model = self.model;
        let mut fields = Vec::new();
        for variable in &model.variables {
            let place = format!("the <data> `{}`", variable.id);
            if !is_tla_identifier(&variable.id) {
                self.refuse(place.clone(), Construct::Name(variable.id.clone()));
            }
            if !variable.src.is_empty() || !variable.content.trim().is_empty() {
                self.refuse(
                    place,
                    Construct::Data("a value read from src or written as content"),
                );
                continue;
            }
            if variable.expr.trim().is_empty() {
                self.refuse(
                    place,
                    Construct::Data("no value, which the model's data do not hold"),
                );
                continue;
            }
            let Some(expr) = self.parsed(&place, &variable.expr) else {
                continue;
            };
            match self.expression(&expr, None) {
                Ok(value) => {
                    fields.push(format!("{} |-> {}", variable.id, value.alone()));
                    self.variables.push((variable.id.clone(), value.ty));
                }
                Err(why) => self.refuse(place, expression_refusal(&variable.expr, why)),
            }
        }
        if fields.is_empty() {
            "[v \\in {} |-> 0]".to_string()
        } else {
            format!("[{}]", fields.join(", "))
        }
    }

    fn type_of(&self, name: &str) -> Option<Ty> {
        self.variables
            .iter()
            .find(|(id, _)| id == name)
            .map(|(_, ty)| *ty)
    }

    /// `text` parsed as ECMAScript, or `None` with its refusal recorded.
    fn parsed(&mut self, place: &str, text: &str) -> Option<Expr> {
        match parse_expression(text) {
            Ok(expr) => Some(expr),
            Err(refusal) => {
                self.refuse(place, expression_refusal(text, refusal.to_string()));
                None
            }
        }
    }

    /// `expr` over the data named `data` (`None`: a `<data>` value, which
    /// reads no variable), or why it has no translation.
    fn expression(&mut self, expr: &Expr, data: Option<&str>) -> Result<Tla, String> {
        let mut scope = DataScope {
            variables: &self.variables,
            data,
            integers: false,
        };
        let translated = translate(expr, &mut scope);
        self.integers |= scope.integers;
        translated
    }

    /// The key of `cond` in `DocGuard`, its translation recorded once, or
    /// `NoCond` for a transition with none.
    fn condition(&mut self, place: &str, cond: &str) -> String {
        let key = cond.split_whitespace().collect::<Vec<_>>().join(" ");
        if key.is_empty() {
            return "NoCond".to_string();
        }
        let quoted = self.string(place, &key);
        if self.guards.iter().any(|(known, _)| *known == key) {
            return quoted;
        }
        if let Some(expr) = self.parsed(place, cond) {
            match self.expression(&expr, Some("data")) {
                Ok(guard) if guard.ty == Ty::Bool => self.guards.push((key, guard.alone())),
                Ok(guard) => self.refuse(
                    place,
                    expression_refusal(cond, format!("a condition that is {}", guard.ty.word())),
                ),
                Err(why) => self.refuse(place, expression_refusal(cond, why)),
            }
        }
        quoted
    }

    /// The name of the block `actions`, its `Effect` recorded, or
    /// `NoContent` for none.
    fn block(&mut self, place: &str, name: String, actions: &[&Action]) -> String {
        if actions.is_empty() {
            return "NoContent".to_string();
        }
        let quoted = self.string(place, &name);
        let effect = self.effect(place, actions);
        self.blocks.push((name, effect));
        quoted
    }

    /// What `actions` do, in order, as an `Effect` of `data` and `event`:
    /// each `<assign>` a datamodel after the one before, and the events each
    /// `<raise>` and `<send>` makes, in the order made.
    fn effect(&mut self, place: &str, actions: &[&Action]) -> String {
        let mut steps: Vec<String> = Vec::new();
        let mut current = "data".to_string();
        let mut raised: Vec<String> = Vec::new();
        let mut sent: Vec<String> = Vec::new();
        let mut later: Vec<String> = Vec::new();
        for action in actions {
            match action.action_type.as_str() {
                "assign" => {
                    if let Some(step) = self.assignment(place, action, &current) {
                        let name = format!("d{}", steps.len() + 1);
                        steps.push(format!("{name} == {step}"));
                        current = name;
                    }
                }
                "raise" => raised.push(self.event_name(place, &action.event)),
                "send" if self.role == Role::Observer => {
                    self.refuse(place, Construct::ObserverSend);
                }
                "send" => {
                    if let Some((event, delayed)) = self.send(place, action) {
                        if delayed {
                            later.push(event);
                        } else {
                            sent.push(event);
                        }
                    }
                }
                other => self.refuse(place, Construct::Action(other.to_string())),
            }
        }
        let sequence = |events: &[String]| format!("<<{}>>", events.join(", "));
        match steps.as_slice() {
            [] => format!(
                "Effect(data, {}, {}, {})",
                sequence(&raised),
                sequence(&sent),
                sequence(&later)
            ),
            [only] => {
                let value = only
                    .split_once(" == ")
                    .map_or(only.as_str(), |(_, value)| value);
                format!(
                    "Effect({value}, {}, {}, {})",
                    sequence(&raised),
                    sequence(&sent),
                    sequence(&later)
                )
            }
            _ => format!(
                "(LET {}\n              IN Effect({current}, {}, {}, {}))",
                steps.join("\n                  "),
                sequence(&raised),
                sequence(&sent),
                sequence(&later)
            ),
        }
    }

    /// One `<assign>` as the datamodel after it, `[data EXCEPT !.v = e]`.
    fn assignment(&mut self, place: &str, action: &Action, data: &str) -> Option<String> {
        let location = action.location.trim();
        let Some(ty) = self.type_of(location) else {
            self.refuse(place, Construct::AssignLocation(location.to_string()));
            return None;
        };
        if !action.content.trim().is_empty() || action.expr.trim().is_empty() {
            self.refuse(
                place,
                Construct::Action("assign with content and no expr".to_string()),
            );
            return None;
        }
        let expr = self.parsed(place, &action.expr)?;
        match self.expression(&expr, Some(data)) {
            Ok(value) if value.ty == ty => {
                Some(format!("[{data} EXCEPT !.{location} = {}]", value.alone()))
            }
            Ok(value) => {
                self.refuse(
                    place,
                    expression_refusal(
                        &action.expr,
                        format!(
                            "assigns {} to {}, which is {}",
                            value.ty.word(),
                            location,
                            ty.word()
                        ),
                    ),
                );
                None
            }
            Err(why) => {
                self.refuse(place, expression_refusal(&action.expr, why));
                None
            }
        }
    }

    /// One `<send>` as the event it sends and whether it is delayed: to the
    /// session's own external queue, by the SCXML Event I/O Processor, its
    /// event and delay written as literals and carrying no data.
    fn send(&mut self, place: &str, action: &Action) -> Option<(String, bool)> {
        let parts: [(bool, &'static str); 12] = [
            (
                action.event.is_empty() && action.eventexpr.is_empty(),
                "no event",
            ),
            (!action.eventexpr.is_empty(), "an eventexpr"),
            (
                !action.target.is_empty() || !action.targetexpr.is_empty(),
                "a target",
            ),
            (
                !(action.send_type.is_empty()
                    || action.send_type
                        == crate::host_processor_analyzer::SCXML_EVENT_PROCESSOR_TYPE)
                    || !action.typeexpr.is_empty(),
                "a type other than the SCXML Event I/O Processor",
            ),
            (!action.delayexpr.is_empty(), "a delayexpr"),
            (action.delay_invalid, "a delay that is no time"),
            (!action.idlocation.is_empty(), "an idlocation"),
            (!action.namelist.is_empty(), "a namelist"),
            (!action.params.is_empty(), "a <param>"),
            (
                !action.content.trim().is_empty() || !action.contentexpr.is_empty(),
                "a <content>",
            ),
            (!action.sendidexpr.is_empty(), "a sendidexpr"),
            (
                action.target_unsupported || action.send_type_unsupported,
                "a target or a type no processor serves",
            ),
        ];
        let mut refused = false;
        for (present, part) in parts {
            if present {
                self.refuse(place, Construct::Send(part));
                refused = true;
            }
        }
        if refused {
            return None;
        }
        Some((
            self.event_name(place, &action.event),
            !action.delay.trim().is_empty(),
        ))
    }

    /// Every state and `<history>` in document order: the states in the
    /// order the tables number them, and each `<history>` among the children
    /// of its state where it is written.
    fn document_order(&self) -> Vec<String> {
        let mut histories: HistoriesByState = BTreeMap::new();
        for row in &self.tables.histories {
            let at = self
                .model
                .history_states
                .get(&row.id)
                .map_or((u32::MAX, u32::MAX), |info| {
                    position(info.source_location.as_ref())
                });
            histories
                .entry(row.parent)
                .or_default()
                .push((at, row.id.clone()));
        }
        for written in histories.values_mut() {
            written.sort();
        }
        let mut order = Vec::new();
        for (number, row) in (0u16..).zip(&self.tables.states) {
            if row.parent.is_none() {
                self.visit(number, &histories, &mut order);
            }
        }
        order
    }

    fn visit(&self, number: u16, histories: &HistoriesByState, order: &mut Vec<String>) {
        let id = &self.tables.states[usize::from(number)].id;
        order.push(id.clone());
        let mut own = histories
            .get(&number)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .peekable();
        for (child, row) in (0u16..).zip(&self.tables.states) {
            if row.parent != Some(number) {
                continue;
            }
            let at = position(
                self.model
                    .states
                    .get(&row.id)
                    .and_then(|state| state.source_location.as_ref()),
            );
            while let Some((_, history)) = own.next_if(|(written, _)| *written < at) {
                order.push(history.clone());
            }
            self.visit(child, histories, order);
        }
        order.extend(own.map(|(_, history)| history.clone()));
    }

    /// The text of the module.
    fn module(&mut self, module: &str, document: &str, w3c_verdict: bool) -> String {
        self.whole_document();
        self.state_ids();
        let data0 = self.data();
        if w3c_verdict {
            for id in ["pass", "fail"] {
                let top_final = self
                    .model
                    .states
                    .get(id)
                    .is_some_and(|state| state.is_final && state.parent.is_none());
                if !top_final {
                    self.refuse("the document", Construct::W3cVerdict(id));
                }
            }
        }
        // Before the head: a condition that needs the integers below zero is
        // found while the tables are written.
        let tables = self.tables(DOCUMENT_PREFIX, &data0);

        let mut text = String::new();
        let line = |text: &mut String, content: &str| {
            text.push_str(content);
            text.push('\n');
        };
        line(&mut text, &format!("---- MODULE {module} ----"));
        for comment in [
            format!("\\* The document {document} as the tables of {SEMANTICS}"),
            "\\* (docs/adr/0115, D7), written by `sce-codegen model`: regenerate it, never"
                .to_string(),
            "\\* edit it. Row r of DocTransitions is the transition the generated code".to_string(),
            "\\* numbers r - 1 (sce-build/src/document_tables.rs): the written".to_string(),
            "\\* transitions by state in document order, then the initial transition of"
                .to_string(),
            "\\* each compound state, the default transition of each <history>, and the"
                .to_string(),
            "\\* document's own. A block of executable content is named by where it is".to_string(),
            "\\* written, and a condition by its expression.".to_string(),
        ] {
            line(&mut text, &comment);
        }
        if self.integers {
            line(&mut text, &format!("EXTENDS {SEMANTICS}, Integers"));
        } else {
            line(&mut text, &format!("EXTENDS {SEMANTICS}"));
        }
        line(&mut text, "");
        text.push_str(&tables);
        line(
            &mut text,
            "\\* Appendix D with every part in, over the same tables: what each part taken",
        );
        line(
            &mut text,
            "\\* out must break, beside the invariant it names.",
        );
        line(&mut text, &format!("Promise == INSTANCE {SEMANTICS} WITH"));
        line(
            &mut text,
            "    ExitOrder <- TRUE, EntryOrder <- TRUE, RemoveConflicts <- TRUE,",
        );
        line(
            &mut text,
            "    EnterAncestors <- TRUE, FillParallel <- TRUE, RecordHistory <- TRUE,",
        );
        line(
            &mut text,
            "    DeepHistory <- TRUE, DoneCompound <- TRUE, DoneParallel <- TRUE,",
        );
        line(&mut text, "    RunToCompletion <- TRUE");
        line(&mut text, "Refines == Promise!Spec");
        if w3c_verdict {
            line(&mut text, "");
            line(
                &mut text,
                "\\* The test's verdict: the session never ends in fail, and it ends in pass.",
            );
            line(&mut text, "NeverFails == outcome # \"fail\"");
            line(&mut text, "Passes == <>(outcome = \"pass\")");
        }
        line(&mut text, "====");
        text
    }

    /// The document's tables as Appendix D takes them, each a definition
    /// named `prefix` and the constant it gives (`DocStates` for `States`
    /// under the prefix `Doc`), each followed by an empty line: its states,
    /// their order, kinds and parents, its histories' types, its transitions
    /// by row, its handlers, `data0` its datamodel when it starts, and its
    /// conditions and blocks. A module of the document and a module that
    /// carries a second document beside it (an observer, `super::properties`)
    /// write the same tables under two prefixes.
    fn tables(&mut self, prefix: &str, data0: &str) -> String {
        let order = self.document_order();
        let quoted_order: Vec<String> = order.iter().map(|id| tla_string(id)).collect();
        let kinds = self.kinds(&order);
        let parents = self.parents(&order);
        let history_types = self.history_types();
        let rows = self.rows();
        let on_entry = self.handlers(&order, Handler::Entry);
        let on_exit = self.handlers(&order, Handler::Exit);
        let states = format!("{prefix}States");

        let mut text = String::new();
        let line = |text: &mut String, content: &str| {
            text.push_str(content);
            text.push('\n');
        };
        line(
            &mut text,
            &wrapped(&format!("{states} == {{"), &quoted_order, "}"),
        );
        line(&mut text, "");
        line(
            &mut text,
            &wrapped(&format!("{prefix}Order == <<"), &quoted_order, ">>"),
        );
        line(&mut text, "");
        line(
            &mut text,
            &format!(
                "{prefix}Kind == {}",
                function("s", &states, &kinds, "\"state\"")
            ),
        );
        line(&mut text, "");
        line(
            &mut text,
            &format!(
                "{prefix}Parent == {}",
                function("s", &states, &parents, "Root")
            ),
        );
        line(&mut text, "");
        let history_ids: Vec<String> = self
            .tables
            .histories
            .iter()
            .map(|row| tla_string(&row.id))
            .collect();
        line(
            &mut text,
            &format!(
                "{prefix}HistoryType == {}",
                function(
                    "h",
                    &format!("{{{}}}", history_ids.join(", ")),
                    &history_types,
                    "\"shallow\""
                )
            ),
        );
        line(&mut text, "");
        line(&mut text, &format!("{prefix}Transitions == <<"));
        let last = rows.len().saturating_sub(1);
        for (at, row) in rows.iter().enumerate() {
            let comma = if at == last { "" } else { "," };
            line(&mut text, &format!("    {row}{comma}"));
        }
        line(&mut text, ">>");
        line(&mut text, "");
        line(
            &mut text,
            &format!(
                "{prefix}OnEntry == {}",
                function("s", &states, &on_entry, "NoContent")
            ),
        );
        line(&mut text, "");
        line(
            &mut text,
            &format!(
                "{prefix}OnExit == {}",
                function("s", &states, &on_exit, "NoContent")
            ),
        );
        line(&mut text, "");
        line(&mut text, &format!("{prefix}Data0 == {data0}"));
        line(&mut text, "");
        if self.guards.is_empty() {
            line(
                &mut text,
                "\\* No transition of the document has a condition.",
            );
            line(
                &mut text,
                &format!("{prefix}Guard(cond, data, event, conf) == FALSE"),
            );
        } else {
            line(
                &mut text,
                &format!("{prefix}Guard(cond, data, event, conf) =="),
            );
            let arms: Vec<(String, String)> = self
                .guards
                .iter()
                .map(|(key, guard)| (format!("cond = {}", tla_string(key)), guard.clone()))
                .collect();
            push_case(&mut text, &arms);
        }
        line(&mut text, "");
        if self.blocks.is_empty() {
            line(&mut text, "\\* The document has no executable content.");
            line(
                &mut text,
                &format!("{prefix}Execute(block, data, event) == Effect(data, <<>>, <<>>, <<>>)"),
            );
        } else {
            line(
                &mut text,
                &format!("{prefix}Execute(block, data, event) =="),
            );
            let arms: Vec<(String, String)> = self
                .blocks
                .iter()
                .map(|(name, effect)| (format!("block = {}", tla_string(name)), effect.clone()))
                .collect();
            push_case(&mut text, &arms);
        }
        line(&mut text, "");
        text
    }

    fn kinds(&self, order: &[String]) -> Vec<(String, String)> {
        order
            .iter()
            .filter_map(|id| {
                let kind = if self.model.history_states.contains_key(id) {
                    "history"
                } else {
                    let number = self.tables.state_id(id)?;
                    match self.tables.states[usize::from(number)].kind {
                        StateKind::Atomic | StateKind::Compound => return None,
                        StateKind::Parallel => "parallel",
                        StateKind::Final => "final",
                    }
                };
                Some((id.clone(), tla_string(kind)))
            })
            .collect()
    }

    fn parents(&self, order: &[String]) -> Vec<(String, String)> {
        order
            .iter()
            .filter_map(|id| {
                let parent = match self.model.history_states.get(id) {
                    Some(info) => info.parent.clone(),
                    None => self.model.states.get(id)?.parent.clone()?,
                };
                Some((id.clone(), tla_string(&parent)))
            })
            .collect()
    }

    fn history_types(&self) -> Vec<(String, String)> {
        self.tables
            .histories
            .iter()
            .filter(|row| row.deep)
            .map(|row| (row.id.clone(), tla_string("deep")))
            .collect()
    }

    /// `OnEntry` or `OnExit`: each state with a block of that kind, its
    /// handlers in document order as one block: no action the model translates
    /// can fail, so no handler stops before the next one runs.
    fn handlers(&mut self, order: &[String], handler: Handler) -> Vec<(String, String)> {
        let model = self.model;
        let kind = handler.element();
        let mut found = Vec::new();
        for id in order {
            let Some(state) = model.states.get(id) else {
                continue;
            };
            let blocks = match handler {
                Handler::Entry => &state.on_entry_blocks,
                Handler::Exit => &state.on_exit_blocks,
            };
            let actions: Vec<&Action> = blocks.iter().flatten().collect();
            if actions.is_empty() {
                continue;
            }
            let place = format!("the <{kind}> of state `{id}`");
            let name = self.block(&place, format!("{kind} {id}"), &actions);
            found.push((id.clone(), name));
        }
        found
    }

    /// The rows of `DocTransitions`, in the tables' order.
    fn rows(&mut self) -> Vec<String> {
        let model = self.model;
        let tables = self.tables;
        let paths: BTreeMap<(String, usize), String> =
            crate::transition_records::transition_records(model)
                .into_iter()
                .map(|record| ((record.from, record.index), record.node_path))
                .collect();
        let target_names = |row: &crate::document_tables::TransitionRow| -> String {
            let targets: Vec<String> = tables.targets[row.targets.range()]
                .iter()
                .map(|target| match target {
                    TargetRef::State(state) => tla_string(&tables.states[usize::from(*state)].id),
                    TargetRef::History(history) => {
                        tla_string(&tables.histories[usize::from(*history)].id)
                    }
                })
                .collect();
            format!("<<{}>>", targets.join(", "))
        };
        let mut rows = Vec::new();
        for row in &tables.transitions {
            let targets = target_names(row);
            let source = row
                .source
                .map(|state| tables.states[usize::from(state)].id.clone());
            match (row.origin, source) {
                (TransitionOrigin::Written { index_in_source }, Some(id)) => {
                    let index = usize::from(index_in_source);
                    let Some(transition) =
                        model.states.get(&id).and_then(|s| s.transitions.get(index))
                    else {
                        continue;
                    };
                    let path = paths
                        .get(&(id.clone(), index))
                        .cloned()
                        .unwrap_or_else(|| format!("states.{id}.transitions[{index}]"));
                    let place = format!("transition {index} of state `{id}`");
                    let events = self.descriptors(&place, &transition.event);
                    let cond = self.condition(&place, &transition.cond);
                    let actions: Vec<&Action> = transition.actions.iter().collect();
                    let content = self.block(&place, path, &actions);
                    let constructor = if row.internal {
                        "InternalTransition"
                    } else {
                        "Transition"
                    };
                    rows.push(format!(
                        "{constructor}({}, {events}, {cond}, {targets}, {content})",
                        tla_string(&id)
                    ));
                }
                (TransitionOrigin::Initial, Some(id)) => {
                    let place = format!("the <initial> of state `{id}`");
                    let actions: Vec<&Action> = model
                        .states
                        .get(&id)
                        .map(|state| state.initial_transition_actions.iter().collect())
                        .unwrap_or_default();
                    let content = self.block(&place, format!("initial {id}"), &actions);
                    rows.push(format!(
                        "Initial({}, {targets}, {content})",
                        tla_string(&id)
                    ));
                }
                (TransitionOrigin::HistoryDefault(history), _) => {
                    let id = tables.histories[usize::from(history)].id.clone();
                    let place = format!("the default transition of <history> `{id}`");
                    let actions: Vec<&Action> = model
                        .history_states
                        .get(&id)
                        .map(|info| info.default_actions.iter().collect())
                        .unwrap_or_default();
                    let content = self.block(&place, format!("default {id}"), &actions);
                    rows.push(format!(
                        "HistoryDefault({}, {targets}, {content})",
                        tla_string(&id)
                    ));
                }
                (TransitionOrigin::Document, _) => {
                    rows.push(format!("Initial(Root, {targets}, NoContent)"));
                }
                (TransitionOrigin::Written { .. } | TransitionOrigin::Initial, None) => {}
            }
        }
        rows
    }
}

/// What an expression reads, apart from its literals and its operators: the
/// names, calls, members and strings it holds, which each context of
/// translation answers for itself. A document's conditions read its data
/// ([`DataScope`]); a property's conditions read an observation
/// (`super::properties`).
pub(super) trait Scope {
    /// The identifier `name`.
    fn ident(&mut self, name: &str) -> Result<Tla, String>;
    /// The call `callee(args)`.
    fn call(&mut self, callee: &Expr, args: &[Expr]) -> Result<Tla, String>;
    /// The member `object.property`, or `object[index]` with `property`
    /// `None` where the index is computed.
    fn member(&mut self, object: &Expr, property: Option<&str>) -> Result<Tla, String>;
    /// The string literal `text`.
    fn string(&mut self, text: &str) -> Result<Tla, String>;
    /// The translation needs the integers below zero (`Integers`).
    fn needs_integers(&mut self);
}

/// `expr` as TLA+ over `scope`, or why it has no translation: the integers
/// and the booleans with their operators, each name, call, member and string
/// as `scope` reads it.
pub(super) fn translate(expr: &Expr, scope: &mut dyn Scope) -> Result<Tla, String> {
    match &expr.kind {
        ExprKind::Number(spelling) => {
            let decimal = !spelling.is_empty()
                && spelling.bytes().all(|b| b.is_ascii_digit())
                && (spelling == "0" || !spelling.starts_with('0'));
            match spelling.parse::<u64>() {
                Ok(value) if decimal && value <= LARGEST_INTEGER => {
                    Ok(Tla::atom(spelling.clone(), Ty::Int))
                }
                _ => Err(format!(
                    "`{spelling}` is no integer written in decimal up to {LARGEST_INTEGER}"
                )),
            }
        }
        ExprKind::Bool(value) => Ok(Tla::atom(
            String::from(if *value { "TRUE" } else { "FALSE" }),
            Ty::Bool,
        )),
        ExprKind::Ident(name) => scope.ident(name),
        ExprKind::Unary { op, operand } => {
            let inner = translate(operand, scope)?;
            match (op, inner.ty) {
                (UnaryOp::Not, Ty::Bool) => Ok(Tla {
                    text: format!("~{}", inner.operand()),
                    ty: Ty::Bool,
                    binding: Binding::Compound,
                }),
                (UnaryOp::Neg, Ty::Int) => {
                    scope.needs_integers();
                    Ok(Tla {
                        text: format!("-{}", inner.operand()),
                        ty: Ty::Int,
                        binding: Binding::Compound,
                    })
                }
                (UnaryOp::Not, ty) | (UnaryOp::Neg, ty) => Err(format!(
                    "`{}` is applied to {}",
                    unary_spelling(*op),
                    ty.word()
                )),
                (UnaryOp::Pos | UnaryOp::BitNot | UnaryOp::TypeOf, _) => Err(format!(
                    "`{}`, which has no translation",
                    unary_spelling(*op)
                )),
            }
        }
        ExprKind::Binary { op, left, right } => {
            let tla_op = match op {
                BinOp::StrictEq | BinOp::LooseEq => "=",
                BinOp::StrictNeq | BinOp::LooseNeq => "#",
                BinOp::Lt => "<",
                BinOp::Gt => ">",
                BinOp::LtEq => "<=",
                BinOp::GtEq => ">=",
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => return Err("`/`, which leaves the integers".to_string()),
                BinOp::Mod => {
                    return Err("`%`, whose sign TLA+ and ECMAScript take differently".to_string())
                }
                BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
                | BinOp::Shl
                | BinOp::Shr
                | BinOp::UShr => {
                    return Err("a bitwise operator, which has no translation".to_string())
                }
                BinOp::InstanceOf | BinOp::In => {
                    return Err("an operator on objects, which the model does not hold".to_string())
                }
            };
            let l = translate(left, scope)?;
            let r = translate(right, scope)?;
            let equality = matches!(
                op,
                BinOp::StrictEq | BinOp::LooseEq | BinOp::StrictNeq | BinOp::LooseNeq
            );
            if equality {
                if l.ty != r.ty {
                    return Err(format!(
                        "compares {} with {}, which ECMAScript converts",
                        l.ty.word(),
                        r.ty.word()
                    ));
                }
            } else if l.ty != Ty::Int || r.ty != Ty::Int {
                let other = if l.ty == Ty::Int { r.ty } else { l.ty };
                return Err(format!("`{tla_op}` is applied to {}", other.word()));
            }
            if matches!(op, BinOp::Sub) {
                scope.needs_integers();
            }
            let arithmetic = matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul);
            Ok(Tla {
                text: format!("{} {tla_op} {}", l.operand(), r.operand()),
                ty: if arithmetic { Ty::Int } else { Ty::Bool },
                binding: Binding::Compound,
            })
        }
        ExprKind::Logical { op, left, right } => {
            let l = translate(left, scope)?;
            let r = translate(right, scope)?;
            if l.ty != Ty::Bool || r.ty != Ty::Bool {
                return Err(
                    "`&&` or `||` of a value that is no boolean, which ECMAScript answers \
                     with an operand"
                        .to_string(),
                );
            }
            let junction = match op {
                LogicalOp::And => "/\\",
                LogicalOp::Or => "\\/",
            };
            Ok(Tla {
                text: format!("{} {junction} {}", l.operand(), r.operand()),
                ty: Ty::Bool,
                binding: Binding::Junction,
            })
        }
        ExprKind::Str(text) => scope.string(text),
        ExprKind::Nullish => {
            Err("null or undefined, which the model's data do not hold".to_string())
        }
        ExprKind::Call { callee, args } => scope.call(callee, args),
        ExprKind::Member { object, property } => scope.member(object, Some(&property.text)),
        ExprKind::Index { object, .. } => scope.member(object, None),
        ExprKind::Conditional { .. } => {
            Err("a conditional expression, which has no translation".to_string())
        }
        ExprKind::This
        | ExprKind::Array(_)
        | ExprKind::Object(_)
        | ExprKind::New { .. }
        | ExprKind::Function { .. } => {
            Err("an object, an array or a function, which the model does not hold".to_string())
        }
        ExprKind::Update { .. } | ExprKind::Assign { .. } => {
            Err("an assignment inside an expression, which has no translation".to_string())
        }
    }
}

/// A document's data, as its conditions and its blocks read them: each
/// `<data>` a field of the record named `data`, and nothing else.
struct DataScope<'v> {
    /// Each `<data>`'s id and type, in document order.
    variables: &'v [(String, Ty)],
    /// The record a name is read from; `None` for a `<data>` value, which
    /// reads no variable.
    data: Option<&'v str>,
    integers: bool,
}

impl Scope for DataScope<'_> {
    fn ident(&mut self, name: &str) -> Result<Tla, String> {
        let Some(data) = self.data else {
            return Err(format!(
                "reads `{name}`, and a <data> value is written with literals alone"
            ));
        };
        match self.variables.iter().find(|(id, _)| id == name) {
            Some((_, ty)) => Ok(Tla::atom(format!("{data}.{name}"), *ty)),
            None => Err(format!("`{name}` is no <data> of the document")),
        }
    }

    fn call(&mut self, _callee: &Expr, _args: &[Expr]) -> Result<Tla, String> {
        Err("a call, In() among them, which has no translation".to_string())
    }

    fn member(&mut self, _object: &Expr, _property: Option<&str>) -> Result<Tla, String> {
        Err("a member of an object, _event among them, which the model does not hold".to_string())
    }

    fn string(&mut self, _text: &str) -> Result<Tla, String> {
        Err("a string, which the model's data do not hold".to_string())
    }

    fn needs_integers(&mut self) {
        self.integers = true;
    }
}

/// Why a state id has no place in the model, if it has none.
fn state_id_refusal(id: &str) -> Option<Construct> {
    if id.contains('.') {
        Some(Construct::DottedStateId(id.to_string()))
    } else if !id.chars().all(|c| (' '..='~').contains(&c)) {
        Some(Construct::Unspellable(id.to_string()))
    } else {
        None
    }
}

fn expression_refusal(text: &str, why: String) -> Construct {
    Construct::Expression {
        text: text.split_whitespace().collect::<Vec<_>>().join(" "),
        why,
    }
}

fn unary_spelling(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Pos => "+",
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
        UnaryOp::TypeOf => "typeof",
    }
}

/// Whether `name` is a TLA+ identifier no word of TLA+ takes: letters, digits
/// and underscores, at least one letter, and not the prefix of a fairness.
fn is_tla_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && name.bytes().any(|b| b.is_ascii_alphabetic())
        && !name.starts_with("WF_")
        && !name.starts_with("SF_")
        && !TLA_WORDS.contains(&name)
}

/// `text` as a TLA+ string literal.
pub(crate) fn tla_string(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for c in text.chars() {
        if matches!(c, '"' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    quoted
}

/// Where an element is written, for an order among elements; one with no
/// position comes after every one that has one.
fn position(location: Option<&SourceLocation>) -> (u32, u32) {
    location.map_or((u32::MAX, u32::MAX), |at| {
        (at.line.unwrap_or(u32::MAX), at.col.unwrap_or(u32::MAX))
    })
}

/// `open`, the items separated by commas and wrapped before the 80th column,
/// and `close`.
fn wrapped(open: &str, items: &[String], close: &str) -> String {
    let indent = " ".repeat(open.len());
    let mut text = open.to_string();
    let mut width = open.len();
    for (at, item) in items.iter().enumerate() {
        let piece = if at + 1 == items.len() {
            format!("{item}{close}")
        } else {
            format!("{item},")
        };
        if at > 0 {
            if width + 1 + piece.len() > 79 {
                text.push('\n');
                text.push_str(&indent);
                width = indent.len();
            } else {
                text.push(' ');
                width += 1;
            }
        }
        text.push_str(&piece);
        width += piece.len();
    }
    if items.is_empty() {
        text.push_str(close);
    }
    text
}

/// A function over `domain` whose value at each key of `entries` is the
/// value given there, and `default` everywhere else: the keys of one value
/// grouped in one arm, the arms in the order their values first appear.
fn function(bound: &str, domain: &str, entries: &[(String, String)], default: &str) -> String {
    let mut arms: Vec<(String, Vec<String>)> = Vec::new();
    for (key, value) in entries {
        if value == default {
            continue;
        }
        match arms.iter_mut().find(|(known, _)| known == value) {
            Some((_, keys)) => keys.push(tla_string(key)),
            None => arms.push((value.clone(), vec![tla_string(key)])),
        }
    }
    if arms.is_empty() {
        return format!("[{bound} \\in {domain} |-> {default}]");
    }
    let mut text = format!("[{bound} \\in {domain} |->\n");
    for (at, (value, keys)) in arms.iter().enumerate() {
        let test = match keys.as_slice() {
            [only] => format!("{bound} = {only}"),
            _ => format!("{bound} \\in {{{}}}", keys.join(", ")),
        };
        let lead = if at == 0 { "    CASE " } else { "      [] " };
        text.push_str(&format!("{lead}{test} -> {value}\n"));
    }
    text.push_str(&format!("      [] OTHER -> {default}]"));
    text
}

/// The arms of a `CASE` of an operator's body, each `(test, value)`.
fn push_case(text: &mut String, arms: &[(String, String)]) {
    for (at, (test, value)) in arms.iter().enumerate() {
        let lead = if at == 0 { "    CASE " } else { "      [] " };
        text.push_str(&format!("{lead}{test} -> {value}\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    /// `text` parsed and analysed, as `sce-codegen model` receives a document.
    fn model_of(text: &str) -> SCXMLModel {
        let mut model = SCXMLParser::new().parse_string(text, "doc").expect("parse");
        crate::analyzer::analyze(&mut model, "");
        model
    }

    fn written(text: &str, options: Options<'_>) -> String {
        write(&model_of(text), "doc.scxml", options)
            .expect("a model")
            .text
    }

    fn refusals(text: &str, options: Options<'_>) -> Vec<String> {
        match write(&model_of(text), "doc.scxml", options) {
            Err(FormalError::NoTranslation { refusals, .. }) => refusals,
            other => panic!("expected the document refused, got {other:?}"),
        }
    }

    const HISTORY: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="ecmascript" initial="s012">
        <datamodel><data id="Var1" expr="0"/></datamodel>
        <state id="s0" initial="s01">
            <onentry><assign location="Var1" expr="Var1 + 1"/></onentry>
            <transition event="entering.s012" cond="Var1 == 1" target="s1">
                <send event="timeout" delay="2s"/>
            </transition>
            <transition event="timeout" target="fail"/>
            <state id="s01" initial="s011">
                <state id="s011"><onentry><raise event="entering.s011"/></onentry></state>
                <state id="s012"><onentry><raise event="entering.s012"/></onentry></state>
            </state>
            <history type="deep" id="h"><transition target="s012"/></history>
        </state>
        <state id="s1"><transition target="h"/></state>
        <final id="pass"/>
        <final id="fail"/>
    </scxml>"#;

    /// Turns red if the rows leave the code's numbering
    /// (`DocumentTables::of`): the written transitions first, by state, then
    /// each compound state's initial transition, each history's default, and
    /// the document's own initial transition last.
    #[test]
    fn the_rows_are_numbered_as_the_code_numbers_them() {
        let text = written(HISTORY, Options::default());
        let rows = [
            r#"    Transition("s0", {<<"entering", "s012">>}, "Var1 == 1", <<"s1">>, "states.s0.transitions[0]"),"#,
            r#"    Transition("s0", {<<"timeout">>}, NoCond, <<"fail">>, NoContent),"#,
            r#"    Transition("s1", {}, NoCond, <<"h">>, NoContent),"#,
            r#"    Initial("s0", <<"s01">>, NoContent),"#,
            r#"    Initial("s01", <<"s011">>, NoContent),"#,
            r#"    HistoryDefault("h", <<"s012">>, NoContent),"#,
            r#"    Initial(Root, <<"s012">>, NoContent)"#,
        ]
        .join("\n");
        assert!(
            text.contains(&format!("DocTransitions == <<\n{rows}\n>>")),
            "{text}"
        );
    }

    /// Turns red if a <history> is placed by its parent alone rather than
    /// where it is written among its parent's children (`visit`).
    #[test]
    fn a_history_stands_in_document_order_where_it_is_written() {
        let text = written(HISTORY, Options::default());
        assert!(
            text.contains(
                r#"DocOrder == <<"s0", "s01", "s011", "s012", "h", "s1", "pass", "fail">>"#
            ),
            "{text}"
        );
    }

    /// Turns red if a kind, a parent or a history's type is written for the
    /// wrong states, or the default of a function is lost.
    #[test]
    fn kinds_parents_and_history_types_are_functions_of_the_tree() {
        let text = written(HISTORY, Options::default());
        assert!(
            text.contains(
                "DocKind == [s \\in DocStates |->\n    CASE s = \"h\" -> \"history\"\n      [] s \\in {\"pass\", \"fail\"} -> \"final\"\n      [] OTHER -> \"state\"]"
            ),
            "{text}"
        );
        assert!(
            text.contains(
                "DocParent == [s \\in DocStates |->\n    CASE s \\in {\"s01\", \"h\"} -> \"s0\"\n      [] s \\in {\"s011\", \"s012\"} -> \"s01\"\n      [] OTHER -> Root]"
            ),
            "{text}"
        );
        assert!(
            text.contains(
                "DocHistoryType == [h \\in {\"h\"} |->\n    CASE h = \"h\" -> \"deep\"\n      [] OTHER -> \"shallow\"]"
            ),
            "{text}"
        );
    }

    /// Turns red if a condition, an assignment, a raise or a delayed send is
    /// translated otherwise than Appendix D's `Guard` and `Execute` read them.
    #[test]
    fn conditions_and_blocks_are_operators_over_the_data() {
        let text = written(HISTORY, Options::default());
        for expected in [
            "DocData0 == [Var1 |-> 0]",
            "DocGuard(cond, data, event, conf) ==\n    CASE cond = \"Var1 == 1\" -> data.Var1 = 1\n",
            "block = \"states.s0.transitions[0]\" -> Effect(data, <<>>, <<>>, <<<<\"timeout\">>>>)",
            "block = \"onentry s0\" -> Effect([data EXCEPT !.Var1 = data.Var1 + 1], <<>>, <<>>, <<>>)",
            "block = \"onentry s011\" -> Effect(data, <<<<\"entering\", \"s011\">>>>, <<>>, <<>>)",
            "DocOnEntry == [s \\in DocStates |->\n    CASE s = \"s0\" -> \"onentry s0\"\n",
            "DocOnExit == [s \\in DocStates |-> NoContent]",
            "EXTENDS AppendixD\n",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in\n{text}");
        }
    }

    const PARALLEL: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="ecmascript" initial="p">
        <datamodel><data id="n" expr="0"/><data id="ok" expr="true"/></datamodel>
        <parallel id="p">
            <state id="a" initial="a1">
                <transition event="go" type="internal" target="a2">
                    <assign location="n" expr="n - 1"/>
                    <assign location="ok" expr="n &lt; 0 &amp;&amp; !ok"/>
                </transition>
                <state id="a1"/>
                <state id="a2"/>
            </state>
            <state id="b">
                <transition event="done.state.a error.*" cond="ok || n == 0" target="end"/>
            </state>
        </parallel>
        <final id="end"/>
    </scxml>"#;

    /// Turns red if an internal transition is written as an external one, a
    /// descriptor's wildcard is kept, or assignments stop reading the
    /// datamodel the one before them left.
    #[test]
    fn assignments_are_threaded_and_internal_transitions_kept() {
        let text = written(PARALLEL, Options::default());
        for expected in [
            "EXTENDS AppendixD, Integers\n",
            "DocData0 == [n |-> 0, ok |-> TRUE]",
            r#"    InternalTransition("a", {<<"go">>}, NoCond, <<"a2">>, "states.a.transitions[0]"),"#,
            r#"    Transition("b", {<<"done", "state", "a">>, <<"error">>}, "ok || n == 0", <<"end">>, NoContent),"#,
            "CASE cond = \"ok || n == 0\" -> (data.ok \\/ (data.n = 0))",
            "(LET d1 == [data EXCEPT !.n = data.n - 1]\n",
            "d2 == [d1 EXCEPT !.ok = ((d1.n < 0) /\\ (~d1.ok))]\n",
            "IN Effect(d2, <<>>, <<>>, <<>>))",
            "CASE s = \"p\" -> \"parallel\"",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in\n{text}");
        }
    }

    /// Turns red if a construct with no translation is translated in part,
    /// dropped in silence, or refused alone while the others go unnamed.
    #[test]
    fn every_construct_without_a_translation_is_refused_by_name() {
        let found = refusals(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
                    datamodel="ecmascript" initial="a">
                <datamodel>
                    <data id="x"/>
                    <data id="k" expr="1"/>
                    <data id="r" expr="1.5"/>
                </datamodel>
                <state id="a">
                    <onentry><log expr="1"/></onentry>
                    <transition event="e" cond="In('a')" target="b">
                        <send event="s" target="#_internal"/>
                    </transition>
                    <transition event="f" cond="'yes'" target="b">
                        <assign location="k" expr="true"/>
                    </transition>
                </state>
                <final id="b"/>
            </scxml>"##,
            Options::default(),
        )
        .join("\n");
        for expected in [
            "the <data> `x`: a <data> with no value",
            "the <data> `r`: `1.5`: `1.5` is no integer written in decimal",
            "the <onentry> of state `a`: <log>, which has no translation",
            "transition 0 of state `a`: `In('a')`: a call, In() among them",
            "transition 0 of state `a`: a <send> with a target",
            "transition 1 of state `a`: `'yes'`: a string",
            "transition 1 of state `a`: `true`: assigns a boolean to k, which is an integer",
        ] {
            assert!(found.contains(expected), "missing {expected:?} in\n{found}");
        }
    }

    /// Turns red if late binding is modelled as the early binding the model
    /// runs.
    #[test]
    fn late_binding_is_refused() {
        let found = refusals(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
                    datamodel="ecmascript" binding="late" initial="a">
                <final id="a"/>
            </scxml>"#,
            Options::default(),
        );
        assert!(
            found.iter().any(|r| r.contains("late binding")),
            "{found:?}"
        );
    }

    const BARE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="null" initial="a">
        <state id="a"><transition event="go" target="pass"/></state>
        <final id="pass"/>
        <final id="fail"/>
    </scxml>"#;

    /// Turns red if a document with no data, condition or block is given
    /// operators TLC cannot read (a `CASE` with no arm, a record with no field).
    #[test]
    fn a_document_without_data_or_content_has_total_operators() {
        let text = written(BARE, Options::default());
        for expected in [
            "DocData0 == [v \\in {} |-> 0]",
            "DocGuard(cond, data, event, conf) == FALSE",
            "DocExecute(block, data, event) == Effect(data, <<>>, <<>>, <<>>)",
            "DocHistoryType == [h \\in {} |-> \"shallow\"]",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in\n{text}");
        }
    }

    /// An event is raised under a name longer than every descriptor that
    /// matches it, and one a descriptor of the document names only as the
    /// wildcard matches.
    const PREFIXED: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="null" initial="s1">
        <state id="s1">
            <onentry><raise event="request.new"/></onentry>
            <transition event="request" target="s2"/>
        </state>
        <state id="s2">
            <onentry><raise event="other.thing"/></onentry>
            <transition event="request.*" target="fail"/>
            <transition event="*" target="pass"/>
        </state>
        <final id="pass"/>
        <final id="fail"/>
    </scxml>"#;

    /// The model holds an event by its whole name, the name a run trace's step
    /// names it by, and a transition by the descriptors that select it, which
    /// AppendixD's `NameMatch` matches token by token as a prefix of the name,
    /// or as the wildcard. So a run that takes `request.new` on `request` is
    /// judged against the event taken, and no step's name is compared with a
    /// descriptor. Turns red if a raised name is cut to the descriptor that
    /// matches it, a descriptor's trailing `.*` is kept, or the wildcard is
    /// written as a name.
    #[test]
    fn an_event_keeps_its_whole_name_and_a_descriptor_its_prefix() {
        let text = written(PREFIXED, Options::default());
        for expected in [
            r#"    Transition("s1", {<<"request">>}, NoCond, <<"s2">>, NoContent),"#,
            r#"    Transition("s2", {<<"request">>}, NoCond, <<"fail">>, NoContent),"#,
            r#"    Transition("s2", {<<"*">>}, NoCond, <<"pass">>, NoContent),"#,
            r#"block = "onentry s1" -> Effect(data, <<<<"request", "new">>>>, <<>>, <<>>)"#,
            r#"block = "onentry s2" -> Effect(data, <<<<"other", "thing">>>>, <<>>, <<>>)"#,
        ] {
            assert!(
                text.contains(expected),
                "missing {expected:?} in
{text}"
            );
        }
    }

    /// Turns red if the W3C verdict is written unasked, not written when
    /// asked, or asked of a document with no final state it names.
    #[test]
    fn the_w3c_verdict_is_written_only_when_asked_and_only_of_its_finals() {
        let verdict = Options {
            module: None,
            w3c_verdict: true,
        };
        let asked = written(BARE, verdict);
        assert!(
            asked.contains(
                "NeverFails == outcome # \"fail\"\nPasses == <>(outcome = \"pass\")\n===="
            ),
            "{asked}"
        );
        assert!(!written(BARE, Options::default()).contains("NeverFails"));
        let found = refusals(&BARE.replace("<final id=\"fail\"/>", ""), verdict);
        assert_eq!(
            found,
            ["the document: the W3C verdict, and the document has no top-level <final id=\"fail\">"]
        );
    }

    /// Turns red if the promise every model makes stops being Appendix D with
    /// every part in, in the lines a module written by hand states it in.
    #[test]
    fn every_model_promises_appendix_d_with_every_part_in() {
        let text = written(BARE, Options::default());
        let promise = "Promise == INSTANCE AppendixD WITH\n    ExitOrder <- TRUE, EntryOrder <- TRUE, RemoveConflicts <- TRUE,\n    EnterAncestors <- TRUE, FillParallel <- TRUE, RecordHistory <- TRUE,\n    DeepHistory <- TRUE, DoneCompound <- TRUE, DoneParallel <- TRUE,\n    RunToCompletion <- TRUE\nRefines == Promise!Spec\n";
        assert!(text.contains(promise), "{text}");
        assert!(text.starts_with("---- MODULE doc ----\n"), "{text}");
        assert!(text.ends_with("====\n"), "{text}");
    }

    /// Turns red if a module is named what TLA+ cannot read as a module's name
    /// or what TLC finds before it.
    #[test]
    fn a_module_is_named_by_a_tla_identifier_of_its_own() {
        assert!(is_tla_identifier("W3CTest388"));
        assert!(is_tla_identifier("test_388"));
        for name in ["test-388", "388", "IF", "WF_x", "", "Var\u{e9}"] {
            assert!(!is_tla_identifier(name), "{name:?}");
        }
        let named = Options {
            module: Some("Naturals"),
            w3c_verdict: false,
        };
        assert!(refusals(BARE, named)
            .iter()
            .any(|r| r.starts_with("the module: `Naturals`")));
        let renamed = Options {
            module: Some("W3CTestBare"),
            w3c_verdict: false,
        };
        assert!(written(BARE, renamed).starts_with("---- MODULE W3CTestBare ----\n"));
    }

    /// Turns red if a state id Appendix D would split into tokens, or one a
    /// TLA+ string cannot hold, is let through.
    #[test]
    fn a_state_id_is_one_token_of_printable_ascii() {
        assert_eq!(
            state_id_refusal("a.b"),
            Some(Construct::DottedStateId("a.b".to_string()))
        );
        assert_eq!(
            state_id_refusal("caf\u{e9}"),
            Some(Construct::Unspellable("caf\u{e9}".to_string()))
        );
        assert_eq!(state_id_refusal("s0"), None);
        assert_eq!(tla_string(r#"a"b\c"#), r#""a\"b\\c""#);
    }

    /// Turns red if a long list runs past the 80th column.
    #[test]
    fn a_long_list_is_wrapped_before_the_80th_column() {
        let items: Vec<String> = (0..30).map(|i| tla_string(&format!("state{i}"))).collect();
        let text = wrapped("DocStates == {", &items, "}");
        assert!(text.lines().count() > 1, "{text}");
        assert!(text.lines().all(|line| line.len() < 80), "{text}");
        assert!(text.ends_with("\"state29\"}"), "{text}");
    }

    /// Turns red if an observer's tables are written under another prefix
    /// than the one asked for, its facts lose a state, or an observer may send
    /// (docs/adr/0115, D15).
    #[test]
    fn an_observer_is_written_under_its_prefix_and_sends_nothing() {
        let tables = observer_tables(&model_of(BARE), "Obs1").expect("the tables of an observer");
        for expected in [
            "Obs1States == {\"a\", \"pass\", \"fail\"}\n",
            "Obs1Kind == [s \\in Obs1States |->\n",
            "Obs1Transitions == <<\n",
            "Obs1Guard(cond, data, event, conf) == FALSE\n",
            "Obs1Execute(block, data, event) == Effect(data, <<>>, <<>>, <<>>)\n",
        ] {
            assert!(
                tables.text.contains(expected),
                "missing {expected:?} in\n{}",
                tables.text
            );
        }
        assert!(!tables.text.contains("DocStates"), "{}", tables.text);
        let mut states = tables.facts.states.clone();
        states.sort();
        assert_eq!(states, ["a", "fail", "pass"]);
        let refused = observer_tables(&model_of(HISTORY), "Obs1").expect_err("a send is refused");
        assert!(
            refused.iter().any(|r| r.to_string()
                == "transition 0 of state `s0`: a <send>, and an observer sends nothing: it is \
                    read by the properties alone"),
            "{refused:?}"
        );
    }

    /// Turns red if a document's own module stops writing its tables under
    /// the prefix its configurations name.
    #[test]
    fn a_documents_tables_are_written_under_its_own_prefix() {
        let text = written(BARE, Options::default());
        assert!(
            text.contains("\nDocStates == {\"a\", \"pass\", \"fail\"}\n"),
            "{text}"
        );
        assert!(text.contains("\nDocData0 == "), "{text}");
    }
}
