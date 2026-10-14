"""A statechart splits, joins, and is judged as `verify` judges it.

Every document the real corpus offered the targets was a transform, so the statechart shapes of
the schema and of the standalone generator (an input that becomes an event, an output that
leaves as a send and maps to symbols) were written from the tool's code and never run. The
made-up lamp is the smallest statechart that exercises them: a button whose press and release
reach the machine as events, and a lamp that leaves as an announcement.

Needs the product's generator, a C++ compiler and the runtime sources for the cases that build
a library; without them those cases are skipped, and the lane that builds them refuses to run
without them.
"""

from __future__ import annotations

import copy
import unittest

import yaml
from jsonschema import Draft202012Validator
from sce_author.pack import load_pack
from sce_author.verify import verify

from tests.support import (STATECHART_FIXTURE, Working, codegen_is_built, compiler,
                           runtime_is_present)


class Lamp(Working):
    fixture = STATECHART_FIXTURE
    component = "lamp"


class TheStatechartInterfaceIsPlatformFree(Lamp):
    def test_the_split_interface_is_the_schemas(self):
        interface = self.interface(self.split())
        errors = list(Draft202012Validator(self.schema()).iter_errors(interface))
        self.assertEqual([], [e.message for e in errors])
        self.assertEqual("statechart", interface["kind"])

    def test_no_address_and_no_number_is_in_it(self):
        text = (self.split() / "logical" / "lamp.interface.yaml").read_text(encoding="utf-8")
        self.assertNotIn("desk/", text, "an address is the profile's")
        for name, signal in yaml.safe_load(text)["signals"].items():
            self.assertIsInstance(signal.get("values", []), list, f"{name}: symbols, never numbers")

    def test_what_the_platform_needs_is_in_the_profile(self):
        profile = yaml.safe_load((self.split() / "profile" / "lamp.yaml").read_text(encoding="utf-8"))
        self.assertEqual("desk/in/button", profile["signals"]["IN_Button"]["address"])
        self.assertEqual("desk/out/lamp", profile["outputs"]["bulb"]["address"])
        self.assertEqual({"lamp.dark": "DARK", "lamp.lit": "LIT"}, profile["outputs"]["bulb"]["map"])

    def test_the_schema_refuses_what_the_platform_owns_here_too(self):
        base = self.interface(self.split())
        validator = Draft202012Validator(self.schema())
        self.assertEqual([], list(validator.iter_errors(base)), "the discriminator")
        numbered = copy.deepcopy(base)
        numbered["signals"]["IN_Button"]["values"] = {"RELEASED": 0, "PRESSED": 1}
        self.assertTrue(list(validator.iter_errors(numbered)), "a number for a symbol")
        addressed = copy.deepcopy(base)
        addressed["signals"]["IN_Button"]["address"] = "desk/in/button"
        self.assertTrue(list(validator.iter_errors(addressed)), "an address on a signal")


class TheStatechartJoinsBack(Lamp):
    def test_a_split_joins_back_to_the_binding(self):
        split_dir = self.split()
        joined = self.pack / "joined.binding.yaml"  # beside the original, so `document:` reads alike
        done = self.run_module("sce_targets.split", "join", split_dir / "logical" / "lamp.interface.yaml",
                               split_dir / "profile" / "lamp.yaml", joined)
        self.assertEqual(0, done.returncode, done.stderr)
        self.assertEqual(yaml.safe_load(self.binding.read_text(encoding="utf-8")),
                         yaml.safe_load(joined.read_text(encoding="utf-8")))


@unittest.skipUnless(codegen_is_built(), "the product's code generator is not built in this tree")
@unittest.skipUnless(compiler(), "no C++ compiler")
@unittest.skipUnless(runtime_is_present(), "the runtime sources and headers are not in this tree")
class TheStatechartLibraryAgreesWithVerify(Lamp):
    def said(self) -> dict:
        result = verify(load_pack(self.pack), self.binding)
        self.assertFalse(result.refusal, result.refusal)
        return {r.name: not r.failures and not r.refusal for r in result.results}

    def test_a_right_document_passes_every_case_in_both(self):
        ran = self.built_and_replayed(self.split())
        self.assertEqual(0, ran["rc"], ran["log"])
        self.assertEqual(self.said(), ran["cases"])
        self.assertEqual(5, len(ran["cases"]))
        self.assertTrue(all(ran["cases"].values()))

    def test_a_document_made_wrong_fails_the_same_cases_in_both(self):
        document = self.pack / "lamp.scxml"
        text = document.read_text(encoding="utf-8")
        wrong = text.replace('<send event="lamp.lit" type="x-sce-host"/>\n    </transition>\n    <transition event="button.released">\n      <send event="lamp.dark"',
                             '<send event="lamp.dark" type="x-sce-host"/>\n    </transition>\n    <transition event="button.released">\n      <send event="lamp.dark"', 1)
        self.assertNotEqual(text, wrong, "the mutation must change the document")
        document.write_text(wrong, encoding="utf-8")

        said = self.said()
        ran = self.built_and_replayed(self.split())
        self.assertEqual(said, ran["cases"])
        self.assertEqual(1, ran["rc"])
        self.assertIn(False, said.values(), "the mutation must be one a case can see")


if __name__ == "__main__":
    unittest.main()
