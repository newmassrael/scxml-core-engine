# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The run of one test case of the test profile (docs/SCE_TEST_PROFILE.md §2 to
§5, §12): its entities, its ports and timers, the round scheduler on the logical
clock, and the test case verdict.

A component's machine is generated from its ``test-component`` and reaches this
run only through its ``ComponentContext`` (§12.3): the snapshot its branches'
guards read, the statement each taken branch runs first, the statements of the
profile, and the doors of a send and of a PTC. The run knows nothing of the
engine or of the record types, so it is tested on its own. How the state is
shared and why it takes no lock is the package's documentation.

Cross-language siblings: ``sce_rust_runtime::test::TestCaseRun`` (Rust),
``SCE::Test::TestCaseRun`` (C++), ``com.sce.runtime.test.TestCaseRun``
(Kotlin), ``testprofile.TestCaseRun`` (Go).
"""

from __future__ import annotations

from contextlib import contextmanager
from dataclasses import dataclass, field
from enum import Enum
from typing import (
    Any,
    Callable,
    Dict,
    Iterator,
    List,
    Optional,
    Protocol,
    Set,
    Tuple,
    Type,
    TypeVar,
)

from ..saved_state import StateRefusal
from .module_parameters import ModuleParameters, copy_of
from .port import Duration, Instant, Message, Port
from .timer import Timer
from .verdict import Verdict, worse

V = TypeVar("V")
R = TypeVar("R")

#: The name of the MTC in a configuration; no PTC takes it (§11.6).
MTC_NAME = "mtc"


@dataclass(frozen=True)
class Endpoint:
    """A component port named in a configuration: the component's name (``mtc``
    or a PTC's id) and the port's id."""

    #: The component's name.
    component: str
    #: The port's id.
    port: str

    def names(self, component: str, port: str) -> bool:
        """Whether this is the port ``port`` of the component ``component``."""
        return self.component == component and self.port == port


@dataclass(frozen=True)
class Connection:
    """One ``<sce:connect>`` pair (-4 §9.10)."""

    a: Endpoint
    b: Endpoint


@dataclass(frozen=True)
class Mapping:
    """One ``<sce:map>`` entry: a component port and its system port
    (-4 §9.32)."""

    port: Endpoint
    system_port: str


@dataclass
class Configuration:
    """A ``test-configuration`` as the run reads it (§11.6).

    Generated from the configuration document. Each port takes part in one
    connection or one mapping (§2.2), which the build has checked.
    """

    #: The MTC's component type.
    mtc_type: str
    #: Each PTC's name and its component type.
    ptc_types: Dict[str, str] = field(default_factory=dict)
    #: The port ids each component type declares.
    ports_of_type: Dict[str, List[str]] = field(default_factory=dict)
    #: The timer ids each component type declares.
    timers_of_type: Dict[str, List[str]] = field(default_factory=dict)
    #: The ``<sce:connect>`` pairs.
    connections: List[Connection] = field(default_factory=list)
    #: The ``<sce:map>`` entries.
    mappings: List[Mapping] = field(default_factory=list)


class LogKind(Enum):
    """What the test log records of a run (§12.5); ``str`` of a kind is its name
    as the other runtimes name it."""

    #: The MTC created and started.
    START = "Start"
    #: A PTC created and started.
    CREATE = "Create"
    #: A component's send on a port.
    SEND = "Send"
    #: A receive branch's removal of the head of a port.
    RECEIVE = "Receive"
    #: A timeout branch's taking of a timer.
    TIMEOUT = "Timeout"
    #: A PTC that terminated (P10).
    DONE = "Done"
    #: A PTC removed.
    KILL = "Kill"
    #: A component's setting of its local verdict.
    VERDICT = "Verdict"
    #: A ``<log>``.
    LOG = "Log"
    #: A dynamic error that ended the run (P13).
    ERROR = "Error"

    def __str__(self) -> str:
        return self.value


@dataclass(frozen=True)
class LogRecord:
    """One line of the test log."""

    #: ``NOW`` when it was recorded.
    at: Instant
    #: The component it is of.
    component: str
    #: What it records.
    kind: LogKind
    #: The port and event, the timer, the verdict and reason, the text or the
    #: error, by kind; empty for a kind that has none.
    detail: str = ""


@dataclass
class TestCaseResult:
    """The outcome of one test case: its verdict, the instant it ended at, and
    its log."""

    # Not a class of tests, though its name begins as pytest's do.
    __test__ = False

    #: The test case verdict (P14).
    verdict: Verdict
    #: ``NOW`` when the run ended.
    ended_at: Instant
    #: The test log.
    log: List[LogRecord]


@dataclass(frozen=True)
class SystemInput:
    """One input of the SUT: the system port it arrives on and the message."""

    system_port: str
    message: Message


class SystemAdapter(Protocol):
    """The SUT's side of the mapped ports (§12.2).

    Shaped after TRI (ES 201 873-5): ``send`` is ``triSend``, and the inputs
    ``take_inputs_due`` returns are what ``triEnqueueMsg`` would enqueue. The
    adapter owns the codec and the transport; the run sees values only (ADR 0091
    C2). The instants are the logical clock's, so a recorded trace replays to
    the same run (§3.1).
    """

    def send(self, system_port: str, message: Message, now: Instant) -> None:
        """A component's send of ``message`` on a port mapped to
        ``system_port``."""

    def next_input(self) -> Optional[Instant]:
        """The instant of the earliest input not yet taken, or ``None`` when
        there is none."""

    def take_inputs_due(self, now: Instant) -> List[SystemInput]:
        """Every input due at or before ``now``, in arrival order."""


class ComponentMachine(Protocol):
    """A component's machine, as the run drives it (§12.3).

    Generated from a ``test-component``: the backend's statechart machine, with
    each branch a transition on the round's snapshot event.
    """

    def start(self, context: "ComponentContext") -> None:
        """Enter the initial configuration and complete the first macrostep. The
        machine keeps ``context``, and every call it makes, now and on each later
        delivery, goes to it."""

    def deliver_snapshot(self) -> None:
        """Deliver the round's snapshot event: the machine takes the first
        branch whose guard holds on the snapshot (P3) and completes its
        macrostep (P4), or takes none."""

    def terminated(self) -> bool:
        """Whether the machine has entered a top-level ``<final>`` (P10)."""


#: Builds the machine of a component type, or answers ``None`` for a type it
#: does not make, which ends the run with ``error`` where a component of that
#: type is created.
MachineFactory = Callable[[str], Optional[ComponentMachine]]


class _ComponentState:
    """What one component holds in the run."""

    __slots__ = ("ports", "timers", "done_snapshot", "verdict", "branch_taken")

    def __init__(self) -> None:
        self.ports: Dict[str, Port] = {}
        self.timers: Dict[str, Timer] = {}
        self.done_snapshot: Set[str] = set()
        self.verdict = Verdict.NONE
        self.branch_taken = False

    def port(self, port_id: str) -> Port:
        port = self.ports.get(port_id)
        if port is None:
            port = self.ports[port_id] = Port()
        return port

    def timer(self, timer_id: str) -> Timer:
        timer = self.timers.get(timer_id)
        if timer is None:
            timer = self.timers[timer_id] = Timer()
        return timer


class _Entity:
    """A component of the run's entity table."""

    __slots__ = ("serial", "state", "machine", "terminated")

    def __init__(self, serial: int, state: _ComponentState) -> None:
        # Which creation this is: a PTC removed and created again in one round
        # is a new component, which took no snapshot in it.
        self.serial = serial
        self.state = state
        # None while the run has it out to call it.
        self.machine: Optional[ComponentMachine] = None
        self.terminated = False


class _RunState:
    """The state of one run, shared by the run and every component's context."""

    def __init__(
        self,
        configuration: Configuration,
        factory: MachineFactory,
        adapter: SystemAdapter,
        guard: Optional[Duration],
        module_parameters: ModuleParameters,
    ) -> None:
        self.configuration = configuration
        self.factory = factory
        self.adapter = adapter
        self.guard = guard
        self.module_parameters = module_parameters
        self.now: Instant = 0
        self.created = 0
        # By name; a name's place here is not the creation order, `order` is.
        self.entities: Dict[str, _Entity] = {}
        self.order: List[str] = []
        self.done: Set[str] = set()
        self.removed_verdict = Verdict.NONE
        self.mtc_terminated = False
        # A dynamic error ended the run (P13).
        self.failed = False
        self.log: List[LogRecord] = []
        # A step of the run is inside the state (`_held`).
        self.held = False

    # -- The round (§3.2, §12.4) ----------------------------------------------

    def update_and_snapshot(self) -> List[Tuple[str, int]]:
        """Step 2, the update: inputs and timer expiries due at ``NOW``. Then
        step 3, the snapshot: every waiting component at once. Answers the
        components that took it, in creation order, each with its serial."""
        for system_input in self.adapter.take_inputs_due(self.now):
            self.deliver_input(system_input.system_port, system_input.message)
        for entity in self.entities.values():
            for timer in entity.state.timers.values():
                timer.update(self.now)
        took: List[Tuple[str, int]] = []
        for name in self.order:
            entity = self.entities.get(name)
            if entity is None or entity.terminated:
                continue
            for port in entity.state.ports.values():
                port.take_snapshot()
            for timer in entity.state.timers.values():
                timer.take_snapshot()
            entity.state.done_snapshot = set(self.done)
            took.append((name, entity.serial))
        return took

    def take_for_delivery(self, name: str, serial: int) -> Optional[ComponentMachine]:
        """The machine of a component that took this round's snapshot, out of
        its entity to be delivered; ``None`` for one removed or terminated
        earlier in the round, which does not select (§3.2 step 5)."""
        entity = self.entities.get(name)
        if entity is None or entity.serial != serial or entity.terminated:
            return None
        entity.state.branch_taken = False
        machine = entity.machine
        entity.machine = None
        return machine

    def branch_taken(self, name: str, serial: int) -> bool:
        entity = self.entities.get(name)
        return entity is not None and entity.serial == serial and entity.state.branch_taken

    def advance(self) -> None:
        """A round in which no branch fired: time advances to the next instant
        something can happen at (§3.3), or the test case can never end (P13)."""
        next_at = self.adapter.next_input()
        for entity in self.entities.values():
            if entity.terminated:
                continue
            for timer in entity.state.timers.values():
                expiry = timer.expiry()
                if expiry is not None and (next_at is None or expiry < next_at):
                    next_at = expiry
        guard = self.guard
        if guard is not None and (next_at is None or guard < next_at):
            next_at = guard
        if next_at is None:
            self.fail(MTC_NAME, "no branch can fire and no instant is pending: the test case can never end")
            return
        if next_at > self.now:
            self.now = next_at
        if guard is not None and self.now >= guard:
            self.fail(MTC_NAME, "the guard of the test case was reached")

    def settle(self, name: str, serial: int, machine: ComponentMachine, terminated: bool) -> None:
        """Put back a component's machine that has just run. If it terminated,
        P10 applies.

        The entity is still there: a component whose machine is running is not
        removed (kill), and nothing else removes one mid-round.
        """
        entity = self.entities.get(name)
        if entity is None or entity.serial != serial:
            raise RuntimeError(f"the component {name} was removed while its machine ran")
        entity.machine = machine
        if entity.terminated or not terminated:
            return
        entity.terminated = True
        if name == MTC_NAME:
            self.mtc_terminated = True
            return
        self.done.add(name)
        self.record(name, LogKind.DONE, "")

    def ended(self) -> bool:
        return self.mtc_terminated or self.failed

    def finish(self) -> TestCaseResult:
        """The end of the run, the MTC terminated or the run in error: every PTC
        still present is removed, in ascending order of name as the other
        runtimes remove them, and the test case verdict is the worst local
        verdict (P10, P14)."""
        for ptc in sorted(name for name in self.entities if name != MTC_NAME):
            self.remove_entity(ptc)
        verdict = self.removed_verdict
        mtc = self.entities.get(MTC_NAME)
        if mtc is not None:
            verdict = worse(verdict, mtc.state.verdict)
        if self.failed:
            verdict = Verdict.ERROR
        return TestCaseResult(verdict=verdict, ended_at=self.now, log=list(self.log))

    # -- Entities (§2.1, P9) --------------------------------------------------

    def enter(self, name: str, component_type: str) -> int:
        """Enter a new component of the type ``component_type`` into the table,
        with its ports and timers, and log its creation. Answers the
        component's serial."""
        self.created += 1
        state = _ComponentState()
        for port in self.configuration.ports_of_type.get(component_type, []):
            state.port(port)
        for timer in self.configuration.timers_of_type.get(component_type, []):
            state.timer(timer)
        self.entities[name] = _Entity(self.created, state)
        self.order.append(name)
        kind = LogKind.START if name == MTC_NAME else LogKind.CREATE
        self.record(name, kind, component_type)
        return self.created

    def ptc_to_create(self, creator: str, ptc: str) -> Optional[str]:
        """The type of the PTC ``ptc`` that ``creator`` creates, or ``None``,
        with the dynamic error, when it cannot be created."""
        component_type = self.configuration.ptc_types.get(ptc)
        if component_type is None:
            self.fail(creator, f"creates '{ptc}', which the configuration does not declare")
            return None
        if ptc in self.entities:
            self.fail(creator, f"creates '{ptc}', which is already present")
            return None
        self.done.discard(ptc)
        return component_type

    def kill(self, killer: str, ptc: str) -> None:
        """The kill of the PTC ``ptc`` by ``killer`` (P9). A component whose
        machine is running, up the call stack of this one, is not removed under
        it: that is a dynamic error."""
        entity = self.entities.get(ptc)
        if entity is not None and entity.machine is None:
            self.fail(killer, f"kills '{ptc}', whose machine is running")
            return
        self.remove_entity(ptc)

    def remove_entity(self, name: str) -> None:
        """Remove a PTC: its local verdict still counts (-4 §9.29a.2), and its
        ports are gone, so a send to one of them is an error."""
        entity = self.entities.pop(name, None)
        if entity is None:
            return
        self.removed_verdict = worse(self.removed_verdict, entity.state.verdict)
        self.record(name, LogKind.KILL, "")
        self.order = [present for present in self.order if present != name]
        self.done.discard(name)

    # -- Ports (§2.2, P6) -----------------------------------------------------

    def send(self, sender: str, port: str, message: Message) -> None:
        self.record(sender, LogKind.SEND, f"{port} {message.event}")
        for connection in self.configuration.connections:
            if connection.a.names(sender, port):
                peer = connection.b
            elif connection.b.names(sender, port):
                peer = connection.a
            else:
                continue
            entity = self.entities.get(peer.component)
            if entity is not None:
                entity.state.port(peer.port).enqueue(message)
                return
            self.fail(sender, f"sends on '{port}', whose peer '{peer.component}' is not present")
            return
        for mapping in self.configuration.mappings:
            if mapping.port.names(sender, port):
                self.adapter.send(mapping.system_port, message, self.now)
                return
        self.fail(sender, f"sends on '{port}', which is neither connected nor mapped")

    def deliver_input(self, system_port: str, message: Message) -> None:
        for mapping in self.configuration.mappings:
            if mapping.system_port != system_port:
                continue
            entity = self.entities.get(mapping.port.component)
            if entity is not None:
                entity.state.port(mapping.port.port).enqueue(message)
                return
            self.fail(
                mapping.port.component,
                f"an input on system port '{system_port}' found no component",
            )
            return
        self.fail(MTC_NAME, f"an input on system port '{system_port}', which no port maps")

    # -- A component's own state, through its context -------------------------

    def component(self, name: str, serial: int) -> _ComponentState:
        entity = self.entities.get(name)
        if entity is None or entity.serial != serial:
            raise RuntimeError(f"the component {name} was removed, and its context outlived it")
        return entity.state

    def receive(self, name: str, serial: int, port: str) -> Message:
        component = self.component(name, serial)
        component.branch_taken = True
        queue = component.ports.get(port)
        if queue is None:
            raise RuntimeError(f"{name} receives on '{port}', which its component type does not declare")
        message = queue.remove_head()
        self.record(name, LogKind.RECEIVE, f"{port} {message.event}")
        return message

    def timeout(self, name: str, serial: int, timer: str) -> None:
        component = self.component(name, serial)
        component.branch_taken = True
        binding = component.timers.get(timer)
        if binding is None:
            raise RuntimeError(f"{name} times out on '{timer}', which its component type does not declare")
        binding.timeout_taken()
        self.record(name, LogKind.TIMEOUT, timer)

    def setverdict(self, name: str, serial: int, verdict: Verdict, reason: str) -> None:
        component = self.component(name, serial)
        component.verdict = worse(component.verdict, verdict)
        detail = str(verdict)
        if reason:
            detail += " " + reason
        self.record(name, LogKind.VERDICT, detail)

    # -- The verdict and the log (§5, §12.5) ----------------------------------

    def fail(self, component: str, why: str) -> None:
        """A dynamic error (P13): the run ends with ``error``."""
        self.failed = True
        self.record(component, LogKind.ERROR, why)

    def record(self, component: str, kind: LogKind, detail: str) -> None:
        self.log.append(LogRecord(at=self.now, component=component, kind=kind, detail=detail))


@contextmanager
def _held(run: _RunState, component: str) -> Iterator[_RunState]:
    """The run's state, held for the length of the ``with`` block on behalf of
    ``component``. A step that finds it held already is re-entrant, and raises
    ``RuntimeError`` naming the component."""
    if run.held:
        raise RuntimeError(f"re-entrant access to the test run from {component}")
    run.held = True
    try:
        yield run
    finally:
        run.held = False


def _create_entity(run: _RunState, name: str, component_type: str) -> None:
    """Create the component ``name`` of the type ``component_type`` and start its
    machine, with the state let go while the factory and the machine run."""
    machine = run.factory(component_type)
    if machine is None:
        with _held(run, name) as state:
            state.fail(
                name,
                f"is of the component type '{component_type}', which the factory of machines "
                "does not make",
            )
        return
    with _held(run, name) as state:
        serial = state.enter(name, component_type)
    machine.start(ComponentContext(run, name, serial))
    terminated = machine.terminated()
    with _held(run, name) as state:
        state.settle(name, serial, machine, terminated)


def _round(run: _RunState) -> None:
    """One round (§3.2, §12.4)."""
    with _held(run, MTC_NAME) as state:
        took = state.update_and_snapshot()
    # Steps 4 and 5: one component at a time, in creation order, each selects on
    # its own snapshot and runs the macrostep of the branch it took. A component
    # removed or terminated earlier in the round does not select, and nothing
    # does once the MTC has terminated (§3.2).
    any_taken = False
    for name, serial in took:
        with _held(run, name) as state:
            if state.ended():
                break
            machine = state.take_for_delivery(name, serial)
        if machine is None:
            continue
        machine.deliver_snapshot()
        terminated = machine.terminated()
        with _held(run, name) as state:
            taken = state.branch_taken(name, serial)
            state.settle(name, serial, machine, terminated)
        any_taken = any_taken or taken
    with _held(run, MTC_NAME) as state:
        if not state.ended() and not any_taken:
            state.advance()


class ComponentContext:
    """What a component's machine reads and does through the run (§12.3).

    The machine keeps it from ``start`` on, and so may the host processors the
    machine registers for the doors of a send and of a PTC. Each call takes the
    run's state for its own length and lets it go before it returns, so no call
    holds it across a call into a machine. A call made while the state is held,
    from inside a snapshot reader say, is re-entrant and raises
    ``RuntimeError`` naming the component.
    """

    __slots__ = ("_run", "_name", "_serial")

    def __init__(self, run: _RunState, name: str, serial: int) -> None:
        self._run = run
        self._name = name
        self._serial = serial

    @property
    def name(self) -> str:
        """The component's name: ``mtc``, or the PTC's id."""
        return self._name

    def __repr__(self) -> str:
        return f"ComponentContext(name={self._name}, serial={self._serial}, ..)"

    # -- The snapshot, read by the branches' guards (P2) ----------------------

    def with_snapshot(self, port: str, read: Callable[[Optional[Message]], R]) -> R:
        """What ``read`` answers of the message the snapshot saw at the head of
        ``port``, or of ``None``. ``read`` runs while the run's state is held,
        so it must not call this context: such a call raises."""
        with _held(self._run, self._name) as state:
            queue = state.component(self._name, self._serial).ports.get(port)
            return read(queue.snapshot() if queue is not None else None)

    def snapshot_present(self, port: str) -> bool:
        """Whether the snapshot saw a message at the head of ``port``: the guard
        of a receive any branch."""
        return self.with_snapshot(port, lambda head: head is not None)

    def timed_out(self, timer: str) -> bool:
        """Whether the snapshot saw ``timer`` timed out."""
        with _held(self._run, self._name) as state:
            binding = state.component(self._name, self._serial).timers.get(timer)
            return binding is not None and binding.snapshot_timed_out

    def done(self, ptc: str) -> bool:
        """Whether the snapshot saw the PTC ``ptc`` in ``DONE``."""
        with _held(self._run, self._name) as state:
            return ptc in state.component(self._name, self._serial).done_snapshot

    # -- Taking a branch (P4 step 1), the first statement of each -------------

    def receive(self, port: str) -> Message:
        """A receive branch: remove the head of ``port``, which the branch's
        content reads as ``_event.data``.

        Raises ``RuntimeError`` when the component type declares no port
        ``port``, or its queue is empty: a receive branch is taken only on a
        snapshot that saw a message there.
        """
        with _held(self._run, self._name) as state:
            return state.receive(self._name, self._serial, port)

    def timeout(self, timer: str) -> None:
        """A timeout branch: ``timer`` becomes idle.

        Raises ``RuntimeError`` when the component type declares no timer
        ``timer``.
        """
        with _held(self._run, self._name) as state:
            state.timeout(self._name, self._serial, timer)

    def done_taken(self, ptc: str) -> None:
        """A done branch on the PTC ``ptc``: nothing is removed (-4 §9.16); the
        component took a branch in this round."""
        with _held(self._run, self._name) as state:
            state.component(self._name, self._serial).branch_taken = True

    # -- Statements of the profile --------------------------------------------

    def setverdict(self, verdict: Verdict, reason: str = "") -> None:
        """``<sce:setverdict>`` (P12): the local verdict becomes the worse of the
        two, and the verdict and its reason are logged."""
        with _held(self._run, self._name) as state:
            state.setverdict(self._name, self._serial, verdict, reason)

    def start_timer(self, timer: str, duration: Duration) -> None:
        """``<sce:start-timer>`` (P7), for ``duration`` microseconds from
        ``NOW``."""
        with _held(self._run, self._name) as state:
            state.component(self._name, self._serial).timer(timer).start(state.now, duration)

    def stop_timer(self, timer: str) -> None:
        """``<sce:stop-timer>`` (P7)."""
        with _held(self._run, self._name) as state:
            state.component(self._name, self._serial).timer(timer).stop()

    def log(self, text: str) -> None:
        """A ``<log>`` (P8): recorded in the test log with ``NOW`` and the
        component."""
        with _held(self._run, self._name) as state:
            state.record(self._name, LogKind.LOG, text)

    # -- Doors (§12.3) --------------------------------------------------------

    def send(self, port: str, message: Message) -> None:
        """A send on ``port`` (P6): to the queue of the connected port, or to the
        system adapter when the port is mapped."""
        with _held(self._run, self._name) as state:
            state.send(self._name, port, message)

    def create(self, ptc: str) -> None:
        """The start of the invocation that creates the PTC ``ptc`` (P9): the PTC
        is created and its machine started before this returns."""
        with _held(self._run, self._name) as state:
            component_type = state.ptc_to_create(self._name, ptc)
        if component_type is not None:
            _create_entity(self._run, ptc, component_type)

    def kill(self, ptc: str) -> None:
        """The cancel of that invocation: the PTC is removed (P9)."""
        with _held(self._run, self._name) as state:
            state.kill(self._name, ptc)

    # -- Module parameters (§11.5, §11.8) -------------------------------------

    def read_module_parameters(self, read: Callable[[ModuleParameters], None]) -> bool:
        """Hand ``read`` the module parameters of the run, which it reads into the
        variables the component declares as module parameters, before the
        component's machine starts.

        A value the run was not given (``ModuleParameters.of``), or one ``read``
        refuses as not of the type the component declares, raises a
        ``StateRefusal``, which is a dynamic error (P13): the run ends with
        ``error`` and ``False`` is answered, so the machine is not started on a
        value made up in its place. ``read`` runs with the run's state let go.
        """
        with _held(self._run, self._name) as state:
            given = state.module_parameters.copy()
        try:
            read(given)
        except StateRefusal as refused:
            with _held(self._run, self._name) as state:
                state.fail(self._name, refused.reason)
            return False
        return True

    def now(self) -> Instant:
        """``NOW``."""
        with _held(self._run, self._name) as state:
            return state.now


def receives(
    context: ComponentContext,
    port: str,
    event: str,
    value_type: Type[V],
    matches: Callable[[V], bool],
) -> bool:
    """Whether a receive branch with a template matches the snapshot of
    ``context`` (P2, docs/SCE_TEST_PROFILE.md §12.3).

    The snapshot saw a message at the head of ``port``, under ``event``, the
    event name of the template's record, whose value is a ``value_type``, and
    ``matches``, the template's ``matches`` bound to its arguments, holds of that
    value. A message of another record, or one whose value is of another type,
    does not match. ``matches`` runs while the run's state is held, as a reader
    of ``ComponentContext.with_snapshot`` does.
    """

    def read(head: Optional[Message]) -> bool:
        if head is None or head.event != event:
            return False
        value: Any = head.value
        return isinstance(value, value_type) and matches(value)

    return context.with_snapshot(port, read)


class TestCaseRun:
    """One run of one test case (§2.3, §3, §4.9 to §4.11, §5).

    The components run on the calling thread, in the order they were created;
    §3.2 shows that the order within a round does not change the result.

    ``guard`` is the test case's guard duration (P11), or ``None`` for none, and
    ``module_parameters`` the values of the module parameters the components
    read (§11.8): the suite's, with any the caller overrides. They are copied,
    so a later change to the caller's leaves the run as it was built; ``None``
    is no module parameter.
    """

    # Not a class of tests, though its name begins as pytest's do.
    __test__ = False

    __slots__ = ("_state", "_started")

    def __init__(
        self,
        configuration: Configuration,
        factory: MachineFactory,
        adapter: SystemAdapter,
        guard: Optional[Duration] = None,
        module_parameters: Optional[ModuleParameters] = None,
    ) -> None:
        self._state = _RunState(configuration, factory, adapter, guard, copy_of(module_parameters))
        # The Rust run is consumed by its `run`; a Python object cannot be, so
        # a second run of it, whose adapter has already been read, is refused.
        self._started = False

    def run(self) -> TestCaseResult:
        """Run the test case to its end and answer its verdict and log.

        Raises ``RuntimeError`` when this run has been run already.
        """
        if self._started:
            raise RuntimeError("a test case run is run once; build another to run the test case again")
        self._started = True
        _create_entity(self._state, MTC_NAME, self._state.configuration.mtc_type)
        while True:
            with _held(self._state, MTC_NAME) as state:
                if state.ended():
                    break
            _round(self._state)
        with _held(self._state, MTC_NAME) as state:
            return state.finish()
