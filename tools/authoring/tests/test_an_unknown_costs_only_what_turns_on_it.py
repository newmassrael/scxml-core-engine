"""Declaring an unknown costs only the positions that turn on it.

A binding may say it does not know which address feeds an input, and that is
the honest answer when nobody has said. It used to stop the whole run: no case
of the component was judged. Writing a plausible address instead cost nothing
and passed -- and a plausible WRONG address is invisible, which is the reason
the key exists. Measured 2026-09-22 across thirty documents written from prose,
the key was used zero times.

So the run no longer refuses and still never invents a value. It asks the one
question that needs no value: does the answer change with it? An unresolved
boolean input has two values, every case runs under both, and only a position
that comes out different is withheld. Asserted here:

    an unknown the answer ignores     is no obstacle at all
    a position that turns on it       is withheld, and the case not passed
    constants alone                   never pass a case     (the first version did)
    a wrong answer that was settled   is still a failure
    an unknown NUMBER                 still refuses -- it has no two values
"""

from __future__ import annotations

import unittest

import yaml

from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import Fixture

HEADER = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="guarded">
  <datamodel>
    <data id="mode" sce:type="bool" sce:direction="in"/>
    <data id="guard" sce:type="bool" sce:direction="in"/>
"""

# The guard is declared and never read: nothing can turn on it.
IGNORES = HEADER + """    <data id="lamp" sce:type="int32" sce:direction="out" expr="mode ? 1 : 0"/>
  </datamodel>
</scxml>
"""

# The guard matters only while the supply is HIGH. At LOW the lamp is off
# whatever it is -- which is what lets one case be judged and not the other.
DEPENDS = HEADER + """    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="(mode &amp;&amp; guard) ? 1 : 0"/>
  </datamodel>
</scxml>
"""

UNKNOWN = "nobody has said which signal carries the guard"

BINDING = {
    "version": 1,
    "document": "guarded.scxml",
    "inputs": {
        "mode": {"address": "Plant.Input.SupplyMode", "equals": "HIGH"},
        "guard": {"unresolved": UNKNOWN},
    },
    "outputs": {
        "lamp": {
            "address": "Plant.Out.Lamp",
            "field": "Stat",
            "map": {0: "OFF", 1: "ON"},
            # A constant the binding always writes. It comes out the same
            # under every value of the guard, which is exactly what let the
            # first version pass a case on it alone.
            "also": {"Value": 7},
        }
    },
}

HIGH = {"name": "supply high",
        "given": {"Plant.Input.SupplyMode": "HIGH"},
        "expect": {"Plant.Out.Lamp.Stat": "ON", "Plant.Out.Lamp.Value": 7}}
LOW = {"name": "supply low",
       "given": {"Plant.Input.SupplyMode": "LOW"},
       "expect": {"Plant.Out.Lamp.Stat": "OFF", "Plant.Out.Lamp.Value": 7}}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class AnUnknownCostsOnlyWhatTurnsOnIt(Fixture):
    def run_with(self, document, *cases, binding=BINDING):
        (self.root / "guarded.scxml").write_text(document, encoding="utf-8")
        path = self.root / "guarded.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        (self.pack_dir / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "cases": list(cases)}), encoding="utf-8")
        return verify(self.pack(), path)

    def by_name(self, result):
        return {c.name: c for c in result.results}

    def test_an_unknown_the_answer_ignores_is_no_obstacle(self):
        """The discriminator for everything below: without it, a verifier
        that withheld every position would pass the other tests."""
        result = self.run_with(IGNORES, HIGH, LOW)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((2, 0, 0),
                         (result.passed, result.failed, result.unjudged),
                         [(c.name, c.refusal) for c in result.results])
        self.assertEqual(0, result.undetermined)
        self.assertEqual({"guard": UNKNOWN}, result.unresolved,
                         "declared even when it cost nothing -- the reason is "
                         "still a question somebody owes an answer to")

    def test_a_position_that_turns_on_the_unknown_is_withheld(self):
        result = self.run_with(DEPENDS, HIGH, LOW)
        self.assertTrue(result.ran, result.refusal)
        cases = self.by_name(result)
        self.assertTrue(cases["supply low"].passed,
                        "off whatever the guard is: judged, and right")
        self.assertEqual(["Plant.Out.Lamp.Stat"],
                         cases["supply high"].undetermined)
        self.assertFalse(cases["supply high"].judged)
        self.assertEqual((1, 0, 1),
                         (result.passed, result.failed, result.unjudged))

    def test_constants_alone_never_pass_a_case(self):
        """⚠ The loophole the first version of this change left open.

        It withheld a case only when EVERY position was withheld. A case that
        expects an event also expects the constants the binding writes beside
        it, those agree under any input, and so a case whose whole point
        turned on the unknown still counted as passed. On the first real probe
        that read 4 passed where 3 of the 4 had their status withheld.
        """
        result = self.run_with(DEPENDS, HIGH)
        case = self.by_name(result)["supply high"]
        self.assertNotIn("Plant.Out.Lamp.Value", case.undetermined,
                         "the constant WAS settled; the point is that it is "
                         "not enough")
        self.assertFalse(case.passed)
        self.assertEqual(0, result.passed)

    def test_a_wrong_answer_that_was_settled_is_still_a_failure(self):
        """Withholding the unknown does not excuse what was known."""
        wrong = {**LOW, "name": "supply low, wrongly expected on",
                 "expect": {"Plant.Out.Lamp.Stat": "ON"}}
        result = self.run_with(DEPENDS, wrong)
        self.assertEqual((0, 1, 0),
                         (result.passed, result.failed, result.unjudged))

    def test_the_command_does_not_exit_green_on_an_open_address(self):
        """⚠ A regression the first version of this change introduced.

        When an open input was a refusal, the refusal carried status 1, and a
        pipeline gating on `verify` could not read an unfinished binding as
        verified. Running open inputs kept the COUNTS honest and let the
        STATUS become 0 whenever nothing failed. The case below is the purest
        form of it: every case passes -- the unknown changes nothing -- and
        the binding still names no address, so it cannot ship.
        """
        from tests.test_the_commands_work_as_a_product import run
        self.run_with(IGNORES, HIGH, LOW)
        argv = ["verify", "--pack", str(self.pack_dir),
                "--binding", str(self.root / "guarded.binding.yaml")]
        code, said, _ = run(argv)
        self.assertIn("2 passed, 0 failed", said, "the premise: nothing failed")
        self.assertEqual(1, code, said)
        self.assertIn("not a pass", said)
        # ⚠ The discriminator: the SAME document with the address supplied
        # exits green. Without it, a command that always exited 1 passes above.
        resolved = {**BINDING, "inputs": {
            **BINDING["inputs"],
            "guard": {"address": "Plant.Input.SupplyMode", "equals": "LOW"}}}
        self.run_with(IGNORES, HIGH, LOW, binding=resolved)
        code, said, _ = run(argv)
        self.assertEqual(0, code, said)
        self.assertNotIn("not a pass", said)

    def test_an_unknown_number_still_refuses(self):
        """A number has no two values to try; any one chosen is invented.

        ⚠ What makes it a number is the DOCUMENT's declaration. It used to be
        the binding's `number: true`, so the same binding into a document
        declaring the guard `int32` ran it as False and then True -- two values
        the document says it can never receive.
        """
        counted = DEPENDS.replace('id="guard" sce:type="bool"',
                                  'id="guard" sce:type="int32"').replace(
            "(mode &amp;&amp; guard)", "(mode &amp;&amp; guard &gt; 0)")
        self.assertNotEqual(counted, DEPENDS, "the premise: the guard is a number")
        result = self.run_with(counted, HIGH)
        self.assertFalse(result.ran)
        self.assertIn("truth value", result.refusal)
        self.assertIn(UNKNOWN, result.refusal)
        # The discriminator: the same binding into the `bool` guard runs.
        self.assertTrue(self.run_with(DEPENDS, HIGH).ran)


# ------------------------------------------------ one layer down: the document

OPEN_DOC = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="guarded">
  <datamodel>
    <data id="mode" sce:type="bool" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out" expr="mode ? 1 : 0"/>
    <!-- Nobody has said how loud the chime is. sce:unresolved is the marker. -->
    <data id="chime" sce:type="int32" sce:direction="out"
          sce:unresolved="CHIME_LEVEL"
          sce:unresolved-reason="the source names the chime and not its level"/>
    <data id="loud" sce:type="int32" sce:direction="out" expr="chime + 1"/>
  </datamodel>
</scxml>
"""

OPEN_BINDING = {
    "version": 1,
    "document": "guarded.scxml",
    "inputs": {"mode": {"address": "Plant.Input.SupplyMode", "equals": "HIGH"}},
    "outputs": {
        "lamp": {"address": "Plant.Out.Lamp", "field": "Stat",
                 "map": {0: "OFF", 1: "ON"}},
        "chime": {"address": "Plant.Out.Lamp", "field": "Value",
                  "passthrough": True},
        "loud": {"address": "Plant.Out.Unwritten", "field": "Stat",
                 "map": {0: "NONE", 1: "OFF", 2: "ON"}},
    },
}

LAMP_ONLY = {"name": "lamp only",
             "given": {"Plant.Input.SupplyMode": "HIGH"},
             "expect": {"Plant.Out.Lamp.Stat": "ON"}}
WITH_CHIME = {"name": "lamp and chime",
              "given": {"Plant.Input.SupplyMode": "HIGH"},
              "expect": {"Plant.Out.Lamp.Stat": "ON", "Plant.Out.Lamp.Value": 3}}
LOUDNESS = {"name": "loudness",
            "given": {"Plant.Input.SupplyMode": "HIGH"},
            "expect": {"Plant.Out.Unwritten.Stat": "ON"}}
WRONG_LAMP = {"name": "lamp wrongly expected off",
              "given": {"Plant.Input.SupplyMode": "HIGH"},
              "expect": {"Plant.Out.Lamp.Stat": "OFF", "Plant.Out.Lamp.Value": 3}}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class AnOpenOutputCostsOnlyWhatReadsIt(Fixture):
    """The same rule for an open value the DOCUMENT declares.

    The product refuses to build a document holding `sce:unresolved`, and for
    shipping that is right. It used to end verification as well: measured
    2026-09-22, five documents written from prose left one value open and all
    five went unverified, although their decision logic -- checked by filling
    that one value by hand -- was as right as the documents that had guessed.
    """

    def run_with(self, document, *cases, binding=OPEN_BINDING):
        (self.root / "guarded.scxml").write_text(document, encoding="utf-8")
        path = self.root / "guarded.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        (self.pack_dir / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "cases": list(cases)}), encoding="utf-8")
        return verify(self.pack(), path)

    def by_name(self, result):
        return {c.name: c for c in result.results}

    def test_what_does_not_read_the_open_value_is_judged(self):
        """The discriminator: a verifier that withheld everything would
        pass every other test in this class."""
        result = self.run_with(OPEN_DOC, LAMP_ONLY)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0),
                         (result.passed, result.failed, result.unjudged))
        self.assertIn("chime", result.unresolved_outputs)
        self.assertIn("not its level", result.unresolved_outputs["chime"],
                      "the author's reason is the message; it has to survive "
                      "the refusal it used to travel in")

    def test_a_position_the_open_value_writes_is_withheld(self):
        result = self.run_with(OPEN_DOC, WITH_CHIME)
        case = self.by_name(result)["lamp and chime"]
        self.assertEqual(["Plant.Out.Lamp.Value"], case.undetermined)
        self.assertFalse(case.passed, "not passed on part of what it asserts")
        self.assertFalse(case.judged)

    def test_an_output_computed_from_it_is_withheld_too(self):
        """⚠ `loud` is computed from the placeholder. Comparing it would be a
        verdict about a value the author never wrote."""
        result = self.run_with(OPEN_DOC, LOUDNESS)
        self.assertIn("loud", result.unresolved_outputs)
        self.assertIn("chime", result.unresolved_outputs["loud"])
        self.assertEqual(["Plant.Out.Unwritten.Stat"],
                         self.by_name(result)["loudness"].undetermined)

    def test_a_settled_wrong_answer_is_still_a_failure(self):
        result = self.run_with(OPEN_DOC, WRONG_LAMP)
        self.assertEqual((0, 1, 0),
                         (result.passed, result.failed, result.unjudged))

    def test_the_document_is_untouched_and_still_refused_for_shipping(self):
        """The copy is the verifier's. What the author wrote, and the
        product's refusal to ship it, are exactly as they were."""
        from sce_author.verify import _emit
        self.run_with(OPEN_DOC, LAMP_ONLY)
        self.assertEqual(OPEN_DOC,
                         (self.root / "guarded.scxml").read_text(encoding="utf-8"))
        shipped = self.root / "shipped"
        shipped.mkdir()
        build = _emit(self.root / "guarded.scxml", _default_codegen(), shipped,
                      (), "python")
        self.assertIn("unresolved placeholder", build.refusal)

    def test_an_open_value_with_no_safe_placeholder_keeps_the_refusal(self):
        """A type this has no placeholder for is refused by the product, in
        its own words, rather than built around with a guess about what
        compiles."""
        text = OPEN_DOC.replace('id="chime" sce:type="int32"',
                                'id="chime" sce:type="string"')
        result = self.run_with(text, LAMP_ONLY)
        self.assertFalse(result.ran)
        self.assertIn("unresolved placeholder", result.refusal)


if __name__ == "__main__":
    unittest.main()
