"""Measure whether a requirement keeps its id when its specification is revised.

Why a measurement: an acceptance lapses when its specification moves, and what
an owner (or a program) needs next is to know WHICH requirements moved. That
question is only answerable if a requirement's id means the same requirement in
both revisions. This reports, per edit an owner makes between two drafts, what
became of every id of the first revision's list.

No model is involved. The corpus (`revision_identity_cases.json`) fixes the
owner's requirement list for each specification and states each edit as a rule
over the prose; applying the rule to the prose and to the list at once is what
makes the answer known by construction. For every requirement of the first
revision the report says exactly one of:

    holds          the id, read in the second revision, names the same
                   requirement (or the same requirement with the edit's words)
    silent wrong   the id exists in the second revision, names a DIFFERENT
                   requirement, and nothing says so. A design that cites it
                   reads `implemented` against a sentence it was never written
                   for. This is the figure that must be zero.
    miscarried     the id names a different requirement and the list says it
                   changed (`status: changed`): a person is sent to read it,
                   which costs a look and cannot mislead. How often this
                   happens to a requirement the owner REPLACED is the price of
                   carrying an id through a rewording.
    lost           the requirement is still there but its id is not in the
                   second list. The product reports the cited id as dangling,
                   so it is loud, and needless: the thread was cut for nothing.
    retired        the requirement's sentence is gone and so is its id. Right.

and, beside them, whether the two manifests carry different `rev`s: the
product's staleness note (`requirement_manifest::classify_citations`) compares a
document's cited `doc_id@rev` with the manifest's, so two revisions of one
specification that carry the same `rev` can never be told apart by it.

Three schemes are read over the same edits:

    reading-order ids     the second list built with no lineage, as the tool
                          always built a list (the baseline)
    lineage, words given   built against the first list's lineage and sidecar
    lineage, hashes only   built against the lineage alone: only equal words
                          keep an id

    python3 tools/authoring/eval/revision_identity.py [--json]

⚠ `holds` for a requirement the edit REWORDED means the id was carried to it;
whether the list called it changed is `changed_flagged`. The `reflow` edit is
the control: it changes no word, so every id has to hold under every scheme,
and a measurement that cannot read that has not measured anything.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys
import textwrap

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from sce_author import requirement_lineage as rl  # noqa: E402
from sce_author import requirement_set as rs  # noqa: E402

CASES = HERE / "revision_identity_cases.json"

TODAY = "reading-order ids"
WITH_WORDS = "lineage, words given"
HASHES_ONLY = "lineage, hashes only"
SCHEMES = (TODAY, WITH_WORDS, HASHES_ONLY)


class CaseError(Exception):
    """The corpus states an edit that cannot be applied, or a list that is not
    the specification's own words."""


def load_cases(path: pathlib.Path = CASES) -> list[dict]:
    return json.loads(path.read_text(encoding="utf-8"))["cases"]


def split_sentences(prose: str) -> list[str]:
    """The sentences of `prose` as `requirement_set` cuts them, so an edit that
    names a sentence names what the section numbering counts."""
    text = rs.normalise(prose)
    return [text[a:b].strip() for a, b in rs.sentences(text)]


def _index_of(sentences: list[str], sentence: str, what: str) -> int:
    found = [i for i, s in enumerate(sentences) if s == sentence]
    if len(found) != 1:
        raise CaseError(f"{what}: the sentence {sentence!r} is in the prose {len(found)} times, "
                        "and an edit names one")
    return found[0]


def revise(case: dict, edit: dict) -> tuple[str, list[dict], dict[int, str]]:
    """The second revision of `case` under `edit`: its prose, its requirements as
    `{quote, statement, modality?, truth, origin}` in the order the owner lists
    them (`origin` is the index in the first list, or None for one the edit
    added), and what became of each first-list requirement the edit took away:
    `gone` (its sentence was deleted) or `replaced` (another sentence stands in
    its place)."""
    sentences = split_sentences(case["prose"])
    if " ".join(sentences) != rs.normalise(case["prose"]):
        raise CaseError(f"{case['id']}: the prose does not survive being cut into sentences")
    items = [dict(item, truth="same", origin=n) for n, item in enumerate(case["requirements"])]
    removed: dict[int, str] = {}
    width = None

    def drop(sentence: str, why: str) -> None:
        nonlocal items
        for i in items:
            if i["origin"] is not None and i["quote"] in sentence:
                removed[i["origin"]] = why
        items = [i for i in items if not (i["origin"] is not None and i["quote"] in sentence)]

    for op in edit["ops"]:
        kind = op["op"]
        if kind == "insert_after":
            at = _index_of(sentences, op["after"], edit["id"])
            sentences.insert(at + 1, op["sentence"])
            items.append(dict(op["requirement"], truth="new", origin=None))
        elif kind == "append":
            sentences.append(op["sentence"])
            items.append(dict(op["requirement"], truth="new", origin=None))
        elif kind == "delete":
            drop(sentences.pop(_index_of(sentences, op["sentence"], edit["id"])), "gone")
        elif kind == "rewrite":
            at = _index_of(sentences, op["sentence"], edit["id"])
            drop(sentences[at], "replaced")
            sentences[at] = op["with"]
            items.append(dict(op["requirement"], truth="new", origin=None))
        elif kind == "replace":
            if not any(op["old"] in s for s in sentences):
                raise CaseError(f"{edit['id']}: {op['old']!r} is nowhere in the prose")
            sentences = [s.replace(op["old"], op["new"]) for s in sentences]
            for item in items:
                if op["old"] in item["quote"]:
                    item["quote"] = item["quote"].replace(op["old"], op["new"])
                    if item["truth"] == "same":
                        item["truth"] = "changed"
        elif kind == "swap":
            a = _index_of(sentences, op["a"], edit["id"])
            b = _index_of(sentences, op["b"], edit["id"])
            sentences[a], sentences[b] = sentences[b], sentences[a]
        elif kind == "reflow":
            width = op["width"]
        else:
            raise CaseError(f"{edit['id']}: no edit named {kind!r}")
    prose = " ".join(sentences)
    if width is not None:
        prose = textwrap.fill(prose, width=width)
    return prose, items, removed


def _list_items(items: list[dict]) -> list[dict]:
    return [{k: v for k, v in i.items() if k in ("quote", "statement", "modality")} for i in items]


def build_first(case: dict) -> rs.Built:
    """The first list of a specification, as `scxml_requirement_set` builds it."""
    return _checked(rs.build(case["prose"], _list_items(case["requirements"]), doc_id=case["id"]),
                    case["id"])


def build_second(scheme: str, case: dict, first: rs.Built, prose: str, items: list[dict]) -> rs.Built:
    """The second revision's list under `scheme`. Reading-order ids is the list built
    with no lineage; the others are built against the first list's."""
    kw = {}
    if scheme != TODAY:
        kw["lineage_text"] = rl.render(first.lineage)
        if scheme == WITH_WORDS:
            kw["previous_sidecar_text"] = json.dumps(first.sidecar)
    return _checked(rs.build(prose, _list_items(items), doc_id=case["id"], **kw), case["id"])


def _checked(built: rs.Built, what: str) -> rs.Built:
    if built.refused:
        raise CaseError(f"{what}: the list is refused: "
                        + "; ".join(f"{r.quote!r}: {r.why}" for r in built.refused))
    return built


def measure_edit(case: dict, edit: dict, scheme: str = TODAY) -> dict:
    first = build_first(case)
    prose2, items2, removed = revise(case, edit)
    second = build_second(scheme, case, first, prose2, items2)
    by_quote_1 = {r.quote: r for r in first.requirements}
    by_quote_2 = {r.quote: r for r in second.requirements}
    ids_2 = {r.id: r for r in second.requirements}
    ancestor = {i["origin"]: i for i in items2 if i["origin"] is not None}
    flagged = scheme != TODAY  # only a lineage list says which requirements changed

    holds = silent = miscarried = lost = retired = moved_sections = changed_flagged = 0
    for n, original in enumerate(case["requirements"]):
        before = by_quote_1[original["quote"]]
        after = by_quote_2[ancestor[n]["quote"]] if n in ancestor else None
        if after is not None and before.sentence != after.sentence:
            moved_sections += 1
        if after is not None and after.id == before.id:
            holds += 1
            changed_flagged += ancestor[n]["truth"] == "changed" and after.status == "changed"
            continue
        named = ids_2.get(before.id)
        if named is not None:
            if flagged and named.status == "changed":
                miscarried += 1  # a different requirement under the id, and the list says it changed
            else:
                silent += 1  # a different requirement under the id, and nothing says so
        elif after is not None:
            lost += 1
        else:
            retired += 1
    return {
        "case": case["id"], "edit": edit["id"], "scheme": scheme,
        "requirements": len(first.requirements),
        "holds": holds, "silent_wrong": silent, "miscarried": miscarried, "lost": lost,
        "retired": retired, "changed_flagged": changed_flagged,
        "replaced": sum(1 for why in removed.values() if why == "replaced"),
        "reworded": sum(1 for i in items2 if i["truth"] == "changed"),
        "added": sum(1 for i in items2 if i["origin"] is None),
        "sections_moved": moved_sections,
        "rev": [first.manifest["rev"], second.manifest["rev"]],
        "same_rev_told_apart_by_nothing": first.manifest["rev"] == second.manifest["rev"]
                                          and first.manifest != second.manifest,
    }


def measure(cases: list[dict] | None = None, scheme: str = TODAY) -> list[dict]:
    rows = []
    for case in cases if cases is not None else load_cases():
        for edit in case["edits"]:
            rows.append(measure_edit(case, edit, scheme))
    return rows


def totals(rows: list[dict]) -> dict:
    keys = ("requirements", "holds", "silent_wrong", "miscarried", "lost", "retired", "added",
            "replaced", "reworded", "changed_flagged", "sections_moved")
    return {k: sum(r[k] for r in rows) for k in keys}


def table(rows: list[dict]) -> str:
    head = ("case", "edit", "reqs", "holds", "silent wrong", "miscarried", "lost", "retired",
            "sections moved", "rev")
    lines = ["| " + " | ".join(head) + " |", "|" + "---|" * len(head)]
    for r in rows:
        lines.append("| " + " | ".join(str(c) for c in (
            r["case"], r["edit"], r["requirements"], r["holds"], r["silent_wrong"], r["miscarried"],
            r["lost"], r["retired"], r["sections_moved"], "{} -> {}".format(*r["rev"]))) + " |")
    t = totals(rows)
    lines.append("| **all** | | {requirements} | {holds} | {silent_wrong} | {miscarried} | {lost} "
                 "| {retired} | {sections_moved} | |".format(**t))
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--json", action="store_true", help="the rows as JSON instead of tables")
    args = ap.parse_args()
    by_scheme = {scheme: measure(scheme=scheme) for scheme in SCHEMES}
    if args.json:
        print(json.dumps([row for rows in by_scheme.values() for row in rows], indent=2))
    else:
        for scheme, rows in by_scheme.items():
            print(f"scheme: {scheme}\n\n{table(rows)}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
