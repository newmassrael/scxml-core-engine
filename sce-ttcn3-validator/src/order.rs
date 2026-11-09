// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The third check of docs/SCE_TEST_PROFILE.md §9.4: the order of each
//! `alt`'s alternatives is P3's, and each block is what taking its
//! transition from its atomic state does, computed by this validator from
//! the documents alone and never read from the export's claims.
//!
//! For each function of a component, the statements before its first label
//! are owed the start of the machine, and the statements each label opens
//! are owed the atomic state's section (§9.5 **Behaviour**): the checks of
//! the eventless transitions of the state and its ancestors, innermost
//! first, each an `if` of its block or, with no guard, the block itself,
//! which ends the section; the start of each invocation of the state and its
//! ancestors, outermost first; and the `alt`, whose alternatives are the
//! branches of the state and then of each ancestor, innermost first (P3). A
//! block is each exited state's onexit and the kill of its PTCs, the
//! transition's content, each entered state's onentry and, for a state
//! entered by default, its initial transition's content, then the `goto` of
//! the atomic state the machine is left in, or, where that is a top-level
//! `<final>`, its exit and the `return` (E12). The states exited and entered
//! are [`crate::scxml313`]'s.
//!
//! A statement is known by the entry whose location names its place. The
//! second check holds the statement at each place to the form of that
//! entry's construct, so this check, which holds the order of the
//! constructs that name a block's places to the order the documents owe,
//! holds the order of the statements. A place named by a construct the
//! documents do not hold is the first check's, and is passed over here.
//!
//! - `alt-order`: the alternatives of an `alt`, in textual order, or its
//!   claim, are not the branches P3 orders; or an alternative claims another
//!   state or transition than the one it is.
//! - `exit-set`, `entry-set`: an alternative claims to exit, or to enter,
//!   other states than its transition does from its atomic state, or in
//!   another order.
//! - `block-order`: a block holds other statements than the ones owed, or
//!   in another order, or a `goto` to another state than the one the
//!   transition leaves the machine in.
//! - `claim-missing`: an `alt` or an alternative claims nothing of what this
//!   check computes.

use std::collections::BTreeMap;

use crate::documents::{ChartTransition, Content, Document, Documents, Statechart};
use crate::place::{extended, Segment};
use crate::rules::{self, Action, Kind, Rule};
use crate::scxml313::{self, Taken};
use crate::ttcn3;
use crate::ttcn3::ast::{Alternative, Definition, Module, Statement};
use crate::violation::Violation;
use crate::witness::{Claim, Construct, Witness};

/// What the documents owe one statement of a block.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Owed {
    /// A statement `rule` writes of `construct`.
    Written { construct: Construct, rule: Rule },
    /// An element no rule writes, which no export holds.
    Unwritten { construct: Construct },
    /// The `goto` that ends a block, `construct`'s by E8, to the label of
    /// the atomic state the machine is left in.
    Goto { construct: Construct, label: String },
    /// An `if` and its two blocks.
    If {
        construct: Construct,
        rule: Rule,
        then: Vec<Owed>,
        otherwise: Vec<Owed>,
    },
    /// The `alt` of an atomic state that waits.
    Alt {
        state: Construct,
        branches: Vec<Branch>,
    },
}

/// One branch an `alt` owes: its transition, its block, and the states
/// taking it exits and enters, each in its order.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Branch {
    transition: Construct,
    block: Vec<Owed>,
    exited: Vec<Construct>,
    entered: Vec<Construct>,
}

impl Owed {
    /// The construct the owed statement is written from, and the rule that
    /// writes it, none where no rule does.
    fn identity(&self) -> (&Construct, Option<Rule>) {
        match self {
            Owed::Written { construct, rule }
            | Owed::If {
                construct, rule, ..
            } => (construct, Some(*rule)),
            Owed::Unwritten { construct } => (construct, None),
            Owed::Goto { construct, .. } => (construct, Some(Rule::E8)),
            Owed::Alt { state, .. } => (state, Some(Rule::E7)),
        }
    }

    fn describe(&self) -> String {
        match self.identity() {
            (construct, Some(rule)) => format!("the statement of {construct} by {rule}"),
            (construct, None) => format!("{construct}, which no rule writes,"),
        }
    }
}

/// The constructs `constructs` as a finding lists them.
fn listed<'c, I>(constructs: I) -> String
where
    I: IntoIterator<Item = &'c Construct>,
{
    let names: Vec<String> = constructs
        .into_iter()
        .map(|construct| construct.node.clone())
        .collect();
    format!("[{}]", names.join(", "))
}

/// What one component's statechart owes the statements of its function.
struct Owing<'d> {
    document: &'d Document,
    chart: &'d Statechart,
}

impl Owing<'_> {
    fn construct(&self, node: usize) -> Construct {
        self.document.construct(node)
    }

    /// The constructs of `states`, in their order.
    fn states(&self, states: &[usize]) -> Vec<Construct> {
        states
            .iter()
            .map(|&state| self.construct(self.chart.states[state].node))
            .collect()
    }

    /// The statements `content` writes, an `<if>`'s `<elseif>`s each an
    /// `if` in the `else` of the one before it.
    fn content(&self, content: &[Content]) -> Vec<Owed> {
        content
            .iter()
            .map(|item| {
                let construct = self.construct(item.node);
                let kind = self.document.nodes[item.node].kind;
                if kind == Kind::Action(Action::If) {
                    let mut otherwise = self.content(&item.otherwise);
                    for (elseif, block) in item.elseifs.iter().rev() {
                        otherwise = vec![Owed::If {
                            construct: self.construct(*elseif),
                            rule: Rule::E8,
                            then: self.content(block),
                            otherwise,
                        }];
                    }
                    return Owed::If {
                        construct,
                        rule: Rule::E8,
                        then: self.content(&item.then),
                        otherwise,
                    };
                }
                match rules::translations(kind).first() {
                    Some((rule, _)) => Owed::Written {
                        construct,
                        rule: *rule,
                    },
                    None => Owed::Unwritten { construct },
                }
            })
            .collect()
    }

    /// The exit of `states`, in their order: each one's onexit, then the
    /// kill of each PTC it invokes (E11).
    fn exits(&self, states: &[usize], owed: &mut Vec<Owed>) {
        for &state in states {
            let state = &self.chart.states[state];
            owed.extend(self.content(&state.onexit));
            for &invoke in &state.invokes {
                owed.push(Owed::Written {
                    construct: self.construct(invoke),
                    rule: Rule::E11,
                });
            }
        }
    }

    /// The block of `taken`, whose transition writes `content` and is
    /// `ending`'s goto.
    fn block(&self, taken: &Taken, content: &[Content], ending: Construct) -> Vec<Owed> {
        let mut owed = Vec::new();
        self.exits(&taken.exited, &mut owed);
        owed.extend(self.content(content));
        for &state in &taken.entered {
            owed.extend(self.content(&self.chart.states[state].onentry));
            if taken.default_entry.contains(&state) {
                owed.extend(self.content(&self.chart.states[state].initial_content));
            }
        }
        let leaf = &self.chart.states[taken.leaf];
        if leaf.top_level_final {
            self.exits(&scxml313::halt(self.chart, taken.leaf), &mut owed);
            owed.push(Owed::Written {
                construct: self.construct(leaf.node),
                rule: Rule::E12,
            });
        } else {
            owed.push(Owed::Goto {
                construct: ending,
                label: leaf.id.clone(),
            });
        }
        owed
    }

    /// The block of `transition`, of the state `source`, taken from the
    /// atomic state `leaf`, and the sets it takes.
    fn take(&self, leaf: usize, source: usize, transition: &ChartTransition) -> (Vec<Owed>, Taken) {
        let taken = scxml313::take(self.chart, leaf, source, transition);
        let block = self.block(&taken, &transition.content, self.construct(transition.node));
        (block, taken)
    }

    /// The statements before the first label: the machine started, its
    /// `goto` the component's own (E8).
    fn start(&self) -> Vec<Owed> {
        match scxml313::start(self.chart) {
            // The component's root, its first construct, has the goto.
            Some(taken) => self.block(&taken, &[], self.construct(0)),
            None => Vec::new(),
        }
    }

    /// The statements the label of the atomic state `leaf` opens.
    fn section(&self, leaf: usize) -> Vec<Owed> {
        let chain = self.chart.chain(leaf);
        let mut owed = Vec::new();
        for &state in &chain {
            for transition in self.chart.states[state]
                .transitions
                .iter()
                .filter(|transition| transition.eventless)
            {
                let (block, _) = self.take(leaf, state, transition);
                if !transition.guarded {
                    owed.extend(block);
                    return owed;
                }
                owed.push(Owed::If {
                    construct: self.construct(transition.node),
                    rule: Rule::E6,
                    then: block,
                    otherwise: Vec::new(),
                });
            }
        }
        for &state in chain.iter().rev() {
            for &invoke in &self.chart.states[state].invokes {
                owed.push(Owed::Written {
                    construct: self.construct(invoke),
                    rule: Rule::E11,
                });
            }
        }
        let mut branches = Vec::new();
        for &state in &chain {
            for transition in self.chart.states[state]
                .transitions
                .iter()
                .filter(|transition| transition.branch)
            {
                let (block, taken) = self.take(leaf, state, transition);
                branches.push(Branch {
                    transition: self.construct(transition.node),
                    block,
                    exited: self.states(&taken.exited),
                    entered: self.states(&taken.entered),
                });
            }
        }
        owed.push(Owed::Alt {
            state: self.construct(self.chart.states[leaf].node),
            branches,
        });
        owed
    }
}

/// Each place of the module, by its path, with the construct and the rule
/// of the first entry that names it.
type Named<'w> = BTreeMap<Vec<Segment>, (&'w Construct, Rule)>;

/// Each place of the module that claims something, with its claim.
type Claims<'w> = BTreeMap<Vec<Segment>, &'w Claim>;

/// A finding of this check, by its code.
#[derive(Debug, Clone, Copy)]
enum Code {
    AltOrder,
    ExitSet,
    EntrySet,
    BlockOrder,
    ClaimMissing,
}

/// The statements of one module held to what the documents owe them.
struct Holding<'a> {
    documents: &'a Documents,
    module: &'a str,
    named: Named<'a>,
    claims: Claims<'a>,
    violations: Vec<Violation>,
}

impl<'a> Holding<'a> {
    fn found(&mut self, code: Code, path: &[Segment], reason: String) {
        let module = self.module.to_string();
        let path = path.to_vec();
        self.violations.push(match code {
            Code::AltOrder => Violation::AltOrder {
                module,
                path,
                reason,
            },
            Code::ExitSet => Violation::ExitSet {
                module,
                path,
                reason,
            },
            Code::EntrySet => Violation::EntrySet {
                module,
                path,
                reason,
            },
            Code::BlockOrder => Violation::BlockOrder {
                module,
                path,
                reason,
            },
            Code::ClaimMissing => Violation::ClaimMissing {
                module,
                path,
                reason,
            },
        });
    }

    /// The construct and the rule whose entry names the place `path`.
    fn named(&self, path: &[Segment]) -> Option<(&'a Construct, Rule)> {
        self.named.get(path).copied()
    }

    /// The statements of a block, the `index`-th at `base` followed by
    /// `step(index)`, held to `owed`. The first statement that is not the
    /// one owed ends the block's check.
    fn block(
        &mut self,
        statements: &[&Statement],
        base: &[Segment],
        step: fn(usize) -> Segment,
        owed: &[Owed],
    ) {
        for (index, owed_statement) in owed.iter().enumerate() {
            let Some(statement) = statements.get(index) else {
                self.found(
                    Code::BlockOrder,
                    base,
                    format!(
                        "the block ends after {} statements, and {} are owed, the next {}",
                        statements.len(),
                        owed.len(),
                        owed_statement.describe()
                    ),
                );
                return;
            };
            let path = extended(base, step(index));
            if !self.statement(statement, &path, owed_statement) {
                return;
            }
        }
        if statements.len() > owed.len() {
            self.found(
                Code::BlockOrder,
                &extended(base, step(owed.len())),
                format!(
                    "the block holds {} statements, and {} are owed",
                    statements.len(),
                    owed.len()
                ),
            );
        }
    }

    /// Whether the statement at `path` is the one owed; each block inside
    /// it held to its own.
    fn statement(&mut self, statement: &Statement, path: &[Segment], owed: &Owed) -> bool {
        let (construct, rule) = owed.identity();
        match self.named(path) {
            Some((named, _)) if !self.documents.holds(named) => return true,
            Some((named, written)) if named == construct && Some(written) == rule => {}
            Some((named, written)) => {
                self.found(
                    Code::BlockOrder,
                    path,
                    format!(
                        "it is the statement of {named} by {written}, where {} is owed",
                        owed.describe()
                    ),
                );
                return false;
            }
            None => {
                self.found(
                    Code::BlockOrder,
                    path,
                    format!(
                        "no location of the witness names it, where {} is owed",
                        owed.describe()
                    ),
                );
                return false;
            }
        }
        match (owed, statement) {
            (Owed::Goto { label, .. }, Statement::Goto(written)) if written != label => {
                self.found(
                    Code::BlockOrder,
                    path,
                    format!(
                        "it is goto {written}, and the transition leaves the machine in '{label}'"
                    ),
                );
                false
            }
            (
                Owed::If {
                    then, otherwise, ..
                },
                Statement::If {
                    then: written_then,
                    otherwise: written_otherwise,
                    ..
                },
            ) => {
                let written_then: Vec<&Statement> = written_then.iter().collect();
                let written_otherwise: Vec<&Statement> = written_otherwise.iter().collect();
                self.block(&written_then, path, Segment::Then, then);
                self.block(&written_otherwise, path, Segment::Else, otherwise);
                true
            }
            (Owed::Alt { state, branches }, Statement::Alt(alternatives)) => {
                self.alt(path, state, branches, alternatives);
                true
            }
            // A statement of another form is the second check's.
            _ => true,
        }
    }

    /// The `alt` at `path` of the atomic state `state`, which owes
    /// `branches`, and its claim; each alternative's block and claim.
    fn alt(
        &mut self,
        path: &[Segment],
        state: &Construct,
        branches: &[Branch],
        alternatives: &[Alternative],
    ) {
        let owed: Vec<&Construct> = branches.iter().map(|branch| &branch.transition).collect();
        let written: Vec<Option<&'a Construct>> = (0..alternatives.len())
            .map(|index| {
                self.named(&extended(path, Segment::Alternative(index)))
                    .filter(|(_, rule)| *rule == Rule::E8)
                    .map(|(construct, _)| construct)
            })
            .collect();
        let in_order = written.len() == owed.len()
            && written
                .iter()
                .zip(&owed)
                .all(|(written, owed)| *written == Some(*owed));
        if !in_order {
            let names: Vec<String> = written
                .iter()
                .map(|construct| {
                    construct.map_or_else(|| "?".to_string(), |construct| construct.node.clone())
                })
                .collect();
            self.found(
                Code::AltOrder,
                path,
                format!(
                    "its alternatives are [{}] in textual order, and P3 orders the branches of \
                     {state} {}",
                    names.join(", "),
                    listed(owed.iter().copied())
                ),
            );
        }
        match self.claims.get(path).copied() {
            Some(Claim::Alt {
                state: claimed,
                alternatives: claimed_alternatives,
            }) => {
                if claimed != state || claimed_alternatives.iter().ne(owed.iter().copied()) {
                    self.found(
                        Code::AltOrder,
                        path,
                        format!(
                            "the alt claims {} of {}, and P3 orders the branches of {state} {}",
                            listed(claimed_alternatives),
                            claimed.node,
                            listed(owed.iter().copied())
                        ),
                    );
                }
            }
            _ => self.found(
                Code::ClaimMissing,
                path,
                "the alt claims no state and no order of its alternatives".to_string(),
            ),
        }
        for (index, alternative) in alternatives.iter().enumerate() {
            let Some(Some(transition)) = written.get(index) else {
                continue;
            };
            // An alternative of no branch of the state is the order's
            // finding above.
            let Some(branch) = branches
                .iter()
                .find(|branch| branch.transition == **transition)
            else {
                continue;
            };
            let at = extended(path, Segment::Alternative(index));
            let block: Vec<&Statement> = alternative.block.iter().collect();
            self.block(&block, &at, Segment::Statement, &branch.block);
            self.branch_claim(&at, state, branch);
        }
    }

    /// The claim of the alternative at `path`, of `branch` from `state`.
    fn branch_claim(&mut self, path: &[Segment], state: &Construct, branch: &Branch) {
        let Some(Claim::Branch {
            state: claimed,
            transition,
            exited,
            entered,
        }) = self.claims.get(path).copied()
        else {
            self.found(
                Code::ClaimMissing,
                path,
                "the alternative claims no states it exits and enters".to_string(),
            );
            return;
        };
        if claimed != state || *transition != branch.transition {
            self.found(
                Code::AltOrder,
                path,
                format!(
                    "the alternative claims {} from {}, and it is {} from {}",
                    transition.node, claimed.node, branch.transition.node, state.node
                ),
            );
        }
        if *exited != branch.exited {
            self.found(
                Code::ExitSet,
                path,
                format!(
                    "the alternative claims it exits {}, and taking {} from {} exits {}",
                    listed(exited),
                    branch.transition.node,
                    state.node,
                    listed(&branch.exited)
                ),
            );
        }
        if *entered != branch.entered {
            self.found(
                Code::EntrySet,
                path,
                format!(
                    "the alternative claims it enters {}, and taking {} from {} enters {}",
                    listed(entered),
                    branch.transition.node,
                    state.node,
                    listed(&branch.entered)
                ),
            );
        }
    }

    /// The function `name`, whose body is `body`, of the component whose
    /// statechart is `owing`'s.
    fn function(&mut self, name: &str, body: &[Statement], owing: &Owing<'_>) {
        let at = vec![Segment::Function(name.to_string())];
        if let Some(own) = ttcn3::opened(body, None) {
            self.block(&own, &at, Segment::Statement, &owing.start());
        }
        for statement in body {
            let Statement::Label(label) = statement else {
                continue;
            };
            // A label of no atomic state is the second check's.
            let Some(leaf) = owing
                .chart
                .state(label)
                .filter(|&leaf| owing.chart.states[leaf].labelled())
            else {
                continue;
            };
            if let Some(opened) = ttcn3::opened(body, Some(label.as_str())) {
                let base = extended(&at, Segment::Label(label.clone()));
                self.block(&opened, &base, Segment::Statement, &owing.section(leaf));
            }
        }
    }
}

/// The violations of the third check: of each function of `module`, the
/// behaviour module, that an entry of E5 names, against the statechart of
/// that entry's component.
pub fn check(documents: &Documents, module: &Module, witness: &Witness) -> Vec<Violation> {
    let mut named: Named<'_> = BTreeMap::new();
    let mut claims: Claims<'_> = BTreeMap::new();
    for entry in &witness.entries {
        for location in &entry.produced {
            if location.module != module.name {
                continue;
            }
            named
                .entry(location.path.clone())
                .or_insert((&entry.construct, entry.rule));
            if let Some(claim) = &location.claim {
                claims.entry(location.path.clone()).or_insert(claim);
            }
        }
    }
    let mut holding = Holding {
        documents,
        module: &module.name,
        named,
        claims,
        violations: Vec::new(),
    };
    for definition in &module.definitions {
        let Definition::Function { name, body, .. } = definition else {
            continue;
        };
        let Some((construct, Rule::E5)) = holding.named(&[Segment::Function(name.clone())]) else {
            continue;
        };
        let Some(document) = documents.document(&construct.document) else {
            continue;
        };
        let Some(chart) = document.statechart.as_ref() else {
            continue;
        };
        holding.function(name, body, &Owing { document, chart });
    }
    holding.violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{
        documents, module, position, rewrite_paths, witness_of, witness_value, BEHAVIOUR,
    };
    use crate::form;
    use serde_json::{json, Value};

    fn codes(violations: &[Violation]) -> Vec<&'static str> {
        violations.iter().map(Violation::code).collect()
    }

    /// The violations of the third check, and of the second, of the
    /// behaviour module `text` and the witness `witness`.
    fn checked(text: &str, witness: &Value) -> (Vec<Violation>, Vec<Violation>) {
        let documents = documents();
        let behaviour = module(text);
        let witness = witness_of(witness);
        (
            check(&documents, &behaviour, &witness),
            form::check(&documents, &[&behaviour], &witness),
        )
    }

    /// The alt of `a`, its alternatives in P3's order and in the other.
    const ALT_OF_A: &str = "    [] t.timeout {
      log(\"leaving a\");
      goto b;
    }
    [] u.timeout {
      log(\"leaving a\");
      log(\"bye\");
      return;
    }";
    const ALT_OF_A_SWAPPED: &str = "    [] u.timeout {
      log(\"leaving a\");
      log(\"bye\");
      return;
    }
    [] t.timeout {
      log(\"leaving a\");
      goto b;
    }";

    #[test]
    fn an_export_whose_blocks_are_what_the_statechart_takes_has_no_violation() {
        let (order, form) = checked(BEHAVIOUR, &witness_value());
        assert_eq!(order, Vec::<Violation>::new());
        assert_eq!(form, Vec::<Violation>::new());
    }

    /// The export writes the alternatives of `a` with its parent's branch
    /// first, and its witness agrees: each place has its form, and the order
    /// is not P3's.
    #[test]
    fn alternatives_out_of_the_order_of_p3_are_found() {
        let swapped = BEHAVIOUR.replace(ALT_OF_A, ALT_OF_A_SWAPPED);
        assert_ne!(swapped, BEHAVIOUR);
        let mut witness = witness_value();
        rewrite_paths(
            &mut witness,
            &json!([{"function": "f_c"}, {"label": "a"}, {"statement": 0}]),
            |segment| json!({"alternative": 1 - segment["alternative"].as_u64().expect("a position")}),
        );
        let (order, form) = checked(&swapped, &witness);
        assert_eq!(codes(&order), ["alt-order"], "{order:?}");
        assert_eq!(form, Vec::<Violation>::new());

        // The alt that claims the order the export wrote is found twice: by
        // its alternatives and by its claim.
        let at = position(&witness, "state 'a'", "E7");
        let alternatives = &mut witness["entries"][at]["produced"][0]["claim"]["alternatives"];
        let claimed = alternatives.as_array_mut().expect("alternatives");
        claimed.swap(0, 1);
        let (order, _) = checked(&swapped, &witness);
        assert_eq!(codes(&order), ["alt-order", "alt-order"], "{order:?}");
    }

    /// The export leaves the onexit of `outer` out of the block that leaves
    /// `b`, its witness agreeing: the block holds the return where the log
    /// is owed.
    #[test]
    fn a_block_missing_an_onexit_is_found() {
        let without = BEHAVIOUR.replacen(
            "  label b;\n  alt {\n    [] u.timeout {\n      log(\"bye\");\n",
            "  label b;\n  alt {\n    [] u.timeout {\n",
            1,
        );
        assert_ne!(without, BEHAVIOUR);
        let mut witness = witness_value();
        let at = position(&witness, "state 'outer' / onexit 1 / log 1", "E13");
        witness["entries"][at]["produced"]
            .as_array_mut()
            .expect("produced")
            .remove(1);
        let at = position(&witness, "final 'done'", "E12");
        witness["entries"][at]["produced"][1]["path"][4] = json!({"statement": 0});
        let (order, form) = checked(&without, &witness);
        assert_eq!(codes(&order), ["block-order"], "{order:?}");
        assert!(order[0].to_string().contains("onexit"), "{}", order[0]);
        assert_eq!(form, Vec::<Violation>::new());
    }

    /// A claim of the states a branch exits, or enters, that is not the set
    /// the statechart gives is found; the block itself is right.
    #[test]
    fn a_claim_of_the_wrong_exit_or_entry_set_is_found() {
        let mut witness = witness_value();
        let at = position(&witness, "state 'a' / transition 1", "E8");
        let claim = &mut witness["entries"][at]["produced"][0]["claim"];
        claim["exited"]
            .as_array_mut()
            .expect("exited")
            .push(json!({"document": "c.scxml", "node": "state 'outer'", "line": 6}));
        let (order, _) = checked(BEHAVIOUR, &witness);
        assert_eq!(codes(&order), ["exit-set"], "{order:?}");

        let mut witness = witness_value();
        let at = position(&witness, "state 'outer' / transition 1", "E8");
        witness["entries"][at]["produced"][1]["claim"]["entered"] = json!([]);
        let (order, _) = checked(BEHAVIOUR, &witness);
        assert_eq!(codes(&order), ["entry-set"], "{order:?}");
    }

    /// A goto to another state than the one the transition leaves the
    /// machine in is found; a goto has the form of E8 whatever its label.
    #[test]
    fn a_goto_to_another_state_is_found() {
        let wrong = BEHAVIOUR.replacen("      goto b;", "      goto a;", 1);
        assert_ne!(wrong, BEHAVIOUR);
        let (order, form) = checked(&wrong, &witness_value());
        assert_eq!(codes(&order), ["block-order"], "{order:?}");
        assert!(order[0].to_string().contains("goto a"), "{}", order[0]);
        assert_eq!(form, Vec::<Violation>::new());
    }

    /// An alt and an alternative that claim nothing are found.
    #[test]
    fn a_missing_claim_is_found() {
        let mut witness = witness_value();
        let at = position(&witness, "state 'b'", "E7");
        witness["entries"][at]["produced"][0]
            .as_object_mut()
            .expect("a location")
            .remove("claim");
        let at = position(&witness, "state 'a' / transition 1", "E8");
        witness["entries"][at]["produced"][0]
            .as_object_mut()
            .expect("a location")
            .remove("claim");
        let (order, _) = checked(BEHAVIOUR, &witness);
        assert_eq!(
            codes(&order),
            ["claim-missing", "claim-missing"],
            "{order:?}"
        );
    }
}
