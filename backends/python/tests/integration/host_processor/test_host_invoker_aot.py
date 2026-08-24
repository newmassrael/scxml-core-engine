# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4.1 — Python compile+run gate for an ``<invoke type>`` the HOST runs.

The clause leaves the invokable set to the platform in the same words 6.2.5
uses for ``<send>``, so the set is open by design. SCE implemented the SCXML
processor and refused everything else with ``error.execution``. The send half of
that gap was repaid across six backends; this one stayed Rust-only, and the
generator refused ``--host-invoker`` for Python by name rather than emit a start
nothing could service.

The refusal was honest, which is what made it a coverage debt rather than a
silent drop. Now the Python runtime carries the registry
(``Engine.register_invoker``) and this file is the channel that says so.

An invoke is not a send: it has a LIFETIME. The scenarios below hold the
outcomes apart, because the configuration alone cannot:

* a registered invoker is STARTED with what the document wrote;
* leaving the state CANCELS it — the half no configuration assertion can see,
  because the machine looks correct whether or not the host was told to stop;
* a cancel is delivered once, and only for an invocation that started;
* a declared type with nothing registered raises ``error.execution``.

Fixture: ``sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml``,
the same document the Rust, C++ and Go channels drive, generated WITH
``--host-invoker x-sce-host``. The declaration is load-bearing: without it
codegen emits the refusal and every case below would measure the refusal
instead of the feature.

Regeneration (after fixture or template edit):
  ``scripts/regen_host_processor_python.sh``
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List, Optional

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

from sce_runtime import HostInvokeEvent, HostInvokeResponse  # noqa: E402

import statechart_host_invoker_sm as _sm  # noqa: E402 — path inserted above

# The type the fixture was compiled for.
# ``scripts/regen_host_processor_python.sh`` passes this same string to
# ``--host-invoker``; a test registering a different one would measure nothing
# and pass, so the ``refused`` counter is asserted rather than the registration
# trusted.
DECLARED_TYPE = "x-sce-host"

Event = _sm.StatechartHostInvokerEvent


def _counter(engine, name: str) -> int:
    """The fixture's ``<assign>``s are the only witness: several of these
    outcomes leave the machine in the same state, so the configuration cannot
    tell them apart."""
    value = getattr(engine.policy, name)()
    assert value is not None, f"the fixture declares `{name}` and the machine could not read it"
    return value


def _recording_invoker(log: List[str]):
    """A recording invoker. Answers a completion on start so the
    ``done.invoke`` path is exercised too, and records both arms so the ORDER
    is assertable."""

    def handler(ev: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        if ev.start is not None:
            within = ev.start.params.get("within", ["absent"])[0]
            log.append(
                f"START id={ev.start.invoke_id} type={ev.start.processor_type} "
                f"src={ev.start.src} within={within}"
            )
            return HostInvokeResponse(done_data="ok")
        if ev.cancel is not None:
            log.append(f"CANCEL id={ev.cancel.invoke_id}")
        return None

    return handler


def _started(with_invoker: bool = True):
    """A machine with the invoker each case decides on, already initialized.

    Registration happens BEFORE ``initialize()``: the fixture's invoke runs at
    the end of the entry macrostep, so an invoker registered afterwards would
    be measuring a run that had already refused."""
    engine = _sm.create_engine()
    log: List[str] = []
    if with_invoker:
        engine.register_invoker(DECLARED_TYPE, _recording_invoker(log))
    engine.initialize()
    return engine, log


def test_a_registered_invoker_is_started_with_what_the_document_wrote() -> None:
    engine, log = _started()

    assert _counter(engine, "started") == 1, "done.invoke never reached the document"
    assert _counter(engine, "refused") == 0, "a started invocation also raised error.execution"
    # The false-positive guard: ordinary entry content must still run. Without
    # it a change that broke the entry chain while leaving the invoke arm
    # working would read as a pass.
    assert _counter(engine, "entered") == 1, "the entry chain stopped running"

    assert len(log) == 1, f"invoker calls: {log}"
    # `src` and `<param>` are how W3C SCXML 6.4.1 lets the document say WHAT to
    # invoke and with what. A request carrying neither would let a document
    # name an invocation it cannot describe.
    assert log[0] == (
        f"START id=probe type={DECLARED_TYPE} src=pane://turn within=2500"
    ), f"the start request lost part of what the document wrote: {log[0]}"


def test_leaving_the_state_cancels_the_invocation() -> None:
    """The invocation ends with the state that started it. Without this the
    host is told to begin work and never told to stop — which no configuration
    assertion can detect, because the machine looks correct either way."""
    engine, log = _started()
    # `send_external` + a macrostep is this backend's delivery pair; there is
    # no single-call `process_event` here, and the sibling channels drive their
    # machines the same way.
    engine.send_external(Event.LEAVE)
    engine.advance_time(0)

    assert _counter(engine, "ended") == 1, "the machine never left the invoking state"
    assert log and log[-1] == "CANCEL id=probe", f"no cancel reached the invoker: {log}"


def test_cancel_is_not_delivered_for_an_invocation_that_never_started() -> None:
    """A cancel is delivered once, and only for an invocation that started.

    The engine, not the emitted code, owns that judgement: the exit chain calls
    ``cancel_host_invoke`` unconditionally, so if the engine did not track what
    started, a state that exits before its macrostep settles would have the
    host tearing down work it never began.

    Asserted at the engine surface rather than through the fixture, for the
    reason the Rust channel records: driving the machine cannot produce the
    "never started" case, because every host call that advances it runs a
    macrostep and the pending invoke executes at the end of that macrostep."""
    engine = _sm.create_engine()
    log: List[str] = []
    engine.register_invoker(DECLARED_TYPE, _recording_invoker(log))

    assert not engine.cancel_host_invoke(
        DECLARED_TYPE, "probe"
    ), "a cancel was reported for an invocation that never started"
    assert log == [], f"the invoker was called for an invocation that never started: {log}"

    # Now let one start, cancel it, and cancel again: the second call has
    # nothing left to do. A registry that answered twice would have the host
    # tear down the same work twice.
    engine.initialize()
    assert engine.cancel_host_invoke(
        DECLARED_TYPE, "probe"
    ), "a started invocation reported nothing to cancel"
    assert not engine.cancel_host_invoke(
        DECLARED_TYPE, "probe"
    ), "the same invocation was cancelled twice"
    cancels = [e for e in log if e.startswith("CANCEL")]
    assert len(cancels) == 1, f"cancel reached the invoker {len(cancels)} times: {log}"


def test_a_declared_type_with_no_invoker_still_raises_error_execution() -> None:
    """The other half. The build declared the type, so codegen emitted a start
    — but nothing was registered, so no process was run. Same event as an
    unsupported type, because from the document's side it is the same fact.

    This is the case that keeps the repair honest: without it the feature could
    start nothing and the document would proceed as though its process were
    running."""
    engine, _log = _started(with_invoker=False)

    assert _counter(engine, "refused") == 1, "an unregistered invoker was silently treated as started"
    assert _counter(engine, "started") == 0, "done.invoke arrived for an invocation nobody ran"


def test_an_invoker_registered_for_another_type_does_not_run_this_one() -> None:
    """Registering some other type does not run this one. The registry is
    keyed, and a lookup that fell back to "any invoker" would hand a document's
    process to one it never named."""
    engine = _sm.create_engine()
    log: List[str] = []
    engine.register_invoker("x-some-other-host", _recording_invoker(log))
    engine.initialize()

    assert _counter(engine, "started") == 0, "an invoker for a different type ran this one"
    assert _counter(engine, "refused") == 1, "the unregistered type was not reported"
    assert log == [], f"the other type's invoker was called: {log}"
