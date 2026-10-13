"""The same statechart, shown the way a reader who is not a programmer can follow it.

Boxes and arrows are one way to read a statechart and a poor one when several states
point at one another. This module draws three others, each from the document itself and
none from a reading of what the document is about:

  table       a table per container: the state you are in, the signal that arrives, where
              it goes. The document's own structure, read from the XML.
  timeline    what happened, step by step, in one scenario: which state each lane was in
              and what was sent out. A scenario is an example the owner confirmed
              (`schemas/sce-scenario-set.v1.schema.json`), played on the engine, so the
              sentence under each timeline is the specification's own.
  simulator   a machine to press buttons on. The behaviour in it is not typed here and not
              a reading of the XML: `views_explore` walks the REAL engine through every
              signal from every reachable situation and writes down what happened, and
              the page is a lookup into that. Before it is offered, random walks are
              played on the engine and on the lookup and must agree.

Nothing here knows what a document is about. The words a person reads come from three
places only: the document's own names, an optional labels file the owner or the writer
supplies (`Labels`), and the scenario set. A name nobody labelled is shown as written,
which is what keeps this honest: a renderer that invented plain words for a state would
be putting a reading of its own into the owner's picture.

⚠ A design is code and is never run in this process. The engine is started only by
`views_explore`, in a child with the kernel's limits (`process.run_isolated`), the way
`scenario_driver` starts `scenario_play`.
"""

from __future__ import annotations

import dataclasses
import html
import json
import pathlib
import re
import xml.etree.ElementTree as ET

from . import process
from .errors import AuthoringError

SCXML = "{http://www.w3.org/2005/07/scxml}"
SCE = "{http://sce.dev/ext}"

#: How many lanes one timeline or simulator draws. A parallel machine of a hundred regions
#: is a table, not a picture.
MAX_LANES = 8

#: What a state's colour is, by its position among its siblings.
PALETTE = 8

TEXT = {
    "ko": {
        "document": "문서 전체", "inside": "{name} 안에서", "now": "지금 상태 ＼ 오는 신호", "entering": "들어가면",
        "arrives": "{name}", "eventless": "신호 없이, 조건이 맞으면", "stay": "그대로",
        "start": "처음 상태", "together": "동시에 동작하는 영역", "final": "끝", "none": "받는 신호 없음",
        "see_table": "안의 상태는 아래 표", "when": "{cond} 일 때", "internal": "(상태를 나가지 않음)",
        "assign": "{location} ← {expr}", "send": "보냄: {event}", "after": "{delay} 뒤", "raise": "내부 신호: {event}",
        "cancel": "예약 취소: {id}", "log": "기록", "script": "스크립트 실행", "if": "조건 {cond} 이면",
        "else": "아니면", "in": "{state} 에 있음", "step": "단계", "begin": "시작", "signal": "신호",
        "time_passes": "{ms} 지남", "outputs": "나간 출력", "order": "신호가 온 순서 (시간 순) →",
        "time_axis": "시간은 시계 시간이 아니라 시나리오가 정한 만큼 흐른 가상 시간입니다.",
        "none_state": "—", "values": "값", "press": "신호를 눌러 보세요",
        "time_button": "시간이 흐름 → 다음 타이머까지", "time_button_due": "시간이 흐름 → {ms} 뒤",
        "reset": "처음으로", "elapsed": "지금까지 흐른 시간", "finished": "끝났습니다 (더 받는 신호가 없습니다)",
        "checked": "엔진과 대조: 임의로 고른 신호 {steps}번을 엔진과 이 표에 똑같이 넣어 보았고 차이는 {bad}번입니다.",
        "too_big": "시뮬레이터를 만들지 않았습니다: {why}", "events": "받는 신호", "heard": "이번 신호로 나간 출력 (켜진 것)",
        "origin": "예시 출처", "auto": "자동으로 고른 경로", "ms": "ms", "second": "초",
        "rule_region": "규칙", "rule_when": "언제", "rule_does": "하는 일", "always": "항상",
        "recheck": "내부 신호가 올 때마다 확인",
    },
    "en": {
        "document": "The whole document", "inside": "Inside {name}", "now": "State now \\ signal arriving",
        "entering": "On entering", "arrives": "{name}", "eventless": "With no signal, when the condition holds",
        "stay": "stays",
        "start": "Starts in", "together": "Regions that run together", "final": "end", "none": "takes no signal",
        "see_table": "states inside: see the table below", "when": "when {cond}", "internal": "(does not leave the state)",
        "assign": "{location} ← {expr}", "send": "sends {event}", "after": "after {delay}", "raise": "internal signal {event}",
        "cancel": "cancels {id}", "log": "logs", "script": "runs a script", "if": "if {cond}",
        "else": "otherwise", "in": "in {state}", "step": "Step", "begin": "Start", "signal": "signal",
        "time_passes": "{ms} pass", "outputs": "Sent out", "order": "the order the signals arrive in (time) →",
        "time_axis": "Time here is virtual time the scenario sets, not the clock.",
        "none_state": "—", "values": "values", "press": "Press a signal",
        "time_button": "Let time pass → next timer", "time_button_due": "Let time pass → {ms}",
        "reset": "Start over", "elapsed": "Time passed", "finished": "Finished (nothing more is taken)",
        "checked": "Checked against the engine: {steps} random signals went into the engine and into this table, {bad} differences.",
        "too_big": "Too many states to build the simulator: {why}", "events": "Signals", "heard": "Sent by the last signal",
        "origin": "Example from", "auto": "A path chosen automatically", "ms": "ms", "second": "s",
        "rule_region": "Rule", "rule_when": "When", "rule_does": "Does", "always": "always",
        "recheck": "checked each time an internal signal arrives",
    },
}


class ViewError(AuthoringError):
    """A view cannot be made, and the words say why."""


# ---------------------------------------------------------------------------- labels
@dataclasses.dataclass
class Labels:
    """The plain words for a document's names, from a file whoever owns the words writes.

    `{"title": ..., "states": {id: words}, "events": {name: words}, "outputs": {name: words},
    "lanes": {container id: words}, "data": {id: words}}`. A state may be given as `{"label": words,
    "note": one line saying what being in it means}`: a table cannot say what a state MEANS, only where it goes,
    and the note is where the owner says it. Anything left out is shown as the document wrote it. A label is a
    reading someone chose, so the document's own name stays on the page (as the tooltip of the label) and the
    label never replaces it."""

    title: str = ""
    states: dict = dataclasses.field(default_factory=dict)
    events: dict = dataclasses.field(default_factory=dict)
    outputs: dict = dataclasses.field(default_factory=dict)
    lanes: dict = dataclasses.field(default_factory=dict)
    data: dict = dataclasses.field(default_factory=dict)

    @classmethod
    def read(cls, path) -> "Labels":
        if path is None:
            return cls()
        try:
            raw = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
        except (OSError, ValueError) as exc:
            raise ViewError(f"{path}: not a labels file ({exc})") from exc
        unknown = sorted(set(raw) - {"title", "states", "events", "outputs", "lanes", "data"})
        if unknown:
            raise ViewError(f"{path}: unknown key(s) {', '.join(unknown)}; a labels file has "
                            f"title, states, events, outputs, lanes and data")
        for kind in ("states", "events", "outputs", "lanes", "data"):
            for name, value in (raw.get(kind) or {}).items():
                ok = isinstance(value, str) or (isinstance(value, dict) and set(value) <= {"label", "note"}
                                                and all(isinstance(v, str) for v in value.values()))
                if not ok:
                    raise ViewError(f"{path}: {kind}[{name!r}] is neither words nor {{\"label\", \"note\"}}")
        return cls(**{key: raw[key] for key in raw})

    def of(self, kind: str, name: str) -> str:
        value = getattr(self, kind).get(name)
        return (value.get("label") or "") if isinstance(value, dict) else (value or "")

    def note(self, kind: str, name: str) -> str:
        value = getattr(self, kind).get(name)
        return (value.get("note") or "") if isinstance(value, dict) else ""


# ---------------------------------------------------------------------------- the document
@dataclasses.dataclass
class Transition:
    events: list
    cond: str
    targets: list
    internal: bool
    actions: list


@dataclasses.dataclass
class State:
    id: str
    kind: str                      # root, state, parallel, final, history
    parent: "State | None" = None
    children: list = dataclasses.field(default_factory=list)
    initial: list = dataclasses.field(default_factory=list)
    transitions: list = dataclasses.field(default_factory=list)
    entry: list = dataclasses.field(default_factory=list)
    exit: list = dataclasses.field(default_factory=list)

    @property
    def substates(self) -> list:
        return [c for c in self.children if c.kind in ("state", "parallel", "final")]

    @property
    def is_container(self) -> bool:
        return self.kind in ("root", "parallel") or bool(self.substates)


@dataclasses.dataclass
class Machine:
    name: str
    root: State
    by_id: dict
    delayed: bool                  # whether any send is delayed, so time can matter
    host_types: list               # `x-...` processor types the document sends to
    own_events: frozenset          # events the machine gives itself (`<raise>`, a `<send>` with no target)

    def containers(self) -> list:
        out = []

        def walk(state):
            if state.is_container:
                out.append(state)
            for child in state.substates:
                walk(child)
        walk(self.root)
        return out

    def events_under(self, container: State) -> set:
        """Every event a transition of any state inside `container` answers."""
        found: set = set()

        def walk(state):
            for child in state.substates:
                for transition in child.transitions:
                    found.update(transition.events)
                walk(child)
        walk(container)
        return found

    def lane_containers(self) -> list:
        """Containers whose active child is worth a row: the root and every compound state.
        A parallel state is not a lane; its regions are."""
        return [c for c in self.containers() if c.kind != "parallel" and len(c.substates) >= 2
                or (c.kind == "root" and c.substates)]


def _tag(element) -> str:
    return element.tag.rsplit("}", 1)[-1]


def read_machine(path) -> Machine:
    try:
        root_element = ET.parse(path).getroot()
    except (OSError, ET.ParseError) as exc:
        raise ViewError(f"{path}: not a readable SCXML document ({exc})") from exc
    if _tag(root_element) != "scxml":
        raise ViewError(f"{path}: the root is <{_tag(root_element)}>, not <scxml>")
    kind = root_element.get(f"{SCE}kind", "statechart")
    if kind != "statechart":
        raise ViewError(f"{path}: this is a {kind} document, and these views draw statecharts")
    by_id: dict = {}
    delayed = False
    host_types: list = []
    own: set = set()

    def actions_of(element) -> list:
        return [child for child in element if _tag(child) not in ("transition", "state", "parallel", "final",
                                                                   "history", "initial", "datamodel", "invoke",
                                                                   "onentry", "onexit")]

    def note(element) -> None:
        nonlocal delayed
        for node in element.iter():
            if _tag(node) == "raise" and node.get("event"):
                own.add(node.get("event"))
            if _tag(node) == "send":
                if node.get("delay") or node.get("delayexpr"):
                    delayed = True
                kind_ = node.get("type") or ""
                if kind_.startswith("x-") and kind_ not in host_types:
                    host_types.append(kind_)
                if not (kind_ or node.get("target") or node.get("targetexpr") or node.get("typeexpr")) and node.get("event"):
                    own.add(node.get("event"))

    def build(element, parent, kind_) -> State:
        state = State(element.get("id") or f"({_tag(element)}{len(by_id)})", kind_, parent)
        by_id[state.id] = state
        for child in element:
            tag = _tag(child)
            if tag in ("state", "parallel", "final", "history"):
                state.children.append(build(child, state, tag))
            elif tag == "transition":
                state.transitions.append(Transition(
                    events=[e[:-2] if e.endswith(".*") else e for e in (child.get("event") or "").split()],
                    cond=child.get("cond") or "", targets=(child.get("target") or "").split(),
                    internal=child.get("type") == "internal", actions=actions_of(child)))
            elif tag == "initial":
                inner = child.find(f"{SCXML}transition")
                if inner is not None and inner.get("target"):
                    state.initial = inner.get("target").split()
            elif tag == "onentry":
                state.entry += actions_of(child)
            elif tag == "onexit":
                state.exit += actions_of(child)
        if element.get("initial"):
            state.initial = element.get("initial").split()
        if not state.initial and kind_ == "state" and state.substates:
            state.initial = [state.substates[0].id]
        return state

    root = State("", "root")
    for child in root_element:
        tag = _tag(child)
        if tag in ("state", "parallel", "final"):
            root.children.append(build(child, root, tag))
    if root_element.get("initial"):
        root.initial = root_element.get("initial").split()
    elif root.substates:
        root.initial = [root.substates[0].id]
    note(root_element)
    if not root.substates:
        raise ViewError(f"{path}: the document has no states")
    return Machine(root_element.get("name") or pathlib.Path(path).stem, root, by_id, delayed, host_types,
                   frozenset(own))


# ---------------------------------------------------------------------------- words
def say(lang: str, key: str, **fields) -> str:
    return TEXT[lang][key].format(**fields)


def esc(text) -> str:
    return html.escape(str(text), quote=True)


def duration(ms: int, lang: str) -> str:
    if ms < 1000:
        return f"{ms} {TEXT[lang]['ms']}"
    seconds = ms / 1000
    return f"{seconds:g} {TEXT[lang]['second']}"


def name_html(labels: Labels, kind: str, name: str, detail: bool = False) -> str:
    """A name as the reader sees it: the words when someone wrote them, with the document's own name kept as their
    tooltip, or the document's name alone. `detail` adds the note that says what the name means (a row heading
    has room for it; the cell a signal points at does not)."""
    words = labels.of(kind, name)
    out = (f'<span class="nm" title="{esc(name)}">{esc(words)}</span>' if words
           else f'<span class="nm id0">{esc(name)}</span>')
    note = labels.note(kind, name) if detail else ""
    return out + (f'<span class="note1">{esc(note)}</span>' if note else "")


_COND_TOKEN = re.compile(r"\s*(In\(\s*'[^']+'\s*\)|&&|\|\||!|\(|\))")


def _parse_cond(text: str):
    """A condition made only of `In('state')`, `&&`, `||`, `!` and parentheses as a tree, or None for anything
    else (a comparison, a call): an expression this does not understand is shown as it was written."""
    tokens, position = [], 0
    while position < len(text):
        match = _COND_TOKEN.match(text, position)
        if match is None:
            if text[position:].strip():
                return None
            break
        tokens.append(match.group(1))
        position = match.end()
    cursor = [0]

    def peek():
        return tokens[cursor[0]] if cursor[0] < len(tokens) else None

    def take():
        cursor[0] += 1
        return tokens[cursor[0] - 1]

    def alternatives():
        parts = [conjunction()]
        while peek() == "||":
            take()
            parts.append(conjunction())
        return parts[0] if len(parts) == 1 else ("or", parts)

    def conjunction():
        parts = [negation()]
        while peek() == "&&":
            take()
            parts.append(negation())
        return parts[0] if len(parts) == 1 else ("and", parts)

    def negation():
        if peek() == "!":
            take()
            return ("not", negation())
        if peek() == "(":
            take()
            inner = alternatives()
            if peek() != ")":
                raise ValueError("unbalanced")
            take()
            return inner
        token = peek()
        if token is None or not token.startswith("In("):
            raise ValueError("unexpected")
        take()
        return ("in", re.search(r"'([^']+)'", token).group(1))

    try:
        tree = alternatives()
    except ValueError:
        return None
    return tree if cursor[0] == len(tokens) else None


def describe_cond(cond: str, labels: Labels, lang: str, machine: "Machine | None" = None) -> str:
    """The condition as a sentence, as HTML. A condition over states reads `the lane is in this state`, an OR is a list,
    and an AND whose terms name the same state is said once ("both ... are in ..."). Anything else is shown as written."""
    text = html.unescape(cond)
    tree = _parse_cond(text)
    if tree is None:
        return esc(say(lang, "when", cond=text))

    def lane_of(state: str) -> str:
        parent = machine.by_id[state].parent if machine and state in machine.by_id else None
        ident = parent.id if parent is not None else ""
        return labels.of("lanes", ident) or labels.of("states", ident) or ident or state

    def atom(state: str) -> str:
        return f"{esc(lane_of(state))}: &lsquo;{esc(labels.of('states', state) or state)}&rsquo;"

    def inline(node, top=False) -> str:
        kind = node[0]
        if kind == "in":
            return atom(node[1])
        if kind == "not":
            return "NOT (" + inline(node[1]) + ")"
        if kind == "and":
            atoms = [n for n in node[1] if n[0] == "in"]
            if len(atoms) == len(node[1]) and len({labels.of("states", n[1]) or n[1] for n in atoms}) == 1:
                lanes = ", ".join(esc(lane_of(n[1])) for n in atoms)
                words = labels.of("states", atoms[0][1]) or atoms[0][1]
                return f"{lanes} &lsquo;{esc(words)}&rsquo;" + (" (모두)" if lang == "ko" else " (all)")
            return (" 그리고 " if lang == "ko" else " and ").join(inline(n) for n in node[1])
        return "(" + (" 또는 " if lang == "ko" else " or ").join(inline(n) for n in node[1]) + ")"

    if tree[0] == "or":
        head = "다음 중 하나일 때" if lang == "ko" else "when any of these holds"
        return head + "<br>" + "<br>".join("&bull; " + inline(n) for n in tree[1])
    return inline(tree) + (" 일 때" if lang == "ko" else "")


def describe_action(element, labels: Labels, lang: str, machine: "Machine | None" = None,
                    internal: bool = False) -> str:
    """One thing a transition or an entry does, as HTML, or "" for what a reader is better without.

    ⚠ A `<raise>` is the machine talking to itself (a rule that re-checks after a state changes). It is true and it is
    in every cell of a table of a machine that does it, so it is the one thing that is left out unless `internal`:
    shown, it buries the one fact each cell exists to give."""
    tag = _tag(element)
    if tag == "raise" and not internal:
        return ""
    if tag == "assign":
        location = element.get("location") or ""
        return esc(say(lang, "assign", location=labels.of("data", location) or location,
                       expr=html.unescape(element.get("expr") or "")))
    if tag == "send":
        event = element.get("event") or element.get("eventexpr") or ""
        kind = element.get("type") or ""
        words = labels.of("outputs", output_key(kind, event)) or labels.of("outputs", event)
        if not words:
            words = f"{event} ({kind})" if kind.startswith("x-") else event
        text = esc(say(lang, "send", event=words))
        delay = element.get("delay") or element.get("delayexpr")
        return text + (esc(f" ({say(lang, 'after', delay=delay)})") if delay else "")
    if tag == "raise":
        return esc(say(lang, "raise", event=element.get("event") or ""))
    if tag == "cancel":
        return esc(say(lang, "cancel", id=element.get("sendid") or element.get("sendidexpr") or ""))
    if tag in ("log", "script"):
        return esc(say(lang, tag))
    if tag == "if":
        def head(cond):
            return describe_cond(cond, labels, lang, machine).replace("<br>", " / ")
        branches, current = [], [head(element.get("cond") or "")]
        for child in element:
            ctag = _tag(child)
            if ctag == "elseif":
                branches.append(current)
                current = [head(child.get("cond") or "")]
            elif ctag == "else":
                branches.append(current)
                current = [esc(say(lang, "else"))]
            else:
                current.append(describe_action(child, labels, lang, machine, internal))
        branches.append(current)
        return "; ".join(" ".join(part for part in branch if part) for branch in branches)
    return esc(f"<{tag}>")


# ---------------------------------------------------------------------------- the table
def _visible(actions, labels: Labels, lang: str, machine: Machine, internal: bool) -> list:
    return [text for text in (describe_action(a, labels, lang, machine, internal) for a in actions) if text]


def _table(container: State, regions: list, **fields) -> dict:
    return {"container": container.id, "kind": container.kind, "initial": [], "events": [], "eventless": False,
            "entering": False, "rows": [], "rules": [], "regions": regions, **fields}


def table_model(machine: Machine, labels: Labels, lang: str = "ko", internal: bool = False) -> list:
    """One table per container: its rows are the states directly inside it.

    A parallel container's compound regions each have a table of their own, so it lists them and tabulates only its
    atomic regions. Regions that only watch and act (no transition leaves a state) are not a state-by-signal table at
    all but a list of rules, "when this holds, do that", and are given as `rules`. A table in which nothing happens
    in any row (a root with one state that only holds the rest) is left out: a row of blanks tells a reader nothing.

    `internal` brings back what the machine says to itself (a `<raise>`), which is in every cell and says nothing a
    reader asked."""
    tables = []
    for container in machine.containers():
        rows = container.substates
        regions = []
        if container.kind == "parallel":
            regions = [r.id for r in rows if r.substates]
            rows = [r for r in rows if not r.substates]
        if not any(r.transitions or r.entry for r in rows):
            if regions:
                tables.append(_table(container, regions))
            continue
        if container.kind == "parallel" and all(not t.targets for r in rows for t in r.transitions):
            rules = []
            for row in rows:
                for transition in row.transitions:
                    actions = _visible(transition.actions, labels, lang, machine, internal)
                    if not actions:
                        continue
                    outside = [e for e in transition.events if e not in machine.own_events]
                    rules.append({"region": row.id, "events": outside,
                                  "own": bool(transition.events) and not outside,
                                  "cond": describe_cond(transition.cond, labels, lang, machine)
                                  if transition.cond else "", "actions": actions})
            tables.append(_table(container, regions, rules=rules))
            continue
        events: list = []
        eventless = False
        for row in rows:
            for transition in row.transitions:
                if not transition.events:
                    eventless = True
                for event in transition.events:
                    if event not in events:
                        events.append(event)
        entry = {row.id: _visible(row.entry, labels, lang, machine, internal) for row in rows}
        table_rows = []
        for row in rows:
            cells = {}
            for key in [*events, None]:
                entries = []
                for transition in row.transitions:
                    if (key is None and not transition.events) or (key is not None and key in transition.events):
                        entries.append({"cond": describe_cond(transition.cond, labels, lang, machine)
                                        if transition.cond else "",
                                        "targets": transition.targets, "internal": transition.internal,
                                        "actions": _visible(transition.actions, labels, lang, machine, internal)})
                cells[key] = entries
            table_rows.append({"state": row.id, "kind": row.kind, "inner": len(row.substates),
                               "entry": entry[row.id], "cells": cells})
        tables.append(_table(container, regions, initial=container.initial if container.kind != "parallel" else [],
                             events=events, eventless=eventless, entering=any(entry.values()), rows=table_rows))
    return tables


def _cell_html(entries: list, source: str, labels: Labels, lang: str) -> str:
    """A cell. One that changes where the machine is stands out; one that leaves it where it is, with nothing else to
    say, is a quiet word, so the cells that matter are the ones the eye lands on."""
    if not entries:
        return '<td class="none"></td>'
    if all(e["targets"] in ([], [source]) and not e["cond"] and not e["actions"] for e in entries):
        return f'<td class="quiet">{esc(say(lang, "stay"))}</td>'
    parts = []
    for entry in entries:
        lines = []
        if entry["cond"]:
            lines.append(f'<span class="cond">{entry["cond"]}</span>')
        if entry["targets"]:
            same = entry["targets"] == [source]
            target = " ".join(name_html(labels, "states", t) for t in entry["targets"])
            lines.append((f'<span class="stay">{esc(say(lang, "stay"))}</span> ' if same else "&rarr; ") + target
                         + (f' <span class="int">{esc(say(lang, "internal"))}</span>' if entry["internal"] else ""))
        for action in entry["actions"]:
            lines.append(f'<span class="act">{action}</span>')
        parts.append("<div>" + "<br>".join(lines) + "</div>")
    moved = any(e["targets"] and e["targets"] != [source] for e in entries)
    return f'<td class="{"moves" if moved else ""}">' + '<div class="or"></div>'.join(parts) + "</td>"


def _rules_html(table: dict, labels: Labels, lang: str) -> str:
    """Rules that only watch and act, as `when this holds, this is done`, one line each."""
    body = []
    rules = table["rules"]
    for index, rule in enumerate(rules):
        when = [f'<b>{esc(labels.of("events", e) or e)}</b>' for e in rule["events"]]
        if rule["cond"]:
            when.append(f'<span class="cond">{rule["cond"]}</span>')
        if rule["own"]:
            when.append(f'<span class="int">{esc(say(lang, "recheck"))}</span>')
        # A region with several rules is named once, spanning them.
        first = index == 0 or rules[index - 1]["region"] != rule["region"]
        span = next((n for n in range(1, len(rules) - index + 1)
                     if index + n == len(rules) or rules[index + n]["region"] != rule["region"]), 1)
        head = (f'<th scope="row" rowspan="{span}">{name_html(labels, "states", rule["region"], detail=True)}</th>'
                if first else "")
        body.append(f'<tr>{head}'
                    f'<td>{"<br>".join(when) or esc(say(lang, "always"))}</td>'
                    f'<td>{"<br>".join(f"<span class=act>{a}</span>" for a in rule["actions"])}</td></tr>')
    head = "".join(f"<th>{esc(say(lang, key))}</th>" for key in ("rule_region", "rule_when", "rule_does"))
    return (f'<div class="sv-scroll"><table class="sv-table sv-rules"><thead><tr>{head}</tr></thead>'
            f'<tbody>{"".join(body)}</tbody></table></div>')


def table_html(machine: Machine, labels: Labels, lang: str = "ko", only=None, internal: bool = False) -> str:
    """The tables, as HTML. `only` names the containers to show (a long design's other lanes are the same shape);
    `internal` brings back what the machine says to itself."""
    out = []
    for table in table_model(machine, labels, lang, internal):
        container = table["container"]
        if only is not None and container not in only:
            continue
        title = (labels.title or say(lang, "document")) if table["kind"] == "root" \
            else say(lang, "inside", name=labels.of("states", container) or container)
        note = ""
        if table["kind"] == "parallel":
            note = (f'<p class="sv-note">{esc(say(lang, "together"))}: '
                    + ", ".join(name_html(labels, "lanes", r) if labels.of("lanes", r) else name_html(labels, "states", r)
                                for r in table["regions"]) + "</p>")
            if table["rules"]:
                out.append(f'<section class="sv-block"><h4>{esc(title)}</h4>{note}{_rules_html(table, labels, lang)}</section>')
                continue
            if not table["rows"]:
                out.append(f'<section class="sv-block"><h4>{esc(title)}</h4>{note}</section>')
                continue
        elif table["initial"]:
            note = (f'<p class="sv-note">{esc(say(lang, "start"))}: '
                    + " ".join(name_html(labels, "states", s) for s in table["initial"]) + "</p>")
        head = f'<th>{esc(say(lang, "now"))}</th>'
        if table["entering"]:
            head += f'<th>{esc(say(lang, "entering"))}</th>'
        for event in table["events"]:
            head += f'<th>{esc(say(lang, "arrives", name=labels.of("events", event) or event))}' \
                    f'{"<span class=id>" + esc(event) + "</span>" if labels.of("events", event) else ""}</th>'
        if table["eventless"]:
            head += f'<th>{esc(say(lang, "eventless"))}</th>'
        body = []
        for row in table["rows"]:
            cells = f'<th scope="row">{name_html(labels, "states", row["state"], detail=True)}'
            if row["kind"] == "final":
                cells += f'<span class="int">{esc(say(lang, "final"))}</span>'
            if row["inner"]:
                cells += f'<span class="int">{esc(say(lang, "see_table"))}</span>'
            cells += "</th>"
            if table["entering"]:
                cells += "<td>" + "<br>".join(f'<span class="act">{a}</span>' for a in row["entry"]) + "</td>"
            for event in [*table["events"], *([None] if table["eventless"] else [])]:
                cells += _cell_html(row["cells"][event], row["state"], labels, lang)
            body.append(f"<tr>{cells}</tr>")
        out.append(f'<section class="sv-block"><h4>{esc(title)}</h4>{note}'
                   f'<div class="sv-scroll"><table class="sv-table"><thead><tr>{head}</tr></thead>'
                   f'<tbody>{"".join(body)}</tbody></table></div></section>')
    return f'<div class="sv sv-tables">{"".join(out)}</div>'


#: Between a processor type and an event name in an output's identity (`x-host:on`). Not a slash: an output is not a
#: location, and a labels file written with one would read like a path.
OUTPUT_SEPARATOR = ":"


def output_key(processor: str, event: str) -> str:
    """An output as the engine names it: the event, with the processor type it left through when it had one. Three
    processors can each send an event called `on`, so the event alone does not say which output it was."""
    return f"{processor}{OUTPUT_SEPARATOR}{event}" if processor else event


def output_event(key: str) -> str:
    return key.rsplit(OUTPUT_SEPARATOR, 1)[-1]


def output_names(keys, labels: Labels) -> dict:
    """The words for each output. A label wins; otherwise the event alone, which reads better and is what a
    specification calls it, unless two outputs of the design share an event name, where the processor is kept to tell
    them apart."""
    keys = list(dict.fromkeys(keys))
    shared: dict = {}
    for key in keys:
        shared.setdefault(output_event(key), set()).add(key)
    return {key: labels.of("outputs", key) or (output_event(key) if len(shared[output_event(key)]) == 1 else key)
            for key in keys}


#: A lane's cell once the machine has finished: no state is active any more, and a blank would read as "unknown".
FINISHED = "\0finished"


# ---------------------------------------------------------------------------- the timeline
def _active_child(container: State, configuration: set):
    for child in container.substates:
        if child.id in configuration:
            return child
    return None


def timeline_model(machine: Machine, scenario: dict, run: dict, lang: str = "ko") -> dict:
    """What happened in one scenario: a column per observation, a lane per container that moved, and
    what was sent out. `run` is `views_explore.play`'s record: the machine as it starts, then one
    observation after each step."""
    observations = run["observations"]
    steps = [{}] + list(scenario["steps"])
    if len(observations) != len(steps):
        raise ViewError(f"scenario {scenario['id']}: {len(observations)} observations for {len(steps)} steps")
    columns, clock = [], 0
    for step, seen in zip(steps, observations):
        column = {"kind": "start", "outputs": list(seen["outs"]), "finished": seen["finished"],
                  "configuration": set(seen["cfg"]), "data": seen.get("data") or {}}
        if "send" in step:
            column.update(kind="send", event=step["send"], payload=step.get("payload"))
        elif "advance_ms" in step:
            clock += step["advance_ms"]
            column.update(kind="time", ms=step["advance_ms"])
        column["clock"] = clock
        columns.append(column)
    lanes = []
    for container in machine.lane_containers():
        cells = []
        for column in columns:
            active = _active_child(container, column["configuration"])
            cells.append(active.id if active else (FINISHED if column["finished"] else None))
        lanes.append({"container": container.id, "cells": cells, "moves": len(set(cells)) > 1,
                      "children": [c.id for c in container.substates]})
    moving = [lane for lane in lanes if lane["moves"]]
    chosen = (moving or lanes[:1])[:MAX_LANES]
    names = sorted({k for column in columns for k in column["data"]})
    values = [{"name": n, "cells": [column["data"].get(n) for column in columns]} for n in names
              if len({json.dumps(c["data"].get(n)) for c in columns}) > 1]
    return {"id": scenario["id"], "quote": scenario.get("quote", ""), "note": scenario.get("note", ""),
            "assumes": scenario.get("assumes") or [], "columns": columns, "lanes": chosen, "values": values}


def _text_width(text: str, size: float) -> float:
    """How wide `text` is at `size`, roughly: a Hangul or other wide glyph takes a whole em, a Latin one about 0.6."""
    return sum(size if ord(ch) >= 0x2E80 else size * 0.6 for ch in text)


def _fit(text: str, width: float, size: float) -> tuple:
    """(text, font size) that fits a bar `width` wide: shrunk first, then cut with an ellipsis."""
    for current in (size, size - 1.5, size - 3):
        if _text_width(text, current) <= width:
            return text, current
    small = size - 3
    keep = text
    while keep and _text_width(keep + "…", small) > width:
        keep = keep[:-1]
    return keep + "…", small


def timeline_html(machine: Machine, model: dict, labels: Labels, lang: str = "ko", titled: bool = True) -> str:
    """One scenario's timeline. `titled=False` leaves out its name and sentence, for a page that has already
    put them above it."""
    columns, lanes = model["columns"], model["lanes"]

    def heading(column) -> tuple:
        if column["kind"] == "start":
            return say(lang, "begin"), ""
        if column["kind"] == "send":
            event = column["event"]
            return labels.of("events", event) or event, event if labels.of("events", event) else ""
        return say(lang, "time_passes", ms=duration(column["ms"], lang)), f"({duration(column['clock'], lang)})"

    # A column is as wide as its longest heading needs, so a plain-language label is read and not shrunk to a smudge.
    longest = max(_text_width(heading(c)[0], 11.5) for c in columns)
    x0, colw, top, rowh, gap = 150, max(104, min(190, int(longest) + 16)), 52, 40, 12
    count = len(columns)
    width = x0 + colw * count + 8
    value_rows = model["values"][:3]
    rows = len(lanes) + len(value_rows)
    out_y = top + rows * (rowh + gap) + 4
    height = out_y + rowh + 50
    svg = [f'<svg viewBox="0 0 {width} {height}" width="{width}" role="img" aria-label="{esc(model["id"])}" '
           f'class="sv-svg">']
    for i, column in enumerate(columns):
        cx = x0 + colw * i
        svg.append(f'<line x1="{cx}" y1="{top - 10}" x2="{cx}" y2="{height - 40}" class="guide"/>')
        head, sub = heading(column)
        text, size = _fit(head, colw - 8, 11.5)
        svg.append(f'<text x="{cx + colw / 2}" y="{top - 26}" text-anchor="middle" font-size="{size}" '
                   f'font-weight="700" class="ink">{esc(text)}</text>')
        if sub:
            svg.append(f'<text x="{cx + colw / 2}" y="{top - 12}" text-anchor="middle" font-size="9" class="faint">{esc(sub)}</text>')
    svg.append(f'<line x1="{x0 + colw * count}" y1="{top - 10}" x2="{x0 + colw * count}" y2="{height - 40}" class="guide"/>')
    for r, lane in enumerate(lanes):
        y = top + r * (rowh + gap)
        container = machine.by_id.get(lane["container"]) or machine.root
        words = labels.of("lanes", lane["container"]) or labels.of("states", lane["container"]) or lane["container"] or \
            (labels.title or machine.name)
        text, size = _fit(words, x0 - 16, 12)
        svg.append(f'<text x="{x0 - 10}" y="{y + rowh / 2 + 4}" text-anchor="end" font-size="{size}" '
                   f'font-weight="600" class="ink">{esc(text)}</text>')
        i = 0
        cells = lane["cells"]
        while i < count:
            j = i
            while j + 1 < count and cells[j + 1] == cells[i]:
                j += 1
            state = cells[i]
            w = colw * (j - i + 1) - 4
            if state is None:
                svg.append(f'<rect x="{x0 + colw * i + 2}" y="{y}" width="{w}" height="{rowh}" rx="5" class="idle"/>')
            elif state == FINISHED:
                svg.append(f'<rect x="{x0 + colw * i + 2}" y="{y}" width="{w}" height="{rowh}" rx="5" class="bar c0"/>')
                svg.append(f'<text x="{x0 + colw * i + 2 + w / 2}" y="{y + rowh / 2 + 4}" text-anchor="middle" '
                           f'font-size="12.5" font-weight="700" class="bar-ink c0">{esc(say(lang, "final"))}</text>')
            else:
                shade = lane["children"].index(state) % PALETTE
                words = labels.of("states", state) or state
                text, size = _fit(words, w - 8, 12.5)
                # The document's own name is the tooltip of the bar, not a second line on it.
                tip = f"<title>{esc(state)}</title>" if labels.of("states", state) else ""
                svg.append(f'<g>{tip}<rect x="{x0 + colw * i + 2}" y="{y}" width="{w}" height="{rowh}" rx="5" '
                           f'class="bar c{shade}"/><text x="{x0 + colw * i + 2 + w / 2}" y="{y + rowh / 2 + 4}" '
                           f'text-anchor="middle" font-size="{size}" font-weight="700" class="bar-ink c{shade}">'
                           f'{esc(text)}</text></g>')
            i = j + 1
    for k, value in enumerate(value_rows):
        y = top + (len(lanes) + k) * (rowh + gap)
        svg.append(f'<text x="{x0 - 10}" y="{y + rowh / 2 + 4}" text-anchor="end" font-size="12" font-weight="600" '
                   f'class="ink">{esc(labels.of("data", value["name"]) or value["name"])}</text>')
        for i, cell in enumerate(value["cells"]):
            svg.append(f'<rect x="{x0 + colw * i + 2}" y="{y + 6}" width="{colw - 4}" height="{rowh - 12}" rx="5" class="val"/>')
            svg.append(f'<text x="{x0 + colw * i + colw / 2}" y="{y + rowh / 2 + 4}" text-anchor="middle" font-size="12" '
                       f'class="ink">{esc("" if cell is None else cell)}</text>')
    svg.append(f'<text x="{x0 - 10}" y="{out_y + rowh / 2 + 4}" text-anchor="end" font-size="12" font-weight="600" '
               f'class="ink">{esc(say(lang, "outputs"))}</text>')
    svg.append(f'<line x1="{x0}" y1="{out_y + rowh / 2}" x2="{x0 + colw * count}" y2="{out_y + rowh / 2}" class="guide"/>')
    names = output_names([o for column in columns for o in column["outputs"]], labels)
    for i, column in enumerate(columns):
        for event in column["outputs"]:
            words = names[event]
            text, size = _fit(words, colw - 14, 11.5)
            cx = x0 + colw * i + colw / 2
            svg.append(f'<rect x="{cx - (colw - 14) / 2}" y="{out_y + 6}" width="{colw - 14}" height="{rowh - 12}" '
                       f'rx="{(rowh - 12) / 2}" class="out"/>')
            svg.append(f'<text x="{cx}" y="{out_y + rowh / 2 + 4}" text-anchor="middle" font-size="{size}" '
                       f'font-weight="700" class="out-ink">{esc(text)}</text>')
    svg.append(f'<text x="{x0}" y="{height - 22}" font-size="10.5" class="faint">{esc(say(lang, "order"))}</text>')
    svg.append("</svg>")
    quote = f'<p class="sv-quote">&ldquo;{esc(model["quote"])}&rdquo;</p>' if model["quote"] and titled else ""
    note = f'<p class="sv-note">{esc(model["note"])}</p>' if model["note"] else ""
    caption = f'<figcaption><b>{esc(model["id"])}</b></figcaption>{quote}' if titled else ""
    return f'<figure class="sv-figure">{caption}<div class="sv-scroll">{"".join(svg)}</div>{note}</figure>'


# ---------------------------------------------------------------------------- the simulator
SIMULATOR_SCRIPT = r"""
(function(){
  var root=document.getElementById('__ID__'); if(!root) return;
  var D=__DATA__;
  var cur=0, elapsed=0, lastOut=[], logLines=[];
  var panel=root.querySelector('.sim-lanes'), data=root.querySelector('.sim-data'), btns=root.querySelector('.sim-btns'),
      lamps=root.querySelector('.sim-lamps'), log=root.querySelector('.sim-log'), clock=root.querySelector('.sim-clock'),
      done=root.querySelector('.sim-done');
  function nm(map,id){ return id===undefined? '—' : (map[id]||id); }
  function fmt(ms){ return ms<1000? ms+' ms' : (ms/1000)+' s'; }
  function build(){
    var h='';
    D.lanes.forEach(function(l,li){
      h+='<div class="pn"><h5>'+esc(nm(D.laneNames,l.id))+'</h5><div class="chips">';
      l.children.forEach(function(c,ci){ h+='<span class="st c'+(ci%8)+'" data-lane="'+li+'" data-s="'+esc(c)+'" title="'+esc(c+(D.stateNotes[c]?' — '+D.stateNotes[c]:''))+'">'+esc(nm(D.stateNames,c))+'</span>'; });
      h+='</div><div class="pn-btns">';
      D.actions.forEach(function(a,i){ if(a.lanes.indexOf(li)>=0) h+='<button type="button" data-a="'+i+'">'+esc(a.label)+'</button>'; });
      h+='</div></div>';
    });
    panel.innerHTML=h;
    h='';
    D.actions.forEach(function(a,i){ if(!a.lanes.length) h+='<button type="button" data-a="'+i+'">'+esc(a.label)+'</button>'; });
    btns.innerHTML=h;
    h=''; D.outputs.forEach(function(o){ h+='<span class="lamp" data-o="'+esc(o)+'">'+esc(nm(D.outputNames,o))+'</span>'; });
    lamps.innerHTML=h;
  }
  function esc(t){ return String(t).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }
  function show(){
    var n=D.nodes[cur], cfg={}; n.cfg.forEach(function(s){ cfg[s]=1; });
    [].forEach.call(panel.querySelectorAll('.st'),function(c){ c.classList.toggle('on',!!cfg[c.getAttribute('data-s')]); });
    var h=''; Object.keys(n.data).forEach(function(k){ h+='<span class="dv"><b>'+esc(nm(D.dataNames,k))+'</b> '+esc(n.data[k])+'</span>'; });
    data.innerHTML=h;
    [].forEach.call(root.querySelectorAll('button[data-a]'),function(b){
      var a=D.actions[+b.getAttribute('data-a')], e=D.edges[cur][+b.getAttribute('data-a')];
      b.disabled=!e;
      if(a.time) b.textContent=(n.due==null)? D.timeLabel : D.timeDueLabel.replace('{ms}',fmt(n.due));
    });
    [].forEach.call(lamps.querySelectorAll('.lamp'),function(l){ l.classList.toggle('lit',lastOut.indexOf(l.getAttribute('data-o'))>=0); });
    clock.textContent=D.elapsedLabel+': '+fmt(elapsed);
    done.hidden=!n.finished;
  }
  function press(i){
    var e=D.edges[cur][i]; if(!e) return;
    var before=D.nodes[cur], a=D.actions[i];
    cur=e[0]; lastOut=e[1].slice(); elapsed+=e[2];
    var moved=[]; D.lanes.forEach(function(l){
      var b=l.children.filter(function(c){return before.cfg.indexOf(c)>=0;})[0], c=l.children.filter(function(c){return D.nodes[cur].cfg.indexOf(c)>=0;})[0];
      /* No active child is "finished" when the whole machine has finished, and "nothing here" otherwise. */
      function word(s,node){ return s===undefined? (node.finished? D.finalLabel : '—') : nm(D.stateNames,s); }
      if(b!==c) moved.push(nm(D.laneNames,l.id)+': '+word(b,before)+' → '+word(c,D.nodes[cur]));
    });
    var line=a.label+(moved.length?' / '+moved.join(', '):'')+(lastOut.length?' / '+lastOut.map(function(o){return nm(D.outputNames,o);}).join(', '):'');
    logLines.unshift({t:line,out:lastOut.length>0});
    var h=''; logLines.slice(0,40).forEach(function(x){ h+='<li'+(x.out?' class="out"':'')+'>'+esc(x.t)+'</li>'; }); log.innerHTML=h;
    show();
  }
  root.addEventListener('click',function(e){ var b=e.target.closest('button[data-a]'); if(b) press(+b.getAttribute('data-a')); });
  root.querySelector('.sim-reset').addEventListener('click',function(){ cur=0; elapsed=0; lastOut=[]; logLines=[]; log.innerHTML=''; show(); });
  build(); show();
  window.__SIM_PROBE__=window.__SIM_PROBE__||{}; window.__SIM_PROBE__['__ID__']={press:press,state:function(){return {cur:cur,elapsed:elapsed,out:lastOut};},data:D};
})();
"""


def simulator_model(machine: Machine, graph: dict, labels: Labels, lang: str = "ko") -> dict:
    """What the page needs, from the explored graph: the lanes (containers whose child moves in the graph),
    the buttons (every signal and, when the document has timers, time passing) and the lookup."""
    lanes = []
    seen_cfg = [set(node["cfg"]) for node in graph["nodes"]]
    for container in machine.lane_containers():
        cells = {next((c.id for c in container.substates if c.id in cfg), None) for cfg in seen_cfg}
        if len(cells - {None}) > 1:
            lanes.append({"id": container.id, "children": [c.id for c in container.substates]})
    lanes = lanes[:MAX_LANES] or [{"id": machine.root.id, "children": [c.id for c in machine.root.substates]}]
    handled = [machine.events_under(machine.by_id.get(l["id"]) or machine.root) for l in lanes]
    actions = []
    for a in graph["alphabet"]:
        if a["kind"] == "time":
            actions.append({"time": True, "label": TEXT[lang]["time_button"], "lanes": []})
        else:
            words = labels.of("events", a["name"]) or a["name"]
            payload = a.get("payload")
            if isinstance(payload, dict) and payload:
                words += " (" + ", ".join(f"{k}={v}" for k, v in sorted(payload.items())) + ")"
            elif payload:
                words += f" ({payload})"
            # A signal's button sits in the panel of the lane that takes it; one no lane takes (a rule of its own,
            # such as a region that only watches for it) stays with the general buttons.
            actions.append({"time": False, "label": words,
                            "lanes": [i for i, events in enumerate(handled) if a["name"] in events]})
    outputs = sorted({o for row in graph["edges"] for e in row if e for o in e[1]})
    return {
        "lanes": lanes, "actions": actions, "nodes": graph["nodes"], "edges": graph["edges"], "outputs": outputs,
        "laneNames": {l["id"]: labels.of("lanes", l["id"]) or labels.of("states", l["id"]) or l["id"] or
                      (labels.title or machine.name) for l in lanes},
        "stateNames": {s: labels.of("states", s) or s for l in lanes for s in l["children"]},
        "stateNotes": {s: labels.note("states", s) for l in lanes for s in l["children"] if labels.note("states", s)},
        "outputNames": output_names(outputs, labels),
        "dataNames": {k: labels.of("data", k) or k for node in graph["nodes"] for k in node["data"]},
        "timeLabel": TEXT[lang]["time_button"], "timeDueLabel": TEXT[lang]["time_button_due"],
        "elapsedLabel": TEXT[lang]["elapsed"], "finalLabel": TEXT[lang]["final"],
    }


_instance = [0]


def simulator_html(machine: Machine, graph: dict, labels: Labels, lang: str = "ko") -> str:
    model = simulator_model(machine, graph, labels, lang)
    _instance[0] += 1
    ident = f"sv-sim-{_instance[0]}"
    script = SIMULATOR_SCRIPT.replace("__ID__", ident).replace("__DATA__", json.dumps(model, ensure_ascii=False))
    check = say(lang, "checked", steps=graph["checked"]["steps"], bad=graph["checked"]["differences"])
    return (f'<div class="sv sv-sim" id="{ident}"><p class="sv-note">{esc(say(lang, "press"))}</p>'
            f'<div class="sim-lanes"></div><div class="sim-data"></div>'
            f'<p class="sv-note">{esc(say(lang, "heard"))}</p><div class="sim-lamps"></div>'
            f'<div class="sim-btns"></div>'
            f'<div class="sim-bar"><button type="button" class="alt sim-reset">{esc(say(lang, "reset"))}</button>'
            f'<span class="sim-clock"></span><span class="sim-done" hidden>{esc(say(lang, "finished"))}</span></div>'
            f'<ol class="sim-log" reversed></ol><p class="sv-note">{esc(check)}</p>'
            f'<script>{script}</script></div>')


# ---------------------------------------------------------------------------- exploring the engine
def _in_the_engine(document, codegen, mode: str, extra: dict, alphabet=None) -> dict:
    """Build the design with the product's generator, then run `views_explore` on it in a child with
    the kernel's limits, and return what it printed. ⚠ The only place the engine is started."""
    import sys

    from .verify import _default_codegen, _scratch, generate

    document = pathlib.Path(document)
    codegen = pathlib.Path(codegen) if codegen else _default_codegen()
    machine = read_machine(document)
    with _scratch() as scratch:
        built = generate(document, codegen, scratch / "design", serves=tuple(machine.host_types))
        if built.refusal:
            raise ViewError(f"the product could not build {document.name}: {built.refusal}")
        request = {"mode": mode, "built": str(scratch / "design"), "document": str(document),
                   "processors": list(built.declared), **extra}
        if mode == "explore":
            request["alphabet"] = _alphabet(machine, built, extra.get("scenario_set"))
        # The walk stops itself at `seconds`; the kernel's limit is the backstop for a design that never returns.
        budget = float(extra.get("seconds") or 60.0)
        limits = process.Limits(wall_seconds=budget + 120.0, cpu_seconds=int(budget) + 110, memory_mb=2048,
                                output_mb=32, open_files=256)
        outcome = process.run_isolated([sys.executable, "-m", "sce_author.views_explore"], limits=limits,
                                       stdin_text=json.dumps(request),
                                       env={"PYTHONPATH": str(pathlib.Path(__file__).resolve().parents[1])})
    if outcome.stopped_by is not None or outcome.returncode != 0:
        raise ViewError(f"the engine could not be run: {outcome.said()}")
    try:
        answer = json.loads(outcome.stdout.strip().splitlines()[-1])
    except (IndexError, ValueError) as exc:
        raise ViewError("the engine run printed no result") from exc
    if "refused" in answer:
        raise ViewError(answer["refused"])
    return answer


def _alphabet(machine: Machine, built, scenario_set) -> list:
    """What a person can do to the machine: every signal it takes (the product's own list), each distinct
    payload an example sends with one, and time passing when the design has a timer."""
    surface = built.surface or {}
    # ⚠ A signal the machine gives itself (a timer's `deadline`, an internal `eval`) is not something a person
    # presses: it is what happens when time passes or another signal is handled. The product lists it as one a
    # caller COULD deliver, and a button for it would let the page do what the machine never does on its own.
    alphabet = [{"kind": "event", "name": name} for name in surface.get("inputs", [])
                if name not in machine.own_events]
    if scenario_set is not None:
        seen = set()
        for scenario in json.loads(pathlib.Path(scenario_set).read_text(encoding="utf-8"))["scenarios"]:
            for step in scenario.get("steps", []):
                if "send" in step and "payload" in step:
                    key = (step["send"], json.dumps(step["payload"], sort_keys=True))
                    if key not in seen:
                        seen.add(key)
                        alphabet.append({"kind": "event", "name": step["send"], "payload": step["payload"]})
    if built.needs_event_scheduler or machine.delayed:
        alphabet.append({"kind": "time"})
    return alphabet


def explore(document, codegen=None, *, scenario_set=None, max_nodes: int = 2000, max_depth: int = 40,
            seconds: float = 300.0, walks: int = 300, walk_length: int = 10, seed: int = 11) -> dict:
    """The behaviour of the design as the real engine produced it: every situation reachable by the
    signals the design takes (and time passing, when it has timers), what each signal does from each,
    and what was sent. `{"nodes", "edges", "alphabet", "checked"}`, or a ViewError saying why the design
    has too many situations, never closes, or is not captured exactly."""
    return _in_the_engine(document, codegen, "explore",
                          {"max_nodes": max_nodes, "max_depth": max_depth, "seconds": seconds, "walks": walks,
                           "walk_length": walk_length, "seed": seed,
                           "scenario_set": str(scenario_set) if scenario_set is not None else None})


def play(document, scenarios: list, codegen=None) -> dict:
    """Each scenario's steps played on the real engine: `{"runs": [{"id", "observations"} | {"id", "refused"}]}`.
    The first observation is the machine as it starts."""
    return _in_the_engine(document, codegen, "play", {"scenarios": scenarios})


# ---------------------------------------------------------------------------- the page
STYLE = """
.sv{--sv-ink:var(--ink,#111820);--sv-ink2:var(--ink2,#58646F);--sv-ink3:var(--ink3,#8592A0);--sv-rule:var(--rule,#D2D9E0);
 --sv-sunk:var(--sunk,#E4E9EE);--sv-slide:var(--slide,#fff);--sv-accent:var(--accent,#1D5D9B);--sv-accent-soft:var(--accent-soft,#E3EDF7);
 color:var(--sv-ink);font-size:13px;line-height:1.55}
.sv .c0{--bg:var(--sunk,#E4E9EE);--fg:var(--ink2,#58646F)} .sv .c1{--bg:var(--accent-soft,#E3EDF7);--fg:var(--accent,#1D5D9B)}
.sv .c2{--bg:var(--wip-soft,#FBF0DC);--fg:var(--wip,#8F5D0E)} .sv .c3{--bg:var(--pass-soft,#E0F0E8);--fg:var(--pass,#1B7351)}
.sv .c4{--bg:var(--fail-soft,#F7E4E3);--fg:var(--fail,#A32A28)} .sv .c5{--bg:#EDE7F6;--fg:#5E35B1}
.sv .c6{--bg:#E0F2F1;--fg:#00796B} .sv .c7{--bg:#FCE4EC;--fg:#AD1457}
.sv h4{margin:14px 0 4px;font-size:13.5px} .sv h5{margin:0 0 6px;font-size:12.5px;font-family:"IBM Plex Mono",monospace}
.sv .sv-note{margin:2px 0 6px;color:var(--sv-ink2);font-size:12px} .sv .sv-scroll{overflow-x:auto}
.sv .nm{font-weight:600} .sv .id{display:block;font-family:"IBM Plex Mono",monospace;font-size:10px;color:var(--sv-ink3);font-weight:400}
.sv .id0{font-family:"IBM Plex Mono",monospace;font-size:11.5px;font-weight:500}
.sv .sv-note .id{display:inline;margin-left:4px}
.sv .note1{display:block;margin-top:1px;font-size:10.5px;line-height:1.35;color:var(--sv-ink3);font-weight:400;white-space:normal;min-width:11em}
.sv-table td.quiet{color:var(--sv-ink3);font-size:11px;text-align:center;vertical-align:middle;background:transparent}
.sv-table td.moves{background:var(--sv-accent-soft)} .sv-table td.moves .nm{font-weight:700}
.sv-rules td:nth-child(2){min-width:18em}
.sv-table{border-collapse:collapse;width:100%;font-size:12px}
.sv-table th,.sv-table td{border:1px solid var(--sv-rule);padding:5px 8px;vertical-align:top;text-align:left}
.sv-table thead th{background:var(--sv-sunk);font-weight:600} .sv-table tbody th{background:var(--sv-sunk);font-weight:500;white-space:nowrap}
.sv-table .cond{color:var(--sv-ink2);font-style:italic} .sv-table .act{color:var(--sv-ink2)} .sv-table .stay{color:var(--sv-ink3)}
.sv-table .int{display:block;font-size:10.5px;color:var(--sv-ink3)} .sv-table .or{border-top:1px dashed var(--sv-rule);margin:4px 0}
.sv-fold{border-top:1px solid var(--sv-rule);padding:6px 0} .sv-fold>summary{cursor:pointer;font-weight:600;font-size:12.5px}
.sv-fold>summary .sv-note{font-weight:400;margin-left:6px}
.sv-figure{margin:12px 0} .sv-figure figcaption{font-size:12.5px} .sv-quote{margin:2px 0 6px;color:var(--sv-ink2);font-size:12.5px}
.sv-svg{display:block;max-width:none;height:auto;font-family:"IBM Plex Sans KR",system-ui,sans-serif}
.sv-svg .guide{stroke:var(--sv-rule);stroke-dasharray:3 4} .sv-svg .ink{fill:var(--sv-ink)} .sv-svg .faint{fill:var(--sv-ink3)}
.sv-svg .bar{fill:var(--bg);stroke:var(--fg)} .sv-svg .bar-ink{fill:var(--fg)} .sv-svg .idle{fill:none;stroke:var(--sv-rule);stroke-dasharray:3 3}
.sv-svg .val{fill:var(--sv-sunk);stroke:var(--sv-rule)} .sv-svg .out{fill:var(--sv-accent)} .sv-svg .out-ink{fill:#fff}
.sv-sim .sim-lanes{display:grid;grid-template-columns:repeat(auto-fit,minmax(230px,1fr));gap:10px}
.sv-sim .pn{border:1px solid var(--sv-rule);border-radius:8px;padding:8px 10px;background:var(--sv-slide)}
.sv-sim .chips{display:flex;flex-wrap:wrap;gap:4px} .sv-sim .pn-btns{display:flex;flex-wrap:wrap;gap:5px;margin-top:8px}
.sv-sim .pn-btns button{font-size:11.5px;padding:3px 8px}
.sv-sim .st{display:inline-block;border-radius:5px;padding:2px 9px;font-size:12px;font-weight:600;background:var(--bg);color:var(--fg);opacity:.38}
.sv-sim .st.on{opacity:1;outline:2px solid var(--sv-ink);outline-offset:1px}
.sv-sim .sim-data{display:flex;gap:10px;flex-wrap:wrap;margin:8px 0;font-size:12px} .sv-sim .dv{background:var(--sv-sunk);border-radius:5px;padding:2px 9px}
.sv-sim .sim-lamps{display:flex;gap:8px;flex-wrap:wrap;margin:10px 0}
.sv-sim .lamp{border:1px solid var(--sv-rule);border-radius:8px;padding:5px 12px;font-size:12.5px;font-weight:600;color:var(--sv-ink3)}
.sv-sim .lamp.lit{background:var(--sv-accent);border-color:var(--sv-accent);color:#fff}
.sv-sim .sim-btns{display:flex;gap:6px;flex-wrap:wrap}
.sv-sim button{font:inherit;font-size:12.5px;font-weight:600;padding:5px 11px;border-radius:6px;border:1px solid var(--sv-accent);background:var(--sv-accent-soft);color:var(--sv-accent);cursor:pointer}
.sv-sim button:hover:not(:disabled){background:var(--sv-accent);color:#fff} .sv-sim button:disabled{opacity:.4;cursor:not-allowed}
.sv-sim button.alt{border-color:var(--sv-ink3);background:var(--sv-sunk);color:var(--sv-ink2)}
.sv-sim button:focus-visible{outline:2px solid var(--sv-ink);outline-offset:2px}
.sv-sim .sim-bar{display:flex;gap:12px;align-items:center;margin:10px 0 0;font-size:12px;flex-wrap:wrap}
.sv-sim .sim-log{margin:8px 0 0;padding-left:24px;font-size:12px;max-height:170px;overflow:auto} .sv-sim .sim-log li.out{color:var(--sv-accent);font-weight:600}
"""


def standalone(title: str, body: str) -> str:
    return (f'<!doctype html><html lang="ko"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">'
            f'<title>{esc(title)}</title><style>body{{margin:0;padding:24px 16px;font-family:"IBM Plex Sans KR",system-ui,sans-serif;'
            f'background:#fff;color:#111820}}main{{max-width:1120px;margin:0 auto}}{STYLE}</style><main>{body}</main></html>')


def build(document, out_dir, *, scenario_set=None, labels_path=None, codegen=None, lang: str = "ko",
          simulator: bool = True, internal: bool = False) -> list:
    """Write `table.html`, `timeline.html` and `simulator.html` for `document` into `out_dir`.
    A page that cannot be made is left out and the reason is returned with the paths: a refusal to draw
    one view never takes the others with it."""
    document, out_dir = pathlib.Path(document), pathlib.Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    machine, labels = read_machine(document), Labels.read(labels_path)
    written, said = [], []

    def write(name: str, body: str) -> None:
        path = out_dir / name
        path.write_text(standalone(labels.title or machine.name, body), encoding="utf-8")
        written.append(str(path))

    write("table.html", table_html(machine, labels, lang, internal=internal))
    if scenario_set is not None:
        try:
            body, notes = timelines_html(document, scenario_set, labels, codegen, lang)
            write("timeline.html", body)
            said += notes
        except ViewError as exc:
            said.append(f"timeline: {exc}")
    if simulator:
        try:
            write("simulator.html", simulator_html(machine, explore(document, codegen, scenario_set=scenario_set),
                                                   labels, lang))
        except ViewError as exc:
            said.append(say(lang, "too_big", why=str(exc)))
    return written + said


def timelines_html(document, scenario_set, labels: Labels, codegen=None, lang: str = "ko", fold: bool = False) -> tuple:
    """A timeline for each runnable scenario of the set, and a line for each one that was not drawn.

    Only a scenario the set calls runnable is played: one that waits on a decision (`awaiting-decision`) or
    on a fact nobody has (`blocked`) describes behaviour the owner has not settled, and drawing it would
    show a machine doing what nobody has said it should."""
    machine = read_machine(document)
    spec = json.loads(pathlib.Path(scenario_set).read_text(encoding="utf-8"))
    chosen = [s for s in spec["scenarios"] if s.get("status", "runnable") == "runnable"]
    notes = [f"scenario {s['id']}: not drawn ({s.get('status')}: it waits on a decision nobody has made)"
             for s in spec["scenarios"] if s.get("status", "runnable") != "runnable"]
    runs = {r["id"]: r for r in play(document, chosen, codegen)["runs"]}
    parts = []
    for scenario in chosen:
        run = runs[scenario["id"]]
        if "refused" in run:
            notes.append(f"scenario {scenario['id']}: not drawn ({run['refused']})")
            continue
        figure = timeline_html(machine, timeline_model(machine, scenario, run, lang), labels, lang, titled=not fold)
        if fold:
            # A long set is a list of one-line headings with the first open: the reader opens the one they want.
            figure = (f'<details class="sv-fold"{" open" if not parts else ""}><summary>{esc(scenario["id"])}'
                      f' <span class="sv-note">{esc(scenario.get("quote", ""))}</span></summary>{figure}</details>')
        parts.append(figure)
    return f'<div class="sv sv-timelines">{"".join(parts)}</div>', notes
