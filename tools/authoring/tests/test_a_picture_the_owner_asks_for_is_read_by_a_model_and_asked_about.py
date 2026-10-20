"""A picture the owner asks to be read is handed to a model alone, and what it read is added and asked about.

Nothing here reaches a model; a stand-in answers. What is held:

  handed over  a figure block's pictures are taken out of the document by name and hash, one unit each; a
               block that is not a figure, is outside the chosen part, marks a picture with no name, or
               names one the document does not hold, cannot show, holds too heavy or holds as another
               picture than the extraction names, is refused by a typed reason and the rest are handed over
  the answer   is read, not trusted: each way it can be malformed is refused by a typed reason, each paired
               with the nearest answer that is taken
  its place    what was read follows the block that writes the mark, under ids of the figure's, so the ids
               the converter by rule gave stay what they were; a unit with no answer adds nothing
  the check    every block read from a picture is asked about and none is matched; one that does not cite
               the one figure that marks its picture is to check; a conversion that says a block was read
               from a picture it keeps no reading of is refused
  the form     a conversion with no reading is written as it was (same revision); one with readings goes
               through the wire and the schema
  the commands `prepare-reading` writes the picture, the prompt and the answer's schema; `finish-reading`
               adds the readings and exits 1 when a unit failed
"""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import pathlib
import tempfile
import unittest
import zipfile

import jsonschema

from sce_author.__main__ import main
from sce_author.conversion import (
    CHECK,
    READ_FROM_PICTURE,
    Conversion,
    MarkdownBlock,
    Reading,
    Seen,
    check_conversion,
    convert_by_rules,
)
from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, Place
from sce_author import reading
from sce_author.reading import (
    BAD_SHAPE,
    EMPTY,
    NO_ANSWER,
    NOT_JSON,
    Answer,
    ReadBlock,
    Refused,
    Runtime,
    parse_answer,
    prompt_for,
    units_for,
    with_readings,
)
from sce_author.scope import Scope

PNG = b"\x89PNG\r\n\x1a\n" + b"a state chart" * 30
SCHEMA = json.loads(
    (pathlib.Path(__file__).resolve().parents[1] / "schema" / "conversion.v1.schema.json").read_text(encoding="utf-8")
)
RUNTIME = Runtime("stand-in", "1", "stand-in-model")


def block(identifier: str, kind: str = "paragraph", text: str = "", **kwargs) -> Block:
    kwargs.setdefault("place", Place("spec.docx", ordinal=int(identifier[1:])))
    return Block(identifier, kind, text, **kwargs)


def document() -> Extraction:
    return Extraction(
        "docx",
        "2",
        "spec.docx",
        (
            block("b1", "heading", "Door lock", level=1),
            block("b2", text="The lock starts locked."),
            block("b3", "figure", "[picture: state.png]"),
            block("b4", text="Figure 1. The states."),
            block("b5", "table", "| a | b |", cells=(("a", "b"),)),
            block("b6", "figure", "[picture: left.png] [picture: right.png]"),
            block("b7", "figure", "[picture]"),
            block("b8", "figure", "[picture: gone.png]"),
            block("b9", "figure", "[picture: drawing.emf]"),
        ),
    )


def answer_text(*blocks: dict, unreadable: list[str] | None = None, nothing: bool = False) -> str:
    return json.dumps({"blocks": list(blocks), "unreadable": unreadable or [], "nothing_to_read": nothing})


def read(markdown: str, kind: str = "diagram", unsure: bool = False) -> dict:
    return {"kind": kind, "markdown": markdown, "unsure": unsure}


class Base(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        self.extraction = document()
        self.docx = self.package({"state.png": PNG, "left.png": PNG + b"l", "right.png": PNG + b"right",
                                  "drawing.emf": b"metafile"})

    def package(self, media: dict[str, bytes], name: str = "spec.docx") -> pathlib.Path:
        path = self.tmp / name
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml", "<w:document/>")
            for media_name, data in media.items():
                z.writestr(f"word/media/{media_name}", data)
        return path

    def units(self, *ids: str, **kwargs):
        return units_for(self.extraction, list(ids), self.docx, **kwargs)


class APictureIsHandedOverByNameAndHash(Base):
    def test_a_figure_is_handed_over_as_the_document_holds_it(self):
        handed, refused = self.units("b3")
        self.assertEqual([], refused)
        (only,) = handed
        self.assertEqual(("b3", "state.png", "image/png"), (only.unit.block, only.unit.picture.name,
                                                            only.unit.picture.media_type))
        self.assertEqual(PNG, only.data)
        self.assertEqual(hashlib.sha256(PNG).hexdigest(), only.unit.picture.sha256)

    def test_each_picture_a_figure_marks_is_a_unit_of_its_own_and_a_block_named_twice_is_read_once(self):
        handed, refused = self.units("b6", "b3", "b6")
        self.assertEqual([], refused)
        self.assertEqual([(1, "b6", "left.png"), (2, "b6", "right.png"), (3, "b3", "state.png")],
                         [(h.unit.n, h.unit.block, h.unit.picture.name) for h in handed])

    def test_what_cannot_be_handed_over_is_refused_by_a_typed_reason_and_the_rest_are_handed_over(self):
        handed, refused = self.units("b5", "b7", "b8", "b9", "b99", "b3")
        self.assertEqual(["b3"], [h.unit.block for h in handed])
        self.assertEqual(
            [("b5", reading.NOT_A_FIGURE), ("b7", reading.PICTURES_UNNAMED), ("b8", reading.PICTURE_NOT_FOUND),
             ("b9", reading.PICTURE_NOT_SHOWABLE), ("b99", reading.NO_SUCH_BLOCK)],
            [(r.block, r.code) for r in refused],
        )

    def test_a_picture_too_heavy_is_refused_alone(self):
        handed, refused = self.units("b6", limit=len(PNG + b"l"))
        self.assertEqual(["left.png"], [h.unit.picture.name for h in handed])
        self.assertEqual([("right.png", reading.PICTURE_TOO_LARGE)], [(r.picture, r.code) for r in refused])

    def test_a_document_that_cannot_be_read_is_refused_whole(self):
        broken = self.tmp / "broken.docx"
        broken.write_bytes(b"not a zip")
        with self.assertRaises(IngestError):
            units_for(self.extraction, ["b3"], broken)

    def test_a_figure_outside_the_chosen_part_is_never_handed_over(self):
        scope = Scope(self.extraction.revision(), ("b1", "b2", "b6"))
        handed, refused = self.units("b3", "b6", scope=scope)
        self.assertEqual({"b6"}, {h.unit.block for h in handed})
        self.assertEqual([("b3", reading.OUTSIDE_SCOPE)], [(r.block, r.code) for r in refused])

    def test_a_picture_that_is_not_the_one_the_extraction_names_is_refused(self):
        blocks = list(self.extraction.blocks)
        blocks[2] = block("b3", "figure", "[picture: state.png]", picture=hashlib.sha256(PNG).hexdigest())
        self.extraction = Extraction("docx", "2", "spec.docx", tuple(blocks))
        self.assertEqual(["b3"], [h.unit.block for h in self.units("b3")[0]])
        self.docx = self.package({"state.png": PNG + b"edited"}, name="other.docx")
        handed, refused = self.units("b3")
        self.assertEqual([], handed)
        self.assertEqual([reading.OTHER_PICTURE], [r.code for r in refused])

    def test_the_model_is_told_the_recipe_and_the_picture_s_name_and_none_of_the_document_s_words(self):
        (only,), _ = self.units("b3")
        prompt = prompt_for(only.unit)
        self.assertIn(f"(recipe {reading.RECIPE_NAME})", prompt)
        self.assertIn("state.png", prompt)
        for words in ("Door lock", "The lock starts locked.", "Figure 1."):
            self.assertNotIn(words, prompt)


class TheAnswerIsReadNotTrusted(unittest.TestCase):
    def taken(self, text: str | None) -> Answer | Refused:
        return parse_answer(1, "b3", text)

    def code(self, text: str | None) -> str:
        taken = self.taken(text)
        self.assertIsInstance(taken, Refused, taken)
        return taken.code

    def test_an_answer_of_the_form_is_taken(self):
        taken = self.taken(answer_text(read("- locked -> open: 1\n"), read("x", "paragraph", True),
                                       unreadable=["the label under box 3"]))
        self.assertEqual(Answer((ReadBlock("diagram", "- locked -> open: 1", False), ReadBlock("paragraph", "x", True)),
                                ("the label under box 3",), False), taken)

    def test_no_answer_and_an_answer_that_is_not_json_are_refused(self):
        self.assertEqual(NO_ANSWER, self.code(None))
        self.assertEqual(NO_ANSWER, self.code("  \n"))
        self.assertEqual(NOT_JSON, self.code("The picture shows a state chart."))

    def test_an_answer_with_a_field_the_form_does_not_have_is_refused_and_one_without_it_is_taken(self):
        good = json.loads(answer_text(read("a")))
        self.assertIsInstance(self.taken(json.dumps(good)), Answer)
        self.assertEqual(BAD_SHAPE, self.code(json.dumps({**good, "complete": True})))
        self.assertEqual(BAD_SHAPE, self.code(json.dumps({**good, "blocks": [{**read("a"), "cites": ["b3"]}]})))
        self.assertEqual(BAD_SHAPE, self.code(json.dumps([read("a")])))

    def test_a_block_of_no_kind_the_form_has_or_with_no_words_is_refused(self):
        self.assertIsInstance(self.taken(answer_text(read("a", "table"))), Answer)
        self.assertEqual(BAD_SHAPE, self.code(answer_text(read("a", "summary"))))
        self.assertEqual(BAD_SHAPE, self.code(answer_text(read("  "))))
        self.assertEqual(BAD_SHAPE, self.code(answer_text({**read("a"), "unsure": "no"})))
        self.assertEqual(BAD_SHAPE, self.code(answer_text(read("a"), unreadable=[""])))

    def test_nothing_to_read_is_an_answer_and_reading_nothing_without_saying_so_is_not(self):
        self.assertEqual(Answer((), (), True), self.taken(answer_text(nothing=True)))
        self.assertEqual(Answer((), ("all of it",), False), self.taken(answer_text(unreadable=["all of it"])))
        self.assertEqual(EMPTY, self.code(answer_text()))
        self.assertEqual(BAD_SHAPE, self.code(answer_text(read("a"), nothing=True)))


class WhatWasReadFollowsTheMarkAndIsAskedAbout(Base):
    def setUp(self):
        super().setUp()
        self.by_rule = convert_by_rules(self.extraction)
        handed, _ = self.units("b3", "b6")
        self.units_ = [h.unit for h in handed]

    def answers(self) -> dict[int, Answer]:
        """Unit 1 is b3's state.png, 2 and 3 are b6's left.png and right.png; right.png is not answered."""
        return {
            1: Answer((ReadBlock("diagram", "- locked -> open", False), ReadBlock("list", "- R1", True)),
                      ("box 4",), False),
            2: Answer((ReadBlock("paragraph", "left words", False),), (), False),
        }

    def test_the_reading_follows_the_block_that_writes_the_mark_and_the_rule_s_ids_stay(self):
        made = with_readings(self.by_rule, self.units_, self.answers(), RUNTIME)
        ids = [b.id for b in made.blocks]
        self.assertEqual([b.id for b in self.by_rule.blocks], [i for i in ids if "-r" not in i])
        mark_of = {b.cites[0]: b.id for b in self.by_rule.blocks if b.cites}
        at = ids.index(mark_of["b3"])
        self.assertEqual(["b3-r1", "b3-r2"], ids[at + 1:at + 3])
        at = ids.index(mark_of["b6"])
        self.assertEqual(["b6-r1"], ids[at + 1:at + 2])
        self.assertEqual("[picture: state.png]", made.blocks[ids.index(mark_of["b3"])].markdown)
        second = made.blocks[ids.index("b3-r2")]
        self.assertEqual(("b3",), second.cites)
        self.assertEqual("state.png", second.picture)
        self.assertTrue(second.unsure)

    def test_each_picture_read_is_recorded_with_what_was_handed_over_and_who_read_it(self):
        made = with_readings(self.by_rule, self.units_, self.answers(), RUNTIME)
        self.assertEqual(
            [("b3", "state.png", ("box 4",), False), ("b6", "left.png", (), False)],
            [(r.block, r.picture.name, r.unreadable, r.nothing_to_read) for r in made.readings],
        )
        state = made.readings[0]
        self.assertEqual((hashlib.sha256(PNG).hexdigest(), reading.RECIPE_NAME, "stand-in", "1", "stand-in-model"),
                         (state.picture.sha256, state.recipe, state.runtime, state.runtime_version, state.model))

    def test_a_unit_with_no_answer_adds_nothing_and_its_mark_stays(self):
        made = with_readings(self.by_rule, self.units_, {}, RUNTIME)
        self.assertEqual(self.by_rule, made)
        self.assertEqual(self.by_rule.revision(), made.revision())

    def test_a_reading_of_a_figure_the_conversion_does_not_write_has_no_place(self):
        without = Conversion(self.by_rule.converter, self.by_rule.extraction,
                             tuple(b for b in self.by_rule.blocks if "b3" not in b.cites))
        with self.assertRaises(IngestError):
            with_readings(without, self.units_, self.answers(), RUNTIME)

    def test_an_answer_to_a_unit_that_was_not_prepared_is_refused(self):
        with self.assertRaises(IngestError):
            with_readings(self.by_rule, self.units_, {9: Answer((), (), True)}, RUNTIME)

    def test_every_block_read_from_a_picture_is_asked_about_and_none_is_matched(self):
        made = with_readings(self.by_rule, self.units_, self.answers(), RUNTIME)
        report = check_conversion(self.extraction, made)
        self.assertEqual(["b3-r1", "b3-r2", "b6-r1"], report.ids(READ_FROM_PICTURE))
        self.assertEqual([], report.ids(CHECK))
        self.assertEqual(report.matched, check_conversion(self.extraction, self.by_rule).matched)
        self.assertEqual([(READ_FROM_PICTURE, i) for i in ("b3-r1", "b3-r2", "b6-r1")], report.queue())
        self.assertEqual(["b3-r1", "b3-r2", "b6-r1"], report.to_dict()["read_from_picture"])

    def test_a_block_read_from_a_picture_that_cites_anything_but_its_figure_is_to_check(self):
        made = with_readings(self.by_rule, self.units_, self.answers(), RUNTIME)
        moved = tuple(
            MarkdownBlock(b.id, b.markdown, ("b2",), b.picture, b.unsure) if b.id == "b3-r1"
            else MarkdownBlock(b.id, b.markdown, ("b3", "b4"), b.picture, b.unsure) if b.id == "b3-r2"
            # A figure, and not the one that marks left.png.
            else MarkdownBlock(b.id, b.markdown, ("b3",), b.picture, b.unsure) if b.id == "b6-r1"
            else b
            for b in made.blocks
        )
        report = check_conversion(self.extraction, Conversion(made.converter, made.extraction, moved, made.readings))
        self.assertEqual([], report.ids(READ_FROM_PICTURE))
        self.assertEqual({"b3-r1", "b3-r2", "b6-r1"}, set(report.ids(CHECK)))

    def test_a_block_that_says_it_was_read_from_a_picture_the_conversion_keeps_no_reading_of_is_refused(self):
        made = with_readings(self.by_rule, self.units_, self.answers(), RUNTIME)
        with self.assertRaises(IngestError):
            Conversion(made.converter, made.extraction, made.blocks, made.readings[:1])


class TheFormCarriesTheReadings(Base):
    def made(self) -> Conversion:
        by_rule = convert_by_rules(self.extraction)
        handed, _ = self.units("b3")
        return with_readings(by_rule, [h.unit for h in handed],
                             {1: Answer((ReadBlock("diagram", "- a -> b", True),), ("box 2",), False)}, RUNTIME)

    def test_a_conversion_with_no_reading_is_written_as_it_was(self):
        by_rule = convert_by_rules(self.extraction)
        wire = by_rule.to_dict()
        self.assertNotIn("readings", wire)
        self.assertTrue(all(set(b) == {"id", "markdown", "cites"} for b in wire["blocks"]))

    def test_readings_go_through_the_wire_and_the_schema(self):
        made = self.made()
        self.assertEqual(made, Conversion.from_json(made.to_json()))
        jsonschema.validate(made.to_dict(), SCHEMA)

    def test_what_the_code_refuses_of_a_reading_the_schema_refuses_too(self):
        wire = self.made().to_dict()
        bad_hash = json.loads(json.dumps(wire))
        bad_hash["readings"][0]["picture"]["sha256"] = "x"
        unknown = json.loads(json.dumps(wire))
        unknown["readings"][0]["complete"] = True
        unsure_of_nothing = json.loads(json.dumps(wire))
        unsure_of_nothing["blocks"][0]["unsure"] = True
        for broken in (bad_hash, unknown):
            with self.assertRaises(IngestError):
                Conversion.from_dict(broken)
            with self.assertRaises(jsonschema.ValidationError):
                jsonschema.validate(broken, SCHEMA)
        # A block unsure of a reading it is not: the schema cannot tie the two fields, so the code holds it.
        with self.assertRaises(IngestError):
            Conversion.from_dict(unsure_of_nothing)

    def test_two_readings_of_one_picture_are_refused(self):
        made = self.made()
        with self.assertRaises(IngestError):
            Conversion(made.converter, made.extraction, made.blocks, made.readings * 2)

    def test_a_reading_with_no_hash_or_no_runtime_is_refused(self):
        with self.assertRaises(IngestError):
            Seen("state.png", "abc", "image/png")
        with self.assertRaises(IngestError):
            Reading("b3", Seen("state.png", "0" * 64, "image/png"), reading.RECIPE_NAME, "", "", "")


class TheCommands(Base):
    def run_main(self, *argv: str) -> tuple[int, str]:
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            code = main(list(argv))
        return code, out.getvalue()

    def write(self, name: str, text: str) -> str:
        path = self.tmp / name
        path.write_text(text, encoding="utf-8")
        return str(path)

    def test_a_reading_is_prepared_finished_and_checked_from_the_command_line(self):
        extraction = self.write("x.json", self.extraction.to_json())
        conversion = self.write("c.json", convert_by_rules(self.extraction).to_json())
        units = self.tmp / "units"
        code, said = self.run_main("prepare-reading", "--extraction", extraction, "--document", str(self.docx),
                                   "--block", "b3", "--block", "b6", "--block", "b5", "--out-dir", str(units),
                                   "--json")
        self.assertEqual(0, code)
        self.assertEqual({"units": 3, "refused": [{"block": "b5", "code": reading.NOT_A_FIGURE,
                                                   "why": "it is a table, not a figure that marks a picture"}]},
                         json.loads(said))
        listing = json.loads((units / "units.json").read_text(encoding="utf-8"))
        self.assertEqual(reading.RECIPE_NAME, listing["recipe"])
        self.assertEqual(PNG, (units / listing["units"][0]["file"]).read_bytes())
        self.assertIn(reading.RECIPE_NAME, (units / listing["units"][0]["prompt"]).read_text(encoding="utf-8"))
        self.assertEqual(reading.ANSWER_SCHEMA, json.loads((units / "answer.schema.json").read_text(encoding="utf-8")))

        (units / "unit-0001.answer.json").write_text(answer_text(read("- locked -> open")), encoding="utf-8")
        (units / "unit-0002.answer.json").write_text(answer_text(nothing=True), encoding="utf-8")
        (units / "unit-0003.answer.json").write_text("the right half shows a lamp", encoding="utf-8")
        out = str(self.tmp / "c2.json")
        code, said = self.run_main("finish-reading", "--extraction", extraction, "--conversion", conversion,
                                   "--dir", str(units), "--runtime", "stand-in", "--out", out, "--json")
        self.assertEqual(1, code)
        report = json.loads(said)
        self.assertEqual(([1], [2], [(3, NOT_JSON)]),
                         (report["read"], report["nothing_to_read"],
                          [(f["n"], f["code"]) for f in report["failed"]]))
        made = Conversion.from_json(pathlib.Path(out).read_text(encoding="utf-8"))
        self.assertEqual(report["revision"], made.revision())
        self.assertEqual([("b3", "state.png"), ("b6", "left.png")],
                         [(r.block, r.picture.name) for r in made.readings])

        code, said = self.run_main("check-conversion", "--extraction", extraction, "--conversion", out, "--json")
        self.assertEqual(0, code)
        self.assertEqual(["b3-r1"], json.loads(said)["read_from_picture"])
        code, said = self.run_main("check-conversion", "--extraction", extraction, "--conversion", out)
        self.assertIn("1 read from a picture", said)

    def test_a_folder_prepared_for_another_extraction_is_refused(self):
        units = self.tmp / "units"
        extraction = self.write("x.json", self.extraction.to_json())
        self.run_main("prepare-reading", "--extraction", extraction, "--document", str(self.docx),
                      "--block", "b3", "--out-dir", str(units))
        other = Extraction("docx", "2", "spec.docx", self.extraction.blocks[:4])
        code, _ = self.run_main("finish-reading", "--extraction", self.write("o.json", other.to_json()),
                                "--conversion", self.write("oc.json", convert_by_rules(other).to_json()),
                                "--dir", str(units), "--runtime", "stand-in", "--out", str(self.tmp / "o2.json"))
        self.assertNotEqual(0, code)
        self.assertFalse((self.tmp / "o2.json").exists())


if __name__ == "__main__":
    unittest.main()
