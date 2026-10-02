"""Procedures that run inside the process that plays a design.

A consumer that drives one machine through many small steps cannot afford an
exchange per step: measured 2026-10-01, `compare` sent 247 requests for one
25-step drive and took 208 ms where the same drive took 12 ms in process, because
a request costs the two processes a wake-up each however little it asks. So the
loop runs here, next to the machine, and one exchange carries the whole drive and
brings back its result as plain data.

These are this package's own functions, not the design's: the design contributes
the module they are handed. A procedure takes the loaded module and plain
arguments, returns plain data (lists, tuples, strings, numbers), and does not
return an object that would have to stay behind as a reference.

`worker` looks a procedure up by name in `PROCEDURES`; the parent asks for one
with `RemoteModule.procedure(name, **arguments)`.
"""

from __future__ import annotations

from .lowering import MAX_TIME_STOPS, SendRecorder, Unplayable, advance, stopped_run


def trace(module, declared, steps, max_time_stops=MAX_TIME_STOPS) -> list:
    """Drive a fresh machine through `steps` and say what was seen after each.

    `steps` is a list of `("event", name)` and `("time", ms)`. The result starts
    with the observation of the initial configuration and has one more per step,
    each `(leaves, sent, finished)`: the active leaves, sorted; the names of the
    host-served sends since the last observation, in order; and whether the run
    has ended. A step that makes the machine raise ends the trace with
    `("raised", type name)`: what the draft did, which is a thing to compare.

    A drive the engine will not play on ends the trace with
    `("unplayable", why)` and no observation of it. That is not something the
    draft did and not something to compare: where such a machine stands is where
    the engine gave up, and two drafts that both gave up there are not alike.
    The ways are the ones a scenario is refused for, through the same functions
    (`lowering.stopped_run`): a macrostep that never reached a stable
    configuration (W3C SCXML 3.13), an error raised that no state answered (W3C
    SCXML 3.12.2), and time that would stop at more than `max_time_stops`
    instants.

    Time moves as `lowering.advance` moves it, deadline to deadline. This is
    `compare`'s drive, moved here whole. It used to run in the server."""
    recorder = SendRecorder()
    try:
        engine = module.create_engine()
        for processor in declared:
            engine.register_event_processor(processor, recorder)
        engine.initialize()
    except Exception as exc:  # noqa: BLE001 - a machine that cannot start is the draft's answer
        # What the engine said, as the draft's own answer and not a traceback out
        # of the comparison: a draft that starts a child session whose module is
        # not there (one document per draft) dies here, and used to take the whole
        # comparison with it.
        return [("unplayable", f"the engine could not start the draft: "
                               f"{type(exc).__name__}: {exc}, so nothing it does is its "
                               f"behaviour")]
    policy = engine.policy
    out: list = []

    def observe() -> None:
        leaves = tuple(sorted(policy.get_state_name(s) for s in engine.active_leaves))
        sent = tuple(getattr(r, "event_name", str(r)) for r in recorder.take())
        out.append((leaves, sent, bool(engine.reached_final)))

    def settled() -> bool:
        stopped = stopped_run(engine, policy)
        if stopped is not None:
            out.append(("unplayable", f"the engine {stopped}, so what the draft "
                                      f"does under these drives is not its behaviour"))
        return stopped is None

    if not settled():
        return out
    observe()
    for kind, value in steps:
        try:
            if kind == "event":
                event = policy.get_event_from_name(value)
                if event is not None:
                    engine.send_event(event)
            else:
                advance(engine, value, max_time_stops)
        except Unplayable as exc:
            out.append(("unplayable", f"a drive {exc}, so the draft cannot be played to "
                                      f"its end"))
            break
        except Exception as exc:  # noqa: BLE001 - a raise is what the draft did
            out.append(("raised", type(exc).__name__))
            break
        if not settled():
            break
        observe()
    return out


def advance_engine(module, engine, ms, max_time_stops=MAX_TIME_STOPS) -> str | None:
    """Move an engine this process holds forward by `ms`, deadline to deadline.

    The engine is a reference the caller holds, and the walk is one exchange:
    walking asks the engine for its next deadline and moves to it, and asking
    across the boundary for each of those would cost a request apiece. None when
    the time was moved. Otherwise the engine's words for why it was not
    (`lowering.Unplayable`), as data: the caller decides what that means for what
    it was judging, and an exception here would arrive as a stand-in class."""
    try:
        advance(engine, ms, max_time_stops)
    except Unplayable as exc:
        return str(exc)
    return None


def check_engine(module, engine) -> str | None:
    """Whether the run an engine this process holds has stopped being the
    design's behaviour (`lowering.stopped_run`): None while it has not, and
    otherwise the clause saying what the engine did, as data. One exchange for
    what is several questions of the engine, asked after every step."""
    return stopped_run(engine, engine.policy)


PROCEDURES = {"trace": trace, "advance": advance_engine, "check": check_engine}
