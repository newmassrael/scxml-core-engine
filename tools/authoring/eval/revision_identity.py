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
    silent wrong   the id exists in the second revision and names a DIFFERENT
                   requirement. Nothing flags it: a design that cites the id
                   reads `implemented` against a sentence it was never written
                   for. This is the figure that must be zero.
    lost           the requirement is still there but its id is not in the
                   second list. The product reports the cited id as dangling,
                   so it is loud, and needless: the thread was cut for nothing.
    retired        the requirement's sentence is gone and so is its id. Right.

and, beside them, whether the two manifests carry different `rev`s: the
product's staleness note (`requirement_manifest::classify_citations`) compares a
document's cited `doc_id@rev` with the manifest's, so two revisions of one
specification that carry the same `rev` can never be told apart by it.

    python3 tools/authoring/eval/revision_identity.py [--json]

⚠ The scheme measured is the one `sce_author.requirement_set.build` implements
today: ids by reading order and `rev` as `scxml_requirement_set` leaves it. A
scheme that changes either replaces that function's behaviour, and the figures
in `docs/adr/0006-a-requirement-keeps-its-id-across-a-revision.md` and the test
that reads them (`test_the_revision_measurement_reads_what_became_of_each_id`)
change in the same commit. The `reflow` edit is the control: it changes no
word, so every id has to hold under any scheme, and a measurement that cannot
read that has not measured anything.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys
import textwrap

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from sce_author import requirement_set as rs  # noqa: E402

CASES = HERE / "revision_identity_cases.json"

SCHEME = "reading-order ids"


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


def revise(case: dict, edit: dict) -> tuple[str, list[dict]]:
    """The second revision of `case` under `edit`: its prose, and its
    requirements as `{quote, statement, modality?, truth, origin}` in the order
    the owner lists them. `origin` is the index in the first revision's list, or
    None for a requirement the edit added."""
    sentences = split_sentences(case["prose"])
    if " ".join(sentences) != rs.normalise(case["prose"]):
        raise CaseError(f"{case['id']}: the prose does not survive being cut into sentences")
    items = [dict(item, truth="same", origin=n) for n, item in enumerate(case["requirements"])]
    width = None
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
            gone = sentences.pop(_index_of(sentences, op["sentence"], edit["id"]))
            items = [i for i in items if not (i["origin"] is not None and i["quote"] in gone)]
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
    return prose, items


def build_today(prose: str, items: list[dict], doc_id: str) -> rs.Built:
    """The list as `scxml_requirement_set` builds it: the client's quotes, the
    document's name, and no revision (the tool does not pass one)."""
    built = rs.build(prose, [{k: v for k, v in i.items() if k in ("quote", "statement", "modality")}
                             for i in items], doc_id=doc_id)
    if built.refused:
        raise CaseError(f"{doc_id}: the list is refused: "
                        + "; ".join(f"{r.quote!r}: {r.why}" for r in built.refused))
    return built


def measure_edit(case: dict, edit: dict) -> dict:
    first = build_today(case["prose"], case["requirements"], case["id"])
    prose2, items2 = revise(case, edit)
    second = build_today(prose2, items2, case["id"])
    by_quote_1 = {r.quote: r for r in first.requirements}
    by_quote_2 = {r.quote: r for r in second.requirements}
    ids_2 = {r.id: r for r in second.requirements}
    ancestor = {i["origin"]: i for i in items2 if i["origin"] is not None}

    holds = silent = lost = retired = moved_sections = 0
    for n, original in enumerate(case["requirements"]):
        before = by_quote_1[original["quote"]]
        if n in ancestor:
            after = by_quote_2[ancestor[n]["quote"]]
            if before.sentence != after.sentence:
                moved_sections += 1
            if after.id == before.id:
                holds += 1
                continue
        else:
            after = None
        named = ids_2.get(before.id)
        if named is not None:
            silent += 1  # the id is there, and it names another requirement now
        elif after is not None:
            lost += 1
        else:
            retired += 1
    return {
        "case": case["id"], "edit": edit["id"], "scheme": SCHEME,
        "requirements": len(first.requirements),
        "holds": holds, "silent_wrong": silent, "lost": lost, "retired": retired,
        "added": sum(1 for i in items2 if i["origin"] is None),
        "sections_moved": moved_sections,
        "rev": [first.manifest["rev"], second.manifest["rev"]],
        "same_rev_told_apart_by_nothing": first.manifest["rev"] == second.manifest["rev"]
                                          and first.manifest != second.manifest,
    }


def measure(cases: list[dict] | None = None) -> list[dict]:
    rows = []
    for case in cases if cases is not None else load_cases():
        for edit in case["edits"]:
            rows.append(measure_edit(case, edit))
    return rows


def table(rows: list[dict]) -> str:
    head = ("case", "edit", "reqs", "holds", "silent wrong", "lost", "retired", "added",
            "sections moved", "rev")
    lines = ["| " + " | ".join(head) + " |", "|" + "---|" * len(head)]
    for r in rows:
        lines.append("| " + " | ".join(str(c) for c in (
            r["case"], r["edit"], r["requirements"], r["holds"], r["silent_wrong"], r["lost"],
            r["retired"], r["added"], r["sections_moved"], "{} -> {}".format(*r["rev"]))) + " |")
    total = {k: sum(r[k] for r in rows) for k in ("requirements", "holds", "silent_wrong", "lost",
                                                  "retired", "added", "sections_moved")}
    lines.append("| **all** | | {requirements} | {holds} | {silent_wrong} | {lost} | {retired} "
                 "| {added} | {sections_moved} | |".format(**total))
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--json", action="store_true", help="the rows as JSON instead of a table")
    args = ap.parse_args()
    rows = measure()
    print(json.dumps(rows, indent=2) if args.json else f"scheme: {SCHEME}\n\n{table(rows)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
