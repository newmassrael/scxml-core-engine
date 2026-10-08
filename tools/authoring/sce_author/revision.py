"""Join what a revision did to the WORDS of each requirement with what it did to the
design's EVIDENCE, and say whether the revision stayed within its reach.

Two facts exist about every requirement of a revised specification and neither is enough
alone (`docs/adr/0009-a-revision-stays-within-the-reach-of-what-changed.md`):

    words      carried / changed / new / retired -- the `delta` that `scxml_requirement_set`
               returns when the revised list is built against its lineage;
    evidence   unchanged / changed / new / dropped -- `scxml_acceptance_delta`, the accepted
               record against the design now.

A requirement whose words did not change and whose evidence did is a design that moved with
nothing asking it to; one whose words changed and whose evidence did not is a design that may not
have heard. Joined by requirement id the two are what an owner needs to look at again, and the
rest -- both read the same -- is what they do not.

Pure: two JSON objects in, one out. The design and the record are read by the product, and the
words by the lineage; nothing here opens a file.

⚠ `carries-over` means the product's closure of what a requirement depends on reads the same
(ADR 0007), not that the requirement is met, and an acceptance still lapses by its bytes. The page
says so on its face.
"""

from __future__ import annotations

from .errors import AuthoringError

WORDS = ("carried", "changed", "new", "retired")
EVIDENCE = ("unchanged", "changed", "new", "dropped")

OK, LOOK, VIOLATION = "ok", "look", "violation"


class RevisionError(AuthoringError):
    """A delta that is not one this module can read."""


def _words_of(delta: object) -> dict[str, str]:
    """requirement id -> its words status, from the `delta` `scxml_requirement_set` returned."""
    if not isinstance(delta, dict) or not isinstance(delta.get("requirements"), dict):
        raise RevisionError("the words `delta` has to be the object scxml_requirement_set returned for "
                            "a revision: it has no `requirements`. A first list has no delta, and there "
                            "is nothing to revise from")
    found = delta["requirements"]
    words: dict[str, str] = {}

    def put(id_: object, status: str) -> None:
        if not isinstance(id_, str) or not id_:
            raise RevisionError(f"the words delta names a requirement that is not an id: {id_!r}")
        if id_ in words:
            raise RevisionError(f"the words delta names {id_} twice ({words[id_]} and {status})")
        words[id_] = status

    for status in ("carried", "new", "retired"):
        listed = found.get(status)
        if not isinstance(listed, list):
            raise RevisionError(f"the words delta has no `{status}` list")
        for id_ in listed:
            put(id_, status)
    listed = found.get("changed")
    if not isinstance(listed, list):
        raise RevisionError("the words delta has no `changed` list")
    for entry in listed:
        put(entry.get("id") if isinstance(entry, dict) else entry, "changed")
    return words


def _evidence_of(delta: object) -> tuple[dict[str, dict], dict]:
    """requirement id -> its evidence line, and the unclaimed line, from `scxml_acceptance_delta`."""
    if not isinstance(delta, dict) or not isinstance(delta.get("requirements"), list):
        raise RevisionError("the evidence delta has to be the object scxml_acceptance_delta returned: "
                            "it has no `requirements` list")
    lines: dict[str, dict] = {}
    for line in delta["requirements"]:
        id_ = line.get("requirement") if isinstance(line, dict) else None
        if not isinstance(id_, str) or line.get("evidence") not in EVIDENCE:
            raise RevisionError(f"an evidence line is not {{requirement, evidence}} with evidence one of "
                                f"{', '.join(EVIDENCE)}: {line!r}")
        if id_ in lines:
            raise RevisionError(f"the evidence delta names {id_} twice")
        lines[id_] = line
    unclaimed = delta.get("unclaimed") or {"added": [], "gone": 0}
    if not isinstance(unclaimed, dict) or not isinstance(unclaimed.get("added", []), list):
        raise RevisionError("the evidence delta's `unclaimed` is not {added, gone}")
    return lines, unclaimed


# (words, evidence) -> (kind, severity). `None` stands for "no evidence line", which is a
# requirement no node cites in the record or in the design.
_TABLE: dict[tuple[str, str | None], tuple[str, str]] = {
    ("carried", "unchanged"): ("carries-over", OK),
    ("carried", "changed"): ("moved-without-reason", VIOLATION),
    ("carried", "dropped"): ("moved-without-reason", VIOLATION),
    ("carried", "new"): ("newly-cited", LOOK),
    ("carried", None): ("carries-over", OK),
    ("changed", "changed"): ("revised", OK),
    ("changed", "unchanged"): ("words-changed-design-same", LOOK),
    ("changed", "dropped"): ("changed-but-uncited", LOOK),
    ("changed", None): ("changed-but-uncited", LOOK),
    ("changed", "new"): ("revised", OK),
    ("new", "new"): ("implemented-new", OK),
    ("new", "changed"): ("implemented-new", OK),
    ("new", "unchanged"): ("unimplemented-new", LOOK),
    ("new", "dropped"): ("unimplemented-new", LOOK),
    ("new", None): ("unimplemented-new", LOOK),
    ("retired", "dropped"): ("retired-cleanly", OK),
    ("retired", None): ("retired-cleanly", OK),
    ("retired", "unchanged"): ("retired-still-cited", VIOLATION),
    ("retired", "changed"): ("retired-still-cited", VIOLATION),
    ("retired", "new"): ("retired-still-cited", VIOLATION),
}


def join(words_delta: object, evidence_delta: object) -> dict:
    """The words and the evidence of a revision, joined by requirement id.

    ⚠ A requirement the words delta calls `carried` and no node cites, in the record or now, is
    `carries-over`: nothing about it moved, and "not cited" is not a finding of this revision
    (`scxml_requirements` already calls it `missing`)."""
    words = _words_of(words_delta)
    evidence, unclaimed = _evidence_of(evidence_delta)
    rows = []
    for id_ in sorted(set(words) | set(evidence)):
        w, line = words.get(id_), evidence.get(id_)
        e = line["evidence"] if line else None
        kind, severity = _TABLE.get((w, e), ("unlisted", LOOK)) if w else ("unlisted", LOOK)
        row = {"requirement": id_, "words": w or "unlisted", "evidence": e or "none",
               "kind": kind, "severity": severity}
        if line:
            for key in ("moved", "at", "gone"):
                if key in line:
                    row[key] = line[key]
        rows.append(row)
    added = list(unclaimed.get("added") or [])
    summary = {kind: sum(1 for r in rows if r["kind"] == kind) for kind in sorted({r["kind"] for r in rows})}
    violations = [r for r in rows if r["severity"] == VIOLATION]
    return {
        "verdict": "outside-reach" if violations else "within-reach",
        "summary": {
            "requirements": len(rows),
            "violations": len(violations),
            "look": sum(1 for r in rows if r["severity"] == LOOK) + (1 if added else 0),
            "ok": sum(1 for r in rows if r["severity"] == OK),
            "kinds": summary,
        },
        "requirements": rows,
        "unclaimed": {"added": added, "gone": int(unclaimed.get("gone") or 0)},
    }


_WHY = {
    "moved-without-reason": "its words did not change and the design moved",
    "retired-still-cited": "the specification dropped it and the design still cites it",
    "newly-cited": "its words did not change and the design now cites it",
    "words-changed-design-same": "its words changed and the design reads the same",
    "changed-but-uncited": "its words changed and no node cites it",
    "unimplemented-new": "it is new and nothing in the design carries it",
    "unlisted": "the design cites it and the revised list does not have it",
    "revised": "its words and the design both moved",
    "implemented-new": "it is new and the design now carries it",
    "retired-cleanly": "the specification dropped it and no node cites it",
}


def render(result: dict, sentences: dict[str, str] | None = None, *, title: str = "") -> str:
    """The page an owner reads: only what to look at again, and the requirements that carry over
    folded into one line. `sentences` (id -> the specification's sentence) is printed only when
    given, and the page then says it carries someone else's sentences."""
    rows = result["requirements"]
    carried = [r["requirement"] for r in rows if r["kind"] == "carries-over"]
    others = [r for r in rows if r["kind"] != "carries-over"]
    out = [f"# Revision report{': ' + title if title else ''}", ""]
    out.append(f"Verdict: {result['verdict']}. "
               + ("The design moved only where the words did." if result["verdict"] == "within-reach"
                  else f"{result['summary']['violations']} requirement(s) moved where the words did not "
                       "ask them to, or are cited though the specification dropped them."))
    out += ["",
            "`carries over` means the words read the same and so does the product's closure of what the "
            "requirement depends on. It does not say the requirement is met, and the acceptance still "
            "lapses by its bytes: accepting again is the owner's.", ""]
    if sentences is not None:
        out += ["⚠ This page carries sentences of the specification. It is a local artefact: do not "
                "check it into a repository.", ""]
    for heading, severity in (("Outside the revision's reach", VIOLATION), ("Look again", LOOK),
                              ("Changed as the words asked", OK)):
        group = [r for r in others if r["severity"] == severity]
        if not group:
            continue
        out += [f"## {heading} ({len(group)})", ""]
        for r in group:
            line = f"- {r['requirement']} -- {r['kind']}: {_WHY.get(r['kind'], r['kind'])}"
            line += f" (words {r['words']}, evidence {r['evidence']})"
            places = list(r.get("moved") or r.get("at") or ())
            if places:
                line += "; now at " + ", ".join(places)
            if r.get("gone"):
                line += f"; {r['gone']} recorded row(s) gone"
            out.append(line)
            if sentences is not None and sentences.get(r["requirement"]):
                out.append(f"    \"{sentences[r['requirement']]}\"")
        out.append("")
    added = result["unclaimed"]["added"]
    if added:
        out += [f"## Rows that claim no requirement (new: {len(added)})", "",
                "Behaviour no sentence asked for: " + ", ".join(added), ""]
    out += [f"## Carries over ({len(carried)})", "",
            (", ".join(carried) if carried else "none"), ""]
    return "\n".join(out)
