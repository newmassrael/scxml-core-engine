"""A division drawn by a model (ADR 0040): what it is asked, and how its answer is taken.

Each case that must refuse is paired with the nearest one that must not. A reading that takes whatever a model
says passes the first half and puts a part where a model guessed, or leaves a part out on its word; a reading
that refuses a whole answer for one wrong item passes the second half and makes a draft worthless the first
time a model slips. What is held is that the owner's choices are the model's only vocabulary, that it may
decline, that nothing it says decides a part is left out, and that the words of the document are in the prompt
and in no report.
"""

from __future__ import annotations

import contextlib
import io
import json
import pathlib
import tempfile
import unittest

from sce_author.__main__ import main
from sce_author.divide_by_model import (
    BAD_SHAPE,
    BAD_TITLE,
    EMPTY,
    LEFT_OUT,
    NO_ANSWER,
    NOT_JSON,
    PLACED_TWICE,
    PROMPT_CHARACTERS,
    UNKNOWN_DESTINATION,
    UNKNOWN_PART,
    Offered,
    offered_from,
    prepare,
    read_answer,
)
from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, Place


def block(number: int, kind: str, text: str, **kwargs) -> Block:
    kwargs.setdefault("place", Place("spec.md", ordinal=number))
    if kind == "heading":
        kwargs.setdefault("level", 1)
    return Block(f"b{number}", kind, text, **kwargs)


def document() -> Extraction:
    """A cover with words, two functions (the first with a part under it) and an empty heading."""
    return Extraction(
        "plain",
        "1",
        "spec.md",
        (
            block(1, "paragraph", "Cover page, confidential."),
            block(2, "heading", "Lamp"),
            block(3, "paragraph", "The lamp is on while the supply is present."),
            block(4, "heading", "Timing", level=2),
            block(5, "paragraph", "It turns off after 30 seconds."),
            block(6, "heading", "Fan"),
            block(7, "paragraph", "The fan runs above 40 degrees."),
            block(8, "heading", "Appendix"),
        ),
    )


LAMP = Offered("d0", "spec", "Lamp control")
FAN = Offered("d1", "new", "Fan control")
BOTH = (LAMP, FAN)


class WhatAModelIsTold(unittest.TestCase):
    def test_it_lists_each_part_by_id_title_size_and_first_words_and_each_destination_by_key(self):
        extraction = document()
        prepared = prepare(extraction, BOTH)

        # s1 is the cover (before the first heading), s2 the lamp, s3 timing, s4 the fan and s5 the appendix. A
        # heading is a paragraph with words, so the appendix, which is only its heading, is a part to be placed.
        self.assertEqual(("s1", "s2", "s3", "s4", "s5"), prepared.parts)
        self.assertIn("[d0] Lamp control (a specification that exists)", prepared.prompt)
        self.assertIn("[d1] Fan control (a specification to be made)", prepared.prompt)
        self.assertIn('[s3] level 2 "Timing" (2 paragraphs, 36 characters): It turns off after 30 seconds.', prepared.prompt)
        # Its heading is its title and is not said again as its words.
        self.assertIn('[s5] level 1 "Appendix" (1 paragraphs, 8 characters)\n', prepared.prompt)

    def test_it_is_told_to_decline_and_never_to_leave_a_part_out(self):
        prompt = prepare(document(), BOTH).prompt

        self.assertIn("do not answer for it", prompt)
        self.assertIn("Do not decide that a part is unwanted", prompt)
        # The answer's form has no place for leaving a part out.
        self.assertNotIn('"out"', prompt)

    def test_the_first_words_are_short_and_never_the_whole_text(self):
        long_words = "word " * 200
        extraction = Extraction("plain", "1", "spec.md", (
            block(1, "heading", "One"), block(2, "paragraph", long_words),
            block(3, "heading", "Two"), block(4, "paragraph", "Short."),
        ))
        prompt = prepare(extraction, (LAMP,)).prompt

        line = next(l for l in prompt.splitlines() if l.startswith("[s1]"))
        self.assertLess(len(line), 300)
        self.assertTrue(line.endswith("…"))

    def test_a_document_that_has_nothing_to_divide_or_nobody_to_divide_among_is_refused(self):
        one_part = Extraction("plain", "1", "spec.md", (block(1, "heading", "Only"), block(2, "paragraph", "Words.")))
        with self.assertRaises(IngestError):
            prepare(one_part, BOTH)
        with self.assertRaises(IngestError):
            prepare(document(), ())
        prepare(document(), BOTH)

    def test_an_outline_too_long_for_one_prompt_is_refused_in_a_typed_word_and_not_cut(self):
        blocks = []
        for number in range(1, 2000):
            blocks.append(block(number * 2 - 1, "heading", f"Part number {number} of the document"))
            blocks.append(block(number * 2, "paragraph", "Some words to say what this part is about, at length."))
        big = Extraction("plain", "1", "spec.md", tuple(blocks))

        with self.assertRaises(IngestError) as raised:
            prepare(big, BOTH)
        self.assertIn("too-many-parts", str(raised.exception))
        self.assertIn(str(PROMPT_CHARACTERS), str(raised.exception))
        small = Extraction("plain", "1", "spec.md", tuple(blocks[:200]))
        prepare(small, BOTH)


class TheDestinationsTheOwnerHas(unittest.TestCase):
    def good(self, *items):
        return {"v": 1, "destinations": [dict(key=k, kind=kind, title=t) for k, kind, t in items]}

    def test_a_key_is_one_the_model_can_answer_by_and_is_not_repeated(self):
        offered_from(self.good(("d0", "spec", "Lamp")))
        for bad in (
            self.good(("a b", "spec", "Lamp")),
            self.good(("", "spec", "Lamp")),
            self.good(("d0", "spec", "Lamp"), ("d0", "new", "Fan")),
            self.good(("d0", "elsewhere", "Lamp")),
            self.good(("d0", "spec", "   ")),
            {"v": 2, "destinations": []},
            {"v": 1, "destinations": []},
            {"v": 1, "destinations": [{"key": "d0", "kind": "spec"}]},
        ):
            with self.assertRaises(IngestError, msg=str(bad)):
                offered_from(bad)


class WhatAModelSaysIsReadItemByItem(unittest.TestCase):
    def setUp(self):
        self.extraction = document()
        self.prepared = prepare(self.extraction, BOTH)

    def read(self, text):
        return read_answer(self.extraction, self.prepared, BOTH, text)

    def test_a_part_goes_to_a_destination_offered_or_to_a_new_one_and_the_rest_are_unplaced(self):
        draft = self.read(
            'Here is the division:\n```json\n[{"part": "s2", "spec": "d0"}, {"part": "s4", "new": "Fan control"}]\n```'
        )

        self.assertIsNone(draft.unreadable)
        # A new title that is a destination's own is that destination.
        self.assertEqual([("s2", "d0", None), ("s4", "d1", None)], [(s.part, s.key, s.title) for s in draft.suggestions])
        self.assertEqual(("s1", "s3", "s5"), draft.unplaced)
        self.assertEqual((), draft.refused)

    def test_a_new_title_that_nobody_has_is_a_new_destination_and_one_title_in_two_cases_is_one(self):
        draft = self.read('[{"part": "s2", "new": "Door lock"}, {"part": "s3", "new": "DOOR LOCK"}, {"part": "s4", "new": "Fan"}]')

        self.assertEqual(
            [("s2", None, "Door lock"), ("s3", None, "Door lock"), ("s4", None, "Fan")],
            [(s.part, s.key, s.title) for s in draft.suggestions],
        )

    def test_a_model_that_declines_is_answered_and_not_refused(self):
        draft = self.read('[{"part": "s2", "spec": "d0"}]')

        self.assertEqual(("s1", "s3", "s4", "s5"), draft.unplaced)
        self.assertIsNone(draft.unreadable)

    def test_an_item_that_is_wrong_is_named_and_the_others_are_kept(self):
        draft = self.read(json.dumps([
            {"part": "s2", "spec": "d0"},
            {"part": "s99", "spec": "d0"},
            {"part": "s3", "spec": "d9"},
            {"part": "s2", "spec": "d1"},
            {"part": "s4", "new": ""},
            "not an object",
            {"part": "s1"},
            {"part": "s1", "spec": "d0", "new": "Both"},
            {"part": "s4", "spec": "d1"},
        ]))

        self.assertEqual([("s2", "d0"), ("s4", "d1")], [(s.part, s.key) for s in draft.suggestions])
        self.assertEqual(
            [(2, UNKNOWN_PART), (3, UNKNOWN_DESTINATION), (4, PLACED_TWICE), (5, BAD_TITLE), (6, BAD_SHAPE),
             (7, BAD_SHAPE), (8, BAD_SHAPE)],
            [(r.item, r.code) for r in draft.refused],
        )
        self.assertEqual(("s1", "s3", "s5"), draft.unplaced)

    def test_leaving_a_part_out_is_refused_by_whatever_name_a_model_gives_it(self):
        for item in (
            {"part": "s1", "spec": "out"},
            {"part": "s1", "spec": "left-out"},
            {"part": "s1", "new": "Left out"},
            {"part": "s1", "new": "  skip "},
            {"part": "s1", "out": True},
            {"part": "s1", "leave": True},
        ):
            draft = self.read(json.dumps([{"part": "s2", "spec": "d0"}, item]))
            self.assertEqual([LEFT_OUT], [r.code for r in draft.refused], msg=str(item))
            self.assertIn("s1", draft.unplaced, msg=str(item))
            self.assertEqual(["s2"], [s.part for s in draft.suggestions], msg=str(item))
        # The nearest that is a destination: a title that merely contains the word is a title.
        draft = self.read('[{"part": "s1", "new": "Skip trace"}]')
        self.assertEqual("Skip trace", draft.suggestions[0].title)

    def test_an_answer_that_cannot_be_read_at_all_is_unreadable_and_places_nothing(self):
        for text, code in ((None, NO_ANSWER), ("   ", NO_ANSWER), ("I would put the lamp first.", NOT_JSON), ("[]", EMPTY)):
            draft = self.read(text)
            self.assertEqual(code, draft.unreadable.code, msg=repr(text))
            self.assertEqual((), draft.suggestions)
            self.assertEqual(self.prepared.parts, draft.unplaced)
        self.assertIsNone(self.read('[{"part": "s2", "spec": "d0"}]').unreadable)

    def test_the_array_is_found_among_words_and_a_fence_and_the_first_one_wins(self):
        draft = self.read('Sure. [see note] ```\n[{"part": "s2", "spec": "d0"}]\n``` and [{"part": "s3", "spec": "d1"}]')

        self.assertEqual(["s2"], [s.part for s in draft.suggestions])


class TheCommandLine(unittest.TestCase):
    def run_main(self, *argv: str) -> tuple[int, str]:
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            code = main(list(argv))
        return code, out.getvalue()

    def test_a_prompt_is_written_an_answer_is_read_and_the_reports_say_ids_and_no_words(self):
        with tempfile.TemporaryDirectory() as folder:
            work = pathlib.Path(folder)
            e, d, out = work / "e.json", work / "d.json", work / "div"
            e.write_text(document().to_json(), encoding="utf-8")
            d.write_text(json.dumps({"v": 1, "destinations": [
                {"key": "d0", "kind": "spec", "title": "Lamp control"},
                {"key": "d1", "kind": "new", "title": "Fan control"},
            ]}), encoding="utf-8")

            code, said = self.run_main("prepare-division", "--extraction", str(e), "--destinations", str(d), "--out-dir", str(out))
            self.assertEqual(0, code)
            self.assertEqual(5, json.loads(said)["parts"])
            self.assertNotIn("lamp is on", said)
            self.assertIn("lamp is on", (out / "prompt.txt").read_text(encoding="utf-8"))

            # No answer yet: unreadable, exit 1.
            code, said = self.run_main("finish-division", "--extraction", str(e), "--dir", str(out))
            self.assertEqual(1, code)
            self.assertEqual("no-answer", json.loads(said)["unreadable"]["code"])

            (out / "answer.txt").write_text('[{"part": "s2", "spec": "d0"}, {"part": "s4", "new": "Fan"}]', encoding="utf-8")
            code, said = self.run_main("finish-division", "--extraction", str(e), "--dir", str(out))
            report = json.loads(said)
            self.assertEqual(0, code)
            self.assertEqual(document().revision(), report["extraction"])
            self.assertEqual([{"part": "s2", "key": "d0", "title": None}, {"part": "s4", "key": None, "title": "Fan"}],
                             report["suggestions"])
            self.assertEqual(["s1", "s3", "s5"], report["unplaced"])
            self.assertNotIn("lamp", said.lower().replace("lamp control", ""))
            self.assertEqual(report, json.loads((out / "draft.json").read_text(encoding="utf-8")))

    def test_a_division_prepared_from_another_extraction_is_not_read_against_this_one(self):
        with tempfile.TemporaryDirectory() as folder:
            work = pathlib.Path(folder)
            e, other, d, out = work / "e.json", work / "o.json", work / "d.json", work / "div"
            e.write_text(document().to_json(), encoding="utf-8")
            other.write_text(Extraction("plain", "1", "spec.md", document().blocks[:-2]).to_json(), encoding="utf-8")
            d.write_text(json.dumps({"v": 1, "destinations": [{"key": "d0", "kind": "spec", "title": "Lamp"}]}), encoding="utf-8")
            self.run_main("prepare-division", "--extraction", str(e), "--destinations", str(d), "--out-dir", str(out))
            (out / "answer.txt").write_text('[{"part": "s2", "spec": "d0"}]', encoding="utf-8")

            code, _ = self.run_main("finish-division", "--extraction", str(other), "--dir", str(out))
            self.assertEqual(2, code)
            code, _ = self.run_main("finish-division", "--extraction", str(e), "--dir", str(out))
            self.assertEqual(0, code)


if __name__ == "__main__":
    unittest.main()
