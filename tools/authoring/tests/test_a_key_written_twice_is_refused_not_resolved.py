"""A key written twice in one mapping is refused, where the author wrote it.

YAML and JSON both let a mapping repeat a key, and both libraries keep the LAST
value and say nothing. Reproduced against the pack loader before this existed:

  * `values: {CLEAR: 0, APPROACHING: 1, APPROACHING: 0}` loaded as
    `APPROACHING: 0`, in YAML and in JSON
  * an entry that wrote `role` twice loaded with the second role, so an input
    became an output
  * a phrase written twice in the conventions loaded with the second reading,
    so `poweredUp` became `!poweredUp`

Each is a file that loads cleanly and means something its author did not
write. Every file this package reads from an author goes through one reader
(`sce_author.structured`): the pack, a binding, and the owner's decision
record.
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest

from sce_author.check import read_binding
from sce_author.decisions import DecisionRecordError, load_record
from sce_author.errors import PackError
from sce_author.pack import load_conventions, load_model
from sce_author.structured import RepeatedKey, read_json, read_yaml

MODEL = """version: 1
entries:
  - address: plant/in/train-approach
    role: input
    names: [IN_TrainApproach]
    values:
      CLEAR: 0
      APPROACHING: 1
      OCCUPIED: 2
"""

CONVENTIONS = """version: 1
name_classes:
  - pattern: '\\bIN_[A-Za-z0-9_]+\\b'
    role: supplied
preconditions:
  inputs:
    poweredUp: "the installation is energised"
  phrases:
    "powered": "poweredUp"
"""


class AKeyWrittenTwiceIsRefused(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = pathlib.Path(temporary.name)

    def write(self, name: str, text: str) -> pathlib.Path:
        path = self.dir / name
        path.write_text(text, encoding="utf-8")
        return path

    def test_a_symbol_written_twice_in_a_value_space_is_refused_in_yaml(self):
        path = self.write("model.yaml", MODEL.replace(
            "      OCCUPIED: 2\n", "      OCCUPIED: 2\n      APPROACHING: 0\n"))
        with self.assertRaises(PackError) as caught:
            load_model([path])
        message = str(caught.exception)
        self.assertIn(str(path), message)
        self.assertIn("'APPROACHING' is written twice", message)
        self.assertIn("line 10 (first written on line 8)", message)

    def test_a_symbol_written_twice_in_a_flow_mapping_is_refused(self):
        path = self.write("model.yaml", MODEL.replace(
            "    values:\n      CLEAR: 0\n      APPROACHING: 1\n      OCCUPIED: 2\n",
            "    values: {CLEAR: 0, APPROACHING: 1, APPROACHING: 0}\n"))
        with self.assertRaises(PackError) as caught:
            load_model([path])
        self.assertIn("'APPROACHING' is written twice", str(caught.exception))

    def test_a_symbol_written_twice_is_refused_in_json_too(self):
        path = self.write("model.json", (
            '{"version": 1, "entries": [{"address": "plant/in/train-approach", '
            '"role": "input", "names": ["IN_TrainApproach"], '
            '"values": {"CLEAR": 0, "APPROACHING": 1, "APPROACHING": 0}}]}'))
        with self.assertRaises(PackError) as caught:
            load_model([path])
        self.assertIn("'APPROACHING' is written twice", str(caught.exception))

    def test_an_entry_that_writes_its_role_twice_is_refused(self):
        path = self.write("model.yaml", MODEL + "    role: output\n")
        with self.assertRaises(PackError) as caught:
            load_model([path])
        self.assertIn("'role' is written twice", str(caught.exception))

    def test_a_phrase_written_twice_cannot_change_its_reading(self):
        path = self.write("conventions.yaml",
                          CONVENTIONS + '    "powered": "!poweredUp"\n')
        with self.assertRaises(PackError) as caught:
            load_conventions([path])
        message = str(caught.exception)
        self.assertIn("'powered' is written twice", message)
        self.assertIn("line 10 (first written on line 9)", message)

    def test_two_spellings_of_one_phrase_in_one_file_are_refused(self):
        """Past what a file reader can see: the table keys a phrase by its
        lower-cased, trimmed spelling, so these are one key to it."""
        path = self.write("conventions.yaml",
                          CONVENTIONS + '    "Powered ": "!poweredUp"\n')
        with self.assertRaises(PackError) as caught:
            load_conventions([path])
        message = str(caught.exception)
        self.assertIn("'powered' and 'Powered '", message)
        self.assertIn("one phrase", message)

    def test_a_later_file_may_still_replace_an_earlier_files_reading(self):
        """The layering the loader documents, which is not a repeated key."""
        first = self.write("a.yaml", CONVENTIONS)
        second = self.write("b.yaml", CONVENTIONS.replace('"powered": "poweredUp"',
                                                           '"powered": "!poweredUp"'))
        conventions = load_conventions([first, second])
        self.assertEqual("!poweredUp", conventions.precondition_phrases["powered"])

    def test_a_binding_that_repeats_a_key_is_refused(self):
        path = self.write("binding.yaml", "version: 1\nversion: 1\n")
        with self.assertRaises(PackError) as caught:
            read_binding(path)
        self.assertIn("'version' is written twice", str(caught.exception))

    def test_an_owners_answer_written_twice_in_one_decision_is_refused(self):
        path = self.write("decisions.json", (
            '{"record": "sce-decision-record", "v": 1, '
            '"specification": {"doc_id": "retry", "rev": "1"}, '
            '"decisions": [{"id": "D1", "question": "Who is the caller?", '
            '"answer": "the parent", "answer": "another processor"}]}'))
        with self.assertRaises(DecisionRecordError) as caught:
            load_record(path)
        message = str(caught.exception)
        self.assertIn("'answer' is written twice", message)
        self.assertNotIn("not well-formed", message)


class WhatIsStillRead(unittest.TestCase):
    """The strict reader refuses a repeat and nothing else."""

    def test_the_same_key_in_two_different_mappings_is_not_a_repeat(self):
        self.assertEqual({"a": {"x": 1}, "b": {"x": 2}},
                         read_yaml("a: {x: 1}\nb: {x: 2}\n"))
        self.assertEqual({"a": {"x": 1}, "b": {"x": 2}},
                         read_json('{"a": {"x": 1}, "b": {"x": 2}}'))

    def test_a_quoted_key_and_the_boolean_it_resembles_are_two_keys(self):
        """`"ON"` is a symbol and bare ON is a boolean: the loader has its own
        refusal for the boolean, and this reader must not mistake the pair."""
        self.assertEqual({"ON": 1, True: 0}, read_yaml('{"ON": 1, ON: 0}'))

    def test_a_merge_key_may_be_overridden_as_yaml_defines(self):
        document = read_yaml("base: &base {x: 1, y: 2}\nuse:\n  <<: *base\n  x: 9\n")
        self.assertEqual({"x": 9, "y": 2}, document["use"])

    def test_a_repeat_inside_a_list_item_is_found(self):
        with self.assertRaises(RepeatedKey) as caught:
            read_yaml("- {a: 1, a: 2}\n")
        self.assertEqual("a", caught.exception.key)

    def test_a_malformed_document_is_still_malformed_not_repeated(self):
        import json

        import yaml

        with self.assertRaises(yaml.YAMLError):
            read_yaml("a: [1, 2\n")
        with self.assertRaises(json.JSONDecodeError):
            read_json('{"a": ')


if __name__ == "__main__":
    unittest.main()
