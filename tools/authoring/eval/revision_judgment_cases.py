"""The cases a judgment of a revision has to pass, in any language.

ADR 0011 (D1 (c)) moves the judgment of a revision into the product: it reads two requirement
lineages and an acceptance, derives what became of each requirement's words, joins that with what
became of the design's evidence, and writes the verdict and the page. The Python in
`sce_author/revision.py` and `sce_author/requirement_lineage.py` is the reference that exists today,
and a second implementation is only "the same" if both are held to the same cases. This writes
them:

    python3 tools/authoring/eval/revision_judgment_cases.py            # writes the file
    python3 tools/authoring/eval/revision_judgment_cases.py --check    # fails when it is not current

The file is `sce-build/tests/fixtures/revision_judgment/cases.json`. Every input is built here from
the real modules (the lineages come out of `requirement_set.build`, not out of hand-typed hashes),
and every expectation is what the Python gives: the verdict, the rows, the page, or the exact
sentence of a refusal. The Python's own tests hold the meaning of those results independently
(`tests/test_a_revision_joins_*`, `test_a_lineage_is_only_ever_appended_to`); this file does not
replace them, it carries them to the second implementation.

A refusal's sentence is part of the case: it is the product's word to a person, and a port that
refuses for the right reason in other words has changed what the person is told.
"""

from __future__ import annotations

import copy
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE.parent))

from sce_author import requirement_lineage as rl, requirement_set as rs, revision, verify  # noqa: E402

CASES = ROOT / "sce-build" / "tests" / "fixtures" / "revision_judgment" / "cases.json"

LAMP = ("The lamp starts off. Pressing the switch turns it on; pressing it again turns it off. "
        "After 30 seconds on, it turns itself off. Nothing else changes it.")
QUOTES = ["The lamp starts off.", "Pressing the switch turns it on", "pressing it again turns it off",
          "After 30 seconds on, it turns itself off.", "Nothing else changes it."]
RESET = "A reset key clears it."


def outcome(call, *args, **kwargs):
    """What the Python gives: `{"ok": value}`, or `{"refused": the sentence}`."""
    try:
        return {"ok": call(*args, **kwargs)}
    except (rl.LineageError, revision.RevisionError) as error:
        return {"refused": str(error)}


def checked(call, *args):
    """For a function that returns nothing and refuses or does not."""
    try:
        call(*args)
        return "ok"
    except (rl.LineageError, revision.RevisionError) as error:
        return {"refused": str(error)}


# -- the join --------------------------------------------------------------------------------

def words(carried=(), changed=(), new=(), retired=(), **extra):
    return {"requirements": {"carried": list(carried),
                             "changed": [{"id": i, "how": "near-match"} for i in changed],
                             "new": list(new), "retired": list(retired)}, **extra}


def evidence(*lines, added=(), gone=0):
    return {"version": 1, "summary": None, "requirements": list(lines),
            "unclaimed": {"added": list(added), "gone": gone}}


def line(requirement, kind, **more):
    return {"requirement": requirement, "evidence": kind, **more}


# Where an evidence line says the rows are, by kind: the product's own shape.
_PLACES = {"changed": {"moved": ["states.a.transitions[0]"], "gone": 1},
           "new": {"at": ["states.b"]},
           "dropped": {"gone": 1},
           "unchanged": {}}

# Every pair the table can meet: (how the words delta lists the id, what the evidence line says).
PAIRS = [("carried", "unchanged"), ("carried", "changed"), ("carried", "dropped"),
         ("carried", "new"), ("carried", None),
         ("changed", "changed"), ("changed", "unchanged"), ("changed", "dropped"),
         ("changed", None), ("changed", "new"),
         ("new", "new"), ("new", "changed"), ("new", "unchanged"), ("new", "dropped"),
         ("new", None),
         ("retired", "dropped"), ("retired", None), ("retired", "unchanged"),
         ("retired", "changed"), ("retired", "new"),
         (None, "unchanged"), (None, "changed"), (None, "new"), (None, "dropped")]


def join_case(name, w, e):
    return {"name": name, "words": w, "evidence": e, "expect": outcome(revision.join, w, e)}


def table_cases():
    cases = []
    for listed, kind in PAIRS:
        w = words(**({listed: ["R1"]} if listed else {}))
        e = evidence(*([line("R1", kind, **_PLACES[kind])] if kind else []))
        cases.append(join_case(f"table: words {listed or 'unlisted'}, evidence {kind or 'none'}", w, e))
    return cases


def neighbour_cases():
    shared = "states.alarm.on_entry_blocks[0][0]"

    def pair(name, delta, *lines, added=()):
        return join_case(f"neighbour: {name}", delta, evidence(*lines, added=added))

    return [
        pair("a new requirement standing on the same place explains a carried one",
             words(carried=["R1"], new=["R2"]),
             line("R1", "changed", moved=[shared], gone=1), line("R2", "new", at=[shared, "states.alarm"])),
        pair("a requirement whose words changed explains it by the places it moved",
             words(carried=["R1"], changed=["R2"]),
             line("R1", "changed", moved=[shared], gone=1), line("R2", "changed", moved=[shared], gone=1)),
        pair("a place the neighbour does not stand on keeps it a violation",
             words(carried=["R1"], new=["R2"]),
             line("R1", "changed", moved=[shared, "states.elsewhere"], gone=1), line("R2", "new", at=[shared])),
        pair("a neighbour that shares none of it",
             words(carried=["R1"], new=["R2"]),
             line("R1", "changed", moved=["states.elsewhere"], gone=1), line("R2", "new", at=[shared])),
        pair("more recorded rows gone than places moved is a loss nothing accounts for",
             words(carried=["R1"], new=["R2"]),
             line("R1", "changed", moved=[shared], gone=2), line("R2", "new", at=[shared])),
        pair("a carried neighbour explains nothing",
             words(carried=["R1", "R2"]),
             line("R1", "changed", moved=[shared], gone=1), line("R2", "changed", moved=[shared], gone=1)),
        pair("a retired neighbour explains nothing",
             words(carried=["R1"], retired=["R2"]),
             line("R1", "changed", moved=[shared], gone=1), line("R2", "dropped", gone=1)),
        pair("a carried requirement whose citations all vanished is not explained",
             words(carried=["R1"], new=["R2"]),
             line("R1", "dropped", gone=1), line("R2", "new", at=[shared])),
        pair("every neighbour that shares a place is named",
             words(carried=["R1"], new=["R2"], changed=["R3"]),
             line("R1", "changed", moved=["a", "b"], gone=2), line("R2", "new", at=["a"]),
             line("R3", "changed", moved=["b"], gone=1)),
        pair("a requirement that moved no place is not explained",
             words(carried=["R1"], new=["R2"]),
             line("R1", "changed", moved=[], gone=0), line("R2", "new", at=[shared])),
    ]


def summary_cases():
    return [
        join_case("summary: an unchanged revision carries everything over",
                  words(carried=["R1", "R2"]),
                  evidence(line("R1", "unchanged"), line("R2", "unchanged"))),
        join_case("summary: rows that claim no requirement are one more look",
                  words(carried=["R1"]),
                  evidence(line("R1", "unchanged"), added=["states.spare"], gone=0)),
        join_case("summary: rows gone that claimed nothing are counted, not added",
                  words(carried=["R1"]),
                  evidence(line("R1", "unchanged"), gone=3)),
        join_case("summary: a check that saw no evidence says seen is 0",
                  words(carried=["R1", "R2"]),
                  evidence()),
        join_case("summary: uncited requirements are counted apart beside ones that were seen",
                  words(carried=["R1", "R2", "R3"]),
                  evidence(line("R1", "unchanged"))),
        join_case("summary: ids come out in sorted order whatever order they went in",
                  words(carried=["R10", "R2", "R1"]),
                  evidence(line("R2", "unchanged"), line("R10", "unchanged"), line("R1", "unchanged"))),
        join_case("summary: a revision that lists no requirement",
                  words(), evidence()),
        join_case("summary: an evidence delta with no unclaimed line",
                  words(carried=["R1"]),
                  {"requirements": [line("R1", "unchanged")]}),
        join_case("summary: a words delta whose changed entries are bare ids",
                  {"requirements": {"carried": [], "changed": ["R1"], "new": [], "retired": []}},
                  evidence(line("R1", "changed", moved=["a"], gone=1))),
    ]


def refusal_cases():
    bad_words = [
        ("not an object", None), ("an empty object", {}), ("requirements is a list", {"requirements": []}),
        ("no retired list", {"requirements": {"carried": []}}),
        ("no changed list", {"requirements": {"carried": [], "new": [], "retired": []}}),
        ("an id listed twice", words(carried=["R1"], new=["R1"])),
        ("an empty id", words(carried=[""])),
        ("an id that is not text", {"requirements": {"carried": [1], "changed": [], "new": [], "retired": []}}),
    ]
    bad_evidence = [
        ("not an object", None), ("no requirements", {}), ("requirements is an object", {"requirements": {}}),
        ("a line with no evidence", {"requirements": [{"requirement": "R1"}]}),
        ("a line with an unknown evidence", {"requirements": [{"requirement": "R1", "evidence": "gone"}]}),
        ("a requirement named twice", {"requirements": [line("R1", "changed"), line("R1", "unchanged")]}),
        ("unclaimed is not an object", {"requirements": [], "unclaimed": {"added": "x"}}),
    ]
    cases = []
    for name, w in bad_words:
        cases.append(join_case(f"refused words delta: {name}", w, evidence()))
    for name, e in bad_evidence:
        cases.append(join_case(f"refused evidence delta: {name}", words(carried=["R1"]), e))
    return cases


def join_cases():
    return table_cases() + neighbour_cases() + summary_cases() + refusal_cases()


# -- the page --------------------------------------------------------------------------------

def render_cases():
    def case(name, w, e, sentences=None, title=""):
        result = revision.join(w, e)
        return {"name": name, "result": result, "sentences": sentences, "title": title,
                "expect": revision.render(result, sentences, title=title)}

    shared = "states.alarm.on_entry_blocks[0][0]"
    return [
        case("a clean revision", words(carried=["R1"]), evidence(line("R1", "unchanged")), title="revision 1 to 2"),
        case("a violation, a look, an ok and a carry-over",
             words(carried=["R1", "R2"], changed=["R3"], new=["R4"], retired=["R5"]),
             evidence(line("R1", "unchanged"), line("R2", "unchanged"),
                      line("R3", "changed", moved=["states.alarm.on_entry_blocks[0][0]"], gone=1),
                      line("R5", "unchanged"), added=["states.spare"])),
        case("sentences are printed only when given",
             words(carried=["R1", "R2"], changed=["R3"], new=["R4"], retired=["R5"]),
             evidence(line("R1", "unchanged"), line("R2", "unchanged"),
                      line("R3", "changed", moved=["states.alarm.on_entry_blocks[0][0]"], gone=1),
                      line("R5", "unchanged")),
             sentences={"R3": "The alarm sounds for five seconds.", "R1": "A sentence that carries over."}),
        case("a neighbour explains a move and is named",
             words(carried=["R1"], new=["R2"]),
             evidence(line("R1", "changed", moved=[shared], gone=1), line("R2", "new", at=[shared]))),
        case("a check that saw nothing says so",
             words(carried=["R1", "R2"]), evidence()),
        case("uncited requirements are listed apart and not carried over",
             words(carried=["R1", "R2"]), evidence(line("R1", "unchanged"))),
        case("sentences given but empty for a listed requirement",
             words(carried=["R1"], new=["R2"]), evidence(line("R1", "unchanged")), sentences={"R2": ""}),
        case("a revision that lists nothing", words(), evidence()),
    ]


# -- is the delta the record's ---------------------------------------------------------------

MANIFEST_SHA = "a" * 64
SIDECAR_SHA = "c" * 64


def delta_of(**changes):
    delta = {"doc_id": "lamp", "from_rev": "3", "from_manifest_sha256": MANIFEST_SHA,
             "from_sidecar_sha256": SIDECAR_SHA,
             "requirements": {"carried": [], "changed": [], "new": [], "retired": []}}
    for key, value in changes.items():
        if value is None:
            delta.pop(key, None)
        else:
            delta[key] = value
    return delta


def record_of(**manifest):
    pin = {"path": "spec/requirements.manifest.json", "doc_id": "lamp", "rev": "3", "sha256": MANIFEST_SHA,
           "sidecar_sha256": SIDECAR_SHA}
    for key, value in manifest.items():
        if value is None:
            pin.pop(key, None)
        else:
            pin[key] = value
    return {"record": "sce-acceptance-record", "v": 1, "manifest": pin}


def belongs_to_cases():
    def case(name, delta, record):
        return {"name": name, "delta": delta, "record": record,
                "expect": checked(revision.belongs_to, delta, record)}

    return [
        case("the delta of the record's own list", delta_of(), record_of()),
        case("another specification", delta_of(doc_id="other"), record_of()),
        case("an earlier step", delta_of(from_rev="2"), record_of()),
        case("a later step", delta_of(from_rev="4"), record_of()),
        case("no specification named", delta_of(doc_id=None), record_of()),
        case("no starting revision named", delta_of(from_rev=None), record_of()),
        case("a specification that is not text", delta_of(doc_id=7), record_of()),
        case("a delta that is not an object", ["lamp"], record_of()),
        case("another copy under the same name and revision",
             delta_of(from_manifest_sha256="b" * 64), record_of()),
        case("no digest of the list it starts from", delta_of(from_manifest_sha256=None), record_of()),
        case("a digest that is not text", delta_of(from_manifest_sha256=7), record_of()),
        # The manifest is coordinates only: another specification of the same shape, or the same
        # ids over other sentences, has the manifest's digest and not the sidecar's.
        case("the same shape over other words", delta_of(from_sidecar_sha256="b" * 64), record_of()),
        case("no digest of the words it starts from", delta_of(from_sidecar_sha256=None), record_of()),
        case("a words digest that is not text", delta_of(from_sidecar_sha256=7), record_of()),
        case("a record taken without the words", delta_of(), record_of(sidecar_sha256=None)),
        case("a record whose words digest is not text", delta_of(), record_of(sidecar_sha256=7)),
        case("a record that pins no manifest", delta_of(), {}),
        case("a record whose manifest has no digest", delta_of(), record_of(sha256=None)),
        case("a record whose manifest has no revision", delta_of(), record_of(rev=None)),
        case("a record that is not an object", delta_of(), None),
    ]


# -- lineages --------------------------------------------------------------------------------

def items(quotes, statement="s"):
    return [{"quote": q, "statement": statement} for q in quotes]


def build(prose, quotes, previous=None, statement="s"):
    kw = {}
    if previous is not None:
        kw = {"lineage_text": rl.render(previous.lineage),
              "previous_sidecar_text": json.dumps(previous.sidecar)}
    built = rs.build(prose, items(quotes, statement), doc_id="lamp", **kw)
    assert not built.refused, built.refused
    return built


def histories():
    first = build(LAMP, QUOTES)
    second = build(LAMP.replace(" Nothing else changes it.", ""), QUOTES[:4], first)
    third = build(LAMP.replace(" Nothing else changes it.", " " + RESET), QUOTES[:4] + [RESET], second)
    reworded = build(LAMP.replace("30 seconds", "45 seconds"),
                     QUOTES[:3] + ["After 45 seconds on, it turns itself off."] + QUOTES[4:], first)
    back = build(LAMP, QUOTES, reworded)
    again = build(LAMP, QUOTES[:4], first)
    reworded_then_added = build(
        LAMP.replace("30 seconds", "45 seconds").replace(
            "Nothing else changes it.", "Nothing else changes it. " + RESET),
        QUOTES[:3] + ["After 45 seconds on, it turns itself off.", QUOTES[4], RESET], reworded)
    return {"first": first, "second": second, "third": third, "reworded": reworded, "back": back,
            "again": again, "reworded then added": reworded_then_added}


class Lineages:
    """The lineages the cases use, each once and by name: a case that named a lineage by value
    would carry its hundreds of lines again for every case, and a port would read a quarter of a
    megabyte to hold twenty lineages."""

    def __init__(self):
        self.held: dict[str, dict] = {}

    def add(self, name: str, lineage: dict) -> str:
        if name in self.held and self.held[name] != lineage:
            raise ValueError(f"two different lineages are both called {name}")
        self.held[name] = lineage
        return name


def _next_behind(h) -> dict:
    behind = copy.deepcopy(h["third"].lineage)
    behind["next"] = h["first"].lineage["next"] - 1
    return behind


def list_cases():
    """Lists a caller saves, each as the three texts one call of `scxml_requirement_set` gives:
    the manifest, the sidecar and the lineage that names them by their digests. What a store that
    judges a lineage is tested with, in any language: a lineage and a list that do not belong
    together are refused, so a test cannot make up one from hand-typed hashes."""
    return {name: {"manifest_text": rs.render_manifest(built.manifest),
                   "sidecar_text": rs.render_sidecar(built.sidecar),
                   "lineage_text": rl.render(built.lineage)}
            for name, built in histories().items()}


def of_list_cases():
    """The lineage of a list a work kept: the one saved with it, or the one adopting a list that
    predates lineages makes. The inputs are the texts a work holds of a list; the expectation is
    the lineage or the exact sentence of the refusal."""
    first = histories()["first"]
    manifest = rs.render_manifest(first.manifest)
    sidecar = rs.render_sidecar(first.sidecar)
    lineage = rl.render(first.lineage)

    def case(name, **held):
        return {"name": name, "held": held, "what": "the work has now",
                "expect": outcome(rl.of_list, held, "the work has now")}

    def manifest_with(edit):
        data = json.loads(manifest)
        edit(data)
        return json.dumps(data, indent=2) + "\n"

    def sidecar_with(edit):
        data = json.loads(sidecar)
        edit(data)
        return json.dumps(data, indent=2) + "\n"

    # The same words laid out across lines and spaces: adoption digests them collapsed.
    wrapped = sidecar_with(lambda s: s["text"].update(
        R1="  The lamp\n  starts   off.  "))
    return [
        case("a list that keeps its lineage", manifest_text=manifest, sidecar_text=sidecar,
             lineage_text=lineage),
        case("a lineage with no sidecar beside it", manifest_text=manifest, lineage_text=lineage),
        case("a list made before lineages is adopted", manifest_text=manifest, sidecar_text=sidecar),
        case("adoption reads the words with their whitespace collapsed", manifest_text=manifest,
             sidecar_text=wrapped),
        case("a list with neither a lineage nor a sidecar", manifest_text=manifest),
        case("a lineage that is not JSON", manifest_text=manifest, sidecar_text=sidecar,
             lineage_text="not json"),
        case("a manifest that is not JSON", manifest_text="not json", sidecar_text=sidecar),
        case("a sidecar that is not JSON", manifest_text=manifest, sidecar_text="not json"),
        case("a manifest that is not an object", manifest_text="[]", sidecar_text=sidecar),
        case("a sidecar that is not an object", manifest_text=manifest, sidecar_text="[]"),
        case("a manifest with no document", manifest_text=manifest_with(lambda m: m.pop("doc_id")),
             sidecar_text=sidecar),
        case("a manifest that carries the source's own ids",
             manifest_text=manifest_with(lambda m: m["extraction"].update(ids="native")),
             sidecar_text=sidecar),
        case("a manifest with no extraction",
             manifest_text=manifest_with(lambda m: m.pop("extraction")), sidecar_text=sidecar),
        case("a revision that is not a whole number",
             manifest_text=manifest_with(lambda m: m.update(rev="a")), sidecar_text=sidecar),
        case("a manifest with no revision",
             manifest_text=manifest_with(lambda m: m.pop("rev")), sidecar_text=sidecar),
        case("a requirement the sidecar has no words for", manifest_text=manifest,
             sidecar_text=sidecar_with(lambda s: s["text"].pop("R1"))),
        case("words that are blank", manifest_text=manifest,
             sidecar_text=sidecar_with(lambda s: s["text"].update(R1="  "))),
        case("no requirements", manifest_text=manifest_with(lambda m: m.update(requirements=[])),
             sidecar_text=sidecar),
    ]


def delta_object_cases():
    """The lines `acceptance-delta` writes, gathered into the one object the judgment reads."""
    def case(name, lines):
        return {"name": name, "lines": lines, "expect": verify.delta_object(lines)}

    requirement = {"v": 1, "kind": "acceptance-delta", "requirement": "R1", "evidence": "unchanged"}
    moved = {"v": 1, "kind": "acceptance-delta", "requirement": "R2", "evidence": "changed",
             "moved": ["states.a.transitions[1]"], "gone": 1}
    unclaimed = {"v": 1, "kind": "acceptance-delta-unclaimed", "added": ["states.b"], "gone": 2}
    summary = {"v": 1, "kind": "acceptance-delta-summary", "record": "r", "requirements": 2,
               "unchanged": 1, "changed": 1, "new": 0, "dropped": 0}
    return [
        case("every kind of line", [requirement, moved, unclaimed, summary]),
        case("no summary", [requirement, moved, unclaimed]),
        case("no unclaimed line", [requirement, moved, summary]),
        case("no lines at all", []),
        case("a line of another kind is not gathered", [requirement, {"v": 1, "kind": "other"}, summary]),
        case("the record named by the summary is not kept", [summary]),
    ]


def parse_cases():
    """What reading a lineage's text refuses. Each case breaks one rule of a lineage this module
    wrote and leaves the rest as the first revision's lineage has them, so that the sentence is the
    rule's own."""
    first = histories()["first"].lineage

    def case(name, text):
        return {"name": name, "text": text, "expect": checked(rl.parse, text)}

    def broken(edit):
        lineage = copy.deepcopy(first)
        edit(lineage)
        return json.dumps(lineage)

    def drop(key):
        return broken(lambda l: l.pop(key))

    def revision_row(edit):
        return broken(lambda l: edit(l["revisions"][0]))

    def requirement_row(edit):
        return broken(lambda l: edit(l["requirements"][0]))

    def second_revision(lineage):
        lineage["revisions"].append(dict(lineage["revisions"][0]))

    return [
        case("a lineage a call gave", json.dumps(first)),
        case("text that is not JSON", "not json"),
        case("JSON that is not an object", "[]"),
        case("an object with a key more", broken(lambda l: l.update(extra=1))),
        case("an object with a key less", drop("next")),
        case("another kind", broken(lambda l: l.update(lineage="sce-something-else"))),
        case("another version", broken(lambda l: l.update(v=2))),
        case("no document", broken(lambda l: l.update(doc_id=""))),
        case("a document that is not text", broken(lambda l: l.update(doc_id=7))),
        case("no revision", broken(lambda l: l.update(revisions=[]))),
        case("revisions that are not a list", broken(lambda l: l.update(revisions="one"))),
        case("a revision row with a key more", revision_row(lambda r: r.update(extra=1))),
        case("a revision row with no revision", revision_row(lambda r: r.pop("rev"))),
        case("a revision that is not text", revision_row(lambda r: r.update(rev=1))),
        case("sentence digests that are not a list", revision_row(lambda r: r.update(sentence_sha256="x"))),
        case("a manifest digest that is not text", revision_row(lambda r: r.update(manifest_sha256=7))),
        case("a sidecar digest that is not text", revision_row(lambda r: r.update(sidecar_sha256=7))),
        case("a revision named twice", broken(second_revision)),
        case("requirements that are not a list", broken(lambda l: l.update(requirements="R1"))),
        case("a requirement row with a key more", requirement_row(lambda r: r.update(extra=1))),
        case("a requirement with no quote", requirement_row(lambda r: r.update(quotes=[]))),
        case("a requirement id that is not text", requirement_row(lambda r: r.update(id=1))),
        case("an id issued twice", broken(lambda l: l["requirements"].append(dict(l["requirements"][0])))),
        case("a first revision the lineage has no row for", requirement_row(lambda r: r.update(first_rev="9"))),
        case("a retired revision the lineage has no row for", requirement_row(lambda r: r.update(retired_rev="9"))),
        case("a quote revision the lineage has no row for",
             requirement_row(lambda r: r["quotes"][0].update(rev="9"))),
        case("a quote row with a key more", requirement_row(lambda r: r["quotes"][0].update(extra=1))),
        case("a quote digest that is not text", requirement_row(lambda r: r["quotes"][0].update(sha256=7))),
        case("a next that is not past every id", broken(lambda l: l.update(next=3))),
        case("a next that is not a number", broken(lambda l: l.update(next="six"))),
        case("a next that is a truth value", broken(lambda l: l.update(next=True))),
    ]


def between_cases(table: Lineages):
    h = histories()

    def case(name, older, newer):
        return {"name": name, "older": older, "newer": newer,
                "expect": outcome(rl.between, table.held[older], table.held[newer])}

    other = copy.deepcopy(h["second"].lineage)
    other["doc_id"] = "other"
    named = {key: table.add(key, built.lineage) for key, built in h.items()}
    other, behind = table.add("second, of another specification", other), table.add(
        "third, whose next is behind", _next_behind(h))
    return [
        case("a state against itself", named["first"], named["first"]),
        case("one step says what it dropped", named["first"], named["second"]),
        case("two steps at once", named["first"], named["third"]),
        case("the second against the third", named["second"], named["third"]),
        case("words that changed keep their id and are changed", named["first"], named["reworded"]),
        case("words reworded and reworded back are carried", named["first"], named["back"]),
        case("the same text read into another list", named["first"], named["again"]),
        case("a lineage that goes back in time", named["third"], named["first"]),
        case("a lineage of another specification", named["first"], other),
        case("a lineage whose next is behind", named["third"], behind),
    ]


def extends_cases(table: Lineages):
    h = histories()
    named = {key: table.add(key, built.lineage) for key, built in h.items()}

    def case(name, previous, following):
        return {"name": name, "previous": previous, "following": following,
                "expect": checked(rl.extends, table.held[previous], table.held[following])}

    def broken(built, edit):
        lineage = copy.deepcopy(built.lineage)
        edit(lineage)
        return lineage

    def rewrite_old(lineage):
        lineage["revisions"][0]["spec_sha256"] = "0" * 64

    def rewrite_last(lineage):
        lineage["revisions"][0]["manifest_sha256"] = "0" * 64

    def rewrite_last_words(lineage):
        lineage["revisions"][0]["sidecar_sha256"] = "0" * 64

    def forget(lineage):
        lineage["requirements"] = [r for r in lineage["requirements"] if r["id"] != "R5"]

    def move_first(lineage):
        lineage["requirements"][0]["first_rev"] = "2"

    retired = next(r for r in h["second"].lineage["requirements"] if r["retired_rev"])

    def revive(lineage):
        next(r for r in lineage["requirements"] if r["id"] == retired["id"])["retired_rev"] = None

    def old_words(lineage):
        row = next(r for r in lineage["requirements"] if r["id"] == "R4")
        row["quotes"] = [{"rev": "1", "sha256": "0" * 64}] + row["quotes"][1:]

    def last_words(lineage):
        row = next(r for r in lineage["requirements"] if r["id"] == "R1")
        row["quotes"] = [{"rev": "1", "sha256": "0" * 64}]

    def small_id(lineage):
        lineage["requirements"].append({"id": "R1x", "first_rev": "2", "retired_rev": None,
                                        "quotes": [{"rev": "2", "sha256": "0" * 64}]})

    def old_id(lineage):
        lineage["requirements"].append({"id": "R0", "first_rev": "3", "retired_rev": None,
                                        "quotes": [{"rev": "3", "sha256": "0" * 64}]})

    def variant(name, base, edit):
        return table.add(name, broken(h[base], edit))

    twice = named["reworded then added"]
    return [
        case("a step after a step", named["first"], named["second"]),
        case("the next step", named["second"], named["third"]),
        case("a step after one that is not the last", named["first"], named["third"]),
        case("a lineage extends itself", named["second"], named["second"]),
        case("the same text read into another list", named["first"], named["again"]),
        case("another specification", named["first"],
             variant("second, of another specification", "second", lambda l: l.update(doc_id="other"))),
        case("fewer revisions", named["third"],
             variant("third, with a revision fewer", "third", lambda l: l["revisions"].pop())),
        case("an earlier revision rewritten", named["first"],
             variant("second, its first revision rewritten", "second", rewrite_old)),
        case("a revision with another after it is final", named["first"],
             variant("second, its first manifest digest rewritten", "second", rewrite_last)),
        case("a revision's words digest with another after it is final", named["first"],
             variant("second, its first sidecar digest rewritten", "second", rewrite_last_words)),
        case("an id forgotten", named["second"], variant("third, an id forgotten", "third", forget)),
        case("an id first issued in another revision", named["second"],
             variant("third, an id first issued elsewhere", "third", move_first)),
        case("a retired id made live again", named["second"],
             variant("third, a retired id live again", "third", revive)),
        case("a requirement's history rewritten", named["reworded"], variant(
            "reworded, a requirement's history rewritten", "reworded", old_words)),
        case("the last words replaced in an earlier revision", named["third"],
             variant("third, an old requirement's words replaced", "third", last_words)),
        case("a new id numbered below the work's next", named["second"],
             variant("third, a new id below next", "third", small_id)),
        case("a new id that was already issued by number", named["second"],
             variant("third, a new id numbered R0", "third", old_id)),
        case("an earlier requirement's history kept and one added", named["reworded"], twice),
        case("a next that is behind", named["third"], table.add(
            "third, whose next is behind", _next_behind(h))),
    ]


def belongs_to_list_cases(table: Lineages):
    h = histories()
    third = h["third"]
    text = rs.render_manifest(third.manifest)
    words = rs.render_sidecar(third.sidecar)

    def case(name, lineage, manifest_text, sidecar_text=words):
        return {"name": name, "lineage": lineage, "manifest_text": manifest_text,
                "sidecar_text": sidecar_text,
                "expect": checked(rl.belongs_to_list, table.held[lineage], manifest_text, sidecar_text)}

    other = json.loads(text)
    other["doc_id"] = "other"
    nodigest = copy.deepcopy(third.lineage)
    nodigest["revisions"][-1]["manifest_sha256"] = None
    gone = copy.deepcopy(third.lineage)
    gone["revisions"][-1].pop("manifest_sha256")
    nowords = copy.deepcopy(third.lineage)
    nowords["revisions"][-1]["sidecar_sha256"] = None
    nowords_at_all = copy.deepcopy(third.lineage)
    nowords_at_all["revisions"][-1].pop("sidecar_sha256")
    # The same ids over other sentences: the manifest is coordinates only, so it is the same text.
    reworded = json.loads(words)
    first_id = next(iter(reworded["text"]))
    reworded["text"][first_id] = reworded["text"][first_id] + " Another sentence."
    named = table.add("third", third.lineage)
    return [
        case("the lineage of a call is the lineage of its list", named, text),
        case("the manifest of another revision", named, rs.render_manifest(h["second"].manifest)),
        case("the manifest of another specification", named, json.dumps(other, indent=2) + "\n"),
        case("the same list spelt another way", named, json.dumps(json.loads(text), indent=4) + "\n"),
        case("a lineage with a null digest", table.add("third, a null manifest digest", nodigest), text),
        case("a lineage with no digest at all", table.add("third, no manifest digest", gone), text),
        case("a manifest that is not JSON", named, "not json"),
        case("a manifest that is not an object", named, "[]"),
        case("the sidecar of another revision", named, text, rs.render_sidecar(h["second"].sidecar)),
        case("the same ids over other words", named, text, rs.render_sidecar(reworded)),
        case("the same sidecar spelt another way", named, text, json.dumps(json.loads(words), indent=4) + "\n"),
        case("no sidecar given", named, text, None),
        case("a lineage with a null words digest", table.add("third, a null sidecar digest", nowords), text),
        case("a lineage with no words digest at all",
             table.add("third, no sidecar digest", nowords_at_all), text),
    ]


def build_cases() -> dict:
    table = Lineages()
    between = between_cases(table)
    extends = extends_cases(table)
    belongs_to_list = belongs_to_list_cases(table)
    return {
        "about": "Cases a judgment of a revision has to pass, in any language. Generated by "
                 "tools/authoring/eval/revision_judgment_cases.py from the Python reference; "
                 "do not edit by hand. docs/adr/0011-a-work-keeps-its-requirement-lineage-with-its-requirement-list.md",
        "version": 1,
        "lineages": table.held,
        "join": join_cases(),
        "render": render_cases(),
        "belongs_to": belongs_to_cases(),
        "between": between,
        "extends": extends,
        "belongs_to_list": belongs_to_list,
        "parse": parse_cases(),
        "of_list": of_list_cases(),
        "delta_object": delta_object_cases(),
        "lists": list_cases(),
    }


def text() -> str:
    return json.dumps(build_cases(), indent=1, ensure_ascii=False, sort_keys=True) + "\n"


def main(argv: list[str]) -> int:
    wanted = text()
    if "--check" in argv:
        held = CASES.read_text(encoding="utf-8") if CASES.exists() else ""
        if held != wanted:
            print(f"{CASES} is not what the Python gives: run this script without --check", file=sys.stderr)
            return 1
        return 0
    CASES.parent.mkdir(parents=True, exist_ok=True)
    CASES.write_text(wanted, encoding="utf-8")
    print(f"wrote {CASES} ({len(wanted)} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
