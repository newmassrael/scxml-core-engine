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

⚠ A requirement that no node cites, in the record or now, has no closure to read: it is `uncited`
and `uncovered`, never `carries-over`. Folding it into "carries over" would say the design was
checked against it when there was nothing to check (found in review, 2026-10-08). For a kind of
document with nowhere to cite a requirement every requirement is `uncited`, and `summary.seen` is
0: the check saw none of them, and the page says so.

⚠ A carried requirement whose evidence moved is a `moved-without-reason` violation unless every
place it moved is one that a requirement whose words changed (or that is new) also stands on. A node
is cited by several requirements, so asking one to change moves the others' evidence; that case is
`moved-with-a-changed-neighbour`, a `look` that names the neighbour (found in a real trial,
2026-10-08, where it made a correct in-place revision read as a violation).
"""

from __future__ import annotations

from .errors import AuthoringError

WORDS = ("carried", "changed", "new", "retired")
EVIDENCE = ("unchanged", "changed", "new", "dropped")

OK, LOOK, VIOLATION, UNCOVERED = "ok", "look", "violation", "uncovered"


class RevisionError(AuthoringError):
    """A delta that is not one this module can read."""


def belongs_to(delta: object, record: object) -> None:
    """Refuse a words delta that is not the one of the revision this acceptance was taken against.

    The join is by requirement id, and an id means nothing outside the specification that issued
    it: another specification's delta that happens to name `R1` to `R5` joins the design's evidence
    without a word of complaint and gives a verdict about nothing (found in review, 2026-10-08).
    A delta names the specification it describes (`doc_id`), the revision it starts from
    (`from_rev`) and the digests of the manifest and the sidecar of that revision's list
    (`from_manifest_sha256`, `from_sidecar_sha256`); the record pins the manifest it was taken
    against (`manifest.doc_id`, `manifest.rev`, `manifest.sha256`) and the sidecar beside it
    (`manifest.sidecar_sha256`). All have to agree: a name and a number are carried by every copy
    of a specification, and only the digest says it is THIS list (a second review, 2026-10-08).
    The manifest's digest is not enough: a manifest is coordinates only (an id, a section, a
    modality), so a list of another specification with the same shape has it too, and its delta
    would join this design's evidence; the sidecar's digest, over the words behind the ids, is
    what tells the two apart (a review, 2026-10-09).
    A delta from a LATER revision than the record's is refused
    too: it says what changed since that revision, not since the one the design was accepted for,
    and a requirement reworded in between would read as carried."""
    if not isinstance(delta, dict) or not isinstance(delta.get("doc_id"), str) \
            or not isinstance(delta.get("from_rev"), str):
        raise RevisionError("the words delta names no specification (`doc_id`) or starting revision "
                            "(`from_rev`): it was not built by this version of scxml_requirement_set, "
                            "so nothing says it describes this record's specification. Build the "
                            "revised list again against its lineage and give the `delta` it returns")
    if not isinstance(delta.get("from_manifest_sha256"), str):
        raise RevisionError("the words delta does not say which list it starts from "
                            "(`from_manifest_sha256`): the lineage it was built against predates "
                            "recording the manifest's digest, or the revision it starts from was "
                            "adopted without the manifest. A name and a revision number do not tell "
                            "two lists of one specification apart; build the revised list again from "
                            "the manifest and sidecar the design was accepted against")
    manifest = record.get("manifest") if isinstance(record, dict) else None
    if not isinstance(manifest, dict) or not isinstance(manifest.get("doc_id"), str) \
            or not isinstance(manifest.get("rev"), str) or not isinstance(manifest.get("sha256"), str):
        raise RevisionError("the acceptance record pins no manifest `doc_id`, `rev` and `sha256`, so "
                            "there is nothing to check the words delta against")
    if delta["doc_id"] != manifest["doc_id"]:
        raise RevisionError(f"the words delta is of specification '{delta['doc_id']}' and the "
                            f"acceptance record was taken against '{manifest['doc_id']}'")
    if delta["from_rev"] != manifest["rev"]:
        raise RevisionError(f"the words delta starts from revision {delta['from_rev']} of "
                            f"'{delta['doc_id']}' and the acceptance record was taken against "
                            f"revision {manifest['rev']}: it describes a different step. Build the "
                            "list again against the lineage as it was at the revision the design was "
                            "accepted for")
    if delta["from_manifest_sha256"] != manifest["sha256"]:
        raise RevisionError(f"the words delta starts from a list whose manifest digest is "
                            f"{delta['from_manifest_sha256'][:12]} and the acceptance record was taken "
                            f"against a manifest whose digest is {manifest['sha256'][:12]}: they are not "
                            f"the same list, though both are revision {manifest['rev']} of "
                            f"'{manifest['doc_id']}' (another copy of the specification, or a list "
                            "written again for the same text)")
    if not isinstance(delta.get("from_sidecar_sha256"), str):
        raise RevisionError("the words delta does not say which words it starts from "
                            "(`from_sidecar_sha256`): the lineage it was built against predates "
                            "recording the sidecar's digest, or the revision it starts from was "
                            "adopted without the sidecar. A manifest is coordinates only, so it does "
                            "not tell two lists of one shape apart; build the revised list again from "
                            "the manifest and sidecar the design was accepted against")
    if not isinstance(manifest.get("sidecar_sha256"), str):
        raise RevisionError("the acceptance record pins no sidecar (`manifest.sidecar_sha256`): it was "
                            "taken without the words behind the ids, and a manifest is coordinates "
                            "only, so nothing says the words delta starts from the list that was "
                            "accepted. Accept the design again, giving the sidecar")
    if delta["from_sidecar_sha256"] != manifest["sidecar_sha256"]:
        raise RevisionError(f"the words delta starts from words whose sidecar digest is "
                            f"{delta['from_sidecar_sha256'][:12]} and the acceptance record was taken "
                            f"against a sidecar whose digest is {manifest['sidecar_sha256'][:12]}: "
                            f"they are not the same list, though the manifest is the same (another "
                            f"specification of the same shape, or the same ids over other sentences)")


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
    ("carried", None): ("uncited", UNCOVERED),
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


SHARED = "moved-with-a-changed-neighbour"


def _explained_by_neighbours(rows: list[dict]) -> None:
    """Re-read, in place, the carried requirements that moved only at places a requirement whose
    words DID change (or that is new) also stands on.

    A design node is cited by several requirements at once, and the product's closure of each one
    includes the node's rows. When the specification asks for one requirement to change, the node
    it changes moves the evidence of every requirement that cites the same node, and those
    requirements' words did not change. Calling that `moved-without-reason` blames the design for
    obeying the specification (a trial on a real component, 2026-10-08: a pop-up state cited by R7
    and R8, the sound of R8 changed, R7 read as a violation).

    It is a `look` and not `ok`: the place is exactly where the changed neighbour's edit could have
    broken the requirement that was not asked to change, and the row names that neighbour. It is
    explained only when EVERY place it moved is one the neighbour moved or newly stands on, and
    when no more recorded rows are gone than places moved (a row that disappeared cannot be located,
    so more rows gone than places gained is a loss nothing accounts for). A carried neighbour never
    explains: two requirements whose words did not change have no reason to move together."""
    asked = {}
    for row in rows:
        if row["words"] in ("changed", "new") and row["evidence"] in ("changed", "new"):
            asked[row["requirement"]] = set(row.get("moved") or row.get("at") or ())
    for row in rows:
        if row["kind"] != "moved-without-reason" or row["evidence"] != "changed":
            continue
        places = set(row.get("moved") or ())
        if not places or int(row.get("gone") or 0) > len(places):
            continue
        sharing = {id_: shared for id_, shared in asked.items() if places & shared}
        if places <= set().union(*sharing.values()):
            row["kind"], row["severity"] = SHARED, LOOK
            row["shared_with"] = sorted(sharing)


def join(words_delta: object, evidence_delta: object) -> dict:
    """The words and the evidence of a revision, joined by requirement id.

    ⚠ A requirement the words delta calls `carried` and no node cites, in the record or now, is
    `uncited` and `uncovered`: nothing about it moved, but nothing was compared either, and the
    result counts it apart (`summary.uncovered`) so that "carries over" only ever means a requirement
    whose evidence was read on both sides. It is not a finding against the revision
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
    _explained_by_neighbours(rows)
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
            "uncovered": sum(1 for r in rows if r["severity"] == UNCOVERED),
            # How many requirements the check saw evidence for, on either side. 0 with requirements
            # listed means it compared nothing, whatever the verdict says.
            "seen": sum(1 for r in rows if r["evidence"] != "none"),
            "kinds": summary,
        },
        "requirements": rows,
        "unclaimed": {"added": added, "gone": int(unclaimed.get("gone") or 0)},
    }


_WHY = {
    "moved-without-reason": "its words did not change and the design moved",
    SHARED: "its words did not change; the design moved only where a requirement whose words did "
            "change also stands, so check that this one still holds there",
    "retired-still-cited": "the specification dropped it and the design still cites it",
    "newly-cited": "its words did not change and the design now cites it",
    "words-changed-design-same": "its words changed and the design reads the same",
    "changed-but-uncited": "its words changed and no node cites it",
    "unimplemented-new": "it is new and nothing in the design carries it",
    "unlisted": "the design cites it and the revised list does not have it",
    "revised": "its words and the design both moved",
    "implemented-new": "it is new and the design now carries it",
    "retired-cleanly": "the specification dropped it and no node cites it",
    "uncited": "its words did not change and no node cites it, before or now, so there was nothing "
               "to compare",
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
    summary = result["summary"]
    if summary["requirements"] and summary["seen"] == 0:
        out += ["⚠ This check saw no evidence for any requirement: no node of the design cites one (or "
                "its kind has nowhere to cite one), so it compared nothing. The verdict above says "
                "nothing about the design.", ""]
    elif summary["uncovered"]:
        out += [f"{summary['uncovered']} requirement(s) are cited by no node, before or now, so this "
                "check could not compare them; they are listed apart below and are not carried over.", ""]
    for heading, severity in (("Outside the revision's reach", VIOLATION), ("Look again", LOOK),
                              ("Changed as the words asked", OK),
                              ("Not covered by this check", UNCOVERED)):
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
            if r.get("shared_with"):
                line += "; shared with " + ", ".join(r["shared_with"])
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
