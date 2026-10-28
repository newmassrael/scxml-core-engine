# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated Python.

A variable is an attribute of the policy, every expression was lowered to Python
at build time, and no datamodel lives in a script engine.

The scenarios here are the ones Kotlin, Rust, C++, Go and the Interpreter
replay -- ``sce-build/tests/fixtures/static_datamodel/scenarios/<machine>.json``,
whose expected values are derived from the document, not observed from a
backend -- so the engines are held to one answer. A scenario is a list of steps,
each an external event with its payload and what the machine must hold after it
runs to quiescence: its current state and any of its published variables.

The machines are the ones ``scripts/regen_static_datamodel_python.sh`` generates,
into this directory and out of version control, with the algorithms they call as
modules beside them: the directory is a package, and a machine is imported as
one of its members so that its ``from . import <algorithm>`` resolves. Those
with no scenario here are generated for their effect: a machine nobody built is
a machine nobody type-checked.

Regeneration (after a fixture or template edit):
  ``scripts/regen_static_datamodel_python.sh`` (local)
"""
from __future__ import annotations

import dataclasses
import importlib
import inspect
import json
import re
import sys
from pathlib import Path
from typing import Optional

import pytest

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import (  # noqa: E402 -- path inserted above
    ActionRealization,
    EventMetadata,
    MeshRaise,
    saved_state_from_json,
)

# The id a shared fixture's root session is given, and the moment on the wall clock
# a scenario's `restore` step saves it at (docs/adr/0045, section 8).
_ROOT_SESSION_ID = "fixture-root"
_SAVED_AT_MS = 1_700_000_000_000

# Where the scenarios live, from this file's directory.
_SCENARIOS = _HERE.parents[4] / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "scenarios"


def _atomic_state(engine) -> str:
    """The current state is the atomic one: a compound state is active for as
    long as one of its children is, and is no more where the machine is."""
    leaves = engine.active_leaves
    if len(leaves) != 1:
        return f"<{len(leaves)} active atomic states>"
    return str(leaves[0])


def _read(policy, readers: dict, variable: str):
    """What the machine holds in ``variable``: through the reader it publishes,
    or -- for a variable the machine keeps to itself, which has none, so that
    renaming it never changes what a host was written against -- from its
    attribute. Reading is all this does."""
    if variable in readers:
        return getattr(policy, readers[variable])()
    attribute = "v_" + re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", variable).lower()
    assert hasattr(policy, attribute), f"the machine holds no variable {variable!r}"
    return getattr(policy, attribute)


def _as_json(value):
    """``value`` as the scenario writes it: an enum member by the name its
    document declares, a record by the schema's field ids, a list element by
    element -- what a host reads back, in the shape JSON states."""
    if hasattr(value, "sce_name"):
        return value.sce_name
    # A byte string as its byte-exact Latin-1 text, each byte the character of
    # that code point (docs/adr/0005, decision 2).
    if isinstance(value, bytes):
        return value.decode("latin-1")
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return {f.name: _as_json(getattr(value, f.name)) for f in dataclasses.fields(value)}
    if isinstance(value, (list, tuple)):
        return [_as_json(item) for item in value]
    return value


def _holds(got, want) -> bool:
    """Whether what the machine holds is the scenario's. A scenario states a
    64-bit integer as the text of its digits, which is how a saved state writes
    one (a JSON number past 2^53 is not every reader's to keep), and Python holds
    an integer of any width as an ``int``: an ``int`` is held to the digits that
    spell it. A string the machine holds is a ``str`` and is not read as a number,
    and a record or a list is held field by field and element by element.
    Everything else must be equal as written."""
    if isinstance(got, dict) and isinstance(want, dict):
        return got.keys() == want.keys() and all(_holds(got[key], want[key]) for key in want)
    if isinstance(got, list) and isinstance(want, list):
        return len(got) == len(want) and all(_holds(g, w) for g, w in zip(got, want))
    if isinstance(got, int) and not isinstance(got, bool) and isinstance(want, str):
        return str(got) == want
    return got == want


class _ScenarioHost:
    """What a machine asked of its host, as a scenario's ``host_calls`` writes it:
    the name the document gives the action, and its arguments in the order the
    document gives them.

    It records and performs nothing, so it declares that it only simulates: a
    machine that states a duty (docs/adr/0038) takes it only when built with
    ``accept_simulated_host=True``, which :func:`replay` does. A machine with no
    duty has no ``realization`` in its protocol and never reads it."""

    realization = ActionRealization.SIMULATES

    def __init__(self) -> None:
        self._calls: list[dict] = []

    def show_attempts(self, count: int, exhausted: bool) -> None:
        self._calls.append({"action": "showAttempts", "args": [count, exhausted]})

    def show_due(self, day: Optional[int], urgent: Optional[bool]) -> None:
        # An argument that may be absent is ``None``, which a scenario states as
        # ``null`` (docs/adr/0055, step 10).
        self._calls.append({"action": "showDue", "args": [day, urgent]})

    def start_keepalive(self) -> None:
        self._calls.append({"action": "start_keepalive", "args": []})

    def note_engaged(self) -> None:
        self._calls.append({"action": "note_engaged", "args": []})

    def stop_keepalive(self) -> None:
        self._calls.append({"action": "stop_keepalive", "args": []})

    def take(self) -> list[dict]:
        """The calls made since this was last called, oldest first."""
        taken, self._calls = self._calls, []
        return taken


class _ChildHosts:
    """The child hosts a machine's host was asked for, in the order asked, and what
    each child asked of its host, as a scenario's ``child_hosts`` writes them. A
    child owns the host it was given, so each host is made here (:meth:`open`)
    and writes into the book this reads."""

    def __init__(self) -> None:
        self._children: list[dict] = []

    def open(self, invoke: str, candidate: str | None = None) -> "_ChildHost":
        """The machine's host was asked for the host of a child its ``invoke``
        started, and answers the one this returns. ``candidate`` is the stem of the
        document when the invoke names one of several."""
        start = 1 + sum(1 for child in self._children if child["invoke"] == invoke)
        child = {
            "invoke": invoke,
            "start": start,
            "candidate": candidate,
            "calls": [],
            "taken": 0,
            "announced": False,
        }
        self._children.append(child)
        return _ChildHost(child["calls"])

    def take(self) -> list[dict]:
        """The children asked for or called since this was last called, oldest
        asked first: the invoke, the start of that invoke, the candidate's document
        stem where the invoke names one, and the calls made in this step alone. A
        child asked for earlier that called nothing since has no entry."""
        fresh = []
        for child in self._children:
            calls = child["calls"][child["taken"] :]
            if child["announced"] and not calls:
                continue
            child["taken"] = len(child["calls"])
            child["announced"] = True
            entry = {"invoke": child["invoke"], "start": child["start"]}
            if child["candidate"] is not None:
                entry["candidate"] = child["candidate"]
            entry["host_calls"] = calls
            fresh.append(entry)
        return fresh


class _ChildHost:
    """A child's host: every act a child of these machines declares, each written
    down as a scenario's ``host_calls`` writes it. A child is only ever asked for
    the acts its own document declares."""

    def __init__(self, calls: list[dict]) -> None:
        self._calls = calls

    def started(self) -> None:
        self._calls.append({"action": "started", "args": []})

    def finished(self, steps: int) -> None:
        self._calls.append({"action": "finished", "args": [steps]})

    def first_ran(self) -> None:
        self._calls.append({"action": "first_ran", "args": []})

    def second_ran(self) -> None:
        self._calls.append({"action": "second_ran", "args": []})


class _ParentHost:
    """The host of a machine that declares no act of its own and whose children are
    given hosts: it answers a host for the child each ``invoke`` starts, under the
    name the generated machine asks for it (``actions_for_<invoke>`` and, for a
    hybrid invoke, ``_<candidate stem>``), and writes it into the book."""

    def __init__(self, children: _ChildHosts) -> None:
        self._children = children

    def actions_for_worker(self) -> _ChildHost:
        return self._children.open("worker")

    def actions_for_work_0j_static_hosted_first(self) -> _ChildHost:
        return self._children.open("work", "static_hosted_first")

    def actions_for_work_0j_static_hosted_second(self) -> _ChildHost:
        return self._children.open("work", "static_hosted_second")


class _ScenarioPeers:
    """What a machine sent its Mesh peers -- a ``<send>`` whose target is
    ``#name`` -- as a scenario's ``sent`` writes it: the target as the document
    writes it, the event's name, and the event's data as the JSON value its text
    spells, left out when the send carried none. It is the router a host
    registers: with none registered such a send is ``error.execution`` and
    reaches nobody."""

    def __init__(self) -> None:
        self._sends: list[dict] = []

    def route(self, request) -> list:
        sent = {"target": request.target, "event": request.event_name}
        if request.event_data:
            sent["data"] = json.loads(request.event_data)
        self._sends.append(sent)
        return []

    def take(self) -> list[dict]:
        """The sends made since this was last called, oldest first."""
        taken, self._sends = self._sends, []
        return taken

    def record_child(self, request, invoke: str, start: int) -> None:
        """A child session's send, written as the machine's own are, with the
        path of invoke ids that started the child and the start of that path."""
        self.route(request)
        self._sends[-1]["invoke"] = invoke
        self._sends[-1]["start"] = start


class _ChildRouters:
    """The routers a driver gives the child sessions of the machine it drives
    (SCE_MESH.md, Child Sessions in Mesh): `factory` answers a router for every
    child, which writes what the child sends into `peers` under the path of
    invoke ids that started it and the start of that path, holds what a step's
    ``invoke`` hands the child until the parent applies it, and answers for the
    child's own children under ``<path>/<invoke>``."""

    def __init__(self, peers: _ScenarioPeers) -> None:
        self._peers = peers
        self._starts: dict[str, int] = {}
        self._running: dict[str, "_ChildRouter"] = {}

    def factory(self, invoke_id: str, _document: str) -> "_ChildRouter":
        return self.open(invoke_id)

    def open(self, path: str) -> "_ChildRouter":
        """A router for a new start of `path`, which is now the session the
        path is running."""
        self._starts[path] = self._starts.get(path, 0) + 1
        router = _ChildRouter(self, path, self._starts[path])
        self._running[path] = router
        return router

    def deliver(self, path: str, event: str, data: str) -> bool:
        """Hand `event`, with `data` as its ``_event.data``, to the running child
        of `path`; ``False`` when no child of that path is running."""
        router = self._running.get(path)
        if router is None:
            return False
        router.inbox.append(MeshRaise(event, EventMetadata(data=data)))
        return True

    def detached(self, router: "_ChildRouter") -> None:
        if self._running.get(router.path) is router:
            del self._running[router.path]

    def record(self, request, path: str, start: int) -> None:
        self._peers.record_child(request, path, start)


class _ChildRouter:
    """The router of one start of one child."""

    def __init__(self, book: _ChildRouters, path: str, start: int) -> None:
        self._book = book
        self.path = path
        self.start = start
        self.inbox: list = []

    def send(self, request) -> list:
        self._book.record(request, self.path, self.start)
        return []

    def invoke(self, _event):
        # A request the child makes is started and never answered: no peer is
        # there to answer it.
        return None

    def take_calls(self) -> list:
        taken, self.inbox = self.inbox, []
        return taken

    def detach(self) -> None:
        self._book.detached(self)

    def child_router(self, invoke_id: str, _document: str) -> "_ChildRouter":
        return self._book.open(f"{self.path}/{invoke_id}")


def _same_json(got, want) -> bool:
    """Whether two JSON values are the same value. Python's own equality says
    ``True == 1``, which JSON does not: a boolean is equal only to a boolean,
    inside an object or an array as at the top."""
    if isinstance(got, bool) or isinstance(want, bool):
        return isinstance(got, bool) and isinstance(want, bool) and got == want
    if isinstance(got, dict) and isinstance(want, dict):
        return got.keys() == want.keys() and all(_same_json(got[key], want[key]) for key in want)
    if isinstance(got, list) and isinstance(want, list):
        return len(got) == len(want) and all(_same_json(g, w) for g, w in zip(got, want))
    return type(got) is type(want) and got == want


def _machine_stem(machine: str) -> str:
    """The name a scenario's machine is generated under: a fixture by its stem, and a
    statechart of the standard library (``sce:std/shared/record_list``) by the last
    segment of its library name, which is the stem it is generated from."""
    return machine.rsplit("/", 1)[-1] if machine.startswith("sce:std/") else machine


def replay(name: str, machine: str | None = None) -> None:
    """Run scenario ``name`` against the machine it names. Every step names an
    event (or none, for the machine as started) and what it must hold
    afterwards. A name no event of the machine matches is a misspelt step unless
    the step says it expects the drop (``"dropped": true``), which is then what
    is held. A name matches an event of the machine as the policy resolves it
    (``resolve_event_by_name``, §scxml-3.12.1): the document need not write it."""
    scenario = json.loads((_SCENARIOS / f"{name}.json").read_text())
    steps = scenario["steps"]
    assert steps, f"scenario {name} has no steps, which judges nothing"
    module = importlib.import_module(
        f"integration.static_datamodel.{_machine_stem(machine or scenario['machine'])}_sm"
    )
    # A machine with `<sce:action>`s is built with the host that performs them, and
    # the generated engine says so by taking it: that host is the one that records.
    # One whose host only answers the hosts of its children (the methods of its
    # actions protocol are all ``actions_for_<invoke>``) is given the host that
    # writes them into the book a step's ``child_hosts`` is read from.
    host = None
    children = None
    parameters = inspect.signature(module.create_engine).parameters
    if "actions" in parameters:
        protocol = getattr(module, str(parameters["actions"].annotation))
        if any(name.startswith("actions_for_") for name in vars(protocol)):
            children = _ChildHosts()
            host = _ParentHost(children)
        else:
            host = _ScenarioHost()
    # The host here records and so only simulates: a machine that states a duty,
    # or builds a child that does, takes it by being told so (docs/adr/0038).
    opt_in = {"accept_simulated_host": True} if "accept_simulated_host" in parameters else {}
    # The shared fixtures give the root's id explicitly, so a saved text of one is
    # the text on every backend (docs/adr/0045, "Questions the owner answered" 2).
    engine = (
        module.create_engine(session_id=_ROOT_SESSION_ID)
        if host is None
        else module.create_engine(host, session_id=_ROOT_SESSION_ID, **opt_in)
    )
    # Every machine is given the router that records, before it starts, so that
    # what its start sends is recorded too: one with no send to a Mesh peer never
    # calls it. A scenario that says `"router": false` is of a machine whose host
    # attached no link (docs/adr/0046, section 4): it is given no router and no
    # factory for its children, so every send to a peer is refused where it is made.
    routed = scenario.get("router", True) is not False
    peers = _ScenarioPeers()
    if routed:
        engine.register_mesh_router(peers.route)
    # A child session the machine starts is given a router of its own by the
    # factory its host registers (SCE_MESH.md, Child Sessions in Mesh), never the
    # machine's: what a child sends is written into the same list, told by the
    # invoke that started it and the start.
    routers = _ChildRouters(peers)
    if routed:
        engine.register_child_mesh_routers(routers.factory)
    engine.initialize()
    policy = engine.policy
    readers = module.SCE_HOST_NAMES["readers"]

    def restore() -> None:
        """Save the machine, discard it and restore it from the text, as a state that
        crossed a process boundary would be: the restored machine carries on where
        the saved one stood (docs/adr/0045, section 8). It is given what the host
        gave the first -- its router and the factory of its children's routers --
        and the children it resumed are asked for theirs when the factory arrives."""
        nonlocal engine, policy
        saved = module.save_at(engine, _SAVED_AT_MS)
        through_its_text = saved_state_from_json(saved.to_json())
        arguments = () if host is None else (host,)
        engine = module.restore(*arguments, through_its_text, wall_now_ms=_SAVED_AT_MS, **opt_in)
        policy = engine.policy
        if routed:
            engine.register_mesh_router(peers.route)
            engine.register_child_mesh_routers(routers.factory)

    for n, step in enumerate(steps):
        where = f"{name} step {n}: {step.get('note', '')}"
        # The machine is saved and restored before the step's event or time, so what
        # the step then expects is what the restored one does.
        if step.get("restore"):
            restore()
        # A step that moves the machine's time on, for a scenario of a delayed
        # send: the engine never reads a clock of its own, so a wait is the one
        # the step names.
        if "advance_ms" in step:
            engine.advance_time(step["advance_ms"])
        event_name = step.get("event")
        if event_name is not None and "invoke" in step:
            # A peer's event for the running child of that invoke, handed to the
            # router the child was given; the machine applies it when it next
            # drives its children, which a turn of its clock that moves no time
            # does. The child's events are not the machine's, so the machine is
            # not asked whether it matches.
            data = json.dumps(step["data"]) if "data" in step else ""
            assert routers.deliver(step["invoke"], event_name, data), (
                f"{where}: no child of {step['invoke']!r} is running to be sent {event_name!r}"
            )
            for _ in range(5):
                engine.advance_time(0)
        elif event_name is not None:
            event = policy.resolve_event_by_name(event_name)
            dropped = step.get("dropped", False)
            assert (event is None) == dropped, (
                f"{where}: the machine's events {'match' if dropped else 'do not match'} {event_name!r}"
            )
            if event is not None:
                data = json.dumps(step["data"]) if "data" in step else ""
                engine.send_event(event, EventMetadata(data=data))
        expect = step["expect"]
        # What the machine asked of its host in this step. Read on every step so
        # that a step which states nothing does not hand its calls to the next, and
        # a step that states some needs a driver that records them.
        asked = host.take() if isinstance(host, _ScenarioHost) else None
        if "host_calls" in expect:
            assert asked is not None, (
                f"{where}: this driver records no host calls, so it cannot hold a step to them"
            )
            assert asked == expect["host_calls"], (
                f"{where}: the machine asked its host {asked!r}, not {expect['host_calls']!r}"
            )
        # The child hosts the machine's host was asked for in this step, and what
        # each child asked of its host, read and held the same way.
        made = children.take() if children is not None else None
        if "child_hosts" in expect:
            assert made is not None, (
                f"{where}: this driver records no child hosts, so it cannot hold a step to them"
            )
            assert _same_json(made, expect["child_hosts"]), (
                f"{where}: the child hosts were {made!r}, not {expect['child_hosts']!r}"
            )
        # What the machine sent its Mesh peers in this step, read on every step for
        # the same reason: every step takes its own, so none is handed on.
        sent = peers.take()
        if "sent" in expect:
            assert _same_json(sent, expect["sent"]), (
                f"{where}: the machine sent its Mesh peers {sent!r}, not {expect['sent']!r}"
            )
        if expect.get("ended"):
            assert engine.reached_final, f"{where}: the machine ended in a top-level <final>"
            if "donedata" in expect:
                # What the final's `<donedata>` left for an invoking parent, as
                # the JSON text every producer writes.
                got = json.loads(engine.done_data)
                assert got == expect["donedata"], (
                    f"{where}: the donedata is {got!r}, not {expect['donedata']!r}"
                )
            continue
        if "state" in expect:
            assert _atomic_state(engine) == expect["state"], where
        # Every active state, a compound or a parallel one with the atomic ones
        # below it. A set: the order a machine lists them in is not part of the
        # answer.
        if "configuration" in expect:
            got = {str(state) for state in engine.active_configuration()}
            assert got == set(expect["configuration"]), (
                f"{where}: the active states are {sorted(got)!r}, "
                f"not {sorted(expect['configuration'])!r}"
            )
        for variable, want in expect.get("variables", {}).items():
            got = _as_json(_read(policy, readers, variable))
            assert _holds(got, want), f"{where}: variable {variable!r} is {got!r}, not {want!r}"


# A counter counts to its flag and lets go: guards, `<assign>`, `<if>` and
# `<elseif>` are native, and `In()` asks the machine's own configuration.
def test_the_counter_counts_to_its_flag_and_lets_go() -> None:
    replay("static_counter")


def test_the_counter_stops_at_its_bound_and_refuses_go() -> None:
    replay("static_counter_bound")


# An event arrives by name from outside the document, so the names it can arrive
# under are open (§scxml-3.12.1): a name the document never writes reaches the
# transition whose descriptor is a token prefix of it, and one no descriptor
# matches is dropped.
def test_an_event_arrives_under_a_name_the_document_does_not_write() -> None:
    replay("static_event_arrival")


# ...and where the document listens with `event="*"`, a name no descriptor it
# writes extends is delivered as the wildcard event instead of being dropped.
def test_an_event_arrives_under_a_name_only_the_wildcard_takes() -> None:
    replay("static_event_wildcard")


# A checked integer operation that overflows fails instead of wrapping.
def test_an_overflowing_operation_fails_instead_of_wrapping() -> None:
    replay("static_overflow")


# An error ends the block it stands in (W3C SCXML 4.9): the statements after a
# failed `<assign>`, after a failed `<if>` cond, or inside a branch that failed
# do not run, while the next block does.
def test_an_error_ends_the_block_it_stands_in() -> None:
    replay("static_block_ends")


# An event's typed payload is read in a guard and in assignments.
def test_an_events_typed_payload_is_read_in_a_guard_and_in_content() -> None:
    replay("static_payload")


# An event's payload carries an enum field, the variant's declared name: a guard
# compares it to a variant and an assignment stores it in a variable of the enum.
def test_an_events_payload_carries_an_enum_field() -> None:
    replay("static_payload_enum")


# The payload of the event a transition is on is carried on as the <param>s of a
# <send>: an enum field as the name its enum declares and an integer, read where
# the send runs.
def test_the_payload_of_an_event_is_carried_on_as_params() -> None:
    replay("static_payload_relay")


# A delivery that carried no payload runs none of the content that reads it: the
# machine sends nothing, so nothing is relayed. The scenarios cannot say so for
# every engine -- the Interpreter fails the expression that reads the field,
# where a generated machine does not run the block at all -- so it is stated here.
def test_a_delivery_without_the_payload_sends_nothing() -> None:
    module = importlib.import_module("integration.static_datamodel.static_payload_relay_sm")
    engine = module.create_engine()
    engine.initialize()
    policy = engine.policy
    engine.send_event(policy.resolve_event_by_name("view.shown"), EventMetadata(data=""))
    assert policy.v_relays == 0, "no payload was read, so nothing was carried on"
    engine.send_event(
        policy.resolve_event_by_name("view.shown"),
        EventMetadata(data=json.dumps({"layout": "week", "zoom": 2})),
    )
    assert policy.v_relays == 1, "a delivery that carried the payload is relayed as any other"


# An enum variable holds a variant of its enum, read back by the name the enum
# document declares.
def test_an_enum_variable_holds_a_variant_of_its_enum() -> None:
    replay("static_enum")


# docs/adr/0047: a value of an enum read as the number its variant declares and a
# number read back as the variant that declares it; a number no variant declares
# is error.execution.
def test_an_enum_is_read_as_its_number_and_back() -> None:
    replay("static_enum_number")


# docs/adr/0048: a condition, a transform, two lookups, an interpolation and a
# validator called as `Alias(args)`; a lookup whose miss is an error fails into
# error.execution.
def test_the_kinds_that_are_functions_are_called_as_an_algorithm_is() -> None:
    replay("static_forge_calls")


# A list is filled to its bound and emptied.
def test_a_list_is_filled_to_its_bound_and_emptied() -> None:
    replay("static_list")


# A list variable takes what an algorithm returns as a list, whole; a call that
# fails leaves it as it was.
def test_a_list_takes_what_an_algorithm_returns_whole() -> None:
    replay("static_list_assign")


# A statechart hands the algorithms the records and lists it holds, and the
# event's own payload, and takes back what they return, written only when the
# call succeeded and held to the bound of the variable it is written to.
def test_a_statechart_hands_its_state_to_an_algorithm_and_takes_it_back() -> None:
    replay("static_state_algorithm")


# A statechart hands the standard text algorithms the record and the list of
# records it holds, whose text is bounded, and takes back what they return: a
# text past the bound is refused where the payload is taken, the state as it was.
def test_a_statechart_hands_bounded_text_to_an_algorithm_and_takes_it_back() -> None:
    replay("static_text_record")


# A statechart hands the algorithms records whose field is a value of an enum, and a
# value of the enum itself, and takes back a list of those records and a value of the
# enum: each crosses by the name its variant is declared with.
def test_a_statechart_hands_an_enum_to_an_algorithm_and_takes_it_back() -> None:
    replay("static_state_algorithm_enum")


# An element is read by its index, and an index outside the list is a failure.
def test_a_list_element_is_read_by_its_index() -> None:
    replay("static_list_index")


# A `<foreach>` walks a list variable.
def test_a_foreach_walks_a_list_variable() -> None:
    replay("static_foreach")


# A 64-bit real is a native binary64 field: a product and a sum, a quotient, a
# guard comparing it with a literal, and a `<foreach>` summing a list of reals.
def test_a_real_is_a_native_binary64_field() -> None:
    replay("static_real")


# A 32-bit real is a native binary32 field: every operation on it is rounded to
# binary32 where it is made, though Python holds it in a double.
def test_a_real_is_a_native_binary32_field() -> None:
    replay("static_real32")


# An append that fails ends its block, the list as it was.
def test_an_append_that_fails_ends_its_block() -> None:
    replay("static_block_ends_list")


# A record is built whole from its `<sce:set>`s and updated a field at a time,
# from the machine's own value and from a typed event payload.
def test_a_record_is_built_whole_and_updated_a_field_at_a_time() -> None:
    replay("static_record_fields")


# A record with a 64-bit real field is built whole, written a field at a time,
# and replaced from a typed payload without losing a bit of the real it carried.
def test_a_record_holds_a_real_field_to_the_bit() -> None:
    replay("static_record_real")


# A record with a 32-bit real field: the double a payload carries lands as the
# single nearest it, and every operation on the field is a single's.
def test_a_record_holds_a_single_field_as_the_single_nearest_the_payload() -> None:
    replay("static_record_real32")


# A record's bytes field is held to the bytes its schema declares: an assignment
# past the bound — from a literal or a bytes variable — writes nothing, raises
# error.execution and ends its block, and a list of such records holds copies with
# their bytes. A scenario states a byte string as its byte-exact Latin-1 text.
def test_a_record_holds_a_bytes_field_within_the_bound_its_schema_declares() -> None:
    replay("static_record_bytes")


# A record is a frozen dataclass of immutable `bytes`, so what a host is handed is the
# value itself and cannot be written into, and two records of the same bytes are equal.
def test_a_record_of_the_same_bytes_is_equal_and_cannot_be_written_into() -> None:
    module = importlib.import_module("integration.static_datamodel.static_record_bytes_sm")
    engine = module.create_engine()
    engine.initialize()
    last = getattr(engine.policy, module.SCE_HOST_NAMES["readers"]["last"])()
    assert isinstance(last.frame, bytes)
    assert last == dataclasses.replace(last, frame=bytes(last.frame))
    assert last != dataclasses.replace(last, frame=b"zz")
    with pytest.raises(dataclasses.FrozenInstanceError):
        last.frame = b"zz"  # type: ignore[misc]


# The bytes an event's payload carries are read into a bytes variable, into a record's
# field and into a whole record, each held to its own bound: a value past it writes
# nothing, raises error.execution and ends its block. The wire spells a byte string as
# its byte-exact Latin-1 text, which a scenario states it as.
def test_the_payload_of_an_event_carries_bytes_held_within_their_bounds() -> None:
    replay("static_payload_bytes")


# A Python `bytes` cannot be written into, so the machine may keep the very object a
# host raised the event with: the host has no way to change what the machine read.
def test_a_host_cannot_change_the_bytes_an_event_it_raised_carried() -> None:
    module = importlib.import_module("integration.static_datamodel.static_payload_bytes_sm")
    engine = module.create_engine()
    engine.initialize()
    carried = b"wxyz"
    module.raise_framed_taken(engine, 100, carried)
    held = getattr(engine.policy, module.SCE_HOST_NAMES["readers"]["held"])()
    assert isinstance(held, bytes)
    assert held == b"wxyz"
    with pytest.raises(TypeError):
        carried[0] = ord("X")  # type: ignore[index]


# A record's string field is held to the UTF-8 bytes its schema declares: an
# assignment past the bound — from a literal, a string variable or a payload —
# writes nothing, raises error.execution and ends its block, and a list of such
# records holds copies with their text.
def test_a_record_holds_a_string_field_within_the_bound_its_schema_declares() -> None:
    replay("static_record_string")


# A list of records is filled by name from a record variable or a loop's item,
# walked by a `<foreach>`, and a record is taken whole.
def test_a_list_holds_records_and_a_foreach_walks_them() -> None:
    replay("static_record_list")


# A field of the element a list of records is indexed at is read like any
# number, and an index outside the list is a failure.
def test_a_field_of_an_indexed_record_is_read() -> None:
    replay("static_record_list_index")


# A 64-bit integer crosses a `<send>`'s `<param>` as the text of its digits and is
# read back from it: the low digits of a value past 2^53 are the ones that were
# sent, and a number past 2^53 - 1 is refused where it is read.
def test_a_64_bit_integer_crosses_as_its_digits() -> None:
    replay("static_wire_wide")


# The payload of an event is a record of its schema taken whole: it replaces a
# record variable in one assignment and is appended whole to a list.
def test_the_payload_of_an_event_is_taken_whole_as_a_record() -> None:
    replay("static_whole_payload")


# A delivery written against an earlier revision of a schema that grew lacks the
# fields a later revision added, and takes their defaults; a delivery that lacks a
# field of the first revision is refused, and a field the schema does not name is
# not read.
def test_a_delivery_of_an_earlier_revision_takes_the_defaults_of_the_fields_added_since() -> None:
    replay("static_schema_revision")


# An enum value as a `<param>` crosses as the name its enum declares for it: a
# variable, a field of a record variable and a conditional, sent and read back
# through the schema.
def test_an_enum_value_crosses_as_the_name_its_enum_declares() -> None:
    replay("static_wire_enum")


# A record may hold an enum: its field is read back by the name the enum
# document declares.
def test_a_record_holds_an_enum_field() -> None:
    replay("static_record_enum")


# A guard calls an imported algorithm with the record's own fields, and the
# call is the module the algorithm's own generation put beside the machine.
def test_a_guard_calls_an_imported_algorithm() -> None:
    replay("static_record")


# A sync run composed of the standard sync rules: each rule an imported
# algorithm, called from a guard or an assignment, and the run's payloads the
# standard event schemas'.
def test_a_sync_run_is_composed_of_the_standard_sync_rules() -> None:
    replay("sync_client")


# One copy of a shared to-do list, as each of the three that took part in one run
# of it was driven: a person's copy stamps a change with its clock and sends it to
# the hub, the hub passes a write on to every copy but the one it came from, and
# each to-do is an element of an add set and a tombstone set, and each of its
# fields a stamped value, so the copies agree whatever order the writes arrive in.
# Every step states what the copy sent its peers and the `adds`, `tombstones` and
# `values` it holds.
def test_a_shared_list_as_alice_took_part_in_it() -> None:
    replay("todo_peer_alice")


def test_a_shared_list_as_bob_took_part_in_it() -> None:
    replay("todo_peer_bob")


def test_a_shared_list_as_the_hub_its_copies_send_to() -> None:
    replay("todo_peer_server")


# The standard shared record list (sce:std/shared/record_list.scxml), named by its
# library name as its scenarios' machine: a person's copy, which sends its changes to
# the hub by `#hub`, and the hub, which passes every change on to the people by
# `#people`.
def test_the_standard_record_list_as_a_persons_copy() -> None:
    replay("record_list_person")


def test_the_standard_record_list_as_the_hub_its_people_send_to() -> None:
    replay("record_list_hub")


# A standard record list a host started has no parent: it tells no one of the
# registers its changes move, and nothing fails.
def test_a_standard_record_list_started_by_a_host_tells_no_parent() -> None:
    replay("record_list_alone")


# A copy given no Mesh router keeps every change it makes: what it does to itself is
# queued before a send to a peer is refused where it is made (docs/adr/0046).
def test_a_standard_record_list_with_no_router_keeps_its_changes() -> None:
    replay("record_list_offline")


# A change is one unit: staged, stamped once, taken in whole, and sent as parts a copy
# holds until they are all there (docs/adr/0046).
def test_a_standard_record_list_takes_a_change_as_one_unit() -> None:
    replay("record_list_change")


# A field is emptied and a row retired by writing over what the copy has seen, and a
# write it had not seen stays (docs/adr/0046).
def test_a_standard_record_list_clears_a_field_and_retires_a_row() -> None:
    replay("record_list_retire")


# A statechart that invokes the standard record list by its library name is told what
# each register a change moved holds: its count and its values in the order of their
# dots, a count of 0 when every value was written over, and nothing for a write the
# list already held.
def test_a_parent_is_told_what_each_register_of_its_list_holds() -> None:
    replay("record_list_parent")


# One copy of the list of who is online, as each of the three that took part in one
# afternoon of it was driven: a client says it is here to the hub, the hub keeps
# what it was told for a ttl and tells every copy on one topic, and an entry is let
# go by the machine's own delayed send, which the `advance_ms` steps move the
# engine's time on to. Every step states what the copy sent its peers.
def test_a_presence_list_as_alice_took_part_in_it() -> None:
    replay("presence_peer_alice")


def test_a_presence_list_as_bob_took_part_in_it() -> None:
    replay("presence_peer_bob")


def test_a_presence_list_as_the_hub_its_copies_send_to() -> None:
    replay("presence_peer_server")


# One copy of a shared sheet, as each of the three that took part in one run of it
# was driven: its cells merge as the to-do list's fields do, and its rows and
# columns are sequences every copy orders alike, with what was removed kept apart
# from what was placed. Every step states what the copy sent its peers.
def test_a_shared_sheet_as_alice_took_part_in_it() -> None:
    replay("sheet_peer_alice")


def test_a_shared_sheet_as_bob_took_part_in_it() -> None:
    replay("sheet_peer_bob")


def test_a_shared_sheet_as_the_hub_its_copies_send_to() -> None:
    replay("sheet_peer_server")


# The `eventexpr` of a `<send>` is a string computed from the machine's fields when
# the send runs, and names the event the send delivers.
def test_a_sends_event_is_named_when_it_runs() -> None:
    replay("static_send_event")


# The `targetexpr` of a `<send>` is a string computed from the machine's fields when
# the send runs, held to the routes the document declares as `sce:targets`
# (docs/adr/0005, decision 3): the send goes by the entry that matches, and a value
# in none of them is error.communication with nothing sent.
def test_a_sends_target_is_chosen_among_the_declared_routes() -> None:
    replay("static_send_target")


# The `typeexpr` of a `<send>` is a string computed from the machine's fields when
# the send runs, held to the processors the document declares as `sce:types`
# (docs/adr/0005, decision 3): the send is delivered by the processor the matching
# entry names, and a value in none of them is error.execution with nothing sent.
def test_a_sends_type_is_chosen_among_the_declared_processors() -> None:
    replay("static_send_type")


# The `delayexpr` of a `<send>` is a string computed from the machine's fields when
# the send runs, and read as the CSS2 time it must be; the scenario's
# `advance_ms` steps move the engine's time on.
def test_a_sends_delay_is_computed_when_it_runs() -> None:
    replay("static_send_delay")


# The `sendidexpr` of a `<cancel>` is a string computed from the machine's fields
# when the cancel runs, the id of the delayed send it removes; the scenario's
# `advance_ms` steps move the engine's time on.
def test_a_cancel_removes_the_send_its_id_names() -> None:
    replay("static_cancel_expr")


# A native host action takes the machine's variables as typed arguments, each
# read when the call is made, and the host the scenario's `host_calls` are read
# from records them: one call per entry of `idle`, with the datamodel as it stood.
def test_a_host_action_tells_its_host_what_the_datamodel_held() -> None:
    replay("static_host_call")


# A duty is taken on entering `engaged` and given back on leaving it, beside a
# notice. The driver's host records, so it simulates, and the machine is built for
# it on purpose (docs/adr/0038): its calls are what `host_calls` is read from.
def test_a_host_duty_is_taken_on_entry_and_given_back_on_exit() -> None:
    replay("static_host_duty")


# A child that declares `<sce:action>`s is built with the host its parent's host
# answers for it, each time the invocation starts, and what the child asked of that
# host is what the scenario's `child_hosts` holds.
def test_each_start_of_a_child_is_given_a_host_of_its_own() -> None:
    replay("static_child_host")


# A hybrid `<invoke>` is given the host answered for the candidate it starts, and for
# no other.
def test_a_candidate_is_given_the_host_answered_for_it() -> None:
    replay("static_child_host_hybrid")


# A child session an `<invoke>` started is driven through its parent by
# autoforward, takes the events it waits for in order, and its end reaches the
# parent as `done.invoke`, which the parent counts.
def test_an_invoked_child_is_driven_and_counted() -> None:
    replay("static_invoke")


# A child session that talks to a Mesh peer is given a router of its own by the
# factory the driver registers as the machine's host (SCE_MESH.md, Child Sessions
# in Mesh): what it sends is recorded under its invoke and start, and a step's
# `invoke` reaches it through that router, a grandchild's through the router its
# parent's router answered for it.
def test_a_child_is_given_a_mesh_router_of_its_own() -> None:
    replay("static_child_mesh")


# An `<invoke>` hands its child the values its `<param>`s and `namelist` name, as
# they stand when the invoke executes, after the entry actions, and once.
def test_an_invoke_tree_restores_between_its_steps() -> None:
    replay("static_invoke_tree")


def test_an_invoke_hands_its_child_its_values_once() -> None:
    replay("static_invoke_params")


# A string an `<invoke>` hands its child is held to the bound the child declared,
# in bytes: a value past it is left out and raises `error.execution`, and the
# child still starts.
def test_a_string_handed_to_a_child_is_held_to_its_bound_in_bytes() -> None:
    replay("static_invoke_string")


# A child's variable of `optional<T>` is handed the whole optional value its
# `<param>` computes (docs/adr/0055, step 10): a value of `T` present, `null` and
# an absent variable absent, and one past the bound the child declared left out.
def test_a_childs_optional_variable_is_handed_an_optional_value() -> None:
    replay("static_invoke_optional")


# Leaving the state that holds an `<invoke>` cancels the child, which then ends
# nothing and counts nothing.
def test_leaving_the_state_of_an_invoke_cancels_the_child() -> None:
    replay("static_invoke_abort")


# A `<history>` remembers what its parent held when it was left, and entering it
# brings that back: the shallow one the child that was active, the deep one the
# atomic states below, and a deep one of a `<parallel>` both regions at once. The
# scenario states the whole active configuration of each step.
def test_a_history_brings_back_what_its_parent_held() -> None:
    replay("static_history")


# Four delayed sends armed on entering a state are delivered when each is due, two
# due the same moment in the order they were sent, and the last takes the machine
# to its final state; the scenario's `advance_ms` steps move the engine's time on.
def test_timers_deliver_each_send_when_it_is_due() -> None:
    replay("static_timers")


# The same machine stopped: a `<cancel>` by the id of the longest send removes that
# one and no other, so the machine never ends.
def test_stopping_cancels_the_send_its_id_names_and_no_other() -> None:
    replay("static_timers_stop")


# The `idlocation` of a `<send>` names a string variable the machine writes the id
# it generates for the send to, which a later `<cancel sendidexpr>` names; the
# scenario's `advance_ms` steps move the engine's time on.
def test_a_send_hands_the_document_an_id_a_cancel_can_name() -> None:
    replay("static_send_idlocation")


# The `<content expr>` of a `<send>` names a record, which crosses as the pairs of
# its fields: a record variable and the payload of the event the transition is
# on, taken whole.
def test_a_send_carries_the_record_its_content_names() -> None:
    replay("static_send_content")


# The `namelist` of a `<send>` names variables the machine holds, each carried as
# the pair `<param name="x" expr="x"/>` it abbreviates, an enum value among them
# as the name its enum declares.
def test_a_send_carries_the_variables_its_namelist_names() -> None:
    replay("static_send_namelist")


# A `<send>` hands its event the pairs of its `<param>`s, each read from the
# machine's fields when the send runs; a pair whose value failed is left out, the
# message still goes, and the receiver finds the field missing.
def test_a_send_hands_its_event_the_pairs_of_its_params() -> None:
    replay("static_send_params")


# A string variable is held to the UTF-8 bytes it declares, not to the characters
# a Python string counts: an assignment past the bound writes nothing, raises
# error.execution and ends its block.
def test_a_string_is_held_to_its_bytes() -> None:
    replay("static_string_capacity")


# A bytes variable is held to the bytes it declares, as a string is to its UTF-8
# bytes: an assignment past the bound writes nothing, raises error.execution and
# ends its block. A scenario states a byte string as its byte-exact Latin-1 text.
def test_a_byte_string_is_held_to_its_bound() -> None:
    replay("static_bytes")


# A byte string crosses a `<param>` and a `<donedata>` as its byte-exact Latin-1 text:
# sent to itself and read back through the typed payload, and carried by the done
# event's data.
def test_a_byte_string_crosses_a_param_as_its_latin1_text() -> None:
    replay("static_bytes_wire")


# A top-level final hands its done event the pairs of its `<donedata>`, each read
# from the machine's fields when the state is entered; a pair whose value failed
# is left out and the others cross.
def test_a_final_hands_its_done_event_the_pairs_of_its_donedata() -> None:
    replay("static_donedata")


# A top-level final whose `<donedata>` names a record in its `<content expr>` hands
# its done event the pairs of the record's fields, read when the state is entered.
def test_a_final_hands_its_done_event_the_record_its_content_names() -> None:
    replay("static_donedata_record")


# A top-level final whose `<donedata>` is inline `<content>` hands its done event
# the text as the string it spells, with no script engine to read it as a number.
def test_a_final_hands_its_done_event_the_text_its_content_spells() -> None:
    replay("static_donedata_content")


# A top-level final whose `<donedata>` carries a `<content expr>` that names one
# value hands its done event that value as its whole data: a number as its
# digits, a string quoted, and one that cannot be computed as the empty string.
def test_a_final_hands_its_done_event_the_value_its_content_names() -> None:
    replay("static_donedata_content_value")
    replay("static_donedata_content_text")
    replay("static_donedata_content_lost")


# docs/adr/0055: a value that may be absent, held as `None` in a variable, a
# record's field and a list's record, carried as JSON `null` by a payload, a
# `<send>`'s params and a `<donedata>`, read only where a test proves it present,
# `??` computing its default only when the value is absent, and a delivery that
# lacks a first-revision key refused.
def test_a_value_that_may_be_absent_is_held_tested_and_carried_as_null() -> None:
    replay("static_optional")


# docs/adr/0055: a statechart hands algorithms values that may be absent and holds
# the ones they give back -- a list of records whose day may be absent, ordered,
# filtered and summed, and an optional day handed in and taken back, `None` among
# them.
def test_a_statechart_hands_an_algorithm_a_value_that_may_be_absent() -> None:
    replay("static_state_algorithm_optional")


# docs/adr/0055, section 6: a log replayed through a revision that widened a
# field. An entry written when the day was an int64 reads it present, one written
# after carries `null` for an absent day, a reminder the later revision added with
# no default reads absent where an entry lacks it, and an entry that lacks the day,
# a field of the first revision, is refused.
def test_an_older_entry_reads_a_widened_field_present_and_a_newer_null_absent() -> None:
    replay("static_optional_widened")


# docs/adr/0055, step 9: a record that may be absent, `None` until one is put into it,
# whole, from a payload, a record variable, a list's element and an algorithm's answer;
# its fields read as `pick.value.<field>` only where a test proves it present and
# written the same way, one after the other; made absent again by `null`; a list that
# holds three refusing a fourth item before anything after the append runs. A record put
# into it is the frozen record the machine holds, so the list's item is not changed by
# the write through `pick`.
def test_a_record_that_may_be_absent_is_held_read_and_written_where_proven() -> None:
    replay("static_optional_record")
