"""A name the specification writes is a WHOLE identifier, not a run of letters inside a longer one.

Three readers asked "does the prose name this?" as a substring, and each was wrong in its own
direction. A short name that begins a longer one the specification does use -- `IN_Obstacle` in
`IN_ObstacleExtended` -- is not named by that use:

  check      refused a correct document: it asked for an input the specification never used
  brief      listed an address as touched that the specification never touched
  questions  called an output mentioned and so did not ask about it

Asserted here, from the shared question outward:

  the question   `Prose.writes` is a whole-identifier test, and agrees with `token` -- the
                 boundary the rest of the tree already uses -- on every spelling tried
  check          the short name inside the long one is quiet; written on its own it is asked for
  brief          the short name's address is not listed as touched until it is written
  questions      the short output name is not "mentioned" until it is written
"""

from __future__ import annotations

import copy
import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.brief import sections
from sce_author.check import check
from sce_author.pack import load_pack
from sce_author.prose import Prose, Source, load_prose, token
from sce_author.questions import mentions

HERE = pathlib.Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "crossing"
SHORT, LONG = "IN_ObstacleDetector", "IN_ObstacleDetectorExtended"
BASE = "plant/in/obstacle-base"


def prose_of(text: str) -> Prose:
    return Prose((Source(pathlib.Path("spec.md"), text),))


class TheQuestion(unittest.TestCase):
    def test_a_name_written_on_its_own_is_written(self):
        self.assertTrue(prose_of("When In_SupplyMode is HIGH the lamp is ON.").writes(["In_SupplyMode"]))

    def test_a_name_that_begins_a_longer_one_is_not_written_by_the_longer_one(self):
        self.assertFalse(prose_of("When In_SupplyModeExtended is HIGH").writes(["In_SupplyMode"]))

    def test_a_name_that_ends_a_longer_one_is_not_written_by_it(self):
        self.assertFalse(prose_of("When In_SupplyMode is HIGH").writes(["SupplyMode"]))

    def test_a_name_inside_a_longer_one_is_not_written_by_it(self):
        self.assertFalse(prose_of("When Pre_Supply_Post is HIGH").writes(["Supply"]))

    def test_punctuation_and_table_bars_do_not_hide_a_name(self):
        for text in ("(In_SupplyMode)", "`In_SupplyMode`,", "| In_SupplyMode |", "In_SupplyMode.",
                     "In_SupplyMode==HIGH", "\nIn_SupplyMode"):
            with self.subTest(text=text):
                self.assertTrue(prose_of(text).writes(["In_SupplyMode"]))

    def test_a_particle_attached_to_a_name_does_not_hide_it(self):
        # A Hangul subject particle, written as escapes: a test file is not the place to carry it.
        self.assertTrue(prose_of("In_SupplyMode\\uac00 HIGH").writes(["In_SupplyMode"]))

    def test_any_one_of_several_names_is_enough(self):
        self.assertTrue(prose_of("OUT_Lamp is ON").writes(["OUT_Other", "OUT_Lamp"]))

    def test_no_names_write_nothing(self):
        self.assertFalse(prose_of("OUT_Lamp is ON").writes([]))

    def test_a_name_with_a_dot_is_asked_of_the_same_boundary(self):
        self.assertFalse(prose_of("Plant.Input.XY is HIGH").writes(["Plant.Input.X"]))
        self.assertTrue(prose_of("Plant.Input.X, is HIGH").writes(["Plant.Input.X"]))

    def test_it_agrees_with_the_token_boundary_on_every_spelling_tried(self):
        texts = ["", "A", "A_B", "A_B_C", "xA", "Ax", "A_", "_A", "A1", "1A", "A A_B A", "(A)", "A.B",
                 "A\nA_B", "\\uac00A\\ub098", "A\\uac00", "A-B", "A_B-A", "a", "AA"]
        names = ["A", "A_B", "B", "A_", "_A", "A1", "1A", "A.B", "A-B", "AA", "a"]
        for text in texts:
            for name in names:
                with self.subTest(text=text, name=name):
                    self.assertEqual(bool(token(name).search(text)), prose_of(text).writes([name]))

    def test_several_files_are_one_body_of_text(self):
        prose = Prose((Source(pathlib.Path("a.md"), "intro"), Source(pathlib.Path("b.md"), "OUT_Lamp is ON")))
        self.assertTrue(prose.writes(["OUT_Lamp"]))


class Crossing(unittest.TestCase):
    """The shared fixture with its one obstacle input written under a LONGER name, and a second
    input carrying the short name, which the cases drive and no rule reads."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.dir = pathlib.Path(self._tmp.name)
        for item in FIXTURE.iterdir():
            if item.is_file():
                shutil.copy(item, self.dir / item.name)
        self.binding = self.dir / "controller.binding.yaml"
        self.lengthen()

    def lengthen(self):
        model_path = self.dir / "interface-model.yaml"
        model = yaml.safe_load(model_path.read_text(encoding="utf-8"))
        for entry in model["entries"]:
            if entry["address"] == "plant/in/obstacle":
                entry["names"] = [LONG]
        model["entries"].append({"address": BASE, "role": "input", "names": [SHORT],
                                 "values": {"NONE": 0, "DETECTED": 1}})
        model_path.write_text(yaml.safe_dump(model, sort_keys=False), encoding="utf-8")
        cases_path = self.dir / "examples.yaml"
        cases = yaml.safe_load(cases_path.read_text(encoding="utf-8"))
        for case in cases["cases"]:
            for step in (case, *(case.get("before") or [])):
                step.setdefault("given", {})[BASE] = "NONE"
        cases_path.write_text(yaml.safe_dump(cases, sort_keys=False), encoding="utf-8")
        spec = self.dir / "specification.md"
        spec.write_text(spec.read_text(encoding="utf-8").replace(SHORT, LONG), encoding="utf-8")

    def prose(self, extra: str = "") -> Prose:
        spec = self.dir / "specification.md"
        if extra:
            spec.write_text(spec.read_text(encoding="utf-8") + "\n" + extra + "\n", encoding="utf-8")
        return load_prose([spec])


class Check(Crossing):
    def asked(self, prose) -> list:
        return [f for f in check(load_pack(self.dir), self.binding, prose) if "reads none of" in f.detail]

    def test_the_short_name_that_only_begins_the_long_one_is_not_asked_for(self):
        self.assertEqual([], [str(f) for f in self.asked(self.prose())])

    def test_the_short_name_written_on_its_own_is_asked_for(self):
        found = self.asked(self.prose(f"{SHORT} is FAULT when the lamp is OFF."))
        self.assertEqual(1, len(found), [str(f) for f in found])
        self.assertIn(BASE, found[0].detail)
        self.assertIn(SHORT, found[0].detail)
        self.assertNotIn("plant/in/obstacle ", found[0].detail)


class Brief(Crossing):
    def touched(self, prose) -> str:
        pack = load_pack(self.dir)
        return "\n".join("\n".join(body) for heading, body in sections(prose, pack)
                         if heading.startswith("2."))

    def test_the_short_names_address_is_not_listed_as_touched(self):
        said = self.touched(self.prose())
        self.assertIn("plant/in/obstacle", said)
        self.assertNotIn(BASE, said)

    def test_it_is_listed_once_the_short_name_is_written(self):
        self.assertIn(BASE, self.touched(self.prose(f"{SHORT} is FAULT when the lamp is OFF.")))


class Questions(unittest.TestCase):
    def test_a_short_output_name_is_not_mentioned_by_a_longer_one(self):
        self.assertFalse(mentions(prose_of("OUT_LampState is FLASHING."), ["OUT_Lamp"]))

    def test_it_is_mentioned_when_written(self):
        self.assertTrue(mentions(prose_of("OUT_Lamp is FLASHING."), ["OUT_Lamp"]))

    def test_the_words_an_identifier_is_made_of_still_count(self):
        """The second half of `mentions` is untouched: an output named in words is mentioned."""
        self.assertTrue(mentions(prose_of("the road signal is FLASHING."), ["OUT_RoadSignal"]))


if __name__ == "__main__":
    unittest.main()
