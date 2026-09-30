"""The owner's decision record, and a draft held to it.

A specification leaves things open, and every draft of it meets those places
on its own: measured 2026-09-29, five drafts of one specification marked
between four and eleven open questions each, the same question spelled up to
three ways, and no two drafts the same set. The decision record is where the
owner answers each question ONCE, beside the specification, and a draft cites
the answer by its id instead of guessing again:

    sce:assumed="D1"      the draft applied decision D1
    sce:unresolved="D3"   the draft asks D3, which the owner has not answered

`hold` reads the draft's markers through the product (`sce-codegen
unresolved`, the same reader `--strict-unresolved` uses, forge expansion
included) and answers, marker by marker, whether the draft keeps to the
record. What it REFUSES is a draft that guessed where no answer licenses it:

    uncited-guess              an `sce:assumed` naming no decision in the record
    guess-on-an-open-question  an `sce:assumed` naming a question not answered
    answered-left-open         an `sce:unresolved` naming an answered decision
    not-a-candidate            the answer's value is not among the candidates
                               the draft lists for it
    holds-another-value        the draft's decision variable holds a value
                               other than the one the owner chose

What it only REPORTS is what a person has to read:

    new-question     an `sce:unresolved` the record has never seen -- the
                     specification may have a gap nobody saw, so it is never
                     refused; it is shown beside every recorded question on
                     the same clause, since whether two sentences ask the
                     same thing is the owner's reading, not this check's
    still-open       an `sce:unresolved` citing a question still unanswered
    applied          an `sce:assumed` citing an answer, where it holds
    house-rule       an `sce:assumed` citing a house rule of the authoring
                     profile the draft is held to -- the owner's standing
                     answer to a gap that recurs, marked by the product, so
                     not an uncited guess; a decision of the same id wins
    answer-not-read  the answer is a value and the draft keeps it where no
                     value can be read, so a person reads it against the answer
    uncited-answer   an answered decision no marker cites -- the draft either
                     needed no guess there or applied it without saying so

⚠ What this cannot do is recognise an answered question asked again under a
new id: whether `timeout-reset` is D4 is a reading of two sentences. The
anchor is what makes that reading cheap -- a new question comes back beside
the recorded ones on its clause -- and the owner makes it.
"""

from __future__ import annotations

import dataclasses
import json
import pathlib

from .check import read_document
from .counterfactual import candidate_site
from .errors import READ_ERRORS, AuthoringError, describe_path
from .pack import _validate
from .verify import requirement_records, unresolved_markers

RECORD_SCHEMA = "decisions.v1.schema.json"

REFUSING = ("uncited-guess", "guess-on-an-open-question", "answered-left-open",
            "not-a-candidate", "holds-another-value")
REPORTED = ("new-question", "still-open", "applied", "house-rule", "answer-not-read",
            "uncited-answer")


class DecisionRecordError(AuthoringError):
    """A decision record could not be used. Names the file and what was wrong."""


@dataclasses.dataclass(frozen=True)
class Decision:
    id: str
    question: str
    answer: str | None = None
    anchor: dict | None = None
    candidates: tuple[str, ...] = ()
    chosen: str | None = None

    @property
    def answered(self) -> bool:
        return self.answer is not None


@dataclasses.dataclass(frozen=True)
class DecisionRecord:
    path: pathlib.Path
    specification: dict
    # In the record's own order, which is the order the owner answered in.
    decisions: dict[str, Decision]


def load_record(path: pathlib.Path) -> DecisionRecord:
    """Read and validate a decision record, or refuse it naming the file."""
    path = pathlib.Path(path)
    try:
        text = path.read_text(encoding="utf-8")
    except READ_ERRORS as exc:
        if not path.is_file():
            raise DecisionRecordError(describe_path(path)) from exc
        raise DecisionRecordError(f"{path}: cannot be read as text ({exc})") from exc
    try:
        doc = json.loads(text)
    except json.JSONDecodeError as exc:
        raise DecisionRecordError(f"{path}: not well-formed JSON ({exc})") from exc
    _validate(doc, RECORD_SCHEMA, path, error=DecisionRecordError)
    decisions: dict[str, Decision] = {}
    for entry in doc["decisions"]:
        ident = entry["id"]
        if ident in decisions:
            raise DecisionRecordError(
                f"{path}: decision {ident!r} is recorded twice; a draft citing "
                f"it could not say which answer it applied")
        decision = Decision(
            id=ident, question=entry["question"], answer=entry.get("answer"),
            anchor=entry.get("anchor"), candidates=tuple(entry.get("candidates", ())),
            chosen=entry.get("chosen"))
        if decision.chosen is not None and decision.candidates \
                and decision.chosen not in decision.candidates:
            raise DecisionRecordError(
                f"{path}: decision {ident!r} chose {decision.chosen!r}, which is "
                f"not among its candidates ({' '.join(decision.candidates)})")
        decisions[ident] = decision
    return DecisionRecord(path=path, specification=doc["specification"],
                          decisions=decisions)


@dataclasses.dataclass
class Finding:
    finding: str
    message: str
    marker: str | None = None
    decision: str | None = None
    node_path: str | None = None
    location: dict | None = None
    # Recorded decisions on the same clause as a new question.
    alongside: list[str] = dataclasses.field(default_factory=list)

    @property
    def refuses(self) -> bool:
        return self.finding in REFUSING

    def as_dict(self) -> dict:
        out = {"finding": self.finding, "refuses": self.refuses, "message": self.message}
        for key in ("marker", "decision", "node_path", "location"):
            if getattr(self, key) is not None:
                out[key] = getattr(self, key)
        if self.alongside:
            out["alongside"] = self.alongside
        return out


def _clause(anchor: dict) -> tuple:
    """Which clause an anchor names: its document and its division. The
    position inside the division is left out, so a question on the same
    clause is shown even when the two anchors point at different lines of
    it -- a pairing shown that is not one costs the owner a glance, and one
    not shown costs them the question."""
    return anchor.get("doc_id"), anchor.get("section")


def _ancestors(node_path: str):
    """`states.closed.transitions[0]`, then `states.closed`, then `states`."""
    while node_path:
        yield node_path
        if node_path.endswith("]") and "[" in node_path:
            node_path = node_path[:node_path.rindex("[")]
        elif "." in node_path:
            node_path = node_path[:node_path.rindex(".")]
        else:
            return


def _where(marker: dict) -> str:
    location = marker.get("location") or {}
    line = f"line {location['line']}" if location.get("line") else None
    return ", ".join(p for p in (marker.get("node_path"), line) if p) or "the document"


def _value_findings(marker: dict, decision: Decision, declared) -> Finding:
    """What the draft holds for an answer that is a value."""
    ident = marker["id"]
    if marker.get("candidates") and decision.chosen not in marker["candidates"]:
        return Finding(
            "not-a-candidate",
            f"{ident} was answered {decision.chosen!r}, and the draft lists "
            f"{' '.join(marker['candidates'])} as the values it could take: "
            f"list the answer among them and hold it",
            marker=ident, decision=ident)
    sites = [data for data, cited in declared.assumed_marker.items() if cited == ident]
    if not sites:
        return Finding(
            "answer-not-read",
            f"{ident} was answered {decision.chosen!r}, and the draft cites it "
            f"on a {marker.get('node_type', 'node')}, where no value is read: "
            f"read {_where(marker)} against the answer, or give the value a "
            f"<data> of its own that the logic reads",
            marker=ident, decision=ident)
    for data in sites:
        listed = declared.assumed_candidates.get(data)
        if not listed:
            return Finding(
                "answer-not-read",
                f"<data id={data!r}> cites {ident} and lists no "
                f"sce:assumed-candidates, so which value it holds cannot be "
                f"read: list them, the answer {decision.chosen!r} among them",
                marker=ident, decision=ident)
        candidates, expr, initial = listed
        _, held, why = candidate_site(expr, candidates, initial)
        if why:
            return Finding("answer-not-read", f"<data id={data!r}>: {why}",
                           marker=ident, decision=ident)
        if held != decision.chosen:
            return Finding(
                "holds-another-value",
                f"<data id={data!r}> holds {held!r} for {ident}, which the "
                f"owner answered {decision.chosen!r} ({decision.answer})",
                marker=ident, decision=ident)
    return Finding("applied", f"{ident} holds the owner's answer {decision.chosen!r}",
                   marker=ident, decision=ident)


def judge(markers: list[dict], anchors: dict, record: DecisionRecord, declared) -> list[Finding]:
    """Every finding about a draft's markers against the record, in the
    product's document order and then the record's.

    `markers` are the product's `unresolved` records; `anchors` maps a node
    path to the `spec_provenance` anchors the product read on that node;
    `declared` is the document as `check.read_document` reads it, for the
    value a decision variable holds."""
    findings: list[Finding] = []
    cited: set[str] = set()
    for marker in markers:
        ident, kind = marker.get("id"), marker.get("kind")
        decision = record.decisions.get(ident)
        where = _where(marker)
        base = {"marker": ident, "node_path": marker.get("node_path"),
                "location": marker.get("location")}
        if decision is not None:
            cited.add(ident)
        if kind == "assumed":
            if decision is None and marker.get("house_rule"):
                # The owner's standing answer, marked by the product because
                # the id is a house rule of the profile the draft is held to.
                # A decision of the same id would win (RFC 5.8: a decision
                # overrides a house rule for its clause), which is why this is
                # asked only when the record holds none.
                findings.append(Finding(
                    "house-rule",
                    f"sce:assumed=\"{ident}\" at {where} applies the profile's "
                    f"house rule {ident}: the owner's standing answer, not the "
                    f"specification's -- read it against the rule", **base))
            elif decision is None:
                findings.append(Finding(
                    "uncited-guess",
                    f"sce:assumed=\"{ident}\" at {where} cites no decision in "
                    f"the record: ask the owner, record the answer, and cite "
                    f"its id", **base))
            elif not decision.answered:
                findings.append(Finding(
                    "guess-on-an-open-question",
                    f"{ident} is asked in the record and not answered yet "
                    f"(\"{decision.question}\"): a draft may ask it, as "
                    f"sce:unresolved=\"{ident}\", not guess it", decision=ident, **base))
            elif decision.chosen is not None:
                found = _value_findings(marker, decision, declared)
                found.node_path, found.location = base["node_path"], base["location"]
                findings.append(found)
            else:
                findings.append(Finding(
                    "applied",
                    f"{ident} is cited at {where}; read it against the answer "
                    f"(\"{decision.answer}\")", decision=ident, **base))
            continue
        if decision is None:
            clauses = {_clause(a) for path in _ancestors(marker.get("node_path") or "")
                       for a in anchors.get(path, ())}
            alongside = [d.id for d in record.decisions.values()
                         if d.anchor is not None and _clause(d.anchor) in clauses]
            also = (f"; the record already holds {', '.join(alongside)} on the "
                    f"same clause -- ask the owner whether it is one of them"
                    if alongside else "")
            findings.append(Finding(
                "new-question",
                f"{ident} at {where} is a question the record has not seen "
                f"({marker.get('reason') or 'no reason given'}){also}",
                alongside=alongside, **base))
        elif decision.answered:
            findings.append(Finding(
                "answered-left-open",
                f"{ident} is answered (\"{decision.answer}\"), and the draft "
                f"still asks it at {where}: apply the answer and cite it as "
                f"sce:assumed=\"{ident}\"", decision=ident, **base))
        else:
            findings.append(Finding(
                "still-open", f"{ident} at {where} is still unanswered "
                              f"(\"{decision.question}\")", decision=ident, **base))
    for decision in record.decisions.values():
        if decision.answered and decision.id not in cited:
            findings.append(Finding(
                "uncited-answer",
                f"{decision.id} is answered and no marker cites it: the draft "
                f"needed no guess there, or applied the answer without saying "
                f"so -- read the clause (\"{decision.question}\")",
                decision=decision.id))
    return findings


def hold(document: pathlib.Path, record_path: pathlib.Path,
         codegen: pathlib.Path | None = None, *,
         profile: pathlib.Path | None = None,
         cwd: pathlib.Path | None = None) -> tuple[str, str]:
    """(report, "") when the draft keeps to the record, ("", report) when it
    does not or the product could not read it -- the shape every
    product-backed answer in this core has.

    `profile` is the owner's authoring profile the draft is held to. A guess
    that cites one of its house rules is the owner's standing answer and is
    licensed like a decision's citation, and reported as the rule it applies
    (`house-rule`); every other uncited guess is refused as before."""

    def here(path: pathlib.Path) -> pathlib.Path:
        path = pathlib.Path(path)
        return path if cwd is None or path.is_absolute() else pathlib.Path(cwd) / path

    record = load_record(here(record_path))
    markers_text, refused = unresolved_markers(document, codegen, profile=here(profile) if profile else None, cwd=cwd)
    if refused:
        return "", refused
    markers = json.loads(markers_text)["markers"] or []
    anchors: dict = {}
    rows_text, rows_refused = requirement_records(document, None, codegen, cwd=cwd)
    anchors_read = not rows_refused
    if anchors_read:
        for row in json.loads(rows_text)["records"] or []:
            if row.get("spec_provenance"):
                anchors[row["node_path"]] = row["spec_provenance"]
    findings = judge(markers, anchors, record, read_document(here(document)))
    refuses = [f for f in findings if f.refuses]
    new = [f for f in findings if f.finding == "new-question"]
    report = {
        "verdict": "refused" if refuses else "holds",
        "document": str(document),
        "decisions": str(record_path),
        "specification": record.specification,
        "findings": [f.as_dict() for f in findings],
        "counts": {name: sum(1 for f in findings if f.finding == name)
                   for name in REFUSING + REPORTED
                   if any(f.finding == name for f in findings)},
    }
    if not anchors_read:
        report["anchors_not_read"] = json.loads(rows_refused)["diagnostics"]
    steps = []
    if refuses:
        steps.append("apply each refused finding to the draft and hold it again")
    if new:
        steps.append("ask the owner each new question -- beside any recorded one "
                     "on the same clause -- and record the answer under an id "
                     "the draft then cites")
    report["next"] = "; ".join(steps) if steps else "the draft keeps to the record"
    text = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    return ("", text) if refuses else (text, "")


def summary(report: dict) -> str:
    """The report for a reader, one line per finding."""
    lines = [f"decisions: {report['verdict']} -- {report['document']} against "
             f"{report['decisions']}"]
    for finding in report["findings"]:
        mark = "REFUSED" if finding["refuses"] else finding["finding"]
        lines.append(f"  {mark}: {finding['message']}")
    if "anchors_not_read" in report:
        lines.append("  (the product could not read the document's anchors, so "
                     "new questions are shown without the recorded ones on "
                     "their clause)")
    lines.append(f"next: {report['next']}")
    return "\n".join(lines)
