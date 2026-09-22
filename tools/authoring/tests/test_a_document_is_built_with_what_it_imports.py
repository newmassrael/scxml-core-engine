"""A document is built together with what it imports, or it cannot be run.

The product's generator builds one document per run. The code it writes names
each `<sce:import>` as a sibling it expects to find, and the generator records
those siblings only as build dependencies. So a caller that builds a document
in order to RUN it owes the whole import closure.

`verify` built the root alone. A document importing so much as an enumeration
therefore died on its own import line -- `cannot import name ...` from the
generated module -- before a single case ran, and over MCP the caller got a
traceback naming the generated module rather than the import that was missing.
Measured on the first real document driven over MCP: a component whose inputs
are enumerations, which is the ordinary case rather than a corner.

Two halves are asserted:

  the closure    every import, and every import of an import, once each,
                 resolved against the document that names it -- and an import
                 that is not there is refused naming who asked for it
  the run        a document typed by an imported enumeration runs its cases
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest

import yaml

from sce_author.check import imports_of
from sce_author.errors import PackError
from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import BINDING, Fixture

ENUM = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="supply_level" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="NONE" value="0"/>
      <sce:variant name="LOW" value="1"/>
      <sce:variant name="HIGH" value="2"/>
      <sce:variant name="MAX" value="3"/>
    </data>
  </datamodel>
</scxml>
"""

# The fixture's document, with its input typed by an IMPORTED enumeration
# rather than read as a truth value. The import sits in a subdirectory so the
# path is resolved against the importing document, not the caller.
IMPORTING = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="fixture_importing">
  <sce:import as="Level" src="levels/supply_level.scxml" kind="enum"/>
  <datamodel>
    <data id="mode" sce:type="enum:Level" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="mode === Level.HIGH ? 1 : 0"/>
  </datamodel>
</scxml>
"""


def document(imports=(), name="doc"):
    lines = "\n".join(f'  <sce:import as="I{i}" src="{src}" kind="enum"/>'
                      for i, src in enumerate(imports))
    return (f'<?xml version="1.0" encoding="UTF-8"?>\n'
            f'<scxml xmlns="http://www.w3.org/2005/07/scxml" '
            f'xmlns:sce="http://sce.dev/ext" version="1.0" name="{name}">\n'
            f'{lines}\n</scxml>\n')


class TheClosureIsEveryImportOnce(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def write(self, relative, text):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        return path

    def test_a_document_that_imports_nothing_owes_nothing(self):
        self.assertEqual([], imports_of(self.write("a.scxml", document())))

    def test_an_import_of_an_import_is_followed_relative_to_its_importer(self):
        """⚠ `c.scxml` is named from INSIDE `sub/`, as `../c.scxml`. Resolving
        against the caller's directory instead would look for it one level
        too high and call it missing."""
        a = self.write("a.scxml", document(["sub/b.scxml"]))
        b = self.write("sub/b.scxml", document(["../c.scxml"]))
        c = self.write("c.scxml", document())
        self.assertEqual([b.resolve(), c.resolve()], imports_of(a))

    def test_a_cycle_ends_and_the_root_is_not_its_own_import(self):
        a = self.write("a.scxml", document(["b.scxml"]))
        b = self.write("b.scxml", document(["a.scxml"]))
        self.assertEqual([b.resolve()], imports_of(a))

    def test_two_paths_to_one_document_build_it_once(self):
        a = self.write("a.scxml", document(["b.scxml", "c.scxml"]))
        b = self.write("b.scxml", document(["c.scxml"]))
        c = self.write("c.scxml", document())
        self.assertEqual([b.resolve(), c.resolve()], imports_of(a))

    def test_a_missing_import_names_the_document_that_asked(self):
        a = self.write("a.scxml", document(["sub/b.scxml"]))
        self.write("sub/b.scxml", document(["gone.scxml"]))
        with self.assertRaises(PackError) as caught:
            imports_of(a)
        message = str(caught.exception)
        self.assertIn("b.scxml", message)
        self.assertIn("'gone.scxml'", message)

    def test_an_import_without_a_source_is_refused(self):
        a = self.write("a.scxml", document()
                       .replace("</scxml>", '  <sce:import as="X" kind="enum"/>\n</scxml>'))
        with self.assertRaises(PackError) as caught:
            imports_of(a)
        self.assertIn("src", str(caught.exception))


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class ADocumentTypedByAnImportRuns(Fixture):
    def test_its_cases_run_rather_than_the_import_failing(self):
        """The case that crashed: an input typed by an imported enumeration.
        The fixture's one case drives the supply HIGH and expects the lamp ON,
        so a run that happened and computed rightly passes it."""
        levels = self.root / "levels"
        levels.mkdir()
        (levels / "supply_level.scxml").write_text(ENUM, encoding="utf-8")
        (self.root / "importing.scxml").write_text(IMPORTING, encoding="utf-8")
        binding = {
            "version": 1,
            "document": "importing.scxml",
            "inputs": {"mode": {"address": "Plant.Input.SupplyMode"}},
            "outputs": BINDING["outputs"],
        }
        path = self.root / "importing.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        result = verify(self.pack(), path)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(0, result.failed)
        self.assertGreater(result.passed, 0)


if __name__ == "__main__":
    unittest.main()
