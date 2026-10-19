"""The scope of an import (ADR 0039): the owner's choice of a part of a document, held by the cut, the check
and the record.

Each case that must refuse is paired with the nearest case that must pass. A scope that is accepted whatever
it names passes the first half, and one that refuses everything passes the second; a check that counts what
is outside the part as missing is the very thing the scope exists to end, and one that lets a conversion
cite outside it lets the maker widen what the owner chose.
"""

from __future__ import annotations

import contextlib
import io
import json
import pathlib
import tempfile
import unittest

from sce_author.__main__ import main
from sce_author.conversion import (
    CHECK,
    Converter,
    Conversion,
    MarkdownBlock,
    check_conversion,
    convert_by_rules,
)
from sce_author.conversion_by_model import split_units
from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, Place
from sce_author.scope import Scope, outline, scope_of, whole


def block(identifier: str, kind: str = "paragraph", text: str = "", page: int | None = None, **kwargs) -> Block:
    kwargs.setdefault("place", Place("spec.md", ordinal=int(identifier[1:]), page=page))
    return Block(identifier, kind, text, **kwargs)


def extraction_of(*blocks: Block) -> Extraction:
    return Extraction("plain", "1", "spec.md", tuple(blocks))


def two_functions() -> Extraction:
    """A document that holds two functions and a cover, as a mixed specification does."""
    return extraction_of(
        block("b1", text="Cover page, confidential."),
        block("b2", "heading", "Lamp", level=1),
        block("b3", text="The lamp is on while the supply is present."),
        block("b4", "heading", "Timing", level=2),
        block("b5", text="It turns off after 30 seconds."),
        block("b6", "heading", "Fan", level=1),
        block("b7", text="The fan runs above 40 degrees."),
    )


def sections_by_title(extraction: Extraction) -> dict[str, str]:
    _, sections = outline(extraction)
    return {s.title or f"({s.kind})": s.id for s in sections}


class TheOutlineOfADocument(unittest.TestCase):
    def test_headings_nest_by_level_and_what_precedes_the_first_is_a_section_of_its_own(self):
        way, sections = outline(two_functions())

        self.assertEqual(way, "headings")
        self.assertEqual([(s.id, s.title, s.parent) for s in sections], [
            ("s1", "", None),
            ("s2", "Lamp", None),
            ("s3", "Timing", "s2"),
            ("s4", "Fan", None),
        ])
        self.assertEqual([s.kind for s in sections], ["preamble", "heading", "heading", "heading"])

    def test_what_precedes_the_first_heading_ends_at_it_whatever_level_the_headings_are(self):
        # A PDF whose headings are all one level below its title has no heading at the first level: the part
        # before the first heading must still end, or it would hold the whole document.
        extraction = extraction_of(
            block("b1", text="Cover."),
            block("b2", "heading", "One", level=2),
            block("b3", text="First."),
            block("b4", "heading", "Two", level=2),
            block("b5", text="Second."),
        )

        _, sections = outline(extraction)

        self.assertEqual([(s.id, s.kind, s.blocks) for s in sections], [
            ("s1", "preamble", 1),
            ("s2", "heading", 2),
            ("s3", "heading", 2),
        ])
        self.assertEqual(scope_of(extraction, ["s3"]).blocks, ("b4", "b5"))

    def test_a_section_counts_its_own_blocks_and_says_what_it_comes_to_with_the_sections_under_it(self):
        _, sections = outline(two_functions())
        by_id = {s.id: s for s in sections}

        self.assertEqual((by_id["s2"].blocks, by_id["s2"].tree_blocks), (2, 4))  # its heading and paragraph; with Timing
        self.assertEqual((by_id["s3"].blocks, by_id["s3"].tree_blocks), (2, 2))
        self.assertEqual((by_id["s1"].blocks, by_id["s1"].tree_blocks), (1, 1))
        # What the parts add up to is the document and no more: nothing is counted in two parts.
        self.assertEqual(sum(s.blocks for s in sections), 7)
        self.assertEqual(
            sum(s.characters for s in sections), sum(len(b.text) for b in two_functions().blocks)
        )

    def test_a_document_with_no_heading_is_divided_by_its_pages(self):
        pages = extraction_of(
            block("b1", text="One.", page=1), block("b2", text="Two.", page=1), block("b3", text="Three.", page=2)
        )

        way, sections = outline(pages)

        self.assertEqual(way, "pages")
        self.assertEqual([(s.id, s.blocks, s.page_first, s.page_last) for s in sections], [("s1", 2, 1, 1), ("s2", 1, 2, 2)])

    def test_a_document_with_neither_is_one_section(self):
        way, sections = outline(extraction_of(block("b1", text="One."), block("b2", text="Two.")))

        self.assertEqual((way, [(s.id, s.blocks) for s in sections]), ("whole", [("s1", 2)]))

    def test_a_division_that_the_document_cannot_have_is_refused_and_not_made_up(self):
        with self.assertRaises(IngestError):
            outline(extraction_of(block("b1", text="One.")), "headings")
        with self.assertRaises(IngestError):
            outline(extraction_of(block("b1", text="One.")), "pages")
        with self.assertRaises(IngestError):
            outline(extraction_of(block("b1", text="One.")), "chapters")
        # The nearest that must pass: a document that has the division asked for.
        self.assertEqual(outline(two_functions(), "headings")[0], "headings")


class AScopeFromTheOwnersChoice(unittest.TestCase):
    def test_a_section_is_its_own_blocks_and_a_tree_is_the_section_with_everything_under_it(self):
        extraction = two_functions()
        ids = sections_by_title(extraction)

        self.assertEqual(scope_of(extraction, [ids["Lamp"]]).blocks, ("b2", "b3"))
        self.assertEqual(scope_of(extraction, [ids["Timing"]]).blocks, ("b4", "b5"))
        self.assertEqual(scope_of(extraction, [], tree_ids=[ids["Lamp"]]).blocks, ("b2", "b3", "b4", "b5"))
        # Named both ways it is taken once.
        self.assertEqual(scope_of(extraction, [ids["Lamp"]], tree_ids=[ids["Lamp"]]).blocks, ("b2", "b3", "b4", "b5"))

    def test_a_part_under_a_chosen_one_can_be_left_out(self):
        # The screen ticks a function and its parts together, and lets a person untick one of the parts: what
        # is named is exactly what is taken, so the unticked part is not carried in by the one above it.
        extraction = two_functions()
        ids = sections_by_title(extraction)

        scope = scope_of(extraction, [ids["Lamp"], ids["Fan"]])

        self.assertEqual(scope.blocks, ("b2", "b3", "b6", "b7"))
        self.assertFalse(scope.covers("b4"))
        self.assertFalse(scope.covers("b5"))

    def test_the_blocks_are_in_the_documents_order_whatever_order_the_sections_were_chosen_in(self):
        extraction = two_functions()
        ids = sections_by_title(extraction)

        scope = scope_of(extraction, [ids["Fan"], ids["Lamp"], ids["Timing"]])

        self.assertEqual(scope.blocks, ("b2", "b3", "b4", "b5", "b6", "b7"))

    def test_nothing_chosen_and_a_section_that_is_not_there_are_refused(self):
        extraction = two_functions()
        with self.assertRaises(IngestError):
            scope_of(extraction, [])
        with self.assertRaises(IngestError):
            scope_of(extraction, ["s99"])
        self.assertTrue(scope_of(extraction, ["s2"]).blocks)

    def test_a_scope_is_named_by_its_text(self):
        extraction = two_functions()
        one = scope_of(extraction, ["s2"])

        self.assertEqual(one.revision(), Scope.from_json(one.to_json()).revision())
        self.assertNotEqual(one.revision(), scope_of(extraction, ["s4"]).revision())


class AScopeThatIsNotWellFormed(unittest.TestCase):
    revision = "a" * 64

    def test_each_defect_is_refused_and_the_nearest_good_scope_is_not(self):
        good = {"v": 1, "extraction": self.revision, "blocks": ["b1", "b2"]}
        Scope.from_dict(good)
        for broken in (
            {**good, "blocks": []},
            {**good, "blocks": ["b1", "b1"]},
            {**good, "blocks": ["b1", ""]},
            {**good, "blocks": "b1"},
            {**good, "extraction": "short"},
            {**good, "v": 2},
            {**good, "extra": 1},
        ):
            with self.subTest(broken=broken), self.assertRaises(IngestError):
                Scope.from_dict(broken)
        with self.assertRaises(IngestError):
            Scope.from_json("not json")

    def test_it_is_held_to_the_extraction_it_is_of(self):
        extraction = two_functions()
        scope = scope_of(extraction, ["s2"])
        self.assertIs(scope.held_by(extraction), scope)

        other = extraction_of(*extraction.blocks[:-1])
        with self.assertRaises(IngestError):
            scope.held_by(other)
        wrong_order = Scope(extraction.revision(), ("b3", "b2"))
        with self.assertRaises(IngestError):
            wrong_order.held_by(extraction)
        unknown = Scope(extraction.revision(), ("b2", "b99"))
        with self.assertRaises(IngestError):
            unknown.held_by(extraction)


class TheCheckUnderAScope(unittest.TestCase):
    def setUp(self) -> None:
        self.extraction = two_functions()
        self.lamp = scope_of(self.extraction, [], tree_ids=["s2"])

    def test_the_part_converted_by_rule_is_complete_and_the_rest_is_counted_not_asked_about(self):
        made = convert_by_rules(self.extraction, self.lamp)

        report = check_conversion(self.extraction, made, self.lamp)

        self.assertTrue(report.complete)
        self.assertEqual([m.cites for m in made.blocks], [("b2",), ("b3",), ("b4",), ("b5",)])
        self.assertEqual(report.missing, ())
        self.assertEqual(report.out_of_scope, 3)  # the cover, the fan heading and its paragraph
        self.assertEqual(report.scope, self.lamp.revision())
        self.assertEqual(report.carried_text, 4)

    def test_the_same_conversion_under_no_scope_leaves_the_rest_missing(self):
        made = convert_by_rules(self.extraction, self.lamp)

        report = check_conversion(self.extraction, made)

        self.assertFalse(report.complete)
        self.assertEqual(report.missing, ("b1", "b6", "b7"))
        self.assertEqual((report.scope, report.out_of_scope), ("", 0))

    def test_a_block_left_out_inside_the_part_is_still_missing(self):
        made = convert_by_rules(self.extraction, self.lamp)
        short = Conversion(made.converter, made.extraction, made.blocks[:-1])

        report = check_conversion(self.extraction, short, self.lamp)

        self.assertEqual(report.missing, ("b5",))

    def test_a_conversion_block_that_cites_outside_the_part_is_to_check_with_the_reason(self):
        made = convert_by_rules(self.extraction, self.lamp)
        widened = Conversion(
            made.converter,
            made.extraction,
            made.blocks + (MarkdownBlock("m9", "The fan runs above 40 degrees.", ("b7",)),),
        )

        report = check_conversion(self.extraction, widened, self.lamp)

        self.assertEqual(report.ids(CHECK), ["m9"])
        reason = next(v.why for v in report.verdicts if v.id == "m9")
        self.assertIn("b7", reason)
        self.assertIn("outside the part", reason)
        self.assertFalse(report.complete)
        # The nearest that must pass: the same words, cited by a block that is inside.
        wider = scope_of(self.extraction, ["s4"], tree_ids=["s2"])
        self.assertEqual(check_conversion(self.extraction, widened, wider).ids(CHECK), [])

    def test_a_scope_of_everything_is_the_same_input_as_no_scope(self):
        everything = whole(self.extraction)
        made = convert_by_rules(self.extraction, everything)

        self.assertEqual(made, convert_by_rules(self.extraction))
        report = check_conversion(self.extraction, made, everything)
        self.assertEqual((report.scope, report.out_of_scope, report.complete), ("", 0, True))

    def test_a_scope_of_another_extraction_is_refused_and_no_report_is_made(self):
        other = extraction_of(*self.extraction.blocks[:-1])
        made = convert_by_rules(other)

        with self.assertRaises(IngestError):
            check_conversion(other, made, self.lamp)
        with self.assertRaises(IngestError):
            convert_by_rules(other, self.lamp)
        with self.assertRaises(IngestError):
            split_units(other, scope=self.lamp)

    def test_the_report_says_the_scope_in_its_form(self):
        said = check_conversion(self.extraction, convert_by_rules(self.extraction, self.lamp), self.lamp).to_dict()

        self.assertEqual((said["scope"], said["out_of_scope"]), (self.lamp.revision(), 3))

    def test_a_scope_that_holds_no_words_has_nothing_to_convert_by_rule(self):
        empty = extraction_of(block("b1", text="Words."), block("b2", "table", "", cells=(("",),)))
        scope = Scope(empty.revision(), ("b2",))

        with self.assertRaises(IngestError):
            convert_by_rules(empty, scope)


class TheCutUnderAScope(unittest.TestCase):
    def test_only_the_chosen_blocks_are_given_to_a_model(self):
        extraction = two_functions()
        fan = scope_of(extraction, ["s4"])

        units = split_units(extraction, scope=fan)

        self.assertEqual([u.blocks for u in units], [("b6", "b7")])

    def test_without_a_scope_every_block_with_words_is_given(self):
        self.assertEqual([u.blocks for u in split_units(two_functions())], [tuple(f"b{i}" for i in range(1, 8))])


class TheCommandLine(unittest.TestCase):
    def run_main(self, *argv: str) -> tuple[int, str]:
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            code = main(list(argv))
        return code, out.getvalue()

    def test_outline_make_scope_prepare_convert_and_check_hold_to_one_part(self):
        with tempfile.TemporaryDirectory() as folder:
            work = pathlib.Path(folder)
            e, s, c, units = (work / "e.json", work / "s.json", work / "c.json", work / "units")
            e.write_text(two_functions().to_json(), encoding="utf-8")

            code, said = self.run_main("outline", "--extraction", str(e), "--json")
            listed = json.loads(said)
            self.assertEqual(code, 0)
            self.assertEqual([x["id"] for x in listed["sections"]], ["s1", "s2", "s3", "s4"])
            self.assertEqual(listed["by"], "headings")

            code, said = self.run_main(
                "make-scope", "--extraction", str(e), "--section", "s4", "--out", str(s), "--json"
            )
            self.assertEqual(code, 0)
            self.assertEqual(json.loads(s.read_text())["blocks"], ["b6", "b7"])
            self.assertEqual(
                json.loads(said), {"blocks": 2, "of": 7, "whole": False, "revision": Scope.from_json(s.read_text()).revision()}
            )
            # The nearest that must say otherwise: every section is every block, which is no scope.
            code, said = self.run_main(
                "make-scope", "--extraction", str(e), "--out", str(work / "all.json"), "--json",
                "--section", "s1", "--tree", "s2", "--section", "s4",
            )
            self.assertEqual(code, 0)
            self.assertEqual((json.loads(said)["whole"], json.loads(said)["blocks"]), (True, 7))
            # And the same without the part under Lamp is not every block: it was left out and stays out.
            code, said = self.run_main(
                "make-scope", "--extraction", str(e), "--out", str(work / "less.json"), "--json",
                "--section", "s1", "--section", "s2", "--section", "s4",
            )
            self.assertEqual((json.loads(said)["whole"], json.loads(said)["blocks"]), (False, 5))
            # Nothing named at all is refused.
            code, _ = self.run_main("make-scope", "--extraction", str(e), "--out", str(work / "none.json"))
            self.assertEqual(code, 2)

            code, said = self.run_main(
                "prepare-conversion", "--extraction", str(e), "--scope", str(s), "--out-dir", str(units)
            )
            self.assertEqual(code, 0)
            listing = json.loads((units / "units.json").read_text())
            self.assertEqual([u["blocks"] for u in listing["units"]], [["b6", "b7"]])
            self.assertEqual(listing["scope"], Scope.from_json(s.read_text()).revision())
            prompt = (units / "unit-0001.prompt.txt").read_text()
            self.assertIn("fan runs", prompt)
            self.assertNotIn("lamp is on", prompt)
            self.assertNotIn("confidential", prompt)

            code, said = self.run_main("convert", "--extraction", str(e), "--scope", str(s), "--out", str(c))
            self.assertEqual(code, 0, said)
            self.assertIn("outside the chosen part", said)

            code, said = self.run_main(
                "check-conversion", "--extraction", str(e), "--conversion", str(c), "--scope", str(s), "--json"
            )
            self.assertEqual(code, 0)
            self.assertEqual(json.loads(said)["out_of_scope"], 5)
            # The nearest that must fail: the same conversion held to the whole document.
            code, said = self.run_main(
                "check-conversion", "--extraction", str(e), "--conversion", str(c), "--json"
            )
            self.assertEqual(code, 1)
            self.assertEqual(json.loads(said)["missing"], ["b1", "b2", "b3", "b4", "b5"])


if __name__ == "__main__":
    unittest.main()
