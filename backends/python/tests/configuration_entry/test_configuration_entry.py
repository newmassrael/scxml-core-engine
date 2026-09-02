# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 3.11 — what ``Engine.enter_at`` accepts, and what it refuses, on
the Python engine.

The door exists so a host can bring a machine back where it was, in a new
process, without replaying the entry actions the earlier run already ran.
Refusals are the part that has to be enumerated rather than sampled: entering
"near" the requested configuration is the one outcome this door must never
produce, because nothing afterwards can detect it — the machine reports itself
running, ``current_state`` answers, and the set behind those answers is one the
document never describes. A gate holding only the accepting case would pass on
an engine that accepted everything.

The Python sibling of ``backends/rust/runtime/tests/configuration_entry.rs``,
``tests/integration/ConfigurationEntryAotTest.cpp`` and
``backends/go/tests/configuration_entry/``, asking the same questions of the
same rules, so a set one engine accepts is one the others accept.

Two machines, because the two halves of the door are different code paths:

- ``parallel_regions_take_own_transitions`` has ``<parallel>`` regions, so its
  configuration holds more than one leaf and the recorded current state is not
  recoverable from the set alone.
- ``statechart_native_action`` has no regions, so its configuration is one leaf
  and its ancestors.

This directory is deliberately NOT a fixture stem: it drives documents that
already exist in the tree rather than adding a document of its own, because the
claim is about a runtime door and not about a topology. ``scripts/gates/
w3c-python.sh`` names it explicitly for that reason — nothing else would.
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
_TESTS = _HERE.parent
sys.path.insert(0, str(_TESTS / "integration" / "parallel_regions_take_own_transitions"))
sys.path.insert(0, str(_TESTS / "integration" / "native_action"))
sys.path.insert(0, str(_HERE.parents[1] / "runtime"))

import parallel_regions_take_own_transitions_sm as _parallel  # noqa: E402
import statechart_native_action_sm as _linear  # noqa: E402
from sce_runtime import ConfigurationRejection, LuaScriptEngine  # noqa: E402

_State = _parallel.ParallelRegionsTakeOwnTransitionsState
_Event = _parallel.ParallelRegionsTakeOwnTransitionsEvent
_LinearState = _linear.StatechartNativeActionState


def _at_work() -> list:
    """A mid-run configuration of the parallel document: both regions live, the
    deeper one in ``working`` and the shallower in ``within``.

    Written out rather than taken from a live run because every refusal below is
    a MUTATION of it — one change each, so a refusal names one rule.
    """
    return [
        _State.RUN,
        _State.DRIVE,
        _State.RUNNING,
        _State.WORKING,
        _State.BUDGET,
        _State.WITHIN,
    ]


class _CountingScriptEngine(LuaScriptEngine):
    """A Lua engine that records how often the engine declared a datamodel
    against it.

    This is the Python channel's own answer to "validation runs before any
    mutation", and it is a sharper one than the sibling channels can give. The
    C++ and Go engines show it by NOT NEEDING a script engine for a refusal;
    this engine always owns one, so the absence of a call is what has to be
    measured instead — and a call counted at zero is a stronger claim than an
    engine left unbuilt, because it holds even when the caller supplied one.
    """

    def __init__(self) -> None:
        super().__init__()
        self.declarations = 0

    def setup_system_variables(self, session_id, machine_name, processors):  # type: ignore[override]
        self.declarations += 1
        return super().setup_system_variables(session_id, machine_name, processors)


def _new_parallel():
    engine_script = _CountingScriptEngine()
    engine_script.initialize()
    return _parallel.create_engine(script_engine=engine_script), engine_script


class _SilentActions:
    """The host for the linear machine. Its every effect is a ``<sce:action>``,
    so it cannot be constructed without one — which is the point of that seam
    and merely plumbing here, except for the two counters, which are what says
    no entry or exit content ran during a resume.
    """

    def __init__(self) -> None:
        self.idle_entries = 0
        self.assembling_exits = 0

    def append_fragment_payload(self, payload: bytes, offset: int) -> None:
        pass

    def reset_slot(self) -> None:
        pass

    def on_idle_entry(self) -> None:
        self.idle_entries += 1

    def on_assembling_exit(self) -> None:
        self.assembling_exits += 1


# The set written above is a configuration of the document, so it is accepted
# and the machine comes back holding exactly it. This is the baseline every
# refusal below is one mutation away from — without it, a validator that refused
# everything would pass every other case in this file.
def test_a_parallel_configuration_is_accepted() -> None:
    engine, _ = _new_parallel()
    configuration = _at_work()

    assert (
        engine.enter_at(configuration, _State.WORKING) is ConfigurationRejection.NONE
    ), "a configuration of the document was refused"
    assert engine.active_configuration() == set(configuration), (
        f"the machine came back holding {engine.active_configuration()}, not the "
        f"configuration it was handed ({set(configuration)})"
    )
    assert engine.is_running, "an accepted entry left the machine stopped"
    assert set(engine.active_leaves) == {_State.WORKING, _State.WITHIN}, (
        f"the restored leaves are {engine.active_leaves}; the leaves of a set are its "
        "members that hold no child in it, and this document's are `working` and `within`"
    )


# This engine derives `current_state` from its leaves rather than storing one,
# so the recorded leaf is validated rather than kept. What the door still owes
# the host is that the recorded leaf is among the leaves it came back with —
# a resume that dropped the region a host was watching would otherwise pass.
def test_the_recorded_leaf_is_one_of_the_restored_leaves() -> None:
    engine, _ = _new_parallel()

    assert (
        engine.enter_at(_at_work(), _State.WORKING) is ConfigurationRejection.NONE
    )
    assert _State.WORKING in engine.active_leaves


# A machine with no `<parallel>` has one leaf, so its configuration is that leaf
# and its ancestors. The round trip has to close there too, through a set with a
# single member.
def test_a_linear_configuration_round_trips() -> None:
    host = _SilentActions()
    engine = _linear.create_engine(host)

    assert (
        engine.enter_at([_LinearState.ASSEMBLING], _LinearState.ASSEMBLING)
        is ConfigurationRejection.NONE
    ), "a single-state configuration was refused"
    assert engine.current_state == _LinearState.ASSEMBLING
    assert engine.active_configuration() == {_LinearState.ASSEMBLING}
    assert engine.is_running
    assert host.idle_entries == 0 and host.assembling_exits == 0, (
        f"entry/exit content ran during a resume: {host.idle_entries} entries, "
        f"{host.assembling_exits} exits"
    )


def test_an_empty_configuration_is_refused() -> None:
    engine, _ = _new_parallel()
    assert (
        engine.enter_at([], _State.WORKING) is ConfigurationRejection.EMPTY
    ), "a machine is never in nothing"


# W3C SCXML 3.11: a compound state holds exactly one active child. `working` and
# `judging` are both children of `running`, and a run stands in one of them.
def test_two_siblings_of_one_region_are_refused() -> None:
    engine, _ = _new_parallel()
    configuration = _at_work() + [_State.JUDGING]

    assert (
        engine.enter_at(configuration, _State.WORKING)
        is ConfigurationRejection.COMPOUND_CHILD_COUNT
    ), "`running` was given two active children, which is a configuration the document has no reading for"


# W3C SCXML 3.11: a `<parallel>` holds EVERY region. Dropping one is the shape a
# host produces when it journals only the region it cares about.
def test_a_parallel_with_a_region_missing_is_refused() -> None:
    engine, _ = _new_parallel()
    configuration = [_State.RUN, _State.DRIVE, _State.RUNNING, _State.WORKING]

    assert (
        engine.enter_at(configuration, _State.WORKING)
        is ConfigurationRejection.PARALLEL_REGION_MISSING
    ), "`budget` is a region of `run` and a run is always in both at once"


# The set has to be ancestor-closed: a state is active only if its parent is.
def test_a_configuration_that_skips_an_ancestor_is_refused() -> None:
    engine, _ = _new_parallel()
    configuration = [
        _State.RUN,
        _State.DRIVE,
        _State.WORKING,
        _State.BUDGET,
        _State.WITHIN,
    ]

    assert (
        engine.enter_at(configuration, _State.WORKING)
        is ConfigurationRejection.ANCESTOR_MISSING
    ), "`working` is a child of `running`, which the set does not hold"


# Checked before the arity counts, because a duplicate would otherwise read as a
# second child and the refusal would name the wrong rule.
def test_a_repeated_state_is_refused() -> None:
    engine, _ = _new_parallel()
    configuration = _at_work() + [_State.WORKING]

    assert (
        engine.enter_at(configuration, _State.WORKING)
        is ConfigurationRejection.DUPLICATE
    )


# W3C SCXML 3.11: a configuration closes on exactly one root. `settled` is a
# top-level `<final>`, so a set holding both it and `run` describes two machines.
def test_two_roots_are_refused() -> None:
    engine, _ = _new_parallel()
    configuration = _at_work() + [_State.SETTLED]

    assert (
        engine.enter_at(configuration, _State.WORKING)
        is ConfigurationRejection.ROOT_COUNT
    )


def test_a_current_state_outside_the_configuration_is_refused() -> None:
    engine, _ = _new_parallel()
    assert (
        engine.enter_at(_at_work(), _State.JUDGING)
        is ConfigurationRejection.CURRENT_NOT_ACTIVE
    ), "the current state is the one the machine is standing in, so it is in the set by definition"


# W3C SCXML 3.11 makes the current state the ATOMIC state the engine descended
# to. A compound one is the shape a host produces when it journals the ancestor
# rather than the leaf.
def test_a_non_atomic_current_state_is_refused() -> None:
    engine, _ = _new_parallel()
    assert (
        engine.enter_at(_at_work(), _State.RUNNING)
        is ConfigurationRejection.CURRENT_NOT_ATOMIC
    )


# The claim that makes every refusal above safe to act on: validation runs
# BEFORE any mutation, so a host that gets a rejection still holds the machine
# it had. Without this the door could half-enter, and a host reading a rejection
# would be told nothing happened while the engine had already moved.
def test_a_refused_entry_leaves_the_engine_untouched() -> None:
    engine, script = _new_parallel()

    assert engine.enter_at([], _State.WORKING) is ConfigurationRejection.EMPTY

    assert not engine.is_running, "a refused entry started the machine"
    assert engine.active_leaves == [], "a refused entry wrote an active set"
    assert engine.active_configuration() == set()
    assert script.declarations == 0, (
        "a refused entry declared the datamodel; W3C SCXML 5.3 declaration is a "
        "mutation of the script-engine session and it must not happen before the "
        "set is known to be a configuration"
    )


# The accepting half of the same claim: an ACCEPTED entry does declare the
# datamodel, exactly once. Without this the count above would also pass on a
# door that never declared at all.
def test_an_accepted_entry_declares_the_datamodel_once() -> None:
    engine, script = _new_parallel()

    assert (
        engine.enter_at(_at_work(), _State.WORKING) is ConfigurationRejection.NONE
    )
    assert script.declarations == 1, (
        f"an accepted entry declared the datamodel {script.declarations} times; "
        "W3C SCXML 5.3 has the variables exist before anything can read them, and "
        "once is what `initialize` does"
    )


# W3C SCXML 3.3: every state this document declares reads back from its own
# name.
#
# A host can only record a configuration as TEXT — the generated state enum is a
# build artefact of one process, and the process that resumes is a different
# one. The forward and reverse tables are emitted from one loop over the
# document's states so they age together; this walks the document's own enum
# rather than a list spelled here, so a document that grows a state grows this
# check with it.
def test_every_state_reads_back_from_its_own_name() -> None:
    engine, _ = _new_parallel()
    policy = engine.policy

    states = list(_State)
    assert len(states) >= 8, (
        f"the document declares {len(states)} states; this walk is measuring "
        "something other than the document it names"
    )

    for state in states:
        name = policy.get_state_name(state)
        back = policy.get_state_from_name(name)
        assert back is not None, (
            f"{name!r} is the name this policy publishes for a state of its own "
            "document, and reading it back reported the name unknown"
        )
        assert back == state, f"{name!r} read back as {back}, not the state it names"

    assert policy.get_state_from_name("a-state-this-document-does-not-declare") is None, (
        "a name the document does not carry was answered with a state rather than "
        "refused; a name guessed at is how a restore reaches a configuration nobody "
        "recorded"
    )


# A configuration that crossed a process: journalled as names, read back through
# the generated reverse table, and handed to the door. This is the whole point
# of the pair — the two halves in one call chain rather than each proved alone.
def test_a_configuration_journalled_as_names_is_accepted_back() -> None:
    writer, _ = _new_parallel()
    writer.initialize()
    writer.send_event(_Event.E)

    journal = [writer.policy.get_state_name(s) for s in writer.active_configuration()]
    current_name = writer.policy.get_state_name(writer.current_state)

    reader, _ = _new_parallel()
    configuration = []
    for name in journal:
        state = reader.policy.get_state_from_name(name)
        assert state is not None, f"the journal names {name!r} and the reader could not read it back"
        configuration.append(state)
    current = reader.policy.get_state_from_name(current_name)
    assert current is not None

    assert (
        reader.enter_at(configuration, current) is ConfigurationRejection.NONE
    ), "a configuration a run actually reached was refused on the way back"
    assert reader.active_configuration() == set(configuration), (
        f"the resumed configuration is {reader.active_configuration()}, not the "
        f"journalled one {set(configuration)}"
    )
