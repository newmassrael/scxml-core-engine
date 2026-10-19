"""A conversion made by a model: cut into units, told what to do, and held to the same check as any other.

Nothing here reaches a model. A stand-in plays one, and each way a real one goes wrong (it summarises, it
changes a number, it leaves a block out, it answers in prose, it answers nothing) is put to the pieces that
read the answer and to the check, each paired with the nearest answer that must pass: a reader of answers
that refuses everything passes the first half, and one that accepts everything passes the second.
"""

from __future__ import annotations

import contextlib
import io
import json
import pathlib
import tempfile
import unittest

from sce_author.__main__ import main
from sce_author.conversion import CHECK, Converter, check_conversion, convert_by_rules
from sce_author.conversion_by_model import (
    BAD_SHAPE,
    EMPTY,
    NO_ANSWER,
    NOT_JSON,
    Unit,
    UnitRefused,
    assemble_units,
    parse_answer,
    prompt_for,
    split_units,
)
from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, Place


def block(identifier: str, kind: str = "paragraph", text: str = "", page: int | None = None, **kwargs) -> Block:
    kwargs.setdefault("place", Place("spec.md", ordinal=int(identifier[1:]), page=page))
    return Block(identifier, kind, text, **kwargs)


def document() -> Extraction:
    return Extraction(
        "plain",
        "1",
        "spec.md",
        (
            block("b1", "heading", "Lamp controller", level=1),
            block("b2", text="The lamp is on while the supply is present."),
            block("b3", text="It turns off after 30 seconds."),
            block("b4", "table", "| state | lamp |", cells=(("state", "lamp"), ("on", "lit"))),
            block("b5", text=""),
            block("b6", text="The delay is five seconds."),
        ),
    )


def faithful(extraction: Extraction, unit: Unit) -> str:
    """What a model that does as it is told answers for `unit`: the converter by rule's own paragraphs."""
    made = convert_by_rules(extraction)
    cited = {block_id: m for m in made.blocks for block_id in m.cites}
    return json.dumps([{"markdown": cited[i].markdown, "cites": list(cited[i].cites)} for i in unit.blocks])


class TheUnitsAModelIsGiven(unittest.TestCase):
    def test_blocks_with_words_in_order_and_none_without(self):
        units = split_units(document())

        self.assertEqual([u.blocks for u in units], [("b1", "b2", "b3", "b4", "b6")])

    def test_a_unit_is_closed_when_it_holds_as_much_as_it_should(self):
        units = split_units(document(), characters=60)

        self.assertGreater(len(units), 1)
        self.assertEqual([i for u in units for i in u.blocks], ["b1", "b2", "b3", "b4", "b6"])
        self.assertEqual([u.n for u in units], list(range(1, len(units) + 1)))

    def test_a_block_is_never_cut_and_one_longer_than_a_unit_is_a_unit_of_its_own(self):
        long = Extraction("plain", "1", "s.md", (block("b1", text="a" * 500), block("b2", text="short")))

        units = split_units(long, characters=100)

        self.assertEqual([u.blocks for u in units], [("b1",), ("b2",)])

    def test_a_page_is_closed_where_the_page_changes(self):
        paged = Extraction(
            "pdf",
            "1",
            "s.pdf",
            (block("b1", text="one", page=1), block("b2", text="two", page=1), block("b3", text="three", page=2)),
        )

        units = split_units(paged, characters=10_000)

        self.assertEqual([u.blocks for u in units], [("b1", "b2"), ("b3",)])

    def test_a_document_with_no_words_has_no_unit_and_a_unit_holds_at_least_a_character(self):
        self.assertEqual(split_units(Extraction("plain", "1", "s.md", (block("b1", text=""),))), [])
        with self.assertRaisesRegex(IngestError, "at least one character"):
            split_units(document(), characters=0)


class WhatAModelIsTold(unittest.TestCase):
    def test_the_recipe_and_every_block_of_the_unit_by_id_and_nothing_of_another_unit(self):
        extraction = document()
        first, second = split_units(extraction, characters=60)[:2]

        prompt = prompt_for(extraction, first)

        self.assertIn("recipe convert-blocks 1", prompt)
        for block_id in first.blocks:
            self.assertIn(f"[{block_id}]", prompt)
        for block_id in second.blocks:
            self.assertNotIn(f"[{block_id}]", prompt)

    def test_a_heading_says_its_level_and_a_table_is_given_as_its_rows(self):
        extraction = document()

        prompt = prompt_for(extraction, split_units(extraction)[0])

        self.assertIn("[b1] heading level 1: Lamp controller", prompt)
        self.assertIn("[b4] table:\n| state | lamp |\n| on | lit |", prompt)

    def test_a_unit_that_names_a_block_the_extraction_does_not_have_is_refused(self):
        with self.assertRaisesRegex(IngestError, "does not have"):
            prompt_for(document(), Unit(1, ("b99",)))


class WhatAModelAnswered(unittest.TestCase):
    ANSWER = [{"markdown": "# Title", "cites": ["b1"]}, {"markdown": "Words.", "cites": ["b2"]}]

    def test_a_plain_array_is_read(self):
        read = parse_answer(1, json.dumps(self.ANSWER))

        self.assertEqual(read, [("# Title", ("b1",)), ("Words.", ("b2",))])

    def test_a_fence_or_words_round_the_array_do_not_matter(self):
        text = json.dumps(self.ANSWER)
        for answer in (
            f"```json\n{text}\n```",
            f"Here is the conversion:\n{text}\nI kept every word.",
            f"<think>a [list] of [things]</think>\n{text}",
        ):
            with self.subTest(answer=answer[:20]):
                self.assertEqual(parse_answer(1, answer), [("# Title", ("b1",)), ("Words.", ("b2",))])

    def test_brackets_inside_the_words_do_not_end_the_array(self):
        answer = json.dumps([{"markdown": "See [1] and [2].", "cites": ["b1"]}])

        self.assertEqual(parse_answer(1, answer), [("See [1] and [2].", ("b1",))])

    def test_what_cannot_be_read_is_a_typed_answer_and_not_an_error(self):
        cases = {
            NO_ANSWER: [None, "", "   \n"],
            NOT_JSON: ["I could not do this.", "[not json", "{\"markdown\": \"x\"}"],
            EMPTY: ["[]"],
            BAD_SHAPE: [
                '[{"markdown": "x"}]',
                '[{"markdown": "x", "cites": "b1"}]',
                '[{"markdown": "x", "cites": ["b1"], "complete": true}]',
                '[{"markdown": "  ", "cites": ["b1"]}]',
                '[{"markdown": "x", "cites": ["b1", "b1"]}]',
                '["just words"]',
            ],
        }
        for code, answers in cases.items():
            for answer in answers:
                with self.subTest(code=code, answer=answer):
                    refused = parse_answer(3, answer)
                    self.assertIsInstance(refused, UnitRefused)
                    self.assertEqual((refused.n, refused.code), (3, code))


class PutTogetherAndHeldToTheCheck(unittest.TestCase):
    def setUp(self):
        self.extraction = document()
        self.units = split_units(self.extraction, characters=60)
        self.converter = Converter("model", "a-model", "1")

    def answers(self, edit=lambda n, text: text):
        return {u.n: parse_answer(u.n, edit(u.n, faithful(self.extraction, u))) for u in self.units}

    def test_what_a_faithful_model_wrote_is_found_complete(self):
        made, failed = assemble_units(self.extraction, self.answers(), self.converter)

        self.assertEqual(failed, [])
        self.assertEqual(made.converter, self.converter)
        self.assertEqual([m.id for m in made.blocks], [f"m{i:04d}" for i in range(1, len(made.blocks) + 1)])
        self.assertTrue(check_conversion(self.extraction, made).complete)

    def test_a_model_that_changed_a_number_is_found_by_the_same_check_as_any_conversion(self):
        def edit(n, text):
            return text.replace("30 seconds", "80 seconds")

        made, failed = assemble_units(self.extraction, self.answers(edit), self.converter)

        report = check_conversion(self.extraction, made)
        self.assertEqual(failed, [])
        self.assertFalse(report.complete)
        self.assertEqual(len(report.ids(CHECK)), 1)

    def test_a_model_that_left_a_block_out_leaves_it_missing(self):
        def edit(n, text):
            items = json.loads(text)
            return json.dumps([i for i in items if i["cites"] != ["b3"]])

        made, _ = assemble_units(self.extraction, self.answers(edit), self.converter)

        self.assertEqual(check_conversion(self.extraction, made).missing, ("b3",))

    def test_a_unit_that_failed_costs_that_unit_and_keeps_the_others(self):
        answers = self.answers()
        gone = self.units[0]
        answers[gone.n] = parse_answer(gone.n, "I cannot do this one.")

        made, failed = assemble_units(self.extraction, answers, self.converter)

        self.assertEqual([(f.n, f.code) for f in failed], [(gone.n, NOT_JSON)])
        report = check_conversion(self.extraction, made)
        self.assertEqual(set(report.missing), set(gone.blocks))
        self.assertGreater(len(made.blocks), 0)

    def test_when_no_unit_was_answered_there_is_nothing_to_convert(self):
        answers = {u.n: parse_answer(u.n, "no") for u in self.units}

        with self.assertRaisesRegex(IngestError, "nothing to convert"):
            assemble_units(self.extraction, answers, self.converter)

    def test_the_same_answers_are_the_same_conversion(self):
        first, _ = assemble_units(self.extraction, self.answers(), self.converter)
        second, _ = assemble_units(self.extraction, self.answers(), self.converter)

        self.assertEqual(first.revision(), second.revision())


def run(*argv: str) -> tuple[int, str, str]:
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = main(list(argv))
    return code, out.getvalue(), err.getvalue()


class TheTwoCommands(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name)
        self.extraction = document()
        (self.root / "e.json").write_text(self.extraction.to_json(), encoding="utf-8")
        self.work = self.root / "work"

    def tearDown(self):
        self.tmp.cleanup()

    def prepare(self, *more: str):
        return run("prepare-conversion", "--extraction", str(self.root / "e.json"), "--out-dir", str(self.work), *more)

    def finish(self, *more: str):
        return run(
            "finish-conversion", "--extraction", str(self.root / "e.json"), "--dir", str(self.work),
            "--converter-name", "a-model", "--out", str(self.root / "c.json"), *more,
        )

    def answer_all(self, edit=lambda n, text: text):
        listing = json.loads((self.work / "units.json").read_text(encoding="utf-8"))
        for unit in listing["units"]:
            text = faithful(self.extraction, Unit(unit["n"], tuple(unit["blocks"])))
            (self.work / f"unit-{unit['n']:04d}.answer.txt").write_text(edit(unit["n"], text), encoding="utf-8")

    def test_prepare_writes_a_prompt_for_each_unit_and_says_counts_only(self):
        code, out, _ = self.prepare("--characters", "60")

        prompts = sorted(p.name for p in self.work.glob("unit-*.prompt.txt"))
        self.assertEqual(code, 0)
        self.assertEqual(prompts[0], "unit-0001.prompt.txt")
        self.assertIn(f"units: {len(prompts)} unit(s) of 5 block(s)", out)
        self.assertNotIn("lamp is on", out.lower())

    def test_a_faithful_model_goes_through_to_a_complete_check(self):
        self.prepare("--characters", "60")
        self.answer_all()

        code, out, _ = self.finish("--json")
        checked = run("check-conversion", "--extraction", str(self.root / "e.json"),
                      "--conversion", str(self.root / "c.json"), "--json")

        report = json.loads(out)
        self.assertEqual(code, 0)
        self.assertEqual(report["failed"], [])
        self.assertEqual(len(report["converted"]), report["units"])
        self.assertEqual(checked[0], 0, checked[1])
        self.assertEqual(json.loads(checked[1])["complete"], True)

    def test_a_unit_with_no_answer_or_an_unreadable_one_is_named_and_the_exit_says_so(self):
        self.prepare("--characters", "60")
        self.answer_all(lambda n, text: "Sorry, I can't." if n == 1 else text)
        (self.work / "unit-0002.answer.txt").unlink()

        code, out, _ = self.finish("--json")

        report = json.loads(out)
        self.assertEqual(code, 1)
        self.assertEqual(sorted((f["n"], f["code"]) for f in report["failed"]), [(1, NOT_JSON), (2, NO_ANSWER)])
        self.assertTrue((self.root / "c.json").exists())

    def test_answers_about_another_extraction_are_not_put_together(self):
        self.prepare()
        self.answer_all()
        other = Extraction("plain", "1", "o.md", (block("b1", text="Something else."),))
        (self.root / "e.json").write_text(other.to_json(), encoding="utf-8")

        code, _, err = self.finish()

        self.assertNotEqual(code, 0)
        self.assertIn("not for this one", err)
        self.assertFalse((self.root / "c.json").exists())

    def test_a_folder_that_prepare_did_not_write_is_refused_by_name(self):
        code, _, err = self.finish()

        self.assertNotEqual(code, 0)
        self.assertIn("units.json", err)

    def test_an_extraction_with_no_words_is_not_cut_into_units(self):
        (self.root / "e.json").write_text(Extraction("plain", "1", "s.md", ()).to_json(), encoding="utf-8")

        code, _, err = self.prepare()

        self.assertNotEqual(code, 0)
        self.assertIn("nothing to ask a model", err)


if __name__ == "__main__":
    unittest.main()
