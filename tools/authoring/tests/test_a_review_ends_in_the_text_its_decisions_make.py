"""The text a review ends in: a conversion with the owner's decisions applied (ADR 0037).

The conversion is not changed by a review, so the text is made once from the conversion, the extraction
and the log, and the same three always make the same text. Each case applies one kind of decision and is
paired with the nearest input that must be refused, since an assembly that refuses every decision passes
the first half and one that accepts every decision passes the second.
"""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import pathlib
import tempfile
import unittest

from sce_author.__main__ import main
from sce_author.assembly import Decision, assemble, changes, decisions_from_json, standing
from sce_author.conversion import Conversion, Converter, MarkdownBlock
from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, Place


def block(identifier: str, kind: str = "paragraph", text: str = "", **kwargs) -> Block:
    kwargs.setdefault("place", Place("spec.md", ordinal=int(identifier[1:])))
    return Block(identifier, kind, text, **kwargs)


def original() -> Extraction:
    return Extraction(
        "plain",
        "1",
        "spec.md",
        (
            block("b1", "heading", "Lamp controller", level=1),
            block("b2", text="The lamp is on while the supply is present."),
            block("b3", text="It turns off when the supply is lost."),
            block("b4", text="The delay is five seconds."),
        ),
    )


def conversion(extraction: Extraction, *blocks: MarkdownBlock) -> Conversion:
    return Conversion(Converter("model", "a-model", "1"), extraction.revision(), tuple(blocks))


# The model carried the heading, the second paragraph and the fourth, and left the third out.
LEFT_OUT_THIRD = (
    MarkdownBlock("m1", "# Lamp controller", ("b1",)),
    MarkdownBlock("m2", "The lamp is on while the supply is present.", ("b2",)),
    MarkdownBlock("m3", "The delay is five seconds.", ("b4",)),
)


def decision(n: int, block_id: str, what: str, text: str | None = None) -> Decision:
    return Decision(n, block_id, what, text)


def assembled(*log: Decision, blocks=LEFT_OUT_THIRD):
    extraction = original()
    return assemble(conversion(extraction, *blocks), extraction, list(log))


class ABlockNobodyDecidedAboutIsWrittenAsTheConversionHasIt(unittest.TestCase):
    def test_no_decision_gives_the_conversions_own_markdown(self):
        made = assembled()

        self.assertEqual(
            made.text,
            "# Lamp controller\n\nThe lamp is on while the supply is present.\n\nThe delay is five seconds.\n",
        )
        self.assertEqual((made.kept, made.corrected, made.dropped, made.added), (("m1", "m2", "m3"), (), (), ()))
        self.assertEqual(made.decisions, 0)

    def test_a_keep_changes_nothing(self):
        self.assertEqual(assembled(decision(1, "m2", "keep")).text, assembled().text)

    def test_the_same_inputs_make_the_same_text(self):
        log = [decision(1, "m2", "correct", "The lamp lights while the supply is present.")]

        self.assertEqual(assembled(*log), assembled(*log))


class ACorrectionStandsWhereTheBlockWas(unittest.TestCase):
    def test_the_owners_words_replace_the_block_in_its_place(self):
        made = assembled(decision(1, "m2", "correct", "  The lamp lights while the supply is present.\n"))

        self.assertEqual(
            made.text,
            "# Lamp controller\n\nThe lamp lights while the supply is present.\n\nThe delay is five seconds.\n",
        )
        self.assertEqual(made.corrected, ("m2",))
        self.assertEqual(made.kept, ("m1", "m3"))

    def test_a_correction_with_no_words_is_not_a_decision(self):
        for words in (None, "", "   "):
            with self.assertRaisesRegex(IngestError, "no words"):
                Decision.from_dict({"n": 1, "block": "m2", "what": "correct", "text": words})

    def test_words_on_any_other_kind_are_refused(self):
        for what in ("keep", "drop", "add"):
            with self.assertRaisesRegex(IngestError, "only a correction"):
                Decision.from_dict({"n": 1, "block": "m2", "what": what, "text": "words"})


class ADropLeavesTheBlockOut(unittest.TestCase):
    def test_the_block_is_not_written_and_the_rest_keep_their_order(self):
        made = assembled(decision(1, "m2", "drop"))

        self.assertEqual(made.text, "# Lamp controller\n\nThe delay is five seconds.\n")
        self.assertEqual(made.dropped, ("m2",))

    def test_dropping_every_block_is_refused_and_not_written_as_an_empty_spec(self):
        everything = [decision(i + 1, f"m{i + 1}", "drop") for i in range(3)]

        with self.assertRaisesRegex(IngestError, "nothing"):
            assembled(*everything)
        assembled(*everything[:2])


class AnAddPutsAnOriginalBlockBackWhereItBelongs(unittest.TestCase):
    def test_it_follows_the_block_that_carries_the_nearest_earlier_block_of_the_original(self):
        made = assembled(decision(1, "b3", "add"))

        self.assertEqual(
            made.text,
            "# Lamp controller\n\nThe lamp is on while the supply is present.\n\n"
            "It turns off when the supply is lost.\n\nThe delay is five seconds.\n",
        )
        self.assertEqual(made.added, ("b3",))

    def test_with_no_earlier_block_carried_it_goes_first(self):
        blocks = (MarkdownBlock("m1", "The delay is five seconds.", ("b4",)),)

        made = assembled(decision(1, "b1", "add"), blocks=blocks)

        self.assertEqual(made.text, "# Lamp controller\n\nThe delay is five seconds.\n")

    def test_two_added_blocks_keep_the_order_the_original_has_them(self):
        blocks = (
            MarkdownBlock("m1", "# Lamp controller", ("b1",)),
            MarkdownBlock("m2", "The delay is five seconds.", ("b4",)),
        )

        later_first = assembled(decision(1, "b3", "add"), decision(2, "b2", "add"), blocks=blocks)

        self.assertEqual(
            later_first.text,
            "# Lamp controller\n\nThe lamp is on while the supply is present.\n\n"
            "It turns off when the supply is lost.\n\nThe delay is five seconds.\n",
        )
        self.assertEqual(later_first.added, ("b2", "b3"))

    def test_an_added_block_stays_after_its_anchor_when_the_anchor_is_dropped(self):
        made = assembled(decision(1, "m2", "drop"), decision(2, "b3", "add"))

        self.assertEqual(
            made.text, "# Lamp controller\n\nIt turns off when the supply is lost.\n\nThe delay is five seconds.\n"
        )

    def test_a_block_the_conversion_already_carries_is_refused(self):
        with self.assertRaisesRegex(IngestError, "already carries"):
            assembled(decision(1, "b2", "add"))

    def test_a_block_the_original_does_not_have_is_refused(self):
        with self.assertRaisesRegex(IngestError, "left out"):
            assembled(decision(1, "b99", "add"))

    def test_a_block_of_the_conversion_is_not_added_but_kept_or_dropped(self):
        with self.assertRaisesRegex(IngestError, "left out"):
            assembled(decision(1, "m2", "add"))

    def test_a_block_with_no_words_has_nothing_to_add(self):
        extraction = Extraction(
            "plain", "1", "spec.md", (block("b1", text="Words."), block("b2", "table", "|  |", cells=(("",),)))
        )
        made = conversion(extraction, MarkdownBlock("m1", "Words.", ("b1",)))

        with self.assertRaisesRegex(IngestError, "no words"):
            assemble(made, extraction, [decision(1, "b2", "add")])


class TheLatestDecisionOnABlockStands(unittest.TestCase):
    def test_a_later_word_replaces_an_earlier_one_and_the_earlier_is_still_counted(self):
        made = assembled(decision(1, "m2", "drop"), decision(2, "m2", "keep"))

        self.assertEqual(made.text, assembled().text)
        self.assertEqual(made.decisions, 2)

    def test_standing_keeps_the_order_the_blocks_were_first_decided(self):
        log = [decision(1, "m2", "keep"), decision(2, "m1", "drop"), decision(3, "m2", "drop")]

        self.assertEqual([(d.block, d.what) for d in standing(log)], [("m2", "drop"), ("m1", "drop")])

    def test_a_decision_on_a_block_nobody_has_is_refused(self):
        with self.assertRaisesRegex(IngestError, "no block of the conversion"):
            assembled(decision(1, "m99", "keep"))

    def test_leaving_out_a_block_the_conversion_left_out_writes_nothing_and_is_said(self):
        made = assembled(decision(1, "b3", "drop"))

        self.assertEqual(made.text, assembled().text)
        self.assertEqual((made.dropped, made.added), (("b3",), ()))
        self.assertEqual(made.decisions, 1)

    def test_only_an_add_or_a_drop_says_what_becomes_of_a_block_of_the_original(self):
        for what, words in (("keep", None), ("correct", "words")):
            with self.subTest(what=what):
                with self.assertRaisesRegex(IngestError, "only an add or a drop"):
                    assembled(decision(1, "b3", what, words))

    def test_a_block_the_conversion_carries_is_decided_by_the_block_that_carries_it(self):
        with self.assertRaisesRegex(IngestError, "decide on that block"):
            assembled(decision(1, "b2", "drop"))


class ADecisionIsReadFromTheLogTheSpecsFolderGives(unittest.TestCase):
    def test_the_list_read_proposal_gives_is_read_and_its_other_fields_are_ignored(self):
        text = json.dumps(
            [{"n": 1, "at": "2026-10-19T00:00:00Z", "block": "m2", "what": "correct", "text": "Words."}]
        )

        self.assertEqual(decisions_from_json(text), [Decision(1, "m2", "correct", "Words.")])

    def test_what_is_not_a_log_is_refused(self):
        for text in ("not json", "{}", "[1]", '[{"n":0,"block":"m1","what":"keep"}]',
                     '[{"n":1,"block":"","what":"keep"}]', '[{"n":1,"block":"m1","what":"shrug"}]'):
            with self.assertRaises(IngestError, msg=text):
                decisions_from_json(text)


class AConversionOfAnotherExtractionIsNotAssembled(unittest.TestCase):
    def test_it_is_refused_and_a_conversion_of_this_one_is_not(self):
        extraction = original()
        other = Conversion(Converter("model", "a-model", "1"), "f" * 64, LEFT_OUT_THIRD)

        with self.assertRaisesRegex(IngestError, "not of this one"):
            assemble(other, extraction, [])
        assemble(conversion(extraction, *LEFT_OUT_THIRD), extraction, [])


class TheTextSaysWhatItChangesInTheSpec(unittest.TestCase):
    def test_a_spec_with_no_text_gains_every_line(self):
        said = changes(None, "One.\n\nTwo.\n")

        self.assertEqual((said["had_text"], said["unchanged"], said["added"], said["removed"]), (False, False, 3, 0))

    def test_the_same_text_changes_nothing(self):
        said = changes("One.\n", "One.\n")

        self.assertEqual((said["had_text"], said["unchanged"], said["hunks"]), (True, True, []))

    def test_a_changed_line_is_one_removed_and_one_added_with_the_lines_around_it(self):
        said = changes("a\nb\nc\nd\ne\n", "a\nb\nX\nd\ne\n")

        self.assertEqual((said["added"], said["removed"]), (1, 1))
        (hunk,) = said["hunks"]
        self.assertEqual((hunk["old_start"], hunk["new_start"]), (1, 1))
        self.assertEqual(
            [(line["op"], line["text"]) for line in hunk["lines"]],
            [("keep", "a"), ("keep", "b"), ("remove", "c"), ("add", "X"), ("keep", "d"), ("keep", "e")],
        )


class TheCommandWritesTheTextExactlyAsItIsToBeSaved(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name)
        extraction = original()
        (self.root / "e.json").write_text(extraction.to_json(), encoding="utf-8")
        (self.root / "c.json").write_text(conversion(extraction, *LEFT_OUT_THIRD).to_json(), encoding="utf-8")

    def tearDown(self):
        self.tmp.cleanup()

    def run_main(self, *argv: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = main(list(argv))
        return code, out.getvalue(), err.getvalue()

    def assemble_command(self, log: list, *more: str) -> tuple[int, str, str]:
        (self.root / "d.json").write_text(json.dumps(log), encoding="utf-8")
        return self.run_main(
            "assemble",
            "--extraction", str(self.root / "e.json"),
            "--conversion", str(self.root / "c.json"),
            "--decisions", str(self.root / "d.json"),
            "--out", str(self.root / "text.md"),
            *more,
        )

    def test_the_file_is_the_text_and_its_hash_is_the_revision_the_report_names(self):
        code, out, _ = self.assemble_command(
            [{"n": 1, "block": "b3", "what": "add"}, {"n": 2, "block": "m2", "what": "drop"}],
            "--current", str(self.root / "e.json"),
            "--json",
        )

        written = (self.root / "text.md").read_bytes()
        report = json.loads(out)
        self.assertEqual(code, 0)
        self.assertEqual(report["revision"], hashlib.sha256(written).hexdigest())
        self.assertEqual((report["decisions"], report["dropped"], report["added"]), (2, ["m2"], ["b3"]))
        self.assertEqual(report["changes"]["had_text"], True)
        self.assertEqual(
            written.decode("utf-8"),
            "# Lamp controller\n\nIt turns off when the supply is lost.\n\nThe delay is five seconds.\n",
        )

    def test_without_json_it_says_counts_and_prints_none_of_the_documents_words(self):
        code, out, _ = self.assemble_command([{"n": 1, "block": "m2", "what": "drop"}])

        self.assertEqual(code, 0)
        self.assertIn("assembled from 1 decision(s): 2 kept, 0 corrected, 1 dropped, 0 added", out)
        self.assertNotIn("lamp", out.lower().replace("lamp controller", ""))

    def test_a_decision_it_cannot_apply_is_refused_and_writes_nothing(self):
        code, _, err = self.assemble_command([{"n": 1, "block": "m99", "what": "keep"}])

        self.assertNotEqual(code, 0)
        self.assertIn("m99", err)
        self.assertFalse((self.root / "text.md").exists())

    def test_a_decisions_file_that_is_not_there_is_refused_by_name(self):
        code, _, err = self.run_main(
            "assemble",
            "--extraction", str(self.root / "e.json"),
            "--conversion", str(self.root / "c.json"),
            "--decisions", str(self.root / "nowhere.json"),
            "--out", str(self.root / "text.md"),
        )

        self.assertNotEqual(code, 0)
        self.assertIn("nowhere.json", err)


if __name__ == "__main__":
    unittest.main()
