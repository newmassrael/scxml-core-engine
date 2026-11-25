// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How many delayed events a Rust machine's delayed-send queue can hold at
//! once, proven from its document (docs/adr/0132).
//!
//! A bare-metal machine keeps its delayed sends in a ring of fixed capacity,
//! and a send armed on a full ring stops the program rather than lose a
//! transition. So the capacity is a claim about every run: an
//! undercount stops a machine. This module makes the claim from the document
//! and says why it holds, or answers [`SchedulerBound::NotProven`] with the
//! construct that keeps it from being made, located where it is written.
//!
//! # What arms an entry, and what takes one away
//!
//! An entry is armed by the run of a `<send>` that waits ([`Action::delays`],
//! docs/adr/0132 decision 1), one entry per run, whatever its target (the hosted profile
//! keeps every delayed send in the one queue, a routed one and a host-served
//! one among them), and by the start of a host-run `<invoke>` that names a
//! deadline ([`arms_host_invoke_deadline`]), which the engine drops when the
//! start ends. A send armed under the id of one already waiting is a second
//! entry: the runtime replaces nothing (`PullScheduler::schedule_event_as`),
//! and the Recommendation does not make an id a slot. An entry leaves when it comes due
//! and is delivered, when a `<cancel>` names its id (every entry of
//! that id, `PullScheduler::cancel_event`), and when a run starts again, which
//! discards what an ended run left waiting.
//!
//! # The bound
//!
//! The entries are partitioned by the site that armed them, and the bound adds
//! up the most entries of each site waiting at once, as the paragraph after
//! the rules says. A site
//! arms at most one entry each time its block runs, times the passes of each
//! `<foreach>` around it (the bound of the list it walks). Its entries are
//! bounded when, between two runs of its block, something takes every entry of
//! the earlier run away:
//!
//! - a send in an entry-time block of a state (`<onentry>`, the state's
//!   initial transition, the default transition of its initial `<history>`):
//!   a `<cancel>` of its id in an `<onexit>` of the same state. A state is
//!   exited before it is entered again, its `<onexit>` runs whenever it is
//!   exited (by a transition out of it or out of an ancestor, and when the run
//!   ends), and exit content runs before entry content;
//! - a send in an `<onexit>` of a state: a `<cancel>` of its id in an
//!   `<onentry>` of the same state, which runs between two of its exits;
//! - a send in a transition's content: a `<cancel>` of its id in an `<onexit>`
//!   of a state the transition targets. A target is entered by the
//!   transition, and when the transition is taken again while it is active it
//!   is exited first, since a transition's domain is a proper ancestor of each
//!   of its targets;
//! - a send in an entry-time block or an `<onexit>` of a state that is entered
//!   at most once in a run: no transition enters it after the initial
//!   configuration. A transition enters only states that are in the
//!   configuration together with its targets after it, so a state is entered
//!   again only where one of its targets (a `<history>` read as its parent)
//!   can be active with it.
//!
//! A `<cancel>` counts only where it surely runs: it names the id as written
//! (`sendid`, the id a `<send id>` has), it is present in every build its
//! send is (ADR 0057), and no statement before it in its block can end the
//! block with an error (each `<onentry>` and `<onexit>` is a block
//! of its own). A send whose id is generated as it runs (`idlocation`) is
//! cancelled through a variable, which this analysis does not follow, so only
//! the at-most-once rule bounds it.
//!
//! The sites whose `<cancel>` runs as a state exits are not added up across
//! the document. An entry of a send in an entry-time block of a state, which
//! an `<onexit>` of that state cancels, waits only while the state is active:
//! it is armed after the state is entered and gone before it leaves. One of a
//! transition's content, which an `<onexit>` of a target cancels, is armed
//! after the transition's exits and before the target is entered, and gone
//! when the target leaves. So within a microstep every such entry waiting
//! belongs to a state of the configuration before it (while its exits run) or
//! after it (from its content on), and between microsteps to a state of the
//! configuration. They count by the legal configuration in which the most
//! wait: a compound state's children one at a time, a `<parallel>`'s all
//! together. Every other site counts whatever is active.
//!
//! Anything else is not proven: a delayed send with no rule that bounds it, in
//! a `<foreach>` over a list whose bound the document does not state, in a
//! `<history>`'s default transition (which runs again while its parent stays
//! active and holds no history yet), or in a `<finalize>`, and a bound past
//! what a queue counts.

use std::collections::{BTreeMap, BTreeSet};

use crate::bound_reading::{self, BlockPlaces, Tree};
use crate::forge::error::SourceLocation;
use crate::forge::value_path::Place;
use crate::host_processor_analyzer::arms_host_invoke_deadline;
use crate::model::{Action, Invoke, SCXMLModel, State, Transition};
use crate::presence_condition::CanonicalCondition;

/// The most entries a queue counts: a bound past it is not proven.
const MOST_COUNTED: usize = u16::MAX as usize;

/// The delayed-send queue's capacity for one document, or why none is proven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerBound {
    /// At most `bound` entries wait at once, the sum of what each site leaves
    /// waiting.
    Proven {
        /// The capacity.
        bound: usize,
        /// Of it, the most entries of sites a state's exit cancels that wait
        /// in one configuration.
        together: usize,
        /// The states of that configuration whose sites wait in it, outermost
        /// first.
        configuration: Vec<String>,
        /// Each site that arms an entry, with how many of its entries wait at
        /// once and why no more do, in the order the walk met them.
        sites: Vec<PendingSite>,
    },
    /// No capacity is proven.
    NotProven {
        /// The construct that keeps the bound from being proven, in words the
        /// generated policy and the refusal repeat.
        reason: String,
        /// Where it is written, when the model knows.
        at: Option<SourceLocation>,
    },
}

/// One site that arms entries on the delayed-send queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSite {
    /// The site, as the basis names it.
    pub what: String,
    /// The most of its entries waiting at once.
    pub count: usize,
    /// Why no more of them wait.
    pub release: Release,
}

/// Why the entries of one site never pile up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Release {
    /// A `<cancel>` of its id surely runs in an `<onexit>` of `state` before
    /// the site's block runs again.
    CancelledOnExit {
        /// The state.
        state: String,
    },
    /// A `<cancel>` of its id surely runs in an `<onentry>` of `state`
    /// before the site's block runs again.
    CancelledOnEntry {
        /// The state.
        state: String,
    },
    /// The site's block runs at most once in a run: `state` is entered only
    /// with the initial configuration.
    EnteredOnce {
        /// The state.
        state: String,
    },
    /// The deadline of a host-run invocation: one start at a time, and the
    /// engine drops its deadline when the start ends.
    InvocationDeadline,
}

impl SchedulerBound {
    /// The bound of `model`, read as the static lowering left it: which
    /// statement can fail, and so end the block before a `<cancel>`, is known
    /// only once it has lowered them.
    pub fn of(model: &SCXMLModel) -> Self {
        let mut analysis = Analysis::new(model);
        if let Err(Unbounded { reason, at }) = analysis.walk() {
            return Self::NotProven { reason, at };
        }
        let sites = analysis.sites;
        // The sites a state's exit cancels, weighed by their state; every
        // other site waits whatever is active.
        let mut weights: BTreeMap<&str, usize> = BTreeMap::new();
        let mut anywhere = 0usize;
        for site in &sites {
            match site.scope() {
                Some(state) => {
                    let weight = weights.entry(state).or_default();
                    *weight = weight.saturating_add(site.count);
                }
                None => anywhere = anywhere.saturating_add(site.count),
            }
        }
        let (together, configuration) = heaviest_configuration(model, &weights);
        let bound = together.saturating_add(anywhere);
        if bound > MOST_COUNTED {
            return Self::NotProven {
                reason: format!(
                    "{bound} delayed events can wait at once, more than a queue counts \
                     ({MOST_COUNTED})"
                ),
                at: None,
            };
        }
        Self::Proven {
            bound,
            together,
            configuration,
            sites,
        }
    }

    /// The proven capacity, `None` when none is.
    pub fn bound(&self) -> Option<usize> {
        match self {
            Self::Proven { bound, .. } => Some(*bound),
            Self::NotProven { .. } => None,
        }
    }

    /// Where the construct that keeps the bound from being proven is written.
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            Self::Proven { .. } => None,
            Self::NotProven { at, .. } => at.as_ref(),
        }
    }

    /// The reason for the capacity, as the generated policy states it, so a
    /// change to the document that moves it is visible in the generated diff.
    pub fn basis(&self) -> String {
        match self {
            Self::Proven { bound: 0, .. } => {
                "Proven capacity 0: nothing this document does arms a delayed event.".to_string()
            }
            Self::Proven {
                bound,
                together,
                configuration,
                sites,
            } => {
                let mut text = format!("Proven capacity {bound}");
                if *together > 0 {
                    let states: Vec<String> = configuration
                        .iter()
                        .map(|state| format!("`{state}`"))
                        .collect();
                    text.push_str(&format!(
                        ": {together} of the sites a state's exit cancels wait at once at most, \
                         with {} active",
                        states.join(", ")
                    ));
                }
                let anywhere = bound - together;
                if anywhere > 0 {
                    let joint = if *together > 0 { ", and" } else { ":" };
                    text.push_str(&format!("{joint} {anywhere} wait whatever is active"));
                }
                let sites: Vec<String> = sites.iter().map(PendingSite::describe).collect();
                text.push_str(&format!(". Sites: {}.", sites.join("; ")));
                text
            }
            Self::NotProven { reason, .. } => format!("No capacity is proven: {reason}."),
        }
    }

    /// [`basis`](Self::basis) in lines of a comment's width.
    pub fn basis_lines(&self) -> Vec<String> {
        bound_reading::wrap(&self.basis(), 72)
    }
}

impl PendingSite {
    /// The state whose activity bounds this site's entries: they wait only
    /// while it is active, since an `<onexit>` of it cancels them.
    pub fn scope(&self) -> Option<&str> {
        match &self.release {
            Release::CancelledOnExit { state } => Some(state.as_str()),
            Release::CancelledOnEntry { .. }
            | Release::EnteredOnce { .. }
            | Release::InvocationDeadline => None,
        }
    }

    /// The site, its count and its release, as the basis states them.
    fn describe(&self) -> String {
        let why = match &self.release {
            Release::CancelledOnExit { state } => {
                format!("cancelled in the <onexit> of `{state}`")
            }
            Release::CancelledOnEntry { state } => {
                format!("cancelled in the <onentry> of `{state}`")
            }
            Release::EnteredOnce { state } => format!("`{state}` is entered once in a run"),
            Release::InvocationDeadline => "dropped when the start ends".to_string(),
        };
        format!("{}: {} at once, {why}", self.what, self.count)
    }
}

/// Why a site's entries are not bounded, and where it is written.
struct Unbounded {
    reason: String,
    at: Option<SourceLocation>,
}

impl Unbounded {
    /// `reason`, located on `action`.
    fn located(action: &Action, reason: String) -> Self {
        Self {
            reason,
            at: action.source_location.clone(),
        }
    }
}

/// The block a site is written in, and the state or transition it belongs to.
#[derive(Clone, Copy)]
enum Origin<'m> {
    /// A block that runs as `state` is entered: an `<onentry>`, its initial
    /// transition's content, the default content of its initial `<history>`.
    Entry {
        state: &'m State,
        block: &'static str,
    },
    /// An `<onexit>` of `state`.
    Exit(&'m State),
    /// The content of `transition`, a transition of `state`.
    Transition {
        state: &'m State,
        transition: &'m Transition,
    },
    /// The default transition's content of the `<history>` `id`.
    HistoryDefault { id: &'m str },
}

impl Origin<'_> {
    /// The block, as the basis names it.
    fn describe(&self) -> String {
        match self {
            Self::Entry { state, block } => format!("{block} of `{}`", state.id),
            Self::Exit(state) => format!("the <onexit> of `{}`", state.id),
            Self::Transition { state, transition } if transition.event.is_empty() => {
                format!("an eventless transition of `{}`", state.id)
            }
            Self::Transition { state, transition } => {
                format!("the transition of `{}` on `{}`", state.id, transition.event)
            }
            Self::HistoryDefault { id } => {
                format!("the default transition of the <history> `{id}`")
            }
        }
    }
}

/// One walk of a document, collecting each site that arms an entry.
struct Analysis<'m> {
    model: &'m SCXMLModel,
    /// The places a `<foreach>` walks, where the walk stands.
    places: BlockPlaces,
    /// The states a transition can enter after the initial configuration.
    reentered: BTreeSet<&'m str>,
    /// Each site counted, with its count and its release.
    sites: Vec<PendingSite>,
    /// Each delayed `<send>` the walk met, so one it did not meet is not
    /// left out of the bound.
    met: Vec<&'m Action>,
}

impl<'m> Analysis<'m> {
    fn new(model: &'m SCXMLModel) -> Self {
        Self {
            model,
            places: BlockPlaces::of(model),
            reentered: reentered(model, &Tree::of(model)),
            sites: Vec::new(),
            met: Vec::new(),
        }
    }

    /// Each site of the document, with its count and its release, or the
    /// first that is not bounded.
    fn walk(&mut self) -> Result<(), Unbounded> {
        let model = self.model;
        // A `<finalize>` is kept as text and run when an event of
        // its child arrives, so a send in it is not a block this walk reads.
        let finalizing = model.invokes.iter().find(|invoke| {
            matches!(invoke, Invoke::Scxml(info) if info.finalize_content.contains("send"))
        });
        if let Some(invoke) = finalizing {
            let base = invoke.base();
            return Err(Unbounded {
                reason: format!(
                    "the <finalize> of the <invoke> `{}` may send, and it runs each time an \
                     event of the child arrives",
                    base.invoke_id
                ),
                at: base.source_location.clone(),
            });
        }
        for state in model.states.values() {
            for block in &state.on_entry_blocks {
                self.block(
                    block,
                    Origin::Entry {
                        state,
                        block: "the <onentry>",
                    },
                )?;
            }
            self.block(
                &state.initial_transition_actions,
                Origin::Entry {
                    state,
                    block: "the initial transition",
                },
            )?;
            self.block(
                &state.initial_history_default_actions,
                Origin::Entry {
                    state,
                    block: "the initial <history>'s default transition",
                },
            )?;
            for block in &state.on_exit_blocks {
                self.block(block, Origin::Exit(state))?;
            }
            for transition in &state.transitions {
                let held = self.places.enter_transition(model, transition);
                let run = self.block(
                    &transition.actions,
                    Origin::Transition { state, transition },
                );
                if held {
                    self.places.release();
                }
                run?;
            }
        }
        // The default content of a `<history>` runs whenever the history is
        // taken with none kept, which repeats while its parent stays active.
        for (id, history) in &model.history_states {
            self.block(
                &history.default_actions,
                Origin::HistoryDefault { id: id.as_str() },
            )?;
        }
        // Every delayed send the model holds was met above: one in a block
        // this walk does not read would be armed and never counted.
        for (_, send) in model.sends() {
            if send.delays() && !self.met.iter().any(|met| std::ptr::eq(*met, send)) {
                return Err(Unbounded::located(
                    send,
                    "a delayed <send> is written in a block this bound does not read".to_string(),
                ));
            }
        }
        for invoke in &model.invokes {
            if arms_host_invoke_deadline(invoke) {
                self.sites.push(PendingSite {
                    what: format!(
                        "the deadline of the host-run <invoke> `{}`",
                        invoke.base().invoke_id
                    ),
                    count: 1,
                    release: Release::InvocationDeadline,
                });
            }
        }
        Ok(())
    }

    fn block(&mut self, actions: &'m [Action], origin: Origin<'m>) -> Result<(), Unbounded> {
        self.list(actions, origin, Some(1))
    }

    /// A list of statements, each run `passes` times at most for one run of
    /// its block; `None` when a `<foreach>` around it walks a list whose bound
    /// the document does not state.
    fn list(
        &mut self,
        actions: &'m [Action],
        origin: Origin<'m>,
        passes: Option<usize>,
    ) -> Result<(), Unbounded> {
        for action in actions {
            self.statement(action, origin, passes)?;
        }
        Ok(())
    }

    fn statement(
        &mut self,
        action: &'m Action,
        origin: Origin<'m>,
        passes: Option<usize>,
    ) -> Result<(), Unbounded> {
        match action.action_type.as_str() {
            "send" if action.delays() => self.delayed_send(action, origin, passes),
            "foreach" => {
                let list = self.places.walked_list(action);
                let item = list.as_ref().and_then(Place::element);
                let held = item.is_some();
                if let Some(item) = item {
                    self.places.hold(&action.item, item);
                }
                let inner = passes
                    .zip(bound_reading::passes(list.as_ref()))
                    .map(|(outer, own)| outer.saturating_mul(own));
                let body = self.list(&action.actions, origin, inner);
                if held {
                    self.places.release();
                }
                body
            }
            // Every other block inside a statement, through the model's one
            // definition of what lies inside an action: an `<if>`'s branches
            // run at most once each time the `<if>` does.
            _ => {
                for block in action.nested_blocks() {
                    self.list(block.actions, origin, passes)?;
                }
                Ok(())
            }
        }
    }

    /// A `<send>` that waits: one entry each time it runs, and the rule that
    /// keeps its entries from piling up.
    fn delayed_send(
        &mut self,
        send: &'m Action,
        origin: Origin<'m>,
        passes: Option<usize>,
    ) -> Result<(), Unbounded> {
        self.met.push(send);
        let what = format!("{} in {}", spelled(send), origin.describe());
        let Some(count) = passes else {
            return Err(Unbounded::located(
                send,
                format!(
                    "{what} runs once for each item of a <foreach> over a list whose bound the \
                     document does not declare"
                ),
            ));
        };
        let Some(release) = self.release(send, origin) else {
            return Err(Unbounded::located(
                send,
                format!(
                    "{what} can run again before its earlier event is delivered or cancelled: \
                     no <cancel> of its id surely runs in between, and its block can run more \
                     than once in a run"
                ),
            ));
        };
        self.sites.push(PendingSite {
            what,
            count,
            release,
        });
        Ok(())
    }

    /// What takes the entries of `send`, written in `origin`, away before
    /// its block runs again; `None` when nothing surely does.
    fn release(&self, send: &Action, origin: Origin<'m>) -> Option<Release> {
        let key = send_key(send);
        let cancelled_in =
            |blocks: &[Vec<Action>]| key.is_some_and(|key| cancels(blocks, key, send));
        match origin {
            Origin::Entry { state, .. } => {
                if cancelled_in(&state.on_exit_blocks) {
                    Some(Release::CancelledOnExit {
                        state: state.id.clone(),
                    })
                } else {
                    self.entered_once(state)
                }
            }
            Origin::Exit(state) => {
                if cancelled_in(&state.on_entry_blocks) {
                    Some(Release::CancelledOnEntry {
                        state: state.id.clone(),
                    })
                } else {
                    self.entered_once(state)
                }
            }
            Origin::Transition { transition, .. } => {
                // A target that is a `<history>` enters what it holds, not
                // itself, and its parent may stay active, so only a state
                // targeted by name is surely entered, and exited before the
                // transition is taken again.
                if transition.history_target.is_some() {
                    return None;
                }
                transition
                    .targets
                    .iter()
                    .filter(|target| !self.model.history_states.contains_key(target.as_str()))
                    .filter_map(|target| self.model.states.get(target))
                    .find(|target| cancelled_in(&target.on_exit_blocks))
                    .map(|target| Release::CancelledOnExit {
                        state: target.id.clone(),
                    })
            }
            Origin::HistoryDefault { .. } => None,
        }
    }

    /// [`Release::EnteredOnce`] for `state`, when no transition can enter it
    /// after the initial configuration.
    fn entered_once(&self, state: &State) -> Option<Release> {
        (!self.reentered.contains(state.id.as_str())).then(|| Release::EnteredOnce {
            state: state.id.clone(),
        })
    }
}

/// The states a transition can enter: each state that can be active together
/// with one of its targets after it, a `<history>` target read as its parent
/// (it enters its parent and states below it). The states the
/// initial configuration enters are entered once more only through one of
/// these.
fn reentered<'m>(model: &'m SCXMLModel, tree: &Tree<'m>) -> BTreeSet<&'m str> {
    let parent_of = |target: &'m str| -> &'m str {
        model
            .history_states
            .get(target)
            .map_or(target, |history| history.parent.as_str())
    };
    let mut entered: Vec<&'m str> = Vec::new();
    for state in model.states.values() {
        for transition in &state.transitions {
            entered.extend(
                transition
                    .targets
                    .iter()
                    .map(|target| parent_of(target.as_str())),
            );
            if let Some(history) = &transition.history_target {
                entered.push(parent_of(history.as_str()));
            }
        }
    }
    model
        .states
        .keys()
        .map(String::as_str)
        .filter(|state| entered.iter().any(|target| tree.coexist(target, state)))
        .collect()
}

/// The most `weights` add up to over the states of one legal configuration
/// (one child of a compound state, every child of a `<parallel>`,
/// one of the top-level states), and the states of that configuration that
/// carry a weight, outermost first. The children are read from each state's
/// parent, the one fact a model of any origin holds.
fn heaviest_configuration(
    model: &SCXMLModel,
    weights: &BTreeMap<&str, usize>,
) -> (usize, Vec<String>) {
    let mut children: BTreeMap<Option<&str>, Vec<&str>> = BTreeMap::new();
    for (id, state) in &model.states {
        children
            .entry(state.parent.as_deref())
            .or_default()
            .push(id.as_str());
    }
    heaviest_below(model, None, &children, weights)
}

/// [`heaviest_configuration`] of what lies below `parent` (the `<scxml>`
/// element for `None`), with `parent`'s own weight.
fn heaviest_below(
    model: &SCXMLModel,
    parent: Option<&str>,
    children: &BTreeMap<Option<&str>, Vec<&str>>,
    weights: &BTreeMap<&str, usize>,
) -> (usize, Vec<String>) {
    let below = children
        .get(&parent)
        .map(|ids| {
            ids.iter()
                .map(|child| heaviest_below(model, Some(*child), children, weights))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let all_at_once = parent
        .and_then(|id| model.states.get(id))
        .is_some_and(|state| state.is_parallel);
    let (weight, mut states) = if all_at_once {
        below
            .into_iter()
            .fold((0usize, Vec::new()), |(sum, mut states), (weight, more)| {
                states.extend(more);
                (sum.saturating_add(weight), states)
            })
    } else {
        below.into_iter().fold((0usize, Vec::new()), |best, next| {
            if next.0 > best.0 {
                next
            } else {
                best
            }
        })
    };
    let own = parent.and_then(|id| weights.get(id)).copied().unwrap_or(0);
    if let Some(id) = parent.filter(|_| own > 0) {
        states.insert(0, id.to_string());
    }
    (own.saturating_add(weight), states)
}

/// The id a `<send>` is armed under when the document wrote it: its `id`, or
/// the one the parser names it by. `None` for an id generated as it runs.
fn send_key(send: &Action) -> Option<&str> {
    if !send.idlocation.is_empty() || !send.native_idlocation.is_empty() {
        None
    } else if !send.id.is_empty() {
        Some(send.id.as_str())
    } else if !send.auto_send_id.is_empty() {
        Some(send.auto_send_id.as_str())
    } else {
        None
    }
}

/// Whether one of `blocks` surely runs a `<cancel>` of `key` wherever
/// `send` is present: one names the id as written, is present in every build
/// `send` is, and no statement before it can end the block.
fn cancels(blocks: &[Vec<Action>], key: &str, send: &Action) -> bool {
    blocks.iter().any(|block| {
        for action in block {
            if action.action_type == "cancel"
                && action.native_sendid.is_empty()
                && action.sendidexpr.is_empty()
                && action.sendid == key
                && present_wherever(send.presence.as_ref(), action.presence.as_ref())
            {
                return true;
            }
            if may_end_block(action) {
                return false;
            }
        }
        false
    })
}

/// Whether a node present under `other` is present in every build a node
/// present under `first` is (ADR 0057 §7): `other` always holds, or each term
/// of `first` holds every feature of some term of `other`. Both are in
/// canonical form, any of their terms, each all of its features, and neither
/// negates, so that is enough; it is not asked whether a feature one term
/// requires is another's.
fn present_wherever(
    first: Option<&CanonicalCondition>,
    other: Option<&CanonicalCondition>,
) -> bool {
    match (first, other) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(first), Some(other)) => first.terms().iter().all(|term| {
            other
                .terms()
                .iter()
                .any(|needed| needed.iter().all(|feature| term.contains(feature)))
        }),
    }
}

/// Whether `action` may raise an error that ends the block it is in
/// (docs/adr/0132 decision 2). Only what cannot is answered `false`: a `<raise>`, a
/// `<cancel>` of an id it names or computes with nothing that fails, a host
/// call (`<sce:action>`, whose error does not end its block), and a statement
/// the static lowering wrote and marked as one that cannot fail.
fn may_end_block(action: &Action) -> bool {
    match action.action_type.as_str() {
        "raise" | "native_action" => false,
        "cancel" => {
            if action.native_sendid.is_empty() {
                !action.sendidexpr.is_empty()
            } else {
                action.native_sendid_fails
            }
        }
        "assign" | "log" | "sce_append" | "sce_render" | "sce_put" | "sce_remove" | "sce_clear" => {
            action.native_code.is_empty() || action.native_fails
        }
        _ => true,
    }
}

/// A `<send>` as the basis names it: with the id it is armed under when the
/// document wrote one.
fn spelled(send: &Action) -> String {
    if !send.id.is_empty() {
        format!("the <send id=\"{}\">", send.id)
    } else if !send.idlocation.is_empty() {
        format!("the <send idlocation=\"{}\">", send.idlocation)
    } else if !send.event.is_empty() {
        format!("the <send event=\"{}\">", send.event)
    } else {
        "a <send>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{AlgorithmValueType, SceType};
    use crate::host_processor_analyzer::HOST_INVOKE_DEADLINE_PARAM;
    use crate::model::{InvokeBase, Param, ScxmlInvokeInfo, UnsupportedInvokeInfo, Variable};
    use crate::parser::SCXMLParser;

    /// `states`, the children of a document whose first state is initial.
    fn parse(states: &str) -> SCXMLModel {
        let document = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">
  <datamodel><data id="where"/></datamodel>
{states}
</scxml>"#
        );
        SCXMLParser::new()
            .parse_string(&document, "scheduler_bound")
            .unwrap_or_else(|e| panic!("parse failed: {:?}", e.error))
    }

    fn bound(states: &str) -> SchedulerBound {
        SchedulerBound::of(&parse(states))
    }

    fn not_proven_because(bound: &SchedulerBound, words: &str) {
        match bound {
            SchedulerBound::NotProven { reason, .. } => assert!(
                reason.contains(words),
                "the reason should name `{words}`: {reason}"
            ),
            proven => panic!("expected no proven capacity ({words}), got {proven:?}"),
        }
    }

    fn releases(bound: &SchedulerBound) -> Vec<Release> {
        match bound {
            SchedulerBound::Proven { sites, .. } => {
                sites.iter().map(|site| site.release.clone()).collect()
            }
            not_proven => panic!("expected a proven capacity, got {not_proven:?}"),
        }
    }

    /// A document that delays nothing arms nothing, and says so.
    #[test]
    fn a_document_that_delays_nothing_has_capacity_zero() {
        let bound = bound(
            r#"<state id="a"><transition event="go" target="b"/></state>
               <state id="b"><transition event="back" target="a"/></state>"#,
        );
        assert_eq!(bound.bound(), Some(0));
        assert!(bound.basis().starts_with("Proven capacity 0:"));
    }

    /// The session lifecycle a bare-metal consumer writes: an overall limit on
    /// a compound state, and a deadline in each of its states, each armed on
    /// entry and cancelled by the same id on exit. One child is active at a
    /// time, so at most the limit and one deadline wait at once, however many
    /// children the session has and however often each is entered again.
    #[test]
    fn deadlines_their_states_exit_cancels_count_by_one_configuration() {
        let bound = bound(
            r#"<state id="session" initial="connecting">
                 <onentry><send id="limit" event="session.timeout" delay="30s"/></onentry>
                 <onexit><cancel sendid="limit"/></onexit>
                 <transition event="session.timeout" target="ended"/>
                 <state id="connecting">
                   <onentry><send id="t_connect" event="connect.timeout" delay="2s"/></onentry>
                   <onexit><cancel sendid="t_connect"/></onexit>
                   <transition event="connected" target="established"/>
                   <transition event="connect.timeout" target="connecting"/>
                 </state>
                 <state id="established">
                   <onentry><send id="t_idle" event="idle.timeout" delay="10s"/></onentry>
                   <onexit><cancel sendid="t_idle"/></onexit>
                   <transition event="traffic" target="established"/>
                   <transition event="idle.timeout" target="closing"/>
                 </state>
                 <state id="closing">
                   <onentry><send id="t_close" event="close.timeout" delay="1s"/></onentry>
                   <onexit><cancel sendid="t_close"/></onexit>
                   <transition event="close.timeout" target="connecting"/>
                 </state>
               </state>
               <final id="ended"/>"#,
        );
        match &bound {
            SchedulerBound::Proven {
                bound,
                together,
                configuration,
                sites,
            } => {
                assert_eq!(*bound, 2, "the limit and one state's deadline");
                assert_eq!(*together, 2);
                assert_eq!(configuration.first().map(String::as_str), Some("session"));
                assert_eq!(configuration.len(), 2);
                assert_eq!(sites.len(), 4, "every site is listed in the basis");
            }
            other => panic!("expected a proven capacity, got {other:?}"),
        }
        assert!(bound
            .basis()
            .contains("cancelled in the <onexit> of `established`"));
        assert!(bound.basis_lines().iter().all(|line| line.len() <= 72));
    }

    /// The regions of a `<parallel>` are active together, so their deadlines
    /// wait together.
    #[test]
    fn the_regions_of_a_parallel_add_their_deadlines() {
        let bound = bound(
            r#"<parallel id="p">
                 <state id="r1">
                   <onentry><send id="t1" event="one" delay="1s"/></onentry>
                   <onexit><cancel sendid="t1"/></onexit>
                   <transition event="one" target="r1"/>
                 </state>
                 <state id="r2">
                   <onentry><send id="t2" event="two" delay="1s"/></onentry>
                   <onexit><cancel sendid="t2"/></onexit>
                   <transition event="two" target="r2"/>
                 </state>
               </parallel>"#,
        );
        assert_eq!(bound.bound(), Some(2));
    }

    /// A state entered again after its timer fired, or was cancelled, arms
    /// one again: its exit cancelled the earlier one first.
    #[test]
    fn a_state_entered_again_arms_one_after_its_exit_cancelled_the_last() {
        let bound = bound(
            r#"<state id="waiting">
                 <onentry><send id="t" event="tick" delay="1s"/></onentry>
                 <onexit><cancel sendid="t"/></onexit>
                 <transition event="tick" target="waiting"/>
                 <transition event="poke" target="waiting"/>
               </state>"#,
        );
        assert_eq!(bound.bound(), Some(1));
        assert_eq!(
            releases(&bound),
            vec![Release::CancelledOnExit {
                state: "waiting".to_string()
            }]
        );
    }

    /// A send never cancelled in a state a transition enters again can pile
    /// up: each entry arms one more while the last still waits.
    #[test]
    fn a_send_never_cancelled_in_a_state_entered_again_is_not_proven() {
        let bound = bound(
            r#"<state id="a">
                 <onentry><send id="t" event="tick" delay="1s"/></onentry>
                 <transition event="go" target="b"/>
               </state>
               <state id="b"><transition event="back" target="a"/></state>"#,
        );
        not_proven_because(&bound, "can run again");
        assert!(bound.location().is_some(), "located on the <send>");
        assert!(bound.basis().starts_with("No capacity is proven:"));
    }

    /// A state no transition enters is entered once in a run, so a send in it
    /// waits at most once whether or not anything cancels it.
    #[test]
    fn a_send_in_a_state_entered_once_counts_once() {
        let bound = bound(
            r#"<state id="boot">
                 <onentry><send event="ready" delay="5ms"/></onentry>
                 <transition event="ready" target="run"/>
               </state>
               <state id="run"><transition event="again" target="run"/></state>"#,
        );
        assert_eq!(bound.bound(), Some(1));
        assert_eq!(
            releases(&bound),
            vec![Release::EnteredOnce {
                state: "boot".to_string()
            }]
        );
    }

    /// A state below one a transition enters is entered with it.
    #[test]
    fn a_state_below_a_target_is_entered_again() {
        let bound = bound(
            r#"<state id="outer" initial="inner">
                 <state id="inner">
                   <onentry><send event="tick" delay="1s"/></onentry>
                 </state>
                 <transition event="restart" target="outer"/>
               </state>"#,
        );
        not_proven_because(&bound, "can run again");
    }

    /// A `<cancel>` after a statement that can end its block with an error is
    /// not surely reached; one before it is.
    #[test]
    fn a_cancel_after_a_statement_that_may_fail_does_not_count() {
        let document = |onexit: &str| {
            format!(
                r#"<state id="a">
                     <onentry><send id="t" event="tick" delay="1s"/></onentry>
                     <onexit>{onexit}</onexit>
                     <transition event="tick" target="a"/>
                   </state>"#
            )
        };
        let after = bound(&document(
            r#"<assign location="where" expr="1"/><cancel sendid="t"/>"#,
        ));
        not_proven_because(&after, "can run again");
        let before = bound(&document(
            r#"<cancel sendid="t"/><assign location="where" expr="1"/>"#,
        ));
        assert_eq!(before.bound(), Some(1));
        // A <raise> cannot fail, so the <cancel> after it is reached.
        let raised = bound(&document(r#"<raise event="left"/><cancel sendid="t"/>"#));
        assert_eq!(raised.bound(), Some(1));
    }

    /// A `<cancel>` of another id, or of a computed one, does not take the
    /// send's entry away.
    #[test]
    fn a_cancel_of_another_id_does_not_count() {
        let document = |cancel: &str| {
            format!(
                r#"<state id="a">
                     <onentry><send id="t" event="tick" delay="1s"/></onentry>
                     <onexit>{cancel}</onexit>
                     <transition event="tick" target="a"/>
                   </state>"#
            )
        };
        not_proven_because(
            &bound(&document(r#"<cancel sendid="u"/>"#)),
            "can run again",
        );
        not_proven_because(
            &bound(&document(r#"<cancel sendidexpr="'t'"/>"#)),
            "can run again",
        );
    }

    /// A send whose id is generated as it runs is cancelled through a
    /// variable, which the analysis does not follow: it is bounded only where
    /// its state is entered once.
    #[test]
    fn a_send_with_an_idlocation_is_bounded_only_by_being_entered_once() {
        let looping = bound(
            r#"<state id="a">
                 <onentry><send idlocation="where" event="tick" delay="1s"/></onentry>
                 <onexit><cancel sendidexpr="where"/></onexit>
                 <transition event="tick" target="a"/>
               </state>"#,
        );
        not_proven_because(&looping, "idlocation");
        let once = bound(
            r#"<state id="a">
                 <onentry><send idlocation="where" event="tick" delay="1s"/></onentry>
                 <transition event="tick" target="b"/>
               </state>
               <state id="b"/>"#,
        );
        assert_eq!(once.bound(), Some(1));
    }

    /// Two runs of sends under one id are two entries: arming an id already
    /// waiting replaces nothing.
    #[test]
    fn two_sends_under_one_id_are_two_entries() {
        let bound = bound(
            r#"<state id="a">
                 <onentry>
                   <send id="t" event="tick" delay="1s"/>
                   <send id="t" event="tock" delay="2s"/>
                 </onentry>
                 <onexit><cancel sendid="t"/></onexit>
                 <transition event="tick" target="a"/>
               </state>"#,
        );
        assert_eq!(bound.bound(), Some(2));
    }

    /// A send in a transition's content waits while its target is active,
    /// when the target's exit cancels it.
    #[test]
    fn a_send_in_a_transition_is_cancelled_by_its_targets_exit() {
        let bound = bound(
            r#"<state id="a">
                 <transition event="go" target="b">
                   <send id="t" event="late" delay="1s"/>
                 </transition>
               </state>
               <state id="b">
                 <onexit><cancel sendid="t"/></onexit>
                 <transition event="late" target="a"/>
                 <transition event="back" target="a"/>
               </state>"#,
        );
        assert_eq!(bound.bound(), Some(1));
        assert_eq!(
            releases(&bound),
            vec![Release::CancelledOnExit {
                state: "b".to_string()
            }]
        );
        // With no target, nothing is entered that could cancel it.
        let targetless = self::bound(
            r#"<state id="a">
                 <onexit><cancel sendid="t"/></onexit>
                 <transition event="go">
                   <send id="t" event="late" delay="1s"/>
                 </transition>
               </state>"#,
        );
        not_proven_because(&targetless, "can run again");
    }

    /// A send in an `<onexit>` waits while its state is not active, when the
    /// state's entry cancels it.
    #[test]
    fn a_send_in_an_onexit_is_cancelled_by_its_states_entry() {
        let bound = bound(
            r#"<state id="a">
                 <onentry><cancel sendid="t"/></onentry>
                 <onexit><send id="t" event="late" delay="1s"/></onexit>
                 <transition event="go" target="b"/>
               </state>
               <state id="b"><transition event="back" target="a"/></state>"#,
        );
        assert_eq!(bound.bound(), Some(1));
        assert_eq!(
            releases(&bound),
            vec![Release::CancelledOnEntry {
                state: "a".to_string()
            }]
        );
    }

    /// A `<foreach>` over a list whose bound the document does not state runs
    /// a delayed send an unknown number of times.
    #[test]
    fn a_foreach_over_a_list_of_no_stated_bound_is_not_proven() {
        let bound = bound(
            r#"<state id="a">
                 <onentry>
                   <foreach array="where" item="each">
                     <send id="t" event="tick" delay="1s"/>
                   </foreach>
                 </onentry>
                 <onexit><cancel sendid="t"/></onexit>
               </state>"#,
        );
        not_proven_because(&bound, "<foreach>");
    }

    /// A `<foreach>` over a `list<T>[N]` runs its body at most `N` times, so
    /// a delayed send in it arms `N` entries under one id, which the state's
    /// exit cancels together.
    #[test]
    fn a_foreach_over_a_bounded_list_arms_one_entry_per_pass() {
        let send = Action {
            action_type: "send".to_string(),
            event: "tick".to_string(),
            id: "t".to_string(),
            auto_send_id: "t".to_string(),
            delay: "1s".to_string(),
            delay_ms: 1000,
            ..Action::default()
        };
        let foreach = Action {
            action_type: "foreach".to_string(),
            array: "xs".to_string(),
            item: "x".to_string(),
            actions: vec![send],
            ..Action::default()
        };
        let cancel = Action {
            action_type: "cancel".to_string(),
            sendid: "t".to_string(),
            ..Action::default()
        };
        let mut model = SCXMLModel::default();
        model.variables.push(Variable {
            id: "xs".to_string(),
            value_type: Some(AlgorithmValueType::List {
                elem: Box::new(AlgorithmValueType::Scalar(SceType::Uint8)),
                max: Some(crate::forge::value_type::Bound::of(3)),
            }),
            ..Variable::default()
        });
        model.states.insert(
            "a".to_string(),
            State {
                id: "a".to_string(),
                on_entry_blocks: vec![vec![foreach]],
                on_exit_blocks: vec![vec![cancel]],
                ..State::default()
            },
        );
        assert_eq!(SchedulerBound::of(&model).bound(), Some(3));
    }

    /// The default content of a `<history>` runs again while its parent
    /// stays active and keeps no history, so a delayed send in it is not
    /// bounded.
    #[test]
    fn a_send_in_a_historys_default_transition_is_not_proven() {
        let bound = bound(
            r#"<state id="p" initial="a">
                 <history id="h">
                   <transition target="a"><send id="t" event="late" delay="1s"/></transition>
                 </history>
                 <state id="a">
                   <onexit><cancel sendid="t"/></onexit>
                   <transition event="go" target="h"/>
                 </state>
               </state>"#,
        );
        not_proven_because(&bound, "can run again");
    }

    /// A `<cancel>` counts only in the builds its send is present in.
    #[test]
    fn a_cancel_counts_only_where_it_is_present_wherever_its_send_is() {
        let condition = |terms: &str| -> CanonicalCondition {
            serde_json::from_str(terms).expect("a condition in canonical form")
        };
        let accept = condition(r#"[["accept"]]"#);
        let accept_and_tls = condition(r#"[["accept","tls"]]"#);
        let either = condition(r#"[["accept"],["tls"]]"#);
        assert!(present_wherever(None, None));
        assert!(present_wherever(Some(&accept), None));
        assert!(!present_wherever(None, Some(&accept)));
        assert!(present_wherever(Some(&accept_and_tls), Some(&accept)));
        assert!(!present_wherever(Some(&accept), Some(&accept_and_tls)));
        assert!(present_wherever(Some(&accept), Some(&either)));
        assert!(!present_wherever(Some(&either), Some(&accept)));
    }

    /// The deadline of a host-run invocation waits in the same queue, one for
    /// each `<invoke>` that names one.
    #[test]
    fn a_host_run_deadline_counts_one() {
        let mut model = parse(r#"<state id="a"/>"#);
        model
            .invokes
            .push(Invoke::Unsupported(UnsupportedInvokeInfo {
                base: InvokeBase {
                    invoke_id: "job".to_string(),
                    params: vec![Param {
                        name: HOST_INVOKE_DEADLINE_PARAM.to_string(),
                        expr: "100".to_string(),
                        ..Param::default()
                    }],
                    ..InvokeBase::default()
                },
                host_served: true,
                ..UnsupportedInvokeInfo::default()
            }));
        let bound = SchedulerBound::of(&model);
        assert_eq!(bound.bound(), Some(1));
        assert_eq!(releases(&bound), vec![Release::InvocationDeadline]);
    }

    /// A `<finalize>` is kept as text and runs for each event of its child, so
    /// a send in it is not bounded.
    #[test]
    fn a_send_in_a_finalize_is_not_proven() {
        let mut model = parse(r#"<state id="a"/>"#);
        let mut info = ScxmlInvokeInfo {
            finalize_content: r#"<send event="late" delay="1s"/>"#.to_string(),
            ..ScxmlInvokeInfo::default()
        };
        info.common.base.invoke_id = "child".to_string();
        model.invokes.push(Invoke::Scxml(info));
        not_proven_because(&SchedulerBound::of(&model), "<finalize>");
    }
}
