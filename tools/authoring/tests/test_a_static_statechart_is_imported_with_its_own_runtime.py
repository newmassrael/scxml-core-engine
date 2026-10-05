"""A statechart under the static data model is judged with the runtime it was generated for.

A document that declares `datamodel="sce-static"` is lowered to Python by the forge
generator, and what it writes imports `sce_forge_runtime`. The product's other Python
runtime, `sce_runtime`, serves the statecharts of every other data model, and `lowering.load`
put only that one on the path. Measured 2026-10-05, three of four documents a writer built
under the static data model failed to import and were reported as "the generator wrote code
it cannot run -- a defect in the generator's lowering": the code was right, and the judge's
path was missing a directory.

So `lowering.load` puts both runtimes on the path. Asserted here:

  the path   the forge runtime's directory is derived from this tree, as the other is
  the load   a statechart generated under `sce-static` is imported, and `sce_forge_runtime`
             resolves from the module that was loaded
"""

from __future__ import annotations

import importlib
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

from sce_author import lowering
from sce_author.verify import _default_codegen, generate

STATIC = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="sce-static" initial="idle" sce:kind="statechart">
  <datamodel>
    <data id="seconds" sce:type="int32" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="engine.on" target="counting"/>
  </state>
  <state id="counting">
    <onentry><send id="tick" event="tick" delay="1s"/></onentry>
    <transition event="tick" target="counting">
      <assign location="seconds" expr="seconds + 1"/>
    </transition>
    <transition event="engine.off" target="idle"/>
  </state>
</scxml>
"""


def generated_modules(*packages: str) -> list[str]:
    """The loaded modules that are one of these packages or inside one: by the name before the
    first dot, and never by a prefix. `builtins` begins with `built`, and a loop that took every
    name starting with the generated package's name deleted the interpreter's own module."""
    return [name for name in sys.modules if name.split(".")[0] in packages]


class TheForgeRuntimeIsKnown(unittest.TestCase):
    def test_it_is_derived_from_this_tree(self):
        runtime = lowering.default_forge_runtime()
        self.assertEqual("forge-runtime", runtime.name)
        self.assertTrue((runtime / "sce_forge_runtime").is_dir(), runtime)

    def test_it_is_not_the_other_runtime(self):
        self.assertNotEqual(lowering.default_runtime(), lowering.default_forge_runtime())


@unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
class AStaticStatechartIsImported(unittest.TestCase):
    def build(self, tmp):
        root = pathlib.Path(tmp)
        document = root / "ticker.scxml"
        document.write_text(STATIC, encoding="utf-8")
        into = root / "built"
        built = generate(document, _default_codegen(), into, backend="python")
        self.assertFalse(built.refusal, built.refusal)
        return document, into

    def load(self, document, into):
        """Load with the path and the modules as they were, whatever happens."""
        before = list(sys.path)
        try:
            return lowering.load(into, document)
        finally:
            sys.path[:] = before
            for name in generated_modules("sce_forge_runtime", "built", "ticker"):
                del sys.modules[name]

    def test_the_module_loads_and_its_runtime_resolves(self):
        with tempfile.TemporaryDirectory() as tmp:
            document, into = self.build(tmp)
            self.assertIsNotNone(self.load(document, into))

    def test_loading_leaves_the_interpreters_own_modules_alone(self):
        """A name that only begins like the generated package is not part of it. `builtins` begins
        with `built`: a module table that lost it hands the next `import builtins` a new module, and
        every later `mock.patch("builtins....")` in the process patches something nothing uses."""
        with tempfile.TemporaryDirectory() as tmp:
            document, into = self.build(tmp)
            before = sys.modules["builtins"]
            self.load(document, into)
            self.assertIs(before, sys.modules.get("builtins"))

    def test_without_the_runtime_on_the_path_the_import_fails(self):
        """The control. Without it the test above proves nothing: a module that imported
        for some other reason would pass it too."""
        with tempfile.TemporaryDirectory() as tmp:
            document, into = self.build(tmp)
            # ⚠ Another test in the same process may already have put the runtime on the path or
            # imported it, and then nothing here could fail. Start from a path and a module table
            # that have neither, and put both back whatever happens.
            real = lowering.default_forge_runtime().resolve()
            path_before, modules_before = list(sys.path), dict(sys.modules)
            sys.path[:] = [p for p in sys.path if pathlib.Path(p or ".").resolve() != real]
            for name in generated_modules("sce_forge_runtime"):
                del sys.modules[name]
            try:
                with mock.patch.object(lowering, "default_forge_runtime",
                                       return_value=pathlib.Path(tmp) / "nowhere"):
                    with self.assertRaises(Exception) as caught:
                        self.load(document, into)
                self.assertIn("sce_forge_runtime", str(caught.exception))
            finally:
                sys.path[:] = path_before
                sys.modules.update(modules_before)


if __name__ == "__main__":
    unittest.main()
