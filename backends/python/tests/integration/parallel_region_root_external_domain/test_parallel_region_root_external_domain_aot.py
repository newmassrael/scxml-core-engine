# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML Appendix D: a ``<parallel>`` is not a transition domain — Python AOT.

``getTransitionDomain`` sends an external transition to ``findLCCA``, which
filters the proper ancestors with ``isCompoundStateOrScxmlElement``. A
``<parallel>`` is neither, so an external transition written on a REGION ROOT
has the document root as its domain: every region exits and re-enters, and a
sibling region's transition on the same event is preempted because the two exit
sets intersect and the sibling's source is not a descendant of this one's.

The engine answered the enclosing ``<parallel>`` here instead. Unlike the other
backends this one is not a codegen bug: the runtime's ``_find_lcca`` walks the
target's ancestors and answers the first one that also appears among the
source's, which is a plain lowest-common-ancestor whatever its kind. That is the
``findLCA`` the appendix distinguishes from ``findLCCA``, and the difference is
invisible until a ``<parallel>`` sits between the source and the first compound
``<state>`` above it — exactly a region root.

Measured 2026-08-25 on ``examples/ai_loop/ai_loop.scxml``: the Kotlin engine,
the only one implementing the filter, ended ``session.lost`` in
``[alive, restarting]`` where C++, Rust and Go ended in
``[rebuilding, restarting]``. That document was then repaired to say
``type="internal"``, which is what its three region-root transitions meant — and
that repair is why this fixture exists rather than the ai_loop suite: with the
document fixed, no committed document reaches the external form.

Sibling of the C++ drivers ``ParallelRegionRootExternalDomainTest.cpp`` and
``ParallelRegionRootExternalDomainAotTest.cpp``, of
``backends/rust/tests/tests/parallel_region_root_external_domain.rs`` and of the
Go driver next to them — all asking the same two clauses of the same document.

Fixture: ``tests/integration/parallel_region_root_external_domain.scxml`` — not
under ``integration_resources/``, because a stem there is a seven-channel
contract enforced by ``integration_stem_registration.rs`` and the C11 engine has
not been repaired yet.

Regeneration (after fixture or template edit):
  ``scripts/regen_parallel_region_root_external_domain_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import parallel_region_root_external_domain_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ParallelRegionRootExternalDomainState
_Event = _sm.ParallelRegionRootExternalDomainEvent


def _configuration(engine) -> str:
    """The whole configuration, sorted, rather than a handful of memberships.

    The way this defect presents is an ILLEGAL configuration — two children of
    the same compound region active at once — and every individual "is this
    state active" question answers yes to that.
    """
    return "[" + " ".join(sorted(str(s) for s in engine.active_configuration())) + "]"


def test_an_external_region_root_transition_exits_every_region() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    assert _configuration(engine) == "[alive drive run watch working]", (
        f"precondition: the fixture is supposed to start with both regions at their "
        f"defaults; it came up as {_configuration(engine)}, so nothing below is "
        "testing what it claims"
    )

    engine.send_event(_Event.RESTART)

    assert _configuration(engine) == "[alive drive restarting run watch]", (
        f"active {_configuration(engine)}. An external transition on a region root has "
        "the DOCUMENT ROOT as its domain (Appendix D findLCCA filters `<parallel>` out "
        "of the candidate ancestors), so every region exits and re-enters, `watch` is "
        "back at its default, and `watch`'s own transition on the same event is "
        "preempted as conflicting"
    )


def test_an_internal_region_root_transition_leaves_the_other_region() -> None:
    """The contrast, and the reason the ai_loop document is spelled the way it is.

    A test that only pinned the external case would pass just as well on an
    engine that sent EVERY region-root transition to the document root.
    """
    engine = _sm.create_engine()
    engine.initialize()

    engine.send_event(_Event.HOLD)

    assert _configuration(engine) == "[drive paused rebuilding run watch]", (
        f"active {_configuration(engine)}. An internal region-root transition has the "
        "region as its domain (source compound, target its descendant), so the sibling "
        "region never exits and answers the event itself"
    )
