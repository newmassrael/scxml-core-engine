"""A transform's document can start from the model, and `check` says what is left.

Measured 2026-09-26: a model given the binding skeleton still wrote the
document's shell wrong three rounds running -- no `sce:kind`, the `sce` prefix
bound to a made-up namespace, `<if>` straight inside a state -- and `check`
answered in a statechart's terms ("no input rule names an `event`"), which
steered it further from the transform it needed. So:

  - `scaffold --kind transform` writes the document beside the binding, with
    the binding's names and every output declared and uncomputed;
  - `check` refuses each uncomputed transform output (the product refuses the
    document without an `expr`), a `sce` prefix bound to another namespace,
    and says, when a document never chose a kind, that it is being read as a
    statechart and what a transform looks like.
"""

from __future__ import annotations

import pathlib
import re
import shutil
import tempfile
import unittest
import xml.etree.ElementTree as ET

import yaml

from sce_author import mcp
from sce_author.check import check
from sce_author.pack import load_pack
from sce_author.scaffold import SCE_NAMESPACE, ScaffoldError, write

HERE = pathlib.Path(__file__).resolve().parent
PACK = HERE / "fixtures" / "crossing"
SCE = "{" + SCE_NAMESPACE + "}"
SCXML = "{http://www.w3.org/2005/07/scxml}"


class ATransformStartsFromTheModel(unittest.TestCase):
    def setUp(self):
        self.pack = load_pack(PACK)
        self.dir = pathlib.Path(tempfile.mkdtemp(prefix="sce_scaffold_transform_"))
        self.binding = self.dir / "controller.binding.yaml"
        self.document = self.dir / "controller.scxml"

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def scaffold(self):
        write(self.pack, "controller.scxml", self.binding, "periodic", "transform")
        return (yaml.safe_load(self.binding.read_text(encoding="utf-8")),
                ET.parse(self.document).getroot())

    def refusals(self):
        return {(f.where, f.detail) for f in check(self.pack, self.binding)}

    def test_the_document_is_a_transform_in_sces_namespace(self):
        _, root = self.scaffold()
        self.assertEqual("transform", root.get(f"{SCE}kind"))
        self.assertIn(f'xmlns:sce="{SCE_NAMESPACE}"', self.document.read_text())

    def test_its_variables_are_the_bindings_rules_and_no_output_is_computed(self):
        binding, root = self.scaffold()
        data = {d.get("id"): d for d in root.iter(f"{SCXML}data")}
        self.assertEqual(set(binding["inputs"]),
                         {i for i, d in data.items() if d.get(f"{SCE}direction") == "in"})
        self.assertEqual(set(binding["outputs"]),
                         {i for i, d in data.items() if d.get(f"{SCE}direction") == "out"})
        for ident, d in data.items():
            self.assertTrue(d.get(f"{SCE}type"), f"{ident} has no sce:type")
            self.assertIsNone(d.get("expr"), f"{ident} carries a decision: {d.get('expr')}")

    def test_check_names_every_output_still_to_compute_and_nothing_about_the_shell(self):
        binding, _ = self.scaffold()
        got = self.refusals()
        uncomputed = {w.split(" ", 1)[1] for w, m in got if "computes nothing" in m}
        self.assertEqual(set(binding["outputs"]), uncomputed)
        for where, message in got:
            self.assertNotIn("prefix `sce`", message)
            self.assertNotIn("does not say what kind", message)

    def test_an_output_with_an_expression_is_no_longer_refused_for_it(self):
        binding, _ = self.scaffold()
        first = next(iter(binding["outputs"]))
        text = self.document.read_text(encoding="utf-8")
        text = text.replace(f'<data id="{first}" ', f'<data id="{first}" expr="0" ', 1)
        self.document.write_text(text, encoding="utf-8")
        uncomputed = {w.split(" ", 1)[1] for w, m in self.refusals() if "computes nothing" in m}
        self.assertNotIn(first, uncomputed)
        self.assertEqual(set(binding["outputs"]) - {first}, uncomputed)

    def test_an_existing_document_is_never_overwritten_and_nothing_is_written(self):
        self.document.write_text("<scxml/>", encoding="utf-8")
        with self.assertRaises(ScaffoldError):
            write(self.pack, "controller.scxml", self.binding, None, "transform")
        self.assertEqual("<scxml/>", self.document.read_text())
        self.assertFalse(self.binding.exists(), "the binding was written alone")

    def test_only_a_transform_has_a_skeleton(self):
        with self.assertRaises(ScaffoldError):
            write(self.pack, "controller.scxml", self.binding, None, "statechart")
        self.assertFalse(self.binding.exists())

    def test_without_a_kind_only_the_binding_is_written(self):
        write(self.pack, "controller.scxml", self.binding)
        self.assertTrue(self.binding.exists())
        self.assertFalse(self.document.exists())

    def test_a_sce_prefix_bound_elsewhere_is_refused_by_the_uri_it_names(self):
        self.scaffold()
        text = self.document.read_text(encoding="utf-8")
        self.document.write_text(text.replace(SCE_NAMESPACE, "http://example.invalid/sce"),
                                 encoding="utf-8")
        named = [m for w, m in self.refusals() if "prefix `sce`" in m]
        self.assertEqual(1, len(named))
        self.assertIn("http://example.invalid/sce", named[0])
        self.assertIn(SCE_NAMESPACE, named[0])

    def test_a_document_that_never_chose_a_kind_is_told_what_a_transform_is(self):
        self.scaffold()
        text = self.document.read_text(encoding="utf-8")
        self.document.write_text(text.replace(' sce:kind="transform"', ""), encoding="utf-8")
        undriven = [m for w, m in self.refusals() if "no input rule names an `event`" in m]
        self.assertEqual(1, len(undriven))
        self.assertIn('sce:kind="transform"', undriven[0])

    def test_a_statechart_that_chose_its_kind_is_not_sent_to_a_transform(self):
        self.scaffold()
        text = self.document.read_text(encoding="utf-8")
        self.document.write_text(text.replace('sce:kind="transform"', 'sce:kind="statechart"'),
                                 encoding="utf-8")
        undriven = [m for w, m in self.refusals() if "no input rule names an `event`" in m]
        self.assertEqual(1, len(undriven))
        self.assertNotIn("does not say what kind", undriven[0])

    def test_the_server_writes_both_when_asked_for_a_transform(self):
        result = mcp.call_tool("scaffold", {"pack": str(PACK), "document": "controller.scxml",
                                            "binding": str(self.binding), "kind": "transform"})
        self.assertFalse(result.get("isError"), result)
        self.assertTrue(self.binding.is_file())
        self.assertTrue(self.document.is_file())
        self.assertTrue(re.search(r'sce:kind="transform"', self.document.read_text()))


if __name__ == "__main__":
    unittest.main()
