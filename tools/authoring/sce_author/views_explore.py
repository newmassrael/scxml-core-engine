"""Walk the real engine for `views`, in a process of its own.

    python -m sce_author.views_explore < request.json > result.json

Two modes, one engine harness:

  play      play the steps of each scenario and report, after every step, where the machine
            stands and what it sent. The first observation is the machine as it starts.
  explore   find every situation the design can reach by the signals it takes (and time
            passing, when it has timers), write down what each signal does from each, and then
            check that table against the engine on random walks.

Why this is not `scenario_play`: that module reports what a scenario's expectations ask about
and its trace carries only an output's event name. A view has to say WHICH output (three
processors can all send an event called `on`), what the data held, and how far the next timer
is, so it needs a harness of its own. The rules about what a driver may not fill in are the
same ones, and the same checks are used: an error no state answered, a macrostep the engine
cut short and a chain of events that did not end each refuse the run.

⚠ The state of a machine is more than its configuration. Two runs in the same states can differ
in what the data holds and in how long a timer has left, and a lookup that merged them would
give a button the wrong answer. A situation is therefore the configuration, every data item the
generated module can read, whether the machine has finished and the time to the next timer. What
the harness cannot see (a history state's memory, a second timer's remaining time) is exactly what
the random walks at the end exist to catch: if the table disagrees with the engine on one step,
no table is offered.
"""

from __future__ import annotations

import json
import pathlib
import random
import sys
import time
import xml.etree.ElementTree as ET

from .errors import VerifyError
from .lowering import (Unplayable, advance, endless_event_chain, endless_macrostep, host_names, load,
                       unanswered_error)
from .scenario_play import _resolve_event
from .views import output_key


class _Refused(Exception):
    pass


def _sent(sink) -> list:
    """What the machine sent since the last call, each as `output_key` names it."""
    return [output_key(r.processor_type, r.event_name) for r in sink.take()]


class Harness:
    """One loaded design: a fresh engine on demand, an action applied to it, and a snapshot of it."""

    def __init__(self, request: dict) -> None:
        self.document = pathlib.Path(request["document"])
        self.processors = request["processors"]
        self.module = load(pathlib.Path(request["built"]), self.document)
        from sce_runtime.event import EventMetadata
        self.metadata = EventMetadata
        declared = [node.get("id") for node in ET.parse(self.document).getroot().iter()
                    if node.tag.rsplit("}", 1)[-1] == "data" and node.get("id")]
        try:
            table = host_names(self.module, "readers")
        except VerifyError:
            table = {}
        self.readers = {name: table[name] for name in declared if name in table}

    def fresh(self):
        from .lowering import SendRecorder
        sink = SendRecorder()
        engine = self.module.create_engine()
        for processor in self.processors:
            engine.register_event_processor(processor, sink)
        engine.initialize()
        return engine, sink

    def observe(self, engine, sink, outs: list) -> dict:
        policy = engine.policy
        data = {}
        for name, method in self.readers.items():
            try:
                value = getattr(policy, method)()
            except Exception:  # noqa: BLE001 - an unreadable item is left out, and the walks check the rest
                continue
            if isinstance(value, (bool, int, float, str)):
                data[name] = value
        return {"cfg": sorted(policy.get_state_name(s) for s in engine.active_configuration()),
                "data": data, "due": engine.time_until_next_scheduled_ms(),
                "finished": bool(engine.reached_final), "outs": outs}

    def apply(self, engine, sink, action: dict):
        """The outputs the action made, or None when the action is not possible here."""
        policy = engine.policy
        if action["kind"] == "time":
            due = engine.time_until_next_scheduled_ms()
            if due is None or not engine.is_running or engine.reached_final:
                return None
            self._advance(engine, due)
            moved = due
        elif action["kind"] == "advance":
            self._advance(engine, action["ms"])
            moved = action["ms"]
        else:
            event, declared = _resolve_event(policy, action["name"])
            if event is None:
                return None
            carried = action["name"] if declared != action["name"] else ""
            engine.send_event(event, self.metadata(data=action.get("payload") or "", name=carried))
            moved = 0
        self._refuse_if_unreliable(engine, policy)
        return _sent(sink), moved

    @staticmethod
    def _advance(engine, ms: int) -> None:
        try:
            advance(engine, ms)
        except Unplayable as exc:
            raise _Refused(f"time passing {exc}") from exc

    @staticmethod
    def _refuse_if_unreliable(engine, policy) -> None:
        for check in (endless_macrostep, endless_event_chain, unanswered_error):
            said = check(engine, policy)
            if said is not None:
                raise _Refused(f"the engine {said}, so where the machine stands is not the design's behaviour"
                               if check is unanswered_error else f"the engine stopped {said}")


def play(harness: Harness, scenarios: list) -> dict:
    runs = []
    for scenario in scenarios:
        try:
            engine, sink = harness.fresh()
            observations = [harness.observe(engine, sink, _sent(sink))]
            for step in scenario["steps"]:
                if "send" in step:
                    action = {"kind": "event", "name": step["send"], "payload": step.get("payload")}
                elif "advance_ms" in step:
                    action = {"kind": "advance", "ms": step["advance_ms"]}
                else:
                    observations.append({**observations[-1], "outs": []})
                    continue
                made = harness.apply(engine, sink, action)
                if made is None:
                    raise _Refused(f"step {step!r}: the design names no event that answers it")
                observations.append(harness.observe(engine, sink, made[0]))
            runs.append({"id": scenario["id"], "observations": observations})
        except _Refused as exc:
            runs.append({"id": scenario["id"], "refused": str(exc)})
        except Exception as exc:  # noqa: BLE001 - the engine failing is the report
            runs.append({"id": scenario["id"], "refused": f"the engine raised {type(exc).__name__}: {exc}"})
    return {"runs": runs}


def _key(snapshot: dict) -> str:
    return json.dumps({k: snapshot[k] for k in ("cfg", "data", "due", "finished")}, sort_keys=True)


def explore(harness: Harness, request: dict) -> dict:
    """Breadth first, so a situation is reached by the fewest signals and the walk stays shallow.

    ⚠ Three ways out, each a refusal and never a table cut short. A table with an edge missing would answer a
    button with "nothing happens", which is not what the engine does: too many situations (`max_nodes`), a
    situation deeper than any a person would press to (`max_depth`: a counter that is never reset keeps finding
    new ones, and the replay of ever longer paths is what used to take minutes), and time (`seconds`)."""
    alphabet = request["alphabet"]
    max_nodes, max_depth = request["max_nodes"], request["max_depth"]
    deadline = time.monotonic() + request["seconds"]
    engine, sink = harness.fresh()
    first = harness.observe(engine, sink, _sent(sink))
    nodes, index, paths, edges, depth = [first], {_key(first): 0}, [[]], [], [0]
    queue = 0
    while queue < len(nodes):
        if time.monotonic() > deadline:
            raise _Refused(f"exploring took more than {request['seconds']:g} seconds with {len(nodes)} situations "
                           f"found and more to try, so the design has too many of them for a table")
        row = []
        for action in alphabet:
            engine, sink = harness.fresh()
            sink.take()
            for earlier in paths[queue]:
                harness.apply(engine, sink, earlier)
            made = harness.apply(engine, sink, action)
            if made is None:
                row.append(None)
                continue
            snapshot = harness.observe(engine, sink, made[0])
            key = _key(snapshot)
            if key not in index:
                if len(nodes) >= max_nodes:
                    raise _Refused(f"more than {max_nodes} situations are reachable, so a table of them would be "
                                   f"larger than a page can carry")
                if depth[queue] + 1 > max_depth:
                    raise _Refused(f"the design keeps reaching new situations after {max_depth} signals in a row, so "
                                   f"its table never closes (a value that only grows, such as a counter nothing "
                                   f"resets, does this)")
                index[key] = len(nodes)
                nodes.append(snapshot)
                paths.append(paths[queue] + [action])
                depth.append(depth[queue] + 1)
            row.append([index[key], made[0], made[1]])
        edges.append(row)
        queue += 1
    steps, bad = _check(harness, alphabet, nodes, edges, request)
    if bad:
        raise _Refused(f"the table of what the engine does disagreed with the engine on {bad} of {steps} "
                       f"random steps, so it does not capture the design exactly (state the harness cannot read, "
                       f"such as a history state or a second timer)")
    for node in nodes:
        node.pop("outs", None)
    return {"nodes": nodes, "edges": edges, "alphabet": alphabet, "checked": {"steps": steps, "differences": bad}}


def _check(harness: Harness, alphabet: list, nodes: list, edges: list, request: dict) -> tuple:
    """Random walks on the engine and on the table must agree on every step."""
    rng = random.Random(request["seed"])
    steps = bad = 0
    for _ in range(request["walks"]):
        engine, sink = harness.fresh()
        sink.take()
        here = 0
        for _ in range(request["walk_length"]):
            options = [i for i, e in enumerate(edges[here]) if e]
            if not options:
                break
            choice = rng.choice(options)
            made = harness.apply(engine, sink, alphabet[choice])
            seen = harness.observe(engine, sink, made[0])
            edge = edges[here][choice]
            steps += 1
            if (made[0] != edge[1] or made[1] != edge[2] or sorted(seen["cfg"]) != nodes[edge[0]]["cfg"]
                    or seen["data"] != nodes[edge[0]]["data"] or seen["due"] != nodes[edge[0]]["due"]
                    or seen["finished"] != nodes[edge[0]]["finished"]):
                bad += 1
            here = edge[0]
    return steps, bad


def main() -> int:
    request = json.loads(sys.stdin.read())
    try:
        harness = Harness(request)
        result = play(harness, request["scenarios"]) if request["mode"] == "play" else explore(harness, request)
    except _Refused as exc:
        print(json.dumps({"refused": str(exc)}, ensure_ascii=False))
        return 0
    except VerifyError as exc:
        print(json.dumps({"refused": str(exc)}, ensure_ascii=False))
        return 0
    print(json.dumps(result, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
