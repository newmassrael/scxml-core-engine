"""The ledger of a divided document (ADR 0040): where each paragraph went, counted by SCE.

Each case that must refuse is paired with the nearest case that must pass. A ledger that is accepted whatever
it holds passes the first half, and one that refuses everything passes the second; a check that calls a ledger
complete with a paragraph in no place is the loss the ledger exists to end, and one that counts a left-out
paragraph as unplaced makes leaving anything out impossible.
"""

from __future__ import annotations

import contextlib
import io
import json
import pathlib
import tempfile
import unittest

from sce_author.__main__ import main
from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, Place
from sce_author.ledger import (
    LEFT_OUT,
    NEW,
    SPEC,
    Allotment,
    Destination,
    Ledger,
    check_ledger,
    make_ledger,
    scopes_of,
)


def block(identifier: str, kind: str = "paragraph", text: str = "", **kwargs) -> Block:
    kwargs.setdefault("place", Place("spec.md", ordinal=int(identifier[1:])))
    return Block(identifier, kind, text, **kwargs)


def document() -> Extraction:
    """A cover, two functions (the first with a part under it) and an empty row that has no words."""
    return Extraction(
        "plain",
        "1",
        "spec.md",
        (
            block("b1", text="Cover page, confidential."),
            block("b2", "heading", "Lamp", level=1),
            block("b3", text="The lamp is on while the supply is present."),
            block("b4", "heading", "Timing", level=2),
            block("b5", text="It turns off after 30 seconds."),
            block("b6", "heading", "Fan", level=1),
            block("b7", text="The fan runs above 40 degrees."),
            block("b8", "table", "|  |", cells=(("",),)),
        ),
    )


def to_spec(identifier: str) -> Destination:
    return Destination(SPEC, spec=identifier)


def to_new(title: str) -> Destination:
    return Destination(NEW, title=title)


OUT = Destination(LEFT_OUT)


def ledger_of(extraction: Extraction, *allotments: Allotment) -> Ledger:
    return Ledger(extraction.revision(), tuple(allotments))


def whole_division(extraction: Extraction) -> Ledger:
    """The lamp, the fan and the cover each go somewhere; nothing is unplaced."""
    return ledger_of(
        extraction,
        Allotment(to_spec("lamp-1"), ("b2", "b3", "b4", "b5")),
        Allotment(to_new("Fan"), ("b6", "b7")),
        Allotment(OUT, ("b1", "b8"), "the cover"),
    )


class TheFormOfALedger(unittest.TestCase):
    revision = "a" * 64

    def test_each_defect_is_refused_and_the_nearest_good_ledger_is_not(self):
        good = {
            "v": 1,
            "extraction": self.revision,
            "allotments": [
                {"to": {"kind": "spec", "spec": "lamp-1"}, "blocks": ["b1", "b2"]},
                {"to": {"kind": "left-out"}, "reason": "the cover", "blocks": ["b3"]},
            ],
        }
        Ledger.from_dict(good)

        def with_allotment(**changes):
            first = dict(good["allotments"][0], **changes)
            return {**good, "allotments": [first, good["allotments"][1]]}

        for name, broken in {
            "another version": {**good, "v": 2},
            "a field nobody defined": {**good, "colour": "red"},
            "an extraction that is not a revision": {**good, "extraction": "short"},
            "allotments that are not a list": {**good, "allotments": "b1"},
            "an allotment of no block": with_allotment(blocks=[]),
            "a block with no id": with_allotment(blocks=["b1", ""]),
            "a block twice in an allotment": with_allotment(blocks=["b1", "b1"]),
            "blocks that are not a list": with_allotment(blocks="b1"),
            "a kind nobody defined": with_allotment(to={"kind": "elsewhere"}),
            "a spec with no id": with_allotment(to={"kind": "spec"}),
            "a spec id that is not one": with_allotment(to={"kind": "spec", "spec": "Lamp/../x"}),
            "a spec with a title as well": with_allotment(to={"kind": "spec", "spec": "lamp-1", "title": "x"}),
            "a new spec with no title": with_allotment(to={"kind": "new", "title": "  "}),
            "a left-out part that names a spec": with_allotment(to={"kind": "left-out", "spec": "lamp-1"}),
            "a reason for a part that is not left out": with_allotment(reason="why"),
            "a field of the destination nobody defined": with_allotment(to={"kind": "spec", "spec": "lamp-1", "x": 1}),
        }.items():
            with self.subTest(name), self.assertRaises(IngestError):
                Ledger.from_dict(broken)
        with self.assertRaises(IngestError):
            Ledger.from_json("not json")

    def test_two_allotments_to_one_destination_are_refused_and_two_left_out_for_different_reasons_are_not(self):
        spec = Allotment(to_spec("lamp-1"), ("b2",))
        with self.assertRaises(IngestError):
            Ledger("a" * 64, (spec, Allotment(to_spec("lamp-1"), ("b3",))))
        with self.assertRaises(IngestError):
            Ledger("a" * 64, (Allotment(to_new("Fan"), ("b6",)), Allotment(to_new("Fan"), ("b7",))))
        with self.assertRaises(IngestError):
            Ledger("a" * 64, (Allotment(OUT, ("b1",), "cover"), Allotment(OUT, ("b3",), "cover")))
        Ledger("a" * 64, (Allotment(OUT, ("b1",), "cover"), Allotment(OUT, ("b3",), "contract terms")))

    def test_a_long_title_and_a_long_reason_are_refused(self):
        with self.assertRaises(IngestError):
            to_new("t" * 201)
        to_new("t" * 200)
        with self.assertRaises(IngestError):
            Allotment(OUT, ("b1",), "r" * 1001)
        Allotment(OUT, ("b1",), "r" * 1000)

    def test_the_same_division_has_one_revision_however_it_was_listed(self):
        extraction = document()
        one = whole_division(extraction)
        parts = list(one.allotments)

        reversed_ = ledger_of(extraction, *reversed(parts))

        self.assertEqual(one.revision(), reversed_.revision())
        self.assertEqual(one.to_json(), reversed_.to_json())
        self.assertEqual(one.revision(), Ledger.from_json(one.to_json()).revision())
        # The nearest that must differ: one paragraph moved to the other spec.
        moved = ledger_of(
            extraction,
            Allotment(to_spec("lamp-1"), ("b2", "b3", "b4")),
            Allotment(to_new("Fan"), ("b5", "b6", "b7")),
            Allotment(OUT, ("b1", "b8"), "the cover"),
        )
        self.assertNotEqual(one.revision(), moved.revision())

    def test_a_reason_is_kept_for_what_is_left_out_even_when_it_is_empty(self):
        said = Allotment(OUT, ("b1",)).to_dict()

        self.assertEqual(said, {"to": {"kind": "left-out"}, "blocks": ["b1"], "reason": ""})
        self.assertNotIn("reason", Allotment(to_spec("lamp-1"), ("b1",)).to_dict())

    def test_it_is_held_to_the_extraction_it_is_of(self):
        extraction = document()
        ledger = whole_division(extraction)
        self.assertIs(ledger.held_by(extraction), ledger)

        other = Extraction("plain", "1", "spec.md", extraction.blocks[:-1])
        with self.assertRaises(IngestError):
            ledger.held_by(other)
        unknown = ledger_of(extraction, Allotment(to_spec("lamp-1"), ("b2", "b99")))
        with self.assertRaises(IngestError):
            unknown.held_by(extraction)
        wrong_order = ledger_of(extraction, Allotment(to_spec("lamp-1"), ("b3", "b2")))
        with self.assertRaises(IngestError):
            wrong_order.held_by(extraction)


class TheCountOfALedger(unittest.TestCase):
    def setUp(self) -> None:
        self.extraction = document()

    def test_every_paragraph_with_words_in_a_place_is_complete(self):
        report = check_ledger(self.extraction, whole_division(self.extraction))

        self.assertTrue(report.complete)
        self.assertEqual((report.words, report.in_specs, report.left_out), (7, 6, 1))
        self.assertEqual((report.unplaced, report.contradicting, report.shared, report.empty), ((), (), (), ()))

    def test_the_counts_add_up_to_the_paragraphs_with_words(self):
        # What a person is told must be a partition: in specs, left out only, and unplaced are all of it.
        ledger = ledger_of(
            self.extraction,
            Allotment(to_spec("lamp-1"), ("b2", "b3")),
            Allotment(OUT, ("b1",), "the cover"),
        )

        report = check_ledger(self.extraction, ledger)

        self.assertEqual(report.words, report.in_specs + report.left_out + len(report.unplaced))
        self.assertEqual((report.in_specs, report.left_out, len(report.unplaced)), (2, 1, 4))

    def test_a_paragraph_in_no_allotment_is_unplaced_and_the_ledger_is_not_complete(self):
        ledger = ledger_of(
            self.extraction,
            Allotment(to_spec("lamp-1"), ("b2", "b3", "b4", "b5")),
            Allotment(OUT, ("b1",), "the cover"),
        )

        report = check_ledger(self.extraction, ledger)

        self.assertFalse(report.complete)
        self.assertEqual(report.unplaced, ("b6", "b7"))
        # The nearest that must pass: the same ledger with the fan given a place.
        self.assertTrue(check_ledger(self.extraction, whole_division(self.extraction)).complete)

    def test_a_ledger_that_places_nothing_names_every_paragraph_with_words(self):
        report = check_ledger(self.extraction, ledger_of(self.extraction))

        self.assertEqual(report.unplaced, ("b1", "b2", "b3", "b4", "b5", "b6", "b7"))
        self.assertFalse(report.complete)

    def test_a_block_with_no_words_has_nothing_to_place_and_may_be_placed_anyway(self):
        unplaced_empty = ledger_of(
            self.extraction,
            Allotment(to_spec("lamp-1"), ("b2", "b3", "b4", "b5")),
            Allotment(to_new("Fan"), ("b6", "b7")),
            Allotment(OUT, ("b1",), "the cover"),
        )

        self.assertTrue(check_ledger(self.extraction, unplaced_empty).complete)
        self.assertTrue(check_ledger(self.extraction, whole_division(self.extraction)).complete)

    def test_a_paragraph_that_is_used_and_left_out_contradicts(self):
        ledger = ledger_of(
            self.extraction,
            Allotment(to_spec("lamp-1"), ("b1", "b2", "b3", "b4", "b5")),
            Allotment(to_new("Fan"), ("b6", "b7")),
            Allotment(OUT, ("b1",), "the cover"),
        )

        report = check_ledger(self.extraction, ledger)

        self.assertEqual(report.contradicting, ("b1",))
        self.assertFalse(report.complete)
        # It is counted once, as used.
        self.assertEqual(report.words, report.in_specs + report.left_out + len(report.unplaced))

    def test_a_paragraph_in_two_specs_is_shared_and_that_is_not_a_fault(self):
        ledger = ledger_of(
            self.extraction,
            Allotment(to_spec("lamp-1"), ("b1", "b2", "b3", "b4", "b5")),
            Allotment(to_new("Fan"), ("b1", "b6", "b7")),
        )

        report = check_ledger(self.extraction, ledger)

        self.assertEqual(report.shared, ("b1",))
        self.assertTrue(report.complete)

    def test_a_spec_given_nothing_with_words_is_named_and_blocks_the_ledger(self):
        ledger = ledger_of(
            self.extraction,
            Allotment(to_spec("lamp-1"), ("b2", "b3", "b4", "b5")),
            Allotment(to_new("Empty"), ("b8",)),
            Allotment(to_spec("fan-1"), ("b6", "b7")),
            Allotment(OUT, ("b1",), "the cover"),
        )

        report = check_ledger(self.extraction, ledger)

        self.assertEqual(report.empty, (2,))  # in the ledger's order: specs by id, then new ones by title
        self.assertEqual(report.unplaced, ())
        self.assertFalse(report.complete)

    def test_what_is_counted_is_per_destination(self):
        report = check_ledger(self.extraction, whole_division(self.extraction))

        self.assertEqual(
            [(d.to.kind, d.blocks, d.with_words) for d in report.destinations],
            [(SPEC, 4, 4), (NEW, 2, 2), (LEFT_OUT, 2, 1)],
        )

    def test_a_ledger_of_another_extraction_is_refused_and_no_report_is_made(self):
        other = Extraction("plain", "1", "spec.md", self.extraction.blocks[:-1])

        with self.assertRaises(IngestError):
            check_ledger(other, whole_division(self.extraction))

    def test_the_report_says_ids_and_counts_and_never_the_documents_words(self):
        said = json.dumps(check_ledger(self.extraction, whole_division(self.extraction)).to_dict())

        for word in ("confidential", "lamp is on", "fan runs"):
            self.assertNotIn(word, said)


class AnAllotmentMadeFromParts(unittest.TestCase):
    def setUp(self) -> None:
        self.extraction = document()

    def plan(self, *allotments):
        return {"v": 1, "allotments": list(allotments)}

    def test_parts_are_expanded_to_the_blocks_they_are_and_a_tree_takes_what_is_under_it(self):
        # The outline: s1 the cover, s2 Lamp (b2, b3), s3 Timing (b4, b5), s4 Fan (b6, b7, b8).
        plan = self.plan(
            {"to": {"kind": "spec", "spec": "lamp-1"}, "trees": ["s2"]},
            {"to": {"kind": "new", "title": "Fan"}, "sections": ["s4"]},
            {"to": {"kind": "left-out"}, "reason": "the cover", "sections": ["s1"]},
        )

        ledger = make_ledger(self.extraction, plan)

        by = {a.identity: a.blocks for a in ledger.allotments}
        self.assertEqual(by[(SPEC, "lamp-1")], ("b2", "b3", "b4", "b5"))
        self.assertEqual(by[(NEW, "Fan")], ("b6", "b7", "b8"))
        self.assertEqual(by[(LEFT_OUT, "the cover")], ("b1",))
        self.assertTrue(check_ledger(self.extraction, ledger).complete)

    def test_a_part_under_a_chosen_one_can_go_elsewhere(self):
        plan = self.plan(
            {"to": {"kind": "spec", "spec": "lamp-1"}, "sections": ["s2"]},
            {"to": {"kind": "spec", "spec": "timing-1"}, "sections": ["s3"]},
        )

        ledger = make_ledger(self.extraction, plan)

        by = {a.identity[1]: a.blocks for a in ledger.allotments}
        self.assertEqual((by["lamp-1"], by["timing-1"]), (("b2", "b3"), ("b4", "b5")))

    def test_a_part_may_go_to_two_destinations(self):
        plan = self.plan(
            {"to": {"kind": "spec", "spec": "lamp-1"}, "sections": ["s1", "s2"]},
            {"to": {"kind": "spec", "spec": "fan-1"}, "sections": ["s1", "s4"]},
        )

        report = check_ledger(self.extraction, make_ledger(self.extraction, plan))

        self.assertEqual(report.shared, ("b1",))

    def test_each_way_a_plan_is_wrong_is_refused_and_the_nearest_good_plan_is_not(self):
        good = {"to": {"kind": "spec", "spec": "lamp-1"}, "sections": ["s2"]}
        make_ledger(self.extraction, self.plan(good))
        for name, plan in {
            "a part nobody listed": self.plan({**good, "sections": ["s99"]}),
            "an allotment naming no part": self.plan({"to": good["to"]}),
            "parts that are not a list": self.plan({**good, "sections": "s2"}),
            "a field nobody defined": self.plan({**good, "colour": "red"}),
            "another version": {"v": 2, "allotments": [good]},
            "a destination twice": self.plan(good, {**good, "sections": ["s4"]}),
            "allotments that are not a list": {"v": 1, "allotments": "x"},
        }.items():
            with self.subTest(name), self.assertRaises(IngestError):
                make_ledger(self.extraction, plan)


class TheScopesAScopedImportIsMadeUnder(unittest.TestCase):
    def setUp(self) -> None:
        self.extraction = document()

    def test_each_spec_has_the_scope_of_its_allotment_and_what_is_left_out_has_none(self):
        made = scopes_of(self.extraction, whole_division(self.extraction))

        self.assertEqual([a.to.kind for a, _ in made], [SPEC, NEW])
        self.assertEqual([s.blocks for _, s in made], [("b2", "b3", "b4", "b5"), ("b6", "b7")])
        for _, scope in made:
            assert scope is not None
            scope.held_by(self.extraction)

    def test_an_allotment_of_every_block_is_no_scope(self):
        everything = ledger_of(self.extraction, Allotment(to_spec("all-1"), tuple(b.id for b in self.extraction.blocks)))

        made = scopes_of(self.extraction, everything)

        self.assertEqual([(a.to.spec, s) for a, s in made], [("all-1", None)])

    def test_a_ledger_that_is_not_complete_is_refused_and_a_complete_one_is_not(self):
        incomplete = ledger_of(self.extraction, Allotment(to_spec("lamp-1"), ("b2", "b3")))

        with self.assertRaises(IngestError):
            scopes_of(self.extraction, incomplete)
        scopes_of(self.extraction, whole_division(self.extraction))


class TheCommandLine(unittest.TestCase):
    def run_main(self, *argv: str) -> tuple[int, str]:
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            code = main(list(argv))
        return code, out.getvalue()

    def test_a_plan_becomes_a_ledger_that_is_counted_and_then_given_to_each_spec_as_a_scope(self):
        with tempfile.TemporaryDirectory() as folder:
            work = pathlib.Path(folder)
            e, p, ledger, scopes = work / "e.json", work / "p.json", work / "l.json", work / "scopes"
            e.write_text(document().to_json(), encoding="utf-8")
            plan = {
                "v": 1,
                "allotments": [
                    {"to": {"kind": "spec", "spec": "lamp-1"}, "trees": ["s2"]},
                    {"to": {"kind": "new", "title": "Fan"}, "sections": ["s4"]},
                ],
            }
            p.write_text(json.dumps(plan), encoding="utf-8")

            code, said = self.run_main(
                "make-ledger", "--extraction", str(e), "--plan", str(p), "--out", str(ledger), "--json"
            )
            made = json.loads(said)
            self.assertEqual(code, 0)
            self.assertFalse(made["complete"])
            self.assertEqual(made["unplaced"], ["b1"])
            self.assertEqual(made["revision"], Ledger.from_json(ledger.read_text()).revision())

            # The cover has no place: the count says so and the exit code is the verdict.
            code, said = self.run_main("check-ledger", "--extraction", str(e), "--ledger", str(ledger), "--json")
            self.assertEqual((code, json.loads(said)["unplaced"]), (1, ["b1"]))
            code, _ = self.run_main("ledger-scopes", "--extraction", str(e), "--ledger", str(ledger), "--out-dir", str(scopes))
            self.assertEqual(code, 1)
            self.assertFalse(scopes.exists())

            # The nearest that must pass: the same plan with the cover left out on purpose.
            plan["allotments"].append({"to": {"kind": "left-out"}, "reason": "the cover", "sections": ["s1"]})
            p.write_text(json.dumps(plan), encoding="utf-8")
            code, _ = self.run_main("make-ledger", "--extraction", str(e), "--plan", str(p), "--out", str(ledger))
            self.assertEqual(code, 0)
            code, said = self.run_main("check-ledger", "--extraction", str(e), "--ledger", str(ledger), "--json")
            self.assertEqual((code, json.loads(said)["complete"]), (0, True))

            code, said = self.run_main(
                "ledger-scopes", "--extraction", str(e), "--ledger", str(ledger), "--out-dir", str(scopes)
            )
            listing = json.loads(said)["scopes"]
            self.assertEqual(code, 0)
            self.assertEqual([(s["to"]["kind"], s["blocks"]) for s in listing], [("spec", 4), ("new", 3)])
            first = json.loads((scopes / listing[0]["scope"]).read_text())
            self.assertEqual(first["blocks"], ["b2", "b3", "b4", "b5"])
            # No word of the document is printed by any of them.
            self.assertNotIn("lamp is on", said)

    def test_a_ledger_of_another_document_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            work = pathlib.Path(folder)
            e, other, ledger = work / "e.json", work / "o.json", work / "l.json"
            e.write_text(document().to_json(), encoding="utf-8")
            extraction = document()
            other.write_text(Extraction("plain", "1", "spec.md", extraction.blocks[:-1]).to_json(), encoding="utf-8")
            ledger.write_text(whole_division(extraction).to_json(), encoding="utf-8")

            code, _ = self.run_main("check-ledger", "--extraction", str(other), "--ledger", str(ledger))

            self.assertEqual(code, 2)
            code, _ = self.run_main("check-ledger", "--extraction", str(e), "--ledger", str(ledger))
            self.assertEqual(code, 0)


if __name__ == "__main__":
    unittest.main()
