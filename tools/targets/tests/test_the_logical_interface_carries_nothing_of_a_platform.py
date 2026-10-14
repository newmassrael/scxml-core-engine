"""The split is only worth having if the half every target shares has none of a platform in it.

What a platform needs -- an address, the record field written, its number for a symbol, its
table of symbols -- is the profile's. The interface is the schema's, and the schema refuses all
of them, so a target built from the interface cannot have read one.

Every refusal is paired with the nearest interface that must not be refused, so a schema that
refuses everything fails here too.
"""

from __future__ import annotations

import copy
import unittest

import yaml
from jsonschema import Draft202012Validator

from tests.support import Working


class TheInterfaceIsSplitFromThePlatform(Working):
    def test_the_split_interface_is_the_schemas(self):
        interface = self.interface(self.split())
        errors = list(Draft202012Validator(self.schema()).iter_errors(interface))
        self.assertEqual([], [e.message for e in errors])

    def test_no_address_and_no_number_for_a_symbol_is_in_it(self):
        text = (self.split() / "logical" / "pump.interface.yaml").read_text(encoding="utf-8")
        self.assertNotIn("tank/", text, "an address is the profile's")
        interface = yaml.safe_load(text)
        for name, signal in interface["signals"].items():
            self.assertIsInstance(signal.get("values", []), list,
                                  f"{name}: symbols are listed, never numbered")

    def test_what_the_platform_needs_is_in_the_profile(self):
        profile = yaml.safe_load(
            (self.split() / "profile" / "pump.yaml").read_text(encoding="utf-8"))
        self.assertEqual("tank/in/mode", profile["signals"]["IN_Mode"]["address"])
        self.assertEqual({"IDLE": "IDLE", "RUN": "RUN"}, profile["outputs"]["pump"]["map"])
        self.assertEqual("value", profile["outputs"]["pump"]["field"])

    def test_the_interface_names_its_document_relative_to_itself(self):
        interface = self.interface(self.split())
        self.assertFalse(interface["document"].startswith("/"), interface["document"])
        self.assertNotIn(str(self.root), interface["document"])


class TheSchemaRefusesAPlatformsPart(Working):
    def setUp(self):
        super().setUp()
        self.base = self.interface(self.split())
        self.validator = Draft202012Validator(self.schema())

    def refused(self, mutate) -> bool:
        interface = copy.deepcopy(self.base)
        mutate(interface)
        return bool(list(self.validator.iter_errors(interface)))

    def test_the_unchanged_interface_is_not_refused(self):
        """The discriminator: without it every case below passes a schema that refuses all."""
        self.assertFalse(self.refused(lambda i: None))

    def test_a_number_for_a_symbol(self):
        self.assertTrue(self.refused(
            lambda i: i["signals"]["IN_Mode"].update(values={"AUTO": 0, "MANUAL": 1, "OFF": 2})))

    def test_an_address_on_a_signal(self):
        self.assertTrue(self.refused(lambda i: i["signals"]["IN_Mode"].update(address="tank/in/mode")))

    def test_a_record_field_on_an_output(self):
        self.assertTrue(self.refused(lambda i: i["outputs"]["pump"].update(field="value")))

    def test_a_platform_table_on_an_output(self):
        self.assertTrue(self.refused(lambda i: i["outputs"]["pump"].update(map={"IDLE": "IDLE"})))

    def test_an_output_that_says_two_things(self):
        self.assertTrue(self.refused(lambda i: i["outputs"]["fill"].update(values=["1"])))

    def test_an_output_that_says_nothing(self):
        self.assertTrue(self.refused(lambda i: i["outputs"].update(fill={"note": "nothing"})))

    def test_a_kind_the_targets_do_not_build(self):
        self.assertTrue(self.refused(lambda i: i.update(kind="periodic")))


if __name__ == "__main__":
    unittest.main()
