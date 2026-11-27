// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The properties of one machine as TLA+ (docs/adr/0115, D15): the module
//! `sce-codegen formal properties` writes beside the machine's own model
//! ([`super::document_model`]), and the configuration of each run that judges
//! it.
//!
//! A run of the machine, as its user sees it, is a sequence of observations,
//! one at the end of each macrostep: the event the macrostep took, the states
//! the machine is in and the values it publishes. The module adds them to the
//! machine's model as history variables (Abadi and Lamport 1991), which take
//! every step the machine takes and refuse none: `obs`, the last observation;
//! `before`, the one before it with the observers then, which a
//! `<sce:step>` reads through `previous(...)`; and `observers`, each
//! observer's machine, a statechart of its own over Appendix D that takes the
//! event of every observation as its event. A step of the interpreter ends a
//! macrostep when the interpreter is then settled: about to leave, or with no
//! eventless transition enabled and nothing on its internal queue.
//!
//! An assumption is of what the environment does and never of the machine: a
//! condition of it reads the event that comes (`_event.name`) and nothing
//! else. An `<sce:always>` of it is a condition every event that comes meets,
//! so the model's environment sends no other, and since an event that comes
//! changes no observation, a guarantee that fails can only fail after the
//! assumption has (Abadi and Lamport 1995). An `<sce:recurs>` of it is an
//! event that keeps coming, a strong fairness of its arrival; the exploration
//! bounds how many come, so a progress guarantee is then judged over the runs
//! that do not end at that bound (a run that does is neither proof nor
//! violation of it).
//!
//! Each guarantee is stated once, and once more for each of its parts with
//! that part replaced by what makes the guarantee stronger: false where it
//! stands under an even count of negations, true elsewhere. A guarantee that
//! holds though one part is so replaced passes for nothing in that part
//! (Purandare and Somenzi 2002), and is refused.
//!
//! What has no translation is refused by name, every such construct at once,
//! and the properties then have no module.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::ecmascript::parser::parse_expression;
use crate::ecmascript::{Expr, ExprKind, UnaryOp};
use crate::forge::error::{ForgeError, Located};
use crate::forge::model::{ExplorationModel, ForgeDocument, PropertiesModel, PropertyForm};
use crate::model::SCXMLModel;

use super::document_model::{
    self, tla_string, Binding, DocumentModel, Facts, Scope, Term, Tla, Ty, Typed, DOCUMENT_PREFIX,
    SEMANTICS,
};
use super::FormalError;

/// The parts of Appendix D, each in, as every judgement of a document's
/// properties runs it: the semantics is the recommendation's, and only a
/// model of the semantics takes a part out.
const PARTS: [&str; 10] = [
    "ExitOrder",
    "EntryOrder",
    "RemoveConflicts",
    "EnterAncestors",
    "FillParallel",
    "RecordHistory",
    "DeepHistory",
    "DoneCompound",
    "DoneParallel",
    "RunToCompletion",
];

/// The constants of Appendix D a document's model gives by its tables, each
/// with the name of the definition that gives it, after the prefix.
const TABLES: [(&str, &str); 11] = [
    ("States", "States"),
    ("Kind", "Kind"),
    ("Parent", "Parent"),
    ("DocumentOrder", "Order"),
    ("HistoryType", "HistoryType"),
    ("Transitions", "Transitions"),
    ("OnEntry", "OnEntry"),
    ("OnExit", "OnExit"),
    ("Data0", "Data0"),
    ("Guard", "Guard"),
    ("Execute", "Execute"),
];

/// The value of `Root` in every run, as in the W3C models: no XML name of a
/// state can be it.
const ROOT: &str = "\"scxml\"";

/// A document of one judgement, as it was read.
#[derive(Debug, Clone)]
pub struct Read<T> {
    pub path: PathBuf,
    pub text: String,
    pub model: T,
}

/// One observer of a properties document, read.
#[derive(Debug, Clone)]
pub struct ReadObserver {
    /// The name a condition reads it by.
    pub id: String,
    pub document: Read<SCXMLModel>,
}

/// The documents of one judgement (docs/adr/0115, D15): the machine, its
/// properties with their observers, and an exploration of it.
#[derive(Debug, Clone)]
pub struct Documents {
    pub machine: Read<SCXMLModel>,
    pub properties: Read<PropertiesModel>,
    pub observers: Vec<ReadObserver>,
    pub exploration: Read<ExplorationModel>,
}

/// Why the documents of a judgement were not read.
#[derive(Debug)]
pub enum Unread {
    /// A document is refused by its own parser, with its own diagnostic.
    Document(Box<Located<ForgeError>>),
    /// A document is not there, is of another kind than its place asks, or an
    /// exploration is of another machine than the properties are
    /// (`formal/declaration`).
    Formal(FormalError),
}

impl From<Located<ForgeError>> for Unread {
    fn from(refused: Located<ForgeError>) -> Self {
        Unread::Document(Box::new(refused))
    }
}

/// Read the properties document at `properties`, the machine and the
/// observers it names, and the exploration at `exploration`, which must name
/// the same machine.
pub fn read(properties: &Path, exploration: &Path) -> Result<Documents, Unread> {
    let (properties_text, properties_model) = read_forge(properties, |document| match document {
        ForgeDocument::Properties(model) => Ok(model),
        other => Err(other.kind().as_attr()),
    })?;
    let (exploration_text, exploration_model) =
        read_forge(exploration, |document| match document {
            ForgeDocument::Exploration(model) => Ok(model),
            other => Err(other.kind().as_attr()),
        })?;
    let machine_path = beside(properties, &properties_model.machine);
    let explored = beside(exploration, &exploration_model.machine);
    if !same_file(&machine_path, &explored) {
        return Err(Unread::Formal(FormalError::Declaration {
            manifest: exploration.display().to_string(),
            problems: vec![format!(
                "explores {}, and the properties {} are of {}",
                explored.display(),
                properties.display(),
                machine_path.display()
            )],
        }));
    }
    let machine = read_statechart(&machine_path)?;
    let mut observers = Vec::with_capacity(properties_model.observers.len());
    for observer in &properties_model.observers {
        observers.push(ReadObserver {
            id: observer.id.clone(),
            document: read_statechart(&beside(properties, &observer.src))?,
        });
    }
    Ok(Documents {
        machine,
        properties: Read {
            path: properties.to_path_buf(),
            text: properties_text,
            model: properties_model,
        },
        observers,
        exploration: Read {
            path: exploration.to_path_buf(),
            text: exploration_text,
            model: exploration_model,
        },
    })
}

/// `src` as a document beside `document` names it.
fn beside(document: &Path, src: &str) -> PathBuf {
    document
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .join(src)
}

/// Whether two paths name one file: by their canonical forms where both have
/// one, else as written.
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// The text of the file at `path`, or the declaration's problem.
fn read_text(path: &Path) -> Result<String, Unread> {
    std::fs::read_to_string(path).map_err(|e| {
        Unread::Formal(FormalError::Declaration {
            manifest: path.display().to_string(),
            problems: vec![format!("cannot be read: {e}")],
        })
    })
}

/// The forge document at `path`, as the kind `take` accepts: `take` answers
/// the kind a document of another kind is.
fn read_forge<T>(
    path: &Path,
    take: impl FnOnce(ForgeDocument) -> Result<T, &'static str>,
) -> Result<(String, T), Unread> {
    let text = read_text(path)?;
    let shown = path.to_string_lossy().into_owned();
    let parsed = crate::forge::parser::parse_forge_with_imports(
        &text,
        crate::DocumentLabel::for_input_path(&shown),
    )?;
    let kind = match parsed {
        Some(parsed) => match take(parsed.document) {
            Ok(model) => return Ok((text, model)),
            Err(kind) => kind,
        },
        None => "statechart",
    };
    Err(Unread::Formal(FormalError::Declaration {
        manifest: shown,
        problems: vec![format!("is a document of sce:kind=\"{kind}\"")],
    }))
}

/// The statechart at `path`, analysed as the generator receives it, so its
/// tables are the ones its code is printed from.
fn read_statechart(path: &Path) -> Result<Read<SCXMLModel>, Unread> {
    let text = read_text(path)?;
    let shown = path.to_string_lossy().into_owned();
    let mut model = crate::parser::SCXMLParser::new().parse_file(&shown)?;
    crate::analyzer::analyze(&mut model, &shown);
    Ok(Read {
        path: path.to_path_buf(),
        text,
        model,
    })
}

/// What a guarantee is judged as by TLC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judged {
    /// An invariant of the safe specification: `<sce:always>`, `<sce:step>`.
    Invariant,
    /// A temporal property of the specification with its fairness:
    /// `<sce:recurs>`, `<sce:leads-to>`.
    Temporal,
    /// `AG EF` over the whole state graph (docs/adr/0115, D9):
    /// `<sce:possible>`.
    Possible,
}

/// A guarantee with one part replaced by what makes it stronger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strengthened {
    /// The part, as the document writes it.
    pub part: String,
    /// What replaces it: false under an even count of negations, else true.
    pub by: bool,
    /// The operator that states the stronger guarantee.
    pub operator: String,
}

/// One guarantee as the module states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guarantee {
    /// The `sce:req` token it states.
    pub req: String,
    /// The form it is written in (`always`, `leads-to`, ...).
    pub form: &'static str,
    pub judged: Judged,
    /// The operator that states it.
    pub operator: String,
    /// Each part of it, made stronger in turn.
    pub strengthened: Vec<Strengthened>,
}

/// The specification a run checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spec {
    /// The machine with its observations, with no fairness: invariants.
    Safe,
    /// With the interpreter's fairness, time's, and the arrivals the
    /// assumptions say keep coming: temporal properties.
    Live,
    /// The safe specification with one more variable, whether a predicate
    /// holds, for the graph `AG EF` is decided over.
    Possible,
}

impl Spec {
    fn name(self) -> &'static str {
        match self {
            Spec::Safe => "PSpec",
            Spec::Live => "PLiveSpec",
            Spec::Possible => "PossibleSpec",
        }
    }
}

/// What a run checks of a specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check<'a> {
    Invariant(&'a str),
    Property(&'a str),
    Nothing,
}

/// The properties of one machine as TLA+, and what judges each.
#[derive(Debug, Clone)]
pub struct PropertiesModule {
    /// The machine's own model, as `sce-codegen model` writes it.
    pub machine: DocumentModel,
    /// The module's name and its text.
    pub module: String,
    pub text: String,
    pub guarantees: Vec<Guarantee>,
    /// The assumptions, by the `sce:req` token each states. None is
    /// discharged by another machine here, so every verdict lists them.
    pub assumptions: Vec<String>,
    /// Whether an assumption says an event keeps coming.
    pub fair_arrivals: bool,
    /// The most events that come from outside, a bound of the search; 0 when
    /// none come.
    pub most: u32,
}

/// The name of the invariant that holds while fewer than the most events
/// have come: a run of it that fails shows the bound was reached.
pub const WITHIN_MOST: &str = "WithinMost";

/// The name of the property every machine is held to: every macrostep ends,
/// so there is always a next observation, or the session has ended.
pub const MACROSTEPS_END: &str = "MacrostepsEnd";

impl PropertiesModule {
    /// The configuration of a run of `spec` checking `check`.
    pub fn config(&self, spec: Spec, check: Check<'_>) -> String {
        let mut text = format!("SPECIFICATION {}\nCONSTANTS\n", spec.name());
        text.push_str(&format!("    Root = {ROOT}\n"));
        for (constant, definition) in TABLES {
            text.push_str(&format!(
                "    {constant} <- {DOCUMENT_PREFIX}{definition}\n"
            ));
        }
        text.push_str("    Outside <- PropOutside\n    MaxOutside <- PropMaxOutside\n");
        for part in PARTS {
            text.push_str(&format!("    {part} = TRUE\n"));
        }
        match check {
            Check::Invariant(name) => text.push_str(&format!("INVARIANT\n    {name}\n")),
            Check::Property(name) => text.push_str(&format!("PROPERTY\n    {name}\n")),
            Check::Nothing => {}
        }
        text.push_str("CHECK_DEADLOCK FALSE\n");
        text
    }

    /// The module whose states carry whether `predicate` holds, for the graph
    /// `AG EF` is decided over (docs/adr/0115, D9): its name and its text.
    pub fn possible_module(&self, predicate: &str) -> (String, String) {
        let name = format!("{}Possible", self.module);
        let text = super::models::possible_module_of(
            &name,
            &self.module,
            predicate,
            ("PInit", "PNext", "pvars"),
        );
        (name, text)
    }
}

/// The module of `documents`' properties, or every construct of them with no
/// translation, every property refused, or what of the exploration is not as
/// the machine is.
pub fn write(documents: &Documents) -> Result<PropertiesModule, FormalError> {
    let machine_label = documents.machine.path.display().to_string();
    let (machine, facts) = document_model::write_with_facts(
        &documents.machine.model,
        &machine_label,
        document_model::Options::default(),
    )?;
    let mut writer = Writer {
        refusals: Vec::new(),
        refused: Vec::new(),
        integers: false,
    };

    let exploration = &documents.exploration.model;
    let mut problems = Vec::new();
    for event in &exploration.outside {
        let taken = documents.machine.model.states.values().any(|state| {
            state.transitions.iter().any(|transition| {
                crate::event_descriptor::attribute_matches(&transition.event, event)
            })
        });
        if !taken {
            problems.push(format!(
                "the event `{event}` is taken by no transition of {machine_label}, so it is no \
                 event of its alphabet"
            ));
        }
        if !printable(event) {
            problems.push(format!(
                "the event `{event}` holds a character a TLA+ string cannot"
            ));
        }
    }
    if !problems.is_empty() {
        return Err(FormalError::Declaration {
            manifest: documents.exploration.path.display().to_string(),
            problems,
        });
    }

    let mut observers: Vec<(String, Facts)> = Vec::new();
    let mut observer_text = String::new();
    for (at, observer) in documents.observers.iter().enumerate() {
        let number = at + 1;
        let prefix = format!("Obs{number}");
        match document_model::observer_tables(&observer.document.model, &prefix) {
            Ok(tables) => {
                writer.integers |= tables.integers;
                observer_text.push_str(&format!(
                    "\\* The observer `{}` ({}), its tables and its machine.\n",
                    observer.id,
                    observer.document.path.display()
                ));
                observer_text.push_str(&tables.text);
                observer_text.push_str(&observer_machine(number, &prefix));
                observers.push((observer.id.clone(), tables.facts));
            }
            Err(refusals) => {
                for refusal in refusals {
                    writer.refusals.push(format!(
                        "the observer `{}` ({}): {refusal}",
                        observer.id,
                        observer.document.path.display()
                    ));
                }
                observers.push((observer.id.clone(), empty_facts()));
            }
        }
    }

    let readable: BTreeSet<String> = documents
        .machine
        .model
        .readable_variables
        .iter()
        .map(|variable| variable.id.clone())
        .collect();
    let world = World {
        facts: &facts,
        readable: &readable,
        observers: &observers,
    };

    let properties = &documents.properties.model;
    let mut assumed = Vec::new();
    let mut recurring = Vec::new();
    for assumption in &properties.assumptions {
        let place = format!("`{}` (<sce:{}>)", assumption.req, assumption.form.element());
        match &assumption.form {
            PropertyForm::Always { cond } => {
                if let Some(text) = writer.arrival(&world, &place, cond) {
                    assumed.push(text);
                }
            }
            PropertyForm::Recurs { cond, unless: None } => {
                if let Some(text) = writer.arrival(&world, &place, cond) {
                    recurring.push(text);
                }
            }
            PropertyForm::Recurs {
                unless: Some(_), ..
            } => writer.refusals.push(format!(
                "{place}: an assumption that an event keeps coming unless another condition \
                 holds, which has no translation: an assumption of the environment is a \
                 condition each event that comes meets (<sce:always>) or an event that keeps \
                 coming (<sce:recurs> with no unless)"
            )),
            PropertyForm::Step { .. }
            | PropertyForm::LeadsTo { .. }
            | PropertyForm::Possible { .. } => writer.refusals.push(format!(
                "{place}: an assumption in this form, which has no translation: an assumption \
                 of the environment is a condition each event that comes meets (<sce:always>) \
                 or an event that keeps coming (<sce:recurs>)"
            )),
        }
    }
    let fair_arrivals = !recurring.is_empty() && exploration.most > 0;

    let mut guarantees = Vec::new();
    let mut statements = String::new();
    for (at, property) in properties.guarantees.iter().enumerate() {
        let operator = format!("Guarantee{}", at + 1);
        let place = format!("`{}` (<sce:{}>)", property.req, property.form.element());
        let Some(slots) = writer.slots(&world, &place, &property.form) else {
            continue;
        };
        let parts: usize = slots.iter().map(|slot| slot.atoms.len()).sum();
        if parts == 0 {
            writer.refused.push(format!(
                "{place}: its condition reads nothing of the machine, so it passes for nothing"
            ));
            continue;
        }
        let judged = match property.form {
            PropertyForm::Always { .. } | PropertyForm::Step { .. } => Judged::Invariant,
            PropertyForm::Recurs { .. } | PropertyForm::LeadsTo { .. } => Judged::Temporal,
            PropertyForm::Possible { .. } => Judged::Possible,
        };
        let escape = fair_arrivals && judged == Judged::Temporal;
        let texts: Vec<String> = slots.iter().map(|slot| slot.text.clone()).collect();
        statements.push_str(&format!(
            "\\* {}\n{operator} ==\n    {}\n\n",
            property.req,
            formula(&property.form, &texts, escape)
        ));
        let mut strengthened = Vec::new();
        let mut number = 0usize;
        for (index, slot) in slots.iter().enumerate() {
            for (atom_at, atom) in slot.atoms.iter().enumerate() {
                number += 1;
                let stronger = strengthen(&slot.expr, atom_at, slot.positive);
                let Some(text) = writer.observation(&world, &place, &stronger, slot.previous)
                else {
                    continue;
                };
                let mut replaced = texts.clone();
                replaced[index] = text;
                let name = format!("{operator}Stronger{number}");
                statements.push_str(&format!(
                    "{name} ==\n    {}\n\n",
                    formula(&property.form, &replaced, escape)
                ));
                strengthened.push(Strengthened {
                    part: atom.part.clone(),
                    by: !atom.positive,
                    operator: name,
                });
            }
        }
        guarantees.push(Guarantee {
            req: property.req.clone(),
            form: property.form.element(),
            judged,
            operator,
            strengthened,
        });
    }

    if !writer.refusals.is_empty() {
        return Err(FormalError::NoTranslation {
            document: documents.properties.path.display().to_string(),
            refusals: writer.refusals,
        });
    }
    if !writer.refused.is_empty() {
        return Err(FormalError::PropertyRefused {
            document: documents.properties.path.display().to_string(),
            refusals: writer.refused,
        });
    }

    let module = format!("{}Properties", machine.module);
    let properties_label = documents.properties.path.display().to_string();
    let text = module_text(&ModuleParts {
        module: &module,
        machine: &machine.module,
        properties: &properties_label,
        integers: writer.integers,
        observers: &observer_text,
        count: observers.len(),
        outside: &exploration.outside,
        most: exploration.most,
        assumed: &assumed,
        recurring: &recurring,
        statements: &statements,
    });
    Ok(PropertiesModule {
        machine,
        module,
        text,
        guarantees,
        assumptions: properties
            .assumptions
            .iter()
            .map(|assumption| assumption.req.clone())
            .collect(),
        fair_arrivals,
        most: exploration.most,
    })
}

/// Facts of an observer that has no tables: its own refusals name why, and
/// the module is not written.
fn empty_facts() -> Facts {
    Facts {
        states: Vec::new(),
        data: Vec::new(),
    }
}

/// Whether `text` is printable ASCII, which a TLA+ string holds as written.
fn printable(text: &str) -> bool {
    text.chars().all(|c| (' '..='~').contains(&c))
}

/// `name` as the sequence of its tokens, as Appendix D holds an event's name.
fn event_name(name: &str) -> String {
    let tokens: Vec<String> = name.split('.').map(tla_string).collect();
    format!("<<{}>>", tokens.join(", "))
}

/// The observer numbered `number`, whose tables are named `prefix`: the
/// semantics instantiated over them, its start, and the event of an
/// observation taken to completion. An observer's macrostep is one step of
/// the module, so each is a function of its machine record, as Appendix D's
/// procedures are.
fn observer_machine(number: usize, prefix: &str) -> String {
    let instance = format!("Observer{number}");
    let mut text = format!("{instance} == INSTANCE {SEMANTICS} WITH\n");
    let mut substitutions: Vec<String> = vec!["Root <- Root".to_string()];
    for (constant, definition) in TABLES {
        substitutions.push(format!("{constant} <- {prefix}{definition}"));
    }
    substitutions.push("Outside <- {}".to_string());
    substitutions.push("MaxOutside <- 0".to_string());
    for part in PARTS {
        substitutions.push(format!("{part} <- TRUE"));
    }
    let last = substitutions.len() - 1;
    for (at, substitution) in substitutions.iter().enumerate() {
        let comma = if at == last { "" } else { "," };
        text.push_str(&format!("    {substitution}{comma}\n"));
    }
    text.push('\n');
    text.push_str(&format!(
        "RECURSIVE {prefix}Settle(_)\n\
         {prefix}Settle(m) ==\n    \
         IF m.phase # \"loop\" THEN m\n    \
         ELSE IF {instance}!SelectEventlessTransitions(m) # <<>>\n         \
         THEN {prefix}Settle({instance}!Microstep({instance}!SelectEventlessTransitions(m), m).m)\n    \
         ELSE IF m.intq # <<>>\n         \
         THEN {prefix}Settle({instance}!TakeEvent([m EXCEPT !.intq = Tail(@), !.event = Head(m.intq)]).m)\n    \
         ELSE m\n\n\
         {prefix}Start ==\n    \
         LET m0 == [conf |-> {{}}, hist |-> [h \\in {instance}!Histories |-> {{}}],\n               \
         data |-> {prefix}Data0, intq |-> <<>>, extq |-> <<>>, delayed |-> <<>>,\n               \
         event |-> NoEvent, phase |-> \"loop\"]\n    \
         IN {prefix}Settle({instance}!EnterStates(<<{instance}!InitialOf[Root]>>, m0).m)\n\n\
         {prefix}Take(m, e) ==\n    \
         IF e = NoEvent \\/ m.phase # \"loop\" THEN m\n    \
         ELSE {prefix}Settle({instance}!TakeEvent([m EXCEPT !.event = e]).m)\n\n"
    ));
    text
}

/// What [`module_text`] writes.
struct ModuleParts<'a> {
    module: &'a str,
    machine: &'a str,
    properties: &'a str,
    integers: bool,
    observers: &'a str,
    count: usize,
    outside: &'a [String],
    most: u32,
    assumed: &'a [String],
    recurring: &'a [String],
    statements: &'a str,
}

/// The text of the properties module.
fn module_text(parts: &ModuleParts<'_>) -> String {
    let mut text = format!("---- MODULE {} ----\n", parts.module);
    for comment in [
        format!(
            "\\* The properties {} of the machine {} (docs/adr/0115, D15),",
            parts.properties, parts.machine
        ),
        "\\* written by `sce-codegen formal properties`: regenerate it, never edit it.".to_string(),
        "\\* An observation is made at the end of each macrostep: the event it took,".to_string(),
        "\\* the states the machine is in and its data. The observations and the".to_string(),
        "\\* observers are history variables: they take every step the machine takes".to_string(),
        "\\* and refuse none.".to_string(),
    ] {
        text.push_str(&comment);
        text.push('\n');
    }
    if parts.integers {
        text.push_str(&format!("EXTENDS {}, Integers\n\n", parts.machine));
    } else {
        text.push_str(&format!("EXTENDS {}\n\n", parts.machine));
    }
    text.push_str(parts.observers);

    let numbers: Vec<usize> = (1..=parts.count).collect();
    let starts: Vec<String> = numbers.iter().map(|n| format!("Obs{n}Start")).collect();
    let takes: Vec<String> = numbers
        .iter()
        .map(|n| format!("Obs{n}Take(os[{n}], e)"))
        .collect();
    let outside: Vec<String> = parts.outside.iter().map(|name| event_name(name)).collect();
    text.push_str(&format!(
        "\\* The events that may come from outside, and the most that come.\n\
         PropOutside == {{{}}}\n\
         PropMaxOutside == {}\n\n",
        outside.join(", "),
        parts.most
    ));
    text.push_str(&format!(
        "VARIABLES taking, obs, before, observers\n\n\
         pvars == <<vars, taking, obs, before, observers>>\n\n\
         NoObservation == [made |-> FALSE, event |-> NoEvent, conf |-> {{}}, data |-> Data0]\n\n\
         ObserversAtStart == <<{}>>\n\n\
         ObserversTaking(os, e) == <<{}>>\n\n",
        starts.join(", "),
        takes.join(", ")
    ));
    text.push_str(
        "\\* The interpreter is settled: about to leave, or with no eventless transition\n\
         \\* enabled and nothing on its internal queue. A step that settles it ends a\n\
         \\* macrostep.\n\
         Settled == phase = \"exit\" \\/ (phase = \"loop\" /\\ MacrostepDone(Machine))\n\n\
         \\* The event the macrostep a step of the interpreter is in took: the one it\n\
         \\* takes from the external queue, none when the interpreter starts.\n\
         Taking == IF CanStart THEN NoEvent ELSE IF CanExternal THEN Head(externalQueue) ELSE taking\n\n\
         Observe ==\n    \
         IF Settled'\n    \
         THEN /\\ before' = [observation |-> obs, observers |-> observers]\n         \
         /\\ obs' = [made |-> TRUE, event |-> taking', conf |-> configuration',\n                    \
         data |-> datamodel']\n         \
         /\\ observers' = ObserversTaking(observers, taking')\n    \
         ELSE UNCHANGED <<obs, before, observers>>\n\n\
         PInit ==\n    \
         /\\ Init\n    \
         /\\ taking = NoEvent\n    \
         /\\ obs = NoObservation\n    \
         /\\ observers = ObserversAtStart\n    \
         /\\ before = [observation |-> NoObservation, observers |-> ObserversAtStart]\n\n\
         PStep == Step /\\ taking' = Taking /\\ Observe\n\n",
    );
    let assumed = if parts.assumed.is_empty() {
        "TRUE".to_string()
    } else {
        parts
            .assumed
            .iter()
            .map(|condition| format!("({condition})"))
            .collect::<Vec<_>>()
            .join(" /\\ ")
    };
    text.push_str(&format!(
        "\\* What the assumptions let come from outside.\n\
         Assumed(e) == {assumed}\n\n\
         PArrive(e) == Arrive(e) /\\ Assumed(e) /\\ UNCHANGED <<taking, obs, before, observers>>\n\n\
         PDue(i) == Due(i) /\\ UNCHANGED <<taking, obs, before, observers>>\n\n\
         PNext ==\n    \
         \\/ PStep\n    \
         \\/ \\E i \\in 1..Len(delayed) : PDue(i)\n    \
         \\/ \\E e \\in Outside : PArrive(e)\n\n\
         PSpec == PInit /\\ [][PNext]_pvars\n\n\
         \\* Time goes on: a delayed send that is due comes.\n\
         TimeGoesOn == WF_pvars(\\E i \\in 1..Len(delayed) : PDue(i))\n\n"
    ));
    let mut fairness = vec![
        "([]<>(~StepEnabled) \\/ []<><<PStep>>_pvars)".to_string(),
        "TimeGoesOn".to_string(),
    ];
    for (at, condition) in parts.recurring.iter().enumerate() {
        let name = format!("Recurring{}", at + 1);
        text.push_str(&format!("{name}(e) == {condition}\n\n"));
        fairness.push(format!(
            "SF_pvars(\\E e \\in Outside : {name}(e) /\\ PArrive(e))"
        ));
    }
    text.push_str(&format!(
        "\\* The interpreter is not starved, time goes on, and the events the\n\
         \\* assumptions say keep coming come.\n\
         PLiveSpec ==\n    \
         /\\ PSpec\n{}\n\n",
        fairness
            .iter()
            .map(|condition| format!("    /\\ {condition}"))
            .collect::<Vec<_>>()
            .join("\n")
    ));
    text.push_str(&format!(
        "\\* Every macrostep ends: there is always a next observation, or the session\n\
         \\* has ended.\n\
         {MACROSTEPS_END} == []<>(Settled \\/ phase = \"end\")\n\n\
         {WITHIN_MOST} == arrived < MaxOutside\n\n\
         BoundReached == <>(arrived = MaxOutside)\n\n"
    ));
    text.push_str(parts.statements);
    text.push_str("====\n");
    text
}

/// The formula of a guarantee in `form`, over its conditions translated
/// (`texts`, in the order [`Writer::slots`] gives them). `escape`: a run that
/// reaches the most events from outside, where an event the assumptions say
/// keeps coming stops coming, judges nothing of it.
fn formula(form: &PropertyForm, texts: &[String], escape: bool) -> String {
    let observed = |condition: &str| format!("(obs.made /\\ ({condition}))");
    let stated = match form {
        PropertyForm::Always { .. } => format!("obs.made => ({})", texts[0]),
        PropertyForm::Step { .. } => {
            format!("(obs.made /\\ before.observation.made) => ({})", texts[0])
        }
        PropertyForm::Recurs { unless: None, .. } => format!("[]<>{}", observed(&texts[0])),
        PropertyForm::Recurs {
            unless: Some(_), ..
        } => format!(
            "([]<>{}) \\/ (<>[]{})",
            observed(&texts[0]),
            observed(&texts[1])
        ),
        PropertyForm::LeadsTo { .. } => {
            format!("{} ~> {}", observed(&texts[0]), observed(&texts[1]))
        }
        PropertyForm::Possible { .. } => observed(&texts[0]),
    };
    if escape {
        format!("BoundReached \\/ ({stated})")
    } else {
        stated
    }
}

/// A part of a condition: a boolean part of it that is no `!`, `&&` or `||`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Atom {
    /// As the document writes it.
    part: String,
    /// Whether it stands under an even count of negations in the guarantee.
    positive: bool,
}

/// The parts of `expr`, written as `source`, in the order they are written.
fn atoms(expr: &Expr, source: &str, positive: bool, out: &mut Vec<Atom>) {
    match &expr.kind {
        ExprKind::Unary {
            op: UnaryOp::Not,
            operand,
        } => atoms(operand, source, !positive, out),
        ExprKind::Logical { left, right, .. } => {
            atoms(left, source, positive, out);
            atoms(right, source, positive, out);
        }
        ExprKind::Bool(_) => {}
        _ => out.push(Atom {
            part: source
                .get(expr.span.clone())
                .map_or_else(|| "?".to_string(), str::to_string),
            positive,
        }),
    }
}

/// `expr` with its part numbered `target` (in the order [`atoms`] gives)
/// replaced by what makes the guarantee stronger.
fn strengthen(expr: &Expr, target: usize, positive: bool) -> Expr {
    fn walk(expr: &Expr, target: usize, positive: bool, seen: &mut usize) -> Expr {
        match &expr.kind {
            ExprKind::Unary {
                op: UnaryOp::Not,
                operand,
            } => Expr::new(
                ExprKind::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(walk(operand, target, !positive, seen)),
                },
                expr.span.clone(),
            ),
            ExprKind::Logical { op, left, right } => Expr::new(
                ExprKind::Logical {
                    op: *op,
                    left: Box::new(walk(left, target, positive, seen)),
                    right: Box::new(walk(right, target, positive, seen)),
                },
                expr.span.clone(),
            ),
            ExprKind::Bool(_) => expr.clone(),
            _ => {
                let here = *seen;
                *seen += 1;
                if here == target {
                    Expr::new(ExprKind::Bool(!positive), expr.span.clone())
                } else {
                    expr.clone()
                }
            }
        }
    }
    walk(expr, target, positive, &mut 0)
}

/// One condition of a guarantee: its expression, its translation, whether it
/// stands where the guarantee is stronger when it is false, whether it may
/// read the observation before, and its parts.
struct Slot {
    expr: Expr,
    text: String,
    positive: bool,
    previous: bool,
    atoms: Vec<Atom>,
}

/// What a condition may read: the machine's states and the data it
/// publishes, and the observers.
struct World<'a> {
    facts: &'a Facts,
    readable: &'a BTreeSet<String>,
    observers: &'a [(String, Facts)],
}

/// Collects every refusal: a construct with no translation, and a property
/// refused for what it says.
struct Writer {
    refusals: Vec<String>,
    refused: Vec<String>,
    integers: bool,
}

impl Writer {
    /// The conditions of a guarantee in `form`, each translated, in the order
    /// [`formula`] reads them, or `None` with each refusal recorded.
    fn slots(&mut self, world: &World<'_>, place: &str, form: &PropertyForm) -> Option<Vec<Slot>> {
        let written: Vec<(&str, bool, bool)> = match form {
            PropertyForm::Always { cond } | PropertyForm::Possible { cond } => {
                vec![(cond.as_str(), true, false)]
            }
            PropertyForm::Step { cond } => vec![(cond.as_str(), true, true)],
            PropertyForm::Recurs { cond, unless } => {
                let mut written = vec![(cond.as_str(), true, false)];
                if let Some(unless) = unless {
                    written.push((unless.as_str(), true, false));
                }
                written
            }
            PropertyForm::LeadsTo { from, to } => {
                vec![(from.as_str(), false, false), (to.as_str(), true, false)]
            }
        };
        let mut slots = Vec::with_capacity(written.len());
        let mut whole = true;
        for (source, positive, previous) in written {
            let trimmed = source.trim();
            let expr = match parse_expression(trimmed) {
                Ok(expr) => expr,
                Err(refusal) => {
                    self.refusals
                        .push(format!("{place}: `{trimmed}`: {refusal}"));
                    whole = false;
                    continue;
                }
            };
            let Some(text) = self.observation(world, place, &expr, previous) else {
                whole = false;
                continue;
            };
            let mut found = Vec::new();
            atoms(&expr, trimmed, positive, &mut found);
            slots.push(Slot {
                expr,
                text,
                positive,
                previous,
                atoms: found,
            });
        }
        whole.then_some(slots)
    }

    /// `expr`, a condition of a guarantee, over the observation, or `None`
    /// with its refusal recorded. `previous`: whether it may read the
    /// observation before (`<sce:step>`).
    fn observation(
        &mut self,
        world: &World<'_>,
        place: &str,
        expr: &Expr,
        previous: bool,
    ) -> Option<String> {
        let mut scope = PropertyScope {
            world,
            reading: Reading::Observation {
                previous_allowed: previous,
                inside_previous: false,
            },
            about_machine: Vec::new(),
            integers: false,
        };
        let translated = document_model::translate(expr, &mut scope);
        self.integers |= scope.integers;
        self.boolean(place, translated)
    }

    /// `cond`, a condition of an assumption, over the event that comes
    /// (`e`), or `None` with its refusal recorded: as an assumption about the
    /// machine itself where it reads what the machine decides.
    fn arrival(&mut self, world: &World<'_>, place: &str, cond: &str) -> Option<String> {
        let trimmed = cond.trim();
        let expr = match parse_expression(trimmed) {
            Ok(expr) => expr,
            Err(refusal) => {
                self.refusals
                    .push(format!("{place}: `{trimmed}`: {refusal}"));
                return None;
            }
        };
        let mut scope = PropertyScope {
            world,
            reading: Reading::Arrival,
            about_machine: Vec::new(),
            integers: false,
        };
        let translated = document_model::translate(&expr, &mut scope);
        self.integers |= scope.integers;
        if !scope.about_machine.is_empty() {
            self.refused.push(format!(
                "{place}: an assumption about the machine itself: it reads {}, and an \
                 assumption is of what the environment does, the event that comes \
                 (_event.name), never of what the machine decides",
                scope.about_machine.join(", ")
            ));
            return None;
        }
        self.boolean(place, translated)
    }

    /// A condition's translation as a boolean, or `None` with why recorded.
    fn boolean(&mut self, place: &str, translated: Result<Typed<Tla>, String>) -> Option<String> {
        match translated {
            // Unbracketed: every formula brackets each condition it holds.
            Ok(typed) if typed.ty == Ty::Bool => Some(typed.term.written().text),
            Ok(typed) => {
                self.refusals
                    .push(format!("{place}: a condition that is {}", typed.ty.word()));
                None
            }
            Err(why) => {
                self.refusals.push(format!("{place}: {why}"));
                None
            }
        }
    }
}

/// What a condition is read over.
#[derive(Debug, Clone, Copy)]
enum Reading {
    /// The last observation, and the one before where `previous_allowed`.
    Observation {
        previous_allowed: bool,
        inside_previous: bool,
    },
    /// The event that comes from outside, `e`.
    Arrival,
}

/// A property's condition, as [`document_model::translate`] reads it.
struct PropertyScope<'a> {
    world: &'a World<'a>,
    reading: Reading,
    /// What an assumption read of the machine, each as written.
    about_machine: Vec<String>,
    integers: bool,
}

/// What a refusal of a condition says where an assumption reads the machine:
/// the assumption is then refused as one about the machine itself, which
/// names what it read, and this text is not shown.
const OF_THE_MACHINE: &str = "reads the machine";

impl<'a> PropertyScope<'a> {
    /// The record of the observation read, and of the observers then.
    fn records(&self) -> (&'static str, &'static str) {
        match self.reading {
            Reading::Observation {
                inside_previous: true,
                ..
            } => ("before.observation", "before.observers"),
            _ => ("obs", "observers"),
        }
    }

    /// Whether an assumption is read, which may read nothing of the machine:
    /// `what` is then recorded.
    fn of_the_machine(&mut self, what: String) -> bool {
        if matches!(self.reading, Reading::Arrival) {
            self.about_machine.push(what);
            true
        } else {
            false
        }
    }

    /// The observer named `id`, by its number, and its facts.
    fn observer(&self, id: &str) -> Option<(usize, &'a Facts)> {
        let observers: &'a [(String, Facts)] = self.world.observers;
        observers
            .iter()
            .position(|(name, _)| name == id)
            .map(|at| (at + 1, &observers[at].1))
    }

    /// The one state id `args` holds, as a string literal.
    fn state_argument(args: &[Expr]) -> Result<&str, String> {
        match args {
            [Expr {
                kind: ExprKind::Str(state),
                ..
            }] => Ok(state.as_str()),
            _ => Err("In() takes one state id, written as a string".to_string()),
        }
    }
}

/// What a property's condition reads, written as TLA+, and its type.
fn reads(tla: Tla, ty: Ty) -> Typed<Tla> {
    Typed::new(Term::Var(tla), ty)
}

impl Scope for PropertyScope<'_> {
    type Var = Tla;

    fn ident(&mut self, name: &str) -> Result<Typed<Tla>, String> {
        if self.of_the_machine(format!("`{name}`")) {
            return Err(OF_THE_MACHINE.to_string());
        }
        let (observation, _) = self.records();
        match self.world.facts.data.iter().find(|(id, _)| id == name) {
            Some((_, ty)) if self.world.readable.contains(name) => {
                Ok(reads(Tla::atom(format!("{observation}.data.{name}")), *ty))
            }
            Some(_) => Err(format!(
                "`{name}` is a <data> the machine does not publish, and a property reads \
                 what the machine's user sees"
            )),
            None => Err(format!("`{name}` is no <data> of the machine")),
        }
    }

    fn call(&mut self, callee: &Expr, args: &[Expr]) -> Result<Typed<Tla>, String> {
        match &callee.kind {
            ExprKind::Ident(name) if name == "In" => {
                let state = Self::state_argument(args)?;
                if self.of_the_machine(format!("In('{state}')")) {
                    return Err(OF_THE_MACHINE.to_string());
                }
                if !self.world.facts.states.iter().any(|known| known == state) {
                    return Err(format!("`{state}` is no state of the machine"));
                }
                let (observation, _) = self.records();
                Ok(reads(
                    Tla {
                        text: format!("{} \\in {observation}.conf", tla_string(state)),
                        binding: Binding::Compound,
                    },
                    Ty::Bool,
                ))
            }
            ExprKind::Ident(name) if name == "previous" => {
                let Reading::Observation {
                    previous_allowed,
                    inside_previous,
                } = self.reading
                else {
                    self.about_machine.push("previous(...)".to_string());
                    return Err(OF_THE_MACHINE.to_string());
                };
                if !previous_allowed {
                    return Err("previous() reads the observation before, which only an \
                                <sce:step> condition reads"
                        .to_string());
                }
                if inside_previous {
                    return Err(
                        "previous() inside previous(), which has no translation: only \
                                the observation before the last one is kept"
                            .to_string(),
                    );
                }
                let [argument] = args else {
                    return Err("previous() takes one expression".to_string());
                };
                self.reading = Reading::Observation {
                    previous_allowed,
                    inside_previous: true,
                };
                let translated = document_model::translate(argument, self);
                self.reading = Reading::Observation {
                    previous_allowed,
                    inside_previous: false,
                };
                translated
            }
            ExprKind::Member { object, property } if property.text == "In" => {
                let ExprKind::Ident(id) = &object.kind else {
                    return Err("a call of In() on what is no observer".to_string());
                };
                let state = Self::state_argument(args)?;
                if self.of_the_machine(format!("{id}.In('{state}')")) {
                    return Err(OF_THE_MACHINE.to_string());
                }
                let Some((number, facts)) = self.observer(id) else {
                    return Err(format!("`{id}` is no observer of the properties"));
                };
                if !facts.states.iter().any(|known| known == state) {
                    return Err(format!("`{state}` is no state of the observer `{id}`"));
                }
                let (_, observers) = self.records();
                Ok(reads(
                    Tla {
                        text: format!("{} \\in {observers}[{number}].conf", tla_string(state)),
                        binding: Binding::Compound,
                    },
                    Ty::Bool,
                ))
            }
            _ => Err("a call other than In() and previous(), which has no translation".to_string()),
        }
    }

    fn member(&mut self, object: &Expr, property: Option<&str>) -> Result<Typed<Tla>, String> {
        let ExprKind::Ident(name) = &object.kind else {
            let of_event = matches!(
                &object.kind,
                ExprKind::Member { object: inner, .. }
                    if matches!(&inner.kind, ExprKind::Ident(name) if name == "_event")
            );
            return Err(if of_event {
                "a member of _event other than its name, which the observation does not hold"
            } else {
                "a member of what is neither _event nor an observer, which has no translation"
            }
            .to_string());
        };
        match (name.as_str(), property) {
            ("_event", Some("name")) => match self.reading {
                Reading::Arrival => Ok(reads(Tla::atom("e".to_string()), Ty::Name)),
                Reading::Observation { .. } => {
                    let (observation, _) = self.records();
                    Ok(reads(Tla::atom(format!("{observation}.event")), Ty::Name))
                }
            },
            ("_event", _) => Err(
                "a member of _event other than its name, which the observation does not hold"
                    .to_string(),
            ),
            (id, Some(field)) => {
                if self.of_the_machine(format!("{id}.{field}")) {
                    return Err(OF_THE_MACHINE.to_string());
                }
                let Some((number, facts)) = self.observer(id) else {
                    return Err(format!("`{id}` is no observer of the properties"));
                };
                let Some((_, ty)) = facts.data.iter().find(|(known, _)| known == field) else {
                    return Err(format!("`{field}` is no <data> of the observer `{id}`"));
                };
                let (_, observers) = self.records();
                Ok(reads(
                    Tla::atom(format!("{observers}[{number}].data.{field}")),
                    *ty,
                ))
            }
            (_, None) => Err("a computed member, which has no translation".to_string()),
        }
    }

    fn string(&mut self, text: &str) -> Result<Typed<Tla>, String> {
        if text.split('.').any(str::is_empty) || !printable(text) || text.contains(' ') {
            return Err(format!(
                "'{text}' is no event name: tokens of printable characters separated by single \
                 dots"
            ));
        }
        Ok(reads(Tla::atom(event_name(text)), Ty::Name))
    }

    fn needs_integers(&mut self) {
        self.integers = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TURNSTILE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="ecmascript" name="turnstile" initial="locked">
        <datamodel><data id="coins" expr="0"/></datamodel>
        <state id="locked">
            <transition event="coin" target="unlocked">
                <assign location="coins" expr="coins + 1"/>
            </transition>
        </state>
        <state id="unlocked">
            <transition event="push" target="locked"/>
        </state>
    </scxml>"#;

    /// An observer that remembers whether a coin was taken since the last push.
    const SINCE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="null" name="since" initial="unpaid">
        <state id="unpaid"><transition event="coin" target="paid"/></state>
        <state id="paid"><transition event="push" target="unpaid"/></state>
    </scxml>"#;

    const EXPLORATION: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
            xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="exploration"
            name="turnstile_exploration">
        <sce:machine src="turnstile.scxml"/>
        <sce:outside most="3">
            <sce:event name="coin"/>
            <sce:event name="push"/>
        </sce:outside>
    </scxml>"#;

    fn statechart(text: &str, name: &str) -> Read<SCXMLModel> {
        let mut model = crate::parser::SCXMLParser::new()
            .parse_string(text, name)
            .expect("the statechart parses");
        crate::analyzer::analyze(&mut model, "");
        Read {
            path: PathBuf::from(format!("{name}.scxml")),
            text: text.to_string(),
            model,
        }
    }

    fn forge(text: &str, name: &str) -> ForgeDocument {
        crate::forge::parser::parse_forge_with_imports(text, crate::DocumentLabel::symmetric(name))
            .expect("the document parses")
            .expect("a forge document")
            .document
    }

    /// The documents of a judgement: the turnstile, the properties whose body
    /// is `body`, observing with `since` when `observed`, and `exploration`.
    fn documents(body: &str, observed: bool, exploration: &str) -> Documents {
        let observer = if observed {
            r#"<sce:observer id="since" src="since.scxml"/>"#
        } else {
            ""
        };
        let text = format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="properties"
                name="turnstile_properties">
            <sce:machine src="turnstile.scxml"/>
            {observer}
            {body}
            </scxml>"#
        );
        let ForgeDocument::Properties(properties) = forge(&text, "turnstile_properties") else {
            panic!("a properties document");
        };
        let ForgeDocument::Exploration(explored) = forge(exploration, "turnstile_exploration")
        else {
            panic!("an exploration document");
        };
        let observers = if observed {
            vec![ReadObserver {
                id: "since".to_string(),
                document: statechart(SINCE, "since"),
            }]
        } else {
            Vec::new()
        };
        Documents {
            machine: statechart(TURNSTILE, "turnstile"),
            properties: Read {
                path: PathBuf::from("turnstile_properties.scxml"),
                text,
                model: properties,
            },
            observers,
            exploration: Read {
                path: PathBuf::from("turnstile_exploration.scxml"),
                text: exploration.to_string(),
                model: explored,
            },
        }
    }

    fn refusals_of(result: Result<PropertiesModule, FormalError>) -> (&'static str, Vec<String>) {
        match result {
            Err(FormalError::NoTranslation { refusals, .. }) => ("no-translation", refusals),
            Err(FormalError::PropertyRefused { refusals, .. }) => ("property-refused", refusals),
            Err(FormalError::Declaration { problems, .. }) => ("declaration", problems),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    const EVERY_FORM: &str = r#"<sce:assume>
            <sce:always req="ENV-1" cond="_event.name != 'kick'"/>
        </sce:assume>
        <sce:guarantee>
            <sce:always req="TURN-1" cond="!(In('locked') &amp;&amp; In('unlocked'))"/>
            <sce:step req="TURN-2" cond="coins &gt;= previous(coins)"/>
            <sce:leads-to req="TURN-3" from="In('unlocked')" to="In('locked')"/>
            <sce:possible req="TURN-4" cond="In('unlocked')"/>
            <sce:always req="TURN-5" cond="!In('unlocked') || since.In('paid')"/>
        </sce:guarantee>"#;

    /// Turns red if a form is written as another formula, a condition stops
    /// reading the observation it names, or the observation before stops
    /// being `before`.
    #[test]
    fn each_form_is_its_formula_over_the_observation() {
        let written = write(&documents(EVERY_FORM, true, EXPLORATION)).expect("a module");
        let text = &written.text;
        for expected in [
            "---- MODULE turnstileProperties ----\n",
            "EXTENDS turnstile",
            "\\* TURN-1\nGuarantee1 ==\n    obs.made => (~((\"locked\" \\in obs.conf) /\\ (\"unlocked\" \\in obs.conf)))\n",
            "Guarantee2 ==\n    (obs.made /\\ before.observation.made) => (obs.data.coins >= before.observation.data.coins)\n",
            "Guarantee3 ==\n    (obs.made /\\ (\"unlocked\" \\in obs.conf)) ~> (obs.made /\\ (\"locked\" \\in obs.conf))\n",
            "Guarantee4 ==\n    (obs.made /\\ (\"unlocked\" \\in obs.conf))\n",
            "Guarantee5 ==\n    obs.made => ((~(\"unlocked\" \\in obs.conf)) \\/ (\"paid\" \\in observers[1].conf))\n",
            "Assumed(e) == (e # <<\"kick\">>)\n",
            "PropOutside == {<<\"coin\">>, <<\"push\">>}\n",
            "PropMaxOutside == 3\n",
            "====\n",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in\n{text}");
        }
        let judged: Vec<(&str, Judged)> = written
            .guarantees
            .iter()
            .map(|g| (g.req.as_str(), g.judged))
            .collect();
        assert_eq!(
            judged,
            [
                ("TURN-1", Judged::Invariant),
                ("TURN-2", Judged::Invariant),
                ("TURN-3", Judged::Temporal),
                ("TURN-4", Judged::Possible),
                ("TURN-5", Judged::Invariant),
            ]
        );
        assert_eq!(written.assumptions, ["ENV-1"]);
        assert!(!written.fair_arrivals);
    }

    /// Turns red if a part is made stronger the wrong way: false where it
    /// stands under an even count of negations (the `to` of a leads-to),
    /// true elsewhere (its `from`, a part under one negation).
    #[test]
    fn each_part_is_made_stronger_by_its_polarity() {
        let written = write(&documents(EVERY_FORM, true, EXPLORATION)).expect("a module");
        let text = &written.text;
        for expected in [
            "Guarantee1Stronger1 ==\n    obs.made => (~(TRUE /\\ (\"unlocked\" \\in obs.conf)))\n",
            "Guarantee1Stronger2 ==\n    obs.made => (~((\"locked\" \\in obs.conf) /\\ TRUE))\n",
            "Guarantee3Stronger1 ==\n    (obs.made /\\ (TRUE)) ~> (obs.made /\\ (\"locked\" \\in obs.conf))\n",
            "Guarantee3Stronger2 ==\n    (obs.made /\\ (\"unlocked\" \\in obs.conf)) ~> (obs.made /\\ (FALSE))\n",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in\n{text}");
        }
        let leads_to = &written.guarantees[2];
        assert_eq!(
            leads_to
                .strengthened
                .iter()
                .map(|s| (s.part.as_str(), s.by))
                .collect::<Vec<_>>(),
            [("In('unlocked')", true), ("In('locked')", false)]
        );
    }

    /// Turns red if the observer stops being a statechart of its own over
    /// Appendix D, taken a macrostep at a time.
    #[test]
    fn an_observer_is_the_semantics_over_its_own_tables() {
        let written = write(&documents(EVERY_FORM, true, EXPLORATION)).expect("a module");
        let text = &written.text;
        for expected in [
            "Obs1States == {",
            "Obs1Guard(cond, data, event, conf) == FALSE\n",
            "Observer1 == INSTANCE AppendixD WITH\n    Root <- Root,\n    States <- Obs1States,\n",
            "    RunToCompletion <- TRUE\n",
            "RECURSIVE Obs1Settle(_)\n",
            "Obs1Take(m, e) ==\n",
            "ObserversAtStart == <<Obs1Start>>\n",
            "ObserversTaking(os, e) == <<Obs1Take(os[1], e)>>\n",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in\n{text}");
        }
    }

    /// Turns red if an assumption may read the machine, which would excuse it
    /// of anything, or is refused without its name.
    #[test]
    fn an_assumption_about_the_machine_itself_is_refused_by_name() {
        let body = r#"<sce:assume>
                <sce:always req="ENV-1" cond="!In('unlocked') || _event.name == 'coin'"/>
                <sce:recurs req="ENV-2" cond="coins &lt; 3"/>
            </sce:assume>
            <sce:guarantee>
                <sce:leads-to req="TURN-1" from="In('unlocked')" to="In('locked')"/>
            </sce:guarantee>"#;
        let (code, found) = refusals_of(write(&documents(body, false, EXPLORATION)));
        assert_eq!(code, "property-refused", "{found:?}");
        let found = found.join("\n");
        for expected in [
            "`ENV-1` (<sce:always>): an assumption about the machine itself: it reads In('unlocked')",
            "`ENV-2` (<sce:recurs>): an assumption about the machine itself: it reads `coins`",
        ] {
            assert!(found.contains(expected), "missing {expected:?} in\n{found}");
        }
    }

    /// Turns red if a construct with no translation is translated in part,
    /// dropped in silence, or refused alone while the others go unnamed.
    #[test]
    fn every_construct_without_a_translation_is_refused_by_name() {
        let body = r#"<sce:assume>
                <sce:leads-to req="ENV-1" from="_event.name == 'coin'" to="_event.name == 'push'"/>
            </sce:assume>
            <sce:guarantee>
                <sce:always req="TURN-1" cond="_event.data.amount == 1"/>
                <sce:always req="TURN-2" cond="previous(coins) == 0"/>
                <sce:always req="TURN-3" cond="In('nowhere')"/>
                <sce:always req="TURN-4" cond="since.In('paid')"/>
                <sce:recurs req="TURN-5" cond="coins"/>
                <sce:step req="TURN-6" cond="previous(previous(coins)) == coins"/>
            </sce:guarantee>"#;
        let (code, found) = refusals_of(write(&documents(body, false, EXPLORATION)));
        assert_eq!(code, "no-translation", "{found:?}");
        let found = found.join("\n");
        for expected in [
            "`ENV-1` (<sce:leads-to>): an assumption in this form, which has no translation",
            "`TURN-1` (<sce:always>): a member of _event other than its name",
            "`TURN-2` (<sce:always>): previous() reads the observation before, which only an <sce:step> condition reads",
            "`TURN-3` (<sce:always>): `nowhere` is no state of the machine",
            "`TURN-4` (<sce:always>): `since` is no observer of the properties",
            "`TURN-5` (<sce:recurs>): a condition that is an integer",
            "`TURN-6` (<sce:step>): previous() inside previous()",
        ] {
            assert!(found.contains(expected), "missing {expected:?} in\n{found}");
        }
    }

    /// Turns red if an observer may send, which would make it more than a
    /// history of what the machine did.
    #[test]
    fn an_observer_that_sends_is_refused_by_name() {
        let body = r#"<sce:guarantee>
                <sce:always req="TURN-1" cond="!In('unlocked') || since.In('paid')"/>
            </sce:guarantee>"#;
        let mut read = documents(body, true, EXPLORATION);
        read.observers[0].document = statechart(
            &SINCE.replace(
                r#"<transition event="coin" target="paid"/>"#,
                r#"<transition event="coin" target="paid"><send event="told"/></transition>"#,
            ),
            "since",
        );
        let (code, found) = refusals_of(write(&read));
        assert_eq!(code, "no-translation", "{found:?}");
        assert!(
            found
                .iter()
                .any(|r| r.starts_with("the observer `since` (since.scxml): ")
                    && r.contains("an observer sends nothing")),
            "{found:?}"
        );
    }

    /// Turns red if a guarantee that reads nothing of the machine is judged,
    /// where it can only pass for nothing.
    #[test]
    fn a_guarantee_that_reads_nothing_is_refused_as_vacuous() {
        let body = r#"<sce:guarantee>
                <sce:always req="TURN-1" cond="true"/>
            </sce:guarantee>"#;
        let (code, found) = refusals_of(write(&documents(body, false, EXPLORATION)));
        assert_eq!(code, "property-refused", "{found:?}");
        assert_eq!(
            found,
            ["`TURN-1` (<sce:always>): its condition reads nothing of the machine, so it passes for nothing"]
        );
    }

    /// Turns red if an exploration may name an event the machine takes on no
    /// transition, which is no value of its alphabet.
    #[test]
    fn an_exploration_names_only_events_the_machine_takes() {
        let body = r#"<sce:guarantee>
                <sce:always req="TURN-1" cond="!In('unlocked')"/>
            </sce:guarantee>"#;
        let exploration = EXPLORATION.replace(
            r#"<sce:event name="push"/>"#,
            r#"<sce:event name="push"/><sce:event name="kick"/>"#,
        );
        let (code, found) = refusals_of(write(&documents(body, false, &exploration)));
        assert_eq!(code, "declaration", "{found:?}");
        assert_eq!(
            found,
            ["the event `kick` is taken by no transition of turnstile.scxml, so it is no event of its alphabet"]
        );
    }

    /// Turns red if a progress guarantee is judged over runs that end at the
    /// bound where the assumptions say an event keeps coming, or a safety one
    /// is excused at it.
    #[test]
    fn progress_is_judged_beyond_the_bound_only_where_arrivals_recur() {
        let body = r#"<sce:assume>
                <sce:recurs req="ENV-1" cond="_event.name == 'push'"/>
            </sce:assume>
            <sce:guarantee>
                <sce:always req="TURN-1" cond="!(In('locked') &amp;&amp; In('unlocked'))"/>
                <sce:leads-to req="TURN-2" from="In('unlocked')" to="In('locked')"/>
            </sce:guarantee>"#;
        let written = write(&documents(body, false, EXPLORATION)).expect("a module");
        assert!(written.fair_arrivals);
        let text = &written.text;
        for expected in [
            "Recurring1(e) == e = <<\"push\">>\n",
            "    /\\ SF_pvars(\\E e \\in Outside : Recurring1(e) /\\ PArrive(e))\n",
            "Guarantee1 ==\n    obs.made => (",
            "Guarantee2 ==\n    BoundReached \\/ ((obs.made /\\ (\"unlocked\" \\in obs.conf)) ~> ",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in\n{text}");
        }
    }

    /// Turns red if a run is configured otherwise than over the machine's own
    /// tables, the exploration's bounds and every part of Appendix D in.
    #[test]
    fn a_run_is_configured_over_the_tables_and_the_exploration() {
        let written = write(&documents(EVERY_FORM, true, EXPLORATION)).expect("a module");
        let config = written.config(Spec::Live, Check::Property("Guarantee3"));
        for expected in [
            "SPECIFICATION PLiveSpec\nCONSTANTS\n    Root = \"scxml\"\n",
            "    States <- DocStates\n",
            "    DocumentOrder <- DocOrder\n",
            "    Outside <- PropOutside\n    MaxOutside <- PropMaxOutside\n",
            "    RunToCompletion = TRUE\n",
            "PROPERTY\n    Guarantee3\nCHECK_DEADLOCK FALSE\n",
        ] {
            assert!(
                config.contains(expected),
                "missing {expected:?} in\n{config}"
            );
        }
        let (name, possible) = written.possible_module("Guarantee4");
        assert_eq!(name, "turnstilePropertiesPossible");
        assert!(
            possible.contains("PossibleInit == PInit /\\ sce_holds = (Guarantee4)"),
            "{possible}"
        );
        assert!(
            possible.contains("[][PossibleNext]_<<pvars, sce_holds>>"),
            "{possible}"
        );
    }
}
