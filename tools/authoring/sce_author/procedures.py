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

from .lowering import SendRecorder


def trace(module, declared, steps) -> list:
    """Drive a fresh machine through `steps` and say what was seen after each.

    `steps` is a list of `("event", name)` and `("time", ms)`. The result starts
    with the observation of the initial configuration and has one more per step,
    each `(leaves, sent, finished)`: the active leaves, sorted; the names of the
    host-served sends since the last observation, in order; and whether the run
    has ended. A step that makes the machine raise ends the trace with
    `("raised", type name)`: what the draft did, which is a thing to compare.

    This is `compare`'s drive, moved here whole. It used to run in the server."""
    recorder = SendRecorder()
    engine = module.create_engine()
    for processor in declared:
        engine.register_event_processor(processor, recorder)
    engine.initialize()
    policy = engine.policy
    out: list = []

    def observe() -> None:
        leaves = tuple(sorted(policy.get_state_name(s) for s in engine.active_leaves))
        sent = tuple(getattr(r, "event_name", str(r)) for r in recorder.take())
        out.append((leaves, sent, bool(engine.reached_final)))

    observe()
    for kind, value in steps:
        try:
            if kind == "event":
                event = policy.get_event_from_name(value)
                if event is not None:
                    engine.send_event(event)
            else:
                engine.advance_time(value)
        except Exception as exc:  # noqa: BLE001 - a raise is what the draft did
            out.append(("raised", type(exc).__name__))
            break
        observe()
    return out


PROCEDURES = {"trace": trace}
