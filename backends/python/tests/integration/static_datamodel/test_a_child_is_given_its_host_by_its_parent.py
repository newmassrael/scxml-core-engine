# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Child sessions" (docs/adr/0005, decision 6) --
Python.

A child that declares ``<sce:action>``s takes the host that performs them when
it is built, because its first ``<onentry>`` can already perform an act: a host
installed afterwards would arrive one act too late. So the host has to exist
when the invocation starts, and the parent obtains it from its own host, which
answers one for the child each time the invocation starts.

``static_child_host.scxml`` invokes ``worker``, which announces itself on entry
(``started``) and reports its steps when it ends (``finished``). The parent
declares no act: its host is there for the child alone. ``static_child_host_hybrid.scxml``
invokes whichever of two candidates its ``srcexpr`` names, each with an act of its
own, and the host answered is the one for THAT candidate.

The Kotlin half is ``AChildIsGivenItsHostByItsParentTest.kt`` and
``AHybridCandidateIsGivenItsHostByItsParentTest.kt``. The machines are the ones
``scripts/regen_static_datamodel_python.sh`` generates.
"""
from __future__ import annotations

import importlib
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))


class _ChildHost:
    """What a child asked of its host, as the acts' names and arguments."""

    def __init__(self) -> None:
        self.calls: list[tuple] = []

    def started(self) -> None:
        self.calls.append(("started",))

    def finished(self, steps: int) -> None:
        self.calls.append(("finished", steps))

    def first_ran(self) -> None:
        self.calls.append(("first_ran",))

    def second_ran(self) -> None:
        self.calls.append(("second_ran",))


class _ParentHost:
    """The parent's host: answers a fresh child host for each question and keeps
    every one, in the order asked, with what it was asked."""

    def __init__(self) -> None:
        self.asked: list[str] = []
        self.children: list[_ChildHost] = []

    def _answer(self, what: str) -> _ChildHost:
        self.asked.append(what)
        child = _ChildHost()
        self.children.append(child)
        return child

    def actions_for_worker(self) -> _ChildHost:
        return self._answer("worker")

    def actions_for_work_static_hosted_first(self) -> _ChildHost:
        return self._answer("first")

    def actions_for_work_static_hosted_second(self) -> _ChildHost:
        return self._answer("second")


def _started(machine: str, host: _ParentHost):
    module = importlib.import_module(f"integration.static_datamodel.{machine}_sm")
    engine = module.create_engine(host)
    engine.initialize()
    return engine


def _send(engine, name: str) -> None:
    engine.send_event(engine.policy.get_event_from_name(name))


def test_the_child_performs_its_first_act_through_the_host_its_parent_answered() -> None:
    host = _ParentHost()
    _started("static_child_host", host)
    assert host.asked == ["worker"], "the parent's host was asked once"
    # The act of its first `<onentry>` is already performed: the host was there
    # when the child was built, not installed after.
    assert host.children[0].calls == [("started",)]


def test_the_child_reports_what_it_did_through_the_same_host() -> None:
    host = _ParentHost()
    engine = _started("static_child_host", host)
    _send(engine, "a")
    _send(engine, "b")
    assert engine.policy.completed() == 1, "the child ended and the parent counted it"
    assert host.children[0].calls == [("started",), ("finished", 2)]
    assert host.asked == ["worker"], "nothing asked the parent's host again"


def test_a_state_invoked_again_is_given_a_host_of_its_own() -> None:
    host = _ParentHost()
    engine = _started("static_child_host", host)
    for name in ("a", "b", "again", "back"):
        _send(engine, name)
    assert host.asked == ["worker", "worker"], "asked once per start"
    assert host.children[0] is not host.children[1]
    # The first child's run is its own, and the second starts from nothing.
    assert host.children[0].calls == [("started",), ("finished", 2)]
    assert host.children[1].calls == [("started",)]


def test_a_candidate_is_given_the_host_answered_for_it_and_no_other() -> None:
    host = _ParentHost()
    engine = _started("static_child_host_hybrid", host)
    assert host.asked == ["first"]
    assert host.children[0].calls == [("first_ran",)]
    assert engine.policy.completed() == 1, "it ran and ended"

    _send(engine, "again")
    _send(engine, "back")
    assert host.asked == ["first", "second"]
    assert host.children[1].calls == [("second_ran",)]
    # The first candidate's run is its own and was not repeated.
    assert host.children[0].calls == [("first_ran",)]
    assert engine.policy.completed() == 2
