# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A generated machine held to the transition function generated beside it.

A document that declares ``sce:transition-function="emit"`` is generated with a
pure function of (state, event) beside its machine
(``docs/SCE_TRANSITION_FUNCTION.md``). The function is computed at build time by
the selection rule the engine follows at run time, so the two are one fact only
if every pair agrees, and this is what asks: a fresh machine per pair, driven to
the state by the shortest event path the function itself gives, then sent the
event. What the engine reaches, and whether it took the event or discarded it,
must be what the function says.

The drivers of the integration stems that declare a function share it, so the
two stems ask the same question the same way.
"""

from __future__ import annotations

from types import ModuleType
from typing import Any, Callable, List, Optional, Set


def shortest_path(function: ModuleType, target: Any) -> Optional[List[Any]]:
    """The shortest sequence of events that takes ``function`` from its initial
    state to ``target``, read from the function alone; None when no sequence
    does."""
    frontier = [(function.INITIAL, [])]
    seen = {function.INITIAL}
    index = 0
    while index < len(frontier):
        state, path = frontier[index]
        index += 1
        if state is target:
            return path
        for event in function.ALL_EVENTS:
            to = function.transition(state, event)
            if to is not None and to not in seen:
                seen.add(to)
                frontier.append((to, path + [event]))
    return None


def expect_every_pair_agrees(function: ModuleType, create_engine: Callable[[], Any]) -> int:
    """Drive a machine ``create_engine`` builds to every state of ``function``,
    send it every event there, and assert that the configuration it reaches,
    and whether it took the event or discarded it, is what the function says.

    The configuration is compared as a set of ids: a state's ancestors are fixed
    by the document, so the set says what the ordered list says. Returns how
    many pairs it checked, which a caller holds to the number of states times
    the number of events.
    """

    def fresh() -> Any:
        engine = create_engine()
        engine.initialize()
        return engine

    def send(engine: Any, event: Any) -> None:
        member = engine.policy.get_event_from_name(event.value)
        if member is not None:
            engine.send_event(member)

    def active(engine: Any) -> Set[str]:
        return {str(state) for state in engine.active_configuration()}

    def terminal(engine: Any) -> Optional[str]:
        state = engine.terminal_state
        return None if state is None else str(state)

    def configuration(state: Any) -> Set[str]:
        return set(function.configuration(state))

    assert active(fresh()) == configuration(function.INITIAL), (
        "the machine does not start where the function does"
    )
    pairs = 0
    for state in function.ALL_STATES:
        path = shortest_path(function, state)
        assert path is not None, f"{state.value} is not reached from the initial state"
        for event in function.ALL_EVENTS:
            pair = f"({state.value}, {event.value})"
            engine = fresh()
            for taken in path:
                send(engine, taken)
            pairs += 1
            if function.is_final(state):
                assert terminal(engine) == state.value, f"{pair}: the path does not end the machine there"
                continue
            assert active(engine) == configuration(state), f"{pair}: the path leaves the machine elsewhere"
            discarded = engine.discarded_external_events()
            send(engine, event)
            to = function.transition(state, event)
            if to is None:
                assert engine.discarded_external_events() == discarded + 1, (
                    f"{pair}: the function refuses it and the machine took it"
                )
                assert active(engine) == configuration(state), (
                    f"{pair}: the machine moved on an event it discarded"
                )
            else:
                assert engine.discarded_external_events() == discarded, (
                    f"{pair}: the function takes it and the machine discarded it"
                )
                if function.is_final(to):
                    assert terminal(engine) == to.value, (
                        f"{pair}: the machine did not end where the function says"
                    )
                else:
                    assert active(engine) == configuration(to), (
                        f"{pair}: the machine is not where the function says"
                    )
    return pairs
