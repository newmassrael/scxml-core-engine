"""Run a scenario set against one design on the Python lowering, and write down
what was seen.

The product owns the examples (`sce-codegen scenarios`) and the verdicts
(`sce-codegen judge-scenarios`). What sits between them is a driver: a program
that stands a machine up, plays each scenario into it and records what came
out. This is one such driver, over the same Python lowering `verify` and
`compare` drive, and what it writes is an observation trace
(`schemas/sce-observation-trace.v1.schema.json`) naming itself `Python
lowering`, because a verdict is about an engine and the examples could pass on
one runtime and fail on another.

⚠ A design is code, and it is never run in the process that serves the client.
This module generates the design (the product's generator, a program this tree
trusts) and then PLAYS each scenario in a child process of its own
(`scenario_play`), supervised from outside by `process.run_isolated`: the
kernel's limits on processor time, memory, output and open files, and a clock
the child cannot reach. Measured 2026-10-01, an endless `<script>` loop ignored
a timer set in its own process and a design that re-sends itself at zero delay
never returned from `initialize()`; both used to keep the whole tool call from
ever answering. Now each costs exactly the one example it was playing, and the
rest keep their verdicts.

What this module will say about a stopped child is deliberately modest. A child
stopped for time, memory, output or a crash is refused with `cause:
environment`: another machine may play the same example to its end, and a
verdict that read it as a defect of the design would be a statement about load.
Whatever the design itself did that made an example unplayable (an error no
state answered, an open route, a macrostep the engine cut short, a design that
would not start) is refused by the child with `cause: design`. The trace also
repeats the limits and the isolation the run really had, so a verdict states
what it was made under; see `process.isolation_level` for what that means on
this host.

The scenario itself is played by `scenario_play`, which holds every rule about
what a driver may and may not fill in; this module holds the set, the
generation, the children, the trace and the owner's answer.
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import sys
from concurrent.futures import ThreadPoolExecutor

from . import process
# How many instants of virtual time one `advance_ms` step may be cut into. It is
# `lowering`'s because a comparison walks time by the same rule under the same
# ceiling; the name stays here so a caller can still play under another.
from .lowering import MAX_TIME_STOPS
from .verify import (VerifyError, _default_codegen, _scratch, generate, generate_companions,
                     scenario_judgement, scenario_set_reading)

ENGINE_NAME = "Python lowering"
RECORD = "sce-observation-trace"
VERSION = 1

#: What each child may use. Replaced as a whole to play under other bounds.
LIMITS = process.Limits()

#: How many children play at once. Each is independent, so the order of the
#: runs in the trace is the set's order whatever finishes first.
MAX_PARALLEL = 4

#: How many open decisions a refusal quotes. The first few name the cause; a
#: design with dozens would bury the sentence they sit in.
OPEN_DECISIONS_QUOTED = 4

#: The directory the child imports this package from.
_PACKAGE_ROOT = str(pathlib.Path(__file__).resolve().parents[1])


class ScenarioDriverError(VerifyError):
    """The scenario set cannot be driven at all. Never a verdict."""


def _open_decisions(manifest: dict) -> str:
    """The design's open decisions, as the generator reported them, in the
    words the author wrote for each; empty when there are none.

    Each distinct sentence once: a decision that blocks several sends is
    reported at every one of them, and the same words three times say nothing
    the first did not."""
    found = list(dict.fromkeys(
        f"{record.get('id')}: {str(record.get('reason')).rstrip('.')}"
        for record in manifest.get("unresolved") or []
        if record.get("node_type") != "kind-basis"))
    if not found:
        return ""
    quoted = "; ".join(found[:OPEN_DECISIONS_QUOTED])
    more = len(found) - OPEN_DECISIONS_QUOTED
    return (f" The design holds {len(found)} open decision(s): {quoted}"
            + (f"; and {more} more." if more > 0 else "."))


def _refused(scenario_id: str, why: str, cause: str | None) -> dict:
    """A run the driver did not make. `cause` is left out when the driver does
    not know it: absent is not `design`."""
    refused = {"why": why}
    if cause:
        refused["cause"] = cause
    return {"scenario": scenario_id, "refused": refused}


class _Built:
    """One design, generated to Python once. Nothing here loads it: the engine
    and the Lua machine are the children's."""

    def __init__(self, document: pathlib.Path, codegen: pathlib.Path,
                 into: pathlib.Path, serves: tuple, others: tuple = ()) -> None:
        self.document = document
        self.into = into
        self.refusal = ""
        self.build = generate(document, codegen, into, serves=serves)
        if self.build.refusal:
            self.refusal = self.build.refusal
            return
        kind = (self.build.manifest.get("document_kind") or {}).get("name")
        if kind != "statechart":
            self.refusal = (f"{document.name} is a {kind or 'document of no known kind'}, and a "
                            f"scenario drives a statechart")
            return
        # The rest of the design, built beside it: a child session the statechart
        # starts imports its module by name when it starts it.
        self.refusal = generate_companions(others, codegen, into, serves=serves)

    @property
    def generator(self) -> str | None:
        return (self.build.manifest or {}).get("generator")

    def request_for(self, scenario: dict, routes: dict, data: list) -> dict:
        """What a child is told: everything it needs and nothing it could use to
        reach beyond the example it plays."""
        return {
            "built": str(self.into),
            "document": str(self.document),
            "declared": list(self.build.declared),
            "routes": {name: list(route) for name, route in routes.items()},
            "data": data,
            "scenario": scenario,
            "opened": _open_decisions(self.build.manifest),
            "manifest_unreadable": self.build.manifest.get("unreadable_variables") or [],
            "max_time_stops": MAX_TIME_STOPS,
        }


def _in_a_child(request: dict, limits: process.Limits) -> dict:
    """The result of playing one scenario in a child of its own."""
    scenario_id = request["scenario"]["id"]
    outcome = process.run_isolated(
        [sys.executable, "-m", "sce_author.scenario_play"], limits=limits,
        stdin_text=json.dumps(request), env={"PYTHONPATH": _PACKAGE_ROOT})
    if outcome.stopped_by is not None or outcome.returncode != 0:
        # The machine stopped it, or it died: another machine may play it to
        # its end, so the verdict says nothing about the design.
        return {"run": _refused(
            scenario_id, f"the process that played it did not finish: {outcome.said()}",
            "environment"), "unreadable": {}, "data_unavailable": ""}
    lines = outcome.stdout.strip().splitlines()
    try:
        return json.loads(lines[-1])
    except (IndexError, ValueError):
        # A child that ended cleanly and said nothing readable is the driver's
        # own defect. It is not the design's and it is not the machine's.
        return {"run": _refused(scenario_id, "the process that played it printed no result",
                                None), "unreadable": {}, "data_unavailable": ""}


def drive(scenario_set: pathlib.Path, document: pathlib.Path,
          codegen: pathlib.Path | None = None, *, limits: process.Limits | None = None,
          others: tuple = ()) -> dict:
    """The observation trace of every runnable scenario in the set, played
    against the design `document` on the Python lowering.

    `others` are the rest of the design: the documents the statechart uses, a
    child session it starts among them. They are built beside it.

    The set is read for what a driver needs and no more; whether it is a
    usable set is `sce-codegen scenarios`' answer, and `judge-scenarios`
    judges nothing from one that is not."""
    scenario_set, document = pathlib.Path(scenario_set), pathlib.Path(document)
    limits = limits or LIMITS
    raw = scenario_set.read_bytes()
    try:
        spec = json.loads(raw)
        interface = spec["interface"]
        scenarios = [s for s in spec["scenarios"] if s.get("status", "runnable") == "runnable"]
        outputs = interface.get("outputs") or []
    except (ValueError, KeyError, AttributeError, TypeError) as exc:
        raise ScenarioDriverError(f"{scenario_set}: not a scenario set a driver can read "
                                  f"({type(exc).__name__}: {exc})") from exc
    routes = {output["name"]: (output["via"]["type"], output["via"]["target"])
              for output in outputs if output.get("via")}
    serves = tuple(dict.fromkeys(route[0] for route in routes.values()))
    data = list(interface.get("data") or [])
    codegen = pathlib.Path(codegen) if codegen else _default_codegen()
    with _scratch() as scratch:
        built = _Built(document, codegen, scratch / "design", serves,
                       tuple(pathlib.Path(other) for other in others))
        if built.refusal:
            # The design could not be built, or is not a statechart: the same
            # on every machine, and no child has anything to play into.
            results = [{"run": _refused(s["id"], built.refusal, "design"),
                        "unreadable": {}, "data_unavailable": ""} for s in scenarios]
        else:
            requests = [built.request_for(s, routes, data) for s in scenarios]
            with ThreadPoolExecutor(max_workers=max(1, min(MAX_PARALLEL, len(requests)))) as pool:
                results = list(pool.map(lambda request: _in_a_child(request, limits), requests))
    unreadable: dict = {}
    for result in results:
        unreadable.update(result["unreadable"])
    data_unavailable = next((r["data_unavailable"] for r in results if r["data_unavailable"]), "")
    detail = "generated Python on the product's Python runtime"
    trace = {
        "record": RECORD,
        "v": VERSION,
        "engine": {"name": ENGINE_NAME,
                   "detail": (f"{detail}, generator {built.generator}" if built.generator
                              else detail)},
        "design": {"path": str(document),
                   "sha256": hashlib.sha256(document.read_bytes()).hexdigest()},
        "scenario_set": {"sha256": hashlib.sha256(raw).hexdigest()},
        "observes": {"outbound": True, "finished": True, "configuration": True,
                     "data": {"unavailable": data_unavailable} if data_unavailable else True},
        # What this run was bounded by and kept apart by, as the supervisor
        # applied it: a verdict says what it was made under.
        "limits": {**limits.record(), "time_stops_per_step": MAX_TIME_STOPS},
        "isolation": process.isolation_level(),
        "runs": [result["run"] for result in results],
    }
    if unreadable:
        trace["unreadable"] = unreadable
    # The names the design presents, as the product read them off it and put them
    # in its manifest. The judge holds the interface the set proposes to them; a
    # driver that left them out would make no comparison, and the judgement says
    # so. Copied, not derived: which events a caller can deliver is the analyzer's
    # answer to give.
    if not built.refusal and built.build.surface is not None:
        trace["surface"] = built.build.surface
    return trace


def judge(scenario_set: pathlib.Path, trace: dict, codegen: pathlib.Path | None = None) -> list:
    """The product's judgement of `trace` against the set: the summary record
    first, then one record per scenario, then each failed check, gap and
    problem, as `sce-codegen judge-scenarios` wrote them.

    The verdicts are the product's and are passed through untouched. A
    judgement computed here would be a second answer to a question that has
    one, free to disagree with the first."""
    with _scratch() as scratch:
        written = scratch / "observation-trace.json"
        written.write_text(json.dumps(trace, ensure_ascii=False), encoding="utf-8")
        report, refusal = scenario_judgement(scenario_set, written, codegen)
    return _records(report, refusal, "the trace could not be judged")


def _records(report: str, refusal: str, what: str) -> list:
    """The records of a product run, or its refusal in the product's words."""
    if refusal:
        reasons = [d.get("message") or d.get("unparsed")
                   for d in json.loads(refusal).get("diagnostics") or []]
        raise ScenarioDriverError(f"{what}: " + ("; ".join(r for r in reasons if r) or refusal))
    return json.loads(report)["records"]


def run(scenario_set: pathlib.Path, document: pathlib.Path,
        codegen: pathlib.Path | None = None, others: tuple = ()) -> dict:
    """Drive the set against the design, and have the product judge what was
    seen: `{"trace": ..., "judgement": [records]}`."""
    trace = drive(scenario_set, document, codegen, others=others)
    return {"trace": trace, "judgement": judge(scenario_set, trace, codegen)}


def read_set(scenario_set: pathlib.Path, specification: pathlib.Path | None = None,
             codegen: pathlib.Path | None = None, cwd: pathlib.Path | None = None) -> list:
    """The product's reading of the set: whether it is usable, and each
    problem it found, as `sce-codegen scenarios` wrote them. With the
    specification every quote is checked word for word; without it none is,
    and the summary says so."""
    report, refusal = scenario_set_reading(scenario_set, specification, codegen, cwd=cwd)
    return _records(report, refusal, "the file is not a scenario set")


def answer(read: list, played: dict | None, withheld: str | None = None) -> dict:
    """What the owner is told: the set's reading, and, when the set was usable
    and was played, the product's verdicts with the engine they are about.

    Every field is the product's record passed through, grouped. The words
    around them say only what the records leave unsaid: which engine, and
    that a pass is not a claim about the design.

    `withheld` says why a usable set was not played, when the design was not
    to be: the set's reading is still the owner's to see, and "set not usable"
    would blame the examples for what was the host's decision."""
    summary = next((r for r in read if r.get("kind") == "scenario-set"), {})
    problems = [r for r in read if r.get("kind") == "problem"]
    reply = {"set": {"usable": summary.get("usable"),
                     "origin": summary.get("origin"),
                     "scenarios": summary.get("scenarios"),
                     "runnable": summary.get("runnable"),
                     "quotes_checked": summary.get("quotes_checked"),
                     "problems": problems}}
    if played is None and withheld is not None and summary.get("usable"):
        reply["verdict"] = "not run"
        reply["says"] = withheld
        return reply
    if played is None:
        reply["verdict"] = "set not usable"
        reply["says"] = ("The examples have problems, so nothing was run: a verdict from a set "
                         "with problems is a verdict about nothing. Fix the problems listed and "
                         "ask again.")
        return reply
    records = played["judgement"]
    judgement = next((r for r in records if r.get("kind") == "judgement"), {})
    reply["verdict"] = "judged" if judgement.get("judged") else "nothing judged"
    reply["engine"] = judgement.get("engine")
    # The bounds and the isolation the verdicts were made under, repeated as the
    # product repeated them. Left out when the trace named none.
    for key in ("limits", "isolation"):
        if key in judgement:
            reply[key] = judgement[key]
    reply["counts"] = {name: judgement.get(name) for name in
                       ("scenarios", "pass", "fail", "not-judged", "blocked",
                        "awaiting-decision")}
    # Where the interface the examples were written against and the names the
    # design presents part, as the judge found it. `checked: false` is passed on
    # as it came: a comparison that was not made is not a match.
    interface = next((r for r in records if r.get("kind") == "interface"), None)
    if interface is not None:
        reply["interface"] = {k: v for k, v in interface.items() if k != "kind"}
    reply["scenarios"] = [{k: r[k] for k in ("id", "verdict", "reason", "cause",
                                             "requirements", "bound") if k in r}
                          for r in records if r.get("kind") == "verdict"]
    said = {r["reason"] for r in reply["scenarios"] if r.get("reason")}
    for kind in ("failure", "gap", "problem"):
        found = [r for r in records if r.get("kind") == kind]
        if kind == "gap":
            # A gap about a whole scenario says what its verdict's reason
            # already says. One about a step or a check says more.
            found = [r for r in found if r.get("step") is not None or r.get("check")
                     or r.get("why") not in said]
        if found:
            reply[f"{kind}s"] = found
    reply["means"] = judgement.get("means")
    return reply
