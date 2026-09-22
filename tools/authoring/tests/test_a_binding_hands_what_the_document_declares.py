"""An address named alone hands the document its own value, as the document types it.

The document declares each input's type and the interface model declares what
each address carries, so a binding does not say either a third time. A rule
naming an address and nothing that compares it hands that address's value
over, read as the document's type -- and `check` and `verify` refuse, in the
same words, when the two declarations disagree.

⚠ What this replaced. `number: true` restated the type, and measured
2026-09-22 over 24 bindings all 24 of its uses fed `int32` inputs while
nothing compared the two. A text input had no key at all: the one binding that
needed one wrote a rule reading back its own previous output, which could never
produce the value its only case expected, and the failure was reported as a
refuted guess about something else. Asserted here:

  the types    what a value of each type IS comes from the product's grammar,
               not a copy of it
  refused      a pairing the declarations disagree on, a comparison into a
               non-bool input, a bound on a non-number, a memory of another
               type, and an enumeration whose numbers contradict the address's
  read         a text, a truth value, a number through the value space, an
               enumeration's number, and absence -- declared or refused
  run          a text input drives a document end to end, and the same rule
               into a `bool` input is refused by `check`
"""

from __future__ import annotations

import pathlib
import unittest
import xml.etree.ElementTree as ET

import yaml

from sce_author import delivery
from sce_author.delivery import DeliveryError, product_types, read_as_is, refusal
from sce_author.pack import Field
from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import CONVENTIONS, MODEL, Fixture

TEXT = Field(name="", values=None, type="text")
TRUTH = Field(name="", values=None, type="boolean")
COUNT = Field(name="", values=None, type="number")
LEVEL = Field(name="", values={"NONE": 0, "LOW": 1, "HIGH": 2, "MAX": 3}, type=None)
AT = {"address": "Plant.Input.X"}


class TheTypesComeFromTheProduct(unittest.TestCase):
    def test_every_type_the_grammar_declares_has_a_reading(self):
        """⚠ Read independently of the module, so a module that quietly kept
        a copy of the list would be caught the day the grammar grew."""
        root = ET.parse(delivery._GRAMMAR).getroot()
        xs = "{http://www.w3.org/2001/XMLSchema}"
        declared = {e.get("value") for s in root.iter(f"{xs}simpleType")
                    if s.get("name") == "sceType" for e in s.iter(f"{xs}enumeration")}
        self.assertGreater(len(declared), 5, "the premise: the grammar was read")
        self.assertEqual(declared, set(product_types()))
        self.assertEqual("number", product_types()["int32"])
        self.assertEqual("text", product_types()["string"])
        self.assertEqual("bool", product_types()["bool"])
        self.assertEqual("enum", delivery.document_class("enum:Level"))


class TheDeclarationsMustAgree(unittest.TestCase):
    def test_a_text_into_a_string_is_handed_over(self):
        self.assertEqual("", refusal("caption", AT, "string", TEXT))

    def test_a_text_into_a_bool_is_refused_and_says_what_to_write(self):
        why = refusal("ready", AT, "bool", TEXT)
        self.assertIn("a text", why)
        self.assertIn("`equals`", why)

    def test_a_truth_value_into_a_bool_is_handed_over(self):
        self.assertEqual("", refusal("ready", AT, "bool", TRUTH))

    def test_a_comparison_into_a_number_is_refused(self):
        why = refusal("count", {**AT, "equals": "HIGH"}, "int32", LEVEL)
        self.assertIn("truth value", why)
        self.assertIn("int32", why)

    def test_a_bound_on_a_text_is_refused(self):
        self.assertIn("`range`", refusal("caption", {**AT, "range": [0, 9]},
                                         "string", TEXT))

    def test_a_memory_of_another_type_is_refused(self):
        why = refusal("was", {"previous_of": "caption", "caller_keeps": "x"},
                      "int32", None, remembered_type="string")
        self.assertIn("string", why)
        self.assertEqual("", refusal("was", {"previous_of": "count",
                                             "caller_keeps": "x"},
                                     "int32", None, remembered_type="int32"))

    def test_an_address_declared_without_a_kind_is_not_given_one(self):
        """⚠ The pack used to type such an address `number` by default, which
        is the builder inventing what the platform never said."""
        unsaid = Field(name="", values=None, type=None)
        self.assertIn("without saying what it carries",
                      refusal("count", AT, "int32", unsaid))
        with self.assertRaises(DeliveryError):
            read_as_is("count", AT, "3", True, "int32", unsaid)
        self.assertEqual("", refusal("count", {**AT, "equals": "ON"}, "bool", unsaid),
                         "a comparison needs no kind; it asks one question")

    def test_an_enumeration_meets_the_address_on_the_number(self):
        self.assertEqual("", refusal("mode", AT, "enum:Level", LEVEL,
                                     variants={"LOW": 1, "HIGH": 2}))

    def test_an_enumeration_whose_numbers_contradict_the_address_is_refused(self):
        """⚠ `number: true` handed the number over with no such check, so a
        document whose enumeration numbered HIGH as 1 would have read the
        platform's LOW as its HIGH, silently."""
        why = refusal("mode", AT, "enum:Level", LEVEL,
                      variants={"LOW": 2, "HIGH": 1})
        self.assertIn("HIGH as 2 and 1", why)

    def test_an_enumeration_nobody_imported_is_refused(self):
        self.assertIn("no enumeration", refusal("mode", AT, "enum:Level", LEVEL))


class TheValueIsRead(unittest.TestCase):
    def test_a_text_is_read_as_it_is(self):
        self.assertEqual("DUSK", read_as_is("c", AT, "DUSK", True, "string", TEXT))

    def test_absence_is_declared_or_refused(self):
        rule = {**AT, "when_absent": ""}
        self.assertEqual("", read_as_is("c", rule, None, False, "string", TEXT))
        self.assertEqual("", read_as_is("c", rule, "timeout", True, "string", TEXT,
                                        absence_tokens=("TIMEOUT",)),
                         "a token the conventions list is absence, not a text")
        with self.assertRaises(DeliveryError) as raised:
            read_as_is("c", AT, None, False, "string", TEXT)
        self.assertIn("`when_absent`", str(raised.exception))

    def test_a_truth_value_is_read_and_anything_else_is_refused(self):
        self.assertIs(True, read_as_is("r", AT, "True", True, "bool", TRUTH))
        self.assertIs(False, read_as_is("r", AT, "0", True, "bool", TRUTH))
        with self.assertRaises(DeliveryError):
            read_as_is("r", AT, "maybe", True, "bool", TRUTH)

    def test_a_number_goes_through_the_value_space(self):
        self.assertEqual(2, read_as_is("m", AT, "HIGH", True, "int32", LEVEL))
        self.assertEqual(7, read_as_is("n", AT, "7.0", True, "int32", COUNT))

    def test_an_enumeration_takes_only_its_own_variants(self):
        variants = {"LOW": 1, "HIGH": 2}
        self.assertEqual(2, read_as_is("m", AT, "HIGH", True, "enum:Level", LEVEL,
                                       variants=variants))
        with self.assertRaises(DeliveryError) as raised:
            read_as_is("m", AT, "MAX", True, "enum:Level", LEVEL, variants=variants)
        self.assertIn("no variant numbered", str(raised.exception))

    def test_an_address_the_model_does_not_describe_is_not_guessed_at(self):
        with self.assertRaises(DeliveryError) as raised:
            read_as_is("c", AT, "x", True, "string", None)
        self.assertIn("does not declare", str(raised.exception))


CAPTIONED = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="captioned">
  <datamodel>
    <data id="caption" sce:type="{caption_type}" sce:direction="in"/>
    <data id="shown" sce:type="{caption_type}" sce:direction="out" expr="caption"/>
  </datamodel>
</scxml>
"""

WITH_TEXT = {**MODEL, "entries": MODEL["entries"] + [
    {"address": "Plant.Input.Caption", "role": "input", "names": ["In_Caption"],
     "type": "text"},
    {"address": "Plant.Out.Caption", "role": "output", "names": ["Fig_Caption"],
     "type": "text"},
]}

CAPTION_BINDING = {
    "version": 1,
    "document": "captioned.scxml",
    "inputs": {"caption": {"address": "Plant.Input.Caption", "when_absent": ""}},
    "outputs": {"shown": {"address": "Plant.Out.Caption", "field": "",
                          "passthrough": True}},
}

SHOWN = {"version": 1, "origin": "written for this test", "independent_cases": True,
         "cases": [{"name": "the caption is shown",
                    "given": {"Plant.Input.Caption": "DUSK"},
                    "expect": {"Plant.Out.Caption": "DUSK"}}]}


class ATextDrivesADocument(Fixture):
    def setUp(self):
        super().setUp()
        self.write_pack(WITH_TEXT, CONVENTIONS, SHOWN)

    def bound(self, caption_type):
        (self.root / "captioned.scxml").write_text(
            CAPTIONED.format(caption_type=caption_type), encoding="utf-8")
        path = self.root / "captioned.binding.yaml"
        path.write_text(yaml.safe_dump(CAPTION_BINDING), encoding="utf-8")
        return path

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_a_text_input_is_run_end_to_end(self):
        """⚠ The discriminator. Before this, the rule above was refused with
        "the binding says nothing about how to read", and no key could say."""
        result = verify(self.pack(), self.bound("string"))
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_check_accepts_it_and_refuses_the_same_rule_into_a_bool(self):
        from sce_author.check import check
        self.assertEqual([], check(self.pack(), self.bound("string")))
        found = [str(f) for f in check(self.pack(), self.bound("bool"))]
        self.assertTrue(any("input caption" in f and "`equals`" in f for f in found),
                        found)


if __name__ == "__main__":
    unittest.main()
