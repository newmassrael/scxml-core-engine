# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML Appendix D — what a microstep selects, exits and enters, asked of
``sce_runtime.microstep`` over a document written out by hand.

Each case states the answer the appendix computes, worked from the pseudo-code,
and asks the transcription for it. The cases, the document and the answers are
those of ``backends/rust/runtime/tests/microstep_algorithms.rs``, and through it
of the C++ engines' ``tests/states/EntrySetAlgorithmsTest.cpp`` and
``tests/states/CompletionAlgorithmsTest.cpp``, so the three transcriptions
answer one set of questions. Three answers are the reason the entry procedures
exist and are pinned by name:

* a target SET enters every target, and a ``<parallel>`` region no target
  descends into still gets its default;
* a compound state entered only as an ANCESTOR of a deeper target is not in
  statesForDefaultEntry, so its initial transition content does not run;
* a history that recorded several regions restores all of them.

::

    <scxml initial="s">
      <state id="s" initial="p">
        <parallel id="p">
          <state id="r1" initial="a1"> a1 a2 </state>
          <state id="r2" initial="b1"> b1 b2
            <history id="hb" type="shallow"> -> b2 </history> </state>
          <state id="r3"> c1 c2 </state>                    (no initial: c1)
        </parallel>
        <state id="q" initial="q1"> q1 <final id="qf"/> </state>
        <history id="hs" type="deep"> -> "a2 b2" </history>
        <state id="m" initial="m1a m2b">
          <parallel id="mp">
            <state id="m1"> m1a m1b </state>
            <state id="m2"> m2a m2b </state>
          </parallel>
        </state>
      </state>
      <state id="t"/>
    </scxml>

This directory is deliberately NOT a fixture stem: it holds the runtime's
transcription to hand-worked answers, which no document drives.
``scripts/gates/w3c-python.sh`` names it explicitly for that reason.
"""
from __future__ import annotations

import sys
from dataclasses import dataclass, field
from enum import IntEnum
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "runtime"))

from sce_runtime.microstep import (  # noqa: E402
    EnabledTransition,
    EntryTransition,
    HistoryTarget,
    StateTarget,
    compute_entry_set,
    compute_exit_set,
    compute_states_to_exit,
    effective_target_states,
    is_descendant,
    is_in_final_state,
    microstep,
    select_transitions,
    transition_domain,
)


class St(IntEnum):
    """Declared in document order, so a value IS the document position."""

    S = 0
    P = 1
    R1 = 2
    A1 = 3
    A2 = 4
    R2 = 5
    B1 = 6
    B2 = 7
    R3 = 8
    C1 = 9
    C2 = 10
    Q = 11
    Q1 = 12
    QF = 13
    M = 14
    MP = 15
    M1 = 16
    M1A = 17
    M1B = 18
    M2 = 19
    M2A = 20
    M2B = 21
    T = 22


HB = "hb"
HS = "hs"
NULL = "null"  # the eventless selection's "no event"

_PARENT: Dict[St, St] = {
    St.P: St.S, St.Q: St.S, St.M: St.S,
    St.R1: St.P, St.R2: St.P, St.R3: St.P,
    St.A1: St.R1, St.A2: St.R1,
    St.B1: St.R2, St.B2: St.R2,
    St.C1: St.R3, St.C2: St.R3,
    St.Q1: St.Q, St.QF: St.Q,
    St.MP: St.M,
    St.M1: St.MP, St.M2: St.MP,
    St.M1A: St.M1, St.M1B: St.M1,
    St.M2A: St.M2, St.M2B: St.M2,
}
_COMPOUND = {St.S, St.R1, St.R2, St.R3, St.Q, St.M, St.M1, St.M2}
_PARALLEL = {St.P, St.MP}
_CHILDREN: Dict[St, Tuple[St, ...]] = {
    St.S: (St.P, St.Q, St.M),
    St.P: (St.R1, St.R2, St.R3),
    St.R1: (St.A1, St.A2),
    St.R2: (St.B1, St.B2),
    St.R3: (St.C1, St.C2),
    St.Q: (St.Q1, St.QF),
    St.M: (St.MP,),
    St.MP: (St.M1, St.M2),
    St.M1: (St.M1A, St.M1B),
    St.M2: (St.M2A, St.M2B),
}
_INITIAL = {
    St.S: (StateTarget(St.P),),
    St.R1: (StateTarget(St.A1),),
    St.R2: (StateTarget(St.B1),),
    St.R3: (StateTarget(St.C1),),
    St.Q: (StateTarget(St.Q1),),
    St.M: (StateTarget(St.M1A), StateTarget(St.M2B)),
    St.M1: (StateTarget(St.M1A),),
    St.M2: (StateTarget(St.M2A),),
}
_HISTORY_PARENT = {HB: St.R2, HS: St.S}
_HISTORY_DEFAULT = {
    HB: (StateTarget(St.B2),),
    HS: (StateTarget(St.A2), StateTarget(St.B2)),
}


@dataclass(frozen=True)
class Tr:
    event: str
    targets: Tuple = ()
    internal: bool = False


# The transitions, per source, in document order:
#
#   a1: E -> a2, F -> a2, H (targetless), K -> a2, R -> a1 (a self-transition)
#   b1: E -> b2, G -> b2, K -> b2, R -> b2
#   p:  F -> t,  G -> t,  H -> t,  K (targetless)
_TRANSITIONS: Dict[St, Tuple[Tr, ...]] = {
    St.A1: (
        Tr("E", (StateTarget(St.A2),)),
        Tr("F", (StateTarget(St.A2),)),
        Tr("H"),
        Tr("K", (StateTarget(St.A2),)),
        Tr("R", (StateTarget(St.A1),)),
    ),
    St.B1: (
        Tr("E", (StateTarget(St.B2),)),
        Tr("G", (StateTarget(St.B2),)),
        Tr("K", (StateTarget(St.B2),)),
        Tr("R", (StateTarget(St.B2),)),
    ),
    St.P: (
        Tr("F", (StateTarget(St.T),)),
        Tr("G", (StateTarget(St.T),)),
        Tr("H", (StateTarget(St.T),)),
        Tr("K"),
    ),
}

INITIAL_CONFIGURATION = [St.S, St.P, St.R1, St.A1, St.R2, St.B1, St.R3, St.C1]


@dataclass
class Chart:
    """The document above, a configuration, and a record of what the microstep
    asked the machine to do."""

    recorded: Dict[str, List[St]] = field(default_factory=dict)
    states: List[St] = field(default_factory=list)
    log: List[str] = field(default_factory=list)
    exits_saw: List[List[St]] = field(default_factory=list)

    @classmethod
    def at_initial_configuration(cls) -> "Chart":
        return cls(states=list(INITIAL_CONFIGURATION))

    # microstep.Document
    def parent_of(self, state: St) -> Optional[St]:
        return _PARENT.get(state)

    def is_compound(self, state: St) -> bool:
        return state in _COMPOUND

    def is_parallel(self, state: St) -> bool:
        return state in _PARALLEL

    def is_final(self, state: St) -> bool:
        return state == St.QF

    def child_states(self, state: St) -> Sequence[St]:
        return _CHILDREN.get(state, ())

    def initial_targets(self, state: St):
        return _INITIAL.get(state, ())

    def history_parent(self, history: str) -> St:
        return _HISTORY_PARENT[history]

    def history_value(self, history: str) -> Optional[Sequence[St]]:
        return self.recorded.get(history)

    def history_default_targets(self, history: str):
        return _HISTORY_DEFAULT[history]

    def document_order(self, state: St) -> int:
        return int(state)

    # microstep.Run
    def configuration(self) -> Sequence[St]:
        return list(self.states)

    def first_enabled_transition(self, state: St, event: str) -> Optional[EnabledTransition]:
        for index, t in enumerate(_TRANSITIONS.get(state, ())):
            if t.event == event:
                return EnabledTransition(state, t.targets, index, t.internal)
        return None

    def exit_state(self, state: St, configuration_before_exit: Sequence[St]) -> None:
        self.exits_saw.append(list(configuration_before_exit))
        self.states.remove(state)
        self.log.append(f"exit {state.name}")

    def execute_transition_content(self, transition: EnabledTransition) -> None:
        self.log.append(f"content {transition.source.name}#{transition.transition_index}")

    def enter_state(self, state: St, is_default_entry: bool) -> None:
        self.states.append(state)
        self.log.append(f"enter {state.name}" + (" (default)" if is_default_entry else ""))

    def execute_history_default_content(self, history: str) -> None:
        self.log.append(f"history default {history}")


def entering(source: Optional[St], targets: Tuple, is_internal: bool = False) -> EntryTransition:
    return EntryTransition(source, targets, is_internal)


def enabled(source: St, index: int) -> EnabledTransition:
    t = _TRANSITIONS[source][index]
    return EnabledTransition(source, t.targets, index, t.internal)


def sources(transitions: Sequence[EnabledTransition]) -> List[Tuple[St, int]]:
    return [(t.source, t.transition_index) for t in transitions]


# ════════════════════════════════════════════════════════════════════════
# computeEntrySet
# ════════════════════════════════════════════════════════════════════════


def test_the_initial_configuration_enters_every_region_by_default():
    entry = compute_entry_set(Chart(), [entering(None, (StateTarget(St.S),))])
    assert entry.states_to_enter == INITIAL_CONFIGURATION
    # A `<parallel>` is not compound, so it has no initial transition to run.
    assert sorted(entry.states_for_default_entry) == [St.S, St.R1, St.R2, St.R3]
    assert not entry.default_history_content


def test_a_target_set_enters_every_target_and_defaults_the_region_no_target_reaches():
    entry = compute_entry_set(
        Chart(), [entering(St.T, (StateTarget(St.A2), StateTarget(St.B2)))]
    )
    assert entry.states_to_enter == [
        St.S, St.P, St.R1, St.A2, St.R2, St.B2, St.R3, St.C1
    ]
    # `s`, `r1` and `r2` are entered as ANCESTORS of a named target, so their
    # initial transitions do not run; only `r3`, reached by nobody, defaults.
    assert entry.states_for_default_entry == [St.R3]
    assert not entry.is_default_entry(St.S)
    assert entry.is_default_entry(St.R3)


def test_an_unrecorded_shallow_history_takes_its_default_and_owes_its_content_to_its_parent():
    entry = compute_entry_set(Chart(), [entering(St.A1, (HistoryTarget(HB),))])
    # The domain is `s`: the history stands for `b2`, and the least compound
    # ancestor of `a1` and `b2` walks past the `<parallel>`.
    assert entry.states_to_enter == [St.P, St.R1, St.A1, St.R2, St.B2, St.R3, St.C1]
    assert sorted(entry.states_for_default_entry) == [St.R1, St.R3]
    assert entry.default_history_content.get(St.R2) == HB
    assert entry.default_history_content.get(St.P) is None


def test_a_recorded_shallow_history_restores_what_it_recorded_and_owes_no_content():
    entry = compute_entry_set(
        Chart(recorded={HB: [St.B1]}), [entering(St.A1, (HistoryTarget(HB),))]
    )
    assert entry.states_to_enter == [St.P, St.R1, St.A1, St.R2, St.B1, St.R3, St.C1]
    assert not entry.default_history_content


def test_a_deep_history_that_recorded_every_region_restores_all_of_them():
    entry = compute_entry_set(
        Chart(recorded={HS: [St.A2, St.B1, St.C2]}),
        [entering(St.T, (HistoryTarget(HS),))],
    )
    assert entry.states_to_enter == [
        St.S, St.P, St.R1, St.A2, St.R2, St.B1, St.R3, St.C2
    ]
    # Nothing is entered by default: every compound state on the way is an
    # ancestor of a recorded state.
    assert not entry.states_for_default_entry
    assert not entry.default_history_content


def test_an_unrecorded_deep_history_with_a_multi_state_default_enters_the_whole_set():
    entry = compute_entry_set(Chart(), [entering(St.T, (HistoryTarget(HS),))])
    assert entry.states_to_enter == [
        St.S, St.P, St.R1, St.A2, St.R2, St.B2, St.R3, St.C1
    ]
    assert entry.states_for_default_entry == [St.R3]
    assert entry.default_history_content.get(St.S) == HS


def test_an_internal_transition_to_a_descendant_leaves_its_source_out_of_the_set():
    entry = compute_entry_set(Chart(), [entering(St.S, (StateTarget(St.Q1),), True)])
    assert entry.states_to_enter == [St.Q, St.Q1]
    assert not entry.states_for_default_entry


def test_an_external_transition_to_a_descendant_reenters_its_source():
    entry = compute_entry_set(Chart(), [entering(St.S, (StateTarget(St.Q1),))])
    assert entry.states_to_enter == [St.S, St.Q, St.Q1]
    assert not entry.states_for_default_entry


def test_an_external_transition_on_a_region_root_reenters_every_sibling_region():
    entry = compute_entry_set(Chart(), [entering(St.R1, (StateTarget(St.A2),))])
    # A `<parallel>` is never a domain, so the domain is `s`, and the sibling
    # regions are exited and entered again at their defaults.
    assert entry.states_to_enter == [St.P, St.R1, St.A2, St.R2, St.B1, St.R3, St.C1]
    assert sorted(entry.states_for_default_entry) == [St.R2, St.R3]


def test_an_internal_transition_on_a_region_root_stays_inside_its_region():
    entry = compute_entry_set(Chart(), [entering(St.R1, (StateTarget(St.A2),), True)])
    assert entry.states_to_enter == [St.A2]


def test_transitions_of_one_microstep_enter_in_document_order():
    # Selected second-region first, as a set is free to be; entry order is
    # document order all the same.
    entry = compute_entry_set(
        Chart(),
        [
            entering(St.B1, (StateTarget(St.B2),)),
            entering(St.A1, (StateTarget(St.A2),)),
        ],
    )
    assert entry.states_to_enter == [St.A2, St.B2]


def test_a_deep_multi_target_initial_defaults_only_the_state_that_names_it():
    entry = compute_entry_set(Chart(), [entering(St.T, (StateTarget(St.M),))])
    assert entry.states_to_enter == [St.S, St.M, St.MP, St.M1, St.M1A, St.M2, St.M2B]
    # `m` is the target and defaults; `m1` and `m2` are ancestors of its
    # initial targets, and `s` an ancestor of the target itself.
    assert entry.states_for_default_entry == [St.M]


def test_a_targetless_transition_enters_nothing():
    entry = compute_entry_set(Chart(), [entering(St.A1, ())])
    assert not entry.states_to_enter
    assert not entry.states_for_default_entry


def test_the_domain_is_asked_of_the_states_a_history_stands_for():
    doc = Chart()
    # Unrecorded, `hs` stands for "a2 b2", whose least compound ancestor with
    # `t` is the document itself.
    unrecorded = effective_target_states(doc, (HistoryTarget(HS),))
    assert unrecorded == [St.A2, St.B2]
    assert transition_domain(doc, St.T, unrecorded, False) is None

    # Recorded inside `q`, an internal transition on `q` keeps `q` as its
    # domain.
    doc.recorded[HS] = [St.Q1]
    recorded = effective_target_states(doc, (HistoryTarget(HS),))
    assert recorded == [St.Q1]
    assert transition_domain(doc, St.Q, recorded, True) == St.Q


# ════════════════════════════════════════════════════════════════════════
# isDescendant
# ════════════════════════════════════════════════════════════════════════


def test_a_child_and_a_grandchild_are_descendants():
    doc = Chart()
    assert is_descendant(doc, St.A1, St.R1)
    assert is_descendant(doc, St.A1, St.P)
    assert is_descendant(doc, St.M2B, St.S)


def test_a_state_is_not_its_own_descendant():
    # Appendix D's isDescendant is strict.
    assert not is_descendant(Chart(), St.R1, St.R1)


def test_states_on_unrelated_branches_are_not_descendants():
    doc = Chart()
    assert not is_descendant(doc, St.A1, St.R2)
    assert not is_descendant(doc, St.Q1, St.P)
    assert not is_descendant(doc, St.T, St.S)


# ════════════════════════════════════════════════════════════════════════
# Domains and exit sets
# ════════════════════════════════════════════════════════════════════════


def test_a_self_transition_exits_only_its_source():
    chart = Chart.at_initial_configuration()
    # §scxml-D-findLCCA chooses among the PROPER ancestors, so the domain of
    # `a1 -> a1` is `r1`, and the exit set is `a1` alone. A source that were
    # its own domain would exit nothing, and the self-transition would not
    # leave and re-enter its state at all.
    assert compute_exit_set(chart, enabled(St.A1, 4), chart.states) == [St.A1]


def test_an_external_transition_leaving_a_parallel_exits_every_region():
    chart = Chart.at_initial_configuration()
    # `p -> t`: the domain is the `<scxml>` element, so the whole configuration
    # goes — the sibling regions of `p` included, which no walk up from the
    # source alone could name.
    assert sorted(compute_exit_set(chart, enabled(St.P, 0), chart.states)) == sorted(
        chart.states
    )


def test_an_internal_transition_from_a_region_root_exits_only_inside_the_region():
    chart = Chart.at_initial_configuration()
    internal = EnabledTransition(St.R1, (StateTarget(St.A2),), 0, True)
    external = EnabledTransition(St.R1, (StateTarget(St.A2),), 0, False)
    assert compute_exit_set(chart, internal, chart.states) == [St.A1]
    # Written external, the same transition's domain is `s` — a `<parallel>`
    # is never a domain — so every region goes with it.
    assert sorted(compute_exit_set(chart, external, chart.states)) == [
        St.P, St.R1, St.A1, St.R2, St.B1, St.R3, St.C1
    ]


def test_a_targetless_transition_exits_nothing():
    chart = Chart.at_initial_configuration()
    assert compute_exit_set(chart, enabled(St.A1, 2), chart.states) == []


def test_the_states_to_exit_come_out_in_reverse_document_order():
    chart = Chart.at_initial_configuration()
    assert compute_states_to_exit(chart, [enabled(St.P, 0)], chart.states) == [
        St.C1, St.R3, St.B1, St.R2, St.A1, St.R1, St.P, St.S
    ]


# ════════════════════════════════════════════════════════════════════════
# selectTransitions / removeConflictingTransitions
# ════════════════════════════════════════════════════════════════════════


def test_every_region_takes_its_own_transition():
    # §scxml-3.4: two regions' transitions have domains in disjoint subtrees,
    # so their exit sets are disjoint and both survive.
    chart = Chart.at_initial_configuration()
    assert sources(select_transitions(chart, "E")) == [(St.A1, 0), (St.B1, 0)]


def test_a_transition_selected_first_preempts_a_later_one_that_is_not_its_descendant():
    # `a1` selects `a1 -> a2`; `b1` walks up to `p -> t`, which exits `a1` too,
    # and `p` does not descend from `a1`: it is preempted.
    chart = Chart.at_initial_configuration()
    assert sources(select_transitions(chart, "F")) == [(St.A1, 1)]


def test_a_descendant_source_preempts_an_ancestor_selected_before_it():
    # `a1` walks up to `p -> t` first; `b1` then selects `b1 -> b2`, which
    # conflicts with it and descends from `p`, so it wins. `c1` reaches
    # `p -> t` again — one transition, already in the set.
    chart = Chart.at_initial_configuration()
    assert sources(select_transitions(chart, "G")) == [(St.B1, 1)]


def test_a_targetless_transition_is_never_preempted():
    # W3C test 403c: an empty exit set conflicts with nothing, so `a1`'s
    # targetless transition survives the `p -> t` that exits `a1` itself.
    chart = Chart.at_initial_configuration()
    assert sources(select_transitions(chart, "H")) == [(St.A1, 2), (St.P, 2)]


def test_a_transition_reached_from_two_regions_is_selected_once():
    # In the initial configuration only `c1` walks up to `p`'s targetless K —
    # `a1` and `b1` answer K themselves. Move the first two regions to `a2` and
    # `b2`, which answer nothing, and all three atomic states reach it: it is
    # still one element of the set.
    chart = Chart.at_initial_configuration()
    assert sources(select_transitions(chart, "K")) == [(St.A1, 3), (St.B1, 2), (St.P, 3)]
    chart.states = [St.S, St.P, St.R1, St.A2, St.R2, St.B2, St.R3, St.C1]
    assert sources(select_transitions(chart, "K")) == [(St.P, 3)]


def test_an_eventless_selection_with_nothing_enabled_selects_nothing():
    assert select_transitions(Chart.at_initial_configuration(), NULL) == []


# ════════════════════════════════════════════════════════════════════════
# The microstep
# ════════════════════════════════════════════════════════════════════════


def test_a_microstep_exits_then_runs_content_in_selection_order_then_enters():
    chart = Chart.at_initial_configuration()
    microstep(chart, select_transitions(chart, "K"))
    # §scxml-D-executeTransitionContent runs content in SELECTION order: `p`'s
    # K was reached last, from `c1`, though `p` comes first in document order.
    assert chart.log == [
        "exit B1",
        "exit A1",
        "content A1#3",
        "content B1#2",
        "content P#3",
        "enter A2",
        "enter B2",
    ]


def test_every_exit_is_handed_the_configuration_before_the_first_exit():
    chart = Chart.at_initial_configuration()
    before = list(chart.states)
    # K exits two states, `b1` then `a1`: the second exit must still be handed
    # the configuration from before the first — a microstep with one exit could
    # not tell that from the configuration as it stands at each exit.
    microstep(chart, select_transitions(chart, "K"))
    assert chart.exits_saw == [before, before]


def test_a_self_transition_leaves_and_reenters_its_state():
    chart = Chart.at_initial_configuration()
    taken = select_transitions(chart, "R")
    assert sources(taken) == [(St.A1, 4), (St.B1, 3)]
    microstep(chart, taken)
    assert chart.log == [
        "exit B1",
        "exit A1",
        "content A1#4",
        "content B1#3",
        "enter A1",
        "enter B2",
    ]


def test_a_history_default_content_runs_after_its_parent_is_entered():
    chart = Chart.at_initial_configuration()
    microstep(chart, [EnabledTransition(St.A1, (HistoryTarget(HB),), 0, False)])
    # The domain is `s`, so all of `p` leaves and comes back; `r2` is an
    # ancestor of the default target `b2`, not entered by default, and the
    # history's default content runs once `r2` has been entered. Unlike the
    # Rust transcription, this one hands every transition's content to the
    # machine, which runs nothing for a transition that has none.
    assert chart.log == [
        "exit C1",
        "exit R3",
        "exit B1",
        "exit R2",
        "exit A1",
        "exit R1",
        "exit P",
        "content A1#0",
        "enter P",
        "enter R1 (default)",
        "enter A1",
        "enter R2",
        "history default hb",
        "enter B2",
        "enter R3 (default)",
        "enter C1",
    ]


# ════════════════════════════════════════════════════════════════════════
# isInFinalState
# ════════════════════════════════════════════════════════════════════════
#
# run (parallel)
#   form > filling, formDone (final)
#   checks (parallel)
#     left  > leftPending,  leftDone (final)
#     right > rightPending, rightDone (final)
#
# `checks` is a `<parallel>` among the regions of a `<parallel>`: it has no
# `<final>` child of its own and is final only because both of its regions are.


class C(IntEnum):
    RUN = 0
    FORM = 1
    FILLING = 2
    FORM_DONE = 3
    CHECKS = 4
    LEFT = 5
    LEFT_PENDING = 6
    LEFT_DONE = 7
    RIGHT = 8
    RIGHT_PENDING = 9
    RIGHT_DONE = 10


_C_PARENT = {
    C.FORM: C.RUN, C.CHECKS: C.RUN,
    C.FILLING: C.FORM, C.FORM_DONE: C.FORM,
    C.LEFT: C.CHECKS, C.RIGHT: C.CHECKS,
    C.LEFT_PENDING: C.LEFT, C.LEFT_DONE: C.LEFT,
    C.RIGHT_PENDING: C.RIGHT, C.RIGHT_DONE: C.RIGHT,
}
_C_CHILDREN = {
    C.RUN: (C.FORM, C.CHECKS),
    C.FORM: (C.FILLING, C.FORM_DONE),
    C.CHECKS: (C.LEFT, C.RIGHT),
    C.LEFT: (C.LEFT_PENDING, C.LEFT_DONE),
    C.RIGHT: (C.RIGHT_PENDING, C.RIGHT_DONE),
}


class Completion:
    def parent_of(self, state: C) -> Optional[C]:
        return _C_PARENT.get(state)

    def is_compound(self, state: C) -> bool:
        return state in (C.FORM, C.LEFT, C.RIGHT)

    def is_parallel(self, state: C) -> bool:
        return state in (C.RUN, C.CHECKS)

    def is_final(self, state: C) -> bool:
        return state in (C.FORM_DONE, C.LEFT_DONE, C.RIGHT_DONE)

    def child_states(self, state: C) -> Sequence[C]:
        return _C_CHILDREN.get(state, ())

    def document_order(self, state: C) -> int:
        return int(state)


def test_a_compound_state_is_final_when_a_final_child_is_active():
    doc = Completion()
    assert is_in_final_state(
        doc,
        C.FORM,
        [C.RUN, C.FORM, C.FORM_DONE, C.CHECKS, C.LEFT, C.LEFT_PENDING, C.RIGHT, C.RIGHT_PENDING],
    )
    assert not is_in_final_state(
        doc,
        C.FORM,
        [C.RUN, C.FORM, C.FILLING, C.CHECKS, C.LEFT, C.LEFT_PENDING, C.RIGHT, C.RIGHT_PENDING],
    )


def test_a_nested_parallel_is_final_only_when_every_region_is():
    doc = Completion()
    assert is_in_final_state(
        doc, C.CHECKS, [C.CHECKS, C.LEFT, C.LEFT_DONE, C.RIGHT, C.RIGHT_DONE]
    )
    assert not is_in_final_state(
        doc, C.CHECKS, [C.CHECKS, C.LEFT, C.LEFT_DONE, C.RIGHT, C.RIGHT_PENDING]
    ), "one region still pending leaves the <parallel> short of final"


def test_a_parallel_region_of_a_parallel_counts():
    doc = Completion()
    assert is_in_final_state(
        doc,
        C.RUN,
        [C.RUN, C.FORM, C.FORM_DONE, C.CHECKS, C.LEFT, C.LEFT_DONE, C.RIGHT, C.RIGHT_DONE],
    ), "`checks` has no <final> child of its own; it is final because both of its regions are"
    assert not is_in_final_state(
        doc,
        C.RUN,
        [C.RUN, C.FORM, C.FORM_DONE, C.CHECKS, C.LEFT, C.LEFT_DONE, C.RIGHT, C.RIGHT_PENDING],
    )


def test_neither_an_atomic_state_nor_a_final_element_is_in_a_final_state():
    doc = Completion()
    configuration = [
        C.RUN, C.FORM, C.FORM_DONE, C.CHECKS, C.LEFT, C.LEFT_PENDING, C.RIGHT, C.RIGHT_PENDING
    ]
    assert not is_in_final_state(doc, C.LEFT_PENDING, configuration), (
        "an atomic state has no <final> child to be active"
    )
    assert not is_in_final_state(doc, C.FORM_DONE, configuration), (
        "being a <final> element is not being IN a final state — the predicate asks about children"
    )
