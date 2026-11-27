// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A document's second encoding (docs/adr/0119, D2): the JANI model (Budde,
//! Dehnert, Hahn, Hartmanns, Junges and Turrini, TACAS 2017) of the document
//! whose TLA+ module `super::document_model` writes, from the same tables and
//! operators ([`ModelTables`]), for Storm to read.
//!
//! JANI holds variables of bounded type, guards and assignments, and no set,
//! sequence or recursion, so Appendix D cannot be written in it once and given
//! tables, as `docs/formal/AppendixD.tla` is. The model is Appendix D
//! specialised to the document instead: the control of a run (the
//! configuration, the history recorded, the two queues and the delayed sends,
//! the current event, the phase, what the last step told, the events that came
//! from outside and the outcome) is evaluated here, step by step, by the
//! operators of AppendixD.tla written again over the tables, and the datamodel
//! is left to the model, each step an edge whose guard is the values of the
//! conditions that select it and whose assignments are what its blocks write.
//! It is partial evaluation of the semantics with the data held symbolic
//! (Jones, Gomard and Sestoft, *Partial Evaluation and Automatic Program
//! Generation*, 1993), and its output is checked rather than the generator
//! proved: the state graph Storm builds of it must be the graph TLC writes of
//! the TLA+ module, state by state (D4.1), and every run TLC accepts must be a
//! path of it (D4.2), which `super::quantities` judges.
//!
//! The control is one bounded variable, `control`, whose values are the
//! controls the evaluation reached, in the order it reached them; each
//! `<data>` is a variable `data_<id>`. [`Encoding::state`] gives a state of
//! the model as the state of the TLA+ module it stands for, over the module's
//! own variables.
//!
//! The machine is deterministic: given the values of its conditions, at most
//! one step of the interpreter is enabled, as in AppendixD.tla with every part
//! in. The environment's choices are edges of their own: a delayed send coming
//! due (`Due`) and an event arriving from outside (`Arrive`), at any moment
//! the interpreter is running, as the TLA+ module takes them. No distribution
//! is given to any choice yet: every edge is taken with probability one, and
//! the model is the Markov decision process whose every choice is the
//! environment's or the interleaving of the environment with the machine. A
//! distribution is an assumption of a properties document (D2) and weighs only
//! these edges, never the machine's, which has no choice to weigh.
//!
//! What the TLA+ generator refuses has no model here either (D5): the tables
//! are the ones it writes, so a refused document is refused before anything is
//! evaluated. A document whose control takes more values than
//! [`LARGEST_CONTROL`], or one step of which is selected under more sets of
//! condition values than [`LARGEST_PATHS`], is refused by name, never written
//! in part.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value as Json};

use super::document_model::{self, Arithmetic, Comparison, Kind, ModelTables, RowKind, Term, Ty};
use super::tla_value::{State, Value};
use super::FormalError;
use crate::model::SCXMLModel;

/// The most values the control of a run may take in a model. Each is a value
/// of the variable `control` and a set of edges, so the bound is what keeps a
/// document whose control does not end (a queue that grows on every step) from
/// being enumerated for ever.
pub const LARGEST_CONTROL: usize = 100_000;

/// The most sets of condition values one step of one control may be selected
/// under: each is an edge, and their count doubles with each condition a
/// selection consults.
pub const LARGEST_PATHS: usize = 4096;

/// The variable of the model that holds the control of a run.
pub const CONTROL: &str = "control";

/// The one location of the model's automaton: the control is a variable.
const LOCATION: &str = "run";

/// The automaton's name.
const AUTOMATON: &str = "machine";

/// What AppendixD.tla's `outcome` is before the session ends (`None`).
pub const NO_OUTCOME: &str = "<none>";

/// The operators of JANI this model writes that are not ASCII: conjunction,
/// disjunction, negation, inequality and the order that admits equality.
/// Greater-than and greater-or-equal are JANI's derived operators, which a
/// model must declare as a feature; they are written as less-than and
/// less-or-equal with their operands swapped.
const AND: &str = "\u{2227}";
const OR: &str = "\u{2228}";
const NOT: &str = "\u{ac}";
const NOT_EQUAL: &str = "\u{2260}";
const LESS_OR_EQUAL: &str = "\u{2264}";

/// An event's name as the sequence of its tokens; the empty one is no event.
pub type Event = Vec<String>;

/// The variable of the model a `<data>` is held in.
pub fn data_variable(name: &str) -> String {
    format!("data_{name}")
}

/// AppendixD.tla's `phase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    Start,
    Loop,
    Exit,
    End,
}

impl Phase {
    /// The phase as AppendixD.tla spells it.
    pub fn name(self) -> &'static str {
        match self {
            Phase::Start => "start",
            Phase::Loop => "loop",
            Phase::Exit => "exit",
            Phase::End => "end",
        }
    }
}

/// AppendixD.tla's `told`: what the last step exited, which rows it took and
/// what it entered, in order. States are numbered by their place in the
/// document order, rows from 1.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Told {
    pub exited: Vec<usize>,
    pub fired: Vec<usize>,
    pub entered: Vec<usize>,
}

/// The control of a run: every variable of AppendixD.tla but the datamodel.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Control {
    pub configuration: BTreeSet<usize>,
    /// The value recorded of each `<history>`, every one of the document.
    pub history: BTreeMap<usize, BTreeSet<usize>>,
    pub internal: Vec<Event>,
    pub external: Vec<Event>,
    pub delayed: Vec<Event>,
    pub event: Event,
    pub phase: Phase,
    pub told: Told,
    pub arrived: u32,
    pub outcome: Option<usize>,
}

/// The kind of a step, the action of its edge: the interpreter's five and the
/// environment's two, as AppendixD.tla names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    Start,
    Eventless,
    Internal,
    External,
    Stop,
    Due,
    Arrive,
}

impl Action {
    pub const ALL: [Action; 7] = [
        Action::Start,
        Action::Eventless,
        Action::Internal,
        Action::External,
        Action::Stop,
        Action::Due,
        Action::Arrive,
    ];

    /// The action's name in the model, which is the kind of step a run trace
    /// names for the interpreter's (`crate::run_trace`).
    pub fn name(self) -> &'static str {
        match self {
            Action::Start => "start",
            Action::Eventless => "eventless",
            Action::Internal => "internal",
            Action::External => "external",
            Action::Stop => "stop",
            Action::Due => "due",
            Action::Arrive => "arrive",
        }
    }

    /// The action named `name`.
    pub fn named(name: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|action| action.name() == name)
    }

    /// Whether the step is the environment's, which no run says.
    pub fn of_environment(self) -> bool {
        matches!(self, Action::Due | Action::Arrive)
    }
}

/// The environment the model is of: what a TLC configuration of the TLA+
/// module gives `Outside` and `MaxOutside`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Environment {
    /// The events that may come from outside, each the sequence of its tokens.
    pub outside: Vec<Event>,
    /// The most of them that come.
    pub max_outside: u32,
}

/// A document's JANI model: its name, its text, and how its states read as the
/// TLA+ module's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JaniModel {
    pub name: String,
    pub text: String,
    pub encoding: Encoding,
}

/// How a state of the JANI model reads as a state of the TLA+ module: the
/// controls by their value of `control`, and the names of the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoding {
    order: Vec<String>,
    histories: Vec<usize>,
    data: Vec<(String, Ty)>,
    controls: Vec<Control>,
    /// The rows of each source's own transitions, in document order
    /// (AppendixD.tla's `TransitionsOf`).
    transitions_of: BTreeMap<String, Vec<usize>>,
}

impl Encoding {
    /// How many values the control takes.
    pub fn control_count(&self) -> usize {
        self.controls.len()
    }

    /// The row of the transition a run trace names by its source and its
    /// position among that source's transitions, or 0, which is no row, as
    /// the trace module's `TraceRow` reads it.
    pub fn trace_row(&self, source: &str, index: u32) -> i64 {
        self.transitions_of
            .get(source)
            .and_then(|rows| rows.get(usize::try_from(index).ok()?))
            .map_or(0, |&row| i64::try_from(row).unwrap_or(0))
    }

    /// The state of the TLA+ module that the state of the JANI model whose
    /// variables have `valuation` stands for, over the module's variables.
    pub fn state(&self, valuation: &Map<String, Json>) -> Result<State, String> {
        let control = valuation
            .get(CONTROL)
            .and_then(Json::as_u64)
            .and_then(|control| usize::try_from(control).ok())
            .ok_or_else(|| format!("the valuation gives no {CONTROL}: {valuation:?}"))?;
        let c = self
            .controls
            .get(control)
            .ok_or_else(|| format!("{CONTROL} = {control}, which the model gives no control"))?;
        let mut data = BTreeMap::new();
        for (name, ty) in &self.data {
            let variable = data_variable(name);
            let value = match (ty, valuation.get(&variable)) {
                (Ty::Int, Some(Json::Number(number))) => Value::Int(
                    number
                        .as_i64()
                        .ok_or_else(|| format!("{variable} = {number}, which is no integer"))?,
                ),
                (Ty::Bool, Some(Json::Bool(value))) => Value::Bool(*value),
                (_, other) => {
                    return Err(format!(
                        "the valuation gives {variable} as {other:?}, not {}",
                        ty.word()
                    ))
                }
            };
            data.insert(Value::string(name), value);
        }
        let name = |state: usize| Value::string(&self.order[state]);
        let names = |states: &[usize]| Value::Seq(states.iter().map(|&s| name(s)).collect());
        let set = |states: &BTreeSet<usize>| Value::Set(states.iter().map(|&s| name(s)).collect());
        let event = |event: &Event| Value::strings(event.iter().map(String::as_str));
        let queue = |events: &[Event]| Value::Seq(events.iter().map(event).collect());
        let empty = BTreeSet::new();
        let mut state = State::new();
        state.insert("configuration".to_string(), set(&c.configuration));
        state.insert(
            "historyValue".to_string(),
            Value::function(
                self.histories
                    .iter()
                    .map(|&h| (name(h), set(c.history.get(&h).unwrap_or(&empty))))
                    .collect(),
            ),
        );
        state.insert("datamodel".to_string(), Value::function(data));
        state.insert("internalQueue".to_string(), queue(&c.internal));
        state.insert("externalQueue".to_string(), queue(&c.external));
        state.insert("delayed".to_string(), queue(&c.delayed));
        state.insert("currentEvent".to_string(), event(&c.event));
        state.insert("phase".to_string(), Value::string(c.phase.name()));
        state.insert(
            "told".to_string(),
            Value::record([
                ("exited", names(&c.told.exited)),
                (
                    "fired",
                    Value::Seq(
                        c.told
                            .fired
                            .iter()
                            .map(|&row| Value::Int(i64::try_from(row).unwrap_or(i64::MAX)))
                            .collect(),
                    ),
                ),
                ("entered", names(&c.told.entered)),
            ]),
        );
        state.insert("arrived".to_string(), Value::Int(i64::from(c.arrived)));
        state.insert(
            "outcome".to_string(),
            Value::string(c.outcome.map_or(NO_OUTCOME, |s| self.order[s].as_str())),
        );
        Ok(state)
    }
}

/// The interpreter's global variables as one value, as AppendixD.tla's
/// `Machine` holds them, the datamodel as a term over the data of the state
/// the step starts from.
#[derive(Debug, Clone)]
struct Machine {
    conf: BTreeSet<usize>,
    hist: BTreeMap<usize, BTreeSet<usize>>,
    data: BTreeMap<String, Term>,
    intq: Vec<Event>,
    extq: Vec<Event>,
    delayed: Vec<Event>,
    event: Event,
    phase: Phase,
}

/// What one step of the interpreter made: its kind, the machine after it,
/// what it told, and the outcome, which only `Stop` moves.
struct Moved {
    action: Action,
    machine: Machine,
    told: Told,
    outcome: Option<Option<usize>>,
}

/// The entry of a microstep, accumulated: `statesToEnter`,
/// `statesForDefaultEntry` and `defaultHistoryContent` by the parent of the
/// history.
#[derive(Debug, Clone, Default)]
struct Entry {
    enter: BTreeSet<usize>,
    default_entry: BTreeSet<usize>,
    history_content: BTreeMap<usize, Option<String>>,
}

/// The values of the conditions one evaluation of a step is given: those it
/// is told, and true for each other it asks, each recorded in the order asked.
struct Oracle {
    told: BTreeMap<String, bool>,
    decided: Vec<(String, bool)>,
}

impl Oracle {
    fn ask(&mut self, key: &str) -> bool {
        if let Some((_, value)) = self.decided.iter().find(|(known, _)| known == key) {
            return *value;
        }
        let value = self.told.get(key).copied().unwrap_or(true);
        self.decided.push((key.to_string(), value));
        value
    }
}

/// The conditions decided on one path of a step, each with its value.
type Path = Vec<(String, bool)>;

/// `run` evaluated on every path of the conditions it asks, depth first: each
/// path with the values it was given and what `run` answered there. The paths
/// partition the values of the conditions, so for any datamodel exactly one of
/// them holds. `Err` with the count reached past [`LARGEST_PATHS`].
fn every_path<T>(mut run: impl FnMut(&mut Oracle) -> T) -> Result<Vec<(Path, T)>, usize> {
    let mut found = Vec::new();
    let mut told: Path = Vec::new();
    loop {
        let mut oracle = Oracle {
            told: told.iter().cloned().collect(),
            decided: Vec::new(),
        };
        let answer = run(&mut oracle);
        let mut next = oracle.decided;
        found.push((next.clone(), answer));
        if found.len() > LARGEST_PATHS {
            return Err(found.len());
        }
        loop {
            match next.pop() {
                None => return Ok(found),
                Some((key, true)) => {
                    next.push((key, false));
                    break;
                }
                Some((_, false)) => {}
            }
        }
        told = next;
    }
}

/// One row of the tables, its states by their place in the document order.
#[derive(Debug, Clone)]
struct IndexedRow {
    source: usize,
    kind: RowKind,
    events: Vec<Vec<String>>,
    cond: Option<String>,
    targets: Vec<usize>,
    internal: bool,
    content: Option<String>,
}

/// The tables, indexed: each state by its place in the document order, the
/// `<scxml>` element one past the last (`Root`), and AppendixD.tla's `None` one
/// past that. The operators are AppendixD.tla's of the same names, with every
/// part in.
struct Doc<'t> {
    tables: &'t ModelTables,
    ids: Vec<String>,
    kind: Vec<Kind>,
    parent: Vec<usize>,
    ancestors: Vec<BTreeSet<usize>>,
    children: Vec<Vec<usize>>,
    rows: Vec<IndexedRow>,
    transitions_of: Vec<Vec<usize>>,
    initial_of: BTreeMap<usize, usize>,
    default_of: BTreeMap<usize, usize>,
    histories: Vec<usize>,
    deep: BTreeSet<usize>,
    on_entry: Vec<Option<String>>,
    on_exit: Vec<Option<String>>,
    root: usize,
    none: usize,
}

impl<'t> Doc<'t> {
    fn new(tables: &'t ModelTables) -> Result<Self, String> {
        let ids = tables.order.clone();
        let n = ids.len();
        let (root, none) = (n, n + 1);
        let index: BTreeMap<&str, usize> = ids
            .iter()
            .enumerate()
            .map(|(at, id)| (id.as_str(), at))
            .collect();
        let find = |id: &str| {
            index
                .get(id)
                .copied()
                .ok_or_else(|| format!("the tables name `{id}`, which their order does not hold"))
        };
        let mut kind = Vec::with_capacity(n);
        let mut parent = Vec::with_capacity(n);
        for id in &ids {
            kind.push(tables.kinds.get(id).copied().unwrap_or(Kind::State));
            parent.push(match tables.parents.get(id).and_then(Option::as_deref) {
                Some(up) => find(up)?,
                None => root,
            });
        }
        let mut ancestors = vec![BTreeSet::new(); n + 1];
        for (state, held) in ancestors.iter_mut().enumerate().take(n) {
            let mut at = state;
            while at != root {
                at = parent[at];
                if !held.insert(at) {
                    return Err(format!("the parents of `{}` make a cycle", ids[state]));
                }
            }
        }
        let mut children = vec![Vec::new(); n + 1];
        for (state, (of, up)) in kind.iter().zip(&parent).enumerate() {
            if *of != Kind::History {
                children[*up].push(state);
            }
        }
        let mut rows = Vec::with_capacity(tables.rows.len());
        for row in &tables.rows {
            rows.push(IndexedRow {
                source: match &row.source {
                    Some(id) => find(id)?,
                    None => root,
                },
                kind: row.kind,
                events: row.events.clone(),
                cond: row.cond.clone(),
                targets: row
                    .targets
                    .iter()
                    .map(|id| find(id.as_str()))
                    .collect::<Result<_, _>>()?,
                internal: row.internal,
                content: row.content.clone(),
            });
        }
        let mut transitions_of = vec![Vec::new(); n + 1];
        let mut initial_of = BTreeMap::new();
        let mut default_of = BTreeMap::new();
        for (at, row) in rows.iter().enumerate() {
            let number = at + 1;
            match row.kind {
                RowKind::Transition => transitions_of[row.source].push(number),
                RowKind::Initial => {
                    initial_of.entry(row.source).or_insert(number);
                }
                RowKind::Default => {
                    default_of.entry(row.source).or_insert(number);
                }
            }
        }
        let histories = (0..n).filter(|&s| kind[s] == Kind::History).collect();
        let deep = tables
            .deep
            .iter()
            .map(|id| find(id.as_str()))
            .collect::<Result<_, _>>()?;
        let mut on_entry = vec![None; n];
        let mut on_exit = vec![None; n];
        for (handlers, blocks) in [
            (&mut on_entry, &tables.on_entry),
            (&mut on_exit, &tables.on_exit),
        ] {
            for (id, block) in blocks {
                handlers[find(id)?] = Some(block.clone());
            }
        }
        Ok(Self {
            tables,
            ids,
            kind,
            parent,
            ancestors,
            children,
            rows,
            transitions_of,
            initial_of,
            default_of,
            histories,
            deep,
            on_entry,
            on_exit,
            root,
            none,
        })
    }

    fn row(&self, number: usize) -> &IndexedRow {
        &self.rows[number - 1]
    }

    fn kind_is(&self, s: usize, kind: Kind) -> bool {
        self.kind.get(s) == Some(&kind)
    }

    fn is_history(&self, s: usize) -> bool {
        self.kind_is(s, Kind::History)
    }

    fn is_final(&self, s: usize) -> bool {
        self.kind_is(s, Kind::Final)
    }

    fn is_parallel(&self, s: usize) -> bool {
        self.kind_is(s, Kind::Parallel)
    }

    fn is_compound(&self, s: usize) -> bool {
        self.kind_is(s, Kind::State) && !self.children[s].is_empty()
    }

    fn is_atomic(&self, s: usize) -> bool {
        self.is_final(s) || (self.kind_is(s, Kind::State) && self.children[s].is_empty())
    }

    fn is_compound_or_scxml(&self, s: usize) -> bool {
        s == self.root || self.is_compound(s)
    }

    fn is_descendant(&self, s1: usize, s2: usize) -> bool {
        self.ancestors
            .get(s1)
            .is_some_and(|ancestors| ancestors.contains(&s2))
    }

    /// `Climb`.
    fn climb(&self, mut s: usize, stop: usize) -> Vec<usize> {
        let mut found = Vec::new();
        while s != self.root && self.parent[s] != stop {
            s = self.parent[s];
            found.push(s);
        }
        found
    }

    /// `ProperAncestors`.
    fn proper_ancestors(&self, s: usize, stop: usize) -> Vec<usize> {
        if stop == self.none || self.is_descendant(s, stop) {
            self.climb(s, stop)
        } else {
            Vec::new()
        }
    }

    /// The event the done event of `s` is (`DoneEvent`).
    fn done_event(&self, s: usize) -> Event {
        vec!["done".to_string(), "state".to_string(), self.ids[s].clone()]
    }

    /// `RunBlock`: the datamodel after each `<assign>`, read from the one
    /// before it, and the events the block makes on their queues.
    fn run_block(&self, block: Option<&str>, m: &mut Machine) {
        let Some(effect) = block.and_then(|name| self.tables.blocks.get(name)) else {
            return;
        };
        for (location, term) in &effect.assigns {
            let value = term.substitute(&|name: &str| m.data.get(name).cloned());
            m.data.insert(location.clone(), value);
        }
        m.intq.extend(effect.raised.iter().cloned());
        m.extq.extend(effect.sent.iter().cloned());
        m.delayed.extend(effect.later.iter().cloned());
    }

    /// `NameMatch`.
    fn name_match(descriptors: &[Vec<String>], name: &[String]) -> bool {
        descriptors.iter().any(|descriptor| {
            (descriptor.len() == 1 && descriptor[0] == "*")
                || (descriptor.len() <= name.len() && name[..descriptor.len()] == descriptor[..])
        })
    }

    /// `EnabledBy`, asking the condition last.
    fn enabled_by(&self, t: usize, event: &[String], oracle: &mut Oracle) -> bool {
        let row = self.row(t);
        let named = if event.is_empty() {
            row.events.is_empty()
        } else {
            !row.events.is_empty() && Self::name_match(&row.events, event)
        };
        named && row.cond.as_deref().is_none_or(|key| oracle.ask(key))
    }

    /// `FirstEnabled`: the first transition in document order of the first
    /// state of `ss` that has one enabled, or 0.
    fn first_enabled(&self, ss: &[usize], event: &[String], oracle: &mut Oracle) -> usize {
        for &s in ss {
            for &t in &self.transitions_of[s] {
                if self.enabled_by(t, event, oracle) {
                    return t;
                }
            }
        }
        0
    }

    /// `EffectiveTargets`.
    fn effective_targets(&self, targets: &[usize], m: &Machine) -> BTreeSet<usize> {
        let mut found = BTreeSet::new();
        for &s in targets {
            if !self.is_history(s) {
                found.insert(s);
                continue;
            }
            match m.hist.get(&s) {
                Some(recorded) if !recorded.is_empty() => found.extend(recorded.iter().copied()),
                _ => {
                    if let Some(&default) = self.default_of.get(&s) {
                        found.extend(self.effective_targets(&self.row(default).targets, m));
                    }
                }
            }
        }
        found
    }

    /// `FindLCCA`.
    fn find_lcca(&self, source: usize, targets: &BTreeSet<usize>) -> usize {
        self.proper_ancestors(source, self.none)
            .into_iter()
            .find(|&a| {
                self.is_compound_or_scxml(a) && targets.iter().all(|&s| self.is_descendant(s, a))
            })
            .unwrap_or(self.root)
    }

    /// `TransitionDomain`.
    fn transition_domain(&self, t: usize, m: &Machine) -> usize {
        let row = self.row(t);
        let tstates = self.effective_targets(&row.targets, m);
        if tstates.is_empty() {
            self.none
        } else if row.source == self.root {
            self.root
        } else if row.internal
            && self.is_compound(row.source)
            && tstates.iter().all(|&s| self.is_descendant(s, row.source))
        {
            row.source
        } else {
            self.find_lcca(row.source, &tstates)
        }
    }

    /// `ComputeExitSet`.
    fn exit_set(&self, ts: &[usize], m: &Machine) -> BTreeSet<usize> {
        let mut found = BTreeSet::new();
        for &t in ts {
            if self.row(t).targets.is_empty() {
                continue;
            }
            let domain = self.transition_domain(t, m);
            found.extend(
                m.conf
                    .iter()
                    .copied()
                    .filter(|&s| self.is_descendant(s, domain)),
            );
        }
        found
    }

    /// `Conflict`.
    fn conflict(&self, t1: usize, t2: usize, m: &Machine) -> bool {
        !self
            .exit_set(&[t1], m)
            .is_disjoint(&self.exit_set(&[t2], m))
    }

    /// `RemoveConflictingTransitions` (`Filtered`).
    fn remove_conflicting(&self, ts: &[usize], m: &Machine) -> Vec<usize> {
        let mut kept: Vec<usize> = Vec::new();
        for &t1 in ts {
            let preempted = kept.iter().any(|&t2| {
                self.conflict(t1, t2, m)
                    && !self.is_descendant(self.row(t1).source, self.row(t2).source)
            });
            if !preempted {
                kept.retain(|&t2| !self.conflict(t1, t2, m));
                kept.push(t1);
            }
        }
        kept
    }

    /// `Select`: for each atomic state in document order, the first enabled
    /// transition of it and then of its ancestors, conflicts removed.
    fn select(&self, event: &[String], m: &Machine, oracle: &mut Oracle) -> Vec<usize> {
        let mut picked: Vec<usize> = Vec::new();
        for &atomic in m.conf.iter().filter(|&&s| self.is_atomic(s)) {
            let mut chain = vec![atomic];
            chain.extend(self.proper_ancestors(atomic, self.none));
            let t = self.first_enabled(&chain, event, oracle);
            if t != 0 && !picked.contains(&t) {
                picked.push(t);
            }
        }
        self.remove_conflicting(&picked, m)
    }

    /// `Recorded`.
    fn recorded(&self, h: usize, conf: &BTreeSet<usize>) -> BTreeSet<usize> {
        let up = self.parent[h];
        if self.deep.contains(&h) {
            conf.iter()
                .copied()
                .filter(|&s| self.is_atomic(s) && self.is_descendant(s, up))
                .collect()
        } else {
            conf.iter()
                .copied()
                .filter(|&s| self.parent.get(s) == Some(&up))
                .collect()
        }
    }

    /// `ExitStates`: the history of each exited state recorded from the
    /// configuration before any is exited, then each runs its onexit and
    /// leaves. What was exited, in exit order.
    fn exit_states(&self, ts: &[usize], m: &mut Machine) -> Vec<usize> {
        let to_exit = self.exit_set(ts, m);
        let order: Vec<usize> = to_exit.iter().rev().copied().collect();
        let before = m.conf.clone();
        for &h in &self.histories {
            if to_exit.contains(&self.parent[h]) {
                m.hist.insert(h, self.recorded(h, &before));
            }
        }
        for &s in &order {
            self.run_block(self.on_exit[s].as_deref(), m);
        }
        m.conf.retain(|s| !to_exit.contains(s));
        order
    }

    /// `AddEach`.
    fn add_each(&self, ancestors: &[usize], acc: &mut Entry, m: &Machine) {
        for &ancestor in ancestors {
            acc.enter.insert(ancestor);
            if ancestor != self.root && self.is_parallel(ancestor) {
                self.fill_children(&self.children[ancestor], acc, m);
            }
        }
    }

    /// `AddAncestorsOf`.
    fn add_ancestors_of(&self, ss: &[usize], ancestor: usize, acc: &mut Entry, m: &Machine) {
        for &s in ss {
            let up = self.proper_ancestors(s, ancestor);
            self.add_each(&up, acc, m);
        }
    }

    /// `AddDescendantsOf`.
    fn add_descendants_of(&self, ss: &[usize], acc: &mut Entry, m: &Machine) {
        for &s in ss {
            self.add_descendants(s, acc, m);
        }
    }

    /// `FillChildren`.
    fn fill_children(&self, cs: &[usize], acc: &mut Entry, m: &Machine) {
        for &c in cs {
            if !acc.enter.iter().any(|&s| self.is_descendant(s, c)) {
                self.add_descendants(c, acc, m);
            }
        }
    }

    /// `AddDescendants`.
    fn add_descendants(&self, s: usize, acc: &mut Entry, m: &Machine) {
        if self.is_history(s) {
            let up = self.parent[s];
            match m.hist.get(&s).filter(|recorded| !recorded.is_empty()) {
                Some(recorded) => {
                    let values: Vec<usize> = recorded.iter().copied().collect();
                    self.add_descendants_of(&values, acc, m);
                    self.add_ancestors_of(&values, up, acc, m);
                }
                None => {
                    if let Some(&default) = self.default_of.get(&s) {
                        let row = self.row(default);
                        acc.history_content.insert(up, row.content.clone());
                        self.add_descendants_of(&row.targets, acc, m);
                        self.add_ancestors_of(&row.targets, up, acc, m);
                    }
                }
            }
            return;
        }
        acc.enter.insert(s);
        if self.is_compound(s) {
            acc.default_entry.insert(s);
            if let Some(&initial) = self.initial_of.get(&s) {
                let targets = &self.row(initial).targets;
                self.add_descendants_of(targets, acc, m);
                self.add_ancestors_of(targets, s, acc, m);
            }
        } else if self.is_parallel(s) {
            self.fill_children(&self.children[s], acc, m);
        }
    }

    /// `InFinalState`.
    fn in_final_state(&self, s: usize, conf: &BTreeSet<usize>) -> bool {
        if self.is_compound(s) {
            self.children[s]
                .iter()
                .any(|&c| self.is_final(c) && conf.contains(&c))
        } else if self.is_parallel(s) {
            self.children[s]
                .iter()
                .all(|&c| self.in_final_state(c, conf))
        } else {
            false
        }
    }

    /// `EnteredFinal`.
    fn entered_final(&self, s: usize, m: &mut Machine) {
        let up = self.parent[s];
        if up == self.root {
            m.phase = Phase::Exit;
            return;
        }
        m.intq.push(self.done_event(up));
        let above = self.parent[up];
        if above != self.root
            && self.is_parallel(above)
            && self.children[above]
                .iter()
                .all(|&c| self.in_final_state(c, &m.conf))
        {
            m.intq.push(self.done_event(above));
        }
    }

    /// `EnterStates` (`ComputeEntrySet`, then `EnterEach` in entry order).
    /// What was entered, in entry order.
    fn enter_states(&self, ts: &[usize], m: &mut Machine) -> Vec<usize> {
        let mut acc = Entry::default();
        for &t in ts {
            let row = self.row(t);
            self.add_descendants_of(&row.targets, &mut acc, m);
            let targets: Vec<usize> = self
                .effective_targets(&row.targets, m)
                .into_iter()
                .collect();
            let domain = self.transition_domain(t, m);
            self.add_ancestors_of(&targets, domain, &mut acc, m);
        }
        let order: Vec<usize> = acc.enter.iter().copied().collect();
        for &s in &order {
            m.conf.insert(s);
            self.run_block(self.on_entry.get(s).and_then(Option::as_deref), m);
            if acc.default_entry.contains(&s) {
                if let Some(&initial) = self.initial_of.get(&s) {
                    self.run_block(self.row(initial).content.as_deref(), m);
                }
            }
            if let Some(content) = acc.history_content.get(&s) {
                self.run_block(content.as_deref(), m);
            }
            if self.is_final(s) {
                self.entered_final(s, m);
            }
        }
        order
    }

    /// `Microstep`: exit, the transitions' content, enter.
    fn microstep(&self, ts: Vec<usize>, m: &mut Machine) -> Told {
        let exited = self.exit_states(&ts, m);
        for &t in &ts {
            self.run_block(self.row(t).content.as_deref(), m);
        }
        let entered = self.enter_states(&ts, m);
        Told {
            exited,
            fired: ts,
            entered,
        }
    }

    /// `TakeEvent` of the event the machine holds.
    fn take_event(&self, m: &mut Machine, oracle: &mut Oracle) -> Told {
        let ts = self.select(&m.event, m, oracle);
        if ts.is_empty() {
            Told::default()
        } else {
            self.microstep(ts, m)
        }
    }

    /// The machine of `c`, its datamodel each `<data>` itself.
    fn machine(&self, c: &Control) -> Machine {
        Machine {
            conf: c.configuration.clone(),
            hist: c.history.clone(),
            data: self
                .tables
                .data
                .iter()
                .map(|(name, _, _)| (name.clone(), Term::Var(name.clone())))
                .collect(),
            intq: c.internal.clone(),
            extq: c.external.clone(),
            delayed: c.delayed.clone(),
            event: c.event.clone(),
            phase: c.phase,
        }
    }

    /// The step of the interpreter from `c` under the conditions `oracle`
    /// gives, or none (`Step`, every part in): `Start`, then in the loop an
    /// eventless microstep, else an internal event, else, the macrostep done,
    /// an external one, and `Stop` once a top-level final state is entered.
    fn interpreter_step(&self, c: &Control, oracle: &mut Oracle) -> Option<Moved> {
        let mut m = self.machine(c);
        match c.phase {
            Phase::Start => {
                let initial = self.initial_of.get(&self.root).copied()?;
                m.phase = Phase::Loop;
                let entered = self.enter_states(&[initial], &mut m);
                Some(Moved {
                    action: Action::Start,
                    machine: m,
                    told: Told {
                        fired: vec![initial],
                        entered,
                        ..Told::default()
                    },
                    outcome: None,
                })
            }
            Phase::Loop => {
                let eventless = self.select(&[], &m, oracle);
                let (action, told) = if !eventless.is_empty() {
                    (Action::Eventless, self.microstep(eventless, &mut m))
                } else if !m.intq.is_empty() {
                    m.event = m.intq.remove(0);
                    (Action::Internal, self.take_event(&mut m, oracle))
                } else if !m.extq.is_empty() {
                    m.event = m.extq.remove(0);
                    (Action::External, self.take_event(&mut m, oracle))
                } else {
                    return None;
                };
                Some(Moved {
                    action,
                    machine: m,
                    told,
                    outcome: None,
                })
            }
            Phase::Exit => {
                let outcome = m
                    .conf
                    .iter()
                    .copied()
                    .find(|&s| self.is_final(s) && self.parent[s] == self.root);
                let order: Vec<usize> = m.conf.iter().rev().copied().collect();
                for &s in &order {
                    self.run_block(self.on_exit[s].as_deref(), &mut m);
                }
                m.conf.clear();
                m.phase = Phase::End;
                Some(Moved {
                    action: Action::Stop,
                    machine: m,
                    told: Told {
                        exited: order,
                        ..Told::default()
                    },
                    outcome: Some(outcome),
                })
            }
            Phase::End => None,
        }
    }

    /// The environment's steps from `c`: each delayed send coming due, and
    /// each event of `Outside` arriving while fewer than `MaxOutside` have.
    fn environment_steps(&self, c: &Control, environment: &Environment) -> Vec<(Action, Control)> {
        let mut steps = Vec::new();
        if c.phase != Phase::Loop {
            return steps;
        }
        for at in 0..c.delayed.len() {
            let mut next = c.clone();
            let due = next.delayed.remove(at);
            next.external.push(due);
            next.told = Told::default();
            steps.push((Action::Due, next));
        }
        if c.arrived < environment.max_outside {
            for event in &environment.outside {
                let mut next = c.clone();
                next.external.push(event.clone());
                next.arrived += 1;
                next.told = Told::default();
                steps.push((Action::Arrive, next));
            }
        }
        steps
    }

    /// The control after `moved`, a step from `c`, and the assignments of the
    /// data it moved.
    fn after(&self, c: &Control, moved: Moved) -> (Action, Control, Vec<(String, Term)>) {
        let m = moved.machine;
        let assigns = self
            .tables
            .data
            .iter()
            .filter_map(|(name, _, _)| {
                let value = m.data.get(name)?;
                (*value != Term::Var(name.clone())).then(|| (name.clone(), value.clone()))
            })
            .collect();
        let control = Control {
            configuration: m.conf,
            history: m.hist,
            internal: m.intq,
            external: m.extq,
            delayed: m.delayed,
            event: m.event,
            phase: m.phase,
            told: moved.told,
            arrived: c.arrived,
            outcome: moved.outcome.unwrap_or(c.outcome),
        };
        (moved.action, control, assigns)
    }

    /// The control a run starts in (`Init`).
    fn initial(&self) -> Control {
        Control {
            configuration: BTreeSet::new(),
            history: self
                .histories
                .iter()
                .map(|&h| (h, BTreeSet::new()))
                .collect(),
            internal: Vec::new(),
            external: Vec::new(),
            delayed: Vec::new(),
            event: Vec::new(),
            phase: Phase::Start,
            told: Told::default(),
            arrived: 0,
            outcome: None,
        }
    }
}

/// One edge of the model: from a control, under the conditions of a path, to
/// a control, with the data it assigns.
struct Edge {
    from: usize,
    action: Action,
    path: Path,
    to: usize,
    assigns: Vec<(String, Term)>,
}

/// The place of `control` among those reached, reaching it if it is new, or
/// none when that would pass [`LARGEST_CONTROL`].
fn reach(
    control: Control,
    index: &mut BTreeMap<Control, usize>,
    controls: &mut Vec<Control>,
) -> Option<usize> {
    if let Some(&at) = index.get(&control) {
        return Some(at);
    }
    if controls.len() >= LARGEST_CONTROL {
        return None;
    }
    let at = controls.len();
    index.insert(control.clone(), at);
    controls.push(control);
    Some(at)
}

/// A term as a JANI expression over the model's variables.
fn jani_term(term: &Term) -> Json {
    let binary = |op: &str, left: &Term, right: &Term| json!({"op": op, "left": jani_term(left), "right": jani_term(right)});
    match term {
        Term::Int(value) => json!(value),
        Term::Bool(value) => json!(value),
        Term::Var(name) => json!(data_variable(name)),
        Term::Not(inner) => json!({"op": NOT, "exp": jani_term(inner)}),
        Term::Negate(inner) => json!({"op": "-", "left": 0, "right": jani_term(inner)}),
        Term::Compare(op, left, right) => match op {
            Comparison::Equal => binary("=", left, right),
            Comparison::NotEqual => binary(NOT_EQUAL, left, right),
            Comparison::Less => binary("<", left, right),
            Comparison::LessOrEqual => binary(LESS_OR_EQUAL, left, right),
            Comparison::Greater => binary("<", right, left),
            Comparison::GreaterOrEqual => binary(LESS_OR_EQUAL, right, left),
        },
        Term::Arithmetic(op, left, right) => binary(
            match op {
                Arithmetic::Add => "+",
                Arithmetic::Subtract => "-",
                Arithmetic::Multiply => "*",
            },
            left,
            right,
        ),
        Term::And(left, right) => binary(AND, left, right),
        Term::Or(left, right) => binary(OR, left, right),
    }
}

/// The conjunction of `parts`, of which there is at least one.
fn conjunction(parts: Vec<Json>) -> Json {
    let mut parts = parts.into_iter();
    let first = parts.next().unwrap_or(Json::Bool(true));
    parts.fold(
        first,
        |left, right| json!({"op": AND, "left": left, "right": right}),
    )
}

/// `text` with every character past ASCII written as a JSON escape, which
/// reads back as the same string: the operators of JANI are not ASCII, and a
/// file of the repository spells them so.
fn ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

/// The JANI model of `model`, read from `document`, in `environment`: the
/// tables are the TLA+ module's, written by `super::document_model::write`
/// with `options`, and a document it refuses is refused here with the same
/// refusals.
pub fn write(
    model: &SCXMLModel,
    document: &str,
    options: document_model::Options<'_>,
    environment: &Environment,
) -> Result<JaniModel, FormalError> {
    let refused = |refusal: String| FormalError::NoTranslation {
        document: document.to_string(),
        refusals: vec![refusal],
    };
    for event in &environment.outside {
        if event.is_empty() || event.iter().any(String::is_empty) {
            return Err(refused(format!(
                "the environment: the event `{}` from outside has an empty token",
                event.join(".")
            )));
        }
    }
    let written = document_model::write(model, document, options)?;
    let tables = &written.tables;
    let mut variables = Vec::with_capacity(tables.data.len());
    for (data, ty, initial) in &tables.data {
        let Some(type_name) = jani_type(*ty) else {
            return Err(refused(format!(
                "the <data> `{data}`: {}, which no variable of the second encoding holds",
                ty.word()
            )));
        };
        variables.push(Variable {
            name: data,
            jani_type: type_name,
            initial,
        });
    }
    let doc = Doc::new(tables).map_err(|why| refused(format!("the tables: {why}")))?;
    let too_many = || {
        refused(format!(
            "the document: the control of a run takes more than {LARGEST_CONTROL} values, \
             which the second encoding does not enumerate"
        ))
    };
    let mut index = BTreeMap::new();
    let mut controls = Vec::new();
    reach(doc.initial(), &mut index, &mut controls).ok_or_else(too_many)?;
    let mut edges = Vec::new();
    let mut at = 0;
    while at < controls.len() {
        let c = controls[at].clone();
        let paths = every_path(|oracle| doc.interpreter_step(&c, oracle)).map_err(|count| {
            refused(format!(
                "the document: a step is selected under {count} sets of condition values, \
                 past the {LARGEST_PATHS} the second encoding writes"
            ))
        })?;
        for (path, moved) in paths {
            let Some(moved) = moved else {
                continue;
            };
            let (action, next, assigns) = doc.after(&c, moved);
            let to = reach(next, &mut index, &mut controls).ok_or_else(too_many)?;
            edges.push(Edge {
                from: at,
                action,
                path,
                to,
                assigns,
            });
        }
        for (action, next) in doc.environment_steps(&c, environment) {
            let to = reach(next, &mut index, &mut controls).ok_or_else(too_many)?;
            edges.push(Edge {
                from: at,
                action,
                path: Vec::new(),
                to,
                assigns: Vec::new(),
            });
        }
        at += 1;
    }
    let text = jani_text(&written.module, tables, &variables, controls.len(), &edges);
    let transitions_of = doc
        .ids
        .iter()
        .enumerate()
        .map(|(s, id)| (id.clone(), doc.transitions_of[s].clone()))
        .collect();
    let encoding = Encoding {
        order: doc.ids.clone(),
        histories: doc.histories.clone(),
        data: tables
            .data
            .iter()
            .map(|(name, ty, _)| (name.clone(), *ty))
            .collect(),
        controls,
        transitions_of,
    };
    Ok(JaniModel {
        name: written.module.clone(),
        text,
        encoding,
    })
}

/// A `<data>` as a variable of the model: its name, its JANI type and the
/// value it starts as.
struct Variable<'t> {
    name: &'t str,
    jani_type: &'static str,
    initial: &'t Term,
}

/// The JANI type of a variable that holds a `<data>` of type `ty`, or none
/// for a type no `<data>` holds: an event name is a value a property reads,
/// never one of the model's data.
fn jani_type(ty: Ty) -> Option<&'static str> {
    match ty {
        Ty::Int => Some("int"),
        Ty::Bool => Some("bool"),
        Ty::Name => None,
    }
}

/// The text of the model: one automaton of one location, the control and the
/// data its `variables`, and an edge per step.
fn jani_text(
    name: &str,
    tables: &ModelTables,
    variables: &[Variable<'_>],
    controls: usize,
    edges: &[Edge],
) -> String {
    let mut declared = vec![json!({
        "name": CONTROL,
        "type": {
            "kind": "bounded",
            "base": "int",
            "lower-bound": 0,
            "upper-bound": controls.saturating_sub(1),
        },
        "initial-value": 0,
    })];
    for variable in variables {
        declared.push(json!({
            "name": data_variable(variable.name),
            "type": variable.jani_type,
            "initial-value": jani_term(variable.initial),
        }));
    }
    let edges: Vec<Json> = edges
        .iter()
        .map(|edge| {
            let mut guard = vec![json!({"op": "=", "left": CONTROL, "right": edge.from})];
            for (key, value) in &edge.path {
                if let Some(term) = tables.guards.get(key) {
                    let condition = jani_term(term);
                    guard.push(if *value {
                        condition
                    } else {
                        json!({"op": NOT, "exp": condition})
                    });
                }
            }
            let mut assignments = vec![json!({"ref": CONTROL, "value": edge.to})];
            for (data, value) in &edge.assigns {
                assignments.push(json!({"ref": data_variable(data), "value": jani_term(value)}));
            }
            json!({
                "location": LOCATION,
                "action": edge.action.name(),
                "guard": {"exp": conjunction(guard)},
                "destinations": [{
                    "location": LOCATION,
                    "probability": {"exp": 1},
                    "assignments": assignments,
                }],
            })
        })
        .collect();
    let actions: Vec<Json> = Action::ALL
        .iter()
        .map(|action| json!({"name": action.name()}))
        .collect();
    let syncs: Vec<Json> = Action::ALL
        .iter()
        .map(|action| json!({"synchronise": [action.name()], "result": action.name()}))
        .collect();
    let model = json!({
        "jani-version": 1,
        "name": name,
        "type": "mdp",
        "actions": actions,
        "variables": declared,
        "properties": [],
        "automata": [{
            "name": AUTOMATON,
            "locations": [{"name": LOCATION}],
            "initial-locations": [LOCATION],
            "edges": edges,
        }],
        "system": {
            "elements": [{"automaton": AUTOMATON}],
            "syncs": syncs,
        },
    });
    let pretty = serde_json::to_string_pretty(&model).expect("a JSON value built here serializes");
    format!("{}\n", ascii(&pretty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn model_of(text: &str) -> SCXMLModel {
        let mut model = SCXMLParser::new().parse_string(text, "doc").expect("parse");
        crate::analyzer::analyze(&mut model, "");
        model
    }

    fn written(text: &str, environment: &Environment) -> JaniModel {
        write(
            &model_of(text),
            "doc.scxml",
            document_model::Options::default(),
            environment,
        )
        .expect("a model")
    }

    /// The edges of a written model, each as its action, the control it
    /// leaves and the control it reaches.
    fn edges(model: &JaniModel) -> Vec<(String, u64, u64)> {
        let json: Json = serde_json::from_str(&model.text).expect("the model is JSON");
        let control_of = |exp: &Json| -> u64 {
            let mut at = exp;
            while at["op"] == json!(AND) {
                at = &at["left"];
            }
            at["right"]
                .as_u64()
                .expect("the guard opens with control = k")
        };
        json["automata"][0]["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .map(|edge| {
                let to = edge["destinations"][0]["assignments"][0]["value"]
                    .as_u64()
                    .expect("the first assignment is of control");
                (
                    edge["action"].as_str().expect("an action").to_string(),
                    control_of(&edge["guard"]["exp"]),
                    to,
                )
            })
            .collect()
    }

    fn valuation(pairs: &[(&str, Json)]) -> Map<String, Json> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.clone()))
            .collect()
    }

    const ARRIVES: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="null" initial="a">
        <state id="a"><transition event="go" target="pass"/></state>
        <final id="pass"/>
        <final id="fail"/>
    </scxml>"#;

    /// Turns red if a step of the interpreter or of the environment is lost,
    /// added, or taken out of AppendixD.tla's order: start, nothing until an
    /// event arrives, the external step it enables, and the stop.
    #[test]
    fn the_steps_are_the_interpreters_and_the_environments() {
        let environment = Environment {
            outside: vec![vec!["go".to_string()]],
            max_outside: 1,
        };
        let model = written(ARRIVES, &environment);
        assert_eq!(model.encoding.control_count(), 5);
        let named = |action: &str, from, to| (action.to_string(), from, to);
        assert_eq!(
            edges(&model),
            vec![
                named("start", 0, 1),
                named("arrive", 1, 2),
                named("external", 2, 3),
                named("stop", 3, 4),
            ]
        );
        let none = Environment::default();
        assert_eq!(
            written(ARRIVES, &none).encoding.control_count(),
            2,
            "with nothing from outside the session waits after its start"
        );
    }

    /// Turns red if a state of the model is read otherwise than as the TLA+
    /// module's state it stands for, over the module's own variables.
    #[test]
    fn a_state_reads_as_the_modules_state() {
        let environment = Environment {
            outside: vec![vec!["go".to_string()]],
            max_outside: 1,
        };
        let model = written(ARRIVES, &environment);
        let state = model
            .encoding
            .state(&valuation(&[(CONTROL, json!(3))]))
            .expect("a state");
        let parsed = super::super::tla_value::parse_state(
            r#"/\ configuration = {"pass"}
               /\ historyValue = <<>>
               /\ datamodel = <<>>
               /\ internalQueue = <<>>
               /\ externalQueue = <<>>
               /\ delayed = <<>>
               /\ currentEvent = <<"go">>
               /\ phase = "exit"
               /\ told = [exited |-> <<"a">>, fired |-> <<1>>, entered |-> <<"pass">>]
               /\ arrived = 1
               /\ outcome = "<none>""#,
        )
        .expect("a state");
        assert_eq!(state, parsed);
        assert_eq!(model.encoding.trace_row("a", 0), 1);
        assert_eq!(model.encoding.trace_row("a", 1), 0);
        assert_eq!(model.encoding.trace_row("nowhere", 0), 0);
        assert!(model
            .encoding
            .state(&valuation(&[(CONTROL, json!(99))]))
            .is_err());
    }

    const GUARDED: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="ecmascript" initial="a">
        <datamodel><data id="x" expr="0"/></datamodel>
        <state id="a">
            <onentry><assign location="x" expr="x + 1"/></onentry>
            <transition cond="x &gt; 1" target="fail"/>
            <transition target="pass"/>
        </state>
        <final id="pass"/>
        <final id="fail"/>
    </scxml>"#;

    /// Turns red if a condition is decided by the generator rather than left
    /// to the model, or an assignment is not written as the edge's: the
    /// eventless step is two edges, one under the condition and one under its
    /// negation, and the start assigns what the onentry block computes.
    #[test]
    fn conditions_are_guards_and_blocks_are_assignments() {
        let model = written(GUARDED, &Environment::default());
        assert_eq!(model.encoding.control_count(), 6);
        let found = edges(&model);
        let from_one: Vec<&(String, u64, u64)> =
            found.iter().filter(|(_, from, _)| *from == 1).collect();
        assert_eq!(from_one.len(), 2, "{found:?}");
        assert!(from_one.iter().all(|(action, _, _)| action == "eventless"));
        let json: Json = serde_json::from_str(&model.text).expect("JSON");
        let all = json["automata"][0]["edges"].as_array().expect("edges");
        let greater = json!({"op": "<", "left": 1, "right": "data_x"});
        let guards: Vec<&Json> = all
            .iter()
            .filter(|edge| edge["action"] == json!("eventless"))
            .map(|edge| &edge["guard"]["exp"]["right"])
            .collect();
        assert_eq!(guards[0], &greater, "x > 1 as 1 < x: {guards:?}");
        assert_eq!(guards[1], &json!({"op": NOT, "exp": greater}));
        let start = &all[0];
        assert_eq!(start["action"], json!("start"));
        assert_eq!(
            start["destinations"][0]["assignments"][1],
            json!({"ref": "data_x", "value": {"op": "+", "left": "data_x", "right": 1}})
        );
        let x = &json["variables"][1];
        assert_eq!(x["name"], json!("data_x"));
        assert_eq!(x["type"], json!("int"));
        assert_eq!(x["initial-value"], json!(0));
    }

    const DELAYED: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="null" initial="a">
        <state id="a">
            <onentry><send event="t" delay="1s"/></onentry>
            <transition event="t" target="pass"/>
        </state>
        <final id="pass"/>
    </scxml>"#;

    /// Turns red if a delayed send stops being the environment's choice of
    /// when it comes due.
    #[test]
    fn a_delayed_send_comes_due_by_the_environment() {
        let model = written(DELAYED, &Environment::default());
        let actions: Vec<String> = edges(&model).into_iter().map(|(a, _, _)| a).collect();
        assert_eq!(actions, ["start", "due", "external", "stop"]);
        assert!(Action::Due.of_environment());
        assert!(!Action::External.of_environment());
        assert_eq!(Action::named("due"), Some(Action::Due));
    }

    /// Turns red if a document the TLA+ generator refuses is given a second
    /// encoding, or an event from outside with an empty token is let in.
    #[test]
    fn what_the_module_refuses_has_no_model() {
        let logged = ARRIVES.replace(
            r#"<state id="a">"#,
            r#"<state id="a"><onentry><log label="seen"/></onentry>"#,
        );
        let refused = write(
            &model_of(&logged),
            "doc.scxml",
            document_model::Options::default(),
            &Environment::default(),
        );
        assert!(
            matches!(refused, Err(FormalError::NoTranslation { .. })),
            "{refused:?}"
        );
        let environment = Environment {
            outside: vec![vec!["go".to_string(), String::new()]],
            max_outside: 1,
        };
        let refused = write(
            &model_of(ARRIVES),
            "doc.scxml",
            document_model::Options::default(),
            &environment,
        );
        assert!(
            matches!(&refused, Err(FormalError::NoTranslation { refusals, .. })
                if refusals[0].contains("empty token")),
            "{refused:?}"
        );
    }

    /// Turns red if the model is written with a character past ASCII, or with
    /// an escape that does not read back as the operator.
    #[test]
    fn the_operators_are_written_as_ascii_escapes() {
        let model = written(GUARDED, &Environment::default());
        assert!(model.text.is_ascii());
        assert!(model.text.contains("\\u2227"), "conjunction escaped");
        assert_eq!(ascii("a\u{2264}b"), "a\\u2264b");
    }

    /// Turns red if a variable of the model is given a type JANI does not
    /// declare, or a `<data>` an event name, which no variable holds.
    #[test]
    fn a_variable_holds_an_integer_or_a_boolean() {
        assert_eq!(jani_type(Ty::Int), Some("int"));
        assert_eq!(jani_type(Ty::Bool), Some("bool"));
        assert_eq!(jani_type(Ty::Name), None);
    }
}
