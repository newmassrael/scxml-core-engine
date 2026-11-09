# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The test profile's runtime (docs/SCE_TEST_PROFILE.md §12.2), held to the rules
of §2 to §5 with components written here by hand, before any machine is
generated: each one keeps its context from ``start`` on, reads its snapshot and
calls its context as a generated machine does (§12.3). The runner of a suite is
held to P11 over test cases made of the same components.

Each test is the twin of the Rust case of the same module and name in
``backends/rust/runtime/tests/a_test_case_runs_as_the_test_profile_says.rs``: the
Rust module is a class named ``Test<Module>`` in Pascal case, and the case its
method ``test_<case>`` (``test_case_run_test::an_answered_request_passes_without_time_moving``
is ``TestTestCaseRunTest.test_an_answered_request_passes_without_time_moving``),
itself the twin of the C++ gtest in ``tests/test_runtime/TestCaseRunTest.cpp``,
of the Kotlin case in ``backends/kotlin/tests/src/test/kotlin/com/sce/runtime/test/``
and of the Go case in ``backends/go/runtime/testprofile/``, with the same
scenario, verdict, ``NOW`` and log. The cases of ``TestTheSharedHandle`` hold
what the Rust runtime's handle holds, which the Python context holds too:
re-entrant use is refused, and a context outlives no component it names.
``TestTheRunIsRunOnce`` holds what only the Go, Kotlin and Python runtimes have
to, since their run is not consumed by running it. The case of ``TestTheDoors``
reaches the run only through the two host processors a generated component
registers on its engine (§12.3), written here with the types the engine
registers them under. The cases of ``TestTheTestLog`` hold the run's test log
(§12.5) to the bytes the Rust module ``the_test_log`` holds its own to.

This directory is not a fixture stem: it holds the runtime to hand-worked
answers, which no document drives. ``scripts/gates/w3c-python.sh`` names it
explicitly for that reason.
"""
from __future__ import annotations

import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Dict, List, Optional, Tuple

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "runtime"))

from sce_runtime import (  # noqa: E402 -- path inserted above
    HostInvokeCancel,
    HostInvokeEvent,
    HostInvokeHandler,
    HostInvokeRequest,
    HostSendHandler,
    HostSendRequest,
)
from sce_runtime.saved_values import (  # noqa: E402
    saved_read_uint32,
    saved_write_string,
    saved_write_uint32,
)
from sce_runtime.testprofile import (  # noqa: E402
    ComponentContext,
    ComponentMachine,
    Configuration,
    Connection,
    Endpoint,
    Execute,
    Features,
    Instant,
    LogKind,
    Mapping,
    Message,
    ModuleParameters,
    ModuleParameterValue,
    OverrideRefusal,
    Port,
    Suite,
    SystemAdapter,
    SystemInput,
    TestCase,
    TestCaseResult,
    TestCaseRun,
    Timer,
    TimerStatus,
    Verdict,
    canonical,
    enabled,
    features_of,
    message_value,
    receives,
    run_suite,
    worse,
)
from sce_runtime.testprofile.canonical import CanonicalValue  # noqa: E402

SECOND = 1_000_000


class _Scripted:
    """A component written by hand: what it does when it starts, and what it
    does on each snapshot. Either may end it, as entering a top-level final
    does."""

    def __init__(self, on_start: "Step", on_snapshot: "Step") -> None:
        self._on_start = on_start
        self._on_snapshot = on_snapshot
        self._context: Optional[ComponentContext] = None
        self._ended = False

    def end(self) -> None:
        self._ended = True

    def start(self, context: ComponentContext) -> None:
        self._context = context
        self._on_start(context, self)

    def deliver_snapshot(self) -> None:
        assert self._context is not None, "a snapshot is delivered only to a started machine"
        self._on_snapshot(self._context, self)

    def terminated(self) -> bool:
        return self._ended


#: What a hand-written component does when it starts, or on a snapshot.
Step = Callable[[ComponentContext, _Scripted], None]


def idle() -> Step:
    return lambda context, self: None


#: A component's two steps: when it starts, and on each snapshot.
Steps = Tuple[Step, Step]


class _Machines:
    """The machines of a test, by component type."""

    def __init__(self) -> None:
        self._by_type: Dict[str, Steps] = {}

    def with_(self, component_type: str, steps: Steps) -> "_Machines":
        self._by_type[component_type] = steps
        return self

    def factory(self) -> Callable[[str], Optional[ComponentMachine]]:
        known = dict(self._by_type)

        def build(component_type: str) -> Optional[ComponentMachine]:
            steps = known.get(component_type)
            if steps is None:
                return None
            return _Scripted(*steps)

        return build


@dataclass(frozen=True)
class _Input:
    """One input of a trace, or one send the adapter was handed."""

    at: Instant
    system_port: str
    event: str


class _TraceAdapter:
    """Replays a trace and records what was sent to it."""

    def __init__(self, trace: Optional[List[_Input]] = None) -> None:
        self._trace = list(trace or [])
        self._next = 0
        self.sent: List[_Input] = []

    def send(self, system_port: str, message: Message, now: Instant) -> None:
        self.sent.append(_Input(now, system_port, message.event))

    def next_input(self) -> Optional[Instant]:
        if self._next < len(self._trace):
            return self._trace[self._next].at
        return None

    def take_inputs_due(self, now: Instant) -> List[SystemInput]:
        due: List[SystemInput] = []
        while self._next < len(self._trace) and self._trace[self._next].at <= now:
            given = self._trace[self._next]
            due.append(SystemInput(given.system_port, message(given.event)))
            self._next += 1
        return due


def no_fields(_: object) -> CanonicalValue:
    """The canonical form of a message whose record holds no field."""
    return {}


def message(event: str) -> Message:
    """A message of the record ``event`` that holds no field: ``{}`` in the test
    log, of the record type ``empty``."""
    return Message.of(event, (), "empty", no_fields)


def logged(result: TestCaseResult, component: str, kind: LogKind) -> bool:
    return any(r.component == component and r.kind is kind for r in result.log)


def by_type(*entries: Tuple[str, str]) -> Dict[str, List[str]]:
    """The ids each component type declares, from (type, id) pairs."""
    ids: Dict[str, List[str]] = {}
    for component_type, declared in entries:
        ids.setdefault(component_type, []).append(declared)
    return ids


def mtc_and_echo() -> Configuration:
    """An MTC and one PTC joined by one connection, the MTC's ``p`` to the PTC's
    ``q``."""
    return Configuration(
        mtc_type="Tester",
        ptc_types={"echo": "Echo"},
        ports_of_type=by_type(("Tester", "p"), ("Echo", "q")),
        timers_of_type=by_type(("Tester", "reply")),
        connections=[Connection(Endpoint("mtc", "p"), Endpoint("echo", "q"))],
    )


def mapped(component_type: str) -> Configuration:
    """An MTC alone, of the type ``component_type``, whose port ``p`` is mapped to
    the system port ``sut``."""
    return Configuration(
        mtc_type=component_type,
        ports_of_type=by_type((component_type, "p")),
        mappings=[Mapping(Endpoint("mtc", "p"), "sut")],
    )


def lone(component_type: str) -> Configuration:
    """An MTC alone, of the type ``component_type``, with no port and no timer."""
    return Configuration(mtc_type=component_type)


def replies_in_time() -> Step:
    def on_snapshot(context: ComponentContext, self: _Scripted) -> None:
        if context.snapshot_present("p"):
            assert context.receive("p").event == "pong"
            context.stop_timer("reply")
            context.setverdict(Verdict.PASS)
            self.end()
        elif context.timed_out("reply"):
            context.timeout("reply")
            context.setverdict(Verdict.FAIL, "no reply in time")
            self.end()

    return on_snapshot


def echo_asker() -> Steps:
    """An MTC that creates the echo, sends to it, and waits for the answer within
    a second: a receive branch passes, a timeout branch fails. Not named with a
    `test` prefix, which pytest would collect as a test."""

    def on_start(context: ComponentContext, self: _Scripted) -> None:
        context.create("echo")
        context.send("p", message("ping"))
        context.start_timer("reply", SECOND)

    return (on_start, replies_in_time())


def run_case(
    configuration: Configuration,
    machines: _Machines,
    adapter: SystemAdapter,
    guard: Optional[int] = None,
    parameters: Optional[ModuleParameters] = None,
) -> TestCaseResult:
    return TestCaseRun(configuration, machines.factory(), adapter, guard, parameters).run()


def saved(value: int) -> ModuleParameterValue:
    """A ``uint32`` module parameter's value, as a generated suite writes its
    default."""
    return saved_write_uint32(value)


@dataclass
class _Slot:
    """What a component read of a module parameter, if it read one."""

    value: Optional[int] = None


def reads_job(job: _Slot) -> Steps:
    """A component that reads the module parameters of its run into its variables
    before it starts (§11.5, §11.8), as a generated machine reads each through
    the reader of the type it declares."""

    def on_start(context: ComponentContext, self: _Scripted) -> None:
        def read(given: ModuleParameters) -> None:
            job.value = saved_read_uint32(given.of("job"), "job")

        if context.read_module_parameters(read):
            context.setverdict(Verdict.PASS)
            self.end()

    return (on_start, idle())


class TestTestVerdict:
    def test_only_ever_becomes_worse(self) -> None:
        assert worse(Verdict.PASS, Verdict.NONE) is Verdict.PASS
        assert worse(Verdict.PASS, Verdict.INCONCLUSIVE) is Verdict.INCONCLUSIVE
        assert worse(Verdict.FAIL, Verdict.PASS) is Verdict.FAIL
        assert worse(Verdict.FAIL, Verdict.ERROR) is Verdict.ERROR


class TestTestPort:
    def test_a_snapshot_sees_only_what_was_queued_when_it_was_taken(self) -> None:
        port = Port()
        port.take_snapshot()
        port.enqueue(message("late"))
        assert port.snapshot() is None, "a message that arrives after the snapshot is not in it"
        port.take_snapshot()
        assert port.snapshot() is not None, "the second snapshot saw the message"
        port.enqueue(message("behind"))
        head = port.snapshot()
        assert head is not None and head.event == "late", "a later message goes behind the head"
        assert port.remove_head().event == "late"
        assert len(port) == 1


class TestTestTimer:
    def test_times_out_at_its_instant_and_restarts_when_started_again(self) -> None:
        timer = Timer()
        timer.start(0, 10)
        timer.update(9)
        assert timer.status is TimerStatus.RUNNING
        timer.start(5, 10)
        assert timer.expiry() == 15, "a running timer that is started again restarts"
        timer.update(15)
        assert timer.status is TimerStatus.TIMEOUT
        timer.take_snapshot()
        assert timer.snapshot_timed_out, "the snapshot saw the timeout"
        timer.timeout_taken()
        assert timer.status is TimerStatus.IDLE
        timer.stop()
        assert timer.status is TimerStatus.IDLE, "stopping a timer that is not running does nothing"


class _FourInputs:
    """The SUT's four messages, all at ``NOW = 0``: a record of another type,
    another event, another value, and the one the template matches."""

    def __init__(self) -> None:
        self._given = False

    def send(self, system_port: str, message: Message, now: Instant) -> None:
        pass

    def next_input(self) -> Optional[Instant]:
        return None

    def take_inputs_due(self, now: Instant) -> List[SystemInput]:
        if now != 0 or self._given:
            return []
        self._given = True
        return [
            SystemInput("sut", Message.of("number", "7", "text", canonical.text)),
            SystemInput("sut", Message.of("other", 7, "number", canonical.integer)),
            SystemInput("sut", Message.of("number", 6, "number", canonical.integer)),
            SystemInput("sut", Message.of("number", 7, "number", canonical.integer)),
        ]


class TestTestCaseRunTest:
    def test_an_answered_request_passes_without_time_moving(self) -> None:
        """P6, P9, §3.2: a message sent in a round is taken in a later round, and
        the whole exchange happens at one instant, because a branch fired in each
        round."""

        def echo(context: ComponentContext, self: _Scripted) -> None:
            if context.snapshot_present("q"):
                assert context.receive("q").event == "ping"
                context.send("q", message("pong"))

        machines = _Machines().with_("Tester", echo_asker()).with_("Echo", (idle(), echo))
        result = run_case(mtc_and_echo(), machines, _TraceAdapter())
        assert result.verdict is Verdict.PASS, result.log
        assert result.ended_at == 0
        assert logged(result, "echo", LogKind.KILL), "the MTC's end removes the PTC (P10)"

    def test_an_unanswered_request_fails_when_its_timer_expires(self) -> None:
        """P7, §3.3: with nothing to receive, time advances to the timer's
        instant."""
        machines = _Machines().with_("Tester", echo_asker()).with_("Echo", (idle(), idle()))
        result = run_case(mtc_and_echo(), machines, _TraceAdapter())
        assert result.verdict is Verdict.FAIL, result.log
        assert result.ended_at == SECOND

    def test_a_test_case_that_can_never_end_is_an_error(self) -> None:
        """P13: no branch can fire and no instant is pending, so the test case
        can never end; the runtime ends it with error."""
        machines = _Machines().with_("Waiter", (idle(), idle()))
        result = run_case(mapped("Waiter"), machines, _TraceAdapter())
        assert result.verdict is Verdict.ERROR
        assert logged(result, "mtc", LogKind.ERROR), result.log

    def test_the_guard_ends_a_test_case_with_error(self) -> None:
        """P11: the guard instant ends a test case that has not ended, with
        error."""
        machines = _Machines().with_(
            "Waiter", (lambda context, self: context.setverdict(Verdict.PASS), idle())
        )
        result = run_case(mapped("Waiter"), machines, _TraceAdapter(), 5 * SECOND)
        assert result.verdict is Verdict.ERROR
        assert result.ended_at == 5 * SECOND

    def test_a_mapped_port_talks_to_the_system_adapter_on_the_logical_clock(self) -> None:
        """§2.2, §12.2: a mapped port's sends reach the adapter, and the trace's
        inputs reach the port at their instants."""
        configuration = mapped("Tester")
        configuration.timers_of_type = by_type(("Tester", "reply"))

        def on_start(context: ComponentContext, self: _Scripted) -> None:
            context.send("p", message("ping"))
            context.start_timer("reply", SECOND)

        machines = _Machines().with_("Tester", (on_start, replies_in_time()))
        adapter = _TraceAdapter([_Input(500, "sut", "pong")])
        result = run_case(configuration, machines, adapter)
        assert result.verdict is Verdict.PASS, result.log
        assert result.ended_at == 500
        assert adapter.sent == [_Input(0, "sut", "ping")]

    def test_a_removed_ptcs_verdict_still_counts(self) -> None:
        """P9, P14: a removed PTC's verdict still counts in the test case
        verdict."""

        def on_start(context: ComponentContext, self: _Scripted) -> None:
            context.create("echo")
            context.start_timer("reply", 10)

        def on_snapshot(context: ComponentContext, self: _Scripted) -> None:
            if context.timed_out("reply"):
                context.timeout("reply")
                context.kill("echo")
                context.setverdict(Verdict.PASS)
                self.end()

        machines = (
            _Machines()
            .with_("Tester", (on_start, on_snapshot))
            .with_("Echo", (lambda context, self: context.setverdict(Verdict.FAIL), idle()))
        )
        result = run_case(mtc_and_echo(), machines, _TraceAdapter())
        assert result.verdict is Verdict.FAIL, result.log

    def test_nothing_selects_after_the_mtc_terminates_in_the_round(self) -> None:
        """§3.2 step 4: components select one at a time in creation order, so when
        the MTC terminates in a round, a PTC that would have selected after it in
        that round does not."""
        configuration = mtc_and_echo()
        configuration.timers_of_type["Echo"] = ["late"]

        def tester_start(context: ComponentContext, self: _Scripted) -> None:
            context.create("echo")
            context.start_timer("reply", 10)

        def tester_snapshot(context: ComponentContext, self: _Scripted) -> None:
            if context.timed_out("reply"):
                context.timeout("reply")
                context.setverdict(Verdict.PASS)
                self.end()

        def echo_snapshot(context: ComponentContext, self: _Scripted) -> None:
            if context.timed_out("late"):
                context.timeout("late")
                context.setverdict(Verdict.FAIL)

        machines = (
            _Machines()
            .with_("Tester", (tester_start, tester_snapshot))
            .with_("Echo", (lambda context, self: context.start_timer("late", 10), echo_snapshot))
        )
        result = run_case(configuration, machines, _TraceAdapter())
        assert result.verdict is Verdict.PASS, result.log
        assert not logged(result, "echo", LogKind.TIMEOUT), "the echo selected after the MTC ended"

    def test_a_done_branch_sees_a_ptc_that_terminated(self) -> None:
        """P2, P10: a PTC that terminates is in DONE from the next snapshot on,
        and a done branch removes nothing."""

        def tester_snapshot(context: ComponentContext, self: _Scripted) -> None:
            if context.done("echo"):
                context.done_taken("echo")
                context.kill("echo")
                self.end()

        def echo_start(context: ComponentContext, self: _Scripted) -> None:
            context.setverdict(Verdict.PASS)
            self.end()

        machines = (
            _Machines()
            .with_("Tester", (lambda context, self: context.create("echo"), tester_snapshot))
            .with_("Echo", (echo_start, idle()))
        )
        result = run_case(mtc_and_echo(), machines, _TraceAdapter())
        assert result.verdict is Verdict.PASS, result.log
        assert logged(result, "echo", LogKind.DONE), result.log

    def test_a_receive_with_a_template_matches_only_its_records_value(self) -> None:
        """P2, §12.3: a receive branch with a template matches only a message of
        its record, under its record's event, whose value its ``matches`` holds;
        the guard reads the snapshot and removes nothing."""
        seen: List[str] = []

        def is_seven(n: int) -> bool:
            return n == 7

        def reader(context: ComponentContext, self: _Scripted) -> None:
            if receives(context, "p", "number", int, is_seven):
                seen.append("seven")
                assert receives(context, "p", "number", int, is_seven), (
                    "a guard that matched removed the message"
                )
                context.receive("p")
                self.end()
            elif context.snapshot_present("p"):
                head = context.with_snapshot("p", lambda head: head)
                assert head is not None
                seen.append(head.event)
                context.receive("p")

        machines = _Machines().with_("Reader", (idle(), reader))
        result = run_case(mapped("Reader"), machines, _FourInputs())
        assert seen == ["number", "other", "number", "seven"], (
            "a value of another type, another event and another value do not match"
        )
        assert result.verdict is Verdict.NONE

    def test_a_component_reads_the_module_parameter_its_run_is_given(self) -> None:
        """§11.8: the value the run is given is the one the component reads, and
        the component starts on it."""
        job = _Slot()
        machines = _Machines().with_("Reader", reads_job(job))
        parameters = ModuleParameters()
        parameters.set("job", saved(7))
        result = run_case(lone("Reader"), machines, _TraceAdapter(), None, parameters)
        assert job.value == 7
        assert result.verdict is Verdict.PASS, result.log

    def test_a_module_parameter_the_run_is_not_given_is_an_error(self) -> None:
        """P13: a module parameter the run has no value for is a dynamic error,
        named in the log, and the component goes no further than the read."""
        job = _Slot()
        machines = _Machines().with_("Reader", reads_job(job))
        result = run_case(lone("Reader"), machines, _TraceAdapter())
        assert result.verdict is Verdict.ERROR
        assert job.value is None
        last = result.log[-1]
        assert last.kind is LogKind.ERROR and "'job'" in last.detail, last

    def test_a_module_parameter_of_another_type_is_an_error(self) -> None:
        """P13: a value that is not of the type the component declares is refused
        as a restored value of another type is, and not converted."""
        job = _Slot()
        machines = _Machines().with_("Reader", reads_job(job))
        parameters = ModuleParameters()
        parameters.set("job", saved_write_string("seven"))
        result = run_case(lone("Reader"), machines, _TraceAdapter(), None, parameters)
        assert result.verdict is Verdict.ERROR
        assert job.value is None
        assert logged(result, "mtc", LogKind.ERROR), result.log

    def test_a_component_type_with_no_machine_is_an_error(self) -> None:
        """P13: a component type the factory of machines makes no machine for ends
        the run with error where a component of it is created."""
        result = TestCaseRun(lone("Ghost"), lambda component_type: None, _TraceAdapter()).run()
        assert result.verdict is Verdict.ERROR
        assert logged(result, "mtc", LogKind.ERROR), result.log
        assert not logged(result, "mtc", LogKind.START), "no component of the type was created"

    def test_a_send_to_an_absent_peer_is_an_error(self) -> None:
        """P6, P13: a send on a port whose peer is not present is a dynamic
        error."""
        machines = _Machines().with_(
            "Tester", (lambda context, self: context.send("p", message("ping")), idle())
        )
        result = run_case(mtc_and_echo(), machines, _TraceAdapter())
        assert result.verdict is Verdict.ERROR
        assert result.ended_at == 0


def ends_with(name: str, component_type: str, verdict: Verdict) -> TestCase:
    """A test case whose MTC, of type ``component_type``, sets ``verdict`` as it
    starts and ends there."""

    def on_start(context: ComponentContext, self: _Scripted) -> None:
        context.setverdict(verdict)
        self.end()

    machines = _Machines().with_(component_type, (on_start, idle()))
    return TestCase(name, lone(component_type), machines.factory())


@dataclass
class _Tracing:
    """The adapters of a suite's run, each a fresh trace adapter, recording the
    test case each was built for."""

    built: List[str] = field(default_factory=list)

    def __call__(self, test_case: str) -> _TraceAdapter:
        self.built.append(test_case)
        return _TraceAdapter()


def selects_x(world: Features) -> bool:
    return enabled(world, "x")


def selects_y(world: Features) -> bool:
    return enabled(world, "y")


class TestTestSuiteRun:
    def test_runs_the_test_cases_a_world_selects_in_the_suites_order(self) -> None:
        """P11: the runner runs the test cases a world selects, in the suite's
        order, and skips the others; a test case with no selection runs in every
        world."""
        suite = Suite(
            "ordered",
            executes=[
                Execute(ends_with("first", "First", Verdict.PASS)),
                Execute(ends_with("second", "Second", Verdict.FAIL), selection=selects_x),
                Execute(ends_with("third", "Third", Verdict.INCONCLUSIVE), selection=selects_y),
            ],
        )
        tracing = _Tracing()
        results = run_suite(suite, features_of("y"), tracing)
        assert [(r.test_case, r.result.verdict) for r in results] == [
            ("first", Verdict.PASS),
            ("third", Verdict.INCONCLUSIVE),
        ]
        assert tracing.built == ["first", "third"], "an adapter is built for each case run"

    def test_each_test_case_is_a_fresh_run_under_its_own_guard(self) -> None:
        """P11: each test case is a fresh run from NOW = 0 under its own guard."""
        waiting = _Machines().with_("Waiter", (idle(), idle()))
        stuck = TestCase("stuck", lone("Waiter"), waiting.factory())
        suite = Suite(
            "guarded",
            executes=[
                Execute(stuck, guard=5 * SECOND),
                Execute(ends_with("quick", "Quick", Verdict.PASS)),
            ],
        )
        results = run_suite(suite, features_of(), _Tracing())
        assert len(results) == 2
        first, second = results[0].result, results[1].result
        assert (first.verdict, first.ended_at) == (Verdict.ERROR, 5 * SECOND), "the guard ends the first"
        assert (second.verdict, second.ended_at) == (Verdict.PASS, 0), "the second starts again at NOW = 0"

    def test_an_override_replaces_the_default_of_a_module_parameter_the_suite_declares(self) -> None:
        """-1 §8.2.1: a value the caller gives a module parameter replaces the
        suite's default, and a parameter the suite does not declare cannot be
        given one."""
        job = _Slot()
        machines = _Machines().with_("Reader", reads_job(job))
        defaults = ModuleParameters()
        defaults.set("job", saved(7))
        suite = Suite(
            "parameters",
            module_parameters=defaults,
            executes=[Execute(TestCase("reads", lone("Reader"), machines.factory(), ["job"]))],
        )
        results = run_suite(suite, features_of(), _Tracing())
        assert [r.result.verdict for r in results] == [Verdict.PASS]
        assert job.value == 7, "the default"

        overrides = ModuleParameters()
        overrides.set("job", saved(9))
        overridden = run_suite(suite, features_of(), _Tracing(), overrides)
        assert len(overridden) == 1
        assert job.value == 9, "the override"

        stray = ModuleParameters()
        stray.set("limit", saved(1))
        with pytest.raises(OverrideRefusal) as refused:
            run_suite(suite, features_of(), _Tracing(), stray)
        assert "'limit'" in refused.value.reason

    def test_a_test_case_that_reads_a_module_parameter_with_no_value_is_an_error(self) -> None:
        """P13: a test case whose components read a module parameter the run has
        no value for ends with error before anything of it runs, its adapter
        included."""
        reads = ends_with("reads", "Reader", Verdict.PASS)
        reads.module_parameters = ["job"]
        suite = Suite("undeclared", executes=[Execute(reads)])
        tracing = _Tracing()
        results = run_suite(suite, features_of(), tracing)
        assert [r.result.verdict for r in results] == [Verdict.ERROR]
        log = results[0].result.log
        assert len(log) == 1 and "'job'" in log[0].detail, log
        assert tracing.built == [], "no adapter is built for a case that cannot start"

    def test_a_test_case_with_no_system_adapter_is_an_error(self) -> None:
        """P13: a test case the caller gives no system adapter ends with error."""
        suite = Suite("unadapted", executes=[Execute(ends_with("only", "Only", Verdict.PASS))])
        results = run_suite(suite, features_of(), lambda test_case: None)
        assert [r.result.verdict for r in results] == [Verdict.ERROR]


class TestTheSharedHandle:
    def test_a_call_into_the_context_from_a_snapshot_reader_panics(self) -> None:
        """A snapshot reader runs while the run's state is held, so a call it
        makes into its context is re-entrant: it raises, naming the component,
        and does not go on inside the state."""

        def on_start(context: ComponentContext, self: _Scripted) -> None:
            context.with_snapshot("p", lambda head: context.log("inside the reader"))

        machines = _Machines().with_("Reader", (on_start, idle()))
        with pytest.raises(RuntimeError) as raised:
            run_case(mapped("Reader"), machines, _TraceAdapter())
        assert str(raised.value) == "re-entrant access to the test run from mtc"

    def test_a_kill_of_a_component_whose_machine_runs_is_an_error(self) -> None:
        """P9, P13: a component whose machine is running is not removed under it.
        The PTC's start kills the MTC, whose own start is still running; that is
        a dynamic error, and the MTC stays."""

        def tester_start(context: ComponentContext, self: _Scripted) -> None:
            context.create("echo")
            context.setverdict(Verdict.PASS)

        machines = (
            _Machines()
            .with_("Tester", (tester_start, idle()))
            .with_("Echo", (lambda context, self: context.kill("mtc"), idle()))
        )
        result = run_case(mtc_and_echo(), machines, _TraceAdapter())
        assert result.verdict is Verdict.ERROR
        assert logged(result, "echo", LogKind.ERROR), result.log
        assert not logged(result, "mtc", LogKind.KILL), "the MTC was not removed"
        assert logged(result, "mtc", LogKind.VERDICT), "the MTC's start ran on after the refused kill"

    def test_a_context_does_not_outlive_its_component(self) -> None:
        """A context names one creation of its component: once the PTC is
        removed, a call through a context it handed out raises instead of
        reaching another component."""
        kept: List[ComponentContext] = []

        def tester_start(context: ComponentContext, self: _Scripted) -> None:
            context.create("echo")
            context.kill("echo")
            assert kept, "the echo kept its context"
            kept[0].setverdict(Verdict.FAIL, "after its removal")

        machines = (
            _Machines()
            .with_("Tester", (tester_start, idle()))
            .with_("Echo", (lambda context, self: kept.append(context), idle()))
        )
        with pytest.raises(RuntimeError) as raised:
            run_case(mtc_and_echo(), machines, _TraceAdapter())
        assert str(raised.value) == "the component echo was removed, and its context outlived it"


class TestTheRunIsRunOnce:
    def test_a_run_that_has_run_is_not_run_again(self) -> None:
        """A run has read its adapter and holds its components' end, so running
        it a second time is refused rather than run on that state."""

        def on_start(context: ComponentContext, self: _Scripted) -> None:
            context.setverdict(Verdict.PASS)
            self.end()

        machines = _Machines().with_("Tester", (on_start, idle()))
        run = TestCaseRun(lone("Tester"), machines.factory(), _TraceAdapter())
        assert run.run().verdict is Verdict.PASS
        with pytest.raises(RuntimeError) as raised:
            run.run()
        assert str(raised.value) == "a test case run is run once; build another to run the test case again"


#: The Event I/O Processor whose addresses are ports (docs/SCE_TEST_PROFILE.md
#: §11.1).
TEST_PORT = "http://sce.dev/ext/test-port"

#: The invocation of a PTC (docs/SCE_TEST_PROFILE.md §11.1).
TEST_COMPONENT = "http://sce.dev/ext/test-component"


def port_processor(context: ComponentContext) -> HostSendHandler:
    """The test-port processor, a send (P6): the request carries the template's
    value, which the run appends to the connected port, or hands the system
    adapter. Not named ``test_port_processor``, as the other runtimes name it,
    since pytest would collect a function of that name as a test."""

    def send(request: HostSendRequest) -> list:
        assert request.value is not None, "a test-port send carries its template's value"
        context.send(request.target, Message(event=request.event_name, test_value=request.value))
        return []

    return send


def component_invoker(context: ComponentContext) -> HostInvokeHandler:
    """A PTC (P9): the start creates it, the cancel removes it."""

    def invoke(event: HostInvokeEvent) -> None:
        if event.start is not None:
            context.create(event.start.invoke_id)
        if event.cancel is not None:
            context.kill(event.cancel.invoke_id)
        return None

    return invoke


@dataclass(frozen=True)
class _Ping:
    """The record a template of the port type reads."""

    n: int


def ping_canonical(value: _Ping) -> CanonicalValue:
    """A ``_Ping`` in the canonical form of the test log, as the unit of a
    template over its record writes it."""
    return canonical.record(("n", canonical.integer(value.n)))


class _DoorsTester:
    """An MTC wired as a generated one is: its start registers the two
    processors, each holding its context, and it reaches the doors of the run
    only through them. Entering its state invokes ``echo`` and sends it a ping; a
    timeout leaves the state, which cancels the invoke."""

    def __init__(self) -> None:
        self._invoker: Optional[HostInvokeHandler] = None
        self._context: Optional[ComponentContext] = None
        self._ended = False

    def start(self, context: ComponentContext) -> None:
        sends = port_processor(context)
        invokes = component_invoker(context)
        invokes(HostInvokeEvent(start=HostInvokeRequest(processor_type=TEST_COMPONENT, invoke_id="echo")))
        sends(
            HostSendRequest(
                processor_type=TEST_PORT,
                event_name="ping",
                target="p",
                value=message_value(_Ping(3), "ping", ping_canonical),
            )
        )
        context.start_timer("reply", 10)
        self._invoker = invokes
        self._context = context

    def deliver_snapshot(self) -> None:
        context = self._context
        assert context is not None and self._invoker is not None, (
            "a snapshot is delivered only to a started machine"
        )
        if not context.timed_out("reply"):
            return
        context.timeout("reply")
        self._invoker(HostInvokeEvent(cancel=HostInvokeCancel(processor_type=TEST_COMPONENT, invoke_id="echo")))
        context.setverdict(Verdict.PASS)
        self._ended = True

    def terminated(self) -> bool:
        return self._ended


def position(result: TestCaseResult, component: str, kind: LogKind) -> int:
    for at, record in enumerate(result.log):
        if record.component == component and record.kind is kind:
            return at
    raise AssertionError(f"no {kind} of {component} in {result.log}")


class TestTheDoors:
    def test_a_send_and_a_ptc_go_through_the_host_processors(self) -> None:
        """§12.3: the PTC the invoker's start creates receives the value the
        processor's send carried, by its type, and the invoker's cancel removes
        it at the timeout, before the MTC's verdict."""

        def echo(context: ComponentContext, self: _Scripted) -> None:
            if receives(context, "q", "ping", _Ping, lambda p: p.n == 3):
                context.receive("q")
                context.setverdict(Verdict.PASS, "the value arrived")
            elif context.snapshot_present("q"):
                context.receive("q")
                context.setverdict(Verdict.FAIL, "another value arrived")

        def factory(component_type: str) -> Optional[ComponentMachine]:
            if component_type == "Tester":
                return _DoorsTester()
            if component_type == "Echo":
                return _Scripted(idle(), echo)
            return None

        result = TestCaseRun(mtc_and_echo(), factory, _TraceAdapter()).run()
        assert result.verdict is Verdict.PASS, result.log
        created = position(result, "echo", LogKind.CREATE)
        sent = position(result, "mtc", LogKind.SEND)
        received = position(result, "echo", LogKind.RECEIVE)
        killed = position(result, "echo", LogKind.KILL)
        verdict = position(result, "mtc", LogKind.VERDICT)
        assert created < sent < received, result.log
        assert result.log[sent].detail == "p ping" and result.log[received].detail == "q ping", result.log
        assert result.log[killed].at == 10, "the cancel came at the timeout"
        assert killed < verdict, "the cancel removed the PTC, not the end"
        assert logged(result, "echo", LogKind.VERDICT), result.log


def pinging() -> Configuration:
    """An MTC and its echo, joined ``p`` to ``q``; the MTC has two timers."""
    return Configuration(
        mtc_type="Tester",
        ptc_types={"echo": "Echo"},
        ports_of_type=by_type(("Tester", "p"), ("Echo", "q")),
        timers_of_type=by_type(("Tester", "t"), ("Tester", "s")),
        connections=[Connection(Endpoint("mtc", "p"), Endpoint("echo", "q"))],
    )


def ping(n: int) -> Message:
    """The ping's value, a record of one number of the record type ``ping``, as
    the unit of a template over it writes it."""
    return Message.of("ping", _Ping(n), "ping", ping_canonical)


def pinging_machines() -> _Machines:
    """The MTC creates the echo, pings it, starts two timers and logs; at the
    first timer's timeout it stops the second, and the first again, which no
    longer runs, and passes. The echo takes the ping, passes with a reason, and
    terminates."""

    def tester_start(context: ComponentContext, self: _Scripted) -> None:
        context.create("echo")
        context.send("p", ping(3))
        context.start_timer("t", 10)
        context.start_timer("s", 20)
        context.log('asked "echo"')

    def tester_snapshot(context: ComponentContext, self: _Scripted) -> None:
        if context.timed_out("t"):
            context.timeout("t")
            context.stop_timer("s")
            context.stop_timer("t")
            context.setverdict(Verdict.PASS)
            self.end()

    def echo_snapshot(context: ComponentContext, self: _Scripted) -> None:
        if context.snapshot_present("q"):
            context.receive("q")
            context.setverdict(Verdict.PASS, "pinged")
            self.end()

    return (
        _Machines()
        .with_("Tester", (tester_start, tester_snapshot))
        .with_("Echo", (idle(), echo_snapshot))
    )


def logged_suite(test_case: TestCase) -> Suite:
    return Suite("logged_suite", executes=[Execute(test_case)])


class TestTheTestLog:
    """The test log (docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091, C3): each
    record written where the semantics places its event, in the order the rounds
    produce them, and the whole log in its one canonical text."""

    def test_a_run_writes_each_record_where_its_event_happens(self) -> None:
        """P4, P6, P7, P8, P9, P10, P12, P14: the whole log of a run through the
        suite's runner. The echo's work comes in the round after the MTC's
        start, at NOW = 0, the timeout at 10; the stop of a timer that no longer
        runs, and the removal of the terminated echo when the MTC ends, write no
        record."""
        suite = logged_suite(TestCase("pings_the_echo", pinging(), pinging_machines().factory()))
        results = run_suite(suite, features_of(), lambda test_case: _TraceAdapter())
        assert len(results) == 1
        assert results[0].canonical_log(suite.name) == "".join(
            [
                '{"records":[',
                '{"at":"0","kind":"test-case-started"},',
                '{"at":"0","component":"mtc","kind":"component-created","type":"Tester"},',
                '{"at":"0","component":"echo","kind":"component-created","type":"Echo"},',
                '{"at":"0","component":"mtc","kind":"message-sent","port":"p",',
                '"type":"ping","value":{"n":"3"}},',
                '{"at":"0","component":"mtc","duration_us":"10","kind":"timer-started",',
                '"timer":"t"},',
                '{"at":"0","component":"mtc","duration_us":"20","kind":"timer-started",',
                '"timer":"s"},',
                '{"at":"0","component":"mtc","kind":"log","text":"asked \\"echo\\""},',
                '{"at":"0","component":"echo","kind":"message-received","port":"q",',
                '"type":"ping","value":{"n":"3"}},',
                '{"at":"0","component":"echo","kind":"verdict-set","reason":"pinged",',
                '"verdict":"pass"},',
                '{"at":"0","component":"echo","kind":"component-done","verdict":"pass"},',
                '{"at":"10","component":"mtc","kind":"timer-timed-out","timer":"t"},',
                '{"at":"10","component":"mtc","kind":"timer-stopped","timer":"s"},',
                '{"at":"10","component":"mtc","kind":"verdict-set","verdict":"pass"},',
                '{"at":"10","component":"mtc","kind":"component-done","verdict":"pass"},',
                '{"at":"10","kind":"test-case-ended","verdict":"pass"}',
                '],"schema":"sce-test-log.v1","suite":"logged_suite",',
                '"test_case":"pings_the_echo","verdict":"pass"}',
            ]
        )

    def test_an_error_the_runtime_sets_is_logged_without_its_cause(self) -> None:
        """P13: an error the runtime sets is a verdict without the runtime's own
        reason, which another runtime would not write alike."""
        machines = _Machines().with_("Waiter", (idle(), idle()))
        result = run_case(lone("Waiter"), machines, _TraceAdapter(), 7)
        assert result.verdict is Verdict.ERROR
        assert result.canonical_log("logged_suite", "waits") == "".join(
            [
                '{"records":[',
                '{"at":"0","kind":"test-case-started"},',
                '{"at":"0","component":"mtc","kind":"component-created","type":"Waiter"},',
                '{"at":"7","component":"mtc","kind":"verdict-set","verdict":"error"},',
                '{"at":"7","kind":"test-case-ended","verdict":"error"}',
                '],"schema":"sce-test-log.v1","suite":"logged_suite",',
                '"test_case":"waits","verdict":"error"}',
            ]
        )

    def test_a_test_case_that_cannot_start_logs_its_start_and_end(self) -> None:
        """P13: a test case that cannot start, for want of an adapter, logs its
        start and its end alone, since no component was created."""
        suite = logged_suite(TestCase("unplayed", pinging(), pinging_machines().factory()))
        results = run_suite(suite, features_of(), lambda test_case: None)
        assert results[0].canonical_log(suite.name) == "".join(
            [
                '{"records":[',
                '{"at":"0","kind":"test-case-started"},',
                '{"at":"0","kind":"test-case-ended","verdict":"error"}',
                '],"schema":"sce-test-log.v1","suite":"logged_suite",',
                '"test_case":"unplayed","verdict":"error"}',
            ]
        )
