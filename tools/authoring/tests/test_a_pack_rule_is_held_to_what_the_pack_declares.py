"""A pack's conditions and rules are held to what the pack itself declares.

Reproduced against the pack loader before this existed -- each loaded cleanly:

  * a phrase read as `poweredUp &&& (`
  * a phrase read as `ghostInput`, an input nothing declares
  * an input whose rule read an address the interface model does not have

The table was read as a bag of identifiers: the ones that named a declared
input were read and everything else was ignored. So a phrase whose input was
misspelt read NO input, and the condition the specification states was missing
from every check with nothing to say so. A malformed regular expression left
the loader as a traceback out of the `re` module.

What is held here is a property of the pack and nothing about the platform: a
pack can be consistent and still describe the platform wrongly, and that is for
whoever produces the pack and for `review`'s figures, not for the loader.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.errors import PackError
from sce_author.expression import ExpressionError, names
from sce_author.pack import load_conventions, load_pack, reads_no_input

HERE = pathlib.Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "crossing"

BASE = {
    "version": 1,
    "name_classes": [{"pattern": r"\bIN_[A-Za-z0-9_]+\b", "role": "supplied"}],
    "preconditions": {
        "inputs": {"poweredUp": "the installation is energised"},
        "phrases": {"powered": "poweredUp"},
    },
}


class TheExpressionLanguage(unittest.TestCase):
    def test_what_the_table_has_always_written_is_still_an_expression(self):
        for text, expected in (("poweredUp", ("poweredUp",)),
                               ("!poweredUp", ("poweredUp",)),
                               ("poweredUp && true", ("poweredUp", "true")),
                               ("a || (b && !c)", ("a", "b", "c")),
                               ("!!a", ("a",)),
                               ("false && true", ("false", "true"))):
            with self.subTest(text=text):
                self.assertEqual(expected, names(text))

    def test_what_is_not_an_expression_is_refused_where_it_stops_being_one(self):
        for text, said in (("", "empty"),
                           ("   ", "empty"),
                           ("poweredUp &&& (", "column 13"),
                           ("a &&", "at its end"),
                           ("a b", "an operator"),
                           ("(a", "close the bracket"),
                           ("a)", "an operator, or nothing more"),
                           ("&& a", "a name or ("),
                           ("a == 1", "column 3"),
                           ("a | b", "column 3")):
            with self.subTest(text=text):
                with self.assertRaises(ExpressionError) as caught:
                    names(text)
                self.assertIn(said, str(caught.exception))

    def test_a_constant_is_still_told_from_a_reading(self):
        for constant in ("true", "false", "!true", "TRUE", "false && true"):
            with self.subTest(constant=constant):
                self.assertTrue(reads_no_input(constant))
        for reading in ("poweredUp", "!poweredUp", "poweredUp && true"):
            with self.subTest(reading=reading):
                self.assertFalse(reads_no_input(reading))


class AConventionsFile(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = pathlib.Path(temporary.name)

    def write(self, name: str, document: dict) -> pathlib.Path:
        path = self.dir / name
        path.write_text(yaml.safe_dump(document, sort_keys=False), encoding="utf-8")
        return path

    @staticmethod
    def changed(**preconditions) -> dict:
        document = yaml.safe_load(yaml.safe_dump(BASE))
        document["preconditions"].update(preconditions)
        return document

    def test_a_malformed_expression_is_refused_naming_its_phrase(self):
        path = self.write("c.yaml", self.changed(phrases={"powered": "poweredUp &&& ("}))
        with self.assertRaises(PackError) as caught:
            load_conventions([path])
        message = str(caught.exception)
        self.assertIn(str(path), message)
        self.assertIn("preconditions.phrases 'powered'", message)
        self.assertIn("column 13", message)

    def test_a_name_nothing_declares_is_refused_with_the_declared_ones_named(self):
        path = self.write("c.yaml", self.changed(phrases={"powered": "poweredup"}))
        with self.assertRaises(PackError) as caught:
            load_conventions([path])
        message = str(caught.exception)
        self.assertIn("'poweredup' is not declared in preconditions.inputs", message)
        self.assertIn("(poweredUp)", message)
        self.assertIn("reads nothing", message)

    def test_every_undeclared_name_is_named_once(self):
        path = self.write("c.yaml", self.changed(phrases={"powered": "ghost && !ghost && poweredUp && other"}))
        with self.assertRaises(PackError) as caught:
            load_conventions([path])
        self.assertIn("'ghost', 'other' are not declared", str(caught.exception))

    def test_an_input_a_later_file_declares_is_declared(self):
        first = self.write("a.yaml", self.changed(inputs={}, phrases={"powered": "poweredUp"}))
        second = self.write("b.yaml", self.changed(phrases={}))
        conventions = load_conventions([first, second])
        self.assertEqual("poweredUp", conventions.precondition_phrases["powered"])
        self.assertEqual({"poweredUp"}, conventions.precondition_reads("poweredUp && true"))

    def test_a_constant_still_needs_its_reason_and_then_loads(self):
        refused = self.write("c.yaml", self.changed(phrases={"powered": "true"}))
        with self.assertRaises(PackError) as caught:
            load_conventions([refused])
        self.assertIn("names no input", str(caught.exception))
        allowed = self.write("d.yaml", self.changed(phrases={
            "powered": {"expression": "true", "assumed": "it runs only while energised"}}))
        self.assertEqual("true", load_conventions([allowed]).precondition_phrases["powered"])

    def test_a_malformed_regular_expression_is_refused_in_a_sentence_that_names_it(self):
        for field, document in (
                ("name_classes pattern",
                 {**self.changed(), "name_classes": [{"pattern": "IN_(", "role": "supplied"}]}),
                ("preconditions.pattern", self.changed(pattern="(?P<phrase>")),
                ("preconditions.normalise from", self.changed(normalise=[{"from": "[", "to": ""}])),
                ("duration_pattern", {**self.changed(), "duration_pattern": "(["}),
                ("comparison_pattern", {**self.changed(), "comparison_pattern": "(?P<name"})):
            with self.subTest(field=field):
                path = self.write("c.yaml", document)
                with self.assertRaises(PackError) as caught:
                    load_conventions([path])
                message = str(caught.exception)
                self.assertIn(field, message)
                self.assertIn("is not a regular expression", message)


class ARuleReadAgainstTheModel(unittest.TestCase):
    """The fixture pack, its `poweredUp` input given a rule to read."""

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name) / "pack"
        shutil.copytree(FIXTURE, self.root)

    def give(self, rule: dict, protocols: dict | None = None) -> None:
        path = self.root / "conventions.yaml"
        document = yaml.safe_load(path.read_text(encoding="utf-8"))
        document["preconditions"]["inputs"]["poweredUp"] = {
            "note": "the installation is energised", "rule": rule}
        if protocols is not None:
            document["protocols"] = protocols
        path.write_text(yaml.safe_dump(document, sort_keys=False), encoding="utf-8")

    def test_a_rule_that_reads_a_declared_address_loads(self):
        self.give({"address": "plant/in/mains-power", "equals": "OK"})
        self.assertIn("poweredUp", load_pack(self.root).conventions.precondition_rules)

    def test_a_rule_that_reads_an_address_nobody_declares_is_refused(self):
        self.give({"address": "plant/in/ghost", "equals": "OK"})
        with self.assertRaises(PackError) as caught:
            load_pack(self.root)
        message = str(caught.exception)
        self.assertIn("preconditions.inputs 'poweredUp' rule", message)
        self.assertIn("'plant/in/ghost', which the interface model does not declare", message)

    def test_a_rule_through_a_protocol_the_pack_does_not_declare_is_refused(self):
        self.give({"protocol": "latched", "parameters": {"set": "plant/in/mains-power"}})
        with self.assertRaises(PackError) as caught:
            load_pack(self.root)
        self.assertIn("protocol 'latched', which the pack's `protocols` does not declare",
                      str(caught.exception))

    def test_a_protocol_parameter_the_rule_leaves_out_or_misplaces_is_refused(self):
        protocols = {"latched": {"parameters": ["set", "reset"]}}
        self.give({"protocol": "latched", "parameters": {"set": "plant/in/mains-power"}},
                  protocols)
        with self.assertRaises(PackError) as caught:
            load_pack(self.root)
        self.assertIn("needs parameter 'reset'", str(caught.exception))
        self.give({"protocol": "latched",
                   "parameters": {"set": "plant/in/mains-power", "reset": "plant/in/ghost"}},
                  protocols)
        with self.assertRaises(PackError) as caught:
            load_pack(self.root)
        self.assertIn("gives 'reset' the address 'plant/in/ghost'", str(caught.exception))

    def test_a_protocol_rule_that_is_whole_loads(self):
        self.give({"protocol": "latched", "parameters": {
            "set": "plant/in/mains-power", "reset": "plant/in/obstacle"}},
            {"latched": {"parameters": ["set", "reset"]}})
        self.assertIn("poweredUp", load_pack(self.root).conventions.precondition_rules)


if __name__ == "__main__":
    unittest.main()
