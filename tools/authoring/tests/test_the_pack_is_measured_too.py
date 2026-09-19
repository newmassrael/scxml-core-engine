"""The one input nothing here could ever test, made measurable instead.

Every command compares a document against the pack, so no answer can be more
right than the pack is. And the pack is produced by an adapter that reads a
platform's own format -- which is not this tree's format, belongs to whoever
owns that platform, and cannot be committed here at all when the platform is
under an agreement. The suite has 175 cases and not one of them has ever seen
the code that writes the packs it runs against.

⚠ TESTING SOMEBODY ELSE'S CONVERTER IS NOT THE ANSWER AND NEVER WILL BE. The
answer is that the two ways a pack goes wrong were written down before
anything computed them, and both are measurable from the pack and the prose:

    the spellings an address goes by   every class that skips an address it
                                       believes unmentioned goes quiet
    which text belongs to which        the gated-output class above all

⚠⚠ What `review` refuses to do is give a verdict. A pack is a claim about a
platform this tree does not have; "correct" is not a thing it can be told. It
reports figures, and separately names the shapes that cannot be right whatever
the platform turns out to be -- a model with no outputs, an address with no
value space, a partition that owns none of the document.

⚠⚠⚠ Measured over 129 subject packs on the day it was written: attribution
ranges from under 10% to over 90%, with 38 packs above 90% and **29 below
20%**. Nothing computed that before, and on those 29 a whole question class
was running blind while the suite stayed green.
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest

import yaml

from sce_author.pack import load_pack
from sce_author.prose import load_prose
from sce_author.review import review

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

CONVENTIONS = {
    "version": 1,
    "name_classes": [{"pattern": r"\bIn_[A-Za-z0-9_]+\b", "role": "supplied"}],
}

MODEL = {
    "version": 1,
    "entries": [
        {"address": "Plant.In.Supply", "role": "input", "names": ["In_Supply"],
         "values": {"LOW": 0, "HIGH": 1}},
        {"address": "Plant.Out.Lamp", "role": "output", "names": ["Out_Lamp"],
         "values": {"OFF": 0, "ON": 1}},
    ],
}

EXAMPLES = {
    "version": 1,
    "origin": "shipped with the fixture",
    "independent_cases": True,
    "cases": [{"name": "the lamp follows the supply",
               "given": {"Plant.In.Supply": 1},
               "expect": {"Plant.Out.Lamp": 1}}],
}

PROSE = "Out_Lamp is ON when In_Supply == HIGH, and OFF otherwise.\n"


class ThePackIsMeasuredToo(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def reviewed(self, model=MODEL, examples=EXAMPLES, prose=PROSE):
        pack_dir = self.root / "pack"
        pack_dir.mkdir(exist_ok=True)
        (pack_dir / "interface-model.yaml").write_text(
            yaml.safe_dump(model), encoding="utf-8")
        (pack_dir / "conventions.yaml").write_text(
            yaml.safe_dump(CONVENTIONS), encoding="utf-8")
        path = pack_dir / "examples.yaml"
        if examples is None:
            path.unlink(missing_ok=True)
        else:
            path.write_text(yaml.safe_dump(examples), encoding="utf-8")
        spec = self.root / "spec.md"
        spec.write_text(prose, encoding="utf-8")
        return review(load_pack(pack_dir), load_prose([spec]))

    # --------------------------------------------------- the quiet failures

    def test_a_pack_that_can_be_trusted_raises_nothing(self):
        """The discriminator. Without it, an alarm list that always had
        something in it would pass every case below.
        """
        got = self.reviewed()
        self.assertEqual([], got.alarms())
        self.assertEqual(2, got.addresses)
        self.assertEqual(1, got.outputs)
        self.assertGreater(got.attribution, 0.5)

    def test_a_model_with_no_output_is_judging_nothing(self):
        model = {"version": 1, "entries": [MODEL["entries"][0]]}
        said = " ".join(self.reviewed(model=model).alarms())
        self.assertIn("no output", said)

    def test_a_pack_with_no_examples_says_it_is_the_weakest_case(self):
        """⚠ Reported as an ALARM rather than as nothing to report. Without
        examples two classes cannot fire and `verify` cannot run at all, so
        the short question list that results reads as a clean specification.
        """
        said = " ".join(self.reviewed(examples=None).alarms())
        self.assertIn("no examples", said)

    def test_an_address_with_no_value_space_passes_every_value(self):
        model = {"version": 1, "entries": [
            *MODEL["entries"],
            {"address": "Plant.Out.Opaque", "role": "output",
             "names": ["Out_Opaque"]},
        ]}
        got = self.reviewed(model=model)
        self.assertIn("Plant.Out.Opaque", got.unspecified)
        self.assertIn("neither a value space nor a type",
                      " ".join(got.alarms()))

    def test_a_partition_that_owns_none_of_the_prose_is_named(self):
        """⚠ The measurement the README specified and nothing computed. A
        class that reads "what the document says about THIS address" is worth
        exactly as much as the share of the document it can attribute.
        """
        got = self.reviewed(prose="Nothing here names anything at all.\n")
        self.assertEqual(0.0, got.attribution)
        self.assertIn("attributes 0%", " ".join(got.alarms()))

    # ------------------------------------ what it measures rather than judges

    def test_an_address_the_prose_never_writes_is_counted_not_condemned(self):
        """It is either a genuine silence or a spelling the pack lacks, and
        nothing here can tell which. Counting it is the honest act; calling it
        a defect would be a claim about a platform this tree does not have.
        """
        model = {"version": 1, "entries": [
            *MODEL["entries"],
            {"address": "Plant.Out.Never", "role": "output",
             "names": ["Out_Never"], "values": {"OFF": 0, "ON": 1}},
        ]}
        got = self.reviewed(model=model)
        self.assertIn("Plant.Out.Never", got.unmentioned)
        self.assertNotIn("Plant.Out.Never", " ".join(got.alarms()))

    def test_the_second_subject_matter_is_measured_at_all(self):
        """A lower bound on the instrument, over a pack nobody wrote for it.
        Without this the figures above could all come from fixtures shaped to
        produce them.
        """
        got = review(load_pack(CROSSING),
                     load_prose([CROSSING / "specification.md"]))
        self.assertGreater(got.addresses, 0)
        self.assertGreater(got.outputs, 0)
        self.assertTrue(got.has_examples)
        self.assertEqual(0, got.driven_undeclared)


if __name__ == "__main__":
    unittest.main()
