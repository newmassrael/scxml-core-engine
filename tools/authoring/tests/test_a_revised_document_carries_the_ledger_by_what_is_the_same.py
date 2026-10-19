"""A revised document carries the ledger by what is the same, and asks about the rest (ADR 0040).

Each case that must not carry is paired with the nearest one that must. A carry that moves a paragraph whose
words changed passes the first half and loses the one thing an owner has to decide ("five seconds" became
"fifty"), and one that carries nothing passes the second half and makes a revision as much work as the first
division. What is held is that the same words go where they went, that everything else is asked and none of
it is placed on a guess, and that the check of the draft counts exactly what was asked.
"""

from __future__ import annotations

import contextlib
import io
import json
import pathlib
import tempfile
import unittest

from sce_author.__main__ import main
from sce_author.carry import carry_ledger
from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, Place
from sce_author.ledger import LEFT_OUT, NEW, SPEC, Allotment, Destination, Ledger, check_ledger


def document(*rows: tuple[str, str]) -> Extraction:
    """An extraction of (kind, text) rows, numbered in order: `b1`, `b2`, ..."""
    blocks = []
    for number, (kind, text) in enumerate(rows, 1):
        blocks.append(Block(f"b{number}", kind, text, place=Place("spec.md", ordinal=number),
                            **({"level": 1} if kind == "heading" else {})))
    return Extraction("plain", "1", "spec.md", tuple(blocks))


LAMP = Destination(SPEC, spec="lamp-1")
FAN = Destination(NEW, title="Fan")
OUT = Destination(LEFT_OUT)

OLD = document(
    ("paragraph", "Cover page, confidential."),
    ("heading", "Lamp"),
    ("paragraph", "The lamp is on while the supply is present."),
    ("paragraph", "It turns off after five seconds."),
    ("heading", "Fan"),
    ("paragraph", "The fan runs above 40 degrees."),
    ("paragraph", "The fan stops below 30 degrees."),
)


def old_ledger() -> Ledger:
    return Ledger(
        OLD.revision(),
        (
            Allotment(LAMP, ("b2", "b3", "b4")),
            Allotment(FAN, ("b5", "b6", "b7")),
            Allotment(OUT, ("b1",), "the cover"),
        ),
    )


def blocks_of(ledger: Ledger, destination: Destination) -> tuple[str, ...]:
    return next((a.blocks for a in ledger.allotments if a.to == destination), ())


class WhatIsTheSameGoesWhereItWent(unittest.TestCase):
    def test_a_document_sent_again_is_carried_whole_and_asks_nothing(self):
        carried = carry_ledger(OLD, old_ledger(), document(*[(b.kind, b.text) for b in OLD.blocks]))

        self.assertEqual(7, carried.carried)
        self.assertEqual((), carried.changed)
        self.assertEqual((), carried.fresh)
        self.assertEqual((), carried.gone)
        self.assertEqual(("b2", "b3", "b4"), blocks_of(carried.ledger, LAMP))
        self.assertEqual(("b1",), blocks_of(carried.ledger, OUT))
        new = document(*[(b.kind, b.text) for b in OLD.blocks])
        self.assertTrue(check_ledger(new, carried.ledger).complete)

    def test_a_paragraph_is_carried_to_its_new_number_when_the_ones_before_it_moved(self):
        new = document(
            ("paragraph", "A foreword that was not there."),
            *[(b.kind, b.text) for b in OLD.blocks],
        )
        carried = carry_ledger(OLD, old_ledger(), new)

        # The lamp's paragraphs are b3 to b5 now, and the foreword is the one that is asked about.
        self.assertEqual(("b3", "b4", "b5"), blocks_of(carried.ledger, LAMP))
        self.assertEqual(("b1",), carried.fresh)
        self.assertEqual((), carried.changed)

    def test_a_paragraph_shared_by_two_specs_stays_shared_and_one_left_out_keeps_its_reason(self):
        shared = Ledger(
            OLD.revision(),
            (
                Allotment(LAMP, ("b2", "b3", "b4", "b6")),
                Allotment(FAN, ("b5", "b6", "b7")),
                Allotment(OUT, ("b1",), "the cover"),
            ),
        )
        carried = carry_ledger(OLD, shared, document(*[(b.kind, b.text) for b in OLD.blocks]))

        self.assertIn("b6", blocks_of(carried.ledger, LAMP))
        self.assertIn("b6", blocks_of(carried.ledger, FAN))
        left_out = next(a for a in carried.ledger.allotments if a.to == OUT)
        self.assertEqual("the cover", left_out.reason)

    def test_a_sentence_that_occurs_twice_is_carried_by_its_place_and_not_by_its_text_alone(self):
        old = document(
            ("heading", "Lamp"),
            ("paragraph", "See the cover."),
            ("heading", "Fan"),
            ("paragraph", "See the cover."),
        )
        ledger = Ledger(old.revision(), (Allotment(LAMP, ("b1", "b2")), Allotment(FAN, ("b3", "b4"))))
        # The customer removed the lamp's sentence: the one left is the fan's, and goes to the fan.
        new = document(("heading", "Lamp"), ("heading", "Fan"), ("paragraph", "See the cover."))
        carried = carry_ledger(old, ledger, new)

        self.assertEqual(("b1",), blocks_of(carried.ledger, LAMP))
        self.assertEqual(("b2", "b3"), blocks_of(carried.ledger, FAN))
        self.assertEqual(("b2",), tuple(g.block for g in carried.gone))
        self.assertEqual(LAMP, carried.gone[0].was[0].to)


class WhatChangedIsAskedAndNeverCarried(unittest.TestCase):
    def revised(self) -> Extraction:
        return document(
            ("paragraph", "Cover page, confidential."),
            ("heading", "Lamp"),
            ("paragraph", "The lamp is on while the supply is present."),
            ("paragraph", "It turns off after fifty seconds."),
            ("heading", "Fan"),
            ("paragraph", "The fan runs above 40 degrees."),
            ("paragraph", "The fan stops below 30 degrees."),
        )

    def test_a_requirement_that_changed_a_word_is_not_carried_and_points_at_the_one_it_stands_for(self):
        new = self.revised()
        carried = carry_ledger(OLD, old_ledger(), new)

        [changed] = carried.changed
        self.assertEqual("b4", changed.block)
        self.assertEqual("b4", changed.from_block)
        self.assertEqual(LAMP, changed.was[0].to)
        self.assertNotIn("b4", blocks_of(carried.ledger, LAMP))
        self.assertEqual(6, carried.carried)

    def test_the_check_of_the_draft_counts_exactly_what_was_asked_as_unplaced(self):
        new = self.revised()
        carried = carry_ledger(OLD, old_ledger(), new)

        report = check_ledger(new, carried.ledger)
        self.assertFalse(report.complete)
        self.assertEqual(("b4",), report.unplaced)
        asked = [c.block for c in carried.changed] + list(carried.fresh)
        self.assertEqual(sorted(asked), sorted(report.unplaced))

    def test_a_paragraph_with_no_old_one_at_its_place_is_new_and_not_a_change(self):
        new = document(
            *[(b.kind, b.text) for b in OLD.blocks],
            ("paragraph", "The fan has a manual switch."),
        )
        carried = carry_ledger(OLD, old_ledger(), new)

        self.assertEqual(("b8",), carried.fresh)
        self.assertEqual((), carried.changed)
        self.assertEqual(("b8",), check_ledger(new, carried.ledger).unplaced)

    def test_a_paragraph_the_customer_removed_is_named_with_where_it_had_been_placed(self):
        new = document(*[(b.kind, b.text) for b in OLD.blocks if b.id != "b7"])
        carried = carry_ledger(OLD, old_ledger(), new)

        [gone] = carried.gone
        self.assertEqual("b7", gone.block)
        self.assertEqual(FAN, gone.was[0].to)
        # What is gone is not asked about: the new document has nothing there to place.
        self.assertTrue(check_ledger(new, carried.ledger).complete)

    def test_a_spec_that_received_nothing_is_still_one_of_the_division(self):
        new = document(("paragraph", "Cover page, confidential."), ("heading", "Fan"),
                       ("paragraph", "The fan runs above 40 degrees."),
                       ("paragraph", "The fan stops below 30 degrees."))
        carried = carry_ledger(OLD, old_ledger(), new)

        self.assertEqual((), blocks_of(carried.ledger, LAMP))
        self.assertIn(LAMP, carried.destinations)
        self.assertEqual([LAMP, FAN, OUT], list(carried.destinations))

    def test_a_block_with_no_words_is_neither_carried_nor_asked(self):
        old = Extraction("plain", "1", "spec.md", OLD.blocks + (
            Block("b8", "table", "|  |", place=Place("spec.md", ordinal=8), cells=(("",),)),))
        ledger = Ledger(old.revision(), (Allotment(LAMP, ("b2", "b3", "b4", "b8")),
                                         Allotment(FAN, ("b5", "b6", "b7")), Allotment(OUT, ("b1",), "x")))
        carried = carry_ledger(old, ledger, old)

        self.assertEqual(7, carried.carried)
        self.assertNotIn("b8", blocks_of(carried.ledger, LAMP))
        self.assertEqual((), carried.fresh)
        self.assertEqual((), carried.gone)


class ADivisionIsOnlyCarriedFromTheDocumentItWasMadeFor(unittest.TestCase):
    def test_a_ledger_of_another_extraction_is_refused_and_the_one_of_this_is_carried(self):
        other = document(("paragraph", "Not the same document."))
        with self.assertRaises(IngestError):
            carry_ledger(other, old_ledger(), OLD)
        carry_ledger(OLD, old_ledger(), OLD)


class TheCommandLine(unittest.TestCase):
    def run_main(self, *argv: str) -> tuple[int, str]:
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            code = main(list(argv))
        return code, out.getvalue()

    def test_a_draft_is_written_and_the_report_says_ids_and_counts_and_no_words_of_the_document(self):
        revised = document(
            ("paragraph", "Cover page, confidential."),
            ("heading", "Lamp"),
            ("paragraph", "The lamp is on while the supply is present."),
            ("paragraph", "It turns off after fifty seconds."),
            ("heading", "Fan"),
            ("paragraph", "The fan runs above 40 degrees."),
        )
        with tempfile.TemporaryDirectory() as folder:
            work = pathlib.Path(folder)
            old_e, new_e, old_l, draft = work / "o.json", work / "n.json", work / "ol.json", work / "d.json"
            old_e.write_text(OLD.to_json(), encoding="utf-8")
            new_e.write_text(revised.to_json(), encoding="utf-8")
            old_l.write_text(old_ledger().to_json(), encoding="utf-8")
            code, said = self.run_main(
                "carry-ledger", "--old-extraction", str(old_e), "--old-ledger", str(old_l),
                "--new-extraction", str(new_e), "--out", str(draft), "--json")

            self.assertEqual(0, code)
            report = json.loads(said)
            self.assertEqual(revised.revision(), report["extraction"])
            self.assertEqual(OLD.revision(), report["old_extraction"])
            self.assertEqual(Ledger.from_json(draft.read_text(encoding="utf-8")).revision(), report["ledger"])
            self.assertEqual(["b4"], [c["block"] for c in report["changed"]])
            self.assertEqual(["b7"], [g["block"] for g in report["gone"]])
            self.assertEqual(3, len(report["destinations"]))
            self.assertNotIn("fifty", said)
            self.assertNotIn("lamp is on", said)

            # The draft is what `make-ledger`'s plan can then be completed from: it is a ledger of the new text.
            code, said = self.run_main(
                "check-ledger", "--extraction", str(new_e), "--ledger", str(draft), "--json")
            self.assertEqual(1, code)
            self.assertEqual(["b4"], json.loads(said)["unplaced"])


class APlanCanNameParagraphsOneByOne(unittest.TestCase):
    """What a revised document is divided by: the draft's paragraphs and the owner's answers, by block."""

    def test_blocks_are_taken_as_they_are_and_added_to_the_parts_and_none_is_taken_twice(self):
        from sce_author.ledger import make_ledger

        ledger = make_ledger(OLD, {"v": 1, "allotments": [
            {"to": {"kind": "spec", "spec": "lamp-1"}, "sections": ["s2"], "blocks": ["b7", "b6"]},
            {"to": {"kind": "left-out"}, "reason": "x", "blocks": ["b1"]},
        ]})

        # The lamp's part (b2 to b4) and the two paragraphs named, in the extraction's order.
        self.assertEqual(("b2", "b3", "b4", "b6", "b7"), blocks_of(ledger, LAMP))
        self.assertEqual(("b1",), blocks_of(ledger, OUT))

    def test_a_block_the_extraction_has_not_is_refused_and_one_it_has_is_not(self):
        from sce_author.ledger import make_ledger

        plan = lambda block: {"v": 1, "allotments": [{"to": {"kind": "left-out"}, "blocks": [block]}]}
        with self.assertRaises(IngestError):
            make_ledger(OLD, plan("b99"))
        make_ledger(OLD, plan("b7"))

    def test_an_allotment_of_nothing_is_still_refused(self):
        from sce_author.ledger import make_ledger

        with self.assertRaises(IngestError):
            make_ledger(OLD, {"v": 1, "allotments": [{"to": {"kind": "left-out"}, "blocks": []}]})


if __name__ == "__main__":
    unittest.main()
