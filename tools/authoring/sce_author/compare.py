"""Compare several drafts of one specification at every level an owner or a
build reads them at, and say where they part.

An author asked twice for the same document writes two different files. How
different is a question with several answers, one per reader:

    bytes       the files as written
    canonical   W3C C14N 2.0, comments and whitespace-only text dropped
    logic       canonical, with every SCE annotation removed (the kind
                basis, `sce:*` attributes other than `sce:kind`) -- what the
                build compiles, so the generated code follows it
    table       the product's review table (`review_rows`), row identity
                (`node_path`, `source`) dropped -- what the acceptance
                report is built from
    page        the review page `pseudo_page` renders -- what the owner reads
    open        the `sce:unresolved` / `sce:assumed` markers -- the list of
                questions the owner is asked
    vocabulary  state ids and event names
    behaviour   what the drafts do when driven alike (statecharts)

Each level is a partition of the drafts into classes that agree.

⚠ Agreement is not correctness. Several drafts that agree are several reads
of the same prose, and they are wrong together as easily as they are right
together. Nothing here says a draft is right; a disagreement marks a place
the owner should look at, and an agreement says only that none was found.

⚠ A draft that cannot be driven is not a behaviour. It is left out of the
behaviour classes and named with the product's reason, and every verdict
carries how many distinct observations each draft's drives produced. Drives
that moved nothing (one observation per draft) judged nothing, and the level
says `not judged` rather than reporting the drafts as alike: measured
2026-09-29, a comparison without these two rules reported five drafts as
behaving alike because each had failed with the same error.

⚠ Each draft is driven with its own event names. SCXML matches an event
descriptor by prefix (W3C SCXML 3.12.1), so driving one draft's name into
another tests a behaviour nobody drives; instead two drafts are compared
under every bijection between their input alphabets, and the one that makes
them equal is reported -- it is exactly the renaming the owner should read.

⚠ It drives the Python lowering, as `verify` does. Two documents compared
under one lowering is a comparison of the documents, not a claim about the
code another backend ships. And it is bounded: a fixed set of seeded random
drives, printed with the verdict, never an equivalence proof.
"""

from __future__ import annotations

import collections
import hashlib
import itertools
import json
import pathlib
import random
import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field

from .verify import (SendRecorder, VerifyError, _default_codegen, _scratch, generate,
                     load, pseudo_page, review_rows, unresolved_markers, validate_scxml)

SCE_NAMESPACE = "http://sce.dev/ext"
SCXML_NAMESPACE = "http://www.w3.org/2005/07/scxml"

# The bound every behaviour verdict states.
DRIVES = 300
STEPS = 25
# How much of each drive is an event rather than time passing.
EVENT_SHARE = 0.7
# The time a drive advances by when no draft schedules anything.
DEFAULT_ADVANCE_MS = 1000
# A rename search tries every bijection; beyond this many input events that
# is more permutations than a comparison should spend, and the pair is
# reported as not searched rather than as different.
MAX_RENAME_ALPHABET = 7
# Drives a candidate renaming must survive before all of them are run.
SCREEN_DRIVES = 12


class CompareError(VerifyError):
    """The drafts cannot be compared at all."""


def _digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]


def canonical(text: str) -> str:
    """W3C C14N 2.0 of the document, comments and whitespace-only text
    dropped. Attribute order and quoting stop mattering; element order does
    not, because SCXML gives it meaning (W3C SCXML 3.13, 3.3)."""
    return ET.canonicalize(text, with_comments=False, strip_text=True)


def logic(text: str) -> str:
    """The canonical form with every SCE annotation removed: the kind
    basis, and each `sce:*` attribute except `sce:kind`, which chooses
    what the document compiles as."""
    root = ET.fromstring(text.encode("utf-8"))

    def strip(node):
        for child in list(node):
            if child.tag == f"{{{SCE_NAMESPACE}}}kind-basis":
                node.remove(child)
            else:
                strip(child)
        for name in list(node.attrib):
            if name.startswith(f"{{{SCE_NAMESPACE}}}") and name != f"{{{SCE_NAMESPACE}}}kind":
                del node.attrib[name]

    strip(root)
    return canonical(ET.tostring(root, encoding="unicode"))


@dataclass
class Vocabulary:
    states: set = field(default_factory=set)
    # Events a transition reacts to.
    events: set = field(default_factory=set)
    # Events the document raises or sends to itself; not inputs.
    own: set = field(default_factory=set)
    # Every delay the document schedules, in milliseconds.
    delays_ms: set = field(default_factory=set)

    @property
    def inputs(self) -> list:
        return sorted(self.events - self.own)


_DELAY = re.compile(r"^\s*(\d+(?:\.\d+)?)\s*(ms|s)\s*$")


def vocabulary(text: str) -> Vocabulary:
    """The names a draft uses, read from its XML."""
    root = ET.fromstring(text.encode("utf-8"))
    found = Vocabulary()
    for node in root.iter():
        tag = node.tag.rsplit("}", 1)[-1]
        if tag in ("state", "parallel", "final") and node.get("id"):
            found.states.add(node.get("id"))
        if tag == "transition" and node.get("event"):
            for descriptor in node.get("event").split():
                # `door.*` and `door` are the same descriptor (3.12.1); `*`
                # names no event anyone could send.
                name = descriptor[:-2] if descriptor.endswith(".*") else descriptor
                if name != "*":
                    found.events.add(name)
        if tag == "raise" and node.get("event"):
            found.own.add(node.get("event"))
        if tag == "send" and node.get("event"):
            # W3C SCXML 6.2.4: with neither target nor type the event goes
            # to this session's own external queue.
            if node.get("target") in (None, "#_internal") and node.get("type") is None:
                found.own.add(node.get("event"))
        if tag == "send" and node.get("delay"):
            match = _DELAY.match(node.get("delay"))
            if match:
                value = float(match.group(1))
                found.delays_ms.add(int(value * (1000 if match.group(2) == "s" else 1)))
    return found


def classes(values: dict) -> list:
    """The drafts, grouped by equal value, largest group first; drafts in
    each group keep the order they were given in."""
    order = list(values)
    groups = collections.OrderedDict()
    for name in order:
        key = json.dumps(values[name], sort_keys=True, default=str)
        groups.setdefault(key, []).append(name)
    return sorted(groups.values(), key=lambda g: (-len(g), order.index(g[0])))


# ------------------------------------------------------------ behaviour


class _Driven:
    """One draft, generated to Python and run from scratch per drive."""

    def __init__(self, document: pathlib.Path, codegen: pathlib.Path, into: pathlib.Path):
        self.build = generate(document, codegen, into)
        if self.build.refusal:
            raise VerifyError(self.build.refusal)
        self.module = load(into, document)

    def trace(self, steps) -> list:
        recorder = SendRecorder()
        engine = self.module.create_engine()
        for processor in self.build.declared:
            engine.register_event_processor(processor, recorder)
        engine.initialize()
        policy = engine.policy
        out = []

        def observe():
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


def _relabel(traces: list) -> list:
    """States renamed by first appearance across all traces, so two drafts
    that differ only in what they call their states compare equal."""
    names: dict = {}
    result = []
    for trace in traces:
        renamed = []
        for obs in trace:
            if obs and obs[0] == "raised":
                renamed.append(obs)
                continue
            leaves, sent, final = obs
            renamed.append((tuple(names.setdefault(s, f"S{len(names)}") for s in leaves),
                            sent, final))
        result.append(renamed)
    return result


def _advances(delays_ms: set) -> list:
    """Each scheduled delay, and one millisecond either side of it -- the
    points at which a deadline has or has not passed."""
    points = {DEFAULT_ADVANCE_MS}
    for delay in delays_ms:
        points.update({max(1, delay - 1), delay, delay + 1})
    return sorted(points)


def _drives(width: int, advances: list, count: int, steps: int) -> list:
    """Seeded drives over input positions `0..width`, so one set of drives
    can be spelled in each draft's own event names."""
    drives = []
    for seed in range(count):
        rnd = random.Random(seed)
        drive = []
        for _ in range(steps):
            if width and rnd.random() < EVENT_SHARE:
                drive.append(("event", rnd.randrange(width)))
            else:
                drive.append(("time", rnd.choice(advances)))
        drives.append(drive)
    return drives


def _spell(drive, names):
    return [(k, names[v]) if k == "event" else (k, v) for k, v in drive]


def _first_difference(a: list, b: list):
    """(drive index, step index) of the earliest observation two relabelled
    trace sets disagree at, over all drives; None when they agree."""
    best = None
    for d, (ta, tb) in enumerate(zip(a, b)):
        for s, (oa, ob) in enumerate(zip(ta, tb)):
            if oa != ob:
                if best is None or s < best[1]:
                    best = (d, s)
                break
        else:
            if len(ta) != len(tb):
                s = min(len(ta), len(tb))
                if best is None or s < best[1]:
                    best = (d, s)
    return best


def compare(documents: list, codegen: pathlib.Path | None = None, *,
            labels: list | None = None,
            drives: int = DRIVES, steps: int = STEPS) -> dict:
    """Every level, for the drafts given. Returns a JSON-ready report.

    Each draft is named in the report by its label, or by its file name
    when no labels are given."""
    if len(documents) < 2:
        raise CompareError("a comparison needs two drafts or more")
    documents = [pathlib.Path(d) for d in documents]
    if labels is not None and len(labels) != len(documents):
        raise CompareError("give one label per draft, or none")
    names = list(labels) if labels is not None else [d.name for d in documents]
    if len(set(names)) != len(names):
        if labels is not None:
            raise CompareError("two drafts were given the same label")
        # The report names a draft by its file name; two alike would be one.
        names = [str(d) for d in documents]
    codegen = pathlib.Path(codegen) if codegen else _default_codegen()
    texts = {n: d.read_text(encoding="utf-8") for n, d in zip(names, documents)}

    kinds, levels, open_sets, refusals = {}, collections.defaultdict(dict), {}, {}
    for name, path in zip(names, documents):
        text = texts[name]
        report, refusal = validate_scxml(path, codegen)
        answer = json.loads(report or refusal)
        reading = (answer.get("manifest") or {}).get("document_kind") or {}
        kinds[name] = reading.get("name")
        if refusal:
            refusals[name] = [d.get("message") or d.get("unparsed") for d in answer["diagnostics"]]
        levels["bytes"][name] = _digest(text)
        levels["canonical"][name] = _digest(canonical(text))
        levels["logic"][name] = _digest(logic(text))
        rows, refused = review_rows(path, kinds[name] == "statechart", codegen)
        if refused:
            levels["table"][name] = "refused"
        else:
            kept = [{k: v for k, v in r.items() if k not in ("node_path", "source")}
                    for r in json.loads(rows)["rows"]]
            levels["table"][name] = _digest("\n".join(sorted(json.dumps(r, sort_keys=True)
                                                             for r in kept)))
        page, refused = pseudo_page(path, codegen, None)
        levels["page"][name] = "refused" if refused else _digest(page)
        markers, refused = unresolved_markers(path, codegen)
        found = [] if refused else json.loads(markers)["markers"] or []
        open_sets[name] = sorted({f"{m.get('kind')}:{m.get('id')}" for m in found})
        levels["open"][name] = open_sets[name]
        if kinds[name] == "statechart":
            v = vocabulary(text)
            levels["vocabulary"][name] = [sorted(v.states), sorted(v.events)]

    report = {
        "documents": names,
        "kind": classes(kinds),
        "refused": refusals,
        "levels": {level: classes(values) for level, values in levels.items()},
        "open_sets": open_sets,
    }
    statecharts = [n for n in names if kinds[n] == "statechart" and n not in refusals]
    if len(statecharts) >= 2:
        report["behaviour"] = _behaviour(
            {n: d for n, d in zip(names, documents) if n in statecharts},
            texts, codegen, drives, steps)
    else:
        report["behaviour"] = {"verdict": "not applicable",
                               "why": "fewer than two accepted statecharts"}
    return report


def _behaviour(documents: dict, texts: dict, codegen, drives: int, steps: int) -> dict:
    vocab = {n: vocabulary(texts[n]) for n in documents}
    advances = _advances(set().union(*(v.delays_ms for v in vocab.values())))
    bound = {"drives": drives, "steps": steps, "advances_ms": advances}
    driven, undriven, distinct = {}, {}, {}
    with _scratch() as scratch:
        for index, (name, path) in enumerate(documents.items()):
            try:
                driven[name] = _Driven(path, codegen, scratch / f"draft{index}")
            except Exception as exc:  # noqa: BLE001 - the product's reason is the report
                undriven[name] = f"{type(exc).__name__}: {exc}"
        # Drives in each draft's own names, for how many observations they make.
        own = {}
        for name, engine in driven.items():
            alphabet = vocab[name].inputs
            traces = [engine.trace(_spell(d, alphabet))
                      for d in _drives(len(alphabet), advances, drives, steps)]
            own[name] = traces
            distinct[name] = len({json.dumps(o, default=str) for t in traces for o in t})
        judged = [n for n in driven if distinct[n] > 1]
        verdict = {"bound": bound, "undriven": undriven, "distinct_observations": distinct}
        if len(judged) < 2:
            verdict.update(
                verdict="not judged",
                why="fewer than two drafts produced more than one observation "
                    "under these drives: the drives moved nothing, so they "
                    "cannot say whether the drafts behave alike")
            return verdict
        groups: list = []  # [representative, [members], {member: mapping}]
        witnesses = []
        for name in judged:
            placed = False
            for group in groups:
                mapping, witness = _alike(group[0], name, driven, vocab, own, advances,
                                          drives, steps)
                if mapping is not None:
                    group[1].append(name)
                    group[2][name] = mapping
                    placed = True
                    break
                if witness is not None and not any(w["drafts"] == [group[0], name]
                                                   for w in witnesses):
                    witness["drafts"] = [group[0], name]
                    witnesses.append(witness)
            if not placed:
                groups.append([name, [name], {}])
        verdict.update(
            verdict="judged",
            classes=[g[1] for g in groups],
            renames={m: mapping for g in groups for m, mapping in g[2].items() if mapping},
            witnesses=[w for w in witnesses
                       if not any(w["drafts"][1] in g[1] and w["drafts"][0] in g[1]
                                  for g in groups)],
            unmoved=[n for n in driven if distinct[n] <= 1])
        return verdict


def _alike(first: str, second: str, driven, vocab, own, advances, drives, steps):
    """(mapping, None) when `second` behaves as `first` under some renaming
    of its input events; (None, witness) when it does not, the witness taken
    under the identity renaming when the two alphabets are one set."""
    a, b = vocab[first].inputs, vocab[second].inputs
    base = _drives(len(a), advances, drives, steps)
    reference = _relabel(own[first])
    if len(a) != len(b):
        return None, {"why": "the drafts react to different numbers of input events",
                      "only_first": sorted(set(a) - set(b)),
                      "only_second": sorted(set(b) - set(a))}
    if len(a) > MAX_RENAME_ALPHABET:
        return None, {"why": f"more than {MAX_RENAME_ALPHABET} input events: the renaming "
                             f"search was not run"}
    identity = list(range(len(b)))
    orders = [identity] + [list(p) for p in itertools.permutations(range(len(b)))
                           if list(p) != identity]
    for order in orders:
        spelled = [b[i] for i in order]
        screen = [driven[second].trace(_spell(d, spelled)) for d in base[:SCREEN_DRIVES]]
        if _relabel(screen) != _relabel(own[first][:SCREEN_DRIVES]):
            continue
        full = [driven[second].trace(_spell(d, spelled)) for d in base]
        if _relabel(full) == reference:
            # Only the names that differ: the rename is what the owner reads.
            return {a[i]: spelled[i] for i in range(len(a)) if a[i] != spelled[i]}, None
    if set(a) != set(b):
        return None, {"why": "no renaming of the input events makes them alike",
                      "only_first": sorted(set(a) - set(b)),
                      "only_second": sorted(set(b) - set(a))}
    mine = [driven[second].trace(_spell(d, a)) for d in base]
    at = _first_difference(_relabel(own[first]), _relabel(mine))
    if at is None:
        return None, {"why": "no renaming makes them alike"}
    d, s = at
    drive = _shrink(_spell(base[d], a)[:s], driven[first], driven[second])
    ends = driven[first].trace(drive)[-1], driven[second].trace(drive)[-1]
    return None, {
        "why": "they part under the same event names",
        "drive": [value if kind == "event" else f"{value} ms" for kind, value in drive],
        "first_then": ends[0],
        "second_then": ends[1],
    }


def _parts(drive, first: _Driven, second: _Driven) -> bool:
    """Whether the two drafts end this drive in different places, states
    compared up to renaming."""
    a, b = _relabel([first.trace(drive)]), _relabel([second.trace(drive)])
    return a[0][-1:] != b[0][-1:] or len(a[0]) != len(b[0])


def _shrink(drive, first: _Driven, second: _Driven) -> list:
    """The drive with every step removed that the difference does not need.

    The witness is for a person reading it against the prose, and a drive
    found at random carries steps that only happened to be there. Each is
    dropped in turn while the two drafts still end apart; what is left is
    minimal in that no single step can go, not the shortest drive that
    exists."""
    current = list(drive)
    changed = True
    while changed:
        changed = False
        for index in reversed(range(len(current))):
            candidate = current[:index] + current[index + 1:]
            if _parts(candidate, first, second):
                current = candidate
                changed = True
    return current


def summary(report: dict) -> str:
    """The report as lines a person reads."""
    lines = [f"{len(report['documents'])} drafts: {', '.join(report['documents'])}"]
    for level in ("kind",):
        lines.append(f"  {level:12s} {len(report[level])} class(es) {report[level]}")
    for level, groups in report["levels"].items():
        lines.append(f"  {level:12s} {len(groups)} class(es) {groups}")
    b = report["behaviour"]
    lines.append(f"  behaviour    {b['verdict']}")
    if b["verdict"] == "judged":
        lines.append(f"    classes    {b['classes']}")
        for draft, mapping in b.get("renames", {}).items():
            lines.append(f"    rename     {draft}: {mapping}")
        for w in b.get("witnesses", []):
            lines.append(f"    witness    {w}")
    elif b.get("why"):
        lines.append(f"    why        {b['why']}")
    if "bound" in b:
        lines.append(f"    bound      {b['bound']}")
        lines.append(f"    observed   {b['distinct_observations']}")
    if b.get("undriven"):
        lines.append(f"    undriven   {b['undriven']}")
    return "\n".join(lines)
