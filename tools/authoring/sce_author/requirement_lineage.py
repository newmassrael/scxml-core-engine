"""The lineage that lets a requirement keep its id when its specification is revised.

`requirement_set.build` used to number the quoted requirements by where their
words sit, so a sentence inserted anywhere but the end renumbered everything
after it, and an id that was still in the new list could name a different
requirement with nothing to say so (`docs/adr/0006-a-requirement-keeps-its-id-
across-a-revision.md`, measured by `eval/revision_identity.py`). The lineage is
what a later revision is built against: it remembers which id was issued for
which requirement, so an id is issued once and follows the requirement.

It holds hashes and numbers and never a word of the specification. A requirement
sentence is often someone else's document, and the manifest is committed
(`sce-build/src/requirement_manifest.rs`, "Why the manifest holds no requirement
text"); the lineage is committed too. What compares two wordings, when the words
are needed, is the previous sidecar the caller still holds.

    {"lineage": "sce-requirement-lineage", "v": 1, "doc_id": "spec", "next": 12,
     "revisions": [{"rev": "1", "spec_sha256": "...", "sentence_sha256": ["...", ...]}],
     "requirements": [{"id": "R1", "first_rev": "1", "retired_rev": null,
                       "quotes": [{"rev": "1", "sha256": "..."}]}]}

Identity and sameness are kept apart. An id is carried to a quote when

    carried     the quote's words are exactly the words the id last had (equal
                sha256): the requirement is the SAME;
    changed     the quote is the one near-match of the id's last words (the
                previous sidecar holds them), or the caller stated it
                (`continues`): the requirement is the id's SUCCESSOR and its
                words are not the id's last words, so it is reported changed;
    new         no id is carried: a fresh id is issued from `next`;
    retired     no quote carries a live id: the id is closed and never issued
                again.

"Same" is claimed on equal hashes only. A wrong succession costs a requirement
reported changed that was in fact replaced, which sends a person to read it; it
cannot report a different requirement as unchanged.

⚠ `SIMILARITY` and `MARGIN` are starting values. The corpus they were first read
against is small (`eval/revision_identity_cases.json`); the figures are in the
ADR with the edits that fooled them. A near-match that is not clearly the only
one is not made: both sides are left to be retired and issued afresh, which is
loud and never wrong.
"""

from __future__ import annotations

import hashlib
import json
import re
from collections import Counter
from dataclasses import dataclass, field

from .errors import AuthoringError

LINEAGE_KIND = "sce-requirement-lineage"
LINEAGE_VERSION = 1

# Dice coefficient over the character trigrams of two quotes' words, and how far
# ahead of the next best candidate (for either side of the pair) a pair has to be
# to be believed.
#
# ⚠ Trigrams and not words. The first version counted whole words, and a quote the
# client was told to keep "as short as the requirement allows" is a few words, one of
# which is often an identifier: `shown with beep_fuel_low_1` against `...low_2` shared two
# of three words (0.67) and a bare `beep_fuel_low_1` none (0.00), so a one-character edit
# was read as a new requirement and a retired one (a trial on a real component, 2026-10-08).
# On trigrams that edit scores 0.81 to 0.94, an edited number 0.90, and requirements that
# share nothing 0.0 to 0.25 (two sentences about one device, 0.55), so the same 0.8 now
# separates them. Below about fourteen characters one edit is too large a share and falls
# short (`beep_low_1` to `2`, 0.75): loudly, as a retirement and a new id. What trigrams do
# not separate -- `When the fully-closed sensor reports, the door is closed.` and `...is
# locked.` (0.90), `...the gate is open.` and `...is shut.` (0.90) -- no similarity can: a
# person has to read those, and the margin rule and the `changed` status are what keep them
# safe.
SIMILARITY = 0.8
MARGIN = 0.1
GRAM = 3

_ID = re.compile(r"R([0-9]+)")
_WORD = re.compile(r"\w+")


class LineageError(AuthoringError):
    """A lineage, or what it is asked to carry, cannot be used."""


def sha256_text(text: str) -> str:
    """The digest of text the caller has already normalised. The lineage does not
    normalise: what counts as the same words is `requirement_set.normalise`'s."""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


@dataclass
class Spec:
    """What a revision of the specification is, as far as the lineage needs it."""

    sha256: str | None
    sentence_sha256: list[str]


@dataclass
class Advance:
    """A list built against a lineage: the next lineage, the id of each quote (in the
    order the quotes were given), the revision, and what happened to every id."""

    lineage: dict
    rev: str
    ids: list[str]
    statuses: list[str]
    delta: dict | None
    notes: list[str] = field(default_factory=list)


def first(doc_id: str, spec: Spec, quotes: list[str], rev: str = "1") -> Advance:
    """The first list of a specification: ids in reading order, as the tool has
    always given them, so a list built once is the list it always was."""
    ids = [f"R{n}" for n in range(1, len(quotes) + 1)]
    lineage = _checked({
        "lineage": LINEAGE_KIND, "v": LINEAGE_VERSION, "doc_id": doc_id,
        "next": len(quotes) + 1,
        "revisions": [_revision_row(rev, spec)],
        "requirements": [
            {"id": id_, "first_rev": rev, "retired_rev": None,
             "quotes": [{"rev": rev, "sha256": sha256_text(quote)}]}
            for id_, quote in zip(ids, quotes)],
    })
    return Advance(lineage, rev, ids, ["new"] * len(ids), None)


def adopt(doc_id: str, manifest: dict, texts: dict[str, str]) -> dict:
    """A lineage for a list that was built before lineages existed: the ids are the
    manifest's, the words behind them are `texts` (the sidecar's, already
    normalised), the revision is the manifest's. Whether the specification
    changed since cannot be told (its digest was never kept), so the next
    revision is a new one."""
    if manifest.get("doc_id") != doc_id:
        raise LineageError(f"the previous manifest is for '{manifest.get('doc_id')}', not '{doc_id}'")
    extraction = manifest.get("extraction")
    if not isinstance(extraction, dict) or extraction.get("ids") != "synthesized":
        raise LineageError("the previous manifest carries the source's own ids; nothing here renumbers "
                           "those, and nothing here would carry them either")
    rev = manifest.get("rev")
    if not isinstance(rev, str) or not rev.isdigit():
        raise LineageError(f"the previous manifest's rev {rev!r} is not a whole number, so the next one "
                           "cannot be told")
    rows, top = [], 0
    for entry in manifest.get("requirements") or []:
        id_ = entry.get("id")
        quote = texts.get(id_)
        if not isinstance(id_, str) or not quote:
            raise LineageError(f"the previous sidecar has no words for the previous manifest's '{id_}'")
        found = _ID.fullmatch(id_)
        if found:
            top = max(top, int(found.group(1)))
        rows.append({"id": id_, "first_rev": rev, "retired_rev": None,
                     "quotes": [{"rev": rev, "sha256": sha256_text(quote)}]})
    if not rows:
        raise LineageError("the previous manifest lists no requirements")
    return _checked({
        "lineage": LINEAGE_KIND, "v": LINEAGE_VERSION, "doc_id": doc_id, "next": top + 1,
        "revisions": [{"rev": rev, "spec_sha256": None, "sentence_sha256": []}],
        "requirements": rows})


def advance(prev: dict, doc_id: str, spec: Spec, quotes: list[str], *,
            texts: dict[str, str] | None = None,
            continues: dict[str, str] | None = None) -> Advance:
    """The list of a revision of the specification, built against its lineage.

    `quotes` are the located quotes in reading order, already normalised. `texts`
    maps an id to the words it last had (the previous sidecar, normalised);
    without them only equal words and stated continuations carry an id.
    `continues` maps a quote of this revision to the id it continues."""
    if prev["doc_id"] != doc_id:
        raise LineageError(f"the lineage is for '{prev['doc_id']}', not '{doc_id}': one lineage follows "
                           "one specification")
    last = prev["revisions"][-1]
    if not last["rev"].isdigit():
        raise LineageError(f"the lineage's last rev {last['rev']!r} is not a whole number")
    same_text = spec.sha256 is not None and last["spec_sha256"] == spec.sha256
    rev = last["rev"] if same_text else str(int(last["rev"]) + 1)

    rows = {row["id"]: row for row in prev["requirements"]}
    live = sorted((i for i, row in rows.items() if row["retired_rev"] is None), key=_id_order)
    latest = {i: rows[i]["quotes"][-1]["sha256"] for i in live}
    digests = [sha256_text(q) for q in quotes]

    carried: dict[int, tuple[str, str, str | None]] = {}  # quote index -> (id, status, how)
    taken: set[str] = set()

    for n, digest in enumerate(digests):
        for i in live:
            if i not in taken and latest[i] == digest:
                carried[n] = (i, "carried", None)
                taken.add(i)
                break

    problems = []
    for quote, id_ in sorted((continues or {}).items()):
        where = [n for n, q in enumerate(quotes) if q == quote]
        if not where:
            problems.append(f"'continues' names a quote that is not in this list: {quote!r}")
        elif id_ not in rows:
            problems.append(f"'continues' names {id_}, which the lineage never issued")
        elif rows[id_]["retired_rev"] is not None:
            problems.append(f"'continues' names {id_}, which was retired in rev {rows[id_]['retired_rev']} "
                            "and is never issued again")
        elif where[0] in carried:
            problems.append(f"{quote!r} already carries {carried[where[0]][0]} by its own words; "
                            "a continuation is for words that changed")
        elif id_ in taken:
            problems.append(f"{id_} is claimed twice")
        else:
            carried[where[0]] = (id_, "changed", "client")
            taken.add(id_)
    if problems:
        raise LineageError("; ".join(problems))

    notes: list[str] = []
    words = _usable_words(texts or {}, latest, taken, notes)
    open_quotes = [n for n in range(len(quotes)) if n not in carried]
    for n, i in _near_matches(quotes, open_quotes, words):
        carried[n] = (i, "changed", "near-match")
        taken.add(i)
    unseen = [i for i in live if i not in taken and i not in words]
    if unseen and any(n not in carried for n in range(len(quotes))):
        notes.append(f"{', '.join(unseen)} ended without a successor and their words were not given "
                     "(`previous_sidecar`), so no near match could be tried for them")

    counter = prev["next"]
    ids, statuses = [], []
    new_rows = []
    for n in range(len(quotes)):
        if n in carried:
            id_, status, _ = carried[n]
        else:
            id_, status = f"R{counter}", "new"
            counter += 1
            new_rows.append({"id": id_, "first_rev": rev, "retired_rev": None,
                             "quotes": [{"rev": rev, "sha256": digests[n]}]})
        ids.append(id_)
        statuses.append(status)
    retired = [i for i in live if i not in taken]

    quote_of = {id_: n for n, (id_, _, _) in carried.items()}
    requirements = []
    for row in prev["requirements"]:
        row = {**row, "quotes": [dict(q) for q in row["quotes"]]}
        if row["id"] in retired:
            row["retired_rev"] = rev
        if row["id"] in quote_of:
            _set_quote(row, rev, digests[quote_of[row["id"]]])
        requirements.append(row)
    requirements += new_rows
    revisions = [dict(r) for r in prev["revisions"]]
    if same_text:
        revisions[-1] = _revision_row(rev, spec)
    else:
        revisions.append(_revision_row(rev, spec))
    lineage = _checked({"lineage": LINEAGE_KIND, "v": LINEAGE_VERSION, "doc_id": doc_id, "next": counter,
                        "revisions": revisions, "requirements": requirements})

    # `doc_id` and `from_rev` say which specification, and which step of it, the ids below are about:
    # `revision.belongs_to` compares them with the acceptance record's manifest.
    delta = {
        "doc_id": doc_id, "from_rev": last["rev"], "rev": rev, "specification_changed": not same_text,
        "requirements": {
            "carried": [ids[n] for n in range(len(quotes)) if statuses[n] == "carried"],
            "changed": [{"id": ids[n], "how": carried[n][2]} for n in range(len(quotes))
                        if statuses[n] == "changed"],
            "new": [ids[n] for n in range(len(quotes)) if statuses[n] == "new"],
            "retired": retired,
        },
        "sentences": (None if last["spec_sha256"] is None
                      else _sentence_delta(last["sentence_sha256"], spec.sentence_sha256, same_text)),
    }
    if last["spec_sha256"] is None:
        notes.append("the lineage was made from a list that predates lineages, so the sentences of the "
                     "previous revision are unknown and no sentence delta is given")
    return Advance(lineage, rev, ids, statuses, delta, notes)


def parse(text: str) -> dict:
    """A lineage read back from its text, or the reason it cannot be used."""
    try:
        data = json.loads(text)
    except ValueError as error:
        raise LineageError(f"the lineage is not JSON: {error}") from error
    return _checked(data)


def render(lineage: dict) -> str:
    return json.dumps(lineage, indent=2, ensure_ascii=False) + "\n"


def _revision_row(rev: str, spec: Spec) -> dict:
    return {"rev": rev, "spec_sha256": spec.sha256, "sentence_sha256": list(spec.sentence_sha256)}


def _set_quote(row: dict, rev: str, digest: str) -> None:
    quotes = row["quotes"]
    if quotes[-1]["rev"] == rev:
        quotes[-1]["sha256"] = digest
    elif quotes[-1]["sha256"] != digest:
        quotes.append({"rev": rev, "sha256": digest})


def _id_order(id_: str) -> tuple[int, int, str]:
    found = _ID.fullmatch(id_)
    return (0, int(found.group(1)), id_) if found else (1, 0, id_)


def _usable_words(texts: dict[str, str], latest: dict[str, str], taken: set[str],
                  notes: list[str]) -> dict[str, str]:
    """The words of each live, unclaimed id that the lineage can vouch for: a text
    whose digest is not the id's last digest belongs to some other revision and is
    left out, said once."""
    usable, stale = {}, []
    for id_, text in sorted(texts.items(), key=lambda kv: _id_order(kv[0])):
        if id_ not in latest or id_ in taken:
            continue
        if sha256_text(text) == latest[id_]:
            usable[id_] = text
        else:
            stale.append(id_)
    if stale:
        notes.append(f"the previous sidecar's words for {', '.join(stale)} are not the words the lineage "
                     "last saw for them, so they were not used")
    return usable


def _grams(text: str) -> Counter:
    """The character trigrams of a quote's words, lower-cased and joined by single spaces, with the
    ends padded so a word at either end counts as much as one in the middle."""
    padded = " " * (GRAM - 1) + " ".join(_WORD.findall(text.lower())) + " " * (GRAM - 1)
    return Counter(padded[i:i + GRAM] for i in range(len(padded) - GRAM + 1))


def _dice(a: str, b: str) -> float:
    left, right = _grams(a), _grams(b)
    both = sum((left & right).values())
    total = sum(left.values()) + sum(right.values())
    return 2 * both / total if total else 0.0


def _near_matches(quotes: list[str], open_quotes: list[int], words: dict[str, str]) -> list[tuple[int, str]]:
    """Pairs of an unclaimed quote and an unclaimed id that are each other's one
    near match: the best score of both, above `SIMILARITY`, and ahead of every
    other pair that shares either side by `MARGIN`. Deterministic: ties are
    broken by reading order, then id order."""
    pairs = sorted(((_dice(quotes[n], words[i]), n, i) for n in open_quotes for i in words),
                   key=lambda p: (-p[0], p[1], _id_order(p[2])))
    settled_quotes: set[int] = set()
    settled_ids: set[str] = set()
    made = []
    for score, n, i in pairs:
        if score < SIMILARITY:
            break
        if n in settled_quotes or i in settled_ids:
            continue
        rivals = [s for s, m, j in pairs if (m == n) != (j == i)
                  and m not in settled_quotes and j not in settled_ids]
        if score - max(rivals, default=0.0) >= MARGIN:
            made.append((n, i))
        settled_quotes.add(n)
        settled_ids.add(i)
    return sorted(made)


def _sentence_delta(before: list[str], after: list[str], same_text: bool) -> dict:
    """Which sentences of the new revision are not in the old one, and which of the old are not in the
    new, by digest and position: `S3` is the third sentence of the revision it is named in."""
    if same_text:
        return {"kept": len(after), "added": [], "removed": []}
    remaining = Counter(before)
    kept = 0
    added = []
    for position, digest in enumerate(after, 1):
        if remaining[digest] > 0:
            remaining[digest] -= 1
            kept += 1
        else:
            added.append(f"S{position}")
    leftover = Counter(remaining)
    removed = []
    for position, digest in enumerate(before, 1):
        if leftover[digest] > 0:
            leftover[digest] -= 1
            removed.append(f"S{position}")
    return {"kept": kept, "added": added, "removed": removed}


def _checked(lineage: object) -> dict:
    """The lineage if it is one this module wrote, else why not."""
    keys = {"lineage", "v", "doc_id", "next", "revisions", "requirements"}
    if not isinstance(lineage, dict) or set(lineage) != keys:
        raise LineageError("not a requirement lineage: it has to be an object with exactly "
                           + ", ".join(sorted(keys)))
    if lineage["lineage"] != LINEAGE_KIND or lineage["v"] != LINEAGE_VERSION:
        raise LineageError(f"not a v{LINEAGE_VERSION} {LINEAGE_KIND}")
    if not isinstance(lineage["doc_id"], str) or not lineage["doc_id"]:
        raise LineageError("the lineage has no doc_id")
    revisions = lineage["revisions"]
    if not isinstance(revisions, list) or not revisions:
        raise LineageError("the lineage holds no revision")
    seen_revs = []
    for row in revisions:
        if not isinstance(row, dict) or set(row) != {"rev", "spec_sha256", "sentence_sha256"} \
                or not isinstance(row["rev"], str) or not isinstance(row["sentence_sha256"], list) \
                or not (row["spec_sha256"] is None or isinstance(row["spec_sha256"], str)):
            raise LineageError("a revision row of the lineage is not {rev, spec_sha256, sentence_sha256}")
        seen_revs.append(row["rev"])
    if len(set(seen_revs)) != len(seen_revs):
        raise LineageError("the lineage names a rev twice")
    rows = lineage["requirements"]
    if not isinstance(rows, list):
        raise LineageError("the lineage's requirements are not a list")
    ids, top = set(), 0
    for row in rows:
        if not isinstance(row, dict) or set(row) != {"id", "first_rev", "retired_rev", "quotes"} \
                or not isinstance(row["id"], str) or not isinstance(row["quotes"], list) \
                or not row["quotes"]:
            raise LineageError("a requirement row of the lineage is not {id, first_rev, retired_rev, quotes}")
        if row["id"] in ids:
            raise LineageError(f"the lineage issues {row['id']} twice")
        ids.add(row["id"])
        for ref in (row["first_rev"], row["retired_rev"], *[q.get("rev") if isinstance(q, dict) else None
                                                            for q in row["quotes"]]):
            if ref is not None and ref not in seen_revs:
                raise LineageError(f"{row['id']} names rev {ref!r}, which the lineage has no row for")
        for q in row["quotes"]:
            if not isinstance(q, dict) or set(q) != {"rev", "sha256"} or not isinstance(q["sha256"], str):
                raise LineageError(f"a quote row of {row['id']} is not {{rev, sha256}}")
        found = _ID.fullmatch(row["id"])
        if found:
            top = max(top, int(found.group(1)))
    if not isinstance(lineage["next"], int) or isinstance(lineage["next"], bool) or lineage["next"] <= top:
        raise LineageError(f"the lineage's next ({lineage['next']!r}) is not past every id it issued (R{top}): "
                           "an id would be issued twice")
    return lineage
