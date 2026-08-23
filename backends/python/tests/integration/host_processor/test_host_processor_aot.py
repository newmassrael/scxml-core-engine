# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2.5 — Python compile+run gate for a ``<send type>`` the HOST serves.

The clause makes the Event I/O Processor identifier extensible, so the set is
open by design. SCE implemented two of them and refused everything else with
``error.execution``; nothing let a platform widen the set. Rust, C++, C11 and
Go grew a registry first, and this backend refused the declaration by name
until it grew one of its own — the refusal being honest is exactly what made
the gap a coverage debt rather than a silent drop.

Fixture: ``sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml``,
the same document the other four channels drive, generated WITH
``--host-processor x-sce-host``. The declaration is load-bearing: without it
codegen emits the refusal and every case below would measure the refusal
instead of the feature.

The pair at the top is the whole contract:

* a registered handler receives the send and its reply arrives as an event —
  the feature working;
* the same machine with nothing registered raises ``error.execution`` — a
  wiring mistake staying visible instead of reading as success.

A gate holding only the first would pass on an engine that dispatched to
nothing and called it delivered, which is the silence being repaid.

Regeneration (after fixture or template edit):
  ``scripts/regen_host_processor_python.sh``
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

from sce_runtime import HostSendRequest, HostSendResponse  # noqa: E402

import statechart_host_processor_sm as _sm  # noqa: E402 — path inserted above

# The type the fixture was compiled for. ``scripts/regen_host_processor_python.sh``
# passes this same string to ``--host-processor``; a test registering a
# different one would measure nothing and pass, so the ``refused`` counter is
# asserted rather than the registration trusted.
DECLARED_TYPE = "x-sce-host"


def _started():
    engine = _sm.create_engine()
    return engine


def _counter(engine, name: str) -> int:
    """The fixture's ``<assign>``s are the only witness: every outcome leaves
    the machine in the same single state, so the configuration cannot tell
    them apart."""
    value = getattr(engine.policy, name)()
    assert value is not None, f"the fixture declares `{name}` and the machine could not read it"
    return value


def test_a_registered_handler_receives_the_send_and_its_reply_arrives() -> None:
    seen: List[HostSendRequest] = []

    engine = _started()

    def handler(request: HostSendRequest) -> List[HostSendResponse]:
        seen.append(request)
        # The request/reply shape: the reply becomes an event the document was
        # already waiting for, which is what lets a state DECLARE an act
        # instead of a host-side table performing it.
        return [HostSendResponse(event_name="turn.done")]

    engine.register_event_processor(DECLARED_TYPE, handler)
    engine.initialize()

    assert _counter(engine, "served") == 1, "the handler's reply never reached the document"
    assert _counter(engine, "refused") == 0, "a served send also raised error.execution"
    # The false-positive guard: an ordinary `<send>` in the same block must
    # still deliver. Without it a change that broke every send while leaving
    # the host branch intact would read as a pass.
    assert _counter(engine, "plain") == 1, "an ordinary <send> in the same block stopped delivering"

    assert len(seen) == 1, f"the handler ran {len(seen)} times"
    request = seen[0]
    assert request.processor_type == DECLARED_TYPE
    assert request.event_name == "watch.turn"
    # The payload the author wrote has to survive the crossing, or the document
    # can name an act but not parameterise it — which is most of the reason to
    # move an act into the document at all.
    assert request.params.get("within") == ["2500"], f"the <param> did not reach the handler: {request.params}"
    # W3C SCXML 6.2.4: correlating a reply, or honouring a `<cancel>`, needs
    # the send id — auto-generated here because the fixture declares none.
    assert request.send_id, "the request carried no send id"


def test_a_declared_type_with_no_handler_still_raises_error_execution() -> None:
    """The other half, and the one that keeps the repair honest: the build
    declared the type so codegen emitted a dispatch, but nothing was
    registered, so nobody performed the act."""
    engine = _started()
    engine.initialize()

    assert _counter(engine, "refused") == 1, "an unregistered processor was silently treated as served"
    assert _counter(engine, "served") == 0


def test_a_handler_that_answers_nothing_is_not_an_error() -> None:
    """A handler may perform work and have nothing to say. That is not an
    error, and reporting it as one would cost every fire-and-forget act a
    spurious ``error.execution``."""
    for answer in ([], None):
        engine = _started()
        ran = []

        def handler(request: HostSendRequest, _answer=answer) -> List[HostSendResponse]:
            ran.append(request)
            return _answer

        engine.register_event_processor(DECLARED_TYPE, handler)
        engine.initialize()

        assert ran, f"the handler never ran (answer={answer!r})"
        assert _counter(engine, "refused") == 0, (
            f"a silent handler was reported as an unsupported processor (answer={answer!r})"
        )
        assert _counter(engine, "served") == 0, (
            f"no reply was sent, so no reply event should have arrived (answer={answer!r})"
        )


def test_a_handler_registered_for_another_type_does_not_serve_this_one() -> None:
    """The registry is keyed. A lookup falling back to "any handler" would
    deliver a document's acts to a processor it never named."""
    engine = _started()
    engine.register_event_processor(
        "x-some-other-host",
        lambda request: [HostSendResponse(event_name="turn.done")],
    )
    engine.initialize()

    assert _counter(engine, "served") == 0, "a handler for a different type answered this send"
    assert _counter(engine, "refused") == 1


def test_a_reply_naming_an_undeclared_event_is_dropped() -> None:
    """A reply may name an event this machine does not declare — a host serving
    several documents, or one that has moved on since. That is dropped, exactly
    as any undeclared event reaching the queue is, and it is not an error."""
    engine = _started()
    engine.register_event_processor(
        DECLARED_TYPE,
        lambda request: [HostSendResponse(event_name="turn.never.declared")],
    )
    engine.initialize()

    assert _counter(engine, "served") == 0, "an undeclared reply name reached a transition"
    assert _counter(engine, "refused") == 0, "a dropped reply was reported as a refusal"
    assert _counter(engine, "plain") == 1, "the machine stopped running after an unknown reply name"


def test_the_registry_reports_what_it_holds() -> None:
    """The query the generated send site uses to tell "ran and said nothing"
    from "was never wired up". Both give the same answer from the dispatch, and
    only the second is an error, so the distinction cannot come from the return
    value alone."""
    engine = _started()
    assert not engine.has_event_processor(DECLARED_TYPE)
    engine.register_event_processor(DECLARED_TYPE, lambda request: [])
    assert engine.has_event_processor(DECLARED_TYPE)
    assert not engine.has_event_processor("x-never-registered")


def test_registering_a_type_twice_replaces() -> None:
    """Appending would leave dispatch depending on registration order, and a
    host re-registering means to change what serves the act — not to add a
    second server whose turn may never come."""
    engine = _started()
    superseded: List[HostSendRequest] = []
    current: List[HostSendRequest] = []

    engine.register_event_processor(DECLARED_TYPE, lambda request: superseded.append(request) or [])
    engine.register_event_processor(
        DECLARED_TYPE,
        lambda request: current.append(request) or [HostSendResponse(event_name="turn.done")],
    )
    engine.initialize()

    assert not superseded, "the superseded handler still served the act"
    assert len(current) == 1, "the current handler never ran"
    assert _counter(engine, "served") == 1
